fn lex_token_count(source: &str) -> usize {
    source.split_whitespace().count()
}

fn check_file_for_lex(path: &Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: lexer: cannot read {}: {e}", path.display()))?;
    if src.contains('\0') {
        return Err(format!(
            "error: lexer: NUL byte not allowed in {}",
            path.display()
        ));
    }
    Ok(())
}

fn walk_and_lex(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: lexer: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: lexer: bad dir entry: {e}"))?
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
    format!("tokens:{}", lex_token_count(source))
}

#[axon_export]
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
