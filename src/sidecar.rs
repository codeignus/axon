#[axon_export]
fn axon_fail(msg: &str) -> String {
    panic!("axon-lang: {msg}")
}

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

#[axon_export]
fn format_source_for_test(input: &str) -> String {
    format_axon_source(input)
}

#[axon_export]
fn format_axon_file(path: &str) -> String {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("axon-lang: read {path}: {e}"));
    let formatted = format_axon_source(&src);
    std::fs::write(path, formatted).unwrap_or_else(|e| panic!("axon-lang: write {path}: {e}"));
    path.to_string()
}
