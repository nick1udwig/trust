# Trust v5: Rust-Native Verified Islands

**Document status:** Draft v5  
**Project name:** Trust  
**Primary artifact:** Rust functions with Trust contracts, verified during normal Cargo builds  
**Audience:** Rust verification engineers, macro implementers, compiler-tooling engineers, language designers, product/design collaborators

---

## 1. Executive summary

Trust is a Rust verification system for selected functions inside ordinary Rust crates.

Trust users write normal Rust function bodies, surrounded by Trust contract syntax:

```rust
#[trust::module]
mod verified {
    trust::total! {
        given executable {
            i < xs.len();
        }

        gives ghost |out| {
            out == xs[i];
        }

        pub fn get(xs: &[i32], i: usize) -> i32 {
            xs[i]
        }
    }
}
```

The executable code is Rust. The contracts are Trust syntax.

Trust verification runs as part of normal Cargo builds:

```bash
cargo build
cargo check
cargo test
```

The user does not run a separate proof command for normal use.

Trust uses two layers:

1. **Procedural macro UX layer**

   Trust macros provide concise contract syntax, generate Rust functions and metadata, insert runtime boundary assertions, and mark verified items.

2. **Compiler-wrapper verification layer**

   A `trust-rustc` wrapper drives or wraps `rustc`, reads the Trust metadata, inspects compiler semantic information for the generated Rust functions, generates verification conditions, calls an SMT solver, and fails the build if proof fails.

The goal is not to verify arbitrary Rust. The goal is to verify deliberately selected Rust functions using ordinary Rust syntax where feasible, explicit contracts, explicit models, and a conservative supported subset.

---

## 2. Product promise

Trust should provide:

```text
Rust-like syntax:      yes, executable bodies are Rust functions
Not a new language:    executable code is Rust; contracts/specs are Trust annotations
No separate command:   normal cargo build/check/test performs verification
Incremental adoption:  verify one module or function at a time
Safe boundary default: public Trust functions assert executable preconditions
```

Trust should not promise:

```text
arbitrary Rust verification
arbitrary stdlib verification
arbitrary trait verification
unsafe Rust verification
full async/concurrency verification
cryptographic security proof
```

---

## 3. Required build integration

Trust verification requires the project to be built through the Trust compiler wrapper.

Recommended project configuration:

```toml
# .cargo/config.toml
[build]
rustc-workspace-wrapper = "trust-rustc"
```

Package-level Trust configuration:

```toml
# Cargo.toml
[package.metadata.trust]
assertions = "always"
solver = "z3"
timeout_ms = 5000
cache = "local"
```

Normal developer commands remain:

```bash
cargo build
cargo check
cargo test
```

If Trust macros are used but the Trust compiler wrapper is not active, the default behavior is to fail closed with a compile error.

Example:

```text
error[trust]: Trust verification requires trust-rustc
   = help: configure .cargo/config.toml with build.rustc-workspace-wrapper = "trust-rustc"
```

An explicitly configured runtime-only mode may be added later, but it is not part of the MVP safety story.

---

## 4. Source structure

### 4.1 Verified module

Trust code should live inside a verified Rust module:

```rust
#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        given executable {
            amount >= 0;
            acct.balance >= amount;
        }

        gives ghost |out| {
            out.id == old(acct.id);
            int(out.balance) == int(old(acct.balance)) - int(old(amount));
        }

        pub fn withdraw(acct: Account, amount: i64) -> Account {
            Account {
                id: acct.id,
                balance: acct.balance - amount,
            }
        }
    }
}
```

A `#[trust::module]` may contain:

- `use` declarations;
- Rust type declarations marked with `#[derive(TrustModel)]`;
- `trust::total!` blocks;
- `trust::spec!` blocks;
- `trust::proof!` blocks;
- inert `trust::trusted_model!` stubs;
- private helper constants accepted by Trust.

A `#[trust::module]` should not contain arbitrary unverified Rust functions in the MVP. This keeps private Trust helper functions from being called by ordinary Rust in the same module.

### 4.2 Total function block

Primary syntax:

```rust
trust::total! {
    given executable {
        // runtime-checkable caller obligations
    }

    given ghost {
        // proof-only caller obligations
    }

    gives executable |out| {
        // runtime-checkable guarantees
    }

    gives ghost |out| {
        // proof-only guarantees
    }

    pub fn f(args...) -> ReturnType {
        // ordinary Rust body in the supported Trust subset
    }
}
```

