// LANG-GAP: filesystem path checking for import resolution — sidecar needed
// until Axon has std::fs or equivalent. Ported import-path logic is in
// resolve.ax (scan_import_lines calls axon_import_path_exists).
fn expected_import_path_exists(import_path: &str) -> bool {
    let root = PathBuf::from("src");
    let direct_file = root.join(format!("{import_path}.ax"));
    if direct_file.exists() {
        return true;
    }
    let module_dir_file = root.join(import_path).join(
        import_path
            .rsplit('/')
            .next()
            .unwrap_or(import_path)
            .to_string()
            + ".ax",
    );
    if module_dir_file.exists() {
        return true;
    }
    let module_dir = root.join(import_path);
    module_dir.exists()
}

// LANG-GAP: func name+arity parsing — duplicated in semantics.ax as
// parse_func_name_from_line / parse_func_arity_from_decl. Sidecar retained
// until project-level walk can be done from Axon.
fn parse_func_name_and_arity(line: &str) -> Option<(String, usize)> {
    let func_part = if let Some(rest) = line.strip_prefix("func ") {
        rest
    } else {
        line.strip_prefix("pub func ")?
    };
    let open = func_part.find('(')?;
    let close = func_part[open + 1..].find(')')? + open + 1;
    let name: String = func_part[..open]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    let params = func_part[open + 1..close].trim();
    if params.is_empty() {
        return Some((name, 0));
    }
    let arity = params
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .count();
    Some((name, arity))
}

// LANG-GAP: call-site name+arity parsing — duplicated in semantics.ax as
// parse_call_name_from_line / parse_call_arity_from_line. Sidecar kept for
// project-level verification (needs filesystem walk).
fn parse_call_name_and_arity(line: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if !(chars[i].is_ascii_alphabetic() || chars[i] == '_') {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if start > 0 {
            let mut k = start;
            while k > 0 && chars[k - 1].is_whitespace() {
                k -= 1;
            }
            if k > 0 && chars[k - 1] == '.' {
                i = end;
                continue;
            }
        }
        i = end;
        let name: String = chars[start..i].iter().collect();
        if i >= chars.len() || chars[i] != '(' {
            continue;
        }
        if matches!(
            name.as_str(),
            "if" | "elif" | "for" | "while" | "func" | "return" | "print" | "assert_eq"
                | "message_is_error"
        ) {
            continue;
        }
        let mut depth = 1usize;
        let mut j = i + 1;
        let mut commas = 0usize;
        let mut has_any = false;
        let mut in_string = false;
        while j < chars.len() {
            let c = chars[j];
            if in_string {
                if c == '\\' {
                    j += 2;
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                j += 1;
                continue;
            }
            if c == '"' {
                in_string = true;
                has_any = true;
                j += 1;
                continue;
            }
            if c == '(' {
                depth += 1;
                has_any = true;
                j += 1;
                continue;
            }
            if c == ')' {
                depth -= 1;
                if depth == 0 {
                    let arity = if has_any { commas + 1 } else { 0 };
                    return Some((name, arity));
                }
                j += 1;
                continue;
            }
            if c == ',' && depth == 1 {
                commas += 1;
                j += 1;
                continue;
            }
            if !c.is_whitespace() {
                has_any = true;
            }
            j += 1;
        }
        break;
    }
    None
}

// LANG-GAP: method call parsing — duplicated in semantics.ax as
// find_dot_method / count_method_args. Sidecar kept for project-level walk.
fn parse_method_call_name_and_arity(line: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '.' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= chars.len() || !(chars[i].is_ascii_alphabetic() || chars[i] == '_') {
            continue;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        let method: String = chars[start..i].iter().collect();
        if i >= chars.len() || chars[i] != '(' {
            continue;
        }
        let mut depth = 1usize;
        let mut j = i + 1;
        let mut commas = 0usize;
        let mut has_any = false;
        let mut in_string = false;
        while j < chars.len() {
            let c = chars[j];
            if in_string {
                if c == '\\' {
                    j += 2;
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                j += 1;
                continue;
            }
            if c == '"' {
                in_string = true;
                has_any = true;
                j += 1;
                continue;
            }
            if c == '(' {
                depth += 1;
                has_any = true;
                j += 1;
                continue;
            }
            if c == ')' {
                depth -= 1;
                if depth == 0 {
                    let arity = if has_any { commas + 1 } else { 0 };
                    return Some((method, arity));
                }
                j += 1;
                continue;
            }
            if c == ',' && depth == 1 {
                commas += 1;
                j += 1;
                continue;
            }
            if !c.is_whitespace() {
                has_any = true;
            }
            j += 1;
        }
        break;
    }
    None
}

// LANG-GAP: snippet-level semantic check — fully ported to semantics.ax
// (axon_semantic_check). This Rust version is kept as a thin shim so the
// `run_semantic_check` #[axon_export] can delegate. When project-level checks
// move to .ax, this becomes unnecessary.
#[axon_export]
fn run_semantic_check(source: &str) -> String {
    if source.trim().is_empty() {
        return "ok:semantic-snippet:empty".to_string();
    }
    let mut seen: HashSet<String> = HashSet::new();
    let mut func_arity: HashMap<String, usize> = HashMap::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("func ")
            .or_else(|| trimmed.strip_prefix("pub func "))
        {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                return "error: malformed function declaration in snippet".to_string();
            }
            if !seen.insert(name.clone()) {
                return format!("error: duplicate function '{name}' in snippet");
            }
            if let Some((fname, arity)) = parse_func_name_and_arity(trimmed) {
                func_arity.insert(fname, arity);
            }
            continue;
        }
        if let Some((callee, got_arity)) = parse_call_name_and_arity(trimmed) {
            if let Some(expected_arity) = func_arity.get(&callee) {
                if *expected_arity != got_arity {
                    return format!(
                        "error: arity mismatch calling '{callee}' (expected {}, got {got_arity}) in snippet",
                        expected_arity
                    );
                }
            } else {
                return format!("error: unresolved symbol '{callee}' in snippet");
            }
        }
        if let Some((method, got_arity)) = parse_method_call_name_and_arity(trimmed) {
            if method == "len" {
                let expected_arity = got_arity + 1;
                let string_ok = func_arity
                    .get("string_len")
                    .map(|a| *a == expected_arity)
                    .unwrap_or(false);
                let array_ok = func_arity
                    .get("array_len")
                    .map(|a| *a == expected_arity)
                    .unwrap_or(false);
                if !string_ok && !array_ok {
                    return format!("error: unresolved method '{method}' in snippet");
                }
            }
        }
    }
    "ok:semantic-snippet".to_string()
}

