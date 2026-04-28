// Target classification moved to targets.ax (Axon-native).
// This file is kept as a minimal placeholder for the module registry.

#[axon_pub_export]
fn classify_check_target(path: &str) -> String {
    // Delegate to Axon — this stub only exists to keep the module in the bridge.
    format!("stub:classify_check_target:{}", path)
}

#[axon_pub_export]
fn classify_test_target(path: &str) -> String {
    format!("stub:classify_test_target:{}", path)
}
