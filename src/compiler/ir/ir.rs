// IR lowering module.
//
// The bootstrap compiler owns the actual MIR lowering
// pipeline, including method lowering (e.g., `.len()` -> `string_len`). This module
// is a structural marker that collects modules for the native build step.

#[axon_export]
fn lower_module(source: &str) -> String {
    format!("ir:module:bytes={}", source.len())
}

#[axon_export]
fn lower_function(name: &str) -> String {
    format!("ir:function:{name}")
}

// `lower_project` is implemented in `ir/lower_project.ax` (Axon-owned lowering + write).

// MIR string encoding helpers. String concatenation via `+` is not yet
// supported by the LLVM codegen for struct-typed values, so these Rust-side
// helpers perform the encoding instead.

#[axon_pub_export]
fn mir_encode2(prefix: &str, a: &str) -> String {
    format!("{prefix}{a}")
}

#[axon_pub_export]
fn mir_encode3(prefix: &str, a: &str, b: &str) -> String {
    format!("{prefix}{a}{b}")
}

#[axon_pub_export]
fn mir_encode4(prefix: &str, a: &str, b: &str, c: &str) -> String {
    format!("{prefix}{a}{b}{c}")
}

#[axon_pub_export]
fn mir_encode5(prefix: &str, a: &str, b: &str, c: &str, d: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}")
}

#[axon_pub_export]
fn mir_encode6(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}")
}

#[axon_pub_export]
fn mir_encode7(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}")
}

#[axon_pub_export]
fn mir_encode8(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str, g: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}{g}")
}

#[axon_pub_export]
fn mir_encode9(prefix: &str, a: &str, b: &str, c: &str, d: &str, e: &str, f: &str, g: &str, h: &str) -> String {
    format!("{prefix}{a}{b}{c}{d}{e}{f}{g}{h}")
}

#[axon_pub_export]
fn mir_colon_parts2(a: &str, b: &str) -> String {
    format!("{a}:{b}")
}

#[axon_pub_export]
fn mir_colon_parts3(a: &str, b: &str, c: &str) -> String {
    format!("{a}:{b}:{c}")
}

#[axon_pub_export]
fn mir_colon_parts4(a: &str, b: &str, c: &str, d: &str) -> String {
    format!("{a}:{b}:{c}:{d}")
}
