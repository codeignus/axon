fn check_file_ownership(path: &Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: ownership: cannot read {}: {e}", path.display()))?;
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("condition_scope_consume(")
            && !path.ends_with("src/compiler/ownership.ax")
        {
            return Err(format!(
                "error: ownership: manual condition consume is forbidden at {}:{}",
                path.display(),
                idx + 1
            ));
        }
        if trimmed.contains("condition_scope_begin(")
            && !path.ends_with("src/compiler/ownership.ax")
        {
            return Err(format!(
                "error: ownership: manual condition scope begin is forbidden at {}:{}",
                path.display(),
                idx + 1
            ));
        }
        if trimmed.contains("dealloc(") || trimmed.contains("free(") {
            return Err(format!(
                "error: ownership: manual deallocation is forbidden at {}:{}",
                path.display(),
                idx + 1
            ));
        }
    }
    Ok(())
}

fn walk_and_check_ownership(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: ownership: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: ownership: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_check_ownership(&path)?;
            continue;
        }
        if is_project_ax_source(&path) {
            check_file_ownership(&path)?;
            checked += 1;
        }
    }
    Ok(checked)
}

#[axon_export]
fn run_ownership_check(source: &str) -> String {
    if source.contains("dealloc(") || source.contains("free(") {
        return "error: ownership: manual deallocation is forbidden".to_string();
    }
    if source.contains("condition_scope_consume(") {
        return "error: ownership: manual condition consume is forbidden".to_string();
    }
    if source.contains("condition_scope_begin(") {
        return "error: ownership: manual condition scope begin is forbidden".to_string();
    }
    "ok:ownership-snippet".to_string()
}

#[axon_export]
fn run_ownership_project_check(root: &str) -> String {
    let root_path = if root.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(root)
    };
    match walk_and_check_ownership(&root_path) {
        Ok(count) => format!("ok:ownership:{count}"),
        Err(err) => err,
    }
}
