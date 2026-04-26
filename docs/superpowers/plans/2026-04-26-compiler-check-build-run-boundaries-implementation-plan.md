# Compiler Check Build Run Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current `not implemented` command path in `axon-lang` with real `check(path)`, `build()`, and `run()` behavior where `check` proves end-to-end compile validity and machine buildability, `build` calls `check("")`, and `run` calls `build()`.

**Architecture:** Keep top-level commands separate in meaning and narrow in surface area. `src/main.ax` stays a thin dispatcher, `src/compiler/entry.ax` becomes the public command orchestrator, and the real work is split across `compiler/proj`, `compiler/diagnostics`, `compiler/bootstrap`, and `compiler/backend` with explicit Rust failure-boundary helpers behind narrow sidecar entrypoints.

**Tech Stack:** Axon project under `axon-lang/`, Rust sidecar helpers, root Rust workspace CLI/compiler, existing `cargo run -p axon -- check|build|test` flows.

---

## Global Implementer Context

Every implementer subagent must be given this context before starting a task:

- Work only inside `axon-lang/` unless a task explicitly requires root workspace changes.
- Axon direction: readable language, Rust/Go interop via static archives, ownership authority stays in Axon.

### DO NOT TOUCH — Already Implemented And Production

These files are real, working, committed code. Do NOT modify, refactor, rename, merge, or "clean up" any of them. If a task appears to require changing one of these files, escalate to the controller instead:

- `axon-lang/src/clap.rs` — real CLI arg parsing via `std::env::args()`. Exports `cli_command()` and `cli_target()`. No struct, no clap derive, no `parse_cli()`.
- `axon-lang/src/tracing.rs` — real `Tracer` struct with `info`/`error`/`warn`/`debug` methods. Exports `init_tracing() -> Tracer`. Uses `tracing_subscriber` with `EnvFilter`.
- `axon-lang/src/sidecar.rs` — real helpers: `axon_fail()`, `format_axon_file()`, `format_source_for_test()`. No tracing code lives here.
- `axon-lang/src/main.ax` — real CLI dispatch using `cli_command()`, `cli_target()`, `init_tracing()`. Already wired and working.
- `axon-lang/build.ax` — real project manifest. Dependencies: `tracing`, `tracing-subscriber`. No `clap` dependency.

If you find yourself wanting to rename `clap.rs` to `sidecar.rs`, merge tracing back into sidecar, reintroduce `clap` derive, add `CliResult` struct, change `parse_cli()` pattern, or move CLI helpers: STOP. That is wrong. The current split is correct and final.

### Current CLI Contract

- `check(path)` is the only target-taking command
- `build()` takes no target and must call `check("")`
- `run()` takes no target and must call `build()`
- `fmt` already works and must stay working

### Compiler Command Rules

- `check(path)` must validate the selected scope end-to-end, including FFI shape, bridge/codegen readiness, native toolchain availability, and native link readiness on the current machine.
- No raw panics or passthrough crashes may escape compiler command paths; unexpected failures must become compiler diagnostics.
- Target syntax for `check(path)` is filesystem-shaped only and must match the existing Rust-backed compiler behavior:
  - `""`, `.`, `./...`, `src/foo.ax`, `src/dir`, `src/dir/...`
- `build()` and `run()` do not accept targets.
- Keep changes minimal and direct. Remove superseded placeholder paths instead of layering compatibility wrappers.

## Required Skills Per Implementer

Every implementer subagent must load or follow these skills/instructions:

- `superpowers:test-driven-development`
- `superpowers:lint-and-validate`
- `superpowers:moyu`
- `superpowers:verification-before-completion`

Use these only when the task actually needs them:

- `superpowers:systematic-debugging` for failing tests or unexpected behavior
- `superpowers:error-handling-patterns` for Rust boundary conversion and diagnostic mapping
- `superpowers:software-architecture` for task packets that touch multiple compiler layers

## Controller Workflow

This plan is optimized for one controller agent using:

