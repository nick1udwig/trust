# Trust

Trust verifies selected Rust functions during ordinary Cargo builds.

The current MVP supports verified islands: Rust functions wrapped in Trust macros,
checked by the `trust-rustc` compiler wrapper, with runtime assertions for public
executable preconditions.

## Current Status

Implemented:

- `#[trust::module]`
- `trust::total!`
- `trust::spec!`
- `trust::proof!`
- `trust::loop_spec!`
- inert `trust::trusted_model!`
- `#[derive(TrustModel)]`
- `trust-rustc` wrapper integration and fail-closed macro handshake
- package config under `[package.metadata.trust]`
- local proof cache with target, rustc, solver, config, contract, and body invalidation
- crate-backed vendored Z3 backend, plus mock solver status tests
- target-aware `usize` proof bounds using the rustc target pointer width
- optional SMT-LIB query dumps with `TRUST_SMT_DUMP_DIR`
- optional rustc HIR/MIR semantic dumps with `TRUST_SEMANTIC_DUMP_DIR`
- runtime public precondition assertions
- MVP checks for `i32`, `i64`, `u32`, `u64`, and `usize` integer overflow,
  signed division/remainder overflow, division and remainder by zero, slice bounds,
  Trust-to-Trust callee preconditions, executable Trust spec calls, simple
  postconditions, proof assertions, loop invariants/decreases, and required
  `TrustModel` field reasoning

Still intentionally limited:

- no async, unsafe, extern, generic total functions, trait dispatch, closures, recursion,
  arbitrary Rust calls, arbitrary method calls, iterators, mutation-heavy collection models,
  or user-declared trusted axioms
- diagnostics are deterministic and tested, but source-span quality is still MVP-level
- Trust macros and `#[derive(TrustModel)]` inside verified modules, including
  nested modules, record module-qualified metadata paths, but Trust metadata
  emitted from ordinary unannotated Rust modules still does not receive full
  Rust module paths
- verification still mostly uses the metadata/token verifier plus default
  HIR/MIR extraction; extracted compiler facts feed return expressions, arithmetic
  operations with concrete Rust integer types, slice index facts with element and
  index types, supported slice `len` receiver and slice-index base alias
  resolution, resolved primitive integer `checked_add` calls, call arguments with
  simple branch guards and attached Trust callee preconditions, `if` branch
  return facts, branch-assignment and carried-local return facts at simple MIR
  joins, Option/Result match arms, loop decreases targets and loop exit facts
  from compiler branch guards, source-local type bindings for local loop
  measures, and TrustModel field projections,
  including through unambiguous local aliases, field types, missing-TrustModel
  field checks, unsupported shift/bitwise/cast-operation and
  signature/source-local type checks, opaque contract-reasoning checks, path
  guards, and struct returns into verification, but it is not the primary VC
  generator yet

## Toolchain

Use Rust 1.90.0 for this workspace:

```bash
rustup override set 1.90.0
```

Build the wrapper:

```bash
cargo build -p trust-rustc --release
```

## Using Trust In A Crate

Add Trust as a dependency:

```toml
[dependencies]
trust = { path = "/path/to/trust/crates/trust" }
```

Configure Cargo to build through the Trust wrapper:

```toml
# .cargo/config.toml
[build]
rustc-workspace-wrapper = "/path/to/trust/target/release/trust-rustc"
```

Configure Trust:

```toml
# Cargo.toml
[package.metadata.trust]
assertions = "always"
solver = "z3"
timeout_ms = 5000
cache = "local"
# Optional: silence the warning for assertions = "assume".
silence_assume_warning = false
```

Then normal Cargo commands verify Trust code:

```bash
cargo build
cargo test
```

## Example

```rust
#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < i32::MAX;
        }

        gives ghost |out| {
            int(out) == int(x) + 1;
        }

        pub fn add_one(x: i32) -> i32 {
            x + 1
        }
    }
}
```

Public executable preconditions are asserted at runtime. In the example above,
calling `add_one(i32::MAX)` panics instead of crossing an unchecked public boundary.

## Useful Environment Variables

- `TRUST_CACHE_DIR=/path/to/cache`: enables proof-cache reuse.
- `TRUST_SMT_DUMP_DIR=/path/to/dumps`: writes SMT-LIB queries for z3-backed VCs.
- `TRUST_SEMANTIC_DUMP_DIR=/path/to/dumps`: writes rustc `hir-tree`, MIR, and
  a Trust-to-compiler-item summary for Trust crates, including resolved MIR
  function paths and typed contract identifier bindings when rustc exposes them.
- `TRUST_SEMANTIC_VERIFY=0`: debug escape hatch that disables feeding default
  rustc HIR/MIR facts into verification when semantic dumps are not requested.
  By default, Trust extracts HIR/MIR for Trust verification items and feeds
  supported facts, including return expressions, guarded arithmetic operations,
  concrete Rust integer types for arithmetic obligations, guarded slice indexes
  with element and index types, supported slice `len` receiver and slice-index
  base alias resolution, resolved primitive integer `checked_add` calls, guarded
  call arguments with attached Trust callee preconditions, `if` branch return
  facts, branch-assignment and carried-local return facts at simple MIR joins,
  Option/Result match arms, loop decreases targets, loop exit facts, and
  source-local type bindings for local loop measures, and TrustModel field
  projections in returns,
  unambiguous local aliases, and path guards plus field types for arithmetic
  obligations, missing-TrustModel checks, and unsupported
  shift/bitwise/cast-operation, signature/source-local type checks, and opaque
  contract-reasoning checks, into verification.
- `TRUST_SOLVER_VERSION=...`: test/debug override for solver-version cache keys.
- `TRUST_SOLVER_STATUS=proved|counterexample|unknown|timeout|solver_error`: mock solver status for tests.

## Testing This Repository

Run the full suite:

```bash
cargo test --workspace
```

The e2e suite builds fixture crates through `trust-rustc` and covers pass/fail
behavior, cache invalidation, runtime assertions, solver failures, diagnostics,
and Z3-backed verification.
