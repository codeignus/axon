use clap::Parser;

#[derive(Parser)]
#[command(name = "axon")]
#[command(version = "0.1.0")]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

enum Command {
    Check { target: String },
    Build,
    Run,
    Test { target: String },
    Fmt { target: String },
    Mcp,
}

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

#[axon_export]
fn cli_target() -> String {
    let args = Cli::parse();
    match args.command {
        Command::Check { target } => target,
        Command::Test { target } => target,
        Command::Fmt { target } => target,
        Command::Build | Command::Run | Command::Mcp => {
            panic!("axon-lang: target requested for command without target")
        }
    }
}
