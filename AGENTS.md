# AGENTS

## Axon project (repository root)

The **axon project** is the language **and** the compiler:

- **`build.ax`** — project manifest.
- **`src/`** — Axon modules (`*.ax`) and Rust **sidecars** (`*.rs`) under the same project.

There is **no separate Rust compiler workspace**. There is **no** `Cargo.toml` at the repo root; this checkout is **not** a Cargo package. Any `Cargo.toml` that exists in this repo lives **inside `src/`** beside the sidecar it serves.

## Status: in the middle of a self-independent migration (positive, complete)

We are **actively migrating** the historical Rust-backed compiler **into this Axon project**. The migration is **complete** in scope, not partial:

- **Goal:** All compiler logic — lexing, parsing, resolution, typechecking, ownership, MIR lowering, codegen orchestration, project graph, command semantics, diagnostics — is owned by **`*.ax`** files in `src/`. Rust survives **only** as `*.rs` sidecars beside `*.ax` for OS/LLVM/process/IO boundaries.
- **No second compiler:** the migration **never** introduces another Axon compiler project; we build outward from this one.
- **Reference checkout for porting only:** the directory **`deprecioated-soon-compiler-do-not-rename/`** is the **read-only mine** we copy behavior from. It is **gitignored**, untracked, and **deleted** when the migration finishes.
- **Plan of record:** `docs/superpowers/plans/2026-04-30-axon-complete-migration.md` (which inherits acceptance criteria from `2026-04-30-axon-self-hosting-cutover.md`).

## Sidecar policy during migration (encouraged, not a workaround)

Adding new **`*.rs`** sidecars is fine — even encouraged — whenever an Axon language feature is missing while a port is in flight:

- A sidecar must live **inside `src/`**, alongside the `*.ax` file that calls it. The repo stays a single Axon project.
- A sidecar must expose a **narrow FFI** (file/process/permissions/LLVM/cargo/go) — never a compiler-policy decision (typecheck rules, ownership rules, name resolution, link plan, diagnostic semantics) once the Axon equivalent exists.
- Mark every temporary sidecar reach with `// LANG-GAP: <feature> needed; using sidecar X until added` so we can find and delete them.
- When the matching Axon language feature lands, the sidecar **shrinks or is deleted** in the same change that promotes its callers to `.ax`.

This policy guarantees the migration **never blocks** waiting for language features. It is normal for the sidecar count to grow before it shrinks.

## Native bring-up (build-time only)

While codegen/codegen-orchestration is still being ported, building the compiler needs:

- **`cargo +nightly`** (Rust 2024 edition is used by the migration driver until it’s replaced).
- **LLVM 21** development libraries compatible with **`llvm-sys`/inkwell**. Set **`LLVM_SYS_211_PREFIX`** if `llvm-sys` cannot find it on its own.

Migration entry points already in this repo:

- **`src/compiler/backend/axon_native_build/`** — small temporary Cargo package under `src/` that exposes a CLI driver (`axon-native-build`). It links the reference `axon-codegen` library via path until the equivalent logic moves into `*.ax` + sidecars. It is **not** another compiler project; it is internal scaffolding that gets deleted at the end of Phase 8 of the migration plan.
- **`src/compiler/backend/backend.rs`** — invokes that driver for native `check`/`build`/`test`. It does **not** subprocess any second `axon` CLI and does **not** point at any other compiler workspace.
- **`AXON_NATIVE_BUILD_BIN`** — optional env var that points `backend.rs` at a prebuilt driver binary so it can skip rebuilding.

## Verification

```bash
./scripts/verify-self-bootstrap.sh
./scripts/verify-self-hosting-cutover.sh
bash scripts/parity-run.sh                                # while migrating phases
```

`verify-self-bootstrap.sh` and `verify-self-hosting-cutover.sh` look for a manifest in this order: `AXON_BOOTSTRAP_MANIFEST` env var, then `bootstrap-compiler/Cargo.toml`, then `deprecioated-soon-compiler-do-not-rename/Cargo.toml` if a reference checkout is present locally. After Phase 11 of the migration plan, only `bootstrap-compiler/Cargo.toml` and `AXON_BOOTSTRAP_MANIFEST` survive.

## Native artifact boundary (`src/compiler/backend/backend.rs`)

`backend.rs` is the single sidecar entry point for build artifacts:

- Build: drive native codegen, publish the artifact to **`target/build/<bin>/<bin>`** and the install layout **`target/build/axon/axon`**.
- Run: execute the published binary.
- Test: drive Axon-owned test orchestration; final state runs produced test binaries directly.
- Preserve: copy the install binary to **`axon_<suffix>`** for self-bootstrap snapshots.

It must never invoke `cargo` against another compiler workspace, and it must never call out to a second `axon` CLI.

## Axon language constraints to respect while porting

Use language features as they exist today; do **not** silently insert constructs that don’t round-trip through the Axon parser:

- No `while`/`for` in `.ax`. Use recursion or sidecar primitives until added.
- Use `func f() Type` return-type syntax.
- Use `&&`/`||`, not `and`/`or`, for boolean logic in expressions.
- Prefer `bool` FFI returns for branching.
- Whenever a needed feature is missing, add a `// LANG-GAP: …` note and use a sidecar primitive — do **not** change generated `.ax` to invalid syntax.

## Principles

- **One compiler:** the Axon project (this repo) is both the language and the only Axon compiler.
- **Sidecars are temporary muscle:** add them freely while porting; remove them as Axon features land.
- **Tests-first migration:** every ported module ships with parity tests against reference fixtures.
- **Minimal code; `*.test.ax` next to Axon sources.**
