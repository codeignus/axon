# Axon Complete Migration Implementation Plan

> **For agentic workers:** This plan is a fully positive migration. The previous plan (`2026-04-30-axon-self-hosting-cutover.md`) is still the correctness contract; this plan **expands** it into a step-by-step bring-up so every blocker becomes a small, completable port rather than a wall. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the **repo-root Axon project (`build.ax` + `src/**/*.ax` + `src/**/*.rs` sidecars)** the **only** Axon compiler. The reference Rust workspace (`depreciating-soon-compiler-do-not-rename/`) is a **temporary read-only mine** of behavior to copy into the Axon project, then it is deleted.

> **Naming:** Use **`depreciating-soon-compiler-do-not-rename/`** only (canonical). The misspelling `deprecioated-…` is obsolete; scripts and docs assume the canonical path. `.gitignore` may list both so old local trees do not pollute `git status`.

**Architecture target (final):**
- Compiler binary is built **only** from `build.ax` + `src/`.
- Logic, policy, diagnostics, MIR, lowering, codegen orchestration, and command semantics live in `src/**/*.ax`.
- Rust survives **only** as `*.rs` **sidecars** beside `*.ax` files for: file/process/permissions, network, LLVM/object emission, linker, foreign archive build (`cargo`/`rustc`/`go`), CLI parsing, panic/format/trace boundaries.
- No second compiler workspace exists.
- No script searches another `Cargo.toml` for compiler logic.

**Tech Stack:** Axon (`*.ax`), Rust sidecars (`*.rs`), shell verification scripts. LLVM 21 + nightly Rust are required while sidecars still own native codegen; this is a **build-time** dependency, not a separate compiler.

**Bootstrap vs self-host:** Stage 0 uses `cargo run` from the reference workspace manifest to produce the first `target/build/axon/axon` (preserved as `axon_rustcompiled1`). Stages 1–3 run that binary’s `check`/`build` on this repo to produce `axon_selfcompiled{1,2,3}`. LLVM 21 may live under different prefixes per host; set `LLVM_SYS_211_PREFIX` or use `./scripts/verify-self-bootstrap.sh`, which probes `llvm-config-21`, `llvm-config` 21.x, and `/usr/lib/llvm/21` when unset. See **AGENTS.md** (section “Bootstrap → self-host chain”).

---

## Mindset & Migration Principles

This plan is **positive**, not subtractive. It assumes:

- Total LOC to port from the reference tree is large but **mechanical**: ~88 Rust source files, ~55k lines including tests/fixtures. Behavior is well-typed and exercised by fixtures we already have.
- Most Rust logic maps to Axon **directly** because the language already has structs, enums, options, results, traits, generics, FFI, build/run/test, and a working MIR-style sidecar surface. Anything missing is a **language gap to file**, not a stop sign.
- An agent can do **many small ports per day**. Velocity is measured in **modules ported and behavior tests passing**, not weeks.

### Sidecar policy during migration

- It is **expected and encouraged** to add **temporary `*.rs` sidecars** when Axon is missing a feature.
- A sidecar is **legitimate** as long as **its file lives under `src/`** and it is invoked via the Axon FFI surface. The repo stays a single Axon project.
- When the corresponding `.ax` capability lands, the sidecar **shrinks or is deleted** in the same change that promotes it.
- A sidecar must **never** drive `cargo` against another compiler workspace, never run another `axon` binary, and never make policy decisions that should belong to language code (e.g. typecheck rules).

### Language-gap protocol

Whenever an Axon language feature is missing while porting:

1. Open a one-line note inside the relevant ported file: `// LANG-GAP: <symbol/feature> needed; using sidecar X until added`.
2. Add or extend a `.rs` sidecar that exposes the missing primitive as an FFI function.
3. Continue the port; do **not** stall the migration.
4. The user (language owner) lands the feature later; the sidecar is then deleted in the same PR that ports the call site.

This guarantees the migration **never blocks** on language work.

---

## Reuse the Old Plan’s Contract (`2026-04-30-axon-self-hosting-cutover.md`)

That plan defines the **acceptance contract**:

- **Blocker A**: backend cannot run another compiler workspace.
- **Blocker B**: `lower_project` must not be a marker counter.
- **Blocker C**: parser/semantics must be Axon-owned over real AST data.
- **Blocker D**: verification scripts must not point at the deprecated tree.
- **Final cutover**: `depreciating-soon-compiler-do-not-rename/` deleted.
- **Final binaries**: `axon_rustcompiled1`, `axon_selfcompiled{1,2,3}` reproducible.
- **Final independence**: `axon check/build/run/test` works on this repo and a non-self fixture without the reference tree.

