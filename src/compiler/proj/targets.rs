// Mirrors `targets.test.ax` / `targets.ax` scopes — Rust wins over Axon in the bridge.
// Same path rules as `pipeline_check/test_fmt.rs` `run_project_tests`.

#[axon_pub_export]
fn classify_check_target(path: &str) -> String {
    if path.is_empty() {
        "project".to_string()
    } else {
        classify_nonempty_path(path)
    }
}

#[axon_pub_export]
fn classify_test_target(path: &str) -> String {
    if path.is_empty() {
        "test:project".to_string()
    } else {
        classify_nonempty_path(path)
    }
}

fn classify_nonempty_path(path: &str) -> String {
    if path == "." {
        String::from(concat!("dir", ":", "."))
    } else if path == "./..." {
        String::from("dir-recursive:./")
    } else if path == "..." {
        String::from("dir-recursive:.")
    } else if path.ends_with("/...") {
        format!("dir-recursive:{}", &path[..path.len() - 4])
    } else if path.ends_with(".ax") {
        format!("file:{}", path)
    } else {
        format!("dir:{}", path)
    }
}
