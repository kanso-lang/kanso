//! Every counter the objective weighs stands on a measurement of its own.
//!
//! It used to be granted one. `entering` gave a counter new to the model a
//! baseline of `now * standing` — the ratio whose satisfaction equalled its
//! dimension's current mean — so that landing day cost the floor nothing, and
//! the floor file recorded which counters were standing on that rule. Nine of
//! twenty-one were, because benchmarks joined the run side one at a time and
//! each arrival at parity would otherwise have scored as a free win for
//! measuring something.
//!
//! The rule was never neutral after landing day. Saturation is concave, so a
//! counter granted a high standing has almost no headroom left and one entered
//! at parity has a great deal, and that difference decided at least one
//! verdict: the carry-tier arms of 2026-09-01 scored 74.31 -> 73.75 with the
//! digest baselines at their dimension's standing, and 70.14 -> 72.99 with the
//! same two arms measured against parity — a decline and an acceptance.
//!
//! Clay's gavel of 2026-09-06 put the run side on one consolidated program.
//! Nothing joins a benchmark at a time now, so the rule has nobody left to
//! grant a baseline to, and what remained of it could only fire on a counter
//! somebody added to the model and forgot to measure. That is the thing to
//! catch, and this is what catches it.

use std::path::Path;
use std::process::Command;

/// Stage `bench/` and run welfare there, editing the floor file first.
/// Answers the exit status and what it printed.
fn scored(key: &str, edit: &dyn Fn(&str) -> String) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-nobaseline-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bench")).expect("a staging directory");
    for entry in std::fs::read_dir(root.join("bench")).expect("bench is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_file() {
            let landing = stage.join("bench").join(path.file_name().expect("named"));
            std::fs::copy(&path, &landing).expect("the golden copies");
        }
    }
    let floor = stage.join("bench/welfare_floor.json");
    let held = std::fs::read_to_string(&floor).expect("the floor reads");
    std::fs::write(&floor, edit(&held)).expect("the floor writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .current_dir(&stage)
        .output()
        .expect("welfare runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&stage);
    (done.status.success(), said)
}

/// The counter names the model reads, off the model rather than out of a list
/// here — a list here would be the next place to forget.
fn weighed() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .arg("--")
        .arg("--counters")
        .current_dir(root)
        .output()
        .expect("welfare runs");
    assert!(done.status.success(), "welfare --counters should hold");
    String::from_utf8_lossy(&done.stdout)
        .lines()
        .filter_map(|l| l.split_once('=').map(|(n, _)| n.to_string()))
        .collect()
}

#[test]
fn a_counter_the_model_weighs_and_nobody_measured_is_refused() {
    // Cut the key and its reading out of the baseline, whatever the reading is
    // and wherever the key sits in it: pinning the number here would make this
    // spec go green the next time the baseline is re-measured, which is the
    // one time it most wants to be red.
    let dropped = |held: &str| {
        let key = "\"run_peak_bytes\":";
        let at = held.find(key).expect("the baseline names the counter");
        let rest = &held[at + key.len()..];
        let end = at + key.len() + rest.find([',', '}']).expect("the reading ends");
        let mut out = held.to_string();
        let cut = if out.as_bytes()[at - 1] == b',' { at - 1 } else { end };
        let keep = if out.as_bytes()[at - 1] == b',' { end } else { end + 1 };
        out.replace_range(cut..keep, "");
        assert!(!out.contains("\"run_peak_bytes\""), "the key should be gone: {out}");
        out
    };
    let (passed, said) = scored("dropped", &dropped);
    assert!(!passed, "welfare scored a counter it has no baseline for:\n{said}");
    assert!(said.contains("no baseline for: run_peak_bytes"), "{said}");
    assert!(said.contains("bench/welfare_floor.json"), "it should say where to write it:\n{said}");
}

/// And the shipped floor answers for every counter, checked directly rather
/// than by waiting for the gate to refuse on CI.
#[test]
fn the_shipped_floor_measures_every_counter_the_model_weighs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let held =
        std::fs::read_to_string(root.join("bench/welfare_floor.json")).expect("the floor reads");
    let counters = weighed();
    assert!(!counters.is_empty(), "the model named no counters");
    let missing: Vec<&String> =
        counters.iter().filter(|c| !held.contains(&format!("\"{c}\":"))).collect();
    assert!(missing.is_empty(), "the floor has no baseline for {missing:?}");
}

/// The retired half, said as a property rather than as an absence: the floor
/// file carries no record of a granted baseline, because nothing grants one.
#[test]
fn the_floor_records_no_granted_baselines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let held =
        std::fs::read_to_string(root.join("bench/welfare_floor.json")).expect("the floor reads");
    assert!(
        !held.contains("\"granted\""),
        "the entering rule is retired; a granted list here would mean it is back"
    );
}
