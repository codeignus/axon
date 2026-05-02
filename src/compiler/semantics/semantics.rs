// ─── Filesystem helpers ───────────────────────────────────────────────
// LANG-GAP: filesystem path checking for import resolution — sidecar needed
// until Axon has std::fs or equivalent. Ported import-path logic is in
// resolve.ax (scan_import_lines calls axon_import_path_exists).

fn expected_import_path_exists(import_path: &str) -> bool {
    let root = PathBuf::from("src");
    if root.join(format!("{import_path}.ax")).exists() { return true; }
    let module_dir_file = root.join(import_path).join(
        import_path.rsplit('/').next().unwrap_or(import_path).to_string() + ".ax",
    );
    if module_dir_file.exists() { return true; }
    root.join(import_path).exists()
}

// LANG-GAP: filesystem walk for counting checked .ax files.
fn walk_and_check(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
        if path.is_dir() { checked += walk_and_check(&path)?; continue; }
        if is_project_ax_source(&path) { checked += 1; }
    }
    Ok(checked)
}

// ─── Shared parse helpers ─────────────────────────────────────────────
// LANG-GAP: fully ported to semantics.ax. Retained ONLY because
// collect_project_function_signatures and verify_project_calls need
// filesystem walks. Delete when Axon gains directory iteration.

fn count_arity_from_params(params: &str) -> usize {
    let p = params.trim();
    if p.is_empty() { return 0; }
    p.split(',').map(str::trim).filter(|s| !s.is_empty()).count()
}

fn extract_name_to_paren(s: &str) -> String {
    s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect()
}

fn parse_func_name_and_arity(line: &str) -> Option<(String, usize)> {
    let func_part = line.strip_prefix("func ").or_else(|| line.strip_prefix("pub func "))?;
    let open = func_part.find('(')?;
    let close = func_part[open + 1..].find(')')? + open + 1;
    let name = extract_name_to_paren(&func_part[..open]);
    if name.is_empty() { return None; }
    Some((name, count_arity_from_params(&func_part[open + 1..close])))
}

fn parse_rust_export_name_and_arity(sig_line: &str) -> Option<(String, usize)> {
    let fn_part = sig_line.split("fn ").nth(1)?.trim_start();
    let open = fn_part.find('(')?;
    let close = fn_part[open + 1..].find(')')? + open + 1;
    let name = extract_name_to_paren(&fn_part[..open]);
    if name.is_empty() { return None; }
    Some((name, count_arity_from_params(&fn_part[open + 1..close])))
}

// LANG-GAP: call-site parsing — fully ported to semantics.ax as
// parse_call_name_from_line / parse_call_arity_from_line. Retained ONLY
// for verify_project_calls. Delete with it.
fn parse_call_name_and_arity(line: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if !(chars[i].is_ascii_alphabetic() || chars[i] == '_') { i += 1; continue; }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
        let prev = chars[..start].iter().rposition(|c| !c.is_whitespace());
        if prev.map_or(false, |p| chars[p] == '.') { continue; }
        let name: String = chars[start..i].iter().collect();
        if i >= chars.len() || chars[i] != '(' { continue; }
        if matches!(name.as_str(), "if"|"elif"|"for"|"while"|"func"|"return"|"print"|"assert_eq"|"message_is_error") { continue; }
        return scan_paren_arity(&chars, i + 1).map(|a| (name, a));
    }
    None
}

// LANG-GAP: method call parsing — fully ported to semantics.ax as
// find_dot_method / count_method_args. Retained ONLY for verify_project_calls.
fn parse_method_call_name_and_arity(line: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '.' { i += 1; continue; }
        i += 1;
        if i >= chars.len() || !(chars[i].is_ascii_alphabetic() || chars[i] == '_') { continue; }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
        let method: String = chars[start..i].iter().collect();
        if i >= chars.len() || chars[i] != '(' { continue; }
        return scan_paren_arity(&chars, i + 1).map(|a| (method, a));
    }
    None
}

fn scan_paren_arity(chars: &[char], start: usize) -> Option<usize> {
    let (mut depth, mut commas, mut has_any, mut in_str) = (1usize, 0usize, false, false);
    let mut j = start;
    while j < chars.len() {
        let c = chars[j];
        if in_str {
            if c == '\\' { j += 2; continue; }
            if c == '"' { in_str = false; }
            j += 1; continue;
        }
        match c {
            '"' => { in_str = true; has_any = true; j += 1; continue; }
            '(' => { depth += 1; has_any = true; j += 1; continue; }
            ')' => {
                depth -= 1;
                if depth == 0 { return Some(if has_any { commas + 1 } else { 0 }); }
                j += 1; continue;
            }
            ',' if depth == 1 => { commas += 1; j += 1; continue; }
            _ => {
                if !c.is_whitespace() { has_any = true; }
                j += 1;
            }
        }
    }
    None
}

