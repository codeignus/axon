# Zero-Cargo Final Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan **supersedes** the post-Phase-9 work in `docs/superpowers/plans/2026-04-30-axon-complete-migration.md` (Phases 10–12) and **inherits** the acceptance contract from `docs/superpowers/plans/2026-04-30-axon-self-hosting-cutover.md` (Tasks 13–15 + Final Acceptance Criteria + Non-Negotiable Completion Rules).

**Goal:** Make the repo-root Axon project the **only** thing that lives in version control: zero source-controlled `Cargo.toml`, zero `native-rust/`, zero `depreciating-soon-compiler-do-not-rename/`, zero second compiler workspace. All compiler logic lives in `src/**/*.ax`. All Rust survives **only** as `*.rs` sidecars under `src/` invoked through Axon's sidecar FFI surface, compiled by the **Axon build system itself** through the generated (and gitignored) `target/cache/app/rust/bridge/Cargo.toml`.

**Architecture (final):**

- `build.ax` (project manifest) and `src/**/*.ax` are the compiler.
- `src/**/*.rs` files are sidecars: each one lives **next to** the `.ax` it serves and exposes a narrow FFI (file/process/permissions/network/LLVM/object-emit/linker/foreign-archive). No sidecar contains compiler policy.
- The Axon build system generates `target/cache/app/rust/bridge/Cargo.toml` at build time. That file is **build output**, not source, and stays under `target/` (gitignored).
- Bootstrap is **external** (out of this repo): either a previously produced `axon` binary or a one-time external manifest pointed to by `AXON_BOOTSTRAP_MANIFEST`. The repo never contains a Cargo manifest.
- `verify-self-hosting-cutover.sh` succeeds in a checkout that contains **no** `Cargo.toml` files anywhere except the generated bridge in `target/`.

**Tech Stack:** Axon (`*.ax`), Rust sidecars (`*.rs`) compiled through the Axon-generated bridge, LLVM 21 + `inkwell` linked through one sidecar, shell verification scripts, Bash + `rg` for assertions. No standalone Cargo workspace.

---

## Predecessor plans and starting state

Two plans precede this one. Both must be considered binding.

### `2026-04-30-axon-self-hosting-cutover.md` — the correctness contract

That plan defines:

- **Blockers A–D** (active-path conditions that must be torn down):
  - **A** — backend invokes a host compiler workspace via `cargo`.
  - **B** — `lower_project` is a marker / file-count writer, not real MIR.
  - **C** — parser/semantics use string-scanning instead of full AST/typed IR.
  - **D** — verification scripts still allow the deprecated tree.
- **Non-Negotiable Completion Rules** (carried forward verbatim):
  - The final task **deletes** the legacy tree.
  - Verification runs **before and after** deletion.
  - **No** task counts as complete if it only adds string encodings, wrappers, metadata, scaffolding, or tests that prove existence rather than behavior.
  - **No** compatibility / fallback path may keep the old compiler alive.
  - **No** script may search for `depreciating-soon-compiler-do-not-rename/Cargo.toml` after the cutover gate begins.
  - `backend.rs` must not invoke `cargo` against another compiler workspace.
  - `lower_project` must not be a marker writer.
  - Rust sidecars do not decide compiler semantics.

**This plan inherits all of the above.** Where the cutover plan says "delete the legacy tree", this plan extends that to "delete every Cargo manifest in the tree."

### `2026-04-30-axon-complete-migration.md` — the phased bring-up

That plan splits the cutover into Phases 0–12. It is the source of truth for which migration sub-tasks are already merged.

**Status from that plan (as of this writing):**

| Phase | Title | Status |
|---|---|---|
| 0 | Migration Foundations (parity scripts, sidecar skeletons, source-map.md) | ✅ done |
| 1 | Token stream owned by Axon | ✅ done |
| 2 | AST + parser owned by Axon | ✅ done **for migration scope** (expression precedence / f-string / full match AST listed as "stretch") |
| 3 | Project graph + build manifest | ✅ done **for migration scope** (3a–3e all done) |
| 4 | Resolver, visibility, imports | ✅ done **for migration scope** (4a–4d all done) |
| Pre-5 gate | Reference type-system refinement | ✅ landed in the reference tree |
| 5 | Typechecker, inference, lints | ✅ done **for migration scope** |
| 6 | Ownership, cleanup, branch reconciliation | ✅ done **for migration scope** |
| 7 | MIR + lowering (real `lower_project`) | ✅ done **for the v3 envelope**; reference `v4` MIR export ships in `axon-native-build export-mir` (still subprocess) |
| 8 | Native codegen boundary (LLVM / linker / foreign archives) | ⚠️ **policy** done in `policy.ax`; **runtime** still routed through the `axon-native-build` driver subprocess |
| 9 | Test command semantics, fixtures, diagnostics renderer | ⚠️ **partial** — Axon-side orchestration in `test_orchestrate.ax`; full renderer parity + colocated private-symbol rules + runtime-scoped integration execution still tracked as follow-ups |
| 10 | Pre-delete quarantine verification | ⚠️ **partial** — `assert-no-legacy-compiler-refs.sh`, `verify-no-legacy-before-delete.sh`, `AXON_PHASE10_QUARANTINE` are in place; **full quarantine acceptance** is gated on driver removal |
| 11 | Delete legacy tree and cascading references | ❌ blocked by Phase 8 driver removal |
| 12 | Post-delete verification | ❌ blocked by Phase 11 |

**Important rename:** the directory historically called `depreciating-soon-compiler-do-not-rename/` was renamed to `native-rust/` and moved into the tree. It is the same code (the seven reference crates `axon-{frontend,types,typecheck,mir,codegen,runtime}` plus `axon-types`). For the rest of this plan, **`native-rust/` and `depreciating-soon-compiler-do-not-rename/` refer to the same thing**; both names must be eliminated.

### Blocker E (added by this plan)

- **E** — the tree contains source-controlled `Cargo.toml` (`src/Cargo.toml`) and a Rust workspace (`native-rust/`). Both must be deleted; bootstrap moves out-of-tree.

This blocker **only** falls when:

