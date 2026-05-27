use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use trust_core::{
    metadata::TrustMetadata,
    verifier::{
        SemanticArithmeticKind, SemanticArithmeticOperation, SemanticBranch, SemanticBranchArm,
        SemanticCall, SemanticContractBinding, SemanticContractBindingKind, SemanticFieldAccess,
        SemanticMatch, SemanticMatchArm, SemanticMatchPayload, SemanticParam, SemanticSliceIndex,
        SemanticTrustCallee, TrustFunctionSemantics,
    },
};

use crate::{deterministic_test_mode, exit_code, metadata_path, plural, rustc_verbose_version};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticItemMatch {
    item_kind: String,
    item_id: String,
    rust_function_path: String,
    resolved_rust_function_path: Option<String>,
    source_span: String,
    hir_match: bool,
    mir_match: bool,
    mir_function: Option<MirFunctionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirFunctionSummary {
    path: String,
    args: Vec<MirArg>,
    return_type: String,
    locals: Vec<MirLocal>,
    debug_locals: Vec<MirDebugLocal>,
    assignments: Vec<MirAssignment>,
    terminators: Vec<MirTerminator>,
    return_expr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirArg {
    place: String,
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirLocal {
    place: String,
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirDebugLocal {
    name: String,
    place: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirAssignment {
    block: Option<String>,
    place: String,
    expression: String,
    statement_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirTerminator {
    block: Option<String>,
    expression: String,
    statement_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSwitchTarget {
    value: String,
    block: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirGuardedEdge {
    predecessor: String,
    successor: String,
    guard: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSemanticReturnFact {
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelFieldMap {
    ty: String,
    fields: Vec<ModelField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelField {
    name: String,
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirVariantProjection {
    place: String,
    variant: String,
    field_index: usize,
    ty: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SemanticViews {
    pub(crate) semantics: Vec<TrustFunctionSemantics>,
    pub(crate) source_spans: Vec<SemanticSourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticSourceSpan {
    pub(crate) rust_function_path: String,
    pub(crate) source_span: String,
}

pub(crate) fn maybe_extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
) -> Result<SemanticViews, String> {
    let dump_dir = env::var_os("TRUST_SEMANTIC_DUMP_DIR").map(PathBuf::from);
    if dump_dir.is_none() && !semantic_verify_enabled() {
        return Ok(SemanticViews::default());
    }

    extract_semantic_views(rustc, rustc_args, metadata, dump_dir)
}

fn semantic_verify_enabled() -> bool {
    !matches!(
        env::var("TRUST_SEMANTIC_VERIFY").as_deref(),
        Ok("0" | "false" | "off")
    )
}

fn extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
    dump_dir: Option<PathBuf>,
) -> Result<SemanticViews, String> {
    let hir = run_rustc_unpretty(rustc, rustc_args, "hir-tree")?;
    let mir = run_rustc_unpretty(rustc, rustc_args, "mir")?;
    let item_matches = semantic_item_matches(metadata, &hir, &mir);
    reject_unmatched_total_items(&item_matches)?;
    let rustc_version = semantic_rustc_version(rustc);

    if let Some(dump_dir) = dump_dir {
        fs::create_dir_all(&dump_dir)
            .map_err(|err| format!("failed to create Trust semantic dump directory: {err}"))?;
        let base = semantic_dump_base(metadata, rustc_args);
        write_semantic_dump(&dump_dir.join(format!("{base}.hir-tree.txt")), &hir)?;
        write_semantic_dump(&dump_dir.join(format!("{base}.mir.txt")), &mir)?;
        write_semantic_dump(
            &dump_dir.join(format!("{base}.trust-semantic.txt")),
            &semantic_summary(metadata, rustc_args, &item_matches, &rustc_version),
        )?;
    }

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

    Ok(SemanticViews {
        semantics: verifier_semantics(metadata, &item_matches),
        source_spans: semantic_source_spans(&item_matches),
    })
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
            let mir_function = extract_mir_function_summary(mir, &item.rust_function_path);
            SemanticItemMatch {
                item_kind: item.item_kind.clone(),
                item_id: item.item_id.clone(),
                rust_function_path: item.rust_function_path.clone(),
                resolved_rust_function_path: mir_function
                    .as_ref()
                    .map(|mir_function| mir_function.path.clone()),
                source_span: hir_function_span(hir, &item.rust_function_path)
                    .unwrap_or_else(|| item.source_span.clone()),
                hir_match: hir_contains_function(hir, &item.rust_function_path),
                mir_match: mir_function.is_some(),
                mir_function,
            }
        })
        .collect()
}

fn semantic_source_spans(item_matches: &[SemanticItemMatch]) -> Vec<SemanticSourceSpan> {
    item_matches
        .iter()
        .filter(|item| item.source_span != "unknown")
        .map(|item| SemanticSourceSpan {
            rust_function_path: item.rust_function_path.clone(),
            source_span: item.source_span.clone(),
        })
        .collect()
}

fn verifier_semantics(
    metadata: &[TrustMetadata],
    item_matches: &[SemanticItemMatch],
) -> Vec<TrustFunctionSemantics> {
    let model_fields = model_field_maps(metadata);
    let trust_callees = semantic_trust_callees(metadata, item_matches);
    item_matches
        .iter()
        .filter(|item| item.item_kind == "total" && item.hir_match)
        .filter_map(|item| {
            let mir_function = item.mir_function.as_ref()?;
            let metadata_item = metadata.iter().find(|metadata_item| {
                metadata_item.item_kind == "total" && metadata_item.item_id == item.item_id
            })?;
            let mut contract_bindings =
                semantic_contract_bindings(metadata_item, mir_function, &model_fields);
            contract_bindings.extend(semantic_local_bindings(mir_function, &model_fields));
            let contract_bindings = dedup_contract_bindings(contract_bindings);

            Some(TrustFunctionSemantics {
                rust_function_path: item.rust_function_path.clone(),
                params: mir_function
                    .args
                    .iter()
                    .map(|arg| SemanticParam {
                        name: mir_function
                            .local_name_for_place(&arg.place)
                            .unwrap_or(&arg.place)
                            .to_string(),
                        ty: arg.ty.clone(),
                    })
                    .collect(),
                return_type: mir_function.return_type.clone(),
                local_types: mir_function.source_local_types(),
                contract_bindings,
                return_expression: mir_function
                    .normalized_return_expression_with_models(&model_fields),
                arithmetic_operations: mir_function
                    .semantic_arithmetic_operations_with_models(&model_fields),
                slice_indexes: mir_function.semantic_slice_indexes_with_models(&model_fields),
                calls: mir_function.semantic_calls_with_models(&model_fields, &trust_callees),
                field_accesses: mir_function.semantic_field_accesses(&model_fields),
                matches: mir_function.semantic_matches(&model_fields),
                branches: mir_function.semantic_branches(&model_fields),
            })
        })
        .collect()
}

fn semantic_trust_callees(
    metadata: &[TrustMetadata],
    item_matches: &[SemanticItemMatch],
) -> Vec<SemanticTrustCallee> {
    item_matches
        .iter()
        .filter(|item| item.item_kind == "total" && item.hir_match)
        .filter_map(|item| {
            let metadata_item = metadata.iter().find(|metadata_item| {
                metadata_item.item_kind == "total" && metadata_item.item_id == item.item_id
            })?;
            let mir_function = item.mir_function.as_ref()?;
            Some(SemanticTrustCallee {
                rust_function_path: mir_function.path.clone(),
                params: mir_function
                    .args
                    .iter()
                    .map(|arg| SemanticParam {
                        name: mir_function
                            .local_name_for_place(&arg.place)
                            .unwrap_or(&arg.place)
                            .to_string(),
                        ty: arg.ty.clone(),
                    })
                    .collect(),
                preconditions: trust_preconditions(metadata_item),
            })
        })
        .collect()
}

fn trust_preconditions(item: &TrustMetadata) -> Vec<String> {
    item.contracts_original
        .iter()
        .zip(item.contract_classes.iter())
        .filter(|(_contract, class)| matches!(class.as_str(), "given executable" | "given ghost"))
        .map(|(contract, _class)| contract.clone())
        .collect()
}

fn semantic_guards_for_base_alias(
    guards: &[String],
    canonical_base: &str,
    alias: &str,
) -> Vec<String> {
    if canonical_base == alias {
        return guards.to_vec();
    }

    guards
        .iter()
        .map(|guard| {
            guard.replace(
                &format!("{canonical_base}.len()"),
                &format!("{alias}.len()"),
            )
        })
        .collect()
}

fn semantic_contract_bindings(
    item: &TrustMetadata,
    mir_function: &MirFunctionSummary,
    model_fields: &[ModelFieldMap],
) -> Vec<SemanticContractBinding> {
    let params = mir_function
        .args
        .iter()
        .map(|arg| SemanticParam {
            name: mir_function
                .local_name_for_place(&arg.place)
                .unwrap_or(&arg.place)
                .to_string(),
            ty: arg.ty.clone(),
        })
        .collect::<Vec<_>>();
    let mut bindings = Vec::new();

    for ((original, normalized), class) in item
        .contracts_original
        .iter()
        .zip(item.contracts_normalized.iter())
        .zip(item.contract_classes.iter())
        .filter(|((_original, _normalized), class)| {
            matches!(
                class.as_str(),
                "given executable" | "given ghost" | "gives executable" | "gives ghost"
            )
        })
    {
        let expression = if class.starts_with("gives") {
            normalized
        } else {
            original
        };
        bindings.extend(contract_bindings_for_expression(
            expression,
            class,
            &params,
            &mir_function.return_type,
            model_fields,
        ));
    }

    dedup_contract_bindings(bindings)
}

fn semantic_local_bindings(
    mir_function: &MirFunctionSummary,
    model_fields: &[ModelFieldMap],
) -> Vec<SemanticContractBinding> {
    let mut bindings = Vec::new();
    for local in mir_function
        .debug_locals
        .iter()
        .filter(|local| !mir_function.is_arg_place(&local.place) && is_ident_token(&local.name))
    {
        let Some(ty) = mir_function.mir_expression_type(&local.place, model_fields) else {
            continue;
        };
        bindings.push(SemanticContractBinding {
            expression: local.name.clone(),
            name: local.name.clone(),
            kind: SemanticContractBindingKind::Local,
            ty: ty.clone(),
        });
        if let Some(initializer) = mir_function.semantic_local_initializer(local, model_fields) {
            bindings.push(SemanticContractBinding {
                expression: initializer,
                name: local.name.clone(),
                kind: SemanticContractBindingKind::LocalInitializer,
                ty,
            });
        }
    }

    dedup_contract_bindings(bindings)
}

fn contract_bindings_for_expression(
    expression: &str,
    class: &str,
    params: &[SemanticParam],
    return_type: &str,
    model_fields: &[ModelFieldMap],
) -> Vec<SemanticContractBinding> {
    let tokens = contract_tokens(expression);
    let mut bindings = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if !is_ident_token(token) || token_is_path_segment(&tokens, idx) {
            continue;
        }

        if let Some(param) = params.iter().find(|param| param.name == *token) {
            bindings.push(SemanticContractBinding {
                expression: expression.to_string(),
                name: param.name.clone(),
                kind: SemanticContractBindingKind::Param,
                ty: param.ty.clone(),
            });
            if let Some(field_binding) = contract_field_binding(
                expression,
                &tokens,
                idx,
                &param.name,
                &param.ty,
                model_fields,
            ) {
                bindings.push(field_binding);
            }
            continue;
        }

        if class.starts_with("gives") && token == "out" {
            bindings.push(SemanticContractBinding {
                expression: expression.to_string(),
                name: "out".to_string(),
                kind: SemanticContractBindingKind::Result,
                ty: return_type.to_string(),
            });
            if let Some(field_binding) =
                contract_field_binding(expression, &tokens, idx, "out", return_type, model_fields)
            {
                bindings.push(field_binding);
            }
        }
    }

    bindings
}

fn contract_field_binding(
    expression: &str,
    tokens: &[String],
    base_idx: usize,
    base_name: &str,
    base_ty: &str,
    model_fields: &[ModelFieldMap],
) -> Option<SemanticContractBinding> {
    if tokens.get(base_idx + 1)? != "." {
        return None;
    }
    let field = tokens.get(base_idx + 2)?;
    if !is_ident_token(field) || tokens.get(base_idx + 3).is_some_and(|token| token == "(") {
        return None;
    }
    let model = model_fields
        .iter()
        .find(|model| model.ty == type_name_tail(base_ty))?;
    let model_field = model
        .fields
        .iter()
        .find(|model_field| model_field.name == *field)?;

    Some(SemanticContractBinding {
        expression: expression.to_string(),
        name: format!("{base_name}.{field}"),
        kind: SemanticContractBindingKind::Field,
        ty: model_field.ty.clone(),
    })
}

fn dedup_contract_bindings(bindings: Vec<SemanticContractBinding>) -> Vec<SemanticContractBinding> {
    let mut deduped = Vec::new();
    for binding in bindings {
        if deduped.iter().any(|existing| existing == &binding) {
            continue;
        }
        deduped.push(binding);
    }
    deduped
}

fn contract_binding_summary(bindings: &[SemanticContractBinding]) -> String {
    bindings
        .iter()
        .map(|binding| {
            format!(
                "{}:{}:{}:{}",
                binding.expression,
                binding.name,
                contract_binding_kind_name(binding.kind),
                binding.ty
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn contract_binding_kind_name(kind: SemanticContractBindingKind) -> &'static str {
    match kind {
        SemanticContractBindingKind::Param => "param",
        SemanticContractBindingKind::Result => "result",
        SemanticContractBindingKind::Field => "field",
        SemanticContractBindingKind::Local => "local",
        SemanticContractBindingKind::LocalInitializer => "local_initializer",
    }
}

fn contract_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }
        if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
        if !ch.is_whitespace() {
            tokens.push(ch.to_string());
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn is_ident_token(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn token_is_path_segment(tokens: &[String], idx: usize) -> bool {
    tokens.get(idx.checked_sub(2).unwrap_or(usize::MAX)) == Some(&":".to_string())
        && tokens.get(idx.checked_sub(1).unwrap_or(usize::MAX)) == Some(&":".to_string())
        || tokens.get(idx + 1) == Some(&":".to_string())
            && tokens.get(idx + 2) == Some(&":".to_string())
}

fn trust_callee_for_call(
    callee: &str,
    trust_callees: &[SemanticTrustCallee],
) -> Option<SemanticTrustCallee> {
    let mut matches = trust_callees
        .iter()
        .filter(|trust_callee| semantic_path_matches_call(&trust_callee.rust_function_path, callee))
        .cloned();
    let first = matches.next()?;
    if matches.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn semantic_path_matches_call(path: &str, call: &str) -> bool {
    path == call || path.ends_with(&format!("::{call}")) || call.ends_with(&format!("::{path}"))
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

fn model_field_maps(metadata: &[TrustMetadata]) -> Vec<ModelFieldMap> {
    metadata
        .iter()
        .filter(|item| item.item_kind == "trust_model")
        .filter_map(|item| {
            let fields = parse_model_fields(&item.function_source);
            if fields.is_empty() {
                return None;
            }
            Some(ModelFieldMap {
                ty: function_leaf_name(&item.rust_function_path).to_string(),
                fields,
            })
        })
        .collect()
}

fn parse_model_fields(source: &str) -> Vec<ModelField> {
    let Some(open) = source.find('{') else {
        return Vec::new();
    };
    let Some(close) = source.rfind('}') else {
        return Vec::new();
    };

    parse_comma_separated(&source[open + 1..close])
        .into_iter()
        .filter_map(|field| {
            let field = field.trim().strip_prefix("pub ").unwrap_or(field.trim());
            let (name, ty) = field.split_once(':')?;
            Some(ModelField {
                name: name.trim().to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect()
}

fn type_name_tail(ty: &str) -> String {
    ty.trim()
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .rsplit("::")
        .next()
        .unwrap_or(ty)
        .trim()
        .to_string()
}

fn semantic_unmodeled_field_access(
    base: &str,
    field: &str,
    owner_type: &str,
    field_ty: &str,
) -> Option<SemanticFieldAccess> {
    if !field_owner_requires_trust_model(owner_type) || !is_source_visible_expression(base) {
        return None;
    }

    Some(SemanticFieldAccess {
        base: base.to_string(),
        field: field.to_string(),
        owner_type: owner_type.to_string(),
        field_type: field_ty.to_string(),
        expression: format!("{base}.{field}"),
    })
}

fn field_owner_requires_trust_model(owner_type: &str) -> bool {
    let owner_type = owner_type.trim();
    !owner_type.is_empty()
        && owner_type != "bool"
        && owner_type != "()"
        && owner_type != "str"
        && !supported_mir_integer_type(owner_type)
        && !owner_type.starts_with('&')
        && !owner_type.starts_with('*')
        && !owner_type.starts_with('[')
        && !owner_type.starts_with('(')
        && !owner_type.starts_with("Option<")
        && !owner_type.starts_with("Result<")
}

fn is_source_visible_expression(expr: &str) -> bool {
    expr.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && expr
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
}

fn hir_contains_function(hir: &str, expected_path: &str) -> bool {
    let matches = hir
        .lines()
        .filter_map(hir_owner_path)
        .filter(|path| hir_function_path_matches(path, expected_path))
        .collect::<Vec<_>>();

    if expected_path.contains("::") {
        !matches.is_empty()
    } else {
        matches.len() == 1
    }
}

fn hir_owner_path(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with("DefId(") || !line.contains("=> OwnerNodes") {
        return None;
    }

    let (_prefix, rest) = line.split_once(" ~ ")?;
    let (path, _suffix) = rest.split_once(") => OwnerNodes")?;
    Some(path.trim())
}

fn hir_function_path_matches(path: &str, expected_path: &str) -> bool {
    if expected_path.contains("::") {
        path == expected_path
            || path.ends_with(&format!("::{expected_path}"))
            || expected_path.ends_with(&format!("::{path}"))
    } else {
        function_leaf_name(path) == expected_path
    }
}

fn hir_function_span(hir: &str, expected_path: &str) -> Option<String> {
    let lines = hir.lines().collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            let path = hir_owner_path(line)?;
            hir_function_path_matches(path, expected_path).then_some(idx)
        })
        .collect::<Vec<_>>();

    if matches.len() != 1 {
        return None;
    }

    hir_owner_item_span(&lines, matches[0])
}

fn hir_owner_item_span(lines: &[&str], owner_idx: usize) -> Option<String> {
    let owner_end = lines
        .iter()
        .enumerate()
        .skip(owner_idx + 1)
        .find(|(_, line)| line.starts_with("DefId("))
        .map(|(idx, _)| idx)
        .unwrap_or(lines.len());
    let mut saw_fn_kind = false;
    let mut best_span: Option<(usize, String)> = None;

    for line in &lines[owner_idx + 1..owner_end] {
        let trimmed = line.trim();
        if trimmed.starts_with("kind: Fn") {
            saw_fn_kind = true;
        }
        if !saw_fn_kind {
            continue;
        }

        let Some(span) = hir_span_line(line) else {
            continue;
        };
        let indent = line.len() - line.trim_start().len();
        if best_span
            .as_ref()
            .is_none_or(|(best_indent, _)| indent < *best_indent)
        {
            best_span = Some((indent, span));
        }
    }

    best_span.map(|(_, span)| span)
}

fn hir_span_line(line: &str) -> Option<String> {
    let span = line.trim().strip_prefix("span: ")?.trim();
    let span = span.strip_suffix(',').unwrap_or(span).trim();
    if span.is_empty() || span.starts_with("no-location") {
        return None;
    }

    Some(span.to_string())
}

fn extract_mir_function_summary(mir: &str, name: &str) -> Option<MirFunctionSummary> {
    let lines = mir.lines().collect::<Vec<_>>();
    let signature_idx = mir_function_signature_idx(&lines, name)?;
    let signature = parse_mir_signature(lines[signature_idx].trim())?;
    let function_end = mir_function_end(&lines, signature_idx).unwrap_or(lines.len());
    let function_lines = &lines[signature_idx + 1..function_end];

    Some(MirFunctionSummary {
        path: signature.path,
        args: signature.args,
        return_type: signature.return_type,
        locals: extract_mir_locals(function_lines),
        debug_locals: extract_mir_debug_locals(function_lines),
        assignments: extract_mir_assignments(function_lines),
        terminators: extract_mir_terminators(function_lines),
        return_expr: extract_mir_return_expr(function_lines),
    })
}

fn mir_function_signature_idx(lines: &[&str], expected_path: &str) -> Option<usize> {
    if expected_path.contains("::") {
        let mut qualified_matches = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| mir_signature_matches_qualified_path(line.trim(), expected_path))
            .map(|(idx, _)| idx);
        if let Some(first) = qualified_matches.next() {
            if qualified_matches.next().is_none() {
                return Some(first);
            }
            return None;
        }
    }

    let leaf = function_leaf_name(expected_path);
    let mut leaf_matches = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| mir_signature_matches_leaf(line.trim(), leaf))
        .map(|(idx, _)| idx);
    let first = leaf_matches.next()?;
    if leaf_matches.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn mir_function_end(lines: &[&str], signature_idx: usize) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(signature_idx + 1)
        .find(|(_, line)| line.trim() == "}" && line.starts_with('}'))
        .map(|(idx, _)| idx)
}

impl MirFunctionSummary {
    fn local_name_for_place(&self, place: &str) -> Option<&str> {
        self.debug_locals
            .iter()
            .find(|local| local.place == place)
            .map(|local| local.name.as_str())
    }

    fn local_names_for_place(&self, place: &str) -> Vec<&str> {
        let mut names = Vec::new();
        for local in self
            .debug_locals
            .iter()
            .filter(|local| local.place == place)
        {
            if !names.contains(&local.name.as_str()) {
                names.push(local.name.as_str());
            }
        }
        names
    }

    fn source_local_types(&self) -> Vec<String> {
        let mut types = Vec::new();
        for local in &self.debug_locals {
            let Some(ty) = self.mir_expression_type(&local.place, &[]) else {
                continue;
            };
            if !types.iter().any(|existing| existing == &ty) {
                types.push(ty);
            }
        }
        types
    }

    fn semantic_local_initializer(
        &self,
        local: &MirDebugLocal,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        let assignments = self
            .assignments
            .iter()
            .filter(|assignment| assignment.place == local.place)
            .collect::<Vec<_>>();
        let first = assignments.first()?;
        if !self.guards_for_assignment(first, model_fields).is_empty() {
            return None;
        }
        if assignments
            .iter()
            .skip(1)
            .any(|assignment| !self.guarded_self_update(local, assignment, model_fields))
        {
            return None;
        }

        self.normalized_mir_expression_with_models(&first.expression, model_fields)
    }

    fn guarded_self_update(
        &self,
        local: &MirDebugLocal,
        assignment: &MirAssignment,
        model_fields: &[ModelFieldMap],
    ) -> bool {
        let guards = self.guards_for_assignment(assignment, model_fields);
        let Some(expression) =
            self.normalized_mir_expression_with_models(&assignment.expression, model_fields)
        else {
            return false;
        };
        !guards.is_empty()
            && guards.iter().any(|guard| {
                guard_variables(guard)
                    .iter()
                    .any(|variable| variable == &local.name)
            })
            && semantic_self_update_expression(&expression, &local.name)
    }

    fn local_aliases_for_place_with_models(
        &self,
        place: &str,
        model_fields: &[ModelFieldMap],
    ) -> Vec<String> {
        let mut aliases = self
            .local_names_for_place(place)
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if let Some(normalized) = self.normalized_mir_expression_with_models(place, model_fields) {
            if !aliases.contains(&normalized) {
                aliases.push(normalized);
            }
        }
        aliases
    }

    fn type_for_place(&self, place: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|arg| arg.place == place)
            .map(|arg| arg.ty.as_str())
            .or_else(|| {
                self.locals
                    .iter()
                    .find(|local| local.place == place)
                    .map(|local| local.ty.as_str())
            })
    }

    #[cfg(test)]
    fn normalized_return_expression(&self) -> Option<String> {
        self.normalized_return_expression_with_models(&[])
    }

    fn normalized_return_expression_with_models(
        &self,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        self.return_expr
            .as_deref()
            .and_then(|expr| self.normalized_mir_expression_with_models(expr, model_fields))
    }

    fn normalized_mir_expression_with_models(
        &self,
        expr: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        self.normalized_mir_expression_with_depth(expr, 0, model_fields)
    }

    fn normalized_mir_expression_with_depth(
        &self,
        expr: &str,
        depth: usize,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        if depth > 8 {
            return None;
        }
        let expr = strip_mir_move_or_copy(expr.trim());
        if let Some(referent) = mir_borrow(expr) {
            return self.normalized_mir_expression_with_depth(referent, depth + 1, model_fields);
        }
        if self.is_arg_place(expr) {
            if let Some(source_name) = self.local_name_for_place(expr) {
                return Some(source_name.to_string());
            }
        }
        if let Some(assignment) = self.assignment_for_place(expr) {
            if let Some(normalized) = self.normalized_mir_expression_with_depth(
                &assignment.expression,
                depth + 1,
                model_fields,
            ) {
                return Some(normalized);
            }
        }
        if let Some(source_name) = self.local_name_for_place(expr) {
            return Some(source_name.to_string());
        }
        if let Some(constant) = mir_const_value(expr) {
            return Some(constant);
        }
        if let Some((ty, fields)) = mir_aggregate_fields(expr) {
            let fields = fields
                .iter()
                .map(|(field, value)| {
                    Some(format!(
                        "{}: {}",
                        field,
                        self.normalized_mir_expression_with_depth(value, depth + 1, model_fields)?
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            return Some(format!("{ty} {{ {} }}", fields.join(", ")));
        }
        if let Some(elements) = mir_tuple_elements(expr) {
            let elements = elements
                .iter()
                .map(|element| {
                    self.normalized_mir_expression_with_depth(element, depth + 1, model_fields)
                })
                .collect::<Option<Vec<_>>>()?;
            if elements.len() == 1 {
                return Some(format!("({},)", elements[0]));
            }
            return Some(format!("({})", elements.join(", ")));
        }
        if let Some(operation) = self.normalized_mir_operation(expr, depth + 1, model_fields) {
            return Some(operation);
        }
        if let Some((callee, args)) = mir_call(expr) {
            if mir_checked_add_type(callee).is_some() && args.len() == 2 {
                let receiver =
                    self.normalized_mir_expression_with_depth(&args[0], depth + 1, model_fields)?;
                let rhs =
                    self.normalized_mir_expression_with_depth(&args[1], depth + 1, model_fields)?;
                return Some(format!("{receiver}.checked_add({rhs})"));
            }
            let args = args
                .iter()
                .map(|arg| self.normalized_mir_expression_with_depth(arg, depth + 1, model_fields))
                .collect::<Option<Vec<_>>>()?;
            return Some(format!("{callee}({})", args.join(",")));
        }
        if let Some((base, index)) = mir_slice_index(expr) {
            return Some(format!(
                "{}[{}]",
                self.normalized_mir_expression_with_depth(base, depth + 1, model_fields)?,
                self.normalized_mir_expression_with_depth(index, depth + 1, model_fields)?
            ));
        }
        if let Some((place, field, ty)) = mir_projection(expr) {
            if let Some(field_access) = self.semantic_field_access(place, field, ty, model_fields) {
                return Some(field_access.expression);
            }
            if field == "0" {
                let assignment = self.assignment_for_place(place)?;
                return self.normalized_mir_operation(
                    &assignment.expression,
                    depth + 1,
                    model_fields,
                );
            }
        }

        None
    }

    fn normalized_mir_operation(
        &self,
        expr: &str,
        depth: usize,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        let (op, args) = expr.split_once('(')?;
        let args = args.strip_suffix(')')?;
        let args = parse_mir_call_args(args);
        match (op.trim(), args.as_slice()) {
            ("Add", [left, right]) | ("AddWithOverflow", [left, right]) => {
                Some(self.normalized_mir_binary_operation(left, "+", right, depth, model_fields)?)
            }
            ("Sub", [left, right]) | ("SubWithOverflow", [left, right]) => {
                Some(self.normalized_mir_binary_operation(left, "-", right, depth, model_fields)?)
            }
            ("Mul", [left, right]) | ("MulWithOverflow", [left, right]) => {
                Some(self.normalized_mir_binary_operation(left, "*", right, depth, model_fields)?)
            }
            ("Div", [left, right]) => {
                Some(self.normalized_mir_binary_operation(left, "/", right, depth, model_fields)?)
            }
            ("Rem", [left, right]) => {
                Some(self.normalized_mir_binary_operation(left, "%", right, depth, model_fields)?)
            }
            ("Neg", [value]) | ("NegWithOverflow", [value]) => Some(format!(
                "-{}",
                self.normalized_mir_expression_with_depth(value, depth + 1, model_fields)?
            )),
            ("PtrMetadata", [base]) => Some(format!(
                "{}.len()",
                self.normalized_mir_expression_with_depth(base, depth + 1, model_fields)?
            )),
            _ => None,
        }
    }

    fn normalized_mir_binary_operation(
        &self,
        left: &str,
        operator: &str,
        right: &str,
        depth: usize,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        let left = self.normalized_mir_expression_with_depth(left, depth + 1, model_fields)?;
        let right = self.normalized_mir_expression_with_depth(right, depth + 1, model_fields)?;
        Some(semantic_binary_expression(&left, operator, &right))
    }

    fn mir_expression_type(&self, expr: &str, model_fields: &[ModelFieldMap]) -> Option<String> {
        let expr = strip_mir_move_or_copy(expr);
        if let Some(ty) = mir_const_type(expr) {
            return Some(ty);
        }
        if let Some(ty) = self.type_for_place(expr) {
            return Some(ty.to_string());
        }
        if let Some((place, field, ty)) = mir_projection(expr) {
            if let Some(field_access) = self.semantic_field_access(place, field, ty, model_fields) {
                return Some(field_access.field_type);
            }
            return Some(ty.to_string());
        }
        if let Some((_kind, args)) = mir_checked_arithmetic_operation(expr) {
            return args
                .iter()
                .find_map(|arg| self.mir_expression_type(arg, model_fields))
                .filter(|ty| supported_mir_integer_type(ty));
        }
        self.assignment_for_place(expr)
            .and_then(|assignment| self.mir_expression_type(&assignment.expression, model_fields))
    }

    fn semantic_arithmetic_type(
        &self,
        args: &[String],
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        args.iter()
            .find_map(|arg| self.mir_expression_type(arg, model_fields))
            .filter(|ty| supported_mir_integer_type(ty))
    }

    fn normalized_mir_predicate_with_models(
        &self,
        expr: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        self.normalized_mir_predicate_with_depth(expr, 0, model_fields)
    }

    fn normalized_mir_predicate_with_depth(
        &self,
        expr: &str,
        depth: usize,
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        if depth > 8 {
            return None;
        }
        let expr = strip_mir_move_or_copy(expr.trim());
        if self.is_arg_place(expr) {
            return None;
        }
        if let Some(assignment) = self.assignment_for_place(expr) {
            return self.normalized_mir_predicate_with_depth(
                &assignment.expression,
                depth + 1,
                model_fields,
            );
        }

        let (op, args) = expr.split_once('(')?;
        let args = args.strip_suffix(')')?;
        let args = parse_mir_call_args(args);
        let operator = mir_comparison_operator(op.trim())?;
        let [left, right] = args.as_slice() else {
            return None;
        };

        Some(format!(
            "{} {operator} {}",
            self.normalized_mir_expression_with_depth(left, depth + 1, model_fields)?,
            self.normalized_mir_expression_with_depth(right, depth + 1, model_fields)?
        ))
    }

    fn guards_for_assignment(
        &self,
        assignment: &MirAssignment,
        model_fields: &[ModelFieldMap],
    ) -> Vec<String> {
        let Some(block) = assignment.block.as_deref() else {
            return Vec::new();
        };

        self.guards_for_location(block, Some(assignment.statement_index), model_fields)
    }

    fn guards_for_location(
        &self,
        block: &str,
        statement_index: Option<usize>,
        model_fields: &[ModelFieldMap],
    ) -> Vec<String> {
        let dominators = self.block_dominators();
        self.guarded_edges(model_fields)
            .into_iter()
            .filter_map(|edge| {
                let guard = edge.guard?;
                if !dominates(&dominators, &edge.successor, block) {
                    return None;
                }
                if self.guard_invalidated_before(
                    &guard,
                    &edge.successor,
                    block,
                    statement_index,
                    &dominators,
                ) {
                    return None;
                }
                Some(guard)
            })
            .fold(Vec::new(), |mut guards, guard| {
                if !guards.iter().any(|existing| existing == &guard) {
                    guards.push(guard);
                }
                guards
            })
    }

    fn guarded_edges(&self, model_fields: &[ModelFieldMap]) -> Vec<MirGuardedEdge> {
        self.terminators
            .iter()
            .flat_map(|terminator| self.guarded_edges_for_terminator(terminator, model_fields))
            .collect()
    }

    fn guarded_edges_for_terminator(
        &self,
        terminator: &MirTerminator,
        model_fields: &[ModelFieldMap],
    ) -> Vec<MirGuardedEdge> {
        let Some(predecessor) = terminator.block.as_ref() else {
            return Vec::new();
        };

        if let Some((condition, targets)) = mir_switch(&terminator.expression) {
            let explicit_values = explicit_mir_switch_values(&targets);
            return targets
                .iter()
                .map(|target| MirGuardedEdge {
                    predecessor: predecessor.clone(),
                    successor: target.block.clone(),
                    guard: self.guard_for_switch_target(
                        condition,
                        &target.value,
                        &explicit_values,
                        model_fields,
                    ),
                })
                .collect();
        }

        mir_successor_targets(&terminator.expression)
            .into_iter()
            .map(|successor| MirGuardedEdge {
                predecessor: predecessor.clone(),
                successor,
                guard: None,
            })
            .collect()
    }

    fn block_dominators(&self) -> BTreeMap<String, BTreeSet<String>> {
        let blocks = self.mir_blocks();
        let Some(entry) = blocks
            .iter()
            .find(|block| block.as_str() == "bb0")
            .or(blocks.first())
        else {
            return BTreeMap::new();
        };
        let all_blocks = blocks.iter().cloned().collect::<BTreeSet<_>>();
        let edges = self
            .terminators
            .iter()
            .flat_map(|terminator| self.guarded_edges_for_terminator(terminator, &[]))
            .collect::<Vec<_>>();
        let mut dominators = BTreeMap::new();

        for block in &blocks {
            let mut set = BTreeSet::new();
            if block == entry {
                set.insert(block.clone());
            } else {
                set = all_blocks.clone();
            }
            dominators.insert(block.clone(), set);
        }

        let mut changed = true;
        while changed {
            changed = false;

            for block in blocks.iter().filter(|block| *block != entry) {
                let predecessor_sets = edges
                    .iter()
                    .filter(|edge| edge.successor == *block)
                    .filter_map(|edge| dominators.get(&edge.predecessor))
                    .cloned()
                    .collect::<Vec<_>>();
                let mut next = intersect_sets(&predecessor_sets).unwrap_or_default();
                next.insert(block.clone());

                if dominators.get(block).is_none_or(|current| current != &next) {
                    dominators.insert(block.clone(), next);
                    changed = true;
                }
            }
        }

        dominators
    }

    fn mir_blocks(&self) -> Vec<String> {
        let mut blocks = Vec::new();
        for block in self
            .assignments
            .iter()
            .filter_map(|assignment| assignment.block.as_ref())
            .chain(
                self.terminators
                    .iter()
                    .filter_map(|terminator| terminator.block.as_ref()),
            )
        {
            push_unique(&mut blocks, block.clone());
        }
        for successor in self
            .terminators
            .iter()
            .flat_map(|terminator| mir_successor_targets(&terminator.expression))
        {
            push_unique(&mut blocks, successor);
        }
        blocks
    }

    fn guard_invalidated_before(
        &self,
        guard: &str,
        edge_successor: &str,
        block: &str,
        statement_index: Option<usize>,
        dominators: &BTreeMap<String, BTreeSet<String>>,
    ) -> bool {
        let guard_variables = guard_variables(guard);
        if guard_variables.is_empty() {
            return false;
        }

        self.assignments.iter().any(|assignment| {
            let Some(assignment_block) = assignment.block.as_deref() else {
                return false;
            };
            let Some(source_name) = self.local_name_for_place(&assignment.place) else {
                return false;
            };
            if !guard_variables
                .iter()
                .any(|variable| variable == source_name)
            {
                return false;
            }
            if !dominates(dominators, edge_successor, assignment_block)
                || !dominates(dominators, assignment_block, block)
            {
                return false;
            }
            if assignment_block == block {
                return statement_index
                    .is_some_and(|statement_index| assignment.statement_index < statement_index);
            }

            true
        })
    }

    fn assignment_for_place(&self, place: &str) -> Option<&MirAssignment> {
        let mut matches = self
            .assignments
            .iter()
            .filter(|assignment| assignment.place == place);
        let assignment = matches.next()?;
        matches.next().is_none().then_some(assignment)
    }

    fn is_arg_place(&self, place: &str) -> bool {
        self.args.iter().any(|arg| arg.place == place)
    }

    #[cfg(test)]
    fn semantic_arithmetic_operations(&self) -> Vec<SemanticArithmeticOperation> {
        self.semantic_arithmetic_operations_with_models(&[])
    }

    fn semantic_arithmetic_operations_with_models(
        &self,
        model_fields: &[ModelFieldMap],
    ) -> Vec<SemanticArithmeticOperation> {
        self.assignments
            .iter()
            .filter_map(|assignment| {
                let (kind, args) = mir_checked_arithmetic_operation(&assignment.expression)?;
                match (kind, args.as_slice()) {
                    (
                        SemanticArithmeticKind::Add
                        | SemanticArithmeticKind::Sub
                        | SemanticArithmeticKind::Div
                        | SemanticArithmeticKind::Mul
                        | SemanticArithmeticKind::Rem,
                        [left, right],
                    ) => {
                        let left =
                            self.normalized_mir_expression_with_models(left, model_fields)?;
                        let right =
                            self.normalized_mir_expression_with_models(right, model_fields)?;
                        let ty = self.semantic_arithmetic_type(&args, model_fields);
                        let operator = semantic_arithmetic_operator(kind);
                        Some(SemanticArithmeticOperation {
                            kind,
                            ty,
                            target: self.semantic_arithmetic_target(assignment, model_fields),
                            expression: semantic_binary_expression(&left, operator, &right),
                            left,
                            right: Some(right),
                            guards: self.guards_for_assignment(assignment, model_fields),
                        })
                    }
                    (SemanticArithmeticKind::Neg, [value]) => {
                        let value =
                            self.normalized_mir_expression_with_models(value, model_fields)?;
                        let ty = self.semantic_arithmetic_type(&args, model_fields);
                        Some(SemanticArithmeticOperation {
                            kind,
                            ty,
                            target: self.semantic_arithmetic_target(assignment, model_fields),
                            expression: format!("-{value}"),
                            left: value,
                            right: None,
                            guards: self.guards_for_assignment(assignment, model_fields),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn semantic_arithmetic_target(
        &self,
        assignment: &MirAssignment,
        _model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        self.local_name_for_place(&assignment.place)
            .map(ToString::to_string)
            .or_else(|| {
                self.assignments.iter().find_map(|target_assignment| {
                    let expr = strip_mir_move_or_copy(&target_assignment.expression);
                    let (place, field, _ty) = mir_projection(expr)?;
                    if place != assignment.place || field != "0" {
                        return None;
                    }
                    self.local_name_for_place(&target_assignment.place)
                        .map(ToString::to_string)
                })
            })
    }

    #[cfg(test)]
    fn semantic_slice_indexes(&self) -> Vec<SemanticSliceIndex> {
        self.semantic_slice_indexes_with_models(&[])
    }

    fn semantic_slice_indexes_with_models(
        &self,
        model_fields: &[ModelFieldMap],
    ) -> Vec<SemanticSliceIndex> {
        self.assignments
            .iter()
            .flat_map(|assignment| {
                let expr = strip_mir_move_or_copy(&assignment.expression);
                let Some((base_place, index_expr)) = mir_slice_index(expr) else {
                    return Vec::new();
                };
                let Some(base_type) = self.type_for_place(base_place).map(ToString::to_string)
                else {
                    return Vec::new();
                };
                let Some(element_type) = slice_element_type(&base_type).map(ToString::to_string)
                else {
                    return Vec::new();
                };
                let Some(index_type) = self.mir_expression_type(index_expr, model_fields) else {
                    return Vec::new();
                };
                let Some(canonical_base) =
                    self.normalized_mir_expression_with_models(base_place, model_fields)
                else {
                    return Vec::new();
                };
                let Some(index) =
                    self.normalized_mir_expression_with_models(index_expr, model_fields)
                else {
                    return Vec::new();
                };
                let guards = self.guards_for_assignment(assignment, model_fields);
                self.local_aliases_for_place_with_models(base_place, model_fields)
                    .into_iter()
                    .map(|base| SemanticSliceIndex {
                        expression: format!("{base}[{index}]"),
                        guards: semantic_guards_for_base_alias(&guards, &canonical_base, &base),
                        base,
                        base_type: base_type.clone(),
                        index: index.clone(),
                        index_type: index_type.clone(),
                        element_type: element_type.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .fold(Vec::new(), |mut indexes, index| {
                if !indexes
                    .iter()
                    .any(|existing: &SemanticSliceIndex| existing.expression == index.expression)
                {
                    indexes.push(index);
                }
                indexes
            })
    }

    fn semantic_len_calls_with_models(&self, model_fields: &[ModelFieldMap]) -> Vec<SemanticCall> {
        let mut calls = Vec::new();

        for assignment in &self.assignments {
            let Some(base) = mir_ptr_metadata(&assignment.expression) else {
                continue;
            };
            let base_place = strip_mir_move_or_copy(base);
            let Some(base_type) = self.type_for_place(base_place) else {
                continue;
            };
            if slice_element_type(base_type).is_none() {
                continue;
            }

            for receiver in self.local_aliases_for_place_with_models(base_place, model_fields) {
                if calls.iter().any(|call: &SemanticCall| {
                    call.callee == "<slice>.len"
                        && call.args.len() == 1
                        && call.args.first() == Some(&receiver)
                }) {
                    continue;
                }
                calls.push(SemanticCall {
                    callee: "<slice>.len".to_string(),
                    trust_callee: None,
                    args: vec![receiver],
                    guards: Vec::new(),
                })
            }
        }

        calls
    }

    #[cfg(test)]
    fn semantic_calls(&self) -> Vec<SemanticCall> {
        self.semantic_calls_with_models(&[], &[])
    }

    fn semantic_calls_with_models(
        &self,
        model_fields: &[ModelFieldMap],
        trust_callees: &[SemanticTrustCallee],
    ) -> Vec<SemanticCall> {
        let runtime_assertion_input_places = self.runtime_assertion_input_places();
        let mut calls = self
            .assignments
            .iter()
            .filter_map(|assignment| {
                let (callee, args) = mir_call(&assignment.expression)?;
                if mir_runtime_assertion_callee(callee)
                    || runtime_assertion_input_places.contains(&assignment.place)
                {
                    return None;
                }
                let trust_callee = trust_callee_for_call(callee, trust_callees);
                let normalized_args = args
                    .iter()
                    .map(|arg| self.normalized_mir_expression_with_models(arg, model_fields))
                    .collect::<Option<Vec<_>>>();
                let args = match normalized_args {
                    Some(args) => args,
                    None if trust_callee.is_none() && mir_checked_add_type(callee).is_none() => {
                        args.iter()
                            .map(|arg| self.lossy_mir_call_arg(arg, model_fields))
                            .collect()
                    }
                    None => return None,
                };

                Some(SemanticCall {
                    callee: callee.to_string(),
                    trust_callee,
                    args,
                    guards: self.guards_for_assignment(assignment, model_fields),
                })
            })
            .collect::<Vec<_>>();
        calls.extend(self.semantic_len_calls_with_models(model_fields));
        calls.extend(self.semantic_unsupported_operator_calls_with_models(model_fields));
        calls
    }

    fn runtime_assertion_input_places(&self) -> BTreeSet<String> {
        let mut places = BTreeSet::new();
        let mut stack = self
            .assignments
            .iter()
            .filter_map(|assignment| {
                let (callee, args) = mir_call(&assignment.expression)?;
                mir_runtime_assertion_callee(callee).then_some(args)
            })
            .flatten()
            .map(|arg| strip_mir_move_or_copy(arg.trim()).to_string())
            .collect::<Vec<_>>();

        while let Some(place) = stack.pop() {
            if !places.insert(place.clone()) {
                continue;
            }
            let Some(assignment) = self.assignment_for_place(&place) else {
                continue;
            };
            stack.extend(mir_local_places(&assignment.expression));
        }

        places
    }

    fn lossy_mir_call_arg(&self, arg: &str, model_fields: &[ModelFieldMap]) -> String {
        self.lossy_mir_expression(arg, model_fields, 0)
    }

    fn lossy_mir_expression(
        &self,
        expr: &str,
        model_fields: &[ModelFieldMap],
        depth: usize,
    ) -> String {
        let expr = strip_mir_move_or_copy(expr.trim());
        if depth > 8 {
            return expr.to_string();
        }
        if let Some(normalized) = self.normalized_mir_expression_with_models(expr, model_fields) {
            return normalized;
        }
        if let Some(referent) = mir_borrow(expr) {
            return self.lossy_mir_expression(referent, model_fields, depth + 1);
        }
        if let Some(assignment) = self.assignment_for_place(expr) {
            return self.lossy_mir_expression(&assignment.expression, model_fields, depth + 1);
        }
        if let Some(elements) = mir_tuple_elements(expr) {
            let elements = elements
                .iter()
                .map(|element| self.lossy_mir_expression(element, model_fields, depth + 1))
                .collect::<Vec<_>>();
            if elements.len() == 1 {
                return format!("({},)", elements[0]);
            }
            return format!("({})", elements.join(", "));
        }
        self.local_name_for_place(expr).unwrap_or(expr).to_string()
    }

    fn semantic_unsupported_operator_calls_with_models(
        &self,
        model_fields: &[ModelFieldMap],
    ) -> Vec<SemanticCall> {
        self.assignments
            .iter()
            .flat_map(|assignment| {
                let guards = self.guards_for_assignment(assignment, model_fields);
                let mut calls = Vec::new();

                if let Some((operator, args)) =
                    mir_unsupported_binary_operation(&assignment.expression)
                {
                    if let [left, right] = args.as_slice() {
                        if let (Some(left), Some(right)) = (
                            self.normalized_mir_expression_with_models(left, model_fields),
                            self.normalized_mir_expression_with_models(right, model_fields),
                        ) {
                            calls.push(SemanticCall {
                                callee: format!("{left} {operator} {right}"),
                                trust_callee: None,
                                args: vec![left, right],
                                guards: guards.clone(),
                            });
                        }
                    }
                }

                if let Some((operator, value)) =
                    self.mir_unsupported_unary_operation(&assignment.expression, model_fields)
                {
                    if let Some(value) =
                        self.normalized_mir_expression_with_models(value, model_fields)
                    {
                        calls.push(SemanticCall {
                            callee: format!("{operator}{value}"),
                            trust_callee: None,
                            args: vec![value],
                            guards: guards.clone(),
                        });
                    }
                }

                if let Some((value, ty, cast_kind)) = mir_unsupported_cast(&assignment.expression) {
                    if let Some(value) =
                        self.normalized_mir_expression_with_models(value, model_fields)
                    {
                        calls.push(SemanticCall {
                            callee: format!("{value} as {ty} ({cast_kind})"),
                            trust_callee: None,
                            args: vec![value],
                            guards,
                        });
                    }
                }

                calls
            })
            .collect()
    }

    fn mir_unsupported_unary_operation<'a>(
        &self,
        expr: &'a str,
        model_fields: &[ModelFieldMap],
    ) -> Option<(&'static str, &'a str)> {
        let (operator, value) = mir_unsupported_unary_operation(expr)?;
        let ty = self.mir_expression_type(value, model_fields);
        if ty.as_deref() == Some("bool") {
            return None;
        }

        Some((operator, value))
    }

    fn semantic_field_accesses(&self, model_fields: &[ModelFieldMap]) -> Vec<SemanticFieldAccess> {
        self.assignments
            .iter()
            .flat_map(|assignment| {
                self.semantic_field_accesses_in_expression(&assignment.expression, model_fields)
            })
            .fold(Vec::new(), |mut field_accesses, field_access| {
                if !field_accesses.iter().any(|existing: &SemanticFieldAccess| {
                    existing.expression == field_access.expression
                        && existing.owner_type == field_access.owner_type
                }) {
                    field_accesses.push(field_access);
                }
                field_accesses
            })
    }

    fn semantic_field_accesses_in_expression(
        &self,
        expr: &str,
        model_fields: &[ModelFieldMap],
    ) -> Vec<SemanticFieldAccess> {
        self.semantic_field_accesses_in_expression_with_depth(expr, 0, model_fields)
    }

    fn semantic_field_accesses_in_expression_with_depth(
        &self,
        expr: &str,
        depth: usize,
        model_fields: &[ModelFieldMap],
    ) -> Vec<SemanticFieldAccess> {
        if depth > 8 {
            return Vec::new();
        }
        let expr = strip_mir_move_or_copy(expr.trim());
        let mut field_accesses = Vec::new();
        if let Some((place, field, ty)) = mir_projection(expr) {
            if let Some(field_access) = self.semantic_field_access(place, field, ty, model_fields) {
                field_accesses.push(field_access);
            }
        }
        if let Some((_kind, args)) = mir_checked_arithmetic_operation(expr) {
            for arg in args {
                field_accesses.extend(self.semantic_field_accesses_in_expression_with_depth(
                    &arg,
                    depth + 1,
                    model_fields,
                ));
            }
        }
        if let Some((_ty, fields)) = mir_aggregate_fields(expr) {
            for (_field, value) in fields {
                field_accesses.extend(self.semantic_field_accesses_in_expression_with_depth(
                    &value,
                    depth + 1,
                    model_fields,
                ));
            }
        }
        if let Some((_callee, args)) = mir_call(expr) {
            for arg in args {
                field_accesses.extend(self.semantic_field_accesses_in_expression_with_depth(
                    &arg,
                    depth + 1,
                    model_fields,
                ));
            }
        }
        if let Some((base, index)) = mir_slice_index(expr) {
            field_accesses.extend(self.semantic_field_accesses_in_expression_with_depth(
                base,
                depth + 1,
                model_fields,
            ));
            field_accesses.extend(self.semantic_field_accesses_in_expression_with_depth(
                index,
                depth + 1,
                model_fields,
            ));
        }

        field_accesses
            .into_iter()
            .fold(Vec::new(), |mut deduped, field_access| {
                if !deduped.iter().any(|existing: &SemanticFieldAccess| {
                    existing.expression == field_access.expression
                        && existing.owner_type == field_access.owner_type
                }) {
                    deduped.push(field_access);
                }
                deduped
            })
    }

    fn semantic_field_access(
        &self,
        place: &str,
        field: &str,
        field_ty: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<SemanticFieldAccess> {
        let field_idx = field.parse::<usize>().ok()?;
        let owner_type = self.type_for_place(place).map(type_name_tail)?;
        let model = model_fields.iter().find(|model| model.ty == owner_type);
        if model.is_none() && !field_owner_requires_trust_model(&owner_type) {
            return None;
        }

        let base = self
            .normalized_mir_expression_with_models(place, model_fields)
            .or_else(|| self.local_name_for_place(place).map(ToString::to_string))
            .unwrap_or_else(|| place.to_string());

        let Some(model) = model else {
            return semantic_unmodeled_field_access(&base, field, &owner_type, field_ty);
        };
        let model_field = model.fields.get(field_idx)?;
        Some(SemanticFieldAccess {
            expression: format!("{}.{}", base, model_field.name),
            base,
            field: model_field.name.clone(),
            owner_type,
            field_type: model_field.ty.clone(),
        })
    }

    fn semantic_matches(&self, model_fields: &[ModelFieldMap]) -> Vec<SemanticMatch> {
        self.assignments
            .iter()
            .filter_map(|assignment| {
                let scrutinee_place = mir_discriminant(&assignment.expression)?;
                let scrutinee = self
                    .normalized_mir_expression_with_models(scrutinee_place, model_fields)
                    .or_else(|| {
                        self.local_name_for_place(scrutinee_place)
                            .map(ToString::to_string)
                    })
                    .unwrap_or_else(|| scrutinee_place.to_string());
                let scrutinee_type = self
                    .type_for_place(scrutinee_place)
                    .map(ToString::to_string)
                    .unwrap_or_default();
                let terminator = self.terminators.iter().find(|terminator| {
                    mir_switch(&terminator.expression).is_some_and(|(condition, _targets)| {
                        strip_mir_move_or_copy(condition) == assignment.place
                    })
                })?;
                let (_condition, targets) = mir_switch(&terminator.expression)?;
                let arms = targets
                    .into_iter()
                    .filter(|target| target.value != "otherwise")
                    .filter_map(|target| {
                        let variant =
                            semantic_variant_for_discriminant(&scrutinee_type, &target.value)?;
                        let payload =
                            self.semantic_match_payload(&target.block, scrutinee_place, &variant);
                        let return_fact = self.semantic_return_fact_for_target_block(
                            &target.block,
                            terminator.block.as_deref(),
                            model_fields,
                        );
                        Some(SemanticMatchArm {
                            variant,
                            discriminant: target.value,
                            payload,
                            assumptions: return_fact
                                .as_ref()
                                .map(|fact| fact.assumptions.clone())
                                .unwrap_or_default(),
                            return_expression: return_fact.map(|fact| fact.expression),
                        })
                    })
                    .collect::<Vec<_>>();
                if arms.is_empty() {
                    return None;
                }

                Some(SemanticMatch {
                    scrutinee,
                    scrutinee_type,
                    arms,
                })
            })
            .collect()
    }

    fn semantic_branches(&self, model_fields: &[ModelFieldMap]) -> Vec<SemanticBranch> {
        self.terminators
            .iter()
            .filter_map(|terminator| {
                let (switch_condition, targets) = mir_switch(&terminator.expression)?;
                let predicate =
                    self.normalized_mir_predicate_with_models(switch_condition, model_fields);
                let condition_expression =
                    self.normalized_mir_expression_with_models(switch_condition, model_fields);
                let condition = predicate.clone().or(condition_expression)?;
                let explicit_values = explicit_mir_switch_values(&targets);
                let arms = targets
                    .iter()
                    .filter_map(|target| {
                        let guard = self.guard_for_switch_target(
                            switch_condition,
                            &target.value,
                            &explicit_values,
                            model_fields,
                        )?;
                        let return_fact = self.semantic_return_fact_for_target_block(
                            &target.block,
                            terminator.block.as_deref(),
                            model_fields,
                        );
                        Some(SemanticBranchArm {
                            guard,
                            assumptions: return_fact
                                .as_ref()
                                .map(|fact| fact.assumptions.clone())
                                .unwrap_or_default(),
                            return_expression: return_fact.map(|fact| fact.expression),
                        })
                    })
                    .collect::<Vec<_>>();
                if arms.len() < 2 {
                    return None;
                }

                Some(SemanticBranch { condition, arms })
            })
            .collect()
    }

    fn guard_for_switch_target(
        &self,
        condition: &str,
        target_value: &str,
        explicit_values: &[String],
        model_fields: &[ModelFieldMap],
    ) -> Option<String> {
        if let Some(predicate) = self.normalized_mir_predicate_with_models(condition, model_fields)
        {
            return mir_boolean_branch_guard(&predicate, target_value);
        }

        let expression = self.normalized_mir_expression_with_models(condition, model_fields)?;
        match target_value {
            "otherwise" if explicit_values.len() == 1 => {
                Some(format!("{expression} != {}", explicit_values[0]))
            }
            "otherwise" => None,
            value => Some(format!("{expression} == {value}")),
        }
    }

    fn semantic_match_payload(
        &self,
        block: &str,
        scrutinee_place: &str,
        variant: &str,
    ) -> Option<SemanticMatchPayload> {
        self.assignments.iter().find_map(|assignment| {
            if assignment.block.as_deref() != Some(block) {
                return None;
            }
            let expr = strip_mir_move_or_copy(&assignment.expression);
            let projection = mir_variant_projection(expr)?;
            if projection.place != scrutinee_place || projection.variant != variant {
                return None;
            }
            Some(SemanticMatchPayload {
                binding: self
                    .local_name_for_place(&assignment.place)
                    .unwrap_or(&assignment.place)
                    .to_string(),
                field_index: projection.field_index,
                ty: projection.ty,
            })
        })
    }

    fn semantic_return_fact_for_target_block(
        &self,
        block: &str,
        predecessor_block: Option<&str>,
        model_fields: &[ModelFieldMap],
    ) -> Option<MirSemanticReturnFact> {
        self.semantic_join_return_fact_for_block(block, model_fields)
            .or_else(|| {
                self.semantic_predecessor_join_return_fact_for_block(
                    block,
                    predecessor_block?,
                    model_fields,
                )
            })
            .or_else(|| self.semantic_direct_return_fact_for_block(block, model_fields))
    }

    fn semantic_direct_return_fact_for_block(
        &self,
        block: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<MirSemanticReturnFact> {
        let assignment = self.assignments.iter().find(|assignment| {
            assignment.block.as_deref() == Some(block) && assignment.place == "_0"
        })?;
        Some(MirSemanticReturnFact {
            expression: self
                .normalized_mir_expression_with_models(&assignment.expression, model_fields)?,
            assumptions: self.guards_for_location(
                block,
                Some(assignment.statement_index),
                model_fields,
            ),
        })
    }

    fn semantic_join_return_fact_for_block(
        &self,
        block: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<MirSemanticReturnFact> {
        let join_block = self.goto_target_for_block(block)?;
        let return_places = self.return_source_places_for_block(join_block);
        let assignment = self.assignments.iter().rev().find(|assignment| {
            assignment.block.as_deref() == Some(block)
                && return_places.iter().any(|place| place == &assignment.place)
        })?;

        Some(MirSemanticReturnFact {
            expression: self
                .normalized_mir_expression_with_models(&assignment.expression, model_fields)?,
            assumptions: self.guards_for_location(
                block,
                Some(assignment.statement_index),
                model_fields,
            ),
        })
    }

    fn semantic_predecessor_join_return_fact_for_block(
        &self,
        block: &str,
        predecessor_block: &str,
        model_fields: &[ModelFieldMap],
    ) -> Option<MirSemanticReturnFact> {
        let return_places = self.join_return_source_places_for_block(block);
        let terminator = self.terminator_for_block(predecessor_block)?;
        let assignment = self.assignments.iter().rev().find(|assignment| {
            assignment.block.as_deref() == Some(predecessor_block)
                && return_places.iter().any(|place| place == &assignment.place)
                && assignment.statement_index < terminator.statement_index
        })?;

        Some(MirSemanticReturnFact {
            expression: self
                .normalized_mir_expression_with_models(&assignment.expression, model_fields)?,
            assumptions: self.guards_for_location(
                block,
                self.return_statement_index_for_block(block).or_else(|| {
                    self.terminator_for_block(block)
                        .map(|terminator| terminator.statement_index)
                }),
                model_fields,
            ),
        })
    }

    fn join_return_source_places_for_block(&self, block: &str) -> Vec<String> {
        let places = self.return_source_places_for_block(block);
        if !places.is_empty() {
            return places;
        }
        let Some(join_block) = self.goto_target_for_block(block) else {
            return Vec::new();
        };
        self.return_source_places_for_block(join_block)
    }

    fn return_source_places_for_block(&self, block: &str) -> Vec<String> {
        let Some(first) = self.return_source_place_for_block(block) else {
            return Vec::new();
        };
        let mut places = vec![first.to_string()];
        let mut current = first.to_string();
        for _ in 0..4 {
            let Some(next) = self.local_copy_source_for_block(block, &current) else {
                break;
            };
            if places.iter().any(|place| place == next) {
                break;
            }
            places.push(next.to_string());
            current = next.to_string();
        }
        places
    }

    fn local_copy_source_for_block<'a>(&'a self, block: &str, place: &str) -> Option<&'a str> {
        let return_statement_index = self.return_statement_index_for_block(block)?;
        self.assignments
            .iter()
            .rev()
            .find(|assignment| {
                assignment.block.as_deref() == Some(block)
                    && assignment.place == place
                    && assignment.statement_index < return_statement_index
            })
            .map(|assignment| strip_mir_move_or_copy(&assignment.expression))
            .filter(|source| source.starts_with('_'))
    }

    fn goto_target_for_block(&self, block: &str) -> Option<&str> {
        self.terminators
            .iter()
            .find(|terminator| terminator.block.as_deref() == Some(block))
            .and_then(|terminator| terminator.expression.strip_prefix("goto -> "))
            .map(str::trim)
    }

    fn terminator_for_block(&self, block: &str) -> Option<&MirTerminator> {
        self.terminators
            .iter()
            .find(|terminator| terminator.block.as_deref() == Some(block))
    }

    fn return_statement_index_for_block(&self, block: &str) -> Option<usize> {
        self.assignments
            .iter()
            .find(|assignment| {
                assignment.block.as_deref() == Some(block) && assignment.place == "_0"
            })
            .map(|assignment| assignment.statement_index)
    }

    fn return_source_place_for_block(&self, block: &str) -> Option<&str> {
        self.assignments
            .iter()
            .find(|assignment| {
                assignment.block.as_deref() == Some(block) && assignment.place == "_0"
            })
            .map(|assignment| strip_mir_move_or_copy(&assignment.expression))
            .filter(|place| place.starts_with('_'))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSignature {
    path: String,
    args: Vec<MirArg>,
    return_type: String,
}

fn mir_signature_matches_qualified_path(line: &str, expected_path: &str) -> bool {
    let Some(path) = mir_signature_name(line) else {
        return false;
    };
    path == expected_path
        || path.ends_with(&format!("::{expected_path}"))
        || expected_path.ends_with(&format!("::{path}"))
}

fn mir_signature_matches_leaf(line: &str, name: &str) -> bool {
    mir_signature_name(line).is_some_and(|path| function_leaf_name(path) == name)
}

fn mir_signature_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("fn ")?;
    let (name, _rest) = rest.split_once('(')?;
    Some(name.trim())
}

fn parse_mir_signature(line: &str) -> Option<MirSignature> {
    let rest = line.strip_prefix("fn ")?;
    let (args, rest) = rest.split_once(") -> ")?;
    let (path, args) = args.split_once('(')?;
    let return_type = rest
        .trim()
        .strip_suffix('{')
        .unwrap_or(rest)
        .trim()
        .to_string();
    Some(MirSignature {
        path: path.trim().to_string(),
        args: parse_mir_args(args),
        return_type,
    })
}

fn parse_mir_args(input: &str) -> Vec<MirArg> {
    parse_comma_separated(input)
        .into_iter()
        .filter_map(|arg| {
            let (place, ty) = arg.split_once(':')?;
            Some(MirArg {
                place: place.trim().to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect()
}

fn extract_mir_locals(lines: &[&str]) -> Vec<MirLocal> {
    lines
        .iter()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("let mut ")
                .or_else(|| line.strip_prefix("let "))?;
            let (place, ty) = rest.split_once(':')?;
            Some(MirLocal {
                place: place.trim().to_string(),
                ty: ty.trim_end_matches(';').trim().to_string(),
            })
        })
        .collect()
}

fn parse_comma_separated(input: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut angle_depth = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            ',' if paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && angle_depth == 0 =>
            {
                let item = input[start..idx].trim();
                if !item.is_empty() {
                    items.push(item.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let item = input[start..].trim();
    if !item.is_empty() {
        items.push(item.to_string());
    }

    items
}

fn extract_mir_debug_locals(lines: &[&str]) -> Vec<MirDebugLocal> {
    lines
        .iter()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("debug ")
                .and_then(|line| line.split_once(" => "))
                .map(|(name, place)| MirDebugLocal {
                    name: name.trim().to_string(),
                    place: place.trim_end_matches(';').trim().to_string(),
                })
        })
        .collect()
}

fn extract_mir_assignments(lines: &[&str]) -> Vec<MirAssignment> {
    let mut assignments = Vec::new();
    let mut current_block = None;

    for (statement_index, line) in lines.iter().enumerate() {
        let line = line.trim();
        if let Some(block) = mir_block_header(line) {
            current_block = Some(block.to_string());
            continue;
        }
        if !line.starts_with('_') {
            continue;
        }
        if let Some((place, expression)) = line.split_once(" = ") {
            assignments.push(MirAssignment {
                block: current_block.clone(),
                place: place.trim().to_string(),
                expression: expression.trim_end_matches(';').trim().to_string(),
                statement_index,
            });
        }
    }

    assignments
}

fn extract_mir_terminators(lines: &[&str]) -> Vec<MirTerminator> {
    let mut terminators = Vec::new();
    let mut current_block = None;

    for (statement_index, line) in lines.iter().enumerate() {
        let line = line.trim();
        if let Some(block) = mir_block_header(line) {
            current_block = Some(block.to_string());
            continue;
        }
        if line.starts_with("switchInt(")
            || line.starts_with("goto -> ")
            || line.starts_with("assert(")
            || line.contains(" -> [return: ")
        {
            terminators.push(MirTerminator {
                block: current_block.clone(),
                expression: line.trim_end_matches(';').trim().to_string(),
                statement_index,
            });
        }
    }

    terminators
}

fn mir_block_header(line: &str) -> Option<&str> {
    let (block, rest) = line.split_once(':')?;
    if block.starts_with("bb") && rest.trim() == "{" {
        Some(block)
    } else {
        None
    }
}

fn extract_mir_return_expr(lines: &[&str]) -> Option<String> {
    let return_exprs = lines
        .iter()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("_0 = ")
                .and_then(|line| line.strip_suffix(';'))
                .map(str::trim)
                .filter(|expr| !expr.is_empty())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    match return_exprs.as_slice() {
        [expr] => Some(expr.clone()),
        _ => None,
    }
}

fn strip_mir_move_or_copy(expr: &str) -> &str {
    expr.strip_prefix("copy ")
        .or_else(|| expr.strip_prefix("move "))
        .unwrap_or(expr)
        .trim()
}

fn mir_borrow(expr: &str) -> Option<&str> {
    let referent = expr.trim().strip_prefix('&')?.trim();
    let referent = referent
        .strip_prefix("mut ")
        .or_else(|| referent.strip_prefix("raw const "))
        .or_else(|| referent.strip_prefix("raw mut "))
        .unwrap_or(referent)
        .trim();
    Some(strip_mir_move_or_copy(referent))
}

fn mir_const_value(expr: &str) -> Option<String> {
    let value = expr.strip_prefix("const ")?.trim();
    if let Some(bound) = mir_integer_bound(value) {
        return Some(bound);
    }
    let value = value
        .split_once('_')
        .map(|(value, _ty)| value)
        .unwrap_or(value);
    let digits = value.strip_prefix('-').unwrap_or(value);
    if !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()) {
        Some(value.to_string())
    } else {
        None
    }
}

fn mir_const_type(expr: &str) -> Option<String> {
    let value = expr.strip_prefix("const ")?.trim();
    if let Some(bound) = mir_integer_bound(value) {
        return bound.split_once("::").map(|(ty, _)| ty.to_string());
    }

    if value == "true" || value == "false" {
        return Some("bool".to_string());
    }

    for ty in ["i32", "i64", "u32", "u64", "usize", "f32", "f64", "bool"] {
        if value.ends_with(ty) {
            return Some(ty.to_string());
        }
    }

    let (_value, ty) = value.rsplit_once('_')?;
    let ty = ty.trim();
    if supported_mir_integer_type(ty) || matches!(ty, "f32" | "f64" | "bool") {
        Some(ty.to_string())
    } else {
        None
    }
}

fn mir_integer_bound(value: &str) -> Option<String> {
    let value = value.trim();
    for bound in ["MAX", "MIN"] {
        let suffix = format!(">::{bound}");
        if let Some(before_bound) = value.strip_suffix(&suffix) {
            if let Some(ty) = before_bound.rsplit_once("<impl ").map(|(_prefix, ty)| ty) {
                return Some(format!("{ty}::{bound}"));
            }
        }
        for ty in ["i32", "i64", "u32", "u64", "usize"] {
            if value == format!("{ty}::{bound}") || value == format!("core::{ty}::{bound}") {
                return Some(format!("{ty}::{bound}"));
            }
        }
    }

    None
}

fn supported_mir_integer_type(ty: &str) -> bool {
    matches!(ty, "i32" | "i64" | "u32" | "u64" | "usize")
}

fn mir_projection(expr: &str) -> Option<(&str, &str, &str)> {
    let expr = expr.strip_prefix('(')?.strip_suffix(')')?;
    let (projection, ty) = expr.split_once(':')?;
    let (place, field) = projection.trim().split_once('.')?;
    Some((place.trim(), field.trim(), ty.trim()))
}

fn mir_aggregate_fields(expr: &str) -> Option<(&str, Vec<(String, String)>)> {
    let open = expr.find('{')?;
    let close = expr.rfind('}')?;
    if close <= open {
        return None;
    }
    let ty = expr[..open].trim();
    if ty.is_empty() || !ty.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    let fields = parse_comma_separated(&expr[open + 1..close])
        .into_iter()
        .map(|field| {
            let (name, value) = field.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect::<Option<Vec<_>>>()?;
    if fields.is_empty() {
        return None;
    }

    Some((ty, fields))
}

fn mir_tuple_elements(expr: &str) -> Option<Vec<String>> {
    let inner = expr.trim().strip_prefix('(')?.strip_suffix(')')?;
    if !inner.trim_end().ends_with(',') {
        return None;
    }
    Some(parse_comma_separated(inner))
}

fn mir_variant_projection(expr: &str) -> Option<MirVariantProjection> {
    let expr = expr.strip_prefix('(')?.strip_suffix(')')?;
    let (projection, ty) = expr.split_once(':')?;
    let (variant_place, field) = projection.trim().split_once(").")?;
    let variant_place = variant_place.strip_prefix('(')?.trim();
    let (place, variant) = variant_place.split_once(" as ")?;
    Some(MirVariantProjection {
        place: place.trim().to_string(),
        variant: variant.trim().to_string(),
        field_index: field.trim().parse().ok()?,
        ty: ty.trim().to_string(),
    })
}

fn mir_discriminant(expr: &str) -> Option<&str> {
    expr.strip_prefix("discriminant(")
        .and_then(|expr| expr.strip_suffix(')'))
        .map(str::trim)
}

fn mir_slice_index(expr: &str) -> Option<(&str, &str)> {
    let (base, index) = expr.split_once('[')?;
    let index = index.strip_suffix(']')?;
    Some((mir_slice_base_place(base.trim())?, index.trim()))
}

fn mir_slice_base_place(base: &str) -> Option<&str> {
    base.strip_prefix("(*")
        .and_then(|base| base.strip_suffix(')'))
        .or_else(|| base.strip_prefix('*'))
        .map(str::trim)
}

fn slice_element_type(ty: &str) -> Option<&str> {
    ty.trim()
        .strip_prefix("&[")?
        .strip_suffix(']')
        .map(str::trim)
}

fn mir_call(expr: &str) -> Option<(&str, Vec<String>)> {
    let (call, _target) = expr.split_once(" -> ")?;
    let (callee, args) = split_mir_call_head(call.trim())?;
    Some((callee.trim(), parse_mir_call_args(args)))
}

fn mir_runtime_assertion_callee(callee: &str) -> bool {
    matches!(
        function_leaf_name(callee),
        "assert_precondition" | "assert_postcondition"
    )
}

fn mir_local_places(expr: &str) -> Vec<String> {
    let mut places = Vec::new();
    let chars = expr.char_indices().collect::<Vec<_>>();
    let mut idx = 0usize;

    while let Some((byte_idx, ch)) = chars.get(idx).copied() {
        if ch != '_' {
            idx += 1;
            continue;
        }

        let mut end_idx = idx + 1;
        while let Some((_byte_idx, ch)) = chars.get(end_idx).copied() {
            if !ch.is_ascii_digit() {
                break;
            }
            end_idx += 1;
        }
        if end_idx > idx + 1 {
            let end_byte = chars
                .get(end_idx)
                .map(|(next_byte_idx, _)| *next_byte_idx)
                .unwrap_or(expr.len());
            let place = expr[byte_idx..end_byte].to_string();
            if !places.contains(&place) {
                places.push(place);
            }
        }
        idx = end_idx;
    }

    places
}

fn split_mir_call_head(call: &str) -> Option<(&str, &str)> {
    let call = call.trim();
    if !call.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (idx, ch) in call.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((call[..idx].trim(), call[idx + 1..call.len() - 1].trim()));
                }
            }
            _ => {}
        }
    }
    None
}

fn mir_checked_add_type(callee: &str) -> Option<&str> {
    let ty = callee
        .strip_prefix("core::num::<impl ")?
        .strip_suffix(">::checked_add")?;
    supported_mir_integer_type(ty).then_some(ty)
}

fn mir_ptr_metadata(expr: &str) -> Option<&str> {
    expr.trim()
        .strip_prefix("PtrMetadata(")?
        .strip_suffix(')')
        .map(str::trim)
}

fn parse_mir_call_args(input: &str) -> Vec<String> {
    parse_comma_separated(input)
}

fn mir_checked_arithmetic_operation(expr: &str) -> Option<(SemanticArithmeticKind, Vec<String>)> {
    let (op, args) = expr.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let kind = match op.trim() {
        "Add" | "AddWithOverflow" => SemanticArithmeticKind::Add,
        "Sub" | "SubWithOverflow" => SemanticArithmeticKind::Sub,
        "Mul" | "MulWithOverflow" => SemanticArithmeticKind::Mul,
        "Neg" | "NegWithOverflow" => SemanticArithmeticKind::Neg,
        "Div" => SemanticArithmeticKind::Div,
        "Rem" => SemanticArithmeticKind::Rem,
        _ => return None,
    };
    Some((kind, parse_mir_call_args(args)))
}

fn mir_unsupported_binary_operation(expr: &str) -> Option<(&'static str, Vec<String>)> {
    let (op, args) = expr.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let operator = match op.trim() {
        "Shl" => "<<",
        "Shr" => ">>",
        "BitAnd" => "&",
        "BitOr" => "|",
        "BitXor" => "^",
        _ => return None,
    };
    Some((operator, parse_mir_call_args(args)))
}

fn mir_unsupported_unary_operation(expr: &str) -> Option<(&'static str, &str)> {
    let value = expr.strip_prefix("Not(")?.strip_suffix(')')?.trim();
    Some(("!", value))
}

fn mir_unsupported_cast(expr: &str) -> Option<(&str, &str, &str)> {
    let (value, cast) = expr.rsplit_once(" as ")?;
    let (ty, cast_kind) = cast.rsplit_once(" (")?;
    Some((value.trim(), ty.trim(), cast_kind.strip_suffix(')')?.trim()))
}

fn mir_comparison_operator(op: &str) -> Option<&'static str> {
    match op {
        "Lt" => Some("<"),
        "Le" => Some("<="),
        "Gt" => Some(">"),
        "Ge" => Some(">="),
        "Eq" => Some("=="),
        "Ne" => Some("!="),
        _ => None,
    }
}

fn semantic_variant_for_discriminant(ty: &str, discriminant: &str) -> Option<String> {
    if type_name_tail(ty).starts_with("Option<") {
        return match discriminant {
            "0" => Some("None".to_string()),
            "1" => Some("Some".to_string()),
            _ => None,
        };
    }
    if type_name_tail(ty).starts_with("Result<") {
        return match discriminant {
            "0" => Some("Ok".to_string()),
            "1" => Some("Err".to_string()),
            _ => None,
        };
    }

    None
}

fn mir_switch(expr: &str) -> Option<(&str, Vec<MirSwitchTarget>)> {
    let rest = expr.strip_prefix("switchInt(")?;
    let (condition, targets) = rest.split_once(") -> [")?;
    let targets = targets.strip_suffix(']')?;
    Some((condition.trim(), parse_mir_switch_targets(targets)))
}

fn mir_boolean_branch_guard(predicate: &str, target_value: &str) -> Option<String> {
    match target_value {
        "otherwise" | "1" | "true" => Some(predicate.to_string()),
        "0" | "false" => negate_predicate(predicate),
        _ => None,
    }
}

fn explicit_mir_switch_values(targets: &[MirSwitchTarget]) -> Vec<String> {
    targets
        .iter()
        .filter(|target| target.value != "otherwise")
        .map(|target| target.value.clone())
        .collect()
}

fn parse_mir_switch_targets(input: &str) -> Vec<MirSwitchTarget> {
    parse_comma_separated(input)
        .into_iter()
        .filter_map(|target| {
            let (value, block) = target.split_once(':')?;
            Some(MirSwitchTarget {
                value: value.trim().to_string(),
                block: block.trim().to_string(),
            })
        })
        .collect()
}

fn mir_successor_targets(expr: &str) -> Vec<String> {
    if let Some((_condition, targets)) = mir_switch(expr) {
        return targets.into_iter().map(|target| target.block).collect();
    }
    if let Some(target) = expr.strip_prefix("goto -> ") {
        return vec![target.trim().to_string()];
    }
    let Some((_head, targets)) = expr.split_once(" -> [") else {
        return Vec::new();
    };
    let Some(targets) = targets.strip_suffix(']') else {
        return Vec::new();
    };
    parse_comma_separated(targets)
        .into_iter()
        .filter_map(|target| {
            let (kind, block) = target.split_once(':')?;
            if matches!(kind.trim(), "success" | "return") {
                Some(block.trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn dominates(
    dominators: &BTreeMap<String, BTreeSet<String>>,
    dominator: &str,
    block: &str,
) -> bool {
    dominators
        .get(block)
        .is_some_and(|dominators| dominators.contains(dominator))
}

fn intersect_sets(sets: &[BTreeSet<String>]) -> Option<BTreeSet<String>> {
    let (first, rest) = sets.split_first()?;
    let mut intersection = first.clone();
    for set in rest {
        intersection.retain(|value| set.contains(value));
    }
    Some(intersection)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn guard_variables(guard: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let mut current = String::new();

    for ch in guard.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }
        push_guard_variable(&mut variables, &mut current);
    }
    push_guard_variable(&mut variables, &mut current);

    variables
}

fn push_guard_variable(variables: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    let variable = std::mem::take(current);
    if variable
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
        || matches!(
            variable.as_str(),
            "true" | "false" | "i32" | "i64" | "u32" | "u64" | "usize" | "MIN" | "MAX"
        )
    {
        return;
    }
    push_unique(variables, variable);
}

fn negate_predicate(predicate: &str) -> Option<String> {
    for (op, negated) in [
        ("<=", ">"),
        (">=", "<"),
        ("!=", "=="),
        ("==", "!="),
        ("<", ">="),
        (">", "<="),
    ] {
        let Some((left, right)) = predicate.split_once(op) else {
            continue;
        };
        return Some(format!("{} {negated} {}", left.trim(), right.trim()));
    }

    None
}

fn semantic_arithmetic_operator(kind: SemanticArithmeticKind) -> &'static str {
    match kind {
        SemanticArithmeticKind::Add => "+",
        SemanticArithmeticKind::Sub => "-",
        SemanticArithmeticKind::Mul => "*",
        SemanticArithmeticKind::Neg => "-",
        SemanticArithmeticKind::Div => "/",
        SemanticArithmeticKind::Rem => "%",
    }
}

fn semantic_binary_expression(left: &str, operator: &str, right: &str) -> String {
    format!(
        "{} {operator} {}",
        semantic_binary_operand(left, operator, false),
        semantic_binary_operand(right, operator, true)
    )
}

fn semantic_self_update_expression(expression: &str, variable: &str) -> bool {
    let expression = expression
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    expression.starts_with(&format!("{variable}+"))
        || expression.starts_with(&format!("{variable}-"))
        || expression.ends_with(&format!("+{variable}"))
}

fn semantic_binary_operand(expr: &str, operator: &str, is_right: bool) -> String {
    if semantic_binary_operand_needs_parentheses(expr, operator, is_right) {
        format!("({expr})")
    } else {
        expr.to_string()
    }
}

fn semantic_binary_operand_needs_parentheses(expr: &str, operator: &str, is_right: bool) -> bool {
    let Some(parent_precedence) = semantic_operator_precedence(operator) else {
        return false;
    };
    let Some(child_precedence) = semantic_expression_lowest_precedence(expr) else {
        return false;
    };

    child_precedence < parent_precedence || (is_right && child_precedence == parent_precedence)
}

fn semantic_expression_has_additive_operator(expr: &str) -> bool {
    expr.contains(" + ") || expr.contains(" - ")
}

fn semantic_expression_lowest_precedence(expr: &str) -> Option<u8> {
    if semantic_expression_has_additive_operator(expr) {
        Some(1)
    } else if expr.contains(" * ") || expr.contains(" / ") || expr.contains(" % ") {
        Some(2)
    } else {
        None
    }
}

fn semantic_operator_precedence(operator: &str) -> Option<u8> {
    match operator {
        "+" | "-" => Some(1),
        "*" | "/" | "%" => Some(2),
        _ => None,
    }
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

fn semantic_arm_assumptions_summary(assumptions: &[String]) -> String {
    if assumptions.is_empty() {
        String::new()
    } else {
        format!("[assume {}]", assumptions.join("&"))
    }
}

fn semantic_arm_assumptions_summary_without_guard(arm: &SemanticBranchArm) -> String {
    let assumptions = arm
        .assumptions
        .iter()
        .filter(|assumption| *assumption != &arm.guard)
        .cloned()
        .collect::<Vec<_>>();
    semantic_arm_assumptions_summary(&assumptions)
}

fn semantic_summary(
    metadata: &[TrustMetadata],
    rustc_args: &[OsString],
    item_matches: &[SemanticItemMatch],
    rustc_version: &str,
) -> String {
    let model_fields = model_field_maps(metadata);
    let trust_callees = semantic_trust_callees(metadata, item_matches);
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
            "item kind={} id={} path={} resolved_path={} span={} hir_match={} mir_match={}\n",
            item.item_kind,
            item.item_id,
            item.rust_function_path,
            item.resolved_rust_function_path
                .as_deref()
                .unwrap_or("unknown"),
            item.source_span,
            item.hir_match,
            item.mir_match
        ));
        if let Some(mir_function) = &item.mir_function {
            let mut contract_bindings = metadata
                .iter()
                .find(|metadata_item| {
                    metadata_item.item_kind == item.item_kind
                        && metadata_item.item_id == item.item_id
                })
                .map(|metadata_item| {
                    semantic_contract_bindings(metadata_item, mir_function, &model_fields)
                })
                .unwrap_or_default();
            contract_bindings.extend(semantic_local_bindings(mir_function, &model_fields));
            let contract_bindings = dedup_contract_bindings(contract_bindings);
            let return_expr = mir_function
                .normalized_return_expression_with_models(&model_fields)
                .or_else(|| mir_function.return_expr.clone())
                .unwrap_or_else(|| "none".to_string());
            let arithmetic_ops = mir_function
                .semantic_arithmetic_operations_with_models(&model_fields)
                .iter()
                .map(|operation| {
                    if operation.guards.is_empty() {
                        operation.expression.clone()
                    } else {
                        format!(
                            "{} guarded_by {}",
                            operation.expression,
                            operation.guards.join("&")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let slice_indexes = mir_function
                .semantic_slice_indexes_with_models(&model_fields)
                .iter()
                .map(|index| {
                    let index_expr = format!(
                        "{} element_type={} index_type={}",
                        index.expression, index.element_type, index.index_type
                    );
                    if index.guards.is_empty() {
                        index_expr
                    } else {
                        format!("{} guarded_by {}", index_expr, index.guards.join("&"))
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let calls = mir_function
                .semantic_calls_with_models(&model_fields, &trust_callees)
                .iter()
                .map(|call| {
                    let mut call_expr = format!("{}({})", call.callee, call.args.join(","));
                    if let Some(trust_callee) = &call.trust_callee {
                        call_expr.push_str(&format!(
                            " trust_callee={} preconditions={}",
                            trust_callee.rust_function_path,
                            trust_callee.preconditions.join("&")
                        ));
                    }
                    if call.guards.is_empty() {
                        call_expr
                    } else {
                        format!("{call_expr} guarded_by {}", call.guards.join("&"))
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let field_accesses = mir_function
                .semantic_field_accesses(&model_fields)
                .iter()
                .map(|field| format!("{}:{}", field.expression, field.field_type))
                .collect::<Vec<_>>()
                .join(",");
            let matches = mir_function
                .semantic_matches(&model_fields)
                .iter()
                .map(|semantic_match| {
                    let arms = semantic_match
                        .arms
                        .iter()
                        .map(|arm| {
                            let payload = arm
                                .payload
                                .as_ref()
                                .map(|payload| {
                                    format!(
                                        "({}:{}#{})",
                                        payload.binding, payload.ty, payload.field_index
                                    )
                                })
                                .unwrap_or_default();
                            let return_expression =
                                arm.return_expression.as_deref().unwrap_or("none");
                            let assumptions = semantic_arm_assumptions_summary(&arm.assumptions);
                            format!(
                                "{}{}{}=>{}",
                                arm.variant, payload, assumptions, return_expression
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    format!("{}:{}", semantic_match.scrutinee, arms)
                })
                .collect::<Vec<_>>()
                .join(",");
            let branches = mir_function
                .semantic_branches(&model_fields)
                .iter()
                .map(|branch| {
                    let arms = branch
                        .arms
                        .iter()
                        .map(|arm| {
                            let return_expression =
                                arm.return_expression.as_deref().unwrap_or("none");
                            let assumptions = semantic_arm_assumptions_summary_without_guard(arm);
                            format!("{}{}=>{}", arm.guard, assumptions, return_expression)
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    format!("{}:{}", branch.condition, arms)
                })
                .collect::<Vec<_>>()
                .join(",");
            summary.push_str(&format!(
                "mir_function path={} args={} return_type={} debug_locals={} local_types={} contract_bindings={} return_expr={} arithmetic_ops={} slice_indexes={} calls={} field_accesses={} matches={} branches={}\n",
                mir_function.path,
                mir_function
                    .args
                    .iter()
                    .map(|arg| format!("{}: {}", arg.place, arg.ty))
                    .collect::<Vec<_>>()
                    .join(","),
                mir_function.return_type,
                mir_function
                    .debug_locals
                    .iter()
                    .map(|local| local.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                mir_function.source_local_types().join(","),
                contract_binding_summary(&contract_bindings),
                return_expr,
                arithmetic_ops,
                slice_indexes,
                calls,
                field_accesses,
                matches,
                branches,
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
    fn hir_function_match_uses_qualified_owner_path() {
        let hir = r#"
DefId(0:0 ~ sample[abcd]) => OwnerNodes {
}
DefId(0:1 ~ sample[abcd]::left::same) => OwnerNodes {
}
DefId(0:2 ~ sample[abcd]::right::same) => OwnerNodes {
}
"#;

        assert!(hir_contains_function(hir, "left::same"));
        assert!(hir_contains_function(hir, "right::same"));
        assert!(!hir_contains_function(hir, "other::same"));
    }

    #[test]
    fn hir_function_match_requires_unique_unqualified_leaf() {
        let duplicate_hir = r#"
DefId(0:1 ~ sample[abcd]::left::same) => OwnerNodes {
}
DefId(0:2 ~ sample[abcd]::right::same) => OwnerNodes {
}
"#;
        let unique_hir = r#"
DefId(0:1 ~ sample[abcd]::only::same) => OwnerNodes {
}
"#;

        assert!(!hir_contains_function(duplicate_hir, "same"));
        assert!(hir_contains_function(unique_hir, "same"));
    }

    #[test]
    fn hir_function_match_ignores_metadata_string_payloads() {
        let hir = r#"
DefId(0:1 ~ sample[abcd]::__TRUST_META_same_1234) => OwnerNodes {
    "{\"rust_function_path\":\"left::same\",\"function_source\":\"pub fn same(x: i32) -> i32 { x }\"}",
}
"#;

        assert!(!hir_contains_function(hir, "left::same"));
        assert!(!hir_contains_function(hir, "same"));
    }

    #[test]
    fn hir_function_span_extracts_item_span() {
        let hir = r#"
DefId(0:6 ~ sample[abcd]::verified::id_i32) => OwnerNodes {
    node: ParentedNode {
        node: Item(
            Item {
                kind: Fn {
                    sig: FnSig {
                        span: src/lib.rs:2:10: 2:13 (#9),
                    },
                },
                span: src/lib.rs:3:5: 5:6 (#9),
            },
        ),
    },
}
"#;

        assert_eq!(
            hir_function_span(hir, "verified::id_i32"),
            Some("src/lib.rs:3:5: 5:6 (#9)".to_string())
        );
    }

    #[test]
    fn hir_function_span_requires_unique_leaf() {
        let hir = r#"
DefId(0:1 ~ sample[abcd]::left::same) => OwnerNodes {
    node: ParentedNode {
        node: Item(
            Item {
                kind: Fn {},
                span: src/lib.rs:1:1: 1:10 (#9),
            },
        ),
    },
}
DefId(0:2 ~ sample[abcd]::right::same) => OwnerNodes {
    node: ParentedNode {
        node: Item(
            Item {
                kind: Fn {},
                span: src/lib.rs:2:1: 2:10 (#9),
            },
        ),
    },
}
"#;

        assert_eq!(hir_function_span(hir, "same"), None);
    }

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
                path: "id_i32".to_string(),
                args: vec![MirArg {
                    place: "_1".to_string(),
                    ty: "i32".to_string(),
                }],
                return_type: "i32".to_string(),
                locals: vec![MirLocal {
                    place: "_0".to_string(),
                    ty: "i32".to_string(),
                }],
                debug_locals: vec![MirDebugLocal {
                    name: "x".to_string(),
                    place: "_1".to_string(),
                }],
                assignments: vec![MirAssignment {
                    block: Some("bb0".to_string()),
                    place: "_0".to_string(),
                    expression: "copy _1".to_string(),
                    statement_index: 4,
                }],
                terminators: Vec::new(),
                return_expr: Some("copy _1".to_string()),
            })
        );
    }

    #[test]
    fn extracts_mir_function_summary_by_qualified_suffix() {
        let mir = r#"
fn left::same(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = copy _1;
        return;
    }
}

fn right::same(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = Add(copy _1, const 1_i32);
        return;
    }
}
"#;

        assert_eq!(
            extract_mir_function_summary(mir, "right::same")
                .expect("right::same summary")
                .path,
            "right::same"
        );
        assert_eq!(
            extract_mir_function_summary(mir, "left::same")
                .expect("left::same summary")
                .return_expr,
            Some("copy _1".to_string())
        );
    }

    #[test]
    fn extracts_mir_function_summary_by_unique_leaf_when_rustc_omits_module() {
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
            extract_mir_function_summary(mir, "verified::id_i32")
                .expect("verified::id_i32 summary")
                .path,
            "id_i32"
        );
    }

    #[test]
    fn extracts_mir_function_summary_when_rustc_omits_outer_module() {
        let mir = r#"
fn left::same(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = copy _1;
        return;
    }
}

fn right::same(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = Add(copy _1, const 1_i32);
        return;
    }
}
"#;

        assert_eq!(
            extract_mir_function_summary(mir, "outer::left::same")
                .expect("outer::left::same summary")
                .path,
            "left::same"
        );
        assert_eq!(
            extract_mir_function_summary(mir, "outer::right::same")
                .expect("outer::right::same summary")
                .path,
            "right::same"
        );
    }

    #[test]
    fn extracts_model_field_return_expression() {
        let mir = r#"
fn balance(_1: Account) -> i64 {
    debug acct => _1;
    let mut _0: i64;

    bb0: {
        _0 = copy (_1.1: i64);
        return;
    }
}
"#;
        let fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![
                ModelField {
                    name: "id".to_string(),
                    ty: "u64".to_string(),
                },
                ModelField {
                    name: "balance".to_string(),
                    ty: "i64".to_string(),
                },
            ],
        }];
        let summary = extract_mir_function_summary(mir, "balance").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression_with_models(&fields),
            Some("acct.balance".to_string())
        );
        assert_eq!(
            summary.semantic_field_accesses(&fields),
            vec![SemanticFieldAccess {
                base: "acct".to_string(),
                field: "balance".to_string(),
                owner_type: "Account".to_string(),
                field_type: "i64".to_string(),
                expression: "acct.balance".to_string(),
            }]
        );
    }

    #[test]
    fn extracts_model_field_access_through_local_alias() {
        let mir = r#"
fn reward_alias(_1: Account) -> i64 {
    debug acct => _1;
    let mut _0: i64;
    let _2: Account;
    let mut _3: (i64, bool);
    debug alias => _2;

    bb0: {
        _2 = move _1;
        _3 = AddWithOverflow(copy (_2.0: i64), const 1_i64);
        assert(!move (_3.1: bool), "overflow", copy (_2.0: i64), const 1_i64) -> [success: bb1, unwind continue];
    }

    bb1: {
        _0 = move (_3.0: i64);
        return;
    }
}
"#;
        let fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![ModelField {
                name: "balance".to_string(),
                ty: "i64".to_string(),
            }],
        }];
        let summary = extract_mir_function_summary(mir, "reward_alias").expect("MIR summary");

        assert_eq!(
            summary.semantic_field_accesses(&fields),
            vec![SemanticFieldAccess {
                base: "acct".to_string(),
                field: "balance".to_string(),
                owner_type: "Account".to_string(),
                field_type: "i64".to_string(),
                expression: "acct.balance".to_string(),
            }]
        );
        assert_eq!(
            summary.semantic_arithmetic_operations_with_models(&fields),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i64".to_string()),
                target: None,
                left: "acct.balance".to_string(),
                right: Some("1".to_string()),
                expression: "acct.balance + 1".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_unmodeled_field_access_through_local_alias() {
        let mir = r#"
fn balance_alias(_1: Account) -> i64 {
    debug acct => _1;
    let mut _0: i64;
    let _2: Account;
    debug alias => _2;

    bb0: {
        _2 = move _1;
        _0 = copy (_2.0: i64);
        return;
    }
}
"#;
        let summary = extract_mir_function_summary(mir, "balance_alias").expect("MIR summary");

        assert_eq!(
            summary.semantic_field_accesses(&[]),
            vec![SemanticFieldAccess {
                base: "acct".to_string(),
                field: "0".to_string(),
                owner_type: "Account".to_string(),
                field_type: "i64".to_string(),
                expression: "acct.0".to_string(),
            }]
        );
    }

    #[test]
    fn normalizes_mir_aggregate_return_expression() {
        let mir = r#"
fn withdraw(_1: Account, _2: i64) -> Account {
    debug acct => _1;
    debug amount => _2;
    let mut _0: Account;
    let mut _3: u64;
    let mut _4: i64;
    let mut _5: i64;
    let mut _6: (i64, bool);

    bb0: {
        _3 = copy (_1.0: u64);
        _5 = copy (_1.1: i64);
        _6 = SubWithOverflow(copy _5, copy _2);
        assert(!move (_6.1: bool), "attempt to compute `{} - {}`, which would overflow", move _5, copy _2) -> [success: bb1, unwind continue];
    }

    bb1: {
        _4 = move (_6.0: i64);
        _0 = Account { id: move _3, balance: move _4 };
        return;
    }
}
"#;
        let fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![
                ModelField {
                    name: "id".to_string(),
                    ty: "u64".to_string(),
                },
                ModelField {
                    name: "balance".to_string(),
                    ty: "i64".to_string(),
                },
            ],
        }];
        let summary = extract_mir_function_summary(mir, "withdraw").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression_with_models(&fields),
            Some("Account { id: acct.id, balance: acct.balance - amount }".to_string())
        );
    }

    #[test]
    fn extracts_if_branch_guards_and_return_expressions() {
        let mir = r#"
fn zero_if_positive(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: bool;

    bb0: {
        _2 = Gt(copy _1, const 0_i32);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _0 = const 1_i32;
        goto -> bb3;
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb3;
    }

    bb3: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "zero_if_positive").expect("MIR summary");

        assert_eq!(
            summary.semantic_branches(&[]),
            vec![SemanticBranch {
                condition: "x > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x <= 0".to_string(),
                        assumptions: vec!["x <= 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x > 0".to_string(),
                        assumptions: vec!["x > 0".to_string()],
                        return_expression: Some("1".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_if_branch_assignment_return_expressions() {
        let mir = r#"
fn zero_or_self(_1: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    let mut _0: i32;
    let mut _2: i32;
    let mut _3: bool;

    bb0: {
        _2 = copy _1;
        _3 = Eq(copy _1, const 0_i32);
        switchInt(move _3) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _2 = const 0_i32;
        goto -> bb3;
    }

    bb2: {
        _2 = copy _1;
        goto -> bb3;
    }

    bb3: {
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "zero_or_self").expect("MIR summary");

        assert_eq!(
            summary.semantic_branches(&[]),
            vec![SemanticBranch {
                condition: "x == 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: vec!["x != 0".to_string()],
                        return_expression: Some("x".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x == 0".to_string(),
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_if_branch_carried_assignment_return_expressions() {
        let mir = r#"
fn zero_or_self(_1: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    let mut _0: i32;
    let mut _2: i32;

    bb0: {
        _2 = copy _1;
        switchInt(copy _1) -> [0: bb1, otherwise: bb2];
    }

    bb1: {
        _2 = const 0_i32;
        goto -> bb3;
    }

    bb2: {
        goto -> bb3;
    }

    bb3: {
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "zero_or_self").expect("MIR summary");

        assert_eq!(
            summary.semantic_branches(&[]),
            vec![SemanticBranch {
                condition: "x".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x == 0".to_string(),
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: vec!["x != 0".to_string()],
                        return_expression: Some("x".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_if_branch_join_copy_return_expressions() {
        let mir = r#"
fn zero_or_self(_1: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    debug z => _3;
    let mut _0: i32;
    let mut _2: i32;
    let _3: i32;

    bb0: {
        _2 = copy _1;
        switchInt(copy _1) -> [0: bb1, otherwise: bb2];
    }

    bb1: {
        _2 = const 0_i32;
        goto -> bb3;
    }

    bb2: {
        goto -> bb3;
    }

    bb3: {
        _3 = copy _2;
        _0 = copy _3;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "zero_or_self").expect("MIR summary");

        assert_eq!(
            summary.semantic_branches(&[]),
            vec![SemanticBranch {
                condition: "x".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x == 0".to_string(),
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: vec!["x != 0".to_string()],
                        return_expression: Some("x".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_if_branch_guards_from_scalar_switch() {
        let mir = r#"
fn zero_or_self(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        switchInt(copy _1) -> [0: bb1, otherwise: bb2];
    }

    bb1: {
        _0 = const 0_i32;
        goto -> bb3;
    }

    bb2: {
        _0 = copy _1;
        goto -> bb3;
    }

    bb3: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "zero_or_self").expect("MIR summary");

        assert_eq!(
            summary.semantic_branches(&[]),
            vec![SemanticBranch {
                condition: "x".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x == 0".to_string(),
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: vec!["x != 0".to_string()],
                        return_expression: Some("x".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_nested_if_branch_path_assumptions() {
        let mir = r#"
fn nested_zero_or_self(_1: i32, _2: bool) -> i32 {
    debug x => _1;
    debug flag => _2;
    let mut _0: i32;
    let mut _3: bool;

    bb0: {
        _3 = Eq(copy _1, const 0_i32);
        switchInt(move _3) -> [0: bb4, otherwise: bb1];
    }

    bb1: {
        switchInt(copy _2) -> [0: bb3, otherwise: bb2];
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb5;
    }

    bb3: {
        _0 = const 0_i32;
        goto -> bb5;
    }

    bb4: {
        _0 = copy _1;
        goto -> bb5;
    }

    bb5: {
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "nested_zero_or_self").expect("MIR summary");
        let branch = summary
            .semantic_branches(&[])
            .into_iter()
            .find(|branch| branch.condition == "flag")
            .expect("inner flag branch");

        assert_eq!(
            branch.arms,
            vec![
                SemanticBranchArm {
                    guard: "flag == 0".to_string(),
                    assumptions: vec!["x == 0".to_string(), "flag == 0".to_string()],
                    return_expression: Some("0".to_string()),
                },
                SemanticBranchArm {
                    guard: "flag != 0".to_string(),
                    assumptions: vec!["x == 0".to_string(), "flag != 0".to_string()],
                    return_expression: Some("0".to_string()),
                },
            ]
        );
    }

    #[test]
    fn extracts_option_match_variants_and_payload() {
        let mir = r#"
fn unwrap_or_zero(_1: Option<i32>) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: isize;
    let _3: i32;
    scope 1 {
        debug v => _3;
    }

    bb0: {
        _2 = discriminant(_1);
        switchInt(move _2) -> [0: bb2, 1: bb3, otherwise: bb1];
    }

    bb1: {
        unreachable;
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb4;
    }

    bb3: {
        _3 = copy ((_1 as Some).0: i32);
        _0 = copy _3;
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "unwrap_or_zero").expect("MIR summary");

        assert_eq!(summary.normalized_return_expression(), None);
        assert_eq!(
            summary.semantic_matches(&[]),
            vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Some".to_string(),
                        discriminant: "1".to_string(),
                        payload: Some(SemanticMatchPayload {
                            binding: "v".to_string(),
                            field_index: 0,
                            ty: "i32".to_string(),
                        }),
                        assumptions: Vec::new(),
                        return_expression: Some("v".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_option_match_from_local_alias() {
        let mir = r#"
fn alias_unwrap_or_zero(_1: Option<i32>) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let _2: Option<i32>;
    let mut _3: isize;
    let _4: i32;
    scope 1 {
        debug y => _2;
        scope 2 {
            debug v => _4;
        }
    }

    bb0: {
        _2 = copy _1;
        _3 = discriminant(_2);
        switchInt(move _3) -> [0: bb2, 1: bb3, otherwise: bb1];
    }

    bb1: {
        unreachable;
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb4;
    }

    bb3: {
        _4 = copy ((_2 as Some).0: i32);
        _0 = copy _4;
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "alias_unwrap_or_zero").expect("MIR summary");

        assert_eq!(
            summary.semantic_matches(&[]),
            vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Some".to_string(),
                        discriminant: "1".to_string(),
                        payload: Some(SemanticMatchPayload {
                            binding: "v".to_string(),
                            field_index: 0,
                            ty: "i32".to_string(),
                        }),
                        assumptions: Vec::new(),
                        return_expression: Some("v".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_nested_match_path_assumptions() {
        let mir = r#"
fn nested_match_zero_or_self(_1: i32, _2: Option<i32>) -> i32 {
    debug x => _1;
    debug choice => _2;
    let mut _0: i32;
    let mut _3: bool;
    let mut _4: isize;

    bb0: {
        _3 = Eq(copy _1, const 0_i32);
        switchInt(move _3) -> [0: bb4, otherwise: bb1];
    }

    bb1: {
        _4 = discriminant(_2);
        switchInt(move _4) -> [0: bb2, 1: bb3, otherwise: bb5];
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb6;
    }

    bb3: {
        _0 = const 0_i32;
        goto -> bb6;
    }

    bb4: {
        _0 = copy _1;
        goto -> bb6;
    }

    bb5: {
        unreachable;
    }

    bb6: {
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "nested_match_zero_or_self").expect("MIR summary");

        assert_eq!(
            summary.semantic_matches(&[]),
            vec![SemanticMatch {
                scrutinee: "choice".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Some".to_string(),
                        discriminant: "1".to_string(),
                        payload: None,
                        assumptions: vec!["x == 0".to_string()],
                        return_expression: Some("0".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn extracts_result_match_variants_and_payload() {
        let mir = r#"
fn unwrap_or_zero(_1: Result<i32, i32>) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: isize;
    let _3: i32;
    scope 1 {
        debug v => _3;
    }

    bb0: {
        _2 = discriminant(_1);
        switchInt(move _2) -> [0: bb3, 1: bb2, otherwise: bb1];
    }

    bb1: {
        unreachable;
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb4;
    }

    bb3: {
        _3 = copy ((_1 as Ok).0: i32);
        _0 = copy _3;
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "unwrap_or_zero").expect("MIR summary");

        assert_eq!(summary.normalized_return_expression(), None);
        assert_eq!(
            summary.semantic_matches(&[]),
            vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Result<i32, i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "Ok".to_string(),
                        discriminant: "0".to_string(),
                        payload: Some(SemanticMatchPayload {
                            binding: "v".to_string(),
                            field_index: 0,
                            ty: "i32".to_string(),
                        }),
                        assumptions: Vec::new(),
                        return_expression: Some("v".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Err".to_string(),
                        discriminant: "1".to_string(),
                        payload: None,
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                ],
            }]
        );
    }

    #[test]
    fn parses_generic_mir_args_without_splitting_type_parameters() {
        assert_eq!(
            parse_mir_args("_1: Result<i32, i32>"),
            vec![MirArg {
                place: "_1".to_string(),
                ty: "Result<i32, i32>".to_string(),
            }]
        );
    }

    #[test]
    fn parses_model_fields_from_metadata_source() {
        assert_eq!(
            parse_model_fields("pub struct Account { pub id: u64, pub balance: i64, }"),
            vec![
                ModelField {
                    name: "id".to_string(),
                    ty: "u64".to_string(),
                },
                ModelField {
                    name: "balance".to_string(),
                    ty: "i64".to_string(),
                },
            ]
        );
    }

    #[test]
    fn normalizes_mir_overflow_add_return_expression() {
        let mir = r#"
fn add_one(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: (i32, bool);

    bb0: {
        _2 = AddWithOverflow(copy _1, const 1_i32);
        assert(!move (_2.1: bool), "overflow") -> [success: bb1, unwind continue];
    }

    bb1: {
        _0 = move (_2.0: i32);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "add_one").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x + 1".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_plain_mir_arithmetic_operations() {
        let mir = r#"
fn arithmetic(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: i32;
    let mut _3: i32;
    let mut _4: i32;
    let mut _5: i32;

    bb0: {
        _2 = Add(copy _1, const 1_i32);
        _3 = Sub(copy _1, const 1_i32);
        _4 = Mul(copy _1, const 2_i32);
        _5 = Neg(copy _1);
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "arithmetic").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x + 1".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Add,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("1".to_string()),
                    expression: "x + 1".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Sub,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("1".to_string()),
                    expression: "x - 1".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Mul,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("2".to_string()),
                    expression: "x * 2".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Neg,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: None,
                    expression: "-x".to_string(),
                    guards: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn normalizes_mir_return_through_local_assignment() {
        let mir = r#"
fn add_one(_1: i32) -> i32 {
    debug x => _1;
    debug y => _3;
    let mut _0: i32;
    let mut _2: (i32, bool);
    let _3: i32;

    bb0: {
        _2 = AddWithOverflow(copy _1, const 1_i32);
        _3 = move (_2.0: i32);
        _0 = copy _3;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "add_one").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x + 1".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: Some("y".to_string()),
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_arithmetic_target_through_overflow_temp_assignment() {
        let mir = r#"
fn count_up(_1: usize, _2: usize) -> usize {
    debug i => _1;
    debug n => _2;
    let mut _0: usize;
    let mut _3: bool;
    let mut _4: (usize, bool);

    bb0: {
        _3 = Lt(copy _1, copy _2);
        switchInt(move _3) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _4 = AddWithOverflow(copy _1, const 1_usize);
        assert(!move (_4.1: bool), "overflow", copy _1, const 1_usize) -> [success: bb3, unwind continue];
    }

    bb2: {
        _0 = copy _1;
        return;
    }

    bb3: {
        _1 = move (_4.0: usize);
        goto -> bb0;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "count_up").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("usize".to_string()),
                target: Some("i".to_string()),
                left: "i".to_string(),
                right: Some("1".to_string()),
                expression: "i + 1".to_string(),
                guards: vec!["i < n".to_string()],
            }]
        );
    }

    #[test]
    fn does_not_inline_reassigned_local_return_value() {
        let mir = r#"
fn choose(_1: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    let mut _0: i32;
    let mut _2: i32;
    let mut _3: bool;

    bb0: {
        _2 = const 0_i32;
        _3 = Gt(copy _1, const 0_i32);
        switchInt(move _3) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _2 = const 1_i32;
        goto -> bb3;
    }

    bb2: {
        _2 = const 2_i32;
        goto -> bb3;
    }

    bb3: {
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "choose").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("y".to_string())
        );
    }

    #[test]
    fn does_not_inline_mutated_argument_in_later_operations() {
        let mir = r#"
fn divide_after_countdown(_1: usize) -> usize {
    debug n => _1;
    let mut _0: usize;
    let mut _2: bool;

    bb0: {
        _2 = Gt(copy _1, const 0_usize);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _1 = Sub(copy _1, const 1_usize);
        goto -> bb0;
    }

    bb2: {
        _0 = Div(const 1_usize, copy _1);
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "divide_after_countdown").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("1 / n".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Sub,
                    ty: Some("usize".to_string()),
                    target: Some("n".to_string()),
                    left: "n".to_string(),
                    right: Some("1".to_string()),
                    expression: "n - 1".to_string(),
                    guards: vec!["n > 0".to_string()],
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Div,
                    ty: Some("usize".to_string()),
                    target: None,
                    left: "1".to_string(),
                    right: Some("n".to_string()),
                    expression: "1 / n".to_string(),
                    guards: vec!["n <= 0".to_string()],
                },
            ]
        );
    }

    #[test]
    fn drops_loop_guard_after_guarded_argument_is_mutated() {
        let mir = r#"
fn divide_inside_loop(_1: usize) -> usize {
    debug n => _1;
    let mut _0: usize;
    let mut _2: bool;

    bb0: {
        _2 = Gt(copy _1, const 0_usize);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _1 = Sub(copy _1, const 1_usize);
        _0 = Div(const 1_usize, copy _1);
        return;
    }

    bb2: {
        _0 = const 0_usize;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "divide_inside_loop").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Sub,
                    ty: Some("usize".to_string()),
                    target: Some("n".to_string()),
                    left: "n".to_string(),
                    right: Some("1".to_string()),
                    expression: "n - 1".to_string(),
                    guards: vec!["n > 0".to_string()],
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Div,
                    ty: Some("usize".to_string()),
                    target: None,
                    left: "1".to_string(),
                    right: Some("n".to_string()),
                    expression: "1 / n".to_string(),
                    guards: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn extracts_branch_guard_from_non_topological_mir_block_order() {
        let mir = r#"
fn add_from_late_branch(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: bool;
    let mut _3: (i32, bool);

    bb0: {
        goto -> bb3;
    }

    bb1: {
        _3 = AddWithOverflow(copy _1, const 1_i32);
        assert(!move (_3.1: bool), "overflow", copy _1, const 1_i32) -> [success: bb4, unwind continue];
    }

    bb2: {
        _0 = copy _1;
        goto -> bb5;
    }

    bb3: {
        _2 = Lt(copy _1, const core::num::<impl i32>::MAX);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb4: {
        _0 = move (_3.0: i32);
        goto -> bb5;
    }

    bb5: {
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "add_from_late_branch").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: vec!["x < i32::MAX".to_string()],
            }]
        );
    }

    #[test]
    fn normalizes_mir_direct_call_return_expression() {
        let mir = r#"
fn has_items(_1: &[i32]) -> bool {
    debug xs => _1;
    let mut _0: bool;

    bb0: {
        _0 = verified::nonempty(copy _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "has_items").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("verified::nonempty(xs)".to_string())
        );
    }

    #[test]
    fn extracts_resolved_checked_add_call() {
        let mir = r#"
fn checked_sum(_1: i32, _2: i32) -> Option<i32> {
    debug x => _1;
    debug y => _2;
    let mut _0: std::option::Option<i32>;

    bb0: {
        _0 = core::num::<impl i32>::checked_add(copy _1, copy _2) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "checked_sum").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x.checked_add(y)".to_string())
        );
        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "core::num::<impl i32>::checked_add".to_string(),
                args: vec!["x".to_string(), "y".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_resolved_trait_method_call_through_borrowed_receiver() {
        let mir = r#"
fn display(_1: i32) -> String {
    debug x => _1;
    let mut _0: std::string::String;
    let mut _2: &i32;

    bb0: {
        _2 = &_1;
        _0 = <i32 as ToString>::to_string(move _2) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "display").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("<i32 as ToString>::to_string(x)".to_string())
        );
        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "<i32 as ToString>::to_string".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_resolved_closure_call_as_unsupported_call() {
        let mir = r#"
fn apply(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let _2: {closure@src/lib.rs:1:1: 1:17};
    let mut _3: &{closure@src/lib.rs:1:1: 1:17};
    let mut _4: (i32,);
    scope 1 {
        debug inc => const ZeroSized: {closure@src/lib.rs:1:1: 1:17};
    }

    bb0: {
        _3 = &_2;
        _4 = (copy _1,);
        _0 = <{closure@src/lib.rs:1:1: 1:17} as Fn<(i32,)>>::call(move _3, move _4) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "apply").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "<{closure@src/lib.rs:1:1: 1:17} as Fn<(i32,)>>::call".to_string(),
                args: vec!["_2".to_string(), "(x,)".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_arithmetic_type_from_mir_local_constant() {
        let mir = r#"
fn overflow() -> i32 {
    debug x => _1;
    let mut _0: i32;
    let _1: i32;
    let mut _2: (i32, bool);

    bb0: {
        _1 = const core::num::<impl i32>::MAX;
        _2 = AddWithOverflow(copy _1, const 1_i32);
        assert(!move (_2.1: bool), "overflow") -> [success: bb1, unwind continue];
    }

    bb1: {
        _0 = move (_2.0: i32);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "overflow").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "i32::MAX".to_string(),
                right: Some("1".to_string()),
                expression: "i32::MAX + 1".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_source_local_types_from_mir_debug_locals() {
        let mir = r#"
fn keep(_1: i32) -> i32 {
    debug x => _1;
    debug y => const 1f32;
    debug count => _2;
    let mut _0: i32;
    let _2: u32;

    bb0: {
        _2 = const 1_u32;
        _0 = copy _1;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "keep").expect("MIR summary");

        assert_eq!(
            summary.source_local_types(),
            vec!["i32".to_string(), "f32".to_string(), "u32".to_string()]
        );
    }

    #[test]
    fn extracts_source_local_bindings_from_mir_debug_locals() {
        let mir = r#"
fn keep(_1: i32) -> i32 {
    debug x => _1;
    debug count => _2;
    let mut _0: i32;
    let _2: u32;

    bb0: {
        _2 = const 1_u32;
        _0 = copy _1;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "keep").expect("MIR summary");

        assert_eq!(
            semantic_local_bindings(&summary, &[]),
            vec![
                SemanticContractBinding {
                    expression: "count".to_string(),
                    name: "count".to_string(),
                    kind: SemanticContractBindingKind::Local,
                    ty: "u32".to_string(),
                },
                SemanticContractBinding {
                    expression: "1".to_string(),
                    name: "count".to_string(),
                    kind: SemanticContractBindingKind::LocalInitializer,
                    ty: "u32".to_string(),
                },
            ]
        );
    }

    #[test]
    fn extracts_loop_local_initializer_from_mir() {
        let mir = r#"
fn count_to(_1: usize) -> usize {
    debug n => _1;
    debug i => _2;
    let mut _0: usize;
    let mut _2: usize;
    let mut _3: bool;
    let mut _4: (usize, bool);

    bb0: {
        _2 = const 0_usize;
        goto -> bb1;
    }

    bb1: {
        _3 = Lt(copy _2, copy _1);
        switchInt(move _3) -> [0: bb3, otherwise: bb2];
    }

    bb2: {
        _4 = AddWithOverflow(copy _2, const 1_usize);
        assert(!move (_4.1: bool), "attempt to compute `{} + {}`, which would overflow", copy _2, const 1_usize) -> [success: bb4, unwind continue];
    }

    bb4: {
        _2 = move (_4.0: usize);
        goto -> bb1;
    }

    bb3: {
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "count_to").expect("MIR summary");

        assert!(
            semantic_local_bindings(&summary, &[]).contains(&SemanticContractBinding {
                expression: "0".to_string(),
                name: "i".to_string(),
                kind: SemanticContractBindingKind::LocalInitializer,
                ty: "usize".to_string(),
            })
        );
    }

    #[test]
    fn skips_local_initializer_when_later_guarded_assignment_is_not_self_update() {
        let mir = r#"
fn adjust_before_loop(_1: usize) -> usize {
    debug n => _1;
    debug i => _2;
    let mut _0: usize;
    let mut _2: usize;
    let mut _3: bool;
    let mut _4: (usize, bool);
    let mut _5: bool;

    bb0: {
        _2 = const 0_usize;
        _3 = Lt(copy _2, copy _1);
        switchInt(move _3) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _4 = AddWithOverflow(copy _1, const 1_usize);
        assert(!move (_4.1: bool), "attempt to compute `{} + {}`, which would overflow", copy _1, const 1_usize) -> [success: bb4, unwind continue];
    }

    bb4: {
        _2 = move (_4.0: usize);
        goto -> bb2;
    }

    bb2: {
        _5 = Lt(copy _2, copy _1);
        switchInt(move _5) -> [0: bb3, otherwise: bb3];
    }

    bb3: {
        _0 = copy _2;
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "adjust_before_loop").expect("MIR summary");

        assert!(
            !semantic_local_bindings(&summary, &[]).contains(&SemanticContractBinding {
                expression: "0".to_string(),
                name: "i".to_string(),
                kind: SemanticContractBindingKind::LocalInitializer,
                ty: "usize".to_string(),
            })
        );
    }

    #[test]
    fn extracts_mir_branch_guard_for_arithmetic_operation() {
        let mir = r#"
fn add_if_safe(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: bool;
    let mut _3: (i32, bool);

    bb0: {
        _2 = Lt(copy _1, const core::num::<impl i32>::MAX);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _3 = AddWithOverflow(copy _1, const 1_i32);
        assert(!move (_3.1: bool), "overflow", copy _1, const 1_i32) -> [success: bb3, unwind continue];
    }

    bb2: {
        _0 = copy _1;
        goto -> bb4;
    }

    bb3: {
        _0 = move (_3.0: i32);
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "add_if_safe").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: vec!["x < i32::MAX".to_string()],
            }]
        );
    }

    #[test]
    fn extracts_mir_branch_guard_for_model_field_arithmetic() {
        let mir = r#"
fn withdraw_if_safe(_1: Account, _2: i64) -> i64 {
    debug acct => _1;
    debug amount => _2;
    let mut _0: i64;
    let mut _3: i64;
    let mut _4: bool;
    let mut _5: (i64, bool);

    bb0: {
        _3 = copy (_1.0: i64);
        _4 = Ge(copy _3, copy _2);
        switchInt(move _4) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _5 = SubWithOverflow(copy _3, copy _2);
        assert(!move (_5.1: bool), "overflow", copy _3, copy _2) -> [success: bb3, unwind continue];
    }

    bb2: {
        _0 = copy _3;
        goto -> bb4;
    }

    bb3: {
        _0 = move (_5.0: i64);
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;
        let fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![ModelField {
                name: "balance".to_string(),
                ty: "i64".to_string(),
            }],
        }];
        let summary = extract_mir_function_summary(mir, "withdraw_if_safe").expect("MIR summary");

        assert_eq!(
            summary.semantic_arithmetic_operations_with_models(&fields),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("i64".to_string()),
                target: None,
                left: "acct.balance".to_string(),
                right: Some("amount".to_string()),
                expression: "acct.balance - amount".to_string(),
                guards: vec!["acct.balance >= amount".to_string()],
            }]
        );
    }

    #[test]
    fn extracts_mir_division_and_remainder_operations() {
        let mir = r#"
fn ratio_and_mod(_1: i32, _2: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    let mut _0: i32;
    let mut _3: i32;

    bb0: {
        _3 = Rem(copy _1, copy _2);
        _0 = Div(copy _1, copy _2);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "ratio_and_mod").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x / y".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Rem,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("y".to_string()),
                    expression: "x % y".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Div,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("y".to_string()),
                    expression: "x / y".to_string(),
                    guards: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn normalizes_nested_mir_arithmetic_with_parentheses() {
        let mir = r#"
fn div(_1: i32, _2: i32) -> i32 {
    debug x => _1;
    debug y => _2;
    let mut _0: i32;
    let mut _3: i32;
    let mut _4: (i32, bool);

    bb0: {
        _4 = AddWithOverflow(copy _2, const 0_i32);
        _3 = move (_4.0: i32);
        _0 = Div(copy _1, move _3);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "div").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x / (y + 0)".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Add,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "y".to_string(),
                    right: Some("0".to_string()),
                    expression: "y + 0".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Div,
                    ty: Some("i32".to_string()),
                    target: None,
                    left: "x".to_string(),
                    right: Some("y + 0".to_string()),
                    expression: "x / (y + 0)".to_string(),
                    guards: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn preserves_right_nested_arithmetic_shape() {
        assert_eq!(semantic_binary_expression("x", "+", "y - z"), "x + (y - z)");
        assert_eq!(semantic_binary_expression("x", "*", "y / z"), "x * (y / z)");
        assert_eq!(semantic_binary_expression("x + y", "*", "z"), "(x + y) * z");
        assert_eq!(semantic_binary_expression("x", "+", "y * z"), "x + y * z");
        assert_eq!(semantic_binary_expression("x / y", "*", "z"), "x / y * z");
    }

    #[test]
    fn extracts_negative_mir_integer_constants() {
        let mir = r#"
fn div_neg_one(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = Div(copy _1, const -1_i32);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "div_neg_one").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x / -1".to_string())
        );
        assert_eq!(
            summary.semantic_arithmetic_operations(),
            vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("-1".to_string()),
                expression: "x / -1".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_module_qualified_slice_index_summary() {
        let mir = r#"
fn verified::get(_1: &[i32], _2: usize) -> i32 {
    debug xs => _1;
    debug i => _2;
    let mut _0: i32;
    let mut _3: usize;
    let mut _4: bool;

    bb0: {
        _3 = PtrMetadata(copy _1);
        _4 = Lt(copy _2, copy _3);
        assert(move _4, "index out of bounds") -> [success: bb1, unwind continue];
    }

    bb1: {
        _0 = copy (*_1)[_2];
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "get").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("xs[i]".to_string())
        );
        assert_eq!(
            summary.semantic_slice_indexes(),
            vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
                expression: "xs[i]".to_string(),
                guards: Vec::new(),
            }]
        );
    }

    #[test]
    fn extracts_mir_branch_guard_for_slice_index() {
        let mir = r#"
fn verified::get_or_zero(_1: &[i32], _2: usize) -> i32 {
    debug xs => _1;
    debug i => _2;
    let mut _0: i32;
    let mut _3: usize;
    let mut _4: bool;

    bb0: {
        _3 = PtrMetadata(copy _1);
        _4 = Lt(copy _2, copy _3);
        switchInt(move _4) -> [0: bb3, otherwise: bb1];
    }

    bb1: {
        _5 = PtrMetadata(copy _1);
        _6 = Lt(copy _2, copy _5);
        assert(move _6, "index out of bounds") -> [success: bb2, unwind continue];
    }

    bb2: {
        _0 = copy (*_1)[_2];
        goto -> bb4;
    }

    bb3: {
        _0 = const 0_i32;
        goto -> bb4;
    }

    bb4: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "get_or_zero").expect("MIR summary");

        assert_eq!(
            summary.semantic_slice_indexes(),
            vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
                expression: "xs[i]".to_string(),
                guards: vec!["i < xs.len()".to_string()],
            }]
        );
    }

    #[test]
    fn extracts_supported_slice_len_alias_calls() {
        let mir = r#"
fn get_or_zero(_1: &[i32], _2: usize) -> i32 {
    debug xs => _1;
    debug i => _2;
    let mut _0: i32;
    let mut _3: bool;
    let mut _4: usize;
    scope 1 {
        debug ys => _1;
    }

    bb0: {
        _4 = PtrMetadata(copy _1);
        _3 = Lt(copy _2, move _4);
        switchInt(move _3) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _0 = copy (*_1)[_2];
        goto -> bb3;
    }

    bb2: {
        _0 = const 0_i32;
        goto -> bb3;
    }

    bb3: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "get_or_zero").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![
                SemanticCall {
                    callee: "<slice>.len".to_string(),
                    args: vec!["xs".to_string()],
                    guards: Vec::new(),
                    trust_callee: None,
                },
                SemanticCall {
                    callee: "<slice>.len".to_string(),
                    args: vec!["ys".to_string()],
                    guards: Vec::new(),
                    trust_callee: None,
                },
            ]
        );
        assert_eq!(
            summary.semantic_slice_indexes(),
            vec![
                SemanticSliceIndex {
                    base: "xs".to_string(),
                    base_type: "&[i32]".to_string(),
                    index: "i".to_string(),
                    index_type: "usize".to_string(),
                    element_type: "i32".to_string(),
                    expression: "xs[i]".to_string(),
                    guards: vec!["i < xs.len()".to_string()],
                },
                SemanticSliceIndex {
                    base: "ys".to_string(),
                    base_type: "&[i32]".to_string(),
                    index: "i".to_string(),
                    index_type: "usize".to_string(),
                    element_type: "i32".to_string(),
                    expression: "ys[i]".to_string(),
                    guards: vec!["i < ys.len()".to_string()],
                },
            ]
        );
    }

    #[test]
    fn extracts_module_qualified_call_summary() {
        let mir = r#"
fn verified::caller(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = verified::inc(copy _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "caller").expect("MIR summary");

        assert_eq!(summary.path, "verified::caller");
        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "verified::inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_unsupported_shift_operation_as_semantic_call() {
        let mir = r#"
fn shift_left(_1: u32, _2: u32) -> u32 {
    debug x => _1;
    debug n => _2;
    let mut _0: u32;

    bb0: {
        _0 = Shl(copy _1, copy _2);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "shift_left").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "x << n".to_string(),
                args: vec!["x".to_string(), "n".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_unsupported_bitwise_operation_as_semantic_call() {
        let mir = r#"
fn bitwise_and(_1: u32, _2: u32) -> u32 {
    debug x => _1;
    debug mask => _2;
    let mut _0: u32;

    bb0: {
        _0 = BitAnd(copy _1, copy _2);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "bitwise_and").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "x & mask".to_string(),
                args: vec!["x".to_string(), "mask".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn extracts_unsupported_integer_not_operation_as_semantic_call() {
        let mir = r#"
fn bitwise_not(_1: u32) -> u32 {
    debug x => _1;
    let mut _0: u32;

    bb0: {
        _0 = Not(copy _1);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "bitwise_not").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "!x".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn ignores_supported_boolean_not_operation_as_semantic_call() {
        let mir = r#"
fn boolean_not(_1: bool) -> bool {
    debug x => _1;
    let mut _0: bool;

    bb0: {
        _0 = Not(copy _1);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "boolean_not").expect("MIR summary");

        assert_eq!(summary.semantic_calls(), Vec::new());
    }

    #[test]
    fn extracts_unsupported_cast_as_semantic_call() {
        let mir = r#"
fn narrow(_1: u64) -> u32 {
    debug x => _1;
    let mut _0: u32;

    bb0: {
        _0 = copy _1 as u32 (IntToInt);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "narrow").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "x as u32 (IntToInt)".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn attaches_trust_callee_contract_metadata_to_call_summary() {
        let mir = r#"
fn verified::caller(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = verified::inc(copy _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;
        let trust_callee = SemanticTrustCallee {
            rust_function_path: "inc".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            preconditions: vec!["x < i32::MAX".to_string()],
        };
        let summary = extract_mir_function_summary(mir, "caller").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls_with_models(&[], &[trust_callee.clone()]),
            vec![SemanticCall {
                callee: "verified::inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: Some(trust_callee),
            }]
        );
    }

    #[test]
    fn trust_callee_metadata_uses_unique_mir_path_suffix() {
        let mir = r#"
fn left::caller(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;

    bb0: {
        _0 = left::inc(copy _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
"#;
        let left_callee = SemanticTrustCallee {
            rust_function_path: "outer::left::inc".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            preconditions: vec!["x > 0".to_string()],
        };
        let right_callee = SemanticTrustCallee {
            rust_function_path: "outer::right::inc".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            preconditions: vec!["x < 0".to_string()],
        };
        let summary = extract_mir_function_summary(mir, "outer::left::caller")
            .expect("outer::left::caller summary");

        assert_eq!(
            summary.semantic_calls_with_models(&[], &[left_callee.clone(), right_callee]),
            vec![SemanticCall {
                callee: "left::inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: Some(left_callee),
            }]
        );
    }

    #[test]
    fn trust_callee_metadata_uses_mir_signature_path() {
        let metadata = vec![TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "unknown".to_string(),
            item_kind: "total".to_string(),
            item_id: "total:inc:test".to_string(),
            source_span: "unknown".to_string(),
            rust_function_path: "inc".to_string(),
            visibility: "public".to_string(),
            contracts_original: vec!["x < i32::MAX".to_string()],
            contracts_normalized: vec!["x < i32::MAX".to_string()],
            contract_classes: vec!["given executable".to_string()],
            assertion_policy: "always".to_string(),
            function_source: "pub fn inc(x: i32) -> i32 { x + 1 }".to_string(),
            loop_specs: Vec::new(),
            body_hash_placeholder: "test".to_string(),
            trust_model_dependencies: Vec::new(),
        }];
        let item_matches = vec![SemanticItemMatch {
            item_kind: "total".to_string(),
            item_id: "total:inc:test".to_string(),
            rust_function_path: "inc".to_string(),
            resolved_rust_function_path: Some("verified::inc".to_string()),
            source_span: "unknown".to_string(),
            hir_match: true,
            mir_match: true,
            mir_function: Some(MirFunctionSummary {
                path: "verified::inc".to_string(),
                args: vec![MirArg {
                    place: "_1".to_string(),
                    ty: "i32".to_string(),
                }],
                return_type: "i32".to_string(),
                locals: Vec::new(),
                debug_locals: vec![MirDebugLocal {
                    name: "x".to_string(),
                    place: "_1".to_string(),
                }],
                assignments: Vec::new(),
                terminators: Vec::new(),
                return_expr: None,
            }),
        }];

        assert_eq!(
            semantic_trust_callees(&metadata, &item_matches),
            vec![SemanticTrustCallee {
                rust_function_path: "verified::inc".to_string(),
                params: vec![SemanticParam {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                }],
                preconditions: vec!["x < i32::MAX".to_string()],
            }]
        );
    }

    #[test]
    fn maps_contract_identifiers_to_typed_program_entities() {
        let item = TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "unknown".to_string(),
            item_kind: "total".to_string(),
            item_id: "total:withdraw:test".to_string(),
            source_span: "unknown".to_string(),
            rust_function_path: "withdraw".to_string(),
            visibility: "public".to_string(),
            contracts_original: vec![
                "account.balance >= amount".to_string(),
                "out.id == account.id".to_string(),
            ],
            contracts_normalized: vec![
                "account.balance >= amount".to_string(),
                "out.id == account.id".to_string(),
            ],
            contract_classes: vec!["given executable".to_string(), "gives ghost".to_string()],
            assertion_policy: "always".to_string(),
            function_source:
                "pub fn withdraw(account: Account, amount: i64) -> Account { account }".to_string(),
            loop_specs: Vec::new(),
            body_hash_placeholder: "test".to_string(),
            trust_model_dependencies: Vec::new(),
        };
        let mir_function = MirFunctionSummary {
            path: "verified::withdraw".to_string(),
            args: vec![
                MirArg {
                    place: "_1".to_string(),
                    ty: "Account".to_string(),
                },
                MirArg {
                    place: "_2".to_string(),
                    ty: "i64".to_string(),
                },
            ],
            return_type: "Account".to_string(),
            locals: Vec::new(),
            debug_locals: vec![
                MirDebugLocal {
                    name: "account".to_string(),
                    place: "_1".to_string(),
                },
                MirDebugLocal {
                    name: "amount".to_string(),
                    place: "_2".to_string(),
                },
            ],
            assignments: Vec::new(),
            terminators: Vec::new(),
            return_expr: None,
        };
        let model_fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![
                ModelField {
                    name: "id".to_string(),
                    ty: "u64".to_string(),
                },
                ModelField {
                    name: "balance".to_string(),
                    ty: "i64".to_string(),
                },
            ],
        }];

        assert_eq!(
            semantic_contract_bindings(&item, &mir_function, &model_fields),
            vec![
                SemanticContractBinding {
                    expression: "account.balance >= amount".to_string(),
                    name: "account".to_string(),
                    kind: SemanticContractBindingKind::Param,
                    ty: "Account".to_string(),
                },
                SemanticContractBinding {
                    expression: "account.balance >= amount".to_string(),
                    name: "account.balance".to_string(),
                    kind: SemanticContractBindingKind::Field,
                    ty: "i64".to_string(),
                },
                SemanticContractBinding {
                    expression: "account.balance >= amount".to_string(),
                    name: "amount".to_string(),
                    kind: SemanticContractBindingKind::Param,
                    ty: "i64".to_string(),
                },
                SemanticContractBinding {
                    expression: "out.id == account.id".to_string(),
                    name: "out".to_string(),
                    kind: SemanticContractBindingKind::Result,
                    ty: "Account".to_string(),
                },
                SemanticContractBinding {
                    expression: "out.id == account.id".to_string(),
                    name: "out.id".to_string(),
                    kind: SemanticContractBindingKind::Field,
                    ty: "u64".to_string(),
                },
                SemanticContractBinding {
                    expression: "out.id == account.id".to_string(),
                    name: "account".to_string(),
                    kind: SemanticContractBindingKind::Param,
                    ty: "Account".to_string(),
                },
                SemanticContractBinding {
                    expression: "out.id == account.id".to_string(),
                    name: "account.id".to_string(),
                    kind: SemanticContractBindingKind::Field,
                    ty: "u64".to_string(),
                },
            ]
        );
    }

    #[test]
    fn maps_custom_result_binder_from_normalized_contract_metadata() {
        let item = TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "unknown".to_string(),
            item_kind: "total".to_string(),
            item_id: "total:balance:test".to_string(),
            source_span: "unknown".to_string(),
            rust_function_path: "balance".to_string(),
            visibility: "public".to_string(),
            contracts_original: vec!["result == account.balance".to_string()],
            contracts_normalized: vec!["out == account.balance".to_string()],
            contract_classes: vec!["gives ghost".to_string()],
            assertion_policy: "always".to_string(),
            function_source: "pub fn balance(account: Account) -> i64 { account.balance }"
                .to_string(),
            loop_specs: Vec::new(),
            body_hash_placeholder: "test".to_string(),
            trust_model_dependencies: Vec::new(),
        };
        let mir_function = MirFunctionSummary {
            path: "verified::balance".to_string(),
            args: vec![MirArg {
                place: "_1".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            locals: Vec::new(),
            debug_locals: vec![MirDebugLocal {
                name: "account".to_string(),
                place: "_1".to_string(),
            }],
            assignments: Vec::new(),
            terminators: Vec::new(),
            return_expr: None,
        };
        let model_fields = vec![ModelFieldMap {
            ty: "Account".to_string(),
            fields: vec![ModelField {
                name: "balance".to_string(),
                ty: "i64".to_string(),
            }],
        }];

        assert_eq!(
            semantic_contract_bindings(&item, &mir_function, &model_fields),
            vec![
                SemanticContractBinding {
                    expression: "out == account.balance".to_string(),
                    name: "out".to_string(),
                    kind: SemanticContractBindingKind::Result,
                    ty: "i64".to_string(),
                },
                SemanticContractBinding {
                    expression: "out == account.balance".to_string(),
                    name: "account".to_string(),
                    kind: SemanticContractBindingKind::Param,
                    ty: "Account".to_string(),
                },
                SemanticContractBinding {
                    expression: "out == account.balance".to_string(),
                    name: "account.balance".to_string(),
                    kind: SemanticContractBindingKind::Field,
                    ty: "i64".to_string(),
                },
            ]
        );
    }

    #[test]
    fn extracts_mir_branch_guard_for_call() {
        let mir = r#"
fn verified::caller(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: bool;

    bb0: {
        _2 = Lt(copy _1, const core::num::<impl i32>::MAX);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }

    bb1: {
        _0 = verified::inc(copy _1) -> [return: bb3, unwind continue];
    }

    bb2: {
        _0 = copy _1;
        goto -> bb3;
    }

    bb3: {
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "caller").expect("MIR summary");

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "verified::inc".to_string(),
                args: vec!["x".to_string()],
                guards: vec!["x < i32::MAX".to_string()],
                trust_callee: None,
            }]
        );
    }

    #[test]
    fn normalizes_mir_return_after_runtime_precondition() {
        let mir = r#"
fn add_one(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let _2: ();
    let mut _3: bool;
    let mut _4: &str;
    let mut _5: &str;
    let mut _6: (i32, bool);
    scope 1 {
        debug y => _0;
    }

    bb0: {
        _3 = Lt(copy _1, const core::num::<impl i32>::MAX);
        _4 = const "add_one";
        _5 = const "x < i32::MAX";
        _2 = assert_precondition(move _3, move _4, move _5) -> [return: bb1, unwind continue];
    }

    bb1: {
        _6 = AddWithOverflow(copy _1, const 1_i32);
        assert(!move (_6.1: bool), "overflow", copy _1, const 1_i32) -> [success: bb2, unwind continue];
    }

    bb2: {
        _0 = move (_6.0: i32);
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "add_one").expect("MIR summary");

        assert_eq!(
            summary.normalized_return_expression(),
            Some("x + 1".to_string())
        );
        assert_eq!(summary.semantic_calls(), Vec::new());
    }

    #[test]
    fn ignores_runtime_precondition_condition_calls() {
        let mir = r#"
fn zero_for_alias_option(_1: Option<i32>) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let _2: ();
    let mut _3: bool;
    let mut _4: &std::option::Option<i32>;
    let mut _5: &std::option::Option<i32>;
    let mut _6: std::option::Option<i32>;
    let mut _7: &str;
    let mut _8: &str;

    bb0: {
        _4 = &_1;
        _6 = Option::<i32>::None;
        _5 = &_6;
        _3 = <Option<i32> as PartialEq>::eq(move _4, move _5) -> [return: bb1, unwind continue];
    }

    bb1: {
        _7 = const "zero_for_alias_option";
        _8 = const "x == None";
        _2 = assert_precondition(move _3, move _7, move _8) -> [return: bb2, unwind continue];
    }

    bb2: {
        _0 = const 0_i32;
        return;
    }
}
"#;

        let summary =
            extract_mir_function_summary(mir, "zero_for_alias_option").expect("MIR summary");

        assert_eq!(summary.semantic_calls(), Vec::new());
    }

    #[test]
    fn ignores_assignments_after_mir_function_end() {
        let mir = r#"
fn add_one(_1: i32) -> i32 {
    debug x => _1;
    let mut _0: i32;
    let mut _2: (i32, bool);

    bb0: {
        _2 = AddWithOverflow(copy _1, const 1_i32);
        _0 = move (_2.0: i32);
        return;
    }
}

const __TRUST_META_add_one: &str = {
    let mut _0: &str;

    bb0: {
        _0 = const "{\"function_source\":\"pub fn add_one(x: i32) -> i32 { x + 1 }\"}";
        return;
    }
}

fn unrelated() -> () {
    let mut _0: ();
    let mut _2: (i32, bool);

    bb0: {
        _2 = StaticTestFn(move _0);
        _0 = const ();
        return;
    }
}
"#;

        let summary = extract_mir_function_summary(mir, "add_one").expect("MIR summary");

        assert_eq!(summary.assignments.len(), 2);
        assert_eq!(
            summary.normalized_return_expression(),
            Some("x + 1".to_string())
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
