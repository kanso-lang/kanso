use std::process::Command;

/// The cohort license's reach pin: a branch-chosen, bound, piped decode
/// still proves its argument a string, and the construction garbage dies
/// at the pop. Watched red (cohort_frees=0) with the yield tracking
/// stashed.
#[test]
fn a_bound_branch_chosen_pipe_still_fires_the_cohort() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/cohort");
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("cohort_bound.kso")
        .env("KANSO_COUNTERS", "1")
        .current_dir(dir)
        .output()
        .expect("kanso binary runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stdout, "held 160\n", "stdout mismatch: {stderr}");
    assert!(stderr.contains("cohort_frees=1"), "the cohort never fired: {stderr}");
    assert!(output.status.success());
}
