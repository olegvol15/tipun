use crate::args::{self, Spec};
use crate::lexer::Word;
use crate::parser::{Command, Pipeline};
use crate::rules::Rewrite;

const SORT: Spec = Spec {
    flags: "ru",
    with_value: "",
};

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    for (i, pair) in pipeline.stages.windows(2).enumerate() {
        if let Some(merged) = merge(&pair[0], &pair[1]) {
            let mut stages = pipeline.stages.clone();
            stages.splice(i..i + 2, [merged]);
            return Some(Rewrite {
                pipeline: Pipeline { stages },
                explanation: "sort -u drops duplicate lines itself, no need for uniq.".to_string(),
            });
        }
    }
    None
}

fn merge(sort: &Command, uniq: &Command) -> Option<Command> {
    if sort.name.value != "sort" || uniq.name.value != "uniq" || !uniq.args.is_empty() {
        return None;
    }
    if !sort.redirects.is_empty() || !uniq.redirects.is_empty() {
        return None;
    }
    let args = args::split(&sort.args, &SORT)?;
    let mut merged = sort.clone();
    if !args.flags.contains(&'u') {
        let unique = Word {
            raw: "-u".to_string(),
            value: "-u".to_string(),
        };
        merged.args.insert(0, unique);
    }
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
        check(&parse_str(input)).map(|rewrite| render(&rewrite.pipeline))
    }

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
            "sort f",
            "uniq f",
            "uniq | sort",
            "sort f | grep x | uniq",
            "sort f | sudo uniq",
            "sort f | uniq in",
            "sort f | uniq in out",
            "sort f > out | uniq",
            "sort f | uniq > out",
            "sort f 2>/dev/null | uniq",
            "sort < f | uniq",
            "sort -- f | uniq",
            "sort - | uniq",
            "sort f -r | uniq",
            "sort --unique f | uniq",
            "sort -o out f | uniq",
        ];
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
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
        for input in cases {
            assert_eq!(rewrite(input), None, "input: {input:?}");
        }
    }

    #[test]
    fn explains_the_rewrite() {
        let rewrite = check(&parse_str("sort f | uniq")).expect("input must be rewritten");
        assert_eq!(
            rewrite.explanation,
            "sort -u drops duplicate lines itself, no need for uniq."
        );
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = ["sort f | uniq", "sort -r 'my file' | uniq | head -n 3"];
        for input in inputs {
            let suggestion = check(&parse_str(input))
                .expect("input must be rewritten")
                .pipeline;
            assert_eq!(
                parse_str(&render(&suggestion)),
                suggestion,
                "input: {input:?}"
            );
            assert_eq!(check(&suggestion), None, "input: {input:?}");
        }
    }
}
