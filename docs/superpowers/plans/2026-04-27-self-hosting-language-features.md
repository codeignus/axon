# Self-Hosting Language Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Axon capable of expressing compiler logic natively — fix same-module calls, add loops, lists, string ops, enums, match, fstring, compound assign.

**Architecture:** Each task is a vertical slice touching MIR types → AST lowering → LLVM codegen. Tasks build on each other sequentially since they share files. Tests run via `cargo test -p axon-mir` and `cargo test -p axon-codegen`.

**Tech Stack:** Rust, LLVM (inkwell crate), libc for runtime ops

**Essential Skills:** `test-driven-development`, `golang-testing` (use `make` patterns), `lint-and-validate`, `debugger`

**Global Constraints:**
- All work in `rust-backed-compiler-for-axon/`
- Test commands: `cargo test -p axon-mir -- <test_name>` and `cargo test -p axon-codegen -- <test_name>`
- After ALL tasks: `cargo test --workspace` must pass
- Run `cargo check --workspace` after each task to catch compile errors early
- NEVER use `go test` — this is Rust

**Key File Reference:**
| File | Path | Purpose |
|------|------|---------|
| MIR types | `crates/axon-mir/src/types.rs` | MirType enum (line 3) |
| MIR core | `crates/axon-mir/src/mir.rs` | MirStmt/MirExpr/MirTerminator (lines 68, 144, 171) |
| Lowering | `crates/axon-mir/src/lower.rs` | AST→MIR (lower_stmt line 2103, lower_expr line 2479) |
| Codegen | `crates/axon-codegen/src/codegen.rs` | LLVM backend (compile_stmt line 721, compile_expr line 1395, compile_terminator line 1267, compile_call_target line 1843, basic_type line 3296) |

---

## Task 1: Fix Same-Module Non-Void Function Calls

**Why first:** Unblocks all other features — same-module composition is fundamental.

**Files:**
- Investigate: `crates/axon-codegen/src/codegen.rs:2095-2123` (ModuleEntry call codegen)
- Investigate: `crates/axon-mir/src/lower.rs:3096-3114` (resolve_call_target)
- Investigate: `crates/axon-codegen/src/codegen.rs:217-233` (function declaration)
- Test: `crates/axon-codegen/src/codegen.rs` (add new test)

**Diagnosis steps:**
- [ ] **Step 1:** Write a failing test that calls a same-module non-void function. Create a test in codegen that lowers + compiles:
```rust
// In a new test or existing test module:
// Source: func add(a: i32, b: i32) i32 { return a + b }
//         func main() i32 { return add(1, 2) }
// Expected: compiles and returns 3
```

- [ ] **Step 2:** Run the test to confirm failure
```bash
cargo test -p axon-codegen -- same_module_nonvoid_call
```

- [ ] **Step 3:** Debug the failure. The likely root cause is in `codegen.rs:2095-2123` where `ModuleEntry` calls are compiled. Compare with how `Foreign` calls (which work) are compiled at lines 2124-2361. Check:
  1. Is the callee function declared before the caller? (lines 217-233)
  2. Does the LLVM function type match the call site?
  3. Is the return value properly loaded/stored?
  4. Compare the LLVM IR output for a working cross-module call vs broken same-module call

- [ ] **Step 4:** Implement the fix based on diagnosis

- [ ] **Step 5:** Run test to verify fix
```bash
cargo test -p axon-codegen -- same_module_nonvoid_call
```

- [ ] **Step 6:** Run full test suite
```bash
cargo test --workspace
```

- [ ] **Step 7:** Commit
```bash
git add -A && git commit -m "fix: same-module non-void function calls"
```

---

## Task 2: Add `while` Loops

**Files:**
- Modify: `crates/axon-mir/src/mir.rs` (no new MIR variants needed — loops use existing Branch/Goto terminators)
- Modify: `crates/axon-mir/src/lower.rs:2414` (replace catch-all with `Stmt::While` lowering)
- Modify: `crates/axon-codegen/src/codegen.rs` (no new codegen needed — Branch/Goto already codegen)

**Pattern:** Reference `Stmt::If` lowering at `lower.rs:2271-2363`. A `while` loop is like an `if` that branches back to the condition check at the end of the body.

