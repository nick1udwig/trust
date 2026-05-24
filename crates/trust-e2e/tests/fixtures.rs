use std::fs;
use trust_test_support::{
    fixture_cache_dir, run_fixture, run_fixture_with_cache, run_fixture_with_cache_and_env,
    run_fixture_with_cache_and_solver_status, run_fixture_with_solver_status,
    run_fixture_without_wrapper, Expected,
};

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
fn pass_option_match_builds() {
    let output = run_fixture("pass_option_match", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_trust_model_struct_allows_field_reasoning() {
    let output = run_fixture("pass_trust_model_struct", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_withdraw_account_proves_old_field_postconditions() {
    let output = run_fixture("pass_withdraw_account", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_trusted_model_stub_is_accepted_and_ignored() {
    run_fixture("pass_trusted_model_stub_ignored", Expected::Pass);
}

#[test]
fn pass_loop_countdown_proves_decreases() {
    let output = run_fixture("pass_loop_countdown", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn fail_loop_without_spec_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_without_spec", Expected::Fail);

    output.assert_contains("error[trust]: loop in `countdown` requires loop_spec");
}

#[test]
fn fail_loop_spec_without_loop_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_spec_without_loop", Expected::Fail);

    output.assert_contains("error[trust]: loop in `id` requires loop_spec");
}

#[test]
fn fail_loop_two_specs_one_loop_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_two_specs_one_loop", Expected::Fail);

    output.assert_contains("error[trust]: multiple loop_spec blocks before loop in `countdown`");
}

#[test]
fn fail_loop_break_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_break", Expected::Fail);

    output.assert_contains("error[trust]: `break` is not supported in loops in `countdown`");
}

#[test]
fn pass_executable_spec_builds_as_runtime_rust() {
    run_fixture("pass_spec_executable", Expected::Pass);
}

#[test]
fn pass_ghost_spec_builds_without_runtime_rust() {
    run_fixture("pass_spec_ghost", Expected::Pass);
}

#[test]
fn pass_proof_function_is_erased() {
    run_fixture("pass_proof_erased", Expected::Pass);
}

#[test]
fn fail_wrong_proof_assert_is_rejected() {
    let output = run_fixture("fail_proof_wrong_assert", Expected::Fail);

    output.assert_contains("error[trust]: could not prove proof obligation in `le_refl`: `a <= a`");
}

#[test]
fn fail_runtime_call_in_proof_is_rejected() {
    let output = run_fixture("fail_proof_runtime_call", Expected::Fail);

    output.assert_contains("error[trust]: unsupported proof step in `le_refl`");
}

#[test]
fn fail_public_proof_is_rejected() {
    let output = run_fixture("fail_public_proof", Expected::Fail);

    output
        .assert_contains("error[trust]: proof functions are erased and cannot be public Rust APIs");
}

#[test]
fn fail_empty_nontrivial_proof_is_rejected() {
    let output = run_fixture("fail_empty_proof", Expected::Fail);

    output.assert_contains("error[trust]: empty proof body cannot prove a nontrivial lemma");
}

#[test]
fn fail_executable_spec_quantifier_is_rejected() {
    let output = run_fixture("fail_executable_spec_quantifier", Expected::Fail);

    output
        .assert_contains("error[trust]: quantifiers are not supported in executable preconditions");
}

#[test]
fn pass_missing_config_uses_defaults() {
    run_fixture("pass_config_missing_defaults", Expected::Pass);
}

#[test]
fn pass_assume_config_omits_runtime_precondition_assertion() {
    run_fixture("pass_config_assume_no_runtime_assertion", Expected::Pass);
}

#[test]
fn fail_invalid_assertion_policy_config_is_rejected() {
    let output = run_fixture("fail_config_invalid_assertions", Expected::Fail);

    output.assert_contains("error[trust]: invalid assertion policy `sometimes`");
}

#[test]
fn fail_invalid_solver_config_is_rejected() {
    let output = run_fixture("fail_config_invalid_solver", Expected::Fail);

    output.assert_contains("error[trust]: unsupported solver `bogus`");
}

#[test]
fn fail_invalid_timeout_config_is_rejected() {
    let output = run_fixture("fail_config_invalid_timeout", Expected::Fail);

    output.assert_contains("error[trust]: invalid timeout_ms `\"soon\"`");
}

#[test]
fn pass_cache_hit_reuses_proof_cache() {
    let suffix = std::process::id();
    let first_target = format!("pass_cache_hit_first_{suffix}");
    let second_target = format!("pass_cache_hit_second_{suffix}");
    let cache_name = format!("pass_cache_hit_shared_{suffix}");
    let first =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &first_target, &cache_name);
    let second = run_fixture_with_cache(
        "pass_cache_hit",
        Expected::Pass,
        &second_target,
        &cache_name,
    );

    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    second.assert_contains("trust: verified 1 function; cache hits 1; cache misses 0");
}

#[test]
fn pass_cache_misses_when_body_changes() {
    let suffix = std::process::id();
    let cache_name = format!("pass_cache_body_changed_shared_{suffix}");
    let first_target = format!("pass_cache_body_changed_first_{suffix}");
    let second_target = format!("pass_cache_body_changed_second_{suffix}");

    let first =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &first_target, &cache_name);
    let second = run_fixture_with_cache(
        "pass_cache_body_changed",
        Expected::Pass,
        &second_target,
        &cache_name,
    );

    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    second.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
}

#[test]
fn pass_cache_misses_when_contract_changes() {
    let suffix = std::process::id();
    let cache_name = format!("pass_cache_contract_changed_shared_{suffix}");
    let first_target = format!("pass_cache_contract_changed_first_{suffix}");
    let second_target = format!("pass_cache_contract_changed_second_{suffix}");

    let first =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &first_target, &cache_name);
    let second = run_fixture_with_cache(
        "pass_cache_contract_changed",
        Expected::Pass,
        &second_target,
        &cache_name,
    );

    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    second.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
}

#[test]
fn pass_cache_misses_when_solver_version_changes() {
    let suffix = std::process::id();
    let cache_name = format!("pass_cache_solver_version_changed_shared_{suffix}");
    let first_target = format!("pass_cache_solver_version_changed_first_{suffix}");
    let second_target = format!("pass_cache_solver_version_changed_second_{suffix}");
    let third_target = format!("pass_cache_solver_version_changed_third_{suffix}");

    let first = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &first_target,
        &cache_name,
        &[("TRUST_SOLVER_VERSION", "mock-solver-v1")],
    );
    let second = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &second_target,
        &cache_name,
        &[("TRUST_SOLVER_VERSION", "mock-solver-v1")],
    );
    let third = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &third_target,
        &cache_name,
        &[("TRUST_SOLVER_VERSION", "mock-solver-v2")],
    );

    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    second.assert_contains("trust: verified 1 function; cache hits 1; cache misses 0");
    third.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
}