- `git ls-files | grep Cargo.toml` is empty.
- `native-rust/`, `depreciating-soon-compiler-do-not-rename/`, `bootstrap-compiler/`, `rust-backed-compiler-for-axon/`, and `rust-self-compiler-for-axon/` are not tracked.
- `axon build`, `axon run`, `axon test` all work from a checkout that has none of the above.

---

## End-state inventory (ground truth)

This is the contract. Phase A through Phase F end here.

```text
Repo root (committed):
├── build.ax                                  # only manifest in the tree
├── AGENTS.md
├── README.md, LICENSE, …
├── src/
│   ├── main.ax
│   ├── clap.rs                               # sidecar
│   ├── tracing.rs                            # sidecar
│   ├── sidecar.rs                            # sidecar
│   └── compiler/
│       ├── **/*.ax                           # ALL compiler policy
│       └── **/*.rs                           # ONLY narrow sidecars
├── tests/                                    # *.ax fixtures
├── scripts/*.sh
├── docs/

Build output (NOT committed; under .gitignore):
└── target/
    ├── cache/app/rust/bridge/Cargo.toml      # generated by axon build
    ├── cache/app/rust/bridge/src/lib.rs      # generated bridge shim
    └── build/axon/axon                       # compiled compiler binary
```

**Hard invariants verified by `scripts/assert-zero-cargo.sh`:**

- `git ls-files | grep -E '(^|/)Cargo\.toml$'` → empty.
- `git ls-files | grep -E '(^|/)Cargo\.lock$'` → empty.
- No directory named `native-rust/`, `depreciating-soon-compiler-do-not-rename/`, `bootstrap-compiler/`, `rust-backed-compiler-for-axon/`, or `rust-self-compiler-for-axon/` is tracked.
- `rg -l 'axon-codegen|axon-mir|axon-typecheck|axon-frontend|axon-runtime|axon-types' src/ scripts/ build.ax` → empty (these names belong only to historical docs).
- `rg -l 'depreciating-soon-compiler-do-not-rename' src/ scripts/ build.ax` → empty.

Anything not in this list is wrong.

---

## Pre-work: snapshot the reference and freeze the migration mine

Before deletion, the reference Rust workspace `native-rust/` must be **exhaustively mapped** so every behavior in it lands either in `.ax` or in a narrow sidecar.

**Files:**

- Modify: `docs/migration/source-map.md` — every reference `*.rs` file mapped to (a) the Axon module that owns its **policy** and (b) the sidecar that owns its **FFI surface** (or `none` when neither is needed).
- Create: `docs/migration/sidecar-allowlist.md` — the closed list of sidecars allowed in the final tree, with the FFI signature for each.

**Tasks:**

- [ ] **Step 1: enumerate reference files**

```bash
ls native-rust/crates/*/src/**/*.rs > /tmp/ref-files.txt
wc -l /tmp/ref-files.txt
```

Expected: ~83 files.

- [ ] **Step 2: classify each file in the source map**

For each row in `/tmp/ref-files.txt`, write a `docs/migration/source-map.md` row:

```markdown
| reference file | policy lands in `*.ax` | FFI sidecar | status |
|---|---|---|---|
| `axon-frontend/src/lexer/mod.rs` | `src/compiler/syntax/lexer.ax` | `src/compiler/syntax/lexer.rs` (file read only) | DONE |
| `axon-codegen/src/codegen.rs` | n/a (LLVM IR construction is intrinsically FFI) | `src/compiler/backend/native_codegen.rs` | OPEN |
| `axon-codegen/src/linker.rs` | `src/compiler/backend/link.ax` | `src/compiler/backend/backend.rs` | PARTIAL |
…
```

Every row labelled `OPEN` or `PARTIAL` becomes a port subtask in Phase A.

- [ ] **Step 3: lock the sidecar allowlist**

Write `docs/migration/sidecar-allowlist.md`. The final tree may contain only these sidecars:

```text
src/clap.rs                        — CLI parse (clap)
src/tracing.rs                     — tracing/log boundary
src/sidecar.rs                     — sidecar runtime helpers
src/main.rs                        — none (Axon main.ax)
src/compiler/syntax/lexer.rs       — read file bytes; no lexing
src/compiler/syntax/parser.rs      — read file bytes; no parsing
src/compiler/proj/discover.rs      — directory list / canonicalize / exists
src/compiler/proj/targets.rs       — target-resolution FS ops only
src/compiler/semantics/semantics.rs — project FS walk only
src/compiler/semantics/ownership.rs — snippet runner only
src/compiler/diagnostics/diagnostics.rs — color/tty detection only
src/compiler/ir/ir.rs              — JSON encode helpers (no policy)
src/compiler/backend/backend.rs    — process exec / chmod / atomic rename / preserve
src/compiler/backend/native_codegen.rs — LLVM IR + object emit (inkwell)
src/compiler/backend/foreign_archive.rs — cargo/go subprocess for user rust_deps/go_deps
src/compiler/backend/toolchain.rs  — probe rustc/cc/go/llvm-config/clang
src/compiler/pipeline_check/test_fmt.rs — render diagnostic line (color only)
src/mcp/mcp.rs                     — MCP server transport
```

Anything not on this list must be deleted before Phase F.

- [ ] **Step 4: commit**

```bash
git add docs/migration/source-map.md docs/migration/sidecar-allowlist.md
git commit -m "docs(migration): freeze sidecar allowlist + reference port map"
```

---

## Phase A — Port the remaining compiler policy out of `native-rust/`

**Goal:** Every behavior in `native-rust/crates/*` either lands in an `.ax` module or in one of the allowlisted sidecars. After this phase, nothing in `src/` calls into `axon_codegen::*`, `axon_mir::*`, `axon_typecheck::*`, `axon_frontend::*`, `axon_runtime::*`, or `axon_types::*` as Rust APIs.

**Inherited from `2026-04-30-axon-complete-migration.md`:** Phases 1–7 are already complete-for-migration-scope (lexer, AST/parser, project graph, resolver, typechecker, ownership, MIR `v3` envelope). The starting point of this phase is therefore **the reference-parity stretch items + Phase 8 driver replacement + Phase 9 follow-ups** that were explicitly deferred in the previous plan. Cite the previous plan's "complete for migration scope" notes when scoping each subphase.

