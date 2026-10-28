use std::process::Command;

fn tipun(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tipun"))
        .args(args)
        .env_remove("TIPUN_DEBUG")
        .output()
        .expect("failed to run tipun")
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
