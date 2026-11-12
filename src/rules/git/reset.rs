use crate::lexer::Word;
use crate::parser::{Command, Pipeline};
use crate::rules::{Rewrite, word};

// Documented exception: needs git 2.23+, does not list the unstaged changes, exits 1 for a path git
// does not know where reset exits 0, and runs the post-checkout hook. Last pipeline only.
pub fn check(pipeline: &Pipeline) -> Option<Rewrite> {
    let rewrite: fn(&Command) -> Option<(Command, String)> = match pipeline.stages.len() {
        1 => rewrite_alone,
        _ => rewrite_piped,
    };
    crate::rules::rewrite_each_stage(pipeline, rewrite)
}

// Without -q the listing is lost, which only reaches the terminal when nothing redirects stdout.
fn rewrite_alone(command: &Command) -> Option<(Command, String)> {
    unstage(command, command.redirects.is_empty())
}

fn rewrite_piped(command: &Command) -> Option<(Command, String)> {
    unstage(command, false)
}

fn unstage(command: &Command, listing_may_go: bool) -> Option<(Command, String)> {
    if command.name.value != "git" {
        return None;
    }
    let [reset, rest @ ..] = command.args.as_slice() else {
        return None;
    };
    let (quiet, rest) = match rest {
        [quiet, rest @ ..] if quiet.value == "-q" => (true, rest),
        rest => (false, rest),
    };
    let [head, paths @ ..] = rest else {
        return None;
    };
    // One unknown path among several makes restore change nothing. `HEAD -- f` is left out because
    // it also works before the first commit, where restore fails.
    let [path] = paths else {
        return None;
    };
    if reset.value != "reset" || head.value != "HEAD" || !is_path(path) {
        return None;
    }
    if !quiet && !listing_may_go {
        return None;
    }
    let args = vec![word("restore"), word("--staged"), path.clone()];
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
            ("git reset HEAD .", "git restore --staged ."),
            ("git reset HEAD 'my file'", "git restore --staged 'my file'"),
            ("git reset -q HEAD f", "git restore --staged f"),
            ("git reset HEAD src/c", "git restore --staged src/c"),
            ("git reset -q HEAD f | cat", "git restore --staged f | cat"),
            ("git reset -q HEAD f > log", "git restore --staged f > log"),
            ("ls | git reset -q HEAD f", "ls | git restore --staged f"),
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
            "git reset HEAD -- f",
            "git reset -q HEAD -- f",
            "git reset HEAD -f",
            "git reset HEAD f --",
            "git reset HEAD ''",
            "git reset HEAD f missing",
            "git reset HEAD -- a b",
            "git reset -q HEAD a b",
            "git reset HEAD f > report",
            "git reset HEAD f | tee report",
            "git reset HEAD f 2>&1",
            "ls | git reset HEAD f",
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
        assert_stable(
            check,
            &["git reset HEAD 'b c'", "git reset -q HEAD f > log"],
        );
    }
}
