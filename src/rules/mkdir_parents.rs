use crate::lexer::Word;
use crate::parser::{Chain, Command, Pipeline};
use crate::rules::{Suggestion, word};

pub fn check(chain: &Chain, usable: usize) -> Option<Suggestion> {
    let paths: Vec<Option<&Word>> = chain.pipelines[..usable].iter().map(path).collect();
    let mut start = 0;
    while start < paths.len() {
        let mut end = start + 1;
        while end < paths.len() && extends(paths[end - 1], paths[end]) {
            end += 1;
        }
        if end - start >= 2 {
            let deepest = paths[end - 1]?;
            let name = &chain.pipelines[start].stages[0].name;
            let mut pipelines = chain.pipelines[..start].to_vec();
            pipelines.push(Pipeline {
                stages: vec![Command {
                    name: name.clone(),
                    args: vec![word("-p"), deepest.clone()],
                    redirects: Vec::new(),
                }],
            });
            pipelines.extend_from_slice(&chain.pipelines[end..]);
            return Some(Suggestion {
                chain: Chain { pipelines },
                explanation: "mkdir -p creates the missing parent directories itself.".to_string(),
            });
        }
        start = end;
    }
    None
}

fn path(pipeline: &Pipeline) -> Option<&Word> {
    let [command] = pipeline.stages.as_slice() else {
        return None;
    };
    if command.name.value != "mkdir" || !command.redirects.is_empty() {
        return None;
    }
    let path = match command.args.as_slice() {
        [flag, path] if flag.value == "-p" => path,
        [path] => path,
        _ => return None,
    };
    let relative = path.value.strip_prefix('/').unwrap_or(&path.value);
    let clean = relative
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != ".." && !part.starts_with('-'));
    clean.then_some(path)
}

fn extends(parent: Option<&Word>, child: Option<&Word>) -> bool {
    let (Some(parent), Some(child)) = (parent, child) else {
        return false;
    };
    child
        .value
        .strip_prefix(&parent.value)
        .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{
        assert_chain_explains, assert_chain_rewrites, assert_chain_silent, assert_chain_stable,
    };

    #[test]
    fn creates_the_deepest_directory_with_its_parents() {
        let cases = [
            ("mkdir -p a && mkdir -p a/b", "mkdir -p a/b"),
            ("mkdir a && mkdir a/b", "mkdir -p a/b"),
            ("mkdir a && mkdir -p a/b", "mkdir -p a/b"),
            (
                "mkdir -p a && mkdir -p a/b && mkdir -p a/b/c",
                "mkdir -p a/b/c",
            ),
            ("mkdir src && mkdir src/x/y", "mkdir -p src/x/y"),
            (
                "mkdir 'my dir' && mkdir 'my dir/sub'",
                "mkdir -p 'my dir/sub'",
            ),
            ("mkdir /tmp/a && mkdir /tmp/a/b", "mkdir -p /tmp/a/b"),
            (
                "ls && mkdir a && mkdir a/b && ls",
                "ls && mkdir -p a/b && ls",
            ),
            ("mkdir x && mkdir a && mkdir a/b", "mkdir x && mkdir -p a/b"),
        ];
        assert_chain_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "mkdir a",
            "mkdir -p a/b",
            "mkdir a && mkdir b",
            "mkdir a && mkdir ab",
            "mkdir a/b && mkdir a",
            "mkdir a && ls && mkdir a/b",
            "mkdir a && mkdir a/",
            "mkdir a/ && mkdir a/b",
            "mkdir a && mkdir a//b",
            "mkdir a && mkdir a/./b",
            "mkdir a && mkdir a/../b",
            "mkdir . && mkdir ./b",
            "mkdir a && mkdir a/-b",
            "mkdir -v a && mkdir -v a/b",
            "mkdir -m 700 a && mkdir -m 700 a/b",
            "mkdir -pv a && mkdir -pv a/b",
            "mkdir a b && mkdir a/c",
            "mkdir -p a && mkdir -p a/b c",
            "mkdir a > log && mkdir a/b",
            "mkdir a | cat && mkdir a/b",
            "mkdir / && mkdir /a",
            "sudo mkdir a && sudo mkdir a/b",
            "mkdir -- a && mkdir -- a/b",
        ];
        assert_chain_silent(check, &cases);
    }

    #[test]
    fn explains_the_parents() {
        let cases = [(
            "mkdir a && mkdir a/b",
            "mkdir -p creates the missing parent directories itself.",
        )];
        assert_chain_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "mkdir 'my dir' && mkdir 'my dir/sub'",
            "ls && mkdir a && mkdir a/b",
        ];
        assert_chain_stable(check, &inputs);
    }
}
