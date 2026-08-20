use std::path::{Path, PathBuf};
use std::process::Command;

/// A spread of what the emitter has distinct paths for. These come from the
/// micro corpus rather than examples/, because an example is a play file and
/// the play verb runs without ever building — a binary is the first step of a
/// real program, which is what a library and its entry are for.
const SLICE_ONE: [&str; 7] = [
    "dispatch_int",
    "arity_overload",
    "err_trap_leaf",
    "bare_field",
    "accessor_value",
    "a_text_block",
    "bitwise_operators",
];

/// A corpus sample exports `play` and is a library, so what gets built is an
/// entry that imports one — the language knows no such name, and the
/// convention of running one lives in the harness. A sample that is already
/// an entry file is staged as it stands.
fn staged_entry(manifest: &Path, work: &Path, name: &str) -> (PathBuf, String) {
    std::fs::create_dir_all(work).expect("temp work dir");
    let sample = manifest.join("tests/golden/micro").join(format!("{name}.kso"));
    let text = std::fs::read_to_string(&sample).expect("the example reads");
    std::fs::copy(&sample, work.join(format!("{name}.kso"))).expect("the example copies");
    if !text.contains("\npub play") && !text.starts_with("pub play") {
        return (work.join(format!("{name}.kso")), name.to_string());
    }
    let binary = format!("built_{name}");
    let entry = work.join(format!("{binary}.kso"));
    std::fs::write(&entry, format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry file writes");
    (entry, binary)
}

#[test]
fn native_builds_match_interpreter_output() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let work = std::env::temp_dir().join("kanso-native-test");
    for name in SLICE_ONE {
        let (program, binary) = staged_entry(&manifest, &work, name);
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
        let native = Command::new(work.join(&binary)).output().expect("native binary runs");
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
    let (program, binary) = staged_entry(&manifest, &work, "dispatch_int");
    let interpreted = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&program)
        .arg("--interp")
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
    let native = Command::new(work.join(&binary)).output().expect("native binary runs");

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
        "fn total n\n  return 0 if n < 1\n  n + total (n - 1)\n\nprint \"{total 2000000}\"\n",
    )
    .expect("program writes");

    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", program.to_str().expect("utf-8")])
        .output()
        .expect("kanso runs");

    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ran out of stack"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
