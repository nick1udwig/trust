# Trust v5 MVP Test Plan

**Document status:** Draft  
**Project name:** Trust  
**Scope:** MVP implementation tests for Rust-native verified islands  
**Primary goal:** Establish a fast build-and-test loop across macros, wrapper integration, metadata, verification, diagnostics, runtime assertions, and cache behavior.

---

## 1. Testing principles

Trust must be tested as a compiler tool, not only as a library.

The MVP needs four test layers:

1. **Unit tests**
   - Parser fragments.
   - Metadata serialization.
   - Contract classification.
   - VC generation.
   - Solver result handling.
   - Cache key construction.

2. **Macro expansion tests**
   - `trust::total!` emits valid Rust.
   - `#[trust::module]` accepts and rejects the right items.
   - `#[derive(TrustModel)]` emits type metadata.
   - Runtime assertions are generated correctly.

3. **Compiler-wrapper integration tests**
   - `trust-rustc` is actually invoked.
   - Trust metadata is discoverable after macro expansion.
   - Target triple, Cargo features, rustc version, and config are captured.
   - Verification failure fails the build.

4. **Fixture crate tests**
   - Real Cargo projects under `tests/fixtures/`.
   - Each fixture is built with Cargo through the Trust wrapper.
   - Expected pass/fail result and expected diagnostic fragments checked.

The MVP should prioritize a small number of high-signal end-to-end fixtures over broad unit coverage alone. Trust's main failure mode will be integration mismatch between macro output, rustc wrapper extraction, verification, and diagnostics.

---

## 2. Recommended test workspace layout

```text
trust/
  crates/
    trust/
    trust-macros/
    trust-core/
    trust-rustc/
    trust-model/
    trust-test-support/

  tests/
    fixtures/
      pass_no_trust/
      pass_total_identity/
      pass_runtime_assert_get/
      fail_missing_wrapper/
      fail_public_ghost_precondition/
      fail_overflow_unproved/
      fail_slice_index_unproved/
      pass_add_one_precondition/
      pass_get_precondition/
      fail_callee_precondition/
      pass_trust_model_struct/
      fail_missing_trust_model/
      pass_trusted_model_stub_ignored/
      pass_loop_countdown/
      fail_loop_invariant_not_preserved/
      pass_cache_hit/
      fail_cache_target_invalidation/

    golden/
      diagnostics/
      expanded/
      metadata/
      vcs/

  xtask/
    src/main.rs
```

`xtask` is optional. The same flows can be run by ordinary `cargo test` if preferred.

---

## 3. Test commands

Recommended developer commands:

```bash
cargo test -p trust-core
cargo test -p trust-macros
cargo test -p trust-rustc
cargo test -p trust-test-support
cargo test --workspace
```

Recommended fixture command shape:

```bash
cargo test -p trust-e2e
```

`trust-e2e` should spawn Cargo on fixture crates with controlled environment variables:

```text
RUSTC_WORKSPACE_WRAPPER=<path-to-built-trust-rustc>
TRUST_TEST_MODE=1
TRUST_SOLVER=mock|tiny|z3
TRUST_CACHE_DIR=<tempdir>
```

The wrapper should also support a test-only mode that makes diagnostics deterministic:

```text
TRUST_TEST_DETERMINISTIC=1
```

That mode should normalize:

- absolute paths;
- temporary directory names;
- rustc invocation noise;
- solver timing;
- cache paths;
- random seeds.

---

## 4. Component test matrix

### 4.1 `trust` runtime crate

Purpose:

- Public support APIs.
- Runtime assertion helper.
- Hidden metadata marker types.
- Stable paths used by generated code.

Unit tests:

| Test | Expected result |
|---|---|
| `precondition_message_format` | Message includes Trust, function name, condition text. |
| `postcondition_message_format` | Message includes Trust, function name, condition text, bug/soundness wording. |
| `assert_policy_always` | Emits active `assert!`. |
| `assert_policy_debug` | Emits debug assertion path. |
| `assert_policy_assume` | Emits no runtime check and records warning metadata. |

Runtime tests:

```rust
#[should_panic(expected = "Trust precondition failed in get: i < xs.len()")]
fn get_panics_on_bad_boundary_input() { ... }
```

---

### 4.2 `trust-macros`: `#[trust::module]`

Purpose:

- Mark verified modules.
- Restrict MVP module contents.
- Emit module-level metadata.

Pass tests:

```rust
#[trust::module]
mod verified {
    trust::total! {
        pub fn is_zero(x: i32) -> bool { x == 0 }
    }
}
```

Reject tests:

```rust
#[trust::module]
mod verified {
    pub fn unverified_helper(x: i32) -> i32 { x }
}
```

Expected diagnostic:

```text
error[trust]: unverified Rust functions are not allowed inside #[trust::module] in the MVP
```

Other tests:

| Test | Expected result |
|---|---|
| `module_accepts_use_items` | `use super::T;` accepted. |
| `module_accepts_trust_model_types` | Structs/enums with `#[derive(TrustModel)]` accepted. |
| `module_rejects_unsafe_mod` | `unsafe` items rejected in verified module. |
| `nested_module_rejected_or_ignored` | Deterministic MVP behavior. |

---

### 4.3 `trust-macros`: `trust::total!`

Purpose:

- Parse Trust contract blocks.
- Preserve Rust function syntax.
- Emit Rust item plus metadata.
- Insert runtime assertions for executable public preconditions.

Pass fixtures:

```rust
trust::total! {
    pub fn is_zero(x: i32) -> bool { x == 0 }
}
```

```rust
trust::total! {
    given executable { i < xs.len(); }

    gives ghost |out| { out == xs[i]; }

    pub fn get(xs: &[i32], i: usize) -> i32 {
        xs[i]
    }
}
```

Macro expansion golden checks:

- A public Rust function named `get` exists.
- The function body contains a runtime precondition assertion for `i < xs.len()` when assertions are `always`.
- A hidden metadata constant exists.
- The metadata contains original contract text, normalized contract AST, function path, visibility, span data, and assertion policy.

Reject tests:

| Source pattern | Expected diagnostic |
|---|---|
| Multiple function items in one `total!` | `expected exactly one Rust function item` |
| `given executable` after function item | `contract blocks must precede function item` |
| malformed `gives |out|` binder | precise parse error |
| `async fn` | `async functions are not supported in Trust MVP` |
| `unsafe fn` | `unsafe functions are not supported in Trust MVP` |
| `extern fn` | `extern functions are not supported in Trust MVP` |
| generic fn, if not MVP | `generic total functions are not supported in MVP` |

---

### 4.4 `trust-macros`: `trust::spec!`

Purpose:

- Accept executable and ghost spec declarations.
- Emit metadata.
- Do not emit public executable code unless explicitly executable and supported.

Pass tests:

```rust
trust::spec! {
    ghost fn sorted(xs: &[i32]) -> bool {
        forall(|i: usize, j: usize| implies(i < j && j < xs.len(), xs[i] <= xs[j]))
    }
}
```

```rust
trust::spec! {
    executable fn nonempty(xs: &[i32]) -> bool {
        xs.len() > 0
    }
}
```

Reject tests:

| Source pattern | Expected diagnostic |
|---|---|
| Quantifier in executable spec | `quantifiers are ghost-only` |
| Unsupported call in executable spec | `not executable in runtime assertion context` |
| Non-boolean spec return for predicate position | `expected bool` |

---

### 4.5 `trust-macros`: `trust::proof!`

Purpose:

- Accept proof function syntax.
- Emit proof metadata.
- Generate no runtime code except hidden metadata.

Pass tests:

```rust
trust::proof! {
    fn le_trans(a: i32, b: i32, c: i32)
    given ghost { a <= b; b <= c; }
    gives ghost { a <= c; }
    {
        assert(a <= c);
    }
}
```

Reject tests:

| Source pattern | Expected diagnostic |
|---|---|
| Proof with `pub` export | `proof functions are erased and cannot be public Rust APIs` |
| Proof uses runtime-only call | `unsupported proof step` |
| Empty proof body for nontrivial lemma | rejected unless solver proves obligations automatically |

---

### 4.6 `trust-macros`: `trust::trusted_model!` MVP stub

Purpose:

- Reserve syntax.
- Emit metadata saying the stub was ignored.
- Do not add axioms.
- Do not affect verification.

Pass test:

```rust
trust::trusted_model! {
    fn external_hash(xs: &[u8]) -> [u8; 32];
}
```

Expected behavior:

- Build still succeeds if the stub is unused.
- Metadata records the declaration as inert.
- Verifier does not add any assumptions.
- Optional diagnostic in verbose/test mode:

```text
note[trust]: trusted_model declarations are inert stubs in the MVP and were ignored
```

Important negative test:

