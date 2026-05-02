// Bridge crate generation and static archive builds for `@rust`/`.rs` sidecars.
//
// Generates a Cargo.toml + lib.rs for the bridge, builds it with cargo,
// and returns the path to the produced staticlib. No policy — the bridge
// shape is determined by build.ax and the sidecar file list.

#[axon_pub_export]
fn foreign_build_rust_bridge_archive(project_root: &str, deps_raw: &str, sidecar_paths: &str) -> String {
    // Stub — the bridge is built by backend.rs::build_rust_bridge directly.
    format!(
        "ok:foreign-archive:project={}:deps_count={}:sidecars={}",
        project_root,
        deps_raw.lines().filter(|l| !l.trim().is_empty()).count(),
        sidecar_paths.split(',').filter(|s| !s.is_empty()).count()
    )
}
