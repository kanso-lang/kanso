use std::process::Command;

fn run(dir: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/reexports/{dir}"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

#[test]
fn reexported_names_join_the_surface_bare_and_qualified() {
    let output = run("app");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "[3 2]\n[1]\n[1 2 3]\n[4 5]\n"
    );
    assert!(output.status.success());
}

#[test]
fn dependency_pubs_stay_off_the_surface_without_a_reexport() {
    let output = run("sealed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown name `sum`"),
        "expected the unreexported name to stay sealed, got: {stderr}"
    );
    assert!(!output.status.success());
}
