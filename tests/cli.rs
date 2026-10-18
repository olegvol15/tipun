use std::process::Command;

fn tipun(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tipun"))
        .args(args)
        .env_remove("TIPUN_DEBUG")
        .output()
        .expect("failed to run tipun")
}

#[test]
fn suggest_is_silent_without_rules() {
    for command in ["cat f | grep x", "echo $HOME", "", "FOO=1 cat f", "ls >"] {
        let output = tipun(&["suggest", command]);
        assert_eq!(output.status.code(), Some(1), "command: {command:?}");
        assert!(output.stdout.is_empty(), "command: {command:?}");
        assert!(output.stderr.is_empty(), "command: {command:?}");
    }
}
