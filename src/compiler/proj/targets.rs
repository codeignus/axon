// Mirrors targets.test.ax scopes. Rust wins over Axon in the bridge.

#[axon_pub_export]
fn classify_check_target(path: &str) -> String {
    match path {
        "" => "project".to_string(),
        p => classify_nonempty_path(p),
    }
}

#[axon_pub_export]
fn classify_test_target(path: &str) -> String {
    match path {
        "" => "test:project".to_string(),
        p => classify_nonempty_path(p),
    }
}

fn classify_nonempty_path(path: &str) -> String {
    match path {
        "." => String::from("dir:."),
        "./..." => String::from("dir-recursive:./"),
        "..." => String::from("dir-recursive:."),
        p if p.ends_with("/...") => format!("dir-recursive:{}", &p[..p.len() - 4]),
        p if p.ends_with(".ax") => format!("file:{}", p),
        p => format!("dir:{}", p),
    }
}
