#[axon_export]
fn run_ownership_check(source: &str) -> String {
    if source.contains("dealloc(") || source.contains("free(") {
        return "error: manual deallocation is forbidden".to_string();
    }
    if source.contains("condition_scope_consume(") {
        return "error: manual condition consume is forbidden".to_string();
    }
    if source.contains("condition_scope_begin(") {
        return "error: manual condition scope begin is forbidden".to_string();
    }
    // No text-based “merge conflict” heuristics: `mut` reassign inside if/elif/else is
    // modeled as pointer/slot updates per control-flow path; last survivor owns heap data.
    // See `src/compiler/ownership.ax` and codegen `owned_locals`.
    "ok:ownership-snippet".to_string()
}
