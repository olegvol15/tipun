mod common;

use std::fs;

use common::{GIT_INIT, assert_same_git_state, fixtures, sh, suggest};

#[test]
fn bundled_git_commit_flags_commit_the_same() {
    let setup =
        "git init -q -b main && git add -A && git commit -q -m init && echo more >> app.log";
    let originals = [
        "git commit -a -m fix",
        "git commit -a -q -m 'fix bug'",
        "git commit -a -mfix",
    ];
    for (i, original) in originals.iter().enumerate() {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let before = fixtures(&format!("bundle-git-original-{i}"));
        let after = fixtures(&format!("bundle-git-suggestion-{i}"));
        let state = "git log --format='%H %s' && git status --porcelain";
        for (dir, command) in [(&before, *original), (&after, suggestion.as_str())] {
            assert_eq!(sh(dir, setup).status.code(), Some(0), "setup");
            assert_eq!(
                sh(dir, command).status.code(),
                Some(0),
                "command: {command:?}"
            );
        }
        assert_eq!(
            sh(&after, state).stdout,
            sh(&before, state).stdout,
            "{original:?} vs {suggestion:?}"
        );
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}

#[test]
fn git_switch_and_restore_match_checkout() {
    let remote = format!(
        "{GIT_INIT} && git remote add origin https://example.invalid/tipun.git \
         && git update-ref refs/remotes/origin/feature HEAD"
    );
    let changed = format!("{GIT_INIT} && echo changed >> app.log && echo x >> 'my file.txt'");
    let cases = [
        (GIT_INIT, "git checkout -b feature"),
        (GIT_INIT, "git checkout -B main"),
        (GIT_INIT, "git checkout -b feature main"),
        (remote.as_str(), "git checkout -b feature origin/feature"),
        (changed.as_str(), "git checkout -- app.log"),
        (changed.as_str(), "git checkout -- app.log 'my file.txt'"),
        (changed.as_str(), "git checkout -- ."),
    ];
    assert_same_git_state("git-checkout", &cases);
}

#[test]
fn git_stash_push_matches_stash_save() {
    let changed = format!("{GIT_INIT} && echo changed >> app.log");
    let cases = [
        (changed.as_str(), "git stash save"),
        (changed.as_str(), "git stash save wip"),
        (changed.as_str(), "git stash save 'fix the bug'"),
    ];
    assert_same_git_state("git-stash", &cases);
}

#[test]
fn merged_git_commands_change_the_repository_the_same() {
    let new_files = format!("{GIT_INIT} && echo one > new1 && echo two > 'new 2'");
    let branches = format!("{GIT_INIT} && git branch a && git branch b");
    let cases = [
        (new_files.as_str(), "git add new1 && git add 'new 2'"),
        (GIT_INIT, "git rm -q case.txt && git rm -q app.log"),
        (GIT_INIT, "git rm -r -q src && git rm -r -q numbers.txt"),
        (branches.as_str(), "git branch -d b && git branch -d a"),
    ];
    assert_same_git_state("merge-git", &cases);
}

#[test]
fn excluding_pathspecs_and_noisy_git_rm_are_not_merged() {
    let pairs = [
        (
            "git add new1 && git add new2 ':!new1'",
            "git add new1 new2 ':!new1'",
        ),
        (
            "git rm -q app.log && git rm -q case.txt ':!app.log'",
            "git rm -q app.log case.txt ':!app.log'",
        ),
        (
            "git rm case.txt && git rm app.log",
            "git rm case.txt app.log",
        ),
    ];
    let setup = format!("{GIT_INIT} && echo one > new1 && echo two > new2");
    let state = "git status --porcelain";
    for (i, (original, merged)) in pairs.iter().enumerate() {
        assert_eq!(suggest(original), None, "original: {original:?}");
        let (before, after) = (
            fixtures(&format!("merge-git-excluded-{i}")),
            fixtures(&format!("merge-git-merged-{i}")),
        );
        let mut results = Vec::new();
        for (dir, command) in [(&before, *original), (&after, *merged)] {
            assert_eq!(sh(dir, &setup).status.code(), Some(0), "setup");
            let output = sh(dir, command);
            assert_eq!(output.status.code(), Some(0), "command: {command:?}");
            results.push((output.stdout, sh(dir, state).stdout));
        }
        assert_ne!(results[0], results[1], "{original:?} vs {merged:?}");
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}
