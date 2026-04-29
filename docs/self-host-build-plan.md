# Self-Independent Axon Build & Test Pipeline — Plan

## 1. Goal

Make `axon build` and `axon test` operate **end-to-end without any runtime dependency on the Rust host compiler at `rust-backed-compiler-for-axon/`**. Today the self-built `axon` binary (in `target/debug/axon`) shells out to `cargo build` + the host `axon` binary inside `rust-backed-compiler-for-axon/` to produce native artifacts (see `src/compiler/backend/backend.rs` `run_lowered_to_artifact`). The host compiler must remain untouched (still used to bootstrap the first stage), but it must **not** be invoked by the self-built binary at runtime.

### Constraints (from user)
- **Do not remove or modify** anything in `rust-backed-compiler-for-axon/`. It stays as-is for bootstrap.
- **Bring the necessary code into the Axon project** as Rust sidecars under `src/compiler/`.
- **Rust is allowed via FFI** — sidecars (`.rs` files alongside `.ax` files), declared rust deps in `build.ax`, and external tools (`clang`, `cargo`, `rustc`) on PATH are all allowed. The Axon language itself is interop-based, so we use Rust where it makes sense and Axon for orchestration.
- **Production end-to-end** — every stage must actually work, not stub.

## 2. Current architecture (today)

```
+-------------------------+      +------------------------------+
| target/debug/axon       |----->| cargo build -p axon          |  (re-builds host)
| (self-built)            |      | rust-backed-compiler-for-axon|
|                         |----->|   target/debug/axon  build   |  (delegates entire build)
+-------------------------+      +------------------------------+
                                            |
                                            v
                                  produces target/build/<bin>
                                            |
                                            v
                                  copied to target/build/axon/app
```

Concretely, `src/compiler/backend/backend.rs::run_lowered_to_artifact` does this delegation. `src/compiler/ir/ir.rs::lower_project` only writes a manifest of `.ax` filenames to `target/cache/lowered.ir` — no real lowering.

## 3. Target architecture (after this plan)

```
+--------------------------+
| target/debug/axon        |
| (self-built, self-host)  |
| Axon orchestration       |
| + Rust sidecars under    |
|   src/compiler/{...}.rs  |
+--------------------------+
        |
        v
   .ax sources -> AST -> Resolver -> Typecheck -> MIR -> LLVM IR
                                                          |
                                                          v
                                                    clang/gcc + system linker
                                                          |
                                                          v
                                                    target/build/<bin>/<bin>
```

