# Axon Self-Hosting Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Use `superpowers:dispatching-parallel-agents` only for explicitly parallel audit/test-writing waves. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete `depreciating-soon-compiler-do-not-rename/` and prove the repo-root Axon compiler is totally self-independent before and after deletion.

**Architecture:** The final repo has one shipped compiler: the repo-root Axon project. Axon owns compiler policy and logic: source model, lexing, parsing, resolution, type checking, ownership, MIR lowering, project graph, command semantics, diagnostics, build/test orchestration, and FFI policy. Rust remains only as thin sidecar code for OS/LLVM/toolchain boundaries: file/path/process primitives, native object emission/linking, CLI/logging, and foreign-language process invocation.

**Tech Stack:** Axon (`src/**/*.ax`), thin Rust sidecars (`src/**/*.rs`), shell verification scripts under `scripts/`, existing native toolchain/LLVM/link integration behind root sidecars only.

---

## Non-Negotiable Completion Rules

- The final task deletes `depreciating-soon-compiler-do-not-rename/`.
- The old tree must be quarantined before deletion and verification must pass while it is unavailable.
- Verification must run both before deletion and after deletion.
- No task may count as complete if it only adds string encodings, wrappers, metadata, scaffolding, or tests that prove existence rather than behavior.
- No compatibility/fallback path may keep the old compiler alive.
- No script may search for `depreciating-soon-compiler-do-not-rename/Cargo.toml` after the cutover gate begins.
- `src/compiler/backend/backend.rs` must not invoke `cargo` in `rust-backed-compiler-for-axon`, `rust-self-compiler-for-axon`, or `depreciating-soon-compiler-do-not-rename` during self-built `axon build`.
- `src/compiler/ir/ir.rs::lower_project` must not be a file counter or marker writer; it must consume Axon-owned compiler output and hand real backend input to the native boundary.
- Rust sidecars may keep LLVM/OS work, but must not decide compiler semantics, call resolution, lowering, ownership, or command behavior.

---

## Representation vs Reality

**Real today:**
- The repo-root Axon project has useful token/AST/MIR/type helper surfaces and some real pipeline orchestration.
- `src/main.ax` dispatches compiler commands through repo-root `src/compiler/entry.ax`.
- `scripts/verify-self-bootstrap.sh` and `scripts/verify-self-hosting-cutover.sh` exist as verification shells.

**Still not real enough for deletion:**
- The active native build path still depends on a Cargo-built host compiler from non-root compiler workspaces.
- `src/compiler/ir/ir.rs::lower_project` is scaffold: it collects `.ax` filenames and writes `target/cache/lowered.ir`; it does not lower AST to executable MIR/backend input.
- `src/compiler/backend/backend.rs::run_lowered_to_artifact` still shells to a host compiler binary and copies that host-produced artifact.
- `src/compiler/backend/backend.rs::run_tests_via_rust_compiler` still shells to a host compiler binary.
- `src/compiler/syntax/parser.ax` and `src/compiler/semantics/*.ax` contain partial/string-scanning behavior; they do not yet replace the old full parser/resolver/typechecker over real AST data.
- The previous suffixed-binary scripts still bootstrap through Cargo manifests, including the deprecated tree unless removed.

**Actual cutover boundary:**
- Cutover is complete only when the old directory is unavailable or deleted and the already-produced repo-root compiler binary can run `check`, `build`, `run`, and `test` on this repo and at least one external fixture without any reference to the old tree or Cargo host compiler workspaces.

---

## Design Completeness Checks

**End-to-end behavior after completion:**
- A user can clone the repo without `depreciating-soon-compiler-do-not-rename/`, provide a previously produced `target/build/axon/axon` or build once from an approved external bootstrap, and then use that binary to repeatedly rebuild the repo-root Axon compiler.
- `target/build/axon/axon_selfcompiled3` can `check`, `build`, `run`, and `test` the Axon repo and a non-self fixture.

**Real boundary:**
- Axon compiler logic ends at a serialized backend request: module graph, typed AST/MIR, ownership metadata, foreign inventory, link inputs, and artifact plan.
- Rust sidecars begin at OS/native boundaries: reading/writing files, invoking Rust/Go/C/linkers, LLVM object emission, process execution, permissions, CLI args, logging.

