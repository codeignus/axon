// String-aware delimiter balance for project-level parse checks.
// `parser.ax`/`check_balance_simple` is a naive count helper only — project checks rely on this.
fn describe_parse_impl(source: &str) -> String {
    let mut stack: Vec<char> = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut in_string = false;
    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            if ch == '\\' {
                i += 2;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
            }
            '(' | '[' | '{' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return "error: mismatched ')'".to_string();
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return "error: mismatched ']'".to_string();
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return "error: mismatched '}'".to_string();
                }
            }
            _ => {}
        }
        i += 1;
    }
    match stack.is_empty() {
        true => "ok".to_string(),
        false => "error: unclosed delimiter".to_string(),
    }
}

#[axon_export]
fn describe_parse(source: &str) -> String {
    describe_parse_impl(source)
}

fn parse_file(path: &std::path::Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
    let result = describe_parse_impl(&src);
    match result.starts_with("error:") {
        true => Err(format!("error: {}: {}", path.display(), result)),
        false => Ok(()),
    }
}

fn walk_and_parse(root: &std::path::Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_parse(&path)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("ax") {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if name.ends_with(".test.ax") {
            continue;
        }
        parse_file(&path)?;
        checked += 1;
    }
    Ok(checked)
}

fn parse_check_message(root: &str) -> String {
    let root_path = match root.is_empty() {
        true => std::path::PathBuf::from("src"),
        false => std::path::PathBuf::from(root),
    };
    match walk_and_parse(&root_path) {
        Ok(count) => format!("ok:parsed:{count}"),
        Err(err) => err,
    }
}

#[axon_pub_export]
fn run_parse_check(root: &str) -> String {
    parse_check_message(root)
}

#[axon_pub_export]
fn parse_stage_failed(root: &str) -> bool {
    parse_check_message(root).starts_with("error")
}
