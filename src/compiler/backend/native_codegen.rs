// LLVM / object emission boundary for Axon-produced MIR payloads.
//
// Phase 8 replaces the temporary `axon-native-build` driver via FFI (JSON MIR → object path bytes).

/// FFI: Serialized MIR/backend request placeholder. Real implementation emits one `.o`
/// via inkwell/llvm-sys and returns `"ok:<path>` or `"error: ..."``.
#[axon_pub_export]
fn native_emit_object_for_module(_payload_json: &str) -> String {
    "error: native codegen not migrated yet — use migration driver until Phase 8".to_string()
}
