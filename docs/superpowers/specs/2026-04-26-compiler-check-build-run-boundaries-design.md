# Compiler Check Build Run Boundaries

## Summary

Define the first real Axon compiler command boundary inside `axon-lang/` after CLI parsing and tracing are complete.

- `check(path)` is the authoritative compile-validity command.
- `build()` has no target argument and must call `check("")` first.
- `run()` has no target argument and must call `build()` first.
- `check(path)` must validate the selected scope end-to-end, including FFI, toolchain readiness, and native link readiness on the current machine.
- Compiler-side unexpected failures must be converted into deterministic compiler diagnostics rather than leaking raw panics, aborts, or passthrough behavior.

## Goals

- Keep command meanings strict and non-overlapping.
- Match the existing Rust-backed target semantics for filesystem-shaped `check(path)` values.
- Make `check(path)` strong enough that a successful check means the selected scope is fully compiler-valid and machine-buildable.
- Ensure `build()` does not rediscover compiler-validity failures after a successful `check("")`.
- Keep `main.ax` as a thin CLI dispatch layer and move compiler behavior under `src/compiler/`.

## Non-Goals

- Add target arguments to `build()` or `run()`.
- Introduce a new logical module-target grammar.
- Preserve fallback or passthrough command behavior.
- Return fake success values for unimplemented internal stages.
- Create a top-level shared `compile_project(...)` command abstraction that blurs the command boundary between `check`, `build`, and `run`.

## Command Contract

### `check(path: String) -> String`

`check` is the full correctness and buildability gate for a selected scope.

Supported target forms are filesystem-shaped only, matching the Rust-backed compiler behavior that already exists:

- `""`: whole project from `build.ax` using the configured entrypoint
- `"."`: current directory scope
- `"./..."`: current directory recursive scope
- `"src/foo.ax"`: single file scope within project context
- `"src/compiler"`: directory/module scope
- `"src/compiler/..."`: recursive directory scope

Directory names are the module structure already, so no separate module-target grammar is added.

### `build() -> String`

- Takes no target argument.
- Resolves the project from the current working directory.
- Must call `check("")` first.
- Only emits build artifacts if `check("")` succeeds.

### `run() -> String`

- Takes no target argument.
- Must call `build()` first.
- Only executes the built artifact if `build()` succeeds.

## End-to-End Behavior

After this phase lands:

- A user can run `check(path)` and get a true answer about whether the selected program scope is compiler-valid and machine-buildable now.
- A user can run `build()` and trust that artifact production only begins after full validation succeeds.
- A user can run `run()` and trust that execution only begins after a successful build.
- Compiler-internal surprises in the command path are surfaced as compiler diagnostics instead of raw crash output.

This phase is only complete when `check` proves real compile validity, not when files, APIs, or module names merely exist.

## Success Semantics

### `check(path)` success means all of the following are true

- the target path resolves correctly
- project discovery from `build.ax` is valid
- selected modules can be loaded
- syntax parses successfully
- imports and names resolve correctly
- semantic checks pass
- ownership and type validation pass
- FFI declarations and exported foreign surfaces are valid
- bridge and codegen preparation succeeds for the selected scope
- required native toolchains for the selected project are available
- native compile and link readiness are valid on this machine now

`check` is therefore not just a frontend validation command. It is a full compile-validity proof without final artifact emission.

### `build()` success means

- `check("")` succeeded
- artifact emission and linking succeeded

### `run()` success means

- `build()` succeeded
- execution launched successfully

## Failure Semantics

These must surface as compiler diagnostics:

- invalid target path or scope
- invalid `build.ax` or project layout
- module loading failures
- syntax, import, name, semantic, ownership, or type failures
- invalid FFI shape, export inventory, bridge, toolchain, or link readiness
- internal compiler faults converted at the command boundary

These must not leak to the user as raw behavior:

- uncaught Rust panics
- aborts
- passthrough stack traces
- partial-success command behavior

## Representation vs Reality

### Representation only

- command function names existing in `main.ax`
- compiler module directories existing under `src/compiler/`
- target-shape helper names existing under `compiler/proj`
- sidecar wrappers existing without real failure conversion

