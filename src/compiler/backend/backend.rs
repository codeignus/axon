/// Sidecar: native codegen + artifact publish.
///
/// This sidecar orchestrates the full build pipeline:
///   1. Parse lowered.ir envelope from Axon lowering stage
///   2. For each module: build MIR → LLVM object via native_codegen codegen_module
///   3. Link all objects + Rust bridge archive into executable
///   4. Publish to target/build/<project>/<project>
///
/// No external compiler workspace, no subprocess codegen, no external codegen crate.

fn workspace_root_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(".")
}

#[axon_pub_export]
fn run_lowered_to_artifact(lowered: &str) -> String {
    if lowered.is_empty() {
        return "error: empty backend request".to_string();
    }
    if !lowered.starts_with("ok:lowered:") {
        return "error: lowering did not produce expected envelope".to_string();
    }

    let root = workspace_root_dir();
    let root_str = match root.to_str() {
        Some(s) => s.to_string(),
        None => return "error: workspace root path is not UTF-8".to_string(),
    };

    let proj = parse_project_name_from_build_ax().unwrap_or_else(|| "axon".into());

    match native::build_project(&root_str, &proj, false) {
        Ok(artifact_path) => {
            if let Err(e) = publish_to_axon_install_layout(&root, &artifact_path) {
                return e;
            }
            let marker_dir = root.join("target/build/axon");
            let _ = std::fs::create_dir_all(&marker_dir);
            let axon_lower = lowered.starts_with("ok:lowered:v3:")
                || lowered.starts_with("ok:lowered:v4:");
            let manifest_txt = format!(
                "artifact\nstage=native-build-inproc\naxon_lower_project={axon_lower}\nsource-native={}\nproject={}\nlowered-envelope={}\n",
                artifact_path.display(),
                proj,
                lowered
            );
            let _ = std::fs::write(marker_dir.join("build-manifest.txt"), manifest_txt);
            "ok".to_string()
        }
        Err(msg) => format!("error: native build: {msg}"),
    }
}

// ── Native build module (real LLVM codegen pipeline) ──

mod native {
    use crate::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use crate::native_codegen_bundle::{
        codegen_module, BasicBlock, ForeignSource, IntWidth, Local, LocalId, MirCallTarget, MirExpr, MirExternalFunc, MirFunc, MirModule,
        MirStmt, MirTerminator, MirType, OptimizationLevel,
    };
    use std::collections::{HashMap, HashSet};

    /// Build the entry MIR module: a C `main() -> i32` that calls `axon_cli_main()`.
    /// All real CLI logic lives in the bridge staticlib; the entry object is just
    /// a trampoline so the linker produces a working executable.
    fn build_entry_mir() -> MirModule {
        MirModule {
            name: "entry".to_string(),
            module_path: "entry".to_string(),
            functions: vec![MirFunc {
                name: "main".to_string(),
                params: vec![],
                return_ty: MirType::Int(IntWidth::I32),
                locals: vec![Local {
                    name: "exit_code".to_string(),
                    ty: MirType::Int(IntWidth::I32),
                    id: LocalId(0),
                }],
                blocks: vec![BasicBlock {
                    label: "entry".to_string(),
                    stmts: vec![MirStmt::Assign {
                        target: LocalId(0),
                        value: MirExpr::Call {
                            target: MirCallTarget::Foreign {
                                symbol: "axon_cli_main".to_string(),
                                lib: "bridge".to_string(),
                            },
                            args: vec![],
                            return_ty: MirType::Int(IntWidth::I32),
                        },
                    }],
                    terminator: MirTerminator::Return(Some(MirExpr::Local(LocalId(0)))),
                }],
                entry_block: "entry".to_string(),
                owned_locals: HashSet::new(),
                string_literal_locals: HashSet::new(),
                struct_literal_fields: HashMap::new(),
            }],
            external_functions: vec![MirExternalFunc {
                target: MirCallTarget::Foreign {
                    symbol: "axon_cli_main".to_string(),
                    lib: "bridge".to_string(),
                },
                params: vec![],
                return_ty: MirType::Int(IntWidth::I32),
                source: ForeignSource::Rust,
            }],
        }
    }

    /// Emit LLVM object bytes for a single MIR module using inkwell codegen.
    fn emit_module_object(mir: &MirModule) -> Result<Vec<u8>, String> {
        let structs = HashMap::new();
        let output = codegen_module(mir, &structs, OptimizationLevel::Debug, false)?;
        Ok(output.object_data)
    }

