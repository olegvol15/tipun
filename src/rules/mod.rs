mod useless_cat;

use crate::parser::Pipeline;

type Rule = fn(&Pipeline) -> Option<Pipeline>;

const RULES: &[Rule] = &[useless_cat::check];

pub fn rewrite(pipeline: &Pipeline) -> Option<Pipeline> {
    RULES.iter().find_map(|rule| rule(pipeline))
}