External tools required at runtime: `clang` (or `gcc`), `cargo`, `rustc`. These are needed because:
- LLVM IR -> object/binary uses `clang` (no `llvm-sys` / `inkwell` library dep, just textual `.ll` emission)
- Bridge crate (generated Rust shim for each project's sidecars) uses `cargo`/`rustc` to produce a `staticlib`

## 4. Code to bring over (size estimate)

From `rust-backed-compiler-for-axon/crates/`:

| Crate / file                                    | LOC    | Brought in as                              |
|-------------------------------------------------|--------|--------------------------------------------|
| `axon-frontend/src/parser/*.rs`                 | ~3000  | `src/compiler/syntax/parser_full/*.rs`     |
| `axon-frontend/src/resolver/*.rs`               | ~1500  | `src/compiler/semantics/resolver/*.rs`     |
| `axon-frontend/src/loader.rs`                   | ~700   | `src/compiler/proj/loader.rs`              |
| `axon-frontend/src/pipeline.rs`                 | ~400   | `src/compiler/syntax/pipeline.rs`          |
| `axon-typecheck/src/{checker,infer,unify,ops,types,env,ownership,diagnostics}.rs` | ~7500 | `src/compiler/semantics/typecheck/*.rs` |
| `axon-mir/src/{lower,mir,types}.rs`             | ~9500  | `src/compiler/mir/*.rs`                    |
| `axon-codegen/src/codegen.rs` (LLVM IR emit)    | ~6000  | `src/compiler/backend/llvm/codegen.rs`     |
| `axon-codegen/src/bridge_gen.rs`                | ~1800  | `src/compiler/backend/bridge_gen.rs`       |
| `axon-codegen/src/rust_compile.rs`              | ~520   | `src/compiler/backend/rust_compile.rs`     |
| `axon-codegen/src/linker.rs`                    | ~300   | `src/compiler/backend/linker.rs`           |
| `axon-codegen/src/test_harness.rs`              | ~1000  | `src/compiler/test_harness/runner.rs`      |
| `axon-codegen/src/{artifacts,prepare,graph,target_resolution,cache,call_resolution,type_marshall,rustc_diagnostics}.rs` | ~3500 | `src/compiler/backend/*.rs` |
| `axon-types/**` (shared types)                  | ~3000  | `src/compiler/types/*.rs` (shared bedrock) |
| `axon-runtime/**` (test summary types)          | ~600   | `src/compiler/runtime/*.rs`                |

**Total**: ~40k LOC of Rust ported (intentional duplication; we accept this per user instruction "no removal").

## 5. Implementation phases

Each phase is independently buildable & testable. Each phase keeps the existing `cargo run -p axon -- build` (host bootstrap) green while enabling more capability in the self-built binary.

### Phase 0 — This plan + scaffolding

- Add `docs/self-host-build-plan.md` (this document).
- Add `docs/self-host-progress.md` for ongoing tracking.
- Create empty module dirs:
  - `src/compiler/syntax/parser_full/`
  - `src/compiler/semantics/typecheck/`
  - `src/compiler/semantics/resolver/`
  - `src/compiler/mir/`
  - `src/compiler/backend/llvm/`
  - `src/compiler/test_harness/`
  - `src/compiler/types/`
  - `src/compiler/runtime/`
- No behavior change.

### Phase 1 — `rust_deps` baseline

Update `build.ax` `rust_deps` to add what the ported sidecars need:

```
rust_deps
    tracing = "0.1"
    tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
    clap = { version = "4", features = ["derive"] }
    serde = { version = "1", features = ["derive"] }      # AST/MIR JSON across sidecar boundaries
    serde_json = "1"
    tempfile = "3"                                          # build temp dirs
    which = "7"                                             # locate clang/cargo on PATH
```

(No `inkwell`/`llvm-sys` — we emit LLVM textual IR via `format!`.)

Validation: `cargo run -p axon -- build` still succeeds (deps present but unused at this stage).

### Phase 2 — Shared types (`axon-types` clone)

Copy `axon-types/src/{token,span,symbol,file_id_map,ast/*,module,foreign,diagnostics/*,builtins,semantics,test_helpers}.rs` into `src/compiler/types/`. Adjust `mod` declarations so they live in a single sidecar tree (no separate Cargo crate).

Sidecar entry: none (pure type definitions).

Validation: `cargo run -p axon -- build` still succeeds; bridge generator picks up new `.rs` files without errors.

### Phase 3 — Loader + Parser (full AST)

Bring over:
- `axon-frontend/src/lexer/*.rs` → `src/compiler/syntax/lexer_full/*.rs` (full lexer with token kinds)
- `axon-frontend/src/parser/*.rs` → `src/compiler/syntax/parser_full/*.rs`
- `axon-frontend/src/loader.rs` → `src/compiler/proj/loader.rs` (module graph discovery)
- `axon-frontend/src/build.rs` → `src/compiler/proj/build_file_load.rs`

Sidecar entries (`#[axon_pub_export]`):
- `parse_module_to_json(source: &str, file_id: u32) -> String`
- `load_project_module_graph_json(root: &str) -> String`

Wire into `pipeline_check.ax`:
- `run_parse_check` (already exists in `parser.rs`) keeps its delimiter check; **add** a new `run_parse_full_check` that calls `parse_module_to_json` and inspects diagnostics.

Validation:
- `./target/debug/axon check` still emits zero errors after rebuild.
- Existing parser unit tests in `parser.test.ax` still pass.
- New fixture: `tests/axon-cli/fixtures/parse_real_func/` containing a small valid module — confirm `parse_module_to_json` returns a non-error JSON.

### Phase 4 — Semantics (Resolver + Typecheck)

Bring over:
- `axon-frontend/src/resolver/*.rs` → `src/compiler/semantics/resolver/*.rs`
- `axon-frontend/src/semantics.rs` → `src/compiler/semantics/semantics_full.rs`
- `axon-frontend/src/pipeline.rs` → `src/compiler/syntax/pipeline.rs` (`check_source` entry)
- `axon-typecheck/src/*.rs` → `src/compiler/semantics/typecheck/*.rs`

Sidecar entries:
- `check_source_full(source: &str, file_id: u32) -> String` (returns diagnostics JSON)
- `check_project_full(root: &str) -> String`

Wire into `pipeline_check.ax`:
- Replace the call to existing `run_semantic_project_check` (snippet-based) with the new full project check. Keep the snippet variant for unit tests.

Validation:
- `axon check` continues to exit 0 with zero E-errors (the E0006 fix already applied stays in effect via the ported checker).
- `src/compiler/semantics/project_parity.test.ax` and `check.test.ax` pass.
- New regression: each fixture in `tests/axon-cli/fixtures/project_*/` returns the expected pass/fail outcome through the new sidecar.

### Phase 5 — MIR lowering

Bring over `axon-mir/src/*.rs` → `src/compiler/mir/*.rs`. This includes the recently-added non-enum match support.

Sidecar entry:
- `lower_project_to_mir(root: &str) -> String` (returns serialized MIR or error message)

Validation:
- Lower a hello-world fixture and inspect MIR for `func main() void { Return Unit }` shape.
- All `axon-mir` unit tests' equivalents pass when invoked through the sidecar (port the test bodies as `*.test.ax` integration tests against the sidecar).

### Phase 6 — LLVM IR codegen

Bring over `axon-codegen/src/codegen.rs` + supporting modules (`type_marshall.rs`, `call_resolution.rs`, `cache.rs`, `graph.rs`, `prepare.rs`, `artifacts.rs`) → `src/compiler/backend/llvm/`.

The LLVM emission uses Rust `format!` to produce textual `.ll` files; no LLVM C library dep.

Sidecar entries:
- `emit_llvm_ir(mir_json: &str, out_dir: &str) -> String` (writes `.ll` files; returns paths or error)

Validation:
- Emit `.ll` for hello-world fixture; run `clang hello.ll -o hello && ./hello` and confirm expected stdout.

### Phase 7 — Bridge generation + Rust sidecar compile

Bring over:
- `axon-codegen/src/bridge_gen.rs` → `src/compiler/backend/bridge_gen.rs`
- `axon-codegen/src/rust_compile.rs` → `src/compiler/backend/rust_compile.rs`

These generate the bridge crate (`Cargo.toml` + wrapped `lib.rs`) for project sidecars and invoke `cargo build` to produce the bridge `staticlib`.

Sidecar entry:
- `build_bridge_static(project_root: &str, deps_raw: &str, sidecar_files: &str) -> String` (returns staticlib path or error)

Validation:
- A fixture project with one `.rs` sidecar exporting `#[axon_pub_export]` functions builds the bridge; the resulting staticlib contains the expected symbols (`nm` check).

### Phase 8 — Linker + final artifact

Bring over:
- `axon-codegen/src/linker.rs` → `src/compiler/backend/linker.rs`

Sidecar entry:
- `link_executable(ll_paths: &str, bridge_staticlib: &str, out_bin: &str) -> String`

Reimplement `src/compiler/backend/backend.rs::run_lowered_to_artifact` to:
1. Call `lower_project_to_mir`
2. Call `emit_llvm_ir`
3. Call `build_bridge_static` (when sidecars present)
4. Call `link_executable`
5. Publish to `target/build/<bin>/<bin>`

**Crucially: remove the `cargo build -p axon` and host-binary invocation from `backend.rs`.** The new path has no reference to `rust-backed-compiler-for-axon/` at runtime.

Validation:
- `./target/debug/axon build` on `tests/axon-cli/fixtures/project_ok/` produces a runnable binary — and the binary executes correctly without `rust-backed-compiler-for-axon/target/` even existing (rename or chmod-block it during the test).
- `./target/debug/axon build` from the repo root produces a fresh `target/build/axon/axon` binary.

### Phase 9 — Test harness

Bring over `axon-codegen/src/test_harness.rs` + `axon-runtime/src/{tests,error,project}.rs` → `src/compiler/test_harness/` + `src/compiler/runtime/`.

Sidecar entry:
- `run_native_tests(project_root: &str, filter: &str) -> String` (returns serialized `TestRunSummary`)

Wire into `compiler/entry.ax::run_tests`:
- Replace current `run_project_tests` (counting-only stub) with a delegation to `run_native_tests`.

Validation:
- `./target/debug/axon test` runs all `.test.ax` files in a fixture project, reports correct pass/fail counts.
- Same counts as `cargo run -p axon -- test` produced.

### Phase 10 — Self-bootstrap cutover

Validation milestones:

1. **Stage1 build**: `cd rust-backed-compiler-for-axon && cargo run -p axon -- build` produces `target/debug/axon` (host bootstrap, unchanged).
2. **Stage2 build (self-host)**: `./target/debug/axon build` from repo root, with `rust-backed-compiler-for-axon/` directory **renamed** during the run, produces `target/build/axon/axon`. This proves zero runtime dependency on the host.
3. **Stage3 build (self-rebuild from stage2)**: `./target/build/axon/axon build` produces a fresh artifact identical (modulo timestamps) to stage2's output.
4. All existing `tests/axon-cli/cli.rs` integration tests pass.
5. `./target/debug/axon test` runs the full project test suite and reports green.

### Phase 11 — Cleanup + docs

- Update `AGENTS.md` to document:
  - `rust-backed-compiler-for-axon/` is **bootstrap only**; the self-built binary at `target/debug/axon` is fully self-hosting.
  - External tools required at runtime: `clang`, `cargo`, `rustc`.
- Update `README.md` (if any) with the new self-host build flow.
- Add `docs/build-pipeline.md` describing each sidecar boundary and JSON format.

## 6. Risks and mitigations

| Risk                                                                 | Mitigation                                                                                              |
|----------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|
| 40k LOC duplication causes drift between host and self-host          | Each phase commit references the source file & SHA at copy time; tests guard against semantic drift.    |
| Bridge gen complexity (1.8k LOC) harder to embed than expected       | Bring it over verbatim; only modify it if a real bug surfaces. Don't redesign during port.              |
| Self-built compiler needs `cargo`/`rustc` to compile its own bridges | Documented as a build-time tool dependency. Same as today's host (which also uses cargo).               |
| `axon-types` is a shared crate with internal `pub` visibility quirks | Port as a single-folder module under `src/compiler/types/`; adjust `mod` paths but not visibility logic.|
| LLVM textual IR may differ across clang versions                     | Pin to a minimum supported clang (e.g. 14+); add a `which clang && clang --version` preflight check.    |
| Test harness needs cargo to compile per-project test driver          | Same toolchain assumption as `build`. Acceptable.                                                       |
| Phase 5/6 (MIR + LLVM) are the longest porting blocks                | Sub-divide: do MIR first end-to-end on a hello-world fixture; only then port LLVM emission.             |

## 7. Success criteria

- [ ] `./target/debug/axon build` produces a working native binary for any valid Axon project, with `rust-backed-compiler-for-axon/target/` deleted/renamed at runtime.
- [ ] `./target/debug/axon test` runs and reports test results (pass/fail counts) for any project.
- [ ] `./target/debug/axon build` rebuilds itself successfully (self-bootstrap loop closes).
- [ ] Existing CLI integration tests in `rust-backed-compiler-for-axon/tests/axon-cli/cli.rs` pass when invoking the self-built binary.
- [ ] `rust-backed-compiler-for-axon/` is unchanged. Confirmed by `git diff rust-backed-compiler-for-axon/` after the work.

## 8. Out of scope (future work)

- Replacing Rust sidecars with Axon-native code (full Axon-language MIR/codegen). This plan is FFI-based — Rust does the heavy lifting via sidecars. Pure-Axon codegen is a follow-on project.
- Removing the `cargo` runtime dependency (would require a built-in `rustc` invocation or an alternative toolchain).
- Removing the `clang` runtime dependency (would require linking via system `ld` directly, plus an integrated assembler — significant additional work).

## 9. Phase completion checklist (for tracking)

| Phase | Description                              | Status   |
|-------|------------------------------------------|----------|
| 0     | Plan + scaffolding                       | proposed |
| 1     | `rust_deps` baseline                     | -        |
| 2     | Shared types (`axon-types` clone)        | -        |
| 3     | Loader + Parser                          | -        |
| 4     | Resolver + Typecheck                     | -        |
| 5     | MIR lowering                             | -        |
| 6     | LLVM IR codegen                          | -        |
| 7     | Bridge gen + Rust sidecar compile        | -        |
| 8     | Linker + final artifact (cutover point)  | -        |
| 9     | Test harness                             | -        |
| 10    | Self-bootstrap cutover                   | -        |
| 11    | Cleanup + docs                           | -        |
