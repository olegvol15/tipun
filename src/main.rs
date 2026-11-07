mod args;
mod lexer;
mod parser;
mod render;
mod rules;

use std::env;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use lexer::{Redirect, Token, Word};
use parser::Pipeline;
use rules::Rewrite;

#[derive(Parser)]
#[command(name = "tipun", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Suggest {
        command: String,
    },
    Analyze {
        #[arg(long, allow_hyphen_values = true)]
        command: String,
        #[arg(long, allow_hyphen_values = true)]
        expanded: Option<String>,
        #[arg(long, default_value_t = 0)]
        exit_code: i32,
    },
    Init {
        shell: Shell,
    },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Zsh,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) if env::args().nth(1).as_deref() == Some("analyze") => {
            if debug() {
                eprintln!("{err}");
            }
            return ExitCode::SUCCESS;
        }
        Err(err) => err.exit(),
    };

    match cli.command {
        Command::Suggest { command } => suggest(&command),
        Command::Analyze {
            command,
            expanded,
            exit_code,
        } => {
            analyze(&command, expanded.as_deref(), exit_code);
            ExitCode::SUCCESS
        }
        Command::Init { shell } => {
            match shell {
                Shell::Zsh => print!("{}", include_str!("init.zsh")),
            }
            ExitCode::SUCCESS
        }
    }
}

fn suggest(command: &str) -> ExitCode {
    match find_rewrite(command) {
        Some(rewrite) => {
            println!("{}", render::render(&rewrite.pipeline));
            ExitCode::SUCCESS
        }
        None => ExitCode::from(1),
    }
}

fn analyze(command: &str, expanded: Option<&str>, exit_code: i32) {
    if exit_code != 0 || env::var_os("CI").is_some() || env::var_os("TIPUN_DISABLE").is_some() {
        return;
    }
    let Some(rewrite) = find_rewrite(command) else {
        return;
    };
    if let Some(expanded) = expanded
        && !same_words(command, expanded)
    {
        if debug() {
            eprintln!("expanded command differs: {expanded:?}");
        }
        return;
    }
    eprintln!("tipun tip: {}", render::render(&rewrite.pipeline));
    eprintln!("  {}", rewrite.explanation);
}

fn same_words(command: &str, expanded: &str) -> bool {
    let (Ok(typed), Ok(expanded)) = (lexer::lex(command), lexer::lex(expanded)) else {
        return false;
    };
    typed.len() == expanded.len()
        && typed.iter().zip(&expanded).all(|pair| match pair {
            (Token::Word(a), Token::Word(b)) => a.value == b.value,
            (a, b) => a == b,
        })
}

fn find_rewrite(command: &str) -> Option<Rewrite> {
    let tokens = match lexer::lex(command) {
        Ok(tokens) => tokens,
        Err(err) => {
            if debug() {
                eprintln!("lex error: {err:?}");
            }
            return None;
        }
    };
    let pipeline = match parser::parse(tokens) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            if debug() {
                eprintln!("parse error: {err:?}");
            }
            return None;
        }
    };
    if debug() {
        print_pipeline(&pipeline);
    }
    rules::rewrite(&pipeline)
}

fn debug() -> bool {
    env::var_os("TIPUN_DEBUG").is_some_and(|v| v == "1")
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
    eprintln!("rendered {}", render::render(pipeline));
}

fn describe(word: &Word) -> String {
    format!("raw={:?} value={:?}", word.raw, word.value)
}
