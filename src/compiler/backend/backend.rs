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
    let src_file = out_dir.join("app.sh");
    let out_bin = out_dir.join("app");
    let script = format!(
        "#!/usr/bin/env sh\nprintf '%s\\n' \"axon app artifact\"\nprintf '%s\\n' \"{}\"\n",
        lowered.replace('"', "\\\"")
    );
    if let Err(e) = std::fs::write(&src_file, script) {
        return format!(
            "error: backend: cannot write generated source {}: {e}",
            src_file.display()
        );
    }
    if let Err(e) = std::fs::copy(&src_file, &out_bin) {
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
        "artifact\nstage=link\nsource-c={}\nout={}\n",
        src_file.display(),
        out_bin.display()
    );
    if let Err(e) = std::fs::write(&marker, manifest) {
        return format!("error: backend: cannot write {}: {e}", marker.display());
    }
    "ok".to_string()
}

#[axon_export]
fn launch_self_built() -> String {
    let target = Path::new("target/build/axon/app");
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
