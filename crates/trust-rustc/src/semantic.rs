use std::collections::hash_map::DefaultHasher;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use trust_core::{
    metadata::TrustMetadata,
    verifier::{
        SemanticArithmeticKind, SemanticArithmeticOperation, SemanticCall, SemanticParam,
        SemanticSliceIndex, TrustFunctionSemantics,
    },
};

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
    args: Vec<MirArg>,
    return_type: String,
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
struct MirDebugLocal {
    name: String,
    place: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirAssignment {
    block: Option<String>,
    place: String,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirTerminator {
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSwitchTarget {
    value: String,
    block: String,
}

pub(crate) fn maybe_extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
) -> Result<Vec<TrustFunctionSemantics>, String> {
    let dump_dir = env::var_os("TRUST_SEMANTIC_DUMP_DIR").map(PathBuf::from);
    if dump_dir.is_none() && !semantic_verify_enabled() {
        return Ok(Vec::new());
    }

    extract_semantic_views(rustc, rustc_args, metadata, dump_dir)
}

fn semantic_verify_enabled() -> bool {
    env::var("TRUST_SEMANTIC_VERIFY").as_deref() == Ok("1")
}

fn extract_semantic_views(
    rustc: &OsString,
    rustc_args: &[OsString],
    metadata: &[TrustMetadata],
    dump_dir: Option<PathBuf>,
) -> Result<Vec<TrustFunctionSemantics>, String> {
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

    Ok(verifier_semantics(&item_matches))
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

fn verifier_semantics(item_matches: &[SemanticItemMatch]) -> Vec<TrustFunctionSemantics> {
    item_matches
        .iter()
        .filter(|item| item.item_kind == "total" && item.hir_match)
        .filter_map(|item| {
            let mir_function = item.mir_function.as_ref()?;
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
                return_expression: mir_function.normalized_return_expression(),
                arithmetic_operations: mir_function.semantic_arithmetic_operations(),
                slice_indexes: mir_function.semantic_slice_indexes(),
                calls: mir_function.semantic_calls(),
            })
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
        .position(|line| mir_signature_matches_leaf(line.trim(), name))?;
    let signature = parse_mir_signature(lines[signature_idx].trim())?;
    let function_end = mir_function_end(&lines, signature_idx).unwrap_or(lines.len());
    let function_lines = &lines[signature_idx + 1..function_end];

    Some(MirFunctionSummary {
        args: signature.args,
        return_type: signature.return_type,
        debug_locals: extract_mir_debug_locals(function_lines),
        assignments: extract_mir_assignments(function_lines),
        terminators: extract_mir_terminators(function_lines),
        return_expr: extract_mir_return_expr(function_lines),
    })
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

    fn normalized_return_expression(&self) -> Option<String> {
        self.return_expr
            .as_deref()
            .and_then(|expr| self.normalized_mir_expression(expr))
    }

    fn normalized_mir_expression(&self, expr: &str) -> Option<String> {
        self.normalized_mir_expression_with_depth(expr, 0)
    }

    fn normalized_mir_expression_with_depth(&self, expr: &str, depth: usize) -> Option<String> {
        if depth > 8 {
            return None;
        }
        let expr = strip_mir_move_or_copy(expr.trim());
        if let Some(assignment) = self.assignment_for_place(expr) {
            if let Some(normalized) =
                self.normalized_mir_expression_with_depth(&assignment.expression, depth + 1)
            {
                return Some(normalized);
            }
        }
        if let Some(source_name) = self.local_name_for_place(expr) {
            return Some(source_name.to_string());
        }
        if let Some(constant) = mir_const_value(expr) {
            return Some(constant);
        }
        if let Some(operation) = self.normalized_mir_operation(expr, depth + 1) {
            return Some(operation);
        }
        if let Some((base, index)) = mir_slice_index(expr) {
            return Some(format!(
                "{}[{}]",
                self.normalized_mir_expression_with_depth(base, depth + 1)?,
                self.normalized_mir_expression_with_depth(index, depth + 1)?
            ));
        }
        if let Some((place, field, _ty)) = mir_projection(expr) {
            if field == "0" {
                let assignment = self.assignment_for_place(place)?;
                return self.normalized_mir_operation(&assignment.expression, depth + 1);
            }
        }

        None
    }

    fn normalized_mir_operation(&self, expr: &str, depth: usize) -> Option<String> {
        let (op, args) = expr.split_once('(')?;
        let args = args.strip_suffix(')')?;
        let args = parse_mir_call_args(args);
        match (op.trim(), args.as_slice()) {
            ("AddWithOverflow", [left, right]) => Some(format!(
                "{} + {}",
                self.normalized_mir_expression_with_depth(left, depth + 1)?,
                self.normalized_mir_expression_with_depth(right, depth + 1)?
            )),
            ("SubWithOverflow", [left, right]) => Some(format!(
                "{} - {}",
                self.normalized_mir_expression_with_depth(left, depth + 1)?,
                self.normalized_mir_expression_with_depth(right, depth + 1)?
            )),
            ("MulWithOverflow", [left, right]) => Some(format!(
                "{} * {}",
                self.normalized_mir_expression_with_depth(left, depth + 1)?,
                self.normalized_mir_expression_with_depth(right, depth + 1)?
            )),
            ("Div", [left, right]) => Some(format!(
                "{} / {}",
                self.normalized_mir_expression_with_depth(left, depth + 1)?,
                self.normalized_mir_expression_with_depth(right, depth + 1)?
            )),
            ("Rem", [left, right]) => Some(format!(
                "{} % {}",
                self.normalized_mir_expression_with_depth(left, depth + 1)?,
                self.normalized_mir_expression_with_depth(right, depth + 1)?
            )),
            ("NegWithOverflow", [value]) => Some(format!(
                "-{}",
                self.normalized_mir_expression_with_depth(value, depth + 1)?
            )),
            _ => None,
        }
    }

    fn normalized_mir_predicate(&self, expr: &str) -> Option<String> {
        self.normalized_mir_predicate_with_depth(expr, 0)
    }

    fn normalized_mir_predicate_with_depth(&self, expr: &str, depth: usize) -> Option<String> {
        if depth > 8 {
            return None;
        }
        let expr = strip_mir_move_or_copy(expr.trim());
        if let Some(assignment) = self.assignment_for_place(expr) {
            return self.normalized_mir_predicate_with_depth(&assignment.expression, depth + 1);
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
            self.normalized_mir_expression_with_depth(left, depth + 1)?,
            self.normalized_mir_expression_with_depth(right, depth + 1)?
        ))
    }

    fn guards_for_block(&self, block: &str) -> Vec<String> {
        self.terminators
            .iter()
            .filter_map(|terminator| {
                let (condition, targets) = mir_switch(&terminator.expression)?;
                let predicate = self.normalized_mir_predicate(condition)?;
                Some(
                    targets
                        .into_iter()
                        .filter_map(|target| {
                            if target.block != block {
                                return None;
                            }
                            match target.value.as_str() {
                                "otherwise" | "1" | "true" => Some(predicate.clone()),
                                "0" | "false" => negate_predicate(&predicate),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn assignment_for_place(&self, place: &str) -> Option<&MirAssignment> {
        self.assignments
            .iter()
            .rev()
            .find(|assignment| assignment.place == place)
    }

    fn semantic_arithmetic_operations(&self) -> Vec<SemanticArithmeticOperation> {
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
                        let left = self.normalized_mir_expression(left)?;
                        let right = self.normalized_mir_expression(right)?;
                        let operator = semantic_arithmetic_operator(kind);
                        Some(SemanticArithmeticOperation {
                            kind,
                            expression: format!("{left} {operator} {right}"),
                            left,
                            right: Some(right),
                            guards: assignment
                                .block
                                .as_deref()
                                .map(|block| self.guards_for_block(block))
                                .unwrap_or_default(),
                        })
                    }
                    (SemanticArithmeticKind::Neg, [value]) => {
                        let value = self.normalized_mir_expression(value)?;
                        Some(SemanticArithmeticOperation {
                            kind,
                            expression: format!("-{value}"),
                            left: value,
                            right: None,
                            guards: assignment
                                .block
                                .as_deref()
                                .map(|block| self.guards_for_block(block))
                                .unwrap_or_default(),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn semantic_slice_indexes(&self) -> Vec<SemanticSliceIndex> {
        self.assignments
            .iter()
            .filter_map(|assignment| {
                let expr = strip_mir_move_or_copy(&assignment.expression);
                let (base, index) = mir_slice_index(expr)?;
                let base = self.normalized_mir_expression(base)?;
                let index = self.normalized_mir_expression(index)?;
                Some(SemanticSliceIndex {
                    expression: format!("{base}[{index}]"),
                    base,
                    index,
                })
            })
            .collect()
    }

    fn semantic_calls(&self) -> Vec<SemanticCall> {
        self.assignments
            .iter()
            .filter_map(|assignment| {
                let (callee, args) = mir_call(&assignment.expression)?;
                Some(SemanticCall {
                    callee: callee.to_string(),
                    args: args
                        .iter()
                        .map(|arg| self.normalized_mir_expression(arg))
                        .collect::<Option<Vec<_>>>()?,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirSignature {
    args: Vec<MirArg>,
    return_type: String,
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
    let (_name, args) = args.split_once('(')?;
    let return_type = rest
        .trim()
        .strip_suffix('{')
        .unwrap_or(rest)
        .trim()
        .to_string();
    Some(MirSignature {
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

fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
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

    for line in lines {
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
            });
        }
    }

    assignments
}

fn extract_mir_terminators(lines: &[&str]) -> Vec<MirTerminator> {
    let mut terminators = Vec::new();

    for line in lines {
        let line = line.trim();
        if mir_block_header(line).is_some() {
            continue;
        }
        if line.starts_with("switchInt(") {
            terminators.push(MirTerminator {
                expression: line.trim_end_matches(';').trim().to_string(),
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
    lines.iter().find_map(|line| {
        line.trim()
            .strip_prefix("_0 = ")
            .and_then(|line| line.strip_suffix(';'))
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string)
    })
}

fn strip_mir_move_or_copy(expr: &str) -> &str {
    expr.strip_prefix("copy ")
        .or_else(|| expr.strip_prefix("move "))
        .unwrap_or(expr)
        .trim()
}

fn mir_const_value(expr: &str) -> Option<String> {
    let value = expr.strip_prefix("const ")?;
    if let Some(bound) = mir_integer_bound(value) {
        return Some(bound);
    }
    let value = value
        .split_once('_')
        .map(|(value, _ty)| value)
        .unwrap_or(value);
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        Some(value.to_string())
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

fn mir_projection(expr: &str) -> Option<(&str, &str, &str)> {
    let expr = expr.strip_prefix('(')?.strip_suffix(')')?;
    let (projection, ty) = expr.split_once(':')?;
    let (place, field) = projection.trim().split_once('.')?;
    Some((place.trim(), field.trim(), ty.trim()))
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

fn mir_call(expr: &str) -> Option<(&str, Vec<String>)> {
    let (call, _target) = expr.split_once(" -> ")?;
    let (callee, args) = call.split_once('(')?;
    let args = args.strip_suffix(')')?;
    Some((callee.trim(), parse_mir_call_args(args)))
}

fn parse_mir_call_args(input: &str) -> Vec<String> {
    parse_comma_separated(input)
}

fn mir_checked_arithmetic_operation(expr: &str) -> Option<(SemanticArithmeticKind, Vec<String>)> {
    let (op, args) = expr.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let kind = match op.trim() {
        "AddWithOverflow" => SemanticArithmeticKind::Add,
        "SubWithOverflow" => SemanticArithmeticKind::Sub,
        "MulWithOverflow" => SemanticArithmeticKind::Mul,
        "NegWithOverflow" => SemanticArithmeticKind::Neg,
        "Div" => SemanticArithmeticKind::Div,
        "Rem" => SemanticArithmeticKind::Rem,
        _ => return None,
    };
    Some((kind, parse_mir_call_args(args)))
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

fn mir_switch(expr: &str) -> Option<(&str, Vec<MirSwitchTarget>)> {
    let rest = expr.strip_prefix("switchInt(")?;
    let (condition, targets) = rest.split_once(") -> [")?;
    let targets = targets.strip_suffix(']')?;
    Some((condition.trim(), parse_mir_switch_targets(targets)))
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
            let return_expr = mir_function
                .normalized_return_expression()
                .or_else(|| mir_function.return_expr.clone())
                .unwrap_or_else(|| "none".to_string());
            let arithmetic_ops = mir_function
                .semantic_arithmetic_operations()
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
                .semantic_slice_indexes()
                .iter()
                .map(|index| index.expression.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let calls = mir_function
                .semantic_calls()
                .iter()
                .map(|call| format!("{}({})", call.callee, call.args.join(",")))
                .collect::<Vec<_>>()
                .join(",");
            summary.push_str(&format!(
                "mir_function path={} args={} return_type={} debug_locals={} return_expr={} arithmetic_ops={} slice_indexes={} calls={}\n",
                item.rust_function_path,
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
                return_expr,
                arithmetic_ops,
                slice_indexes,
                calls,
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
                args: vec![MirArg {
                    place: "_1".to_string(),
                    ty: "i32".to_string(),
                }],
                return_type: "i32".to_string(),
                debug_locals: vec![MirDebugLocal {
                    name: "x".to_string(),
                    place: "_1".to_string(),
                }],
                assignments: vec![MirAssignment {
                    block: Some("bb0".to_string()),
                    place: "_0".to_string(),
                    expression: "copy _1".to_string(),
                }],
                terminators: Vec::new(),
                return_expr: Some("copy _1".to_string()),
            })
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
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: Vec::new(),
            }]
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
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: Vec::new(),
            }]
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
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: vec!["x < i32::MAX".to_string()],
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
                    left: "x".to_string(),
                    right: Some("y".to_string()),
                    expression: "x % y".to_string(),
                    guards: Vec::new(),
                },
                SemanticArithmeticOperation {
                    kind: SemanticArithmeticKind::Div,
                    left: "x".to_string(),
                    right: Some("y".to_string()),
                    expression: "x / y".to_string(),
                    guards: Vec::new(),
                },
            ]
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
                index: "i".to_string(),
                expression: "xs[i]".to_string(),
            }]
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

        assert_eq!(
            summary.semantic_calls(),
            vec![SemanticCall {
                callee: "verified::inc".to_string(),
                args: vec!["x".to_string()],
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
