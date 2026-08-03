//! Lowering the objective needs a reason that names what the drop buys.
//!
//! The gate refuses a fall and tells you to argue the weights, but `--set`
//! took any sentence at all in either direction — so a twenty-three point
//! drop could be banked with the word "whatever" and the argument the message
//! asked for never had to happen. That is the Pareto failure the objective
//! exists to forbid: something worse, nothing better, nobody saying so.
//!
//! A RISE is not a trade and stays free to bank. Requiring a gain word there
//! would be asking somebody to justify an improvement, and a rule that fires
//! on both directions is a rule nobody reads.
//!
//! The words are the ones .github/goldens-move-licenses.sh asks for, so one
//! habit serves both gates.

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

/// The one that matters: this is the shape the hole had.
#[test]
fn a_fall_that_names_no_gain_is_refused() {
    assert!(
        !banked(99.0, "whatever"),
        "a twenty-three point fall was banked with a reason that names nothing"
    );
}

#[test]
fn a_fall_that_names_what_it_buys_is_banked() {
    assert!(
        banked(99.0, "decode allocations rise, and what it buys is a linear walk"),
        "a fall stating its compensating gain was refused"
    );
}

/// Reasons are written by hand, and a sentence starts with a capital.
#[test]
fn a_gain_is_read_whatever_its_case() {
    assert!(banked(99.0, "Better decode locality"), "a capitalised gain word was not read");
}

/// Banking an improvement is not a trade, so it answers to nobody.
#[test]
fn a_rise_needs_no_gain_named() {
    assert!(banked(50.0, "the decoder got faster"), "a rise was refused for naming no trade");
}
