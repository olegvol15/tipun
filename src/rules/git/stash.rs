use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    crate::rules::rewrite_each_stage(pipeline, rewrite)
}

fn rewrite(command: &Command) -> Option<(Command, String)> {
    if command.name.value != "git" {
        return None;
    }
    let [stash, save, message @ ..] = command.args.as_slice() else {
        return None;
    };
    if stash.value != "stash" || save.value != "save" {
        return None;
    }
    let mut args = vec![stash.clone()];
    match message {
        [] => {}
        [message] if !message.value.is_empty() && !message.value.starts_with('-') => {
            args.extend([word("push"), word("-m"), message.clone()]);
        }
        _ => return None,
    }
    let command = Command {
        name: command.name.clone(),
        args,
        redirects: command.redirects.clone(),
    };
    let explanation = "git stash save is deprecated; git stash push does the same.";
    Some((command, explanation.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_stash_save_to_push() {
        let cases = [
            ("git stash save", "git stash"),
            ("git stash save wip", "git stash push -m wip"),
            (
                "git stash save 'fix the bug'",
                "git stash push -m 'fix the bug'",
            ),
            (
                "git stash save \"wip: parser\"",
                "git stash push -m \"wip: parser\"",
            ),
            ("git stash save wip > out", "git stash push -m wip > out"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "git stash",
            "git stash push -m wip",
            "git stash pop",
            "git stash save fix the bug",
            "git stash save -u wip",
            "git stash save -k",
            "git stash save --include-untracked wip",
            "git stash save ''",
            "git stash save -- wip",
            "git -C dir stash save wip",
            "git save wip",
            "sudo git stash save wip",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        assert_stable(check, &["git stash save", "git stash save 'fix the bug'"]);
    }
}
