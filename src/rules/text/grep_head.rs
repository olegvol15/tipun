use crate::args::{self, Spec};
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

const GREP: Spec = Spec {
    flags: "ivwxnEFs",
    with_value: "e",
};

// Documented exception, the same as grep-count: with no matching lines `grep -m` exits 1 while
// the pipeline exits 0 with `head`, so it only runs on the last pipeline of a chain.
pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    let [grep, head] = pipeline.stages.as_slice() else {
        return None;
    };
    if grep.name.value != "grep" || head.name.value != "head" || !grep.redirects.is_empty() {
        return None;
    }
    if crate::rules::reads_redirected_input(head)
        || crate::rules::redirects_stderr(head)
        || !crate::rules::only_harmless_redirects(pipeline)
    {
        return None;
    }
    let lines = lines(head)?;
    let args = args::split(&grep.args, &GREP)?;
    let patterns = usize::from(!args.flags.contains(&'e'));
    if args.operands.len() != patterns + 1 {
        return None;
    }
    let mut args = vec![word("-m"), word(lines)];
    args.extend_from_slice(&grep.args);
    let merged = Command {
        name: grep.name.clone(),
        args,
        redirects: head.redirects.clone(),
    };
    Some(Rewrite {
        pipeline: Pipeline {
            stages: vec![merged],
        },
        explanation: "grep -m stops after the first matching lines, no need for head.".to_string(),
    })
}

// BSD grep treats `-m 0` as no limit and rejects counts that overflow, so N stays 1 to 9 digits.
fn lines(head: &Command) -> Option<&str> {
    let count = match head.args.as_slice() {
        [] => "10",
        [flag, count] if flag.value == "-n" => count.value.as_str(),
        [flag] => flag
            .value
            .strip_prefix("-n")
            .or_else(|| flag.value.strip_prefix('-'))?,
        _ => return None,
    };
    let valid = (1..=9).contains(&count.len())
        && !count.starts_with('0')
        && count.bytes().all(|byte| byte.is_ascii_digit());
    valid.then_some(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_supported_pipelines() {
        let cases = [
            ("grep x f | head -n 3", "grep -m 3 x f"),
            ("grep x f|head -n 3", "grep -m 3 x f"),
            ("grep x f | head -n3", "grep -m 3 x f"),
            ("grep x f | head -3", "grep -m 3 x f"),
            ("grep x f | head", "grep -m 10 x f"),
            ("grep x f | head -n 1", "grep -m 1 x f"),
            ("grep x f | head -n 123456789", "grep -m 123456789 x f"),
            ("grep -v x f | head -n 2", "grep -m 2 -v x f"),
            ("grep -in x f | head -n 2", "grep -m 2 -in x f"),
            ("grep -E 'a|b' f | head -n 2", "grep -m 2 -E 'a|b' f"),
            ("grep -e x -e y f | head -n 2", "grep -m 2 -e x -e y f"),
            (
                "grep \"a b\" 'my file' | head -1",
                "grep -m 1 \"a b\" 'my file'",
            ),
            ("grep x f | head -n 2 > out", "grep -m 2 x f > out"),
            ("grep x f | head -n 2 >> out", "grep -m 2 x f >> out"),
            (
                "grep x f | head -n 2 2>/dev/null",
                "grep -m 2 x f 2> /dev/null",
            ),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "grep x f",
            "head -n 2 f",
            "grep x | head -n 2",
            "grep -e x | head -n 2",
            "grep x f g | head -n 2",
            "grep -e x f g | head -n 2",
            "grep x - | head -n 2",
            "grep -- x f | head -n 2",
            "grep x f -i | head -n 2",
            "grep x < f | head -n 2",
            "grep x f 2> err | head -n 2",
            "grep x f | head -n 2 2> err",
            "grep x f | head -n 2 2>&1",
            "grep x f | head -n 2 > out 2>&1",
            "cat f | grep x | head -n 2",
            "tee out < f | grep x | head -n 2",
            "grep x f | head -n 2 | sort",
            "grep x f | head -n 2 g",
            "grep x f | head -n 2 < g",
            "grep x f | head -n 2 > f",
            "grep x f | head -n 2 0> out",
            "grep x f | head -c 2",
            "grep x f | head -n 0",
            "grep x f | head -0",
            "grep x f | head -n 02",
            "grep x f | head -n 1234567890",
            "grep x f | head -n -2",
            "grep x f | head -n +2",
            "grep x f | head -n 1k",
            "grep x f | head -n ''",
            "grep x f | head -n",
            "grep x f | head --lines=2",
            "grep x f | head -q -n 2",
            "grep x f | sudo head -n 2",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn stays_silent_on_flags_that_change_what_is_printed() {
        let cases = [
            "grep -o x f | head -n 2",
            "grep -A 1 x f | head -n 2",
            "grep -C 1 x f | head -n 2",
            "grep -c x f | head -n 2",
            "grep -l x f | head -n 2",
            "grep -L x f | head -n 2",
            "grep -q x f | head -n 2",
            "grep -H x f | head -n 2",
            "grep -r x f | head -n 2",
            "grep -m 5 x f | head -n 2",
            "grep -f patterns f | head -n 2",
            "grep --color=auto x f | head -n 2",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        assert_explains(
            check,
            &[(
                "grep x f | head -n 2",
                "grep -m stops after the first matching lines, no need for head.",
            )],
        );
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "grep x f | head -n 2",
            "grep -e x 'my file' | head -3 > out",
        ];
        assert_stable(check, &inputs);
    }
}