This new plan **inherits** every acceptance criterion of that plan. It only changes the **how**: phased, sidecar-friendly, with concrete LOC-shaped bring-up steps.

---

## Execution Policy (positive defaults)

- Default to **shipping behavior every step**, not adding scaffolding.
- Land in **branch + PR per phase**; do not batch unrelated phases.
- Keep `target/build/axon/axon` runnable after every merged step; the bridge driver is allowed to keep compiling reference codegen until the matching phase replaces it.
- Tests are **fixtures-first**: each task ports the fixture-set from the reference tree (`tests/axon-cli/fixtures/**`, `tests/axon-frontend/fixtures/**`, etc.) into `tests/` here and asserts that the new Axon-owned path produces identical pass/fail/diagnostics output.
- Behavior parity is verified with a **delta runner** (`scripts/parity-run.sh`) that runs the same fixture through the in-repo compiler **and** the migration driver and diffs results until the migration driver is removed.

---

## File Responsibility Map (final state)

### Axon-Owned Compiler Logic (`*.ax`)
- `src/compiler/syntax/{token,ast,lexer,parser}.ax` — token kinds, AST data model, full lexer with indentation/raw blocks/string-literal handling, full parser to AST.
- `src/compiler/semantics/{resolve,types,ownership,check,lint}.ax` — symbol tables, visibility, type model + inference + unification, ownership analysis & cleanup metadata, semantic orchestration.
- `src/compiler/ir/{ir,lower}.ax` — typed MIR module/function/block/local/statement/terminator + AST-to-MIR lowering with ownership annotations.
- `src/compiler/proj/{build_file,discover,module_graph,targets,command_targets}.ax` — `build.ax` parsing, source discovery, module graph, command-target resolution.
- `src/compiler/backend/{artifacts,link,ffi,bootstrap}.ax` — artifact layout, link plan, FFI inventory & validation, bootstrap orchestration.
- `src/compiler/diagnostics/diagnostic.ax` — diagnostic codes, severity, formatting policy.
- `src/compiler/pipeline_check/pipeline_check.ax` — staged check pipeline + failure propagation.
- `src/compiler/entry.ax` — `check`, `build`, `run`, `test`, `fmt` semantics.

### Allowed Rust Sidecars (`*.rs`, all under `src/`)
- `src/compiler/proj/discover.rs` — directory listing, canonicalize, exists.
- `src/compiler/syntax/{lexer,parser}.rs` — file read/walk only after Axon owns logic.
- `src/compiler/semantics/{semantics,ownership}.rs` — file read/walk only after Axon owns logic.
- `src/compiler/ir/ir.rs` — serialization helpers if needed (no policy).
- `src/compiler/backend/backend.rs` — native artifact write, linker invocation, process exec, permissions, suffixed-binary preservation, LLVM object emission glue.
- `src/compiler/backend/toolchain.rs` — probe `rustc`/`cc`/`go`/`clang`/`llvm-config`.
- `src/compiler/backend/native_codegen.rs` *(new)* — LLVM/inkwell-based MIR-to-object emission, called from Axon over a stable JSON request shape.
- `src/compiler/backend/foreign_archive.rs` *(new)* — generate Rust/Go bridge crates and run `cargo`/`go` to produce `.a` archives.
- `src/clap.rs`, `src/tracing.rs`, `src/sidecar.rs` — CLI parsing, logging, panic/format boundaries.

The migration driver binary is built from **`src/Cargo.toml`** (package `axon-sidecars`, bin `axon-native-build`, source `src/axon_native_build_bin/main.rs`). It is **temporary** scaffolding that links reference `axon-codegen`, then disappears in Phase 8.

---

## Phase 0 — Migration Foundations (shared infra)

**Files:**
- Create: `scripts/parity-run.sh`
- Create: `scripts/parity-fixture-list.txt`
- Modify: `AGENTS.md` (positive migration messaging — done in this PR).
- Create: `docs/migration/source-map.md` (reference Rust file → Axon module mapping).
- Create: `src/compiler/backend/native_codegen.rs` (skeleton with one FFI: `emit_object_for_module(json_mir)` returning object bytes/path).
- Create: `src/compiler/backend/foreign_archive.rs` (skeleton FFI: `build_rust_bridge_archive(project_root, deps_raw, sidecar_files)`).

