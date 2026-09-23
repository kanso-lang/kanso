//! Whether this host's clang takes `preserve_none` is asked once per clang, not
//! once per build.
//!
//! The answer comes from compiling a two-define module, a whole clang run of
//! 32,201,483 instructions, and every `kanso build` paid it to learn what the
//! build before it had learned. It is kept in the temp directory now, keyed by
//! the clang the PATH resolves to. This spec puts a clang in front of the real
//! one that writes down each command it is given, builds the same program
//! twice against one temp directory, and counts the probe's compiles.

use std::path::{Path, PathBuf};
use std::process::Command;

fn real_clang() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|d| d.join("clang"))
        .find(|p| p.is_file())
        .expect("clang is on the PATH")
}

fn build(dir: &Path, path: &str) {
    let status = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso"])
        .current_dir(dir)
        .env("PATH", path)
        .env("TMPDIR", dir.join("tmp"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("kanso runs");
    assert!(status.success(), "the build failed");
}

#[test]
fn a_second_build_does_not_probe_again() {
    let dir = std::env::temp_dir().join(format!("kanso-probe-once-{}", std::process::id()));
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
    std::fs::write(dir.join("main.kso"), "print \"x\"\n").expect("the program writes");

    let path = format!("{}:{}", shims.display(), std::env::var("PATH").expect("PATH is set"));
    build(&dir, &path);
    build(&dir, &path);

    let commands = std::fs::read_to_string(&log).expect("the shim logged");
    let probes = commands.lines().filter(|l| l.contains("kanso_pn_probe")).count();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(probes, 1, "the probe ran {probes} times over two builds:\n{commands}");
}
