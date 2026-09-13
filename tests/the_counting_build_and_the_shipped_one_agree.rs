//! `kanso build --counters` and a plain `kanso build` are two binaries now, and
//! the point of the split is that the shipped one has no allocation-counter
//! gates in its inlined fast paths. That buys 25,968,820 instructions on the run
//! program, 1.2128%, and it costs the project a second binary that could quietly
//! disagree with the first.
//!
//! So this pins the property the split rests on: the SAME program, built both
//! ways, prints the same bytes. What a program does cannot depend on whether
//! somebody is counting.
//!
//! It also pins the other half, off the emitted IR the code goldens already
//! read: the counting build carries the gates and the shipped one carries none.
//! Without that, a change that stopped stripping them would leave this test
//! green on stdout and silently give back the whole win.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A sample whose inner loop is the in-place append -- the fast path the gates
/// sit in front of. A program that never appends would pass this test with the
/// stripping broken.
const SAMPLE: &str = "an_in_place_append_takes_a_whole_string";

fn build(stage: &std::path::Path, entry: &str, counters: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg(format!("{entry}.kso")).arg("--release");
    if counters {
        cmd.arg("--counters");
    }
    // The child must not inherit a KANSO_COUNTERS from whatever is running the
    // suite: a build under that variable keeps the gates on purpose, which
    // would make the two arms identical and the test vacuous.
    let built = cmd.env_remove("KANSO_COUNTERS").current_dir(stage).output().expect("kanso runs");
    assert!(
        built.status.success(),
        "the {} build failed: {}",
        match counters {
            true => "--counters",
            false => "plain",
        },
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::read_to_string(stage.join(format!("{entry}.ll"))).expect("the emitted ir reads")
}

fn run(stage: &std::path::Path, entry: &str) -> String {
    let out = Command::new(stage.join(entry)).current_dir(stage).output().expect("the binary runs");
    assert!(out.status.success(), "the binary failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn the_counting_build_and_the_shipped_one_agree() {
    let source = manifest_dir().join("tests/golden/micro").join(format!("{SAMPLE}.kso"));
    let stage = std::env::temp_dir().join("kanso-counters-split");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the stage is made");
    std::fs::copy(&source, stage.join(format!("{SAMPLE}.kso"))).expect("the sample copies");
    let entry = format!("run_{SAMPLE}");
    std::fs::write(
        stage.join(format!("{entry}.kso")),
        format!("import \"./{SAMPLE}\"\n\n{SAMPLE}/play\n"),
    )
    .expect("the entry file writes");

    let counting_ir = build(&stage, &entry, true);
    let counting_out = run(&stage, &entry);
    let shipped_ir = build(&stage, &entry, false);
    let shipped_out = run(&stage, &entry);

    assert_eq!(
        counting_out, shipped_out,
        "the two builds print different bytes; what a program does must not depend on \
         whether it is being counted"
    );

    let gates = |ir: &str| ir.matches("load i32, ptr @k_stats_on").count();
    assert_eq!(
        gates(&counting_ir),
        kanso::codegen::STATS_GATE_SITES,
        "the --counters build lost a gate: its inlined fast paths would stop counting"
    );
    assert_eq!(
        gates(&shipped_ir),
        0,
        "the shipped build still asks whether statistics are on; the 1.2128% is being paid \
         for a question whose answer is fixed"
    );
}
