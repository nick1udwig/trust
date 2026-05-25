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
fn pass_total_identity_writes_hir_mir_semantic_dumps_when_requested() {
    let suffix = std::process::id();
    let dump_dir = fixture_cache_dir(&format!("semantic_dump_{suffix}"));
    let dump_dir_str = dump_dir
        .to_str()
        .expect("semantic dump path should be UTF-8");
    let output = run_fixture_with_cache_and_env(
        "pass_total_identity",
        Expected::Pass,
        &format!("pass_total_identity_semantic_dump_{suffix}"),
        &format!("pass_total_identity_semantic_dump_{suffix}"),
        &[("TRUST_SEMANTIC_DUMP_DIR", dump_dir_str)],
    );

    output.assert_contains("trust: extracted HIR/MIR for 1 total function");
    output.assert_contains("trust: proved 1 total function");

    let dumps = fs::read_dir(&dump_dir)
        .expect("read semantic dump dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("read semantic dump entries");
    let dump_paths = dumps.iter().map(|entry| entry.path()).collect::<Vec<_>>();
    let hir_path = dump_paths
        .iter()
        .find(|path| path.to_string_lossy().ends_with(".hir-tree.txt"))
        .expect("expected HIR tree dump");
    let mir_path = dump_paths
        .iter()
        .find(|path| path.to_string_lossy().ends_with(".mir.txt"))
        .expect("expected MIR dump");
    let summary_path = dump_paths
        .iter()
        .find(|path| path.to_string_lossy().ends_with(".trust-semantic.txt"))
        .expect("expected Trust semantic summary");

    let hir = fs::read_to_string(hir_path).expect("read HIR dump");
    let mir = fs::read_to_string(mir_path).expect("read MIR dump");
    let summary = fs::read_to_string(summary_path).expect("read semantic summary");
    assert!(hir.contains("ident: id_i32#"));
    assert!(mir.contains("fn id_i32("));
    assert!(summary.contains("format=trust-semantic-dump-v1"));
    assert!(summary.contains("rustc_version=rustc-test"));
    assert!(summary.contains("item kind=total"));
    assert!(summary.contains("path=id_i32"));
    assert!(summary.contains("hir_match=true"));
    assert!(summary.contains("mir_match=true"));
    assert!(summary.contains("mir_function path=id_i32"));
    assert!(summary.contains("args=_1: i32"));
    assert!(summary.contains("return_type=i32"));
    assert!(summary.contains("debug_locals=x"));
    assert!(summary.contains("return_expr=x"));
}

#[test]
fn pass_semantic_let_return_postcondition_uses_mir_return_expression() {
    let suffix = std::process::id();
    let output = run_fixture_with_cache_and_env(
        "pass_semantic_let_return_postcondition",
        Expected::Pass,
        &format!("pass_semantic_let_return_postcondition_{suffix}"),
        &format!("pass_semantic_let_return_postcondition_{suffix}"),
        &[("TRUST_SEMANTIC_VERIFY", "1")],
    );

    output.assert_contains("trust: extracted HIR/MIR for 1 total function");
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
fn fail_trust_model_unsupported_field_type_is_rejected() {
    let output = run_fixture("fail_trust_model_unsupported_field_type", Expected::Fail);

    output.assert_contains("error[trust]: field type is not supported by TrustModel MVP");
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
    let suffix = std::process::id();
    let target_name = format!("pass_config_assume_warning_{suffix}");
    let cache_name = format!("pass_config_assume_warning_{suffix}");
    let output = run_fixture_with_cache(
        "pass_config_assume_no_runtime_assertion",
        Expected::Pass,
        &target_name,
        &cache_name,
    );

    output.assert_contains(
        "warning[trust]: assertions = \"assume\" disables runtime checks for executable Trust contracts",
    );
}

#[test]
fn pass_assume_config_warning_can_be_silenced() {
    let suffix = std::process::id();
    let target_name = format!("pass_config_assume_warning_silenced_{suffix}");
    let cache_name = format!("pass_config_assume_warning_silenced_{suffix}");
    let output = run_fixture_with_cache(
        "pass_config_assume_warning_silenced",
        Expected::Pass,
        &target_name,
        &cache_name,
    );

    output.assert_not_contains("warning[trust]: assertions = \"assume\"");
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
fn pass_cache_entry_records_fingerprint_summary() {
    let suffix = std::process::id();
    let target_name = format!("pass_cache_entry_summary_{suffix}");
    let cache_name = format!("pass_cache_entry_summary_{suffix}");
    let output =
        run_fixture_with_cache("pass_cache_hit", Expected::Pass, &target_name, &cache_name);

    output.assert_contains("trust: verified 1 function; cache hits 0; cache misses 1");
    let proof_entries = proof_cache_entries(&cache_name);
    assert_eq!(proof_entries.len(), 1, "expected one proof cache entry");
    let contents = fs::read_to_string(&proof_entries[0]).expect("read proof cache entry");
    assert!(contents.contains("format=trust-proof-cache-v2\n"));
    assert!(contents.contains("status=proved\n"));
    assert!(contents.contains("entry_fingerprint="));
    assert!(contents.contains("verified_items=1\n"));
    assert!(contents.contains("solver=mock\n"));
    assert!(contents.contains("vc_fingerprints="));
    assert!(contents.contains("generated_rust_fingerprint="));
    assert!(contents.contains("diagnostics_summary=none\n"));
    assert!(contents.contains("solver_transcript_path=none\n"));
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
fn fail_unknown_method_call_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_unknown_method_call", Expected::Fail);

    output.assert_contains("error[trust]: unsupported function call in `abs_value`: `x.abs`");
}

#[test]
fn fail_trait_dispatch_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_trait_dispatch", Expected::Fail);

    output.assert_contains("error[trust]: unsupported function call in `display`: `x.to_string`");
}

#[test]
fn fail_ordinary_call_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_ordinary_call", Expected::Fail);

    output.assert_contains("error[trust]: unsupported function call in `call_helper`: `helper`");
}

#[test]
fn fail_closure_body_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_closure_body", Expected::Fail);

    output.assert_contains("error[trust]: closures are not supported in `apply`");
}

