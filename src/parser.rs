use std::mem;

use crate::lexer::{Redirect, Token, Word};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub stages: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub name: Word,
    pub args: Vec<Word>,
    pub redirects: Vec<Redirection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    pub op: Redirect,
    pub target: Option<Word>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    EmptyStage,
    MissingTarget,
    AssignmentPrefix,
}

pub fn parse(tokens: Vec<Token>) -> Result<Pipeline, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut stages = Vec::new();
    let mut words = Vec::new();
    let mut redirects = Vec::new();
    let mut tokens = tokens.into_iter();
    while let Some(token) = tokens.next() {
        match token {
            Token::Word(word) => words.push(word),
            Token::Pipe => {
                let stage = finish_stage(mem::take(&mut words), mem::take(&mut redirects))?;
                stages.push(stage);
            }
            Token::Redirect(op) => {
                let target = match op {
                    Redirect::Dup { .. } => None,
                    Redirect::Out { .. } | Redirect::Append { .. } | Redirect::In => {
                        match tokens.next() {
                            Some(Token::Word(word)) => Some(word),
                            _ => return Err(ParseError::MissingTarget),
                        }
                    }
                };
                redirects.push(Redirection { op, target });
            }
        }
    }
    stages.push(finish_stage(words, redirects)?);
    Ok(Pipeline { stages })
}

fn finish_stage(words: Vec<Word>, redirects: Vec<Redirection>) -> Result<Command, ParseError> {
    let mut words = words.into_iter();
    let name = words.next().ok_or(ParseError::EmptyStage)?;
    if name.raw.contains('=') {
        return Err(ParseError::AssignmentPrefix);
    }
    Ok(Command {
        name,
        args: words.collect(),
        redirects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_str(input: &str) -> Result<Pipeline, ParseError> {
        parse(lex(input).expect("test input must lex"))
    }

    fn w(raw: &str, value: &str) -> Word {
        Word {
            raw: raw.to_string(),
            value: value.to_string(),
        }
    }

    fn plain(s: &str) -> Word {
        w(s, s)
    }

    fn cmd(name: &str, args: &[&str], redirects: Vec<Redirection>) -> Command {
        Command {
            name: plain(name),
            args: args.iter().map(|a| plain(a)).collect(),
            redirects,
        }
    }

    fn to(op: Redirect, target: &str) -> Redirection {
        Redirection {
            op,
            target: Some(plain(target)),
        }
    }

    fn dup(fd: u8, to: u8) -> Redirection {
        Redirection {
            op: Redirect::Dup { fd, to },
            target: None,
        }
    }

    const OUT: Redirect = Redirect::Out { fd: None };

    #[test]
    fn parses_supported_input() {
        let cases = vec![
            ("ls", vec![cmd("ls", &[], vec![])]),
            (
                "cat f | grep x",
                vec![cmd("cat", &["f"], vec![]), cmd("grep", &["x"], vec![])],
            ),
            (
                "cat f | sort | uniq -c",
                vec![
                    cmd("cat", &["f"], vec![]),
                    cmd("sort", &[], vec![]),
                    cmd("uniq", &["-c"], vec![]),
                ],
            ),
            (
                "grep x f > out",
                vec![cmd("grep", &["x", "f"], vec![to(OUT, "out")])],
            ),
            (
                "> out grep x f",
                vec![cmd("grep", &["x", "f"], vec![to(OUT, "out")])],
            ),
            (
                "grep x > out f",
                vec![cmd("grep", &["x", "f"], vec![to(OUT, "out")])],
            ),
            (
                "wc -l < f",
                vec![cmd("wc", &["-l"], vec![to(Redirect::In, "f")])],
            ),
            (
                "ls >> log",
                vec![cmd(
                    "ls",
                    &[],
                    vec![to(Redirect::Append { fd: None }, "log")],
                )],
            ),
            (
                "ls 2>&1 | wc -l",
                vec![cmd("ls", &[], vec![dup(2, 1)]), cmd("wc", &["-l"], vec![])],
            ),
            (
                "ls > f 2>&1",
                vec![cmd("ls", &[], vec![to(OUT, "f"), dup(2, 1)])],
            ),
            (
                "ls 2>&1 > f",
                vec![cmd("ls", &[], vec![dup(2, 1), to(OUT, "f")])],
            ),
            ("grep a=b f", vec![cmd("grep", &["a=b", "f"], vec![])]),
        ];
        for (input, stages) in cases {
            assert_eq!(
                parse_str(input),
                Ok(Pipeline { stages }),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn keeps_quoted_words() {
        let expected = Pipeline {
            stages: vec![Command {
                name: plain("grep"),
                args: vec![w("\"a b\"", "a b")],
                redirects: vec![Redirection {
                    op: OUT,
                    target: Some(w("'my file'", "my file")),
                }],
            }],
        };
        assert_eq!(parse_str("grep \"a b\" > 'my file'"), Ok(expected));
    }

    #[test]
    fn rejects_unsupported_input() {
        use ParseError::{AssignmentPrefix, Empty, EmptyStage, MissingTarget};
        let cases = vec![
            ("", Empty),
            ("   ", Empty),
            ("| a", EmptyStage),
            ("a |", EmptyStage),
            ("a | | b", EmptyStage),
            ("> f", EmptyStage),
            ("> f | cat", EmptyStage),
            ("ls >", MissingTarget),
            ("ls > | wc", MissingTarget),
            ("ls > > f", MissingTarget),
            ("ls > 2>&1", MissingTarget),
            ("cat <", MissingTarget),
            ("FOO=1 cat f", AssignmentPrefix),
            ("cat f | LC_ALL=C sort", AssignmentPrefix),
            ("> out FOO=1 cat f", AssignmentPrefix),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_str(input), Err(expected), "input: {input:?}");
        }
    }
}
