use crate::lexer::{self, Token};
use crate::render;

// bash has no hook that hands over the expanded command, so the bash adapter passes the output of
// `alias` and command words are expanded here.
pub fn expand_aliases(typed: &str, aliases: &str) -> Option<String> {
    let aliases = aliases
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse_alias)
        .collect::<Option<Vec<_>>>()?;
    let mut parts = Vec::new();
    let mut at_command = true;
    for token in lexer::lex(typed).ok()? {
        match token {
            Token::Word(word) => {
                let alias = aliases
                    .iter()
                    .find(|(name, _)| at_command && word.raw == *name);
                match alias {
                    Some((name, value)) => parts.push(checked_value(name, value, &aliases)?),
                    None => parts.push(word.raw),
                }
                at_command = false;
            }
            Token::Pipe => {
                parts.push("|".to_string());
                at_command = true;
            }
            Token::And => {
                parts.push("&&".to_string());
                at_command = true;
            }
            Token::Redirect(_) if at_command => return None,
            Token::Redirect(op) => parts.push(render::redirect_op(&op)),
        }
    }
    Some(parts.join(" "))
}

fn parse_alias(line: &str) -> Option<(&str, String)> {
    let (name, quoted) = line.strip_prefix("alias ")?.split_once('=')?;
    let inner = quoted.strip_prefix('\'')?.strip_suffix('\'')?;
    let pieces: Vec<&str> = inner.split(r"'\''").collect();
    if name.is_empty() || pieces.iter().any(|piece| piece.contains('\'')) {
        return None;
    }
    Some((name, pieces.join("'")))
}

// A value ending in a blank makes bash expand the next word too, and a value starting with another
// alias is expanded again. Both are left to silence.
fn checked_value(name: &str, value: &str, aliases: &[(&str, String)]) -> Option<String> {
    let first = value.split_whitespace().next()?;
    let chained = first != name && aliases.iter().any(|(other, _)| *other == first);
    (!value.ends_with([' ', '\t']) && !chained).then(|| value.to_string())
}

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

fn drop_harmless_grep_options(expanded: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut at_command = true;
    let mut after_grep = false;
    for chunk in expanded.split(' ') {
        if after_grep && is_harmless_grep_option(chunk) {
            continue;
        }
        after_grep = at_command && chunk == "grep";
        at_command = matches!(chunk, "|" | "&&");
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
            (
                "cd src && cat f | grep x",
                format!("cd src && cat f | grep {OH_MY_ZSH} x"),
            ),
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
            ("cd src && cat f | grep x", "cd lib && cat f | grep x"),
            ("cat f | grep x && ls", "cat f | grep x && ls -G"),
        ];
        for (typed, expanded) in cases {
            assert!(!matches_typed(typed, expanded), "{typed:?} vs {expanded:?}");
        }
    }

    const BASH_ALIASES: &str = r"alias cat='bat'
alias egrep='egrep --color=auto'
alias grep='grep --color=auto'
alias l='ls -CF'
alias ll='ls -alF'
alias ls='ls --color=auto'
alias q=''\'''
alias sudo='sudo '
";

    #[test]
    fn expands_aliases_in_command_position() {
        let cases = [
            ("sort f | uniq", "sort f | uniq"),
            ("cat f | grep x", "bat f | grep --color=auto x"),
            ("grep x f | wc -l", "grep --color=auto x f | wc -l"),
            ("echo grep cat", "echo grep cat"),
            ("cd src && grep x f", "cd src && grep --color=auto x f"),
            ("grep x f > out 2>&1", "grep --color=auto x f > out 2>&1"),
            ("'grep' x f", "'grep' x f"),
            ("ls", "ls --color=auto"),
            ("q", "'"),
        ];
        for (typed, expected) in cases {
            let expanded = expand_aliases(typed, BASH_ALIASES);
            assert_eq!(expanded.as_deref(), Some(expected), "typed: {typed:?}");
        }
        assert_eq!(expand_aliases("cat f", "").as_deref(), Some("cat f"));
    }

    #[test]
    fn stays_silent_on_aliases_it_cannot_expand_safely() {
        let cases = [
            ("sudo grep x f", BASH_ALIASES),
            ("ll", BASH_ALIASES),
            ("l", BASH_ALIASES),
            ("> out grep x f", BASH_ALIASES),
            ("cat f | grep x; ls", BASH_ALIASES),
            ("cat f", "alias cat='bat"),
            ("cat f", "alias cat=bat"),
            ("cat f", "alias ='bat'"),
            ("cat f", "alias cat='b'a't'"),
            ("cat f", "cat='bat'"),
            ("cat f", "alias cat='bat\nmore'"),
        ];
        for (typed, aliases) in cases {
            assert_eq!(
                expand_aliases(typed, aliases),
                None,
                "{typed:?} with {aliases:?}"
            );
        }
    }

    #[test]
    fn grep_color_alias_keeps_the_tip_and_cat_alias_drops_it() {
        let grep_only = "alias grep='grep --color=auto'\n";
        let expanded = expand_aliases("cat f | grep x", grep_only).expect("aliases expand");
        assert!(matches_typed("cat f | grep x", &expanded));
        let expanded = expand_aliases("cat f | grep x", BASH_ALIASES).expect("aliases expand");
        assert!(!matches_typed("cat f | grep x", &expanded));
    }
}
