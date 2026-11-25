mod bundle_flags;
mod cd_parent;
mod git;
mod merge_repeated;
mod mkdir_parents;
#[cfg(test)]
mod testing;
mod text;

use crate::lexer::{Redirect, Word};
use crate::parser::{Chain, Command, Pipeline, Redirection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub chain: Chain,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub pipeline: Pipeline,
    pub explanation: String,
}

type Rule = fn(&Pipeline) -> Option<Rewrite>;
// A chain rule may only rewrite the first `usable` pipelines, but it sees the whole chain.
type ChainRule = fn(chain: &Chain, usable: usize) -> Option<Suggestion>;

const CHAIN_RULES: &[ChainRule] = &[
    mkdir_parents::check,
    merge_repeated::check,
    cd_parent::check,
    git::branch_switch::check,
];

const RULES: &[Rule] = &[git::stash::check, bundle_flags::check];

// These may change the exit status, which decides whether the rest of an `&&` chain runs. Merging
// stages does: `cat missing | sort` exits 0 with sort's status, `sort missing` exits 2.
const LAST_PIPELINE_RULES: &[Rule] = &[
    git::checkout::check,
    git::reset::check,
    text::useless_cat::check,
    text::sort_uniq::check,
    text::grep_count::check,
    text::grep_head::check,
    text::grep_v_merge::check,
];

// Builtins that change how later commands run (`exec > out` redirects everything after it), and
// prefixes that run a builtin in this shell (`time exec`). Rules only see their own pipeline, so
// they stop at the first one.
const SHELL_STATE_BUILTINS: &[&str] = &[
    "-",
    ".",
    "alias",
    "autoload",
    "builtin",
    "command",
    "coproc",
    "declare",
    "disable",
    "emulate",
    "enable",
    "eval",
    "exec",
    "export",
    "fc",
    "float",
    "functions",
    "hash",
    "integer",
    "limit",
    "local",
    "nocorrect",
    "noglob",
    "r",
    "readonly",
    "repeat",
    "rehash",
    "set",
    "setopt",
    "shopt",
    "source",
    "time",
    "trap",
    "typeset",
    "ulimit",
    "umask",
    "unalias",
    "unfunction",
    "unhash",
    "unlimit",
    "unset",
    "unsetopt",
    "zmodload",
];

pub fn rewrite(chain: &Chain) -> Option<Suggestion> {
    let last = chain.pipelines.len().saturating_sub(1);
    let unchanged_state = chain
        .pipelines
        .iter()
        .position(changes_shell_state)
        .unwrap_or(chain.pipelines.len());
    if let Some(suggestion) = CHAIN_RULES
        .iter()
        .find_map(|rule| rule(chain, unchanged_state))
    {
        return Some(suggestion);
    }
    chain.pipelines[..unchanged_state]
        .iter()
        .enumerate()
        .find_map(|(i, pipeline)| {
            let last_only = LAST_PIPELINE_RULES.iter().filter(|_| i == last);
            let rewrite = RULES
                .iter()
                .chain(last_only)
                .find_map(|rule| rule(pipeline))?;
            let mut pipelines = chain.pipelines.clone();
            pipelines[i] = rewrite.pipeline;
            Some(Suggestion {
                chain: Chain { pipelines },
                explanation: rewrite.explanation,
            })
        })
}

fn changes_shell_state(pipeline: &Pipeline) -> bool {
    pipeline
        .stages
        .iter()
        .any(|stage| SHELL_STATE_BUILTINS.contains(&stage.name.value.as_str()))
}

fn rewrite_each_stage(
    pipeline: &Pipeline,
    rewrite: fn(&Command) -> Option<(Command, String)>,
) -> Option<Rewrite> {
    pipeline.stages.iter().enumerate().find_map(|(i, stage)| {
        let (command, explanation) = rewrite(stage)?;
        let mut stages = pipeline.stages.clone();
        stages[i] = command;
        Some(Rewrite {
            pipeline: Pipeline { stages },
            explanation,
        })
    })
}

