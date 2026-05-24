use crate::metadata::TrustMetadata;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    IntegerAdditionOverflow {
        function: String,
        expression: String,
    },
    IntegerSubtractionOverflow {
        function: String,
        expression: String,
    },
    IntegerNegationOverflow {
        function: String,
        expression: String,
    },
    IntegerMultiplicationOverflow {
        function: String,
        expression: String,
    },
    PostconditionUnproved {
        function: String,
        condition: String,
    },
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationError::IntegerAdditionOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer addition cannot overflow in `{function}`: `{expression}`"
            ),
            VerificationError::IntegerSubtractionOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer subtraction cannot overflow in `{function}`: `{expression}`"
            ),
            VerificationError::IntegerNegationOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer negation cannot overflow in `{function}`: `{expression}`"
            ),
            VerificationError::IntegerMultiplicationOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer multiplication cannot overflow in `{function}`: `{expression}`"
            ),
            VerificationError::PostconditionUnproved {
                function,
                condition,
            } => write!(f, "could not prove postcondition in `{function}`: `{condition}`"),
        }
    }
}

impl std::error::Error for VerificationError {}

pub fn verify_total(metadata: &TrustMetadata) -> Result<(), VerificationError> {
    if metadata.item_kind != "total" {
        return Ok(());
    }

    let source = normalize(&metadata.function_source);
    let contracts = executable_preconditions(metadata);
    let params = parse_params(&source);
    let body = body(&source);

    for obligation in addition_obligations(body, &params) {
        if !addition_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::IntegerAdditionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in subtraction_obligations(body, &params) {
        if !subtraction_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::IntegerSubtractionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in negation_obligations(body, &params) {
        if !negation_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::IntegerNegationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in multiplication_obligations(body, &params) {
        if !multiplication_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::IntegerMultiplicationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for postcondition in postconditions(metadata) {
        if !postcondition_proved(&postcondition, body) {
            return Err(VerificationError::PostconditionUnproved {
                function: metadata.rust_function_path.clone(),
                condition: postcondition.original,
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Contract {
    original: String,
    normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Param {
    name: String,
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddObligation {
    variable: String,
    ty: String,
    constant: i128,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubObligation {
    variable: String,
    ty: String,
    constant: i128,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NegObligation {
    variable: String,
    ty: String,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MulObligation {
    variable: String,
    ty: String,
    constant: i128,
    expression: String,
}

fn executable_preconditions(metadata: &TrustMetadata) -> Vec<String> {
    metadata
        .contracts_original
        .iter()
        .zip(metadata.contract_classes.iter())
        .filter(|(_contract, class)| class.as_str() == "given executable")
        .map(|(contract, _class)| normalize(contract))
        .collect()
}

fn postconditions(metadata: &TrustMetadata) -> Vec<Contract> {
    metadata
        .contracts_original
        .iter()
        .zip(metadata.contract_classes.iter())
        .filter(|(_contract, class)| matches!(class.as_str(), "gives executable" | "gives ghost"))
        .map(|(contract, _class)| Contract {
            original: contract.clone(),
            normalized: normalize(&remove_int_wrappers(contract)),
        })
        .collect()
}

fn parse_params(source: &str) -> Vec<Param> {
    let Some(params_start) = source.find('(') else {
        return Vec::new();
    };
    let Some(params_end) = source[params_start + 1..].find(')') else {
        return Vec::new();
    };
    let params = &source[params_start + 1..params_start + 1 + params_end];

    params
        .split(',')
        .filter_map(|param| {
            let (name, ty) = param.split_once(':')?;
            Some(Param {
                name: name.trim().to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect()
}

fn body(source: &str) -> &str {
    let Some(start) = source.find('{') else {
        return "";
    };
    let Some(end) = source.rfind('}') else {
        return "";
    };
    &source[start + 1..end]
}

fn return_expression(body: &str) -> String {
    body.trim()
        .trim_start_matches("return")
        .trim_end_matches(';')
        .to_string()
}

fn postcondition_proved(postcondition: &Contract, body: &str) -> bool {
    let return_expression = return_expression(body);
    let Some((left, right)) = postcondition.normalized.split_once("==") else {
        return false;
    };

    (left == "out" && right == return_expression) || (right == "out" && left == return_expression)
}

fn addition_obligations(body: &str, params: &[Param]) -> Vec<AddObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();

    for window in tokens.windows(3) {
        let [left, op, right] = window else {
            continue;
        };
        if op != "+" {
            continue;
        }

        if let Some(param) = params.iter().find(|param| param.name == *left) {
            if let Ok(constant) = right.parse::<i128>() {
                if is_supported_integer(&param.ty) {
                    obligations.push(AddObligation {
                        variable: left.clone(),
                        ty: param.ty.clone(),
                        constant,
                        expression: format!("{left} + {right}"),
                    });
                }
            }
        }
    }

    obligations
}

fn subtraction_obligations(body: &str, params: &[Param]) -> Vec<SubObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();

    for window in tokens.windows(3) {
        let [left, op, right] = window else {
            continue;
        };
        if op != "-" {
            continue;
        }

        if let Some(param) = params.iter().find(|param| param.name == *left) {
            if let Ok(constant) = right.parse::<i128>() {
                if constant > 0 && is_supported_integer(&param.ty) {
                    obligations.push(SubObligation {
                        variable: left.clone(),
                        ty: param.ty.clone(),
                        constant,
                        expression: format!("{left} - {right}"),
                    });
                }
            }
        }
    }

    obligations
}

fn negation_obligations(body: &str, params: &[Param]) -> Vec<NegObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if token != "-" || idx + 1 >= tokens.len() {
            continue;
        }

        let variable = &tokens[idx + 1];
        let Some(param) = params.iter().find(|param| param.name == *variable) else {
            continue;
        };
        if !is_signed_integer(&param.ty) || !looks_unary_minus(&tokens, idx) {
            continue;
        }

        obligations.push(NegObligation {
            variable: variable.clone(),
            ty: param.ty.clone(),
            expression: format!("-{variable}"),
        });
    }

    obligations
}

fn multiplication_obligations(body: &str, params: &[Param]) -> Vec<MulObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();

    for window in tokens.windows(3) {
        let [left, op, right] = window else {
            continue;
        };
        if op != "*" {
            continue;
        }

        if let Some(param) = params.iter().find(|param| param.name == *left) {
            if let Ok(constant) = right.parse::<i128>() {
                if constant > 1 && is_supported_integer(&param.ty) {
                    obligations.push(MulObligation {
                        variable: left.clone(),
                        ty: param.ty.clone(),
                        constant,
                        expression: format!("{left} * {right}"),
                    });
                }
            }
        }
    }

    obligations
}

fn addition_obligation_proved(obligation: &AddObligation, contracts: &[String]) -> bool {
    let Some(max) = max_value(&obligation.ty) else {
        return false;
    };
    let required_bound = max - obligation.constant;
    let lt_exact = format!("{}<{}::MAX", obligation.variable, obligation.ty);
    let le_required = format!(
        "{}<={}",
        obligation.variable,
        constant_with_type(required_bound, &obligation.ty)
    );
    let le_unqualified = format!("{}<={required_bound}", obligation.variable);

    if obligation.constant == 1 && contracts.iter().any(|contract| contract == &lt_exact) {
        return true;
    }

    contracts
        .iter()
        .any(|contract| contract == &le_required || contract == &le_unqualified)
}

fn subtraction_obligation_proved(obligation: &SubObligation, contracts: &[String]) -> bool {
    let Some(min) = min_value(&obligation.ty) else {
        return false;
    };
    let required_bound = min + obligation.constant;
    let gt_min = format!("{}>{}::MIN", obligation.variable, obligation.ty);
    let ge_required = format!(
        "{}>={}",
        obligation.variable,
        min_bound_with_type(required_bound, &obligation.ty)
    );
    let ge_unqualified = format!("{}>={required_bound}", obligation.variable);

    if obligation.constant == 1 && contracts.iter().any(|contract| contract == &gt_min) {
        return true;
    }

    contracts
        .iter()
        .any(|contract| contract == &ge_required || contract == &ge_unqualified)
}

fn negation_obligation_proved(obligation: &NegObligation, contracts: &[String]) -> bool {
    let Some(min) = min_value(&obligation.ty) else {
        return false;
    };
    let required_bound = min + 1;
    let gt_min = format!("{}>{}::MIN", obligation.variable, obligation.ty);
    let ge_required = format!(
        "{}>={}",
        obligation.variable,
        min_bound_with_type(required_bound, &obligation.ty)
    );
    let ge_unqualified = format!("{}>={required_bound}", obligation.variable);

    contracts.iter().any(|contract| {
        contract == &gt_min || contract == &ge_required || contract == &ge_unqualified
    })
}

fn multiplication_obligation_proved(obligation: &MulObligation, contracts: &[String]) -> bool {
    let Some(max) = max_value(&obligation.ty) else {
        return false;
    };
    let upper_symbolic = format!(
        "{}<={}::MAX/{}",
        obligation.variable, obligation.ty, obligation.constant
    );
    let upper_numeric = format!("{}<={}", obligation.variable, max / obligation.constant);
    let upper_proved = contracts
        .iter()
        .any(|contract| contract == &upper_symbolic || contract == &upper_numeric);

    if obligation.ty == "usize" {
        return upper_proved;
    }

    let Some(min) = min_value(&obligation.ty) else {
        return false;
    };
    let lower_symbolic = format!(
        "{}>={}::MIN/{}",
        obligation.variable, obligation.ty, obligation.constant
    );
    let lower_numeric = format!("{}>={}", obligation.variable, min / obligation.constant);
    let lower_proved = contracts
        .iter()
        .any(|contract| contract == &lower_symbolic || contract == &lower_numeric);

    upper_proved && lower_proved
}

fn is_supported_integer(ty: &str) -> bool {
    matches!(ty, "i32" | "i64" | "usize")
}

fn is_signed_integer(ty: &str) -> bool {
    matches!(ty, "i32" | "i64")
}

fn max_value(ty: &str) -> Option<i128> {
    match ty {
        "i32" => Some(i32::MAX as i128),
        "i64" => Some(i64::MAX as i128),
        "usize" => Some(usize::MAX as i128),
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

fn constant_with_type(value: i128, ty: &str) -> String {
    match (value, ty) {
        (2147483646, "i32") => "i32::MAX-1".to_string(),
        (9223372036854775806, "i64") => "i64::MAX-1".to_string(),
        _ => value.to_string(),
    }
}

fn min_bound_with_type(value: i128, ty: &str) -> String {
    match (value, ty) {
        (-2147483647, "i32") => "i32::MIN+1".to_string(),
        (-9223372036854775807, "i64") => "i64::MIN+1".to_string(),
        _ => value.to_string(),
    }
}

fn looks_unary_minus(tokens: &[String], minus_idx: usize) -> bool {
    if minus_idx == 0 {
        return true;
    }

    let previous = &tokens[minus_idx - 1];
    matches!(
        previous.as_str(),
        "{" | "(" | "[" | "," | "return" | "=>" | "=" | "<" | ">" | "<=" | ">=" | "==" | "!="
    )
}

fn remove_int_wrappers(input: &str) -> String {
    let mut out = input.to_string();

    while let Some(start) = out.find("int(") {
        let inner_start = start + "int(".len();
        let Some(end) = matching_paren(&out, inner_start - 1) else {
            break;
        };
        let inner = out[inner_start..end].to_string();
        out.replace_range(start..=end, &inner);
    }

    out
}

fn matching_paren(input: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in input.char_indices().skip_while(|(idx, _)| *idx < open_idx) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }

    None
}

fn normalize(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn tokens(input: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(function_source: &str, contracts: &[&str]) -> TrustMetadata {
        metadata_named("add_one", function_source, contracts)
    }

    fn metadata_named(name: &str, function_source: &str, contracts: &[&str]) -> TrustMetadata {
        metadata_named_with_classes(
            name,
            function_source,
            contracts,
            &vec!["given executable"; contracts.len()],
        )
    }

    fn metadata_named_with_classes(
        name: &str,
        function_source: &str,
        contracts: &[&str],
        classes: &[&str],
    ) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            item_kind: "total".to_string(),
            item_id: "total:add_one:test".to_string(),
            rust_function_path: name.to_string(),
            contracts_original: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
            contract_classes: classes.iter().map(|class| class.to_string()).collect(),
            function_source: function_source.to_string(),
        }
    }

    #[test]
    fn proves_i32_add_one_from_executable_precondition() {
        let metadata = metadata("pub fn add_one(x: i32) -> i32 { x + 1 }", &["x < i32::MAX"]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_i32_add_one() {
        let metadata = metadata("pub fn add_one(x: i32) -> i32 { x + 1 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add_one".to_string(),
                expression: "x + 1".to_string(),
            })
        );
    }

    #[test]
    fn ignores_non_arithmetic_identity() {
        let metadata = metadata("pub fn id_i32(x: i32) -> i32 { x }", &[]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn proves_i32_sub_one_from_executable_precondition() {
        let metadata = metadata("pub fn sub_one(x: i32) -> i32 { x - 1 }", &["x > i32::MIN"]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_i32_sub_one() {
        let metadata = metadata_named("sub_one", "pub fn sub_one(x: i32) -> i32 { x - 1 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow {
                function: "sub_one".to_string(),
                expression: "x - 1".to_string(),
            })
        );
    }

    #[test]
    fn proves_usize_sub_one_from_executable_precondition() {
        let metadata = metadata("pub fn pred(n: usize) -> usize { n - 1 }", &["n >= 1"]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_usize_sub_one() {
        let metadata = metadata_named("pred", "pub fn pred(n: usize) -> usize { n - 1 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow {
                function: "pred".to_string(),
                expression: "n - 1".to_string(),
            })
        );
    }

    #[test]
    fn proves_i32_negation_from_executable_precondition() {
        let metadata = metadata_named(
            "abs_nonmin",
            "pub fn abs_nonmin(x: i32) -> i32 { if x < 0 { -x } else { x } }",
            &["x > i32::MIN"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_i32_negation() {
        let metadata = metadata_named(
            "abs_nonmin",
            "pub fn abs_nonmin(x: i32) -> i32 { if x < 0 { -x } else { x } }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerNegationOverflow {
                function: "abs_nonmin".to_string(),
                expression: "-x".to_string(),
            })
        );
    }

    #[test]
    fn proves_i32_mul_two_from_executable_preconditions() {
        let metadata = metadata_named(
            "double",
            "pub fn double(x: i32) -> i32 { x * 2 }",
            &["x <= i32::MAX / 2", "x >= i32::MIN / 2"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_i32_mul_two() {
        let metadata = metadata_named("double", "pub fn double(x: i32) -> i32 { x * 2 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerMultiplicationOverflow {
                function: "double".to_string(),
                expression: "x * 2".to_string(),
            })
        );
    }

    #[test]
    fn proves_usize_mul_two_from_executable_precondition() {
        let metadata = metadata_named(
            "double",
            "pub fn double(n: usize) -> usize { n * 2 }",
            &["n <= usize::MAX / 2"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn proves_executable_identity_postcondition() {
        let metadata = metadata_named_with_classes(
            "id_i32",
            "pub fn id_i32(x: i32) -> i32 { x }",
            &["out == x"],
            &["gives executable"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn proves_ghost_arithmetic_postcondition() {
        let metadata = metadata_named_with_classes(
            "add_one",
            "pub fn add_one(x: i32) -> i32 { x + 1 }",
            &["x < i32::MAX", "int(out) == int(x) + 1"],
            &["given executable", "gives ghost"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_false_postcondition() {
        let metadata = metadata_named_with_classes(
            "zero",
            "pub fn zero() -> i32 { 0 }",
            &["out == 1"],
            &["gives ghost"],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::PostconditionUnproved {
                function: "zero".to_string(),
                condition: "out == 1".to_string(),
            })
        );
    }

    #[test]
    fn does_not_use_postcondition_as_assumption() {
        let metadata = metadata_named_with_classes(
            "add_one",
            "pub fn add_one(x: i32) -> i32 { x + 1 }",
            &["out == x + 1"],
            &["gives executable"],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add_one".to_string(),
                expression: "x + 1".to_string(),
            })
        );
    }
}
