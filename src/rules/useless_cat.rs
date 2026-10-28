use crate::args::{self, Spec};
use crate::parser::{Command, Pipeline};

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

pub fn check(pipeline: &Pipeline) -> Option<Pipeline> {
    let [cat, next, rest @ ..] = pipeline.stages.as_slice() else {
        return None;
    };
    if cat.name.value != "cat" || !reads_only_stdin(next) {
        return None;
    }
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
    if file.value.is_empty() || file.value.starts_with('-') {
        return None;
    }
    let mut command = next.clone();
    command.args.push(file.clone());
    let mut stages = vec![command];
    stages.extend_from_slice(rest);
    Some(Pipeline { stages })
}

fn reads_only_stdin(command: &Command) -> bool {
    let name = command.name.value.as_str();
    let spec = match name {
        "grep" => &GREP,
        "head" | "tail" => &HEAD_TAIL,
        "sort" => &SORT,
        "uniq" => &UNIQ,
        _ => return false,
    };
    let Some(args) = args::split(&command.args, spec) else {
        return false;
    };
    let pattern_operands = usize::from(name == "grep" && !args.flags.contains(&'e'));
    args.operands.len() == pattern_operands
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
            "cat f | grep x > out",
            "cat f | grep x | sort > out",
            "cat f | grep x < g",
            "cat f 2>&1 | grep x",
            "cat f > g | grep x",
            "cat < f | grep x",
            "cat f | grep x 2>/dev/null",
            "cat f | wc -l",
            "cat f | tr a b",
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
