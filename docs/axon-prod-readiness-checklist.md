# Axon Prod Readiness Checklist

## Phase 1 - Semantics and Ownership

- [ ] Make project parity strict in `src/compiler/semantics/project_parity.test.ax`.
- [ ] Add deterministic fixture assertions for unresolved symbol and arity mismatch.
- [ ] Add deterministic fixture assertion for argument type mismatch.
- [ ] Add ownership merge-point regression tests for `mut` branch/merge behavior.

Verify:
- `cd rust-backed-compiler-for-axon && cargo run -p axon -- build`
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 2 - Resolver Identity

- [ ] Remove free-name fallback in semantics project checks.
- [ ] Enforce module-aware keys and alias/import conflict diagnostics only.
- [ ] Add fixtures covering alias collisions and ambiguous module symbol resolution.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 3 - Native Syntax

- [ ] Move tokenization truth into `src/compiler/syntax/lexer.ax`.
- [ ] Move parse-balance and parse contracts into `src/compiler/syntax/parser.ax`.
- [ ] Keep sidecar syntax wrappers as boundary adapters only.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 4 - MIR and Backend Native

- [ ] Complete method lowering for complex types and remove builtin `len` dependency.
- [ ] Remove script artifact generation from backend path.
- [ ] Keep sidecar strictly for external toolchain invocation boundaries.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon build`
- `/Projects/opensource-projects/axon/target/build/axon/axon run`

## Phase 5 - Diagnostics and CLI

- [ ] Enforce diagnostics schema `stage + code + reason` across all error paths.
- [ ] Expand fixture coverage for `check/build/run/test/fmt/mcp`.
- [ ] Ensure command behavior is stable from repository root.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`
- `/Projects/opensource-projects/axon/target/build/axon/axon build`
- `/Projects/opensource-projects/axon/target/build/axon/axon run`
- `/Projects/opensource-projects/axon/target/build/axon/axon test .`
- `/Projects/opensource-projects/axon/target/build/axon/axon fmt .`
- `/Projects/opensource-projects/axon/target/build/axon/axon mcp`
