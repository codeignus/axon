use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "axon")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check { target: Option<String> },
    Build { target: Option<String> },
    Run { target: Option<String> },
    Test { target: Option<String> },
    Fmt { target: Option<String> },
    Mcp,
}

pub struct CliResult {
    command: String,
    target: Option<String>,
}

impl CliResult {
    pub fn command(&self) -> String {
        self.command.clone()
    }

    pub fn target(&self) -> String {
        self.target.clone().unwrap_or_default()
    }
}

fn parse_cli_args() -> CliResult {
    let cli = Cli::parse();
    let (command, target) = match cli.command {
        Command::Check { target } => ("check".to_string(), target),
        Command::Build { target } => ("build".to_string(), target),
        Command::Run { target } => ("run".to_string(), target),
        Command::Test { target } => ("test".to_string(), target),
        Command::Fmt { target } => ("fmt".to_string(), target),
        Command::Mcp => ("mcp".to_string(), None),
    };
    CliResult { command, target }
}

#[no_mangle]
pub extern "C" fn parse_cli() -> *mut u8 {
    Box::into_raw(Box::new(parse_cli_args())) as *mut u8
}

#[no_mangle]
pub extern "C" fn CliResult__drop(handle: *mut u8) {
    unsafe {
        drop(Box::from_raw(handle as *mut CliResult));
    }
}
