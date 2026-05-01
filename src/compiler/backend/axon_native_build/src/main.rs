//! Migration driver: runs the real check/build/test pipeline from `axon-codegen` (Rust)
//! against the cwd project, then publishes the app binary under `target/build/<bin>/`.

use axon_codegen::target_resolution::find_project_root_from_cwd;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args_os()
        .map(|os| os.to_string_lossy().into_owned())
        .skip(1)
        .collect();
    if args.is_empty() {
        eprintln!("usage: axon-native-build check [path]|build [--release]|test [path]");
        std::process::exit(2);
    }
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: cwd: {e}");
            std::process::exit(1);
        }
    };

    match args[0].as_str() {
        "check" => {
            let input = args.get(1).filter(|s| !s.is_empty());
            let result = axon_codegen::compile::check_target(&cwd, input.map(|s| s.as_str()));
            for d in &result.diagnostics {
                eprintln!("{}: {}", d.severity.as_str(), d.message);
            }
            std::process::exit(result.exit_code);
        }
        "build" => {
            let release = args.iter().any(|a| a == "--release");
            let root = match find_project_root_from_cwd(&cwd) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };
            let result = axon_codegen::compile::build(root.to_str().unwrap_or("."), release);
            for d in &result.diagnostics {
                eprintln!("{}: {}", d.severity.as_str(), d.message);
            }
            if !result.success() {
                std::process::exit(result.exit_code.max(1));
            }
            let native = match result.binary_path {
                Some(p) => p,
                None => {
                    eprintln!("error: build reported success but no binary path");
                    std::process::exit(1);
                }
            };
            if let Err(e) = publish_to_axon_install_layout(&cwd, &native) {
                eprintln!("{e}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        "test" => {
            let path = args.get(1).filter(|s| !s.is_empty()).map(|s| s.as_str());
            match axon_codegen::compile::run_tests_target(&cwd, path, None) {
                Ok(summary) => {
                    println!("running {} test(s)", summary.results.len());
                    for case in &summary.results {
                        if case.ok {
                            println!("  {} ... ok", case.name);
                        } else {
                            eprintln!("  {} ... FAILED", case.name);
                            if let Some(err) = &case.error {
                                eprintln!("{:?}", err.kind);
                            }
                        }
                        if !case.stdout.is_empty() {
                            print!("{}", case.stdout);
                        }
                    }
                    println!();
                    println!("{} passed, {} failed", summary.passed, summary.failed);
                    std::process::exit(if summary.failed == 0 {
                        0
                    } else {
                        1
                    });
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("unknown command: {:?}", args[0]);
            std::process::exit(2);
        }
    }
}

/// Copy built artifact into `target/build/<project>/<project>`.
///
/// Always stages to a temp file then renames onto `out_bin`.  The parent CLI process may
/// still be the previous `target/build/axon/axon` image — opening that path with `copy`
/// truncates/corrupts it.
fn publish_to_axon_install_layout(workspace_root: &Path, native_artifact: &Path) -> Result<(), String> {
    let build_ax_path = workspace_root.join("build.ax");
    let project_name =
        extract_bin_name_or_project(&build_ax_path).unwrap_or_else(|| "axon".into());
    let out_dir = workspace_root.join("target/build").join(&project_name);
    std::fs::create_dir_all(&out_dir).map_err(|e| {
        format!(
            "error: cannot create {}: {e}",
            out_dir.display()
        )
    })?;
    let out_bin = out_dir.join(&project_name);

    // Stage → chmod → atomic rename.  Never open `out_bin` for write (the running
    // process image lives there and `copy` would truncate it to zero bytes).
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
        format!(
            "error: stage artifact {} → {}: {e}",
            native_artifact.display(),
            tmp_path.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755)).map_err(
            |e| format!("error: chmod staged {}: {e}", tmp_path.display()),
        )?;
    }
    std::fs::rename(&tmp_path, &out_bin).map_err(|e| {
        format!(
            "error: install {} → {}: {e}",
            tmp_path.display(),
            out_bin.display()
        )
    })?;

    // Fixed layout expected by tooling: `target/build/axon/axon`
    let compat_dir = workspace_root.join("target/build/axon");
    std::fs::create_dir_all(&compat_dir).map_err(|e| {
        format!(
            "error: cannot create {}: {e}",
            compat_dir.display()
        )
    })?;
    let compat_bin = compat_dir.join("axon");

    // Only create the compat copy when it is a *different* path from out_bin
    // (for project "axon" they are the same inode).
    if compat_bin != out_bin {
        let _ = std::fs::remove_file(&compat_bin);
        std::fs::copy(&out_bin, &compat_bin).map_err(|e| {
            format!(
                "error: cannot publish compat {} ← {}: {e}",
                compat_bin.display(),
                out_bin.display()
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&compat_bin, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| {
                    format!("error: chmod compat {}: {e}", compat_bin.display())
                })?;
        }
    }

    Ok(())
}

fn extract_bin_name_or_project(build_ax: &Path) -> Option<String> {
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