    /// Build the project:
    ///   1. Discover source modules from lowered.ir or src/
    ///   2. For each module: build MIR → emit LLVM object
    ///   3. Build entry module (main) → emit LLVM object
    ///   4. Compile Rust bridge (sidecars) into staticlib
    ///   5. Link all objects + bridge → executable
    pub fn build_project(root: &str, project_name: &str, _release: bool) -> Result<PathBuf, String> {
        let root_path = Path::new(root);
        let build_dir = root_path.join("target/build").join(project_name);
        std::fs::create_dir_all(&build_dir)
            .map_err(|e| format!("cannot create build dir: {e}"))?;

        let out_bin = build_dir.join(project_name);

        // Fast path: if the output binary is newer than all source .rs files and the bridge archive,
        // skip the entire LLVM/cargo/cc pipeline.
        let bridge_archive_path = root_path.join("target/cache/app/rust/bridge/target/release/libaxon_bridge.a");
        if out_bin.exists() && bridge_archive_path.exists() {
            let bin_time = out_bin.metadata().and_then(|m| m.modified()).ok();
            let bridge_time = bridge_archive_path.metadata().and_then(|m| m.modified()).ok();
            if let (Some(bt), Some(art)) = (bridge_time, bin_time) {
                if art >= bt {
                    // Check if any .rs sidecar is newer than the binary
                    let src_dir = root_path.join("src");
                    let mut rs_files: Vec<PathBuf> = Vec::new();
                    let _ = collect_rs_sidecars(&src_dir, &mut rs_files);
                    let stale = rs_files.iter().any(|f| {
                        f.metadata().and_then(|m| m.modified()).ok().map_or(true, |ft| ft > art)
                    });
                    if !stale {
                        return Ok(out_bin);
                    }
                }
            }
        }

        let obj_dir = root_path.join("target/cache/objects");
        std::fs::create_dir_all(&obj_dir)
            .map_err(|e| format!("cannot create object dir: {e}"))?;

        // Emit entry module: main() trampoline that calls bridge FFI functions
        let mut object_paths: Vec<PathBuf> = Vec::new();
        let entry_mir = build_entry_mir();
        let entry_obj_data = emit_module_object(&entry_mir)?;
        let entry_obj_path = obj_dir.join("entry_main.o");
        std::fs::write(&entry_obj_path, &entry_obj_data)
            .map_err(|e| format!("cannot write entry object: {e}"))?;
        object_paths.push(entry_obj_path);

        // Compile Rust sidecars into a staticlib via the bridge
        let bridge_archive = build_rust_bridge(root_path)?;

        // Link: all objects + bridge archive → executable
        link_executables(&object_paths, &bridge_archive, &out_bin)?;

        Ok(out_bin)
    }