// ─── Project-level semantic analysis ──────────────────────────────────
// LANG-GAP: project-level signature collection — needs filesystem walk.
// Axon-native duplicate check is in resolve.ax. This does cross-file arity
// unification and Rust #[axon_export] discovery. Delete when .ax can walk dirs.

fn collect_project_function_signatures(root: &Path) -> Result<HashMap<String, usize>, String> {
    fn walk(root: &Path, sigs: &mut HashMap<String, usize>, seen_local: &mut HashMap<PathBuf, HashSet<String>>) -> Result<(), String> {
        for entry in std::fs::read_dir(root).map_err(|e| format!("error: cannot read {}: {e}", root.display()))? {
            let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
            if path.is_dir() { walk(&path, sigs, seen_local)?; continue; }
            if !is_project_ax_source(&path) { continue; }
            let src = std::fs::read_to_string(&path).map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let mut file_seen = HashSet::new();
            for line in src.lines() {
                if let Some((fname, arity)) = parse_func_name_and_arity(line.trim()) {
                    if !file_seen.insert(fname.clone()) {
                        return Err(format!("error: duplicate function '{fname}' in {}", path.display()));
                    }
                    if let Some(prev) = sigs.get(&fname) {
                        if *prev != arity { return Err(format!("error: conflicting arity for function '{fname}' across project")); }
                    } else { sigs.insert(fname, arity); }
                }
            }
            seen_local.insert(path, file_seen);
        }
        Ok(())
    }

    fn collect_rust_exports(dir: &Path, sigs: &mut HashMap<String, usize>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| format!("error: cannot read {}: {e}", dir.display()))? {
            let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
            if path.is_dir() { collect_rust_exports(&path, sigs)?; continue; }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
            if ext != "rs" && ext != "ax" { continue; }
            let src = std::fs::read_to_string(&path).map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let mut lines = src.lines().peekable();
            while let Some(line) = lines.next() {
                if line.trim() == "#[axon_export]" {
                    if let Some(next) = lines.peek() {
                        if let Some((name, arity)) = parse_rust_export_name_and_arity(next.trim()) {
                            sigs.entry(name).or_insert(arity);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    let mut sigs = HashMap::new();
    let mut seen_local = HashMap::new();
    walk(root, &mut sigs, &mut seen_local)?;
    let src_root = PathBuf::from("src");
    if src_root.exists() { collect_rust_exports(&src_root, &mut sigs)?; }
    Ok(sigs)
}

// LANG-GAP: project-level call verification — needs filesystem walk.
// Import resolution, visibility, symbol resolution now in resolve.ax.
// This handles cross-module arity matching and Rust export discovery.
// Delete when project-level checks can walk directories from .ax.

fn verify_project_calls(root: &Path, sigs: &HashMap<String, usize>) -> Result<(), String> {
    #[derive(Clone, Copy)]
    struct SymbolInfo { arity: usize, is_pub: bool }

    fn module_key_for_file(root: &Path, file: &Path) -> String {
        let parent = file.parent().unwrap_or(root);
        let comps: Vec<String> = parent.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        if let Some(i) = comps.iter().position(|p| p == "src") {
            return comps[i + 1..].join("/");
        }
        let rel = parent.strip_prefix(root).unwrap_or(parent);
        let key = rel.to_string_lossy().replace('\\', "/").trim_matches('/').to_string();
        if key.is_empty() || key == "." { "".to_string() } else { key }
    }

    fn collect_module_symbols(root: &Path) -> Result<HashMap<String, HashMap<String, SymbolInfo>>, String> {
        fn walk(root: &Path, dir: &Path, out: &mut HashMap<String, HashMap<String, SymbolInfo>>) -> Result<(), String> {
            for entry in std::fs::read_dir(dir).map_err(|e| format!("error: cannot read {}: {e}", dir.display()))? {
                let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
                if path.is_dir() { walk(root, &path, out)?; continue; }
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if ext != "ax" && ext != "rs" { continue; }
                let key = module_key_for_file(root, &path);
                let bucket = out.entry(key).or_default();
                let src = std::fs::read_to_string(&path).map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
                if ext == "ax" {
                    for line in src.lines() {
                        let t = line.trim();
                        if let Some((name, arity)) = parse_func_name_and_arity(t) {
                            bucket.entry(name).or_insert(SymbolInfo { arity, is_pub: t.starts_with("pub func ") });
                        }
                    }
                } else {
                    let mut lines = src.lines().peekable();
                    while let Some(line) = lines.next() {
                        let t = line.trim();
                        if t == "#[axon_export]" || t == "#[axon_pub_export]" {
                            let is_pub = true;
                            if let Some(next) = lines.peek() {
                                if let Some((name, arity)) = parse_rust_export_name_and_arity(next.trim()) {
                                    bucket.entry(name).or_insert(SymbolInfo { arity, is_pub });
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        let mut out = HashMap::new();
        walk(root, root, &mut out)?;
        Ok(out)
    }

    fn resolve_import_module_key<'a>(target: &'a str, syms: &'a HashMap<String, HashMap<String, SymbolInfo>>) -> Option<&'a HashMap<String, SymbolInfo>> {
        syms.get(target).or_else(|| target.rsplit_once('/').and_then(|(parent, _)| syms.get(parent)))
    }

    #[derive(Clone)]
    struct ImportBinding { symbol: String, module: String }

    // LANG-GAP: duplicated in resolve.ax (check_duplicate_braced_imports,
    // collect_import_symbols). Retained for cross-file arity matching.
    fn parse_import_bindings(source: &str) -> (Vec<ImportBinding>, Vec<String>) {
        let (mut bindings, mut errors) = (Vec::new(), Vec::new());
        let (mut in_import, mut seen_modules, mut seen_symbols, mut seen_brace) = (false, HashSet::<String>::new(), HashSet::<String>::new(), HashSet::<String>::new());
        let (mut open_brace_module, mut brace_depth) = (None::<String>, 0u32);
        for (line_num, line) in source.lines().enumerate() {
            let t = line.trim();
            if t == "import" { in_import = true; continue; }
            if !in_import { continue; }
            if t.is_empty() || t.starts_with("func ") || t.starts_with("pub func ") || t.starts_with("test ") { in_import = false; continue; }
            if brace_depth > 0 {
                // Inside a multi-line braced import: collect symbols until closing '}'
                let close = t.find('}');
                let end = close.unwrap_or(t.len());
                for sym in t[..end].split(',') {
                    let s = sym.trim();
                    if !s.is_empty() {
                        if let Some(ref module) = open_brace_module {
                            if !seen_symbols.insert(s.to_string()) { errors.push(format!("error: duplicate import '{s}' in {}", line_num + 1)); }
                            bindings.push(ImportBinding { symbol: s.to_string(), module: module.clone() });
                        }
                    }
                }
                if close.is_some() { brace_depth = 0; open_brace_module = None; }
                continue;
            }
            if let Some(open) = t.find('{') {
                let module = t[..open].trim().to_string();
                if !seen_brace.insert(module.clone()) { errors.push(format!("error: duplicate import of module '{}' at line {}", module, line_num + 1)); }
                let close_pos = t.find('}');
                let end = close_pos.unwrap_or(t.len());
                for sym in t[open + 1..end].split(',') {
                    let s = sym.trim();
                    if !s.is_empty() {
                        if !seen_symbols.insert(s.to_string()) { errors.push(format!("error: duplicate import '{s}' in {}", line_num + 1)); }
                        bindings.push(ImportBinding { symbol: s.to_string(), module: module.clone() });
                    }
                }
                if close_pos.is_none() { brace_depth = 1; open_brace_module = Some(module); }
            } else {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if parts.len() == 1 && !seen_modules.insert(parts[0].to_string()) {
                    errors.push(format!("error: duplicate module import '{}' in {}", parts[0], line_num + 1));
                } else if parts.len() == 2 && !seen_modules.insert(parts[1].to_string()) {
                    errors.push(format!("error: duplicate alias '{}' for module '{}' in {}", parts[1], parts[0], line_num + 1));
                }
            }
        }
        (bindings, errors)
    }

    fn walk(root: &Path, _sigs: &HashMap<String, usize>, module_symbols: &HashMap<String, HashMap<String, SymbolInfo>>) -> Result<(), String> {
        for entry in std::fs::read_dir(root).map_err(|e| format!("error: cannot read {}: {e}", root.display()))? {
            let path = entry.map_err(|e| format!("error: bad dir entry: {e}"))?.path();
            if path.is_dir() { walk(&path, _sigs, module_symbols)?; continue; }
            if !is_project_ax_source(&path) { continue; }
            let src = std::fs::read_to_string(&path).map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let current_module = module_key_for_file(root, &path);
            let (imports, import_errors) = parse_import_bindings(&src);
            for err in &import_errors { return Err(err.clone()); }
            let local_symbols = module_symbols.get(&current_module).cloned().unwrap_or_default();
            for line in src.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("func ") || trimmed.starts_with("pub func ") { continue; }
                if let Some((callee, got_arity)) = parse_call_name_and_arity(trimmed) {
                    if callee.starts_with("axon_") { continue; }
                    let expected = local_symbols.get(&callee).map(|l| l.arity).or_else(|| {
                        for b in &imports {
                            if b.symbol == callee {
                                return resolve_import_module_key(&b.module, module_symbols).and_then(|ts| {
                                    let sym = ts.get(&callee)?;
                                    if sym.is_pub { Some(sym.arity) } else { None }
                                });
                            }
                        }
                        None
                    });
                    match expected {
                        Some(e) if e != got_arity => return Err(format!("error: arity mismatch calling '{callee}' (expected {e}, got {got_arity}) in {}", path.display())),
                        None => return Err(format!("error: unresolved symbol '{callee}' in {}", path.display())),
                        _ => {}
                    }
                }
                if let Some((method, got_arity)) = parse_method_call_name_and_arity(trimmed) {
                    if method == "len" {
                        let ea = got_arity + 1;
                        let ok = module_symbols.values().any(|syms| syms.get("string_len").map_or(false, |s| s.arity == ea && s.is_pub))
                            || module_symbols.values().any(|syms| syms.get("array_len").map_or(false, |s| s.arity == ea && s.is_pub));
                        if !ok { return Err(format!("error: unresolved method '{}' in {}", method, path.display())); }
                    }
                }
            }
        }
        Ok(())
    }

    let module_symbols = collect_module_symbols(root)?;
    walk(root, sigs, &module_symbols)
}

// LANG-GAP: project semantic orchestrator — thin wrapper around filesystem ops.
// Per-file logic now in .ax files. Delete when project-level checks are fully .ax.
fn semantic_check_message(root: &str) -> String {
    let root_path = if root.is_empty() { project_entry_root_path() } else { PathBuf::from(root) };
    match walk_and_check(&root_path) {
        Ok(count) => {
            let sigs = match collect_project_function_signatures(&root_path) { Ok(s) => s, Err(e) => return e };
            if let Err(e) = verify_project_calls(&root_path, &sigs) { return e; }
            format!("ok:semantic:{count}")
        }
        Err(e) => e,
    }
}

// ─── FFI exports ──────────────────────────────────────────────────────

#[axon_export]
fn axon_import_path_exists(path: &str) -> bool { expected_import_path_exists(path) }

#[axon_pub_export]
fn run_semantic_project_check(root: &str) -> String { semantic_check_message(root) }

#[axon_pub_export]
fn semantic_stage_failed(root: &str) -> bool { semantic_check_message(root).starts_with("error") }

// ─── String FFI primitives ───────────────────────────────────────────
// LANG-GAP: string primitives — FFI operations used by .ax code.
// Remain until Axon has native string indexing.

#[axon_export]
fn axon_string_char_at(s: &str, index: i64) -> String {
    s.chars().nth(index as usize).unwrap_or_default().to_string()
}

#[axon_export]
fn axon_string_byte_at(s: &str, index: i64) -> i64 {
    let i = index as usize;
    s.as_bytes().get(i).map(|&b| b as i64).unwrap_or(-1)
}

#[axon_export]
fn axon_string_starts_with(s: &str, prefix: &str) -> bool { s.starts_with(prefix) }

#[axon_export]
fn axon_string_contains(haystack: &str, needle: &str) -> bool { haystack.contains(needle) }

#[axon_export]
fn axon_string_sub(s: &str, start: i64, len: i64) -> String {
    if start < 0 || len < 0 { return String::new(); }
    let (start, len) = (start as usize, len as usize);
    if start >= s.len() { return String::new(); }
    s[start..std::cmp::min(start + len, s.len())].to_string()
}

#[axon_export]
fn axon_string_count(haystack: &str, needle: &str) -> i64 {
    if needle.is_empty() { return haystack.len() as i64; }
    haystack.matches(needle).count() as i64
}

#[axon_export]
fn axon_string_trim(s: &str) -> String { s.trim().to_string() }
