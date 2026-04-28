fn diagnostics_collector() -> &'static Mutex<Vec<String>> {
    static COLLECTOR: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    COLLECTOR.get_or_init(|| Mutex::new(Vec::new()))
}

#[axon_export]
fn diag_clear() {
    diagnostics_collector().lock().unwrap().clear();
}

#[axon_export]
fn diag_push(msg: &str) {
    diagnostics_collector()
        .lock()
        .unwrap()
        .push(msg.to_string());
}

#[axon_export]
fn diag_has_errors() -> String {
    let diags = diagnostics_collector().lock().unwrap();
    for d in diags.iter() {
        if d.starts_with("error") {
            return "yes".to_string();
        }
    }
    "no".to_string()
}

#[axon_export]
fn diag_error_count() -> u32 {
    diagnostics_collector()
        .lock()
        .unwrap()
        .iter()
        .filter(|d| d.starts_with("error"))
        .count() as u32
}

#[axon_export]
fn diag_warning_count() -> u32 {
    diagnostics_collector()
        .lock()
        .unwrap()
        .iter()
        .filter(|d| d.starts_with("warning"))
        .count() as u32
}

#[axon_export]
fn diag_render_summary() -> String {
    let diags = diagnostics_collector().lock().unwrap();
    let errors = diags.iter().filter(|d| d.starts_with("error")).count();
    let warnings = diags.iter().filter(|d| d.starts_with("warning")).count();
    format!("{} error(s), {} warning(s)", errors, warnings)
}

#[axon_export]
fn diag_render_all() -> String {
    let diags = diagnostics_collector().lock().unwrap();
    diags.join("\n")
}

// Kept as FFI because message_is_error/warning/severity are called
// cross-module as foreign functions (main.ax, pipeline_check.ax, entry.ax).
// Formatting functions (diag_error, diag_warn, etc.) are now in diagnostic.ax.

#[axon_pub_export]
fn message_is_error(s: &str) -> String {
    if !s.is_empty() && s.starts_with("error:") {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

#[axon_export]
fn message_is_warning(s: &str) -> String {
    if !s.is_empty() && s.starts_with("warning:") {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

#[axon_export]
fn message_severity(s: &str) -> String {
    if s.starts_with("error") {
        "error".to_string()
    } else if s.starts_with("warning") {
        "warning".to_string()
    } else if s.starts_with("note") {
        "note".to_string()
    } else {
        "unknown".to_string()
    }
}

#[axon_pub_export]
fn diag_error(kind: &str, msg: &str) -> String {
    format!("error: stage={} code=E1000 reason={}", kind, msg)
}

#[axon_pub_export]
fn diag_warn(kind: &str, msg: &str) -> String {
    format!("warning: stage={} code=W1000 reason={}", kind, msg)
}

#[axon_pub_export]
fn diag_note(kind: &str, msg: &str) -> String {
    format!("note: stage={} code=N1000 reason={}", kind, msg)
}

#[axon_pub_export]
fn diag_internal(msg: &str) -> String {
    format!("error: stage=internal code=E0001 reason={}", msg)
}
