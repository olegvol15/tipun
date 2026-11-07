use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type Vars<'a> = &'a [(&'a str, &'a str)];

fn tipun(args: &[&str]) -> Output {
    tipun_with_env(args, &[])
}

fn tipun_with_env(args: &[&str], vars: Vars) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tipun"))
        .args(args)
        .env_remove("TIPUN_DEBUG")
        .env_remove("TIPUN_DISABLE")
        .env_remove("CI")
        .env("TIPUN_NO_UPDATE", "1")
        .envs(vars.iter().copied())
        .output()
        .expect("failed to run tipun")
}

fn state_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tipun-state-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create state dir");
    dir
}

fn tipun_with_updates(args: &[&str], state: &Path, vars: Vars) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tipun"))
        .args(args)
        .env_remove("TIPUN_DEBUG")
        .env_remove("TIPUN_DISABLE")
        .env_remove("TIPUN_NO_UPDATE")
        .env_remove("CI")
        .env("TIPUN_STATE_DIR", state)
        .envs(vars.iter().copied())
        .output()
        .expect("failed to run tipun")
}

#[test]
fn analyze_records_the_daily_update_check() {
    let state = state_dir("check");
    let output = tipun_with_updates(&["analyze", "--command", "ls"], &state, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(state.join("last-update-check").exists());
    fs::remove_dir_all(state).expect("remove state dir");
}

#[test]
fn analyze_skips_the_update_check_when_disabled() {
    let cases: [Vars; 3] = [
        &[("TIPUN_NO_UPDATE", "1")],
        &[("CI", "true")],
        &[("TIPUN_DISABLE", "1")],
    ];
    for vars in cases {
        let state = state_dir("disabled");
        let output = tipun_with_updates(&["analyze", "--command", "ls"], &state, vars);
        assert_eq!(output.status.code(), Some(0), "env: {vars:?}");
        assert!(!state.join("last-update-check").exists(), "env: {vars:?}");
        fs::remove_dir_all(state).expect("remove state dir");
    }
}

#[test]
fn analyze_prints_the_update_notice_once() {
    let state = state_dir("notice");
    fs::write(state.join("last-update-check"), "").expect("write stamp");
    fs::write(state.join("updated"), "v9.9.9\n").expect("write notice");
    let first = tipun_with_updates(&["analyze", "--command", "ls"], &state, &[]);
    assert_eq!(
        String::from_utf8_lossy(&first.stderr),
        "tipun updated to v9.9.9\n"
    );
    assert!(!state.join("updated").exists());
    let second = tipun_with_updates(&["analyze", "--command", "ls"], &state, &[]);
    assert!(second.stderr.is_empty());
    fs::remove_dir_all(state).expect("remove state dir");
}

#[test]
fn update_refuses_copies_not_installed_by_the_script() {
    let state = state_dir("refuse");
    let elsewhere = state.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("create dir");
    for recorded in [None, Some(&elsewhere)] {
        if let Some(dir) = recorded {
            fs::write(state.join("install-path"), dir.display().to_string()).expect("write");
        }
        let output = tipun_with_updates(&["update"], &state, &[]);
        assert_eq!(output.status.code(), Some(1), "recorded: {recorded:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "tipun was not installed by install.sh; update it the way you installed it\n"
        );
        let quiet = tipun_with_updates(&["update", "--quiet"], &state, &[]);
        assert_eq!(quiet.status.code(), Some(1), "recorded: {recorded:?}");
        assert!(quiet.stderr.is_empty() && quiet.stdout.is_empty());
    }
    fs::remove_dir_all(state).expect("remove state dir");
}

#[test]
fn suggest_prints_the_rewrite() {
    let output = tipun(&["suggest", "cat 'my file' | grep ERROR"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"grep ERROR 'my file'\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn suggest_is_silent_without_a_rewrite() {
    for command in ["ls | grep x", "echo $HOME", "", "FOO=1 cat f", "ls >"] {
        let output = tipun(&["suggest", command]);
        assert_eq!(output.status.code(), Some(1), "command: {command:?}");
        assert!(output.stdout.is_empty(), "command: {command:?}");
        assert!(output.stderr.is_empty(), "command: {command:?}");
    }
}

#[test]
fn analyze_prints_the_tip_on_stderr() {
    let cases: [&[&str]; 6] = [
        &["analyze", "--command", "cat app.log | grep ERROR"],
        &[
            "analyze",
            "--command=cat app.log | grep ERROR",
            "--expanded=cat app.log | grep --color=auto --exclude-dir={.bzr,CVS,.git,.hg,.svn,.idea,.tox,.venv,venv} ERROR",
        ],
        &[
            "analyze",
            "--command=cat  app.log|grep ERROR",
            "--expanded=cat app.log | grep 'ERROR'",
        ],
        &[
            "analyze",
            "--command=cat app.log | grep ERROR",
            "--expanded=cat app.log | grep ERROR",
        ],
        &[
            "analyze",
            "--command",
            "cat app.log | grep ERROR",
            "--exit-code",
            "0",
        ],
        &["analyze", "--command=cat app.log | grep ERROR"],
    ];
    for args in cases {
        let output = tipun(args);
        assert_eq!(output.status.code(), Some(0), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "tipun tip: grep ERROR app.log\n  grep can read the file itself, no need for cat.\n",
            "args: {args:?}"
        );
    }
}

#[test]
fn analyze_is_silent_when_aliases_change_the_command() {
    let command = "--command=cat app.log | grep ERROR";
    for expanded in [
        "--expanded=bat app.log | grep ERROR",
        "--expanded=cat app.log | grep --color=always ERROR",
        "--expanded=cat app.log | grep -v ERROR",
        "--expanded=cat app.log | grep ERROR | less",
        "--expanded=cat app.log | grep ERROR > out",
        "--expanded=cat $LOG | grep ERROR",
        "--expanded=",
    ] {
        let output = tipun(&["analyze", command, expanded]);
        assert_eq!(output.status.code(), Some(0), "{expanded:?}");
        assert!(output.stdout.is_empty(), "{expanded:?}");
        assert!(output.stderr.is_empty(), "{expanded:?}");
    }
}

#[test]
fn init_zsh_prints_the_hooks() {
    let output = tipun(&["init", "zsh"]);
    assert_eq!(output.status.code(), Some(0));
    let script = String::from_utf8_lossy(&output.stdout);
    assert!(script.contains("add-zsh-hook preexec _tipun_preexec"));
    assert!(script.contains("add-zsh-hook precmd _tipun_precmd"));
    assert!(output.stderr.is_empty());
}

#[test]
fn init_rejects_unknown_shells() {
    for args in [&["init", "bash"][..], &["init"]] {
        let output = tipun(args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
    }
}

#[test]
fn analyze_is_silent_and_exits_zero() {
    let command = "cat app.log | grep ERROR";
    let cases: [(&[&str], Vars); 9] = [
        (&["analyze", "--command", "ls | grep x"], &[]),
        (&["analyze", "--command", "echo $HOME"], &[]),
        (&["analyze", "--command", ""], &[]),
        (&["analyze", "--command", "-rf"], &[]),
        (&["analyze", "--command", command, "--exit-code", "1"], &[]),
        (&["analyze", "--command", command], &[("CI", "true")]),
        (
            &["analyze", "--command", command],
            &[("TIPUN_DISABLE", "1")],
        ),
        (&["analyze"], &[]),
        (&["analyze", "--command", command, "--exit-code", "x"], &[]),
    ];
    for (args, vars) in cases {
        let output = tipun_with_env(args, vars);
        assert_eq!(
            output.status.code(),
            Some(0),
            "args: {args:?} env: {vars:?}"
        );
        assert!(output.stdout.is_empty(), "args: {args:?} env: {vars:?}");
        assert!(output.stderr.is_empty(), "args: {args:?} env: {vars:?}");
    }
}