**Tasks:**
- [x] Inventory the reference tree to a port map — checked into `docs/migration/source-map.md` (expand per phase).
- [x] Add `scripts/parity-run.sh` that runs `axon check` and (when available) migration driver `check` on fixtures; compare exit codes; optional **`PARITY_STRICT=1`** for stdout/stderr diff.
- [x] Add `scripts/parity-fixture-list.txt` with stable ordering (starts at `project_typecheck_valid`).
- [x] Land sidecar skeletons `native_codegen.rs`, `foreign_archive.rs` returning **not migrated yet** placeholders.

**Verification:**
```bash
bash -n scripts/parity-run.sh
./target/build/axon/axon check ""
```

Expected: parity script runs and reports "no parity diff" for the fixtures already covered by Axon-owned check.

---

## Phase 1 — Token Stream Owned By Axon

**Reference sources:** `crates/axon-frontend/src/lexer/{mod,literals}.rs`, `crates/axon-types/src/token.rs`.

**Files:**
- Modify: `src/compiler/syntax/token.ax` (full token kind set, span model).
- Modify: `src/compiler/syntax/lexer.ax` (full tokenizer including indentation, raw `@rust`/`@go` blocks, string/char/f-string starts, numeric underscores, comments, EOF).
- Modify: `src/compiler/syntax/lexer.rs` (reduce to file read/walk only).
- Test: `src/compiler/syntax/lexer.test.ax` + ported fixtures from reference `lexer/tests.rs`.

**Tasks:**
- [x] **Full lexer parity (Phase 1 done):** `lexer.rs::Lexer` mirrors axon-frontend: indent stack, bracket depth, newline-in-parens, raw `@rust`/`@go`…`@end`, `f"…"`, `_` in numbers, newline-in-string error. **`axon_lex_token_stream`** is the FFI used by **`lex_all_tokens`** in `lexer.ax` so tests and tooling share one tokenizer.
- [x] **`validate_tokens` / `token_count_native`** use **`lex_all_tokens`** → same stream.
- [x] **Tests:** `lexer.test.ax` covers indent/dedent/raw/fstring via `lex_all_tokens`.
- LANG-GAP: only if parity gaps appear on specific fixtures (`lexer.rs`).

**Verification:**
```bash
./target/build/axon/axon check ""
bash scripts/parity-run.sh
```

Expected: lexer diagnostics come from Axon. Parity diff: zero.

---

## Phase 2 — AST Model + Parser Owned By Axon

**Reference sources:** `crates/axon-types/src/ast/*.rs`, `crates/axon-frontend/src/parser/*.rs`.

**Files:**
- Modify: `src/compiler/syntax/ast.ax` (AST data model + accessors).
- Modify: `src/compiler/syntax/parser.ax` (full token-stream → AST).
- Modify: `src/compiler/syntax/parser.rs` (file walk only).
- Test: `src/compiler/syntax/parser.test.ax` + ported fixtures from reference `parser/decl.rs`, `expr.rs`, `stmt.rs`, `ty.rs`, `pattern.rs`.

**Tasks:**
- [x] **Incremental Phase 2:** `parser.ax` adds **`validate_token_stream_delimiters`** (paren/bracket/brace over lexer stream via **`lex_all_tokens`**) and **`validate_delimiters_char_scan`** / **`describe_parse_source`** (string-aware stack, mirrors `parser.rs`). Tests in **`parser.test.ax`**. Full AST/parser port remains open.
- [x] **Phase 2 AST:** `ast.ax` defines full node vocabulary — Decl (func/import/struct/enum/trait/error/type/test/method), Expr (call/binary/unary/ident/int/float/string/bool/nil/member/index/constructor/fstring/tuple/list/try/catch/await/orelse/ordefault/ref/deref), Stmt (block/binding/mut/return/if/for/while/match/assign/break/continue/defer/errdefer), Type (named/generic/func/tuple/array/optional/ref/fallible), Pattern (binding/constructor/tuple/wildcard/literal). Node encoding helpers (`node_make`, `node_kind`, `node_data`, `node_append_child`).
- [x] **Phase 2 Parser:** `parser.ax` gains token stream navigation, matching bracket scanner, function/method header parsing with return type, dedent tracking. Tests in **`parser.test.ax`** for func/method/struct/enum/import parsing.
- [ ] Port remaining: full expression precedence, f-string, match arms (parser surface for `?T`/`T!`/`?T!` is exercised in **`type_sigils.test.ax`**; full AST for match arms still open).
- [x] **`parser.rs`** reduced to file walk + minimal delimiter scan (LANG-GAP: mirrors **`validate_delimiters_char_scan`** in **`parser.ax`** until pipeline calls Axon directly).
- [x] Convert `lexer.ax` mut/while helpers to recursion. All `.ax` files now pure recursion — zero `mut`/`while` constructs remain across entire `src/` tree.

