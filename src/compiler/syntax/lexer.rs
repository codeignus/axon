// Sidecar: file I/O + lex pass that mirrors `lexer.ax` (`lex_single_token` / `collect_tokens`).
// Policy (keyword set, operators) stays aligned with Axon lexer; Rust only walks bytes/chars here.

#[inline]
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn keyword_kind(text: &str) -> Option<&'static str> {
    match text {
        "func" => Some("kw_func"),
        "method" => Some("kw_method"),
        "type" => Some("kw_type"),
        "struct" => Some("kw_struct"),
        "enum" => Some("kw_enum"),
        "trait" => Some("kw_trait"),
        "error" => Some("kw_error"),
        "test" => Some("kw_test"),
        "mut" => Some("kw_mut"),
        "if" => Some("kw_if"),
        "elif" => Some("kw_elif"),
        "else" => Some("kw_else"),
        "for" => Some("kw_for"),
        "in" => Some("kw_in"),
        "while" => Some("kw_while"),
        "break" => Some("kw_break"),
        "continue" => Some("kw_continue"),
        "match" => Some("kw_match"),
        "return" => Some("kw_return"),
        "ref" => Some("kw_ref"),
        "async" => Some("kw_async"),
        "await" => Some("kw_await"),
        "shared" => Some("kw_shared"),
        "buffer" => Some("kw_buffer"),
        "defer" => Some("kw_defer"),
        "errdefer" => Some("kw_errdefer"),
        "self" => Some("kw_self"),
        "and" => Some("kw_and"),
        "or" => Some("kw_or"),
        "not" => Some("kw_not"),
        "nil" => Some("kw_nil"),
        "try" => Some("kw_try"),
        "catch" => Some("kw_catch"),
        "orelse" => Some("kw_orelse"),
        "ordefault" => Some("kw_ordefault"),
        "import" => Some("kw_import"),
        "include" => Some("kw_include"),
        "pub" => Some("kw_pub"),
        "project" => Some("kw_project"),
        "bin" => Some("kw_bin"),
        "deps" => Some("kw_deps"),
        "rust" => Some("kw_rust"),
        "rust_deps" => Some("kw_rust_deps"),
        "go" => Some("kw_go"),
        "go_deps" => Some("kw_go_deps"),
        "python_deps" => Some("kw_python_deps"),
        "end" => Some("kw_end"),
        _ => None,
    }
}

fn peek_byte(chars: &[char], pos: usize) -> Option<u8> {
    chars.get(pos).and_then(|c| {
        let mut buf = [0u8; 4];
        c.encode_utf8(&mut buf);
        buf.first().copied()
    })
}