```rust
trust::trusted_model! {
    axiom false_is_true: false;
}

trust::total! {
    gives ghost |out| { out == 1; }
    pub fn zero() -> i32 { 0 }
}
```

Expected result:

```text
error[trust]: could not prove postcondition
```

The inert trusted model must not let the false postcondition pass.

---

### 4.7 `trust-macros`: `trust::loop_spec!`

Purpose:

- Attach loop invariants and decreases clauses to the immediately following loop.
- Expand to no-op Rust plus metadata.

Pass source:

```rust
trust::loop_spec! {
    invariant(i <= n);
    decreases(n - i);
}
while i < n {
    i += 1;
}
```

Tests:

| Test | Expected result |
|---|---|
| `loop_spec_before_while` | Metadata associated with following loop. |
| `loop_spec_without_loop` | Rejected by verifier. |
| `two_specs_one_loop` | Rejected as ambiguous. |
| `loop_without_spec` | Rejected in total function if loop termination not otherwise supported. |
| `break_continue_rejected` | Unsupported in MVP. |

---

### 4.8 `#[derive(TrustModel)]`

Purpose:

- Required for external user-defined types used in Trust contracts or bodies.
- Emit field/type metadata.
- Reject unsupported shape.

Pass source:

```rust
#[derive(TrustModel)]
pub struct Account {
    pub id: u64,
    pub balance: i64,
}
```

Pass source:

```rust
#[derive(TrustModel)]
pub enum Small {
    A,
    B(i32),
}
```

Reject tests:

| Source pattern | Expected diagnostic |
|---|---|
| Generic struct, if unsupported | `generic TrustModel types are not supported in MVP` |
| Lifetime parameter, if unsupported | `lifetime parameters are not supported in TrustModel MVP` |
| Private field used across module boundary | visibility diagnostic |
| Field with unsupported type | `field type is not supported by TrustModel MVP` |
| `unsafe`/union | `unions are not supported by TrustModel` |

Integration test:

```rust
pub struct Account { pub id: u64, pub balance: i64 }

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        pub fn balance(acct: Account) -> i64 { acct.balance }
    }
}
```

Expected diagnostic:

```text
error[trust]: type Account must derive TrustModel before Trust may reason about its fields
```

---

## 5. Metadata tests

Trust metadata is the contract between macros and `trust-rustc`.

### 5.1 Metadata content

Every `trust::total!` item should produce metadata containing:

- item kind: `total`;
- stable item name/path;
- visibility;
- original source span range;
- generated Rust function path;
- contract blocks, original text;
- contract blocks, normalized AST;
- contract class: executable or ghost;
- return binder name for `gives`;
- assertion policy;
- module identity;
- Trust macro version;
- metadata schema version.

### 5.2 Round-trip tests

Unit tests:

```text
metadata_json_roundtrip
metadata_rejects_unknown_schema
metadata_preserves_contract_order
metadata_hash_stable_under_whitespace
metadata_hash_changes_on_contract_change
metadata_hash_changes_on_body_change
metadata_hash_changes_on_target_change
```

### 5.3 Span mapping tests

Golden tests should assert that proof failures point at user source, not generated code.

Example source:

```rust
pub fn add_one(x: i32) -> i32 {
    x + 1
}
```

Expected diagnostic span points to:

```text
x + 1
```

not to hidden generated modules.

---

## 6. `trust-rustc` wrapper tests

### 6.1 Forwarding behavior

Fixture: `pass_no_trust`

```rust
pub fn ordinary(x: i32) -> i32 { x }
```

Expected:

- Cargo build succeeds.
- Wrapper forwards to rustc.
- No Trust verification needed.

### 6.2 Active wrapper handshake

Fixture: `fail_missing_wrapper`

Uses Trust macros but no `RUSTC_WORKSPACE_WRAPPER`.

Expected:

```text
error[trust]: Trust verification requires trust-rustc
```

Test inverse:

- Wrapper sets `TRUST_RUSTC_ACTIVE=1` for rustc.
- Macros do not fail closed.

### 6.3 Metadata discovery

Fixture: `pass_total_identity`

Expected wrapper log in test mode:

```text
trust: discovered 1 Trust total function
```

### 6.4 Target context capture

Test with controlled target values.

Expected captured context includes:

- target triple;
- pointer width;
- endianness;
- target features;
- enabled Cargo features;
- rustc version;
- Trust wrapper version.

### 6.5 Wrapper failure propagation

If verification fails:

- Cargo exits nonzero.
- rustc does not silently emit final artifact as if verified.
- Diagnostic is emitted once, not duplicated by multiple rustc phases.

---

## 7. Rust semantic extraction tests

Purpose:

- Confirm `trust-rustc` can map macro metadata to real compiler items.
- Extract enough typed information for MVP verification.

Tests:

| Test | Expected extraction |
|---|---|
| `extract_identity_body` | Args, return type, body local, return expression. |
| `extract_if_expr` | Branch CFG and path conditions. |
| `extract_match_option` | Variant tests and payload bindings. |
| `extract_slice_index` | Base expression, index expression, element type. |
| `extract_integer_add` | Operation kind and concrete Rust integer type. |
| `extract_field_read_trustmodel` | Field name, type, owner type metadata. |
| `extract_function_call` | Resolved callee path and Trust contract metadata if Trust function. |
| `reject_method_call_unknown` | Unsupported method call diagnostic. |
| `reject_trait_dispatch` | Unsupported trait call diagnostic. |
| `reject_closure` | Unsupported closure diagnostic, except quantifier syntax in specs. |

---

## 8. Contract classification tests

Purpose:

- Ensure executable contracts can become runtime assertions.
- Ensure ghost contracts never accidentally generate runtime code.

Executable pass:

```rust
given executable {
    i < xs.len();
    x >= 0;
}
```

Executable reject:

```rust
given executable {
    forall(|i: usize| i < xs.len());
}
```

Expected:

```text
error[trust]: quantifiers are not executable
```

Ghost pass:

```rust
gives ghost |out| {
    forall(|i: usize| implies(i < xs.len(), out >= xs[i]));
}
```

Public boundary reject:

```rust
trust::total! {
    given ghost { sorted(xs); }
    pub fn insert_sorted(xs: &[i32], x: i32) -> usize { 0 }
}
```

Expected:

```text
error[trust]: public function has ghost-only precondition
```

---

## 9. Verification core tests

### 9.1 Empty/trivial proof

Pass:

```rust
trust::total! {
    pub fn id_i32(x: i32) -> i32 { x }
}
```

Obligations:

- Body supported.
- Terminates trivially.
- No partial operations.

### 9.2 Postcondition proof

Pass:

```rust
trust::total! {
    gives executable |out| { out == x; }
    pub fn id_i32(x: i32) -> i32 { x }
}
```

Fail:

```rust
trust::total! {
    gives executable |out| { out == x + 1; }
    pub fn id_i32(x: i32) -> i32 { x }
}
```

Expected:

```text
error[trust]: could not prove postcondition
```

### 9.3 Integer overflow

Fail:

```rust
trust::total! {
    pub fn add_one(x: i32) -> i32 { x + 1 }
}
```

Expected:

```text
error[trust]: could not prove integer addition cannot overflow
```

Pass:

```rust
trust::total! {
    given executable { x < i32::MAX; }
    gives ghost |out| { int(out) == int(x) + 1; }
    pub fn add_one(x: i32) -> i32 { x + 1 }
}
```

### 9.4 Slice bounds

Fail:

```rust
trust::total! {
    pub fn first(xs: &[i32]) -> i32 { xs[0] }
}
```

Pass:

```rust
trust::total! {
    given executable { xs.len() > 0; }
    pub fn first(xs: &[i32]) -> i32 { xs[0] }
}
```

### 9.5 Callee preconditions

Pass:

```rust
trust::total! {
    given executable { i < xs.len(); }
    pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }
}

trust::total! {
    given executable { xs.len() > 0; }
    pub fn first(xs: &[i32]) -> i32 { get(xs, 0) }
}
```

Fail:

```rust
trust::total! {
    pub fn maybe_first(xs: &[i32]) -> i32 { get(xs, 0) }
}
```

Expected:

```text
error[trust]: could not prove callee precondition
```

### 9.6 `old(...)`

Pass:

```rust
#[derive(TrustModel)]
pub struct Account { pub id: u64, pub balance: i64 }

trust::total! {
    given executable { amount >= 0; acct.balance >= amount; }
    gives ghost |out| {
        out.id == old(acct.id);
        int(out.balance) == int(old(acct.balance)) - int(old(amount));
    }
    pub fn withdraw(acct: Account, amount: i64) -> Account {
        Account { id: acct.id, balance: acct.balance - amount }
    }
}
```

Fail variant:

- Body changes `id`.
- Expected postcondition failure.

### 9.7 Option match

