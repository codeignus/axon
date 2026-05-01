// Bridge crate generation and static archive builds for `@rust`/`.rs`/`rust_deps`.
//
// Thin sidecar only: callers in `.ax` own FFI policy inventory.

/// FFI: Produce a static archive for Rust sidecars. Placeholder returns error until ported.
#[axon_pub_export]
fn foreign_build_rust_bridge_archive(_project_root: &str, _deps_raw: &str, _sidecar_paths: &str) -> String {
    "error: rust bridge archive build not migrated yet — use migration driver until Phase 8".to_string()
}
