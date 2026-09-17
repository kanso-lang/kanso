//! The compile rows count the compile, not the line that says it finished.
//!
//! The three gates anchor at `kanso::main` inclusive, and `kanso check`'s
//! `println!("{file}: ok")` is inside that frame. It is 802 instructions all in
//! — 610 in `write_fmt`, 185 in `core::slice::memchr::memrchr`, which is
//! `LineWriter` looking backwards for the last newline in nineteen bytes — and
//! the memrchr half does not reproduce.
//!
//! Measured 2026-09-17: two sittings of one branch, one `cargo fmt` over one
//! test file apart, identical `.text`/`.data`/`.bss`, identical per-process
//! floor, same runner model, all three compile rows exactly thirteen apart.
//! kanso#1474's frame digest found one bucket of thirty-two differing and one
//! frame of the forty in it — memrchr, 185 against 198 — and the frame's cost
//! follows what is printed: a corpus name twenty-four characters longer read 63
//! against 81 on one binary.
//!
//! So `KANSO_QUIET` takes the line out of the measured region, for 0.0022% of
//! the module row. This spec holds both halves: the binary obeys the variable,
//! and every measured run in the three gates sets it.

use std::path::Path;
use std::process::Command;

const GATES: [&str; 3] = [
    "scripts/gates/compile_instructions.sh",
    "scripts/gates/entry_instructions.sh",
    "scripts/gates/library_instructions.sh",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A one-file library, checked the way the gates check a corpus.
fn sample() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso_quiet_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the sample directory is made");
    let f = dir.join("quiet_sample.kso");
    std::fs::write(&f, "pub fn twice n\n  n + n\n").expect("the sample writes");
    f
}

#[test]
fn the_result_line_is_printed_by_default_and_withheld_under_the_variable() {
    let f = sample();
    let run = |quiet: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_kanso"));
        c.arg("check").arg(&f);
        c.env_remove("KANSO_QUIET");
        if quiet {
            c.env("KANSO_QUIET", "1");
        }
        let out = c.output().expect("kanso runs");
        assert!(
            out.status.success(),
            "kanso check failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    let loud = run(false);
    let quiet = run(true);
    let _ = std::fs::remove_file(&f);

    assert!(
        loud.contains(": ok"),
        "the default lost the result line, which is what a person reads: {loud:?}"
    );
    assert_eq!(
        quiet,
        "",
        "KANSO_QUIET left {} bytes on stdout, and every one of them is in the \
         compile rows",
        quiet.len()
    );
}

#[test]
fn every_measured_run_in_the_three_gates_asks_for_quiet() {
    for gate in GATES {
        let s =
            std::fs::read_to_string(root().join(gate)).unwrap_or_else(|e| panic!("{gate}: {e}"));
        let measured: Vec<&str> =
            s.lines().filter(|l| l.contains("valgrind --tool=callgrind")).collect();
        assert!(
            !measured.is_empty(),
            "{gate} runs no callgrind, so this spec is pointed at the wrong file"
        );
        for line in &measured {
            assert!(
                line.contains("KANSO_QUIET=1"),
                "{gate} measures a run that still prints its result line, and \
                 the memrchr under it does not reproduce:\n  {line}"
            );
        }
    }
}
