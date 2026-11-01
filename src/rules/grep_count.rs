use crate::args::{self, Spec};
use crate::lexer::Word;
use crate::parser::{Command, Pipeline};

const GREP: Spec = Spec {
    flags: "ivwxEFs",
    with_value: "e",
};

// Documented exceptions to the equivalence contract: with no matching lines `grep -c` exits 1,
// while the pipeline exits 0 with `wc`; and BSD `wc -l` pads the count with leading spaces.
pub fn check(pipeline: &Pipeline) -> Option<Pipeline> {
    for (i, pair) in pipeline.stages.windows(2).enumerate() {
        if let Some(merged) = merge(&pair[0], &pair[1]) {
            let mut stages = pipeline.stages.clone();
            stages.splice(i..i + 2, [merged]);
            return Some(Pipeline { stages });
        }
    }
    None
}

fn merge(grep: &Command, wc: &Command) -> Option<Command> {
    if grep.name.value != "grep" || wc.name.value != "wc" {
        return None;
    }
    if !matches!(wc.args.as_slice(), [flag] if flag.value == "-l") {
        return None;
    }
    if !grep.redirects.is_empty() || !wc.redirects.is_empty() {
        return None;
    }
    let args = args::split(&grep.args, &GREP)?;
    let patterns = usize::from(!args.flags.contains(&'e'));
    let files = args.operands.len().checked_sub(patterns)?;
    if files > 1 {
        return None;
    }
    let mut merged = grep.clone();
    let count = Word {
        raw: "-c".to_string(),
        value: "-c".to_string(),
    };
    merged.args.insert(0, count);
    Some(merged)
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
            ("grep x f | wc -l", "grep -c x f"),
            ("grep x f|wc -l", "grep -c x f"),
            (
                "grep \"a b\" 'my file' | wc -l",
                "grep -c \"a b\" 'my file'",
            ),
            ("grep -i x f | wc -l", "grep -c -i x f"),
            ("grep -vw x f | wc -l", "grep -c -vw x f"),
            ("grep -E 'a|b' f | wc -l", "grep -c -E 'a|b' f"),
            ("grep -e x -e y f | wc -l", "grep -c -e x -e y f"),
            ("grep -e x | wc -l", "grep -c -e x"),
            ("grep x | wc -l", "grep -c x"),
            ("grep '' f | wc -l", "grep -c '' f"),
            ("cat f | grep x | wc -l", "cat f | grep -c x"),
            ("sort f | grep x | wc -l", "sort f | grep -c x"),
            ("grep x f | wc -l | tr -d ' '", "grep -c x f | tr -d ' '"),
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
            "grep x f",
            "wc -l f",
            "wc -l | grep x",
            "grep x f | sort | wc -l",
            "grep x f | wc",
            "grep x f | wc -w",
            "grep x f | wc -lw",
            "grep x f | wc -l -l",
            "grep x f | wc -l g",
            "grep x f g | wc -l",
            "grep -e x f g | wc -l",
            "grep | wc -l",
            "grep x f > out | wc -l",
            "grep x f | wc -l > out",
            "grep x < f | wc -l",
            "grep x f 2>/dev/null | wc -l",
            "grep -- x f | wc -l",
            "grep x - | wc -l",
            "grep x f -i | wc -l",
            "grep --count x f | wc -l",
            "grep x f | sudo wc -l",
        ];
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
    }

    #[test]
    fn stays_silent_on_flags_that_change_what_is_printed() {
        let cases = [
            "grep -o x f | wc -l",
            "grep -A 1 x f | wc -l",
            "grep -B 1 x f | wc -l",
            "grep -C 1 x f | wc -l",
            "grep -q x f | wc -l",
            "grep -c x f | wc -l",
            "grep -H x f | wc -l",
            "grep -n x f | wc -l",
            "grep -l x f | wc -l",
            "grep -L x f | wc -l",
            "grep -r x | wc -l",
            "grep -m 2 x f | wc -l",
            "grep -f patterns f | wc -l",
        ];
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = ["grep x f | wc -l", "cat f | grep -e x | wc -l | sort"];
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