Pass:

```rust
trust::total! {
    pub fn unwrap_or_zero(x: Option<i32>) -> i32 {
        match x {
            Some(v) => v,
            None => 0,
        }
    }
}
```

Fail:

```rust
trust::total! {
    pub fn bad_unwrap(x: Option<i32>) -> i32 {
        x.unwrap()
    }
}
```

Expected:

```text
error[trust]: unchecked unwrap is not supported; prove Some or use match
```

---

## 10. Loop verification tests

### 10.1 Countdown pass

```rust
trust::total! {
    gives executable |out| { out == 0; }
    pub fn countdown(mut n: usize) -> usize {
        trust::loop_spec! {
            invariant(n >= 0);
            decreases(n);
        }
        while n > 0 {
            n = n - 1;
        }
        n
    }
}
```

Expected:

- Termination proved.
- No underflow because body path has `n > 0`.
- Postcondition proved.

### 10.2 Missing decreases fail

```rust
trust::loop_spec! {
    invariant(i <= n);
}
while i < n { i += 1; }
```

Expected:

```text
error[trust]: loop in total function requires decreases measure
```

### 10.3 Invariant not preserved fail

```rust
trust::loop_spec! {
    invariant(i <= n);
    decreases(n - i);
}
while i < n {
    i = i + 2;
}
```

Expected:

```text
error[trust]: loop invariant may not be preserved
```

### 10.4 Decreases not decreasing fail

```rust
trust::loop_spec! {
    invariant(i <= n);
    decreases(n - i);
}
while i < n {
    // i unchanged
}
```

Expected:

```text
error[trust]: loop decreases measure may not strictly decrease
```

---

## 11. Solver backend tests

### 11.1 Solver trait unit tests

Use a deterministic mock solver for these statuses:

```text
proved
counterexample
unknown
timeout
solver_error
```

Tests:

| Solver status | Compiler result |
|---|---|
| proved | build may continue |
| counterexample | verification failure |
| unknown | verification failure, not logical disproof |
| timeout | verification failure, not logical disproof |
| solver_error | internal/tooling failure |

### 11.2 Tiny arithmetic solver tests

Before full SMT integration, a tiny internal solver can support MVP arithmetic fragments:

- conjunctions of integer inequalities;
- `x < MAX` implies `x + 1` safe;
- `i < len` proves slice bound;
- path condition from `if` and `while` condition.

This gives a fast test loop before external solver packaging is complete.

### 11.3 SMT golden output

For each VC kind, keep optional golden SMT-LIB output:

```text
vcs/add_one_overflow.smt2
vcs/get_bounds.smt2
vcs/callee_precondition.smt2
vcs/countdown_decreases.smt2
```

Golden tests should normalize symbol names.

---

## 12. Cache tests

### 12.1 Cache hit

Fixture: `pass_cache_hit`

Run same Cargo build twice with same cache dir.

Expected first run:

```text
trust: verified 1 function; cache hits 0; cache misses 1
```

Expected second run:

```text
trust: verified 1 function; cache hits 1; cache misses 0
```

### 12.2 Cache invalidation

Change each input and assert cache miss:

| Changed input | Expected result |
|---|---|
| function body | miss |
| executable precondition | miss |
| ghost postcondition | miss |
| called Trust function contract | miss |
| target triple | miss or separate cache entry |
| pointer width | miss |
| Trust version | miss |
| solver version | miss |
| solver options | miss |
| enabled Cargo feature | miss |
| `TrustModel` field type | miss |

### 12.3 Cache safety

Tests:

- corrupted cache entry ignored and recomputed;
- partially written cache entry ignored;
- concurrent builds do not corrupt cache;
- timeout is not cached as proof;
- unknown is not cached as proof;
- `proved` entry includes all required context fields.

---

## 13. Configuration tests

Fixture variants:

```toml
[package.metadata.trust]
assertions = "always"
```

```toml
[package.metadata.trust]
assertions = "debug"
```

```toml
[package.metadata.trust]
assertions = "assume"
```

Tests:

| Config | Expected behavior |
|---|---|
| missing config | defaults applied |
| invalid assertion policy | compile failure |
| invalid solver name | compile failure |
| timeout value invalid | compile failure |
| workspace/member config conflict | deterministic diagnostic |
| config changed | proof cache invalidated if proof-affecting |

---

## 14. Runtime assertion tests

### 14.1 Public precondition assertion

```rust
trust::total! {
    given executable { i < xs.len(); }
    pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }
}
```

