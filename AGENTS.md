# AGENTS

## Axon project (repository root)

The **axon project** is the language **and** the compiler:

- `build.ax` — project manifest.
- `src/` — Axon modules (`*.ax`) and Rust **sidecars** (`*.rs`) under the same project.

There is **no** `Cargo.toml` tracked in this repository. The Axon build system generates `target/cache/app/rust/bridge/Cargo.toml` at build time (gitignored) to compile sidecars.

## Status: self-hosting, zero Cargo manifests

The migration from the historical Rust-backed compiler **into this Axon project** is complete:

- All compiler logic — lexing, parsing, resolution, typechecking, ownership, MIR lowering, codegen orchestration, project graph, command semantics, diagnostics — is owned by `*.ax` files in `src/`.
- Rust survives **only** as `*.rs` sidecars beside `*.ax` for OS/LLVM/process/IO boundaries.
- There is **no** second compiler workspace, no local Rust crate graph, no `src/Cargo.toml`, no `native-rust/`, no `depreciating-soon-compiler-do-not-rename/`.
- The LLVM codegen dependency (`axon-codegen` from `codeignus/axon-rust-compiler`) is declared in `build.ax` under `rust_deps` and pulled via git by the generated bridge Cargo.toml.

## Sidecar policy

Adding `*.rs` sidecars is fine whenever an Axon language feature is missing:

- A sidecar must live **inside `src/`**, alongside the `*.ax` file that calls it.
- A sidecar must expose a **narrow FFI** (file/process/permissions/LLVM/cargo/go) — never a compiler-policy decision.
- Mark every temporary sidecar reach with `// LANG-GAP: <feature> needed; using sidecar X until added`.
- When the matching Axon language feature lands, the sidecar **shrinks or is deleted**.

See `docs/migration/sidecar-allowlist.md` for the closed list of sidecars in this tree.

## Bootstrap policy

This repo contains **zero** Cargo manifests. It is not a Cargo workspace. The Axon build system generates `target/cache/app/rust/bridge/Cargo.toml` at build time (gitignored) to compile sidecars + user `rust_deps`.

To build the compiler from scratch you need exactly one of:

- `AXON_PREBUILT_BIN=/path/to/axon` — any previously produced `axon` binary (e.g. a release artifact or a `target/build/axon/axon` from a different checkout).
- `AXON_BOOTSTRAP_MANIFEST=/path/to/external/Cargo.toml` — a manifest **outside** this repo that produces an `axon` binary.

Point `AXON_PREBUILT_BIN` at any working `axon` (or use an external manifest); the in-tree build never reads a source-controlled `Cargo.toml`.

## Native bring-up (build-time only)

Building the compiler needs:

- `cargo +nightly` (Rust 2024 edition is used until the sidecar bridge moves to stable).
- **LLVM 21** development libraries compatible with `llvm-sys`/inkwell. Set `LLVM_SYS_211_PREFIX` if `llvm-sys` cannot find it on its own.

## Verification

Hand-check the tree as needed, for example:

```bash
axon build   # or ./target/build/axon/axon build after a bootstrap
git ls-files | rg -n 'Cargo\.toml$' || true   # expect no tracked manifests
```

## Native artifact boundary (`src/compiler/backend/backend.rs`)

`backend.rs` is the single sidecar entry point for build artifacts:

- Build: calls `axon_codegen::compile::build` in-process, publishes the artifact to `target/build/<bin>/<bin>` and the install layout `target/build/axon/axon`.
- Run: execute the published binary.
- Test: calls `axon_codegen::compile::run_tests_target` in-process.
- Preserve: copy the install binary to `axon_<suffix>` for self-bootstrap snapshots.

It must never invoke `cargo` against another compiler workspace, and it must never call out to a second `axon` CLI.

## Axon language constraints to respect while porting

Use language features as they exist today; do **not** silently insert constructs that don't round-trip through the Axon parser:

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
- **Zero Cargo manifests:** the repo tracks zero `Cargo.toml` files; the generated bridge is build output.