**Verification:**
```bash
./target/build/axon/axon check ""
bash scripts/parity-run.sh
```

Expected: parser builds AST for all repo-root sources and migration fixtures with zero diff.

---

## Phase 3 — Project Graph + Build Manifest

**Reference sources:** `crates/axon-frontend/src/{build,loader}.rs`, `crates/axon-codegen/src/{graph,target_resolution}.rs`.

**Files:**
- Modify: `src/compiler/proj/{build_file,discover,module_graph,targets,command_targets}.ax`.
- Modify: `src/compiler/proj/discover.rs` (filesystem primitives only).
- Test: `src/compiler/proj/{loading,targets}.test.ax` + `src/compiler/semantics/project_parity.test.ax`.

**Tasks:**
- [x] `build_file.ax`: `main:` / `version:` strip optional quotes; `manifest_has_rust_deps` heuristic; **`discover.rs` `discover_entry`** reads `main:` from `build.ax`; tests in `proj/build_file.test.ax`.
- [x] **Incremental Phase 3:** **`manifest_has_go_deps`**, **`manifest_has_python_deps`** in **`build_file.ax`** + tests in **`build_file.test.ax`**. Remaining loader parity still open.
- [x] **Incremental Phase 3b:** **`manifest_has_deps`**, **`extract_deps_body`** (indented body extraction for `rust_deps`/`go_deps`/`python_deps`/`deps` blocks); hyphenated project name edge cases; tests in **`build_file.test.ax`**.
- [x] **`deps` / `rust_deps` / `go_deps` / `python_deps`:** body extraction and entry parsing in **`build_file.ax`**; remaining parity is edge cases vs reference loader only.
- [x] **Incremental Phase 3c:** sidecar association (**`classify_file_pair`** in **`discover.ax`** — checks `.rs` beside `.ax`); import-path → file-path conversion (**`import_path_to_file_path`** — resolves `compiler/proj/build_file` to `src/compiler/proj/build_file.ax`, directory module, or not-found). Tests in **`loading.test.ax`**.
- [x] **Incremental Phase 3d:** check/test target scopes now Axon-native in **`targets.ax`** (`axon_classify_check_target` / `axon_classify_test_target`) — no longer delegates to `targets.rs` sidecar. Covers project, dir, dir-recursive, file, and test:project scopes. Existing tests in **`targets.test.ax`** pass unchanged.
- [x] **Incremental Phase 3e:** multi-bin target support (**`extract_all_bin_targets`**, **`scan_bin_main`**); integration test discovery (**`discover_integration_tests`**, **`discover_colocated_tests`**, **`discover_all_test_files`**).
- [x] **`discover.rs`** remains FS-only (list/read/exists); **`collect_app_files`** uses **`list_all_ax_files`** so app sources exclude `.test.ax` consistently with **`collect_all_source_files`**.

**Verification:**
```bash
./target/build/axon/axon check ""
./target/build/axon/axon test
```

Expected: project/module errors and target-scope errors are produced by Axon-owned code with diagnostics matching the reference fixtures.

---

## Phase 4 — Resolver, Visibility, Imports

**Reference sources:** `crates/axon-frontend/src/resolver/mod.rs`, `crates/axon-frontend/src/semantics.rs`, `crates/axon-types/src/symbol.rs`.

**Files:**
- Modify: `src/compiler/semantics/{resolve,check}.ax`.
- Modify: `src/compiler/semantics/semantics.rs` (file walk only).
- Test: `src/compiler/semantics/{check,project_parity}.test.ax` + ported fixtures `tests/axon-semantics/fixtures/**`.

**Tasks:**
- [x] **Incremental Phase 4:** **`semantics.rs`** `parse_import_bindings` now flags **duplicate braced import lines** for the same module path (same-line duplicate symbols were already caught). Full resolver parity still open.
- [x] **Incremental Phase 4b:** **`resolve.ax`** `check_duplicate_braced_imports` detects cross-line duplicate symbols in braced imports using pure Axon; tests in **`check.test.ax`**. **`command_targets.ax`** `validate_test_file_path` enforces test file must be `src/**/*.test.ax` or `tests/**/*.ax`.
- [x] **Incremental Phase 4c:** **`resolve.ax`** gains `check_duplicate_declarations_axon`, `check_self_import_axon`, `check_import_collision_axon`, `check_visibility_axon`, `build_symbol_table_axon`, `resolve_all_imports_axon` — all pure Axon, no `mut`/`while`. **`check.ax`** gains `run_full_semantic_check` chaining all checks. Tests in **`check.test.ax`** cover duplicate funcs/structs/enums/traits, self-import, import collision, visibility, symbol table, and full semantic chain.
- [x] **Incremental Phase 4d:** struct/enum/trait member duplicate detection, method self-rule checking, import alias conflict detection and resolution. **`semantics.rs`** reduced from ~810 to ~394 lines with LANG-GAP markers.
- [ ] Reduce `semantics.rs` further to file iteration / string transport only (remaining LANG-GAP functions).