The `pub fn ... { ... }` item is ordinary Rust function syntax.

The Trust macro parses the contract blocks, preserves or rewrites the function item into generated Rust, and emits Trust metadata for the compiler wrapper.

### 4.3 Minimal total function

```rust
trust::total! {
    pub fn is_zero(x: i32) -> bool {
        x == 0
    }
}
```

Even without explicit contracts, Trust must prove:

- termination;
- no unsupported operations;
- no unchecked panic paths in the modeled subset;
- no arithmetic overflow;
- all called Trust preconditions, if any.

### 4.4 Public boundary function

```rust
trust::total! {
    given executable {
        i < xs.len();
    }

    gives ghost |out| {
        out == xs[i];
    }

    pub fn get(xs: &[i32], i: usize) -> i32 {
        xs[i]
    }
}
```

Generated shape:

```rust
pub fn get(xs: &[i32], i: usize) -> i32 {
    assert!(
        i < xs.len(),
        "Trust precondition failed in get: i < xs.len()"
    );

    __trust_get_body_7f3a(xs, i)
}

fn __trust_get_body_7f3a(xs: &[i32], i: usize) -> i32 {
    xs[i]
}
```

The exact generated symbol names are implementation details.

---

## 5. Contract classes

Trust has two contract classes: executable and ghost.

### 5.1 Executable contracts

Executable contracts can be lowered to runtime Rust checks.

Example:

```rust
given executable {
    i < xs.len();
}
```

Allowed MVP expression features:

- boolean operators;
- integer comparisons;
- simple arithmetic when overflow-safe or lowered through checked operations;
- `len` on supported slices;
- field reads on `TrustModel` types;
- calls to executable Trust specs;
- simple `Option`/`Result` pattern checks where modeled.

Executable contracts may be asserted at runtime.

### 5.2 Ghost contracts

Ghost contracts exist only for verification.

Example:

```rust
gives ghost |out| {
    sorted(model(out));
}
```

Ghost contracts may use:

- mathematical integers;
- quantifiers;
- abstract sequences;
- abstract sets/maps/multisets if modeled;
- ghost spec functions;
- proof lemmas;
- `old(...)`.

Ghost contracts are not emitted as runtime assertions.

### 5.3 Public precondition rule

A Rust-callable public Trust function must not require ghost-only preconditions.

Rejected:

```rust
trust::total! {
    given ghost {
        sorted(model(xs));
    }

    pub fn insert_sorted(xs: &[i32], x: i32) -> Vec<i32> {
        // ...
    }
}
```

Reason: ordinary Rust callers cannot be runtime-checked against ghost-only assumptions.

Allowed alternatives:

1. Keep the function private inside a `#[trust::module]`.
2. Provide an executable validator.
3. Prove a link between the executable validator and the ghost predicate.

Example:

```rust
trust::spec! {
    executable fn sorted_exec(xs: &[i32]) -> bool {
        // executable checked implementation, if supported
    }
}

trust::spec! {
    ghost fn sorted(xs: Seq<i32>) -> bool {
        forall(|i: usize, j: usize|
            implies(i < j && j < xs.len(), xs[i] <= xs[j])
        )
    }
}

trust::proof! {
    fn sorted_exec_implies_sorted(xs: &[i32])
    given executable {
        sorted_exec(xs);
    }
    gives ghost {
        sorted(model(xs));
    }
    {
        // proof body
    }
}
```

---

## 6. Assertion policy

Configuration:

```toml
[package.metadata.trust]
assertions = "always"
```

Allowed values:

```toml
assertions = "always" # default: use assert!
assertions = "debug"  # use debug_assert!
assertions = "assume" # no runtime checks; dangerous
```

Meaning:

- `always`: normal Rust callers are checked at runtime;
- `debug`: checked in debug builds, assumed in release builds;
- `assume`: never checked at runtime.

`assume` must produce a warning unless explicitly silenced.

Postcondition assertions are optional diagnostics for executable postconditions. They are not the proof mechanism.

---

## 7. `old(...)` and result binders

Postconditions may refer to entry-state values using `old(...)`.

```rust
gives ghost |out| {
    out.balance == old(acct.balance) - old(amount);
}
```

Rules:

- `old(expr)` is valid in `gives` clauses and proof contexts that reason about function entry state.
- `old(expr)` is ghost by default.
- Executable postconditions using `old(expr)` require runtime snapshots.
- MVP snapshots are allowed only for simple `Copy` values and simple field reads.

Example:

```rust
trust::total! {
    given executable {
        x < i32::MAX;
    }

    gives executable |out| {
        out > old(x);
    }

    pub fn add_one(x: i32) -> i32 {
        x + 1
    }
}
```

Trust may lower this by snapshotting `x` before the body.

---

## 8. Specification functions

### 8.1 Ghost specs

Default spec functions are ghost-only.

```rust
trust::spec! {
    ghost fn sorted(xs: Seq<i32>) -> bool {
        forall(|i: usize, j: usize|
            implies(i < j && j < xs.len(), xs[i] <= xs[j])
        )
    }
}
```

Ghost specs may use quantifiers and abstract models.

### 8.2 Executable specs

Executable specs can be lowered to Rust and used in runtime assertions.

```rust
trust::spec! {
    executable fn nonempty(xs: &[i32]) -> bool {
        xs.len() > 0
    }
}
```

Executable specs must use only executable Trust expressions.

### 8.3 Implication

MVP implication syntax:

```rust
implies(a, b)
```

Symbolic implication syntax such as `a ==> b` is not required for MVP.

### 8.4 Quantifiers

Ghost-only:

```rust
forall(|i: usize| condition)
forall(|i: usize, j: usize| condition)
exists(|i: usize| condition)
```

Quantifiers may not appear in executable contracts.

---

## 9. Proof functions

Proof functions are checked at build time and erased from generated runtime code.

```rust
trust::proof! {
    fn add_one_gt(x: i32)
    given ghost {
        x < i32::MAX;
    }
    gives ghost {
        int(x) + 1 > int(x);
    }
    {
        assert(int(x) + 1 > int(x));
    }
}
```

MVP proof language includes:

- `assert(...)` proof steps;
- calls to other proof lemmas;
- local ghost bindings;
- simple case splits;
- calculation chains, if implemented.

An empty proof body does not mean trusted. Every `gives` clause must be proved unless the item is an explicit trusted stub, and MVP trusted stubs do not affect proof.

---

## 10. Loop annotations

Trust uses normal Rust loops plus marker macros for loop obligations.

Example:

```rust
trust::total! {
    given executable {
        xs.len() <= 1024;
    }

    pub fn sum_nonnegative(xs: &[i32]) -> i32 {
        let mut i: usize = 0;
        let mut acc: i32 = 0;

        while i < xs.len() {
            trust::invariant!(i <= xs.len());
            trust::invariant!(acc >= 0);
            trust::decreases!(xs.len() - i);

            acc = acc + xs[i];
            i = i + 1;
        }

        acc
    }
}
```

MVP loop-marker rules:

- `trust::invariant!` and `trust::decreases!` must appear before executable statements in the loop body.
- They apply to the nearest enclosing loop.
- They are proof markers, not runtime code.
- They may reference loop-local variables in scope.
- Trust extracts them through macro metadata and compiler spans.

For each loop, Trust proves:

1. invariants hold before loop entry;
2. invariants are preserved by each iteration;
3. the decreases measure is nonnegative;
4. the decreases measure strictly decreases on continuing iterations;
5. after loop exit, Trust may use `invariants && !condition`.

MVP loop restrictions:

- `while` loops only;
- explicit `decreases` required unless Trust proves a simple bounded counter pattern;
- no `break`;
- no `continue`;
- no mutation through aliases;
- no iterator desugaring;
- no async or generator loops.

---

## 11. Arithmetic semantics

### 11.1 Executable code

In verified Rust bodies, ordinary arithmetic uses Rust bounded integer semantics.

Trust must prove overflow cannot occur for:

```rust
x + y
x - y
x * y
-x
```

Example:

```rust
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
```

Without the precondition, rejected.

### 11.2 Explicit arithmetic APIs

Trust may model selected explicit APIs:

```rust
x.checked_add(y)
x.wrapping_add(y)
x.saturating_add(y)
```

MVP should support `checked_add` first.

### 11.3 Ghost arithmetic

Ghost specs use mathematical integers through explicit conversion:

```rust
int(out) == int(x) + 1
```

MVP should require explicit `int(...)` in ghost arithmetic.

### 11.4 `usize`

`usize` is modeled according to the target pointer width.