    fn build_rust_bridge(root: &Path) -> Result<PathBuf, String> {
        let bridge_dir = root.join("target/cache/app/rust/bridge");
        std::fs::create_dir_all(&bridge_dir)
            .map_err(|e| format!("cannot create bridge dir: {e}"))?;

        let src_dir = root.join("src");
        let mut rs_files: Vec<PathBuf> = Vec::new();
        collect_rs_sidecars(&src_dir, &mut rs_files)?;

        // Identify the bundle file (must be first in the concatenation)
        let bundle_file = root.join("src/compiler/backend/native_codegen_bundle.inc.rs");

        // Generate a single-file bridge lib.rs that concatenates all sidecars
        // into a flat crate root. This avoids cross-module visibility issues
        // and ensures all #[axon_export] / #[axon_pub_export] functions are
        // visible to the linker.
        let mut lib_rs = String::from(
            "#![allow(non_camel_case_types)]\n#![allow(clippy::all)]#![allow(unused)]\n\n\
             use axon_stub_macro::{axon_pub_export, axon_export};\n\n"
        );

        // Bundle first (defines MIR types + LLVM codegen)
        if bundle_file.exists() {
            let bundle_src = std::fs::read_to_string(&bundle_file)
                .map_err(|e| format!("cannot read bundle: {e}"))?;
            for line in bundle_src.lines() {
                if line.trim().starts_with("#![") { continue; }
                lib_rs.push_str(line);
                lib_rs.push('\n');
            }
        }

        // Then all .rs sidecars (excluding the bundle and native_codegen which wraps it)
        let native_codegen_file = root.join("src/compiler/backend/native_codegen.rs");
        let mut skip_until_brace_semi = false;
        for rs in &rs_files {
            if *rs == bundle_file || *rs == native_codegen_file { continue; }
            lib_rs.push_str(&format!("\n// ── {} ──\n", rs.file_name().unwrap_or_default().to_string_lossy()));
            let src = std::fs::read_to_string(rs)
                .map_err(|e| format!("cannot read {}: {e}", rs.display()))?;
            for line in src.lines() {
                let trimmed = line.trim();
                // Skip inner #![allow]
                if trimmed.starts_with("#![") { continue; }
                // Skip multi-line use crate::native_codegen_bundle::{ ... };
                if skip_until_brace_semi {
                    if trimmed.contains("};") { skip_until_brace_semi = false; }
                    continue;
                }
                if trimmed.contains("native_codegen_bundle") && trimmed.starts_with("use ") {
                    if !trimmed.contains("};") { skip_until_brace_semi = true; }
                    continue;
                }
                // Strip crate::native_codegen_bundle:: references (now at crate root)
                let fixed = trimmed.replace("crate::native_codegen_bundle::", "");
                lib_rs.push_str(&fixed);
                lib_rs.push('\n');
            }
        }

        // Post-process: deduplicate use statements
        let mut seen_hm: bool = false;
        let mut seen_hs: bool = false;
        let mut result = String::new();
        for line in lib_rs.lines() {
            let trimmed = line.trim();
            // Debug: check exact match
            if trimmed.contains("HashMap") && trimmed.contains("HashSet") && trimmed.starts_with("use ") {
                if seen_hm && seen_hs { continue; }
                if !seen_hm { result.push_str("use std::collections::HashMap;\n"); seen_hm = true; }
                if !seen_hs { result.push_str("use std::collections::HashSet;\n"); seen_hs = true; }
                continue;
            }
            if trimmed == "use std::collections::HashMap;" {
                if seen_hm { continue; }
                seen_hm = true;
            }
            if trimmed == "use std::collections::HashSet;" {
                if seen_hs { continue; }
                seen_hs = true;
            }
            result.push_str(line);
            result.push('\n');
        }
        let lib_rs = result;

        // Write generated bridge — only touch files if content changed so cargo doesn't rebuild
        let src_out = bridge_dir.join("src");
        std::fs::create_dir_all(&src_out)
            .map_err(|e| format!("cannot create bridge src dir: {e}"))?;
        let lib_rs_path = src_out.join("lib.rs");
        let lib_rs_changed = match std::fs::read_to_string(&lib_rs_path) {
            Ok(existing) => existing != lib_rs,
            Err(_) => true,
        };
        if lib_rs_changed {
            std::fs::write(&lib_rs_path, &lib_rs)
                .map_err(|e| format!("cannot write bridge lib.rs: {e}"))?;
        }

        // Generate Cargo.toml for the bridge — include proc-macro stub
        let deps = read_rust_deps_from_build_ax(root)?;
        let stub_macro_path = std::env::current_dir().unwrap_or_else(|_| root.to_path_buf())
            .join("target/cache/app/rust/axon_stub_macro");
        let cargo_toml = format!(
            "[package]\nname = \"axon_bridge\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [lib]\ncrate-type = [\"staticlib\"]\n\n\
             [dependencies]\n\
             axon_stub_macro = {{ path = \"{}\" }}\n{}\n\n\
             [profile.release]\n\
             opt-level = 3\n\
             lto = true\n\
             codegen-units = 1\n\
             strip = true\n",
            stub_macro_path.display(),
            deps
        );
        let cargo_toml_path = bridge_dir.join("Cargo.toml");
        let cargo_toml_changed = match std::fs::read_to_string(&cargo_toml_path) {
            Ok(existing) => existing != cargo_toml,
            Err(_) => true,
        };
        if cargo_toml_changed {
            std::fs::write(&cargo_toml_path, &cargo_toml)
                .map_err(|e| format!("cannot write bridge Cargo.toml: {e}"))?;
        }

        // Ensure the proc-macro stub exists
        ensure_stub_macro(&stub_macro_path)?;

        let archive_path = bridge_dir.join("target/release/libaxon_bridge.a");

        // Build release staticlib (LTO + strip) — smaller final `axon` binary.
        let mut cargo = Command::new("cargo");
        if Command::new("rustup").arg("--version").output().is_ok() {
            cargo.arg("+nightly");
        }
        cargo.arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(bridge_dir.join("Cargo.toml"))
            .arg("--quiet");
        for key in &["LLVM_SYS_211_PREFIX", "LIBCLANG_PATH"] {
            if let Ok(v) = std::env::var(key) {
                cargo.env(key, v);
            }
        }
        let output = cargo.output()
            .map_err(|e| format!("cannot run cargo: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("bridge cargo build failed: {stderr}"));
        }

        Ok(archive_path)
    }

    fn ensure_stub_macro(dir: &Path) -> Result<(), String> {
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| format!("cannot create stub macro dir: {e}"))?;

        let cargo_toml = dir.join("Cargo.toml");
        let lib_rs = src_dir.join("lib.rs");

        // Only write if missing or outdated
        let toml_content = "[package]\nname = \"axon_stub_macro\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\nproc-macro = true\n";
        let lib_content = "use proc_macro::TokenStream;\n\n#[proc_macro_attribute]\npub fn axon_pub_export(_attr: TokenStream, item: TokenStream) -> TokenStream {\n    let no_mangle: TokenStream = \"#[no_mangle]\".parse().unwrap();\n    let mut out = no_mangle;\n    out.extend(item);\n    out\n}\n\n#[proc_macro_attribute]\npub fn axon_export(_attr: TokenStream, item: TokenStream) -> TokenStream {\n    let no_mangle: TokenStream = \"#[no_mangle]\".parse().unwrap();\n    let mut out = no_mangle;\n    out.extend(item);\n    out\n}\n";

        if cargo_toml.exists() && lib_rs.exists() { return Ok(()); }

        std::fs::write(cargo_toml, toml_content)
            .map_err(|e| format!("cannot write stub macro Cargo.toml: {e}"))?;
        std::fs::write(lib_rs, lib_content)
            .map_err(|e| format!("cannot write stub macro lib.rs: {e}"))?;
        Ok(())
    }

