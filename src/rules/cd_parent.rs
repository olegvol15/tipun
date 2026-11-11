use crate::parser::{Chain, Command, Pipeline};
use crate::rules::Suggestion;

// Documented exception to the equivalence contract: OLDPWD (`cd -`), the AUTO_PUSHD directory
// stack and chpwd hooks see one directory change instead of several. OLDPWD is exported, so the
// run must end the chain, where no later command in the line can read it.
pub fn check(chain: &Chain, usable: usize) -> Option<Suggestion> {
    let pipelines = &chain.pipelines;
    if usable < pipelines.len() {
        return None;
    }
    let start = pipelines
        .iter()
        .rposition(|pipeline| levels(pipeline).is_none())
        .map_or(0, |i| i + 1);
    let run = &pipelines[start..];
    if run.len() < 2 {
        return None;
    }
    let total: usize = run.iter().filter_map(levels).sum();
    let path = vec![".."; total].join("/");
    let first = &run[0].stages[0];
    let mut merged = pipelines[..start].to_vec();
    merged.push(Pipeline {
        stages: vec![Command {
            name: first.name.clone(),
            args: vec![super::word(&path)],
            redirects: Vec::new(),
        }],
    });
    Some(Suggestion {
        chain: Chain { pipelines: merged },
        explanation: format!("cd {path} climbs the same {total} directories in one step."),
    })
}

fn levels(pipeline: &Pipeline) -> Option<usize> {
    let [command] = pipeline.stages.as_slice() else {
        return None;
    };
    let [path] = command.args.as_slice() else {
        return None;
    };
    if command.name.value != "cd" || !command.redirects.is_empty() {
        return None;
    }
    let path = path.value.strip_suffix('/').unwrap_or(&path.value);
    let parts: Vec<&str> = path.split('/').collect();
    parts
        .iter()
        .all(|part| *part == "..")
        .then_some(parts.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{
        assert_chain_explains, assert_chain_rewrites, assert_chain_silent, assert_chain_stable,
    };

    #[test]
    fn climbs_in_one_step() {
        let cases = [
            ("cd .. && cd ..", "cd ../.."),
            ("cd .. && cd .. && cd ..", "cd ../../.."),
            ("cd ../ && cd ..", "cd ../.."),
            ("cd ../.. && cd ..", "cd ../../.."),
            ("cd .. && cd ../../", "cd ../../.."),
            ("cd '..' && cd \"..\"", "cd ../.."),
            ("cd src && cd .. && cd ..", "cd src && cd ../.."),
            ("ls && cd .. && cd ..", "ls && cd ../.."),
            ("cd .. && ls && cd .. && cd ..", "cd .. && ls && cd ../.."),
        ];
        assert_chain_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "cd ..",
            "cd ../..",
            "cd .. && ls",
            "cd .. && cd .. && ls",
            "cd .. && cd .. && pwd",
            "cd .. && cd .. && cd -",
            "cd .. && cd src",
            "cd src && cd ..",
            "cd . && cd ..",
            "cd ./.. && cd ..",
            "cd ..// && cd ..",
            "cd ../src && cd ..",
            "cd ... && cd ..",
            "cd .. && cd",
            "cd - && cd ..",
            "cd -P .. && cd -P ..",
            "cd .. .. && cd ..",
            "cd .. > log && cd ..",
            "cd .. | cat && cd ..",
            "pushd .. && pushd ..",
            "builtin cd .. && builtin cd ..",
        ];
        assert_chain_silent(check, &cases);
    }

    #[test]
    fn explains_the_climb() {
        let cases = [(
            "cd .. && cd ..",
            "cd ../.. climbs the same 2 directories in one step.",
        )];
        assert_chain_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        assert_chain_stable(check, &["cd .. && cd ..", "ls && cd ../ && cd .."]);
    }
}
