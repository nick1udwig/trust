# Trust v5 MVP Implementation Plan

**Document status:** Draft  
**Project name:** Trust  
**Scope:** Fastest path to a testable MVP implementation  
**Primary goal:** Build a vertical slice quickly, then widen feature coverage behind fixture tests.

---

## 1. Implementation strategy

Build Trust as a sequence of vertical slices.

Each slice should include:

- syntax accepted by macros;
- generated Rust that compiles;
- metadata emitted by macros;
- `trust-rustc` wrapper discovery;
- verification or deliberate rejection;
- fixture crate pass/fail test;
- diagnostic or runtime assertion check.

Avoid building large internal subsystems before an end-to-end test exists.

The first green target is not "prove interesting math." The first green target is:

```text
cargo test on a fixture crate using Trust syntax, through trust-rustc, with one verified trivial function.
```

After that, every feature lands with a fixture.

---

## 2. MVP repository shape

Recommended workspace:

```text
trust/
  Cargo.toml

  crates/
    trust/
      src/lib.rs

    trust-macros/
      src/lib.rs

    trust-core/
      src/lib.rs
      src/config.rs
      src/metadata.rs
      src/contracts.rs
      src/model.rs
      src/vc.rs
      src/solver.rs
      src/cache.rs
      src/diagnostics.rs

    trust-rustc/
      src/main.rs

    trust-model/
      src/lib.rs

    trust-test-support/
      src/lib.rs

  tests/
    fixtures/
      ...

    e2e/
      ...
```

Crate responsibilities:

```text
trust
  user-facing crate
  re-exports macros
  runtime assertion helpers
  hidden metadata marker APIs

trust-macros
  #[trust::module]
  trust::total!
  trust::spec!
  trust::proof!
  trust::loop_spec!
  trust::trusted_model!
  #[derive(TrustModel)]

trust-core
  metadata schema
  config parser
  contract parser/normalizer
  model definitions
  VC generator
  solver interface
  cache
  diagnostics

trust-rustc
  Cargo rustc wrapper binary
  active-wrapper handshake
  rustc forwarding
  metadata discovery
  compiler semantic extraction
  verification driver

trust-model
  built-in trusted models for MVP
  primitives, slices, Option, Result, TrustModel structs

trust-test-support
  fixture runner
  path normalization
  golden diagnostic helpers
  mock solver
```

---

## 3. Non-negotiable MVP constraints

The implementation should reject unsupported cases early.

Do not silently approximate:

- async;
- unsafe;
- trait dispatch;
- arbitrary method calls;
- closures in executable code;
- arbitrary iterators;
- full `Vec<T>` mutation;
- unmarked external structs;
- public ghost-only preconditions;
- user-declared trusted axioms.

`trust::trusted_model!` is an inert stub in the MVP. It must not add assumptions.

External user-defined types require:

```rust
#[derive(TrustModel)]
```

No `TrustModel`, no field reasoning.

---

## 4. Metadata channel

Use a simple, inspectable metadata channel first.

Recommended MVP expansion shape:

```rust
pub fn get(xs: &[i32], i: usize) -> i32 {
    ::trust::__rt::assert_precondition(
        i < xs.len(),
        "get",
        "i < xs.len()",
    );
    xs[i]
}

#[doc(hidden)]
#[allow(non_upper_case_globals)]
const __TRUST_META_get_abc123: &str = r#"{ ... }"#;
```

Why this shape:

- easy for rustc wrapper to find in HIR or expanded item stream;
- easy for tests to inspect;
- no linker-section complexity at first;
- stable enough for fixture tests;
- metadata schema can evolve behind a version field.

Metadata must include:

```text
schema_version
trust_macro_version
module_id
item_id
item_kind
source_span
rust_function_path
visibility
contracts_original
contracts_normalized
contract_classes
assertion_policy
body_hash_placeholder
trust_model_dependencies
```

The body hash may be filled by `trust-rustc` after semantic extraction.

---

## 5. Wrapper handshake

The macros must fail closed when Trust syntax is used without `trust-rustc`.

Mechanism:

1. `trust-rustc` invokes the real rustc.
2. `trust-rustc` sets:

   ```text
   TRUST_RUSTC_ACTIVE=1
   TRUST_RUSTC_VERSION=<version>
   ```

