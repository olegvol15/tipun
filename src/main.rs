mod lexer;
mod parser;

use std::env;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use lexer::{Redirect, Word};
use parser::Pipeline;

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
    let debug = env::var_os("TIPUN_DEBUG").is_some_and(|v| v == "1");
    let tokens = match lexer::lex(command) {
        Ok(tokens) => tokens,
        Err(err) => {
            if debug {
                eprintln!("lex error: {err:?}");
            }
            return ExitCode::from(1);
        }
    };
    match parser::parse(tokens) {
        Ok(pipeline) if debug => print_pipeline(&pipeline),
        Err(err) if debug => eprintln!("parse error: {err:?}"),
        Ok(_) | Err(_) => {}
    }
    ExitCode::from(1)
}

fn print_pipeline(pipeline: &Pipeline) {
    for (i, stage) in pipeline.stages.iter().enumerate() {
        eprintln!("stage {}", i + 1);
        eprintln!("  name {}", describe(&stage.name));
        for arg in &stage.args {
            eprintln!("  arg {}", describe(arg));
        }
        for redirect in &stage.redirects {
            let op = match redirect.op {
                Redirect::Out { fd } => format!("out fd={fd:?}"),
                Redirect::Append { fd } => format!("append fd={fd:?}"),
                Redirect::In => "in".to_string(),
                Redirect::Dup { fd, to } => format!("dup {fd}>&{to}"),
            };
            match &redirect.target {
                Some(target) => eprintln!("  redirect {op} target {}", describe(target)),
                None => eprintln!("  redirect {op}"),
            }
        }
    }
}

fn describe(word: &Word) -> String {
    format!("raw={:?} value={:?}", word.raw, word.value)
}
