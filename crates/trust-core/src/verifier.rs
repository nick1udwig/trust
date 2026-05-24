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
        }
    }
}

impl std::error::Error for VerificationError {}

pub fn verify_total(metadata: &TrustMetadata) -> Result<(), VerificationError> {
    if metadata.item_kind != "total" {
        return Ok(());
    }

    let source = normalize(&metadata.function_source);
    let contracts = metadata
        .contracts_original
        .iter()
        .map(|contract| normalize(contract))
        .collect::<Vec<_>>();
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

    Ok(())
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

fn is_supported_integer(ty: &str) -> bool {
    matches!(ty, "i32" | "i64" | "usize")
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
        TrustMetadata {
            schema_version: 1,
            item_kind: "total".to_string(),
            item_id: "total:add_one:test".to_string(),
            rust_function_path: "add_one".to_string(),
            contracts_original: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
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
        let metadata = metadata("pub fn sub_one(x: i32) -> i32 { x - 1 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow {
                function: "add_one".to_string(),
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
        let metadata = metadata("pub fn pred(n: usize) -> usize { n - 1 }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow {
                function: "add_one".to_string(),
                expression: "n - 1".to_string(),
            })
        );
    }
}
