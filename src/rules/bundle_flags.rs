use crate::args::{self, Spec};
use crate::parser::{Command, Pipeline};
use crate::rules::Rewrite;

const LS: Spec = Spec {
    flags: "1aAcdFGghilnoRrStu",
    with_value: "",
};
const RM: Spec = Spec {
    flags: "dfirRv",
    with_value: "",
};
const CP: Spec = Spec {
    flags: "afinpRrv",
    with_value: "",
};
const MV: Spec = Spec {
    flags: "finv",
    with_value: "",
};
const MKDIR: Spec = Spec {
    flags: "pv",
    with_value: "m",
};
const TOUCH: Spec = Spec {
    flags: "acm",
    with_value: "dtr",
};
const GREP: Spec = Spec {
    flags: "cinoqsvwxEF",
    with_value: "e",
};
const GIT_COMMIT: Spec = Spec {
    flags: "anqsv",
    with_value: "m",
};

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    super::rewrite_each_stage(pipeline, bundle)
}

fn bundle(command: &Command) -> Option<(Command, String)> {
    let (name, skip, spec) = match (command.name.value.as_str(), command.args.first()) {
        ("ls", _) => ("ls", 0, &LS),
        ("rm", _) => ("rm", 0, &RM),
        ("cp", _) => ("cp", 0, &CP),
        ("mv", _) => ("mv", 0, &MV),
        ("mkdir", _) => ("mkdir", 0, &MKDIR),
        ("touch", _) => ("touch", 0, &TOUCH),
        ("grep", _) => ("grep", 0, &GREP),
        ("git", Some(sub)) if sub.value == "commit" => ("git commit", 1, &GIT_COMMIT),
        _ => return None,
    };
    let words = &command.args[skip..];
    args::split(words, spec)?;
    let groups: Vec<&str> = words
        .iter()
        .map(|word| word.value.as_str())
        .take_while(|value| value.starts_with('-'))
        .collect();
    let [before_last @ .., _] = groups.as_slice() else {
        return None;
    };
    let takes_value = |group: &&str| group.contains(|c| spec.with_value.contains(c));
    if before_last.is_empty() || before_last.iter().any(takes_value) {
        return None;
    }
    if words[..groups.len()]
        .iter()
        .any(|word| word.raw != word.value)
    {
        return None;
    }
    let cluster: String = groups.iter().map(|group| &group[1..]).collect();
    let merged = format!("-{cluster}");
    let explanation = format!("{name} accepts {} bundled as {merged}.", groups.join(" "));
    let mut args = command.args[..skip].to_vec();
    args.push(super::word(&merged));
    args.extend_from_slice(&words[groups.len()..]);
    Some((
        Command {
            name: command.name.clone(),
            args,
            redirects: command.redirects.clone(),
        },
        explanation,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn bundles_separate_short_flags() {
        let cases = [
            ("ls -l -a", "ls -la"),
            ("ls -l -a -h", "ls -lah"),
            ("ls -la -h", "ls -lah"),
            ("ls -l -a src", "ls -la src"),
            ("ls -1 -t", "ls -1t"),
            ("rm -r -f build", "rm -rf build"),
            ("rm -r -f a 'b c'", "rm -rf a 'b c'"),
            ("cp -r -v src dst", "cp -rv src dst"),
            ("mv -i -v a b", "mv -iv a b"),
            ("mkdir -p -v a/b", "mkdir -pv a/b"),
            ("mkdir -p -m 700 a", "mkdir -pm 700 a"),
            ("mkdir -p -m700 a", "mkdir -pm700 a"),
            ("touch -a -c f", "touch -ac f"),
            ("grep -i -v x f", "grep -iv x f"),
            ("grep -i -e x f", "grep -ie x f"),
            ("git commit -a -m fix", "git commit -am fix"),
            ("git commit -a -m 'fix bug'", "git commit -am 'fix bug'"),
            ("git commit -a -mfix", "git commit -amfix"),
            ("grep -i -ex f", "grep -iex f"),
            ("git commit -s -a", "git commit -sa"),
            ("ls -l -a | grep x", "ls -la | grep x"),
            ("ls -l -a > out", "ls -la > out"),
            ("ls | grep -i -v x", "ls | grep -iv x"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "ls",
            "ls -la",
            "ls -l",
            "ls -l src",
            "ls src -l -a",
            "ls -l src -a",
            "ls -l --all",
            "ls --color -l -a",
            "ls -l -- -a",
            "ls -l -",
            "ls -l -I x",
            "ls -l -w 80",
            "rm -r -P x",
            "rm -r --force x",
            "mkdir -m 700 -p a",
            "mkdir -m700 -p a",
            "touch -t 202601010000 -c f",
            "grep -e x -i f",
            "grep -n5 -i x f",
            "git commit -m fix -a",
            "git commit -m a -m b",
            "git commit -a -m'fix bug'",
            "grep -i -e'a b' f",
            "ls -l '-a'",
            "git commit --amend -a",
            "git -C dir commit -a -m x",
            "git add -A -v",
            "git",
            "chmod -x -v f",
            "sudo rm -r -f x",
            "tar -x -v -f a.tar",
            "cat -n -b f",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_bundle() {
        let cases = [
            ("ls -l -a", "ls accepts -l -a bundled as -la."),
            (
                "git commit -a -m x",
                "git commit accepts -a -m bundled as -am.",
            ),
        ];
        assert_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = [
            "ls -l -a",
            "git commit -a -m 'fix bug'",
            "mkdir -p -m 700 a",
        ];
        assert_stable(check, &inputs);
    }
}
