use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APP_LOG: &str = "\
ERROR disk full
info started
error lower case
ERROR disk full
warn 10 retries
warn 2 retries
info started
";

const SPACED: &str = "b line\na line\nb line\n";

fn fixtures(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tipun-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create fixture dir");
    fs::write(dir.join("app.log"), APP_LOG).expect("write app.log");
    fs::write(dir.join("my file.txt"), SPACED).expect("write my file.txt");
    dir
}

fn sh(dir: &Path, command: &str) -> Output {
    Command::new("/bin/sh")
        .args(["-c", command])
        .current_dir(dir)
        .output()
        .expect("failed to run sh")
}

fn suggest(command: &str) -> Option<String> {
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
    ];
    for original in originals {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        let expected = sh(&dir, original);
        let actual = sh(&dir, &suggestion);
        assert_eq!(expected.status.code(), Some(0), "original: {original:?}");
        assert_eq!(actual.status.code(), Some(0), "suggestion: {suggestion:?}");
        assert_eq!(
            actual.stdout, expected.stdout,
            "{original:?} vs {suggestion:?}"
        );
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn excluded_flags_really_change_the_output() {
    let dir = fixtures("useless-cat-excluded");
    let pairs = [
        ("cat app.log | grep -H ERROR", "grep -H ERROR app.log"),
        ("cat app.log | grep -l ERROR", "grep -l ERROR app.log"),
    ];
    for (original, rewrite) in pairs {
        assert_eq!(suggest(original), None, "original: {original:?}");
        assert_ne!(
            sh(&dir, original).stdout,
            sh(&dir, rewrite).stdout,
            "original: {original:?}"
        );
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}
