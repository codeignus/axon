/// FFI: Checks whether `rustc` is available and functional.
/// Returns `"ok"` or `"error: toolchain: ..."`.
#[axon_export]
fn check_rust_toolchain() -> String {
    match std::process::Command::new("rustc").arg("--version").output() {
        Ok(output) if output.status.success() => "ok".to_string(),
        Ok(_) => "error: toolchain: rustc found but not working".to_string(),
        Err(_) => "error: toolchain: rustc not found".to_string(),
    }
}

/// FFI: Checks whether the C compiler (from $CC or `cc`) is available.
/// Returns `"ok"` or `"error: toolchain: ..."`.
#[axon_export]
fn check_cc_toolchain() -> String {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    match std::process::Command::new(&cc).arg("--version").output() {
        Ok(output) if output.status.success() => "ok".to_string(),
        Ok(_) => "error: toolchain: cc found but not working".to_string(),
        Err(_) => "error: toolchain: cc not found".to_string(),
    }
}

/// FFI: Checks whether `cargo` is available.
/// Returns `"ok"` or `"error: toolchain: ..."`.
#[axon_export]
fn check_cargo_available() -> String {
    match std::process::Command::new("cargo").arg("--version").output() {
        Ok(output) if output.status.success() => "ok".to_string(),
        Ok(_) => "error: toolchain: cargo found but not working".to_string(),
        Err(_) => "error: toolchain: cargo not found".to_string(),
    }
}

/// FFI: Validates both `rustc` and `cc` are available in one call.
/// Returns `"ok"` if both work, otherwise the first error encountered.
#[axon_export]
fn validate_native_toolchain() -> String {
    match std::process::Command::new("rustc").arg("--version").output() {
        Ok(output) if output.status.success() => {}
        Ok(_) => return "error: toolchain: rustc found but not working".to_string(),
        Err(_) => return "error: toolchain: rustc not found".to_string(),
    }
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    match std::process::Command::new(&cc).arg("--version").output() {
        Ok(output) if output.status.success() => "ok".to_string(),
        Ok(_) => "error: toolchain: cc found but not working".to_string(),
        Err(_) => "error: toolchain: cc not found".to_string(),
    }
}