fn line_col_at_byte(source: &str, byte_off: usize) -> (usize, usize) {
    let slice = source.get(..byte_off.min(source.len())).unwrap_or("");
    let mut line = 1usize;
    let mut col = 1usize;
    for ch in slice.chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn char_idx_byte_offset(chars: &[char], char_idx: usize) -> usize {
    chars[..char_idx].iter().map(|c| c.len_utf8()).sum()
}

/// Returns (token_kind_or_error, exclusive end **char index**).
fn lex_single(chars: &[char], pos: usize, source: &str) -> Result<(String, usize), String> {
    let len = chars.len();
    if pos >= len {
        return Ok(("eof".to_string(), pos));
    }
    let c = chars[pos];

    // skip spaces/tabs (match `lex_skip_whitespace` recurse)
    if c == ' ' || c == '\t' {
        let mut p = pos;
        while p < len && (chars[p] == ' ' || chars[p] == '\t') {
            p += 1;
        }
        return lex_single(chars, p, source);
    }

    if c == '\n' {
        return Ok(("newline".to_string(), pos + 1));
    }
    if c == '\r' {
        if pos + 1 < len && chars[pos + 1] == '\n' {
            return Ok(("newline".to_string(), pos + 2));
        }
        return Ok(("newline".to_string(), pos + 1));
    }

    if c == '"' {
        let mut p = pos + 1;
        while p < len {
            match chars[p] {
                '\\' => {
                    p = p.saturating_add(2);
                    continue;
                }
                '"' => {
                    p += 1;
                    return Ok(("string".to_string(), p));
                }
                _ => p += 1,
            }
        }
        return Ok(("error".to_string(), len));
    }

    if c == '\'' {
        let mut p = pos + 1;
        if p < len && chars[p] == '\\' {
            p = p.saturating_add(2);
        } else if p < len {
            p += 1;
        }
        if p < len && chars[p] == '\'' {
            p += 1;
        }
        return Ok(("char".to_string(), p));
    }

    if c == '/' && pos + 1 < len {
        let nxt = chars[pos + 1];
        if nxt == '/' {
            let mut p = pos + 2;
            while p < len && chars[p] != '\n' {
                p += 1;
            }
            return Ok(("comment".to_string(), p));
        }
        if nxt == '*' {
            let mut p = pos + 2;
            while p < len {
                if chars[p] == '*' && p + 1 < len && chars[p + 1] == '/' {
                    p += 2;
                    return Ok(("comment".to_string(), p));
                }
                p += 1;
            }
            return Ok(("error".to_string(), len));
        }
    }

    if peek_byte(chars, pos).is_some_and(|b| (b'0'..=b'9').contains(&b)) {
        let mut p = pos;
        while p < len {
            let b = peek_byte(chars, p).unwrap_or(0);
            if !(b'0'..=b'9').contains(&b) {
                break;
            }
            p += 1;
        }
        if p < len && chars[p] == '.' {
            let next = p + 1;
            if next < len {
                let nb = peek_byte(chars, next).unwrap_or(0);
                if (b'0'..=b'9').contains(&nb) {
                    p = next;
                    while p < len {
                        let b = peek_byte(chars, p).unwrap_or(0);
                        if !(b'0'..=b'9').contains(&b) {
                            break;
                        }
                        p += 1;
                    }
                    return Ok(("float".to_string(), p));
                }
            }
        }
        return Ok(("int".to_string(), p));
    }

    if c.is_ascii_alphabetic() || c == '_' {
        let mut p = pos;
        while p < len && is_ident_continue(chars[p]) {
            p += 1;
        }
        let text: String = chars[pos..p].iter().collect();
        if let Some(kind) = keyword_kind(&text) {
            let k = if text == "true" || text == "false" {
                "bool".to_string()
            } else {
                kind.to_string()
            };
            return Ok((k, p));
        }
        if text == "true" || text == "false" {
            return Ok(("bool".to_string(), p));
        }
        if text == "nil" {
            return Ok(("kw_nil".to_string(), p));
        }
        return Ok(("ident".to_string(), p));
    }

    // operators / punctuation (mirror `lex_operator`)
    match c {
        ':' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("colon_eq".to_string(), pos + 2));
            }
            return Ok(("colon".to_string(), pos + 1));
        }
        '=' => {
            if pos + 1 < len && chars[pos + 1] == '>' {
                return Ok(("fat_arrow".to_string(), pos + 2));
            }
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("eq".to_string(), pos + 2));
            }
            return Ok(("assign".to_string(), pos + 1));
        }
        '!' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("ne".to_string(), pos + 2));
            }
            return Ok(("bang".to_string(), pos + 1));
        }
        '<' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("le".to_string(), pos + 2));
            }
            return Ok(("lt".to_string(), pos + 1));
        }
        '>' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("ge".to_string(), pos + 2));
            }
            return Ok(("gt".to_string(), pos + 1));
        }
        '+' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("plus_eq".to_string(), pos + 2));
            }
            return Ok(("plus".to_string(), pos + 1));
        }
        '-' => {
            if pos + 1 < len && chars[pos + 1] == '=' {
                return Ok(("minus_eq".to_string(), pos + 2));
            }
            return Ok(("minus".to_string(), pos + 1));
        }
        '*' => return Ok(("star".to_string(), pos + 1)),
        '/' => return Ok(("slash".to_string(), pos + 1)),
        '%' => return Ok(("percent".to_string(), pos + 1)),
        '&' => {
            if pos + 1 < len && chars[pos + 1] == '&' {
                return Ok(("andand".to_string(), pos + 2));
            }
            return Ok(("amp".to_string(), pos + 1));
        }
        '|' => {
            if pos + 1 < len && chars[pos + 1] == '|' {
                return Ok(("oror".to_string(), pos + 2));
            }
            return Ok(("pipe".to_string(), pos + 1));
        }
        '.' => {
            if pos + 1 < len && chars[pos + 1] == '.' {
                return Ok(("dot_dot".to_string(), pos + 2));
            }
            return Ok(("dot".to_string(), pos + 1));
        }
        ',' => return Ok(("comma".to_string(), pos + 1)),
        ';' => return Ok(("semi".to_string(), pos + 1)),
        '?' => return Ok(("question".to_string(), pos + 1)),
        '(' => return Ok(("lparen".to_string(), pos + 1)),
        ')' => return Ok(("rparen".to_string(), pos + 1)),
        '[' => return Ok(("lbrack".to_string(), pos + 1)),
        ']' => return Ok(("rbrack".to_string(), pos + 1)),
        '{' => return Ok(("lbrace".to_string(), pos + 1)),
        '}' => return Ok(("rbrace".to_string(), pos + 1)),
        '@' => return Ok(("at".to_string(), pos + 1)),
        _ => {
            let byte = char_idx_byte_offset(chars, pos);
            let (line, col) = line_col_at_byte(source, byte);
            return Err(format!(
                "{}:{}: lex: error token {:?}",
                line,
                col,
                chars[pos]
            ));
        }
    }
}

