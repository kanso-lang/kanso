//! A build runs the jobs clang's driver would run, without the driver, and
//! makes the same binary.
//!
//! The driver is a whole libLLVM process, 83% of it the dynamic loader, whose
//! only work in a build is deciding two commands. `kanso build` asks it once
//! with `-###` and runs the two commands itself from then on, with the
//! build's own file names put back. What that must not change is the binary:
//! the same bytes the driver would have linked, on both tiers.
//!
//! `KANSO_CLANG_DRIVER` sends a build through the driver, so the spec builds
//! each tier twice and compares. The second build of each tier is the replay:
//! the first asked the driver and remembered what it said.
//!
//! Watched red with the replay leaving the output's placeholder name in place
//! of the build's own: the dev build left no `main` where the driver's had
//! been.

#![cfg(target_os = "linux")]

use std::process::Command;

fn build(dir: &std::path::Path, release: bool, driver: bool) -> Vec<u8> {
    let _ = std::fs::remove_file(dir.join("main"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg("main.kso").current_dir(dir);
    if release {
        cmd.arg("--release");
    }
    match driver {
        true => cmd.env("KANSO_CLANG_DRIVER", "1"),
        false => cmd.env_remove("KANSO_CLANG_DRIVER"),
    };
    let built = cmd.output().expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(dir.join("main")).output().expect("the binary runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "without the driver\n");
    std::fs::read(dir.join("main")).expect("the binary reads")
}

#[test]
fn the_replayed_jobs_link_the_drivers_binary() {
    let dir = std::env::temp_dir().join(format!("kanso_replay_spec_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("main.kso"), "print \"without the driver\"\n").expect("writes");
    for release in [false, true] {
        let driven = build(&dir, release, true);
        let asked = build(&dir, release, false);
        let replayed = build(&dir, release, false);
        assert!(
            driven == asked,
            "release {release}: the build that asked differs from the driver's"
        );
        assert!(
            driven == replayed,
            "release {release}: the replayed build differs from the driver's"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
