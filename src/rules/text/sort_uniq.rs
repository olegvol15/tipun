use crate::args::{self, Spec};
use crate::lexer::Word;
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

const SORT: Spec = Spec {
    flags: "ru",
    with_value: "",
};

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    pipeline
        .stages
        .windows(2)
        .enumerate()
        .find_map(|(i, pair)| {
            let args = unique_args(&pair[0], &pair[1])?;
            let explanation = "sort -u drops duplicate lines itself, no need for uniq.";
            crate::rules::absorb(pipeline, i, args, explanation)
        })
}

fn unique_args(sort: &Command, uniq: &Command) -> Option<Vec<Word>> {
    if sort.name.value != "sort" || uniq.name.value != "uniq" || !uniq.args.is_empty() {
        return None;
    }
    let flags = args::split(&sort.args, &SORT)?.flags;
    let mut args = sort.args.clone();
    if !flags.contains(&'u') {
        args.insert(0, word("-u"));
    }
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_supported_pipelines() {
        let cases = [
            ("sort | uniq", "sort -u"),
            ("sort f | uniq", "sort -u f"),
            ("sort f|uniq", "sort -u f"),
            ("sort 'my file' | uniq", "sort -u 'my file'"),
            ("sort f g | uniq", "sort -u f g"),
            ("sort -r f | uniq", "sort -u -r f"),
            ("sort -u f | uniq", "sort -u f"),
            ("sort -ru f | uniq", "sort -ru f"),
            ("cat f | sort | uniq", "cat f | sort -u"),
            (
                "grep x f | sort | uniq | wc -l",
                "grep x f | sort -u | wc -l",
            ),
            ("sort f | uniq | sort | uniq", "sort -u f | sort | uniq"),
            ("sort f | uniq > out", "sort -u f > out"),
            ("sort f | uniq >> out 2>&1", "sort -u f >> out 2>&1"),
            ("sort f | uniq 2>/dev/null", "sort -u f 2> /dev/null"),
            ("sort f | uniq | head > out", "sort -u f | head > out"),
            ("grep x f > out | sort | uniq", "grep x f > out | sort -u"),
            ("tr a b < f | sort | uniq", "tr a b < f | sort -u"),
            (
                "tr a b < f | sort | uniq > out",
                "tr a b < f | sort -u > out",
            ),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "sort f",
            "uniq f",
            "uniq | sort",
            "sort f | grep x | uniq",
            "sort f | sudo uniq",
            "sort f | uniq in",
            "sort f | uniq in out",
            "sort f > out | uniq",
            "sort f | uniq > f",
            "sort dir/f | uniq >> f",
            "sort f | uniq | head > f",
            "sort f | uniq < g",
            "sort f | uniq > out < g",
            "tr a b < f | sort | uniq > f",
            "sort f | uniq > /dev/stdout",
            "sort f | uniq 2>&0",
            "sort f 2>/dev/null | uniq",
            "sort < f | uniq",
            "sort -- f | uniq",
            "sort - | uniq",
            "sort f -r | uniq",
            "sort --unique f | uniq",
            "sort -o out f | uniq",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn stays_silent_on_flags_that_change_equality() {
        let cases = [
            "sort f | uniq -c",
            "sort f | uniq -d",
            "sort f | uniq -u",
            "sort f | uniq -i",
            "sort -f f | uniq",
            "sort -n f | uniq",
            "sort -g f | uniq",
            "sort -h f | uniq",
            "sort -k 2 f | uniq",
            "sort -t , -k 2 f | uniq",
            "sort -b f | uniq",
            "sort -d f | uniq",
            "sort -i f | uniq",
            "sort -M f | uniq",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        assert_explains(
            check,
            &[(
                "sort f | uniq",
                "sort -u drops duplicate lines itself, no need for uniq.",
            )],
        );
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "sort f | uniq",
            "sort -r 'my file' | uniq | head -n 3",
            "sort f | uniq > out 2>&1",
        ];
        assert_stable(check, &inputs);
    }
}
