//! A dev build in an empty temp directory does not compile the runtime.
//!
//! The dev tier links `runtime.c` as an object kept in the temp directory, and
//! the first build in a fresh one compiled it: 5.7 s of a 7.2 s cold build of
//! senro's pantry (kanso#2373). build.rs compiles that object once and the
//! binary carries it. This spec puts a clang in front of the real one that
//! writes down each command it is given, builds a program against an empty
//! temp directory, runs what it built, and looks for a compile of the runtime
//! among the commands.

use std::path::{Path, PathBuf};
use std::process::Command;

fn real_clang() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|d| d.join("clang"))
        .find(|p| p.is_file())
        .expect("clang is on the PATH")
}

fn kanso(dir: &Path, path: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(dir)
        .env("PATH", path)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("kanso runs")
}

#[test]
fn a_cold_dev_build_does_not_compile_the_runtime() {
    let dir = std::env::temp_dir().join(format!("kanso-cold-runtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shims = dir.join("bin");
    std::fs::create_dir_all(&shims).expect("the shim directory is made");
    std::fs::create_dir_all(dir.join("tmp")).expect("the temp directory is made");
    let log = dir.join("clang.log");
    let shim = shims.join("clang");
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\necho \"$@\" >> {}\nexec {} \"$@\"\n",
            log.display(),
            real_clang().display()
        ),
    )
    .expect("the shim writes");
    Command::new("chmod").arg("+x").arg(&shim).status().expect("chmod runs");
    std::fs::write(dir.join("main.kso"), "print \"cold\"\n").expect("the program writes");

    let path = format!("{}:{}", shims.display(), std::env::var("PATH").expect("PATH is set"));
    let built = kanso(&dir, &path, &["run", "main.kso"]);
    let commands = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        String::from_utf8_lossy(&built.stdout),
        "cold\n",
        "the program did not print its line: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    // The link names the object, `kanso_runtime_dev_<key>.o`; only a compile
    // names the source.
    let compiles =
        commands.lines().filter(|l| l.contains("kanso_runtime_dev_") && l.contains(".c ")).count();
    assert_eq!(compiles, 0, "a cold build compiled the runtime:\n{commands}");
}