Runtime test:

```rust
#[test]
#[should_panic(expected = "Trust precondition failed in get: i < xs.len()")]
fn get_bad_index_panics() {
    verified::get(&[], 0);
}
```

### 14.2 Private internal Trust call

Private Trust-to-Trust calls should be statically verified.

Runtime assertion policy for private helpers should be explicit in the implementation.

Recommended MVP behavior:

- public wrappers assert executable preconditions;
- private generated bodies do not assert when called from verified Trust code;
- if a private function remains callable by ordinary Rust due to visibility, emit assertions or reject.

Test whichever rule is chosen.

### 14.3 Ghost preconditions never asserted

```rust
given ghost { sorted(xs); }
```

Expected:

- no runtime assertion generated;
- public function rejected unless all ghost preconditions are also covered by executable checks or function is not public.

---

## 15. Diagnostics golden tests

Each diagnostic should test:

- error code or category;
- source span;
- obligation kind;
- known facts when available;
- help text;
- no references to hidden generated code unless in verbose mode.

Golden diagnostic categories:

```text
missing_wrapper
unsupported_async
unsupported_trait_call
missing_trust_model
ghost_public_precondition
overflow_unproved
slice_index_unproved
callee_precondition_unproved
postcondition_unproved
loop_missing_decreases
loop_invariant_not_preserved
solver_unknown
solver_timeout
cache_corrupt_recomputed
trusted_model_stub_ignored
```

---

## 16. End-to-end fixture list

### 16.1 Passing fixtures

| Fixture | Purpose |
|---|---|
| `pass_no_trust` | Wrapper forwards normal crate. |
| `pass_total_identity` | First Trust function, no contracts. |
| `pass_is_zero` | Simple boolean postcondition. |
| `pass_add_one_precondition` | Integer overflow proof. |
| `pass_get_precondition` | Slice bounds proof + runtime assertion. |
| `pass_callee_precondition` | Trust-to-Trust call proof. |
| `pass_option_match` | Supported enum match. |
| `pass_trust_model_struct` | `#[derive(TrustModel)]` field reasoning. |
| `pass_withdraw_account` | Old-state and arithmetic proof. |
| `pass_trusted_model_stub_ignored` | Stub accepted, no axioms. |
| `pass_loop_countdown` | Loop decreases proof. |
| `pass_cache_hit` | Cache reused safely. |

### 16.2 Failing fixtures

| Fixture | Purpose |
|---|---|
| `fail_missing_wrapper` | Trust macros require wrapper. |
| `fail_unverified_fn_in_module` | Module restriction. |
| `fail_async_total` | Unsupported async. |
| `fail_unsafe_total` | Unsupported unsafe. |
| `fail_public_ghost_precondition` | Boundary safety. |
| `fail_overflow_unproved` | Arithmetic safety. |
| `fail_slice_index_unproved` | Bounds safety. |
| `fail_callee_precondition` | Call obligation. |
| `fail_missing_trust_model` | External type metadata required. |
| `fail_postcondition_false` | Contract proof failure. |
| `fail_loop_missing_decreases` | Termination obligation. |
| `fail_loop_invariant_not_preserved` | Preservation obligation. |
| `fail_trusted_model_not_axiom` | Stub cannot prove false. |
| `fail_solver_unknown` | Unknown fails closed. |

---

## 17. Minimal green loop target

The first useful end-to-end test should be:

```rust
#[trust::module]
mod verified {
    trust::total! {
        pub fn id_i32(x: i32) -> i32 { x }
    }
}

#[test]
fn id_works() {
    assert_eq!(verified::id_i32(7), 7);
}
```

It should exercise:

- Cargo invokes `trust-rustc`;
- wrapper sets active handshake;
- macro accepts `trust::total!`;
- macro emits function and metadata;
- wrapper discovers metadata;
- verifier accepts trivial function;
- Cargo build succeeds;
- test binary runs.

That is the first real build/test loop.

---

## 18. MVP exit test suite

MVP should not be considered implemented until these fixture categories are green:

```text
macro parsing and expansion
wrapper active/missing behavior
metadata round-trip
TrustModel derive required and used
primitive arithmetic verification
slice bound verification
callee precondition verification
public ghost precondition rejection
runtime precondition assertion
loop decreases verification
inert trusted_model stubs
cache hit/miss/invalidation
clear diagnostics
```

A smaller demo can ship earlier, but not as the MVP.
