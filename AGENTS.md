# AGENTS

## Axon project (repository root)

The **axon project** is only what ships the language pipeline:

- **`build.ax`** — project manifest
- **`src/`** — Axon modules (`*.ax`) and Rust **sidecars** (`*.rs`) for interop only

There is **no `Cargo.toml` at the repo root**. This checkout is **not** a Cargo package.

## Bootstrap compiler (optional, separate subdirectory)

LLVM-backed tooling that compiles Axon sources lives **`only** under **`bootstrap-compiler/`** (its own Cargo workspace). That directory is tooling you run when you need a native `axon` binary; keep it **out** of CI or clone if you insist on “Axon sources only”—or mount it externally and set **`AXON_NATIVE_TOOLCHAIN`**.

First artifact must exist (from any machine that can compile the bootstrap workspace once). Prefer **`axon build`** from repo root once you have **`./target/build/axon/axon`**:

```bash
export AXON_NATIVE_TOOLCHAIN=/path/to/bootstrap-compiler   # Cargo workspace root (can live in /tmp after mv)
./target/build/axon/axon build
```

Only use Cargo directly when you still have **no** `axon` binary yet, e.g. **`cargo run --manifest-path …/Cargo.toml -p axon -- build`** one time.

Axon-only verification (no `cargo` in the script):

```bash
AXON_NATIVE_TOOLCHAIN=/path/to/workspace ./scripts/verify-independent-axon.sh
```

Self-built artifact (still under Axon outputs):

```
./target/build/axon/axon check ""
./target/build/axon/axon build
```

## Native link resolution (`src/compiler/backend/backend.rs`)

Order: **`AXON_NATIVE_TOOLCHAIN`** → **`./bootstrap-compiler/`** (must contain `crates/axon-cli`) → **`./rust-self-compiler-for-axon/`**. If none apply, self-hosted `axon` uses the self-reinvoke path (no Cargo in-tree).

## Verification

```
./scripts/verify-self-bootstrap.sh
```

Uses `bootstrap-compiler/Cargo.toml`, or **`AXON_BOOTSTRAP_MANIFEST`**.

## Axon codegen limitations

See previous team notes: no `while`/`for` in `.ax`; `func f() Type` return syntax; use `&&`/`||`; prefer bool FFI for branching.

## Principles

Minimal code; **`*.test.ax`** next to Axon sources.