- `superpowers:dispatching-parallel-agents` to run independent task packets concurrently
- `superpowers:subagent-driven-development` to execute each task packet with:
  - one fresh implementer subagent
  - one spec-compliance review
  - one code-quality review

### Parallel Windows

Use this sequence:

1. Task 1 alone
2. Tasks 2, 3, and 4 in parallel
3. Task 5 after Tasks 2-4 land
4. Tasks 6 and 7 in parallel
5. Task 8 alone

Do not run parallel tasks that edit the same files.

## File Map

### Existing files to modify

- `axon-lang/src/compiler/entry.ax`
  - replace version-only stub with public `check`, `build`, `run` orchestration
- `axon-lang/src/compiler/proj/build_file.ax`
  - replace string stub with real build-file loading and configured-entrypoint access
- `axon-lang/src/compiler/proj/discover.ax`
  - real project discovery rooted at cwd
- `axon-lang/src/compiler/proj/module_graph.ax`
  - real scope/module selection rather than fixed string listing
- `axon-lang/src/compiler/proj/targets.ax`
  - real target normalization and scope selection for filesystem-shaped `check(path)`
- `axon-lang/src/compiler/proj/command_targets.ax`
  - remove trivial placeholder helpers or narrow them to actual command-target helpers if still needed
- `axon-lang/src/compiler/diagnostics/diagnostic.ax`
  - central compiler error shaping and unexpected-failure reporting
- `axon-lang/src/compiler/bootstrap/rust_bridge.ax`
  - replace hard-fail placeholders with actual bridge/toolchain readiness calls
- `axon-lang/src/compiler/backend/driver.ax`
  - real validation/build entry helpers for compile readiness and artifact emission
- `axon-lang/src/compiler/backend/ffi.ax`
  - real FFI validation surface, not placeholder descriptions
- `axon-lang/src/compiler/backend/link.ax`
  - link-readiness and final-link helpers

### New files likely needed

- `axon-lang/src/compiler/proj/target_scope.ax`
  - focused target-shape parsing helpers if `targets.ax` becomes too mixed
- `axon-lang/src/compiler/backend/toolchain.ax`
  - host toolchain detection and readiness checks
- `axon-lang/src/compiler/backend/readiness.ax`
  - compile-readiness aggregation used by `check`
- `axon-lang/src/compiler/diagnostics/boundary.ax`
  - command-path conversion of unexpected Rust/helper failures into compiler diagnostics
- `axon-lang/tests/check_command_targets.ax`
  - behavior tests for `check(path)` target resolution
- `axon-lang/tests/check_failures.ax`
  - behavior tests for syntax/semantic/ffi/toolchain/link failure reporting
- `axon-lang/tests/build_run_contract.ax`
  - behavior tests proving `build -> check` and `run -> build`

Create new files only if existing files become too mixed. Prefer updating focused existing files first.

## Task 1: Lock Command Contract With RED Tests

**Purpose:** Replace misleading placeholder coverage with behavior tests that define the real command contract before implementation.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:moyu`
- `superpowers:verification-before-completion`

**Files:**
- Modify: `axon-lang/tests/cli_routing.ax`
- Modify: `axon-lang/tests/pipeline_e2e.ax`
- Create: `axon-lang/tests/check_command_targets.ax`
- Create: `axon-lang/tests/build_run_contract.ax`

- [ ] **Step 1: Add failing tests for command dispatch contract**

Add tests that express the final contract instead of the current placeholder behavior.

Use Axon test cases shaped like:

```axon
test build_command_uses_no_target
    assert_eq(run_command_name("build"), "build")

test check_command_accepts_empty_target
    assert_eq(resolve_check_target(""), "project")

test check_command_accepts_recursive_dir
    assert_eq(resolve_check_target("./..."), "dir-recursive:./")

test run_command_uses_build_contract
    assert_eq(scope_name_for_command("run"), "project-run")
```

- [ ] **Step 2: Add failing tests for `build()` and `run()` sequencing**

Create behavior tests that assert command intent rather than artifact output.

Use test shapes like:

```axon
test build_contract_calls_project_check
    assert_eq(build_prereq_name(), "check:")