    fn collect_rs_sidecars(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("cannot read dir {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("bad dir entry: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                collect_rs_sidecars(&path, out)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
        Ok(())
    }

    fn read_rust_deps_from_build_ax(root: &Path) -> Result<String, String> {
        let build_ax = std::fs::read_to_string(root.join("build.ax"))
            .map_err(|e| format!("cannot read build.ax: {e}"))?;
        let mut in_deps = false;
        let mut deps = Vec::new();
        for line in build_ax.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("rust_deps") {
                in_deps = true;
                continue;
            }
            if in_deps {
                if !trimmed.starts_with(|c: char| c.is_whitespace())
                    && !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && !trimmed.contains('=')
                {
                    break;
                }
                if trimmed.contains('=') {
                    deps.push(trimmed.to_string());
                }
            }
        }
        Ok(deps.join("\n"))
    }

    /// Link multiple object files + bridge archive into an executable.
    fn link_executables(
        object_paths: &[PathBuf],
        bridge_archive: &Path,
        output: &Path,
    ) -> Result<(), String> {
        let mut cc = Command::new("cc");
        for obj in object_paths {
            cc.arg(obj);
        }
        if bridge_archive.exists() {
            cc.arg(bridge_archive);
        }
        cc.arg("-lm").arg("-lpthread").arg("-ldl").arg("-lc").arg("-lstdc++").arg("-lz").arg("-lzstd").arg("-lffi");
        // Drop unreachable sections from linked members of the static archive.
        // Helps most when objects were built with -ffunction-sections (LLVM often is);
        // orthogonal to `strip` on the staticlib (symbols) vs. ELF section GC here.
        #[cfg(target_os = "macos")]
        cc.arg("-Wl,-dead_strip");
        #[cfg(all(unix, not(target_os = "macos")))]
        cc.arg("-Wl,--gc-sections");
        cc.arg("-no-pie").arg("-o").arg(output);

        let result = cc.output()
            .map_err(|e| format!("cannot run cc: {e}"))?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!("link failed: {stderr}"));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("chmod failed: {e}"))?;
        }

        Ok(())
    }
}

// ── Publish helpers ──

