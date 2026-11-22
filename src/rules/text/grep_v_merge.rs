use crate::args::{self, Spec};
use crate::lexer::{Redirect, Word};
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

const GREP: Spec = Spec {
    flags: "viwxFE",
    with_value: "",
};

struct Exclusion<'a> {
    flags: Vec<char>,
    flag_words: &'a [Word],
    pattern: &'a Word,
    file: Option<&'a Word>,
}

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    if !crate::rules::only_harmless_redirects(pipeline) {
        return None;
    }
    let stages = &pipeline.stages;
    let exclusions: Vec<Option<Exclusion>> = stages.iter().map(exclusion).collect();
    let start = (0..stages.len().saturating_sub(1)).find(|&i| joins(&exclusions, stages, i))?;
    let mut end = start + 1;
    while end + 1 < stages.len() && joins(&exclusions, stages, end) {
        end += 1;
    }
    let first = exclusions[start].as_ref()?;
    let mut args = first.flag_words.to_vec();
    for exclusion in exclusions[start..=end].iter().flatten() {
        args.push(word("-e"));
        args.push(exclusion.pattern.clone());
    }
    args.extend(first.file.cloned());
    let mut redirects = stages[start].redirects.clone();
    redirects.extend_from_slice(&stages[end].redirects);
    let merged = Command {
        name: stages[start].name.clone(),
        args,
        redirects,
    };
    let mut merged_stages = stages[..start].to_vec();
    merged_stages.push(merged);
    merged_stages.extend_from_slice(&stages[end + 1..]);
    Some(Rewrite {
        pipeline: Pipeline {
            stages: merged_stages,
        },
        explanation: "One grep -v can drop lines matching any of several -e patterns.".to_string(),
    })
}

// Stage `i + 1` reads only what stage `i` prints, with the same flags and no file of its own.
// Only input redirects stay on stage `i`, and they can only be on the first stage of the run.
fn joins(exclusions: &[Option<Exclusion>], stages: &[Command], i: usize) -> bool {
    let (Some(left), Some(right)) = (&exclusions[i], &exclusions[i + 1]) else {
        return false;
    };
    left.flags == right.flags
        && right.file.is_none()
        && stages[i]
            .redirects
            .iter()
            .all(|redirect| redirect.op == Redirect::In)
        && !crate::rules::reads_redirected_input(&stages[i + 1])
        && !crate::rules::redirects_stderr(&stages[i + 1])
}

fn exclusion(command: &Command) -> Option<Exclusion<'_>> {
    if command.name.value != "grep" {
        return None;
    }
    let split = args::split(&command.args, &GREP)?;
    let mut flags = split.flags;
    if flags.contains(&'E') && flags.contains(&'F') {
        return None;
    }
    flags.sort_unstable();
    flags.dedup();
    if !flags.contains(&'v') {
        return None;
    }
    let (pattern, file) = match split.operands.as_slice() {
        [pattern] => (*pattern, None),
        [pattern, file] => (*pattern, Some(*file)),
        _ => return None,
    };
    let flag_words = &command.args[..command.args.len() - split.operands.len()];
    Some(Exclusion {
        flags,
        flag_words,
        pattern,
        file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn merges_consecutive_exclusions() {
        let cases = [
            ("grep -v x f | grep -v y", "grep -v -e x -e y f"),
            ("grep -v x | grep -v y", "grep -v -e x -e y"),
            ("grep -v x f|grep -v y", "grep -v -e x -e y f"),
            (
                "grep -v x f | grep -v y | grep -v z",
                "grep -v -e x -e y -e z f",
            ),
            ("grep -vi x f | grep -iv y", "grep -vi -e x -e y f"),
            ("grep -v -i x f | grep -vi y", "grep -v -i -e x -e y f"),
            ("grep -vwx x f | grep -vxw y", "grep -vwx -e x -e y f"),
            ("grep -vF a.b f | grep -vF c", "grep -vF -e a.b -e c f"),
            ("grep -vE 'a|b' f | grep -vE c", "grep -vE -e 'a|b' -e c f"),
            ("grep -v '' f | grep -v x", "grep -v -e '' -e x f"),
            (
                "grep -v \"a b\" 'my file' | grep -v c",
                "grep -v -e \"a b\" -e c 'my file'",
            ),
            ("cat f | grep -v x | grep -v y", "cat f | grep -v -e x -e y"),
            (
                "grep -v x f | grep -v y | sort",
                "grep -v -e x -e y f | sort",
            ),
            (
                "grep a f | grep -v x | grep -v y",
                "grep a f | grep -v -e x -e y",
            ),
            (
                "grep -v x f | grep -vi y | grep -vi z",
                "grep -v x f | grep -vi -e y -e z",
            ),
            ("grep -v x f | grep -v y > out", "grep -v -e x -e y f > out"),
            (
                "grep -v x f | grep -v y 2>/dev/null",
                "grep -v -e x -e y f 2> /dev/null",
            ),
            ("grep -v x < f | grep -v y", "grep -v -e x -e y < f"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "grep -v x f",
            "grep x f | grep y",
            "grep -v x f | grep y",
            "grep x f | grep -v y",
            "grep -v x f | grep -vi y",
            "grep -vw x f | grep -vx y",
            "grep -v x f | grep -v y g",
            "grep -v x f g | grep -v y",
            "grep -v x f | sort | grep -v y",
            "grep -v x f > out | grep -v y",
            "grep -v x f 2> err | grep -v y",
            "grep -v x f | grep -v y 2> err",
            "grep -v x f | grep -v y 2>&1",
            "grep -v x f | grep -v y > out 2>&1",
            "grep -v x f | grep -v y < g",
            "grep -v x f | grep -v y > f",
            "grep -v x f | grep -v y 0> out",
            "grep -v f | grep -v",
            "grep -v -e x f | grep -v y",
            "grep -v x f | grep -v -e y",
            "grep -v -- x f | grep -v y",
            "grep -v x - | grep -v y",
            "grep -v x f -i | grep -v y",
            "grep -vn x f | grep -vn y",
            "grep -vc x f | grep -vc y",
            "grep -vo x f | grep -vo y",
            "grep -vs x f | grep -vs y",
            "grep -vl x f | grep -vl y",
            "grep -vh x f | grep -vh y",
            "grep -vA 1 x f | grep -v y",
            "grep -v x f | grep -v y -m 1",
            "grep -vr x f | grep -vr y",
            "grep -vEF z f | grep -vFE a.b",
            "grep -vE -F z f | grep -vE -F a.b",
            "grep --invert-match x f | grep -v y",
            "grep -v --color=auto x f | grep -v y",
            "egrep -v x f | egrep -v y",
            "sudo grep -v x f | grep -v y",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        assert_explains(
            check,
            &[(
                "grep -v x f | grep -v y",
                "One grep -v can drop lines matching any of several -e patterns.",
            )],
        );
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "grep -v x f | grep -v y",
            "cat f | grep -vi 'a b' | grep -vi c > out",
        ];
        assert_stable(check, &inputs);
    }
}
