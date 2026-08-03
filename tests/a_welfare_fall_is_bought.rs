//! Lowering the objective is not a thing `--set` can do at all.
//!
//! RULED (Clay, 2026-08-03): welfare cannot fall. Not with a reason, not
//! with a named trade. This gate went through two weaker versions — the
//! first took any sentence ("whatever" banked a twenty-three point drop),
//! the second demanded the reason name a compensating gain — and each
//! loosening was an escape that looked reasonable. Now the tool refuses a
//! fall unconditionally; if a change genuinely cannot hold the number, that
//! is a conversation with Clay, and the only mechanical override is editing
//! bench/welfare_floor.json by hand where a reviewer sees it in the diff.
//!
//! A RISE is not a trade and stays free to bank. Requiring justification for
//! an improvement is how a rule stops being read.

use std::process::Command;

/// Run welfare against a staged `bench/` whose floor has been set to `floor`,
/// so the real one is never touched and examples cannot race each other.
fn banked(floor: f64, reason: &str) -> bool {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Keyed by the reason itself: cargo runs these concurrently, and two
    // reasons of equal length staged into one directory tore each other down.
    let slug: String = reason.chars().filter(|c| c.is_ascii_alphanumeric()).take(40).collect();
    let stage = std::env::temp_dir().join(format!("kanso-welfare-{slug}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bench")).expect("a staging directory");
    for entry in std::fs::read_dir(root.join("bench")).expect("bench is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_file() {
            let landing = stage.join("bench").join(path.file_name().expect("named"));
            std::fs::copy(&path, &landing).expect("the golden copies");
        }
    }

    // The encoder sorts keys, so the top-level "floor" always sits before
    // "history" — whose entries carry a "floor" of their own.
    let held = stage.join("bench/welfare_floor.json");
    let text = std::fs::read_to_string(&held).expect("the floor reads");
    let at = text.find("\"floor\":").expect("the floor is recorded");
    let value = at + "\"floor\":".len();
    let end = value + text[value..].find(',').expect("a field ends somewhere");
    let doctored = format!("{}{floor}{}", &text[..value], &text[end..]);
    std::fs::write(&held, doctored).expect("the floor writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare.kso"))
        .arg("--")
        .arg("--set")
        .arg(reason)
        .current_dir(&stage)
        .output()
        .expect("welfare runs");
    let said = String::from_utf8_lossy(&done.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&stage);
    said.contains("floor moved")
}

/// The shape the first hole had: a junk reason.
#[test]
fn a_fall_with_a_junk_reason_is_refused() {
    assert!(
        !banked(99.0, "whatever"),
        "a twenty-three point fall was banked with a reason that names nothing"
    );
}

/// The shape the second hole had: a reason naming a trade. RULED closed —
/// no sentence lowers the objective, however good.
#[test]
fn a_fall_naming_a_compensating_gain_is_refused_too() {
    assert!(
        !banked(99.0, "decode allocations rise, and what it buys is a linear walk"),
        "a fall was banked because its reason named a gain — the ruling says no reason suffices"
    );
}

/// Banking an improvement is not a trade, so it answers to nobody.
#[test]
fn a_rise_needs_no_gain_named() {
    assert!(banked(50.0, "the decoder got faster"), "a rise was refused for naming no trade");
}
