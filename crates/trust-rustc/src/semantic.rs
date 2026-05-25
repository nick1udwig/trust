use std::collections::hash_map::DefaultHasher;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use trust_core::metadata::TrustMetadata;

use crate::{deterministic_test_mode, exit_code, metadata_path, plural, rustc_verbose_version};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticItemMatch {
    item_kind: String,
    item_id: String,
    rust_function_path: String,
    source_span: String,
    hir_match: bool,
    mir_match: bool,
    mir_function: Option<MirFunctionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirFunctionSummary {
    args: Vec<String>,
    return_type: String,
    debug_locals: Vec<String>,
    return_expr: Option<String>,
}

pub(crate) fn maybe_extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
) -> Result<(), String> {
    let Some(dump_dir) = env::var_os("TRUST_SEMANTIC_DUMP_DIR").map(PathBuf::from) else {
        return Ok(());
    };

    extract_semantic_views(rustc, rustc_args, metadata, dump_dir)
}

fn extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
    dump_dir: PathBuf,
) -> Result<(), String> {
    let hir = run_rustc_unpretty(rustc, rustc_args, "hir-tree")?;
    let mir = run_rustc_unpretty(rustc, rustc_args, "mir")?;
    let item_matches = semantic_item_matches(metadata, &hir, &mir);
    reject_unmatched_total_items(&item_matches)?;
    let rustc_version = semantic_rustc_version(rustc);

    fs::create_dir_all(&dump_dir)
        .map_err(|err| format!("failed to create Trust semantic dump directory: {err}"))?;
    let base = semantic_dump_base(metadata, rustc_args);
    write_semantic_dump(&dump_dir.join(format!("{base}.hir-tree.txt")), &hir)?;
    write_semantic_dump(&dump_dir.join(format!("{base}.mir.txt")), &mir)?;
    write_semantic_dump(
        &dump_dir.join(format!("{base}.trust-semantic.txt")),
        &semantic_summary(metadata, rustc_args, &item_matches, &rustc_version),
    )?;

    if deterministic_test_mode() {
        let total_matches = item_matches
            .iter()
            .filter(|item| item.item_kind == "total" && item.hir_match && item.mir_match)
            .count();
        eprintln!(
            "trust: extracted HIR/MIR for {total_matches} total function{}",
            plural(total_matches)
        );
    }

    Ok(())
}

fn run_rustc_unpretty(
    rustc: &OsString,
    rustc_args: &[OsString],
    mode: &str,
) -> Result<String, String> {
    let metadata_path = metadata_path();
    let output = Command::new(rustc)
        .args(rustc_args)
        .arg("-Z")
        .arg(format!("unpretty={mode}"))
        .env("RUSTC_BOOTSTRAP", "1")
        .env("TRUST_RUSTC_ACTIVE", "1")
        .env("TRUST_RUSTC_VERSION", env!("CARGO_PKG_VERSION"))
        .env("TRUST_METADATA_OUT", &metadata_path)
        .output()
        .map_err(|err| format!("failed to invoke rustc for Trust semantic extraction: {err}"))?;
    let _ = fs::remove_file(&metadata_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "rustc semantic extraction failed for `{mode}` with exit code {}\n{}",
            exit_code(output.status),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn semantic_rustc_version(rustc: &OsString) -> String {
    if deterministic_test_mode() {
        return env::var("TRUST_TEST_RUSTC_VERSION").unwrap_or_else(|_| "rustc-test".to_string());
    }

    rustc_verbose_version(rustc).unwrap_or_else(|| "unknown".to_string())
}

fn semantic_item_matches(
    metadata: &[TrustMetadata],
    hir: &str,
    mir: &str,
) -> Vec<SemanticItemMatch> {
    metadata
        .iter()
        .filter(|item| matches!(item.item_kind.as_str(), "total" | "proof"))
        .map(|item| {
            let leaf = function_leaf_name(&item.rust_function_path);
            let mir_function = extract_mir_function_summary(mir, leaf);
            SemanticItemMatch {
                item_kind: item.item_kind.clone(),
                item_id: item.item_id.clone(),
                rust_function_path: item.rust_function_path.clone(),
                source_span: item.source_span.clone(),
                hir_match: hir_contains_function(hir, leaf),
                mir_match: mir_function.is_some(),
                mir_function,
            }
        })
        .collect()
}

fn reject_unmatched_total_items(item_matches: &[SemanticItemMatch]) -> Result<(), String> {
    let missing: Vec<_> = item_matches
        .iter()
        .filter(|item| item.item_kind == "total" && (!item.hir_match || !item.mir_match))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }

    let names = missing
        .iter()
        .map(|item| item.rust_function_path.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Trust semantic extraction could not map total function{} to rustc HIR/MIR: {names}",
        plural(missing.len())
    ))
}

