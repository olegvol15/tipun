use std::iter::Peekable;
use std::str::CharIndices;

const MAX_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub raw: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirect {
    Out { fd: Option<u8> },
    Append { fd: Option<u8> },
    In,
    Dup { fd: u8, to: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
    Pipe,
    Redirect(Redirect),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    TooLong,
    Unsupported,
    UnterminatedQuote,
}

type Chars<'a> = Peekable<CharIndices<'a>>;

pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    if input.len() > MAX_LEN {
        return Err(LexError::TooLong);
    }
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        match c {
            ' ' | '\t' => {
                chars.next();
            }
            '|' => {
                chars.next();
                if next_is(&mut chars, '|') {
                    return Err(LexError::Unsupported);
                }
                tokens.push(Token::Pipe);
            }
            '>' | '<' => tokens.push(Token::Redirect(lex_redirect(&mut chars, None)?)),
            '0'..='9' if input.as_bytes().get(start + 1) == Some(&b'>') => {
                chars.next();
                let fd = c as u8 - b'0';
                tokens.push(Token::Redirect(lex_redirect(&mut chars, Some(fd))?));
            }
            _ => {
                let word = lex_word(&mut chars)?;
                let all_digits = word.raw.bytes().all(|b| b.is_ascii_digit());
                if all_digits && (next_is(&mut chars, '<') || next_is(&mut chars, '>')) {
                    return Err(LexError::Unsupported);
                }
                tokens.push(Token::Word(word));
            }
        }
    }
    Ok(tokens)
}

fn next_is(chars: &mut Chars, expected: char) -> bool {
    chars.peek().is_some_and(|&(_, c)| c == expected)
}

fn lex_redirect(chars: &mut Chars, fd: Option<u8>) -> Result<Redirect, LexError> {
    match chars.next() {
        Some((_, '<')) => {
            if next_is(chars, '<') || next_is(chars, '>') || next_is(chars, '&') {
                return Err(LexError::Unsupported);
            }
            Ok(Redirect::In)
        }
        Some((_, '>')) => {
            if next_is(chars, '>') {
                chars.next();
                if next_is(chars, '&') || next_is(chars, '|') || next_is(chars, '>') {
                    return Err(LexError::Unsupported);
                }
                return Ok(Redirect::Append { fd });
            }
            if next_is(chars, '&') {
                chars.next();
                return lex_dup(chars, fd);
            }
            if next_is(chars, '|') {
                return Err(LexError::Unsupported);
            }
            Ok(Redirect::Out { fd })
        }
        _ => Err(LexError::Unsupported),
    }
}

fn lex_dup(chars: &mut Chars, fd: Option<u8>) -> Result<Redirect, LexError> {
    let Some(fd) = fd else {
        return Err(LexError::Unsupported);
    };
    let to = match chars.next() {
        Some((_, c @ '0'..='9')) => c as u8 - b'0',
        _ => return Err(LexError::Unsupported),
    };
    match chars.peek() {
        None | Some((_, ' ' | '\t' | '|')) => Ok(Redirect::Dup { fd, to }),
        Some(_) => Err(LexError::Unsupported),
    }
}

fn lex_word(chars: &mut Chars) -> Result<Word, LexError> {
    let mut raw = String::new();
    let mut value = String::new();
    while let Some(&(_, c)) = chars.peek() {
        match c {
            ' ' | '\t' | '|' | '<' | '>' => break,
            '\'' | '"' => {
                chars.next();
                raw.push(c);
                lex_quoted(chars, c, &mut raw, &mut value)?;
            }
            '=' if raw.is_empty() => return Err(LexError::Unsupported),
            _ if is_word_char(c) => {
                chars.next();
                raw.push(c);
                value.push(c);
            }
            _ => return Err(LexError::Unsupported),
        }
    }
    Ok(Word { raw, value })
}