**Verification:**
```bash
./target/build/axon/axon check ""
```

Expected: resolver diagnostics match reference behavior across all reference resolver fixtures.

---

## Pre-Phase 5 gate — Language + reference codegen (type system refinement)

This section is **not** a numbered migration phase; it records work that must land **before** Axon-side Phase 5 (typechecker port) so the language and the **reference** `axon-codegen` match what migration assumes.

**Reference checkout:** clone `git@github.com:codeignus/axon-rust-compiler.git` on branch **`cursor/type-system-refinement`** into **`depreciating-soon-compiler-do-not-rename/`** at the repo root (gitignored). `src/Cargo.toml` depends on `crates/axon-codegen` from that tree.

**Already carried into this repo (Axon project):**

- [x] Types layout: `compiler/types/primitives/*`, `compiler/types/composites/*` (replaces old `complex_types/` split).
- [x] Parser / resolver surface for fallible types: postfix `T!`, prefix `?`, combined `?T!`; **`type_sigils.test.ax`** exercises parse/resolve paths.

**Lives in the reference tree only** (agents port behavior into `.ax` during Phases 5–8):

- Match on `Option` / `Result` in MIR lowering (`InspectOption` / `InspectResult`, extractors); tests moved off legacy `try`/`catch`/`or_else`/`or_default` where applicable.
- FFI: `validate_ffi_type` / `ffi_validate` allow nested `Option<>` / `Result<,>` on the Rust FFI surface; **`coerce_to_type`** after non-string foreign calls vs declared MIR return (fixes LLVM `icmp` width mismatches on `bool` from FFI).
- `bridge_syn.rs` / `bridge_gen`: syn-first `@rust` block extraction with line-based fallback.

**Follow-ups (optional, not blocking Phase 5 start):**

- [ ] Full rewrite of `type_marshall` to derive only from `AxonType` (deeper than FFI guard + validation).
- [ ] `cargo test -p axon-cli-tests` green in the reference workspace (integration); treat regressions as CI debt, not migration phase numbers.

**Verification (reference tree):** `cargo test -p axon-mir`, `cargo test -p axon-codegen`, `cargo test -p axon-frontend` (as appropriate); full workspace `cargo check` when LLVM/Rust toolchain matches `axon-codegen` (LLVM 21 per crate features).

---

## Phase 5 — Typechecker, Inference, Lints

**Reference sources:** `crates/axon-typecheck/src/{checker,infer,unify,types,ops,env,ownership,diagnostics}.rs`, `crates/axon-frontend/src/lint.rs`.

**Files:**
- Modify: `src/compiler/semantics/{types,check,lint}.ax`.
- Modify: `src/compiler/diagnostics/diagnostic.ax`.
- Test: `src/compiler/semantics/{types,check,lint}.test.ax`.

**Tasks:**
- [ ] Optional stub (deferred from type-refinement scope): **`check_match_exhaustiveness_axon`** in **`check.ax`** + resolver hooks when the Axon typechecker owns match typing; not required to start porting primitives.
- [x] **Incremental:** **`lint.ax`** + **`run_full_semantic_check`** — lint runs after core semantic pass (unreachable-after-**`return`** + placeholder path for more rules).
- [x] **Incremental:** **`types.ax`** — string-encoded type helpers: **`type_name_is_option`**, **`type_name_is_result`**, **`type_strip_one_optional`** (+ tests in **`types.test.ax`**).
- [x] **Incremental:** **`semantics.ax`** snippet checker — literal inference uses **`bool`** / **`i32`** / **`void`** for **`nil`** (aligned with **`types.ax`**); call arg checks use **`type_compatible`**; **`?T`** prefix stripped when parsing param types from decls.
- [x] **`typecheck.ax`** + **`pipeline_check.ax`** **`check_stage_typecheck`** — **`run_typecheck_project_path`** walks **`list_all_ax_files(<root>/src)`** and runs **`run_full_semantic_check`** per app file (same Axon chain as snippet tests). Deeper inference/unification still ports from reference **`axon-typecheck`**.
- [ ] Port the type model: primitives, integer widths/overflow, floats, bool, string, options/results, tuples, generics, traits, methods, associated funcs.
- [ ] Port inference + unification + expected-type propagation.
- [ ] Port operator typing rules and call/return checks.
- [x] **Incremental:** **`lint.ax`** — unreachable code after **`return`** on same function body (skips blank/`//`/nested decl starts); unused locals + suppression still open.
- [ ] Remove all string-line heuristic type decisions.

