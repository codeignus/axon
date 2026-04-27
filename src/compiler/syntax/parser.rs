fn parse_balance(source: &str) -> Result<(), String> {
    let mut stack: Vec<char> = Vec::new();
    for ch in source.chars() {
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return Err("mismatched ')'".to_string());
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return Err("mismatched ']'".to_string());
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return Err("mismatched '}'".to_string());
                }
            }
            _ => {}
        }
    }
    if stack.is_empty() {
        Ok(())
    } else {
        Err("unclosed delimiter".to_string())
    }
}

fn parse_file(path: &Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: parser: cannot read {}: {e}", path.display()))?;
    parse_balance(&src).map_err(|e| format!("error: parser: {}: {e}", path.display()))
}

fn walk_and_parse(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: parser: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: parser: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_parse(&path)?;
            continue;
        }
        if is_project_ax_source(&path) {
            parse_file(&path)?;
            checked += 1;
        }
    }
    Ok(checked)
}

#[axon_export]
fn describe_parse(source: &str) -> String {
    match parse_balance(source) {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("error: parser: {e}"),
    }
}

#[axon_export]
fn run_parse_check(root: &str) -> String {
    let root_path = if root.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(root)
    };
    match walk_and_parse(&root_path) {
        Ok(count) => format!("ok:parsed:{count}"),
        Err(err) => err,
    }
}
