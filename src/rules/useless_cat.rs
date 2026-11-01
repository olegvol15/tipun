use crate::args::{self, Spec};
use crate::lexer::Redirect;
use crate::parser::{Command, Pipeline, Redirection};

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
const TR: Spec = Spec {
    flags: "cds",
    with_value: "",
};

enum FileInput {
    Operand,
    Redirect,
}

pub fn check(pipeline: &Pipeline) -> Option<Pipeline> {
    let [cat, next, rest @ ..] = pipeline.stages.as_slice() else {
        return None;
    };
    if cat.name.value != "cat" {
        return None;
    }
    let input = file_input(next)?;
    if pipeline
        .stages
        .iter()
        .any(|stage| !stage.redirects.is_empty())
    {
        return None;
    }
    let [file] = cat.args.as_slice() else {
        return None;
    };
    if file.value.is_empty() || file.value.starts_with('-') || is_process_dependent(&file.value) {
        return None;
    }
    let mut command = next.clone();
    match input {
        FileInput::Operand => command.args.push(file.clone()),
        FileInput::Redirect => command.redirects.push(Redirection {
            op: Redirect::In,
            target: Some(file.clone()),
        }),
    }
    let mut stages = vec![command];
    stages.extend_from_slice(rest);
    Some(Pipeline { stages })
}

// Entries like /proc/self/status describe whichever process opens them, so moving the read from
// `cat` to the next command changes their content. `..` is rejected rather than collapsed,
// because collapsing it is only correct without symlinks.
fn is_process_dependent(path: &str) -> bool {
    let Some(rest) = path.strip_prefix('/') else {
        return false;
    };
    let parts: Vec<&str> = rest
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    matches!(parts.first(), Some(&("proc" | "dev"))) || parts.contains(&"..")
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
        _ => return None,
    };
    let args = args::split(&command.args, spec)?;
    let operands = args.operands.len();
    let fits = match name {
        "grep" => operands == usize::from(!args.flags.contains(&'e')),
        // GNU wc pads several counts to a width taken from the file size, but not for a pipe.
        "wc" => operands == 0 && args.flags.len() == 1,
        "tr" => (1..=2).contains(&operands),
        _ => operands == 0,
    };
    let input = match name {
        "wc" | "tr" => FileInput::Redirect,
        _ => FileInput::Operand,
    };
    fits.then_some(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::render::render;

    fn parse_str(input: &str) -> Pipeline {
        parse(lex(input).expect("test input must lex")).expect("test input must parse")
    }

    fn rewrite(input: &str) -> Option<String> {
        check(&parse_str(input)).map(|pipeline| render(&pipeline))
    }

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
            ("cat f | grep x | wc -l", "grep x f | wc -l"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                rewrite(input).as_deref(),
                Some(expected),
                "input: {input:?}"
            );
        }
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
            "cat f | grep x > out",
            "cat f | grep x | sort > out",
            "cat f | grep x < g",
            "cat f 2>&1 | grep x",
            "cat f > g | grep x",
            "cat < f | grep x",
            "cat f | grep x 2>/dev/null",
            "cat f | wc -l g",
            "cat f | wc -l > out",
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
        ];
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
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
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "cat app.log | grep ERROR",
            "cat 'my file' | sort -t , -k 2 | uniq -c",
            "cat f | grep -e -x",
            "cat f | wc -l",
            "cat 'my file' | tr a b | sort",
        ];
        for input in inputs {
            let suggestion = check(&parse_str(input)).expect("input must be rewritten");
            assert_eq!(
                parse_str(&render(&suggestion)),
                suggestion,
                "input: {input:?}"
            );
            assert_eq!(check(&suggestion), None, "input: {input:?}");
        }
    }
}
