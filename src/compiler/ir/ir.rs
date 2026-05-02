// IR lowering module.
//
// The bootstrap compiler owns the actual MIR lowering
// pipeline, including method lowering (e.g., `.len()` -> `string_len`). This module
// is a structural marker that collects modules for the native build step.

#[axon_export]
fn lower_module(source: &str) -> String {
    format!("ir:module:bytes={}", source.len())
}

#[axon_export]
fn lower_function(name: &str) -> String {
    format!("ir:function:{name}")
}

fn json_escape_path(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Phase 7 bridge: emit a versioned MIR **envelope** (JSON line + human-readable tail)
/// so `lower_project` is no longer only a file-count marker. Full AST→MIR lowering stays in `.ax`.
#[axon_pub_export]
fn lower_project(root: &str) -> String {
    let root_path = match root.is_empty() {
        true => project_entry_root_path(),
        false => PathBuf::from(root),
    };
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = collect_all_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();

    let mut json = String::from("{\"v\":2,\"kind\":\"axon-mir-envelope\",\"modules\":[");
    for (i, file) in files.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str("{\"path\":");
        json.push_str(&json_escape_path(file));
        json.push_str(",\"mir\":\"stub\"}");
    }
    json.push_str("]}");

    let mut ir = json;
    ir.push('\n');
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
    format!("ok:lowered:v2:{}", files.len())
}

// MIR string encoding helpers. String concatenation via `+` is not yet
// supported by the LLVM codegen for struct-typed values, so these Rust-side
// helpers perform the encoding instead.

#[axon_pub_export]
fn mir_encode2(prefix: &str, a: &str) -> String {
    format!("{prefix}{a}")
}

#[axon_pub_export]
fn mir_encode3(prefix: &str, a: &str, b: &str) -> String {
    format!("{prefix}{a}{b}")
}

#[axon_pub_export]
fn mir_encode4(prefix: &str, a: &str, b: &str, c: &str) -> String {
    format!("{prefix}{a}{b}{c}")
}

#[axon_pub_export]
fn mir_encode5(prefix: &str, a: &str, b: &str, c: &str, d: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}")
}

#[axon_pub_export]
fn mir_encode6(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}")
}

#[axon_pub_export]
fn mir_encode7(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}")
}

#[axon_pub_export]
fn mir_encode8(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str, g: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}{g}")
}

#[axon_pub_export]
fn mir_encode9(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str, g: &str, h: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}{g}{h}")
}

#[axon_pub_export]
fn mir_colon_parts2(a: &str, b: &str) -> String {
    format!("{a}:{b}")
}

#[axon_pub_export]
fn mir_colon_parts3(a: &str, b: &str, c: &str) -> String {
    format!("{a}:{b}:{c}")
}

#[axon_pub_export]
fn mir_colon_parts4(a: &str, b: &str, c: &str, d: &str) -> String {
    format!("{a}:{b}:{c}:{d}")
}