This phase is large; it expands into one PR per reference crate. Each PR:

1. Picks one reference crate (or one stretch item carried forward from the previous plan).
2. Splits it into `*.ax` policy + narrow sidecar FFI per the allowlist.
3. Adds Axon-side tests under `src/**/*.test.ax` covering behavior parity with the reference fixture set.
4. Removes the corresponding `extern crate` / `use axon_*::…` references from `src/` once the Axon path is the only caller.

The PR sequence follows the migration plan's order; each subphase below is a **delta** over what's already merged:

### A.1 — `axon-frontend` residue (parser stretch items + lint completion)

> **Already merged in migration plan Phases 1, 2, 3, 4:** lexer (full), AST + parser for shipped language surface, project graph manifest parsing, resolver + import semantics. Pick up only the items the previous plan flagged as **stretch / open**.

**Files:**

- Modify: `src/compiler/syntax/{parser,ast}.ax` — close the **stretch** gaps the migration plan called out: expression precedence parity (full operator table), f-string interpolation parsing, full `match` AST. Cite reference fixtures `native-rust/crates/axon-frontend/src/parser/{expr,pattern}.rs::tests` row-by-row.
- Modify: `src/compiler/syntax/parser.rs` — keep file IO + `validate_delimiters_char_scan` only; delete any token classification that lingers.
- Modify: `src/compiler/proj/build_file.ax` — finish residual `axon-frontend/src/build.rs` edge cases (multi-line `rust_deps` continuations, Cargo-ish quoting). Migration plan 3b/3c covered the common cases.
- Modify: `src/compiler/semantics/{lint,resolve}.ax` — finish lints flagged "still open" in migration Phase 5: unused locals + `#[allow]`-style suppression. Cite `native-rust/crates/axon-frontend/src/lint.rs`.

**Tasks:**

- [ ] **Step 1**: inventory open rows in `docs/migration/source-map.md` for `axon-frontend`. For each `OPEN` row:
  1. Read the reference file with the Read tool.
  2. Decide where the policy goes (cite the row in the commit).
  3. Write a failing `*.test.ax` fixture mirroring the reference test (or import the fixture directly under `tests/axon-frontend/fixtures/<name>/`).
  4. Implement the Axon code; trim any sidecar Rust to file IO.
  5. Mark the row `DONE`.
  6. Commit per file/group.

- [ ] **Step 2**: prove no caller links `axon_frontend::*`:

```bash
rg 'axon_frontend' src/
```

Expected: empty.

### A.2 — `axon-types` (token / span / ast / symbol / diagnostics / foreign / builtins)

**Files:**

- Modify: `src/compiler/syntax/{token,ast}.ax`, `src/compiler/diagnostics/diagnostic.ax`, `src/compiler/semantics/{resolve,types}.ax`, `src/compiler/backend/ffi.ax`.

**Tasks:**

- [ ] **Step 1**: port `axon-types/src/{token,span,symbol,foreign,builtins,module,file_id_map,semantics,test_helpers}.rs` behavior. Most of these are pure data plus light helpers — translate to `.ax` records and helpers; `*.test.ax` mirrors `axon-types/src/diagnostics/types.rs::tests`, `renderer.rs::tests`, etc.
- [ ] **Step 2**: port `axon-types/src/diagnostics/{renderer,constructors,sink,suppression,codes,color,types,mod}.rs`:
  - Severity / code catalog / suppression policy → `src/compiler/diagnostics/diagnostic.ax`.
  - Renderer output (text, ANSI) → `.ax`; only the ANSI **probe** (is-tty, NO_COLOR env) stays in `src/compiler/diagnostics/diagnostics.rs`.
- [ ] **Step 3**: port `axon-types/src/ast/*.rs` — node accessors, kind constants, child-walk helpers — all into `src/compiler/syntax/ast.ax`.
- [ ] **Step 4**: prove no caller links `axon_types::*`:

```bash
rg 'axon_types' src/
```

Expected: empty.

### A.3 — `axon-typecheck` (checker/infer/unify/types/ops/env/ownership)

**Files:**

- Modify: `src/compiler/semantics/{types,typecheck,check,ownership}.ax`, `src/compiler/diagnostics/diagnostic.ax`.

**Tasks:**

- [ ] **Step 1**: port `axon-typecheck/src/{types,unify,ops,env}.rs` to `src/compiler/semantics/types.ax` (record-shape model + unify + ops). Existing `types.test.ax` must continue to pass; add fixtures from `axon-typecheck/tests`.
- [ ] **Step 2**: port `axon-typecheck/src/{checker,infer,diagnostics}.rs` to `src/compiler/semantics/typecheck.ax` (full project typecheck without snippet mode).
- [ ] **Step 3**: port `axon-typecheck/src/ownership.rs` into `src/compiler/semantics/ownership.ax`. The sidecar `ownership.rs` is reduced to a one-shot snippet runner used only by tests.
- [ ] **Step 4**: prove no caller links `axon_typecheck::*`:

```bash
rg 'axon_typecheck' src/
```

Expected: empty.

### A.4 — `axon-mir` (mir/types/lower)

> **Already merged in migration plan Phase 7:** `lower_project` is Axon-owned (`v3` envelope); reference-driven `v4` MIR export ships in `axon-native-build export-mir`. The remaining work is to make the **Axon path** emit per-module typed MIR (so the `v4` driver call goes away in Phase B) and bump the envelope to `v5`.

**Files:**

- Modify: `src/compiler/ir/{ir,lower,lower_project}.ax`, `src/compiler/ir/ir.rs`.

**Tasks:**

- [ ] **Step 1**: port `native-rust/crates/axon-mir/src/{mir,types}.rs` into `src/compiler/ir/ir.ax` (typed MIR data model). Today `ir.ax` carries constants/helpers only.
- [ ] **Step 2**: port `native-rust/crates/axon-mir/src/lower.rs` (~4000 lines) into `src/compiler/ir/lower.ax`. Split into per-construct files only if a single file exceeds ~2000 lines; otherwise keep one file. Emit `ok:lowered:v5:` envelope (bumped from `v4`) with per-module typed MIR — the same payload `prepare::export_mir_debug_bundle_for_lowered_ir` produces today.
- [ ] **Step 3**: shrink `src/compiler/ir/ir.rs` to JSON encode/decode helpers only — no lowering, no policy. This satisfies cutover-plan **Blocker B** for the active path.
- [ ] **Step 4**: delete `axon-native-build export-mir` from the driver in preparation for Phase B (no longer needed once Axon emits `v5` directly).
- [ ] **Step 5**: prove no caller links `axon_mir::*`:

```bash
rg 'axon_mir' src/
```

Expected: empty.

### A.5 — `axon-codegen` policy (graph/target_resolution/cache/call_resolution/type_marshall/artifacts/prepare/rustc_diagnostics/test_harness/json_rpc/bridge_syn)

**Files:**

- Modify: `src/compiler/proj/{module_graph,targets,command_targets,build_file}.ax`, `src/compiler/backend/{artifacts,link,ffi}.ax`, `src/compiler/pipeline_check/{pipeline_check,test_orchestrate}.ax`, `src/compiler/diagnostics/diagnostic.ax`, `src/compiler/syntax/parser.ax`.

**Tasks:**

- [ ] **Step 1**: port `axon-codegen/src/graph.rs` → `src/compiler/proj/module_graph.ax`. Tests in `module_graph.test.ax`.
- [ ] **Step 2**: port `axon-codegen/src/target_resolution.rs` → `src/compiler/proj/targets.ax` + `command_targets.ax`. Drop `targets.rs` policy; sidecar keeps only canonicalize/exists FS calls.
- [ ] **Step 3**: port `axon-codegen/src/cache.rs` → `src/compiler/backend/artifacts.ax` (cache key shape + invalidation). The sidecar that hashes file bytes stays in `backend.rs`.
- [ ] **Step 4**: port `axon-codegen/src/call_resolution.rs` → `src/compiler/semantics/resolve.ax` (already partial).
- [ ] **Step 5**: port `axon-codegen/src/type_marshall.rs` → `src/compiler/backend/ffi.ax` + a tiny C-ABI helper inside `foreign_archive.rs` for primitive marshalling.
- [ ] **Step 6**: port `axon-codegen/src/artifacts.rs` → `src/compiler/backend/artifacts.ax` (artifact path policy + stable layout).
- [ ] **Step 7**: port `axon-codegen/src/prepare.rs` (the big one — symbol prep / monomorphization / MIR debug bundle export) → `src/compiler/ir/lower.ax` + `src/compiler/backend/link.ax`. The MIR export envelope `v5:` carries everything `prepare::export_mir_debug_bundle_for_lowered_ir` used to emit, but built in Axon directly.
- [ ] **Step 8**: port `axon-codegen/src/rustc_diagnostics.rs` (parse `rustc --error-format=json` output for the foreign archive) → `src/compiler/diagnostics/diagnostic.ax` parser; sidecar stays as the subprocess + stdout pipe owner inside `foreign_archive.rs`.
- [ ] **Step 9**: port `axon-codegen/src/test_harness.rs` → `src/compiler/pipeline_check/test_orchestrate.ax`. Test discovery, suite plan, and result aggregation live in `.ax`; only the **launch** of each test binary stays in `backend.rs`.
- [ ] **Step 10**: port `axon-codegen/src/json_rpc.rs` → `src/sidecar.rs` (transport only) + `.ax` for any policy.
- [ ] **Step 11**: port `axon-codegen/src/bridge_syn.rs` (extract `@rust` blocks via `syn`-based parsing) → `src/compiler/syntax/parser.ax` for the line scanner; the `syn`-fallback parser is kept in a tiny new sidecar `src/compiler/syntax/rust_block_extract.rs` that just calls `syn::parse_file` and returns a span list.

### A.6 — `axon-codegen` runtime layer (codegen.rs / linker.rs / bridge_gen.rs / rust_compile.rs / go_compile.rs / compile.rs)

> **Already merged in migration plan Phase 8:** `policy.ax` carries `describe_native_codegen_boundary`, `describe_link_artifact_contract`, `assert_no_second_compiler_workspace`, and `describe_phase8_migration_codegen_bridge`; `backend.rs::run_lowered_to_artifact` accepts `ok:lowered:v3:` and `ok:lowered:v4:` envelopes; `native_codegen.rs::native_emit_object_for_module` documents the inkwell/staticlib split. The driver subprocess (`axon-native-build`) is still the one calling `axon_codegen::codegen_module`. This subphase replaces that subprocess with in-process FFIs.

These are the **legitimate** sidecar parts: LLVM IR emission, linker invocation, foreign-archive build. Policy must move out; the Rust that remains must do nothing but issue OS / LLVM / process calls on requests sent from Axon.

**Files:**

- Modify: `src/compiler/backend/native_codegen.rs` — receives a JSON MIR module, emits an object file using `inkwell`. **No** decisions about what to lower; those come from `lower.ax`.
- Modify: `src/compiler/backend/backend.rs` — atomic write, chmod, link plan executor, preserve suffixed binaries, exec child processes.
- Modify: `src/compiler/backend/foreign_archive.rs` — generate Rust/Go bridge sources (using shapes Axon hands it) and call `cargo`/`go`/`rustc`. Returns a `.a` path. No policy decisions about which deps to use.
- Modify: `src/compiler/backend/link.ax` — produce link plans (object list, archives, lib search dirs, rpath). The sidecar consumes the plan literally.
- Modify: `src/compiler/backend/artifacts.ax` — own the cache shape; no Rust counterpart needed.
- Modify: `src/compiler/entry.ax` — `check`, `build`, `run`, `test` semantics.

**Tasks:**

- [ ] **Step 1**: enumerate every `pub fn` in `native-rust/crates/axon-codegen/src/codegen.rs`. Each one becomes either:
  - An Axon function in `lower.ax` (if it makes a policy decision or shapes a module),
  - A `#[axon_pub_export]` FFI in `native_codegen.rs` (if it writes IR for a single MIR primitive).

  The FFI surface is **finite and small**: `emit_module_object(json: &str) -> String` and a handful of declared-target setters. It must not expose `inkwell::*` types to Axon.