**Deferred work not allowed for this cutover:**
- Full parser, resolver, typechecker, MIR lowering, build/test orchestration, and backend request generation cannot be deferred.
- Deleting the old tree cannot be deferred.

**Behavior tests required:**
- Every phase below includes behavior tests that prove the active path changed, not just that helpers exist.

---

## Execution Policy

- Required implementer skills per task: `moyu`, `test-driven-development`, `lint-and-validate`.
- Required before completion: `verification-before-completion`, `requesting-code-review`.
- Use `subagent-driven-development` for one fresh implementer per task.
- Use two reviews after each task: spec compliance first, code quality second.
- Use `dispatching-parallel-agents` only for inventory/test-writing tasks that touch disjoint files.
- Do not commit unless explicitly asked.

---

## File Responsibility Map

### Axon-Owned Compiler Logic
- `src/compiler/syntax/token.ax`: token kinds, spans, token values, token stream operations.
- `src/compiler/syntax/ast.ax`: real AST data model and AST accessors.
- `src/compiler/syntax/lexer.ax`: full tokenization, indentation, literals, comments, raw foreign blocks.
- `src/compiler/syntax/parser.ax`: full parser from token stream to AST.
- `src/compiler/semantics/resolve.ax`: module/import/name resolution and symbol tables.
- `src/compiler/semantics/types.ax`: type model, inference, unification, operator typing.
- `src/compiler/semantics/ownership.ax`: ownership analysis and cleanup metadata.
- `src/compiler/semantics/check.ax`: semantic pipeline orchestration over AST/project graph.
- `src/compiler/ir/ir.ax`: MIR model, typed local/block/function/module representation.
- `src/compiler/ir/lower.ax`: AST to MIR lowering and ownership annotation emission.
- `src/compiler/proj/*.ax`: `build.ax` parsing, source discovery policy, module graph, command target selection.
- `src/compiler/backend/ffi.ax`: foreign inventory and validation policy.
- `src/compiler/backend/artifacts.ax`: artifact layout policy.
- `src/compiler/backend/link.ax`: link plan policy.
- `src/compiler/entry.ax`: command semantics for `check`, `build`, `run`, `test`, `fmt`.
- `src/compiler/pipeline_check/pipeline_check.ax`: check pipeline stages and failure propagation.
- `src/compiler/diagnostics/diagnostic.ax`: diagnostic codes, severity, formatting policy.

### Thin Rust FFI Only
- `src/compiler/proj/discover.rs`: filesystem/path/string primitives only.
- `src/compiler/syntax/lexer.rs`: delete or reduce to file read/walk primitive only after Axon lexer owns logic.
- `src/compiler/syntax/parser.rs`: delete or reduce to file read/walk primitive only after Axon parser owns logic.
- `src/compiler/semantics/semantics.rs`: delete or reduce to file read/walk primitive only after Axon semantics owns logic.
- `src/compiler/semantics/ownership.rs`: delete or reduce to file read/walk primitive only after Axon ownership owns logic.
- `src/compiler/ir/ir.rs`: delete scaffold lowering; keep only serialization/native-boundary helpers if necessary.
- `src/compiler/backend/backend.rs`: no Cargo host compiler invocation; keep native artifact writing, process launching, binary copying, LLVM/link boundary.
- `src/compiler/backend/toolchain.rs`: native tool probes only; no policy requiring Cargo host compiler during self-build.
- `src/clap.rs`, `src/tracing.rs`, `src/sidecar.rs`: CLI/logging/panic/formatting boundaries only.

---

## Critical Active-Path Blockers

### Blocker A: Backend Still Invokes Host Compiler
Current active dependency:
- `src/compiler/backend/backend.rs::run_lowered_to_artifact` selects `rust-self-compiler-for-axon` or `rust-backed-compiler-for-axon`, runs `cargo build -p axon`, then runs that host binary.
- `src/compiler/backend/backend.rs::run_tests_via_rust_compiler` repeats that host compiler selection.

Completion condition:
- `axon build` from a self-built binary never runs Cargo for a compiler workspace.
- If Cargo/Rust/Go are invoked, it is only for user sidecars or foreign archive generation, not for compiling Axon’s compiler.

