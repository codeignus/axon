/// Sidecar: native codegen driver + artifact publish. Builds the migration driver
/// binary via `src/Cargo.toml` (the single Cargo package under src/) which links
/// `axon-codegen`. No separate Cargo workspace — the Cargo.toml lives beside the
/// .ax files, matching the sidecar policy in AGENTS.md.
///
// LANG-GAP: using sidecar for native codegen until Axon codegen lands.
// The axon-codegen crate pulls in LLVM (C++ library) via inkwell, which
// cannot be linked through the staticlib bridge used by .rs sidecars.
// Instead, src/Cargo.toml produces a binary (axon-native-build) that this
// sidecar invokes as a subprocess.

const MIGRATION_CRATE_REL: &str = "src/Cargo.toml";

fn workspace_root_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(".")
}

fn default_migration_binary_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("target/native-build-driver/debug/axon-native-build")
}

fn ensure_migration_driver_binary(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let override_bin = std::env::var("AXON_NATIVE_BUILD_BIN").unwrap_or_default();
    if !override_bin.is_empty() {
        let p = std::path::PathBuf::from(&override_bin);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "error: AXON_NATIVE_BUILD_BIN set but missing: {}",
            p.display()
        ));
    }

    let candidate = default_migration_binary_path(root);
    if candidate.is_file() {
        return Ok(candidate);
    }

    let manifest_abs = workspace_root_dir().join(MIGRATION_CRATE_REL);
    if !manifest_abs.is_file() {
        return Err(format!(
            "error: migration manifest missing {}; cannot build native driver",
            manifest_abs.display()
        ));
    }

    let manifest_str = manifest_abs
        .to_str()
        .ok_or_else(|| "error: manifest path must be UTF-8".to_string())?;
    let target_dir = root.join("target/native-build-driver");
    let mut cargo = std::process::Command::new("cargo");
    // Use `+nightly` only when rustup is present; systems with a native nightly-ish
    // toolchain (e.g. Rust ≥1.95) build edition-2024 without it.
    if std::process::Command::new("rustup").arg("--version").output().is_ok() {
        cargo.arg("+nightly");
    }
    cargo.arg("build");
    cargo.arg("--manifest-path").arg(manifest_str);
    cargo.arg("-p").arg("axon-sidecars");
    cargo.arg("--quiet");
    cargo.env("CARGO_TARGET_DIR", &target_dir);
    for key in [
        "LLVM_SYS_211_PREFIX",
        "LLVM_SYS_210_PREFIX",
        "LIBCLANG_PATH",
        "LIBCLANG_STATIC_PATH",
    ] {
        if let Ok(v) = std::env::var(key) {
            cargo.env(key, v);
        }
    }
    match cargo.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return Err(format!(
                "error: cargo build migration driver failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Err(e) => return Err(format!("error: cannot run cargo for migration driver: {e}")),
    }

    let built = target_dir.join("debug/axon-native-build");
    if !built.is_file() {
        return Err("error: migration driver binary not found after build".to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&built, std::fs::Permissions::from_mode(0o755));
    }

    Ok(built)
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
    let driver = match ensure_migration_driver_binary(&root) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let mut cmd = std::process::Command::new(&driver);
    cmd.arg("build");
    cmd.current_dir(&root);
    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let mut combined = stderr;
            if !stdout.is_empty() {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str(&stdout);
            }
            if combined.is_empty() {
                combined = format!("native build exited with {}", out.status);
            }
            return format!("error: native build: {combined}");
        }
        Err(e) => return format!("error: cannot run migration driver build: {e}"),
    }

    let marker_dir = root.join("target/build/axon");
    if let Err(e) = std::fs::create_dir_all(&marker_dir) {
        return format!("error: cannot create {}: {e}", marker_dir.display());
    }
    let marker = marker_dir.join("build-manifest.txt");
    let proj = parse_project_name_from_build_ax().unwrap_or_else(|| "axon".into());
    let native = root.join("target/build").join(&proj).join(&proj);
    let axon_lower = lowered.starts_with("ok:lowered:v3:");
    let manifest_txt = format!(
        "artifact\nstage=migration-native-build\naxon_lower_project={axon_lower}\nmigration-driver={}\nsource-native={}\nproject={}\nlowered-envelope={}\n",
        driver.display(),
        native.display(),
        proj,
        lowered
    );
    if let Err(e) = std::fs::write(&marker, manifest_txt) {
        return format!("error: cannot write {}: {e}", marker.display());
    }
    "ok".to_string()
}

