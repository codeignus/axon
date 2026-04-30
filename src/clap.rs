use clap::Parser;

#[derive(Parser)]
#[command(name = "axon")]
#[command(version = "0.1.0")]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Check { target: Option<String> },
    Build,
    Run,
    Test { target: Option<String> },
    Fmt { target: Option<String> },
    Mcp,
}

// TODO(self-host ownership/typecheck milestone):
// Collapse `cli_command` + `cli_target` into one export that returns
// a tuple shaped like Axon `(String, Option[String])`.
//
// Why deferred:
// - This requires complete Option/tuple typing in the Axon compiler pipeline.
// - The future self-hosted typechecker should validate real struct/Option types
//   instead of relying on parser-level name heuristics.
// - Once that is stable, this split API can be replaced by a single typed return.

/// FFI: Parses CLI args and returns the subcommand name as a string.
/// One of: "check", "build", "run", "test", "fmt", "mcp".
#[axon_export]
fn cli_command() -> String {
    let args = Cli::parse();
    match args.command {
        Command::Check { .. } => "check".to_string(),
        Command::Build => "build".to_string(),
        Command::Run => "run".to_string(),
        Command::Test { .. } => "test".to_string(),
        Command::Fmt { .. } => "fmt".to_string(),
        Command::Mcp => "mcp".to_string(),
    }
}

/// FFI: Parses CLI args and returns the optional target argument.
/// Returns empty string if the subcommand takes no target or none was provided.
#[axon_export]
fn cli_target() -> String {
    let args = Cli::parse();
    match args.command {
        Command::Check { target } => target.unwrap_or_default(),
        Command::Test { target } => target.unwrap_or_default(),
        Command::Fmt { target } => target.unwrap_or_default(),
        Command::Build | Command::Run | Command::Mcp => "".to_string(),
    }
}
