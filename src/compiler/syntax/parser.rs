// File-I/O sidecar for the Axon-native parser.
// Parsing truth lives in parser.ax. This file provides:
//   1. Filesystem walk for the parse-check pipeline
//   2. run_parse_check / parse_stage_failed as #[axon_pub_export] FFI
//      needed by pipeline_check.ax
//
// LANG-GAP: parse_file_content duplicates validate_delimiters_char_scan
// from parser.ax. Kept until the parse pipeline calls the Axon function
// directly via FFI. The Axon version is authoritative; this is a fallback.
// LANG-GAP: walk_and_parse stays here until Axon can walk directories natively.

fn parse_file_content(source: &str) -> String {
    let mut stack: Vec<char> = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut in_string = false;
    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            if ch == '\\' { i += 2; continue; }
            if ch == '"' { in_string = false; }
            i += 1;
            continue;
        }
        match ch {
            '"' => { in_string = true; }
            '(' | '[' | '{' => stack.push(ch),
            ')' => { if stack.pop() != Some('(') { return "error: mismatched ')'".into(); } }
            ']' => { if stack.pop() != Some('[') { return "error: mismatched ']'".into(); } }
            '}' => { if stack.pop() != Some('{') { return "error: mismatched '}'".into(); } }
            _ => {}
        }
        i += 1;
    }
    if stack.is_empty() { "ok".into() } else { "error: unclosed delimiter".into() }
}

fn walk_and_parse(root: &std::path::Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
        if path.is_dir() { checked += walk_and_parse(&path)?; continue; }
        if path.extension().and_then(|e| e.to_str()) != Some("ax") { continue; }
        if path.file_name().and_then(|n| n.to_str()).unwrap_or("").ends_with(".test.ax") { continue; }
        let src = std::fs::read_to_string(&path)
            .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
        let result = parse_file_content(&src);
        if result.starts_with("error:") {
            return Err(format!("error: {}: {}", path.display(), result));
        }
        checked += 1;
    }
    Ok(checked)
}

fn parse_check_message(root: &str) -> String {
    let root_path = if root.is_empty() { std::path::PathBuf::from("src") } else { std::path::PathBuf::from(root) };
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