#[test]
fn pass_cache_misses_when_target_changes() {
    let suffix = std::process::id();
    let cache_name = format!("pass_cache_target_changed_shared_{suffix}");
    let first_target = format!("pass_cache_target_changed_first_{suffix}");
    let second_target = format!("pass_cache_target_changed_second_{suffix}");
    let third_target = format!("pass_cache_target_changed_third_{suffix}");

    let target_a = [
        ("TRUST_TEST_TARGET_TRIPLE", "trust-test-target-a"),
        ("TRUST_TEST_TARGET_POINTER_WIDTH", "64"),
        ("TRUST_TEST_TARGET_ENDIANNESS", "little"),
    ];
    let target_b = [
        ("TRUST_TEST_TARGET_TRIPLE", "trust-test-target-b"),
        ("TRUST_TEST_TARGET_POINTER_WIDTH", "32"),
        ("TRUST_TEST_TARGET_ENDIANNESS", "big"),
    ];

    let first = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &first_target,
        &cache_name,
        &target_a,
    );
    let second = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &second_target,
        &cache_name,
        &target_a,
    );
    let third = run_fixture_with_cache_and_env(
        "pass_cache_hit",
        Expected::Pass,
        &third_target,
        &cache_name,
        &target_b,
    );

    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    second.assert_contains("trust: verified 1 function; cache hits 1; cache misses 0");
    third.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
}

