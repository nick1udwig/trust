use std::collections::hash_map::DefaultHasher;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use trust_core::{
    metadata::parse_metadata_line,
    solver::VerificationOptions,
    verifier::{verify_totals, verify_totals_with_options},
};
use z3::{ast::Bool, Config, SatResult, Solver};

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
        verify_metadata(&metadata, &cache_context, &config)?
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
    fingerprint: String,
}

impl TrustConfig {
    fn load() -> Result<Self, String> {
        let mut config = Self {
            assertions: "always".to_string(),
            solver: env::var("TRUST_SOLVER").unwrap_or_else(|_| "mock".to_string()),
            timeout_ms: 5000,
            cache: "local".to_string(),
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

    if let Some(cache_file) = cache_file(metadata, cache_context) {
        if matches!(
            fs::read_to_string(&cache_file),
            Ok(contents) if contents == "status=proved\n"
        ) {
            return Ok(CacheStats {
                hits: verification_items,
                misses: 0,
            });
        }

        verify_all(metadata, config)?;
        write_cache_entry(&cache_file)?;
        return Ok(CacheStats {
            hits: 0,
            misses: verification_items,
        });
    }

    verify_all(metadata, config)?;
    Ok(CacheStats {
        hits: 0,
        misses: verification_items,
    })
}

fn verify_all(
    metadata: &[trust_core::metadata::TrustMetadata],
    config: &TrustConfig,
) -> Result<(), String> {
    match config.solver.as_str() {
        "mock" => verify_totals(metadata).map_err(|err| err.to_string()),
        "z3" => verify_totals_with_options(metadata, VerificationOptions::z3(config.timeout_ms))
            .map_err(|err| err.to_string()),
        solver => Err(format!("unsupported solver `{solver}`")),
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
    cache_context: &CacheContext,
) -> Option<PathBuf> {
    let cache_dir = env::var_os("TRUST_CACHE_DIR").map(PathBuf::from)?;
    Some(cache_path(&cache_dir, metadata, cache_context))
}

fn cache_path(
    cache_dir: &PathBuf,
    metadata: &[trust_core::metadata::TrustMetadata],
    cache_context: &CacheContext,
) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    "trust-proof-cache-v1".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    cache_context.hash(&mut hasher);
    for item in metadata {
        item.schema_version.hash(&mut hasher);
        item.item_kind.hash(&mut hasher);
        item.item_id.hash(&mut hasher);
        item.rust_function_path.hash(&mut hasher);
        item.contracts_original.hash(&mut hasher);
        item.contract_classes.hash(&mut hasher);
        item.function_source.hash(&mut hasher);
    }

    cache_dir.join(format!("{:016x}.proof", hasher.finish()))
}

fn write_cache_entry(cache_file: &PathBuf) -> Result<(), String> {
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create Trust cache directory: {err}"))?;
    }

    let temp_file = cache_file.with_extension(format!("{}.tmp", process::id()));
    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|err| format!("failed to write Trust cache entry: {err}"))?;
        file.write_all(b"status=proved\n")
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