fn lex_quoted(
    chars: &mut Chars,
    quote: char,
    raw: &mut String,
    value: &mut String,
) -> Result<(), LexError> {
    loop {
        match chars.next() {
            None => return Err(LexError::UnterminatedQuote),
            Some((_, c)) if c == quote => {
                raw.push(c);
                return Ok(());
            }
            Some((_, '\n')) => return Err(LexError::Unsupported),
            Some((_, '$' | '`' | '\\' | '!')) if quote == '"' => {
                return Err(LexError::Unsupported);
            }
            Some((_, c)) => {
                raw.push(c);
                value.push(c);
            }
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-_./:,=+@%".contains(c) || !c.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(raw: &str, value: &str) -> Token {
        Token::Word(Word {
            raw: raw.to_string(),
            value: value.to_string(),
        })
    }

    fn plain(s: &str) -> Token {
        w(s, s)
    }

    fn r(redirect: Redirect) -> Token {
        Token::Redirect(redirect)
    }

    #[test]
    fn accepts_supported_input() {
        let cases = vec![
            ("", vec![]),
            ("   \t ", vec![]),
            ("ls", vec![plain("ls")]),
            (
                "cat app.log | grep ERROR",
                vec![
                    plain("cat"),
                    plain("app.log"),
                    Token::Pipe,
                    plain("grep"),
                    plain("ERROR"),
                ],
            ),
            ("a|b", vec![plain("a"), Token::Pipe, plain("b")]),
            ("grep\t-n\tx", vec![plain("grep"), plain("-n"), plain("x")]),
            ("grep 'a b'", vec![plain("grep"), w("'a b'", "a b")]),
            ("grep \"a b\"", vec![plain("grep"), w("\"a b\"", "a b")]),
            ("grep ''", vec![plain("grep"), w("''", "")]),
            (
                "echo a\"b c\"'d'",
                vec![plain("echo"), w("a\"b c\"'d'", "ab cd")],
            ),
            (
                "grep '$HOME *'",
                vec![plain("grep"), w("'$HOME *'", "$HOME *")],
            ),
            ("grep \"it's\"", vec![plain("grep"), w("\"it's\"", "it's")]),
            (
                "grep 'a|b' f",
                vec![plain("grep"), w("'a|b'", "a|b"), plain("f")],
            ),
            ("grep a=b x", vec![plain("grep"), plain("a=b"), plain("x")]),
            ("grep ''=x", vec![plain("grep"), w("''=x", "=x")]),
            (
                "cp -r ./src/a.rs /tmp/b:c,d+e@f%g",
                vec![
                    plain("cp"),
                    plain("-r"),
                    plain("./src/a.rs"),
                    plain("/tmp/b:c,d+e@f%g"),
                ],
            ),
            (
                "grep café файл",
                vec![plain("grep"), plain("café"), plain("файл")],
            ),
            ("grep 🦀 f", vec![plain("grep"), plain("🦀"), plain("f")]),
            (
                "ls > f",
                vec![plain("ls"), r(Redirect::Out { fd: None }), plain("f")],
            ),
            (
                "ls >f",
                vec![plain("ls"), r(Redirect::Out { fd: None }), plain("f")],
            ),
            (
                "ls >> f",
                vec![plain("ls"), r(Redirect::Append { fd: None }), plain("f")],
            ),
            (
                "wc -l < f",
                vec![plain("wc"), plain("-l"), r(Redirect::In), plain("f")],
            ),
            (
                "ls 2> f",
                vec![plain("ls"), r(Redirect::Out { fd: Some(2) }), plain("f")],
            ),
            (
                "ls 2>> f",
                vec![plain("ls"), r(Redirect::Append { fd: Some(2) }), plain("f")],
            ),
            (
                "ls 2>&1",
                vec![plain("ls"), r(Redirect::Dup { fd: 2, to: 1 })],
            ),
            (
                "ls 2>&1 | wc",
                vec![
                    plain("ls"),
                    r(Redirect::Dup { fd: 2, to: 1 }),
                    Token::Pipe,
                    plain("wc"),
                ],
            ),
            (
                "ls a2>f",
                vec![
                    plain("ls"),
                    plain("a2"),
                    r(Redirect::Out { fd: None }),
                    plain("f"),
                ],
            ),
            (
                "ls \"2\">f",
                vec![
                    plain("ls"),
                    w("\"2\"", "2"),
                    r(Redirect::Out { fd: None }),
                    plain("f"),
                ],
            ),
            (
                "head -n 2 f",
                vec![plain("head"), plain("-n"), plain("2"), plain("f")],
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(lex(input), Ok(expected), "input: {input:?}");
        }
    }

    #[test]
    fn rejects_unsupported_input() {
        use LexError::{TooLong, Unsupported, UnterminatedQuote};
        let long = "a".repeat(MAX_LEN + 1);
        let cases = vec![
            (long.as_str(), TooLong),
            ("a && b", Unsupported),
            ("a || b", Unsupported),
            ("echo =ls", Unsupported),
            ("echo \"$HOME\"", Unsupported),
            ("echo \"`date`\"", Unsupported),
            ("echo \"a\\b\"", Unsupported),
            ("echo \"hi!\"", Unsupported),
            ("echo 'a\nb'", Unsupported),
            ("echo 'abc", UnterminatedQuote),
            ("echo \"abc", UnterminatedQuote),
            ("ls >& f", Unsupported),
            ("ls 2>&-", Unsupported),
            ("ls 2>&f", Unsupported),
            ("ls 2>&12", Unsupported),
            ("ls &> f", Unsupported),
            ("ls >| f", Unsupported),
            ("ls >>| f", Unsupported),
            ("ls >>& f", Unsupported),
            ("cat << EOF", Unsupported),
            ("cat <> f", Unsupported),
            ("cat <& 3", Unsupported),
            ("cat 0< f", Unsupported),
            ("ls 12> f", Unsupported),
        ];
        for (input, expected) in cases {
            assert_eq!(lex(input), Err(expected), "input: {input:?}");
        }
    }

    #[test]
    fn rejects_unsupported_characters() {
        let chars = [
            ';', '&', '(', ')', '{', '}', '[', ']', '*', '?', '~', '#', '!', '$', '\\', '^', '`',
            '\n',
        ];
        for c in chars {
            for input in [format!("ls {c}b"), format!("ls a{c}b")] {
                assert_eq!(lex(&input), Err(LexError::Unsupported), "input: {input:?}");
            }
        }
    }

    #[test]
    fn accepts_input_at_length_limit() {
        let input = "a".repeat(MAX_LEN);
        assert_eq!(lex(&input), Ok(vec![plain(&input)]));
    }
}
