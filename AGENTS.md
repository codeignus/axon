# AGENTS

## Layout
- Axon source: `src/`, `build.ax` (repo root)
- Rust-backed compiler: `rust-backed-compiler-for-axon/` (host compiler, to be deprecated once self-compiler is complete)
- Docs: `docs/`

## Running Commands
Build the axon binary (from repo root):
```
cd rust-backed-compiler-for-axon && cargo run -p axon -- build
```
This produces `target/build/axon/axon` — a native binary you can run directly:
```
target/build/axon/axon check src/main.ax
target/build/axon/axon test
```
The cargo runner is the host compiler (Rust-backed). The output binary is the self-compiled axon compiler.

For quick iteration without rebuilding:
```
cd rust-backed-compiler-for-axon && cargo run -p axon -- <command>
```
CWD must be `rust-backed-compiler-for-axon/` so the host compiler finds `../src/` and `../build.ax`.

## Bootstrap vs Self-Hosted Binary

- **`cargo run -p axon --`** in `rust-backed-compiler-for-axon/` invokes the Rust-backed **host compiler** (frontend, MIR, codegen glue to LLVM and the linker). That build links and runs Axon IR as needed to produce **`target/build/axon/axon`** — the Axon-sources compiler artifact.
- The produced **`axon` binary executes Axon bytecode** emitted for `src/` (CLI, pipeline, backends). Some bridge steps remain host-driven (compilation of emitted Rust/C shims); self-hosted behavior is exercised by `./target/build/axon/axon check|test|build` from the repo root (with CWD/layout as documented above).
- **Contract**: Axon-language semantics and ownership guarantees are enforced via **MIR and codegen**, not `.ax` lex text heuristics. Source-level checks in `ownership.rs` forbid explicit manual freeing or hook APIs outside the sanctioned doc file.

## Axon Codegen Limitations
- No `while`/`for` loops — use recursion
- `->` is not valid Axon return type syntax. Use `func name() Type` not `func name() -> Type`

## Principles
- Minimal working code. No extra abstractions, no over-engineering, no speculative features
- Step-by-step with user. Ask at every decision point
- Tests are colocated: `*.test.ax` next to source files
