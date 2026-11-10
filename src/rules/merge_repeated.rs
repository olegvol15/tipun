use crate::args::{self, Spec};
use crate::lexer::Word;
use crate::parser::{Chain, Command, Pipeline};
use crate::rules::Suggestion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Operands,
    ModeThenFiles,
    IntoDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kind {
    name: &'static str,
    skip: usize,
    flags: &'static str,
    with_value: &'static str,
    needs_option: bool,
    shape: Shape,
}

const fn kind(name: &'static str, skip: usize, flags: &'static str, shape: Shape) -> Kind {
    Kind {
        name,
        skip,
        flags,
        with_value: "",
        needs_option: false,
        shape,
    }
}

const TOUCH: Kind = kind("touch", 0, "acm", Shape::Operands);
const RM: Kind = kind("rm", 0, "dfiRrv", Shape::Operands);
const MKDIR: Kind = Kind {
    with_value: "m",
    ..kind("mkdir", 0, "pv", Shape::Operands)
};
const CHMOD: Kind = kind("chmod", 0, "", Shape::ModeThenFiles);
const CP: Kind = kind("cp", 0, "afinpRrv", Shape::IntoDirectory);
const MV: Kind = kind("mv", 0, "finv", Shape::IntoDirectory);
const GIT_ADD: Kind = kind("git add", 1, "", Shape::Operands);
const GIT_RM: Kind = kind("git rm", 1, "fqr", Shape::Operands);
const GIT_BRANCH: Kind = Kind {
    needs_option: true,
    ..kind("git branch", 1, "dD", Shape::Operands)
};

struct Parts<'a> {
    kind: Kind,
    command: &'a Command,
    options: &'a [Word],
    fixed: Option<&'a Word>,
    operands: Vec<&'a Word>,
}

pub fn check(chain: &Chain) -> Option<Suggestion> {
    let parts: Vec<Option<Parts>> = chain.pipelines.iter().map(parts).collect();
    let mut start = 0;
    while start < parts.len() {
        let Some(first) = &parts[start] else {
            start += 1;
            continue;
        };
        let end = start
            + parts[start..]
                .iter()
                .take_while(|other| other.as_ref().is_some_and(|other| same_call(first, other)))
                .count();
        let run: Vec<&Parts> = parts[start..end].iter().flatten().collect();
        if run.len() >= 2
            && let Some(merged) = merge(&run)
        {
            let mut pipelines = chain.pipelines[..start].to_vec();
            pipelines.push(merged);
            pipelines.extend_from_slice(&chain.pipelines[end..]);
            return Some(Suggestion {
                chain: Chain { pipelines },
                explanation: format!(
                    "{} accepts several operands, so one call does the work of {}.",
                    first.kind.name,
                    run.len()
                ),
            });
        }
        start = end;
    }
    None
}

fn parts(pipeline: &Pipeline) -> Option<Parts<'_>> {
    let [command] = pipeline.stages.as_slice() else {
        return None;
    };
    if !command.redirects.is_empty() {
        return None;
    }
    let sub = command.args.first().map(|word| word.value.as_str());
    let kind = match (command.name.value.as_str(), sub) {
        ("touch", _) => TOUCH,
        ("rm", _) => RM,
        ("mkdir", _) => MKDIR,
        ("chmod", _) => CHMOD,
        ("cp", _) => CP,
        ("mv", _) => MV,
        ("git", Some("add")) => GIT_ADD,
        ("git", Some("rm")) => GIT_RM,
        ("git", Some("branch")) => GIT_BRANCH,
        _ => return None,
    };
    let words = &command.args[kind.skip..];
    let spec = Spec {
        flags: kind.flags,
        with_value: kind.with_value,
    };
    let split = args::split(words, &spec)?;
    if kind.needs_option && split.flags.is_empty() {
        return None;
    }
    let options = &words[..words.len() - split.operands.len()];
    let mut operands = split.operands;
    let fixed = match kind.shape {
        Shape::Operands => None,
        Shape::ModeThenFiles if operands.is_empty() => return None,
        Shape::ModeThenFiles => Some(operands.remove(0)),
        Shape::IntoDirectory => {
            let target = operands.pop()?;
            if !target.value.ends_with('/') {
                return None;
            }
            Some(target)
        }
    };
    if operands.is_empty() {
        return None;
    }
    Some(Parts {
        kind,
        command,
        options,
        fixed,
        operands,
    })
}

fn same_call(a: &Parts, b: &Parts) -> bool {
    fn values<'a>(parts: &Parts<'a>) -> (Vec<&'a str>, Option<&'a str>) {
        let options = parts.options.iter().map(|w| w.value.as_str()).collect();
        (options, parts.fixed.map(|word| word.value.as_str()))
    }
    a.kind == b.kind && values(a) == values(b)
}

