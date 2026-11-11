use crate::lexer::Word;
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

// Documented exception: git switch and git restore need git 2.23 or later, and git switch refuses
// to run during a merge, revert or rebase.
pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    crate::rules::rewrite_each_stage(pipeline, rewrite)
}

fn rewrite(command: &Command) -> Option<(Command, String)> {
    if command.name.value != "git" {
        return None;
    }
    let [checkout, option, rest @ ..] = command.args.as_slice() else {
        return None;
    };
    if checkout.value != "checkout" || !rest.iter().all(is_operand) {
        return None;
    }
    let (mut args, explanation) = match option.value.as_str() {
        "-b" | "-B" if (1..=2).contains(&rest.len()) => {
            let create = if option.value == "-b" { "-c" } else { "-C" };
            let explanation = format!(
                "git switch {create} creates and switches to a branch; checkout also restores files."
            );
            (vec![word("switch"), word(create)], explanation)
        }
        "--" if !rest.is_empty() => {
            let explanation =
                "git restore discards changes to files; checkout also switches branches.";
            (vec![word("restore")], explanation.to_string())
        }
        _ => return None,
    };
    args.extend_from_slice(rest);
    let command = Command {
        name: command.name.clone(),
        args,
        redirects: command.redirects.clone(),
    };
    Some((command, explanation))
}

fn is_operand(word: &Word) -> bool {
    !word.value.is_empty() && !word.value.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_checkout_to_switch_and_restore() {
        let cases = [
            ("git checkout -b feature", "git switch -c feature"),
            ("git checkout -B feature", "git switch -C feature"),
            ("git checkout -b feature main", "git switch -c feature main"),
            (
                "git checkout -b feature origin/feature",
                "git switch -c feature origin/feature",
            ),
            ("git checkout -b 'fix/a b'", "git switch -c 'fix/a b'"),
            ("git checkout -- app.log", "git restore app.log"),
            ("git checkout -- a b src/c", "git restore a b src/c"),
            ("git checkout -- .", "git restore ."),
            ("git checkout -- 'my file'", "git restore 'my file'"),
            (
                "git checkout -b x 2>/dev/null",
                "git switch -c x 2> /dev/null",
            ),
            ("git checkout -- f | cat", "git restore f | cat"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "git checkout main",
            "git checkout feature",
            "git checkout app.log",
            "git checkout",
            "git checkout -b",
            "git checkout -b a b c",
            "git checkout -bfeature",
            "git checkout -b -x",
            "git checkout -b ''",
            "git checkout -b feature --track",
            "git checkout -b feature -t origin/feature",
            "git checkout --track origin/feature",
            "git checkout -t origin/feature",
            "git checkout -q -b feature",
            "git checkout -f -b feature",
            "git checkout --orphan gh-pages",
            "git checkout -",
            "git checkout --",
            "git checkout -- -f",
            "git checkout -- a --",
            "git checkout HEAD -- app.log",
            "git checkout main -- app.log",
            "git checkout -p -- app.log",
            "git -C dir checkout -b feature",
            "git switch -c feature",
            "git restore app.log",
            "sudo git checkout -b feature",
            "checkout -b feature",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        let cases = [
            (
                "git checkout -b x",
                "git switch -c creates and switches to a branch; checkout also restores files.",
            ),
            (
                "git checkout -B x",
                "git switch -C creates and switches to a branch; checkout also restores files.",
            ),
            (
                "git checkout -- f",
                "git restore discards changes to files; checkout also switches branches.",
            ),
        ];
        assert_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        let inputs = ["git checkout -b 'fix/a b' main", "git checkout -- a 'b c'"];
        assert_stable(check, &inputs);
    }
}