/// FFI: Executes `target/build/axon/axon` (compiler install layout expected by CLI).
#[axon_pub_export]
fn launch_self_built() -> String {
    let build_ax_bin = infer_install_binary_path();
    let target = if build_ax_bin.is_file() {
        build_ax_bin
    } else {
        std::path::PathBuf::from("target/build/axon/axon")
    };

    if !target.exists() {
        return format!(
            "error: {} not found, run build first",
            target.display()
        );
    }
    let run = std::process::Command::new(&target).output();
    match run {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let text = stdout.trim();
            if text.is_empty() {
                "ok:run".to_string()
            } else {
                format!("ok:run:{text}")
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Usage:")
                && stderr.contains("Commands:")
                && stderr.contains("check")
            {
                match std::process::Command::new(&target).arg("check").output() {
                    Ok(retry) if retry.status.success() => {
                        let out = String::from_utf8_lossy(&retry.stdout);
                        let text = out.trim();
                        if text.is_empty() {
                            return "ok:run".to_string();
                        }
                        return format!("ok:run:{text}");
                    }
                    Ok(retry) => {
                        let err = String::from_utf8_lossy(&retry.stderr);
                        return format!(
                            "error: launch retry failed for {}: {}",
                            target.display(),
                            err.trim()
                        );
                    }
                    Err(e) => {
                        return format!(
                            "error: cannot execute retry for {}: {e}",
                            target.display()
                        );
                    }
                }
            }
            format!(
                "error: launch failed for {}: {}",
                target.display(),
                stderr.trim()
            )
        }
        Err(e) => format!(
            "error: cannot execute {}: {e}",
            target.display()
        ),
    }
}

#[axon_pub_export]
fn run_compiler_tests_native(target: &str) -> String {
    let root = workspace_root_dir();
    let driver = match ensure_migration_driver_binary(&root) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut cmd = std::process::Command::new(&driver);
    cmd.arg("test");
    if !target.is_empty() {
        cmd.arg(target);
    }
    cmd.current_dir(&root);
    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                let text = stdout.trim();
                if text.is_empty() {
                    stderr.trim().to_string()
                } else {
                    text.to_string()
                }
            } else {
                format!(
                    "error: tests failed: {}",
                    if stderr.trim().is_empty() {
                        stdout.trim()
                    } else {
                        stderr.trim()
                    }
                )
            }
        }
        Err(e) => format!("error: cannot run tests: {e}"),
    }
}

fn infer_install_binary_path() -> std::path::PathBuf {
    let nm = infer_bin_target_name_from_build_ax().unwrap_or_else(|| "axon".into());
    std::path::PathBuf::from("target/build").join(&nm).join(nm)
}

/// Reads `project <name>` from `build.ax`.
fn parse_project_name_from_build_ax() -> Option<String> {
    let build_ax = std::fs::read_to_string("build.ax").ok()?;
    scan_build_ax_named_line(&build_ax, "project ")
}

fn infer_bin_target_name_from_build_ax() -> Option<String> {
    let build_ax = std::fs::read_to_string("build.ax").ok()?;
    scan_build_ax_named_line(&build_ax, "bin ").or_else(|| scan_build_ax_named_line(&build_ax, "project "))
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
            return format!(
                "error: cannot copy {} to {}: {e}",
                src.display(),
                dst.display()
            )
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) =
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755))
        {
            return format!(
                "error: cannot set executable permissions on {}: {e}",
                dst.display()
            );
        }
    }
    format!("ok:preserved:{}", dst.display())
}
