mod common;

use std::fs;

use common::{assert_same_files, assert_same_output, fixtures};

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
