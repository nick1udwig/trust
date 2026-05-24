use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    Pass,
    Fail,
}

#[derive(Debug)]
pub struct FixtureOutput {
    pub status_success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl FixtureOutput {
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    pub fn assert_contains(&self, needle: &str) {
        let combined = self.combined();
        assert!(
            combined.contains(needle),
            "expected fixture output to contain `{needle}`\n\n{}",
            normalize_paths(&combined)
        );
    }
}

pub fn run_fixture(name: &str, expected: Expected) -> FixtureOutput {
    let wrapper = build_trust_rustc();
    run_fixture_inner(name, expected, Some(&wrapper), name, name, None)
}

pub fn run_fixture_with_cache(
    name: &str,
    expected: Expected,
    target_name: &str,
    cache_name: &str,
) -> FixtureOutput {
    let wrapper = build_trust_rustc();
    run_fixture_inner(
        name,
        expected,
        Some(&wrapper),
        target_name,
        cache_name,
        None,
    )
}

pub fn run_fixture_with_solver_status(
    name: &str,
    expected: Expected,
    solver_status: &str,
) -> FixtureOutput {
    let wrapper = build_trust_rustc();
    run_fixture_inner(
        name,
        expected,
        Some(&wrapper),
        name,
        name,
        Some(solver_status),
    )
}

pub fn run_fixture_without_wrapper(name: &str, expected: Expected) -> FixtureOutput {
    run_fixture_inner(name, expected, None, name, name, None)
}

fn run_fixture_inner(
    name: &str,
    expected: Expected,
    wrapper: Option<&Path>,
    target_name: &str,
    cache_name: &str,
    solver_status: Option<&str>,
) -> FixtureOutput {
    let root = workspace_root();
    let manifest = root
        .join("tests")
        .join("fixtures")
        .join(name)
        .join("Cargo.toml");
    let target_dir = root.join("target").join("fixtures").join(target_name);

    let mut command = Command::new(cargo());
    command
        .arg("test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir)
        .env("TRUST_TEST_MODE", "1")
        .env("TRUST_TEST_DETERMINISTIC", "1")
        .env("TRUST_SOLVER", "mock")
        .env(
            "TRUST_CACHE_DIR",
            root.join("target")
                .join("trust-cache-tests")
                .join(cache_name),
        )
        .env_remove("TRUST_MACRO_UNIT_TEST")
        .env_remove("TRUST_RUSTC_ACTIVE")
        .env_remove("TRUST_METADATA_OUT");

    if let Some(wrapper) = wrapper {
        command.env("RUSTC_WORKSPACE_WRAPPER", wrapper);
    } else {
        command.env_remove("RUSTC_WORKSPACE_WRAPPER");
    }
    if let Some(solver_status) = solver_status {
        command.env("TRUST_SOLVER_STATUS", solver_status);
    } else {
        command.env_remove("TRUST_SOLVER_STATUS");
    }

    let output = command.output().unwrap_or_else(|err| {
        panic!(
            "failed to run fixture `{name}` through Cargo at {}: {err}",
            manifest.display()
        )
    });
    let output = fixture_output(output);

    match expected {
        Expected::Pass if !output.status_success => {
            panic!(
                "fixture `{name}` should have passed\n\n{}",
                normalize_paths(&output.combined())
            );
        }
        Expected::Fail if output.status_success => {
            panic!(
                "fixture `{name}` should have failed\n\n{}",
                normalize_paths(&output.combined())
            );
        }
        _ => {}
    }

    output
}

fn build_trust_rustc() -> PathBuf {
    let root = workspace_root();
    let output = Command::new(cargo())
        .arg("build")
        .arg("-p")
        .arg("trust-rustc")
        .current_dir(&root)
        .output()
        .unwrap_or_else(|err| panic!("failed to build trust-rustc: {err}"));

    if !output.status.success() {
        panic!(
            "failed to build trust-rustc\n\n{}",
            normalize_paths(&fixture_output(output).combined())
        );
    }

    let mut wrapper = target_root(&root).join("debug").join("trust-rustc");
    if cfg!(windows) {
        wrapper.set_extension("exe");
    }
    wrapper
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("trust-test-support should live in crates/trust-test-support")
        .to_path_buf()
}

fn target_root(root: &Path) -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR") {
        Some(target) => {
            let target = PathBuf::from(target);
            if target.is_absolute() {
                target
            } else {
                root.join(target)
            }
        }
        None => root.join("target"),
    }
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn fixture_output(output: Output) -> FixtureOutput {
    FixtureOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

pub fn normalize_paths(input: &str) -> String {
    input.replace(&workspace_root().display().to_string(), "$WORKSPACE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_workspace_paths() {
        let root = workspace_root();
        let text = format!("{}{}", root.display(), "/tests/fixtures/pass_no_trust");

        assert_eq!(
            normalize_paths(&text),
            "$WORKSPACE/tests/fixtures/pass_no_trust"
        );
    }
}
