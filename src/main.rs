mod lexer;

use std::env;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use lexer::{LexError, Redirect, Token};

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
    let tokens = lexer::lex(command);
    if env::var_os("TIPUN_DEBUG").is_some_and(|v| v == "1") {
        print_tokens(&tokens);
    }
    ExitCode::from(1)
}

fn print_tokens(tokens: &Result<Vec<Token>, LexError>) {
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("lex error: {err:?}");
            return;
        }
    };
    for token in tokens {
        match token {
            Token::Word(word) => eprintln!("word raw={:?} value={:?}", word.raw, word.value),
            Token::Pipe => eprintln!("pipe"),
            Token::Redirect(Redirect::Out { fd }) => eprintln!("redirect out fd={fd:?}"),
            Token::Redirect(Redirect::Append { fd }) => eprintln!("redirect append fd={fd:?}"),
            Token::Redirect(Redirect::In) => eprintln!("redirect in"),
            Token::Redirect(Redirect::Dup { fd, to }) => eprintln!("redirect dup {fd}>&{to}"),
        }
    }
}
