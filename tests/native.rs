use std::path::PathBuf;
use std::process::Command;

const SLICE_ONE: [&str; 7] =
    ["hello", "pipes", "dispatch", "errors", "records", "effects", "constants"];

#[test]
fn native_builds_match_interpreter_output() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let work = std::env::temp_dir().join("kanso-native-test");
    std::fs::create_dir_all(&work).expect("temp work dir");
    for name in SLICE_ONE {
        let program = manifest.join("examples").join(format!("{name}.kso"));
        // `run` compiles native; the oracle needs --interp, or this compares
        // the native engine against itself and proves nothing.
        let interpreted = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&program)
            .arg("--interp")
            .output()
            .expect("interpreter runs");
        assert!(interpreted.status.success(), "interpreter failed on {name}");
        let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("build")
            .arg(&program)
            .current_dir(&work)
            .output()
            .expect("kanso build runs");
        assert!(
            built.status.success(),
            "build failed on {name}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        let native = Command::new(work.join(name)).output().expect("native binary runs");
        assert_eq!(
            String::from_utf8_lossy(&native.stdout),
            String::from_utf8_lossy(&interpreted.stdout),
            "native output diverges from interpreter for {name}"
        );
        assert!(native.status.success(), "native binary failed on {name}");
    }
}

#[test]
fn release_build_matches_interpreter_output() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let work = std::env::temp_dir().join("kanso-native-test-release");
    std::fs::create_dir_all(&work).expect("temp work dir");
    let program = manifest.join("examples").join("dispatch.kso");
    let interpreted = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&program)
        .output()
        .expect("interpreter runs");
    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(&program)
        .arg("--release")
        .current_dir(&work)
        .output()
        .expect("kanso build runs");
    assert!(
        built.status.success(),
        "release build failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let native = Command::new(work.join("dispatch")).output().expect("native binary runs");

    assert_eq!(
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "release output diverges from interpreter"
    );
}

/// A program the operating system kills carries no exit code, and the driver
/// used to pass that through as a bare failure with nothing on stderr — the
/// reader got no cause at all.
#[test]
fn a_program_killed_by_the_operating_system_says_what_ended_it() {
    let work = std::env::temp_dir().join("kanso-native-test");
    std::fs::create_dir_all(&work).expect("temp work dir");
    let program = work.join("out_of_stack.kso");
    std::fs::write(
        &program,
        "fn total n\n  return 0 if n < 1\n  n + total (n - 1)\n\npub play = print \"{total 2000000}\"\n",
    )
    .expect("program writes");

    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", program.to_str().expect("utf-8")])
        .output()
        .expect("kanso runs");

    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ran out of stack"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
