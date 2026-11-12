use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

// macOS ships bash 3.2 as /bin/bash, while `bash` on PATH may be a newer one.
const BASHES: [&str; 2] = ["/bin/bash", "bash"];

fn run_interactive_bash(bash: &str, name: &str, lines: &[&str]) -> String {
    let dir = std::env::temp_dir().join(format!("tipun-bash-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    fs::write(dir.join("words.txt"), "b\na\na\n").expect("write fixture");

    let bin_dir = Path::new(env!("CARGO_BIN_EXE_tipun"))
        .parent()
        .expect("binary has a parent directory");
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut child = Command::new(bash)
        .args(["--norc", "--noprofile", "-i"])
        .current_dir(&dir)
        .env("PATH", path)
        .env("HISTFILE", "/dev/null")
        .env_remove("HISTCONTROL")
        .env_remove("PROMPT_COMMAND")
        .env_remove("TIPUN_DEBUG")
        .env_remove("TIPUN_DISABLE")
        .env_remove("CI")
        .env("TIPUN_NO_UPDATE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash must be installed to run this test");
    let mut script = String::from(
        "PS1=\nset -o history\nPROMPT_COMMAND='echo other-hook >&2'\neval \"$(tipun init bash)\"\n",
    );
    for line in lines {
        script.push_str(line);
        script.push('\n');
    }
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(script.as_bytes())
        .expect("write to bash");
    let output = child.wait_with_output().expect("wait for bash");
    fs::remove_dir_all(&dir).expect("remove temp dir");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const TIP: &str =
    "tipun tip: sort -u words.txt\n  sort -u drops duplicate lines itself, no need for uniq.\n";

#[test]
fn hook_prints_a_tip_after_each_matching_command() {
    for bash in BASHES {
        let lines = ["sort words.txt | uniq", "sort words.txt | uniq"];
        let stderr = run_interactive_bash(bash, "tip", &lines);
        assert_eq!(stderr.matches(TIP).count(), 2, "{bash}: {stderr:?}");
    }
}

#[test]
fn hook_is_silent_for_failures_empty_lines_and_aliases() {
    for bash in BASHES {
        let lines = [
            "ls words.txt",
            "cat missing.txt | grep a",
            "",
            "alias sort='sort -r'",
            "sort words.txt | uniq",
            "unalias sort",
            "alias grep='grep --color=auto'",
            "sort words.txt | uniq",
        ];
        let stderr = run_interactive_bash(bash, "silent", &lines);
        assert_eq!(
            stderr.matches("tipun tip:").count(),
            1,
            "{bash}: {stderr:?}"
        );
        assert_eq!(stderr.matches(TIP).count(), 1, "{bash}: {stderr:?}");
    }
}

#[test]
fn hook_keeps_the_exit_status_and_other_prompt_commands() {
    for bash in BASHES {
        let lines = [
            "false",
            "echo \"status $?\" >&2",
            "set -u",
            "sort words.txt | uniq",
        ];
        let stderr = run_interactive_bash(bash, "status", &lines);
        assert!(stderr.contains("status 1\n"), "{bash}: {stderr:?}");
        assert!(stderr.contains(TIP), "{bash}: {stderr:?}");
        assert!(!stderr.contains("unbound"), "{bash}: {stderr:?}");
        assert!(
            stderr.matches("other-hook").count() >= 4,
            "{bash}: {stderr:?}"
        );
    }
}
