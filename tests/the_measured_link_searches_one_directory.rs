//! A link given `KANSO_LINK_DIR` searches that directory and nothing else.
//!
//! `ld.gold` reads every directory it may search, whole, before it looks a
//! library up, so the dev codegen row counted the files a machine happened to
//! have installed. One file added to `/usr/lib/x86_64-linux-gnu` moved the row
//! by 3,559 instructions and removing it put the row back to the instruction.
//! On CI the row read 130,465,044 on runner image `ubuntu24/20260927.320` and
//! 130,465,609 on `20261004.327`, with one binary and one tree, and that
//! difference had been read as the CPU for a day because the two images
//! happened to sit on different machines.
//!
//! The codegen gate stages a directory holding the five libraries the link
//! resolves and names it in `KANSO_LINK_DIR`. kanso replaces the link's `-L`
//! list with it and adds `-nostdlib`, without which gold still read its four
//! built-in defaults and the added file still moved the row by 1,864.
//!
//! What a user can see is whether the link finds its libraries, so that is
//! what this asserts: an empty directory must fail the build on both tiers,
//! and the staged one must link a binary that runs. A link that still searched
//! the system would pass the first half with an empty directory too.
//!
//! Watched red with the variable ignored: the empty-directory dev build linked.
//! Watched red with `-nostdlib` commented out, by the second test only.

#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::Command;

fn build(dir: &Path, link: &Path, release: bool) -> std::process::Output {
    let _ = std::fs::remove_file(dir.join("main"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg("main.kso").current_dir(dir).env("KANSO_LINK_DIR", link);
    cmd.env_remove("KANSO_CLANG_DRIVER");
    if release {
        cmd.arg("--release");
    }
    cmd.output().expect("kanso runs")
}

/// The gate's staging, done the same way: each library where clang finds it.
fn staged(at: &Path) {
    std::fs::create_dir_all(at).expect("the link directory");
    for lib in ["libm.so", "libgcc.a", "libgcc_s.so", "libgcc_s.so.1", "libc.so"] {
        let found = Command::new("clang")
            .arg(format!("-print-file-name={lib}"))
            .output()
            .expect("clang runs");
        let path = String::from_utf8_lossy(&found.stdout).trim().to_string();
        assert!(path.starts_with('/'), "clang cannot find {lib}: {path:?}");
        std::os::unix::fs::symlink(&path, at.join(lib)).expect("the link stages");
    }
}

#[test]
fn the_link_finds_its_libraries_in_the_named_directory_only() {
    let dir = std::env::temp_dir().join(format!("kanso_link_dir_spec_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("main.kso"), "print \"one directory\"\n").expect("writes");
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).expect("an empty directory");
    let full = dir.join("full");
    staged(&full);

    for release in [false, true] {
        let refused = build(&dir, &empty, release);
        assert!(
            !refused.status.success(),
            "release {release}: the link found its libraries with an empty KANSO_LINK_DIR, \
             so it searched somewhere else"
        );

        let built = build(&dir, &full, release);
        assert!(
            built.status.success(),
            "release {release}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        let ran = Command::new(dir.join("main")).output().expect("the binary runs");
        assert_eq!(String::from_utf8_lossy(&ran.stdout), "one directory\n", "release {release}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `-nostdlib` changes what gold READS, not what it finds: without it the
/// empty-directory build above still fails, because gold's built-in defaults
/// do not answer `-l`. It still opened and read all four of them, and that is
/// the 1,864 instructions the added file moved. No output shows it, so this
/// asks the code, the way the `-save-temps=obj` spec does.
#[test]
fn the_swap_tells_the_linker_to_search_nothing_else() {
    let main = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("src/main.rs reads");
    let body = main
        .split_once("fn searching_only")
        .expect("main.rs defines searching_only")
        .1
        .split_once("\nfn ")
        .expect("searching_only is followed by another item")
        .0;
    let code: String =
        body.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    assert!(code.contains("\"-nostdlib\""), "searching_only does not pass -nostdlib:\n{code}");
}
