use crate::args::{self, Spec};
use crate::lexer::{Redirect, Word};
use crate::parser::{Command, Pipeline, Redirection};
use crate::rules::Rewrite;

const GREP: Spec = Spec {
    flags: "ivwxEFs",
    with_value: "e",
};

// Documented exceptions to the equivalence contract: with no matching lines `grep -c` exits 1,
// while the pipeline exits 0 with `wc`, so it only runs on the last pipeline of a chain;
// and BSD `wc -l` pads the count with leading spaces.
// Only the end of a pipeline is rewritten and its stdout is never redirected, so the padding
// never reaches another command or a file.
pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    let [rest @ .., grep, wc] = pipeline.stages.as_slice() else {
        return None;
    };
    let merged = merge(grep, wc)?;
    if !crate::rules::only_harmless_redirects(pipeline) {
        return None;
    }
    let mut stages = rest.to_vec();
    stages.push(merged);
    Some(Rewrite {
        pipeline: Pipeline { stages },
        explanation: "grep -c counts matching lines itself, no need for wc -l.".to_string(),
    })
}

fn merge(grep: &Command, wc: &Command) -> Option<Command> {
    if grep.name.value != "grep" || wc.name.value != "wc" {
        return None;
    }
    if !matches!(wc.args.as_slice(), [flag] if flag.value == "-l") {
        return None;
    }
    if !grep.redirects.is_empty()
        || crate::rules::reads_redirected_input(wc)
        || wc.redirects.iter().any(writes_stdout)
    {
        return None;
    }
    let args = args::split(&grep.args, &GREP)?;
    let patterns = usize::from(!args.flags.contains(&'e'));
    let files = args.operands.len().checked_sub(patterns)?;
    if files > 1 {
        return None;
    }
    let mut merged = grep.clone();
    merged.redirects = wc.redirects.clone();
    let count = Word {
        raw: "-c".to_string(),
        value: "-c".to_string(),
    };
    merged.args.insert(0, count);
    Some(merged)
}

fn writes_stdout(redirect: &Redirection) -> bool {
    match redirect.op {
        Redirect::Out { fd } | Redirect::Append { fd } => matches!(fd, None | Some(1)),
        Redirect::Dup { fd, .. } => fd == 1,
        Redirect::In => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

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
            ("grep x f | wc -l 2> err", "grep -c x f 2> err"),
            ("grep x f | wc -l 2>&1", "grep -c x f 2>&1"),
            ("grep x f | wc -l 2>/dev/null", "grep -c x f 2> /dev/null"),
            (
                "sort f > sorted | grep x | wc -l",
                "sort f > sorted | grep -c x",
            ),
            ("tr a b < f | grep x | wc -l", "tr a b < f | grep -c x"),
        ];
        assert_rewrites(check, &cases);
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
            "grep x f | wc -l > f",
            "grep x dir/f | wc -l >> f",
            "cat f | grep x | wc -l > f",
            "grep x f | wc -l < g",
            "grep x f | wc -l > count",
            "grep x f | wc -l >> count",
            "grep x f | wc -l 1> count",
            "grep x f | wc -l 2> err > count",
            "grep x f | wc -l 1>&2",
            "tr a b < f | grep x | wc -l > f",
            "grep x f | wc -l 0> out",
            "grep x f | wc -l > /dev/stdout",
            "grep x < f | wc -l",
            "grep x f 2>/dev/null | wc -l",
            "grep -- x f | wc -l",
            "grep x - | wc -l",
            "grep x f -i | wc -l",
            "grep --count x f | wc -l",
            "grep x f | sudo wc -l",
            "grep x f | wc -l | tr -d ' '",
            "grep x f | wc -l | cut -c1",
            "grep x f | wc -l | sort",
        ];
        assert_silent(check, &cases);
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
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        assert_explains(
            check,
            &[(
                "grep x f | wc -l",
                "grep -c counts matching lines itself, no need for wc -l.",
            )],
        );
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "grep x f | wc -l",
            "cat f | grep -e x | wc -l",
            "grep x f | wc -l 2> err",
        ];
        assert_stable(check, &inputs);
    }
}
