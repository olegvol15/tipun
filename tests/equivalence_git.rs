mod common;

use std::fs;

use common::{GIT_INIT, GIT_STATE, assert_same_git_state, fixtures, sh, suggest, worktree};

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

#[test]
fn git_switch_c_creates_and_switches_like_branch_and_checkout() {
    let detached = format!("{GIT_INIT} && git checkout -q --detach");
    let upstream = format!(
        "{GIT_INIT} && git remote add origin https://example.invalid/tipun.git \
         && git update-ref refs/remotes/origin/main HEAD && git branch -q -u origin/main"
    );
    let cases = [
        (GIT_INIT, "git branch feature && git checkout feature"),
        (GIT_INIT, "git branch fix/a && git switch fix/a"),
        (
            detached.as_str(),
            "git branch feature && git checkout feature",
        ),
        (
            upstream.as_str(),
            "git branch feature && git checkout feature",
        ),
    ];
    assert_same_git_state("branch-switch", &cases);
}

#[test]
fn git_switch_c_leaves_a_dirty_tree_the_same_but_does_not_list_it() {
    let (original, suggestion) = (
        "git branch feature && git checkout feature",
        "git switch -c feature",
    );
    assert_eq!(suggest(original).as_deref(), Some(suggestion));
    let setup = format!("{GIT_INIT} && echo more >> app.log");
    let (before, after) = (
        fixtures("branch-switch-dirty-original"),
        fixtures("branch-switch-dirty-suggestion"),
    );
    let mut outputs = Vec::new();
    for (dir, command) in [(&before, original), (&after, suggestion)] {
        assert_eq!(sh(dir, &setup).status.code(), Some(0), "setup");
        let output = sh(dir, command);
        assert_eq!(output.status.code(), Some(0), "command: {command:?}");
        outputs.push(output.stdout);
    }
    assert_eq!(outputs, [b"M\tapp.log\n".to_vec(), Vec::new()]);
    assert_eq!(sh(&after, GIT_STATE).stdout, sh(&before, GIT_STATE).stdout);
    assert_eq!(worktree(&after), worktree(&before));
    fs::remove_dir_all(before).expect("remove fixture dir");
    fs::remove_dir_all(after).expect("remove fixture dir");
}

#[test]
fn branches_from_a_tracked_start_point_are_not_switched() {
    let (original, switched) = (
        "git branch feature origin/feature && git checkout feature",
        "git switch -c feature origin/feature",
    );
    assert_eq!(suggest(original), None);
    let setup = format!(
        "{GIT_INIT} && git remote add origin https://example.invalid/tipun.git \
         && git update-ref refs/remotes/origin/feature HEAD"
    );
    let (before, after) = (
        fixtures("branch-switch-start-original"),
        fixtures("branch-switch-start-switched"),
    );
    let mut outputs = Vec::new();
    for (dir, command) in [(&before, original), (&after, switched)] {
        assert_eq!(sh(dir, &setup).status.code(), Some(0), "setup");
        let output = sh(dir, command);
        assert_eq!(output.status.code(), Some(0), "command: {command:?}");
        outputs.push(output.stdout);
    }
    assert_ne!(outputs[0], outputs[1]);
    fs::remove_dir_all(before).expect("remove fixture dir");
    fs::remove_dir_all(after).expect("remove fixture dir");
}

