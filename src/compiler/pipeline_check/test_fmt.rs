#[axon_pub_export]
fn run_project_tests(target: &str) -> String {
    let scope = if target.is_empty() {
        "test:project".to_string()
    } else if target == "." {
        "dir:.".to_string()
    } else if target == "./..." {
        "dir-recursive:./".to_string()
    } else if target == "..." {
        "dir-recursive:.".to_string()
    } else if target.ends_with("/...") {
        format!("dir-recursive:{}", &target[..target.len() - 4])
    } else if target.ends_with(".ax") {
        format!("file:{}", target)
    } else {
        format!("dir:{}", target)
    };
    let root_path = if target.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(target)
    };
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = collect_all_ax_files(&root_path, &mut files) {
        return err;
    }
    let mut test_count = 0usize;
    for file in &files {
        if file.ends_with(".test.ax") {
            test_count += 1;
        }
    }
    format!(
        "ok:test:{scope}:files={}:tests={test_count}",
        files.len()
    )
}

#[axon_pub_export]
fn run_fmt_target(target: &str) -> String {
    let root_path = if target.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(target)
    };
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = collect_all_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    let mut touched = 0usize;
    for file in &files {
        let path = PathBuf::from(file);
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => return format!("error: fmt: cannot read {}: {e}", path.display()),
        };
        let mut out = String::new();
        for line in src.lines() {
            out.push_str(line.trim_end());
            out.push('\n');
        }
        if let Err(e) = std::fs::write(&path, out) {
            return format!("error: fmt: cannot write {}: {e}", path.display());
        }
        touched += 1;
    }
    format!("ok:fmt:files={touched}")
}
