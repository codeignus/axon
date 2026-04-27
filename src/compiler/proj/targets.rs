fn classify_check_target_impl(path: &str) -> String {
    if path.is_empty() {
        "project".to_string()
    } else if path == "." {
        "dir:.".to_string()
    } else if path == "./..." {
        "dir-recursive:./".to_string()
    } else if path == "..." {
        "dir-recursive:.".to_string()
    } else if path.ends_with("/...") {
        format!("dir-recursive:{}", &path[..path.len() - 4])
    } else if path.ends_with(".ax") {
        format!("file:{}", path)
    } else {
        format!("dir:{}", path)
    }
}

fn classify_test_target_impl(path: &str) -> String {
    if path.is_empty() {
        "test:project".to_string()
    } else if path == "." {
        "dir:.".to_string()
    } else if path == "./..." {
        "dir-recursive:./".to_string()
    } else if path == "..." {
        "dir-recursive:.".to_string()
    } else if path.ends_with("/...") {
        format!("dir-recursive:{}", &path[..path.len() - 4])
    } else if path.ends_with(".ax") {
        format!("file:{}", path)
    } else {
        format!("dir:{}", path)
    }
}

#[axon_export]
fn classify_check_target(path: &str) -> String {
    classify_check_target_impl(path)
}

#[axon_export]
fn classify_test_target(path: &str) -> String {
    classify_test_target_impl(path)
}