**Verification:**
```bash
./target/build/axon/axon check ""
bash scripts/parity-run.sh
```

Expected: every typecheck fixture (`tests/axon-frontend/fixtures/typecheck/**`, `tests/axon-cli/fixtures/project_typecheck_*`) yields the same diagnostic IDs and severities through the Axon-owned path.

---

## Phase 6 — Ownership, Cleanup, Branch Reconciliation

**Reference sources:** `crates/axon-typecheck/src/ownership.rs`, `crates/axon-mir/src/lower.rs` ownership-summary plumbing.

**Files:**
- Modify: `src/compiler/semantics/ownership.ax`, `src/compiler/ir/lower.ax`.
- Modify: `src/compiler/semantics/ownership.rs` (file walk only).
- Test: `src/compiler/semantics/ownership.test.ax`, `src/compiler/ir/ir.test.ax`.

**Tasks:**
- [x] **`ownership_summary.ax`** — **`build_ownership_summary_stub`** returns **`ok:ownership-summary:app-files=<n>`** from **`list_all_ax_files(<root>/src)`**; **`ownership.ax`** passes **`discover_project_root()`**.
- [ ] Port canonical-owner selection, returned-local handling, returned-field-from-param/local handling, alias invalidation by mut reassignment / field mutation.
- [ ] Port branch reconciliation across `if/elif/else` and `match` arms.
- [ ] Tuple returns are path groups → no aggregate shell cleanup.
- [ ] Aggregate shell cleanup frees only inline-owned fields and skips pointer-backed fields.
- [ ] Emit real ownership summaries for MIR lowering (stub is a placeholder hook only).

**Verification:**
```bash
./target/build/axon/axon check ""
./target/build/axon/axon build
```

Expected: ownership-related diagnostics and codegen requests are Axon-owned; backend request payload includes ownership metadata.

---

## Phase 7 — MIR + Lowering (Real `lower_project`)

**Reference sources:** `crates/axon-mir/src/{mir,types,lower}.rs`.

**Files:**
- Modify: `src/compiler/ir/ir.ax` (typed MIR module/function/block/local/stmt/terminator).
- Modify: `src/compiler/ir/lower.ax` (real AST-to-MIR + ownership annotations).
- Modify: `src/compiler/ir/ir.rs` (serialization helpers only; remove file-counter lowering).
- Test: `src/compiler/ir/ir.test.ax` covering literals, operators, calls, returns, bindings, assignments, control flow, ownership annotations, struct/enum constructors.

**Tasks:**
- [ ] Implement real MIR data model in Axon (beyond constants/helpers in **`ir.ax`** / **`lower.ax`**).
- [ ] Lower literals, identifiers, binary/unary ops, calls, returns, bindings, assignments, `if/elif/else`, `while`, `break`, `continue`, `match`, struct/enum constructors, tuple/list literals, options/results.
- [ ] Emit owned locals, string-literal locals, aggregate field modes.
- [x] **Incremental:** **`ir.rs` `lower_project`** writes **`target/cache/lowered.ir`** with a **JSON v2 envelope** line (`{"v":2,"kind":"axon-mir-envelope",...}`) plus legacy `module` lines; return **`ok:lowered:v2:<n>`** (still accepted by **`backend.rs`** as **`ok:lowered:*`**). Per-module MIR bodies remain **`stub`** until lowering is ported from reference **`axon-mir`**.

**Verification:**
```bash
./target/build/axon/axon check ""
./target/build/axon/axon build
```

Expected: `target/cache/lowered.ir` (or replacement) carries real MIR records; backend cannot succeed from a marker string.

---

## Phase 8 — Native Codegen Boundary (LLVM / Linker / Foreign Archives)

**Reference sources:** `crates/axon-codegen/src/{codegen,linker,artifacts,prepare,call_resolution,type_marshall,bridge_gen,rust_compile,go_compile,rustc_diagnostics}.rs`.

