use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tipun", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Suggest { command: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Suggest { command } => suggest(&command),
    }
}

fn suggest(command: &str) -> ExitCode {
    let _ = command;
    ExitCode::from(1)
}
