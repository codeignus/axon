// IR lowering module.
//
// The host compiler (rust-backed-compiler-for-axon) owns the actual MIR lowering
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
