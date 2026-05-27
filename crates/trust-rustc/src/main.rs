use std::collections::{hash_map::DefaultHasher, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use trust_core::{
    metadata::{parse_metadata_line, TrustMetadata},
    solver::VerificationOptions,
    verifier::{
        verify_totals_with_options, verify_totals_with_semantics, TrustFunctionSemantics,
        VerificationError,
    },
};
use z3::{ast::Bool, Config, SatResult, Solver};

mod semantic;

fn main() {
    cleanup_z3_trace_file();
    let code = match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error[trust]: {err}");
            1
        }
    };
    cleanup_z3_trace_file();
    process::exit(code);
}

fn run() -> Result<i32, String> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let (rustc, rustc_args) = split_rustc_args(args);
    let metadata_path = metadata_path();
    let config = TrustConfig::load()?;

    let status = Command::new(&rustc)
        .args(&rustc_args)
        .env("TRUST_RUSTC_ACTIVE", "1")
        .env("TRUST_RUSTC_VERSION", env!("CARGO_PKG_VERSION"))
        .env("TRUST_METADATA_OUT", &metadata_path)
        .env("TRUST_ASSERTION_POLICY", &config.assertions)
        .env("TRUST_SOLVER", &config.solver)
        .status()
        .map_err(|err| format!("failed to invoke rustc through trust-rustc: {err}"))?;

    if !status.success() {
        return Ok(exit_code(status));
    }

    let metadata = read_metadata(&metadata_path)?;
    reject_ambiguous_metadata_paths(&metadata)?;
    emit_config_warnings(&metadata, &config);
    let semantic_views = if metadata_has_verification_item(&metadata) {
        semantic::maybe_extract_semantic_views(&rustc, &rustc_args, &metadata)?
    } else {
        semantic::SemanticViews::default()
    };
    let semantics = semantic_views.semantics;
    let semantic_source_spans = semantic_views.source_spans;
    let totals = metadata
        .iter()
        .filter(|item| item.item_kind == "total")
        .count();
    let verification_items = metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
        .count();
    let cache_stats = if verification_items == 0 {
        CacheStats { hits: 0, misses: 0 }
    } else {
        let cache_context = CacheContext::from_rustc_args(&rustc, &rustc_args, &config)?;
        verify_metadata(
            &metadata,
            &semantics,
            &semantic_source_spans,
            &cache_context,
            &config,
        )?
    };

    if deterministic_test_mode() && totals > 0 {
        eprintln!(
            "trust: discovered {totals} total function{}",
            plural(totals)
        );
        eprintln!("trust: proved {totals} total function{}", plural(totals));
        eprintln!(
            "trust: verified {totals} function{}; cache hits {}; cache misses {}",
            plural(totals),
            cache_stats.hits,
            cache_stats.misses
        );
    }

    let _ = fs::remove_file(&metadata_path);
    Ok(exit_code(status))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CacheStats {
    hits: usize,
    misses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustConfig {
    assertions: String,
    solver: String,
    timeout_ms: u64,
    cache: String,
    silence_assume_warning: bool,
    fingerprint: String,
}

impl TrustConfig {
    fn load() -> Result<Self, String> {
        let mut config = Self {
            assertions: "always".to_string(),
            solver: env::var("TRUST_SOLVER").unwrap_or_else(|_| "mock".to_string()),
            timeout_ms: 5000,
            cache: "local".to_string(),
            silence_assume_warning: false,
            fingerprint: "missing-config".to_string(),
        };

        let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from) else {
            config.validate()?;
            return Ok(config);
        };
        let manifest = manifest_dir.join("Cargo.toml");
        let Ok(contents) = fs::read_to_string(&manifest) else {
            config.validate()?;
            return Ok(config);
        };

        let mut in_trust_section = false;
        let mut trust_lines = Vec::new();
        for raw_line in contents.lines() {
            let line = raw_line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_trust_section = line == "[package.metadata.trust]";
                continue;
            }
            if !in_trust_section {
                continue;
            }

            trust_lines.push(line.to_string());
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("invalid Trust config entry `{line}`"));
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "assertions" => config.assertions = parse_config_string(value, key)?,
                "solver" => config.solver = parse_config_string(value, key)?,
                "timeout_ms" => config.timeout_ms = parse_timeout_ms(value)?,
                "cache" => config.cache = parse_config_string(value, key)?,
                "silence_assume_warning" => {
                    config.silence_assume_warning = parse_config_bool(value, key)?
                }
                _ => {}
            }
        }

        if !trust_lines.is_empty() {
            config.fingerprint = trust_lines.join("\n");
        }
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if !matches!(self.assertions.as_str(), "always" | "debug" | "assume") {
            return Err(format!("invalid assertion policy `{}`", self.assertions));
        }
        if !matches!(self.solver.as_str(), "mock" | "z3") {
            return Err(format!("unsupported solver `{}`", self.solver));
        }
        if self.timeout_ms == 0 {
            return Err("invalid timeout_ms `0`".to_string());
        }
        if self.cache != "local" {
            return Err(format!("unsupported cache mode `{}`", self.cache));
        }

        Ok(())
    }
}