- [ ] **Step 2**: port `axon-codegen/src/compile.rs` (`check_target`, `build`, `run_tests_target`) → `src/compiler/entry.ax`. The sidecar `axon-native-build` binary disappears in Phase B; `entry.ax` calls FFIs directly.

- [ ] **Step 3**: port `axon-codegen/src/linker.rs` → `src/compiler/backend/link.ax` (plan) + `backend.rs` (executor). The plan is JSON-shaped.

- [ ] **Step 4**: port `axon-codegen/src/bridge_gen.rs` → `src/compiler/backend/ffi.ax` (which generates the Rust bridge source and `Cargo.toml` body string from `build.ax`). `foreign_archive.rs` writes the bytes to `target/cache/app/rust/bridge/` and runs `cargo`.

- [ ] **Step 5**: port `axon-codegen/src/{rust_compile,go_compile}.rs` → behavior owned by `foreign_archive.rs` only as **process drivers**; the **command shape** (which targets, which features, which cargo profile) is built in `link.ax`.

- [ ] **Step 6**: prove no caller links `axon_codegen::*`:

```bash
rg 'axon_codegen' src/
```

Expected: empty.

### A.7 — `axon-runtime` (lib/error/tests/project)

**Files:**

- Modify: `src/compiler/pipeline_check/test_orchestrate.ax`, `src/compiler/diagnostics/diagnostic.ax`, `src/compiler/proj/build_file.ax`.

**Tasks:**

- [ ] **Step 1**: port `axon-runtime/src/{tests,error,project}.rs` to `.ax` modules listed above. Test runner result shapes, error categories, project layout helpers. Anything that needed `std::process::exit` is owned by `backend.rs`.
- [ ] **Step 2**: prove no caller links `axon_runtime::*`:

```bash
rg 'axon_runtime' src/
```

Expected: empty.

### A.8 — Phase A acceptance

- [ ] **Step 1**: each port-PR is merged.
- [ ] **Step 2**: assertion:

```bash
rg 'axon[-_](codegen|mir|typecheck|frontend|runtime|types)' src/ build.ax scripts/
```

Expected: empty.

- [ ] **Step 3**: `./target/build/axon/axon check ""` and `./target/build/axon/axon test` still succeed (they use the `axon-native-build` driver subprocess — which still links those crates — until Phase B). Phase A only removes Axon-side calls; the bridge subprocess is severed in B.

- [ ] **Step 4**: commit each port-PR with `feat(migration): port <crate> policy to .ax`.

---

## Phase B — Delete the `axon-native-build` driver and `src/Cargo.toml`

After Phase A, every Axon module that used to call `axon_codegen::*` instead calls a sidecar FFI directly (e.g. `native_codegen.rs::emit_module_object`). The driver subprocess has zero remaining callers.

**Files:**

- Delete: `src/Cargo.toml`.
- Delete: `src/Cargo.lock`.
- Delete: `src/axon_native_build_bin/` (entire directory).
- Modify: `src/compiler/backend/backend.rs` — remove `ensure_native_build_driver_binary`, `run_lowered_to_artifact`'s subprocess path, and the `AXON_NATIVE_BUILD_BIN` shim. Replace with a direct call into `native_codegen.rs` FFIs that publishes the artifact via the existing atomic-rename helper.
- Modify: `src/compiler/backend/policy.ax` — drop `describe_phase8_migration_codegen_bridge`; replace with `describe_native_codegen_inproc_contract` that documents the in-process FFI surface.
- Modify: `src/compiler/backend/native_codegen.rs` — must now expose `#[axon_pub_export] fn emit_module_object(json_request: &str) -> String` and `#[axon_pub_export] fn link_artifact(json_plan: &str) -> String` and a `#[axon_pub_export] fn publish_axon_install_layout(native_path: &str) -> String` (the same publish semantics that lived in `axon_native_build_bin/main.rs`).
- Modify: `src/compiler/entry.ax` — call those FFIs through the bridge (declared via `build.ax`), not through `Command::new("axon-native-build")`.
- Modify: `build.ax` — declare `inkwell` (and any other LLVM-side rust crates) under `rust_deps` so the bridge `Cargo.toml` (generated under `target/cache/app/rust/bridge/Cargo.toml`) carries them. **Never** add `axon-codegen` here; it no longer exists as a Rust dep.

**Tasks:**

- [ ] **Step 1**: write a failing test that proves there is no `src/Cargo.toml`:

`scripts/assert-zero-cargo.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail=0
tracked_cargos="$(git ls-files | grep -E '(^|/)Cargo\.(toml|lock)$' || true)"
if [[ -n "$tracked_cargos" ]]; then
  echo "FAIL: tracked Cargo.* files:" >&2
  echo "$tracked_cargos" >&2
  fail=1
fi
forbidden_dirs=(native-rust depreciating-soon-compiler-do-not-rename bootstrap-compiler rust-backed-compiler-for-axon rust-self-compiler-for-axon)
for d in "${forbidden_dirs[@]}"; do
  if git ls-files --error-unmatch "$d" >/dev/null 2>&1; then
    echo "FAIL: tracked directory $d" >&2
    fail=1
  fi
done
if rg -q --hidden -g '!.git' -g '!target' \
   'axon[-_](codegen|mir|typecheck|frontend|runtime|types)' src/ build.ax scripts/ 2>/dev/null; then
  echo "FAIL: tree references legacy crate names outside docs/" >&2
  rg --hidden -g '!.git' -g '!target' \
     'axon[-_](codegen|mir|typecheck|frontend|runtime|types)' src/ build.ax scripts/ >&2 || true
  fail=1
fi
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
echo "PASS: zero-cargo invariants hold"
```

- [ ] **Step 2**: run it; expect FAIL:

```bash
chmod +x scripts/assert-zero-cargo.sh
./scripts/assert-zero-cargo.sh
```

Expected output: `FAIL: tracked Cargo.* files: src/Cargo.toml`.

- [ ] **Step 3**: rewrite `src/compiler/backend/native_codegen.rs` to expose the three FFIs above. Each one accepts a JSON string from Axon and returns a JSON string. The `inkwell` calls are wrapped in `catch_unwind` and a structured error envelope so panics propagate as diagnostics, not aborts. Land this as its own commit before deleting anything.