### Blocker B: Lowering Is Not Real
Current active dependency:
- `src/compiler/ir/ir.rs::lower_project` writes a file list and returns `ok:lowered:<count>`.

Completion condition:
- `lower_project` or its replacement consumes real parsed/typed AST/project graph and returns backend-ready MIR/link input.
- The backend cannot fabricate a successful build from a marker string.

### Blocker C: Parser/Semantics Are Not Full Active Replacements
Current active dependency:
- Axon parser/semantic files include structural/string-scanning helpers but do not yet produce/consume full AST and typed IR equivalent to the old compiler.

Completion condition:
- `axon check` catches the old compiler’s representative parser/resolver/typechecker/ownership errors via Axon-owned code.
- Rust sidecars do not perform compiler-rule decisions.

### Blocker D: Verification Still Allows Deprecated Tree
Current active dependency:
- Existing scripts search Cargo manifests, including deprecated locations.

Completion condition:
- Pre-delete verification quarantines the old tree and proves the self-built binary works.
- Post-delete verification repeats the same checks after removing all references.

---

## Implementation Tasks

### Task 0: Freeze Current Reality And Add Failure Gates To The Plan

**Files:**
- Modify: `docs/superpowers/plans/2026-04-30-axon-self-hosting-cutover.md`

- [ ] Record the four blockers above in the plan.
- [ ] Add a rule that model helpers do not count as completed compiler stages.
- [ ] Add a rule that Task 12 cannot start until old-tree quarantine verification passes.
- [ ] Verify the plan contains no acceptance criterion based only on wrappers, string encodings, or marker files.

### Task 1: Add A Dependency Sentinel That Fails On Old-Tree Use

**Files:**
- Modify: `scripts/verify-self-hosting-cutover.sh`
- Modify: `scripts/verify-self-bootstrap.sh`
- Create: `scripts/assert-no-legacy-compiler-refs.sh`

- [ ] Add a shell sentinel that searches `src/`, `scripts/`, `AGENTS.md`, and `docs/` for `depreciating-soon-compiler-do-not-rename`.
- [ ] The sentinel must fail unless it is running in the explicit pre-cleanup documentation phase.
- [ ] Remove deprecated manifest search entries from bootstrap scripts before cutover verification.
- [ ] Add a pre-delete quarantine variable: `AXON_LEGACY_COMPILER_QUARANTINE=1`.
- [ ] Verification command:
  ```bash
  bash -n scripts/assert-no-legacy-compiler-refs.sh
  bash -n scripts/verify-self-bootstrap.sh
  bash -n scripts/verify-self-hosting-cutover.sh
  ```
- [ ] Expected: all scripts parse; sentinel fails before cleanup and passes after cleanup tasks remove references.

### Task 2: Replace Backend Host-Compiler Invocation With Self-Compiler Backend Boundary

**Files:**
- Modify: `src/compiler/backend/backend.rs`
- Modify: `src/compiler/backend/artifacts.ax`
- Modify: `src/compiler/backend/link.ax`
- Modify: `src/compiler/backend/bootstrap.ax`
- Test: `src/compiler/entry.test.ax`

