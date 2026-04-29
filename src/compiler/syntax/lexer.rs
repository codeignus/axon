// Sidecar: unknown-token rejection mirrors `lexer.ax` classify_token.

fn classify_token(trimmed: &str) -> &'static str {
    if trimmed.is_empty() {
        return "empty";
    }
    if matches!(
        trimmed,
        ":=" | "==" | "!=" | "<=" | ">="
    ) {
        return "operator";
    }
    let Some(first) = trimmed.chars().next() else {
        return "empty";
    };
    if first == '"' {
        return "string_literal";
    }
    if first.is_ascii_digit() {
        return "int_literal";
    }
    if matches!(
        first,
        '(' | ')' | '{' | '}' | '[' | ']' | ':' | ',' | '.'
    ) {
        return "delimiter";
    }
    if matches!(
        first,
        '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/'
    ) {
        return "operator";
    }
    if matches!(first, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9') {
        if matches!(
            trimmed,
            "func" | "pub" | "import" | "return" | "if" | "elif" | "else" | "mut"
                | "for" | "while" | "test"
        ) {
            return "keyword";
        }
        if trimmed == "true" || trimmed == "false" {
            return "bool_literal";
        }
        return "identifier";
    }
    "unknown"
}

fn scan_lex_contract(source: &str) -> Result<usize, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut tokens = 0usize;

    let mut bump_pos = |c: char, col: &mut usize, line: &mut usize| {
        if c == '\n' {
            *line += 1;
            *col = 1;
        } else if c != '\r' {
            *col += 1;
        }
    };

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            bump_pos(c, &mut col, &mut line);
            i += 1;
            continue;
        }

        if c == '"' {
            i += 1;
            bump_pos(c, &mut col, &mut line);
            while i < chars.len() {
                if chars[i] == '\\' {
                    bump_pos(chars[i], &mut col, &mut line);
                    i += 1;
                    if i < chars.len() {
                        bump_pos(chars[i], &mut col, &mut line);
                        i += 1;
                    }
                    continue;
                }
                if chars[i] == '"' {
                    bump_pos(chars[i], &mut col, &mut line);
                    i += 1;
                    break;
                }
                bump_pos(chars[i], &mut col, &mut line);
                i += 1;
            }
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            i += 2;
            col += 2;
            while i < chars.len() && chars[i] != '\n' {
                bump_pos(chars[i], &mut col, &mut line);
                i += 1;
            }
            continue;
        }

        let start_line = line;
        let start_col = col;
        let start = i;

        while i < chars.len() {
            let t = chars[i];
            if t.is_whitespace() {
                break;
            }
            if t == '"' {
                break;
            }
            if t == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                break;
            }
            bump_pos(t, &mut col, &mut line);
            i += 1;
        }

        let token: String = chars[start..i].iter().collect();
        tokens += 1;

        if classify_token(&token) == "unknown" {
            return Err(format!(
                "{}:{}: lex: unknown token {:?}",
                start_line, start_col, token
            ));
        }
    }

    Ok(tokens)
}

fn check_file_for_lex(path: &std::path::Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
    if src.contains('\0') {
        return Err(format!(
            "error: NUL byte not allowed in {}",
            path.display()
        ));
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

#[axon_pub_export]
fn run_lex_check(root: &str) -> String {
    let root_path = if root.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(root)
    };
    match walk_and_lex(&root_path) {
        Ok(count) => format!("ok:lexed:{count}"),
        Err(err) => err,
    }
}
