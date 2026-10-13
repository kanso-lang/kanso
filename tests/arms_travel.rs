//! A value carries the arms its type's owner wrote (the 2026-10-12 gavel on
//! #1894, rule 1). `report` never imports `geo`, and calls `area` bare on a
//! circle it was handed and on each circle of a list it was handed. Both
//! engines print what geo's arm answers. Main refused both calls with
//! `unknown name `area``, from `kanso run` and from `kanso check report`.
//!
//! A module that writes the name for a type of its own keeps its arm and is
//! offered the owners' arms of every other shape (rule 4): `report` in
//! `tests/golden/travel/shade` writes `area` for a square, is handed a circle
//! and a square, and each takes its owner's arm. Main and rule 1 alone
//! stopped at run time with "no overload of `report/area` matches".
//!
//! A module a program imports is checked before the program fills its
//! shaded groups, so it is checked with each such call back on its own group.
//! In `tests/golden/travel/told`, `told?` is reached only by a call on a
//! parameter; checked as shaded, it read as never called, its arm answering
//! `b` answered nothing, and the `?` rule refused a group that answers a
//! boolean.
//!
//! The refusals are in the error corpus beside this: an arm called on a value
//! the file made (`tests/golden/errors_module/an_arm_called_on_a_value_the_file_made`),
//! a name no owner writes (`.../a_traveling_call_no_owner_answers`), and two
//! owners' arms tied on one call (`.../two_owners_tie_on_a_traveling_call`).

use std::process::Command;

fn kanso(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

#[test]
fn an_arm_travels_into_a_library_that_never_imported_its_owner() {
    for engine in [&[][..], &["--interp"][..]] {
        let mut args = vec!["run", "tests/golden/travel/area"];
        args.extend_from_slice(engine);
        let output = kanso(&args);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "12\n[3 27]\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "engine {engine:?}");
    }
}

#[test]
fn a_local_arm_shadows_only_the_shape_it_writes() {
    for engine in [&[][..], &["--interp"][..]] {
        let mut args = vec!["run", "tests/golden/travel/shade"];
        args.extend_from_slice(engine);
        let output = kanso(&args);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "12\n25\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "engine {engine:?}");
    }
}

#[test]
fn a_module_is_checked_with_its_own_calls_at_home() {
    let output = kanso(&["run", "tests/golden/travel/told"]);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "[\"yes\" false]\n",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success());
}

/// Checked alone, the library cannot see which owners its values will
/// carry, so it takes the call on a value it was handed. The program that
/// hands the value over is where an owner's arm is found or refused.
#[test]
fn a_library_checked_alone_takes_a_call_on_a_value_it_was_handed() {
    let output = kanso(&["check", "tests/golden/travel/area/report"]);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "tests/golden/travel/area/report: ok\n",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success());
}