// LANG-GAP: project-level signature collection — needs filesystem walk
// (std::fs). The Axon-native duplicate declaration check is in resolve.ax
// (check_duplicate_declarations_axon). This sidecar does cross-file arity
// unification which requires Axon to gain directory iteration primitives.
fn collect_project_function_signatures(
    root: &Path,
) -> Result<HashMap<String, usize>, String> {

    fn walk(
        root: &Path,
        sigs: &mut HashMap<String, usize>,
        seen_local: &mut HashMap<PathBuf, HashSet<String>>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(root)
            .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("error: bad dir entry: {e}"))?
                .path();
            if path.is_dir() {
                walk(&path, sigs, seen_local)?;
                continue;
            }
            if !is_project_ax_source(&path) {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let mut file_seen = HashSet::new();
            for line in src.lines() {
                let trimmed = line.trim();
                if let Some((fname, arity)) = parse_func_name_and_arity(trimmed) {
                    if !file_seen.insert(fname.clone()) {
                        return Err(format!(
                            "error: duplicate function '{fname}' in {}",
                            path.display()
                        ));
                    }
                    if let Some(prev) = sigs.get(&fname) {
                        if *prev != arity {
                            return Err(format!(
                                "error: conflicting arity for function '{fname}' across project"
                            ));
                        }
                    } else {
                        sigs.insert(fname, arity);
                    }
                }
            }
            seen_local.insert(path, file_seen);
        }
        Ok(())
    }

    fn parse_rust_export_arity(sig_line: &str) -> Option<(String, usize)> {
        let fn_part = sig_line.split("fn ").nth(1)?.trim_start();
        let open = fn_part.find('(')?;
        let close = fn_part[open + 1..].find(')')? + open + 1;
        let name: String = fn_part[..open]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            return None;
        }
        let params = fn_part[open + 1..close].trim();
        if params.is_empty() {
            return Some((name, 0));
        }
        let arity = params
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .count();
        Some((name, arity))
    }

    fn collect_rust_exports(dir: &Path, sigs: &mut HashMap<String, usize>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("error: cannot read {}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("error: bad dir entry: {e}"))?
                .path();
            if path.is_dir() {
                collect_rust_exports(&path, sigs)?;
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
            if ext != "rs" && ext != "ax" {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let mut lines = src.lines().peekable();
            while let Some(line) = lines.next() {
                if line.trim() == "#[axon_export]" {
                    if let Some(next) = lines.peek() {
                        if let Some((name, arity)) = parse_rust_export_arity(next.trim()) {
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
    if src_root.exists() {
        collect_rust_exports(&src_root, &mut sigs)?;
    }
    Ok(sigs)
}

// LANG-GAP: project-level call verification — needs filesystem walk.
// Import resolution, visibility checking, and symbol resolution are now in
// resolve.ax (check_self_import_axon, check_import_collision_axon,
// check_visibility_axon, resolve_all_imports_axon). This sidecar handles
// cross-module arity matching and Rust #[axon_export] discovery which
// require filesystem access.
fn verify_project_calls(
    root: &Path,
    sigs: &HashMap<String, usize>,
) -> Result<(), String> {
    #[derive(Clone, Copy)]
    struct SymbolInfo {
        arity: usize,
        is_pub: bool,
    }

    fn module_key_for_file(root: &Path, file: &Path) -> String {
        let parent = file.parent().unwrap_or(root);
        fn key_after_src(dir: &Path) -> Option<String> {
            let comps: Vec<String> = dir
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            if let Some(i) = comps.iter().position(|p| p == "src") {
                let tail = &comps[i + 1..];
                Some(tail.join("/"))
            } else {
                None
            }
        }

        if let Some(key) = key_after_src(parent) {
            return key;
        }

        let rel = parent.strip_prefix(root).unwrap_or(parent);
        let key = rel.to_string_lossy().replace('\\', "/").trim_matches('/').to_string();
        if key.is_empty() || key == "." {
            "".to_string()
        } else {
            key
        }
    }

    fn parse_rust_export_decl(sig_line: &str) -> Option<(String, usize)> {
        let fn_part = sig_line.split("fn ").nth(1)?.trim_start();
        let open = fn_part.find('(')?;
        let close = fn_part[open + 1..].find(')')? + open + 1;
        let name: String = fn_part[..open]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            return None;
        }
        let params = fn_part[open + 1..close].trim();
        if params.is_empty() {
            return Some((name, 0));
        }
        let arity = params
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .count();
        Some((name, arity))
    }

    fn collect_module_symbols(root: &Path) -> Result<HashMap<String, HashMap<String, SymbolInfo>>, String> {
        fn walk(
            root: &Path,
            dir: &Path,
            out: &mut HashMap<String, HashMap<String, SymbolInfo>>,
        ) -> Result<(), String> {
            let entries = std::fs::read_dir(dir)
                .map_err(|e| format!("error: cannot read {}: {e}", dir.display()))?;
            for entry in entries {
                let path = entry
                    .map_err(|e| format!("error: bad dir entry: {e}"))?
                    .path();
                if path.is_dir() {
                    walk(root, &path, out)?;
                    continue;
                }
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if ext != "ax" && ext != "rs" {
                    continue;
                }
                let key = module_key_for_file(root, &path);
                let bucket = out.entry(key).or_default();
                let src = std::fs::read_to_string(&path)
                    .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
                if ext == "ax" {
                    for line in src.lines() {
                        let t = line.trim();
                        if let Some((name, arity)) = parse_func_name_and_arity(t) {
                            let is_pub = t.starts_with("pub func ");
                            bucket.entry(name).or_insert(SymbolInfo { arity, is_pub });
                        }
                    }
                } else {
                    let mut lines = src.lines().peekable();
                    while let Some(line) = lines.next() {
                        let t = line.trim();
                        if t == "#[axon_export]" || t == "#[axon_pub_export]" {
                            let is_pub = t == "#[axon_pub_export]";
                            if let Some(next) = lines.peek() {
                                if let Some((name, arity)) = parse_rust_export_decl(next.trim()) {
                                    bucket.entry(name).or_insert(SymbolInfo { arity, is_pub });
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        let mut out: HashMap<String, HashMap<String, SymbolInfo>> = HashMap::new();
        walk(root, root, &mut out)?;
        Ok(out)
    }

    fn resolve_import_module_key<'a>(
        target_module: &'a str,
        module_symbols: &'a HashMap<String, HashMap<String, SymbolInfo>>,
    ) -> Option<&'a HashMap<String, SymbolInfo>> {
        if let Some(found) = module_symbols.get(target_module) {
            return Some(found);
        }
        if let Some((parent, _)) = target_module.rsplit_once('/') {
            return module_symbols.get(parent);
        }
        None
    }

    #[derive(Clone)]
    struct ImportBinding {
        symbol: String,
        module: String,
        line_num: usize,
    }

    fn parse_import_bindings(source: &str) -> (Vec<ImportBinding>, Vec<String>) {
        let mut bindings = Vec::new();
        let mut errors = Vec::new();
        let mut in_import = false;
        let mut seen_modules: HashSet<String> = HashSet::new();
        let mut seen_symbols: HashSet<String> = HashSet::new();
        let mut seen_brace_modules: HashSet<String> = HashSet::new();
        let mut line_num = 0usize;
        for line in source.lines() {
            line_num += 1;
            let t = line.trim();
            if t == "import" {
                in_import = true;
                continue;
            }
            if !in_import {
                continue;
            }
            if t.is_empty() || t.starts_with("func ") || t.starts_with("pub func ") || t.starts_with("test ") {
                in_import = false;
                continue;
            }
            if let Some(open) = t.find('{') {
                let module = t[..open].trim();
                if !seen_brace_modules.insert(module.to_string()) {
                    errors.push(format!(
                        "error: duplicate import of module '{}' at line {}",
                        module, line_num
                    ));
                }
                let close = t.find('}').unwrap_or(t.len());
                let inside = &t[open + 1..close];
                for sym in inside.split(',') {
                    let s = sym.trim();
                    if !s.is_empty() {
                        if !seen_symbols.insert(s.to_string()) {
                            errors.push(format!(
                                "error: duplicate import '{s}' in {}",
                                line_num
                            ));
                        }
                        bindings.push(ImportBinding {
                            symbol: s.to_string(),
                            module: module.to_string(),
                            line_num,
                        });
                    }
                }
            } else {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if parts.len() == 1 {
                    let mod_path = parts[0];
                    if !seen_modules.insert(mod_path.to_string()) {
                        errors.push(format!(
                            "error: duplicate module import '{mod_path}' in {}",
                            line_num
                        ));
                    }
                } else if parts.len() == 2 {
                    let mod_path = parts[0];
                    let alias = parts[1];
                    if !seen_modules.insert(alias.to_string()) {
                        errors.push(format!(
                            "error: duplicate alias '{alias}' for module '{mod_path}' in {}",
                            line_num
                        ));
                    }
                }
            }
        }
        (bindings, errors)
    }

    fn walk(
        root: &Path,
        sigs: &HashMap<String, usize>,
        module_symbols: &HashMap<String, HashMap<String, SymbolInfo>>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(root)
            .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("error: bad dir entry: {e}"))?
                .path();
            if path.is_dir() {
                walk(&path, sigs, module_symbols)?;
                continue;
            }
            if !is_project_ax_source(&path) {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
            let current_module = module_key_for_file(root, &path);
            let (imports, import_errors) = parse_import_bindings(&src);
            for err in &import_errors {
                return Err(err.clone());
            }
            let local_symbols = module_symbols
                .get(&current_module)
                .cloned()
                .unwrap_or_default();
            for line in src.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("func ") || trimmed.starts_with("pub func ") {
                    continue;
                }
                if let Some((callee, got_arity)) = parse_call_name_and_arity(trimmed) {
                    if callee.starts_with("axon_") {
                        continue;
                    }
                    let expected = if let Some(local) = local_symbols.get(&callee) {
                        Some(local.arity)
                    } else {
                        let mut found: Option<usize> = None;
                        for binding in &imports {
                            if binding.symbol == callee {
                                if let Some(target_syms) =
                                    resolve_import_module_key(&binding.module, module_symbols)
                                {
                                    if let Some(sym) = target_syms.get(&callee) {
                                        if sym.is_pub {
                                            found = Some(sym.arity);
                                        } else {
                                            return Err(format!(
                                                "error: '{}' is not public in module '{}' (private access) in {}",
                                                callee, binding.module, path.display()
                                            ));
                                        }
                                    } else {
                                        return Err(format!(
                                            "error: '{}' not found in module '{}' in {}",
                                            callee, binding.module, path.display()
                                        ));
                                    }
                                } else {
                                    return Err(format!(
                                        "error: module '{}' not found for import '{}' in {}",
                                        binding.module, callee, path.display()
                                    ));
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(expected_arity) = expected {
                        if expected_arity != got_arity {
                            return Err(format!(
                                "error: arity mismatch calling '{callee}' (expected {expected_arity}, got {got_arity}) in {}",
                                path.display()
                            ));
                        }
                    } else {
                        return Err(format!(
                            "error: unresolved symbol '{callee}' in {}",
                            path.display()
                        ));
                    }
                }
                if let Some((method, got_arity)) = parse_method_call_name_and_arity(trimmed) {
                    if method == "len" {
                        let expected_arity = got_arity + 1;
                        let string_ok = module_symbols
                            .values()
                            .any(|syms| {
                                syms.get("string_len")
                                    .map(|s| s.arity == expected_arity && s.is_pub)
                                    .unwrap_or(false)
                            });
                        let array_ok = module_symbols
                            .values()
                            .any(|syms| {
                                syms.get("array_len")
                                    .map(|s| s.arity == expected_arity && s.is_pub)
                                    .unwrap_or(false)
                            });
                        if !string_ok && !array_ok {
                            return Err(format!(
                                "error: unresolved method '{}' in {}",
                                method,
                                path.display()
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    let module_symbols = collect_module_symbols(root)?;
    walk(root, sigs, &module_symbols)
}

// LANG-GAP: project semantic orchestrator — thin wrapper around filesystem
// operations. The per-file semantic logic is now in .ax files. This remains
// as a sidecar until Axon can walk directories.
fn semantic_check_message(root: &str) -> String {
    let root_path = match root.is_empty() {
        true => project_entry_root_path(),
        false => PathBuf::from(root),
    };
    match walk_and_check(&root_path) {
        Ok(count) => {
            let sigs = match collect_project_function_signatures(&root_path) {
                Ok(s) => s,
                Err(err) => return err,
            };
            if let Err(err) = verify_project_calls(&root_path, &sigs) {
                return err;
            }
            format!("ok:semantic:{count}")
        }
        Err(err) => err,
    }
}

// LANG-GAP: filesystem walk for counting checked .ax files.
fn walk_and_check(root: &Path) -> Result<usize, String> {
    let mut checked = 0usize;
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            checked += walk_and_check(&path)?;
            continue;
        }
        if is_project_ax_source(&path) {
            checked += 1;
        }
    }
    Ok(checked)
}

// LANG-GAP: string primitives — these are FFI string operations used by .ax
// code. They remain as sidecars until Axon has native string indexing. See
// resolve.ax and semantics.ax for the semantic logic that calls these.

#[axon_export]
fn axon_import_path_exists(path: &str) -> bool {
    expected_import_path_exists(path)
}

#[axon_pub_export]
fn run_semantic_project_check(root: &str) -> String {
    semantic_check_message(root)
}

#[axon_pub_export]
fn semantic_stage_failed(root: &str) -> bool {
    semantic_check_message(root).starts_with("error")
}

#[axon_export]
fn axon_string_char_at(s: &str, index: i64) -> String {
    s.chars().nth(index as usize).unwrap_or_default().to_string()
}

#[axon_export]
fn axon_string_byte_at(s: &str, index: i64) -> i64 {
    let bytes = s.as_bytes();
    let i = index as usize;
    if i >= bytes.len() {
        -1
    } else {
        bytes[i] as i64
    }
}

#[axon_export]
fn axon_string_starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

#[axon_export]
fn axon_string_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[axon_export]
fn axon_string_sub(s: &str, start: i64, len: i64) -> String {
    if start < 0 || len < 0 {
        return String::new();
    }
    let start = start as usize;
    let len = len as usize;
    if start >= s.len() {
        return String::new();
    }
    let end = std::cmp::min(start + len, s.len());
    s[start..end].to_string()
}

#[axon_export]
fn axon_string_count(haystack: &str, needle: &str) -> i64 {
    if needle.is_empty() {
        return haystack.len() as i64;
    }
    haystack.matches(needle).count() as i64
}

#[axon_export]
fn axon_string_trim(s: &str) -> String {
    s.trim().to_string()
}