fn function_leaf_name(path: &str) -> &str {
    path.rsplit("::")
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

fn hir_contains_function(hir: &str, name: &str) -> bool {
    hir.contains(&format!("ident: {name}#"))
}

fn extract_mir_function_summary(mir: &str, name: &str) -> Option<MirFunctionSummary> {
    let lines = mir.lines().collect::<Vec<_>>();
    let signature_idx = lines
        .iter()
        .position(|line| line.trim_start().starts_with(&format!("fn {name}(")))?;
    let signature = parse_mir_signature(lines[signature_idx].trim(), name)?;
    let function_end = lines
        .iter()
        .enumerate()
        .skip(signature_idx + 1)
        .find(|(_, line)| line.trim_start().starts_with("fn "))
        .map(|(idx, _)| idx)
        .unwrap_or(lines.len());
    let function_lines = &lines[signature_idx + 1..function_end];

    Some(MirFunctionSummary {
        args: signature.args,
        return_type: signature.return_type,
        debug_locals: extract_mir_debug_locals(function_lines),
        return_expr: extract_mir_return_expr(function_lines),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSignature {
    args: Vec<String>,
    return_type: String,
}

fn parse_mir_signature(line: &str, name: &str) -> Option<MirSignature> {
    let prefix = format!("fn {name}(");
    let rest = line.strip_prefix(&prefix)?;
    let (args, rest) = rest.split_once(") -> ")?;
    let return_type = rest
        .trim()
        .strip_suffix('{')
        .unwrap_or(rest)
        .trim()
        .to_string();
    Some(MirSignature {
        args: parse_comma_separated(args),
        return_type,
    })
}

fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn extract_mir_debug_locals(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("debug ")
                .and_then(|line| line.split_once(" => "))
                .map(|(name, _)| name.trim().to_string())
        })
        .collect()
}

fn extract_mir_return_expr(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        line.trim()
            .strip_prefix("_0 = ")
            .and_then(|line| line.strip_suffix(';'))
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string)
    })
}

fn semantic_dump_base(metadata: &[TrustMetadata], rustc_args: &[OsString]) -> String {
    let crate_name = crate_name_arg(rustc_args).unwrap_or_else(|| "crate".to_string());
    let mut hasher = DefaultHasher::new();
    "trust-semantic-dump-v1".hash(&mut hasher);
    crate_name.hash(&mut hasher);
    for item in metadata {
        item.item_kind.hash(&mut hasher);
        item.item_id.hash(&mut hasher);
        item.rust_function_path.hash(&mut hasher);
        item.function_source.hash(&mut hasher);
    }
    format!(
        "{}-{:016x}",
        sanitize_file_component(&crate_name),
        hasher.finish()
    )
}

fn crate_name_arg(rustc_args: &[OsString]) -> Option<String> {
    let mut args = rustc_args.iter();
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "--crate-name" {
            return args
                .next()
                .and_then(|name| name.to_str())
                .map(str::to_string);
        }
        if let Some(name) = arg.strip_prefix("--crate-name=") {
            return Some(name.to_string());
        }
    }

    None
}

fn sanitize_file_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "crate".to_string()
    } else {
        sanitized
    }
}

fn semantic_summary(
    metadata: &[TrustMetadata],
    rustc_args: &[OsString],
    item_matches: &[SemanticItemMatch],
    rustc_version: &str,
) -> String {
    let mut summary = String::new();
    summary.push_str("format=trust-semantic-dump-v1\n");
    summary.push_str(&format!("rustc_version={rustc_version}\n"));
    if let Some(crate_name) = crate_name_arg(rustc_args) {
        summary.push_str(&format!("crate_name={crate_name}\n"));
    }
    summary.push_str(&format!("metadata_items={}\n", metadata.len()));
    summary.push_str(&format!("semantic_items={}\n", item_matches.len()));
    for item in item_matches {
        summary.push_str(&format!(
            "item kind={} id={} path={} span={} hir_match={} mir_match={}\n",
            item.item_kind,
            item.item_id,
            item.rust_function_path,
            item.source_span,
            item.hir_match,
            item.mir_match
        ));
        if let Some(mir_function) = &item.mir_function {
            summary.push_str(&format!(
                "mir_function path={} args={} return_type={} debug_locals={} return_expr={}\n",
                item.rust_function_path,
                mir_function.args.join(","),
                mir_function.return_type,
                mir_function.debug_locals.join(","),
                mir_function.return_expr.as_deref().unwrap_or("none")
            ));
        }
    }
    summary
}

fn write_semantic_dump(path: &PathBuf, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|err| {
        format!(
            "failed to write Trust semantic dump {}: {err}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_mir_function_summary() {
        let mir = r#"
fn id_i32(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = copy _1;
        return;
    }
}
"#;

        assert_eq!(
            extract_mir_function_summary(mir, "id_i32"),
            Some(MirFunctionSummary {
                args: vec!["_1: i32".to_string()],
                return_type: "i32".to_string(),
                debug_locals: vec!["x".to_string()],
                return_expr: Some("copy _1".to_string()),
            })
        );
    }

    #[test]
    fn extracts_crate_name_from_rustc_args() {
        assert_eq!(
            crate_name_arg(&[OsString::from("--crate-name"), OsString::from("demo")]),
            Some("demo".to_string())
        );
        assert_eq!(
            crate_name_arg(&[OsString::from("--crate-name=demo_two")]),
            Some("demo_two".to_string())
        );
    }
}
