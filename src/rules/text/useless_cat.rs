use crate::args::{self, Spec};
use crate::lexer::Redirect;
use crate::parser::{Command, Pipeline, Redirection};
use crate::rules::Rewrite;

const GREP: Spec = Spec {
    flags: "cinoqsvwxEF",
    with_value: "e",
};
const HEAD_TAIL: Spec = Spec {
    flags: "",
    with_value: "nc",
};
const SORT: Spec = Spec {
    flags: "bfnru",
    with_value: "kt",
};
const UNIQ: Spec = Spec {
    flags: "cdiu",
    with_value: "fs",
};
const WC: Spec = Spec {
    flags: "lwcm",
    with_value: "",
};
const CUT: Spec = Spec {
    flags: "s",
    with_value: "bcdf",
};
const SED: Spec = Spec {
    flags: "nEr",
    with_value: "",
};
const TR: Spec = Spec {
    flags: "cds",
    with_value: "",
};

enum FileInput {
    Operand,
    Redirect,
}

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    let [cat, next, rest @ ..] = pipeline.stages.as_slice() else {
        return None;
    };
    if cat.name.value != "cat" {
        return None;
    }
    let input = file_input(next)?;
    if !cat.redirects.is_empty()
        || crate::rules::reads_redirected_input(next)
        || !crate::rules::only_harmless_redirects(pipeline)
    {
        return None;
    }
    let [file] = cat.args.as_slice() else {
        return None;
    };
    if file.value.is_empty()
        || file.value.starts_with('-')
        || crate::rules::is_process_dependent(&file.value)
    {
        return None;
    }
    let name = &next.name.value;
    let mut command = next.clone();
    let explanation = match input {
        FileInput::Operand => {
            command.args.push(file.clone());
            format!("{name} can read the file itself, no need for cat.")
        }
        FileInput::Redirect => {
            command.redirects.push(Redirection {
                op: Redirect::In,
                target: Some(file.clone()),
            });
            format!("{name} can read the file through <, no need for cat.")
        }
    };
    let mut stages = vec![command];
    stages.extend_from_slice(rest);
    Some(Rewrite {
        pipeline: Pipeline { stages },
        explanation,
    })
}

fn file_input(command: &Command) -> Option<FileInput> {
    let name = command.name.value.as_str();
    let spec = match name {
        "grep" => &GREP,
        "head" | "tail" => &HEAD_TAIL,
        "sort" => &SORT,
        "uniq" => &UNIQ,
        "wc" => &WC,
        "tr" => &TR,
        "cut" => &CUT,
        "sed" => &SED,
        _ => return None,
    };
    let args = args::split(&command.args, spec)?;
    let operands = args.operands.len();
    let fits = match name {
        "grep" => operands == usize::from(!args.flags.contains(&'e')),
        // GNU wc pads several counts to a width taken from the file size, but not for a pipe.
        "wc" => operands == 0 && args.flags.len() == 1,
        "tr" => (1..=2).contains(&operands),
        "cut" => operands == 0 && args.flags.iter().any(|flag| "bcf".contains(*flag)),
        "sed" => matches!(args.operands.as_slice(), [script] if is_plain_sed(&script.value)),
        _ => operands == 0,
    };
    let input = match name {
        "wc" | "tr" => FileInput::Redirect,
        _ => FileInput::Operand,
    };
    fits.then_some(input)
}

// One s, p, d, q or = command. Others read or write files that can be /dev/stdin (r, R, w),
// run commands (e) or print the file name (F), which is - for a pipe.
fn is_plain_sed(script: &str) -> bool {
    let Some(rest) = sed_range(script) else {
        return false;
    };
    match rest.split_at_checked(1) {
        Some(("p" | "d" | "q" | "=", "")) => true,
        Some(("s", rest)) => is_plain_substitution(rest),
        _ => false,
    }
}

fn sed_range(script: &str) -> Option<&str> {
    let rest = sed_address(script)?;
    match rest.strip_prefix(',') {
        Some(second) if second.starts_with(|c: char| c.is_ascii_digit() || "$/".contains(c)) => {
            sed_address(second)
        }
        Some(_) => None,
        None => Some(rest),
    }
}

fn sed_address(script: &str) -> Option<&str> {
    if let Some(regex) = script.strip_prefix('/') {
        return delimited(regex, '/');
    }
    if let Some(rest) = script.strip_prefix('$') {
        return Some(rest);
    }
    Some(script.trim_start_matches(|c: char| c.is_ascii_digit()))
}

fn is_plain_substitution(rest: &str) -> bool {
    let mut chars = rest.chars();
    let Some(delimiter) = chars
        .next()
        .filter(|c| c.is_ascii_punctuation() && *c != '\\')
    else {
        return false;
    };
    delimited(chars.as_str(), delimiter)
        .and_then(|replacement| delimited(replacement, delimiter))
        .is_some_and(|flags| {
            flags
                .chars()
                .all(|c| c.is_ascii_digit() || "gpiI".contains(c))
        })
}

