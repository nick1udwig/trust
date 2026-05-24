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
    SliceIndexOutOfBounds {
        function: String,
        expression: String,
    },
    CalleePreconditionUnproved {
        function: String,
        callee: String,
        condition: String,
    },
    MissingTrustModel {
        function: String,
        ty: String,
    },
    LoopMissingSpec {
        function: String,
    },
    LoopMissingDecreases {
        function: String,
    },
    LoopInvariantNotPreserved {
        function: String,
        invariant: String,
    },
    LoopDecreasesNotDecreasing {
        function: String,
        measure: String,
    },
    UnsupportedLoopControl {
        function: String,
        keyword: String,
    },
    UncheckedUnwrap {
        function: String,
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
            VerificationError::SliceIndexOutOfBounds {
                function,
                expression,
            } => write!(
                f,
                "could not prove index is in bounds in `{function}`: `{expression}`"
            ),
            VerificationError::CalleePreconditionUnproved {
                function,
                callee,
                condition,
            } => write!(
                f,
                "could not prove callee precondition in `{function}` for `{callee}`: `{condition}`"
            ),
            VerificationError::MissingTrustModel { function: _, ty } => write!(
                f,
                "type {ty} must derive TrustModel before Trust may reason about its fields"
            ),
            VerificationError::LoopMissingSpec { function } => {
                write!(f, "loop in `{function}` requires loop_spec")
            }
            VerificationError::LoopMissingDecreases { function: _ } => {
                write!(f, "loop in total function requires decreases measure")
            }
            VerificationError::LoopInvariantNotPreserved {
                function: _,
                invariant: _,
            } => write!(f, "loop invariant may not be preserved"),
            VerificationError::LoopDecreasesNotDecreasing {
                function: _,
                measure: _,
            } => write!(f, "loop decreases measure may not strictly decrease"),
            VerificationError::UnsupportedLoopControl { function, keyword } => write!(
                f,
                "`{keyword}` is not supported in loops in `{function}`"
            ),
            VerificationError::UncheckedUnwrap { function: _ } => write!(
                f,
                "unchecked unwrap is not supported; prove Some or use match"
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
    verify_total_with_env(metadata, &[], &[])
}

pub fn verify_totals(metadata: &[TrustMetadata]) -> Result<(), VerificationError> {
    let env = function_env(metadata);
    let model_types = model_env(metadata);
    for item in metadata {
        verify_total_with_env(item, &env, &model_types)?;
    }

    Ok(())
}

fn verify_total_with_env(
    metadata: &TrustMetadata,
    env: &[TrustFunctionSummary],
    model_types: &[String],
) -> Result<(), VerificationError> {
    if metadata.item_kind != "total" {
        return Ok(());
    }

    let source = normalize(&metadata.function_source);
    let mut contracts = executable_preconditions(metadata);
    let given_contracts = given_preconditions(metadata);
    let params = parse_params(&source);
    let raw_body = body(&metadata.function_source);
    let body = body(&source);
    let loop_facts = verify_loops(raw_body, &metadata.rust_function_path)?;
    contracts.extend(loop_facts.into_iter().map(|fact| fact.condition));

    if contains_unchecked_unwrap(body) {
        return Err(VerificationError::UncheckedUnwrap {
            function: metadata.rust_function_path.clone(),
        });
    }

    for obligation in field_access_obligations(body, &params) {
        if !model_types
            .iter()
            .any(|model_type| model_type == &type_name_tail(&obligation.ty))
        {
            return Err(VerificationError::MissingTrustModel {
                function: metadata.rust_function_path.clone(),
                ty: obligation.ty,
            });
        }
    }

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

    for obligation in slice_index_obligations(body, &params) {
        if !slice_index_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::SliceIndexOutOfBounds {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in call_obligations(body, env) {
        if !callee_precondition_proved(&obligation.condition, &given_contracts) {
            return Err(VerificationError::CalleePreconditionUnproved {
                function: metadata.rust_function_path.clone(),
                callee: obligation.callee,
                condition: obligation.condition,
            });
        }
    }

    for postcondition in postconditions(metadata) {
        if !postcondition_proved(&postcondition, body, raw_body) {
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
struct TrustFunctionSummary {
    name: String,
    params: Vec<Param>,
    preconditions: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SliceIndexObligation {
    base: String,
    index: String,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallObligation {
    callee: String,
    condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldAccessObligation {
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopFact {
    condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopSpec {
    invariant: Option<String>,
    decreases: Option<String>,
}

fn function_env(metadata: &[TrustMetadata]) -> Vec<TrustFunctionSummary> {
    metadata
        .iter()
        .filter(|item| item.item_kind == "total")
        .map(|item| {
            let source = normalize(&item.function_source);
            TrustFunctionSummary {
                name: item.rust_function_path.clone(),
                params: parse_params(&source),
                preconditions: given_preconditions(item),
            }
        })
        .collect()
}

fn model_env(metadata: &[TrustMetadata]) -> Vec<String> {
    metadata
        .iter()
        .filter(|item| item.item_kind == "trust_model")
        .map(|item| item.rust_function_path.clone())
        .collect()
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

fn given_preconditions(metadata: &TrustMetadata) -> Vec<String> {
    metadata
        .contracts_original
        .iter()
        .zip(metadata.contract_classes.iter())
        .filter(|(_contract, class)| matches!(class.as_str(), "given executable" | "given ghost"))
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
            normalized: normalize(&remove_old_wrappers(&remove_int_wrappers(contract))),
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
            let name = name.trim();
            let name = name.strip_prefix("mut").unwrap_or(name);
            Some(Param {
                name: name.to_string(),
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
    let trimmed = body
        .trim()
        .trim_start_matches("return")
        .trim_end_matches(';');
    if let Some(after_last_group) = trimmed.rsplit_once('}').map(|(_head, tail)| tail.trim()) {
        if !after_last_group.is_empty() {
            return after_last_group.trim_end_matches(';').to_string();
        }
    }

    trimmed.to_string()
}

fn postcondition_proved(postcondition: &Contract, body: &str, raw_body: &str) -> bool {
    let return_expression = return_expression(body);
    let Some((left, right)) = postcondition.normalized.split_once("==") else {
        return false;
    };

    (left == "out" && right == return_expression)
        || (right == "out" && left == return_expression)
        || (left == "out" && loop_exit_proves_value(raw_body, &return_expression, right))
        || (right == "out" && loop_exit_proves_value(raw_body, &return_expression, left))
        || output_field_equals_return_field(left, right, &return_expression)
        || output_field_equals_return_field(right, left, &return_expression)
}

fn output_field_equals_return_field(output: &str, expected: &str, return_expression: &str) -> bool {
    let Some(field) = output.strip_prefix("out.") else {
        return false;
    };
    return_field_expression(return_expression, field).as_deref() == Some(expected)
}

fn return_field_expression(return_expression: &str, field: &str) -> Option<String> {
    let open = return_expression.find('{')?;
    let close = return_expression.rfind('}')?;
    let fields = &return_expression[open + 1..close];
    let tokens = tokens(fields);
    let mut idx = 0;

    while idx + 2 < tokens.len() {
        if tokens[idx] != field || tokens[idx + 1] != ":" {
            idx += 1;
            continue;
        }

        let start = idx + 2;
        let mut end = start;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        while end < tokens.len() {
            match tokens[end].as_str() {
                "," if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => break,
                "(" => paren_depth += 1,
                ")" => paren_depth -= 1,
                "[" => bracket_depth += 1,
                "]" => bracket_depth -= 1,
                "{" => brace_depth += 1,
                "}" => brace_depth -= 1,
                _ => {}
            }
            end += 1;
        }

        return Some(token_expression(&tokens[start..end]));
    }

    None
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

fn slice_index_obligations(body: &str, params: &[Param]) -> Vec<SliceIndexObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();
    let mut idx = 0;

    while idx + 3 < tokens.len() {
        let base = &tokens[idx];
        if tokens[idx + 1] != "[" || !is_read_only_slice_param(base, params) {
            idx += 1;
            continue;
        }

        let mut depth = 1usize;
        let mut end = idx + 2;
        while end < tokens.len() {
            match tokens[end].as_str() {
                "[" => depth += 1,
                "]" => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }

        if end >= tokens.len() || end == idx + 2 {
            idx += 1;
            continue;
        }

        let index = token_expression(&tokens[idx + 2..end]);
        obligations.push(SliceIndexObligation {
            base: base.clone(),
            index: index.clone(),
            expression: format!("{base}[{index}]"),
        });
        idx += 1;
    }

    obligations
}

fn field_access_obligations(body: &str, params: &[Param]) -> Vec<FieldAccessObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();

    for (idx, window) in tokens.windows(3).enumerate() {
        let [base, dot, field] = window else {
            continue;
        };
        if dot != "." || !is_ident(field) {
            continue;
        }
        if tokens.get(idx + 3).is_some_and(|token| token == "(") {
            continue;
        }

        let Some(param) = params.iter().find(|param| param.name == *base) else {
            continue;
        };
        obligations.push(FieldAccessObligation {
            ty: param.ty.clone(),
        });
    }

    obligations
}

fn verify_loops(body: &str, function: &str) -> Result<Vec<LoopFact>, VerificationError> {
    let tokens = tokens(body);
    let mut facts = Vec::new();
    let mut pending_spec = None;
    let mut idx = 0;

    while idx < tokens.len() {
        if let Some(open_idx) = loop_spec_open_idx(&tokens, idx) {
            let Some(close_idx) = matching_token_group(&tokens, open_idx, "{", "}") else {
                idx += 1;
                continue;
            };
            pending_spec = Some(parse_loop_spec(&tokens[open_idx + 1..close_idx]));
            idx = close_idx + 1;
            continue;
        }

        if tokens[idx] != "while" {
            idx += 1;
            continue;
        }

        let Some(spec) = pending_spec.take() else {
            return Err(VerificationError::LoopMissingSpec {
                function: function.to_string(),
            });
        };
        let Some(body_open_idx) = tokens[idx + 1..]
            .iter()
            .position(|token| token == "{")
            .map(|offset| idx + 1 + offset)
        else {
            idx += 1;
            continue;
        };
        let Some(body_close_idx) = matching_token_group(&tokens, body_open_idx, "{", "}") else {
            idx += 1;
            continue;
        };

        let condition = token_expression(&tokens[idx + 1..body_open_idx]);
        let loop_body = &tokens[body_open_idx + 1..body_close_idx];
        verify_loop_spec(&spec, &condition, loop_body, function)?;
        facts.push(LoopFact { condition });
        idx = body_close_idx + 1;
    }

    if pending_spec.is_some() {
        return Err(VerificationError::LoopMissingSpec {
            function: function.to_string(),
        });
    }

    Ok(facts)
}

fn loop_spec_open_idx(tokens: &[String], idx: usize) -> Option<usize> {
    if tokens.get(idx) == Some(&"trust".to_string())
        && tokens.get(idx + 1) == Some(&":".to_string())
        && tokens.get(idx + 2) == Some(&":".to_string())
        && tokens.get(idx + 3) == Some(&"loop_spec".to_string())
        && tokens.get(idx + 4) == Some(&"!".to_string())
        && tokens.get(idx + 5) == Some(&"{".to_string())
    {
        return Some(idx + 5);
    }

    if tokens.get(idx) == Some(&"loop_spec".to_string())
        && tokens.get(idx + 1) == Some(&"!".to_string())
        && tokens.get(idx + 2) == Some(&"{".to_string())
    {
        return Some(idx + 2);
    }

    None
}

fn parse_loop_spec(tokens: &[String]) -> LoopSpec {
    let mut spec = LoopSpec {
        invariant: None,
        decreases: None,
    };
    let mut idx = 0;

    while idx + 2 < tokens.len() {
        let clause = &tokens[idx];
        if !matches!(clause.as_str(), "invariant" | "decreases") || tokens[idx + 1] != "(" {
            idx += 1;
            continue;
        }
        let Some(end) = matching_token_group(tokens, idx + 1, "(", ")") else {
            idx += 1;
            continue;
        };
        let expression = token_expression(&tokens[idx + 2..end]);
        if clause == "invariant" {
            spec.invariant = Some(expression);
        } else {
            spec.decreases = Some(expression);
        }
        idx = end + 1;
    }

    spec
}

fn verify_loop_spec(
    spec: &LoopSpec,
    condition: &str,
    loop_body: &[String],
    function: &str,
) -> Result<(), VerificationError> {
    let Some(measure) = &spec.decreases else {
        return Err(VerificationError::LoopMissingDecreases {
            function: function.to_string(),
        });
    };

    for keyword in ["break", "continue"] {
        if loop_body.iter().any(|token| token == keyword) {
            return Err(VerificationError::UnsupportedLoopControl {
                function: function.to_string(),
                keyword: keyword.to_string(),
            });
        }
    }

    if let Some(invariant) = &spec.invariant {
        if loop_invariant_may_not_be_preserved(invariant, condition, loop_body) {
            return Err(VerificationError::LoopInvariantNotPreserved {
                function: function.to_string(),
                invariant: invariant.clone(),
            });
        }
    }

    if !loop_measure_decreases(measure, loop_body) {
        return Err(VerificationError::LoopDecreasesNotDecreasing {
            function: function.to_string(),
            measure: measure.clone(),
        });
    }

    Ok(())
}

fn loop_invariant_may_not_be_preserved(
    invariant: &str,
    condition: &str,
    loop_body: &[String],
) -> bool {
    let Some((left, right)) = invariant.split_once("<=") else {
        return false;
    };
    if condition != format!("{left}<{right}") {
        return false;
    }

    increment_amount(loop_body, left).is_some_and(|amount| amount > 1)
}

fn loop_measure_decreases(measure: &str, loop_body: &[String]) -> bool {
    if decrements_variable(loop_body, measure) {
        return true;
    }

    let Some((left, right)) = measure.split_once('-') else {
        return false;
    };
    decrements_variable(loop_body, left) || increment_amount(loop_body, right).is_some()
}

fn loop_exit_proves_value(body: &str, return_expression: &str, expected: &str) -> bool {
    expected == "0"
        && tokens(body).windows(4).any(|window| {
            let [keyword, variable, op, value] = window else {
                return false;
            };
            keyword == "while" && variable == return_expression && op == ">" && value == "0"
        })
}

fn contains_unchecked_unwrap(body: &str) -> bool {
    tokens(body).windows(4).any(|window| {
        let [dot, unwrap, open, close] = window else {
            return false;
        };
        dot == "." && unwrap == "unwrap" && open == "(" && close == ")"
    })
}

fn call_obligations(body: &str, env: &[TrustFunctionSummary]) -> Vec<CallObligation> {
    let tokens = tokens(body);
    let mut obligations = Vec::new();
    let mut idx = 0;

    while idx + 1 < tokens.len() {
        let callee_name = &tokens[idx];
        if tokens[idx + 1] != "(" || idx.checked_sub(1).is_some_and(|prev| tokens[prev] == ".") {
            idx += 1;
            continue;
        }

        let Some(callee) = env.iter().find(|function| function.name == *callee_name) else {
            idx += 1;
            continue;
        };
        let Some(end) = matching_token_group(&tokens, idx + 1, "(", ")") else {
            idx += 1;
            continue;
        };

        let args = split_arguments(&tokens[idx + 2..end])
            .iter()
            .map(|tokens| token_expression(tokens))
            .collect::<Vec<_>>();
        if args.len() == callee.params.len() {
            for precondition in &callee.preconditions {
                obligations.push(CallObligation {
                    callee: callee.name.clone(),
                    condition: substitute_params(precondition, &callee.params, &args),
                });
            }
        }

        idx += 1;
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
    if obligation.ty == "usize"
        && obligation.constant == 1
        && contracts
            .iter()
            .any(|contract| contract == &format!("{}>0", obligation.variable))
    {
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

fn slice_index_obligation_proved(obligation: &SliceIndexObligation, contracts: &[String]) -> bool {
    let index_lt_len = format!("{}<{}.len()", obligation.index, obligation.base);
    let len_gt_index = format!("{}.len()>{}", obligation.base, obligation.index);

    contracts
        .iter()
        .any(|contract| contract == &index_lt_len || contract == &len_gt_index)
}

fn callee_precondition_proved(condition: &str, contracts: &[String]) -> bool {
    if contracts.iter().any(|contract| contract == condition) {
        return true;
    }

    let Some(flipped) = flipped_inequality(condition) else {
        return false;
    };
    contracts.iter().any(|contract| contract == &flipped)
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

fn is_read_only_slice_param(name: &str, params: &[Param]) -> bool {
    params
        .iter()
        .any(|param| param.name == name && param.ty.starts_with("&[") && param.ty.ends_with(']'))
}

fn type_name_tail(ty: &str) -> String {
    ty.rsplit("::").next().unwrap_or(ty).to_string()
}

fn is_ident(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn decrements_variable(tokens: &[String], variable: &str) -> bool {
    tokens.windows(5).any(|window| {
        let [target, equals, source, op, amount] = window else {
            return false;
        };
        target == variable
            && equals == "="
            && source == variable
            && op == "-"
            && amount.parse::<i128>().is_ok_and(|amount| amount > 0)
    })
}

fn increment_amount(tokens: &[String], variable: &str) -> Option<i128> {
    for window in tokens.windows(5) {
        let [target, equals, source, op, amount] = window else {
            continue;
        };
        if target == variable && equals == "=" && source == variable && op == "+" {
            if let Ok(amount) = amount.parse::<i128>() {
                return Some(amount);
            }
        }
    }

    for window in tokens.windows(4) {
        let [target, plus, equals, amount] = window else {
            continue;
        };
        if target == variable && plus == "+" && equals == "=" {
            if let Ok(amount) = amount.parse::<i128>() {
                return Some(amount);
            }
        }
    }

    None
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

fn remove_old_wrappers(input: &str) -> String {
    let mut out = input.to_string();

    while let Some(start) = out.find("old(") {
        let inner_start = start + "old(".len();
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

fn matching_token_group(
    tokens: &[String],
    open_idx: usize,
    open: &str,
    close: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, token) in tokens.iter().enumerate().skip(open_idx) {
        if token == open {
            depth += 1;
        } else if token == close {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }

    None
}

fn split_arguments(tokens: &[String]) -> Vec<Vec<String>> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for token in tokens {
        match token.as_str() {
            "," if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                args.push(std::mem::take(&mut current));
            }
            "(" => {
                paren_depth += 1;
                current.push(token.clone());
            }
            ")" => {
                paren_depth -= 1;
                current.push(token.clone());
            }
            "[" => {
                bracket_depth += 1;
                current.push(token.clone());
            }
            "]" => {
                bracket_depth -= 1;
                current.push(token.clone());
            }
            "{" => {
                brace_depth += 1;
                current.push(token.clone());
            }
            "}" => {
                brace_depth -= 1;
                current.push(token.clone());
            }
            _ => current.push(token.clone()),
        }
    }

    args.push(current);
    args
}

fn substitute_params(condition: &str, params: &[Param], args: &[String]) -> String {
    let mut out = String::new();
    for token in tokens(condition) {
        if let Some(param_idx) = params.iter().position(|param| param.name == token) {
            out.push_str(&args[param_idx]);
        } else {
            out.push_str(&token);
        }
    }
    out
}

fn flipped_inequality(condition: &str) -> Option<String> {
    for (op, flipped_op) in [("<=", ">="), (">=", "<="), ("<", ">"), (">", "<")] {
        let Some((left, right)) = condition.split_once(op) else {
            continue;
        };
        return Some(format!("{right}{flipped_op}{left}"));
    }

    None
}

fn token_expression(tokens: &[String]) -> String {
    tokens.concat()
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

    fn model_metadata(name: &str) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            item_kind: "trust_model".to_string(),
            item_id: format!("model:{name}:test"),
            rust_function_path: name.to_string(),
            contracts_original: Vec::new(),
            contract_classes: Vec::new(),
            function_source: format!("pub struct {name} {{ pub balance: i64 }}"),
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
    fn proves_slice_index_from_executable_precondition() {
        let metadata = metadata_named(
            "get",
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }",
            &["i < xs.len()"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn proves_first_slice_index_from_nonempty_precondition() {
        let metadata = metadata_named(
            "first",
            "pub fn first(xs: &[i32]) -> i32 { xs[0] }",
            &["xs.len() > 0"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_slice_index() {
        let metadata = metadata_named("first", "pub fn first(xs: &[i32]) -> i32 { xs[0] }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::SliceIndexOutOfBounds {
                function: "first".to_string(),
                expression: "xs[0]".to_string(),
            })
        );
    }

    #[test]
    fn proves_callee_precondition_from_caller_precondition() {
        let get = metadata_named(
            "get",
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }",
            &["i < xs.len()"],
        );
        let first = metadata_named(
            "first",
            "pub fn first(xs: &[i32]) -> i32 { get(xs, 0) }",
            &["xs.len() > 0"],
        );

        assert_eq!(verify_totals(&[get, first]), Ok(()));
    }

    #[test]
    fn rejects_unproved_callee_precondition() {
        let get = metadata_named(
            "get",
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }",
            &["i < xs.len()"],
        );
        let bad_first = metadata_named(
            "bad_first",
            "pub fn bad_first(xs: &[i32]) -> i32 { get(xs, 0) }",
            &[],
        );

        assert_eq!(
            verify_totals(&[get, bad_first]),
            Err(VerificationError::CalleePreconditionUnproved {
                function: "bad_first".to_string(),
                callee: "get".to_string(),
                condition: "0<xs.len()".to_string(),
            })
        );
    }

    #[test]
    fn proves_field_access_for_trust_model_type() {
        let account = model_metadata("Account");
        let balance = metadata_named(
            "balance",
            "pub fn balance(acct: Account) -> i64 { acct.balance }",
            &[],
        );

        assert_eq!(verify_totals(&[account, balance]), Ok(()));
    }

    #[test]
    fn rejects_field_access_without_trust_model_type() {
        let balance = metadata_named(
            "balance",
            "pub fn balance(acct: Account) -> i64 { acct.balance }",
            &[],
        );

        assert_eq!(
            verify_totals(&[balance]),
            Err(VerificationError::MissingTrustModel {
                function: "balance".to_string(),
                ty: "Account".to_string(),
            })
        );
    }

    #[test]
    fn proves_countdown_loop_decreases_and_postcondition() {
        let metadata = metadata_named_with_classes(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { invariant(n >= 0); decreases(n); } while n > 0 { n = n - 1; } n }",
            &["out == 0"],
            &["gives executable"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_loop_missing_decreases() {
        let metadata = metadata_named(
            "count_up",
            "pub fn count_up(mut i: usize, n: usize) -> usize { trust::loop_spec! { invariant(i <= n); } while i < n { i += 1; } i }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopMissingDecreases {
                function: "count_up".to_string(),
            })
        );
    }

    #[test]
    fn rejects_loop_invariant_not_preserved() {
        let metadata = metadata_named(
            "count_up",
            "pub fn count_up(mut i: usize, n: usize) -> usize { trust::loop_spec! { invariant(i <= n); decreases(n - i); } while i < n { i = i + 2; } i }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopInvariantNotPreserved {
                function: "count_up".to_string(),
                invariant: "i<=n".to_string(),
            })
        );
    }

    #[test]
    fn rejects_loop_decreases_not_decreasing() {
        let metadata = metadata_named(
            "count_up",
            "pub fn count_up(mut i: usize, n: usize) -> usize { trust::loop_spec! { invariant(i <= n); decreases(n - i); } while i < n { } i }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopDecreasesNotDecreasing {
                function: "count_up".to_string(),
                measure: "n-i".to_string(),
            })
        );
    }

    #[test]
    fn proves_struct_field_postconditions_with_old_values() {
        let account = model_metadata("Account");
        let withdraw = metadata_named_with_classes(
            "withdraw",
            "pub fn withdraw(acct: Account, amount: i64) -> Account { Account { id: acct.id, balance: acct.balance - amount } }",
            &[
                "amount >= 0",
                "acct.balance >= amount",
                "out.id == old(acct.id)",
                "int(out.balance) == int(old(acct.balance)) - int(old(amount))",
            ],
            &[
                "given executable",
                "given executable",
                "gives ghost",
                "gives ghost",
            ],
        );

        assert_eq!(verify_totals(&[account, withdraw]), Ok(()));
    }

    #[test]
    fn rejects_wrong_struct_field_postcondition() {
        let account = model_metadata("Account");
        let withdraw = metadata_named_with_classes(
            "withdraw",
            "pub fn withdraw(acct: Account, amount: i64) -> Account { Account { id: 0, balance: acct.balance - amount } }",
            &["out.id == old(acct.id)"],
            &["gives ghost"],
        );

        assert_eq!(
            verify_totals(&[account, withdraw]),
            Err(VerificationError::PostconditionUnproved {
                function: "withdraw".to_string(),
                condition: "out.id == old(acct.id)".to_string(),
            })
        );
    }

    #[test]
    fn accepts_option_match() {
        let metadata = metadata_named(
            "unwrap_or_zero",
            "pub fn unwrap_or_zero(x: Option<i32>) -> i32 { match x { Some(v) => v, None => 0, } }",
            &[],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unchecked_option_unwrap() {
        let metadata = metadata_named(
            "bad_unwrap",
            "pub fn bad_unwrap(x: Option<i32>) -> i32 { x.unwrap() }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UncheckedUnwrap {
                function: "bad_unwrap".to_string(),
            })
        );
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
