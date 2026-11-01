mod grep_count;
mod sort_uniq;
mod useless_cat;

use crate::parser::Pipeline;

type Rule = fn(&Pipeline) -> Option<Pipeline>;

const RULES: &[Rule] = &[useless_cat::check, sort_uniq::check, grep_count::check];

pub fn rewrite(pipeline: &Pipeline) -> Option<Pipeline> {
    RULES.iter().find_map(|rule| rule(pipeline))
}
