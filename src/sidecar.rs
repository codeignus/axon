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
