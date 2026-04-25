#[axon_export]
fn compiler_version() -> String {
    "0.1.0".to_string()
}

#[axon_export]
fn axon_fail(msg: &str) -> String {
    panic!("axon-lang: {msg}")
}