3. `trust-macros` reads the environment.
4. If a Trust macro expands and `TRUST_RUSTC_ACTIVE` is absent, emit:

   ```text
   error[trust]: Trust verification requires trust-rustc
   ```

Test modes may disable this only for macro unit tests.

Recommended test-only escape hatch:

```text
TRUST_MACRO_UNIT_TEST=1
```

Never document that as a user feature.

---

## 6. Milestone 0: workspace and fixture harness

### Goal

Cargo can run unit tests and fixture builds.

### Implement

- Workspace crates.
- Empty `trust` re-export crate.
- Placeholder `trust-rustc` binary that forwards to real rustc.
- `trust-test-support` fixture runner.
- Fixture directory layout.
- Path normalization for diagnostics.

### Tests

- `pass_no_trust`: ordinary crate builds through wrapper.
- `wrapper_forwards_rustc_args`: wrapper does not break normal Cargo builds.
- `fixture_runner_detects_pass_fail`: test harness can assert expected result.

### Done when

```bash
cargo test --workspace
```

runs fixture crates and proves that the wrapper can sit in the build path without changing ordinary Rust behavior.

---

## 7. Milestone 1: macro syntax skeleton

### Goal

Trust syntax compiles and emits ordinary Rust plus metadata.

### Implement

- `#[trust::module]` attribute.
- `trust::total!` with no contracts.
- Minimal parser: exactly one Rust `fn` item inside `total!`.
- Metadata const emitted next to generated function.
- Runtime crate path for metadata helpers.

Accepted source:

```rust
#[trust::module]
mod verified {
    trust::total! {
        pub fn id_i32(x: i32) -> i32 { x }
    }
}
```

Generated behavior:

- public Rust function exists;
- function body unchanged;
- metadata const emitted;
- no verification yet beyond trivial acceptance.

### Tests

- `pass_total_identity` fixture builds through wrapper.
- Macro expansion golden contains `pub fn id_i32`.
- Metadata golden contains item kind `total`.
- `fail_missing_wrapper` emits compile error.
- `fail_multiple_fn_in_total` rejected.

### Done when

A fixture crate can call `verified::id_i32(7)` from a Rust test and pass.

---

## 8. Milestone 2: wrapper discovers Trust metadata

### Goal

`trust-rustc` discovers macro-emitted metadata and reports it deterministically.

### Implement

- Metadata scan in compiler wrapper.
- JSON or compact metadata decode.
- Schema version check.
- Test-mode logging.
- Failure if metadata malformed.

Simplest acceptable first version:

- collect hidden consts with `__TRUST_META_` prefix after macro expansion;
- parse string contents;
- map to a function path/name.

### Tests

- `metadata_discovered_one_total`.
- `metadata_discovered_multiple_totals`.
- `metadata_unknown_schema_rejected`.
- `metadata_corrupt_rejected`.

### Done when

Fixture output can include:

```text
trust: discovered 1 total function
```

in deterministic test mode.

---

## 9. Milestone 3: trivial verifier and unsupported construct rejection

### Goal

Wrapper makes an actual verification decision.

### Implement

- MVP supported Rust subset checker.
- Accept trivial functions:

  ```rust
  pub fn id_i32(x: i32) -> i32 { x }
  pub fn is_zero(x: i32) -> bool { x == 0 }
  ```

- Reject obviously unsupported forms:

  ```rust
  async fn
  unsafe fn
  extern fn
  trait dispatch
  closure in executable body
  unsupported method call
  ```

- Diagnostics point to source spans.

### Tests

Passing:

- `pass_total_identity`.
- `pass_is_zero`.

Failing:

- `fail_async_total`.
- `fail_unsafe_total`.
- `fail_unknown_method_call`.
- `fail_trait_dispatch`.

### Done when

Build success depends on Trust's verifier accepting or rejecting functions, not merely macro expansion.

---

## 10. Milestone 4: executable `given` and runtime assertions

### Goal

Executable preconditions parse, generate runtime assertions, and feed verifier assumptions.

### Implement

Syntax:

```rust
trust::total! {
    given executable {
        i < xs.len();
    }

    pub fn get(xs: &[i32], i: usize) -> i32 {
        xs[i]
    }
}
```

Implement:

- contract parser;
- executable expression classifier;
- assertion injection for public functions;
- assertion policy: `always`, `debug`, `assume`;
- metadata for preconditions;
- verifier assumption environment.

### Tests

Passing:

