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
fn pass_get_precondition_proves_slice_bounds() {
    let output = run_fixture("pass_get_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_private_ghost_precondition_builds_without_runtime_assertion() {
    let output = run_fixture("pass_private_ghost_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_callee_precondition_proves_trust_to_trust_call() {
    let output = run_fixture("pass_callee_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 2 total functions");
    output.assert_contains("trust: proved 2 total functions");
}

#[test]
fn pass_add_one_precondition_proves_i32_overflow_safety() {
    let output = run_fixture("pass_add_one_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_sub_one_precondition_proves_i32_overflow_safety() {
    let output = run_fixture("pass_sub_one_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_usize_sub_one_precondition_proves_underflow_safety() {
    let output = run_fixture("pass_usize_sub_one_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_abs_nonmin_precondition_proves_negation_overflow_safety() {
    let output = run_fixture("pass_abs_nonmin_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_double_precondition_proves_i32_overflow_safety() {
    let output = run_fixture("pass_double_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_usize_double_precondition_proves_overflow_safety() {
    let output = run_fixture("pass_usize_double_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_id_postcondition_proves_executable_postcondition() {
    let output = run_fixture("pass_id_postcondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_add_one_postcondition_proves_ghost_arithmetic_postcondition() {
    let output = run_fixture("pass_add_one_postcondition", Expected::Pass);

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

#[test]
fn fail_executable_precondition_quantifier_is_rejected() {
    let output = run_fixture("fail_executable_precondition_quantifier", Expected::Fail);

    output
        .assert_contains("error[trust]: quantifiers are not supported in executable preconditions");
}

#[test]
fn fail_executable_precondition_call_is_rejected() {
    let output = run_fixture("fail_executable_precondition_call", Expected::Fail);

    output.assert_contains("error[trust]: unsupported function call in executable precondition");
}

#[test]
fn fail_public_ghost_precondition_is_rejected() {
    let output = run_fixture("fail_public_ghost_precondition", Expected::Fail);

    output.assert_contains("error[trust]: public function has ghost-only precondition");
}

#[test]
fn fail_callee_precondition_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_callee_precondition", Expected::Fail);

    output.assert_contains("error[trust]: could not prove callee precondition");
}

#[test]
fn fail_overflow_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_overflow_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer addition cannot overflow");
}

#[test]
fn fail_subtraction_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_subtraction_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer subtraction cannot overflow");
}

#[test]
fn fail_usize_subtraction_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_usize_subtraction_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer subtraction cannot overflow");
}

#[test]
fn fail_negation_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_negation_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer negation cannot overflow");
}

#[test]
fn fail_multiplication_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_multiplication_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer multiplication cannot overflow");
}

#[test]
fn fail_slice_index_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_slice_index_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove index is in bounds");
}

#[test]
fn fail_postcondition_false_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_postcondition_false", Expected::Fail);

    output.assert_contains("error[trust]: could not prove postcondition");
}

#[test]
fn fail_postcondition_wrong_expression_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_postcondition_wrong_expression", Expected::Fail);

    output.assert_contains("error[trust]: could not prove postcondition");
}
