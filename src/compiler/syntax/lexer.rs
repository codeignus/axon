// Full lexer: indent/dedent, parens/brackets/braces depth, `@rust`|`@go` + `@end` raw,
// `f"..."`, numeric `_`, newline-in-string error. Mirrors deprecioated axon-frontend lexer.

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

fn lex_err(source: &str, byte_off: usize, msg: impl AsRef<str>) -> String {
    let (l, c) = line_col_at_byte(source, byte_off.min(source.len()));
    format!("{}:{}: lex: {}", l, c, msg.as_ref())
}

fn esc_pipe(s: &str) -> String {
    s.replace('|', "\\u007c")
}

fn tok(kind: &str, text: &str, sb: usize, eb: usize) -> String {
    format!("{}|{}|{}|{}", kind, esc_pipe(text), sb, eb)
}

fn tok_kind(line: &str) -> &str {
    line.split('|').next().unwrap_or("")
}

fn kw_kind(ident: &str) -> &'static str {
    match ident {
        "func" => "kw_func",
        "method" => "kw_method",
        "type" => "kw_type",
        "struct" => "kw_struct",
        "enum" => "kw_enum",
        "trait" => "kw_trait",
        "error" => "kw_error",
        "test" => "kw_test",
        "mut" => "kw_mut",
        "if" => "kw_if",
        "elif" => "kw_elif",
        "else" => "kw_else",
        "for" => "kw_for",
        "in" => "kw_in",
        "while" => "kw_while",
        "break" => "kw_break",
        "continue" => "kw_continue",
        "match" => "kw_match",
        "return" => "kw_return",
        "ref" => "kw_ref",
        "async" => "kw_async",
        "await" => "kw_await",
        "shared" => "kw_shared",
        "buffer" => "kw_buffer",
        "defer" => "kw_defer",
        "errdefer" => "kw_errdefer",
        "true" | "false" => "bool",
        "self" => "kw_self",
        "and" => "kw_and",
        "or" => "kw_or",
        "not" => "kw_not",
        "nil" => "kw_nil",
        "try" => "kw_try",
        "catch" => "kw_catch",
        "orelse" => "kw_orelse",
        "ordefault" => "kw_ordefault",
        "import" => "kw_import",
        "include" => "kw_include",
        "pub" => "kw_pub",
        "project" => "kw_project",
        "bin" => "kw_bin",
        "deps" => "kw_deps",
        "rust_deps" => "kw_rust_deps",
        "go_deps" => "kw_go_deps",
        "python_deps" => "kw_python_deps",
        "rust" => "kw_rust",
        "go" => "kw_go",
        "end" => "kw_end",
        _ => "ident",
    }
}

struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    byte_off: Vec<usize>,
    pos: usize,
    indent_stack: Vec<u32>,
    paren_depth: u32,
    at_line_start: bool,
    pending: Vec<String>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        let mut chars = Vec::new();
        let mut byte_off = Vec::new();
        let mut b = 0usize;
        for c in source.chars() {
            byte_off.push(b);
            b += c.len_utf8();
            chars.push(c);
        }
        Self {
            source,
            chars,
            byte_off,
            pos: 0,
            indent_stack: vec![0],
            paren_depth: 0,
            at_line_start: true,
            pending: Vec::new(),
        }
    }

    fn b_ci(&self, ci: usize) -> usize {
        *self.byte_off.get(ci.min(self.chars.len())).unwrap_or(&self.source.len())
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.advance();
        }
    }

    fn count_indent(&mut self) -> u32 {
        let mut n: u32 = 0;
        loop {
            match self.peek() {
                Some(' ') => {
                    self.advance();
                    n += 1;
                }
                Some('\t') => {
                    self.advance();
                    n = (n / 4 + 1) * 4;
                }
                _ => break,
            }
        }
        n
    }

    fn handle_line_start(&mut self) -> Result<(), String> {
        if self.paren_depth > 0 {
            self.skip_inline_ws();
            self.at_line_start = false;
            return Ok(());
        }
        loop {
            if self.pos >= self.chars.len() {
                self.at_line_start = false;
                return Ok(());
            }
            if self.peek() == Some('\n') {
                self.advance();
                continue;
            }
            self.skip_inline_ws();
            if self.peek() == Some('/') && self.peek_at(1) == Some('/') {
                let sb = self.b_ci(self.pos);
                self.advance();
                self.advance();
                while self.peek() != None && self.peek() != Some('\n') {
                    self.advance();
                }
                let eb = self.b_ci(self.pos);
                let snippet = self
                    .source
                    .get(sb.min(self.source.len())..eb.min(self.source.len()))
                    .unwrap_or("")
                    .trim();
                self.pending.push(tok("comment", snippet, sb, eb));
                continue;
            }
            break;
        }

        let indent = self.count_indent();
        let cur = *self.indent_stack.last().unwrap();
        if indent > cur {
            self.indent_stack.push(indent);
            let eb = self.b_ci(self.pos);
            self.pending.push(tok("indent", "", eb, eb));
        } else {
            while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > indent {
                self.indent_stack.pop();
                let eb = self.b_ci(self.pos);
                self.pending.push(tok("dedent", "", eb, eb));
            }
            if *self.indent_stack.last().unwrap() != indent {
                let eb = self.b_ci(self.pos);
                self.pending.push(tok("error", "inconsistent indentation", eb, eb));
                self.indent_stack.push(indent);
            }
        }
        self.at_line_start = false;
        Ok(())
    }

    fn read_string(&mut self, open_sb: usize) -> Result<String, String> {
        self.advance(); // "
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    let eb = self.b_ci(self.pos);
                    let txt = self
                        .source
                        .get(open_sb.min(self.source.len())..eb.min(self.source.len()))
                        .unwrap_or("");
                    return Ok(tok("string", txt, open_sb, eb));
                }
                Some('\\') => {
                    self.advance();
                    if self.advance().is_none() {
                        return Err(lex_err(self.source, open_sb, "unterminated string"));
                    }
                }
                Some('\n') | None => {
                    return Err(lex_err(self.source, open_sb, "unterminated string"));
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
    }

    fn read_fstring(&mut self, outer_sb: usize) -> Result<String, String> {
        self.advance(); // f
        if self.advance() != Some('"') {
            return Err(lex_err(self.source, outer_sb, "invalid f-string"));
        }
        let mut depth = 0i32;
        loop {
            match self.peek() {
                Some('"') if depth == 0 => {
                    self.advance();
                    let eb = self.b_ci(self.pos);
                    let raw = self
                        .source
                        .get(outer_sb.min(self.source.len())..eb.min(self.source.len()))
                        .unwrap_or("");
                    return Ok(tok("fstring_start", raw, outer_sb, eb));
                }
                Some('{') => {
                    depth += 1;
                    self.advance();
                }
                Some('}') => {
                    depth = depth.saturating_sub(1);
                    self.advance();
                }
                Some('\\') => {
                    self.advance();
                    if self.advance().is_none() {
                        return Err(lex_err(self.source, outer_sb, "unterminated f-string"));
                    }
                }
                Some(_) => {
                    self.advance();
                }
                None => return Err(lex_err(self.source, outer_sb, "unterminated f-string")),
            }
        }
    }

    fn read_number(&mut self, outer_sb: usize) -> Result<String, String> {
        let start_c = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        let end_b = self.b_ci(self.pos);
        let raw: String = self.chars[start_c..self.pos].iter().collect();
        let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
        if is_float {
            Ok(tok("float", &cleaned, outer_sb, end_b))
        } else {
            Ok(tok("int", &cleaned, outer_sb, end_b))
        }
    }

    fn read_identifier(&mut self, outer_sb: usize) -> String {
        let sc = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let eb = self.b_ci(self.pos);
        let txt: String = self.chars[sc..self.pos].iter().collect();
        let k = kw_kind(&txt);
        tok(k, &txt, outer_sb, eb)
    }

    fn read_line_comment_here(&mut self, sb: usize) -> String {
        while self.peek() != None && self.peek() != Some('\n') {
            self.advance();
        }
        let eb = self.b_ci(self.pos);
        let snippet = self
            .source
            .get(sb.min(self.source.len())..eb.min(self.source.len()))
            .unwrap_or("")
            .trim();
        tok("comment", snippet, sb, eb)
    }

    fn read_block_comment(&mut self, sb: usize) -> Result<String, String> {
        self.advance(); // /
        self.advance(); // *
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some('*'), Some('/')) => {
                    self.advance();
                    self.advance();
                    let eb = self.b_ci(self.pos);
                    let t = self.source.get(sb..eb.min(self.source.len())).unwrap_or("");
                    return Ok(tok("comment", t, sb, eb));
                }
                (Some(_), _) => self.advance(),
                (None, _) => return Err(lex_err(self.source, sb, "unterminated block comment")),
            }
        }
    }

    fn read_char(&mut self, outer_sb: usize) -> Result<String, String> {
        self.advance(); // '
        match self.peek() {
            Some('\\') => {
                self.advance();
                if self.advance().is_none() {
                    return Err(lex_err(self.source, outer_sb, "unterminated char literal"));
                }
            }
            Some('\'') => {
                return Err(lex_err(self.source, outer_sb, "empty char literal"));
            }
            Some(_) => {
                self.advance();
            }
            None => {
                return Err(lex_err(self.source, outer_sb, "unterminated char literal"));
            }
        }
        if self.peek() == Some('\'') {
            self.advance();
        }
        let eb = self.b_ci(self.pos);
        let raw = self
            .source
            .get(outer_sb.min(self.source.len())..eb.min(self.source.len()))
            .unwrap_or("");
        Ok(tok("char", raw, outer_sb, eb))
    }

    fn capture_raw_until_end(&mut self, content_sb: usize) -> Result<String, String> {
        loop {
            let b = self.b_ci(self.pos);
            let suffix = self.source.get(b.min(self.source.len())..).unwrap_or("");
            if suffix.starts_with("@end")
                && (b == 0 || self.source.as_bytes().get(b.wrapping_sub(1)).map_or(false, |x| *x == b'\n'))
            {
                let at_end_b = b;
                for expect in "@end".chars() {
                    if Some(expect) != self.peek() {
                        return Err(lex_err(self.source, b, "@end malformed"));
                    }
                    self.advance();
                }
                self.skip_inline_ws();
                let end_byte = self.b_ci(self.pos);
                let inner = self
                    .source
                    .get(content_sb.min(self.source.len())..at_end_b.min(self.source.len()))
                    .unwrap_or("")
                    .strip_prefix('\n')
                    .unwrap_or("")
                    .trim_end();
                return Ok(tok(
                    "raw_block",
                    inner,
                    content_sb.min(self.source.len()),
                    end_byte.min(self.source.len()),
                ));
            }
            if self.pos >= self.chars.len() {
                return Err(lex_err(
                    self.source,
                    content_sb,
                    "expected @end to close @rust block",
                ));
            }
            self.advance();
        }
    }

    fn lex_at_kw(&mut self) -> String {
        let at_sb_byte = self.b_ci(self.pos);
        self.advance(); // @
        let after_at_byte = self.b_ci(self.pos);
        let kw_sc = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let kw_eb_byte = self.b_ci(self.pos);
        let txt: String = self.chars[kw_sc..self.pos].iter().collect();
        let kind = kw_kind(&txt);
        // Emit order: `@` then keyword (matching reference lexer queued tokens).
        self.pending.push(tok(kind, &txt, after_at_byte, kw_eb_byte));
        tok("at", "@", at_sb_byte, after_at_byte)
    }

    fn next_token(&mut self) -> Result<String, String> {
        if !self.pending.is_empty() {
            return Ok(self.pending.remove(0));
        }
        if self.at_line_start {
            self.handle_line_start()?;
            if !self.pending.is_empty() {
                return Ok(self.pending.remove(0));
            }
        }

        let sb = self.b_ci(self.pos);
        match self.peek() {
            None => {
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    let eb = self.b_ci(self.pos.min(self.chars.len()));
                    self.pending.push(tok("dedent", "", eb, eb));
                }
                if !self.pending.is_empty() {
                    return Ok(self.pending.remove(0));
                }
                Ok(tok("eof", "", sb, sb))
            }
            Some('\n') => {
                self.advance();
                if self.paren_depth > 0 {
                    self.skip_inline_ws();
                    return self.next_token();
                }
                self.at_line_start = true;
                Ok(tok("newline", "\n", sb, self.b_ci(self.pos)))
            }
            Some('\r') => {
                if self.peek_at(1) == Some('\n') {
                    self.advance();
                    self.advance();
                } else {
                    self.advance();
                }
                if self.paren_depth > 0 {
                    self.skip_inline_ws();
                    return self.next_token();
                }
                self.at_line_start = true;
                Ok(tok("newline", "\n", sb, self.b_ci(self.pos)))
            }
            Some(' ') | Some('\t') => {
                self.skip_inline_ws();
                self.next_token()
            }
            Some('/') if self.peek_at(1) == Some('/') => {
                Ok(self.read_line_comment_here(sb))
            }
            Some('/') if self.peek_at(1) == Some('*') => self.read_block_comment(sb),
            Some('"') => self.read_string(sb),
            Some('f') if self.peek_at(1) == Some('"') => self.read_fstring(sb),
            Some('\'') => self.read_char(sb),
            Some(ch) if ch.is_ascii_digit() => self.read_number(sb),
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => Ok(self.read_identifier(sb)),
            Some('@') => Ok(self.lex_at_kw()),
            Some('(') => {
                self.advance();
                self.paren_depth += 1;
                Ok(tok("lparen", "(", sb, self.b_ci(self.pos)))
            }
            Some(')') => {
                self.advance();
                self.paren_depth = self.paren_depth.saturating_sub(1);
                Ok(tok("rparen", ")", sb, self.b_ci(self.pos)))
            }
            Some('[') => {
                self.advance();
                self.paren_depth += 1;
                Ok(tok("lbrack", "[", sb, self.b_ci(self.pos)))
            }
            Some(']') => {
                self.advance();
                self.paren_depth = self.paren_depth.saturating_sub(1);
                Ok(tok("rbrack", "]", sb, self.b_ci(self.pos)))
            }
            Some('{') => {
                self.advance();
                self.paren_depth += 1;
                Ok(tok("lbrace", "{", sb, self.b_ci(self.pos)))
            }
            Some('}') => {
                self.advance();
                self.paren_depth = self.paren_depth.saturating_sub(1);
                Ok(tok("rbrace", "}", sb, self.b_ci(self.pos)))
            }
            Some(':') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("colon_eq", ":=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("colon", ":", sb, self.b_ci(self.pos)))
                }
            }
            Some('=') => {
                let t = if self.peek_at(1) == Some('>') {
                    self.advance();
                    self.advance();
                    tok("fat_arrow", "=>", sb, self.b_ci(self.pos))
                } else if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    tok("eq", "==", sb, self.b_ci(self.pos))
                } else {
                    self.advance();
                    tok("assign", "=", sb, self.b_ci(self.pos))
                };
                Ok(t)
            }
            Some('!') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("ne", "!=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("bang", "!", sb, self.b_ci(self.pos)))
                }
            }
            Some('?') => {
                self.advance();
                Ok(tok("question", "?", sb, self.b_ci(self.pos)))
            }
            Some('<') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("le", "<=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("lt", "<", sb, self.b_ci(self.pos)))
                }
            }
            Some('>') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("ge", ">=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("gt", ">", sb, self.b_ci(self.pos)))
                }
            }
            Some('-') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("minus_eq", "-=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("minus", "-", sb, self.b_ci(self.pos)))
                }
            }
            Some('+') => {
                if self.peek_at(1) == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(tok("plus_eq", "+=", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("plus", "+", sb, self.b_ci(self.pos)))
                }
            }
            Some('*') => {
                self.advance();
                Ok(tok("star", "*", sb, self.b_ci(self.pos)))
            }
            Some('/') => {
                self.advance();
                Ok(tok("slash", "/", sb, self.b_ci(self.pos)))
            }
            Some('%') => {
                self.advance();
                Ok(tok("percent", "%", sb, self.b_ci(self.pos)))
            }
            Some('&') => {
                if self.peek_at(1) == Some('&') {
                    self.advance();
                    self.advance();
                    Ok(tok("andand", "&&", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("amp", "&", sb, self.b_ci(self.pos)))
                }
            }
            Some('|') => {
                if self.peek_at(1) == Some('|') {
                    self.advance();
                    self.advance();
                    Ok(tok("oror", "||", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("pipe", "|", sb, self.b_ci(self.pos)))
                }
            }
            Some('.') => {
                if self.peek_at(1) == Some('.') {
                    self.advance();
                    self.advance();
                    Ok(tok("dot_dot", "..", sb, self.b_ci(self.pos)))
                } else {
                    self.advance();
                    Ok(tok("dot", ".", sb, self.b_ci(self.pos)))
                }
            }
            Some(',') => {
                self.advance();
                Ok(tok("comma", ",", sb, self.b_ci(self.pos)))
            }
            Some(';') => {
                self.advance();
                Ok(tok("semi", ";", sb, self.b_ci(self.pos)))
            }
            Some(ch) => Err(lex_err(
                self.source,
                sb,
                format!("error token {:?}", ch),
            )),
        }
    }

    fn tokenize(mut self) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        loop {
            let tok_line = self.next_token()?;
            let kind = tok_kind(&tok_line);

            let is_eof = kind == "eof";
            let mut check_raw = kind == "kw_rust" || kind == "kw_go";
            if kind == "at" && !self.pending.is_empty() {
                let pk = tok_kind(&self.pending[0]);
                check_raw = pk == "kw_rust" || pk == "kw_go";
            }

            out.push(tok_line);

            while !self.pending.is_empty() {
                let follow = self.pending.remove(0);
                let fk = tok_kind(&follow);
                out.push(follow);
                if check_raw && (fk == "kw_rust" || fk == "kw_go") {
                    let content_sb = self.b_ci(self.pos);
                    if self.peek() == Some('\n') {
                        self.advance();
                        self.at_line_start = false;
                    }
                    out.push(self.capture_raw_until_end(content_sb)?);
                    check_raw = false;
                }
            }

            if is_eof {
                break;
            }
        }
        Ok(out)
    }
}

/// FFI for Axon callers: newline-separated tokens (kind|text|start_byte|end_byte).
#[axon_export]
fn axon_lex_token_stream(source: &str) -> String {
    match Lexer::new(source).tokenize() {
        Ok(v) => {
            if v.iter().any(|t| tok_kind(t) == "error") {
                return v.join("\n");
            }
            v.join("\n")
        }
        Err(e) => format!("error|{}|0|0", esc_pipe(&e)),
    }
}

fn scan_lex_contract(source: &str) -> Result<usize, String> {
    let out = Lexer::new(source).tokenize()?;
    if let Some(bad) = out.iter().find(|t| tok_kind(t) == "error") {
        return Err(bad.clone());
    }
    Ok(out.len())
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
