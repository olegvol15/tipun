mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::{
    assert_excluded_and_different, assert_same_files, assert_same_output, fixtures, sh, suggest,
};

#[test]
fn useless_cat_suggestions_match_the_original() {
    let dir = fixtures("useless-cat");
    let originals = [
        "cat app.log | grep ERROR",
        "cat app.log | grep -ci error",
        "cat app.log | grep -n -v info",
        "cat app.log | grep -e warn -e ERROR",
        "cat app.log | grep -o 'disk full'",
        "cat 'my file.txt' | grep -w line",
        "cat app.log | head -n 2",
        "cat app.log | head -n3",
        "cat app.log | head -c 20",
        "cat app.log | tail -n 2",
        "cat app.log | tail -n +3",
        "cat app.log | tail -c 10",
        "cat app.log | sort",
        "cat app.log | sort -rn",
        "cat app.log | sort -t ' ' -k 2",
        "cat app.log | sort | uniq -c",
        "cat 'my file.txt' | sort -u",
        "cat app.log | uniq -c",
        "cat app.log | uniq -i -f 1",
        "cat app.log | grep warn | sort | head -n 1",
        "cat app.log | wc -l",
        "cat app.log | wc -w",
        "cat app.log | wc -c",
        "cat app.log | wc -m",
        "cat 'my file.txt' | wc -l",
        "cat app.log | grep ERROR | wc -l",
        "cat app.log | tr a-z A-Z",
        "cat app.log | tr -d 0-9",
        "cat app.log | tr -s ' '",
        "cat app.log | tr -cd 'a-z\\n'",
        "cat app.log | tr ' ' '\\n' | sort | uniq -c",
    ];
    assert_same_output(&dir, &originals);
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn suggestions_with_redirects_write_the_same_files() {
    let originals = [
        "cat app.log | grep ERROR > errors.txt",
        "cat app.log | grep ERROR >> errors.txt",
        "cat app.log | grep ERROR 2>/dev/null",
        "cat app.log | grep ERROR > errors.txt 2>&1",
        "cat app.log | grep warn | sort > sorted.txt",
        "cat app.log | grep ERROR > errors.txt | wc -c",
        "cat app.log | head -n 2 > 'top lines.txt'",
        "cat app.log | wc -l > count.txt",
        "cat app.log | tr a-z A-Z 2> err.txt",
        "cat app.log | grep ERROR | tr a-z A-Z < case.txt",
        "sort app.log | uniq > unique.txt",
        "sort app.log | uniq >> unique.txt 2>&1",
        "sort app.log | uniq | head -n 2 > top.txt",
        "tr ' ' '\\n' < app.log | sort | uniq > words.txt",
    ];
    assert_same_files("redirect", &originals);
}

#[test]
fn excluded_flags_really_change_the_output() {
    let dir = fixtures("useless-cat-excluded");
    let pairs = [
        ("cat app.log | grep -H ERROR", "grep -H ERROR app.log"),
        ("cat app.log | grep -l ERROR", "grep -l ERROR app.log"),
    ];
    assert_excluded_and_different(&dir, &pairs);
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn sort_uniq_suggestions_match_the_original() {
    let dir = fixtures("sort-uniq");
    let originals = [
        "sort app.log | uniq",
        "sort -r app.log | uniq",
        "sort -u app.log | uniq",
        "sort app.log 'my file.txt' | uniq",
        "sort case.txt | uniq",
        "sort numbers.txt | uniq",
        "grep -v info app.log | sort | uniq",
        "tr ' ' '\\n' < app.log | sort | uniq | wc -l",
    ];
    assert_same_output(&dir, &originals);
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn sort_flags_that_change_equality_really_change_the_output() {
    let dir = fixtures("sort-uniq-excluded");
    let pairs = [
        ("sort -f case.txt | uniq", "sort -u -f case.txt"),
        ("sort -n numbers.txt | uniq", "sort -u -n numbers.txt"),
    ];
    assert_excluded_and_different(&dir, &pairs);
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[cfg(target_os = "linux")]
#[test]
fn gnu_wc_pads_several_counts_differently_for_a_file() {
    let dir = fixtures("wc-padding");
    assert_eq!(suggest("cat app.log | wc"), None);
    assert_ne!(
        sh(&dir, "cat app.log | wc").stdout,
        sh(&dir, "wc < app.log").stdout
    );
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

fn count(dir: &Path, command: &str) -> (String, Option<i32>) {
    let output = sh(dir, command);
    let stdout = String::from_utf8(output.stdout).expect("count is UTF-8");
    let unpadded = stdout.trim_start_matches(' ');
    (unpadded.to_string(), output.status.code())
}

#[test]
fn grep_count_suggestions_match_the_original_count() {
    let dir = fixtures("grep-count");
    let originals = [
        "grep ERROR app.log | wc -l",
        "grep -i error app.log | wc -l",
        "grep -v info app.log | wc -l",
        "grep -w line 'my file.txt' | wc -l",
        "grep -x a case.txt | wc -l",
        "grep -E 'warn|info' app.log | wc -l",
        "grep -F 'disk full' app.log | wc -l",
        "grep -e warn -e ERROR app.log | wc -l",
        "grep '' app.log | wc -l",
        "sort app.log | grep info | wc -l",
        "grep ERROR app.log | wc -l 2>/dev/null",
    ];
    for original in originals {
        let Some(suggestion) = suggest(original) else {
            panic!("tipun must suggest a rewrite for {original:?}");
        };
        let expected = count(&dir, original);
        assert_eq!(expected.1, Some(0), "original: {original:?}");
        assert_eq!(
            count(&dir, &suggestion),
            expected,
            "{original:?} vs {suggestion:?}"
        );
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn grep_count_without_matches_differs_only_in_exit_status() {
    let dir = fixtures("grep-count-no-match");
    let original = "grep missing app.log | wc -l";
    let suggestion = suggest(original).expect("tipun must suggest a rewrite");
    assert_eq!(count(&dir, original), ("0\n".to_string(), Some(0)));
    assert_eq!(count(&dir, &suggestion), ("0\n".to_string(), Some(1)));
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn grep_flags_that_change_what_is_printed_really_change_the_count() {
    let dir = fixtures("grep-count-excluded");
    let pairs = [
        ("grep -o r app.log | wc -l", "grep -c -o r app.log"),
        (
            "grep -A 1 ERROR app.log | wc -l",
            "grep -c -A 1 ERROR app.log",
        ),
        (
            "grep ERROR app.log 'my file.txt' | wc -l",
            "grep -c ERROR app.log 'my file.txt'",
        ),
    ];
    for (original, rewrite) in pairs {
        assert_eq!(suggest(original), None, "original: {original:?}");
        assert_ne!(
            count(&dir, original).0,
            count(&dir, rewrite).0,
            "original: {original:?}"
        );
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

const OH_MY_ZSH_GREP: &str =
    "grep --color=auto --exclude-dir={.bzr,CVS,.git,.hg,.svn,.idea,.tox,.venv,venv}";

fn with_grep_alias(command: &str) -> String {
    command.replace("grep ", &format!("{OH_MY_ZSH_GREP} "))
}

fn zsh(dir: &Path, command: &str) -> Vec<u8> {
    let output = Command::new("zsh")
        .args(["-f", "-c", command])
        .current_dir(dir)
        .output()
        .expect("zsh must be installed to run this test");
    assert_eq!(output.status.code(), Some(0), "command: {command:?}");
    output.stdout
}

fn zsh_on_terminal(dir: &Path, command: &str) -> Vec<u8> {
    let mut script = Command::new("script");
    if cfg!(target_os = "macos") {
        script.args(["-q", "/dev/null", "zsh", "-f", "-c", "eval \"$TIPUN_CMD\""]);
    } else {
        script.args([
            "-q",
            "-e",
            "-c",
            "zsh -f -c 'eval \"$TIPUN_CMD\"'",
            "/dev/null",
        ]);
    }
    let output = script
        .env("TIPUN_CMD", command)
        .env("TERM", "xterm-256color")
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("failed to run script");
    assert_eq!(output.status.code(), Some(0), "command: {command:?}");
    output.stdout
}

#[test]
fn harmless_grep_alias_options_keep_suggestions_equivalent() {
    let dir = fixtures("grep-alias");
    let originals = [
        ("cat app.log | grep ERROR", true),
        ("cat app.log | grep -v info", false),
        ("cat 'my file.txt' | grep -w line", true),
        ("cat app.log | grep -e warn -e ERROR | sort", false),
    ];
    for (original, colored_on_terminal) in originals {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let (original, suggestion) = (with_grep_alias(original), with_grep_alias(&suggestion));
        assert_eq!(
            zsh(&dir, &suggestion),
            zsh(&dir, &original),
            "{original:?} vs {suggestion:?}"
        );
        let colored = zsh_on_terminal(&dir, &original);
        assert_eq!(
            colored.windows(2).any(|pair| pair == b"\x1b["),
            colored_on_terminal,
            "color on a terminal: {original:?}"
        );
        assert_eq!(
            zsh_on_terminal(&dir, &suggestion),
            colored,
            "{original:?} vs {suggestion:?}"
        );
    }
    let counts = [
        "grep ERROR app.log | wc -l",
        "grep -i error app.log | wc -l",
    ];
    for original in counts {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let (original, suggestion) = (with_grep_alias(original), with_grep_alias(&suggestion));
        let unpadded = |output: Vec<u8>| {
            let text = String::from_utf8(output).expect("count is UTF-8");
            text.replace(' ', "")
        };
        assert_eq!(
            unpadded(zsh(&dir, &suggestion)),
            unpadded(zsh(&dir, &original)),
            "{original:?} vs {suggestion:?}"
        );
        assert_eq!(
            unpadded(zsh_on_terminal(&dir, &suggestion)),
            unpadded(zsh_on_terminal(&dir, &original)),
            "{original:?} vs {suggestion:?}"
        );
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn rewrites_inside_and_chains_match_the_original() {
    let originals = [
        "cd src && cat main.txt | grep line",
        "cat app.log | grep ERROR > errors.txt && wc -l < errors.txt",
        "mkdir out && cat app.log | sort | uniq > out/sorted.txt",
        "touch new.txt && ls -l -a src && cat app.log | wc -l",
        "cat app.log | grep warn && cat 'my file.txt' | grep line",
    ];
    assert_same_files("chain", &originals);
}
