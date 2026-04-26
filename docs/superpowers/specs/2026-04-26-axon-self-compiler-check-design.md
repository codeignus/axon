# Axon Self-Compiler Check: Lexer + Minimal Parser

## Goal
Make `compiler.check(path)` in the self-compiler (`axon-lang/src/compiler/entry.ax`) actually parse and validate `.ax` source files using Axon-written logic, with zero dependency on the Rust-hosted compiler pipeline.

## Architecture

```
check(path) flow:
  1. discover source files → @rust fs helpers (list .ax files, read contents)
  2. for each file: lex(source) → token stream (newline-delimited string)
  3. for each file: parse(token_stream) → validate top-level declarations
  4. report errors across all files, return "ok" if clean
```

## Components

### 1. @rust primitives in lexer.ax
Bare char/string operations Axon cannot express natively. Only ASCII initially.

- `char_at(source: String, index: u32) -> String` — single char at position
- `source_len(source: String) -> u32` — byte length
- `is_alpha(ch: String) -> bool` — a-zA-Z_
- `is_digit(ch: String) -> bool` — 0-9
- `is_whitespace(ch: String) -> bool` — space/tab/newline/cr
- `char_eq(a: String, b: String) -> bool` — compare single chars
- `substring(source: String, start: u32, end: u32) -> String` — slice

### 2. Lexer (compiler/syntax/lexer.ax)
- State threaded through functions: `source`, `pos`, accumulated token string
- Scan loop: peek char via `char_at`, classify, advance `pos`, emit token
- Token format: newline-separated string, each line `"kind:value"` (e.g. `"ident:foo\n colon\n ident:bar\n eof:"`)
- Handle: identifiers, keywords (func, pub, import, struct, test, return, if, elif, else, for, while, match, and, or, not, void, String, u32, bool, mut, @rust, @end), numbers, double-quoted strings, operators (`:=`, `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`, `/`, `%`, `->`, `::`), delimiters (`(`, `)`, `{`, `}`, `[`, `]`, `,`, `.`, `:`, `@`), comments (`#`), whitespace/indentation
- Returns full token string on success, or `"error:line N: message"` on invalid token

### 3. Minimal parser (compiler/syntax/parser.ax)
- Receives token string from lexer
- Walks tokens validating top-level declaration structure:
  - `func`, `pub func` — name identifier, params in parens, return type after `:`, body (indented block)
  - `import` — module paths (identifiers separated by `/`, optional `{...}`)
  - `struct` — name, fields in indented block
  - `test` — name, body in indented block
  - `@rust ... @end` — skip content between markers
- Structural checks: balanced parens/braces/brackets, correct keyword positions, no stray tokens at top level
- Returns `"ok"` or `"error:token N: message"`

### 4. File discovery @rust helpers in compiler/proj/discover.ax
- `list_ax_files(dir: String) -> String` — newline-separated list of `.ax` file paths under dir (recursive)
- `read_file(path: String) -> String` — file contents or `"error:..."` if unreadable

### 5. entry.ax check flow update
```
pub func check(path: String) -> String
    files := discover.list_ax_files("src")
    # split files string by newline, iterate
    # for each file: read, lex, parse
    # accumulate errors
    # return "ok" if no errors, else error report
```
Splitting strings by newline needs an @rust helper (`split_lines(s: String) -> String` returns lines joined by a different delimiter, or `count_lines` + `get_line` helpers).

## Constraints
- No Axon arrays/lists — tokens as newline-delimited strings
- No AST tree — parser validates structure directly from token stream
- ASCII-only (byte indexing)
- @rust helpers are bare primitives only (char ops, fs ops), no compiler logic
- Minimal validation: catch syntax errors, not semantic errors (no type checking yet)

## Success Criteria
- `cargo run -p axon -- run` from `axon-lang/` invokes self-compiler which runs `check("")`
- `check("")` discovers all `.ax` files under `src/`, lexes and parses each
- Reports real syntax errors if any exist
- Reports "ok" when the project source is valid
