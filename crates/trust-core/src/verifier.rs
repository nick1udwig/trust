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
    pub local_types: Vec<String>,
    pub contract_bindings: Vec<SemanticContractBinding>,
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
pub struct SemanticContractBinding {
    pub expression: String,
    pub name: String,
    pub kind: SemanticContractBindingKind,
    pub ty: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticContractBindingKind {
    Param,
    Result,
    Field,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticArithmeticOperation {
    pub kind: SemanticArithmeticKind,
    pub ty: Option<String>,
    pub target: Option<String>,
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
    pub base_type: String,
    pub index: String,
    pub index_type: String,
    pub element_type: String,
    pub expression: String,
    pub guards: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticCall {
    pub callee: String,
    pub args: Vec<String>,
    pub guards: Vec<String>,
    pub trust_callee: Option<SemanticTrustCallee>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticTrustCallee {
    pub rust_function_path: String,
    pub params: Vec<SemanticParam>,
    pub preconditions: Vec<String>,
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
    pub assumptions: Vec<String>,
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
    pub assumptions: Vec<String>,
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
    IntegerDivisionOverflow {
        function: String,
        expression: String,
    },
    IntegerRemainderOverflow {
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
    UnsupportedType {
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
    LoopInvariantNotEstablished {
        function: String,
        invariant: String,
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
    SemanticExtractionIncomplete {
        function: String,
        category: String,
        expression: String,
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
            VerificationError::IntegerDivisionOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer division cannot overflow in `{function}`: `{expression}`"
            ),
            VerificationError::IntegerRemainderOverflow {
                function,
                expression,
            } => write!(
                f,
                "could not prove integer remainder cannot overflow in `{function}`: `{expression}`"
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
            VerificationError::UnsupportedType { function, ty } => {
                write!(f, "unsupported type in `{function}`: `{ty}`")
            }
            VerificationError::LoopMissingSpec { function } => {
                write!(f, "loop in `{function}` requires loop_spec")
            }
            VerificationError::LoopAmbiguousSpec { function } => {
                write!(f, "multiple loop_spec blocks before loop in `{function}`")
            }
            VerificationError::LoopMissingDecreases { function: _ } => {
                write!(f, "loop in total function requires decreases measure")
            }
            VerificationError::LoopInvariantNotEstablished {
                function: _,
                invariant: _,
            } => write!(f, "loop invariant may not hold before loop entry"),
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
            VerificationError::SemanticExtractionIncomplete {
                function,
                category,
                expression,
            } => write!(
                f,
                "rustc semantic extraction did not cover {category} in `{function}`: `{expression}`"
            ),
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
    let contracts = executable_preconditions(metadata);
    let given_contracts = given_preconditions(metadata);
    let params = verification_params(&source, semantics);
    let return_type = verification_return_type(&source, semantics);
    let local_types = verification_local_types(semantics);
    if let Some(ty) = unsupported_function_type(&params, &return_type, &local_types, model_types) {
        return Err(VerificationError::UnsupportedType {
            function: metadata.rust_function_path.clone(),
            ty,
        });
    }
    if let Some(ty) = opaque_contract_type(&params, &return_type, metadata, model_types) {
        return Err(VerificationError::UnsupportedType {
            function: metadata.rust_function_path.clone(),
            ty,
        });
    }
    let value_params = verification_value_params(&params, semantics);
    let raw_body = cfg_selected_body(body(&metadata.function_source), options);
    let body = normalize(&raw_body);
    let raw_body = raw_body.as_str();
    let semantic_return_expression = semantics
        .and_then(|semantics| semantics.return_expression.as_deref())
        .map(normalize);
    let loop_facts = verify_loops(
        raw_body,
        &metadata.rust_function_path,
        semantics,
        &given_contracts,
        &value_params,
        options,
    )?;
    if !postconditions(metadata).is_empty() {
        if let Some(expression) = semantic_loop_exit_extraction_gap(&loop_facts, semantics) {
            return Err(VerificationError::SemanticExtractionIncomplete {
                function: metadata.rust_function_path.clone(),
                category: "loop exit".to_string(),
                expression,
            });
        }
    }
    let loop_postcondition_facts = loop_postcondition_facts(&loop_facts, semantics.is_some());
    let postcondition_assumptions =
        contracts_with_assumptions(&given_contracts, &loop_postcondition_facts);
    let call_env = verification_call_env(env, semantics);

    if contains_unchecked_unwrap(raw_body) || contains_semantic_unchecked_unwrap(semantics) {
        return Err(VerificationError::UncheckedUnwrap {
            function: metadata.rust_function_path.clone(),
        });
    }
    if contains_explicit_panic(raw_body) || contains_semantic_explicit_panic(semantics) {
        return Err(VerificationError::ExplicitPanic {
            function: metadata.rust_function_path.clone(),
        });
    }
    if contains_closure(raw_body) || contains_semantic_closure(semantics) {
        return Err(VerificationError::UnsupportedClosure {
            function: metadata.rust_function_path.clone(),
        });
    }
    let token_unsupported_call = unsupported_call(
        raw_body,
        &params,
        &call_env,
        &metadata.rust_function_path,
        semantics,
    );
    if let Some(callee) = unsupported_semantic_call(semantics, env, &metadata.rust_function_path) {
        let callee = token_unsupported_call.unwrap_or(callee);
        return Err(VerificationError::UnsupportedCall {
            function: metadata.rust_function_path.clone(),
            callee,
        });
    }
    if let Some(callee) = token_unsupported_call {
        if semantics.is_some() {
            return Err(VerificationError::SemanticExtractionIncomplete {
                function: metadata.rust_function_path.clone(),
                category: "unsupported call".to_string(),
                expression: callee,
            });
        }
        return Err(VerificationError::UnsupportedCall {
            function: metadata.rust_function_path.clone(),
            callee,
        });
    }
    if let Some(expression) =
        semantic_arithmetic_extraction_gap(raw_body, &value_params, semantics, options)
    {
        return Err(VerificationError::SemanticExtractionIncomplete {
            function: metadata.rust_function_path.clone(),
            category: "arithmetic operation".to_string(),
            expression,
        });
    }
    if let Some(expression) = semantic_slice_index_extraction_gap(raw_body, &params, semantics) {
        return Err(VerificationError::SemanticExtractionIncomplete {
            function: metadata.rust_function_path.clone(),
            category: "slice index".to_string(),
            expression,
        });
    }
    if let Some(expression) = semantic_call_extraction_gap(raw_body, &call_env, semantics) {
        return Err(VerificationError::SemanticExtractionIncomplete {
            function: metadata.rust_function_path.clone(),
            category: "Trust call".to_string(),
            expression,
        });
    }
    if let Some(expression) = semantic_field_access_extraction_gap(raw_body, &params, semantics) {
        return Err(VerificationError::SemanticExtractionIncomplete {
            function: metadata.rust_function_path.clone(),
            category: "field access".to_string(),
            expression,
        });
    }

    for obligation in verification_field_access_obligations(raw_body, &params, semantics) {
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

    for obligation in verification_addition_obligations(raw_body, &value_params, semantics, options)
    {
        if !addition_obligation_proved(&obligation, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerAdditionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in
        verification_subtraction_obligations(raw_body, &value_params, semantics, options)
    {
        if !subtraction_obligation_proved(&obligation, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerSubtractionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_negation_obligations(raw_body, &value_params, semantics, options)
    {
        if !negation_obligation_proved(&obligation, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerNegationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in
        verification_multiplication_obligations(raw_body, &value_params, semantics, options)
    {
        if !multiplication_obligation_proved(&obligation, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerMultiplicationOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_division_obligations(raw_body, &params, semantics) {
        let contracts = contracts_with_assumptions(&contracts, &obligation.assumptions);
        if !denominator_nonzero(&obligation.denominator, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerDivisionByZero {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_remainder_obligations(raw_body, &params, semantics) {
        let contracts = contracts_with_assumptions(&contracts, &obligation.assumptions);
        if !denominator_nonzero(&obligation.denominator, &contracts, &value_params, options) {
            return Err(VerificationError::IntegerRemainderByZero {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in
        verification_division_overflow_obligations(raw_body, &value_params, semantics, options)
    {
        if !signed_division_overflow_obligation_proved(
            &obligation,
            &contracts,
            &value_params,
            options,
        ) {
            return Err(VerificationError::IntegerDivisionOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in
        verification_remainder_overflow_obligations(raw_body, &value_params, semantics, options)
    {
        if !signed_division_overflow_obligation_proved(
            &obligation,
            &contracts,
            &value_params,
            options,
        ) {
            return Err(VerificationError::IntegerRemainderOverflow {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    if let Some(expression) = unsupported_index_expression(raw_body, &params, semantics) {
        return Err(VerificationError::UnsupportedIndex {
            function: metadata.rust_function_path.clone(),
            expression,
        });
    }

    for obligation in verification_slice_index_obligations(raw_body, &params, semantics) {
        if !slice_index_obligation_proved(&obligation, &contracts) {
            return Err(VerificationError::SliceIndexOutOfBounds {
                function: metadata.rust_function_path.clone(),
                expression: obligation.expression,
            });
        }
    }

    for obligation in verification_call_obligations(raw_body, &call_env, semantics) {
        let contracts = contracts_with_assumptions(&given_contracts, &obligation.assumptions);
        if !callee_precondition_proved(&obligation.condition, &contracts, &value_params, options) {
            return Err(VerificationError::CalleePreconditionUnproved {
                function: metadata.rust_function_path.clone(),
                callee: obligation.callee,
                condition: obligation.condition,
            });
        }
    }

    for postcondition in postconditions(metadata) {
        match postcondition_proved(
            &postcondition,
            &body,
            raw_body,
            semantic_return_expression.as_deref(),
            semantics,
            &postcondition_assumptions,
            &value_params,
            options,
        ) {
            PostconditionProof::Proved => {}
            PostconditionProof::Unproved => {
                return Err(VerificationError::PostconditionUnproved {
                    function: metadata.rust_function_path.clone(),
                    condition: postcondition.original,
                });
            }
            PostconditionProof::SemanticExtractionIncomplete { expression } => {
                return Err(VerificationError::SemanticExtractionIncomplete {
                    function: metadata.rust_function_path.clone(),
                    category: "return expression".to_string(),
                    expression,
                });
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PostconditionProof {
    Proved,
    Unproved,
    SemanticExtractionIncomplete { expression: String },
}

fn postcondition_proof(proved: bool) -> PostconditionProof {
    if proved {
        PostconditionProof::Proved
    } else {
        PostconditionProof::Unproved
    }
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
struct DivOverflowObligation {
    left: String,
    right: String,
    ty: String,
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
struct TokenFieldAccess {
    ty: String,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopFact {
    condition: String,
    invariants: Vec<String>,
    semantic_exit_facts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenSegment {
    tokens: Vec<String>,
    assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopSpec {
    invariant: Option<String>,
    decreases: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopMeasureProof {
    Proved,
    Unproved,
    SemanticExtractionIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopInvariantProof {
    Proved,
    Unproved,
    SemanticExtractionIncomplete,
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
        .filter(|item| item.item_kind == "total" || is_executable_spec(item))
        .map(|item| {
            let source = normalize(&item.function_source);
            TrustFunctionSummary {
                name: item.rust_function_path.clone(),
                params: verification_params(&source, semantic_for(item, semantics)),
                preconditions: if item.item_kind == "total" {
                    given_preconditions(item)
                } else {
                    Vec::new()
                },
            }
        })
        .collect()
}

fn is_executable_spec(item: &TrustMetadata) -> bool {
    item.item_kind == "spec" && item.item_id.starts_with("spec:executable:")
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

fn verification_return_type(source: &str, semantics: Option<&TrustFunctionSemantics>) -> String {
    semantics
        .map(|semantics| semantics.return_type.clone())
        .unwrap_or_else(|| parse_return_type(source))
}

fn verification_local_types(semantics: Option<&TrustFunctionSemantics>) -> Vec<String> {
    semantics
        .map(|semantics| semantics.local_types.clone())
        .unwrap_or_default()
}

fn parse_return_type(source: &str) -> String {
    let Some(params_start) = source.find('(') else {
        return "()".to_string();
    };
    let Some(params_end) = source[params_start + 1..].find(')') else {
        return "()".to_string();
    };
    let after_params = &source[params_start + 1 + params_end + 1..];
    let Some(after_arrow) = after_params.strip_prefix("->") else {
        return "()".to_string();
    };
    after_arrow
        .split_once('{')
        .map(|(ty, _body)| ty)
        .unwrap_or(after_arrow)
        .trim()
        .to_string()
}

fn unsupported_function_type(
    params: &[Param],
    return_type: &str,
    local_types: &[String],
    model_types: &[String],
) -> Option<String> {
    params
        .iter()
        .find_map(|param| unsupported_mvp_type(&param.ty, model_types))
        .or_else(|| unsupported_mvp_type(return_type, model_types))
        .or_else(|| {
            local_types
                .iter()
                .find_map(|ty| unsupported_mvp_type(ty, model_types))
        })
}

fn unsupported_mvp_type(ty: &str, model_types: &[String]) -> Option<String> {
    match mvp_type_support(ty, model_types) {
        MvpTypeSupport::Unsupported(ty) => Some(ty),
        MvpTypeSupport::Supported | MvpTypeSupport::Opaque => None,
    }
}

fn opaque_contract_type(
    params: &[Param],
    return_type: &str,
    metadata: &TrustMetadata,
    model_types: &[String],
) -> Option<String> {
    let contract_tokens = metadata
        .contracts_normalized
        .iter()
        .flat_map(|contract| tokens(contract))
        .collect::<Vec<_>>();
    if contract_tokens.is_empty() {
        return None;
    }

    if contract_tokens.iter().any(|token| token == "out")
        && mvp_type_support(return_type, model_types) == MvpTypeSupport::Opaque
    {
        return Some(return_type.trim().to_string());
    }

    params.iter().find_map(|param| {
        if contract_tokens.iter().any(|token| token == &param.name)
            && mvp_type_support(&param.ty, model_types) == MvpTypeSupport::Opaque
        {
            Some(param.ty.trim().to_string())
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MvpTypeSupport {
    Supported,
    Opaque,
    Unsupported(String),
}

fn mvp_type_support(ty: &str, model_types: &[String]) -> MvpTypeSupport {
    let ty = ty.trim();
    if ty.is_empty()
        || ty == "()"
        || ty == "bool"
        || is_supported_integer(ty)
        || model_types
            .iter()
            .any(|model_type| model_type == &type_name_tail(ty))
    {
        return MvpTypeSupport::Supported;
    }
    if matches!(ty, "f32" | "f64")
        || ty.starts_with("*const ")
        || ty.starts_with("*mut ")
        || ty.starts_with("&mut ")
        || ty.starts_with("dyn ")
        || ty.contains(" dyn ")
    {
        return MvpTypeSupport::Unsupported(ty.to_string());
    }
    if let Some(element) = slice_element_type(ty) {
        return mvp_type_support(element, model_types);
    }
    if let Some(inner) = single_type_arg(ty, "Option").or_else(|| single_type_arg(ty, "Some")) {
        return mvp_type_support(inner, model_types);
    }
    if let Some(args) = type_args(ty, "Result") {
        return combine_type_support(
            split_type_args(args)
                .into_iter()
                .map(|arg| mvp_type_support(arg, model_types)),
        );
    }

    MvpTypeSupport::Opaque
}

fn combine_type_support(supports: impl IntoIterator<Item = MvpTypeSupport>) -> MvpTypeSupport {
    let mut saw_opaque = false;
    for support in supports {
        match support {
            MvpTypeSupport::Unsupported(ty) => return MvpTypeSupport::Unsupported(ty),
            MvpTypeSupport::Opaque => saw_opaque = true,
            MvpTypeSupport::Supported => {}
        }
    }
    if saw_opaque {
        MvpTypeSupport::Opaque
    } else {
        MvpTypeSupport::Supported
    }
}

fn single_type_arg<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    type_args(ty, name).and_then(|args| {
        let args = split_type_args(args);
        (args.len() == 1).then_some(args[0])
    })
}

fn type_args<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let ty = ty.trim();
    let args = ty
        .strip_prefix(name)
        .or_else(|| ty.strip_prefix(&format!("core::option::{name}")))
        .or_else(|| ty.strip_prefix(&format!("std::option::{name}")))
        .or_else(|| ty.strip_prefix(&format!("core::result::{name}")))
        .or_else(|| ty.strip_prefix(&format!("std::result::{name}")))?;
    args.strip_prefix('<')?.strip_suffix('>').map(str::trim)
}

fn split_type_args(args: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0usize;
    for (idx, ch) in args.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            ',' if angle_depth == 0 => {
                let part = args[start..idx].trim();
                if !part.is_empty() {
                    parts.push(part);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let part = args[start..].trim();
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn verification_value_params(
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<Param> {
    let mut value_params = params.to_vec();
    let Some(semantics) = semantics else {
        return value_params;
    };

    for field in &semantics.field_accesses {
        if value_params
            .iter()
            .any(|param| param.name == field.expression)
        {
            continue;
        }
        value_params.push(Param {
            name: field.expression.clone(),
            ty: field.field_type.clone(),
        });
    }
    for binding in &semantics.contract_bindings {
        if !matches!(
            binding.kind,
            SemanticContractBindingKind::Field | SemanticContractBindingKind::Local
        ) || value_params.iter().any(|param| param.name == binding.name)
        {
            continue;
        }
        value_params.push(Param {
            name: binding.name.clone(),
            ty: binding.ty.clone(),
        });
    }

    value_params
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
) -> PostconditionProof {
    let return_expression = return_expression(body);
    let semantic_return_proved = semantic_return_proves_postcondition(
        postcondition,
        raw_body,
        semantic_return_expression,
        contracts,
        params,
        options,
    );
    if semantic_return_proved == Some(true) {
        return PostconditionProof::Proved;
    }
    if let Some(proved) = semantic_match_proves_postcondition(
        postcondition,
        raw_body,
        semantics,
        contracts,
        params,
        options,
    ) {
        return postcondition_proof(proved);
    }
    if let Some(proved) = semantic_branch_proves_postcondition(
        postcondition,
        raw_body,
        semantics,
        contracts,
        params,
        options,
    ) {
        return postcondition_proof(proved);
    }
    if semantic_return_proved == Some(false) {
        return PostconditionProof::Unproved;
    }

    let fallback_proved = postcondition_proved_by_return_expression_with_assumptions(
        postcondition,
        raw_body,
        &return_expression,
        contracts,
        params,
        options,
        semantics.is_none(),
    );
    if fallback_proved && semantics.is_some() {
        return PostconditionProof::SemanticExtractionIncomplete {
            expression: return_expression,
        };
    }

    postcondition_proof(fallback_proved)
}

fn semantic_return_proves_postcondition(
    postcondition: &Contract,
    raw_body: &str,
    semantic_return_expression: Option<&str>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> Option<bool> {
    semantic_return_expression.map(|return_expression| {
        postcondition_proved_by_return_expression_with_assumptions(
            postcondition,
            raw_body,
            return_expression,
            contracts,
            params,
            options,
            false,
        )
    })
}

fn semantic_match_proves_postcondition(
    postcondition: &Contract,
    raw_body: &str,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> Option<bool> {
    let mut complete_match_seen = false;

    if let Some(semantics) = semantics {
        for semantic_match in &semantics.matches {
            if !semantic_match_is_exhaustive(semantic_match) {
                continue;
            }

            let reachable_arms = semantic_match
                .arms
                .iter()
                .filter(|arm| semantic_match_arm_reachable(semantic_match, arm, contracts))
                .collect::<Vec<_>>();
            if reachable_arms.is_empty()
                || reachable_arms
                    .iter()
                    .any(|arm| arm.return_expression.is_none())
            {
                continue;
            }

            complete_match_seen = true;
            if reachable_arms.iter().all(|arm| {
                arm.return_expression
                    .as_ref()
                    .is_some_and(|return_expression| {
                        let return_expression = normalize(return_expression);
                        let arm_contracts = contracts_with_assumptions(
                            contracts,
                            &semantic_arm_assumptions(&arm.assumptions),
                        );
                        postcondition_proved_by_return_expression_with_assumptions(
                            postcondition,
                            raw_body,
                            &return_expression,
                            &arm_contracts,
                            params,
                            options,
                            false,
                        )
                    })
            }) {
                return Some(true);
            }
        }
    }

    complete_match_seen.then_some(false)
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
    }) && semantic_arm_assumptions(&arm.assumptions)
        .iter()
        .all(|assumption| {
            !contracts
                .iter()
                .any(|contract| semantic_condition_excluded_by_contract(assumption, contract))
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
) -> Option<bool> {
    let mut complete_branch_seen = false;

    if let Some(semantics) = semantics {
        for branch in &semantics.branches {
            let reachable_arms = branch
                .arms
                .iter()
                .filter(|arm| semantic_branch_arm_reachable(arm, contracts))
                .collect::<Vec<_>>();
            if reachable_arms.is_empty()
                || reachable_arms
                    .iter()
                    .any(|arm| arm.return_expression.is_none())
            {
                continue;
            }

            complete_branch_seen = true;
            if reachable_arms.iter().all(|arm| {
                arm.return_expression
                    .as_ref()
                    .is_some_and(|return_expression| {
                        let return_expression = normalize(return_expression);
                        let branch_contracts = contracts_with_assumptions(
                            contracts,
                            &semantic_branch_arm_assumptions(arm),
                        );
                        postcondition_proved_by_return_expression_with_assumptions(
                            postcondition,
                            raw_body,
                            &return_expression,
                            &branch_contracts,
                            params,
                            options,
                            false,
                        )
                    })
            }) {
                return Some(true);
            }
        }
    }

    complete_branch_seen.then_some(false)
}

fn semantic_branch_arm_reachable(arm: &SemanticBranchArm, contracts: &[String]) -> bool {
    semantic_branch_arm_assumptions(arm)
        .iter()
        .all(|assumption| {
            !contracts
                .iter()
                .any(|contract| semantic_condition_excluded_by_contract(assumption, contract))
        })
}

fn semantic_branch_arm_assumptions(arm: &SemanticBranchArm) -> Vec<String> {
    let mut assumptions = vec![arm.guard.clone()];
    assumptions.extend(arm.assumptions.iter().cloned());
    semantic_arm_assumptions(&assumptions)
}

fn semantic_arm_assumptions(assumptions: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for assumption in assumptions {
        push_unique(&mut normalized, normalize(assumption));
        if let Some(reversed) = reversed_condition(assumption) {
            push_unique(&mut normalized, normalize(&reversed));
        }
    }
    normalized
}

fn semantic_condition_excluded_by_contract(condition: &str, contract: &str) -> bool {
    condition_negates(condition, contract)
}

fn negated_condition(condition: &str) -> Option<String> {
    let condition = canonical_condition(condition);
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

fn conditions_equivalent(left: &str, right: &str) -> bool {
    let left = canonical_condition(left);
    let right = canonical_condition(right);

    left == right
        || reversed_condition(&left).as_deref() == Some(right.as_str())
        || reversed_condition(&right).as_deref() == Some(left.as_str())
}

fn condition_negates(condition: &str, other: &str) -> bool {
    condition_variants(condition).iter().any(|variant| {
        negated_condition(variant)
            .as_deref()
            .is_some_and(|negated| conditions_equivalent(negated, other))
    })
}

fn condition_variants(condition: &str) -> Vec<String> {
    let mut variants = vec![canonical_condition(condition)];
    if let Some(reversed) = reversed_condition(condition) {
        push_unique(&mut variants, reversed);
    }
    variants
}

fn canonical_condition(condition: &str) -> String {
    let mut condition = normalize(condition);
    while let Some(inner) = strip_balanced_outer_parentheses(&condition) {
        condition = inner.to_string();
    }
    condition
}

fn strip_balanced_outer_parentheses(condition: &str) -> Option<&str> {
    if !condition.starts_with('(') || !condition.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (idx, ch) in condition.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            _ => {}
        }

        if depth == 0 && idx + ch.len_utf8() < condition.len() {
            return None;
        }
    }

    Some(&condition[1..condition.len() - 1])
}

fn postcondition_proved_by_return_expression(
    postcondition: &Contract,
    raw_body: &str,
    return_expression: &str,
    allow_token_loop_exit: bool,
) -> bool {
    let Some((left, right)) = postcondition.normalized.split_once("==") else {
        return false;
    };

    (left == "out" && right == return_expression)
        || (right == "out" && left == return_expression)
        || (allow_token_loop_exit
            && left == "out"
            && loop_exit_proves_value(raw_body, &return_expression, right))
        || (allow_token_loop_exit
            && right == "out"
            && loop_exit_proves_value(raw_body, &return_expression, left))
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
    allow_token_loop_exit: bool,
) -> bool {
    if postcondition_proved_by_return_expression(
        postcondition,
        raw_body,
        return_expression,
        allow_token_loop_exit,
    ) {
        return true;
    }
    if postcondition_proved_by_assumptions(postcondition, return_expression, contracts, params) {
        return true;
    }

    let conclusion = substitute_out(&postcondition.normalized, return_expression);
    z3_proves_conclusion(&conclusion, contracts, params, options).is_some_and(|proved| proved)
}

fn postcondition_proved_by_assumptions(
    postcondition: &Contract,
    return_expression: &str,
    contracts: &[String],
    params: &[Param],
) -> bool {
    let Some((left, right)) = postcondition.normalized.split_once("==") else {
        return false;
    };

    if left == "out" {
        return expression_equals_expected_from_assumptions(
            return_expression,
            right,
            contracts,
            params,
        );
    }
    if right == "out" {
        return expression_equals_expected_from_assumptions(
            return_expression,
            left,
            contracts,
            params,
        );
    }

    false
}

fn expression_equals_expected_from_assumptions(
    expression: &str,
    expected: &str,
    contracts: &[String],
    params: &[Param],
) -> bool {
    if contracts_prove_equality(expression, expected, contracts, params) {
        return true;
    }
    if expected != "0" {
        return false;
    }
    let Some(ty) = param_type(expression, params) else {
        return false;
    };
    if !is_unsigned_integer(ty) {
        return false;
    }

    contracts
        .iter()
        .any(|contract| contract_proves_unsigned_zero_upper_bound(expression, contract))
}

fn contracts_prove_equality(
    expression: &str,
    expected: &str,
    contracts: &[String],
    params: &[Param],
) -> bool {
    if expression == expected {
        return true;
    }
    if !is_known_postcondition_term(expected, params) {
        return false;
    }

    let eq = format!("{expression}=={expected}");
    if contracts
        .iter()
        .any(|contract| conditions_equivalent(contract, &eq))
    {
        return true;
    }

    contracts_prove_order(expression, "<=", expected, contracts)
        && contracts_prove_order(expression, ">=", expected, contracts)
}

fn is_known_postcondition_term(term: &str, params: &[Param]) -> bool {
    integer_literal_value(term).is_some() || param_type(term, params).is_some()
}

fn contracts_prove_order(left: &str, op: &str, right: &str, contracts: &[String]) -> bool {
    let condition = format!("{left}{op}{right}");
    contracts
        .iter()
        .any(|contract| conditions_equivalent(contract, &condition))
}

fn contract_proves_unsigned_zero_upper_bound(expression: &str, contract: &str) -> bool {
    let expected = [
        format!("{expression}==0"),
        format!("0=={expression}"),
        format!("{expression}<=0"),
        format!("0>={expression}"),
        format!("{expression}<1"),
        format!("1>{expression}"),
    ];

    expected.iter().any(|expected| contract == expected)
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
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            addition_obligations_from_tokens(&segment.tokens, params, &segment.assumptions)
        })
        .collect()
}

fn addition_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    assumptions: &[String],
) -> Vec<AddObligation> {
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
                assumptions: assumptions.to_vec(),
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
                assumptions: assumptions.to_vec(),
            });
        } else if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
            obligations.push(AddObligation {
                variable: right.clone(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression,
                assumptions: assumptions.to_vec(),
            });
        } else if expression_needs_integer_proof(left, right, params) {
            obligations.push(AddObligation {
                variable: left.clone(),
                ty: param_type(left, params)
                    .or_else(|| param_type(right, params))
                    .map(|ty| ty.to_string()),
                constant: None,
                expression,
                assumptions: assumptions.to_vec(),
            });
        }
    }

    obligations
}

fn subtraction_obligations(body: &str, params: &[Param]) -> Vec<SubObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            subtraction_obligations_from_tokens(&segment.tokens, params, &segment.assumptions)
        })
        .collect()
}

fn subtraction_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    assumptions: &[String],
) -> Vec<SubObligation> {
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
                assumptions: assumptions.to_vec(),
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
                    assumptions: assumptions.to_vec(),
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
                assumptions: assumptions.to_vec(),
            });
        }
    }

    obligations
}

fn negation_obligations(body: &str, params: &[Param]) -> Vec<NegObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            negation_obligations_from_tokens(&segment.tokens, params, &segment.assumptions)
        })
        .collect()
}

fn negation_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    assumptions: &[String],
) -> Vec<NegObligation> {
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
            assumptions: assumptions.to_vec(),
        });
    }

    obligations
}

fn multiplication_obligations(body: &str, params: &[Param]) -> Vec<MulObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            multiplication_obligations_from_tokens(&segment.tokens, params, &segment.assumptions)
        })
        .collect()
}

fn multiplication_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    assumptions: &[String],
) -> Vec<MulObligation> {
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
                assumptions: assumptions.to_vec(),
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
                    assumptions: assumptions.to_vec(),
                });
            }
        } else if let (Ok(constant), Some(ty)) = (left.parse::<i128>(), param_type(right, params)) {
            if constant > 1 {
                obligations.push(MulObligation {
                    variable: right.clone(),
                    ty: Some(ty.to_string()),
                    constant: Some(constant),
                    expression,
                    assumptions: assumptions.to_vec(),
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
                assumptions: assumptions.to_vec(),
            });
        }
    }

    obligations
}

fn verification_addition_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    options: VerificationOptions,
) -> Vec<AddObligation> {
    let semantic_obligations =
        semantic_addition_obligations(semantics, params, options.target_pointer_width);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        addition_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_addition_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
    target_pointer_width: Option<u32>,
) -> Vec<AddObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Add)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_addition_obligation(
                &operation.left,
                right,
                &operation.expression,
                params,
                operation.ty.as_deref(),
                target_pointer_width,
            )
            .map(|mut obligation| {
                obligation.assumptions = semantic_guard_assumptions(operation);
                obligation
            })
        })
        .collect()
}

fn verification_subtraction_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    options: VerificationOptions,
) -> Vec<SubObligation> {
    let semantic_obligations =
        semantic_subtraction_obligations(semantics, params, options.target_pointer_width);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        subtraction_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_subtraction_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
    target_pointer_width: Option<u32>,
) -> Vec<SubObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Sub)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_subtraction_obligation(
                &operation.left,
                right,
                &operation.expression,
                params,
                operation.ty.as_deref(),
                target_pointer_width,
            )
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
    options: VerificationOptions,
) -> Vec<NegObligation> {
    let semantic_obligations =
        semantic_negation_obligations(semantics, params, options.target_pointer_width);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        negation_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_negation_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
    target_pointer_width: Option<u32>,
) -> Vec<NegObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Neg)
        .filter_map(|operation| {
            let ty = param_type(&operation.left, params)
                .or_else(|| supported_operation_type(operation.ty.as_deref()))?;
            if !is_signed_integer(ty) {
                return None;
            }
            if integer_constant_value(&operation.left, Some(ty), target_pointer_width)
                .is_some_and(|value| min_value(ty) != Some(value))
            {
                return None;
            }
            Some(NegObligation {
                variable: operation.left.clone(),
                ty: ty.to_string(),
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
    options: VerificationOptions,
) -> Vec<MulObligation> {
    let semantic_obligations =
        semantic_multiplication_obligations(semantics, params, options.target_pointer_width);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        multiplication_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_multiplication_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
    target_pointer_width: Option<u32>,
) -> Vec<MulObligation> {
    semantic_arithmetic_operations(semantics, SemanticArithmeticKind::Mul)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            semantic_multiplication_obligation(
                &operation.left,
                right,
                &operation.expression,
                params,
                operation.ty.as_deref(),
                target_pointer_width,
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
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        division_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn verification_remainder_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<DenominatorObligation> {
    let semantic_obligations = semantic_remainder_obligations(semantics);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        remainder_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn verification_division_overflow_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    options: VerificationOptions,
) -> Vec<DivOverflowObligation> {
    verification_signed_division_overflow_obligations(
        body,
        params,
        semantics,
        SemanticArithmeticKind::Div,
        "/",
        options.target_pointer_width,
    )
}

fn verification_remainder_overflow_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    options: VerificationOptions,
) -> Vec<DivOverflowObligation> {
    verification_signed_division_overflow_obligations(
        body,
        params,
        semantics,
        SemanticArithmeticKind::Rem,
        "%",
        options.target_pointer_width,
    )
}

fn verification_signed_division_overflow_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    kind: SemanticArithmeticKind,
    op: &str,
    target_pointer_width: Option<u32>,
) -> Vec<DivOverflowObligation> {
    let semantic_obligations = semantic_signed_division_overflow_obligations(
        semantics,
        params,
        kind,
        target_pointer_width,
    );
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        signed_division_overflow_obligations(body, params, op, target_pointer_width),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_arithmetic_extraction_gap(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
    options: VerificationOptions,
) -> Option<String> {
    semantics?;

    semantic_obligation_gap(
        &semantic_addition_obligations(semantics, params, options.target_pointer_width),
        addition_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    )
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_subtraction_obligations(semantics, params, options.target_pointer_width),
            subtraction_obligations(body, params),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_negation_obligations(semantics, params, options.target_pointer_width),
            negation_obligations(body, params),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_multiplication_obligations(semantics, params, options.target_pointer_width),
            multiplication_obligations(body, params),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_division_obligations(semantics),
            division_obligations(body, params),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_remainder_obligations(semantics),
            remainder_obligations(body, params),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_signed_division_overflow_obligations(
                semantics,
                params,
                SemanticArithmeticKind::Div,
                options.target_pointer_width,
            ),
            signed_division_overflow_obligations(body, params, "/", options.target_pointer_width),
            |obligation| obligation.expression.as_str(),
        )
    })
    .or_else(|| {
        semantic_obligation_gap(
            &semantic_signed_division_overflow_obligations(
                semantics,
                params,
                SemanticArithmeticKind::Rem,
                options.target_pointer_width,
            ),
            signed_division_overflow_obligations(body, params, "%", options.target_pointer_width),
            |obligation| obligation.expression.as_str(),
        )
    })
}

fn semantic_obligation_gap<T>(
    semantic_obligations: &[T],
    fallback_obligations: Vec<T>,
    expression: impl Fn(&T) -> &str,
) -> Option<String> {
    let semantic_expressions = semantic_obligations
        .iter()
        .map(|obligation| normalize(expression(obligation)))
        .collect::<Vec<_>>();

    fallback_obligations
        .into_iter()
        .map(|obligation| expression(&obligation).to_string())
        .filter(|expression| !expression.contains('{') && !expression.contains('}'))
        .find(|fallback_expression| {
            let normalized = normalize(fallback_expression);
            !semantic_expressions
                .iter()
                .any(|semantic_expression| semantic_expression == &normalized)
        })
}

fn semantic_signed_division_overflow_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
    kind: SemanticArithmeticKind,
    target_pointer_width: Option<u32>,
) -> Vec<DivOverflowObligation> {
    semantic_arithmetic_operations(semantics, kind)
        .filter_map(|operation| {
            let right = operation.right.as_ref()?;
            signed_division_overflow_obligation(
                &operation.left,
                right,
                &operation.expression,
                params,
                operation.ty.as_deref(),
                target_pointer_width,
            )
            .map(|mut obligation| {
                obligation.assumptions = semantic_guard_assumptions(operation);
                obligation
            })
        })
        .collect()
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
    let mut assumptions = Vec::new();
    for guard in &operation.guards {
        push_unique(&mut assumptions, normalize(guard));
        if let Some(reversed) = reversed_condition(guard) {
            push_unique(&mut assumptions, normalize(&reversed));
        }
    }
    assumptions
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
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

fn signed_division_overflow_obligation(
    left: &str,
    right: &str,
    expression: &str,
    params: &[Param],
    operation_ty: Option<&str>,
    target_pointer_width: Option<u32>,
) -> Option<DivOverflowObligation> {
    let operation_ty = supported_operation_type(operation_ty).filter(|ty| is_signed_integer(ty));
    let ty = param_type(left, params)
        .filter(|ty| is_signed_integer(ty))
        .or_else(|| param_type(right, params).filter(|ty| is_signed_integer(ty)))
        .or(operation_ty)?;
    let min = min_value(ty)?;

    if integer_constant_value(left, Some(ty), target_pointer_width)
        .is_some_and(|value| value != min)
    {
        return None;
    }
    if integer_constant_value(right, Some(ty), target_pointer_width)
        .is_some_and(|value| value != -1)
    {
        return None;
    }

    Some(DivOverflowObligation {
        left: left.to_string(),
        right: right.to_string(),
        ty: ty.to_string(),
        expression: expression.to_string(),
        assumptions: Vec::new(),
    })
}

fn semantic_addition_obligation(
    left: &str,
    right: &str,
    expression: &str,
    params: &[Param],
    operation_ty: Option<&str>,
    target_pointer_width: Option<u32>,
) -> Option<AddObligation> {
    let operation_ty = supported_operation_type(operation_ty);
    if typed_constant_binary_is_safe(
        left,
        right,
        operation_ty,
        i128::checked_add,
        target_pointer_width,
    ) {
        return None;
    }

    let left_ty = param_type(left, params).or(operation_ty);
    let right_ty = param_type(right, params).or(operation_ty);
    if let (Some(ty), Some(constant)) = (
        left_ty,
        integer_constant_value(right, left_ty, target_pointer_width),
    ) {
        return Some(AddObligation {
            variable: left.to_string(),
            ty: Some(ty.to_string()),
            constant: Some(constant),
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }
    if let (Some(constant), Some(ty)) = (
        integer_constant_value(left, right_ty, target_pointer_width),
        right_ty,
    ) {
        return Some(AddObligation {
            variable: right.to_string(),
            ty: Some(ty.to_string()),
            constant: Some(constant),
            expression: expression.to_string(),
            assumptions: Vec::new(),
        });
    }
    if expression_needs_integer_proof(left, right, params) || operation_ty.is_some() {
        return Some(AddObligation {
            variable: left.to_string(),
            ty: param_type(left, params)
                .or_else(|| param_type(right, params))
                .or(operation_ty)
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
    operation_ty: Option<&str>,
    target_pointer_width: Option<u32>,
) -> Option<SubObligation> {
    let operation_ty = supported_operation_type(operation_ty);
    if typed_constant_binary_is_safe(
        left,
        right,
        operation_ty,
        i128::checked_sub,
        target_pointer_width,
    ) {
        return None;
    }

    let left_ty = param_type(left, params).or(operation_ty);
    if let (Some(ty), Some(constant)) = (
        left_ty,
        integer_constant_value(right, left_ty, target_pointer_width),
    ) {
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
    } else if expression_needs_integer_proof(left, right, params) || operation_ty.is_some() {
        return Some(SubObligation {
            variable: left.to_string(),
            ty: param_type(left, params)
                .or(operation_ty)
                .map(str::to_string),
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
    operation_ty: Option<&str>,
    target_pointer_width: Option<u32>,
) -> Option<MulObligation> {
    let operation_ty = supported_operation_type(operation_ty);
    if typed_constant_binary_is_safe(
        left,
        right,
        operation_ty,
        i128::checked_mul,
        target_pointer_width,
    ) {
        return None;
    }

    let left_ty = param_type(left, params).or(operation_ty);
    let right_ty = param_type(right, params).or(operation_ty);
    if let (Some(ty), Some(constant)) = (
        left_ty,
        integer_constant_value(right, left_ty, target_pointer_width),
    ) {
        if constant > 1 {
            return Some(MulObligation {
                variable: left.to_string(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression: expression.to_string(),
                assumptions: Vec::new(),
            });
        }
    } else if let (Some(constant), Some(ty)) = (
        integer_constant_value(left, right_ty, target_pointer_width),
        right_ty,
    ) {
        if constant > 1 {
            return Some(MulObligation {
                variable: right.to_string(),
                ty: Some(ty.to_string()),
                constant: Some(constant),
                expression: expression.to_string(),
                assumptions: Vec::new(),
            });
        }
    } else if expression_needs_integer_proof(left, right, params) || operation_ty.is_some() {
        return Some(MulObligation {
            variable: left.to_string(),
            ty: param_type(left, params)
                .or_else(|| param_type(right, params))
                .or(operation_ty)
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

fn signed_division_overflow_obligations(
    body: &str,
    params: &[Param],
    op: &str,
    target_pointer_width: Option<u32>,
) -> Vec<DivOverflowObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            signed_division_overflow_obligations_from_tokens(
                &segment.tokens,
                params,
                op,
                target_pointer_width,
                &segment.assumptions,
            )
        })
        .collect()
}

fn signed_division_overflow_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    op: &str,
    target_pointer_width: Option<u32>,
    assumptions: &[String],
) -> Vec<DivOverflowObligation> {
    let mut obligations = Vec::new();

    for op_idx in 1..tokens.len().saturating_sub(1) {
        if tokens[op_idx] != op {
            continue;
        }
        let Some((left, _left_start)) = simple_signed_value_operand_before(&tokens, op_idx) else {
            continue;
        };
        let Some((right, _right_end)) = simple_signed_value_operand_after(&tokens, op_idx + 1)
        else {
            continue;
        };
        let expression = format!("{left} {op} {right}");
        if let Some(mut obligation) = signed_division_overflow_obligation(
            &left,
            &right,
            &expression,
            params,
            None,
            target_pointer_width,
        ) {
            obligation.assumptions = assumptions.to_vec();
            obligations.push(obligation);
        }
    }

    obligations
}

fn denominator_obligations(body: &str, params: &[Param], op: &str) -> Vec<DenominatorObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            denominator_obligations_from_tokens(&segment.tokens, params, op, &segment.assumptions)
        })
        .collect()
}

fn denominator_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    op: &str,
    assumptions: &[String],
) -> Vec<DenominatorObligation> {
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
                assumptions: assumptions.to_vec(),
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
                assumptions: assumptions.to_vec(),
            });
        }
    }

    obligations
}

fn slice_index_obligations(body: &str, params: &[Param]) -> Vec<SliceIndexObligation> {
    executable_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            slice_index_obligations_from_tokens(&segment.tokens, params, &segment.assumptions)
        })
        .collect()
}

fn slice_index_obligations_from_tokens(
    tokens: &[String],
    params: &[Param],
    assumptions: &[String],
) -> Vec<SliceIndexObligation> {
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

        let index = simple_grouped_value_expression(&tokens[idx + 2..end]);
        obligations.push(SliceIndexObligation {
            base: base.clone(),
            index: index.clone(),
            expression: format!("{base}[{index}]"),
            assumptions: assumptions.to_vec(),
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
    if semantics.is_some() {
        return semantic_obligations;
    }
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        slice_index_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    );
    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| existing.expression == fallback.expression,
    )
}

fn semantic_slice_index_obligations(
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<SliceIndexObligation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.slice_indexes.iter())
        .filter(|index| semantic_slice_index_supported(index))
        .map(|index| SliceIndexObligation {
            base: index.base.clone(),
            index: index.index.clone(),
            expression: index.expression.clone(),
            assumptions: index.guards.iter().map(|guard| normalize(guard)).collect(),
        })
        .collect()
}

fn semantic_slice_index_extraction_gap(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
    let semantics = semantics?;

    if let Some(index) = semantics
        .slice_indexes
        .iter()
        .find(|index| !semantic_slice_index_supported(index))
    {
        return Some(index.expression.clone());
    }

    semantic_obligation_gap(
        &semantic_slice_index_obligations(Some(semantics)),
        slice_index_obligations(body, params),
        |obligation| obligation.expression.as_str(),
    )
}

fn semantic_slice_index_supported(index: &SemanticSliceIndex) -> bool {
    index.index_type == "usize"
        && slice_element_type(&index.base_type)
            .is_some_and(|element_type| element_type == index.element_type)
}

fn unsupported_index_expression(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
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
        let index = simple_grouped_value_expression(&tokens[idx + 2..end]);
        let expression = format!("{base}[{index}]");
        if !is_read_only_slice_param(base, params)
            && !supported_semantic_index_expression(&expression, semantics)
        {
            return Some(expression);
        }
        idx = end + 1;
    }

    None
}

fn supported_semantic_index_expression(
    expression: &str,
    semantics: Option<&TrustFunctionSemantics>,
) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics
            .slice_indexes
            .iter()
            .any(|index| semantic_slice_index_supported(index) && index.expression == expression)
    })
}

fn field_access_obligations(body: &str, params: &[Param]) -> Vec<FieldAccessObligation> {
    token_field_accesses(body, params)
        .into_iter()
        .map(|access| FieldAccessObligation { ty: access.ty })
        .collect()
}

fn token_field_accesses(body: &str, params: &[Param]) -> Vec<TokenFieldAccess> {
    let tokens = tokens(body);
    let mut accesses = Vec::new();

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
        accesses.push(TokenFieldAccess {
            ty: param.ty.clone(),
            expression: format!("{base}.{field}"),
        });
    }

    accesses
}

fn verification_field_access_obligations(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<FieldAccessObligation> {
    let semantic_obligations = semantic_field_access_obligations(semantics);
    if semantics.is_some() {
        return semantic_obligations;
    }
    extend_unique_by(
        semantic_obligations,
        field_access_obligations(body, params),
        |existing, fallback| existing.ty == fallback.ty,
    )
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

fn semantic_field_access_extraction_gap(
    body: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
    let semantics = semantics?;
    let semantic_accesses = semantics
        .field_accesses
        .iter()
        .map(|field| {
            (
                normalize(&field.expression),
                type_name_tail(&field.owner_type),
            )
        })
        .collect::<Vec<_>>();

    token_field_accesses(body, params)
        .into_iter()
        .find(|access| {
            let expression = normalize(&access.expression);
            let ty = type_name_tail(&access.ty);
            !semantic_accesses
                .iter()
                .any(|(semantic_expression, owner_type)| {
                    semantic_expression == &expression || owner_type == &ty
                })
        })
        .map(|access| access.expression)
}

fn verify_loops(
    body: &str,
    function: &str,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> Result<Vec<LoopFact>, VerificationError> {
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
        let prefix = &tokens[..idx];
        let invariants = verify_loop_spec(
            &spec, &condition, loop_body, prefix, function, semantics, contracts, params, options,
        )?;
        facts.push(LoopFact {
            invariants,
            semantic_exit_facts: semantic_loop_exit_facts(semantics, &condition),
            condition,
        });
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
    prefix: &[String],
    function: &str,
    semantics: Option<&TrustFunctionSemantics>,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> Result<Vec<String>, VerificationError> {
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

    let mut invariants = Vec::new();
    if let Some(invariant) = &spec.invariant {
        match loop_invariant_preserved(invariant, condition, loop_body, semantics, params) {
            LoopInvariantProof::Proved => {}
            LoopInvariantProof::Unproved => {
                return Err(VerificationError::LoopInvariantNotPreserved {
                    function: function.to_string(),
                    invariant: invariant.clone(),
                });
            }
            LoopInvariantProof::SemanticExtractionIncomplete => {
                return Err(VerificationError::SemanticExtractionIncomplete {
                    function: function.to_string(),
                    category: "loop invariant".to_string(),
                    expression: invariant.clone(),
                });
            }
        }
        push_unique(&mut invariants, canonical_condition(invariant));
    }

    if !loop_measure_nonnegative(measure, spec.invariant.as_deref(), params) {
        return Err(VerificationError::LoopDecreasesNotDecreasing {
            function: function.to_string(),
            measure: measure.clone(),
        });
    }

    match loop_measure_decreases(measure, condition, loop_body, semantics) {
        LoopMeasureProof::Proved => {}
        LoopMeasureProof::Unproved => {
            return Err(VerificationError::LoopDecreasesNotDecreasing {
                function: function.to_string(),
                measure: measure.clone(),
            });
        }
        LoopMeasureProof::SemanticExtractionIncomplete => {
            return Err(VerificationError::SemanticExtractionIncomplete {
                function: function.to_string(),
                category: "loop decreases".to_string(),
                expression: measure.clone(),
            });
        }
    }

    if let Some(invariant) = &spec.invariant {
        if !loop_invariant_established(invariant, prefix, contracts, params, options) {
            return Err(VerificationError::LoopInvariantNotEstablished {
                function: function.to_string(),
                invariant: invariant.clone(),
            });
        }
    }

    Ok(invariants)
}

fn loop_invariant_preserved(
    invariant: &str,
    condition: &str,
    loop_body: &[String],
    semantics: Option<&TrustFunctionSemantics>,
    params: &[Param],
) -> LoopInvariantProof {
    let Some((left, op, right)) = comparison_parts(invariant) else {
        return LoopInvariantProof::Unproved;
    };

    if invariant_is_unsigned_nonnegative(&left, &op, &right, params) {
        return LoopInvariantProof::Proved;
    }

    if op == "<=" && conditions_equivalent(condition, &format!("{left}<{right}")) {
        if let Some(amount) = semantic_increment_amount(semantics, condition, &left) {
            return if amount <= 1 {
                LoopInvariantProof::Proved
            } else {
                LoopInvariantProof::Unproved
            };
        }
        if let Some(amount) = increment_amount(loop_body, &left) {
            return token_loop_invariant_proof(amount <= 1, semantics);
        }
        return token_loop_invariant_proof(
            !tokens_assign_to_any(loop_body, &[left.as_str(), right.as_str()]),
            semantics,
        );
    }

    if op == ">=" && conditions_equivalent(condition, &format!("{left}>{right}")) {
        if let Some(amount) = semantic_decrement_amount(semantics, condition, &left) {
            return if amount <= 1 {
                LoopInvariantProof::Proved
            } else {
                LoopInvariantProof::Unproved
            };
        }
        if let Some(amount) = decrement_amount(loop_body, &left) {
            return token_loop_invariant_proof(amount <= 1, semantics);
        }
        return token_loop_invariant_proof(
            !tokens_assign_to_any(loop_body, &[left.as_str(), right.as_str()]),
            semantics,
        );
    }

    token_loop_invariant_proof(
        !tokens_assign_to_any(loop_body, &[left.as_str(), right.as_str()]),
        semantics,
    )
}

fn token_loop_invariant_proof(
    proved: bool,
    semantics: Option<&TrustFunctionSemantics>,
) -> LoopInvariantProof {
    if !proved {
        return LoopInvariantProof::Unproved;
    }
    if semantics.is_some() {
        LoopInvariantProof::SemanticExtractionIncomplete
    } else {
        LoopInvariantProof::Proved
    }
}

fn loop_invariant_established(
    invariant: &str,
    prefix: &[String],
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let initialized = substitute_simple_initial_values(invariant, prefix, params);
    condition_proved(&initialized, contracts, params, options)
}

fn loop_measure_nonnegative(measure: &str, invariant: Option<&str>, params: &[Param]) -> bool {
    if integer_constant_value(measure, None, None).is_some_and(|value| value >= 0) {
        return true;
    }
    if param_type(measure, params).is_some_and(is_unsigned_integer) {
        return true;
    }

    let Some((left, right)) = measure.split_once('-') else {
        return false;
    };
    let left = left.trim();
    let right = right.trim();
    let Some(invariant) = invariant else {
        return false;
    };

    conditions_equivalent(invariant, &format!("{right}<={left}"))
        || conditions_equivalent(invariant, &format!("{left}>={right}"))
}

fn condition_proved(
    condition: &str,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let condition = canonical_condition(condition);
    if contracts
        .iter()
        .any(|contract| conditions_equivalent(contract, &condition))
    {
        return true;
    }
    if condition_is_trivially_true(&condition, params) {
        return true;
    }

    z3_proves_conclusion(&condition, contracts, params, options).is_some_and(|proved| proved)
}

fn condition_is_trivially_true(condition: &str, params: &[Param]) -> bool {
    let Some((left, op, right)) = comparison_parts(condition) else {
        return false;
    };
    if left == right && matches!(op.as_str(), "==" | "<=" | ">=") {
        return true;
    }
    if let (Some(left), Some(right)) = (integer_literal_value(&left), integer_literal_value(&right))
    {
        return compare_integer_values(left, &op, right);
    }

    invariant_is_unsigned_nonnegative(&left, &op, &right, params)
}

fn invariant_is_unsigned_nonnegative(left: &str, op: &str, right: &str, params: &[Param]) -> bool {
    match op {
        ">=" | ">" => {
            param_type(left, params).is_some_and(is_unsigned_integer)
                && integer_literal_value(right).is_some_and(|value| value <= 0)
        }
        "<=" | "<" => {
            param_type(right, params).is_some_and(is_unsigned_integer)
                && integer_literal_value(left).is_some_and(|value| value <= 0)
        }
        _ => false,
    }
}

fn compare_integer_values(left: i128, op: &str, right: i128) -> bool {
    match op {
        "==" => left == right,
        "!=" => left != right,
        "<=" => left <= right,
        ">=" => left >= right,
        "<" => left < right,
        ">" => left > right,
        _ => false,
    }
}

fn comparison_parts(condition: &str) -> Option<(String, String, String)> {
    let condition = canonical_condition(condition);
    for op in ["<=", ">=", "!=", "==", "<", ">"] {
        let Some((left, right)) = condition.split_once(op) else {
            continue;
        };
        return Some((
            left.trim().to_string(),
            op.to_string(),
            right.trim().to_string(),
        ));
    }

    None
}

fn substitute_simple_initial_values(
    condition: &str,
    prefix: &[String],
    params: &[Param],
) -> String {
    token_expression(
        &tokens(condition)
            .into_iter()
            .map(|token| {
                if is_ident(&token) && param_type(&token, params).is_none() {
                    simple_initial_value(prefix, &token).unwrap_or(token)
                } else {
                    token
                }
            })
            .collect::<Vec<_>>(),
    )
}

fn simple_initial_value(tokens: &[String], variable: &str) -> Option<String> {
    let mut value = None;
    let mut idx = 0;

    while idx < tokens.len() {
        if tokens[idx] == "let" {
            let name_idx = if tokens.get(idx + 1).is_some_and(|token| token == "mut") {
                idx + 2
            } else {
                idx + 1
            };
            if tokens.get(name_idx).is_some_and(|name| name == variable)
                && tokens.get(name_idx + 1).is_some_and(|token| token == "=")
            {
                value = simple_assigned_expression(tokens, name_idx + 1);
            }
            idx += 1;
            continue;
        }

        if tokens.get(idx).is_some_and(|token| token == variable) {
            match tokens.get(idx + 1).map(String::as_str) {
                Some("=") => value = simple_assigned_expression(tokens, idx + 1),
                Some("+") | Some("-") if tokens.get(idx + 2).is_some_and(|token| token == "=") => {
                    value = None;
                }
                _ => {}
            }
        }
        idx += 1;
    }

    value
}

fn simple_assigned_expression(tokens: &[String], equals_idx: usize) -> Option<String> {
    let start = equals_idx + 1;
    let end = tokens[start..]
        .iter()
        .position(|token| token == ";")
        .map(|offset| start + offset)
        .unwrap_or(tokens.len());
    let expression = compact_parenthesized_value_tokens(&tokens[start..end]);
    if expression.len() == 1 && is_value_operand(&expression[0]) {
        return Some(expression[0].clone());
    }

    None
}

fn tokens_assign_to_any(tokens: &[String], variables: &[&str]) -> bool {
    tokens.iter().enumerate().any(|(idx, token)| {
        variables.iter().any(|variable| token == variable)
            && (tokens.get(idx + 1).is_some_and(|next| next == "=")
                || (matches!(
                    tokens.get(idx + 1).map(String::as_str),
                    Some("+") | Some("-")
                ) && tokens.get(idx + 2).is_some_and(|next| next == "=")))
    })
}

fn integer_literal_value(value: &str) -> Option<i128> {
    value.parse::<i128>().ok()
}

fn loop_measure_decreases(
    measure: &str,
    condition: &str,
    loop_body: &[String],
    semantics: Option<&TrustFunctionSemantics>,
) -> LoopMeasureProof {
    if semantic_measure_decreases(measure, condition, semantics) {
        return LoopMeasureProof::Proved;
    }

    if token_measure_decreases(measure, loop_body) {
        return if semantics.is_some() {
            LoopMeasureProof::SemanticExtractionIncomplete
        } else {
            LoopMeasureProof::Proved
        };
    }

    LoopMeasureProof::Unproved
}

fn semantic_measure_decreases(
    measure: &str,
    condition: &str,
    semantics: Option<&TrustFunctionSemantics>,
) -> bool {
    if semantic_decrements_variable(semantics, condition, measure) {
        return true;
    }

    let Some((left, right)) = measure.split_once('-') else {
        return false;
    };
    let left = left.trim();
    let right = right.trim();
    semantic_decrements_variable(semantics, condition, left)
        || semantic_increment_amount(semantics, condition, right).is_some()
}

fn token_measure_decreases(measure: &str, loop_body: &[String]) -> bool {
    if decrements_variable(loop_body, measure) {
        return true;
    }

    let Some((left, right)) = measure.split_once('-') else {
        return false;
    };
    let left = left.trim();
    let right = right.trim();
    decrements_variable(loop_body, left) || increment_amount(loop_body, right).is_some()
}

fn semantic_decrements_variable(
    semantics: Option<&TrustFunctionSemantics>,
    condition: &str,
    variable: &str,
) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics.arithmetic_operations.iter().any(|operation| {
            operation.kind == SemanticArithmeticKind::Sub
                && operation.target.as_deref() == Some(variable)
                && operation.left == variable
                && operation
                    .right
                    .as_deref()
                    .and_then(|right| right.parse::<i128>().ok())
                    .is_some_and(|amount| amount > 0)
                && semantic_operation_guarded_by(operation, condition)
        })
    })
}

fn semantic_increment_amount(
    semantics: Option<&TrustFunctionSemantics>,
    condition: &str,
    variable: &str,
) -> Option<i128> {
    semantics.into_iter().find_map(|semantics| {
        semantics
            .arithmetic_operations
            .iter()
            .filter(|operation| {
                operation.kind == SemanticArithmeticKind::Add
                    && operation.target.as_deref() == Some(variable)
                    && operation.left == variable
                    && semantic_operation_guarded_by(operation, condition)
            })
            .find_map(|operation| {
                operation
                    .right
                    .as_deref()
                    .and_then(|right| right.parse::<i128>().ok())
            })
    })
}

fn semantic_decrement_amount(
    semantics: Option<&TrustFunctionSemantics>,
    condition: &str,
    variable: &str,
) -> Option<i128> {
    semantics.into_iter().find_map(|semantics| {
        semantics
            .arithmetic_operations
            .iter()
            .filter(|operation| {
                operation.kind == SemanticArithmeticKind::Sub
                    && operation.target.as_deref() == Some(variable)
                    && operation.left == variable
                    && semantic_operation_guarded_by(operation, condition)
            })
            .find_map(|operation| {
                operation
                    .right
                    .as_deref()
                    .and_then(|right| right.parse::<i128>().ok())
            })
    })
}

fn semantic_operation_guarded_by(operation: &SemanticArithmeticOperation, condition: &str) -> bool {
    operation
        .guards
        .iter()
        .any(|guard| conditions_equivalent(guard, condition))
}

fn reversed_condition(condition: &str) -> Option<String> {
    let condition = canonical_condition(condition);
    for (op, reversed) in [
        ("<=", ">="),
        (">=", "<="),
        ("!=", "!="),
        ("==", "=="),
        ("<", ">"),
        (">", "<"),
    ] {
        let Some((left, right)) = condition.split_once(op) else {
            continue;
        };
        return Some(format!("{}{}{}", right.trim(), reversed, left.trim()));
    }

    None
}

fn loop_postcondition_facts(facts: &[LoopFact], semantic_available: bool) -> Vec<String> {
    let mut postcondition_facts = Vec::new();

    for fact in facts {
        for invariant in &fact.invariants {
            push_unique(&mut postcondition_facts, canonical_condition(invariant));
        }
        if !semantic_available {
            if let Some(exit_fact) = loop_exit_fact(&fact.condition) {
                push_unique(&mut postcondition_facts, exit_fact);
            }
        }
        for exit_fact in &fact.semantic_exit_facts {
            push_unique(&mut postcondition_facts, canonical_condition(exit_fact));
        }
    }

    postcondition_facts
}

fn semantic_loop_exit_facts(
    semantics: Option<&TrustFunctionSemantics>,
    condition: &str,
) -> Vec<String> {
    let mut facts = Vec::new();
    let Some(semantics) = semantics else {
        return facts;
    };

    for branch in &semantics.branches {
        if !conditions_equivalent(&branch.condition, condition) {
            continue;
        }

        for arm in &branch.arms {
            if condition_negates(condition, &arm.guard) {
                push_unique(&mut facts, canonical_condition(&arm.guard));
            }
        }
    }

    facts
}

fn semantic_loop_exit_extraction_gap(
    facts: &[LoopFact],
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
    semantics?;

    facts
        .iter()
        .find(|fact| {
            loop_exit_fact(&fact.condition).is_some() && fact.semantic_exit_facts.is_empty()
        })
        .map(|fact| fact.condition.clone())
}

fn loop_exit_fact(condition: &str) -> Option<String> {
    negated_condition(condition)
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

fn contains_semantic_unchecked_unwrap(semantics: Option<&TrustFunctionSemantics>) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics
            .calls
            .iter()
            .any(|call| semantic_call_is_unchecked_unwrap(&call.callee))
    })
}

fn semantic_call_is_unchecked_unwrap(callee: &str) -> bool {
    let leaf = function_leaf_name(callee);
    matches!(leaf, "unwrap" | "expect")
        && (callee.starts_with("Option::<") || callee.starts_with("Result::<"))
}

fn contains_semantic_explicit_panic(semantics: Option<&TrustFunctionSemantics>) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics
            .calls
            .iter()
            .any(|call| semantic_call_is_explicit_panic(&call.callee))
    })
}

fn semantic_call_is_explicit_panic(callee: &str) -> bool {
    matches!(
        function_leaf_name(callee),
        "panic_fmt" | "panic_display" | "panic_str" | "begin_panic" | "begin_panic_fmt"
    )
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

fn contains_semantic_closure(semantics: Option<&TrustFunctionSemantics>) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics
            .calls
            .iter()
            .any(|call| semantic_call_is_closure(&call.callee))
    })
}