- [ ] **Step 4**: rewrite `src/compiler/backend/backend.rs::run_lowered_to_artifact` to invoke `native_codegen::emit_module_object` per module, write objects to `target/cache/objects/<hash>.o`, then call `native_codegen::link_artifact` and `native_codegen::publish_axon_install_layout`. No `cargo` is invoked from here; foreign archives go through `foreign_archive.rs`.

- [ ] **Step 5**: rewrite `src/compiler/entry.ax::run_build`, `run_check`, `run_run`, `run_tests` to drive these FFIs directly. Update `entry.test.ax` accordingly.

- [ ] **Step 6**: delete the driver:

```bash
git rm -r src/axon_native_build_bin/
git rm src/Cargo.toml
git rm src/Cargo.lock 2>/dev/null || true
```

- [ ] **Step 7**: re-run the assertion:

```bash
./scripts/assert-zero-cargo.sh
```

Expected: it now fails only on `native-rust/` (Phase C) — not on Cargo files.

- [ ] **Step 8**: build proof. Use a previously preserved binary (Phase 0 or external bootstrap):

```bash
./target/build/axon/axon_selfcompiled3 check ""
./target/build/axon/axon_selfcompiled3 build
./target/build/axon/axon_selfcompiled3 test
test -x target/build/axon/axon
./target/build/axon/axon check ""
```

Expected: every command exits 0. The new `target/build/axon/axon` was produced by the in-process FFI path; no `axon-native-build` subprocess was executed.

- [ ] **Step 9**: commit:

```bash
git add -A
git commit -m "feat(backend): replace axon-native-build subprocess with in-process FFI; delete src/Cargo.toml"
```

---

## Phase C — Delete `native-rust/` and break the bootstrap dependency

After Phase B, nothing in `src/` calls `axon_codegen::*` (Rust types) directly. Yet `target/build/axon/axon` was originally produced by linking those crates. To break this, the bootstrap must come from outside the repo or a previously produced binary; the in-tree `native-rust/` is then redundant.

**Note on naming:** `native-rust/` is the renamed `depreciating-soon-compiler-do-not-rename/`. The cutover plan's Task 14 ("Delete Legacy Compiler Tree And Cascading References") and the migration plan's Phase 11 are both fulfilled by deleting `native-rust/` here. Both legacy names — and any future renames of the same content — are forbidden post-Phase-C.

**Files:**

- Delete: `native-rust/`.
- Modify: `.gitignore` — keep entries for legacy directories so old local trees do not pollute `git status`.
- Modify: `scripts/verify-self-bootstrap.sh` and `scripts/verify-self-hosting-cutover.sh` — drop `depreciating-soon-compiler-do-not-rename/Cargo.toml` and any `native-rust/Cargo.toml` from the manifest search; require `AXON_BOOTSTRAP_MANIFEST` or a pre-built `target/build/axon/axon`.
- Modify: `scripts/assert-no-legacy-compiler-refs.sh` — add `native-rust` to the legacy term list. Keep allowlist tight: only `docs/`, `AGENTS.md`, and the script itself may mention any of these strings.
- Delete: `scripts/verify-no-legacy-before-delete.sh` (its quarantine semantics are absorbed by `assert-zero-cargo.sh`).
- Modify: `AGENTS.md` — drop bootstrap references to `native-rust/` and `depreciating-soon-compiler-do-not-rename/`. Replace with the `AXON_BOOTSTRAP_MANIFEST` policy and the `target/build/axon/axon` reuse policy.
- Modify: `docs/superpowers/plans/2026-04-30-axon-self-hosting-cutover.md` — append a closeout note pointing at this plan.
- Modify: `docs/superpowers/plans/2026-04-30-axon-complete-migration.md` — close Phases 10–12 with a back-pointer to this plan.

**Tasks:**

- [ ] **Step 1**: confirm no remaining caller. From a clean checkout:

```bash
rg --hidden -g '!.git' -g '!target' \
   'native-rust|depreciating-soon-compiler-do-not-rename' src/ scripts/ build.ax
```

Expected: empty.

- [ ] **Step 2**: rewrite `scripts/verify-self-bootstrap.sh`. Replace the `for c in …; do` block with:

```bash
MANIFEST="${AXON_BOOTSTRAP_MANIFEST:-}"
PREBUILT="${AXON_PREBUILT_BIN:-}"
if [[ -z "$MANIFEST" && -z "$PREBUILT" ]]; then
  echo "error: provide AXON_BOOTSTRAP_MANIFEST=/path/to/external/Cargo.toml" >&2
  echo "       or AXON_PREBUILT_BIN=/path/to/axon (e.g. a release artifact)." >&2
  echo "       This repo has zero Cargo manifests on purpose." >&2
  exit 1
fi
```

The rest of the script is unchanged (stages 1–3 use the produced binary).

- [ ] **Step 3**: rewrite `scripts/verify-self-hosting-cutover.sh` the same way; drop the optional `AXON_PHASE10_QUARANTINE` block.

- [ ] **Step 4**: delete `native-rust/`:

```bash
git rm -r native-rust
```

- [ ] **Step 5**: update `assert-no-legacy-compiler-refs.sh`:

```bash
LEGACY_TERMS=(
  'depreciating-soon-compiler-do-not-rename'
  'native-rust'
  'rust-backed-compiler-for-axon'
  'rust-self-compiler-for-axon'
  'bootstrap-compiler'
)
```

For each term, run the same allowlist check the existing script already implements. Only `docs/`, `AGENTS.md`, `.gitignore`, and this script may match.

- [ ] **Step 6**: delete `scripts/verify-no-legacy-before-delete.sh`:

```bash
git rm scripts/verify-no-legacy-before-delete.sh
```

- [ ] **Step 7**: update `AGENTS.md`. Drop the entire "Stage 0 — reference workspace" / `depreciating-soon-compiler-do-not-rename` / `bootstrap-compiler` paragraph and replace with:

