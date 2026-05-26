use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::str::FromStr;
use z3::{
    ast::{Bool, Int},
    Config, SatResult, Solver,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverBackend {
    Internal,
    Z3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationOptions {
    pub solver: SolverBackend,
    pub timeout_ms: u64,
    pub target_pointer_width: Option<u32>,
}

impl VerificationOptions {
    pub fn z3(timeout_ms: u64) -> Self {
        Self {
            solver: SolverBackend::Z3,
            timeout_ms,
            target_pointer_width: None,
        }
    }

    pub fn with_target_pointer_width(mut self, pointer_width: u32) -> Self {
        self.target_pointer_width = Some(pointer_width);
        self
    }
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            solver: SolverBackend::Internal,
            timeout_ms: 5000,
            target_pointer_width: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofResult {
    Proved,
    Unproved,
    Unsupported,
}

pub fn prove_addition_overflow_safety(
    variable: &str,
    ty: &str,
    constant: i128,
    contracts: &[String],
    params: &[(String, String)],
    target_pointer_width: Option<u32>,
    timeout_ms: u64,
) -> ProofResult {
    if constant < 0 {
        return ProofResult::Unsupported;
    }
    let Some(max) = max_value(ty, target_pointer_width) else {
        return ProofResult::Unsupported;
    };
    let Some(required_bound) = max.checked_sub(constant) else {
        return ProofResult::Unproved;
    };

    prove_integer_implication(
        contracts,
        &Predicate {
            left: Expr::Var(variable.to_string()),
            op: CmpOp::Le,
            right: Expr::Const(required_bound),
        },
        params,
        target_pointer_width,
        timeout_ms,
    )
}

pub fn prove_integer_predicate(
    assumptions: &[String],
    conclusion: &str,
    params: &[(String, String)],
    target_pointer_width: Option<u32>,
    timeout_ms: u64,
) -> ProofResult {
    let Some(conclusion) = parse_predicate(conclusion, target_pointer_width) else {
        return ProofResult::Unsupported;
    };

    prove_integer_implication(
        assumptions,
        &conclusion,
        params,
        target_pointer_width,
        timeout_ms,
    )
}

fn prove_integer_implication(
    assumptions: &[String],
    conclusion: &Predicate,
    params: &[(String, String)],
    target_pointer_width: Option<u32>,
    timeout_ms: u64,
) -> ProofResult {
    let parsed_assumptions = assumptions
        .iter()
        .filter_map(|assumption| parse_predicate(assumption, target_pointer_width))
        .collect::<Vec<_>>();
    if predicate_uses_unknown_variable(conclusion, params)
        || parsed_assumptions
            .iter()
            .any(|assumption| predicate_uses_unknown_variable(assumption, params))
    {
        return ProofResult::Unsupported;
    }

    let mut cfg = Config::new();
    cfg.set_bool_param_value("trace", false);
    cfg.set_timeout_msec(timeout_ms);

    z3::with_z3_config(&cfg, || {
        let solver = Solver::new_for_logic("QF_LIA").unwrap_or_else(Solver::new);
        let mut env = Z3Env::new(params, target_pointer_width);
        for assertion in env.domain_assertions() {
            solver.assert(&assertion);
        }
        for assumption in parsed_assumptions
            .iter()
            .filter_map(|predicate| predicate.to_z3(&mut env))
        {
            solver.assert(&assumption);
        }

        let Some(z3_conclusion) = conclusion.to_z3(&mut env) else {
            return ProofResult::Unsupported;
        };
        solver.assert(&z3_conclusion.not());
        dump_smt_if_requested(&solver, assumptions, conclusion, params);
        match solver.check() {
            SatResult::Unsat => ProofResult::Proved,
            SatResult::Sat | SatResult::Unknown => ProofResult::Unproved,
        }
    })
}

fn predicate_uses_unknown_variable(predicate: &Predicate, params: &[(String, String)]) -> bool {
    referenced_variables(predicate)
        .into_iter()
        .any(|variable| !params.iter().any(|(name, _ty)| name == &variable))
}

fn dump_smt_if_requested(
    solver: &Solver,
    assumptions: &[String],
    conclusion: &Predicate,
    params: &[(String, String)],
) {
    let Some(dump_dir) = env::var_os("TRUST_SMT_DUMP_DIR").map(PathBuf::from) else {
        return;
    };

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    "trust-smt-dump-v1".hash(&mut hasher);
    assumptions.hash(&mut hasher);
    conclusion.hash(&mut hasher);
    params.hash(&mut hasher);
    let smt = format!("{}\n(check-sat)\n", solver.to_smt2());
    smt.hash(&mut hasher);

    if fs::create_dir_all(&dump_dir).is_err() {
        return;
    }
    let path = dump_dir.join(format!("vc-{:016x}.smt2", hasher.finish()));
    let _ = fs::write(path, smt);
}

struct Z3Env {
    declarations: HashMap<String, Int>,
    integer_params: Vec<(String, String)>,
    target_pointer_width: Option<u32>,
}

impl Z3Env {
    fn new(params: &[(String, String)], target_pointer_width: Option<u32>) -> Self {
        Self {
            declarations: HashMap::new(),
            integer_params: params
                .iter()
                .filter(|(_name, ty)| is_supported_integer(ty))
                .cloned()
                .collect(),
            target_pointer_width,
        }
    }

    fn int_var(&mut self, name: &str) -> Int {
        if let Some(existing) = self.declarations.get(name) {
            return existing.clone();
        }
        let variable = Int::new_const(symbol_name(name));
        self.declarations.insert(name.to_string(), variable.clone());
        variable
    }

    fn domain_assertions(&mut self) -> Vec<Bool> {
        let mut assertions = Vec::new();
        for (name, ty) in self.integer_params.clone() {
            let variable = self.int_var(&name);
            if let Some(min) = min_value(&ty).and_then(int_literal) {
                assertions.push(variable.ge(&min));
            }
            if let Some(max) = max_value(&ty, self.target_pointer_width).and_then(int_literal) {
                assertions.push(variable.le(&max));
            }
        }
        assertions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Predicate {
    left: Expr,
    op: CmpOp,
    right: Expr,
}

impl Predicate {
    fn to_z3(&self, env: &mut Z3Env) -> Option<Bool> {
        let left = self.left.to_z3(env)?;
        let right = self.right.to_z3(env)?;
        Some(match self.op {
            CmpOp::Lt => left.lt(&right),
            CmpOp::Le => left.le(&right),
            CmpOp::Gt => left.gt(&right),
            CmpOp::Ge => left.ge(&right),
            CmpOp::Eq => left.eq(&right),
            CmpOp::Ne => left.ne(&right),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Expr {
    Var(String),
    Const(i128),
}

impl Expr {
    fn to_z3(&self, env: &mut Z3Env) -> Option<Int> {
        match self {
            Expr::Var(name) => Some(env.int_var(name)),
            Expr::Const(value) => int_literal(*value),
        }
    }
}

fn parse_predicate(input: &str, target_pointer_width: Option<u32>) -> Option<Predicate> {
    for (needle, op) in [
        ("<=", CmpOp::Le),
        (">=", CmpOp::Ge),
        ("!=", CmpOp::Ne),
        ("==", CmpOp::Eq),
        ("<", CmpOp::Lt),
        (">", CmpOp::Gt),
    ] {
        let Some((left, right)) = input.split_once(needle) else {
            continue;
        };
        return Some(Predicate {
            left: parse_expr(left, target_pointer_width)?,
            op,
            right: parse_expr(right, target_pointer_width)?,
        });
    }

    None
}

fn parse_expr(input: &str, target_pointer_width: Option<u32>) -> Option<Expr> {
    let input = input.trim();
    if let Some(value) = parse_const_expr(input, target_pointer_width) {
        return Some(Expr::Const(value));
    }
    if input
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
    {
        return Some(Expr::Var(input.to_string()));
    }

    None
}

fn parse_const_expr(input: &str, target_pointer_width: Option<u32>) -> Option<i128> {
    if let Ok(value) = input.parse::<i128>() {
        return Some(value);
    }
    if let Some(value) = rust_integer_bound(input, target_pointer_width) {
        return Some(value);
    }

    for op in ['+', '-', '/'] {
        for idx in operator_indices(input, op).into_iter().rev() {
            let left = parse_const_expr(&input[..idx], target_pointer_width)?;
            let right = parse_const_expr(&input[idx + 1..], target_pointer_width)?;
            return match op {
                '+' => left.checked_add(right),
                '-' => left.checked_sub(right),
                '/' if right != 0 => Some(left / right),
                '/' => None,
                _ => None,
            };
        }
    }

    None
}

fn operator_indices(input: &str, op: char) -> Vec<usize> {
    input
        .char_indices()
        .filter_map(|(idx, ch)| {
            if ch != op || idx == 0 {
                return None;
            }
            if op == '-' && input.as_bytes().get(idx - 1) == Some(&b':') {
                return None;
            }
            Some(idx)
        })
        .collect()
}

fn int_literal(value: i128) -> Option<Int> {
    Int::from_str(&value.to_string()).ok()
}

fn symbol_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "trust_var".to_string()
    } else {
        out
    }
}

fn is_supported_integer(ty: &str) -> bool {
    matches!(ty, "i32" | "i64" | "usize")
}

fn max_value(ty: &str, target_pointer_width: Option<u32>) -> Option<i128> {
    match ty {
        "i32" => Some(i32::MAX as i128),
        "i64" => Some(i64::MAX as i128),
        "usize" => Some(usize_max_value(target_pointer_width)),
        _ => None,
    }
}

fn min_value(ty: &str) -> Option<i128> {
    match ty {
        "i32" => Some(i32::MIN as i128),
        "i64" => Some(i64::MIN as i128),
        "usize" => Some(0),
        _ => None,
    }
}

fn rust_integer_bound(input: &str, target_pointer_width: Option<u32>) -> Option<i128> {
    match input {
        "i32::MAX" => Some(i32::MAX as i128),
        "i32::MIN" => Some(i32::MIN as i128),
        "i64::MAX" => Some(i64::MAX as i128),
        "i64::MIN" => Some(i64::MIN as i128),
        "usize::MAX" => Some(usize_max_value(target_pointer_width)),
        "usize::MIN" => Some(0),
        _ => None,
    }
}

fn usize_max_value(target_pointer_width: Option<u32>) -> i128 {
    match target_pointer_width {
        Some(16) => u16::MAX as i128,
        Some(32) => u32::MAX as i128,
        Some(64) => u64::MAX as i128,
        _ => usize::MAX as i128,
    }
}

fn referenced_variables(predicate: &Predicate) -> std::collections::BTreeSet<String> {
    let mut variables = std::collections::BTreeSet::new();
    for expression in [&predicate.left, &predicate.right] {
        if let Expr::Var(name) = expression {
            variables.insert(name.clone());
        }
    }
    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i32_param(name: &str) -> Vec<(String, String)> {
        vec![(name.to_string(), "i32".to_string())]
    }

    #[test]
    fn parses_rust_bounds_as_constants() {
        assert_eq!(parse_const_expr("i32::MAX-1", None), Some(2147483646));
        assert_eq!(parse_const_expr("i32::MIN+1", None), Some(-2147483647));
        assert_eq!(
            parse_const_expr("i64::MAX/2", None),
            Some(i64::MAX as i128 / 2)
        );
    }

    #[test]
    fn parses_usize_bounds_for_target_pointer_width() {
        assert_eq!(
            parse_const_expr("usize::MAX", Some(32)),
            Some(u32::MAX as i128)
        );
        assert_eq!(
            parse_const_expr("usize::MAX/2", Some(16)),
            Some(u16::MAX as i128 / 2)
        );
    }

    #[test]
    fn parses_simple_predicate_variables() {
        let predicate = parse_predicate("x<2147483647", None).unwrap();

        assert_eq!(
            referenced_variables(&predicate),
            std::collections::BTreeSet::from(["x".to_string()])
        );
    }

    #[test]
    fn z3_proves_add_one_bound_from_strict_numeric_max() {
        let contracts = vec!["x<2147483647".to_string()];

        assert_eq!(
            prove_addition_overflow_safety("x", "i32", 1, &contracts, &i32_param("x"), None, 5000),
            ProofResult::Proved
        );
    }

    #[test]
    fn z3_rejects_unbounded_add_one() {
        assert_eq!(
            prove_addition_overflow_safety("x", "i32", 1, &[], &i32_param("x"), None, 5000),
            ProofResult::Unproved
        );
    }

    #[test]
    fn z3_rejects_unknown_variables_in_conclusion() {
        assert_eq!(
            prove_integer_predicate(&[], "y>0", &i32_param("x"), None, 5000),
            ProofResult::Unsupported
        );
    }

    #[test]
    fn z3_rejects_unknown_variables_in_assumptions() {
        let contracts = vec!["y==x".to_string()];

        assert_eq!(
            prove_integer_predicate(&contracts, "x==x", &i32_param("x"), None, 5000),
            ProofResult::Unsupported
        );
    }

    #[test]
    fn z3_proves_nonzero_from_positive_lower_bound() {
        let contracts = vec!["y>=1".to_string()];

        assert_eq!(
            prove_integer_predicate(&contracts, "y!=0", &i32_param("y"), None, 5000),
            ProofResult::Proved
        );
    }
}
