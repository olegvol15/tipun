use crate::lexer::lex;
use crate::parser::{Chain, Pipeline, parse};
use crate::render::render_pipeline as render;
use crate::rules::Rule;

pub fn parse_chain(input: &str) -> Chain {
    parse(lex(input).expect("test input must lex")).expect("test input must parse")
}

pub fn parse_str(input: &str) -> Pipeline {
    let mut chain = parse_chain(input);
    assert_eq!(chain.pipelines.len(), 1, "input: {input:?}");
    chain.pipelines.remove(0)
}

pub fn assert_rewrites(rule: Rule, cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        let actual = rule(&parse_str(input)).map(|rewrite| render(&rewrite.pipeline));
        assert_eq!(actual.as_deref(), Some(*expected), "input: {input:?}");
    }
}

pub fn assert_silent(rule: Rule, inputs: &[&str]) {
    for input in inputs {
        assert_eq!(rule(&parse_str(input)), None, "input: {input:?}");
    }
}

pub fn assert_explains(rule: Rule, cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        let rewrite = rule(&parse_str(input)).expect("input must be rewritten");
        assert_eq!(rewrite.explanation, *expected, "input: {input:?}");
    }
}

pub fn assert_stable(rule: Rule, inputs: &[&str]) {
    for input in inputs {
        let suggestion = rule(&parse_str(input))
            .expect("input must be rewritten")
            .pipeline;
        assert_eq!(
            parse_str(&render(&suggestion)),
            suggestion,
            "input: {input:?}"
        );
        assert_eq!(rule(&suggestion), None, "input: {input:?}");
    }
}