test run_contract_calls_build
    assert_eq(run_prereq_name(), "build")
```

- [ ] **Step 3: Run the focused Axon test command and confirm RED**

Run from repo root:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- new tests fail because target helpers and command contract helpers are still placeholders

- [ ] **Step 4: Remove assertions that hard-code obsolete placeholder strings**

Update or delete tests that only protect the old strings such as:

- `"check:project"`
- `"test:project"`
- fixed compiler module listings that no longer represent real behavior

- [ ] **Step 5: Re-run the same focused test command and keep the suite RED only for intended missing behavior**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- failures are only from the newly introduced contract tests

## Task 2: Project Discovery And Filesystem Target Resolution

**Parallel window:** Can run in parallel with Tasks 3 and 4.

**Purpose:** Replace fixed-string project and target helpers with real scope resolution matching the Rust-backed compiler behavior.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:software-architecture`
- `superpowers:moyu`

**Files:**
- Modify: `axon-lang/src/compiler/proj/build_file.ax`
- Modify: `axon-lang/src/compiler/proj/discover.ax`
- Modify: `axon-lang/src/compiler/proj/targets.ax`
- Modify: `axon-lang/src/compiler/proj/command_targets.ax`
- Modify: `axon-lang/src/compiler/proj/module_graph.ax`
- Create if needed: `axon-lang/src/compiler/proj/target_scope.ax`
- Modify: `axon-lang/tests/check_command_targets.ax`
- Modify: `axon-lang/tests/pipeline_e2e.ax`

- [ ] **Step 1: Add narrow failing tests for target normalization and project entry discovery**

Add or extend tests with exact expected outputs such as:

```axon
test resolve_empty_target_to_project
    assert_eq(resolve_check_target(""), "project")

test resolve_dot_target_to_current_dir
    assert_eq(resolve_check_target("."), "dir:.")

test resolve_recursive_target
    assert_eq(resolve_check_target("./..."), "dir-recursive:./")

test discover_project_entry_from_build_file
    assert_eq(discover_entry(), "./src/main.ax")
```

- [ ] **Step 2: Run only the target-resolution-focused tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- target resolution tests fail against current placeholders

- [ ] **Step 3: Implement real project discovery and build-file loading**

Replace fixed strings with functions that derive the configured main entry from `build.ax` and cwd.

Implementation shape:

```axon
pub func discover_entry() String
    project := load_project_build_file()
    return project_main_entry(project)

pub func resolve_check_target(path: String) String
    if path == ""
        return "project"
    elif path == "."
        return "dir:."
    elif path == "./..."
        return "dir-recursive:./"
    else
        return classify_path_target(path)
```

If target parsing starts bloating `targets.ax`, move only the parsing helpers into `target_scope.ax`.

- [ ] **Step 4: Make module-graph selection reflect selected scope rather than a fixed compiler-module string**

Refactor `module_graph.ax` so it returns scope-derived selections instead of a frozen list.

Implementation shape:

```axon
pub func selected_scope_modules(target: String) String
    return modules_for_scope(resolve_check_target(target))
```

- [ ] **Step 5: Run the focused tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- project discovery and target-resolution tests pass

- [ ] **Step 6: Self-review for scope creep and remove unused placeholder helpers**

Delete or narrow helpers that only mirrored command names without adding behavior.

## Task 3: Diagnostic Boundary And Unexpected Failure Conversion

**Parallel window:** Can run in parallel with Tasks 2 and 4.

**Purpose:** Establish the rule that compiler command paths never leak raw internal failures.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:error-handling-patterns`
- `superpowers:moyu`

**Files:**
- Modify: `axon-lang/src/compiler/diagnostics/diagnostic.ax`
- Create if needed: `axon-lang/src/compiler/diagnostics/boundary.ax`
- Modify: `axon-lang/src/sidecar.rs`
- Modify: `axon-lang/src/tracing.rs`
- Create: `axon-lang/tests/check_failures.ax`

- [ ] **Step 1: Add failing tests for unexpected-failure conversion**

Add tests that assert compiler-facing failure strings instead of raw panics.

Use shapes like:

```axon
test check_converts_internal_failure_to_diagnostic
    assert_eq(simulate_boundary_failure("boom"), "error: internal compiler failure: boom")
