use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn run_interactive_zsh(name: &str, lines: &[&str]) -> String {
    let dir = std::env::temp_dir().join(format!("tipun-zsh-{name}-{}", std::process::id()));
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

    // Without +Z an interactive zsh started from a terminal reads its commands from the terminal
    // instead of the piped stdin, and fails with "error on TTY read".
    let mut child = Command::new("zsh")
        .args(["-f", "-i", "+Z"])
        .current_dir(&dir)
        .env("PATH", path)
        .env_remove("TIPUN_DEBUG")
        .env_remove("TIPUN_DISABLE")
        .env_remove("CI")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("zsh must be installed to run this test");
    let mut script =
        String::from("PS1=\nunsetopt prompt_sp prompt_cr\neval \"$(tipun init zsh)\"\n");
    for line in lines {
        script.push_str(line);
        script.push('\n');
    }
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(script.as_bytes())
        .expect("write to zsh");
    let output = child.wait_with_output().expect("wait for zsh");
    fs::remove_dir_all(&dir).expect("remove temp dir");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn hook_prints_a_tip_after_each_matching_command() {
    let stderr = run_interactive_zsh("tip", &["sort words.txt | uniq", "sort words.txt | uniq"]);
    let tip =
        "tipun tip: sort -u words.txt\n  sort -u drops duplicate lines itself, no need for uniq.\n";
    assert_eq!(stderr.matches(tip).count(), 2, "stderr: {stderr:?}");
}

#[test]
fn hook_is_silent_for_failures_aliases_and_other_commands() {
    let stderr = run_interactive_zsh(
        "silent",
        &[
            "ls words.txt",
            "cat missing.txt | grep a",
            "alias sort='sort -r'",
            "sort words.txt | uniq",
            "unalias sort",
            "sort words.txt | uniq",
        ],
    );
    let tip =
        "tipun tip: sort -u words.txt\n  sort -u drops duplicate lines itself, no need for uniq.\n";
    assert_eq!(stderr.matches("tipun").count(), 1, "stderr: {stderr:?}");
    assert!(stderr.ends_with(tip), "stderr: {stderr:?}");
}
