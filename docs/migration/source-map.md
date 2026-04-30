# Reference Rust → Axon port map

Reference checkout (local, gitignored): `deprecioated-soon-compiler-do-not-rename/`.

This table is the authoritative **first-pass** routing for migration phases in `docs/superpowers/plans/2026-04-30-axon-complete-migration.md`. Update it when splitting or renaming modules.

## Crates overview

| Reference crate | Role | Destination in Axon project |
|---|---|---|
| `crates/axon-types` | tokens, spans, AST, diagnostics, module graph types | `src/compiler/syntax/token.ax`, `ast.ax`, `src/compiler/diagnostics/*`, structural types in `.ax`; tiny serde/FFI shim in `.rs` only if needed transiently |
| `crates/axon-frontend` | lexer, parser, resolver, build.ax loader | `src/compiler/syntax/{lexer,parser}.ax`, `src/compiler/proj/*`, `src/compiler/semantics/resolve.ax`; `lexer.rs`, `parser.rs`, `discover.rs` shrink to IO |
| `crates/axon-typecheck` | type env, inference, unification, checker, ownership summaries | `src/compiler/semantics/{types,check,ownership,lint}.ax`; `semantics.rs`, `ownership.rs` shrink |
| `crates/axon-mir` | MIR model + lowering | `src/compiler/ir/{ir,lower}.ax`; `ir.rs` keeps serialization FFI only |
| `crates/axon-codegen` | prepare, codegen (LLVM), link, bridge_gen, rustc/go compile | policy in `.ax`; **LLVM/objects** → `native_codegen.rs`; **bridge + archives** → `foreign_archive.rs`; **link/exec** stays in `backend.rs` orchestration |
| `crates/axon-runtime` | project metadata, runtime errors, tests harness | `.ax` project/test orchestration + thin `backend.rs` process spawning |
| `crates/axon-cli` | clap dispatcher | Already split: `src/main.ax`, `src/clap.rs`, `tracing.rs` |

## File-level map (axon-codegen ↔ sidecars)

| Reference file | Port target |
|---|---|
| `axon-codegen/src/compile.rs` | `entry.ax` + `backend.rs` + `native_codegen.rs` / `foreign_archive.rs` split |
| `axon-codegen/src/prepare.rs` | `backend/*.ax` + `ir/lower.ax` + `native_codegen.rs` glue |
| `axon-codegen/src/codegen.rs` (LLVM module) | `native_codegen.rs` |
| `axon-codegen/src/linker.rs` | `link.ax` + `backend.rs` |
| `axon-codegen/src/artifacts.rs` | `artifacts.ax` + `backend.rs` |
| `axon-codegen/src/bridge_gen.rs` | Axon FFI policy slices + `foreign_archive.rs` |
| `axon-codegen/src/rust_compile.rs`, `go_compile.rs` | `foreign_archive.rs` |
| `axon-codegen/src/call_resolution.rs` | `.ax` (call/site resolution feeding MIR) |
| `axon-codegen/src/type_marshall.rs` | `.ax` + minimal `foreign_archive.rs` C ABI helpers |
| `axon-codegen/src/cache.rs`, `graph.rs`, `target_resolution.rs` | `proj/*.ax` + `targets.rs` traversal |

## File-level map (axon-frontend)

| Reference file | Port target |
|---|---|
| `axon-frontend/src/lexer/mod.rs`, `lexer/literals.rs` | `syntax/lexer.ax` |
| `axon-frontend/src/parser/*.rs` | `syntax/parser.ax`, `syntax/ast.ax` |
| `axon-frontend/src/resolver/mod.rs` | `semantics/resolve.ax` |
| `axon-frontend/src/build.rs`, `loader.rs` | `proj/build_file.ax`, `discover.ax`, `module_graph.ax` |
| `axon-frontend/src/semantics.rs`, `ffi_validate.rs` | `semantics/check.ax`, `backend/ffi.ax` |

## Temp scaffolding (delete by end of Phase 8)

| Item | Purpose |
|---|---|
| `src/compiler/backend/axon_native_build/` | Links `axon-codegen` until `native_codegen.rs` + `.ax` policy replace it |
| `scripts/parity-run.sh` | Exit-code (and optional strict) parity vs driver during migration |