```markdown
## Bootstrap policy

This repo contains **zero** Cargo manifests. It is not a Cargo workspace. The Axon
build system generates `target/cache/app/rust/bridge/Cargo.toml` at build time
(gitignored) to compile sidecars + user `rust_deps`.

To build the compiler from scratch you need exactly one of:

- `AXON_PREBUILT_BIN=/path/to/axon` — any previously produced `axon` binary
  (e.g. a release artifact or a `target/build/axon/axon_*` from a different
  checkout). Then `./target/build/axon/axon build` from this repo onward.
- `AXON_BOOTSTRAP_MANIFEST=/path/to/external/Cargo.toml` — a manifest
  **outside** this repo that produces an `axon` binary by other means
  (e.g. a separate, archived bootstrap repo). Used once.

`scripts/verify-self-bootstrap.sh` and `scripts/verify-self-hosting-cutover.sh`
require one of those env vars; they never search this repo for a Cargo file.
```

- [ ] **Step 8**: run the assertion suite:

```bash
./scripts/assert-zero-cargo.sh
./scripts/assert-no-legacy-compiler-refs.sh
```

Expected: both PASS.

- [ ] **Step 9**: commit:

```bash
git add -A
git commit -m "feat(migration): delete native-rust; bootstrap is external only"
```

---

## Phase D — In-tree sidecar discovery owns the bridge `Cargo.toml`

The Axon build system already generates `target/cache/app/rust/bridge/Cargo.toml` to compile user sidecars. Before Phase B that file imported `axon-codegen` because `build.ax` listed it. After Phase B it imports `inkwell` directly. This phase makes the generation path explicit, tested, and immune to regressions.

**Files:**

- Modify: `src/compiler/backend/ffi.ax` — owner of bridge `Cargo.toml` body.
- Modify: `src/compiler/backend/foreign_archive.rs` — writes the body to `target/cache/app/rust/bridge/Cargo.toml`, runs `cargo build`.
- Modify: `build.ax` — `rust_deps` block lists only **end-user-visible** deps (e.g. `tracing`, `clap`, `inkwell`). No compiler crates.
- Test: `src/compiler/backend/ffi.test.ax`.

**Tasks:**

- [ ] **Step 1**: write a failing fixture under `tests/axon-cli/fixtures/zero_cargo_bridge/` with a tiny `build.ax` that has a single sidecar and one `rust_dep`. Add a parity test that asserts the generated `target/cache/app/rust/bridge/Cargo.toml` contains exactly the requested deps (plus the implicit ones the sidecar runtime needs).

- [ ] **Step 2**: implement `ffi.ax::generate_bridge_cargo_toml(build_ax_text, sidecar_files)`. Pure function, returns `String`. Tests in `ffi.test.ax`.

- [ ] **Step 3**: rewrite `foreign_archive.rs::build_rust_bridge_archive` to:
  1. Call `ffi.ax`-derived FFI to obtain the body string.
  2. Write to `target/cache/app/rust/bridge/Cargo.toml`.
  3. Run `cargo build` with `CARGO_TARGET_DIR=target/cache/app/rust/target`.
  4. Return path to the produced staticlib.

- [ ] **Step 4**: assertion:

```bash
./target/build/axon/axon build
test -f target/cache/app/rust/bridge/Cargo.toml
! git check-ignore -q target/cache/app/rust/bridge/Cargo.toml \
   && echo "FAIL: bridge Cargo.toml is not ignored" && exit 1
echo "OK: generated bridge is gitignored"
```

- [ ] **Step 5**: commit.

---

## Phase E — Bring the test command back online without `axon-codegen`

Phases A.6 and A.9 already moved `test_harness.rs` into `test_orchestrate.ax`. This phase is the **end-to-end** acceptance for `axon test` from a binary that has zero links to legacy compiler crates.

**Files:**

- Modify: `src/compiler/pipeline_check/test_orchestrate.ax`.
- Modify: `src/compiler/backend/backend.rs` (test binary launcher only).

**Tasks:**

- [ ] **Step 1**: from the binary built in Phase D:

```bash
./target/build/axon/axon test
```

Expected: every `*.test.ax` in `src/` and every fixture under `tests/axon-cli/fixtures/` runs; the summary "<n> passed, <m> failed" prints; exit 0 when all green.

- [ ] **Step 2**: external-fixture acceptance:

```bash
./target/build/axon/axon test tests/axon-cli/fixtures/project_typecheck_valid
./target/build/axon/axon test tests/axon-cli/fixtures/project_private_cross_mod
```

