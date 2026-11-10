mod args;
mod expanded;
mod lexer;
mod parser;
mod render;
mod rules;
mod tip;
mod update;

use std::env;
use std::io::{self, IsTerminal};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use lexer::{Redirect, Word};
use parser::Chain;
use rules::Suggestion;

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
        #[arg(long)]
        columns: Option<String>,
    },
    Init {
        shell: Shell,
    },
    Update {
        #[arg(long)]
        quiet: bool,
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
            columns,
        } => {
            analyze(&command, expanded.as_deref(), exit_code, columns.as_deref());
            ExitCode::SUCCESS
        }
        Command::Init { shell } => {
            match shell {
                Shell::Zsh => print!("{}", include_str!("init.zsh")),
            }
            ExitCode::SUCCESS
        }
        Command::Update { quiet } => update(quiet),
    }
}

fn update(quiet: bool) -> ExitCode {
    let message = match update::run(quiet) {
        Ok(update::Outcome::UpToDate) => {
            format!("tipun {} is up to date", env!("CARGO_PKG_VERSION"))
        }
        Ok(update::Outcome::Updated(tag)) => format!("tipun updated to {tag}"),
        Err(update::UpdateError::NotInstalledByScript) => {
            if !quiet {
                eprintln!(
                    "tipun was not installed by install.sh; update it the way you installed it"
                );
            }
            return ExitCode::from(1);
        }
        Err(err) => {
            if !quiet {
                eprintln!("tipun update failed: {err}");
            }
            return ExitCode::from(1);
        }
    };
    if !quiet {
        println!("{message}");
    }
    ExitCode::SUCCESS
}

fn suggest(command: &str) -> ExitCode {
    match find_rewrite(command) {
        Some(rewrite) => {
            println!("{}", render::render(&rewrite.chain));
            ExitCode::SUCCESS
        }
        None => ExitCode::from(1),
    }
}

fn analyze(command: &str, expanded: Option<&str>, exit_code: i32, columns: Option<&str>) {
    if env::var_os("CI").is_some() || env::var_os("TIPUN_DISABLE").is_some() {
        return;
    }
    if let Err(err) = update::in_background()
        && debug()
    {
        eprintln!("update check failed: {err}");
    }
    if exit_code != 0 {
        return;
    }
    let Some(rewrite) = find_rewrite(command) else {
        return;
    };
    if let Some(expanded) = expanded
        && !expanded::matches_typed(command, expanded)
    {
        if debug() {
            eprintln!("expanded command differs: {expanded:?}");
        }
        return;
    }
    let suggestion = render::render(&rewrite.chain);
    eprint!(
        "{}",
        tip::render(&suggestion, &rewrite.explanation, terminal(columns))
    );
}

fn terminal(columns: Option<&str>) -> Option<tip::Terminal> {
    if !io::stderr().is_terminal() {
        return None;
    }
    let color = env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
        && env::var_os("TERM").is_none_or(|term| term != "dumb");
    Some(tip::Terminal {
        columns: columns.and_then(|columns| columns.parse().ok()),
        color,
    })
}

fn find_rewrite(command: &str) -> Option<Suggestion> {
    let tokens = match lexer::lex(command) {
        Ok(tokens) => tokens,
        Err(err) => {
            if debug() {
                eprintln!("lex error: {err:?}");
            }
            return None;
        }
    };
    let chain = match parser::parse(tokens) {
        Ok(chain) => chain,
        Err(err) => {
            if debug() {
                eprintln!("parse error: {err:?}");
            }
            return None;
        }
    };
    if debug() {
        print_chain(&chain);
    }
    rules::rewrite(&chain)
}

fn debug() -> bool {
    env::var_os("TIPUN_DEBUG").is_some_and(|v| v == "1")
}

fn print_chain(chain: &Chain) {
    for (p, pipeline) in chain.pipelines.iter().enumerate() {
        for (i, stage) in pipeline.stages.iter().enumerate() {
            eprintln!("pipeline {} stage {}", p + 1, i + 1);
            print_command(stage);
        }
    }
    eprintln!("rendered {}", render::render(chain));
}

fn print_command(stage: &parser::Command) {
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

fn describe(word: &Word) -> String {
    format!("raw={:?} value={:?}", word.raw, word.value)
}
