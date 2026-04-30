/// FFI: Accepts a lowered IR string (prefixed `ok:lowered:`), builds the
/// native artifact via the host compiler workspace, and publishes the
/// resulting binary to `target/build/axon/axon`.
///
/// Protocol: returns `"ok"` on success, or `"error: ..."` on failure.
/// Handles self-overwrite by staging through a temp file + atomic rename.
#[axon_pub_export]
fn run_lowered_to_artifact(lowered: &str) -> String {
    if lowered.is_empty() {
        return "error: empty IR module".to_string();
    }
    if !lowered.starts_with("ok:lowered:") {
        return "error: lowering did not produce expected result".to_string();
    }
    let workspace_root = std::path::Path::new(".");
    let host_root = workspace_root.join("rust-self-compiler-for-axon");
    let host_root = if host_root.join("Cargo.toml").exists() {
        host_root
    } else {
        workspace_root.join("rust-backed-compiler-for-axon")
    };
    let host_target = host_root.join("target");
    let host_bin = host_target.join("debug/axon");

    if !host_bin.exists() {
        let mut build_cmd = std::process::Command::new("cargo");
        build_cmd.arg("build").arg("-p").arg("axon");
        build_cmd.current_dir(&host_root);
        build_cmd.env("CARGO_TARGET_DIR", &host_target);
        match build_cmd.output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return format!(
                    "error: host compiler build failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )
            }
            Err(e) => return format!("error: cannot build host compiler: {e}"),
        }
    }

    let mut native_build = std::process::Command::new(&host_bin);
    native_build.arg("build");
    native_build.current_dir(workspace_root);
    native_build.env("CARGO_TARGET_DIR", &host_target);
    match native_build.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return format!(
                "error: native build failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        }
        Err(e) => return format!("error: cannot run native build: {e}"),
    }

    let out_dir = std::path::Path::new("target/build/axon");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        return format!("error: cannot create {}: {e}", out_dir.display());
    }
    let out_bin = out_dir.join("axon");

    let native_artifact = match resolve_native_artifact_path() {
        Some(p) => p,
        None => return "error: native artifact not found after build".to_string(),
    };

    let out_abs = std::fs::canonicalize(out_dir).unwrap_or_else(|_| out_dir.to_path_buf());
    let exe_abs = std::env::current_exe().unwrap_or_default();
    let is_self_overwrite = exe_abs.starts_with(&out_abs);

    if is_self_overwrite {
        let tmp_name = format!("axon.tmp.{}", std::process::id());
        let tmp_path = out_dir.join(&tmp_name);
        if let Err(e) = std::fs::copy(&native_artifact, &tmp_path) {
            return format!("error: cannot stage artifact {}: {e}", tmp_path.display());
        }
        let new_path = out_dir.join("axon.new");
        if let Err(e) = std::fs::rename(&tmp_path, &new_path) {
            return format!("error: cannot publish artifact {}: {e}", new_path.display());
        }
    } else {
        if let Err(e) = std::fs::copy(&native_artifact, &out_bin) {
            return format!(
                "error: cannot publish artifact {}: {e}",
                out_bin.display()
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&out_bin, std::fs::Permissions::from_mode(0o755))
            {
                return format!(
                    "error: cannot set executable permissions on {}: {e}",
                    out_bin.display()
                );
            }
        }
    }

    let marker = out_dir.join("build-manifest.txt");
    let manifest = format!(
        "artifact\nstage=native-link\nsource-native={}\nout={}\nlowered={}\n",
        native_artifact.display(),
        out_bin.display()
        ,
        lowered
    );
    if let Err(e) = std::fs::write(&marker, manifest) {
        return format!("error: cannot write {}: {e}", marker.display());
    }
    "ok".to_string()
}

/// FFI: Executes the previously built `target/build/axon/axon` binary.
/// Returns `"ok:run"` or `"ok:run:<stdout>"` on success, `"error: ..."` on failure.
/// If the binary prints usage (compiler CLI), retries with `check` subcommand.
#[axon_pub_export]
fn launch_self_built() -> String {
    let target = std::path::Path::new("target/build/axon/axon");
    if !target.exists() {
        return format!(
            "error: {} not found, run build first",
            target.display()
        );
    }
    let run = std::process::Command::new(target).output();
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
                match std::process::Command::new(target).arg("check").output() {
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

/// FFI: Runs tests via the vendored (or fallback) Rust compiler workspace.
/// Makes `axon test` self-independent from any external host compiler.
///
/// Params: `target` — optional test target filter (empty string = all).
/// Returns `"ok:tests:passed"` or test output on success, `"error: ..."` on failure.
#[axon_pub_export]
fn run_tests_via_rust_compiler(target: &str) -> String {
    let workspace_root = std::path::Path::new(".");
    let host_root = workspace_root.join("rust-self-compiler-for-axon");
    let host_root = if host_root.join("Cargo.toml").exists() {
        host_root
    } else {
        workspace_root.join("rust-backed-compiler-for-axon")
    };
    let host_target = host_root.join("target");
    let host_bin = host_target.join("debug/axon");

    if !host_bin.exists() {
        let mut build_cmd = std::process::Command::new("cargo");
        build_cmd.arg("build").arg("-p").arg("axon");
        build_cmd.current_dir(&host_root);
        build_cmd.env("CARGO_TARGET_DIR", &host_target);
        match build_cmd.output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return format!(
                    "error: host compiler build failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )
            }
            Err(e) => return format!("error: cannot build host compiler: {e}"),
        }
    }

    let mut test_cmd = std::process::Command::new(&host_bin);
    test_cmd.arg("test");
    if !target.is_empty() {
        test_cmd.arg(target);
    }
    test_cmd.current_dir(workspace_root);
    test_cmd.env("CARGO_TARGET_DIR", &host_target);
    match test_cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                let text = stdout.trim();
                if text.is_empty() {
                    "ok:tests:passed".to_string()
                } else {
                    text.to_string()
                }
            } else {
                format!("error: tests failed: {}", stderr.trim())
            }
        }
        Err(e) => format!("error: cannot run tests: {e}"),
    }
}

/// Parses `build.ax` for a `project <name>` declaration.
/// Returns the project name if found. This is config-file reading, not compiler logic.
fn parse_project_name_from_build_ax() -> Option<String> {
    let build_ax = std::fs::read_to_string("build.ax").ok()?;
    for line in build_ax.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("project ") {
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

/// Resolves the path to the native artifact based on the project name
/// read from `build.ax`. Returns `target/build/<project>/<project>`.
fn resolve_native_artifact_path() -> Option<std::path::PathBuf> {
    let project = parse_project_name_from_build_ax()?;
    let direct = std::path::PathBuf::from("target/build").join(&project).join(&project);
    if direct.exists() {
        return Some(direct);
    }
    None
}

/// FFI: Copies the current `target/build/axon/axon` binary to
/// `target/build/axon/axon_{suffix}`, preserving executable permissions.
/// Returns `"ok"` on success or `"error: ..."` on failure.
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