// Replaces stages `i` and `i + 1` with one command that takes over the redirects of stage `i + 1`.
fn absorb(pipeline: &Pipeline, i: usize, args: Vec<Word>, explanation: &str) -> Option<Rewrite> {
    let [first, second] = pipeline.stages.get(i..=i + 1)? else {
        return None;
    };
    if !first.redirects.is_empty()
        || reads_redirected_input(second)
        || !only_harmless_redirects(pipeline)
    {
        return None;
    }
    let merged = Command {
        name: first.name.clone(),
        args,
        redirects: second.redirects.clone(),
    };
    let mut stages = pipeline.stages.clone();
    stages.splice(i..=i + 1, [merged]);
    Some(Rewrite {
        pipeline: Pipeline { stages },
        explanation: explanation.to_string(),
    })
}

fn word(text: &str) -> Word {
    Word {
        raw: text.to_string(),
        value: text.to_string(),
    }
}

// The shell truncates an output file before the pipeline reads anything, so a target that may
// be one of the pipeline's files makes both forms lose data in different ways. Without touching
// the disk the check can only compare names: symlinks and hard links are not detected.
fn only_harmless_redirects(pipeline: &Pipeline) -> bool {
    let inputs = pipeline.stages.iter().flat_map(|stage| {
        let args = stage.args.iter();
        let redirects = stage
            .redirects
            .iter()
            .filter_map(|redirect| match redirect.op {
                Redirect::In => redirect.target.as_ref(),
                Redirect::Out { .. } | Redirect::Append { .. } | Redirect::Dup { .. } => None,
            });
        args.chain(redirects)
    });
    let names: Vec<&str> = inputs.filter_map(|word| file_name(&word.value)).collect();
    pipeline
        .stages
        .iter()
        .flat_map(|stage| &stage.redirects)
        .all(|redirect| is_harmless(redirect, &names))
}

fn reads_redirected_input(command: &Command) -> bool {
    command
        .redirects
        .iter()
        .any(|redirect| redirect.op == Redirect::In)
}

// A stage merged into grep hands its stderr redirect to grep, whose warnings then land there.
fn redirects_stderr(command: &Command) -> bool {
    command.redirects.iter().any(|redirect| match redirect.op {
        Redirect::Out { fd } | Redirect::Append { fd } => {
            fd == Some(2)
                && redirect
                    .target
                    .as_ref()
                    .is_none_or(|target| target.value != "/dev/null")
        }
        Redirect::Dup { fd, .. } => fd == 2,
        Redirect::In => false,
    })
}

fn is_harmless(redirect: &Redirection, names: &[&str]) -> bool {
    let fd_is_output = |fd: Option<u8>| matches!(fd, None | Some(1 | 2));
    match redirect.op {
        Redirect::In => return true,
        Redirect::Out { fd } | Redirect::Append { fd } if fd_is_output(fd) => {}
        Redirect::Dup { fd, to } if fd_is_output(Some(fd)) && fd_is_output(Some(to)) => {}
        Redirect::Out { .. } | Redirect::Append { .. } | Redirect::Dup { .. } => return false,
    }
    let Some(target) = &redirect.target else {
        return true;
    };
    if target.value == "/dev/null" {
        return true;
    }
    if is_process_dependent(&target.value) || target.value.split('/').any(|part| part == "..") {
        return false;
    }
    file_name(&target.value).is_some_and(|name| !names.contains(&name))
}

fn file_name(path: &str) -> Option<&str> {
    path.split('/')
        .rfind(|part| !part.is_empty() && *part != ".")
}