Target pointer width is part of:

- verification context;
- proof cache key;
- diagnostics;
- target model.

Trust must not use host pointer width when verifying a different target.

---

## 12. Type modeling and `TrustModel`

### 12.1 Required derive

Any user-defined Rust type that Trust reasons about structurally must derive `TrustModel`.

```rust
#[derive(TrustModel)]
pub struct Account {
    pub id: u64,
    pub balance: i64,
}
```

Then Trust may reason about:

```rust
acct.id
acct.balance
Account { id, balance }
```

Without `TrustModel`, the type is opaque.

### 12.2 Opaque types

Opaque external types may be passed through verified functions only under strict rules:

- no field access;
- no construction by fields;
- no equality reasoning unless modeled;
- no cloning/copying assumptions unless modeled;
- no invariant assumptions.

Example rejected:

```rust
pub struct Account {
    pub id: u64,
    pub balance: i64,
}

trust::total! {
    pub fn balance(acct: Account) -> i64 {
        acct.balance // rejected: Account lacks TrustModel
    }
}
```

### 12.3 Supported `TrustModel` shapes in MVP

MVP derives may support:

- simple structs with named fields;
- tuple structs, if simple;
- C-like enums;
- simple algebraic enums with modeled fields;
- non-generic types first;
- generics later.

MVP derives should reject:

- raw pointer fields;
- `UnsafeCell` fields;
- trait object fields;
- function pointer fields unless opaque;
- interior mutability unless explicitly modeled;
- complex generic constraints;
- private fields that cannot be modeled soundly.

### 12.4 What `TrustModel` generates

`#[derive(TrustModel)]` may generate:

- hidden type metadata;
- field model metadata;
- logical equality model, if allowed;
- constructor/destructor model metadata;
- layout-independent structural model;
- fingerprints for proof cache invalidation.

It should not change runtime representation.

---

## 13. Trusted models

A trusted model is a verifier-side meaning for Rust constructs that Trust does not prove from Rust source.

Examples of built-in trusted models:

- `bool`;
- fixed-width integer bounds;
- `Option<T>`;
- simple `Result<T, E>`;
- read-only slices;
- selected checked arithmetic APIs;
- field access for `TrustModel` structs;
- equality for supported modeled types.

Trusted models are part of Trust's trusted computing base. If a built-in model is wrong, Trust can prove incorrect results.

### 13.1 Built-in trusted models

MVP may include built-in models for:

```text
bool
i32, i64, u32, u64, usize
Option<T>
Result<T, E> in simple cases
&[T] read-only slices
TrustModel structs/enums
selected checked arithmetic
```

### 13.2 Derived models

`#[derive(TrustModel)]` creates derived model metadata for user types.

The derive is trusted only in the sense that Trust trusts its own generated metadata and model interpretation.

### 13.3 User-declared trusted model stubs

MVP may accept user-declared trusted model syntax as a stub:

```rust
trust::trusted_model! {
    fn external_hash(bytes: &[u8]) -> [u8; 32];
}
```

MVP behavior:

- parse or store the declaration;
- emit metadata, if useful;
- optionally print a warning or note;
- do not add axioms;
- do not affect verification;
- do not let proofs assume the declaration;
- do not make unsupported functions callable from Trust.

Example diagnostic:

```text
warning[trust]: user-declared trusted models are stubs in the MVP and are ignored by verification
```

This reserves syntax without creating an unsound escape hatch.

---

## 14. Supported Rust subset for total functions

Trust verifies Rust bodies after macro expansion, using rustc semantic information and Trust's own supported subset checks.

MVP supported:

- `bool`;
- `i32`, `i64`, `u32`, `u64`, `usize`;
- simple `let` bindings;
- `mut` locals with simple alias restrictions;
- `if`;
- `match` over supported enums;
- field access on `TrustModel` structs;
- construction of `TrustModel` structs;
- `Option<T>`;
- simple `Result<T, E>`;
- read-only slices `&[T]`;
- indexing when bounds are proved;
- simple while loops with markers;
- calls to Trust total functions;
- calls to executable Trust specs;
- selected modeled standard-library functions.

MVP excluded:

- arbitrary traits;
- arbitrary methods;
- arbitrary stdlib calls;
- async/await;
- threads;
- atomics;
- unsafe;
- raw pointers;
- FFI;
- floating point;
- closures except quantifier binders and approved proof syntax;
- iterators;
- allocation-sensitive reasoning;
- arbitrary `Vec<T>` mutation;
- interior mutability;
- panics;
- macros inside verified bodies except approved Trust markers.