```

- [ ] **Step 2: Run the focused failure tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- tests fail because command-path helpers still panic or hard-fail directly

- [ ] **Step 3: Implement a single diagnostic-shaping path in Axon**

Refactor diagnostics to make one canonical compiler error string builder.

Implementation shape:

```axon
pub func compiler_error(kind: String, msg: String) String
    return "error: " + kind + ": " + msg

pub func internal_compiler_error(msg: String) String
    return compiler_error("internal compiler failure", msg)
```

If Axon string concatenation is too limited, keep the formatter in Rust and expose only one Axon wrapper.

- [ ] **Step 4: Add Rust-side boundary wrappers that catch internal command-path failures and return deterministic error strings**

Use `std::panic::catch_unwind` around Rust helper entrypoints used by compiler commands.

Implementation shape in `sidecar.rs`:

```rust
fn boundary_string<F>(op: F) -> String
where
    F: FnOnce() -> String + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(op) {
        Ok(value) => value,
        Err(_) => "error: internal compiler failure: unexpected panic".to_string(),
    }
}
```

Keep this narrowly scoped to compiler command helpers. Do not wrap unrelated formatting helpers.

- [ ] **Step 5: Re-run focused failure tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- failure-conversion tests pass

## Task 4: Toolchain, FFI, And Link-Readiness Validation Surface

**Parallel window:** Can run in parallel with Tasks 2 and 3.

**Purpose:** Replace backend/bootstrap placeholders with real readiness checks that `check(path)` can call before any artifact emission.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:software-architecture`
- `superpowers:error-handling-patterns`

**Files:**
- Modify: `axon-lang/src/compiler/bootstrap/rust_bridge.ax`
- Modify: `axon-lang/src/compiler/backend/driver.ax`
- Modify: `axon-lang/src/compiler/backend/ffi.ax`
- Modify: `axon-lang/src/compiler/backend/link.ax`
- Create if needed: `axon-lang/src/compiler/backend/toolchain.ax`
- Create if needed: `axon-lang/src/compiler/backend/readiness.ax`
- Modify: `axon-lang/tests/check_failures.ax`

- [ ] **Step 1: Add failing tests for FFI/toolchain/link readiness outcomes**

Add tests that define the readiness contract.

Use shapes like:

```axon
test ffi_validation_reports_export_error
    assert_eq(validate_ffi_surface("bad-export"), "error: ffi: bad-export")

test link_readiness_reports_missing_toolchain
    assert_eq(validate_link_readiness("missing-cc"), "error: toolchain: missing-cc")
```

- [ ] **Step 2: Run the focused readiness tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- readiness tests fail because backend helpers still hard-fail or describe strings only

- [ ] **Step 3: Replace placeholder backend helpers with readiness-returning helpers**

Implementation shape:

```axon
pub func validate_ffi_surface(scope: String) String
    return rust_validate_ffi_surface(scope)

pub func validate_link_readiness(scope: String) String
    return rust_validate_link_readiness(scope)

pub func validate_compile_readiness(scope: String) String
    ffi := validate_ffi_surface(scope)
    if ffi != "ok"
        return ffi
    return validate_link_readiness(scope)
```

Expose only what `check` needs. Do not invent extra status objects unless the current Axon surface requires them.

- [ ] **Step 4: Keep final artifact emission separate from readiness**

Refactor `backend/driver.ax` so one path validates readiness and a separate path emits/link artifacts for `build()`.

Implementation shape:

```axon
pub func ensure_buildable(scope: String) String
    return validate_compile_readiness(scope)

pub func emit_project_artifacts() String
    return rust_emit_project_artifacts()
```

- [ ] **Step 5: Re-run readiness tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- readiness and FFI failure tests pass

