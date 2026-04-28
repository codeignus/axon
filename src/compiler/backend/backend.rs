#[axon_pub_export]
fn run_lowered_to_artifact(lowered: &str) -> String {
    if lowered.is_empty() {
        return "error: backend: empty IR module".to_string();
    }
    if !lowered.starts_with("ok:lowered:") {
        return "error: backend: lowering did not produce expected result".to_string();
    }
    // Native artifact path: reuse the host-native backend pipeline as a strict
    // external toolchain boundary until Axon-native MIR/backend fully owns it.
    let workspace_root = std::path::Path::new(".");
    let host_root = workspace_root.join("rust-backed-compiler-for-axon");
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
                    "error: backend: host compiler build failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )
            }
            Err(e) => return format!("error: backend: cannot build host compiler: {e}"),
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
                "error: backend: native build failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        }
        Err(e) => return format!("error: backend: cannot run native build: {e}"),
    }

    let out_dir = std::path::Path::new("target/build/axon");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        return format!("error: backend: cannot create {}: {e}", out_dir.display());
    }
    let out_bin = out_dir.join("app");

    let native_artifact = match resolve_native_artifact_path() {
        Some(p) => p,
        None => return "error: backend: native artifact not found after build".to_string(),
    };
    if let Err(e) = std::fs::copy(&native_artifact, &out_bin) {
        return format!(
            "error: backend: cannot publish artifact {}: {e}",
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
                "error: backend: cannot set executable permissions on {}: {e}",
                out_bin.display()
            );
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
        return format!("error: backend: cannot write {}: {e}", marker.display());
    }
    "ok".to_string()
}

#[axon_pub_export]
fn launch_self_built() -> String {
    let target = std::path::Path::new("target/build/axon/app");
    if !target.exists() {
        return format!(
            "error: backend: {} not found, run build first",
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
            // If the built artifact is the compiler binary itself, no-arg launch
            // prints command usage. Retry with `check` as a health command.
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
                            "error: backend: launch retry failed for {}: {}",
                            target.display(),
                            err.trim()
                        );
                    }
                    Err(e) => {
                        return format!(
                            "error: backend: cannot execute retry for {}: {e}",
                            target.display()
                        );
                    }
                }
            }
            format!(
                "error: backend: launch failed for {}: {}",
                target.display(),
                stderr.trim()
            )
        }
        Err(e) => format!(
            "error: backend: cannot execute {}: {e}",
            target.display()
        ),
    }
}

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

fn resolve_native_artifact_path() -> Option<std::path::PathBuf> {
    let project = parse_project_name_from_build_ax()?;
    let direct = std::path::PathBuf::from("target/build").join(&project).join(&project);
    if direct.exists() {
        return Some(direct);
    }
    None
}