Unsupported features must be rejected, not approximated unsoundly.

---

## 15. Collections

### 15.1 Read-only slices

MVP supports:

```rust
xs.len()
xs[i]
```

Indexing requires proof:

```rust
i < xs.len()
```

### 15.2 Ghost sequences

Ghost specs may use abstract sequences:

```rust
Seq<T>
```

A model bridge may relate a runtime slice to a ghost sequence:

```rust
model(xs)
```

The slice-to-sequence bridge is a built-in trusted model.

### 15.3 `Vec<T>`

Useful `Vec<T>` verification is not MVP.

MVP may allow `Vec<T>` only as:

- opaque;
- passed through without structural reasoning;
- converted to a slice where Rust permits and Trust supports the slice view.

Post-MVP `Vec<T>` support requires a model for:

- length;
- indexing;
- push/pop;
- mutation;
- allocation behavior policy;
- relation to ghost `Seq<T>`;
- panic behavior;
- capacity irrelevance or explicit capacity model.

---

## 16. Calls

### 16.1 Calls from ordinary Rust

Ordinary Rust may call public Trust functions.

Executable preconditions are checked according to assertion policy.

Ghost preconditions are not allowed on public Rust-callable Trust functions in MVP.

### 16.2 Calls from Trust functions

When a verified Trust function calls another Trust function, Trust must prove the callee's preconditions.

Example:

```rust
trust::total! {
    given executable {
        xs.len() > 0;
    }

    pub fn first(xs: &[i32]) -> i32 {
        get(xs, 0)
    }
}
```

If `get` requires `i < xs.len()`, Trust proves:

```text
0 < xs.len()
```

from:

```text
xs.len() > 0
```

### 16.3 Calls to ordinary Rust functions

MVP total functions may not call arbitrary ordinary Rust functions.

Allowed calls:

- other Trust total functions;
- executable Trust specs;
- built-in modeled operations;
- selected modeled standard-library functions.

All other calls are rejected unless they are covered by a supported model. User-declared trusted model stubs do not provide such support in MVP.

---

## 17. Termination

Every `trust::total!` function must terminate.

MVP termination mechanisms:

- acyclic call graph for simple functions;
- explicit loop `decreases!` markers;
- no recursion unless explicitly supported.

Recursive functions are post-MVP unless Trust implements:

```rust
decreases(expr)
```

for recursive calls and proves strict decrease.

---

## 18. Panic and partial-operation policy

Trust must prove absence of unchecked partial behavior in the modeled subset.

Rejected or proved safe:

- out-of-bounds indexing;
- integer overflow;
- division by zero;
- remainder by zero;
- unchecked `unwrap`;
- explicit `panic!`;
- non-exhaustive modeled matches;
- unsupported standard-library calls;
- unsupported target-specific operations.

Trust's MVP claim is:

```text
No unchecked partial operations in the supported modeled subset.
```

Not:

```text
No possible panic under all runtime conditions on all targets.
```

Allocation failure, stack overflow, and unsupported platform behavior are outside the MVP proof claim.

---

## 19. Verification architecture

### 19.1 Crates and tools

Recommended split:

```text
trust
  user-facing runtime support
  re-exports macros
  assertion support

trust-macros
  proc macro crate
  #[trust::module]
  trust::total!
  trust::spec!
  trust::proof!
  trust::invariant!
  trust::decreases!
  #[derive(TrustModel)]

trust-rustc
  rustc wrapper / compiler driver
  invoked by Cargo
  extracts Trust metadata
  inspects rustc semantic information
  generates VCs
  calls solver
  emits diagnostics

trust-core
  contract parser
  spec parser
  model checker
  VC generator
  proof cache
  solver interface
  diagnostics

trust-model
  built-in trusted models
  model fingerprints
```

### 19.2 Macro layer responsibilities

Trust macros:

- parse contract syntax;
- preserve Rust function bodies as Rust syntax;
- emit generated Rust wrappers and private bodies;
- insert executable precondition assertions;
- emit metadata for contracts/specs/proofs;
- emit metadata for `TrustModel` types;
- emit marker metadata for loop invariants/decreases;
- fail closed if required verification integration is absent.

Macros should not be the main proof engine.