**Files:**
- Modify: `src/compiler/backend/{artifacts,link,ffi,backend}.ax`.
- Modify: `src/compiler/backend/native_codegen.rs` (LLVM/inkwell MIR-to-object).
- Modify: `src/compiler/backend/foreign_archive.rs` (Rust/Go bridge build).
- Modify: `src/compiler/backend/backend.rs` (object emit + link + publish; **no second compiler workspace**).
- Modify: `src/compiler/backend/toolchain.rs` (probe `rustc`, `cc`, `go`, `llvm-config`).
- Test: backend behavior tests adjacent to backend modules, including link plan, artifact path policy, FFI validation, ownership-cleanup contract.

**Tasks:**
- [x] **Incremental:** **`backend/policy.ax`** — Axon-side policy strings (**`describe_native_codegen_boundary`**, **`describe_link_artifact_contract`**, **`assert_no_second_compiler_workspace`**) as scaffolding until policy moves out of **`axon-codegen`**.
- [ ] Move policy decisions out of Rust: symbol naming, builtin lowering contract, type marshalling contract, ownership cleanup contract, artifact path policy, link plan policy, FFI inventory, foreign-archive plan.
- [ ] Leave `native_codegen.rs` responsible for **only** LLVM IR construction + object emission for a single MIR module, given a JSON request from Axon.
- [ ] Leave `foreign_archive.rs` responsible for **only** generating bridge sources and invoking `cargo`/`go`/`rustc` to build static archives from project sidecars (not the compiler).
- [ ] Replace `axon test` test-runner delegation with Axon-owned test orchestration plus process exec for produced test binaries.
- [ ] Shrink the migration driver (`src/Cargo.toml` + `axon-native-build`) until it is replaced by `native_codegen.rs` calls. **Delete that Cargo package at the end of this phase.**

**Verification:**
```bash
./target/build/axon/axon build
./target/build/axon/axon run
./target/build/axon/axon test
```

Expected: native binary behavior comes from Axon-produced backend request fed through `native_codegen.rs`; no second compiler workspace is invoked anywhere.

---

## Phase 9 — Test Command Semantics, Fixtures, Diagnostics Renderer

**Reference sources:** `crates/axon-runtime/src/{lib,project,tests,error}.rs`, `crates/axon-types/src/diagnostics/{renderer,types,constructors,sink,suppression}.rs`, `crates/axon-cli/src/main.rs`.

**Files:**
- Modify: `src/compiler/entry.ax`, `src/compiler/pipeline_check/pipeline_check.ax`, `src/compiler/proj/targets.ax`, `src/compiler/diagnostics/diagnostic.ax`.
- Modify: `src/clap.rs`, `src/tracing.rs`, `src/sidecar.rs` (CLI/logging only).
- Test: `src/compiler/entry.test.ax` (colocated test access to private items, integration tests scope, file/module/tree filtering).

**Tasks:**
- [ ] Port colocated `src/**/*.test.ax` access to private module items.
- [ ] Port isolated `tests/**/*.ax` integration programs limited to public app surface.
- [ ] Port diagnostic renderer, color, severity, code catalog, and stack trace rendering. Keep `tracing` / `clap` only at CLI/logging boundary.
- [ ] Implement Axon-owned test target expansion + test binary plan generation.
- [ ] Drop `run_compiler_tests_native` delegation (or shrink to a thin process exec for produced test binaries).

**Verification:**
```bash
./target/build/axon/axon test
./target/build/axon/axon test tests/axon-cli/fixtures/project_typecheck_valid
```

Expected: tests run through repo-root compiler path only; reference test fixtures pass with the same outcomes.

---

## Phase 10 — Pre-Delete Quarantine Verification

**Files:**
- Modify: `scripts/verify-self-hosting-cutover.sh`, `scripts/verify-independent-axon.sh`.
- Create: `scripts/verify-no-legacy-before-delete.sh`, `scripts/assert-no-legacy-compiler-refs.sh`.

**Tasks:**
- [ ] Move `depreciating-soon-compiler-do-not-rename/` to `target/quarantine/` for the duration of the run; trap to restore on failure.
- [ ] Run `assert-no-legacy-compiler-refs.sh` while the tree is unavailable; verify it fails before cleanup and passes after.
- [ ] Run end-to-end:
  ```bash
  target/build/axon/axon check ""
  target/build/axon/axon build
  target/build/axon/axon run
  target/build/axon/axon test
  ```