- [ ] **Step 1:** Write failing test for while loop lowering
```bash
# Test in lower.rs test module or new test file
# Source: func count(n: i32) i32
#           i := 0
#           total := 0
#           while i < n
#               total = total + i
#               i = i + 1
#           return total
```
```bash
cargo test -p axon-mir -- while_loop
```

- [ ] **Step 2:** Add `Stmt::While` lowering in `lower.rs` after the `Stmt::If` arm (after line 2363). Pattern:
```
1. Lower condition expr → cond_local
2. Create blocks: while_cond_N, while_body_N, while_end_N
3. Set current block terminator → Branch(cond, while_body, while_end)
4. Switch to while_body block, lower body stmts
5. Set while_body terminator → Goto(while_cond) [back-edge]
6. Create while_end block, set as current
```

- [ ] **Step 3:** Handle `break` and `continue` in the lowering context:
- `break` → set current block terminator to `Goto(while_end)`, create new dead block
- `continue` → set current block terminator to `Goto(while_cond)`, create new dead block
- Requires tracking the current loop's cond and end labels in `LoweringContext` (add `loop_stack: Vec<(String, String)>` field)

- [ ] **Step 4:** Run test
```bash
cargo test -p axon-mir -- while_loop
```

- [ ] **Step 5:** Write codegen integration test — compile and run a while loop
```bash
cargo test -p axon-codegen -- while_loop_codegen
```

- [ ] **Step 6:** Run full suite
```bash
cargo test --workspace
```

- [ ] **Step 7:** Commit
```bash
git add -A && git commit -m "feat: while loop support"
```

---

## Task 3: Add `for` Loops (integer range)

**Files:**
- Modify: `crates/axon-mir/src/lower.rs:2414` (add `Stmt::For` lowering — desugar to while loop)

**Pattern:** `for i in 0..n` desugars to `i := 0; while i < n { body; i = i + 1 }`. No new MIR types needed.

- [ ] **Step 1:** Write failing test
```bash
cargo test -p axon-mir -- for_range_loop
```

- [ ] **Step 2:** Add `Stmt::For` lowering. The for loop AST has `var`, `iter` (the range expression), `body`. Desugar:
```
1. Evaluate range start and end → alloc locals
2. Alloc loop var local, assign start value
3. Create while-like structure: cond = loop_var < end
4. Lower body with loop_var in scope
5. After body: loop_var = loop_var + 1
6. Back-edge to condition
```

- [ ] **Step 3:** Run tests
```bash
cargo test -p axon-mir -- for_range_loop
cargo test -p axon-codegen -- for_range_loop_codegen
```

- [ ] **Step 4:** Run full suite + commit
```bash
cargo test --workspace
git add -A && git commit -m "feat: for loop (integer range) support"
```

---

## Task 4: Add Lists (List\<T\> type)

**Files:**
- Modify: `crates/axon-mir/src/types.rs` (add `MirType::List(Box<MirType>)`)
- Modify: `crates/axon-mir/src/mir.rs` (add MirStmt/MirExpr variants for list ops)
- Modify: `crates/axon-mir/src/lower.rs` (lower list literals, index, len, push)
- Modify: `crates/axon-codegen/src/codegen.rs` (codegen list ops, declare libc functions)

**Runtime representation:** `{ ptr: *mut T, len: u64, cap: u64 }` — 3-word struct.

- [ ] **Step 1:** Add `MirType::List(Box<MirType>)` to `types.rs` after line 46

- [ ] **Step 2:** Add MIR stmt/expr variants to `mir.rs`:
```
MirStmt::ConstructList { target: LocalId, elements: Vec<MirExpr> }
MirStmt::ListPush { target: LocalId, value: MirExpr }
MirExpr::LoadListLen { source: LocalId }
MirExpr::LoadListIndex { source: LocalId, index: MirExpr, ty: MirType }
```

- [ ] **Step 3:** Write lowering tests
```bash
cargo test -p axon-mir -- list_literal
cargo test -p axon-mir -- list_index
```

- [ ] **Step 4:** Add lowering in `lower.rs`:
- List literal `[a, b, c]`: allocate list local, `ConstructList` with elements
- Index `list[i]`: `LoadListIndex`
- `len(list)` builtin: `LoadListLen`
- `push(list, item)` builtin: `ListPush`

