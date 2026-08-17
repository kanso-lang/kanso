use std::process::Command;

fn run(dir: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/reexports/{dir}"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

/// GAVEL 51, Clay 2026-08-17: "one module", and routes are not names. This
/// asked for `geo/list/order` until the fork that minted it was removed —
/// `order` is geo's own rename of list's `sort`, so that spelling reached
/// through geo's PRIVATE import under a name list does not have, and it only
/// resolved because geo's copy of list was a second instance.
///
/// The bare name is what a re-export puts on the surface today. The qualified
/// door name `geo/order` is the other half and does not exist yet — it is not
/// this change's regression (it never resolved, on this branch or on main),
/// and it has its own task.
#[test]
fn reexported_names_join_the_surface_bare() {
    let output = run("app");

    assert_eq!(String::from_utf8_lossy(&output.stdout), "[3 2]\n[1]\n[1 2 3]\n12.56636\n");
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

#[test]
fn a_bare_call_two_imports_answer_alike_is_refused() {
    let output = run("torn/use");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("a bare `strength` reaches `brew/strength` and `steep/strength` alike"),
        "expected the torn bare call to be refused, got: {stderr}"
    );
    assert!(!output.status.success());
}