### 19.3 Compiler-wrapper responsibilities

`trust-rustc`:

- receives Cargo's rustc invocation;
- configures or drives rustc;
- identifies Trust-marked modules/functions/types;
- obtains resolved types, paths, and lowered control-flow information;
- checks that each verified body stays within the supported subset;
- maps contracts to typed program entities;
- generates verification conditions;
- queries the solver;
- stores proof results in the proof cache;
- forwards successful builds to normal compilation;
- fails the build on unproved obligations.

### 19.4 Why rustc semantic information matters

Trust should use compiler semantic information for:

- resolved names;
- function item identity;
- type information;
- method resolution where supported;
- match exhaustiveness facts;
- borrow-checker acceptedness;
- MIR/HIR-level control flow where appropriate;
- target cfg selection.

Trust still has its own supported subset and model rules. Rust acceptance alone does not imply Trust acceptance.

---

## 20. Build reproducibility

A Trust proof is valid for a specific build context.

The build context includes:

- source code;
- Trust contracts;
- derived model metadata;
- built-in model version;
- dependency Trust metadata;
- Cargo features;
- target triple;
- target pointer width;
- target endian where relevant;
- target features where relevant;
- rustc version;
- Trust wrapper version;
- Trust core/model version;
- solver name and version;
- solver options;
- assertion policy if it changes generated code or obligations.

Changing any proof-relevant part may require re-verification.

### 20.1 Host and target

The Trust wrapper runs on the build host.

The verified program targets the compilation target.

Example:

```text
host:   x86_64-unknown-linux-gnu
target: aarch64-unknown-linux-gnu
```

The wrapper binary must run on the host. The verification model must match the target.

Trust does not require one wrapper binary per target. It requires one wrapper binary per host platform/toolchain, parameterized by target information.

### 20.2 Target triple

The target triple is part of the proof context.

Examples:

```bash
cargo build --target x86_64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu
cargo build --target i686-unknown-linux-gnu
```

These may produce separate proof cache entries.

For many simple functions, the proof result may be identical across targets. Trust should still treat the target model explicitly.

### 20.3 Cargo features

Different feature sets may compile different code:

```bash
cargo build
cargo build --features fast
cargo build --all-features
```

Trust cache keys must distinguish feature sets.

### 20.4 Dependency metadata

If a Trust function depends on a verified function from another crate, the caller's proof depends on the callee's contract and model fingerprint.

Changing a dependency contract invalidates dependent proofs.

### 20.5 Incremental compilation

Rust incremental compilation is not a proof cache.

Trust must maintain its own proof cache keyed by proof-relevant content.

---

## 21. Proof cache

### 21.1 Location

Default local cache:

```text
target/trust-cache/
```

or another location selected by Trust configuration.

When run through Cargo wrappers, Trust must handle:

- workspaces;
- custom target directories;
- cross-compilation target directories;
- concurrent builds;
- rust-analyzer or check-only contexts.

### 21.2 Cache key

Include:

- normalized contracts;
- generated Rust body fingerprint;
- rustc semantic fingerprint where needed;
- function signature;
- type model fingerprints;
- spec fingerprints;
- proof lemma fingerprints;
- called Trust function fingerprints;
- trusted built-in model fingerprints;
- target triple and target model facts;
- enabled features;
- rustc version;
- Trust version;
- solver name/version/options;
- VC-generation settings.

### 21.3 Cache value

Store:

- verification status;
- VC fingerprints;
- solver responses;
- diagnostic summaries;
- optional SMT-LIB dumps;
- dependency fingerprints.

### 21.4 Status handling

Only this status permits compilation:

```text
proved
```

These statuses fail the Trust build:

```text
counterexample
unknown
timeout
unsupported
solver_error
internal_error
```

`timeout` and `unknown` mean not proved, not necessarily false.

### 21.5 Concurrency

Cache writes must use:

- atomic writes;
- file locks or equivalent coordination;
- content-addressed entries;
- corruption detection.

---

## 22. Solver backend

MVP requires an SMT solver.

Supported strategies:

- bundled pinned solver;
- linked solver library;
- external pinned solver binary;
- SMT-LIB backend with explicit solver discovery.

Verification must fail closed if the configured solver is unavailable.

Diagnostics should include:

- solver name;
- solver version;
- timeout;
- relevant options;
- Trust version;
- target model.

---

## 23. Diagnostics