- [ ] Produce `axon_rustcompiled1`, `axon_selfcompiled{1,2,3}` snapshots **without** the quarantined tree.
- [ ] Run `check`/`build`/`run`/`test` on at least one non-self fixture from `tests/axon-cli/fixtures/`.

**Expected:** Every command passes while the old tree is unavailable.

---

## Phase 11 — Delete Legacy Tree And Cascading References

**Files:**
- Delete: `depreciating-soon-compiler-do-not-rename/`.
- Modify: `AGENTS.md` (drop bootstrap/migration mentions of the legacy tree path; describe sidecar-only architecture).
- Modify: `scripts/*.sh` (drop legacy manifest fallback).
- Modify: docs referencing the old tree (keep historical mentions, drop active workflow).

**Tasks:**
- [ ] Delete the entire reference directory.
- [ ] Remove all docs/scripts/env-var instructions pointing at it.
- [ ] Run `scripts/assert-no-legacy-compiler-refs.sh` and confirm zero active references.

---

## Phase 12 — Post-Delete Verification

**Files:**
- Modify: `scripts/verify-self-hosting-cutover.sh` if needed for post-delete mode only.

**Tasks:**
- [ ] From a tree without the reference directory, run `scripts/verify-self-hosting-cutover.sh`.
- [ ] Verify all suffixed binaries are present and runnable:
  ```bash
  test -x target/build/axon/axon_rustcompiled1
  test -x target/build/axon/axon_selfcompiled1
  test -x target/build/axon/axon_selfcompiled2
  test -x target/build/axon/axon_selfcompiled3
  ```
- [ ] Verify final binary handles repo + non-self fixture for `check`/`build`/`run`/`test`.

**Expected:** Every command succeeds without the old tree, old manifest, or any second compiler workspace.

---

## Parallelization Guidance

### Safe parallel waves
- Phase 0 inventory + Phase 1 lexer test authoring can run with Phase 2 parser test authoring (disjoint files).
- Phase 5 typechecker test authoring can run with Phase 6 ownership test authoring.
- Phase 8 sub-tasks split cleanly: codegen (LLVM) vs foreign-archive (cargo/go) vs link plan; each is an isolated PR.

### Sequential-only work
- Parser implementation after token-stream contract is fixed.
- Resolver after parser AST contract is fixed.
- Typechecker after resolver symbol-table contract is fixed.
- MIR lowering after typed AST + ownership contract is fixed.
- Native codegen replacement after MIR lowering exists.
- Pre-delete quarantine verification.
- Actual deletion.
- Post-delete verification.

---

## Final Acceptance Criteria

The migration is complete only if all are true:

- `depreciating-soon-compiler-do-not-rename/` is **deleted**.
- The repo contains **only one Cargo configuration** if any: small per-sidecar `Cargo.toml`s under `src/` for `*.rs` boundary code. There is **no** standalone Rust compiler workspace.
- `src/compiler/backend/backend.rs` does not invoke another compiler workspace; it only writes objects, links, publishes binaries, sets executable bits, and preserves suffixed binaries.
- `src/compiler/ir/ir.rs::lower_project` (or successor) consumes real Axon MIR and is not a marker/file-count writer.
- Rust sidecars contain no compiler-policy decisions beyond OS/LLVM/process/file/native boundary work.
- Pre-delete quarantine verification passes with the old tree unavailable.
- Post-delete verification passes after the old tree is removed.
- `target/build/axon/axon_rustcompiled1`, `axon_selfcompiled1`, `axon_selfcompiled2`, and `axon_selfcompiled3` exist and are executable.
- `axon_selfcompiled3` can `check`, `build`, `run`, and `test` this repo and at least one non-self Axon fixture.
- Behavior tests prove lexing, parsing, resolution, typechecking, ownership, MIR lowering, build orchestration, FFI validation, and test command behavior through the repo-root compiler path.

---

## Target Verification Commands

```bash
# parity during migration (any phase)
bash scripts/parity-run.sh

# pre-delete sanity
bash scripts/verify-no-legacy-before-delete.sh

# delete
rm -rf depreciating-soon-compiler-do-not-rename

# guard
bash scripts/assert-no-legacy-compiler-refs.sh

# final cutover proof
bash scripts/verify-self-hosting-cutover.sh

# final binary proof
target/build/axon/axon_selfcompiled3 check ""
target/build/axon/axon_selfcompiled3 build
target/build/axon/axon_selfcompiled3 run
target/build/axon/axon_selfcompiled3 test
```

Expected: all commands succeed from a repo that does not contain `depreciating-soon-compiler-do-not-rename/`.