#[test]
fn pass_cache_corrupt_entry_is_recomputed() {
    let suffix = std::process::id();
    let cache_name = format!("pass_cache_corrupt_recomputed_shared_{suffix}");
    let first_target = format!("pass_cache_corrupt_recomputed_first_{suffix}");
    let second_target = format!("pass_cache_corrupt_recomputed_second_{suffix}");
    let third_target = format!("pass_cache_corrupt_recomputed_third_{suffix}");

    let first =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &first_target, &cache_name);
    first.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");

    let proof_entries = proof_cache_entries(&cache_name);
    assert_eq!(proof_entries.len(), 1, "expected one proof cache entry");
    fs::write(&proof_entries[0], "status=partial\n").expect("corrupt proof cache entry");

    let second = run_fixture_with_cache(
        "pass_cache_hit",
        Expected::Pass,
        &second_target,
        &cache_name,
    );
    let third =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &third_target, &cache_name);

    second.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    third.assert_contains("trust: verified 1 function; cache hits 1; cache misses 0");
}

#[test]
fn fail_cache_timeout_is_not_reused_as_proof() {
    let suffix = std::process::id();
    let cache_name = format!("fail_cache_timeout_not_reused_shared_{suffix}");
    let timeout_target = format!("fail_cache_timeout_not_reused_timeout_{suffix}");
    let proved_target = format!("fail_cache_timeout_not_reused_proved_{suffix}");
    let repeat_target = format!("fail_cache_timeout_not_reused_repeat_{suffix}");

    let timeout = run_fixture_with_cache_and_solver_status(
        "pass_cache_hit",
        Expected::Fail,
        &timeout_target,
        &cache_name,
        "timeout",
    );
    timeout.assert_contains("error[trust]: solver timed out");
    assert!(
        proof_cache_entries(&cache_name).is_empty(),
        "timeout result should not write a proof cache entry"
    );

    let proved = run_fixture_with_cache(
        "pass_cache_hit",
        Expected::Pass,
        &proved_target,
        &cache_name,
    );
    let repeat = run_fixture_with_cache(
        "pass_cache_hit",
        Expected::Pass,
        &repeat_target,
        &cache_name,
    );

    proved.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    repeat.assert_contains("trust: verified 1 function; cache hits 1; cache misses 0");
}

fn proof_cache_entries(cache_name: &str) -> Vec<std::path::PathBuf> {
    let cache_dir = fixture_cache_dir(cache_name);
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Vec::new();
    };

    entries
        .map(|entry| entry.expect("read proof cache entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("proof"))
        .collect()
}

#[test]
fn pass_add_one_precondition_proves_i32_overflow_safety() {
    let output = run_fixture("pass_add_one_precondition", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_i64_add_one_precondition_proves_overflow_safety() {
    let output = run_fixture("pass_i64_add_one_precondition", Expected::Pass);

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
fn fail_extern_total_is_rejected() {
    let output = run_fixture("fail_extern_total", Expected::Fail);

    output.assert_contains("error[trust]: extern functions are not supported in Trust MVP");
}

#[test]
fn fail_generic_total_is_rejected() {
    let output = run_fixture("fail_generic_total", Expected::Fail);

    output.assert_contains("error[trust]: generic total functions are not supported in MVP");
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
fn fail_option_unwrap_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_option_unwrap", Expected::Fail);

    output.assert_contains(
        "error[trust]: unchecked unwrap is not supported; prove Some or use match",
    );
}

#[test]
fn fail_result_expect_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_result_expect", Expected::Fail);

    output.assert_contains(
        "error[trust]: unchecked unwrap is not supported; prove Some or use match",
    );
}

#[test]
fn pass_result_match_builds() {
    let output = run_fixture("pass_result_match", Expected::Pass);

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn fail_explicit_panic_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_explicit_panic", Expected::Fail);

    output.assert_contains("error[trust]: explicit panic is not supported in `fail`");
}

#[test]
fn fail_missing_trust_model_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_missing_trust_model", Expected::Fail);

    output.assert_contains(
        "error[trust]: type Account must derive TrustModel before Trust may reason about its fields",
    );
}

#[test]
fn fail_withdraw_account_wrong_id_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_withdraw_account_wrong_id", Expected::Fail);

    output.assert_contains("error[trust]: could not prove postcondition");
}

#[test]
fn fail_solver_unknown_is_rejected_by_wrapper_verifier() {
    let output = run_fixture_with_solver_status("fail_solver_unknown", Expected::Fail, "unknown");

    output.assert_contains("error[trust]: solver returned unknown");
}

#[test]
fn fail_solver_counterexample_is_rejected_by_wrapper_verifier() {
    let output =
        run_fixture_with_solver_status("fail_solver_unknown", Expected::Fail, "counterexample");

    output.assert_contains("error[trust]: solver found counterexample");
}

#[test]
fn fail_solver_timeout_is_rejected_by_wrapper_verifier() {
    let output = run_fixture_with_solver_status("fail_solver_unknown", Expected::Fail, "timeout");

    output.assert_contains("error[trust]: solver timed out");
}

#[test]
fn fail_solver_error_is_rejected_by_wrapper_verifier() {
    let output =
        run_fixture_with_solver_status("fail_solver_unknown", Expected::Fail, "solver_error");

    output.assert_contains("error[trust]: solver error");
}

#[test]
fn fail_trusted_model_stub_cannot_prove_false() {
    let output = run_fixture("fail_trusted_model_not_axiom", Expected::Fail);

    output.assert_contains("error[trust]: could not prove postcondition");
}

#[test]
fn fail_loop_missing_decreases_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_missing_decreases", Expected::Fail);

    output.assert_contains("error[trust]: loop in total function requires decreases measure");
}

