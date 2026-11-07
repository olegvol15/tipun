use crate::lexer::{self, Token};

pub fn matches_typed(typed: &str, expanded: &str) -> bool {
    let expanded = drop_harmless_grep_options(expanded);
    let (Ok(typed), Ok(expanded)) = (lexer::lex(typed), lexer::lex(&expanded)) else {
        return false;
    };
    typed.len() == expanded.len()
        && typed.iter().zip(&expanded).all(|pair| match pair {
            (Token::Word(a), Token::Word(b)) => a.value == b.value,
            (a, b) => a == b,
        })
}

// zsh reports the expanded command with its words separated by spaces, so splitting on spaces
// finds options added by an alias such as oh-my-zsh's `grep --color=auto --exclude-dir={...}`
// without lexing the brace expansion. Quoted text is never alias-expanded, so a quoted chunk
// that happens to look like an option is identical in the typed command, and dropping it can
// only make the comparison fail.
fn drop_harmless_grep_options(expanded: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut at_command = true;
    let mut after_grep = false;
    for chunk in expanded.split(' ') {
        if after_grep && is_harmless_grep_option(chunk) {
            continue;
        }
        after_grep = at_command && chunk == "grep";
        at_command = chunk == "|";
        kept.push(chunk);
    }
    kept.join(" ")
}

fn is_harmless_grep_option(chunk: &str) -> bool {
    match chunk.strip_prefix("--exclude-dir=") {
        Some(dirs) => {
            !dirs.is_empty()
                && dirs
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-,{}*".contains(c))
        }
        None => matches!(chunk, "--color=auto" | "--colour=auto"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OH_MY_ZSH: &str =
        "--color=auto --exclude-dir={.bzr,CVS,.git,.hg,.svn,.idea,.tox,.venv,venv}";

    #[test]
    fn matches_when_only_spacing_quoting_or_harmless_grep_options_differ() {
        let cases = [
            ("cat f | grep x", "cat f | grep x".to_string()),
            ("cat  f|grep x", "cat f | grep x".to_string()),
            ("cat f | grep 'x'", "cat f | grep x".to_string()),
            ("cat f | grep x", format!("cat f | grep {OH_MY_ZSH} x")),
            ("grep x f | wc -l", format!("grep {OH_MY_ZSH} x f | wc -l")),
            ("cat f | grep x", "cat f | grep --color=auto x".to_string()),
            ("cat f | grep x", "cat f | grep --colour=auto x".to_string()),
            (
                "cat f | grep -v x",
                "cat f | grep --color=auto -v x".to_string(),
            ),
            (
                "grep x f | grep y",
                format!("grep {OH_MY_ZSH} x f | grep {OH_MY_ZSH} y"),
            ),
            ("sort f | uniq", "sort f | uniq".to_string()),
        ];
        for (typed, expanded) in cases {
            assert!(matches_typed(typed, &expanded), "{typed:?} vs {expanded:?}");
        }
    }

    #[test]
    fn differs_when_an_alias_changes_meaning() {
        let cases = [
            ("cat f | grep x", "bat f | grep x"),
            ("cat f | grep x", "cat f | grep -n x"),
            ("cat f | grep x", "cat f | grep --color=always x"),
            ("cat f | grep x", "cat f | grep --color x"),
            ("cat f | grep x", "cat f | grep --exclude=.git x"),
            ("cat f | grep x", "cat f | grep --exclude-dir= x"),
            ("cat f | grep x", "cat f | grep --exclude-dir=x; rm x"),
            ("cat f | grep x", "cat f | grep --exclude-dir=$HOME x"),
            ("cat f | grep x", "cat f | grep x --color=auto"),
            ("cat f | grep x", "cat f | rg --color=auto x"),
            ("cat f | grep x", "cat f | egrep --color=auto x"),
            ("sort f | uniq", "sort --color=auto f | uniq"),
            ("echo grep --color=auto", "echo grep"),
            ("echo grep", "echo grep --color=auto"),
            ("cat f | grep x", "cat f | grep x | less"),
            ("cat f | grep x", "cat f | grep x > out"),
            ("cat f | grep x", "cat $F | grep x"),
            ("cat f | grep x", ""),
        ];
        for (typed, expanded) in cases {
            assert!(!matches_typed(typed, expanded), "{typed:?} vs {expanded:?}");
        }
    }
}
