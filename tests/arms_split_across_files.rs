//! A module's files share their declarations, and a function's arms may sit
//! in different files at different arities. The per-file check counted only
//! the arms in the file it was reading, so toml's 2-argument `lookup` calling
//! its 3-argument arm in the next file was refused with "arms take 2". The
//! merged check, which sees every arm, now counts a split name.

use std::process::Command;

fn kanso(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/split_arms"))
        .output()
        .expect("kanso runs")
}

/// Watched red: "no 3-argument arm of `lookup` (arms take 2)" on both engines.
#[test]
fn an_arm_calls_its_sibling_files_arm_at_another_arity() {
    for engine in [&["run", "runs/main.kso"][..], &["run", "runs/main.kso", "--interp"][..]] {
        let done = kanso(engine);
        assert_eq!(
            String::from_utf8_lossy(&done.stdout),
            "d.p.0\n",
            "{engine:?}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    }
}

/// A count no file's arms take is still refused, and the refusal names every
/// arity the module declares.
#[test]
fn a_count_no_file_takes_is_refused_naming_every_arity() {
    let done = kanso(&["check", "too_many/m"]);
    assert_eq!(
        String::from_utf8_lossy(&done.stderr),
        "error[arity]: no 4-argument arm of `lookup` (arms take 2, 3) (module too_many/m)\n  \
         --> too_many/m/a.kso:2:3\n   2 |   lookup doc path 0 1\n         ^\n"
    );
    assert_eq!(done.status.code(), Some(2));
}