// Entries like /proc/self/status describe whichever process opens them, so moving the read from
// one command to another changes their content. `..` is rejected rather than collapsed, because
// collapsing it is only correct without symlinks.
fn is_process_dependent(path: &str) -> bool {
    let Some(rest) = path.strip_prefix('/') else {
        return false;
    };
    let parts: Vec<&str> = rest
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    matches!(parts.first(), Some(&("proc" | "dev"))) || parts.contains(&"..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render;
    use crate::rules::testing::{parse_chain, parse_str};

    #[test]
    fn rewrites_the_first_matching_pipeline_of_a_chain() {
        let cases = [
            ("cd src && cat f | grep x", "cd src && grep x f"),
            (
                "cat f | grep x && cat g | grep y",
                "cat f | grep x && grep y g",
            ),
            ("ls && sort f | uniq > out", "ls && sort -u f > out"),
            ("cd src && grep x f | wc -l", "cd src && grep -c x f"),
            (
                "grep x f | wc -l && cat f | grep y",
                "grep x f | wc -l && grep y f",
            ),
            ("ls -l -a && cat f | grep x", "ls -la && cat f | grep x"),
            ("ls && git checkout -b x", "ls && git switch -c x"),
            ("ls && git reset HEAD f", "ls && git restore --staged f"),
            ("ls -l -a && exec > out", "ls -la && exec > out"),
            ("echo exec && cat f | grep x", "echo exec && grep x f"),
            (
                "touch a && touch b && exec > out && touch c && touch d",
                "touch a b && exec > out && touch c && touch d",
            ),
            (
                "touch a && touch b && cat f | grep x",
                "touch a b && cat f | grep x",
            ),
            ("ls && cd .. && cd ..", "ls && cd ../.."),
            ("mkdir -p a && mkdir -p a/b", "mkdir -p a/b"),
            ("mkdir -p a && mkdir -p b", "mkdir -p a b"),
            (
                "ls && git branch x && git checkout x",
                "ls && git switch -c x",
            ),
        ];
        for (input, expected) in cases {
            let suggestion = rewrite(&parse_chain(input)).expect("input must be rewritten");
            assert_eq!(render(&suggestion.chain), expected, "input: {input:?}");
        }
        let silent = [
            "cd src && ls",
            "ls && cd ..",
            "grep x f | wc -l && touch marker",
            "cat f | grep x && ls",
            "cat missing | sort && touch marker",
            "sort missing | uniq && echo continued",
            "fc -e - exec && grep x f | wc -l",
            "r exec && cat f | grep x",
            "exec > out && touch a && touch b",
            "touch a && setopt x && touch b",
            "cd .. && cd .. && exec > out",
            "git checkout -b x && touch marker",
            "git reset HEAD f && touch marker",
            "git checkout -- f && ls",
            "git branch x && git checkout x && exec > out",
            "setopt auto_pushd && cd .. && cd ..",
            "exec > out && git branch x && git checkout x",
            "grep x f | wc -l && ls && cd ..",
            "exec > out && grep x f | wc -l",
            "exec > out && cat f | grep x",
            "builtin exec > out && cat f | grep x",
            "time exec > out && grep x f | wc -l",
            "repeat 1 exec > out && cat f | grep x",
            "- exec > out && cat f | grep x",
            "setopt sh_word_split && cat f | grep x",
            "export LC_ALL=C && sort f | uniq",
            "cd src && alias cat=bat && cat f | grep x",
            "ls | exec cat > out && cat f | grep x",
        ];
        for input in silent {
            assert_eq!(rewrite(&parse_chain(input)), None, "input: {input:?}");
        }
    }

    #[test]
    fn accepts_output_redirects_to_other_files() {
        let cases = [
            "grep x f",
            "grep x f > out",
            "grep x f >> out",
            "grep x f 1> out",
            "grep x f 2> err",
            "grep x f 2>> err",
            "grep x f 2>/dev/null",
            "grep x f > /dev/null",
            "grep x f 2>&1",
            "grep x f 1>&2",
            "grep x f > out 2>&1",
            "grep x f > logs/out",
            "grep x f > /tmp/out",
            "grep x f > 'my out'",
            "sort f | uniq > out",
            "grep x f > out | sort > sorted",
            "grep x < f",
            "grep x f < g > out",
            "tr a b < f | sort | uniq > out",
        ];
        for input in cases {
            assert!(
                only_harmless_redirects(&parse_str(input)),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn finds_input_redirects() {
        let cases = [
            ("grep x f", false),
            ("grep x f > out", false),
            ("grep x f 2>&1", false),
            ("grep x < f", true),
            ("grep x > out < f", true),
        ];
        for (input, expected) in cases {
            let pipeline = parse_str(input);
            assert_eq!(
                reads_redirected_input(&pipeline.stages[0]),
                expected,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn rejects_redirects_that_change_meaning() {
        let cases = [
            "grep x < f > f",
            "grep x f < g | sort > ./g",
            "grep x f 0> out",
            "grep x f 3> out",
            "grep x f 0>&2",
            "grep x f 2>&0",
            "grep x f 3>&1",
            "grep x f 1>&3",
            "grep x f > f",
            "grep x f >> f",
            "grep x f 2> f",
            "grep x f > ./f",
            "grep x f > dir/f",
            "grep x dir/f > f",
            "grep x f > f/",
            "grep x f > ../out",
            "grep x f > logs/../out",
            "grep x f > ..",
            "grep x f > /",
            "grep x f > /dev/stdin",
            "grep x f > /dev/stdout",
            "grep x f > /dev/fd/0",
            "grep x f > /proc/self/fd/0",
            "grep x f > //dev/null",
            "sort f | uniq > f",
            "grep x f | sort > f",
            "grep out f > out",
        ];
        for input in cases {
            assert!(
                !only_harmless_redirects(&parse_str(input)),
                "input: {input:?}"
            );
        }
    }

    fn readme_examples() -> Vec<(String, String)> {
        let mut examples = Vec::new();
        for row in include_str!("../../README.md").lines() {
            if !row.starts_with("| `") {
                continue;
            }
            let cells: Vec<Vec<String>> = row
                .replace("\\|", "\u{1}")
                .split('|')
                .map(|cell| {
                    cell.split('`')
                        .skip(1)
                        .step_by(2)
                        .map(|code| code.replace('\u{1}', "|"))
                        .collect()
                })
                .collect();
            let [_, typed, suggested, _] = cells.as_slice() else {
                panic!("README row has two cells: {row}");
            };
            assert_eq!(typed.len(), suggested.len(), "README row: {row}");
            examples.extend(typed.iter().cloned().zip(suggested.iter().cloned()));
        }
        examples
    }

    #[test]
    fn readme_examples_get_their_suggestions() {
        let examples = readme_examples();
        assert!(examples.len() > 10, "README table not found");
        for (typed, suggested) in examples {
            let suggestion = rewrite(&parse_chain(&typed)).map(|s| render(&s.chain));
            assert_eq!(suggestion.as_deref(), Some(suggested.as_str()), "{typed:?}");
        }
    }

    #[test]
    fn every_rule_has_a_readme_example() {
        let examples = readme_examples();
        let fires = |apply: &dyn Fn(&Chain) -> Option<Chain>| {
            examples.iter().any(|(typed, suggested)| {
                apply(&parse_chain(typed)).is_some_and(|chain| render(&chain) == *suggested)
            })
        };
        for (i, rule) in CHAIN_RULES.iter().enumerate() {
            let apply = |chain: &Chain| rule(chain, chain.pipelines.len()).map(|s| s.chain);
            assert!(fires(&apply), "CHAIN_RULES[{i}] has no README example");
        }
        for (i, rule) in RULES.iter().chain(LAST_PIPELINE_RULES).enumerate() {
            let apply = |chain: &Chain| match chain.pipelines.as_slice() {
                [pipeline] => rule(pipeline).map(|rewrite| Chain {
                    pipelines: vec![rewrite.pipeline],
                }),
                _ => None,
            };
            assert!(fires(&apply), "pipeline rule {i} has no README example");
        }
    }
}