Expected: same outcomes as before the cutover (parity with the historical reference behavior captured by the fixture's `expected.txt`).

- [ ] **Step 3**: commit any final test-orchestrate fixes.

---

## Phase F — Final acceptance and lock

**Files:**

- Modify: `scripts/verify-self-hosting-cutover.sh` — last edit: at the end, run `./scripts/assert-zero-cargo.sh` so the cutover gate fails if anyone reintroduces a Cargo file.

**Tasks:**

- [ ] **Step 1**: from a fresh checkout (no `target/`):

```bash
git clean -fdx
test -z "$(git ls-files | grep -E '(^|/)Cargo\.(toml|lock)$')" || { echo "FAIL: tracked Cargo files"; exit 1; }
```

- [ ] **Step 2**: bootstrap from a previously produced binary (no in-repo Cargo):

```bash
export AXON_PREBUILT_BIN=/path/to/known-good/axon
"$AXON_PREBUILT_BIN" build
test -x target/build/axon/axon
```

- [ ] **Step 3**: full self-host loop:

```bash
./scripts/verify-self-hosting-cutover.sh
```

Expected: `RESULT: ALL PASSED — self-hosting cutover verified`. Stages 1–3 produce `axon_selfcompiled1..3`. The closing call to `./scripts/assert-zero-cargo.sh` passes.

- [ ] **Step 4**: independence proof:

```bash
./scripts/assert-zero-cargo.sh
./scripts/assert-no-legacy-compiler-refs.sh --post-delete
```

Expected: both PASS.

- [ ] **Step 5**: external project proof:

```bash
mkdir /tmp/extproj && cd /tmp/extproj
cat > build.ax <<'EOF'
project hello
    version: "0.1.0"

bin hello
    main: "./src/main.ax"
EOF
mkdir src && cat > src/main.ax <<'EOF'
func main()
    return 0
EOF
"$OLDPWD/target/build/axon/axon_selfcompiled3" check ""
"$OLDPWD/target/build/axon/axon_selfcompiled3" build
./target/build/hello/hello
```

Expected: `check` exit 0; `build` produces an executable; the executable runs and exits 0.

- [ ] **Step 6**: commit the lock:

```bash
git add scripts/verify-self-hosting-cutover.sh
git commit -m "feat(verify): cutover gate now asserts zero Cargo invariants"
```

---

## File-by-file final-state checklist

Use this as the merge-blocker checklist for the last PR. Items marked **[cutover]** come straight from `2026-04-30-axon-self-hosting-cutover.md` § "Final Acceptance Criteria"; items marked **[migration]** come from `2026-04-30-axon-complete-migration.md` § "Final Acceptance Criteria"; items marked **[zero-cargo]** are added by this plan.

### Source tree invariants

- [ ] **[zero-cargo]** `git ls-files | grep Cargo.toml` → empty.
- [ ] **[zero-cargo]** `git ls-files | grep Cargo.lock` → empty.
- [ ] **[cutover]** `git ls-files | grep -E '^(depreciating-soon-compiler-do-not-rename|bootstrap-compiler|rust-backed-compiler-for-axon|rust-self-compiler-for-axon)/'` → empty.
- [ ] **[zero-cargo]** `git ls-files | grep -E '^native-rust/'` → empty.
- [ ] **[zero-cargo]** `git ls-files src/ | grep '\.rs$'` returns only entries listed in `docs/migration/sidecar-allowlist.md`.
- [ ] **[migration]** `rg 'axon[-_](codegen|mir|typecheck|frontend|runtime|types)' src/ build.ax scripts/` → empty.
- [ ] **[zero-cargo]** `rg '#\\[axon_pub_export\\]' src/` returns only Phase D / Phase B sidecar functions; each one has a matching declaration consumed by an `.ax` caller.
- [ ] **[cutover]** `rg 'depreciating-soon-compiler-do-not-rename' src/ build.ax scripts/` → empty.
- [ ] **[zero-cargo]** `rg 'native-rust' src/ build.ax scripts/` → empty.

### Behavior invariants

- [ ] **[cutover]** `backend.rs` does not invoke `cargo` for a compiler workspace; only path: `target/cache/app/rust/bridge/`.
- [ ] **[cutover]** `lower_project` (or successor) consumes real Axon MIR — no marker / file-count writer.
- [ ] **[cutover]** Rust sidecars contain no compiler-policy decisions beyond OS / LLVM / process / file / native-boundary work.

### Verification scripts

- [ ] **[zero-cargo]** `./scripts/assert-zero-cargo.sh` → PASS.
- [ ] **[migration]** `./scripts/assert-no-legacy-compiler-refs.sh --post-delete` → PASS.
- [ ] **[cutover]** `./scripts/verify-self-hosting-cutover.sh` → PASS (uses `AXON_PREBUILT_BIN` or `AXON_BOOTSTRAP_MANIFEST` set out-of-tree).

### Self-host chain

- [ ] **[cutover]** `target/build/axon/axon_rustcompiled1` exists and is executable (produced by `AXON_PREBUILT_BIN`/`AXON_BOOTSTRAP_MANIFEST`, **not** any in-tree manifest).
- [ ] **[cutover]** `target/build/axon/axon_selfcompiled{1,2,3}` exist and are executable.
- [ ] **[cutover]** `axon_selfcompiled3 check ""` / `build` / `run` / `test` succeed on this repo.
- [ ] **[cutover]** `axon_selfcompiled3 check / build / run / test <fixture>` succeed on at least one non-self fixture (e.g. `tests/axon-cli/fixtures/project_typecheck_valid`).

### Built binary smoke

- [ ] `./target/build/axon/axon check ""` → exit 0.
- [ ] `./target/build/axon/axon build` → produces `target/build/axon/axon`.
- [ ] `./target/build/axon/axon run` → exit 0.
- [ ] `./target/build/axon/axon test` → all green.
- [ ] `./target/build/axon/axon test tests/axon-cli/fixtures/project_typecheck_valid` → parity with `expected.txt`.

---

## Risk register

| Risk | Mitigation |
|---|---|
| LLVM IR emission depends on `inkwell` API surface that's hard to stabilize through a JSON FFI. | Phase B keeps the FFI **module-scoped** (one MIR module → one object file) and uses `serde_json` + `catch_unwind`. Errors return as JSON, never panics. |
| Subagents reintroduce a `Cargo.toml` for "convenience". | `assert-zero-cargo.sh` runs in CI and at the end of the cutover script; merging is blocked. |
| `target/cache/app/rust/bridge/Cargo.toml` accidentally gets tracked. | `.gitignore` covers `target/`; `assert-zero-cargo.sh` checks `git ls-files`, not the working tree. |
| Bootstrap problem: a fresh contributor can't build without `AXON_PREBUILT_BIN`. | Releases attach `axon` binaries for major OS/arch. `AGENTS.md` documents both paths. |
| Reference-parity regressions during Phase A surface only as runtime test failures. | Each port-PR ships its fixture **before** moving the policy. `scripts/parity-run.sh` keeps running until `native-rust/` is deleted. |
| `rust_deps` declared in `build.ax` interfere with the sidecar bridge layout. | `ffi.ax::generate_bridge_cargo_toml` is the single owner; tests in `ffi.test.ax` lock the schema (named keys, ordering). |

---

## Self-review checklist

- **Spec coverage**: every requirement of the user request — "no references to anything not native", "all code in `.ax` or `.rs` sidecars", "zero Cargo.toml" — has a phase that produces evidence (`assert-zero-cargo.sh`, `sidecar-allowlist.md`, in-process `native_codegen.rs`). ✅
- **Placeholder scan**: no "TBD" / "later" / "appropriate" patterns; every step lists files, commands, and expected output. ✅
- **Type consistency**: FFI signatures `emit_module_object`, `link_artifact`, `publish_axon_install_layout`, `generate_bridge_cargo_toml` are referenced consistently across Phases B, D, E. The MIR envelope is `ok:lowered:v5:` everywhere it appears (Phase A.4, A.6). ✅
- **Order independence**: Phases A → F are sequential as written, but A.1 .. A.7 may run in parallel PRs because each one is gated only by its own subset of the source map.