fn collect_tokens(chars: &[char], source: &str) -> Result<usize, String> {
    let mut pos = 0usize;
    let mut tokens = 0usize;
    loop {
        let (kind, end) = lex_single(chars, pos, source)?;
        if kind == "error" {
            let byte = char_idx_byte_offset(chars, pos);
            let (line, col) = line_col_at_byte(source, byte);
            return Err(format!(
                "{}:{}: lex: unterminated string or comment",
                line, col
            ));
        }
        tokens += 1;
        if kind == "eof" {
            break;
        }
        if end <= pos && kind != "eof" {
            let byte = char_idx_byte_offset(chars, pos);
            let (line, col) = line_col_at_byte(source, byte);
            return Err(format!(
                "{}:{}: lex: lexer made no progress",
                line,
                col
            ));
        }
        pos = end;
    }
    Ok(tokens)
}

fn scan_lex_contract(source: &str) -> Result<usize, String> {
    let chars: Vec<char> = source.chars().collect();
    collect_tokens(&chars, source)
}

fn check_file_for_lex(path: &std::path::Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
    if src.contains('\0') {
        return Err(format!("error: NUL byte not allowed in {}", path.display()));
    }
    scan_lex_contract(&src)
        .map(|_| ())
        .map_err(|detail| format!("error: {}: {}", path.display(), detail))
}

fn walk_and_lex(root: &std::path::Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_lex(&path)?;
            continue;
        }
        if is_project_ax_source(&path) {
            check_file_for_lex(&path)?;
            checked += 1;
        }
    }
    Ok(checked)
}

#[axon_export]
fn describe_tokenization(source: &str) -> String {
    match scan_lex_contract(source) {
        Ok(n) => format!("tokens:{}", n),
        Err(_) => "tokens:error".to_string(),
    }
}

fn lex_check_message(root: &str) -> String {
    let root_path = match root.is_empty() {
        true => project_entry_root_path(),
        false => std::path::PathBuf::from(root),
    };
    match walk_and_lex(&root_path) {
        Ok(count) => format!("ok:lexed:{count}"),
        Err(err) => err,
    }
}

#[axon_pub_export]
fn run_lex_check(root: &str) -> String {
    lex_check_message(root)
}

#[axon_pub_export]
fn lex_stage_failed(root: &str) -> bool {
    lex_check_message(root).starts_with("error")
}
