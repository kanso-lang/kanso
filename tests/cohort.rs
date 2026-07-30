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

/// The survivor-ratio guard's pin: a qualified utf8 over sliced bytes
/// trips the block threshold, and the guard keeps the region because the
/// string is the growth. Watched red twice: guard disabled reads
/// cohort_frees=2 (the wasted copy), license narrowed reads
/// cohort_kept=0 (the wrap never lands).
#[test]
fn a_result_heavy_cohort_is_kept_not_copied() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/cohort");
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("cohort_kept.kso")
        .env("KANSO_COUNTERS", "1")
        .current_dir(dir)
        .output()
        .expect("kanso binary runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stdout, "chars 2400000\n", "stdout mismatch: {stderr}");
    assert!(stderr.contains("cohort_kept=1"), "the guard never engaged: {stderr}");
    // An internal verdict, asserted deliberately: for this fixture the peak is
    // the same whether the cohort fires or not, so the counter is the only
    // signal that the kernel is still there — which is what makes it worth
    // pinning, and what makes it wrong to read as a cost. Measured: with the
    // rewind threshold raised out of reach, this program's peak does not move.
    assert!(stderr.contains("cohort_frees=1"), "free count moved: {stderr}");
    assert!(output.status.success());
}
