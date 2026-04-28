#[axon_export]
fn run_ownership_check(source: &str) -> String {
    if source.contains("dealloc(") || source.contains("free(") {
        return "error: ownership: manual deallocation is forbidden".to_string();
    }
    if source.contains("condition_scope_consume(") {
        return "error: ownership: manual condition consume is forbidden".to_string();
    }
    if source.contains("condition_scope_begin(") {
        return "error: ownership: manual condition scope begin is forbidden".to_string();
    }
    "ok:ownership-snippet".to_string()
}
