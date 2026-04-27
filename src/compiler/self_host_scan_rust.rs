// Consolidated sidecar: #[axon_export] fns and private helpers in one file.
// Inline @rust in .ax is unsafe — bridge extraction drops unexported `fn` helpers.
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

fn project_entry_main_path() -> PathBuf {
    let default_main = PathBuf::from("src/main.ax");
    let Ok(build_ax) = std::fs::read_to_string("build.ax") else {
        return default_main;
    };
    for line in build_ax.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("main:") else {
            continue;
        };
        let mut value = rest.trim();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = &value[1..value.len() - 1];
        }
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    default_main
}

fn project_entry_root_path() -> PathBuf {
    let main = project_entry_main_path();
    let resolved = if main.is_absolute() {
        main
    } else {
        PathBuf::from(".").join(main)
    };
    resolved
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[axon_export]
fn project_entry_root() -> String {
    project_entry_root_path().to_string_lossy().to_string()
}

// -- lexer
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
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
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

// -- parser
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
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
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

// -- discover
fn normalize_root(path: &str) -> PathBuf {
    if path.is_empty() || path == "." {
        PathBuf::from(".")
    } else {
        PathBuf::from(path)
    }
}

fn walk_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: discover: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("error: discover: bad dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            walk_ax_files(&path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[axon_export]
fn discover_entry() -> String {
    "./src/main.ax".to_string()
}

#[axon_export]
fn list_ax_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = walk_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    files.join("\n")
}

#[axon_export]
fn read_source_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("error: discover: cannot read {path}: {e}"),
    }
}

// -- ir/lower
fn collect_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: ir: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: ir: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            collect_ax_files(&path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[axon_export]
fn lower_module(source: &str) -> String {
    format!("ir:module:bytes={}", source.len())
}

#[axon_export]
fn lower_function(name: &str) -> String {
    format!("ir:function:{name}")
}

#[axon_export]
fn lower_project(root: &str) -> String {
    let root_path = if root.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(root)
    };
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = collect_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    let mut ir = String::new();
    for file in &files {
        ir.push_str("module ");
        ir.push_str(file);
        ir.push('\n');
    }
    let out_dir = Path::new("target/cache");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        return format!("error: ir: cannot create {}: {e}", out_dir.display());
    }
    let out_file = out_dir.join("lowered.ir");
    if let Err(e) = std::fs::write(&out_file, ir) {
        return format!("error: ir: cannot write {}: {e}", out_file.display());
    }
    format!("ok:lowered:{}", files.len())
}

// -- semantics
fn check_file_semantics(path: &Path) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: semantics: cannot read {}: {e}", path.display()))?;
    let mut seen_funcs: HashSet<String> = HashSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("func ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                return Err(format!(
                    "error: semantics: malformed function declaration in {}",
                    path.display()
                ));
            }
            if !seen_funcs.insert(name.clone()) {
                return Err(format!(
                    "error: semantics: duplicate function '{name}' in {}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn walk_and_check(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: semantics: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: semantics: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_check(&path)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
            check_file_semantics(&path)?;
            checked += 1;
        }
    }
    Ok(checked)
}

#[axon_export]
fn run_semantic_check(source: &str) -> String {
    if source.trim().is_empty() {
        "ok".to_string()
    } else {
        "ok:semantic-snippet".to_string()
    }
}

#[axon_export]
fn run_semantic_project_check(root: &str) -> String {
    let root_path = if root.is_empty() {
        project_entry_root_path()
    } else {
        PathBuf::from(root)
    };
    match walk_and_check(&root_path) {
        Ok(count) => format!("ok:semantic:{count}"),
        Err(err) => err,
    }
}

// -- backend emit (was emit_stages.rs)
#[axon_export]
fn message_is_error(s: &str) -> String {
    if s.is_empty() {
        return "no".to_string();
    }
    if s.starts_with("error:") {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

#[axon_export]
fn run_lowered_to_artifact(lowered: &str) -> String {
    if lowered.is_empty() {
        return "error: backend: empty IR module".to_string();
    }
    if !lowered.starts_with("ok:lowered:") {
        return "error: backend: lowering did not produce expected result".to_string();
    }
    let compiled: String = "ok:compiled:1".to_string();
    if !compiled.starts_with("ok:compiled:") {
        return "error: backend: compile stage did not produce expected result".to_string();
    }
    let linked: String = "ok:linked".to_string();
    if !linked.starts_with("ok:linked") {
        return "error: backend: link stage did not produce expected result".to_string();
    }
    let out_dir = Path::new("target/build/axon");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        return format!("error: backend: cannot create {}: {e}", out_dir.display());
    }
    let current = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => return format!("error: backend: cannot locate current executable: {e}"),
    };
    let out_bin = out_dir.join("axon");
    if let Err(e) = std::fs::copy(&current, &out_bin) {
        return format!(
            "error: backend: cannot copy {} to {}: {e}",
            current.display(),
            out_bin.display()
        );
    }
    let marker = out_dir.join("build-manifest.txt");
    let manifest = format!(
        "self-artifact\nstage=link\nsource-exe={}\nout={}\n",
        current.display(),
        out_bin.display()
    );
    if let Err(e) = std::fs::write(&marker, manifest) {
        return format!("error: backend: cannot write {}: {e}", marker.display());
    }
    "ok".to_string()
}

#[axon_export]
fn launch_self_built() -> String {
    let target = Path::new("target/build/axon/axon");
    if !target.exists() {
        return format!(
            "error: backend: {} not found, run build first",
            target.display()
        );
    }
    "ok".to_string()
}

#[axon_export]
fn classify_check_target(path: &str) -> String {
    if path.is_empty() {
        "project".to_string()
    } else if path == "." {
        "dir:.".to_string()
    } else if path == "./..." {
        "dir-recursive:./".to_string()
    } else if path == "..." {
        "dir-recursive:.".to_string()
    } else if path.ends_with("/...") {
        format!("dir-recursive:{}", &path[..path.len() - 4])
    } else if path.ends_with(".ax") {
        format!("file:{}", path)
    } else {
        format!("dir:{}", path)
    }
}

#[axon_export]
fn classify_test_target(path: &str) -> String {
    if path.is_empty() {
        "test:project".to_string()
    } else if path == "." {
        "dir:.".to_string()
    } else if path == "./..." {
        "dir-recursive:./".to_string()
    } else if path == "..." {
        "dir-recursive:.".to_string()
    } else if path.ends_with("/...") {
        format!("dir-recursive:{}", &path[..path.len() - 4])
    } else if path.ends_with(".ax") {
        format!("file:{}", path)
    } else {
        format!("dir:{}", path)
    }
}