- [ ] Write a failing test proving `build()` rejects marker-only lowered strings such as `ok:lowered:3`.
- [ ] Write a failing test proving `build()` does not require `rust-backed-compiler-for-axon`, `rust-self-compiler-for-axon`, or `depreciating-soon-compiler-do-not-rename` directories to exist.
- [ ] Replace `run_lowered_to_artifact` behavior so it accepts a real backend request format, not `ok:lowered:<count>`.
- [ ] Remove host-root selection and all Cargo compiler workspace invocations from `run_lowered_to_artifact`.
- [ ] Preserve only native boundary work: write objects, invoke linker, publish binary, set executable bits, preserve suffixed binaries.
- [ ] Replace `run_tests_via_rust_compiler` with Axon-owned test orchestration plus process execution for produced test binaries.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ./target/build/axon/axon build
  ```
- [ ] Expected: no process command includes `cargo build -p axon` for a compiler workspace.

### Task 3: Make `lower_project` Real And Remove Marker Lowering

**Files:**
- Modify: `src/compiler/ir/ir.rs`
- Modify: `src/compiler/ir/ir.ax`
- Modify: `src/compiler/ir/lower.ax`
- Test: `src/compiler/ir/ir.test.ax`

- [ ] Write a failing test that `lower_project` output includes function/module records, not only a file count.
- [ ] Write a failing test that a simple function with `return 42` lowers to a MIR function with an entry block and return terminator.
- [ ] Write a failing test that a function call lowers to a call target with argument expressions.
- [ ] Implement real MIR module/function/block/local/statement/terminator representation in Axon.
- [ ] Implement AST-to-MIR lowering for literals, identifiers, binary/unary ops, calls, returns, bindings, assignments, `if/elif/else`, `while`, `break`, `continue`.
- [ ] Emit ownership metadata needed by backend: owned locals, string literal locals, aggregate field modes.
- [ ] Remove or rewrite `src/compiler/ir/ir.rs::lower_project` so it cannot succeed by counting files.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ./target/build/axon/axon build
  ```
- [ ] Expected: `target/cache/lowered.ir` or replacement contains real MIR/backend request records for project modules.

### Task 4: Make Lexer Fully Axon-Owned In The Active Check Path

**Files:**
- Modify: `src/compiler/syntax/lexer.ax`
- Modify: `src/compiler/syntax/lexer.rs`
- Test: `src/compiler/syntax/lexer.test.ax`

- [ ] Write failing tests for indentation/dedent stacks, inconsistent indentation, blank lines, comment-only lines, newline suppression inside delimiters.
- [ ] Write failing tests for `@rust ... @end`, `@go ... @end`, unterminated raw blocks, string escapes, char escapes, f-string starts, numeric underscores, all keywords/operators.
- [ ] Implement Axon-owned token stream generation with real spans and EOF.
- [ ] Reduce `lexer.rs` to file read/walk only, or delete it if Axon can drive file iteration through existing path primitives.
- [ ] Update `pipeline_check.ax` to call Axon-owned lexer output, not Rust token classification.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ```
- [ ] Expected: lexer failures come from Axon diagnostic code paths; no token classification remains in Rust.

### Task 5: Make Parser Fully Axon-Owned And Produce Real AST

**Files:**
- Modify: `src/compiler/syntax/ast.ax`
- Modify: `src/compiler/syntax/parser.ax`
- Modify: `src/compiler/syntax/parser.rs`
- Test: `src/compiler/syntax/parser.test.ax`

- [ ] Write failing tests for every declaration kind: project, bin, deps, import, include, raw foreign block, func, method, struct, enum, trait, error, test.
- [ ] Write failing tests for statements: binding, typed mut binding, tuple destructuring, assignment, `+=`, `-=`, return, if/elif/else, while, break, continue, match, defer, errdefer, labels.
- [ ] Write failing tests for expression precedence, calls, member/index access, constructors, tuple/list literals, f-strings, try/catch, orelse, ordefault.
- [ ] Write failing tests for type syntax: generics, tuple returns, `?T`, `!T`, grouped `!(?T)`, invalid stacked sigils.
- [ ] Implement parser over the Axon token stream and produce AST records consumed by semantics and lowering.
- [ ] Reduce `parser.rs` to file read/walk only, or delete it if no longer needed.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ```
- [ ] Expected: parser can build AST for all repo-root source files and representative fixtures.

### Task 6: Port Project Graph And Build Manifest Semantics

**Files:**
- Modify: `src/compiler/proj/build_file.ax`
- Modify: `src/compiler/proj/discover.ax`
- Modify: `src/compiler/proj/module_graph.ax`
- Modify: `src/compiler/proj/targets.ax`
- Modify: `src/compiler/proj/command_targets.ax`
- Modify: `src/compiler/proj/discover.rs`
- Test: `src/compiler/proj/loading.test.ax`
- Test: `src/compiler/proj/targets.test.ax`
- Test: `src/compiler/semantics/project_parity.test.ax`