### Materially real behavior required in this phase

- `check(path)` performing real project and compiler validation through FFI and native readiness
- `build()` refusing to emit anything until `check("")` passes
- `run()` refusing to execute until `build()` passes
- unexpected internal faults being translated into compiler diagnostics

## Module Boundaries

### `src/main.ax`

- CLI dispatch only
- calls:
  - `check(cli_target())`
  - `build()`
  - `run()`
  - `run_fmt(cli_target())`

No deep compiler logic belongs here.

### `src/compiler/entry.ax`

Public compiler command surface only:

- `pub func check(path: String) String`
- `pub func build() String`
- `pub func run() String`

This file orchestrates command flow and does not become a dumping ground for parsing, graph loading, diagnostics formatting, or backend details.

### `src/compiler/proj/*`

- `build.ax` loading
- project discovery
- `check(path)` target and scope resolution
- module graph selection

### `src/compiler/syntax/*`

- lexing
- parsing
- AST loading

### `src/compiler/semantics/*`

- resolution
- semantic validation
- ownership and type validation

### `src/compiler/bootstrap/*`

- compiler-side foreign bootstrap coordination
- bridge/bootstrap preparation needed before buildability can be proven

### `src/compiler/backend/*`

- bridge and codegen preparation
- toolchain and link-readiness validation for `check`
- artifact emission for `build`
- execution launch handoff support for `run`

### `src/compiler/diagnostics/*`

- stable user-facing diagnostic shaping
- conversion of internal command-path failures into compiler errors

### `src/compiler/ir/*`

- used only if required by the real compile-validity path
- not required as a top-level command boundary of its own

## Command Flow

### `check(path)`

1. Resolve project and selected scope.
2. Load the module graph for that scope in project context.
3. Run syntax and semantics validation.
4. Validate FFI inventory and exported foreign surfaces.
5. Run bridge/codegen preparation and native toolchain/link-readiness checks far enough to prove buildability.
6. Return success or a compiler diagnostic failure.

### `build()`

1. Call `check("")`.
2. Stop immediately if `check` fails.
3. Emit artifacts and perform final linking.
4. Return success or a build failure.

### `run()`

1. Call `build()`.
2. Stop immediately if `build` fails.
3. Launch the built artifact.
4. Return success or a run failure.

## Error Boundary Rules

- All command paths that cross into Rust helpers for project loading, foreign inventory, bridge generation, toolchain inspection, artifact emission, or execution must use explicit failure boundaries.
- A Rust-side unexpected internal failure must be converted into a deterministic compiler error before it returns to Axon command flow.
- `check` must not succeed if later `build` would predictably fail due to compiler logic, bridge generation, FFI shape, or host toolchain unavailability.
- `build` may still fail after `check` only for artifact-emission or final execution-adjacent issues that happen after readiness was proven.

## Deferred Work

This spec defines the command contract and compiler command boundary. It does not yet define:

- the concrete diagnostic data model
- exact textual formatting of command output
- how much of current Rust workspace functionality is reused directly versus wrapped through new Axon entrypoints
- the precise internal APIs between `proj`, `syntax`, `semantics`, `bootstrap`, and `backend`

Those are implementation-planning details, not open product questions.

## Testing Requirements

Tests for this phase must prove behavior, not file existence.

Required behavior coverage:

- `check("")` validates a healthy project end-to-end
- `check(path)` accepts the supported filesystem target forms and selects the right scope
- `check` fails on syntax, semantic, ownership/type, FFI, toolchain, and link-readiness failures
- `check` converts internal command-path failures into compiler diagnostics instead of leaking raw crashes
- `build()` calls `check("")` first and stops on failure
- `run()` calls `build()` first and stops on failure
- `build()` does not accept target arguments
- `run()` does not accept target arguments

## Replacement Plan

This phase replaces the current placeholder command path in `main.ax` where `check`, `build`, and `run` hard-fail with `not implemented`.

When implementation starts, the old placeholder chain should be removed directly rather than kept behind compatibility helpers or fallback branches.