- runtime assertion present under `always`;
- bad boundary input panics with correct message;
- `debug` changes assertion kind;
- `assume` emits no assertion and records warning metadata.

Failing:

- quantifier in executable precondition;
- unsupported function call in executable precondition;
- invalid assertion policy.

### Done when

The `get` runtime boundary behavior works even before full slice proof lands.

---

## 11. Milestone 5: integer arithmetic proof

### Goal

Prove or reject overflow for simple integer operations.

### Implement

Supported operations:

```text
+
-
*
unary -
```

Supported types first:

```text
i32
i64
usize
```

Implement VC generation:

```text
x + 1 on i32 requires x <= i32::MAX - 1
x - 1 on i32 requires x >= i32::MIN + 1
n - 1 on usize requires n >= 1
```

Start with a tiny arithmetic prover or solver adapter.

Minimum facts:

- facts from `given executable`;
- facts from `if` conditions;
- facts from loop conditions later.

### Tests

Pass:

```rust
trust::total! {
    given executable { x < i32::MAX; }
    pub fn add_one(x: i32) -> i32 { x + 1 }
}
```

Fail:

```rust
trust::total! {
    pub fn add_one(x: i32) -> i32 { x + 1 }
}
```

Pass:

```rust
trust::total! {
    pub fn abs_nonmin(x: i32) -> i32 {
        if x < 0 { -x } else { x }
    }
}
```

Expected fail unless precondition excludes `i32::MIN`.

### Done when

Overflow diagnostics are reliable and span the exact arithmetic expression.

---

## 12. Milestone 6: ghost `gives` and postcondition proof

### Goal

Prove simple postconditions.

### Implement

- `gives ghost |out| { ... }`.
- `gives executable |out| { ... }`.
- Return binder handling.
- `int(...)` in ghost specs.
- Basic equality and inequality VCs.
- Optional runtime postcondition assertion for executable postconditions.

### Tests

Pass:

```rust
trust::total! {
    gives executable |out| { out == x; }
    pub fn id_i32(x: i32) -> i32 { x }
}
```

Pass:

```rust
trust::total! {
    given executable { x < i32::MAX; }
    gives ghost |out| { int(out) == int(x) + 1; }
    pub fn add_one(x: i32) -> i32 { x + 1 }
}
```

Fail:

```rust
trust::total! {
    gives ghost |out| { out == 1; }
    pub fn zero() -> i32 { 0 }
}
```

### Done when

A false postcondition fails compilation.

---

## 13. Milestone 7: slices and indexing

### Goal

Prove slice bounds for read-only slices.

### Implement

Supported operations:

```rust
xs.len()
xs[i]
```

VC:

```text
index xs[i] requires i < xs.len()
```

Need model facts:

- `xs.len()` is a `usize`;
- index expression type is `usize`;
- indexed element type known;
- no mutable alias reasoning in MVP.

### Tests

Pass:

```rust
trust::total! {
    given executable { i < xs.len(); }
    gives ghost |out| { out == xs[i]; }
    pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }
}
```

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

### Done when

Safe indexing is the first compelling demo.

---

## 14. Milestone 8: public ghost precondition rejection

### Goal

Prevent uncheckable public boundaries.

### Implement

Rule:

- `pub` total functions may not have ghost-only preconditions unless each ghost condition is covered by an approved executable validator.
- Executable validators can be deferred.
- Therefore, MVP rejects public ghost preconditions.

### Tests

Fail:

```rust
trust::total! {
    given ghost { sorted(xs); }
    pub fn first_sorted(xs: &[i32]) -> i32 { xs[0] }
}
```

Pass:

```rust
trust::total! {
    given ghost { sorted(xs); }
    fn private_first_sorted(xs: &[i32]) -> i32 { xs[0] }
}
```

Private pass still requires proof of any body obligations.

### Done when

Boundary soundness has a hard test.

---

## 15. Milestone 9: Trust-to-Trust calls

### Goal

Verify callee preconditions at call sites.

### Implement

- Function environment inside a verified module.
- Contract lookup for Trust functions.
- Call expression extraction.
- VC generation for callee `given` clauses.

### Tests

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
    pub fn bad_first(xs: &[i32]) -> i32 { get(xs, 0) }
}
```

### Done when

A verified function can safely compose another verified function.

---

## 16. Milestone 10: `#[derive(TrustModel)]`

### Goal