- [ ] Write failing tests for `build.ax`: project name, hyphenated project/bin names, `main`, `deps`, `rust_deps`, `go_deps`, `python_deps`.
- [ ] Write failing tests for module discovery: app files, colocated test files, integration tests, sidecar association, import path to module path conversion.
- [ ] Write failing tests for check/test target scopes: project, file, module, tree `...`, invalid outside-project path.
- [ ] Implement Axon-owned project graph and command-target resolution.
- [ ] Keep Rust only for directory listing, path canonicalization, file reads, and existence checks.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ./target/build/axon/axon test
  ```
- [ ] Expected: project/module errors and target-scope errors are produced by Axon-owned code.

### Task 7: Port Resolver And Import Semantics Over Real AST

**Files:**
- Modify: `src/compiler/semantics/resolve.ax`
- Modify: `src/compiler/semantics/check.ax`
- Modify: `src/compiler/semantics/semantics.rs`
- Test: `src/compiler/semantics/check.test.ax`
- Test: `src/compiler/semantics/project_parity.test.ax`

- [ ] Write failing tests for duplicate declarations across functions, types, structs, enums, traits, errors, tests.
- [ ] Write failing tests for unresolved imports, self-import, duplicate import, private direct import, public direct import, namespace import, alias namespace access, import/declaration collision.
- [ ] Write failing tests for struct field duplicates, enum variant duplicates, trait method duplicates, invalid method `self`, invalid associated `self`.
- [ ] Implement symbol tables over project AST.
- [ ] Implement module import resolution and visibility rules.
- [ ] Reduce `semantics.rs` to file iteration/string transport only, or delete if no longer needed.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ```
- [ ] Expected: resolver diagnostics match representative old behavior without Rust semantic decisions.

### Task 8: Port Typechecker And Lint Semantics Over Real AST

**Files:**
- Modify: `src/compiler/semantics/types.ax`
- Modify: `src/compiler/semantics/check.ax`
- Modify: `src/compiler/diagnostics/diagnostic.ax`
- Test: `src/compiler/semantics/types.test.ax`
- Test: `src/compiler/semantics/check.test.ax`

- [ ] Write failing tests for primitive/literal typing, integer widths/overflow, return mismatch, call arity/type mismatch, binary/unary operator typing.
- [ ] Write failing tests for struct constructors, field access, field assignment mutability, methods, associated funcs.
- [ ] Write failing tests for options/results, `try`, `catch`, `orelse`, `ordefault`, tuple returns, tuple destructuring.
- [ ] Write failing tests for warnings: unused local, unreachable code after `return`/`break`/`continue`, warning suppression by code.
- [ ] Implement scoped type environment, type inference, unification, expected-type propagation, and diagnostics over AST.
- [ ] Remove string-line heuristic type decisions from Rust and Axon wrappers.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ```
- [ ] Expected: self-source type errors/warnings are Axon-generated and no host typechecker dependency remains.

### Task 9: Port Ownership Analysis And Cleanup Metadata

**Files:**
- Modify: `src/compiler/semantics/ownership.ax`
- Modify: `src/compiler/semantics/ownership.rs`
- Modify: `src/compiler/ir/lower.ax`
- Test: `src/compiler/semantics/ownership.test.ax`
- Test: `src/compiler/ir/ir.test.ax`

- [ ] Write failing tests for canonical owner selection, returned locals, returned fields from params/locals, aliases invalidated by mut reassignment and field mutation.
- [ ] Write failing tests for branch reconciliation across if/elif/else and match arms.
- [ ] Write failing tests proving tuple returns are path groups and do not emit aggregate shell cleanup.
- [ ] Write failing tests proving aggregate shell cleanup frees only inline-owned fields and skips pointer-backed fields.
- [ ] Implement ownership summaries in Axon.
- [ ] Feed ownership summaries into MIR lowering.
- [ ] Reduce `ownership.rs` to file iteration only, or delete if no longer needed.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ./target/build/axon/axon build
  ```
- [ ] Expected: generated backend request contains ownership metadata used by native boundary.

### Task 10: Port FFI Inventory And Validation Policy

**Files:**
- Modify: `src/compiler/backend/ffi.ax`
- Modify: `src/compiler/backend/link.ax`
- Modify: `src/compiler/backend/backend.rs`
- Modify: `src/compiler/backend/toolchain.rs`
- Test: backend/FFI tests adjacent to `src/compiler/backend/ffi.ax`