## Task 5: Real `check(path)` Command Orchestration

**Purpose:** Make `src/compiler/entry.ax` the real command boundary and replace `main.ax` check placeholder wiring.

**Depends on:** Tasks 2, 3, and 4.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:software-architecture`
- `superpowers:moyu`

**Files:**
- Modify: `axon-lang/src/compiler/entry.ax`
- Modify: `axon-lang/src/main.ax`
- Modify: `axon-lang/tests/pipeline_e2e.ax`
- Modify: `axon-lang/tests/compiler_smoke.ax`
- Modify: `axon-lang/tests/self_host_check.ax`

- [ ] **Step 1: Add failing tests for `check(path)` orchestration**

Add tests that assert `check` walks the full validation chain in order.

Use shapes like:

```axon
test check_project_returns_ok_for_self_host
    assert_eq(check(""), "ok")

test check_recursive_scope_returns_ok
    assert_eq(check("./..."), "ok")
```

- [ ] **Step 2: Run the focused compiler command tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `check` tests fail because `entry.ax` is still version-only and `main.ax` still routes to placeholders

- [ ] **Step 3: Implement `check(path)` as the public command orchestrator**

Refactor `entry.ax` to orchestrate the full path and return either `ok` or a compiler diagnostic string.

Implementation shape:

```axon
pub func check(path: String) String
    scope := resolve_check_target(path)
    modules := selected_scope_modules(path)
    syntax := validate_scope_syntax(modules)
    if syntax != "ok"
        return syntax
    semantics := validate_scope_semantics(modules)
    if semantics != "ok"
        return semantics
    return ensure_buildable(scope)
```

Keep command orchestration here; do not sink target parsing or diagnostics formatting into `entry.ax`.

- [ ] **Step 4: Replace `main.ax` placeholder command bodies with compiler entrypoint calls**

Target shape:

```axon
pub func run_check(path: String) String
    return check(path)

pub func run_build() String
    return build()

pub func run_run() String
    return run()
```

Also remove the now-wrong target passing for `build` and `run` from `main()`.

- [ ] **Step 5: Re-run compiler command tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `check` tests pass for self-host and selected-scope coverage

## Task 6: `build()` Artifact Emission After Successful `check("")`

**Parallel window:** Can run in parallel with Task 7 once Task 5 lands.

**Purpose:** Make `build()` obey the check-first contract and emit artifacts only after validation succeeds.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:moyu`
- `superpowers:verification-before-completion`

**Files:**
- Modify: `axon-lang/src/compiler/entry.ax`
- Modify: `axon-lang/src/compiler/backend/driver.ax`
- Modify: `axon-lang/src/compiler/backend/artifacts.ax`
- Modify: `axon-lang/tests/build_run_contract.ax`

- [ ] **Step 1: Add failing tests for `build()` sequencing and artifact emission contract**

Use tests like:

```axon
test build_stops_when_check_fails
    assert_eq(simulate_build_after_failed_check(), "error: check failed")

test build_returns_ok_after_emit
    assert_eq(simulate_build_after_successful_check(), "ok")
```

- [ ] **Step 2: Run the focused build tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `build` tests fail because `build()` is still missing or placeholder-backed

- [ ] **Step 3: Implement `build()` in `entry.ax`**

Implementation shape:

```axon
pub func build() String
    checked := check("")
    if checked != "ok"
        return checked
    return emit_project_artifacts()
```

- [ ] **Step 4: Keep artifact path helpers truthful**

If `artifacts.ax` remains, ensure it only describes real artifact directories actually used by `build()`.

- [ ] **Step 5: Re-run focused build tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `build` contract tests pass

## Task 7: `run()` Execution After Successful `build()`

**Parallel window:** Can run in parallel with Task 6 once Task 5 lands.

**Purpose:** Make `run()` obey the build-first contract without reintroducing target arguments.

**Required skills for implementer:**
- `superpowers:test-driven-development`
- `superpowers:error-handling-patterns`
- `superpowers:moyu`

