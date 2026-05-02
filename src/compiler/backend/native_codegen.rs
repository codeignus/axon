// LLVM MIR → object emission. Core lowering is bundled in native_codegen_bundle.inc.rs.
//
// Regenerate bundle when the optional migration mirror crates change:
//   python3 scripts/gen-native-codegen-bundle.py

#[allow(non_camel_case_types)]
#[allow(clippy::all)]
#[allow(unused)]
mod native_codegen_bundle {
    include!("native_codegen_bundle.inc.rs");
}
pub use native_codegen_bundle::*;

#[derive(Debug, serde::Deserialize)]
struct NativeCodegenRequest {
    module: MirModule,
    #[serde(default)]
    axon_structs: std::collections::HashMap<String, AxonStructInfo>,
    #[serde(default)]
    optimization: String,
    #[serde(default)]
    has_go_deps: bool,
}

/// FFI: Check if LLVM codegen is available.
#[axon_pub_export]
fn native_codegen_available() -> String {
    "ok:inkwell-linked".to_string()
}

/// Deserialize JSON codegen request; emit ELF object bytes for one MIR module.
/// Request JSON: `{ "module": {...}, "axon_structs": {...}, "optimization": "debug"|"aggressive", "has_go_deps": false }`
/// Success: `{ "module_name": "...", "object_hex": "..." }`.
#[axon_pub_export]
fn native_emit_object_for_module(payload_json: &str) -> String {
    match serde_json::from_str::<NativeCodegenRequest>(payload_json) {
        Err(e) => format!("error: invalid codegen JSON: {e}"),
        Ok(req) => {
            let level = match req.optimization.as_str() {
                "aggressive" => OptimizationLevel::Aggressive,
                _ => OptimizationLevel::Debug,
            };
            match codegen_module(&req.module, &req.axon_structs, level, req.has_go_deps) {
                Ok(out) => {
                    let object_hex: String =
                        out.object_data.iter().map(|b| format!("{b:02x}")).collect();
                    serde_json::json!({
                        "module_name": out.module_name,
                        "object_hex": object_hex,
                    })
                    .to_string()
                }
                Err(msg) => format!("error: {msg}"),
            }
        }
    }
}