fn merge(run: &[&Parts]) -> Option<Pipeline> {
    let first = run[0];
    let operands: Vec<&Word> = run
        .iter()
        .flat_map(|parts| parts.operands.clone())
        .collect();
    if first.kind.shape == Shape::IntoDirectory && !distinct_sources(&operands) {
        return None;
    }
    let command = first.command;
    let mut args = command.args[..first.kind.skip].to_vec();
    args.extend_from_slice(first.options);
    let (before, after) = match first.kind.shape {
        Shape::Operands => (None, None),
        Shape::ModeThenFiles => (first.fixed, None),
        Shape::IntoDirectory => (None, first.fixed),
    };
    args.extend(before.cloned());
    args.extend(operands.into_iter().cloned());
    args.extend(after.cloned());
    Some(Pipeline {
        stages: vec![Command {
            name: command.name.clone(),
            args,
            redirects: Vec::new(),
        }],
    })
}

// One cp or mv call refuses to overwrite a file it has just created, and BSD cp copies the
// contents of `dir/` rather than the directory.
fn distinct_sources(sources: &[&Word]) -> bool {
    let mut names = Vec::new();
    for source in sources {
        if source.value.ends_with('/') {
            return false;
        }
        match super::file_name(&source.value) {
            Some(name) if name != ".." && !names.contains(&name) => names.push(name),
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{
        assert_chain_explains, assert_chain_rewrites, assert_chain_silent, assert_chain_stable,
    };

    #[test]
    fn merges_repeated_commands() {
        let cases = [
            ("touch a && touch b", "touch a b"),
            ("touch a && touch b && touch c", "touch a b c"),
            ("touch -c a && touch -c 'b c'", "touch -c a 'b c'"),
            ("rm -rf build && rm -rf dist", "rm -rf build dist"),
            ("rm a b && rm c", "rm a b c"),
            ("mkdir -p a/b && mkdir -p c", "mkdir -p a/b c"),
            ("mkdir -m 700 a && mkdir -m 700 b", "mkdir -m 700 a b"),
            ("chmod +x a.sh && chmod +x b.sh", "chmod +x a.sh b.sh"),
            ("chmod 644 a && chmod 644 b c", "chmod 644 a b c"),
            ("cp a dir/ && cp b dir/", "cp a b dir/"),
            (
                "cp -r src lib out/ && cp -r docs out/",
                "cp -r src lib docs out/",
            ),
            ("mv a.txt done/ && mv b.txt done/", "mv a.txt b.txt done/"),
            ("git add a && git add b", "git add a b"),
            ("git add src && git add 'my file'", "git add src 'my file'"),
            ("git rm -r old && git rm -r older", "git rm -r old older"),
            ("git branch -d a && git branch -d b", "git branch -d a b"),
            ("git branch -D a && git branch -D b", "git branch -D a b"),
            ("cd src && touch a && touch b", "cd src && touch a b"),
            ("touch a && touch b && ls", "touch a b && ls"),
            (
                "ls && touch a && touch b && cat f",
                "ls && touch a b && cat f",
            ),
            (
                "touch a && rm b && rm c && touch d",
                "touch a && rm b c && touch d",
            ),
            (
                "touch a && touch b && rm c && rm d",
                "touch a b && rm c && rm d",
            ),
        ];
        assert_chain_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "touch a",
            "touch a && rm b",
            "touch a && ls && touch b",
            "touch -c a && touch b",
            "touch a && touch -c b",
            "rm -r a && rm -f b",
            "mkdir -p a && mkdir b",
            "mkdir -m 700 a && mkdir -m 755 b",
            "chmod +x a && chmod -x b",
            "chmod +x a && chmod 755 b",
            "chmod -R 755 a && chmod -R 755 b",
            "chmod 755 a && chmod 755",
            "touch a > log && touch b",
            "touch a | cat && touch b",
            "touch a 2>&1 && touch b",
            "touch -t 202601010000 a && touch -t 202601010000 b",
            "touch -- a && touch -- b",
            "touch --no-create a && touch --no-create b",
            "rm -rf && rm -rf",
            "touch && touch",
            "git branch a && git branch b",
            "git branch -f a && git branch -f b",
            "git add -A && git add -A",
            "git add . && git commit -m x",
            "git rm --cached a && git rm --cached b",
            "git -C x add a && git -C x add b",
            "cp a dir && cp b dir",
            "cp a dir/ && cp b other/",
            "cp a dir/ && cp b ./dir/",
            "cp x/a dir/ && cp y/a dir/",
            "cp a dir/ && cp a dir/",
            "cp -r src/ out/ && cp -r lib out/",
            "cp -r .. out/ && cp a out/",
            "cp a && cp b",
            "cp dir/ && cp b dir/",
            "cp -r a d/ && cp b d/",
            "mv a dir && mv b dir",
            "mv x/a d/ && mv y/a d/",
            "sudo rm a && sudo rm b",
            "ls -l && ls -l",
            "chmod && chmod",
        ];
        assert_chain_silent(check, &cases);
    }

    #[test]
    fn explains_the_merge() {
        let cases = [
            (
                "touch a && touch b",
                "touch accepts several operands, so one call does the work of 2.",
            ),
            (
                "git add a && git add b && git add c",
                "git add accepts several operands, so one call does the work of 3.",
            ),
        ];
        assert_chain_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "touch a && touch 'b c'",
            "cp -r src out/ && cp -r docs out/",
            "chmod 644 a && chmod 644 b",
            "git branch -d a && git branch -d b",
        ];
        assert_chain_stable(check, &inputs);
    }
}