### 23.1 Overflow

```text
error[trust]: could not prove integer addition cannot overflow
  --> src/lib.rs:18:9
   |
18 |         x + 1
   |         ^^^^^
   |
   = type: i32
   = needed: x < i32::MAX
   = help: add `given executable { x < i32::MAX; }`, use `checked_add`, or use explicit wrapping arithmetic
```

### 23.2 Bounds

```text
error[trust]: could not prove index is in bounds
  --> src/lib.rs:22:9
   |
22 |         xs[i]
   |         ^^^^^
   |
   = needed: i < xs.len()
   = known: i <= xs.len()
```

### 23.3 Public ghost precondition

```text
error[trust]: public Trust function has a ghost-only precondition
  --> src/lib.rs:40:9
   |
40 |         pub fn insert_sorted(...)
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = ghost precondition: sorted(model(xs))
   = reason: ordinary Rust callers cannot runtime-check this condition
   = help: make the function private in a Trust module or provide an executable validator
```

### 23.4 Missing `TrustModel`

```text
error[trust]: cannot reason about fields of type `Account`
  --> src/lib.rs:51:13
   |
51 |             acct.balance
   |             ^^^^^^^^^^^^
   |
   = reason: `Account` does not derive `TrustModel`
   = help: add `#[derive(TrustModel)]` to `Account` if it satisfies Trust's model restrictions
```

### 23.5 Ignored trusted model stub

```text
warning[trust]: user-declared trusted model is ignored in MVP
  --> src/lib.rs:70:5
   |
70 |     trust::trusted_model! { ... }
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: this declaration does not add axioms or affect verification
```

### 23.6 Solver unknown

```text
error[trust]: solver returned unknown
  --> src/lib.rs:88:5
   |
88 |     trust::total! { ... }
   |     ^^^^^^^^^^^^^^^^^^^^^
   |
   = obligation: postcondition
   = help: add intermediate assertions, strengthen invariants, or simplify quantified specs
```

---

## 24. Examples

### 24.1 Safe indexing

```rust
#[trust::module]
mod verified {
    trust::total! {
        given executable {
            i < xs.len();
        }

        gives ghost |out| {
            out == xs[i];
        }

        pub fn get(xs: &[i32], i: usize) -> i32 {
            xs[i]
        }
    }
}
```

### 24.2 Safe arithmetic

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

### 24.3 Account transition

```rust
use trust::TrustModel;

#[derive(TrustModel)]
pub struct Account {
    pub id: u64,
    pub balance: i64,
}

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        given executable {
            amount >= 0;
            acct.balance >= amount;
        }

        gives ghost |out| {
            out.id == old(acct.id);
            int(out.balance) == int(old(acct.balance)) - int(old(amount));
        }

        pub fn withdraw(acct: Account, amount: i64) -> Account {
            Account {
                id: acct.id,
                balance: acct.balance - amount,
            }
        }
    }
}
```

### 24.4 Private helper with ghost precondition

```rust
#[trust::module]
mod verified {
    trust::spec! {
        ghost fn sorted(xs: Seq<i32>) -> bool {
            forall(|i: usize, j: usize|
                implies(i < j && j < xs.len(), xs[i] <= xs[j])
            )
        }
    }

    trust::total! {
        given ghost {
            sorted(model(xs));
        }

        fn assume_sorted_helper(xs: &[i32]) -> usize {
            xs.len()
        }
    }
}
```

This is private to the verified module. It is not a public Rust boundary.

---

## 25. MVP scope

### 25.1 MVP includes

- `#[trust::module]`;
- `trust::total!` with Rust `fn` body;
- `trust::spec!`;
- `trust::proof!`;
- `trust::invariant!`;
- `trust::decreases!`;
- `#[derive(TrustModel)]` for simple structs/enums;
- executable and ghost contracts;
- public executable precondition assertions;
- built-in trusted models for primitives and read-only slices;
- user-declared trusted model stubs that do not affect verification;
- rustc wrapper integration;
- proof cache;
- SMT solver backend;
- diagnostics with source spans.

### 25.2 MVP excludes

- arbitrary Rust verification;
- public functions with ghost preconditions;
- arbitrary trait reasoning;
- arbitrary method calls;
- arbitrary standard-library modeling;
- user-defined axioms;
- effective user trusted models;
- `Vec<T>` mutation reasoning;
- async;
- concurrency;
- unsafe;
- raw pointers;
- FFI;
- floating point;
- cross-crate proof dependencies unless explicitly implemented.

