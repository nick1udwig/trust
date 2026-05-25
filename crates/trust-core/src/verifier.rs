use crate::{
    metadata::TrustMetadata,
    solver::{self, ProofResult, SolverBackend, VerificationOptions},
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustFunctionSemantics {
    pub rust_function_path: String,
    pub params: Vec<SemanticParam>,
    pub return_type: String,
    pub return_expression: Option<String>,
    pub arithmetic_operations: Vec<SemanticArithmeticOperation>,
    pub slice_indexes: Vec<SemanticSliceIndex>,
    pub calls: Vec<SemanticCall>,
    pub field_accesses: Vec<SemanticFieldAccess>,
    pub matches: Vec<SemanticMatch>,
    pub branches: Vec<SemanticBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticParam {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticArithmeticOperation {
    pub kind: SemanticArithmeticKind,
    pub left: String,
    pub right: Option<String>,
    pub expression: String,
    pub guards: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticArithmeticKind {
    Add,
    Sub,
    Mul,
    Neg,
    Div,
    Rem,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticSliceIndex {
    pub base: String,
    pub index: String,
    pub expression: String,
    pub guards: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticCall {
    pub callee: String,
    pub args: Vec<String>,
    pub guards: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticFieldAccess {
    pub base: String,
    pub field: String,
    pub owner_type: String,
    pub field_type: String,
    pub expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticMatch {
    pub scrutinee: String,
    pub scrutinee_type: String,
    pub arms: Vec<SemanticMatchArm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticMatchArm {
    pub variant: String,
    pub discriminant: String,
    pub payload: Option<SemanticMatchPayload>,
    pub return_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticMatchPayload {
    pub binding: String,
    pub field_index: usize,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticBranch {
    pub condition: String,
    pub arms: Vec<SemanticBranchArm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticBranchArm {
    pub guard: String,
    pub return_expression: Option<String>,
}

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
    IntegerDivisionByZero {
        function: String,
        expression: String,
    },
    IntegerRemainderByZero {
        function: String,
        expression: String,
    },
    SliceIndexOutOfBounds {
        function: String,
        expression: String,
    },
    UnsupportedIndex {
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
    LoopAmbiguousSpec {
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
    UnsupportedCall {
        function: String,
        callee: String,
    },
    UnsupportedClosure {
        function: String,
    },
    ExplicitPanic {
        function: String,
    },
    UncheckedUnwrap {
        function: String,
    },
    PostconditionUnproved {
        function: String,
        condition: String,
    },
    ProofObligationUnproved {
        proof: String,
        condition: String,
    },
    UnsupportedProofStep {
        proof: String,
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
            VerificationError::IntegerDivisionByZero {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer division denominator is nonzero in `{function}`: `{expression}`"
            ),
            VerificationError::IntegerRemainderByZero {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer remainder denominator is nonzero in `{function}`: `{expression}`"
            ),
            VerificationError::SliceIndexOutOfBounds {
                function,
                expression,
            } => write!(
                f,
                "could not prove index is in bounds in `{function}`: `{expression}`"
            ),
            VerificationError::UnsupportedIndex {
                function,
                expression,
            } => write!(
                f,
                "unsupported index expression in `{function}`: `{expression}`"
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
            VerificationError::LoopAmbiguousSpec { function } => {
                write!(f, "multiple loop_spec blocks before loop in `{function}`")
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
            VerificationError::UnsupportedCall { function, callee } => {
                write!(f, "unsupported function call in `{function}`: `{callee}`")
            }
            VerificationError::UnsupportedClosure { function } => {
                write!(f, "closures are not supported in `{function}`")
            }
            VerificationError::ExplicitPanic { function } => {
                write!(f, "explicit panic is not supported in `{function}`")
            }
            VerificationError::UncheckedUnwrap { function: _ } => write!(
                f,
                "unchecked unwrap is not supported; prove Some or use match"
            ),
            VerificationError::PostconditionUnproved {
                function,
                condition,
            } => write!(f, "could not prove postcondition in `{function}`: `{condition}`"),
            VerificationError::ProofObligationUnproved { proof, condition } => {
                write!(f, "could not prove proof obligation in `{proof}`: `{condition}`")
            }
            VerificationError::UnsupportedProofStep { proof } => {
                write!(f, "unsupported proof step in `{proof}`")
            }
        }
    }
}

impl std::error::Error for VerificationError {}

pub fn verify_total(metadata: &TrustMetadata) -> Result<(), VerificationError> {
    verify_total_with_env(metadata, None, &[], &[], VerificationOptions::default())
}

pub fn verify_totals(metadata: &[TrustMetadata]) -> Result<(), VerificationError> {
    verify_totals_with_options(metadata, VerificationOptions::default())
}

pub fn verify_totals_with_options(
    metadata: &[TrustMetadata],
    options: VerificationOptions,
) -> Result<(), VerificationError> {
    verify_totals_with_semantics(metadata, &[], options)
}

pub fn verify_totals_with_semantics(
    metadata: &[TrustMetadata],
    semantics: &[TrustFunctionSemantics],
    options: VerificationOptions,
) -> Result<(), VerificationError> {
    let env = function_env(metadata, semantics);
    let model_types = model_env(metadata);
    for item in metadata {
        verify_total_with_env(
            item,
            semantic_for(item, semantics),
            &env,
            &model_types,
            options,
        )?;
        verify_proof(item)?;
    }

    Ok(())
}

fn verify_proof(metadata: &TrustMetadata) -> Result<(), VerificationError> {
    if metadata.item_kind != "proof" {
        return Ok(());
    }

    let body = body(&metadata.function_source);
    if contains_unsupported_proof_step(body) {
        return Err(VerificationError::UnsupportedProofStep {
            proof: metadata.rust_function_path.clone(),
        });
    }

    for obligation in proof_obligations(metadata) {
        if !proof_body_asserts(body, &obligation) {
            return Err(VerificationError::ProofObligationUnproved {
                proof: metadata.rust_function_path.clone(),
                condition: obligation,
            });
        }
    }

    Ok(())
}

fn verify_total_with_env(
    metadata: &TrustMetadata,
    semantics: Option<&TrustFunctionSemantics>,
    env: &[TrustFunctionSummary],
    model_types: &[String],
    options: VerificationOptions,
) -> Result<(), VerificationError> {
    if metadata.item_kind != "total" {
        return Ok(());
    }

    let source = normalize(&metadata.function_source);
    let mut contracts = executable_preconditions(metadata);
    let given_contracts = given_preconditions(metadata);
    let params = verification_params(&source, semantics);
    let raw_body = body(&metadata.function_source);
    let body = body(&source);
    let semantic_return_expression = semantics
        .and_then(|semantics| semantics.return_expression.as_deref())
        .map(normalize);
    let loop_facts = verify_loops(raw_body, &metadata.rust_function_path)?
        .into_iter()
        .map(|fact| fact.condition)
        .collect::<Vec<_>>();
    contracts.extend(loop_facts.iter().cloned());
    let postcondition_assumptions = contracts_with_assumptions(&given_contracts, &loop_facts);

    if contains_unchecked_unwrap(body) {
        return Err(VerificationError::UncheckedUnwrap {
            function: metadata.rust_function_path.clone(),
        });
    }
    if contains_explicit_panic(body) {
        return Err(VerificationError::ExplicitPanic {
            function: metadata.rust_function_path.clone(),
        });
    }
    if contains_closure(body) {
        return Err(VerificationError::UnsupportedClosure {
            function: metadata.rust_function_path.clone(),
        });
    }
    if let Some(callee) = unsupported_call(body, &params, env, &metadata.rust_function_path) {
        return Err(VerificationError::UnsupportedCall {
            function: metadata.rust_function_path.clone(),
            callee,
        });
    }

    for obligation in verification_field_access_obligations(body, &params, semantics) {
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

    for obligation in verification_addition_obligations(body, &params, semantics) {
        if !addition_obligation_proved(&obligation, &contracts, &params, options) {
            return Err(VerificationError::IntegerAdditionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_subtraction_obligations(body, &params, semantics) {
        if !subtraction_obligation_proved(&obligation, &contracts, &params, options) {
            return Err(VerificationError::IntegerSubtractionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_negation_obligations(body, &params, semantics) {
        if !negation_obligation_proved(&obligation, &contracts, &params, options) {
            return Err(VerificationError::IntegerNegationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_multiplication_obligations(body, &params, semantics) {
        if !multiplication_obligation_proved(&obligation, &contracts, &params, options) {
            return Err(VerificationError::IntegerMultiplicationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_division_obligations(body, &params, semantics) {
        let contracts = contracts_with_assumptions(&contracts, &obligation.assumptions);
        if !denominator_nonzero(&obligation.denominator, &contracts, &params, options) {
            return Err(VerificationError::IntegerDivisionByZero {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_remainder_obligations(body, &params, semantics) {
        let contracts = contracts_with_assumptions(&contracts, &obligation.assumptions);
        if !denominator_nonzero(&obligation.denominator, &contracts, &params, options) {
            return Err(VerificationError::IntegerRemainderByZero {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    if let Some(expression) = unsupported_index_expression(body, &params) {
        return Err(VerificationError::UnsupportedIndex {
            function: metadata.rust_function_path.clone(),
            expression,
        });
    }

    for obligation in verification_slice_index_obligations(body, &params, semantics) {
        if !slice_index_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::SliceIndexOutOfBounds {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_call_obligations(body, env, semantics) {
        let contracts = contracts_with_assumptions(&given_contracts, &obligation.assumptions);
        if !callee_precondition_proved(&obligation.condition, &contracts, &params, options) {
            return Err(VerificationError::CalleePreconditionUnproved {
                function: metadata.rust_function_path.clone(),
                callee: obligation.callee,
                condition: obligation.condition,
            });
        }
    }

    for postcondition in postconditions(metadata) {
        if !postcondition_proved(
            &postcondition,
            body,
            raw_body,
            semantic_return_expression.as_deref(),
            semantics,
            &postcondition_assumptions,
            &params,
            options,
        ) {
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
    ty: Option<String>,
    constant: Option<i128>,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubObligation {
    variable: String,
    ty: Option<String>,
    constant: Option<i128>,
    rhs: Option<String>,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NegObligation {
    variable: String,
    ty: String,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MulObligation {
    variable: String,
    ty: Option<String>,
    constant: Option<i128>,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DenominatorObligation {
    denominator: String,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SliceIndexObligation {
    base: String,
    index: String,
    expression: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallObligation {
    callee: String,
    condition: String,
    assumptions: Vec<String>,
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

fn semantic_for<'a>(
    metadata: &TrustMetadata,
    semantics: &'a [TrustFunctionSemantics],
) -> Option<&'a TrustFunctionSemantics> {
    semantics
        .iter()
        .find(|semantics| semantics.rust_function_path == metadata.rust_function_path)
}

fn function_env(
    metadata: &[TrustMetadata],
    semantics: &[TrustFunctionSemantics],
) -> Vec<TrustFunctionSummary> {
    metadata
        .iter()
        .filter(|item| item.item_kind == "total")
        .map(|item| {
            let source = normalize(&item.function_source);
            TrustFunctionSummary {
                name: item.rust_function_path.clone(),
                params: verification_params(&source, semantic_for(item, semantics)),
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

fn proof_obligations(metadata: &TrustMetadata) -> Vec<String> {
    metadata
        .contracts_original
        .iter()
        .zip(metadata.contract_classes.iter())
        .filter(|(_contract, class)| class.starts_with("gives"))
        .map(|(contract, _class)| contract.clone())
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

fn verification_params(source: &str, semantics: Option<&TrustFunctionSemantics>) -> Vec<Param> {
    let semantic_params = semantics
        .map(|semantics| {
            semantics
                .params
                .iter()
                .map(|param| Param {
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if semantic_params.is_empty() {
        parse_params(source)
    } else {
        semantic_params
    }
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

fn postcondition_proved(
    postcondition: &Contract,
    body: &str,
    raw_body: &str,
    semantic_return_expression: Option<&str>,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let return_expression = return_expression(body);
    if semantic_return_expression.is_some_and(|return_expression| {
        postcondition_proved_by_return_expression_with_assumptions(
            postcondition,
            raw_body,
            return_expression,
            contracts,
            params,
            options,
        )
    }) {
        return true;
    }
    if semantic_match_proves_postcondition(
        postcondition,
        raw_body,
        semantics,
        contracts,
        params,
        options,
    ) {
        return true;
    }
    if semantic_branch_proves_postcondition(
        postcondition,
        raw_body,
        semantics,
        contracts,
        params,
        options,
    ) {
        return true;
    }

    postcondition_proved_by_return_expression_with_assumptions(
        postcondition,
        raw_body,
        &return_expression,
        contracts,
        params,
        options,
    )
}

fn semantic_match_proves_postcondition(
    postcondition: &Contract,
    raw_body: &str,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics.matches.iter().any(|semantic_match| {
            if !semantic_match_is_exhaustive(semantic_match) {
                return false;
            }

            let reachable_arms = semantic_match
                .arms
                .iter()
                .filter(|arm| semantic_match_arm_reachable(semantic_match, arm, contracts))
                .collect::<Vec<_>>();
            !reachable_arms.is_empty()
                && reachable_arms.iter().all(|arm| {
                    arm.return_expression
                        .as_ref()
                        .is_some_and(|return_expression| {
                            let return_expression = normalize(return_expression);
                            postcondition_proved_by_return_expression_with_assumptions(
                                postcondition,
                                raw_body,
                                &return_expression,
                                contracts,
                                params,
                                options,
                            )
                        })
                })
        })
    })
}

fn semantic_match_is_exhaustive(semantic_match: &SemanticMatch) -> bool {
    let variants = semantic_match
        .arms
        .iter()
        .map(|arm| arm.variant.as_str())
        .collect::<Vec<_>>();
    if type_name_tail(&semantic_match.scrutinee_type).starts_with("Option<") {
        return variants.contains(&"None") && variants.contains(&"Some");
    }
    if type_name_tail(&semantic_match.scrutinee_type).starts_with("Result<") {
        return variants.contains(&"Ok") && variants.contains(&"Err");
    }

    false
}

fn semantic_match_arm_reachable(
    semantic_match: &SemanticMatch,
    arm: &SemanticMatchArm,
    contracts: &[String],
) -> bool {
    !contracts.iter().any(|contract| {
        semantic_variant_excluded_by_contract(&semantic_match.scrutinee, &arm.variant, contract)
    })
}

fn semantic_variant_excluded_by_contract(scrutinee: &str, variant: &str, contract: &str) -> bool {
    match variant {
        "None" => {
            contract == format!("{scrutinee}!=None") || contract == format!("None!={scrutinee}")
        }
        "Some" => {
            contract == format!("{scrutinee}==None") || contract == format!("None=={scrutinee}")
        }
        _ => false,
    }
}

fn semantic_branch_proves_postcondition(
    postcondition: &Contract,
    raw_body: &str,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics.branches.iter().any(|branch| {
            let reachable_arms = branch
                .arms
                .iter()
                .filter(|arm| semantic_branch_arm_reachable(arm, contracts))
                .collect::<Vec<_>>();
            !reachable_arms.is_empty()
                && reachable_arms.iter().all(|arm| {
                    arm.return_expression
                        .as_ref()
                        .is_some_and(|return_expression| {
                            let return_expression = normalize(return_expression);
                            let branch_contracts =
                                contracts_with_assumptions(contracts, &[normalize(&arm.guard)]);
                            postcondition_proved_by_return_expression_with_assumptions(
                                postcondition,
                                raw_body,
                                &return_expression,
                                &branch_contracts,
                                params,
                                options,
                            )
                        })
                })
        })
    })
}

fn semantic_branch_arm_reachable(arm: &SemanticBranchArm, contracts: &[String]) -> bool {
    !contracts.iter().any(|contract| {
        semantic_condition_excluded_by_contract(&normalize(&arm.guard), &normalize(contract))
    })
}

fn semantic_condition_excluded_by_contract(condition: &str, contract: &str) -> bool {
    negated_condition(condition).as_deref() == Some(contract)
}

fn negated_condition(condition: &str) -> Option<String> {
    for (op, negated) in [
        ("<=", ">"),
        (">=", "<"),
        ("!=", "=="),
        ("==", "!="),
        ("<", ">="),
        (">", "<="),
    ] {
        let Some((left, right)) = condition.split_once(op) else {
            continue;
        };
        return Some(format!("{}{}{}", left.trim(), negated, right.trim()));
    }

    None
}

fn postcondition_proved_by_return_expression(
    postcondition: &Contract,
    raw_body: &str,
    return_expression: &str,
) -> bool {
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

fn postcondition_proved_by_return_expression_with_assumptions(
    postcondition: &Contract,
    raw_body: &str,
    return_expression: &str,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    if postcondition_proved_by_return_expression(postcondition, raw_body, return_expression) {
        return true;
    }

    let conclusion = substitute_out(&postcondition.normalized, return_expression);
    z3_proves_conclusion(&conclusion, contracts, params, options).is_some_and(|proved| proved)
}

fn substitute_out(condition: &str, return_expression: &str) -> String {
    token_expression(
        &tokens(condition)
            .into_iter()
            .map(|token| {
                if token == "out" {
                    return_expression.to_string()
                } else {
                    token
                }
            })
            .collect::<Vec<_>>(),
    )
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
    let tokens = executable_tokens(body);
    let mut obligations = Vec::new();

    for idx in 0..tokens.len().saturating_sub(2) {
        let left = &tokens[idx];
        let op = &tokens[idx + 1];
        let right = &tokens[idx + 2];
        if op != "+" {
            continue;
        }
        if let Some(expression) = complex_binary_expression(&tokens, idx + 1) {
            obligations.push(AddObligation {
                variable: left.clone(),
                ty: param_type(left, params)
                    .or_else(|| param_type(right, params))
                    .map(|ty| ty.to_string()),
                constant: None,
                expression,
                assumptions: Vec::new(),
            });
            continue;
        }
        if !is_value_operand(left) || !is_value_operand(right) {
            continue;
        }

        let expression = format!("{left} + {right}");
        if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
            obligations.push(AddObligation {
                variable: left.clone(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression,
                assumptions: Vec::new(),
            });
        } else if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
            obligations.push(AddObligation {
                variable: right.clone(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression,
                assumptions: Vec::new(),
            });
        } else if expression_needs_integer_proof(left, right, params) {
            obligations.push(AddObligation {
                variable: left.clone(),
                ty: param_type(left, params)
                    .or_else(|| param_type(right, params))
                    .map(|ty| ty.to_string()),
                constant: None,
                expression,
                assumptions: Vec::new(),
            });
        }
    }

    obligations
}

fn subtraction_obligations(body: &str, params: &[Param]) -> Vec<SubObligation> {
    let tokens = executable_tokens(body);
    let mut obligations = Vec::new();

    for idx in 0..tokens.len().saturating_sub(2) {
        let left = &tokens[idx];
        let op = &tokens[idx + 1];
        let right = &tokens[idx + 2];
        if op != "-" {
            continue;
        }
        if let Some(expression) = complex_binary_expression(&tokens, idx + 1) {
            obligations.push(SubObligation {
                variable: left.clone(),
                ty: param_type(left, params).map(|ty| ty.to_string()),
                constant: None,
                rhs: Some(right.clone()),
                expression,
                assumptions: Vec::new(),
            });
            continue;
        }
        if !is_value_operand(left) || !is_value_operand(right) {
            continue;
        }

        let variable = field_expression_before(&tokens, idx + 1).unwrap_or_else(|| left.clone());
        let expression = format!("{variable} - {right}");
        if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
            if constant > 0 {
                obligations.push(SubObligation {
                    variable,
                    ty: Some(ty.to_string()),
                    constant: Some(constant),
                    rhs: None,
                    expression,
                    assumptions: Vec::new(),
                });
            }
        } else if expression_needs_integer_proof(left, right, params)
            || field_expression_before(&tokens, idx + 1).is_some()
        {
            obligations.push(SubObligation {
                variable,
                ty: param_type(left, params).map(|ty| ty.to_string()),
                constant: None,
                rhs: Some(right.clone()),
                expression,
                assumptions: Vec::new(),
            });
        }
    }

    obligations
}

fn negation_obligations(body: &str, params: &[Param]) -> Vec<NegObligation> {
    let tokens = executable_tokens(body);
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
            assumptions: Vec::new(),
        });
    }

    obligations
}

fn multiplication_obligations(body: &str, params: &[Param]) -> Vec<MulObligation> {
    let tokens = executable_tokens(body);
    let mut obligations = Vec::new();

    for idx in 0..tokens.len().saturating_sub(2) {
        let left = &tokens[idx];
        let op = &tokens[idx + 1];
        let right = &tokens[idx + 2];
        if op != "*" {
            continue;
        }
        if let Some(expression) = complex_binary_expression(&tokens, idx + 1) {
            obligations.push(MulObligation {
                variable: left.clone(),
                ty: param_type(left, params)
                    .or_else(|| param_type(right, params))
                    .map(|ty| ty.to_string()),
                constant: None,
                expression,
                assumptions: Vec::new(),
            });
            continue;
        }
        if !is_value_operand(left) || !is_value_operand(right) {
            continue;
        }

        let expression = format!("{left} * {right}");
        if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
            if constant > 1 {
                obligations.push(MulObligation {
                    variable: left.clone(),
                    ty: Some(ty.to_string()),
                    constant: Some(constant),
                    expression,
                    assumptions: Vec::new(),
                });
            }
        } else if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
            if constant > 1 {
                obligations.push(MulObligation {
                    variable: right.clone(),
                    ty: Some(ty.to_string()),
                    constant: Some(constant),
                    expression,
                    assumptions: Vec::new(),
                });
            }
        } else if expression_needs_integer_proof(left, right, params) {
            obligations.push(MulObligation {
                variable: left.clone(),
                ty: param_type(left, params)
                    .or_else(|| param_type(right, params))
                    .map(|ty| ty.to_string()),
                constant: None,
                expression,
                assumptions: Vec::new(),
            });
        }
    }

    obligations
}

fn verification_addition_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<AddObligation> {
    let semantic_obligations = semantic_addition_obligations(semantics, params);
    if semantic_obligations.is_empty() {
        addition_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_addition_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
) -> Vec<AddObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Add)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_addition_obligation(&operation.left, right, &operation.expression, params).map(
                |mut obligation| {
                    obligation.assumptions = semantic_guard_assumptions(operation);
                    obligation
                },
            )
        })
        .collect()
}

fn verification_subtraction_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<SubObligation> {
    let semantic_obligations = semantic_subtraction_obligations(semantics, params);
    if semantic_obligations.is_empty() {
        subtraction_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_subtraction_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
) -> Vec<SubObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Sub)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_subtraction_obligation(&operation.left, right, &operation.expression, params)
                .map(|mut obligation| {
                    obligation.assumptions = semantic_guard_assumptions(operation);
                    obligation
                })
        })
        .collect()
}

fn verification_negation_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<NegObligation> {
    let semantic_obligations = semantic_negation_obligations(semantics, params);
    if semantic_obligations.is_empty() {
        negation_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_negation_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
) -> Vec<NegObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Neg)
        .filter_map(|operation| {
            let param = params
                .iter()
                .find(|param| param.name == operation.left && is_signed_integer(&param.ty))?;
            Some(NegObligation {
                variable: operation.left.clone(),
                ty: param.ty.clone(),
                expression: operation.expression.clone(),
                assumptions: semantic_guard_assumptions(operation),
            })
        })
        .collect()
}

fn verification_multiplication_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<MulObligation> {
    let semantic_obligations = semantic_multiplication_obligations(semantics, params);
    if semantic_obligations.is_empty() {
        multiplication_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_multiplication_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
) -> Vec<MulObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Mul)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_multiplication_obligation(
                &operation.left,
                right,
                &operation.expression,
                params,
            )
            .map(|mut obligation| {
                obligation.assumptions = semantic_guard_assumptions(operation);
                obligation
            })
        })
        .collect()
}

fn semantic_division_obligations(
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<DenominatorObligation> {
    semantic_denominator_obligations(semantics, SemanticArithmeticKind::Div)
}

fn semantic_remainder_obligations(
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<DenominatorObligation> {
    semantic_denominator_obligations(semantics, SemanticArithmeticKind::Rem)
}

fn verification_division_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<DenominatorObligation> {
    let semantic_obligations = semantic_division_obligations(semantics);
    if semantic_obligations.is_empty() {
        division_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn verification_remainder_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<DenominatorObligation> {
    let semantic_obligations = semantic_remainder_obligations(semantics);
    if semantic_obligations.is_empty() {
        remainder_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_arithmetic_operations(
    semantics: Option<&TrustFunctionSemantics>,
    kind: SemanticArithmeticKind,
) -> impl Iterator<Item = &SemanticArithmeticOperation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.arithmetic_operations.iter())
        .filter(move |operation| operation.kind == kind)
}

fn semantic_guard_assumptions(operation: &SemanticArithmeticOperation) -> Vec<String> {
    operation
        .guards
        .iter()
        .map(|guard| normalize(guard))
        .collect()
}

fn semantic_denominator_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    kind: SemanticArithmeticKind,
) -> Vec<DenominatorObligation> {
    semantic_arithmetic_operations(semantics, kind)
        .filter_map(|operation| {
            let denominator = operation.right.as_ref()?;
            Some(DenominatorObligation {
                denominator: denominator.clone(),
                expression: operation.expression.clone(),
                assumptions: semantic_guard_assumptions(operation),
            })
        })
        .collect()
}

fn semantic_addition_obligation(
    left: &str,
    right: &str,
    expression: &str,
    params: &[Param],
) -> Option<AddObligation> {
    if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
        return Some(AddObligation {
            variable: left.to_string(),
            ty: Some(ty.to_string()),
            constant: Some(constant),
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }
    if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
        return Some(AddObligation {
            variable: right.to_string(),
            ty: Some(ty.to_string()),
            constant: Some(constant),
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }
    if expression_needs_integer_proof(left, right, params) {
        return Some(AddObligation {
            variable: left.to_string(),
            ty: param_type(left, params)
                .or_else(|| param_type(right, params))
                .map(str::to_string),
            constant: None,
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }

    None
}

fn semantic_subtraction_obligation(
    left: &str,
    right: &str,
    expression: &str,
    params: &[Param],
) -> Option<SubObligation> {
    if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
        if constant > 0 {
            return Some(SubObligation {
                variable: left.to_string(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                rhs: None,
                expression: expression.to_string(),
                assumptions: Vec::new(),
            });
        }
    } else if expression_needs_integer_proof(left, right, params) {
        return Some(SubObligation {
            variable: left.to_string(),
            ty: param_type(left, params).map(str::to_string),
            constant: None,
            rhs: Some(right.to_string()),
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }

    None
}

fn semantic_multiplication_obligation(
    left: &str,
    right: &str,
    expression: &str,
    params: &[Param],
) -> Option<MulObligation> {
    if let (Some(ty), Ok(constant)) = (param_type(left, params), right.parse::<i128>()) {
        if constant > 1 {
            return Some(MulObligation {
                variable: left.to_string(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression: expression.to_string(),
                assumptions: Vec::new(),
            });
        }
    } else if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
        if constant > 1 {
            return Some(MulObligation {
                variable: right.to_string(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression: expression.to_string(),
                assumptions: Vec::new(),
            });
        }
    } else if expression_needs_integer_proof(left, right, params) {
        return Some(MulObligation {
            variable: left.to_string(),
            ty: param_type(left, params)
                .or_else(|| param_type(right, params))
                .map(str::to_string),
            constant: None,
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }

    None
}

fn division_obligations(body: &str, params: &[Param]) -> Vec<DenominatorObligation> {
    denominator_obligations(body, params, "/")
}

fn remainder_obligations(body: &str, params: &[Param]) -> Vec<DenominatorObligation> {
    denominator_obligations(body, params, "%")
}

fn denominator_obligations(body: &str, params: &[Param], op: &str) -> Vec<DenominatorObligation> {
    let tokens = executable_tokens(body);
    let mut obligations = Vec::new();

    for idx in 0..tokens.len().saturating_sub(2) {
        let left = &tokens[idx];
        let operator = &tokens[idx + 1];
        let right = &tokens[idx + 2];
        if operator != op {
            continue;
        }
        if let Some((denominator, expression)) =
            complex_denominator_expression(&tokens, idx + 1, op)
        {
            obligations.push(DenominatorObligation {
                denominator,
                expression,
                assumptions: Vec::new(),
            });
            continue;
        }
        if !is_value_operand(left) || !is_value_operand(right) {
            continue;
        }
        if right.parse::<i128>().is_ok_and(|value| value != 0) {
            continue;
        }
        if param_type(left, params).is_some()
            || param_type(right, params).is_some()
            || right.parse::<i128>() == Ok(0)
        {
            obligations.push(DenominatorObligation {
                denominator: right.clone(),
                expression: format!("{left} {op} {right}"),
                assumptions: Vec::new(),
            });
        }
    }

    obligations
}

fn slice_index_obligations(body: &str, params: &[Param]) -> Vec<SliceIndexObligation> {
    let tokens = executable_tokens(body);
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
            assumptions: Vec::new(),
        });
        idx += 1;
    }

    obligations
}

fn verification_slice_index_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<SliceIndexObligation> {
    let semantic_obligations = semantic_slice_index_obligations(semantics);
    if semantic_obligations.is_empty() {
        slice_index_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_slice_index_obligations(
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<SliceIndexObligation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.slice_indexes.iter())
        .map(|index| SliceIndexObligation {
            base: index.base.clone(),
            index: index.index.clone(),
            expression: index.expression.clone(),
            assumptions: index.guards.iter().map(|guard| normalize(guard)).collect(),
        })
        .collect()
}

fn unsupported_index_expression(body: &str, params: &[Param]) -> Option<String> {
    let tokens = executable_tokens(body);
    let mut idx = 0;

    while idx + 3 < tokens.len() {
        let base = &tokens[idx];
        if tokens[idx + 1] != "[" {
            idx += 1;
            continue;
        }
        let Some(end) = matching_token_group(&tokens, idx + 1, "[", "]") else {
            idx += 1;
            continue;
        };
        if !is_read_only_slice_param(base, params) {
            let index = token_expression(&tokens[idx + 2..end]);
            return Some(format!("{base}[{index}]"));
        }
        idx = end + 1;
    }

    None
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

fn verification_field_access_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<FieldAccessObligation> {
    let semantic_obligations = semantic_field_access_obligations(semantics);
    if semantic_obligations.is_empty() {
        field_access_obligations(body, params)
    } else {
        semantic_obligations
    }
}

fn semantic_field_access_obligations(
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<FieldAccessObligation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.field_accesses.iter())
        .map(|field| FieldAccessObligation {
            ty: field.owner_type.clone(),
        })
        .collect()
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
            if pending_spec.is_some() {
                return Err(VerificationError::LoopAmbiguousSpec {
                    function: function.to_string(),
                });
            }
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
    tokens(body).windows(3).any(|window| {
        let [dot, method, open] = window else {
            return false;
        };
        dot == "." && matches!(method.as_str(), "unwrap" | "expect") && open == "("
    })
}

fn contains_explicit_panic(body: &str) -> bool {
    tokens(body).windows(3).any(|window| {
        let [name, bang, open] = window else {
            return false;
        };
        matches!(name.as_str(), "panic" | "todo" | "unimplemented") && bang == "!" && open == "("
    })
}

fn contains_closure(body: &str) -> bool {
    let tokens = tokens(body);
    tokens.iter().enumerate().any(|(idx, token)| {
        token == "|"
            && (idx == 0
                || tokens[idx - 1] == "="
                || tokens[idx - 1] == "("
                || tokens.get(idx + 1).is_some_and(|next| {
                    next == "|" || is_ident(next) || matches!(next.as_str(), "mut" | "move" | "_")
                }))
    })
}

fn unsupported_call(
    body: &str,
    params: &[Param],
    env: &[TrustFunctionSummary],
    function: &str,
) -> Option<String> {
    let tokens = tokens(body);

    for window in tokens.windows(4) {
        let [base, dot, method, open] = window else {
            continue;
        };
        if dot != "." || open != "(" || !is_ident(method) {
            continue;
        }
        if method == "len" && is_read_only_slice_param(base, params) {
            continue;
        }

        return Some(format!("{base}.{method}"));
    }

    for (idx, window) in tokens.windows(3).enumerate() {
        let [name, bang, open] = window else {
            continue;
        };
        if bang == "!" && matches!(open.as_str(), "(" | "[" | "{") {
            if name == "loop_spec" {
                continue;
            }
            return Some(format!("{name}!"));
        }

        if bang != "(" || !is_ident(name) {
            continue;
        }
        if idx.checked_sub(1).is_some_and(|prev| tokens[prev] == ".") {
            continue;
        }
        if allowed_builtin_call(name) {
            continue;
        }
        if name == function {
            return Some(name.clone());
        }
        if env.iter().any(|callee| callee.name == *name) {
            continue;
        }

        return Some(name.clone());
    }

    None
}

fn allowed_builtin_call(name: &str) -> bool {
    matches!(
        name,
        "Some"
            | "None"
            | "Ok"
            | "Err"
            | "if"
            | "while"
            | "match"
            | "return"
            | "invariant"
            | "decreases"
            | "assert"
    )
}

fn contains_unsupported_proof_step(body: &str) -> bool {
    let tokens = tokens(body);
    for (idx, window) in tokens.windows(3).enumerate() {
        let [name, bang, open] = window else {
            continue;
        };
        if bang == "!" && open == "(" && name != "assert" {
            return true;
        }
        if bang == "("
            && is_ident(name)
            && idx.checked_sub(1).is_none_or(|prev| tokens[prev] != ".")
            && !matches!(
                name.as_str(),
                "assert" | "int" | "old" | "forall" | "exists" | "implies"
            )
        {
            return true;
        }
    }

    false
}

fn proof_body_asserts(body: &str, obligation: &str) -> bool {
    let tokens = tokens(body);
    let normalized_obligation = normalize(obligation);
    let mut idx = 0;

    while idx + 1 < tokens.len() {
        if tokens[idx] != "assert" {
            idx += 1;
            continue;
        }
        let open_idx = if tokens.get(idx + 1) == Some(&"!".to_string())
            && tokens.get(idx + 2) == Some(&"(".to_string())
        {
            idx + 2
        } else if tokens.get(idx + 1) == Some(&"(".to_string()) {
            idx + 1
        } else {
            idx += 1;
            continue;
        };
        let Some(close_idx) = matching_token_group(&tokens, open_idx, "(", ")") else {
            idx += 1;
            continue;
        };
        let assertion = token_expression(&tokens[open_idx + 1..close_idx]);
        if normalize(&assertion) == normalized_obligation {
            return true;
        }
        idx = close_idx + 1;
    }

    false
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
                    assumptions: Vec::new(),
                });
            }
        }

        idx += 1;
    }

    obligations
}

fn verification_call_obligations(
    body: &str,
    env: &[TrustFunctionSummary],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<CallObligation> {
    let semantic_obligations = semantic_call_obligations(semantics, env);
    if semantic_obligations.is_empty() {
        call_obligations(body, env)
    } else {
        semantic_obligations
    }
}

fn semantic_call_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    env: &[TrustFunctionSummary],
) -> Vec<CallObligation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.calls.iter())
        .filter_map(|call| {
            let callee = env
                .iter()
                .find(|function| function_name_matches_call(&function.name, &call.callee))?;
            if call.args.len() != callee.params.len() {
                return None;
            }
            Some(
                callee
                    .preconditions
                    .iter()
                    .map(|precondition| CallObligation {
                        callee: callee.name.clone(),
                        condition: substitute_params(precondition, &callee.params, &call.args),
                        assumptions: call.guards.iter().map(|guard| normalize(guard)).collect(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect()
}

fn function_name_matches_call(function: &str, call: &str) -> bool {
    function == call || function_leaf_name(call) == function
}

fn function_leaf_name(path: &str) -> &str {
    path.rsplit("::")
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

fn contracts_with_assumptions(contracts: &[String], assumptions: &[String]) -> Vec<String> {
    let mut combined = contracts.to_vec();
    combined.extend(assumptions.iter().cloned());
    combined
}

fn addition_obligation_proved(
    obligation: &AddObligation,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
    let Some(constant) = obligation.constant else {
        return false;
    };
    if constant == 0 {
        return true;
    }
    let Some(ty) = &obligation.ty else {
        return false;
    };
    let Some(max) = max_value(ty) else {
        return false;
    };
    let required_bound = max - constant;
    let lt_exact = format!("{}<{}::MAX", obligation.variable, ty);
    let le_required = format!(
        "{}<={}",
        obligation.variable,
        constant_with_type(required_bound, ty)
    );
    let le_unqualified = format!("{}<={required_bound}", obligation.variable);

    if options.solver == SolverBackend::Z3 {
        let params = params
            .iter()
            .filter(|param| is_supported_integer(&param.ty))
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>();
        match solver::prove_addition_overflow_safety(
            &obligation.variable,
            ty,
            constant,
            contracts,
            &params,
            options.timeout_ms,
        ) {
            ProofResult::Proved => return true,
            ProofResult::Unproved => return false,
            ProofResult::Unsupported => {}
        }
    }

    if constant == 1 && contracts.iter().any(|contract| contract == &lt_exact) {
        return true;
    }

    contracts
        .iter()
        .any(|contract| contract == &le_required || contract == &le_unqualified)
}

fn subtraction_obligation_proved(
    obligation: &SubObligation,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
    if let Some(constant) = obligation.constant {
        if constant == 0 {
            return true;
        }
        let Some(ty) = &obligation.ty else {
            return false;
        };
        let Some(min) = min_value(ty) else {
            return false;
        };
        let required_bound = min + constant;
        let gt_min = format!("{}>{ty}::MIN", obligation.variable);
        let ge_required = format!(
            "{}>={}",
            obligation.variable,
            min_bound_with_type(required_bound, ty)
        );
        let ge_unqualified = format!("{}>={required_bound}", obligation.variable);

        if z3_proves_conclusion(&ge_unqualified, contracts, params, options)
            .is_some_and(|proved| proved)
        {
            return true;
        }

        if constant == 1 && contracts.iter().any(|contract| contract == &gt_min) {
            return true;
        }
        if ty == "usize"
            && constant == 1
            && contracts
                .iter()
                .any(|contract| contract == &format!("{}>0", obligation.variable))
        {
            return true;
        }

        return contracts
            .iter()
            .any(|contract| contract == &ge_required || contract == &ge_unqualified);
    }

    let Some(rhs) = &obligation.rhs else {
        return false;
    };
    let ge_rhs = format!("{}>={rhs}", obligation.variable);
    let rhs_nonnegative = format!("{rhs}>=0");

    if z3_proves_conclusion(&ge_rhs, contracts, params, options).is_some_and(|proved| proved)
        && z3_proves_conclusion(&rhs_nonnegative, contracts, params, options)
            .is_some_and(|proved| proved)
    {
        return true;
    }

    contracts.iter().any(|contract| contract == &ge_rhs)
        && contracts
            .iter()
            .any(|contract| contract == &rhs_nonnegative)
}

fn negation_obligation_proved(
    obligation: &NegObligation,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
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

    if z3_proves_conclusion(&ge_unqualified, contracts, params, options)
        .is_some_and(|proved| proved)
    {
        return true;
    }

    contracts.iter().any(|contract| {
        contract == &gt_min || contract == &ge_required || contract == &ge_unqualified
    })
}

fn multiplication_obligation_proved(
    obligation: &MulObligation,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
    let Some(constant) = obligation.constant else {
        return false;
    };
    if constant == 0 || constant == 1 {
        return true;
    }
    let Some(ty) = &obligation.ty else {
        return false;
    };
    let Some(max) = max_value(ty) else {
        return false;
    };
    let upper_symbolic = format!("{}<={}::MAX/{}", obligation.variable, ty, constant);
    let upper_numeric = format!("{}<={}", obligation.variable, max / constant);
    let upper_proved = contracts
        .iter()
        .any(|contract| contract == &upper_symbolic || contract == &upper_numeric);
    let z3_upper_proved = z3_proves_conclusion(&upper_numeric, contracts, params, options)
        .is_some_and(|proved| proved);

    if ty == "usize" {
        return z3_upper_proved || upper_proved;
    }

    let Some(min) = min_value(ty) else {
        return false;
    };
    let lower_symbolic = format!("{}>={}::MIN/{}", obligation.variable, ty, constant);
    let lower_numeric = format!("{}>={}", obligation.variable, min / constant);
    let lower_proved = contracts
        .iter()
        .any(|contract| contract == &lower_symbolic || contract == &lower_numeric);
    let z3_lower_proved = z3_proves_conclusion(&lower_numeric, contracts, params, options)
        .is_some_and(|proved| proved);

    (z3_upper_proved && z3_lower_proved) || (upper_proved && lower_proved)
}

fn denominator_nonzero(
    denominator: &str,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    if denominator.parse::<i128>().is_ok_and(|value| value != 0) {
        return true;
    }

    let ne_zero = format!("{denominator}!=0");
    let zero_ne = format!("0!={denominator}");
    let gt_zero = format!("{denominator}>0");
    let lt_zero = format!("{denominator}<0");
    if z3_proves_conclusion(&ne_zero, contracts, params, options).is_some_and(|proved| proved) {
        return true;
    }
    contracts.iter().any(|contract| {
        contract == &ne_zero || contract == &zero_ne || contract == &gt_zero || contract == &lt_zero
    })
}

fn z3_proves_conclusion(
    conclusion: &str,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> Option<bool> {
    if options.solver != SolverBackend::Z3 {
        return None;
    }

    let params = params
        .iter()
        .filter(|param| is_supported_integer(&param.ty))
        .map(|param| (param.name.clone(), param.ty.clone()))
        .collect::<Vec<_>>();
    match solver::prove_integer_predicate(contracts, conclusion, &params, options.timeout_ms) {
        ProofResult::Proved => Some(true),
        ProofResult::Unproved => Some(false),
        ProofResult::Unsupported => None,
    }
}

fn slice_index_obligation_proved(obligation: &SliceIndexObligation, contracts: &[String]) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
    let index_lt_len = format!("{}<{}.len()", obligation.index, obligation.base);
    let len_gt_index = format!("{}.len()>{}", obligation.base, obligation.index);

    contracts
        .iter()
        .any(|contract| contract == &index_lt_len || contract == &len_gt_index)
}

fn callee_precondition_proved(
    condition: &str,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    if contracts.iter().any(|contract| contract == condition) {
        return true;
    }

    if flipped_inequality(condition)
        .is_some_and(|flipped| contracts.iter().any(|contract| contract == &flipped))
    {
        return true;
    }

    z3_proves_conclusion(condition, contracts, params, options).is_some_and(|proved| proved)
}

fn executable_tokens(body: &str) -> Vec<String> {
    let tokens = tokens(body);
    let mut executable = Vec::new();
    let mut idx = 0;

    while idx < tokens.len() {
        if let Some(open_idx) = loop_spec_open_idx(&tokens, idx) {
            if let Some(close_idx) = matching_token_group(&tokens, open_idx, "{", "}") {
                idx = close_idx + 1;
                continue;
            }
        }

        executable.push(tokens[idx].clone());
        idx += 1;
    }

    compact_parenthesized_value_tokens(&executable)
}

fn compact_parenthesized_value_tokens(tokens: &[String]) -> Vec<String> {
    let mut compacted = Vec::new();
    let mut idx = 0;

    while idx < tokens.len() {
        if idx + 2 < tokens.len()
            && tokens[idx] == "("
            && tokens[idx + 2] == ")"
            && is_value_operand(&tokens[idx + 1])
        {
            compacted.push(tokens[idx + 1].clone());
            idx += 3;
            continue;
        }

        compacted.push(tokens[idx].clone());
        idx += 1;
    }

    compacted
}

fn param_type<'a>(name: &str, params: &'a [Param]) -> Option<&'a str> {
    params
        .iter()
        .find(|param| param.name == name && is_supported_integer(&param.ty))
        .map(|param| param.ty.as_str())
}

fn expression_needs_integer_proof(left: &str, right: &str, params: &[Param]) -> bool {
    param_type(left, params).is_some()
        || param_type(right, params).is_some()
        || (is_ident(left) && is_ident(right))
}

fn is_value_operand(token: &str) -> bool {
    is_ident(token) || token.parse::<i128>().is_ok()
}

fn field_expression_before(tokens: &[String], op_idx: usize) -> Option<String> {
    if op_idx < 3 || tokens.get(op_idx - 2) != Some(&".".to_string()) {
        return None;
    }
    let base = tokens.get(op_idx - 3)?;
    let field = tokens.get(op_idx - 1)?;
    if !is_ident(base) || !is_ident(field) {
        return None;
    }

    Some(format!("{base}.{field}"))
}

fn complex_binary_expression(tokens: &[String], op_idx: usize) -> Option<String> {
    let op = tokens.get(op_idx)?;
    if tokens.get(op_idx + 1) == Some(&"(".to_string()) {
        let close_idx = matching_token_group(tokens, op_idx + 1, "(", ")")?;
        let right = token_expression(&tokens[op_idx + 2..close_idx]);
        if !is_value_operand(&right) {
            let left = tokens.get(op_idx.checked_sub(1)?)?;
            return Some(format!("{left} {op} ({right})"));
        }
    }

    if op_idx > 0 && tokens.get(op_idx - 1) == Some(&")".to_string()) {
        let open_idx = matching_open_token_group(tokens, op_idx - 1, "(", ")")?;
        let left = token_expression(&tokens[open_idx + 1..op_idx - 1]);
        if !is_value_operand(&left) {
            let right = tokens.get(op_idx + 1)?;
            return Some(format!("({left}) {op} {right}"));
        }
    }

    None
}

fn complex_denominator_expression(
    tokens: &[String],
    op_idx: usize,
    op: &str,
) -> Option<(String, String)> {
    if tokens.get(op_idx + 1) != Some(&"(".to_string()) {
        return None;
    }

    let close_idx = matching_token_group(tokens, op_idx + 1, "(", ")")?;
    let denominator = token_expression(&tokens[op_idx + 2..close_idx]);
    if is_value_operand(&denominator) {
        return None;
    }

    let left = tokens.get(op_idx.checked_sub(1)?)?;
    Some((denominator.clone(), format!("{left} {op} ({denominator})")))
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

fn matching_open_token_group(
    tokens: &[String],
    close_idx: usize,
    open: &str,
    close: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, token) in tokens.iter().enumerate().take(close_idx + 1).rev() {
        if token == close {
            depth += 1;
        } else if token == open {
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
            trust_macro_version: "test".to_string(),
            module_id: "test-module".to_string(),
            item_kind: "total".to_string(),
            item_id: format!("total:{name}:test"),
            source_span: "test-span".to_string(),
            rust_function_path: name.to_string(),
            visibility: "public".to_string(),
            contracts_original: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
            contracts_normalized: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
            contract_classes: classes.iter().map(|class| class.to_string()).collect(),
            assertion_policy: "always".to_string(),
            function_source: function_source.to_string(),
            body_hash_placeholder: format!("{name}-hash"),
            trust_model_dependencies: Vec::new(),
        }
    }

    fn model_metadata(name: &str) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "test-module".to_string(),
            item_kind: "trust_model".to_string(),
            item_id: format!("model:{name}:test"),
            source_span: "test-span".to_string(),
            rust_function_path: name.to_string(),
            visibility: "public".to_string(),
            contracts_original: Vec::new(),
            contracts_normalized: Vec::new(),
            contract_classes: Vec::new(),
            assertion_policy: "always".to_string(),
            function_source: format!("pub struct {name} {{ pub balance: i64 }}"),
            body_hash_placeholder: format!("{name}-hash"),
            trust_model_dependencies: Vec::new(),
        }
    }

    fn proof_metadata(name: &str, function_source: &str, contracts: &[&str]) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "test-module".to_string(),
            item_kind: "proof".to_string(),
            item_id: format!("proof:{name}:test"),
            source_span: "test-span".to_string(),
            rust_function_path: name.to_string(),
            visibility: "private".to_string(),
            contracts_original: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
            contracts_normalized: contracts
                .iter()
                .map(|contract| contract.to_string())
                .collect(),
            contract_classes: vec!["gives ghost".to_string(); contracts.len()],
            assertion_policy: "always".to_string(),
            function_source: function_source.to_string(),
            body_hash_placeholder: format!("{name}-hash"),
            trust_model_dependencies: Vec::new(),
        }
    }

    #[test]
    fn proves_i32_add_one_from_executable_precondition() {
        let metadata = metadata("pub fn add_one(x: i32) -> i32 { x + 1 }", &["x < i32::MAX"]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn z3_proves_i32_add_one_from_strict_numeric_max() {
        let metadata = metadata_named(
            "add_one",
            "pub fn add_one(x: i32) -> i32 { x + 1 }",
            &["x < 2147483647"],
        );

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow { .. })
        ));
        assert_eq!(
            verify_totals_with_options(&[metadata], VerificationOptions::z3(5000)),
            Ok(())
        );
    }

    #[test]
    fn proves_proof_obligation_from_assert_step() {
        let proof = proof_metadata(
            "le_refl",
            "fn le_refl(a: i32) gives ghost { a <= a; } { assert(a <= a); }",
            &["a <= a"],
        );

        assert_eq!(verify_totals(&[proof]), Ok(()));
    }

    #[test]
    fn rejects_unproved_proof_obligation() {
        let proof = proof_metadata(
            "le_refl",
            "fn le_refl(a: i32, b: i32) gives ghost { a <= a; } { assert(a <= b); }",
            &["a <= a"],
        );

        assert_eq!(
            verify_totals(&[proof]),
            Err(VerificationError::ProofObligationUnproved {
                proof: "le_refl".to_string(),
                condition: "a <= a".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unsupported_proof_step() {
        let proof = proof_metadata(
            "le_refl",
            "fn le_refl(a: i32) gives ghost { a <= a; } { println!(\"runtime\"); assert(a <= a); }",
            &["a <= a"],
        );

        assert_eq!(
            verify_totals(&[proof]),
            Err(VerificationError::UnsupportedProofStep {
                proof: "le_refl".to_string(),
            })
        );
    }

    #[test]
    fn proves_i64_add_one_from_executable_precondition() {
        let metadata = metadata_named(
            "add_one_i64",
            "pub fn add_one_i64(x: i64) -> i64 { x + 1 }",
            &["x < i64::MAX"],
        );

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
    fn rejects_unproved_variable_addition() {
        let metadata = metadata_named("add", "pub fn add(x: i32, y: i32) -> i32 { x + y }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add".to_string(),
                expression: "x + y".to_string(),
            })
        );
    }

    #[test]
    fn rejects_parenthesized_variable_addition() {
        let metadata = metadata_named("add", "pub fn add(x: i32, y: i32) -> i32 { x + (y) }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add".to_string(),
                expression: "x + y".to_string(),
            })
        );
    }

    #[test]
    fn rejects_complex_parenthesized_addition_operand() {
        let metadata = metadata_named(
            "add",
            "pub fn add(x: i32, y: i32) -> i32 { x + (y + 1) }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add".to_string(),
                expression: "x + (y+1)".to_string(),
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
    fn z3_proves_i32_sub_one_from_strict_numeric_min() {
        let metadata = metadata_named(
            "sub_one",
            "pub fn sub_one(x: i32) -> i32 { x - 1 }",
            &["x > -2147483648"],
        );

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow { .. })
        ));
        assert_eq!(
            verify_totals_with_options(&[metadata], VerificationOptions::z3(5000)),
            Ok(())
        );
    }

    #[test]
    fn proves_field_subtraction_from_nonnegative_bound() {
        let account = model_metadata("Account");
        let withdraw = metadata_named(
            "withdraw",
            "pub fn withdraw(acct: Account, amount: i64) -> Account { Account { balance: acct.balance - amount } }",
            &["amount >= 0", "acct.balance >= amount"],
        );

        assert_eq!(verify_totals(&[account, withdraw]), Ok(()));
    }

    #[test]
    fn rejects_unproved_variable_subtraction() {
        let metadata = metadata_named("sub", "pub fn sub(x: i32, y: i32) -> i32 { x - y }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerSubtractionOverflow {
                function: "sub".to_string(),
                expression: "x - y".to_string(),
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
    fn z3_proves_i32_mul_two_from_strict_numeric_upper_bound() {
        let metadata = metadata_named(
            "double",
            "pub fn double(x: i32) -> i32 { x * 2 }",
            &["x < 1073741824", "x >= -1073741824"],
        );

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::IntegerMultiplicationOverflow { .. })
        ));
        assert_eq!(
            verify_totals_with_options(&[metadata], VerificationOptions::z3(5000)),
            Ok(())
        );
    }

    #[test]
    fn rejects_unproved_variable_multiplication() {
        let metadata = metadata_named("mul", "pub fn mul(x: i32, y: i32) -> i32 { x * y }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerMultiplicationOverflow {
                function: "mul".to_string(),
                expression: "x * y".to_string(),
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
    fn proves_integer_division_denominator_nonzero() {
        let metadata = metadata_named(
            "div",
            "pub fn div(x: i32, y: i32) -> i32 { x / y }",
            &["y != 0"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn z3_proves_integer_division_denominator_from_positive_lower_bound() {
        let metadata = metadata_named(
            "div",
            "pub fn div(x: i32, y: i32) -> i32 { x / y }",
            &["y >= 1"],
        );

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero { .. })
        ));
        assert_eq!(
            verify_totals_with_options(&[metadata], VerificationOptions::z3(5000)),
            Ok(())
        );
    }

    #[test]
    fn rejects_unproved_integer_division_denominator() {
        let metadata = metadata_named("div", "pub fn div(x: i32, y: i32) -> i32 { x / y }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero {
                function: "div".to_string(),
                expression: "x / y".to_string(),
            })
        );
    }

    #[test]
    fn rejects_parenthesized_integer_division_denominator() {
        let metadata = metadata_named("div", "pub fn div(x: i32, y: i32) -> i32 { x / (y) }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero {
                function: "div".to_string(),
                expression: "x / y".to_string(),
            })
        );
    }

    #[test]
    fn rejects_complex_parenthesized_integer_division_denominator() {
        let metadata = metadata_named(
            "div",
            "pub fn div(x: i32, y: i32) -> i32 { x / (y & 1) }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero {
                function: "div".to_string(),
                expression: "x / (y&1)".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unproved_integer_remainder_denominator() {
        let metadata = metadata_named("rem", "pub fn rem(x: i32, y: i32) -> i32 { x % y }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerRemainderByZero {
                function: "rem".to_string(),
                expression: "x % y".to_string(),
            })
        );
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
    fn rejects_unsupported_vec_index() {
        let metadata = metadata_named(
            "get_vec",
            "pub fn get_vec(xs: Vec<i32>, i: usize) -> i32 { xs[i] }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedIndex {
                function: "get_vec".to_string(),
                expression: "xs[i]".to_string(),
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
    fn z3_proves_callee_precondition_from_equivalent_numeric_bound() {
        let inc = metadata_named(
            "inc",
            "pub fn inc(x: i32) -> i32 { x + 1 }",
            &["x < i32::MAX"],
        );
        let caller = metadata_named(
            "caller",
            "pub fn caller(x: i32) -> i32 { inc(x) }",
            &["x < 2147483647"],
        );

        assert!(matches!(
            verify_totals(&[inc.clone(), caller.clone()]),
            Err(VerificationError::CalleePreconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_options(&[inc, caller], VerificationOptions::z3(5000)),
            Ok(())
        );
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
    fn rejects_loop_without_spec() {
        let metadata = metadata_named(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { while n > 0 { n = n - 1; } n }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopMissingSpec {
                function: "countdown".to_string(),
            })
        );
    }

    #[test]
    fn rejects_loop_spec_without_loop() {
        let metadata = metadata_named(
            "id",
            "pub fn id(n: usize) -> usize { trust::loop_spec! { decreases(n); } n }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopMissingSpec {
                function: "id".to_string(),
            })
        );
    }

    #[test]
    fn rejects_two_specs_one_loop() {
        let metadata = metadata_named(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } trust::loop_spec! { decreases(n); } while n > 0 { n = n - 1; } n }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopAmbiguousSpec {
                function: "countdown".to_string(),
            })
        );
    }

    #[test]
    fn rejects_loop_break() {
        let metadata = metadata_named(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { break; } n }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedLoopControl {
                function: "countdown".to_string(),
                keyword: "break".to_string(),
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
            &[
                "amount >= 0",
                "acct.balance >= amount",
                "out.id == old(acct.id)",
            ],
            &["given executable", "given executable", "gives ghost"],
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
    fn rejects_unchecked_result_expect() {
        let metadata = metadata_named(
            "bad_expect",
            "pub fn bad_expect(x: Result<i32, i32>) -> i32 { x.expect(\"ok\") }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UncheckedUnwrap {
                function: "bad_expect".to_string(),
            })
        );
    }

    #[test]
    fn rejects_ordinary_rust_call() {
        let metadata = metadata_named(
            "call_helper",
            "pub fn call_helper(x: i32) -> i32 { helper(x) }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedCall {
                function: "call_helper".to_string(),
                callee: "helper".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unknown_method_call() {
        let metadata = metadata_named("abs", "pub fn abs(x: i32) -> i32 { x.abs() }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedCall {
                function: "abs".to_string(),
                callee: "x.abs".to_string(),
            })
        );
    }

    #[test]
    fn allows_slice_len_method() {
        let metadata = metadata_named("len", "pub fn len(xs: &[i32]) -> usize { xs.len() }", &[]);

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_closure_body() {
        let metadata = metadata_named(
            "apply",
            "pub fn apply(x: i32) -> i32 { let inc = |n: i32| n + 1; inc(x) }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedClosure {
                function: "apply".to_string(),
            })
        );
    }

    #[test]
    fn rejects_recursive_total_call() {
        let recurse = metadata_named(
            "recurse",
            "pub fn recurse(x: i32) -> i32 { recurse(x) }",
            &[],
        );

        assert_eq!(
            verify_totals(&[recurse]),
            Err(VerificationError::UnsupportedCall {
                function: "recurse".to_string(),
                callee: "recurse".to_string(),
            })
        );
    }

    #[test]
    fn rejects_explicit_panic() {
        let metadata = metadata_named("fail", "pub fn fail() -> i32 { panic!(\"boom\") }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::ExplicitPanic {
                function: "fail".to_string(),
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
    fn semantic_return_expression_can_prove_postcondition() {
        let metadata = metadata_named_with_classes(
            "id_i32",
            "pub fn id_i32(x: i32) -> i32 { 0 }",
            &["out == x"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "id_i32".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_field_return_expression_can_prove_postcondition() {
        let account = model_metadata("Account");
        let metadata = metadata_named_with_classes(
            "balance",
            "pub fn balance(acct: Account) -> i64 { let out = acct.balance; out }",
            &["out == acct.balance"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "balance".to_string(),
            params: vec![SemanticParam {
                name: "acct".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            return_expression: Some("acct.balance".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: vec![SemanticFieldAccess {
                base: "acct".to_string(),
                field: "balance".to_string(),
                owner_type: "Account".to_string(),
                field_type: "i64".to_string(),
                expression: "acct.balance".to_string(),
            }],
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_totals(&[account.clone(), metadata.clone()]),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(
                &[account, metadata],
                &[semantics],
                VerificationOptions::default()
            ),
            Ok(())
        );
    }

    #[test]
    fn semantic_match_arms_can_prove_postcondition() {
        let metadata = metadata_named_with_classes(
            "maybe_zero",
            "pub fn maybe_zero(x: Option<i32>) -> i32 { match x { Some(_) => 0, None => 0 } }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "maybe_zero".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "Option<i32>".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
                        return_expression: Some("0".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Some".to_string(),
                        discriminant: "1".to_string(),
                        payload: None,
                        return_expression: Some("0".to_string()),
                    },
                ],
            }],
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_match_requires_reachable_arms_to_prove_postcondition() {
        let metadata = metadata_named_with_classes(
            "unwrap_or_zero",
            "pub fn unwrap_or_zero(x: Option<i32>) -> i32 { match x { Some(v) => v, None => 0 } }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "unwrap_or_zero".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "Option<i32>".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
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
                        return_expression: Some("v".to_string()),
                    },
                ],
            }],
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
    }

    #[test]
    fn semantic_match_precondition_can_restrict_option_arm() {
        let metadata = metadata_named_with_classes(
            "unwrap_or_zero",
            "pub fn unwrap_or_zero(x: Option<i32>) -> i32 { match x { Some(v) => v, None => 0 } }",
            &["x == None", "out == 0"],
            &["given executable", "gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "unwrap_or_zero".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "Option<i32>".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: vec![SemanticMatch {
                scrutinee: "x".to_string(),
                scrutinee_type: "Option<i32>".to_string(),
                arms: vec![
                    SemanticMatchArm {
                        variant: "None".to_string(),
                        discriminant: "0".to_string(),
                        payload: None,
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
                        return_expression: Some("v".to_string()),
                    },
                ],
            }],
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_branch_arms_can_prove_postcondition() {
        let metadata = metadata_named_with_classes(
            "zero_for_any_i32",
            "pub fn zero_for_any_i32(x: i32) -> i32 { if x > 0 { 0 } else { 0 } }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "zero_for_any_i32".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "x > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x > 0".to_string(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x <= 0".to_string(),
                        return_expression: Some("0".to_string()),
                    },
                ],
            }],
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_branch_guard_assumption_can_prove_postcondition_with_z3() {
        let metadata = metadata_named_with_classes(
            "zero_or_self",
            "pub fn zero_or_self(x: i32) -> i32 { if x == 0 { 0 } else { x } }",
            &["out == x"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "zero_or_self".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "x == 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "x == 0".to_string(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        return_expression: Some("x".to_string()),
                    },
                ],
            }],
        };

        assert!(matches!(
            verify_totals_with_semantics(
                &[metadata.clone()],
                &[semantics.clone()],
                VerificationOptions::default()
            ),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::z3(5000)),
            Ok(())
        );
    }

    #[test]
    fn semantic_arithmetic_catches_block_addition_overflow() {
        let metadata = metadata_named(
            "add_one",
            "pub fn add_one(x: i32) -> i32 { x + { 1 } }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "add_one".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: Some("x + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: Vec::new(),
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add_one".to_string(),
                expression: "x + 1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_branch_guard_proves_checked_addition() {
        let metadata = metadata_named(
            "add_if_safe",
            "pub fn add_if_safe(x: i32) -> i32 { if x < i32::MAX { x + 1 } else { x } }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "add_if_safe".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                left: "x".to_string(),
                right: Some("1".to_string()),
                expression: "x + 1".to_string(),
                guards: vec!["x < i32::MAX".to_string()],
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_arithmetic_catches_block_division_by_zero() {
        let metadata = metadata_named(
            "divide",
            "pub fn divide(x: i32, y: i32) -> i32 { x / { y } }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "divide".to_string(),
            params: vec![
                SemanticParam {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                },
                SemanticParam {
                    name: "y".to_string(),
                    ty: "i32".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            return_expression: Some("x / y".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                left: "x".to_string(),
                right: Some("y".to_string()),
                expression: "x / y".to_string(),
                guards: Vec::new(),
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::IntegerDivisionByZero {
                function: "divide".to_string(),
                expression: "x / y".to_string(),
            })
        );
    }

    #[test]
    fn semantic_arithmetic_proves_block_division_precondition() {
        let metadata = metadata_named(
            "divide",
            "pub fn divide(x: i32, y: i32) -> i32 { x / { y } }",
            &["y != 0"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "divide".to_string(),
            params: vec![
                SemanticParam {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                },
                SemanticParam {
                    name: "y".to_string(),
                    ty: "i32".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            return_expression: Some("x / y".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                left: "x".to_string(),
                right: Some("y".to_string()),
                expression: "x / y".to_string(),
                guards: Vec::new(),
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_slice_index_proves_block_index_precondition() {
        let metadata = metadata_named(
            "get",
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[{ i }] }",
            &["i < xs.len()"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "get".to_string(),
            params: vec![
                SemanticParam {
                    name: "xs".to_string(),
                    ty: "&[i32]".to_string(),
                },
                SemanticParam {
                    name: "i".to_string(),
                    ty: "usize".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            return_expression: Some("xs[i]".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                index: "i".to_string(),
                expression: "xs[i]".to_string(),
                guards: Vec::new(),
            }],
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::SliceIndexOutOfBounds { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_branch_guard_proves_slice_index_bound() {
        let metadata = metadata_named(
            "get_or_zero",
            "pub fn get_or_zero(xs: &[i32], i: usize) -> i32 { if i < xs.len() { xs[i] } else { 0 } }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "get_or_zero".to_string(),
            params: vec![
                SemanticParam {
                    name: "xs".to_string(),
                    ty: "&[i32]".to_string(),
                },
                SemanticParam {
                    name: "i".to_string(),
                    ty: "usize".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                index: "i".to_string(),
                expression: "xs[i]".to_string(),
                guards: vec!["i < xs.len()".to_string()],
            }],
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_total(&metadata),
            Err(VerificationError::SliceIndexOutOfBounds { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_call_proves_block_argument_callee_precondition() {
        let inc = metadata_named(
            "inc",
            "pub fn inc(x: i32) -> i32 { x + 1 }",
            &["x < i32::MAX"],
        );
        let caller = metadata_named(
            "caller",
            "pub fn caller(x: i32) -> i32 { inc({ x }) }",
            &["x < i32::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "caller".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: Some("inc(x)".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_totals(&[inc.clone(), caller.clone()]),
            Err(VerificationError::CalleePreconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(
                &[inc, caller],
                &[semantics],
                VerificationOptions::default()
            ),
            Ok(())
        );
    }

    #[test]
    fn semantic_branch_guard_proves_call_precondition() {
        let inc = metadata_named(
            "inc",
            "pub fn inc(x: i32) -> i32 { x + 1 }",
            &["x < i32::MAX"],
        );
        let caller = metadata_named(
            "caller",
            "pub fn caller(x: i32) -> i32 { if x < i32::MAX { inc(x) } else { x } }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "caller".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "inc".to_string(),
                args: vec!["x".to_string()],
                guards: vec!["x < i32::MAX".to_string()],
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_totals(&[inc.clone(), caller.clone()]),
            Err(VerificationError::CalleePreconditionUnproved { .. })
        ));
        assert_eq!(
            verify_totals_with_semantics(
                &[inc, caller],
                &[semantics],
                VerificationOptions::default()
            ),
            Ok(())
        );
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