- [ ] **Step 5:** Add codegen for list ops in `codegen.rs`:
- `basic_type` for `MirType::List` → `{ptr, i64, i64}` struct
- `ConstructList`: malloc(capacity * elem_size), store elements, set len/cap
- `LoadListLen`: load the len field (gep index 1)
- `LoadListIndex`: gep ptr + index, load
- `ListPush`: check len < cap, if full realloc(2x), store at ptr[len], len++
- Declare `malloc`, `realloc`, `free` as external LLVM functions

- [ ] **Step 6:** Add list cleanup in ownership system — free list ptr on scope exit for owned locals

- [ ] **Step 7:** Run tests
```bash
cargo test -p axon-mir -- list
cargo test -p axon-codegen -- list
cargo test --workspace
```

- [ ] **Step 8:** Commit
```bash
git add -A && git commit -m "feat: List<T> type with literals, index, len, push"
```

---

## Task 5: Add String Builtins (len, char_at, concat)

**Files:**
- Modify: `crates/axon-codegen/src/codegen.rs:1859-2093` (add new builtin cases)
- Modify: `crates/axon-mir/src/lower.rs:3046-3071` (ensure builtins recognized)

**Runtime:** String is already `{ptr, len}`. Operations:
- `len(s)` → load the len field directly
- `char_at(s, i)` → gep ptr + i, load 1 byte, create new 1-char String
- String `+` (concat) → malloc new buffer, memcpy both, create new String

- [ ] **Step 1:** Write codegen test
```bash
cargo test -p axon-codegen -- string_len
cargo test -p axon-codegen -- string_concat
```

- [ ] **Step 2:** Add `len` builtin codegen — for String type, load the i64 len field (gep index 1). For List type, same pattern. This is at `compile_call_target` around line 1859.

- [ ] **Step 3:** Add `char_at(s, i)` builtin — create new 1-char String from source ptr offset

- [ ] **Step 4:** Add string concatenation — handle `BinOp::Add` where both operands are String type in `compile_binop`. malloc new buffer, memcpy left, memcpy right, create result String `{ptr, new_len}`

- [ ] **Step 5:** Run tests
```bash
cargo test -p axon-codegen -- string_builtins
cargo test --workspace
```

- [ ] **Step 6:** Commit
```bash
git add -A && git commit -m "feat: string len, char_at, concat builtins"
```

---

## Task 6: Add Enums with Data (tagged unions)

**Files:**
- Modify: `crates/axon-mir/src/types.rs` (add `MirType::Enum`)
- Modify: `crates/axon-mir/src/mir.rs` (add enum construct/inspect MIR stmts)
- Modify: `crates/axon-mir/src/lower.rs` (lower enum decl, construction, field access)
- Modify: `crates/axon-codegen/src/codegen.rs` (codegen tagged unions)

**Runtime representation:** Like Rust/Zig — `{ tag: u32, payload: union { V0(fields), V1(fields), ... } }`. Payload size = max variant size.

- [ ] **Step 1:** Add `MirType::Enum { name: String, variants: Vec<EnumVariant> }` and `EnumVariant { name: String, fields: Vec<MirType> }` to `types.rs`

- [ ] **Step 2:** Add MIR stmts to `mir.rs`:
```
MirStmt::ConstructEnum { target, enum_name, variant_idx: u32, fields: Vec<MirExpr> }
MirStmt::InspectEnum { value, variants: Vec<(u32, String)> }  // branch per variant
MirExpr::LoadEnumPayload { source, variant_idx, field_idx, ty }
```