---

## 26. Extension path

Likely extension order:

1. more integer types;
2. richer `TrustModel` derives;
3. simple recursion with `decreases`;
4. better proof language;
5. executable validators for common ghost predicates;
6. ghost `Seq`, `Set`, `Map`, `Multiset` libraries;
7. restricted `Vec<T>` model;
8. cross-crate Trust metadata;
9. limited generics;
10. selected trait models;
11. selected iterator models;
12. richer standard-library models.

High-risk extensions:

- arbitrary traits;
- arbitrary async;
- concurrency;
- atomics;
- unsafe code;
- layout-sensitive proofs;
- FFI;
- full standard-library coverage.

These require deeper models and may require tighter compiler integration.

---

## 27. Soundness and trusted computing base

Trust soundness depends on:

- Trust macros preserving intended Rust code;
- Trust metadata correctness;
- `TrustModel` derive correctness;
- compiler-wrapper extraction correctness;
- rustc semantic data interpretation;
- supported-subset checker correctness;
- VC generator correctness;
- solver result handling;
- built-in trusted model correctness;
- code generation correctness;
- runtime boundary assertion generation;
- proof cache validity and invalidation;
- target model correctness.

User-declared trusted model stubs do not affect proof in MVP and therefore are not part of the proof TCB.

Unsupported constructs must be rejected.

---

## 28. Open questions

### 28.1 Exact rustc integration strategy

Options:

- wrapper that invokes rustc and consumes side metadata;
- custom rustc driver;
- rustc plugin-like integration through nightly internals;
- separate verifier pass coordinated by the wrapper.

The MVP should choose the least fragile path that still exposes enough type and control-flow information.

### 28.2 Toolchain stability

The compiler-wrapper approach may require pinned rustc versions or version-specific Trust releases.

Open policy:

```text
stable-only support vs pinned nightly vs managed Trust toolchain
```

### 28.3 Metadata transport

Trust metadata may be transported through:

- generated hidden Rust items;
- sidecar files;
- rustc attributes;
- linker-section-like metadata;
- wrapper-managed temporary files.

The format must support spans, dependency fingerprints, and incremental rebuilds.

### 28.4 Contract name resolution

Contracts are parsed by Trust but refer to Rust names.

The wrapper must map contract identifiers to rustc-resolved function parameters, fields, functions, and types.

This is a central implementation risk.

### 28.5 Private ghost-precondition functions

Private functions with ghost preconditions are safe only if ordinary Rust cannot call them unchecked.

The MVP module restrictions should be validated carefully against Rust privacy and macro expansion behavior.

### 28.6 Loop marker robustness

Loop marker macros are convenient but subtle.

Open details:

- exact span association with enclosing loop;
- handling nested loops;
- generated no-op code shape;
- diagnostics when markers are misplaced;
- preserving borrow-checker behavior.

### 28.7 Executable spec checking

Executable specs should be checked both as Rust code and as Trust-supported expressions.

Open question: whether executable specs are generated as real Rust functions, Trust-evaluated expressions, or both.

### 28.8 `Vec<T>` model

Useful `Vec<T>` support is important but not MVP.

Open model choices:

- capacity ignored vs modeled;
- allocation failure assumptions;
- push/pop semantics;
- alias restrictions;
- relation to ghost sequences.

### 28.9 Cross-crate Trust metadata

Cross-crate verification needs exported metadata for:

- contracts;
- specs;
- proof lemmas;
- model fingerprints;
- Trust version;
- solver/model compatibility.

This is post-MVP unless required early.

### 28.10 IDE support

Macro-based contract syntax may have weaker IDE support than pure attributes.

Possible mitigations:

- rust-analyzer integration;
- formatter support;
- generated virtual files;
- lightweight parser diagnostics in proc macros.

---

## 29. References

- Rust procedural macros: https://doc.rust-lang.org/reference/procedural-macros.html
- Cargo configuration: https://doc.rust-lang.org/cargo/reference/config.html
- Cargo environment variables: https://doc.rust-lang.org/cargo/reference/environment-variables.html
- rustc driver and rustc interface: https://rustc-dev-guide.rust-lang.org/rustc-driver/intro.html
- rustc MIR overview: https://rustc-dev-guide.rust-lang.org/mir/index.html
