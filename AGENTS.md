# AGENTS

## Axon project (repository root)

The **axon project** is only what ships the language pipeline:

- **`build.ax`** — project manifest
- **`src/`** — Axon modules (`*.ax`) and Rust **sidecars** (`*.rs`) for interop only

There is **no `Cargo.toml` at the repo root**. This checkout is **not** a Cargo package.

## Bootstrap + migration (LLVM / reference compiler)

LLVM-backed Axon requires **rustc nightly** (`cargo +nightly`) plus **LLVM 21 development libraries** compatible with **`llvm-sys`/inkwell**. Set **`LLVM_SYS_211_PREFIX`** if `llvm-sys` cannot find LLVM on its own.

- **Bootstrap** produces the first `./target/build/axon/axon` binary. Scripts look for **`AXON_BOOTSTRAP_MANIFEST`**, then `bootstrap-compiler/Cargo.toml`, then **`deprecioated-soon-compiler-do-not-rename/Cargo.toml`** on disk (clone the reference checkout next to Axon sources; see `.gitignore` note).
- **`src/compiler/backend/axon_native_build/`** is a **temporary** Cargo package linking **`axon-codegen`** from that reference checkout as a migration bridge. Prefer building it once (`cargo +nightly build …`) so **`AXON_NATIVE_BUILD_BIN`** can point at `./target/native-build-driver/debug/axon-native-build` and skip rebuilds during `axon build`.
- **`src/compiler/backend/backend.rs`** invokes that driver for real native `check`/`build`/`test` (**no subprocess to a second `axon` CLI**). Logic is ported from Rust → `.ax` over time until the Cargo bridge can shrink.

Verification:

```
./scripts/verify-self-bootstrap.sh
```

Uses **`AXON_BOOTSTRAP_MANIFEST`**, then the manifest search paths above.

## Native artifact boundary (`src/compiler/backend/backend.rs`)

Migrating pipeline: FFI entrypoints orchestrate codegen + **`target/build/axon/axon`** layout; codegen still lives largely in **`axon-codegen`** until it is ported into Axon sources and sidecars here.


## Axon codegen limitations

See previous team notes: no `while`/`for` in `.ax`; `func f() Type` return syntax; use `&&`/`||`; prefer bool FFI for branching.

## Principles

Minimal code; **`*.test.ax`** next to Axon sources.
