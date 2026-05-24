use std::env;
use std::ffi::OsString;
use std::fs;
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
    verify_metadata(&metadata)?;
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
    }

    let _ = fs::remove_file(&metadata_path);
    Ok(exit_code(status))
}

fn verify_metadata(metadata: &[trust_core::metadata::TrustMetadata]) -> Result<(), String> {
    verify_totals(metadata).map_err(|err| err.to_string())
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