fn delimited(text: &str, delimiter: char) -> Option<&str> {
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next()?;
        } else if c == delimiter {
            return Some(&text[i + c.len_utf8()..]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_supported_pipelines() {
        let cases = [
            ("cat app.log | grep ERROR", "grep ERROR app.log"),
            ("cat app.log|grep ERROR", "grep ERROR app.log"),
            ("cat 'my file' | grep \"a b\"", "grep \"a b\" 'my file'"),
            ("cat f | grep -ci x", "grep -ci x f"),
            ("cat f | grep -v -n x", "grep -v -n x f"),
            ("cat f | grep -e x -e y", "grep -e x -e y f"),
            ("cat f | grep -ex", "grep -ex f"),
            ("cat f | grep -e -x", "grep -e -x f"),
            ("cat f | grep ''", "grep '' f"),
            ("cat f | grep -E 'a|b'", "grep -E 'a|b' f"),
            ("cat f | head", "head f"),
            ("cat f | head -n 5", "head -n 5 f"),
            ("cat f | head -n5", "head -n5 f"),
            ("cat f | head -c 100", "head -c 100 f"),
            ("cat f | tail -n 5", "tail -n 5 f"),
            ("cat f | tail -n +2", "tail -n +2 f"),
            ("cat f | sort", "sort f"),
            ("cat f | sort -rn", "sort -rn f"),
            ("cat f | sort -t , -k 2", "sort -t , -k 2 f"),
            ("cat f | uniq -c", "uniq -c f"),
            ("cat f | uniq -f 1", "uniq -f 1 f"),
            ("cat f | sort | uniq -c", "sort f | uniq -c"),
            (
                "cat f | grep x | sort | head -n 3",
                "grep x f | sort | head -n 3",
            ),
            ("'cat' f | grep x", "grep x f"),
            ("cat /var/log/app.log | grep x", "grep x /var/log/app.log"),
            ("cat /proceeds.txt | grep x", "grep x /proceeds.txt"),
            ("cat ../app.log | grep x", "grep x ../app.log"),
            ("cat app.log | wc -l", "wc -l < app.log"),
            ("cat f | wc -w", "wc -w < f"),
            ("cat f | wc -c", "wc -c < f"),
            ("cat f | wc -m", "wc -m < f"),
            ("cat 'my file' | wc -l", "wc -l < 'my file'"),
            ("cat f | tr a-z A-Z", "tr a-z A-Z < f"),
            ("cat f | tr -d '\\r'", "tr -d '\\r' < f"),
            ("cat f | tr -s ' '", "tr -s ' ' < f"),
            ("cat f | tr -cd a-z", "tr -cd a-z < f"),
            ("cat f | tr a b | sort", "tr a b < f | sort"),
            ("cat f | cut -d , -f 2", "cut -d , -f 2 f"),
            ("cat f | cut -d, -f1,3", "cut -d, -f1,3 f"),
            ("cat f | cut -c 1-3", "cut -c 1-3 f"),
            ("cat f | cut -b 2", "cut -b 2 f"),
            ("cat f | cut -sd , -f 2", "cut -sd , -f 2 f"),
            ("cat f | sed 's/a/b/'", "sed 's/a/b/' f"),
            ("cat f | sed -n 2p", "sed -n 2p f"),
            ("cat f | sed -E 's/(a)/\\1/'", "sed -E 's/(a)/\\1/' f"),
            ("cat f | sed -nr '/x/p'", "sed -nr '/x/p' f"),
            ("cat f | sed q | sort", "sed q f | sort"),
            ("cat f | sed 3q", "sed 3q f"),
            ("cat f | sed -n '$='", "sed -n '$=' f"),
            ("cat f | sed -n '2,$p'", "sed -n '2,$p' f"),
            ("cat f | sed '/a/,/b/d'", "sed '/a/,/b/d' f"),
            ("cat f | sed 's|a/b|c|g'", "sed 's|a/b|c|g' f"),
            ("cat f | sed 's/a\\/b/c/2'", "sed 's/a\\/b/c/2' f"),
            ("cat f | sed '/x\\/y/s/e/r/I'", "sed '/x\\/y/s/e/r/I' f"),
            ("cat f | sed 's/error/rewrite/'", "sed 's/error/rewrite/' f"),
            ("cat f | grep x | wc -l", "grep x f | wc -l"),
            ("cat f | grep x > out", "grep x f > out"),
            ("cat f | grep x >> out", "grep x f >> out"),
            ("cat f | grep x 2>/dev/null", "grep x f 2> /dev/null"),
            ("cat f | grep x > out 2>&1", "grep x f > out 2>&1"),
            ("cat f | grep x | sort > out", "grep x f | sort > out"),
            ("cat f | grep x > out | sort", "grep x f > out | sort"),
            ("cat f | head -n 3 > 'my out'", "head -n 3 f > 'my out'"),
            ("cat f | wc -l > out", "wc -l > out < f"),
            ("cat f | tr a b 2> err", "tr a b 2> err < f"),
            ("cat f | grep x | tr a b < g", "grep x f | tr a b < g"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "cat f",
            "ls | grep x",
            "grep x | cat f",
            "cat | grep x",
            "cat f g | grep x",
            "cat -n f | grep x",
            "cat - | grep x",
            "cat -- f | grep x",
            "cat '' | grep x",
            "cat '-f' | grep x",
            "cat /proc/self/status | grep Vm",
            "cat /proc/meminfo | grep MemTotal",
            "cat '/proc/self/comm' | tr a-z A-Z",
            "cat /dev/stdin | wc -l",
            "cat /dev/fd/0 | sort",
            "cat //proc/self/status | grep Vm",
            "cat /./proc/self/status | grep Vm",
            "cat /.//dev/stdin | wc -l",
            "cat /proc | grep x",
            "cat /tmp/../proc/self/status | grep Vm",
            "cat f | grep x < g",
            "cat f 2>&1 | grep x",
            "cat f 2>/dev/null | grep x",
            "cat f > g | grep x",
            "cat < f | grep x",
            "cat f | grep x > f",
            "cat f | grep x >> ./f",
            "cat dir/f | sort > f",
            "cat f | grep x | sort > f",
            "cat f | grep x > ../out",
            "cat f | grep x > /dev/stdin",
            "cat f | grep x 0>&2",
            "cat f | grep x 3> out",
            "cat f | wc -l 0> out",
            "cat f | tr a b > f",
            "cat f | wc -l < g",
            "cat f | grep x | tr a b < g > g",
            "cat f | wc -l g",
            "cat f | wc -l x -w",
            "cat f | tr",
            "cat f | tr a b c",
            "cat f | tr a b < g",
            "cat f | cat",
            "cat f | sudo grep x",
            "cat f | grep",
            "cat f | grep x g",
            "cat f | grep -e x g",
            "cat f | grep x -i",
            "cat f | grep -- x",
            "cat f | grep --color x",
            "cat f | grep -e",
            "cat f | head -5",
            "cat f | head -n",
            "cat f | head g",
            "cat f | tail -n 5 g",
            "cat f | sort -o out",
            "cat f | sort g",
            "cat f | uniq in",
            "cat f | uniq in out",
            "cat f | cut -f 1 g",
            "cat f | cut",
            "cat f | cut -f",
            "cat f | cut --complement -f 1",
            "cat f | cut -z -f 1",
            "cat f | cut -f 1 -",
            "cat f | sed",
            "cat f | sed s/a/b/ g",
            "cat f | sed -e s/a/b/",
            "cat f | sed -f script",
            "cat f | sed -i s/a/b/",
            "cat f | sed -i '' s/a/b/",
            "cat f | sed -s s/a/b/",
            "cat f | sed -u s/a/b/",
            "cat f | sed -z s/a/b/",
            "cat f | sed --posix s/a/b/",
            "cat f | sed F",
            "cat f | sed -n '1F'",
            "cat f | sed 's/F/x/w out'",
            "cat f | sed 'r /dev/stdin'",
            "cat f | sed 'r /dev/fd/0'",
            "cat f | sed '1r other'",
            "cat f | sed 'R /dev/stdin'",
            "cat f | sed 'w out'",
            "cat f | sed 'W out'",
            "cat f | sed 'e cat'",
            "cat f | sed 's/x/cat/e'",
            "cat f | sed 's/x/y/w /dev/stdout'",
            "cat f | sed '1!G;h;$!d'",
            "cat f | sed 'p;p'",
            "cat f | sed q5",
            "cat f | sed ''",
            "cat f | sed ' p'",
            "cat f | sed '1,p'",
            "cat f | sed '/a'",
            "cat f | sed 's/a/b'",
            "cat f | sed 'sxaxbx'",
            "cat f | sed 's\\a\\b\\'",
            "cat f | sed '/a/Ip'",
            "cat f | sed 'y/a/b/'",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn stays_silent_on_flags_that_change_output() {
        let cases = [
            "cat f | grep -H x",
            "cat f | grep -l x",
            "cat f | grep -L x",
            "cat f | grep -r x",
            "cat f | grep -R x",
            "cat f | grep -f patterns",
            "cat f | head -v",
            "cat f | head -q",
            "cat f | tail -f",
            "cat f | tail -F",
            "cat f | tail -r",
            "cat f | sort -c",
            "cat f | sort -m",
            "cat f | uniq -D",
            "cat f | wc",
            "cat f | wc -lw",
            "cat f | wc -l -w",
            "cat f | wc -L",
            "cat f | tr -t ab x",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_how_the_file_is_passed() {
        let cases = [
            (
                "cat f | grep x",
                "grep can read the file itself, no need for cat.",
            ),
            (
                "cat f | wc -l",
                "wc can read the file through <, no need for cat.",
            ),
        ];
        assert_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "cat app.log | grep ERROR",
            "cat 'my file' | sort -t , -k 2 | uniq -c",
            "cat f | grep -e -x",
            "cat f | wc -l",
            "cat 'my file' | tr a b | sort",
            "cat f | grep x > out 2>&1",
            "cat f | wc -l > out",
        ];
        assert_stable(check, &inputs);
    }
}
