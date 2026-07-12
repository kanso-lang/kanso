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
        let interpreted = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&program)
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
