use crate::lexer::Word;
use crate::parser::{Chain, Command, Pipeline};
use crate::rules::{Suggestion, word};

pub fn check(chain: &Chain, usable: usize) -> Option<Suggestion> {
    let pipelines = &chain.pipelines[..usable];
    let i = (0..pipelines.len().saturating_sub(1)).find(|&i| {
        let apply = stash(&pipelines[i], "apply");
        apply.is_some() && apply == stash(&pipelines[i + 1], "drop")
    })?;
    let command = &pipelines[i].stages[0];
    let mut args = vec![command.args[0].clone(), word("pop")];
    args.extend_from_slice(&command.args[2..]);
    let mut merged = chain.pipelines[..i].to_vec();
    merged.push(Pipeline {
        stages: vec![Command {
            name: command.name.clone(),
            args,
            redirects: Vec::new(),
        }],
    });
    merged.extend_from_slice(&chain.pipelines[i + 2..]);
    Some(Suggestion {
        chain: Chain { pipelines: merged },
        explanation: "git stash pop applies the stash and drops it in one step.".to_string(),
    })
}

// The values after `apply` or `drop`: an optional -q and an optional stash number.
fn stash<'a>(pipeline: &'a Pipeline, action: &str) -> Option<Vec<&'a str>> {
    let [command] = pipeline.stages.as_slice() else {
        return None;
    };
    if command.name.value != "git" || !command.redirects.is_empty() {
        return None;
    }
    let [stash, verb, rest @ ..] = command.args.as_slice() else {
        return None;
    };
    if stash.value != "stash" || verb.value != action {
        return None;
    }
    let rest: Vec<&str> = rest.iter().map(|word: &Word| word.value.as_str()).collect();
    let valid = match rest.as_slice() {
        [] | ["-q"] => true,
        [number] | ["-q", number] => is_number(number),
        _ => false,
    };
    valid.then_some(rest)
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{
        assert_chain_explains, assert_chain_rewrites, assert_chain_silent, assert_chain_stable,
    };

    #[test]
    fn pops_instead_of_applying_and_dropping() {
        let cases = [
            ("git stash apply && git stash drop", "git stash pop"),
            ("git stash apply 1 && git stash drop 1", "git stash pop 1"),
            (
                "git stash apply -q && git stash drop -q",
                "git stash pop -q",
            ),
            (
                "git stash apply -q 2 && git stash drop -q 2",
                "git stash pop -q 2",
            ),
            (
                "ls && git stash apply && git stash drop && ls",
                "ls && git stash pop && ls",
            ),
            (
                "git stash apply && git stash drop && git status",
                "git stash pop && git status",
            ),
        ];
        assert_chain_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "git stash apply",
            "git stash drop",
            "git stash drop && git stash apply",
            "git stash apply && ls && git stash drop",
            "git stash apply 1 && git stash drop",
            "git stash apply && git stash drop 1",
            "git stash apply 1 && git stash drop 2",
            "git stash apply -q && git stash drop",
            "git stash apply && git stash drop -q",
            "git stash apply --index && git stash drop",
            "git stash apply --quiet && git stash drop --quiet",
            "git stash apply 1 -q && git stash drop 1 -q",
            "git stash apply x && git stash drop x",
            "git stash apply '' && git stash drop ''",
            "git stash apply > out && git stash drop",
            "git stash apply && git stash drop 2> err",
            "git stash apply | cat && git stash drop",
            "git -C dir stash apply && git -C dir stash drop",
            "sudo git stash apply && sudo git stash drop",
            "git stash pop",
        ];
        assert_chain_silent(check, &cases);
    }

    #[test]
    fn explains_the_pop() {
        let cases = [(
            "git stash apply && git stash drop",
            "git stash pop applies the stash and drops it in one step.",
        )];
        assert_chain_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "git stash apply && git stash drop",
            "ls && git stash apply -q 1 && git stash drop -q 1",
        ];
        assert_chain_stable(check, &inputs);
    }
}
