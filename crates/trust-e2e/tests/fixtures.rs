use trust_test_support::{run_fixture, run_fixture_without_wrapper, Expected};

#[test]
fn pass_no_trust_builds_through_wrapper() {
    run_fixture("pass_no_trust", Expected::Pass);
}

#[test]
fn pass_total_identity_builds_and_metadata_is_discovered() {
    let output = run_fixture("pass_total_identity", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_is_zero_builds_and_metadata_is_discovered() {
    let output = run_fixture("pass_is_zero", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_runtime_assert_get_checks_public_precondition() {
    let output = run_fixture("pass_runtime_assert_get", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn fail_missing_wrapper_requires_trust_rustc() {
    let output = run_fixture_without_wrapper("fail_missing_wrapper", Expected::Fail);

    output.assert_contains("error[trust]: Trust verification requires trust-rustc");
}

#[test]
fn fail_unverified_fn_in_module_is_rejected() {
    let output = run_fixture("fail_unverified_fn_in_module", Expected::Fail);

    output.assert_contains(
        "error[trust]: unverified Rust functions are not allowed inside #[trust::module] in the MVP",
    );
}

#[test]
fn fail_async_total_is_rejected() {
    let output = run_fixture("fail_async_total", Expected::Fail);

    output.assert_contains("error[trust]: async functions are not supported in Trust MVP");
}

#[test]
fn fail_unsafe_total_is_rejected() {
    let output = run_fixture("fail_unsafe_total", Expected::Fail);

    output.assert_contains("error[trust]: unsafe functions are not supported in Trust MVP");
}

#[test]
fn fail_multiple_fn_in_total_is_rejected() {
    let output = run_fixture("fail_multiple_fn_in_total", Expected::Fail);

    output.assert_contains("error[trust]: trust::total! accepts exactly one Rust fn item");
}