fn parse_config_string(value: &str, key: &str) -> Result<String, String> {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
        .ok_or_else(|| format!("Trust config `{key}` must be a string"))
}

fn parse_timeout_ms(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid timeout_ms `{value}`"))
}

fn parse_config_bool(value: &str, key: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Trust config `{key}` must be a boolean")),
    }
}

fn emit_config_warnings(metadata: &[trust_core::metadata::TrustMetadata], config: &TrustConfig) {
    if config.assertions == "assume"
        && !config.silence_assume_warning
        && metadata_has_verification_item(metadata)
    {
        eprintln!(
            "warning[trust]: assertions = \"assume\" disables runtime checks for executable Trust contracts"
        );
        eprintln!(
            "help[trust]: set `silence_assume_warning = true` in [package.metadata.trust] if this is intentional"
        );
    }
}

fn metadata_has_verification_item(metadata: &[trust_core::metadata::TrustMetadata]) -> bool {
    metadata
        .iter()
        .any(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheContext {
    rustc_version: String,
    target_triple: String,
    pointer_width: String,
    endianness: String,
    target_features: Vec<String>,
    cargo_features: Vec<String>,
    solver_name: String,
    solver_version: String,
    solver_options: String,
    trust_config_fingerprint: String,
}

impl CacheContext {
    fn from_rustc_args(
        rustc: &OsString,
        rustc_args: &[OsString],
        config: &TrustConfig,
    ) -> Result<Self, String> {
        let rustc_version = if deterministic_test_mode() {
            env::var("TRUST_TEST_RUSTC_VERSION").unwrap_or_else(|_| "rustc-test".to_string())
        } else {
            rustc_verbose_version(rustc).unwrap_or_else(|| "unknown-rustc-version".to_string())
        };
        let target_triple = if deterministic_test_mode() {
            env::var("TRUST_TEST_TARGET_TRIPLE").ok()
        } else {
            None
        }
        .or_else(|| target_arg(rustc_args))
        .or_else(|| host_target_from_verbose(&rustc_version))
        .unwrap_or_else(|| "unknown-target".to_string());
        let target_cfg = if deterministic_test_mode() {
            TargetCfg {
                pointer_width: env::var("TRUST_TEST_TARGET_POINTER_WIDTH")
                    .unwrap_or_else(|_| "unknown".to_string()),
                endianness: env::var("TRUST_TEST_TARGET_ENDIANNESS")
                    .unwrap_or_else(|_| "unknown".to_string()),
                target_features: Vec::new(),
            }
        } else {
            rustc_target_cfg(rustc, &target_triple).unwrap_or_default()
        };

        let mut target_features = target_cfg.target_features;
        target_features.extend(target_feature_args(rustc_args));
        target_features.sort();
        target_features.dedup();

        let mut cargo_features = cargo_feature_cfgs(rustc_args);
        cargo_features.sort();
        cargo_features.dedup();

        Ok(Self {
            rustc_version,
            target_triple,
            pointer_width: target_cfg.pointer_width,
            endianness: target_cfg.endianness,
            target_features,
            cargo_features,
            solver_name: config.solver.clone(),
            solver_version: solver_version(config)?,
            solver_options: format!(
                "timeout_ms={};{}",
                config.timeout_ms,
                env::var("TRUST_SOLVER_OPTIONS").unwrap_or_default()
            ),
            trust_config_fingerprint: config.fingerprint.clone(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TargetCfg {
    pointer_width: String,
    endianness: String,
    target_features: Vec<String>,
}

fn verify_metadata(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    semantic_source_spans: &[semantic::SemanticSourceSpan],
    cache_context: &CacheContext,
    config: &TrustConfig,
) -> Result<CacheStats, String> {
    let verification_items = metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
        .count();
    if verification_items == 0 {
        return Ok(CacheStats { hits: 0, misses: 0 });
    }
    check_solver_status(config)?;

    if let Some(cache_file) = cache_file(metadata, semantics, cache_context) {
        if matches!(
            fs::read_to_string(&cache_file),
            Ok(contents) if cache_entry_proved(&contents, metadata, semantics, cache_context)
        ) {
            return Ok(CacheStats {
                hits: verification_items,
                misses: 0,
            });
        }

        verify_all(
            metadata,
            semantics,
            semantic_source_spans,
            cache_context,
            config,
        )?;
        write_cache_entry(&cache_file, metadata, semantics, cache_context)?;
        return Ok(CacheStats {
            hits: 0,
            misses: verification_items,
        });
    }

    verify_all(
        metadata,
        semantics,
        semantic_source_spans,
        cache_context,
        config,
    )?;
    Ok(CacheStats {
        hits: 0,
        misses: verification_items,
    })
}

fn verify_all(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    semantic_source_spans: &[semantic::SemanticSourceSpan],
    cache_context: &CacheContext,
    config: &TrustConfig,
) -> Result<(), String> {
    let options = verification_options(config, cache_context);
    let result = match config.solver.as_str() {
        "mock" if semantics.is_empty() => verify_totals_with_options(metadata, options),
        "mock" => verify_totals_with_semantics(metadata, semantics, options),
        "z3" if semantics.is_empty() => verify_totals_with_options(metadata, options),
        "z3" => verify_totals_with_semantics(metadata, semantics, options),
        solver => return Err(format!("unsupported solver `{solver}`")),
    };

    result.map_err(|err| format_verification_error(&err, metadata, semantic_source_spans))
}

fn verification_options(config: &TrustConfig, cache_context: &CacheContext) -> VerificationOptions {
    let mut options = match config.solver.as_str() {
        "z3" => VerificationOptions::z3(config.timeout_ms),
        _ => VerificationOptions::default(),
    };

    if let Some(width) = target_pointer_width(&cache_context.pointer_width) {
        options = options.with_target_pointer_width(width);
    }

    options
}

fn target_pointer_width(pointer_width: &str) -> Option<u32> {
    match pointer_width {
        "16" => Some(16),
        "32" => Some(32),
        "64" => Some(64),
        _ => None,
    }
}

fn format_verification_error(
    err: &VerificationError,
    metadata: &[TrustMetadata],
    semantic_source_spans: &[semantic::SemanticSourceSpan],
) -> String {
    let mut diagnostic = err.to_string();
    let Some(item_name) = verification_error_item_name(err) else {
        return diagnostic;
    };
    let Some(item) = metadata.iter().find(|item| {
        matches!(item.item_kind.as_str(), "total" | "proof") && item.rust_function_path == item_name
    }) else {
        return diagnostic;
    };

    diagnostic.push_str(&format!(
        "\n  --> Trust {} `{}`",
        item.item_kind, item.rust_function_path
    ));
    if let Some(source_span) = diagnostic_source_span(item, semantic_source_spans) {
        diagnostic.push_str(&format!(" at {source_span}"));
    }

    let snippet = diagnostic_source_snippet(err, item)
        .or_else(|| first_source_line(&item.function_source))
        .unwrap_or_else(|| item.function_source.trim().to_string());
    if !snippet.is_empty() {
        diagnostic.push_str("\n   |");
        diagnostic.push_str(&format!("\n   | {snippet}"));
        diagnostic.push_str("\n   |");
    }

    if let Some(help) = diagnostic_help(err) {
        diagnostic.push_str(&format!("\nhelp[trust]: {help}"));
    }

    diagnostic
}

fn diagnostic_source_span<'a>(
    item: &'a TrustMetadata,
    semantic_source_spans: &'a [semantic::SemanticSourceSpan],
) -> Option<&'a str> {
    if item.source_span != "unknown" {
        return Some(item.source_span.as_str());
    }

    semantic_source_spans
        .iter()
        .find(|source_span| {
            source_span.rust_function_path == item.rust_function_path
                && source_span.source_span != "unknown"
        })
        .map(|source_span| source_span.source_span.as_str())
}

fn verification_error_item_name(err: &VerificationError) -> Option<&str> {
    match err {
        VerificationError::IntegerAdditionOverflow { function, .. }
        | VerificationError::IntegerSubtractionOverflow { function, .. }
        | VerificationError::IntegerNegationOverflow { function, .. }
        | VerificationError::IntegerMultiplicationOverflow { function, .. }
        | VerificationError::IntegerDivisionOverflow { function, .. }
        | VerificationError::IntegerRemainderOverflow { function, .. }
        | VerificationError::IntegerDivisionByZero { function, .. }
        | VerificationError::IntegerRemainderByZero { function, .. }
        | VerificationError::SliceIndexOutOfBounds { function, .. }
        | VerificationError::UnsupportedIndex { function, .. }
        | VerificationError::CalleePreconditionUnproved { function, .. }
        | VerificationError::MissingTrustModel { function, .. }
        | VerificationError::UnsupportedType { function, .. }
        | VerificationError::LoopMissingSpec { function }
        | VerificationError::LoopAmbiguousSpec { function }
        | VerificationError::LoopMissingDecreases { function }
        | VerificationError::LoopInvariantNotEstablished { function, .. }
        | VerificationError::LoopInvariantNotPreserved { function, .. }
        | VerificationError::LoopDecreasesNotDecreasing { function, .. }
        | VerificationError::UnsupportedLoopControl { function, .. }
        | VerificationError::UnsupportedCall { function, .. }
        | VerificationError::UnsupportedClosure { function }
        | VerificationError::SemanticExtractionIncomplete { function, .. }
        | VerificationError::ExplicitPanic { function }
        | VerificationError::UncheckedUnwrap { function }
        | VerificationError::PostconditionUnproved { function, .. } => Some(function),
        VerificationError::ProofObligationUnproved { proof, .. }
        | VerificationError::UnsupportedProofStep { proof } => Some(proof),
    }
}

fn diagnostic_source_snippet(err: &VerificationError, item: &TrustMetadata) -> Option<String> {
    let needle = verification_error_expression(err)?;
    source_line_containing(&item.function_source, needle)
}

fn verification_error_expression(err: &VerificationError) -> Option<&str> {
    match err {
        VerificationError::IntegerAdditionOverflow { expression, .. }
        | VerificationError::IntegerSubtractionOverflow { expression, .. }
        | VerificationError::IntegerNegationOverflow { expression, .. }
        | VerificationError::IntegerMultiplicationOverflow { expression, .. }
        | VerificationError::IntegerDivisionOverflow { expression, .. }
        | VerificationError::IntegerRemainderOverflow { expression, .. }
        | VerificationError::IntegerDivisionByZero { expression, .. }
        | VerificationError::IntegerRemainderByZero { expression, .. }
        | VerificationError::SliceIndexOutOfBounds { expression, .. }
        | VerificationError::UnsupportedIndex { expression, .. }
        | VerificationError::SemanticExtractionIncomplete { expression, .. } => Some(expression),
        VerificationError::CalleePreconditionUnproved { condition, .. }
        | VerificationError::LoopInvariantNotEstablished {
            invariant: condition,
            ..
        }
        | VerificationError::LoopInvariantNotPreserved {
            invariant: condition,
            ..
        }
        | VerificationError::LoopDecreasesNotDecreasing {
            measure: condition, ..
        }
        | VerificationError::PostconditionUnproved { condition, .. }
        | VerificationError::ProofObligationUnproved { condition, .. } => Some(condition),
        VerificationError::UnsupportedLoopControl { keyword, .. }
        | VerificationError::UnsupportedCall {
            callee: keyword, ..
        } => Some(keyword),
        VerificationError::MissingTrustModel { .. }
        | VerificationError::UnsupportedType { .. }
        | VerificationError::LoopMissingSpec { .. }
        | VerificationError::LoopAmbiguousSpec { .. }
        | VerificationError::LoopMissingDecreases { .. }
        | VerificationError::UnsupportedClosure { .. }
        | VerificationError::ExplicitPanic { .. }
        | VerificationError::UncheckedUnwrap { .. }
        | VerificationError::UnsupportedProofStep { .. } => None,
    }
}

fn source_line_containing(source: &str, needle: &str) -> Option<String> {
    let normalized_needle = normalize_for_diagnostic_search(needle);
    if normalized_needle.is_empty() {
        return None;
    }

    source
        .lines()
        .map(str::trim)
        .find(|line| normalize_for_diagnostic_search(line).contains(&normalized_needle))
        .map(str::to_string)
}

fn first_source_line(source: &str) -> Option<String> {
    source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn normalize_for_diagnostic_search(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn diagnostic_help(err: &VerificationError) -> Option<&'static str> {
    match err {
        VerificationError::IntegerAdditionOverflow { .. }
        | VerificationError::IntegerSubtractionOverflow { .. }
        | VerificationError::IntegerNegationOverflow { .. }
        | VerificationError::IntegerMultiplicationOverflow { .. }
        | VerificationError::IntegerDivisionOverflow { .. }
        | VerificationError::IntegerRemainderOverflow { .. } => {
            Some("add executable preconditions that bound the arithmetic expression")
        }
        VerificationError::IntegerDivisionByZero { .. }
        | VerificationError::IntegerRemainderByZero { .. } => {
            Some("add an executable precondition proving the denominator is nonzero")
        }
        VerificationError::SliceIndexOutOfBounds { .. } => {
            Some("add an executable precondition proving the index is below the slice length")
        }
        VerificationError::MissingTrustModel { .. } => {
            Some("derive TrustModel for the type if it satisfies the MVP model restrictions")
        }
        VerificationError::UnsupportedType { .. } => {
            Some("use an MVP-supported type or introduce an explicit TrustModel where structural reasoning is supported")
        }
        VerificationError::CalleePreconditionUnproved { .. } => {
            Some("strengthen the caller preconditions or prove the callee requirement before the call")
        }
        VerificationError::PostconditionUnproved { .. } => {
            Some("make the returned expression match the stated postcondition or strengthen the proof facts")
        }
        VerificationError::SemanticExtractionIncomplete { .. } => {
            Some("rerun with TRUST_SEMANTIC_DUMP_DIR to inspect the HIR/MIR facts Trust extracted")
        }
        VerificationError::LoopMissingSpec { .. } | VerificationError::LoopMissingDecreases { .. } => {
            Some("add a loop_spec block with an invariant and decreases measure before the loop")
        }
        _ => None,
    }
}

fn check_solver_status(config: &TrustConfig) -> Result<(), String> {
    match config.solver.as_str() {
        "mock" => check_mock_solver_status(),
        "z3" => check_z3_solver_status(config),
        solver => Err(format!("unsupported solver `{solver}`")),
    }
}

fn check_mock_solver_status() -> Result<(), String> {
    solver_result_from_status(
        env::var("TRUST_SOLVER_STATUS")
            .as_deref()
            .unwrap_or("proved"),
    )
}

fn check_z3_solver_status(config: &TrustConfig) -> Result<(), String> {
    let mut cfg = Config::new();
    cfg.set_bool_param_value("trace", false);
    cfg.set_timeout_msec(config.timeout_ms);
    let result = z3::with_z3_config(&cfg, || {
        let solver = Solver::new_for_logic("QF_LIA").unwrap_or_else(Solver::new);
        solver.assert(Bool::from_bool(false));
        solver.check()
    });

    solver_result_from_sat_result(result)
}

fn solver_version(config: &TrustConfig) -> Result<String, String> {
    if let Ok(version) = env::var("TRUST_SOLVER_VERSION") {
        return Ok(version);
    }

    match config.solver.as_str() {
        "mock" => Ok("mock-v1".to_string()),
        "z3" => Ok(z3::full_version().to_string()),
        solver => Err(format!("unsupported solver `{solver}`")),
    }
}

fn solver_result_from_sat_result(result: SatResult) -> Result<(), String> {
    match result {
        SatResult::Unsat => Ok(()),
        SatResult::Sat => Err("solver found counterexample".to_string()),
        SatResult::Unknown => Err("solver returned unknown".to_string()),
    }
}

fn solver_result_from_status(status: &str) -> Result<(), String> {
    match status {
        "proved" | "unsat" => Ok(()),
        "counterexample" | "sat" => Err("solver found counterexample".to_string()),
        "unknown" => Err("solver returned unknown".to_string()),
        "timeout" => Err("solver timed out".to_string()),
        "error" | "solver_error" => Err("solver error".to_string()),
        status => Err(format!("unsupported solver status `{status}`")),
    }
}

fn cleanup_z3_trace_file() {
    // Vendored debug Z3 opens this at library load even when tracing is disabled.
    let path = PathBuf::from(".z3-trace");
    if matches!(fs::metadata(&path), Ok(metadata) if metadata.len() == 0) {
        let _ = fs::remove_file(path);
    }
}

fn rustc_verbose_version(rustc: &OsString) -> Option<String> {
    let output = Command::new(rustc).arg("-vV").output().ok()?;
    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn host_target_from_verbose(version: &str) -> Option<String> {
    version.lines().find_map(|line| {
        line.strip_prefix("host: ")
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .map(str::to_string)
    })
}

fn rustc_target_cfg(rustc: &OsString, target_triple: &str) -> Option<TargetCfg> {
    let output = Command::new(rustc)
        .args(["--print", "cfg", "--target", target_triple])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let mut cfg = TargetCfg::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(width) = quoted_cfg_value(line, "target_pointer_width") {
            cfg.pointer_width = width;
        } else if let Some(endian) = quoted_cfg_value(line, "target_endian") {
            cfg.endianness = endian;
        } else if let Some(feature) = quoted_cfg_value(line, "target_feature") {
            cfg.target_features.push(feature);
        }
    }

    Some(cfg)
}

fn quoted_cfg_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=\"");
    line.strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
}

fn split_rustc_args(args: Vec<OsString>) -> (OsString, Vec<OsString>) {
    let mut args = args;
    if let Some(first) = args.first() {
        if !starts_with_dash(first) && file_name_contains_rustc(first) {
            let rustc = args.remove(0);
            return (rustc, args);
        }
    }

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    (rustc, args)
}

fn starts_with_dash(arg: &OsString) -> bool {
    arg.to_string_lossy().starts_with('-')
}

fn file_name_contains_rustc(arg: &OsString) -> bool {
    PathBuf::from(arg)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains("rustc"))
}

fn target_arg(rustc_args: &[OsString]) -> Option<String> {
    let mut args = rustc_args.iter();
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "--target" {
            return args
                .next()
                .map(|target| target.to_string_lossy().to_string());
        }
        if let Some(target) = arg.strip_prefix("--target=") {
            return Some(target.to_string());
        }
    }

    None
}

fn target_feature_args(rustc_args: &[OsString]) -> Vec<String> {
    let mut features = Vec::new();
    let mut args = rustc_args.iter();
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "-C" {
            if let Some(value) = args.next().and_then(|value| value.to_str()) {
                collect_codegen_target_arg(value, &mut features);
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix("-C") {
            collect_codegen_target_arg(value, &mut features);
        }
    }

    features
}

fn collect_codegen_target_arg(value: &str, features: &mut Vec<String>) {
    if let Some(target_feature) = value.strip_prefix("target-feature=") {
        features.push(format!("arg:target-feature={target_feature}"));
    } else if let Some(target_cpu) = value.strip_prefix("target-cpu=") {
        features.push(format!("arg:target-cpu={target_cpu}"));
    }
}

fn cargo_feature_cfgs(rustc_args: &[OsString]) -> Vec<String> {
    let mut features = Vec::new();
    let mut args = rustc_args.iter();
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "--cfg" {
            if let Some(value) = args.next().and_then(|value| value.to_str()) {
                collect_feature_cfg(value, &mut features);
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix("--cfg=") {
            collect_feature_cfg(value, &mut features);
        }
    }

    features
}

fn collect_feature_cfg(value: &str, features: &mut Vec<String>) {
    if let Some(feature) = value
        .strip_prefix("feature=\"")
        .and_then(|feature| feature.strip_suffix('"'))
    {
        features.push(feature.to_string());
    }
}

fn read_metadata(path: &PathBuf) -> Result<Vec<trust_core::metadata::TrustMetadata>, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };

    let mut metadata = Vec::new();
    for (line_idx, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        metadata.push(parse_metadata_line(line).map_err(|err| {
            format!(
                "invalid Trust metadata in {} on line {}: {err}",
                path.display(),
                line_idx + 1
            )
        })?);
    }
    Ok(metadata)
}

fn reject_ambiguous_metadata_paths(metadata: &[TrustMetadata]) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for item in metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof" | "trust_model"))
    {
        if !seen.insert(item.rust_function_path.as_str())
            && !duplicates
                .iter()
                .any(|duplicate| duplicate == &item.rust_function_path)
        {
            duplicates.push(item.rust_function_path.clone());
        }
    }

    if duplicates.is_empty() {
        return Ok(());
    }

    let names = duplicates
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "duplicate Trust metadata path{} {names}; Trust MVP requires unique total/proof/model paths within a crate so HIR/MIR facts map unambiguously",
        plural(duplicates.len())
    ))
}