- [ ] Write failing tests for explicit Rust export inventory, Go export inventory, unsupported types, async FFI rejection, `void` param/return rejection.
- [ ] Write failing tests for Rust lifetime stripping and `std::sync::Arc<T>` type splitting.
- [ ] Write failing tests for foreign constructor/function handle registration, read-only property assignment rejection, instance/type method call rules.
- [ ] Implement FFI policy in Axon.
- [ ] Keep Rust only for invoking cargo/go/rustc, generating archive/bridge files, reading compiler diagnostics, and linking static archives.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon check ""
  ./target/build/axon/axon build
  ```
- [ ] Expected: FFI validation diagnostics come from Axon-owned policy.

### Task 11: Port Codegen Policy To Axon Backend Requests

**Files:**
- Modify: `src/compiler/backend/artifacts.ax`
- Modify: `src/compiler/backend/link.ax`
- Modify: `src/compiler/backend/backend.rs`
- Modify: `src/compiler/ir/lower.ax`
- Test: backend behavior tests adjacent to backend modules.

- [ ] Write failing tests for backend request content: functions, globals, call targets, link inputs, ownership cleanup actions.
- [ ] Write failing tests for string equality/comparison by content, bool branch coercion, builtin consumers, pointer-string cleanup skipping, aggregate cleanup recursion.
- [ ] Move policy decisions out of Rust: symbol naming, builtin lowering contract, type marshalling contract, ownership cleanup contract, artifact path policy.
- [ ] Leave Rust responsible for LLVM object emission and linker invocation only.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon build
  ./target/build/axon/axon run
  ```
- [ ] Expected: native binary behavior comes from Axon-produced backend request, not a host compiler subprocess.

### Task 12: Port Test Command Semantics

**Files:**
- Modify: `src/compiler/entry.ax`
- Modify: `src/compiler/pipeline_check/pipeline_check.ax`
- Modify: `src/compiler/proj/targets.ax`
- Modify: `src/compiler/backend/backend.rs`
- Test: `src/compiler/entry.test.ax`

- [ ] Write failing tests for colocated `src/**/*.test.ax` access to private module items.
- [ ] Write failing tests for isolated `tests/**/*.ax` integration programs seeing only public app surface.
- [ ] Write failing tests for file/module/tree test target filtering.
- [ ] Implement Axon-owned test target expansion and test binary plan generation.
- [ ] Remove `run_tests_via_rust_compiler` host-compiler delegation.
- [ ] Verification command:
  ```bash
  ./target/build/axon/axon test
  ```
- [ ] Expected: tests run through repo-root compiler path only.

### Task 13: Pre-Delete Quarantine Verification

**Files:**
- Modify: `scripts/verify-self-hosting-cutover.sh`
- Modify: `scripts/verify-independent-axon.sh`
- Create: `scripts/verify-no-legacy-before-delete.sh`

- [ ] Script must move `depreciating-soon-compiler-do-not-rename/` to `target/quarantine/depreciating-soon-compiler-do-not-rename/` or otherwise make it unavailable.
- [ ] Script must trap failures and restore the directory from quarantine.
- [ ] Script must run `scripts/assert-no-legacy-compiler-refs.sh` while the old tree is unavailable.
- [ ] Script must run:
  ```bash
  target/build/axon/axon check ""
  target/build/axon/axon build
  target/build/axon/axon run
  target/build/axon/axon test
  ```
- [ ] Script must produce `axon_rustcompiled1`, `axon_selfcompiled1`, `axon_selfcompiled2`, `axon_selfcompiled3` snapshots without using the quarantined tree.
- [ ] Script must run `check`, `build`, `run`, and `test` on at least one non-self fixture.
- [ ] Expected: all commands pass while the old tree is unavailable.

### Task 14: Delete Legacy Compiler Tree And Cascading References

**Files:**
- Delete: `depreciating-soon-compiler-do-not-rename/`
- Modify: `AGENTS.md`
- Modify: `scripts/*.sh`
- Modify: docs referencing the old tree

