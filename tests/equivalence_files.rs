mod common;

use std::fs;
use std::process::Command;

use common::{assert_same_files, assert_same_output, fixtures, sh, suggest};

#[test]
fn bundled_flags_read_the_same() {
    let dir = fixtures("bundle-read");
    let originals = [
        "ls -a -l src",
        "ls -1 -t src",
        "ls -l -a -h src",
        "grep -i -v error app.log",
        "grep -i -e warn app.log",
        "grep -i -ewarn app.log",
    ];
    assert_same_output(&dir, &originals);
    fs::remove_dir_all(dir).expect("remove fixture dir");
}

#[test]
fn bundled_flags_change_files_the_same() {
    let originals = [
        "rm -r -f src",
        "rm -f -v app.log",
        "cp -r -v src copy",
        "mv -i -v app.log moved.log",
        "mkdir -p -v a/b/c",
        "mkdir -p -m 700 a/b",
        "mkdir -p -m700 a/b",
        "touch -a -c app.log",
        "touch -c -m missing.txt",
    ];
    assert_same_files("bundle", &originals);
}

#[test]
fn merged_commands_change_files_the_same() {
    let originals = [
        "touch a && touch 'b c'",
        "touch -c app.log && touch -c missing.txt",
        "rm app.log && rm case.txt 'my file.txt'",
        "rm -rf src && rm -rf missing",
        "mkdir -p a/b && mkdir -p c",
        "mkdir a && mkdir a/b",
        "mkdir -m 700 a && mkdir -m 700 b",
        "chmod 600 app.log && chmod 600 case.txt",
        "chmod +x app.log && chmod +x 'my file.txt'",
        "mkdir out && cp app.log out/ && cp case.txt out/",
        "mkdir out && cp -p app.log src/main.txt out/ && cp -p 'my file.txt' out/",
        "mkdir -v a && mkdir -v b",
        "rm -v case.txt && rm -v app.log",
    ];
    assert_same_files("merge", &originals);
}

#[test]
fn moves_and_recursive_copies_that_create_the_target_are_not_merged() {
    let pairs = [
        (
            "mkdir d && cp -r src out/ && cp -r d out/",
            "mkdir d && cp -r src d out/",
        ),
        (
            "mkdir d && mv src out/ && mv d out/",
            "mkdir d && mv src d out/",
        ),
    ];
    for (i, (original, merged)) in pairs.iter().enumerate() {
        assert_eq!(suggest(original), None, "original: {original:?}");
        let (before, after) = (
            fixtures(&format!("merge-dir-{i}")),
            fixtures(&format!("merge-dir-merged-{i}")),
        );
        assert_eq!(
            sh(&before, original).status.code(),
            Some(0),
            "original: {original:?}"
        );
        assert_ne!(
            sh(&after, merged).status.code(),
            Some(0),
            "merged: {merged:?}"
        );
        fs::remove_dir_all(before).expect("remove fixture dir");
        fs::remove_dir_all(after).expect("remove fixture dir");
    }
}

#[test]
fn one_cd_climbs_to_the_same_directory() {
    let dir = fixtures("cd-parent");
    let setup = sh(&dir, "mkdir -p real/a/b/c/src && ln -s real/a/b/c link");
    assert_eq!(setup.status.code(), Some(0), "setup");
    let originals = [
        "cd .. && cd ..",
        "cd ../ && cd ..",
        "cd .. && cd ../..",
        "cd src && cd .. && cd .. && cd ..",
    ];
    let shells = [
        ("zsh", ""),
        ("zsh", "setopt chase_links; "),
        ("zsh", "setopt chase_dots; "),
        ("sh", ""),
        ("bash", "set -P; "),
    ];
    for original in originals {
        let suggestion = suggest(original).expect("tipun must suggest a rewrite");
        for (shell, options) in shells {
            let run = |command: &str| {
                let script = format!("{options}cd link && {command} && pwd && pwd -P");
                let output = Command::new(shell)
                    .args(["-c", &script])
                    .current_dir(&dir)
                    .env_remove("CDPATH")
                    .output()
                    .expect("run the shell");
                assert_eq!(output.status.code(), Some(0), "{shell}: {script:?}");
                output.stdout
            };
            assert_eq!(
                run(&suggestion),
                run(original),
                "{shell} {options:?}: {original:?} vs {suggestion:?}"
            );
        }
    }
    fs::remove_dir_all(dir).expect("remove fixture dir");
}
