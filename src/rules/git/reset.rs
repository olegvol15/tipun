use crate::lexer::Word;
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

// Documented exception: git restore needs git 2.23 or later, does not list the unstaged changes on
// stdout, and exits 1 for a path git does not know where reset exits 0. Last pipeline only.
pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    crate::rules::rewrite_each_stage(pipeline, rewrite)
}

fn rewrite(command: &Command) -> Option<(Command, String)> {
    if command.name.value != "git" {
        return None;
    }
    let [reset, rest @ ..] = command.args.as_slice() else {
        return None;
    };
    let rest = match rest {
        [quiet, rest @ ..] if quiet.value == "-q" => rest,
        rest => rest,
    };
    let [head, paths @ ..] = rest else {
        return None;
    };
    let paths = match paths {
        [separator, paths @ ..] if separator.value == "--" => paths,
        paths => paths,
    };
    if reset.value != "reset" || head.value != "HEAD" || paths.is_empty() {
        return None;
    }
    if !paths.iter().all(is_path) {
        return None;
    }
    let mut args = vec![word("restore"), word("--staged")];
    args.extend_from_slice(paths);
    let command = Command {
        name: command.name.clone(),
        args,
        redirects: command.redirects.clone(),
    };
    let explanation = "git restore --staged unstages files; reset also moves branches.";
    Some((command, explanation.to_string()))
}

fn is_path(word: &Word) -> bool {
    !word.value.is_empty() && !word.value.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{assert_explains, assert_rewrites, assert_silent, assert_stable};

    #[test]
    fn rewrites_reset_to_restore() {
        let cases = [
            ("git reset HEAD f", "git restore --staged f"),
            ("git reset HEAD -- f", "git restore --staged f"),
            ("git reset HEAD a b src/c", "git restore --staged a b src/c"),
            ("git reset HEAD .", "git restore --staged ."),
            ("git reset HEAD 'my file'", "git restore --staged 'my file'"),
            ("git reset -q HEAD f", "git restore --staged f"),
            ("git reset -q HEAD -- f", "git restore --staged f"),
            ("git reset HEAD f | cat", "git restore --staged f | cat"),
        ];
        assert_rewrites(check, &cases);
    }

    #[test]
    fn stays_silent_when_a_precondition_fails() {
        let cases = [
            "git reset",
            "git reset HEAD",
            "git reset HEAD --",
            "git reset f",
            "git reset -- f",
            "git reset ORIG_HEAD f",
            "git reset @ f",
            "git reset main f",
            "git reset --hard HEAD",
            "git reset --soft HEAD f",
            "git reset --mixed HEAD f",
            "git reset -p HEAD f",
            "git reset HEAD -p f",
            "git reset HEAD -q f",
            "git reset --quiet HEAD f",
            "git reset -q",
            "git reset HEAD -- -f",
            "git reset HEAD -f",
            "git reset HEAD f --",
            "git reset HEAD ''",
            "git -C dir reset HEAD f",
            "git restore --staged f",
            "sudo git reset HEAD f",
            "reset HEAD f",
        ];
        assert_silent(check, &cases);
    }

    #[test]
    fn explains_the_rewrite() {
        let cases = [(
            "git reset HEAD f",
            "git restore --staged unstages files; reset also moves branches.",
        )];
        assert_explains(check, &cases);
    }

    #[test]
    fn suggestion_parses_again_and_is_not_rewritten_twice() {
        assert_stable(check, &["git reset HEAD -- a 'b c'", "git reset -q HEAD f"]);
    }
}