- [ ] Delete the entire old compiler directory.
- [ ] Remove all docs/scripts/env-var descriptions that instruct use of the old path.
- [ ] Keep references only if they describe historical removal, not active workflow.
- [ ] Run:
  ```bash
  scripts/assert-no-legacy-compiler-refs.sh
  ```
- [ ] Expected: no active references remain.

### Task 15: Post-Delete Verification

**Files:**
- Modify: `scripts/verify-self-hosting-cutover.sh` if needed for post-delete mode only.

- [ ] From a tree where `depreciating-soon-compiler-do-not-rename/` is absent, run:
  ```bash
  scripts/verify-self-hosting-cutover.sh
  ```
- [ ] Verify all suffixed binaries exist:
  ```bash
  test -x target/build/axon/axon_rustcompiled1
  test -x target/build/axon/axon_selfcompiled1
  test -x target/build/axon/axon_selfcompiled2
  test -x target/build/axon/axon_selfcompiled3
  ```
- [ ] Verify final binary:
  ```bash
  target/build/axon/axon_selfcompiled3 check ""
  target/build/axon/axon_selfcompiled3 build
  target/build/axon/axon_selfcompiled3 run
  target/build/axon/axon_selfcompiled3 test
  ```
- [ ] Verify non-self fixture:
  ```bash
  target/build/axon/axon_selfcompiled3 check <fixture-path>
  target/build/axon/axon_selfcompiled3 build <fixture-path>
  target/build/axon/axon_selfcompiled3 run <fixture-path>
  target/build/axon/axon_selfcompiled3 test <fixture-path>
  ```
- [ ] Expected: every command succeeds without old tree, old manifest, or host compiler workspace fallback.

---

## Parallelization Guidance

### Safe Parallel Waves
- Lexer fixture/test authoring can run in parallel with parser fixture/test authoring only before implementation starts.
- Typechecker test authoring can run in parallel with ownership test authoring if they do not edit the same files.
- FFI validation tests can run in parallel with project graph tests.

### Sequential-Only Work
- Parser implementation after token stream contract is fixed.
- Resolver after parser AST contract is fixed.
- Typechecker after resolver symbol table contract is fixed.
- MIR lowering after typed AST and ownership metadata contracts are fixed.
- Backend host-compiler removal after real lowering exists.
- Pre-delete quarantine verification.
- Actual deletion.
- Post-delete verification.

---

## Final Acceptance Criteria

The plan is complete only if all are true:

- `depreciating-soon-compiler-do-not-rename/` is deleted.
- No active file in `src/`, `scripts/`, `AGENTS.md`, or docs references the deleted tree as an available build path.
- `src/compiler/backend/backend.rs` does not invoke Cargo to build or run a compiler workspace during `axon build`, `axon run`, or `axon test`.
- `src/compiler/ir/ir.rs::lower_project` is not marker/file-count lowering.
- Rust sidecars contain no compiler-policy decisions beyond OS/LLVM/process/file/native boundary work.
- Pre-delete quarantine verification passes with the old tree unavailable.
- Post-delete verification passes after the old tree is removed.
- `target/build/axon/axon_rustcompiled1`, `axon_selfcompiled1`, `axon_selfcompiled2`, and `axon_selfcompiled3` exist and are executable.
- `axon_selfcompiled3` can `check`, `build`, `run`, and `test` this repo.
- `axon_selfcompiled3` can `check`, `build`, `run`, and `test` at least one non-self Axon fixture.
- Behavior tests prove lexing, parsing, resolution, typechecking, ownership, MIR lowering, build orchestration, FFI validation, and test command behavior through the repo-root compiler path.

---

## Target Verification Commands

```bash
# before deletion: old tree must be made unavailable by quarantine
scripts/verify-no-legacy-before-delete.sh

# deletion
rm -rf depreciating-soon-compiler-do-not-rename

# reference guard
scripts/assert-no-legacy-compiler-refs.sh

# after deletion: full cutover verification
scripts/verify-self-hosting-cutover.sh

# final binary proof
target/build/axon/axon_selfcompiled3 check ""
target/build/axon/axon_selfcompiled3 build
target/build/axon/axon_selfcompiled3 run
target/build/axon/axon_selfcompiled3 test
```

Expected: all commands succeed from a repo that does not contain `depreciating-soon-compiler-do-not-rename/`.