Allow reasoning about external user-defined structs/enums only when explicitly modeled.

### Implement

- Derive macro for simple structs.
- Derive macro for simple enums if feasible.
- Field metadata.
- Type fingerprint.
- Visibility check.
- Reject unsupported generic/lifetime/union shapes.

Supported first:

```rust
#[derive(TrustModel)]
pub struct Account {
    pub id: u64,
    pub balance: i64,
}
```

### Tests

Pass:

```rust
trust::total! {
    pub fn balance(acct: Account) -> i64 { acct.balance }
}
```

Fail without derive:

```text
error[trust]: type Account must derive TrustModel before Trust may reason about its fields
```

Pass account transition:

```rust
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

### Done when

Trust can prove a small state transition over a user domain type.

---

## 17. Milestone 11: inert `trusted_model!` stubs

### Goal

Reserve syntax without adding unsound assumptions.

### Implement

```rust
trust::trusted_model! {
    fn external_hash(xs: &[u8]) -> [u8; 32];
}
```

Behavior:

- parse;
- emit metadata as inert;
- optional note in verbose/test mode;
- no axioms;
- no proof assumptions;
- no callable model unless separately implemented as a built-in model.

### Tests

Pass unused stub.

Fail false-proof attempt:

```rust
trust::trusted_model! {
    axiom false_is_true: false;
}

trust::total! {
    gives ghost |out| { out == 1; }
    pub fn zero() -> i32 { 0 }
}
```

Expected:

```text
error[trust]: could not prove postcondition
```

### Done when

The syntax exists but cannot create unsound proofs.

---

## 18. Milestone 12: loops and decreases

### Goal

Support simple `while` loops with explicit loop specs.

### Implement

Syntax:

```rust
trust::loop_spec! {
    invariant(i <= n);
    decreases(n - i);
}
while i < n {
    i += 1;
}
```

Verifier association rule:

- loop spec applies to immediately following `while` loop;
- reject if no loop follows;
- reject if loop has no spec;
- reject ambiguous multiple specs.

VCs:

- invariant initialization;
- preservation;
- decreases nonnegative;
- decreases strictly decreases;
- exit facts available after loop.

Restrictions:

- no `break`;
- no `continue`;
- no iterator loops;
- no mutation through aliases;
- simple local variable mutation only.

### Tests

Pass:

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

Fail:

- missing `decreases`;
- invariant not preserved;
- measure not decreasing;
- loop without spec;
- `break`/`continue`.

### Done when

A loop proof produces useful diagnostics when weakened or broken.

---

## 19. Milestone 13: cache

### Goal

Avoid re-verifying unchanged functions unsafely.

### Implement

Cache key includes:

```text
normalized contracts
semantic body fingerprint
TrustModel fingerprints
called Trust function fingerprints
built-in model version
target triple
pointer width
endianness
target features
enabled Cargo features
rustc version
Trust wrapper version
solver name/version/options
```

Cache values include:

```text
proved status only for accepted proof
VC fingerprints
diagnostics summary
generated Rust fingerprint
optional solver transcript path
```

Only `proved` may be reused as proof.

### Tests

- first build miss;
- second build hit;
- body change invalidates;
- contract change invalidates;
- target change invalidates;
- solver version change invalidates;
- corrupted cache ignored;
- concurrent write safe;
- timeout not cached as proof.

### Done when

Cache behavior is observable in fixture logs and never changes build correctness.

---

## 20. Milestone 14: diagnostics polish

### Goal

Proof failures are understandable enough for MVP users.

### Implement diagnostic categories:

```text
missing_wrapper
unsupported_feature
missing_trust_model
public_ghost_precondition
overflow_unproved
slice_index_unproved
callee_precondition_unproved
postcondition_unproved
loop_invariant_failure
loop_decreases_failure
solver_unknown
solver_timeout
cache_error
```

Each diagnostic should include:

- primary source span;
- failed obligation;
- relevant known facts;
- contract or invariant source, if relevant;
- help text;
- hidden generated code only in verbose mode.

### Tests

Golden diagnostics for every category.

### Done when

A user can fix `add_one`, `get`, `first`, and `countdown` based on diagnostics alone.

---

## 21. Solver integration path

### Stage A: mock solver

Use for unit tests and deterministic status handling.

Statuses:

```text
proved
counterexample
unknown
timeout
error
```

### Stage B: tiny internal prover

Support enough arithmetic/bounds facts for the first demos:

- conjunctions;
- simple inequalities;
- integer bounds;
- path conditions;
- direct equality substitution.

This keeps early fixtures fast and avoids blocking on solver packaging.

### Stage C: SMT backend

Add external or bundled SMT backend behind a solver trait.

Must include:

- solver version capture;
- timeout;
- deterministic options where possible;
- SMT-LIB dump in debug mode;
- cache key integration.

MVP can use Stage B for a narrow demo, but a serious MVP should include Stage C before public release.

---

## 22. Fastest vertical slice order

Implement in this order:

```text
1. wrapper forwards ordinary Cargo builds
2. macro accepts trust::total! with one Rust fn
3. macro emits metadata const
4. wrapper discovers metadata
5. trivial verifier accepts identity function
6. unsupported features rejected
7. executable given parsed and asserted at runtime
8. arithmetic overflow proof for i32/usize
9. ghost gives proof for equality/arithmetic
10. read-only slice bounds proof
11. public ghost precondition rejection
12. Trust-to-Trust call preconditions
13. derive(TrustModel) for simple structs
14. inert trusted_model! stubs
15. loops with loop_spec!
16. proof cache
17. diagnostics polish
```

This order gets a working build/test loop before hard verification features.

---

## 23. First fixture crate in detail

Path:

```text
tests/fixtures/pass_total_identity/
```

`Cargo.toml`:

```toml
[package]
name = "pass_total_identity"
version = "0.0.0"
edition = "2021"