#[test]
fn fail_loop_invariant_not_preserved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_loop_invariant_not_preserved", Expected::Fail);

    output.assert_contains("error[trust]: loop invariant may not be preserved");
}

#[test]
fn fail_overflow_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_overflow_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer addition cannot overflow");
}

#[test]
fn fail_i64_overflow_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_i64_overflow_unproved", Expected::Fail);

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

#[test]
fn golden_diagnostics_cover_main_failure_modes() {
    const CASES: &[(&str, Option<&str>, &str)] = &[
        (
            "fail_missing_wrapper",
            None,
            include_str!("../../../tests/golden/diagnostics/missing_wrapper.txt"),
        ),
        (
            "fail_async_total",
            None,
            include_str!("../../../tests/golden/diagnostics/unsupported_feature.txt"),
        ),
        (
            "fail_missing_trust_model",
            None,
            include_str!("../../../tests/golden/diagnostics/missing_trust_model.txt"),
        ),
        (
            "fail_public_ghost_precondition",
            None,
            include_str!("../../../tests/golden/diagnostics/public_ghost_precondition.txt"),
        ),
        (
            "fail_overflow_unproved",
            None,
            include_str!("../../../tests/golden/diagnostics/overflow_unproved.txt"),
        ),
        (
            "fail_slice_index_unproved",
            None,
            include_str!("../../../tests/golden/diagnostics/slice_index_unproved.txt"),
        ),
        (
            "fail_callee_precondition",
            None,
            include_str!("../../../tests/golden/diagnostics/callee_precondition_unproved.txt"),
        ),
        (
            "fail_postcondition_false",
            None,
            include_str!("../../../tests/golden/diagnostics/postcondition_unproved.txt"),
        ),
        (
            "fail_loop_invariant_not_preserved",
            None,
            include_str!("../../../tests/golden/diagnostics/loop_invariant_failure.txt"),
        ),
        (
            "fail_loop_missing_decreases",
            None,
            include_str!("../../../tests/golden/diagnostics/loop_decreases_failure.txt"),
        ),
        (
            "fail_solver_unknown",
            Some("unknown"),
            include_str!("../../../tests/golden/diagnostics/solver_unknown.txt"),
        ),
        (
            "fail_solver_unknown",
            Some("timeout"),
            include_str!("../../../tests/golden/diagnostics/solver_timeout.txt"),
        ),
    ];

    for (fixture, solver_status, golden) in CASES {
        let output = if let Some(solver_status) = solver_status {
            run_fixture_with_solver_status(fixture, Expected::Fail, solver_status)
        } else if *fixture == "fail_missing_wrapper" {
            run_fixture_without_wrapper(fixture, Expected::Fail)
        } else {
            run_fixture(fixture, Expected::Fail)
        };

        output.assert_trust_diagnostics_golden(golden);
    }
}
