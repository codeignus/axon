# TODO

- Invalid imports must fail with a real compiler error instead of being tolerated or misclassified.
- Rust interop functions should be callable from Axon only through the Axon registry/inventory path, not by ad hoc direct exposure.
- If a Rust function should be callable outside its defining module, it should use an explicit public export marker such as `axon_pub_export` rather than relying on incidental visibility.
- `match` statement: parsed and typechecked but **not lowered** to MIR (`crates/axon-mir/src/lower.rs` falls through to "unsupported statement type"). End-to-end compilation will fail. Implement `Stmt::Match` lowering (branch on discriminant, arm bodies) then switch CLI dispatch from `if/elif` to `match`.

## Deferred: Check Parity to 100 Percent

- Make project parity test strict (not smoke): replace `assert_eq(got, got)` with explicit expected outcomes using controlled fixtures.
- Add 3 project fixtures for parity: unresolved symbol, arity mismatch, valid project.
- Improve function identity model: move from global `name -> arity` to module-aware keys (for example `module::name`) to avoid false conflicts/collisions.
- Respect imports/aliases during call resolution rather than only free-name matching.
- Strengthen type semantics beyond return literals: validate call argument kinds against declared parameter types (`Int`/`String`/`Bool` first).
- Add deterministic diagnostics for argument type mismatches.
- Deepen ownership flow checks: add branch/merge escape checks for `mut` assignments across `if/elif` paths.
- Add tests that assert merge-point ownership outcomes.
- Harden diagnostic parity: stabilize message schema to include stage + code + concise reason consistently.
- Add fixture expectations per stage so regressions are caught by tests, not manual runs.
- `check` still lacks strongest-form parity in three areas:
  - module-aware symbol identity/import alias resolution is still simplified.
  - type semantics are not yet fully deep (argument type-check depth is partial).
  - ownership dataflow across complex branch/merge cases is still guardrail-level vs full analysis.
