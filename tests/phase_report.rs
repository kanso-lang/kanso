//! `KANSO_PHASES` is a diagnostic, so the one thing it must never do is change
//! what a compile answers or cost anything when nobody asked for it.

use std::process::Command;

fn kanso() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
}

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_phase_report_is_silent_unless_it_is_asked_for() {
    let out = kanso().current_dir(root()).args(["check", "lib/json"]).output().expect("runs");
    let said = String::from_utf8_lossy(&out.stderr);

    assert!(!said.contains(" ms"), "a compile nobody asked to profile printed timings: {said}");
}

#[test]
fn the_phase_report_names_every_phase_it_watches() {
    let out = kanso()
        .current_dir(root())
        .env("KANSO_PHASES", "1")
        .args(["check", "lib/json"])
        .output()
        .expect("runs");
    let said = String::from_utf8_lossy(&out.stderr);

    for phase in ["lex", "parse", "load_dependencies", "infer", "check_merged"] {
        assert!(said.contains(phase), "the report skipped `{phase}`:\n{said}");
    }
}

#[test]
fn profiling_does_not_change_what_a_compile_answers() {
    let plain = kanso().current_dir(root()).args(["check", "lib/json"]).output().expect("runs");
    let watched = kanso()
        .current_dir(root())
        .env("KANSO_PHASES", "1")
        .args(["check", "lib/json"])
        .output()
        .expect("runs");

    assert_eq!(plain.stdout, watched.stdout);
}
