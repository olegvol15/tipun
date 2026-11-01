mod grep_count;
mod sort_uniq;
mod useless_cat;

use crate::parser::Pipeline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub pipeline: Pipeline,
    pub explanation: String,
}

type Rule = fn(&Pipeline) -> Option<Rewrite>;

const RULES: &[Rule] = &[useless_cat::check, sort_uniq::check, grep_count::check];

pub fn rewrite(pipeline: &Pipeline) -> Option<Rewrite> {
    RULES.iter().find_map(|rule| rule(pipeline))
}
