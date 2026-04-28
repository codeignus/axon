# Map<K,V> Design

## Goal
Add a Go-like `Map<K,V>` type to unblock moving compiler logic, especially semantic symbol tables, from Rust sidecars into Axon. Axon's current generic type syntax is bracket-based, so source code writes `Map[String,String]`.

## Design
`Map<K,V>` is a language-level generic type. Phase 1 supports `String`, integer, and `Bool` keys at the type boundary, with implementation focused first on `Map[String,V]` because compiler symbols, modules, imports, and file paths are string-keyed.

The initial API uses builtin functions rather than method syntax to minimize parser and resolver changes:

```axon
mut m: Map[String,String] = map_new()
map_set(m, "main", "func")
if map_contains(m, "main"):
	value := map_get(m, "main")
```

## End-To-End Behavior
After this phase, Axon code can allocate a map, insert key/value pairs, check whether a key exists, get an inserted value, and ask for map length. This behavior must be exercised by compiler tests, not just represented in MIR types.

## Boundary
The real phase boundary is executable map behavior in Axon programs compiled by the Rust-backed host. LLVM/runtime details may remain in Rust codegen. Compiler logic that uses map operations should live in `.ax` files.

## Representation vs Reality
`Map<K,V>` syntax and MIR types are representation. Real behavior is only complete for the implemented key/value combinations that pass executable tests. Unsupported key types should be rejected or remain unused until implemented.

## Deferred Work
- Method syntax like `m.set("x", value)`.
- Iteration over keys/values.
- Removal.
- Ordered maps.
- Struct keys and trait/protocol-based hashing.
- Fully native Axon hash-table implementation; a linear or runtime-backed implementation is acceptable for bootstrapping.

## First Compiler Use
Start replacing `semantics.rs` logic with Axon code using `Map[String,String]` and simple string encodings for signatures where that keeps the first rewrite small.
