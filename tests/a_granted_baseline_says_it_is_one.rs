//! Nine of the model's twenty-one counters have a baseline nobody measured,
//! and until this printed, nothing said which.
//!
//! `entering` gives a counter new to the model a baseline of `now * standing`
//! — the ratio whose satisfaction equals its dimension's current mean — so
//! landing day is neutral. That rule should stay: entering at parity instead
//! would make a measurement-only change spend the floor, and an objective
//! that charges you for measuring is paying you not to.
//!
//! It is not neutral about anything AFTER landing day. Saturation is concave,
//! so a counter granted a high standing has little headroom left and one at
//! parity has a great deal. On the carry-tier arms of 2026-09-01: with the
//! digest baselines at their dimension's standing the trade scored
//! 74.31 -> 73.75 and was declined; the same two arms with those baselines at
//! parity score 70.14 -> 72.99, an acceptance. The entering rule decided that
//! verdict, and the floor file recorded the granted baselines exactly like the
//! measured ones.

use std::path::Path;
use std::process::Command;

/// Stage `bench/` and run welfare there, optionally editing the floor file
/// first. Answers the exit status, what it printed, and the floor file after.
fn scored(key: &str, edit: &dyn Fn(&str) -> String, args: &[&str]) -> (bool, String, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-granted-{key}"));
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
        .args(args)
        .current_dir(&stage)
        .output()
        .expect("welfare runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let after = std::fs::read_to_string(&floor).expect("the floor reads back");
    let _ = std::fs::remove_dir_all(&stage);
    (done.status.success(), said, after)
}

fn unchanged(held: &str) -> String {
    held.to_string()
}

#[test]
fn welfare_names_the_counters_standing_on_a_rule() {
    let (passed, said, _) = scored("named", &unchanged, &[]);
    assert!(passed, "the score should still hold:\n{said}");
    assert!(
        said.contains("granted baselines"),
        "the report should name the granted counters:\n{said}"
    );
    // The three the digest brought in on 2026-08-31, and the two the compile
    // terms brought in on 2026-08-25. Each is checkable against the floor
    // file's own history: the commit that first wrote the key is a descendant
    // of the one that added `entering`.
    for counter in [
        "digest_peak_bytes",
        "digest_instructions",
        "digest_arena_blocks",
        "compile_allocs",
        "compile_instructions",
        "pending_instructions",
        "scan_peak_bytes",
        "scan_arena_blocks",
        "deep_instructions",
    ] {
        assert!(
            said.contains(counter),
            "{counter} entered under the rule and should say so:\n{said}"
        );
    }
    // And one that did not: decode_instructions predates `entering` by a
    // fortnight, so it stands on a measurement.
    let line = said.lines().find(|l| l.starts_with("granted baselines")).expect("the granted line");
    assert!(
        !line.contains("decode_instructions"),
        "a counter that predates the rule is not granted: {line}"
    );
}

#[test]
fn a_counter_granted_this_run_is_written_into_the_floor() {
    // Drop one counter's baseline so this run has to grant it, and drop the
    // floor so `--set` cannot refuse on a fall it did not cause.
    let dropped = |held: &str| {
        let mut out = held.replace("\"digest_peak_bytes\":610272059,", "");
        assert!(!out.contains("\"digest_peak_bytes\":610"), "the key should be gone");
        let at = out.find("\"floor\":").expect("a floor key");
        let end = out[at..].find(',').expect("a comma after the floor") + at;
        out.replace_range(at..end, "\"floor\":1.0");
        out
    };
    let (passed, said, after) = scored("written", &dropped, &["--", "--set", "a spec"]);
    assert!(passed, "--set should bank a rise:\n{said}");
    assert!(
        after.contains("\"granted\""),
        "the floor file should record which baselines were granted:\n{after}"
    );
    let granted = after.split("\"granted\":").nth(1).expect("a granted list").to_string();
    assert!(
        granted.contains("digest_peak_bytes"),
        "the counter granted on this run should be written down: {granted}"
    );
    assert!(
        granted.contains("compile_allocs"),
        "and the ones an earlier run granted should survive: {granted}"
    );
}
