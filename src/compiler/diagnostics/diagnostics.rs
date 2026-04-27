fn diagnostics_collector() -> &'static Mutex<Vec<String>> {
    static COLLECTOR: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    COLLECTOR.get_or_init(|| Mutex::new(Vec::new()))
}

fn format_diag_code_inner(num: u32) -> String {
    if num >= 100 {
        format!("W{:04}", num)
    } else {
        format!("E{:04}", num)
    }
}

// -- collector

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

// -- severity messages

#[axon_export]
fn message_is_error(s: &str) -> String {
    if s.is_empty() {
        return "no".to_string();
    }
    if s.starts_with("error:") {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

#[axon_export]
fn message_is_warning(s: &str) -> String {
    if s.is_empty() {
        return "no".to_string();
    }
    if s.starts_with("warning:") {
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

#[axon_export]
fn diag_error(kind: &str, msg: &str) -> String {
    format!("error: {}: {}", kind, msg)
}

#[axon_export]
fn diag_warn(kind: &str, msg: &str) -> String {
    format!("warning: {}: {}", kind, msg)
}

#[axon_export]
fn diag_note(kind: &str, msg: &str) -> String {
    format!("note: {}: {}", kind, msg)
}

#[axon_export]
fn diag_internal(msg: &str) -> String {
    format!("error: internal compiler failure: {}", msg)
}

// -- error code variants

#[axon_export]
fn format_diag_code(code_num: u32) -> String {
    format_diag_code_inner(code_num)
}

#[axon_export]
fn diag_error_code(code: u32, kind: &str, msg: &str) -> String {
    format!(
        "error[{}]: {}: {}",
        format_diag_code_inner(code),
        kind,
        msg
    )
}

#[axon_export]
fn diag_warn_code(code: u32, kind: &str, msg: &str) -> String {
    format!(
        "warning[{}]: {}: {}",
        format_diag_code_inner(code),
        kind,
        msg
    )
}

// -- file+line context

#[axon_export]
fn diag_error_at(file: &str, ln: u32, kind: &str, msg: &str) -> String {
    format!("error: {}:{}: {}: {}", file, ln, kind, msg)
}

#[axon_export]
fn diag_error_code_at(file: &str, ln: u32, code: u32, kind: &str, msg: &str) -> String {
    format!(
        "error[{}]: {}:{}: {}: {}",
        format_diag_code_inner(code),
        file,
        ln,
        kind,
        msg
    )
}