- [ ] **Step 3:** Add lowering in `lower.rs`:
- `Decl::Enum`: register enum info in lowering context (don't skip at line 690)
- `Expr::Constructor` when type is enum: `ConstructEnum` with variant index
- Match on enum: `InspectEnum` + `LoadEnumPayload` per arm

- [ ] **Step 4:** Add codegen in `codegen.rs`:
- `basic_type` for `MirType::Enum` → compute payload union size (max variant), create `{u32, [i8 x N]}` struct
- `ConstructEnum`: store tag + memcpy field data into payload
- `InspectEnum`: load tag, build switch/conditional branches
- `LoadEnumPayload`: gep into payload area, load field

- [ ] **Step 5:** Write tests
```bash
cargo test -p axon-mir -- enum_construction
cargo test -p axon-codegen -- enum_tagged_union
cargo test --workspace
```

- [ ] **Step 6:** Commit
```bash
git add -A && git commit -m "feat: enums with data (tagged unions)"
```

---

## Task 7: Add Match Expressions

**Files:**
- Modify: `crates/axon-mir/src/lower.rs:2414` (add `Stmt::Match` lowering)
- Modify: `crates/axon-codegen/src/codegen.rs` (no new codegen — uses existing Branch + enum ops)

**Pattern:** Lower match to chained if-else (for literals/wildcard) or enum inspect (for enum patterns).

- [ ] **Step 1:** Write failing test
```bash
cargo test -p axon-mir -- match_expression
```

- [ ] **Step 2:** Add `Stmt::Match` lowering:
1. Lower the match value to a local
2. For each arm:
   - If pattern is literal → compare, branch
   - If pattern is constructor (enum variant) → InspectEnum + LoadEnumPayload
   - If pattern is ident → bind value to name
   - If pattern is wildcard → always match
3. Generate: check_block → body_block → end_block chain

- [ ] **Step 3:** Run tests
```bash
cargo test -p axon-mir -- match
cargo test -p axon-codegen -- match_codegen
cargo test --workspace
```

- [ ] **Step 4:** Commit
```bash
git add -A && git commit -m "feat: match expressions with pattern lowering"
```

---

## Task 8: Add FString Interpolation

**Files:**
- Modify: `crates/axon-mir/src/lower.rs:3014` (replace catch-all with `Expr::FString` lowering)

**Pattern:** `f"hello {name}"` lowers to: concat("hello ", to_string(name)). Each `{expr}` part is lowered as a separate expression, results concatenated.

- [ ] **Step 1:** Write failing test
```bash
cargo test -p axon-mir -- fstring_interpolation
```

- [ ] **Step 2:** Add FString lowering — iterate parts, for each literal part create a String local, for each interpolation part lower the expr, then chain concat calls

- [ ] **Step 3:** Run tests + commit
```bash
cargo test -p axon-mir -- fstring
cargo test --workspace
git add -A && git commit -m "feat: f-string interpolation"
```

---

## Task 9: Add Compound Assignment

**Files:**
- Modify: `crates/axon-mir/src/lower.rs:2414` (add `Stmt::CompoundAssign` lowering)

**Pattern:** `x += 1` → load x, add 1, store back. Desugar to simple assignment.

- [ ] **Step 1:** Write failing test
```bash
cargo test -p axon-mir -- compound_assign
```

- [ ] **Step 2:** Add `Stmt::CompoundAssign` lowering — load target, compute with op, store result

- [ ] **Step 3:** Run tests + commit
```bash
cargo test -p axon-mir -- compound_assign
cargo test --workspace
git add -A && git commit -m "feat: compound assignment operators"
```

---

## Task 10: Integration Test — Self-Compiler Lexer in Axon

**Files:**
- Create: `src/compiler/syntax/lexer.ax` (rewrite with real lexer logic using new language features)
- Modify: `src/compiler/syntax/lexer.rs` (remove tokenization logic, keep only file I/O FFI helpers)

**This is the validation task** — use all new features together to write a real lexer in Axon that tokenizes Axon source files.

- [ ] **Step 1:** Write a minimal token type using enum:
```
enum TokenKind
    Ident(name: String)
    IntLit(value: String)
    StringLit(value: String)
    ...
```

- [ ] **Step 2:** Write lexer that uses `for` loop over chars, builds token list, handles indentation

- [ ] **Step 3:** Run `cargo run -p axon -- test` and verify the self-compiler tests pass

- [ ] **Step 4:** Commit
```bash
git add -A && git commit -m "feat: real lexer written in Axon"
```

---

## Dependency Graph

```
Task 1 (same-module fix)
  └→ Task 2 (while loops)
      └→ Task 3 (for loops)
  └→ Task 4 (lists)
  └→ Task 5 (string builtins)
      └→ Task 8 (fstring — uses string concat)
Task 6 (enums)
  └→ Task 7 (match — uses enum inspect)
Tasks 2-9 all → Task 10 (integration)
```

Tasks 4, 5, 6 can run in **parallel** after Task 1.
Task 2 and 3 are sequential (for depends on while pattern).
Task 7 depends on 6 (match uses enum ops).
Task 8 depends on 5 (fstring uses string concat).
Task 9 is independent after Task 1.
