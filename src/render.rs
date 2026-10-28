use crate::lexer::Redirect;
use crate::parser::{Command, Pipeline, Redirection};

pub fn render(pipeline: &Pipeline) -> String {
    let stages: Vec<String> = pipeline.stages.iter().map(render_command).collect();
    stages.join(" | ")
}

fn render_command(command: &Command) -> String {
    let mut parts = vec![command.name.raw.clone()];
    parts.extend(command.args.iter().map(|arg| arg.raw.clone()));
    parts.extend(command.redirects.iter().map(render_redirection));
    parts.join(" ")
}

fn render_redirection(redirection: &Redirection) -> String {
    let op = match redirection.op {
        Redirect::Out { fd } => format!("{}>", render_fd(fd)),
        Redirect::Append { fd } => format!("{}>>", render_fd(fd)),
        Redirect::In => "<".to_string(),
        Redirect::Dup { fd, to } => format!("{fd}>&{to}"),
    };
    match &redirection.target {
        Some(target) => format!("{op} {}", target.raw),
        None => op,
    }
}

fn render_fd(fd: Option<u8>) -> String {
    fd.map(|fd| fd.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn parse_str(input: &str) -> Pipeline {
        parse(lex(input).expect("test input must lex")).expect("test input must parse")
    }

    #[test]
    fn renders_canonical_form() {
        let cases = [
            ("ls", "ls"),
            ("cat f | grep x", "cat f | grep x"),
            ("cat   f|grep\tx", "cat f | grep x"),
            ("grep x f>out", "grep x f > out"),
            ("> out grep x f", "grep x f > out"),
            ("grep x > out f", "grep x f > out"),
            ("wc -l <f", "wc -l < f"),
            ("ls >>log", "ls >> log"),
            ("ls 2>err", "ls 2> err"),
            ("ls 2>>err", "ls 2>> err"),
            ("ls 1>out", "ls 1> out"),
            ("ls 2>&1|wc -l", "ls 2>&1 | wc -l"),
            ("ls >f 2>&1", "ls > f 2>&1"),
            ("ls 2>&1 >f", "ls 2>&1 > f"),
            ("grep \"a b\" 'my file'", "grep \"a b\" 'my file'"),
            ("grep x\"y z\"'w' f", "grep x\"y z\"'w' f"),
            ("grep '' f", "grep '' f"),
            ("echo 2 >f", "echo 2 > f"),
            ("grep ü f", "grep ü f"),
        ];
        for (input, expected) in cases {
            assert_eq!(render(&parse_str(input)), expected, "input: {input:?}");
        }
    }

    #[test]
    fn round_trips_through_lexer_and_parser() {
        let inputs = [
            "ls",
            "cat f | sort | uniq -c",
            "> out grep x f",
            "grep x > out f",
            "ls 2>&1 > f",
            "ls > f 2>&1",
            "ls 2>>err >>out < in",
            "grep \"a b\" > 'my file'",
            "grep x\"y z\"'w' f",
            "grep '' \"\" f",
            "echo 2 > f",
            "grep a=b f",
            "grep '|' f",
            "grep \">\" f",
        ];
        for input in inputs {
            let pipeline = parse_str(input);
            let rendered = render(&pipeline);
            assert_eq!(parse_str(&rendered), pipeline, "input: {input:?}");
        }
    }
}
