/// FFI: Panic handler for the Axon runtime. Causes a Rust panic with the
/// given message prefixed by `axon-lang:`. Does not return.
#[axon_export]
fn axon_fail(msg: &str) -> String {
    panic!("axon-lang: {msg}")
}

/// Normalizes whitespace in Axon source: trims trailing whitespace per line
/// and converts leading spaces to tabs (2 spaces = 1 tab).
/// This is a formatting utility, not compiler logic.
fn format_axon_source(input: &str) -> String {
    let mut out = String::new();
    for raw_line in input.lines() {
        let trimmed_end = raw_line.trim_end();
        let mut width = 0usize;
        let mut content_start = 0usize;
        for (idx, ch) in trimmed_end.char_indices() {
            if ch == ' ' {
                width += 1;
                content_start = idx + 1;
            } else if ch == '\t' {
                width += 2;
                content_start = idx + 1;
            } else {
                break;
            }
        }
        let tabs = width / 2;
        out.push_str(&"\t".repeat(tabs));
        out.push_str(&trimmed_end[content_start..]);
        out.push('\n');
    }
    if out.is_empty() {
        out.push('\n');
    }
    out
}

/// FFI: Formats an Axon source string in-memory. Used by tests.
/// Params: `input` — raw source text. Returns the reformatted source.
#[axon_export]
fn format_source_for_test(input: &str) -> String {
    format_axon_source(input)
}

/// FFI: Reads an Axon file, reformats it in-place (whitespace normalization),
/// and writes it back. Returns the file path on success, panics on I/O error.
#[axon_export]
fn format_axon_file(path: &str) -> String {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("axon-lang: read {path}: {e}"));
    let formatted = format_axon_source(&src);
    std::fs::write(path, formatted).unwrap_or_else(|e| panic!("axon-lang: write {path}: {e}"));
    path.to_string()
}