**Files:**
- Modify: `axon-lang/src/compiler/entry.ax`
- Modify: `axon-lang/src/compiler/backend/driver.ax`
- Modify: `axon-lang/tests/build_run_contract.ax`

- [ ] **Step 1: Add failing tests for `run()` sequencing**

Use tests like:

```axon
test run_stops_when_build_fails
    assert_eq(simulate_run_after_failed_build(), "error: build failed")

test run_returns_ok_after_launch
    assert_eq(simulate_run_after_successful_build(), "ok")
```

- [ ] **Step 2: Run the focused run tests and confirm RED**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `run` tests fail because `run()` is still placeholder-backed

- [ ] **Step 3: Implement `run()` in `entry.ax`**

Implementation shape:

```axon
pub func run() String
    built := build()
    if built != "ok"
        return built
    return launch_project_binary()
```

Keep launch support in backend helpers; do not pull execution details into `entry.ax`.

- [ ] **Step 4: Re-run focused run tests until they pass**

Run:

```bash
cargo run -p axon -- test axon-lang
```

Expected:
- `run` contract tests pass

## Task 8: Cleanup, Full Verification, And Superseded Path Removal

**Purpose:** Remove remaining placeholder chains and verify the new command path end-to-end.

**Required skills for implementer:**
- `superpowers:lint-and-validate`
- `superpowers:verification-before-completion`
- `superpowers:moyu`

**Files:**
- Modify: any files still containing obsolete `not implemented` command placeholders in the new check/build/run path
- Modify: tests that still assert obsolete fixed strings or placeholder behavior

- [ ] **Step 1: Search for obsolete placeholder paths in the command flow**

Look for strings and helpers such as:

- `not implemented: check`
- `not implemented: build`
- `not implemented: run`
- command-name mirroring helpers that add no behavior

- [ ] **Step 2: Remove superseded code directly**

Delete or inline helpers that only existed for the placeholder command path. Do not keep dead compatibility branches.

- [ ] **Step 3: Run repo-level verification for the self-host project path**

Run from repo root:

```bash
cargo run -p axon -- check axon-lang
cargo run -p axon -- test axon-lang
cargo check --workspace
```

Expected:
- `axon-lang` check succeeds
- `axon-lang` tests pass
- workspace check succeeds

- [ ] **Step 4: Run formatting/lint validation required by the repo**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

Expected:
- no formatting diffs
- no clippy warnings

- [ ] **Step 5: Final self-review against spec requirements**

Confirm all of these are true before handing off:

- `check(path)` is the only target-taking command
- `build()` calls `check("")`
- `run()` calls `build()`
- `check` covers FFI/toolchain/link readiness
- command-path internal failures no longer leak raw crashes
- placeholder command chain is removed

## Review Checklist For Controller

After each task packet completes:

1. Run spec-compliance review before code-quality review.
2. Reject any extra command-surface expansion.
3. Reject any fallback path that keeps old placeholder behavior alive.
4. Reject any implementation that makes `build` or `run` accept targets.
5. Reject any `check` implementation that skips toolchain or FFI readiness.
6. **REJECT any change to `clap.rs`, `tracing.rs`, `sidecar.rs`, `build.ax`, or `main.ax`.** These files are production and must not be touched. If an implementer modified any of them, revert immediately and re-dispatch with explicit instructions.

## Spec Coverage Map

- Separate command semantics: Tasks 1, 5, 6, 7
- Filesystem-only `check(path)` targets: Tasks 1, 2
- `check` as full compile-validity + machine-buildability proof: Tasks 4, 5
- No raw panic/passthrough failures: Tasks 3, 5, 8
- `build()` depends on `check("")`: Tasks 1, 6
- `run()` depends on `build()`: Tasks 1, 7
- Placeholder-chain removal: Tasks 1, 8

## Execution Handoff

Plan complete and saved to `axon-lang/docs/superpowers/plans/2026-04-26-compiler-check-build-run-boundaries-implementation-plan.md`.

Two execution options:

1. Subagent-Driven (recommended) - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. Inline Execution - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
