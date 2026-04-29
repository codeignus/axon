// Project-wide ownership scan (directory walk). Lives beside `ownership.ax` so the
// symbol shares module `compiler` with `validate_ownership_invariants` without
// importing the whole `compiler/semantics` graph.
//
// Policy-only checks live here (forbidden APIs). Conditional `mut`/merge behavior is
// not implemented as line-scan heuristics: `mut`/`non-mut` sharing the last-survivor
// model lives in MIR lowering / codegen (`owned_locals`, reverse-use).

fn check_file_ownership(path: &Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("condition_scope_consume(")
            && !path.ends_with("src/compiler/ownership.ax")
        {
            return Err(format!(
                "error: manual condition consume is forbidden at {}:{}",
                path.display(),
                idx + 1
            ));
        }
        if trimmed.contains("condition_scope_begin(")
            && !path.ends_with("src/compiler/ownership.ax")
        {
            return Err(format!(
                "error: manual condition scope begin is forbidden at {}:{}",
                path.display(),
                idx + 1
            ));
        }
        if trimmed.contains("dealloc(") || trimmed.contains("free(") {
            return Err(format!(
                "error: manual deallocation is forbidden at {}:{}",
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
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: bad dir entry: {e}"))?
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

fn ownership_check_message(root: &str) -> String {
    let root_path = match root.is_empty() {
        true => project_entry_root_path(),
        false => PathBuf::from(root),
    };
    match walk_and_check_ownership(&root_path) {
        Ok(count) => format!("ok:ownership:{count}"),
        Err(err) => err,
    }
}

#[axon_pub_export]
fn run_ownership_project_check(root: &str) -> String {
    ownership_check_message(root)
}

#[axon_pub_export]
fn ownership_stage_failed(root: &str) -> bool {
    ownership_check_message(root).starts_with("error")
}