[dependencies]
trust = { path = "../../../crates/trust" }

[package.metadata.trust]
assertions = "always"
solver = "mock"
```

`src/lib.rs`:

```rust
#[trust::module]
mod verified {
    trust::total! {
        pub fn id_i32(x: i32) -> i32 { x }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn id_i32_works() {
        assert_eq!(super::verified::id_i32(7), 7);
    }
}
```

Expected fixture behavior:

```text
cargo test succeeds
trust-rustc active
1 Trust total function discovered
1 Trust total function proved
0 proof cache hits on first run
```

This is the anchor test. Keep it green at all times.

---

## 24. Implementation risks and containment

### Risk: rustc integration too hard early

Containment:

- begin with metadata discovery and shallow body extraction;
- support only trivial bodies first;
- use fixture tests to drive extraction incrementally.

### Risk: solver integration delays everything

Containment:

- define solver trait immediately;
- use mock solver for status plumbing;
- use tiny internal prover for early arithmetic/slice demos;
- add SMT backend later without changing tests.

### Risk: diagnostics blocked on precise spans

Containment:

- store source spans in macro metadata;
- use generated-code spans only temporarily;
- add golden tests early for one diagnostic category.

### Risk: metadata format churn

Containment:

- version metadata schema;
- centralize serialization in `trust-core`;
- keep round-trip tests.

### Risk: public/private wrapper semantics unclear

Containment:

- choose one MVP rule and test it;
- recommended: public wrappers assert executable preconditions; Trust-to-Trust internal calls are statically checked; private ordinary-Rust-callable functions either assert or are rejected.

### Risk: inert trusted models accidentally become axioms

Containment:

- represent stubs as metadata kind `trusted_model_stub`;
- verifier ignores them for assumption generation;
- keep `fail_trusted_model_not_axiom` permanently.

---

## 25. MVP completion checklist

MVP implementation complete when all are true:

```text
[ ] Cargo build/test through trust-rustc works for fixture crates.
[ ] Trust macros fail closed without trust-rustc.
[ ] trust::total! emits Rust function plus metadata.
[ ] Executable preconditions produce runtime assertions.
[ ] Public ghost-only preconditions are rejected.
[ ] Arithmetic overflow proof works for basic i32/i64/usize cases.
[ ] Slice bounds proof works for read-only slices.
[ ] Simple postconditions are proved.
[ ] Trust-to-Trust callee preconditions are proved.
[ ] #[derive(TrustModel)] required and useful for simple structs.
[ ] trusted_model! stubs are accepted but ignored by the verifier.
[ ] Simple loops require loop_spec and prove decreases.
[ ] Unsupported Rust features fail with clear diagnostics.
[ ] Cache hits and invalidation are tested.
[ ] Solver statuses fail closed except proved.
[ ] Golden diagnostics cover main failure modes.
```
