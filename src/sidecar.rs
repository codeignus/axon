use clap::Command;
use std::sync::Once;
use tracing::info;

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().without_time().try_init();
    });
}

#[axon_export]
fn compiler_version() -> String {
    "0.1.0".to_string()
}

#[axon_export]
fn axon_fail(msg: &str) -> String {
    panic!("axon-lang: {msg}")
}

#[axon_export]
fn normalize_command_name(name: &str) -> String {
    let app = Command::new("axon")
        .subcommand(Command::new("check"))
        .subcommand(Command::new("test"))
        .subcommand(Command::new("fmt"))
        .subcommand(Command::new("mcp"));
    let matches = app
        .try_get_matches_from(["axon", name])
        .unwrap_or_else(|_| panic!("axon-lang: unknown command {name}"));
    matches.subcommand_name().unwrap().to_string()
}

#[axon_export]
fn axon_trace_info(msg: &str) -> String {
    init_tracing();
    info!("{msg}");
    msg.to_string()
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
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("axon-lang: read {path}: {e}"));
    let formatted = format_axon_source(&src);
    std::fs::write(path, formatted).unwrap_or_else(|e| panic!("axon-lang: write {path}: {e}"));
    path.to_string()
}