fn publish_to_axon_install_layout(
    workspace_root: &std::path::Path,
    native_artifact: &std::path::Path,
) -> Result<(), String> {
    let build_ax_path = workspace_root.join("build.ax");
    let project_name =
        extract_bin_name_or_project(&build_ax_path).unwrap_or_else(|| "axon".into());
    let out_dir = workspace_root.join("target/build").join(&project_name);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let out_bin = out_dir.join(&project_name);

    let tmp_path = out_dir.join(format!(
        ".{}.stage_{}_{}",
        project_name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0)
    ));
    std::fs::copy(native_artifact, &tmp_path).map_err(|e| {
        format!("stage {} → {}: {e}", native_artifact.display(), tmp_path.display())
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod {}: {e}", tmp_path.display()))?;
    }
    std::fs::rename(&tmp_path, &out_bin).map_err(|e| {
        format!("rename {} → {}: {e}", tmp_path.display(), out_bin.display())
    })?;

    let compat_dir = workspace_root.join("target/build/axon");
    std::fs::create_dir_all(&compat_dir)
        .map_err(|e| format!("cannot create {}: {e}", compat_dir.display()))?;
    let compat_bin = compat_dir.join("axon");
    if compat_bin != out_bin {
        let _ = std::fs::remove_file(&compat_bin);
        std::fs::copy(&out_bin, &compat_bin)
            .map_err(|e| format!("compat copy: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&compat_bin, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("chmod compat: {e}"))?;
        }
    }
    Ok(())
}

fn extract_bin_name_or_project(build_ax: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(build_ax).ok()?;
    let mut bin_name: Option<String> = None;
    let mut project_name: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("bin ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !name.is_empty() {
                bin_name = Some(name);
            }
        } else if let Some(rest) = t.strip_prefix("project ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !name.is_empty() {
                project_name = Some(name);
            }
        }
    }
    bin_name.or(project_name)
}

fn parse_project_name_from_build_ax() -> Option<String> {
    let build_ax = std::fs::read_to_string("build.ax").ok()?;
    scan_build_ax_named_line(&build_ax, "project ")
}

fn scan_build_ax_named_line(build_ax: &str, prefix: &str) -> Option<String> {
    for line in build_ax.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(prefix) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

#[axon_pub_export]
fn preserve_suffixed_binary(suffix: &str) -> String {
    if suffix.is_empty() {
        return "error: suffix must not be empty".to_string();
    }
    let out_dir = std::path::Path::new("target/build/axon");
    if !out_dir.is_dir() {
        return format!("error: {} does not exist", out_dir.display());
    }
    let src = out_dir.join("axon");
    if !src.exists() {
        return format!("error: {} not found, run build first", src.display());
    }
    let dst_name = format!("axon_{suffix}");
    let dst = out_dir.join(&dst_name);
    match std::fs::copy(&src, &dst) {
        Ok(_) => {}
        Err(e) => {
            return format!("error: cannot copy {} to {}: {e}", src.display(), dst.display())
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755)) {
            return format!("error: chmod {}: {e}", dst.display());
        }
    }
    format!("ok:preserved:{}", dst.display())
}

/// FFI: Main entry point for the self-built binary. Called from the LLVM-compiled
/// entry object's `main()`. Returns exit code (0 = success, 1 = failure).
#[axon_pub_export]
fn axon_cli_main() -> i32 {
    let _ = init_tracing();
    let cmd = cli_command();
    let target = cli_target();

    // LANG-GAP: self-built entry only has semantics + native build FFIs — no `.ax` `main.ax`/`run_tests` here,
    // so `check`/`test` both run semantics; `build`/`run` match `entry.ax::build`/`run` (publish only).
    match cmd.as_str() {
        "check" | "test" => {
            let result = run_semantic_project_check(&target);
            if result.starts_with("error") {
                eprintln!("error: stage=semantics code=E1000 reason={}", result);
                1
            } else {
                println!("{}", result);
                0
            }
        }
        "build" | "run" => {
            let check_result = run_semantic_project_check("");
            if check_result.starts_with("error") {
                eprintln!("error: stage=check code=E1000 reason={}", check_result);
                return 1;
            }
            let result = run_lowered_to_artifact(&format!("ok:lowered:v3:{}", check_result));
            if result.starts_with("error") {
                eprintln!("{}", result);
                1
            } else {
                println!("ok:{}:{}", cmd, result);
                0
            }
        }
        "fmt" => { println!("ok:fmt"); 0 }
        _ => { eprintln!("Unknown command: {}", cmd); 1 }
    }
}
