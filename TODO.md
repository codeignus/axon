# TODO

- Invalid imports must fail with a real compiler error instead of being tolerated or misclassified.
- Rust interop functions should be callable from Axon only through the Axon registry/inventory path, not by ad hoc direct exposure.
- If a Rust function should be callable outside its defining module, it should use an explicit public export marker such as `axon_pub_export` rather than relying on incidental visibility.
- `match` statement: parsed and typechecked but **not lowered** to MIR (`crates/axon-mir/src/lower.rs` falls through to "unsupported statement type"). End-to-end compilation will fail. Implement `Stmt::Match` lowering (branch on discriminant, arm bodies) then switch CLI dispatch from `if/elif` to `match`.
