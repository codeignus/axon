# Axon Prod Readiness Checklist

## Phase 1 - Semantics and Ownership

- [x] Make project parity strict in `src/compiler/semantics/project_parity.test.ax`.
- [x] Add deterministic fixture assertions for unresolved symbol and arity mismatch.
- [x] Add deterministic fixture assertion for argument type mismatch.
- [x] Document slot + last-survivor ownership for conditionals; forbid manual dealloc; enforcement in MIR/codegen (not line-scan heuristics).

Verify:
- `cd rust-backed-compiler-for-axon && cargo run -p axon -- build`
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 2 - Resolver Identity

- [x] Remove free-name fallback in semantics project checks.
- [x] Enforce module-aware keys and alias/import conflict diagnostics only.
- [x] Add fixtures covering alias collisions and ambiguous module symbol resolution.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 3 - Native Syntax

- [x] Move tokenization truth into `src/compiler/syntax/lexer.ax`.
- [x] Move parse-balance and parse contracts into `src/compiler/syntax/parser.ax`.
- [x] Keep sidecar syntax wrappers as boundary adapters only.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`

## Phase 4 - MIR and Backend Native

- [x] Complete method lowering for complex types and remove builtin `len` dependency.
- [x] Remove script artifact generation from backend path.
- [x] Keep sidecar strictly for external toolchain invocation boundaries.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon build`
- `/Projects/opensource-projects/axon/target/build/axon/axon run`

## Phase 5 - Diagnostics and CLI

- [x] Enforce diagnostics schema `stage + code + reason` across all error paths.
- [x] Expand fixture coverage for `check/build/run/test/fmt/mcp`.
- [x] Ensure command behavior is stable from repository root.

Verify:
- `/Projects/opensource-projects/axon/target/build/axon/axon check`
- `/Projects/opensource-projects/axon/target/build/axon/axon build`
- `/Projects/opensource-projects/axon/target/build/axon/axon run`
- `/Projects/opensource-projects/axon/target/build/axon/axon test .`
- `/Projects/opensource-projects/axon/target/build/axon/axon fmt .`
- `/Projects/opensource-projects/axon/target/build/axon/axon mcp`
