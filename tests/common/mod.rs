// Each test file uses only some of these helpers.
#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const APP_LOG: &str = "\
ERROR disk full
info started
error lower case
ERROR disk full
warn 10 retries
warn 2 retries
info started
";

pub const SPACED: &str = "b line\na line\nb line\n";

pub const MIXED_CASE: &str = "b\nA\na\nB\na\n";

pub const NUMBERS: &str = "1\n01\n2\n1\n";

pub fn fixtures(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tipun-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create fixture dir");
    fs::write(dir.join("app.log"), APP_LOG).expect("write app.log");
    fs::write(dir.join("my file.txt"), SPACED).expect("write my file.txt");
    fs::write(dir.join("case.txt"), MIXED_CASE).expect("write case.txt");
    fs::write(dir.join("numbers.txt"), NUMBERS).expect("write numbers.txt");
    fs::create_dir_all(dir.join("src")).expect("create src");
    fs::write(dir.join("src/main.txt"), SPACED).expect("write src/main.txt");
    dir
}

pub fn assert_same_output(dir: &Path, originals: &[&str]) {
    for original in originals {
        let Some(suggestion) = suggest(original) else {
            panic!("tipun must suggest a rewrite for {original:?}");
        };
        let expected = sh(dir, original);
        let actual = sh(dir, &suggestion);
        assert_eq!(expected.status.code(), Some(0), "original: {original:?}");
        assert_eq!(actual.status.code(), Some(0), "suggestion: {suggestion:?}");
        assert_eq!(
            actual.stdout, expected.stdout,
            "{original:?} vs {suggestion:?}"
        );
    }
}

pub fn assert_excluded_and_different(dir: &Path, pairs: &[(&str, &str)]) {
    for (original, rewrite) in pairs {
        assert_eq!(suggest(original), None, "original: {original:?}");
        assert_ne!(
            sh(dir, original).stdout,
            sh(dir, rewrite).stdout,
            "original: {original:?}"
        );
    }
}

pub fn sh(dir: &Path, command: &str) -> Output {
    Command::new("/bin/sh")
        .args(["-c", command])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_PREFIX")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "tipun")
        .env("GIT_AUTHOR_EMAIL", "tipun@example.com")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "tipun")
        .env("GIT_COMMITTER_EMAIL", "tipun@example.com")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .output()
        .expect("failed to run sh")
}

pub fn suggest(command: &str) -> Option<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_tipun"))
        .args(["suggest", command])
        .env_remove("TIPUN_DEBUG")
        .output()
        .expect("failed to run tipun");
    let stdout = String::from_utf8(output.stdout).expect("suggestion is UTF-8");
    output
        .status
        .success()
        .then(|| stdout.trim_end().to_string())
}

pub type Entry = (String, u32, Option<Vec<u8>>);

pub fn files(dir: &Path) -> Vec<Entry> {
    let mut found = Vec::new();
    collect_files(dir, dir, &mut found);
    found.sort();
    found
}

pub fn collect_files(root: &Path, dir: &Path, found: &mut Vec<Entry>) {
    for entry in fs::read_dir(dir).expect("read fixture dir") {
        let path = entry.expect("read dir entry").path();
        let name = path
            .strip_prefix(root)
            .expect("entry is under the root")
            .to_string_lossy()
            .into_owned();
        let mode = fs::metadata(&path)
            .expect("stat fixture entry")
            .permissions()
            .mode()
            & 0o7777;
        if path.is_dir() {
            found.push((name, mode, None));
            collect_files(root, &path, found);
        } else {
            let content = fs::read(&path).expect("read fixture file");
            found.push((name, mode, Some(content)));
        }
    }
}

pub fn assert_same_files(name: &str, originals: &[&str]) {
    for (i, original) in originals.iter().enumerate() {
        let Some(suggestion) = suggest(original) else {
            panic!("tipun must suggest a rewrite for {original:?}");
        };
        let before = fixtures(&format!("{name}-original-{i}"));
        let after = fixtures(&format!("{name}-suggestion-{i}"));
        let expected = sh(&before, original);
        let actual = sh(&after, &suggestion);
        assert_eq!(expected.status.code(), Some(0), "original: {original:?}");
        assert_eq!(actual.status.code(), Some(0), "suggestion: {suggestion:?}");
        assert_eq!(
            actual.stdout, expected.stdout,
            "{original:?} vs {suggestion:?}"
        );
        assert_eq!(
            files(&after),
            files(&before),
            "{original:?} vs {suggestion:?}"
        );
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}

pub const GIT_INIT: &str = "git init -q -b main && git add -A && git commit -q -m init";

pub const GIT_STATE: &str = "git branch --show-current; \
    git for-each-ref --format='%(refname) %(objectname) %(upstream)'; \
    git config --get-regexp '^branch[.]'; \
    git status --porcelain; \
    git stash list --format='%gs %H'";

pub fn worktree(dir: &Path) -> Vec<Entry> {
    files(dir)
        .into_iter()
        .filter(|(name, _, _)| !name.starts_with(".git"))
        .collect()
}

pub fn assert_same_git_state(name: &str, cases: &[(&str, &str)]) {
    for (i, (setup, original)) in cases.iter().enumerate() {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let before = fixtures(&format!("{name}-original-{i}"));
        let after = fixtures(&format!("{name}-suggestion-{i}"));
        let mut outputs = Vec::new();
        for (dir, command) in [(&before, *original), (&after, suggestion.as_str())] {
            assert_eq!(sh(dir, setup).status.code(), Some(0), "setup: {setup:?}");
            let output = sh(dir, command);
            assert_eq!(output.status.code(), Some(0), "command: {command:?}");
            outputs.push(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        assert_eq!(
            outputs[1], outputs[0],
            "stdout of {original:?} vs {suggestion:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&sh(&after, GIT_STATE).stdout),
            String::from_utf8_lossy(&sh(&before, GIT_STATE).stdout),
            "{original:?} vs {suggestion:?}"
        );
        assert_eq!(
            worktree(&after),
            worktree(&before),
            "{original:?} vs {suggestion:?}"
        );
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}
