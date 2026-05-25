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
- optional SMT-LIB query dumps with `TRUST_SMT_DUMP_DIR`
- runtime public precondition assertions
- MVP checks for integer overflow, division and remainder by zero, slice bounds,
  Trust-to-Trust callee preconditions, simple postconditions, proof assertions,
  loop invariants/decreases, and required `TrustModel` field reasoning

Still intentionally limited:

- no async, unsafe, extern, generic total functions, trait dispatch, closures, recursion,
  arbitrary Rust calls, arbitrary method calls, iterators, mutation-heavy collection models,
  or user-declared trusted axioms
- diagnostics are deterministic and tested, but source-span quality is still MVP-level
- semantic extraction is metadata/token based rather than a full rustc HIR/MIR verifier

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
