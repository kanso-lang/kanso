//! The interpreter looks a called name up once, however often it is called.
//!
//! A call through a function reference used to hash the name's text, about
//! a hundred instructions of every interpreted call. The answer is kept in
//! a table, and a declaration's frame in another. Until 2026-11-11 both
//! tables took their slots from addresses: of the name's text, and of the
//! declaration. Two names shared a slot when their texts happened to be
//! allocated a multiple of the table apart, and each then put the other out
//! on every call. Where a text lands depends on everything allocated before
//! it, so a change to the front end alone moved how often the tables missed.
//! Two such changes in a row moved the interpreter's instruction count by
//! 723,752 and by 105,487 (kanso#2444).
//!
//! `resolve` now numbers a name the first time the run meets it, and a
//! declaration's place in `program.fns` is its number. Each table is
//! indexed by the number, so no two share a slot and nothing about an
//! address decides a slot.
//!
//! WHAT IS PINNED is the count of lookups by name and of frames made, which
//! the interpreter prints under KANSO_COUNTERS. Both are counted where a
//! table is filled, so a call the table answers pays nothing for them. The
//! program calls eight names. Three hundred laps and three thousand read
//! the same eight.

use std::path::PathBuf;
use std::process::Command;

fn program(laps: u32) -> String {
    format!(
        r#"import "std/list"

fn double x
  x * 2

fn bump x
  x + 1

fn both x
  bump (double x)

fn walk 0 acc
  acc

fn walk n acc
  walk (n - 1) (acc + both n)

print "{{walk {laps} 0}}"
print "{{list/sum (list/map [1 2 3] both)}}"
"#
    )
}

/// What the run printed, and one of its counters.
fn ran(laps: u32) -> (String, Vec<(String, u64)>) {
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-looked-up-once-{laps}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a temp directory");
    std::fs::write(stage.join("main.kso"), program(laps)).expect("the program is written");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", "main.kso", "--interp"])
        .env("KANSO_COUNTERS", "1")
        .current_dir(&stage)
        .output()
        .expect("kanso runs");
    assert!(out.status.success(), "the run failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let counted = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter_map(|(key, value)| Some((key.to_string(), value.trim().parse().ok()?)))
        .collect();
    (String::from_utf8_lossy(&out.stdout).to_string(), counted)
}

fn counter(counted: &[(String, u64)], name: &str) -> u64 {
    counted
        .iter()
        .find(|(key, _)| key == name)
        .unwrap_or_else(|| panic!("the run reports no `{name}`"))
        .1
}

#[test]
fn a_name_called_three_thousand_times_is_looked_up_once() {
    let (few_out, few) = ran(300);
    let (many_out, many) = ran(3_000);
    assert_eq!(few_out, "90600\n15\n");
    assert_eq!(many_out, "9006000\n15\n");
    for (laps, counted) in [(300, &few), (3_000, &many)] {
        assert_eq!(
            counter(counted, "interp_callee_lookups"),
            8,
            "{laps} laps: the program calls eight names, and each is looked up by its \
             name the first time it is called. A count that grows with the laps is a \
             table that stopped answering."
        );
        assert_eq!(
            counter(counted, "interp_frames_made"),
            8,
            "{laps} laps: the run enters eight declarations, and each is given its \
             frame the first time it is entered."
        );
    }
}
