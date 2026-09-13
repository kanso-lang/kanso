//! The runtime's twenty-seven counter sites are compiled out of a shipped
//! binary, which is worth 13,187,834 instructions on the run program (0.6159%)
//! and 2,944 bytes of .text. That leaves a question the emitted half already
//! raised and could not answer: what should a SHIPPED binary do when somebody
//! sets `KANSO_COUNTERS` at run time?
//!
//! The wrong answer is the one that shipped in the emitted half alone, and it
//! is wrong in the worst available way. There, the runtime still counted and
//! only the inlined fast paths did not, so the block printed was mostly right:
//! on escapebench every row a runtime site owns read correctly and the two an
//! inlined path owns read `push_mut_fast=0` against 3,000 and
//! `push_mut_slow=12,000` against 1,200,000. Twenty-odd rows agreeing is what
//! makes that dangerous -- a reader has no reason to distrust the two that do
//! not.
//!
//! So a shipped binary says it cannot report and prints nothing. This pins
//! both halves of that by running the two binaries, because a spec asserting
//! it off the source would pass with the refusal removed.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The same sample the split's other spec uses: its inner loop is the in-place
/// append, so it moves counters a shipped binary would get wrong.
const SAMPLE: &str = "an_in_place_append_takes_a_whole_string";

fn build(stage: &std::path::Path, entry: &str, counters: bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg(format!("{entry}.kso")).arg("--release");
    if counters {
        cmd.arg("--counters");
    }
    // A build under KANSO_COUNTERS keeps the gates on purpose, so inheriting
    // one from the suite would make both arms counting and the test vacuous.
    let built = cmd.env_remove("KANSO_COUNTERS").current_dir(stage).output().expect("kanso runs");
    assert!(built.status.success(), "the build failed: {}", String::from_utf8_lossy(&built.stderr));
}

/// Counters go to stderr, and asking for them is an environment variable at
/// RUN time -- which is the whole point: the binary is already linked by then
/// and cannot go back and add the sites.
fn counter_report(stage: &std::path::Path, entry: &str) -> String {
    let out = Command::new(stage.join(entry))
        .env("KANSO_COUNTERS", "1")
        .current_dir(stage)
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "the binary failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_shipped_binary_refuses_to_report_counters() {
    let source = manifest_dir().join("tests/golden/micro").join(format!("{SAMPLE}.kso"));
    let stage = std::env::temp_dir().join("kanso-counters-refusal");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the stage is made");
    std::fs::copy(&source, stage.join(format!("{SAMPLE}.kso"))).expect("the sample copies");
    let entry = format!("run_{SAMPLE}");
    std::fs::write(
        stage.join(format!("{entry}.kso")),
        format!("import \"./{SAMPLE}\"\n\n{SAMPLE}/play\n"),
    )
    .expect("the entry file writes");

    build(&stage, &entry, true);
    let counting = counter_report(&stage, &entry);
    build(&stage, &entry, false);
    let shipped = counter_report(&stage, &entry);

    assert!(
        counting.contains("push_mut_fast="),
        "the --counters build stopped reporting; it is the one binary that must:\n{counting}"
    );
    assert!(
        counting.contains("allocs="),
        "the --counters build lost the allocation rows:\n{counting}"
    );

    assert!(
        shipped.contains("built without them"),
        "a shipped binary asked for counters said nothing about not having them:\n{shipped}"
    );
    assert!(
        !shipped.contains("push_mut_fast="),
        "a shipped binary printed a counter block. Every row a runtime site owns would read \
         right and every row an inlined path owns would read wrong, which is worse than \
         refusing:\n{shipped}"
    );
    assert!(
        !shipped.contains("allocs="),
        "a shipped binary printed allocation rows it cannot have counted:\n{shipped}"
    );
}