fn metadata_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir()
        .join("trust")
        .join("metadata")
        .join(format!("rustc-{}-{nanos}.jsonl", process::id()))
}

fn deterministic_test_mode() -> bool {
    env::var("TRUST_TEST_DETERMINISTIC").as_deref() == Ok("1")
}

fn cache_file(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> Option<PathBuf> {
    let cache_dir = env::var_os("TRUST_CACHE_DIR").map(PathBuf::from)?;
    Some(cache_path(&cache_dir, metadata, semantics, cache_context))
}

fn cache_path(
    cache_dir: &PathBuf,
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    "trust-proof-cache-v2".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    cache_context.hash(&mut hasher);
    for item in metadata {
        hash_metadata_item(item, &mut hasher);
    }
    hash_semantics_if_present(semantics, &mut hasher);

    cache_dir.join(format!("{:016x}.proof", hasher.finish()))
}

fn cache_entry_proved(
    contents: &str,
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> bool {
    contents.lines().any(|line| line == "status=proved")
        && contents.lines().any(|line| {
            line == format!(
                "entry_fingerprint={:016x}",
                cache_entry_fingerprint(metadata, semantics, cache_context)
            )
        })
}

fn cache_entry_fingerprint(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    "trust-proof-cache-entry-v2".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    cache_context.hash(&mut hasher);
    for item in metadata {
        hash_metadata_item(item, &mut hasher);
    }
    hash_semantics_if_present(semantics, &mut hasher);
    hasher.finish()
}

fn hash_metadata_item(item: &trust_core::metadata::TrustMetadata, hasher: &mut DefaultHasher) {
    item.schema_version.hash(hasher);
    item.trust_macro_version.hash(hasher);
    item.module_id.hash(hasher);
    item.item_kind.hash(hasher);
    item.item_id.hash(hasher);
    item.source_span.hash(hasher);
    item.rust_function_path.hash(hasher);
    item.visibility.hash(hasher);
    item.contracts_original.hash(hasher);
    item.contracts_normalized.hash(hasher);
    item.contract_classes.hash(hasher);
    item.assertion_policy.hash(hasher);
    item.function_source.hash(hasher);
    item.body_hash_placeholder.hash(hasher);
    item.trust_model_dependencies.hash(hasher);
}

fn vc_fingerprints(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> Vec<String> {
    metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
        .map(|item| vc_fingerprint(item, semantics, cache_context))
        .collect()
}

fn vc_fingerprint(
    item: &trust_core::metadata::TrustMetadata,
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> String {
    let mut hasher = DefaultHasher::new();
    "trust-vc-fingerprint-v1".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    hash_verification_model_context(cache_context, &mut hasher);
    hash_metadata_item(item, &mut hasher);
    hash_semantics_if_present(semantics, &mut hasher);
    format!("{:016x}", hasher.finish())
}

fn hash_verification_model_context(cache_context: &CacheContext, hasher: &mut DefaultHasher) {
    cache_context.target_triple.hash(hasher);
    cache_context.pointer_width.hash(hasher);
    cache_context.endianness.hash(hasher);
    cache_context.target_features.hash(hasher);
    cache_context.cargo_features.hash(hasher);
}

fn generated_rust_fingerprint(metadata: &[trust_core::metadata::TrustMetadata]) -> String {
    let mut hasher = DefaultHasher::new();
    "trust-generated-rust-fingerprint-v1".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    for item in metadata {
        item.item_kind.hash(&mut hasher);
        item.item_id.hash(&mut hasher);
        item.assertion_policy.hash(&mut hasher);
        item.function_source.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn semantic_fingerprint(semantics: &[TrustFunctionSemantics]) -> String {
    if semantics.is_empty() {
        return "none".to_string();
    }

    let mut hasher = DefaultHasher::new();
    hash_semantics_if_present(semantics, &mut hasher);
    format!("{:016x}", hasher.finish())
}

fn hash_semantics_if_present(semantics: &[TrustFunctionSemantics], hasher: &mut DefaultHasher) {
    if semantics.is_empty() {
        return;
    }

    "trust-semantic-verification-v1".hash(hasher);
    for item in semantics {
        item.hash(hasher);
    }
}

fn solver_transcript_path() -> String {
    env::var("TRUST_SOLVER_TRANSCRIPT_PATH")
        .or_else(|_| env::var("TRUST_SMT_DUMP_DIR"))
        .unwrap_or_else(|_| "none".to_string())
}

fn cache_entry_contents(
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> String {
    let verification_items = metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
        .count();
    let vc_fingerprints = vc_fingerprints(metadata, semantics, cache_context).join(",");
    let generated_rust_fingerprint = generated_rust_fingerprint(metadata);
    let semantic_fingerprint = semantic_fingerprint(semantics);
    let solver_transcript_path = solver_transcript_path();
    format!(
        "format=trust-proof-cache-v2\nstatus=proved\nentry_fingerprint={:016x}\nverified_items={verification_items}\nsolver={}\nvc_fingerprints={vc_fingerprints}\ngenerated_rust_fingerprint={generated_rust_fingerprint}\nsemantic_fingerprint={semantic_fingerprint}\ndiagnostics_summary=none\nsolver_transcript_path={solver_transcript_path}\n",
        cache_entry_fingerprint(metadata, semantics, cache_context),
        cache_context.solver_name,
    )
}

fn write_cache_entry(
    cache_file: &PathBuf,
    metadata: &[trust_core::metadata::TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    cache_context: &CacheContext,
) -> Result<(), String> {
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create Trust cache directory: {err}"))?;
    }

    let temp_file = cache_file.with_extension(format!("{}.tmp", process::id()));
    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|err| format!("failed to write Trust cache entry: {err}"))?;
        file.write_all(cache_entry_contents(metadata, semantics, cache_context).as_bytes())
            .map_err(|err| format!("failed to write Trust cache entry: {err}"))?;
        file.sync_all()
            .map_err(|err| format!("failed to persist Trust cache entry: {err}"))?;
    }
    fs::rename(&temp_file, cache_file)
        .map_err(|err| format!("failed to install Trust cache entry: {err}"))
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata_item(kind: &str, path: &str) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "test-module".to_string(),
            item_id: format!("{kind}:{path}:test"),
            item_kind: kind.to_string(),
            source_span: "test-span".to_string(),
            rust_function_path: path.to_string(),
            visibility: "public".to_string(),
            contracts_original: Vec::new(),
            contracts_normalized: Vec::new(),
            contract_classes: Vec::new(),
            assertion_policy: "always".to_string(),
            function_source: format!("pub fn {path}() {{}}"),
            loop_specs: Vec::new(),
            body_hash_placeholder: format!("{path}-hash"),
            trust_model_dependencies: Vec::new(),
        }
    }

    fn trust_config(solver: &str) -> TrustConfig {
        TrustConfig {
            assertions: "always".to_string(),
            solver: solver.to_string(),
            timeout_ms: 1234,
            cache: "local".to_string(),
            silence_assume_warning: false,
            fingerprint: "test-config".to_string(),
        }
    }

    fn cache_context_with_pointer_width(pointer_width: &str) -> CacheContext {
        CacheContext {
            rustc_version: "rustc-test".to_string(),
            target_triple: "i686-unknown-linux-gnu".to_string(),
            pointer_width: pointer_width.to_string(),
            endianness: "little".to_string(),
            target_features: Vec::new(),
            cargo_features: Vec::new(),
            solver_name: "z3".to_string(),
            solver_version: "z3-test".to_string(),
            solver_options: "timeout_ms=1234;".to_string(),
            trust_config_fingerprint: "test-config".to_string(),
        }
    }

    #[test]
    fn wrapper_splits_cargo_style_rustc_args() {
        let (rustc, args) = split_rustc_args(vec![
            OsString::from("/toolchain/bin/rustc"),
            OsString::from("--crate-name"),
            OsString::from("demo"),
        ]);

        assert_eq!(rustc, OsString::from("/toolchain/bin/rustc"));
        assert_eq!(
            args,
            vec![OsString::from("--crate-name"), OsString::from("demo")]
        );
    }

    #[test]
    fn wrapper_can_be_invoked_like_rustc_directly() {
        let (_rustc, args) = split_rustc_args(vec![OsString::from("--version")]);

        assert_eq!(args, vec![OsString::from("--version")]);
    }

    #[test]
    fn removes_vendored_z3_trace_file() {
        cleanup_z3_trace_file();
    }

    #[test]
    fn accepts_unique_metadata_paths() {
        let metadata = vec![
            metadata_item("total", "first"),
            metadata_item("proof", "second"),
        ];

        assert_eq!(reject_ambiguous_metadata_paths(&metadata), Ok(()));
    }

    #[test]
    fn accepts_duplicate_leaf_names_with_distinct_metadata_paths() {
        let metadata = vec![
            metadata_item("total", "left::same"),
            metadata_item("total", "right::same"),
        ];

        assert_eq!(reject_ambiguous_metadata_paths(&metadata), Ok(()));
    }

    #[test]
    fn rejects_duplicate_metadata_paths() {
        let metadata = vec![
            metadata_item("total", "same"),
            metadata_item("total", "same"),
        ];

        assert_eq!(
            reject_ambiguous_metadata_paths(&metadata),
            Err("duplicate Trust metadata path `same`; Trust MVP requires unique total/proof/model paths within a crate so HIR/MIR facts map unambiguously".to_string())
        );
    }

    #[test]
    fn verification_error_uses_semantic_span_when_metadata_span_unknown() {
        let mut item = metadata_item("total", "verified::add_one");
        item.source_span = "unknown".to_string();
        item.function_source = "pub fn add_one(x: i32) -> i32 { x + 1 }".to_string();
        let err = VerificationError::IntegerAdditionOverflow {
            function: "verified::add_one".to_string(),
            expression: "x + 1".to_string(),
        };
        let diagnostic = format_verification_error(
            &err,
            &[item],
            &[semantic::SemanticSourceSpan {
                rust_function_path: "verified::add_one".to_string(),
                source_span: "src/lib.rs:3:5: 3:42 (#0)".to_string(),
            }],
        );

        assert!(
            diagnostic.contains("--> Trust total `verified::add_one` at src/lib.rs:3:5: 3:42 (#0)")
        );
        assert!(diagnostic.contains("pub fn add_one(x: i32) -> i32 { x + 1 }"));
    }

    #[test]
    fn verification_options_use_target_pointer_width() {
        let config = trust_config("z3");
        let cache_context = cache_context_with_pointer_width("32");

        assert_eq!(
            verification_options(&config, &cache_context),
            VerificationOptions::z3(1234).with_target_pointer_width(32)
        );
    }

    #[test]
    fn verification_options_ignore_unknown_pointer_width() {
        let config = trust_config("z3");
        let cache_context = cache_context_with_pointer_width("unknown");

        assert_eq!(
            verification_options(&config, &cache_context),
            VerificationOptions::z3(1234)
        );
    }

    #[test]
    fn solver_result_maps_unsat_to_proved() {
        assert_eq!(solver_result_from_sat_result(SatResult::Unsat), Ok(()));
    }

    #[test]
    fn solver_result_maps_sat_to_counterexample() {
        assert_eq!(
            solver_result_from_sat_result(SatResult::Sat),
            Err("solver found counterexample".to_string())
        );
    }

    #[test]
    fn solver_result_maps_unknown_to_failure() {
        assert_eq!(
            solver_result_from_sat_result(SatResult::Unknown),
            Err("solver returned unknown".to_string())
        );
    }
}