#[test]
fn git_switch_refuses_during_an_unfinished_merge() {
    let setup = format!(
        "{GIT_INIT} && git checkout -q -b other && echo b > new && git add new \
         && git commit -q -m b && git checkout -q main && git merge -q --no-commit --no-ff other"
    );
    let originals = [
        "git branch feature && git checkout feature",
        "git checkout -b feature",
    ];
    for chained in [
        "git branch feature && git checkout feature && touch marker",
        "git checkout -b feature && touch marker",
    ] {
        assert_eq!(suggest(chained), None, "{chained:?} would skip touch");
    }
    for (i, original) in originals.iter().enumerate() {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let (before, after) = (
            fixtures(&format!("switch-merging-original-{i}")),
            fixtures(&format!("switch-merging-suggestion-{i}")),
        );
        let mut codes = Vec::new();
        for (dir, command) in [(&before, *original), (&after, suggestion.as_str())] {
            assert_eq!(sh(dir, &setup).status.code(), Some(0), "setup");
            codes.push(sh(dir, command).status.code());
        }
        assert_eq!(
            codes,
            [Some(0), Some(128)],
            "{original:?} vs {suggestion:?}"
        );
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}

#[test]
fn git_restore_staged_unstages_like_reset() {
    let staged =
        format!("{GIT_INIT} && echo more >> app.log && echo x >> 'my file.txt' && git add -A");
    let new_file = format!("{GIT_INIT} && echo n > new && git add new");
    let conflict = format!(
        "{GIT_INIT} && git checkout -q -b other && echo o > case.txt && git commit -q -am o \
         && git checkout -q main && echo m > case.txt && git commit -q -am m \
         && {{ git merge -q other > /dev/null 2>&1 || true; }}"
    );
    let cases = [
        (staged.as_str(), "git reset -q HEAD app.log"),
        (
            staged.as_str(),
            "git reset -q HEAD -- app.log 'my file.txt'",
        ),
        (staged.as_str(), "git reset -q HEAD ."),
        (new_file.as_str(), "git reset HEAD new"),
        (new_file.as_str(), "git reset HEAD -- new"),
        (conflict.as_str(), "git reset -q HEAD case.txt"),
    ];
    assert_same_git_state("reset-restore", &cases);
}

#[test]
fn git_restore_staged_does_not_list_the_unstaged_changes() {
    let (original, suggestion) = ("git reset HEAD app.log", "git restore --staged app.log");
    assert_eq!(suggest(original).as_deref(), Some(suggestion));
    let setup = format!("{GIT_INIT} && echo more >> app.log && git add app.log");
    let (before, after) = (
        fixtures("reset-restore-listing-original"),
        fixtures("reset-restore-listing-suggestion"),
    );
    let mut outputs = Vec::new();
    for (dir, command) in [(&before, original), (&after, suggestion)] {
        assert_eq!(sh(dir, &setup).status.code(), Some(0), "setup");
        let output = sh(dir, command);
        assert_eq!(output.status.code(), Some(0), "command: {command:?}");
        outputs.push(output.stdout);
    }
    assert_eq!(
        outputs,
        [
            b"Unstaged changes after reset:\nM\tapp.log\n".to_vec(),
            Vec::new()
        ]
    );
    assert_eq!(sh(&after, GIT_STATE).stdout, sh(&before, GIT_STATE).stdout);
    assert_eq!(worktree(&after), worktree(&before));
    fs::remove_dir_all(before).expect("remove fixture dir");
    fs::remove_dir_all(after).expect("remove fixture dir");
}

#[test]
fn git_restore_staged_fails_on_unknown_paths_so_it_must_end_the_chain() {
    assert_eq!(suggest("git reset HEAD missing && touch marker"), None);
    let (original, suggestion) = ("git reset HEAD missing", "git restore --staged missing");
    assert_eq!(suggest(original).as_deref(), Some(suggestion));
    let (before, after) = (
        fixtures("reset-restore-unknown-original"),
        fixtures("reset-restore-unknown-suggestion"),
    );
    let mut codes = Vec::new();
    for (dir, command) in [(&before, original), (&after, suggestion)] {
        assert_eq!(sh(dir, GIT_INIT).status.code(), Some(0), "setup");
        codes.push(sh(dir, command).status.code());
    }
    assert_eq!(codes, [Some(0), Some(1)], "{original:?} vs {suggestion:?}");
    fs::remove_dir_all(before).expect("remove fixture dir");
    fs::remove_dir_all(after).expect("remove fixture dir");
}
