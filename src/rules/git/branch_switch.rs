use crate::lexer::Word;
use crate::parser::{Chain, Command, Pipeline};
use crate::rules::{Suggestion, word};

// Documented exception: needs git 2.23+ and refuses during a merge, revert or rebase. It checks out
// no tree, so stdout skips modified files and upstream status; the reflog says "Created from HEAD".
pub fn check(chain: &Chain, usable: usize) -> Option<Suggestion> {
    let pipelines = &chain.pipelines;
    if usable < pipelines.len() || pipelines.len() < 2 {
        return None;
    }
    // A refused switch would skip the rest of the chain, so the pair must end it.
    let i = pipelines.len() - 2;
    let name = git(&pipelines[i], &["branch"])?;
    let switched = git(&pipelines[i + 1], &["checkout", "switch"])?;
    if name.value != switched.value {
        return None;
    }
    let command = &pipelines[i].stages[0];
    let switch = Command {
        name: command.name.clone(),
        args: vec![word("switch"), word("-c"), name.clone()],
        redirects: Vec::new(),
    };
    let mut merged = pipelines[..i].to_vec();
    merged.push(Pipeline {
        stages: vec![switch],
    });
    Some(Suggestion {
        chain: Chain { pipelines: merged },
        explanation: "git switch -c creates the branch and switches to it in one step.".to_string(),
    })
}

fn git<'a>(pipeline: &'a Pipeline, subcommands: &[&str]) -> Option<&'a Word> {
    let [command] = pipeline.stages.as_slice() else {
        return None;
    };
    let [subcommand, name] = command.args.as_slice() else {
        return None;
    };
    let named = !name.value.is_empty() && !name.value.starts_with('-');
    (command.name.value == "git"
        && command.redirects.is_empty()
        && subcommands.contains(&subcommand.value.as_str())
        && named)
        .then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{
        assert_chain_explains, assert_chain_rewrites, assert_chain_silent, assert_chain_stable,
    };

    #[test]
    fn creates_and_switches_in_one_step() {
        let cases = [
            ("git branch x && git checkout x", "git switch -c x"),
            ("git branch x && git switch x", "git switch -c x"),
            (
                "git branch fix/a && git checkout fix/a",
                "git switch -c fix/a",
            ),
            (
                "git branch 'a b' && git checkout \"a b\"",
                "git switch -c 'a b'",
            ),
            (
                "ls && git branch x && git checkout x",
                "ls && git switch -c x",
            ),
            (
                "git branch x && git checkout x && git branch y && git checkout y",
                "git branch x && git checkout x && git switch -c y",
            ),
        ];
        assert_chain_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "git branch x",
            "git checkout x",
            "git branch x && git checkout y",
            "git branch x && git checkout x && touch marker",
            "git branch x && ls && git checkout x",
            "git checkout x && git branch x",
            "git branch x main && git checkout x",
            "git branch x origin/x && git checkout x",
            "git branch -f x && git checkout x",
            "git branch -d x && git checkout x",
            "git branch x && git checkout -q x",
            "git branch x && git checkout x -- f",
            "git branch x && git checkout -f x",
            "git branch x && git switch -d x",
            "git branch -x && git checkout -x",
            "git branch '' && git checkout ''",
            "git branch x > log && git checkout x",
            "git branch x && git checkout x 2>&1",
            "git branch x | cat && git checkout x",
            "git -C dir branch x && git -C dir checkout x",
            "git branch x && git restore x",
            "sudo git branch x && sudo git checkout x",
            "hg branch x && hg checkout x",
        ];
        assert_chain_silent(check, &cases);
    }

    #[test]
    fn explains_the_switch() {
        let cases = [(
            "git branch x && git checkout x",
            "git switch -c creates the branch and switches to it in one step.",
        )];
        assert_chain_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "git branch 'a b' && git checkout 'a b'",
            "ls && git branch x && git switch x",
        ];
        assert_chain_stable(check, &inputs);
    }
}
