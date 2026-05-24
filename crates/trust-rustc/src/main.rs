use std::collections::hash_map::DefaultHasher;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use trust_core::{metadata::parse_metadata_line, verifier::verify_totals};

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("error[trust]: {err}");
            process::exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let (rustc, rustc_args) = split_rustc_args(args);
    let metadata_path = metadata_path();

    let status = Command::new(&rustc)
        .args(&rustc_args)
        .env("TRUST_RUSTC_ACTIVE", "1")
        .env("TRUST_RUSTC_VERSION", env!("CARGO_PKG_VERSION"))
        .env("TRUST_METADATA_OUT", &metadata_path)
        .status()
        .map_err(|err| format!("failed to invoke rustc through trust-rustc: {err}"))?;

    if !status.success() {
        return Ok(exit_code(status));
    }

    let metadata = read_metadata(&metadata_path)?;
    let cache_stats = verify_metadata(&metadata)?;
    let totals = metadata
        .iter()
        .filter(|item| item.item_kind == "total")
        .count();

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

fn verify_metadata(metadata: &[trust_core::metadata::TrustMetadata]) -> Result<CacheStats, String> {
    let totals = metadata
        .iter()
        .filter(|item| item.item_kind == "total")
        .count();
    if totals == 0 {
        return Ok(CacheStats { hits: 0, misses: 0 });
    }
    check_mock_solver_status()?;

    if let Some(cache_file) = cache_file(metadata) {
        if matches!(
            fs::read_to_string(&cache_file),
            Ok(contents) if contents == "status=proved\n"
        ) {
            return Ok(CacheStats {
                hits: totals,
                misses: 0,
            });
        }

        verify_totals(metadata).map_err(|err| err.to_string())?;
        write_cache_entry(&cache_file)?;
        return Ok(CacheStats {
            hits: 0,
            misses: totals,
        });
    }

    verify_totals(metadata).map_err(|err| err.to_string())?;
    Ok(CacheStats {
        hits: 0,
        misses: totals,
    })
}

fn check_mock_solver_status() -> Result<(), String> {
    match env::var("TRUST_SOLVER_STATUS").as_deref() {
        Ok("proved") | Err(_) => Ok(()),
        Ok("counterexample") => Err("solver found counterexample".to_string()),
        Ok("unknown") => Err("solver returned unknown".to_string()),
        Ok("timeout") => Err("solver timed out".to_string()),
        Ok("error") => Err("solver error".to_string()),
        Ok(status) => Err(format!("unsupported mock solver status `{status}`")),
    }
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

fn cache_file(metadata: &[trust_core::metadata::TrustMetadata]) -> Option<PathBuf> {
    let cache_dir = env::var_os("TRUST_CACHE_DIR").map(PathBuf::from)?;
    let mut hasher = DefaultHasher::new();
    "trust-proof-cache-v1".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    env::var("TRUST_SOLVER")
        .unwrap_or_else(|_| "mock".to_string())
        .hash(&mut hasher);
    env::var("TRUST_SOLVER_STATUS")
        .unwrap_or_else(|_| "proved".to_string())
        .hash(&mut hasher);
    for item in metadata {
        item.schema_version.hash(&mut hasher);
        item.item_kind.hash(&mut hasher);
        item.item_id.hash(&mut hasher);
        item.rust_function_path.hash(&mut hasher);
        item.contracts_original.hash(&mut hasher);
        item.contract_classes.hash(&mut hasher);
        item.function_source.hash(&mut hasher);
    }

    Some(cache_dir.join(format!("{:016x}.proof", hasher.finish())))
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
}
