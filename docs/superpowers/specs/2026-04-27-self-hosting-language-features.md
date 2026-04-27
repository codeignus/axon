# Axon Self-Hosting Language Features

## Goal
Make Axon capable of expressing compiler logic natively — loops, lists, enums, string ops, same-module calls — so compiler stages can be written in Axon instead of Rust sidecars.

## Features

### 1. Same-Module Non-Void Function Calls (P0)
Fix the codegen bug where calling a same-module function that returns non-void fails. Root cause: likely symbol declaration mismatch or calling convention issue between the caller and callee in the same LLVM module.

### 2. `for`/`while` Loops (P0)
- `for x in range` and `while cond` blocks
- MIR: lower to basic blocks with back-edge (conditional branch from loop end to loop start)
- Support `break` and `continue`
- No iterators for now — `for` uses integer range `for i in 0..n` or `for item in list`

### 3. Lists (P0)
- Type: `List<T>` — dynamically sized, heap-backed
- Representation: `{ ptr: *mut T, len: u64, cap: u64 }`
- Operations: literal `[a, b, c]`, index `list[i]`, `len(list)`, `push(list, item)`
- Runtime: malloc/realloc/free via libc
- Ownership: list owns its elements, freed on scope exit (string fields freed, scalars skipped)

### 4. String Operations (P1)
- `len(s)` returns string length as u64
- `char_at(s, i)` returns the character at byte offset as a new single-char String
- String concatenation via `+` operator or `concat(a, b)` builtin
- These are enough for a lexer

### 5. Enums with Data (P1)
- Tagged union: `{ tag: u32, payload: union { variant1, variant2, ... } }`
- Construction: `TokenKind.Ident("foo")` or `TokenKind.IntLit("42")`
- Match: `match val { Variant(x) => ... }` extracts payload
- Tag is implicit auto-incrementing u32

### 6. Match Expressions (P1)
- Lower `match val { pattern => body, ... }` to chained conditional branches
- Patterns: ident binding, constructor (enum variant with payload), wildcard `_`, literal
- Exhaustiveness not enforced initially

### 7. FString Interpolation (P2)
- `f"hello {name}"` lowers to string concatenation of literal parts and expression parts

### 8. Compound Assignment (P2)
- `x += 1` lowers to `x = x + 1`
- All compound ops: `+=`, `-=`, `*=`, `/=`, `%=`

## Implementation Layers

Each feature touches up to 4 layers:
1. **MIR types** (`mir.rs`, `types.rs`) — new type/stmt variants
2. **Lowering** (`lower.rs`) — AST → MIR translation
3. **Codegen** (`codegen.rs`) — MIR → LLVM IR generation
4. **Runtime** — libc calls for allocation, string ops

## Order of Implementation

All together, but in dependency sequence:
1. Same-module call fix (diagnose + fix)
2. Loops (MIR blocks with back-edges)
3. Lists (type + literals + index + len + push + runtime)
4. String ops (len, char_at, concat builtins)
5. Enums (tagged union type + construct + match destructuring)
6. Match (pattern lowering to branches)
7. FString (lowering to concat chain)
8. Compound assignment (lowering to assign)