#[test]
fn fail_recursive_total_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_recursive_total", Expected::Fail);

    output.assert_contains("error[trust]: unsupported function call in `recurse`: `recurse`");
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
fn pass_z3_solver_runs_crate_backend() {
    let suffix = std::process::id();
    let output = run_fixture_with_cache_and_env(
        "pass_z3_solver",
        Expected::Pass,
        &format!("pass_z3_solver_crate_{suffix}"),
        &format!("pass_z3_solver_crate_{suffix}"),
        &[],
    );

    output.assert_contains("trust: discovered 1 total function");
    output.assert_contains("trust: proved 1 total function");
}

#[test]
fn pass_z3_solver_writes_smt_dump_when_requested() {
    let suffix = std::process::id();
    let dump_dir = fixture_cache_dir(&format!("z3_smt_dump_{suffix}"));
    let dump_dir_str = dump_dir.to_str().expect("SMT dump path should be UTF-8");
    let output = run_fixture_with_cache_and_env(
        "pass_z3_solver",
        Expected::Pass,
        &format!("pass_z3_solver_smt_dump_{suffix}"),
        &format!("pass_z3_solver_smt_dump_{suffix}"),
        &[("TRUST_SMT_DUMP_DIR", dump_dir_str)],
    );

    output.assert_contains("trust: proved 1 total function");
    let dumps = fs::read_dir(&dump_dir)
        .expect("read SMT dump dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("read SMT dump entries");
    assert!(
        dumps
            .iter()
            .any(|entry| entry.path().extension().is_some_and(|ext| ext == "smt2")),
        "expected an SMT-LIB dump in {}",
        dump_dir.display()
    );
    let smt = fs::read_to_string(dumps[0].path()).expect("read SMT dump");
    assert!(smt.contains("(check-sat)"));
    assert!(smt.contains("assert"));
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
    output.assert_contains("--> Trust total `add_one`");
    output.assert_contains("x + 1");
    output.assert_contains("help[trust]: add executable preconditions");
}

#[test]
fn fail_semantic_block_add_unproved_is_rejected_by_mir_verifier() {
    let suffix = std::process::id();
    let output = run_fixture_with_cache_and_env(
        "fail_semantic_block_add_unproved",
        Expected::Fail,
        &format!("fail_semantic_block_add_unproved_{suffix}"),
        &format!("fail_semantic_block_add_unproved_{suffix}"),
        &[("TRUST_SEMANTIC_VERIFY", "1")],
    );

    output.assert_contains("trust: extracted HIR/MIR for 1 total function");
    output.assert_contains("error[trust]: could not prove integer addition cannot overflow");
    output.assert_contains("x + 1");
}

#[test]
fn fail_semantic_block_div_unproved_is_rejected_by_mir_verifier() {
    let suffix = std::process::id();
    let output = run_fixture_with_cache_and_env(
        "fail_semantic_block_div_unproved",
        Expected::Fail,
        &format!("fail_semantic_block_div_unproved_{suffix}"),
        &format!("fail_semantic_block_div_unproved_{suffix}"),
        &[("TRUST_SEMANTIC_VERIFY", "1")],
    );

    output.assert_contains("trust: extracted HIR/MIR for 1 total function");
    output.assert_contains("error[trust]: could not prove integer division denominator is nonzero");
    output.assert_contains("x / y");
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
fn fail_variable_addition_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_variable_addition", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer addition cannot overflow");
}

#[test]
fn fail_division_by_zero_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_division_by_zero", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer division denominator is nonzero");
}

#[test]
fn fail_parenthesized_division_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_parenthesized_division", Expected::Fail);

    output.assert_contains("error[trust]: could not prove integer division denominator is nonzero");
}

#[test]
fn fail_remainder_by_zero_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_remainder_by_zero", Expected::Fail);

    output
        .assert_contains("error[trust]: could not prove integer remainder denominator is nonzero");
}

#[test]
fn fail_slice_index_unproved_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_slice_index_unproved", Expected::Fail);

    output.assert_contains("error[trust]: could not prove index is in bounds");
}

#[test]
fn fail_vec_index_is_rejected_by_wrapper_verifier() {
    let output = run_fixture("fail_vec_index", Expected::Fail);

    output.assert_contains("error[trust]: unsupported index expression in `get_vec`: `xs[i]`");
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