fn semantic_call_is_closure(callee: &str) -> bool {
    callee.contains("{closure@")
        || callee.contains(" as Fn")
        || callee.contains(" as FnMut")
        || callee.contains(" as FnOnce")
}

fn unsupported_call(
    body: &str,
    params: &[Param],
    env: &[TrustFunctionSummary],
    function: &str,
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
    let tokens = tokens(body);

    for window in tokens.windows(4) {
        let [base, dot, method, open] = window else {
            continue;
        };
        if dot != "." || open != "(" || !is_ident(method) {
            continue;
        }
        if method == "len" && supported_len_method(base, params, semantics) {
            continue;
        }
        if method == "checked_add" && supported_checked_add_method(base, semantics) {
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
        if function_name_matches_call(function, name) {
            return Some(name.clone());
        }
        if env
            .iter()
            .any(|callee| function_name_matches_call(&callee.name, name))
        {
            continue;
        }

        return Some(name.clone());
    }

    None
}

fn supported_len_method(
    receiver: &str,
    params: &[Param],
    semantics: Option<&TrustFunctionSemantics>,
) -> bool {
    is_read_only_slice_param(receiver, params)
        || semantics.into_iter().any(|semantics| {
            semantics.calls.iter().any(|call| {
                call.callee == "<slice>.len" && call.args.iter().any(|arg| arg == receiver)
            })
        })
}

fn unsupported_semantic_call(
    semantics: Option<&TrustFunctionSemantics>,
    env: &[TrustFunctionSummary],
    function: &str,
) -> Option<String> {
    semantics.into_iter().find_map(|semantics| {
        semantics.calls.iter().find_map(|call| {
            if function_name_matches_call(function, &call.callee) {
                return Some(call.callee.clone());
            }
            if call.trust_callee.is_some() {
                return None;
            }
            if call.callee == "<slice>.len" {
                return None;
            }
            if semantic_checked_add_call_supported(call) {
                return None;
            }
            if semantic_index_call(&call.callee) {
                return None;
            }
            if unique_semantic_function_for_call(env, &call.callee).is_some() {
                return None;
            }
            if allowed_builtin_call(function_leaf_name(&call.callee)) {
                return None;
            }

            Some(call.callee.clone())
        })
    })
}

fn semantic_index_call(callee: &str) -> bool {
    callee.contains(" as Index<") && callee.ends_with(">::index")
}

fn supported_checked_add_method(
    receiver: &str,
    semantics: Option<&TrustFunctionSemantics>,
) -> bool {
    semantics.into_iter().any(|semantics| {
        semantics.calls.iter().any(|call| {
            semantic_checked_add_call_supported(call)
                && call.args.first().is_some_and(|arg| arg == receiver)
        })
    })
}

fn semantic_checked_add_call_supported(call: &SemanticCall) -> bool {
    call.args.len() == 2 && semantic_checked_add_integer_type(&call.callee).is_some()
}

fn semantic_checked_add_integer_type(callee: &str) -> Option<&str> {
    let ty = callee
        .strip_prefix("core::num::<impl ")?
        .strip_suffix(">::checked_add")?;
    is_supported_integer(ty).then_some(ty)
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
    call_obligations_with_ambiguity(body, env, true)
}

fn unambiguous_call_obligations(body: &str, env: &[TrustFunctionSummary]) -> Vec<CallObligation> {
    call_obligations_with_ambiguity(body, env, false)
}

fn call_obligations_with_ambiguity(
    body: &str,
    env: &[TrustFunctionSummary],
    allow_ambiguous_leaf_matches: bool,
) -> Vec<CallObligation> {
    executable_call_token_segments(body)
        .into_iter()
        .flat_map(|segment| {
            call_obligations_from_tokens(
                &segment.tokens,
                env,
                allow_ambiguous_leaf_matches,
                &segment.assumptions,
            )
        })
        .collect()
}

fn call_obligations_from_tokens(
    tokens: &[String],
    env: &[TrustFunctionSummary],
    allow_ambiguous_leaf_matches: bool,
    assumptions: &[String],
) -> Vec<CallObligation> {
    let mut obligations = Vec::new();
    let mut idx = 0;

    while idx + 1 < tokens.len() {
        let callee_name = &tokens[idx];
        if tokens[idx + 1] != "(" || idx.checked_sub(1).is_some_and(|prev| tokens[prev] == ".") {
            idx += 1;
            continue;
        }

        let mut matching_callees = env
            .iter()
            .filter(|function| function_name_matches_call(&function.name, callee_name));
        let Some(callee) = matching_callees.next() else {
            idx += 1;
            continue;
        };
        if !allow_ambiguous_leaf_matches && matching_callees.next().is_some() {
            idx += 1;
            continue;
        }
        let Some(end) = matching_token_group(&tokens, idx + 1, "(", ")") else {
            idx += 1;
            continue;
        };

        let args = split_arguments(&tokens[idx + 2..end])
            .iter()
            .map(|tokens| simple_grouped_value_expression(tokens))
            .collect::<Vec<_>>();
        if args.len() == callee.params.len() {
            for precondition in &callee.preconditions {
                obligations.push(CallObligation {
                    callee: callee.name.clone(),
                    condition: substitute_params(precondition, &callee.params, &args),
                    assumptions: assumptions.to_vec(),
                });
            }
        }

        idx += 1;
    }

    obligations
}

fn simple_grouped_value_expression(tokens: &[String]) -> String {
    if tokens.len() == 3
        && ((tokens[0] == "{" && tokens[2] == "}") || (tokens[0] == "(" && tokens[2] == ")"))
        && is_value_operand(&tokens[1])
    {
        return tokens[1].clone();
    }

    token_expression(tokens)
}

fn verification_call_obligations(
    body: &str,
    env: &[TrustFunctionSummary],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<CallObligation> {
    let semantic_obligations = semantic_call_obligations(semantics, env);
    if semantics.is_some() {
        return semantic_obligations;
    }
    let token_obligations = if semantic_obligations.is_empty() {
        call_obligations(body, env)
    } else {
        unambiguous_call_obligations(body, env)
    };
    let fallback_obligations = mergeable_token_fallback_obligations(
        &semantic_obligations,
        token_obligations,
        |obligation| &obligation.condition,
    );

    extend_unique_by(
        semantic_obligations,
        fallback_obligations,
        |existing, fallback| {
            existing.callee == fallback.callee && existing.condition == fallback.condition
        },
    )
}

fn semantic_call_obligations(
    semantics: Option<&TrustFunctionSemantics>,
    env: &[TrustFunctionSummary],
) -> Vec<CallObligation> {
    semantics
        .into_iter()
        .flat_map(|semantics| semantics.calls.iter())
        .filter_map(|call| {
            let callee = semantic_call_callee(call, env)?;
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

fn semantic_call_extraction_gap(
    body: &str,
    env: &[TrustFunctionSummary],
    semantics: Option<&TrustFunctionSemantics>,
) -> Option<String> {
    semantics?;

    let semantic_obligations = semantic_call_obligations(semantics, env);
    let semantic_conditions = semantic_obligations
        .iter()
        .map(|obligation| normalize(&obligation.condition))
        .collect::<Vec<_>>();

    unambiguous_call_obligations(body, env)
        .into_iter()
        .filter(|obligation| token_call_obligation_needs_semantic_coverage(&obligation.condition))
        .find(|obligation| {
            let condition = normalize(&obligation.condition);
            !semantic_conditions
                .iter()
                .any(|semantic_condition| semantic_condition == &condition)
        })
        .map(|obligation| obligation.condition)
}

fn token_call_obligation_needs_semantic_coverage(condition: &str) -> bool {
    !condition.contains('{')
        && !condition.contains('}')
        && !tokens(condition).contains(&"let".to_string())
}

fn semantic_call_callee(
    call: &SemanticCall,
    env: &[TrustFunctionSummary],
) -> Option<TrustFunctionSummary> {
    if let Some(callee) = &call.trust_callee {
        return Some(trust_callee_summary(callee));
    }

    unique_semantic_function_for_call(env, &call.callee)
}

fn unique_semantic_function_for_call(
    env: &[TrustFunctionSummary],
    call: &str,
) -> Option<TrustFunctionSummary> {
    let mut matches = env
        .iter()
        .filter(|function| semantic_function_name_matches_call(&function.name, call))
        .cloned();
    let first = matches.next()?;
    if matches.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn semantic_function_name_matches_call(function: &str, call: &str) -> bool {
    function == call
        || function.ends_with(&format!("::{call}"))
        || call.ends_with(&format!("::{function}"))
}

fn verification_call_env(
    env: &[TrustFunctionSummary],
    semantics: Option<&TrustFunctionSemantics>,
) -> Vec<TrustFunctionSummary> {
    let mut call_env = env.to_vec();

    for trust_callee in semantics
        .into_iter()
        .flat_map(|semantics| semantics.calls.iter())
        .filter_map(|call| call.trust_callee.as_ref())
    {
        let summary = trust_callee_summary(trust_callee);
        if call_env
            .iter()
            .any(|function| function_name_matches_call(&function.name, &summary.name))
        {
            continue;
        }
        call_env.push(summary);
    }

    call_env
}

fn trust_callee_summary(callee: &SemanticTrustCallee) -> TrustFunctionSummary {
    TrustFunctionSummary {
        name: callee.rust_function_path.clone(),
        params: callee
            .params
            .iter()
            .map(|param| Param {
                name: param.name.clone(),
                ty: param.ty.clone(),
            })
            .collect(),
        preconditions: callee
            .preconditions
            .iter()
            .map(|precondition| normalize(precondition))
            .collect(),
    }
}

fn function_name_matches_call(function: &str, call: &str) -> bool {
    function == call || function_leaf_name(call) == function || function_leaf_name(function) == call
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

fn extend_unique_by<T>(
    mut primary: Vec<T>,
    fallback: Vec<T>,
    same_obligation: impl Fn(&T, &T) -> bool,
) -> Vec<T> {
    for obligation in fallback {
        if primary
            .iter()
            .any(|existing| same_obligation(existing, &obligation))
        {
            continue;
        }
        primary.push(obligation);
    }

    primary
}

fn mergeable_token_fallback_obligations<T>(
    semantic_obligations: &[T],
    fallback_obligations: Vec<T>,
    expression: impl Fn(&T) -> &str,
) -> Vec<T> {
    if semantic_obligations.is_empty() {
        return fallback_obligations;
    }

    fallback_obligations
        .into_iter()
        .filter(|obligation| {
            let expression = expression(obligation);
            !expression.contains('{') && !expression.contains('}')
        })
        .collect()
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
    let Some(max) = max_value(ty, options.target_pointer_width) else {
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
            options.target_pointer_width,
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
    if constant == 1
        && is_unsigned_integer(ty)
        && contracts.iter().any(|contract| {
            contract_bounds_variable_below_unsigned(&obligation.variable, contract, params)
        })
    {
        return true;
    }

    contracts
        .iter()
        .any(|contract| contract == &le_required || contract == &le_unqualified)
}

fn contract_bounds_variable_below_unsigned(
    variable: &str,
    contract: &str,
    params: &[Param],
) -> bool {
    let Some((left, op, right)) = comparison_parts(contract) else {
        return false;
    };

    match op.as_str() {
        "<" => left == variable && param_type(&right, params).is_some_and(is_unsigned_integer),
        ">" => right == variable && param_type(&left, params).is_some_and(is_unsigned_integer),
        _ => false,
    }
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
                .any(|contract| contract == &normalize(&format!("{}>0", obligation.variable)))
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
    if rhs == "1"
        && contracts
            .iter()
            .any(|contract| contract == &normalize(&format!("{}>0", obligation.variable)))
    {
        return true;
    }

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
    let Some(max) = max_value(ty, options.target_pointer_width) else {
        return false;
    };
    let upper_symbolic = format!("{}<={}::MAX/{}", obligation.variable, ty, constant);
    let upper_numeric = format!("{}<={}", obligation.variable, max / constant);
    let upper_proved = contracts
        .iter()
        .any(|contract| contract == &upper_symbolic || contract == &upper_numeric);
    let z3_upper_proved = z3_proves_conclusion(&upper_numeric, contracts, params, options)
        .is_some_and(|proved| proved);

    if is_unsigned_integer(ty) {
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

fn signed_division_overflow_obligation_proved(
    obligation: &DivOverflowObligation,
    contracts: &[String],
    params: &[Param],
    options: VerificationOptions,
) -> bool {
    let contracts = contracts_with_assumptions(contracts, &obligation.assumptions);
    let contracts = contracts.as_slice();
    let Some(min) = min_value(&obligation.ty) else {
        return false;
    };

    if integer_constant_value(
        &obligation.left,
        Some(&obligation.ty),
        options.target_pointer_width,
    )
    .is_some_and(|value| value != min)
    {
        return true;
    }
    if integer_constant_value(
        &obligation.right,
        Some(&obligation.ty),
        options.target_pointer_width,
    )
    .is_some_and(|value| value != -1)
    {
        return true;
    }

    let left_ne_min = format!("{}!={}::MIN", obligation.left, obligation.ty);
    let right_ne_neg_one = format!("{}!=-1", obligation.right);
    if z3_proves_conclusion(&left_ne_min, contracts, params, options).is_some_and(|proved| proved)
        || z3_proves_conclusion(&right_ne_neg_one, contracts, params, options)
            .is_some_and(|proved| proved)
    {
        return true;
    }

    contracts_prove_not_min(&obligation.left, &obligation.ty, min, contracts)
        || contracts_prove_not_negative_one(&obligation.right, contracts)
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
    match solver::prove_integer_predicate(
        contracts,
        conclusion,
        &params,
        options.target_pointer_width,
        options.timeout_ms,
    ) {
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
    compact_parenthesized_value_tokens(&executable_tokens_without_loop_specs(body))
}

fn cfg_selected_body(body: &str, options: VerificationOptions) -> String {
    let tokens = tokens(body);
    cfg_selected_tokens(&tokens, options).join(" ")
}

fn cfg_selected_tokens(tokens: &[String], options: VerificationOptions) -> Vec<String> {
    let mut selected = Vec::new();
    let mut idx = 0;

    while idx < tokens.len() {
        if let Some((enabled, attr_end)) = cfg_attr(tokens, idx, options) {
            idx = attr_end;
            if enabled {
                continue;
            }
            idx = cfg_attributed_end(tokens, idx);
            continue;
        }

        selected.push(tokens[idx].clone());
        idx += 1;
    }

    selected
}

fn cfg_attr(tokens: &[String], idx: usize, options: VerificationOptions) -> Option<(bool, usize)> {
    if tokens.get(idx)? != "#" || tokens.get(idx + 1)? != "[" || tokens.get(idx + 2)? != "cfg" {
        return None;
    }
    let open_idx = idx + 3;
    if tokens.get(open_idx)? != "(" {
        return None;
    }
    let close_idx = matching_token_group(tokens, open_idx, "(", ")")?;
    if tokens.get(close_idx + 1)? != "]" {
        return None;
    }
    let enabled = cfg_predicate_enabled(&tokens[open_idx + 1..close_idx], options).unwrap_or(true);
    Some((enabled, close_idx + 2))
}

fn cfg_attributed_end(tokens: &[String], idx: usize) -> usize {
    let Some(token) = tokens.get(idx) else {
        return idx;
    };
    if matches!(token.as_str(), "{" | "(" | "[") {
        let close = match token.as_str() {
            "{" => "}",
            "(" => ")",
            "[" => "]",
            _ => unreachable!(),
        };
        return matching_token_group(tokens, idx, token, close)
            .map(|idx| idx + 1)
            .unwrap_or(idx + 1);
    }

    tokens[idx..]
        .iter()
        .position(|token| token == ";")
        .map(|offset| idx + offset + 1)
        .unwrap_or(tokens.len())
}

fn cfg_predicate_enabled(tokens: &[String], options: VerificationOptions) -> Option<bool> {
    match tokens {
        [name, open, close] if name == "any" && open == "(" && close == ")" => Some(false),
        [name, open, close] if name == "all" && open == "(" && close == ")" => Some(true),
        [name, open, inner @ .., close] if name == "not" && open == "(" && close == ")" => {
            cfg_predicate_enabled(inner, options).map(|enabled| !enabled)
        }
        [name, equals, value] if name == "target_pointer_width" && equals == "=" => {
            cfg_target_pointer_width_matches(value, options)
        }
        [name, equals, quote_open, value, quote_close]
            if name == "target_pointer_width"
                && equals == "="
                && quote_open == "\""
                && quote_close == "\"" =>
        {
            cfg_target_pointer_width_matches(value, options)
        }
        _ => None,
    }
}

fn cfg_target_pointer_width_matches(value: &str, options: VerificationOptions) -> Option<bool> {
    options
        .target_pointer_width
        .map(|width| value == width.to_string())
}

fn executable_tokens_without_loop_specs(body: &str) -> Vec<String> {
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

    executable
}

fn executable_token_segments(body: &str) -> Vec<TokenSegment> {
    scoped_token_segments(&executable_tokens(body), &[])
}

fn executable_call_token_segments(body: &str) -> Vec<TokenSegment> {
    scoped_token_segments(&executable_tokens_without_loop_specs(body), &[])
}

fn scoped_token_segments(tokens: &[String], assumptions: &[String]) -> Vec<TokenSegment> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut idx = 0;

    while idx < tokens.len() {
        if tokens[idx] != "while" {
            current.push(tokens[idx].clone());
            idx += 1;
            continue;
        }

        let Some(body_open_idx) = tokens[idx + 1..]
            .iter()
            .position(|token| token == "{")
            .map(|offset| idx + 1 + offset)
        else {
            current.push(tokens[idx].clone());
            idx += 1;
            continue;
        };
        let Some(body_close_idx) = matching_token_group(tokens, body_open_idx, "{", "}") else {
            current.push(tokens[idx].clone());
            idx += 1;
            continue;
        };

        if !current.is_empty() {
            segments.push(TokenSegment {
                tokens: std::mem::take(&mut current),
                assumptions: assumptions.to_vec(),
            });
        }

        let condition_tokens = tokens[idx + 1..body_open_idx].to_vec();
        if !condition_tokens.is_empty() {
            segments.push(TokenSegment {
                tokens: condition_tokens,
                assumptions: assumptions.to_vec(),
            });
        }

        let mut loop_assumptions = assumptions.to_vec();
        let condition = token_expression(&tokens[idx + 1..body_open_idx]);
        if !condition.is_empty() {
            loop_assumptions.push(condition);
        }
        segments.extend(scoped_token_segments(
            &tokens[body_open_idx + 1..body_close_idx],
            &loop_assumptions,
        ));
        idx = body_close_idx + 1;
    }

    if !current.is_empty() {
        segments.push(TokenSegment {
            tokens: current,
            assumptions: assumptions.to_vec(),
        });
    }

    segments
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

fn supported_operation_type(ty: Option<&str>) -> Option<&str> {
    ty.filter(|ty| is_supported_integer(ty))
}

fn integer_constant_value(
    token: &str,
    ty: Option<&str>,
    target_pointer_width: Option<u32>,
) -> Option<i128> {
    if let Ok(value) = token.parse::<i128>() {
        return Some(value);
    }

    let ty = ty?;
    if token == format!("{ty}::MAX") {
        return max_value(ty, target_pointer_width);
    }
    if token == format!("{ty}::MIN") {
        return min_value(ty);
    }

    None
}

fn typed_constant_binary_is_safe(
    left: &str,
    right: &str,
    ty: Option<&str>,
    operation: fn(i128, i128) -> Option<i128>,
    target_pointer_width: Option<u32>,
) -> bool {
    let Some(ty) = ty else {
        return false;
    };
    let (Some(left), Some(right)) = (
        integer_constant_value(left, Some(ty), target_pointer_width),
        integer_constant_value(right, Some(ty), target_pointer_width),
    ) else {
        return false;
    };
    let Some(value) = operation(left, right) else {
        return false;
    };
    let Some(min) = min_value(ty) else {
        return false;
    };
    let Some(max) = max_value(ty, target_pointer_width) else {
        return false;
    };

    min <= value && value <= max
}

fn expression_needs_integer_proof(left: &str, right: &str, params: &[Param]) -> bool {
    param_type(left, params).is_some()
        || param_type(right, params).is_some()
        || (is_ident(left) && is_ident(right))
}

fn is_value_operand(token: &str) -> bool {
    is_ident(token) || token.parse::<i128>().is_ok()
}

fn simple_signed_value_operand_before(tokens: &[String], op_idx: usize) -> Option<(String, usize)> {
    let idx = op_idx.checked_sub(1)?;
    let token = tokens.get(idx)?;
    if token.parse::<i128>().is_ok()
        && idx > 0
        && tokens.get(idx - 1) == Some(&"-".to_string())
        && looks_unary_minus(tokens, idx - 1)
    {
        return Some((format!("-{token}"), idx - 1));
    }
    if is_value_operand(token) {
        return Some((token.clone(), idx));
    }

    None
}

fn simple_signed_value_operand_after(tokens: &[String], idx: usize) -> Option<(String, usize)> {
    let token = tokens.get(idx)?;
    if token == "-" {
        let value = tokens.get(idx + 1)?;
        if value.parse::<i128>().is_ok() && looks_unary_minus(tokens, idx) {
            return Some((format!("-{value}"), idx + 2));
        }
        return None;
    }
    if is_value_operand(token) {
        return Some((token.clone(), idx + 1));
    }

    None
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
    matches!(ty, "i32" | "i64" | "u32" | "u64" | "usize")
}

fn is_signed_integer(ty: &str) -> bool {
    matches!(ty, "i32" | "i64")
}

fn is_unsigned_integer(ty: &str) -> bool {
    matches!(ty, "u32" | "u64" | "usize")
}

fn max_value(ty: &str, target_pointer_width: Option<u32>) -> Option<i128> {
    match ty {
        "i32" => Some(i32::MAX as i128),
        "i64" => Some(i64::MAX as i128),
        "u32" => Some(u32::MAX as i128),
        "u64" => Some(u64::MAX as i128),
        "usize" => Some(usize_max_value(target_pointer_width)),
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

fn min_value(ty: &str) -> Option<i128> {
    match ty {
        "i32" => Some(i32::MIN as i128),
        "i64" => Some(i64::MIN as i128),
        "u32" | "u64" => Some(0),
        "usize" => Some(0),
        _ => None,
    }
}

fn is_read_only_slice_param(name: &str, params: &[Param]) -> bool {
    params
        .iter()
        .any(|param| param.name == name && param.ty.starts_with("&[") && param.ty.ends_with(']'))
}

fn slice_element_type(ty: &str) -> Option<&str> {
    ty.trim()
        .strip_prefix("&[")?
        .strip_suffix(']')
        .map(str::trim)
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

fn decrement_amount(tokens: &[String], variable: &str) -> Option<i128> {
    for window in tokens.windows(5) {
        let [target, equals, source, op, amount] = window else {
            continue;
        };
        if target == variable && equals == "=" && source == variable && op == "-" {
            if let Ok(amount) = amount.parse::<i128>() {
                return Some(amount);
            }
        }
    }

    for window in tokens.windows(4) {
        let [target, minus, equals, amount] = window else {
            continue;
        };
        if target == variable && minus == "-" && equals == "=" {
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
        (4294967294, "u32") => "u32::MAX-1".to_string(),
        (18446744073709551614, "u64") => "u64::MAX-1".to_string(),
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

fn contracts_prove_not_min(expr: &str, ty: &str, min: i128, contracts: &[String]) -> bool {
    let min_plus_one = min + 1;
    let min_plus_one_typed = min_bound_with_type(min_plus_one, ty);
    let expected = [
        format!("{expr}!={ty}::MIN"),
        format!("{ty}::MIN!={expr}"),
        format!("{expr}!={min}"),
        format!("{min}!={expr}"),
        format!("{expr}>{ty}::MIN"),
        format!("{ty}::MIN<{expr}"),
        format!("{expr}>{min}"),
        format!("{min}<{expr}"),
        format!("{expr}>={min_plus_one_typed}"),
        format!("{min_plus_one_typed}<={expr}"),
        format!("{expr}>={min_plus_one}"),
        format!("{min_plus_one}<={expr}"),
    ];

    contracts
        .iter()
        .any(|contract| expected.iter().any(|expected| contract == expected))
}

fn contracts_prove_not_negative_one(expr: &str, contracts: &[String]) -> bool {
    let expected = [
        format!("{expr}!=-1"),
        format!("-1!={expr}"),
        format!("{expr}>-1"),
        format!("-1<{expr}"),
        format!("{expr}>0"),
        format!("0<{expr}"),
        format!("{expr}>=0"),
        format!("0<={expr}"),
        format!("{expr}<-1"),
        format!("-1>{expr}"),
        format!("{expr}<=-2"),
        format!("-2>={expr}"),
    ];

    contracts
        .iter()
        .any(|contract| expected.iter().any(|expected| contract == expected))
}

fn looks_unary_minus(tokens: &[String], minus_idx: usize) -> bool {
    if minus_idx == 0 {
        return true;
    }

    let previous = &tokens[minus_idx - 1];
    matches!(
        previous.as_str(),
        "{" | "("
            | "["
            | ","
            | "return"
            | "=>"
            | "="
            | "<"
            | ">"
            | "<="
            | ">="
            | "=="
            | "!="
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
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

    fn executable_spec_metadata(name: &str, function_source: &str) -> TrustMetadata {
        TrustMetadata {
            schema_version: 1,
            trust_macro_version: "test".to_string(),
            module_id: "test-module".to_string(),
            item_kind: "spec".to_string(),
            item_id: format!("spec:executable:{name}:test"),
            source_span: "test-span".to_string(),
            rust_function_path: name.to_string(),
            visibility: "private".to_string(),
            contracts_original: Vec::new(),
            contracts_normalized: Vec::new(),
            contract_classes: Vec::new(),
            assertion_policy: "always".to_string(),
            function_source: function_source.to_string(),
            body_hash_placeholder: format!("{name}-hash"),
            trust_model_dependencies: Vec::new(),
        }
    }

    fn ghost_spec_metadata(name: &str, function_source: &str) -> TrustMetadata {
        let mut metadata = executable_spec_metadata(name, function_source);
        metadata.item_id = format!("spec:ghost:{name}:test");
        metadata
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
    fn target_pointer_width_controls_usize_addition_bounds() {
        let metadata = metadata_named(
            "inc",
            "pub fn inc(n: usize) -> usize { n + 1 }",
            &["n <= 5000000000"],
        );

        assert_eq!(
            verify_totals_with_options(
                &[metadata.clone()],
                VerificationOptions::z3(5000).with_target_pointer_width(64),
            ),
            Ok(())
        );
        assert_eq!(
            verify_totals_with_options(
                &[metadata],
                VerificationOptions::z3(5000).with_target_pointer_width(32),
            ),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "inc".to_string(),
                expression: "n + 1".to_string(),
            })
        );
    }

    #[test]
    fn target_cfg_disabled_code_is_not_verified_from_tokens() {
        let metadata = metadata_named_with_classes(
            "cfg_zero",
            "pub fn cfg_zero(x: i32) -> i32 { #[cfg(target_pointer_width = \"16\")] { x + 1 } #[cfg(not(target_pointer_width = \"16\"))] { 0 } }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "cfg_zero".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("0".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "n > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "n > 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                    SemanticBranchArm {
                        guard: "n <= 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                ],
            }],
        };

        assert_eq!(
            verify_totals_with_semantics(
                &[metadata],
                &[semantics],
                VerificationOptions::default().with_target_pointer_width(64),
            ),
            Ok(())
        );
    }

    #[test]
    fn z3_does_not_prove_postcondition_from_unbound_contract_identifier() {
        let mut metadata = metadata_named_with_classes(
            "id",
            "fn id(x: i32) -> i32 { x }",
            &["y == x", "out == y"],
            &["given ghost", "gives ghost"],
        );
        metadata.visibility = "private".to_string();

        assert_eq!(
            verify_totals_with_options(&[metadata], VerificationOptions::z3(5000)),
            Err(VerificationError::PostconditionUnproved {
                function: "id".to_string(),
                condition: "out == y".to_string(),
            })
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
    fn proves_u32_add_one_from_executable_precondition() {
        let metadata = metadata_named(
            "add_one_u32",
            "pub fn add_one_u32(x: u32) -> u32 { x + 1 }",
            &["x < u32::MAX"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_unproved_u32_add_one() {
        let metadata = metadata_named(
            "add_one_u32",
            "pub fn add_one_u32(x: u32) -> u32 { x + 1 }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "add_one_u32".to_string(),
                expression: "x + 1".to_string(),
            })
        );
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
            &["y != 0", "x > i32::MIN"],
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
            "pub fn div(x: i32, y: i32) -> i32 { x / (y + 0) }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero {
                function: "div".to_string(),
                expression: "x / (y+0)".to_string(),
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
    fn rejects_unproved_signed_division_overflow() {
        let metadata = metadata_named(
            "div_neg_one",
            "pub fn div_neg_one(x: i32) -> i32 { x / -1 }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionOverflow {
                function: "div_neg_one".to_string(),
                expression: "x / -1".to_string(),
            })
        );
    }

    #[test]
    fn rejects_signed_division_overflow_when_only_denominator_nonzero_is_proved() {
        let metadata = metadata_named(
            "div",
            "pub fn div(x: i32, y: i32) -> i32 { x / y }",
            &["y != 0"],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionOverflow {
                function: "div".to_string(),
                expression: "x / y".to_string(),
            })
        );
    }

    #[test]
    fn proves_signed_division_overflow_from_left_lower_bound() {
        let metadata = metadata_named(
            "div_neg_one",
            "pub fn div_neg_one(x: i32) -> i32 { x / -1 }",
            &["x > i32::MIN"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn proves_signed_division_overflow_from_denominator_bound() {
        let metadata = metadata_named(
            "div",
            "pub fn div(x: i32, y: i32) -> i32 { x / y }",
            &["y > 0"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn z3_proves_signed_division_overflow_from_denominator_lower_bound() {
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
    fn rejects_unproved_signed_remainder_overflow() {
        let metadata = metadata_named(
            "rem_neg_one",
            "pub fn rem_neg_one(x: i32) -> i32 { x % -1 }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerRemainderOverflow {
                function: "rem_neg_one".to_string(),
                expression: "x % -1".to_string(),
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
    fn rejects_unsupported_float_signature_type() {
        let metadata = metadata_named("id_f32", "pub fn id_f32(x: f32) -> f32 { x }", &[]);

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedType {
                function: "id_f32".to_string(),
                ty: "f32".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unsupported_nested_float_signature_type() {
        let metadata = metadata_named(
            "maybe",
            "pub fn maybe(x: Option<f32>) -> Option<f32> { x }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedType {
                function: "maybe".to_string(),
                ty: "f32".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unsupported_semantic_local_float_type() {
        let metadata = metadata_named(
            "keep",
            "pub fn keep(x: i32) -> i32 { let y = 1.0f32; let _ = y; x }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "keep".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: vec!["i32".to_string(), "f32".to_string()],
            contract_bindings: Vec::new(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::UnsupportedType {
                function: "keep".to_string(),
                ty: "f32".to_string(),
            })
        );
    }

    #[test]
    fn allows_opaque_type_pass_through_without_contract_reasoning() {
        let metadata = metadata_named(
            "id_string",
            "pub fn id_string(x: String) -> String { x }",
            &[],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn rejects_opaque_type_contract_reasoning() {
        let metadata = metadata_named_with_classes(
            "id_string",
            "pub fn id_string(x: String) -> String { x }",
            &["out == x"],
            &["gives ghost"],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedType {
                function: "id_string".to_string(),
                ty: "String".to_string(),
            })
        );
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
    fn partial_semantic_field_accesses_fail_closed() {
        let balance = metadata_named(
            "balance",
            "pub fn balance(acct: Account) -> i64 { acct.balance }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "balance".to_string(),
            params: vec![SemanticParam {
                name: "acct".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("acct.balance".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[balance], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "balance".to_string(),
                category: "field access".to_string(),
                expression: "acct.balance".to_string(),
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
    fn semantic_loop_decreases_uses_mir_assignment_target() {
        let metadata = metadata_named_with_classes(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { n -= 1; } n }",
            &["out == 0"],
            &["gives executable"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown".to_string(),
            params: vec![SemanticParam {
                name: "n".to_string(),
                ty: "usize".to_string(),
            }],
            return_type: "usize".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("n".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("usize".to_string()),
                target: Some("n".to_string()),
                left: "n".to_string(),
                right: Some("1".to_string()),
                expression: "n - 1".to_string(),
                guards: vec!["n > 0".to_string()],
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "n > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "n > 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                    SemanticBranchArm {
                        guard: "n <= 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                ],
            }],
        };

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopDecreasesNotDecreasing {
                function: "countdown".to_string(),
                measure: "n".to_string(),
            })
        );
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn partial_semantic_loop_decreases_fails_closed() {
        let metadata = metadata_named_with_classes(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { n = n - 1; } n }",
            &[],
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown".to_string(),
            params: vec![SemanticParam {
                name: "n".to_string(),
                ty: "usize".to_string(),
            }],
            return_type: "usize".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("n".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "countdown".to_string(),
                category: "loop decreases".to_string(),
                expression: "n".to_string(),
            })
        );
    }

    #[test]
    fn partial_semantic_loop_invariant_fails_closed() {
        let metadata = metadata_named_with_classes(
            "countdown_with_stable_bound",
            "pub fn countdown_with_stable_bound(mut i: usize, n: usize) -> usize { let x = 0; trust::loop_spec! { invariant(x <= n); decreases(i); } while i > 0 { i = i - 1; } i }",
            &[],
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown_with_stable_bound".to_string(),
            params: vec![
                SemanticParam {
                    name: "i".to_string(),
                    ty: "usize".to_string(),
                },
                SemanticParam {
                    name: "n".to_string(),
                    ty: "usize".to_string(),
                },
            ],
            return_type: "usize".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("i".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("usize".to_string()),
                target: Some("i".to_string()),
                left: "i".to_string(),
                right: Some("1".to_string()),
                expression: "i - 1".to_string(),
                guards: vec!["i > 0".to_string()],
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
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "countdown_with_stable_bound".to_string(),
                category: "loop invariant".to_string(),
                expression: "x<=n".to_string(),
            })
        );
    }

    #[test]
    fn partial_semantic_loop_exit_fails_closed() {
        let metadata = metadata_named_with_classes(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { n -= 1; } n }",
            &["out == 0"],
            &["gives executable"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown".to_string(),
            params: vec![SemanticParam {
                name: "n".to_string(),
                ty: "usize".to_string(),
            }],
            return_type: "usize".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("n".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("usize".to_string()),
                target: Some("n".to_string()),
                left: "n".to_string(),
                right: Some("1".to_string()),
                expression: "n - 1".to_string(),
                guards: vec!["n > 0".to_string()],
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "countdown".to_string(),
                category: "loop exit".to_string(),
                expression: "n>0".to_string(),
            })
        );
    }

    #[test]
    fn semantic_loop_exit_facts_use_mir_branch_guard() {
        let metadata = metadata_named_with_classes(
            "countdown",
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while (n > 0) { n -= 1; } n }",
            &["out == 0"],
            &["gives executable"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown".to_string(),
            params: vec![SemanticParam {
                name: "n".to_string(),
                ty: "usize".to_string(),
            }],
            return_type: "usize".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("n".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("usize".to_string()),
                target: Some("n".to_string()),
                left: "n".to_string(),
                right: Some("1".to_string()),
                expression: "n - 1".to_string(),
                guards: vec!["n > 0".to_string()],
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "n > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "n > 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                    SemanticBranchArm {
                        guard: "n <= 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                ],
            }],
        };

        assert_eq!(
            semantic_loop_exit_facts(Some(&semantics), "(n > 0)"),
            vec!["n<=0".to_string()]
        );
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn loop_invariant_and_exit_fact_prove_postcondition() {
        let metadata = metadata_named_with_classes(
            "count_to",
            "pub fn count_to(n: usize) -> usize { let mut i = 0; trust::loop_spec! { invariant(i <= n); decreases(n - i); } while i < n { i += 1; } i }",
            &["out == n"],
            &["gives executable"],
        );

        assert_eq!(verify_total(&metadata), Ok(()));
    }

    #[test]
    fn semantic_local_binding_proves_local_loop_measure_and_postcondition() {
        let metadata = metadata_named_with_classes(
            "countdown_local",
            "pub fn countdown_local(n: usize) -> usize { let mut i = n; trust::loop_spec! { invariant(i >= 0); decreases(i); } while i > 0 { i -= 1; } i }",
            &["out == 0"],
            &["gives executable"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "countdown_local".to_string(),
            params: vec![SemanticParam {
                name: "n".to_string(),
                ty: "usize".to_string(),
            }],
            return_type: "usize".to_string(),
            local_types: vec!["usize".to_string()],
            contract_bindings: vec![SemanticContractBinding {
                expression: "i".to_string(),
                name: "i".to_string(),
                kind: SemanticContractBindingKind::Local,
                ty: "usize".to_string(),
            }],
            return_expression: Some("i".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Sub,
                ty: Some("usize".to_string()),
                target: Some("i".to_string()),
                left: "i".to_string(),
                right: Some("1".to_string()),
                expression: "i - 1".to_string(),
                guards: vec!["i > 0".to_string()],
            }],
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "i > 0".to_string(),
                arms: vec![
                    SemanticBranchArm {
                        guard: "i > 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                    SemanticBranchArm {
                        guard: "i <= 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: None,
                    },
                ],
            }],
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn rejects_loop_invariant_not_established() {
        let metadata = metadata_named(
            "count_from",
            "pub fn count_from(mut i: usize, n: usize) -> usize { trust::loop_spec! { invariant(i <= n); decreases(n - i); } while i < n { i += 1; } i }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::LoopInvariantNotEstablished {
                function: "count_from".to_string(),
                invariant: "i<=n".to_string(),
            })
        );
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
    fn rejects_division_using_loop_exit_value() {
        let metadata = metadata_named(
            "divide_after_countdown",
            "pub fn divide_after_countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { n = n - 1; } 1 / n }",
            &[],
        );

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::IntegerDivisionByZero {
                function: "divide_after_countdown".to_string(),
                expression: "1 / n".to_string(),
            })
        );
    }

    #[test]
    fn loop_body_arithmetic_uses_loop_condition_assumption() {
        let source =
            "pub fn countdown(mut n: usize) -> usize { trust::loop_spec! { decreases(n); } while n > 0 { n = n - 1; } n }";
        let params = parse_params(&normalize(source));
        let body = body(source);

        assert_eq!(
            executable_token_segments(body),
            vec![
                TokenSegment {
                    tokens: vec!["n".to_string(), ">".to_string(), "0".to_string()],
                    assumptions: Vec::new(),
                },
                TokenSegment {
                    tokens: vec![
                        "n".to_string(),
                        "=".to_string(),
                        "n".to_string(),
                        "-".to_string(),
                        "1".to_string(),
                        ";".to_string(),
                    ],
                    assumptions: vec!["n>0".to_string()],
                },
                TokenSegment {
                    tokens: vec!["n".to_string()],
                    assumptions: Vec::new(),
                },
            ]
        );

        assert_eq!(
            subtraction_obligations(body, &params),
            vec![SubObligation {
                variable: "n".to_string(),
                ty: Some("usize".to_string()),
                constant: Some(1),
                rhs: None,
                expression: "n - 1".to_string(),
                assumptions: vec!["n>0".to_string()],
            }]
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
    fn semantic_call_rejects_unchecked_option_unwrap() {
        let metadata = metadata_named(
            "bad_unwrap",
            "pub fn bad_unwrap(x: Option<i32>) -> i32 { 0 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "bad_unwrap".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "Option<i32>".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("Option::<i32>::unwrap(x)".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "Option::<i32>::unwrap".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
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
    fn semantic_call_rejects_unchecked_result_expect() {
        let metadata = metadata_named(
            "bad_expect",
            "pub fn bad_expect(x: Result<i32, i32>) -> i32 { 0 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "bad_expect".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "Result<i32, i32>".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("Result::<i32, i32>::expect(x,const \"ok\")".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "Result::<i32, i32>::expect".to_string(),
                args: vec!["x".to_string(), "const \"ok\"".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
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
    fn semantic_len_call_allows_slice_alias_method() {
        let metadata = metadata_named(
            "get_or_zero",
            "pub fn get_or_zero(xs: &[i32], i: usize) -> i32 { let ys = xs; if i < ys.len() { ys[i] } else { 0 } }",
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "ys".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
                expression: "ys[i]".to_string(),
                guards: vec!["i < ys.len()".to_string()],
            }],
            calls: vec![SemanticCall {
                callee: "<slice>.len".to_string(),
                args: vec!["ys".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedCall {
                function: "get_or_zero".to_string(),
                callee: "ys.len".to_string(),
            })
        );
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_checked_add_call_allows_supported_integer_method() {
        let metadata = metadata_named_with_classes(
            "checked_sum",
            "pub fn checked_sum(x: i32, y: i32) -> Option<i32> { x.checked_add(y) }",
            &["out == x.checked_add(y)"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "checked_sum".to_string(),
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
            return_type: "Option<i32>".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x.checked_add(y)".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "core::num::<impl i32>::checked_add".to_string(),
                args: vec!["x".to_string(), "y".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedCall {
                function: "checked_sum".to_string(),
                callee: "x.checked_add".to_string(),
            })
        );
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
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
    fn semantic_call_rejects_closure_body() {
        let metadata = metadata_named("apply", "pub fn apply(x: i32) -> i32 { x }", &[]);
        let semantics = TrustFunctionSemantics {
            rust_function_path: "apply".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "<{closure@src/lib.rs:1:1: 1:17} as Fn<(i32,)>>::call".to_string(),
                args: vec!["_2".to_string(), "(x,)".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
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
    fn semantic_call_rejects_explicit_panic() {
        let metadata = metadata_named("fail", "pub fn fail() -> i32 { 0 }", &[]);
        let semantics = TrustFunctionSemantics {
            rust_function_path: "fail".to_string(),
            params: Vec::new(),
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "core::panicking::panic_fmt".to_string(),
                args: vec!["message".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
    fn semantic_return_expression_blocks_source_return_fallback() {
        let metadata = metadata_named_with_classes(
            "id_i32",
            "pub fn id_i32(x: i32) -> i32 { 0 }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "id_i32".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert!(matches!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
    }

    #[test]
    fn partial_semantic_return_expression_fails_closed() {
        let metadata = metadata_named_with_classes(
            "zero",
            "pub fn zero() -> i32 { 0 }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "zero".to_string(),
            params: Vec::new(),
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "zero".to_string(),
                category: "return expression".to_string(),
                expression: "0".to_string(),
            })
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
    fn semantic_contract_field_binding_adds_typed_value_param() {
        let semantics = TrustFunctionSemantics {
            rust_function_path: "reward".to_string(),
            params: vec![SemanticParam {
                name: "acct".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            local_types: Vec::new(),
            contract_bindings: vec![SemanticContractBinding {
                expression: "acct.balance < i64::MAX".to_string(),
                name: "acct.balance".to_string(),
                kind: SemanticContractBindingKind::Field,
                ty: "i64".to_string(),
            }],
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verification_value_params(
                &[Param {
                    name: "acct".to_string(),
                    ty: "Account".to_string(),
                }],
                Some(&semantics),
            ),
            vec![
                Param {
                    name: "acct".to_string(),
                    ty: "Account".to_string(),
                },
                Param {
                    name: "acct.balance".to_string(),
                    ty: "i64".to_string(),
                },
            ]
        );
    }

    #[test]
    fn semantic_field_addition_requires_typed_overflow_proof() {
        let account = model_metadata("Account");
        let metadata = metadata_named(
            "reward",
            "pub fn reward(acct: Account) -> i64 { acct.balance + 1 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "reward".to_string(),
            params: vec![SemanticParam {
                name: "acct".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("acct.balance + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i64".to_string()),
                target: None,
                left: "acct.balance".to_string(),
                right: Some("1".to_string()),
                expression: "acct.balance + 1".to_string(),
                guards: Vec::new(),
            }],
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

        assert_eq!(
            verify_totals_with_semantics(
                &[account, metadata],
                &[semantics],
                VerificationOptions::default()
            ),
            Err(VerificationError::IntegerAdditionOverflow {
                function: "reward".to_string(),
                expression: "acct.balance + 1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_field_addition_uses_typed_contract_bound() {
        let account = model_metadata("Account");
        let metadata = metadata_named(
            "reward",
            "pub fn reward(acct: Account) -> i64 { acct.balance + 1 }",
            &["acct.balance < i64::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "reward".to_string(),
            params: vec![SemanticParam {
                name: "acct".to_string(),
                ty: "Account".to_string(),
            }],
            return_type: "i64".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("acct.balance + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i64".to_string()),
                target: None,
                left: "acct.balance".to_string(),
                right: Some("1".to_string()),
                expression: "acct.balance + 1".to_string(),
                guards: Vec::new(),
            }],
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticMatchArm {
                        variant: "Some".to_string(),
                        discriminant: "1".to_string(),
                        payload: None,
                        assumptions: Vec::new(),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
            }],
            branches: Vec::new(),
        };

        assert!(matches!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
    }

    #[test]
    fn semantic_match_blocks_source_return_fallback_when_complete() {
        let metadata = metadata_named_with_classes(
            "unwrap_or_zero",
            "pub fn unwrap_or_zero(x: Option<i32>) -> i32 { match x { Some(v) => v, None => 0 } 0 }",
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
            }],
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
            }],
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_match_path_assumptions_can_prove_nested_postcondition() {
        let metadata = metadata_named_with_classes(
            "nested_match_zero_or_self",
            "pub fn nested_match_zero_or_self(x: i32, choice: Option<i32>) -> i32 { if x == 0 { match choice { Some(_) => 0, None => 0 } } else { x } }",
            &["out == x"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "nested_match_zero_or_self".to_string(),
            params: vec![
                SemanticParam {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                },
                SemanticParam {
                    name: "choice".to_string(),
                    ty: "Option<i32>".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: vec![SemanticMatch {
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x <= 0".to_string(),
                        assumptions: Vec::new(),
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
    fn semantic_branch_blocks_source_return_fallback_when_complete() {
        let metadata = metadata_named_with_classes(
            "zero_or_self",
            "pub fn zero_or_self(x: i32) -> i32 { if x == 0 { 0 } else { x } 0 }",
            &["out == 0"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "zero_or_self".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: Some("x".to_string()),
                    },
                ],
            }],
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert!(matches!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::PostconditionUnproved { .. })
        ));
    }

    #[test]
    fn semantic_branch_guard_equality_assumption_can_prove_postcondition() {
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
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
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: Some("x".to_string()),
                    },
                ],
            }],
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn semantic_branch_can_prove_postcondition_after_local_return_expression() {
        let metadata = metadata_named_with_classes(
            "zero_or_self_via_local",
            "pub fn zero_or_self_via_local(x: i32) -> i32 { let mut y = x; if x == 0 { y = 0; } else { y = x; } y }",
            &["out == x"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "zero_or_self_via_local".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: vec!["i32".to_string()],
            contract_bindings: Vec::new(),
            return_expression: Some("y".to_string()),
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
                        assumptions: Vec::new(),
                        return_expression: Some("0".to_string()),
                    },
                    SemanticBranchArm {
                        guard: "x != 0".to_string(),
                        assumptions: Vec::new(),
                        return_expression: Some("x".to_string()),
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
    fn semantic_branch_path_assumptions_can_prove_nested_postcondition() {
        let metadata = metadata_named_with_classes(
            "nested_zero_or_self",
            "pub fn nested_zero_or_self(x: i32, flag: bool) -> i32 { if x == 0 { if flag { 0 } else { 0 } } else { x } }",
            &["out == x"],
            &["gives ghost"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "nested_zero_or_self".to_string(),
            params: vec![
                SemanticParam {
                    name: "x".to_string(),
                    ty: "i32".to_string(),
                },
                SemanticParam {
                    name: "flag".to_string(),
                    ty: "bool".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: vec![SemanticBranch {
                condition: "flag".to_string(),
                arms: vec![
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
                ],
            }],
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
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
    fn partial_semantic_arithmetic_fails_closed() {
        let metadata = metadata_named(
            "add_both",
            "pub fn add_both(x: i32, y: i32) -> i32 { let _a = x + 1; y + 1 }",
            &["x < i32::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "add_both".to_string(),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("y + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
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

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "add_both".to_string(),
                category: "arithmetic operation".to_string(),
                expression: "y + 1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_arithmetic_uses_mir_type_for_constant_addition_overflow() {
        let metadata = metadata_named(
            "overflow",
            "pub fn overflow() -> i32 { let x: i32 = i32::MAX; x + 1 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "overflow".to_string(),
            params: Vec::new(),
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("i32::MAX + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "i32::MAX".to_string(),
                right: Some("1".to_string()),
                expression: "i32::MAX + 1".to_string(),
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
                function: "overflow".to_string(),
                expression: "i32::MAX + 1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_arithmetic_uses_mir_u32_type_for_constant_addition_overflow() {
        let metadata = metadata_named(
            "overflow_u32",
            "pub fn overflow_u32() -> u32 { let x: u32 = u32::MAX; x + 1 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "overflow_u32".to_string(),
            params: Vec::new(),
            return_type: "u32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("u32::MAX + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("u32".to_string()),
                target: None,
                left: "u32::MAX".to_string(),
                right: Some("1".to_string()),
                expression: "u32::MAX + 1".to_string(),
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
                function: "overflow_u32".to_string(),
                expression: "u32::MAX + 1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_arithmetic_allows_safe_typed_constant_addition() {
        let metadata = metadata_named(
            "safe_add",
            "pub fn safe_add() -> i32 { let x: i32 = 40; x + 1 }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "safe_add".to_string(),
            params: Vec::new(),
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("40 + 1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
                left: "40".to_string(),
                right: Some("1".to_string()),
                expression: "40 + 1".to_string(),
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
    fn semantic_arithmetic_uses_mir_type_for_constant_negation_overflow() {
        let metadata = metadata_named(
            "negate",
            "pub fn negate() -> i32 { let x: i32 = i32::MIN; -x }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "negate".to_string(),
            params: Vec::new(),
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("-i32::MIN".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Neg,
                ty: Some("i32".to_string()),
                target: None,
                left: "i32::MIN".to_string(),
                right: None,
                expression: "-i32::MIN".to_string(),
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
            Err(VerificationError::IntegerNegationOverflow {
                function: "negate".to_string(),
                expression: "-i32::MIN".to_string(),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Add,
                ty: Some("i32".to_string()),
                target: None,
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x / y".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                ty: Some("i32".to_string()),
                target: None,
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
            &["y != 0", "y != -1"],
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x / y".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                ty: Some("i32".to_string()),
                target: None,
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
    fn semantic_arithmetic_catches_signed_division_overflow() {
        let metadata = metadata_named("divide", "pub fn divide(x: i32) -> i32 { x / { -1 } }", &[]);
        let semantics = TrustFunctionSemantics {
            rust_function_path: "divide".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x / -1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("-1".to_string()),
                expression: "x / -1".to_string(),
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
            Err(VerificationError::IntegerDivisionOverflow {
                function: "divide".to_string(),
                expression: "x / -1".to_string(),
            })
        );
    }

    #[test]
    fn semantic_arithmetic_proves_signed_division_overflow_precondition() {
        let metadata = metadata_named(
            "divide",
            "pub fn divide(x: i32) -> i32 { x / { -1 } }",
            &["x > i32::MIN"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "divide".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x / -1".to_string()),
            arithmetic_operations: vec![SemanticArithmeticOperation {
                kind: SemanticArithmeticKind::Div,
                ty: Some("i32".to_string()),
                target: None,
                left: "x".to_string(),
                right: Some("-1".to_string()),
                expression: "x / -1".to_string(),
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
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[{ let j = i; j }] }",
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("xs[i]".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
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
    fn partial_semantic_slice_indexes_fail_closed() {
        let metadata = metadata_named(
            "get_second",
            "pub fn get_second(xs: &[i32], i: usize, j: usize) -> i32 { let _a = xs[i]; xs[j] }",
            &["i < xs.len()"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "get_second".to_string(),
            params: vec![
                SemanticParam {
                    name: "xs".to_string(),
                    ty: "&[i32]".to_string(),
                },
                SemanticParam {
                    name: "i".to_string(),
                    ty: "usize".to_string(),
                },
                SemanticParam {
                    name: "j".to_string(),
                    ty: "usize".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("xs[j]".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
                expression: "xs[i]".to_string(),
                guards: Vec::new(),
            }],
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "get_second".to_string(),
                category: "slice index".to_string(),
                expression: "xs[j]".to_string(),
            })
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "i32".to_string(),
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
    fn semantic_slice_index_requires_matching_element_type() {
        let metadata = metadata_named(
            "get",
            "pub fn get(xs: &[i32], i: usize) -> i32 { xs[{ let j = i; j }] }",
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("xs[i]".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: vec![SemanticSliceIndex {
                base: "xs".to_string(),
                base_type: "&[i32]".to_string(),
                index: "i".to_string(),
                index_type: "usize".to_string(),
                element_type: "u8".to_string(),
                expression: "xs[i]".to_string(),
                guards: Vec::new(),
            }],
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "get".to_string(),
                category: "slice index".to_string(),
                expression: "xs[i]".to_string(),
            })
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
            "pub fn caller(x: i32) -> i32 { inc({ let y = x; y }) }",
            &["x < i32::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "caller".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("inc(x)".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
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
    fn partial_semantic_calls_fail_closed() {
        let inc = metadata_named(
            "inc",
            "pub fn inc(x: i32) -> i32 { x + 1 }",
            &["x < i32::MAX"],
        );
        let caller = metadata_named(
            "caller",
            "pub fn caller(x: i32, y: i32) -> i32 { let _ = inc(x); inc(y) }",
            &["x < i32::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "caller".to_string(),
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(
                &[inc, caller],
                &[semantics],
                VerificationOptions::default()
            ),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "caller".to_string(),
                category: "Trust call".to_string(),
                expression: "y<i32::MAX".to_string(),
            })
        );
    }

    #[test]
    fn semantic_call_merge_does_not_reintroduce_ambiguous_leaf_callee() {
        let left_inc = metadata_named(
            "outer::left::inc",
            "pub fn inc(x: i32) -> i32 { x }",
            &["x > 0"],
        );
        let right_inc = metadata_named(
            "outer::right::inc",
            "pub fn inc(x: i32) -> i32 { x }",
            &["x < 0"],
        );
        let caller = metadata_named(
            "outer::right::caller",
            "pub fn caller(x: i32) -> i32 { inc(x) }",
            &["x < 0"],
        );
        let right_callee = SemanticTrustCallee {
            rust_function_path: "outer::right::inc".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            preconditions: vec!["x < 0".to_string()],
        };
        let semantics = TrustFunctionSemantics {
            rust_function_path: "outer::right::caller".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "right::inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: Some(right_callee),
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(
                &[left_inc, right_inc, caller],
                &[semantics],
                VerificationOptions::default()
            ),
            Ok(())
        );
    }

    #[test]
    fn semantic_call_uses_embedded_trust_callee_contract_metadata() {
        let caller = metadata_named(
            "caller",
            "pub fn caller(x: i32) -> i32 { verified::inc({ x }) }",
            &["x < i32::MAX"],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "caller".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("verified::inc(x)".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "verified::inc".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: Some(SemanticTrustCallee {
                    rust_function_path: "inc".to_string(),
                    params: vec![SemanticParam {
                        name: "x".to_string(),
                        ty: "i32".to_string(),
                    }],
                    preconditions: vec!["x < i32::MAX".to_string()],
                }),
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[caller], &[semantics], VerificationOptions::default()),
            Ok(())
        );
    }

    #[test]
    fn executable_spec_calls_are_supported_without_callee_obligations() {
        let spec = executable_spec_metadata(
            "nonempty",
            "fn nonempty(xs: &[i32]) -> bool { xs.len() > 0 }",
        );
        let caller = metadata_named(
            "caller",
            "pub fn caller(xs: &[i32]) -> bool { nonempty(xs) }",
            &[],
        );

        assert_eq!(verify_totals(&[spec, caller]), Ok(()));
    }

    #[test]
    fn ghost_spec_calls_are_not_supported_runtime_callees() {
        let spec = ghost_spec_metadata("sorted", "fn sorted(xs: &[i32]) -> bool { true }");
        let caller = metadata_named(
            "caller",
            "pub fn caller(xs: &[i32]) -> bool { sorted(xs) }",
            &[],
        );

        assert_eq!(
            verify_totals(&[spec, caller]),
            Err(VerificationError::UnsupportedCall {
                function: "caller".to_string(),
                callee: "sorted".to_string(),
            })
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
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "inc".to_string(),
                args: vec!["x".to_string()],
                guards: vec!["x < i32::MAX".to_string()],
                trust_callee: None,
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
    fn semantic_call_rejects_unsupported_callee() {
        let metadata = metadata_named("abs_value", "pub fn abs_value(x: i32) -> i32 { x }", &[]);
        let semantics = TrustFunctionSemantics {
            rust_function_path: "abs_value".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "core::num::<impl i32>::abs".to_string(),
                args: vec!["x".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(verify_total(&metadata), Ok(()));
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::UnsupportedCall {
                function: "abs_value".to_string(),
                callee: "core::num::<impl i32>::abs".to_string(),
            })
        );
    }

    #[test]
    fn partial_semantic_unsupported_call_fails_closed() {
        let metadata = metadata_named(
            "abs_value",
            "pub fn abs_value(x: i32) -> i32 { x.abs() }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "abs_value".to_string(),
            params: vec![SemanticParam {
                name: "x".to_string(),
                ty: "i32".to_string(),
            }],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: Some("x".to_string()),
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_total(&metadata),
            Err(VerificationError::UnsupportedCall {
                function: "abs_value".to_string(),
                callee: "x.abs".to_string(),
            })
        );
        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::SemanticExtractionIncomplete {
                function: "abs_value".to_string(),
                category: "unsupported call".to_string(),
                expression: "x.abs".to_string(),
            })
        );
    }

    #[test]
    fn unsupported_index_diagnostic_precedes_semantic_index_call() {
        let metadata = metadata_named(
            "get_vec",
            "pub fn get_vec(xs: Vec<i32>, i: usize) -> i32 { xs[i] }",
            &[],
        );
        let semantics = TrustFunctionSemantics {
            rust_function_path: "get_vec".to_string(),
            params: vec![
                SemanticParam {
                    name: "xs".to_string(),
                    ty: "Vec<i32>".to_string(),
                },
                SemanticParam {
                    name: "i".to_string(),
                    ty: "usize".to_string(),
                },
            ],
            return_type: "i32".to_string(),
            local_types: Vec::new(),
            contract_bindings: Vec::new(),
            return_expression: None,
            arithmetic_operations: Vec::new(),
            slice_indexes: Vec::new(),
            calls: vec![SemanticCall {
                callee: "<Vec<i32> as Index<usize>>::index".to_string(),
                args: vec!["xs".to_string(), "i".to_string()],
                guards: Vec::new(),
                trust_callee: None,
            }],
            field_accesses: Vec::new(),
            matches: Vec::new(),
            branches: Vec::new(),
        };

        assert_eq!(
            verify_totals_with_semantics(&[metadata], &[semantics], VerificationOptions::default()),
            Err(VerificationError::UnsupportedIndex {
                function: "get_vec".to_string(),
                expression: "xs[i]".to_string(),
            })
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
