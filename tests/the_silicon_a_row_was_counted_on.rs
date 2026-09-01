//! The instruction veins count what a process executes, and glibc resolves
//! memcpy, memcmp, strlen and their neighbours by ifunc at load time, reading
//! CPU features. So one libc runs different code on different CPUs, and two
//! rows counted on different silicon are not comparable. kq#85 spent a whole
//! pull request establishing that by hand: four rows moved 0.06% to 0.10%
//! between two runs whose every printed version agreed to the commit hash, and
//! the only difference was the Azure region.
//!
//! The first fix for that was a pin — record one host's feature block, refuse
//! anywhere else — and CI killed it in two runs. The runner pool is not one
//! CPU: an AMD EPYC Zen 3 (family 0x19, model 0x1), an Intel Ice Lake-SP
//! (0x6/0x6a), against the Cascade Lake (0x6/0x55) this was written on. A
//! check refusing every run but one is red for a reason no pull request
//! causes.
//!
//! So `scripts/gates/dispatch.sh` never refuses. It answers, and the gates ask
//! only about a row that already moved — a row landing on its recorded value
//! is right whatever counted it. These pin the three answers, because a gate
//! reading the wrong one either blames the silicon for a real regression or
//! blames a pull request for the pool.

use std::path::Path;
use std::process::Command;

/// This host's block, filtered the way the script filters it.
fn this_hosts_block() -> String {
    let out = Command::new("/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2")
        .arg("--list-diagnostics")
        .output()
        .expect("the loader reports its diagnostics");
    let held = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut lines: Vec<&str> = held
        .lines()
        .filter(|l| l.starts_with("x86.cpu_features"))
        .filter(|l| !l.contains("features[0x0].cpuid[0x1]="))
        .collect();
    lines.sort_unstable();
    format!("{}\n", lines.join("\n"))
}

/// Run the script and answer its exit code and everything it said. `ci` sets
/// GITHUB_ACTIONS the way a runner would.
fn asked(verb: &str, block: Option<&str>, key: &str, ci: bool) -> (i32, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-silicon-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let at = stage.join("dispatch.txt");
    if let Some(text) = block {
        std::fs::write(&at, text).expect("the block writes");
    }

    let mut run = Command::new("sh");
    run.arg(root.join("scripts/gates/dispatch.sh")).arg(verb).arg(&at);
    if ci {
        run.env("GITHUB_ACTIONS", "true");
    } else {
        run.env_remove("GITHUB_ACTIONS");
    }
    let done = run.output().expect("the script runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    (done.status.code().expect("it exited"), said)
}

/// Doctor one line the header prices: Prefer_ERMS selects a different memcpy.
fn one_line_off() -> String {
    let held = this_hosts_block();
    let doctored = held.replacen(
        "x86.cpu_features.preferred.Prefer_ERMS=0x0",
        "x86.cpu_features.preferred.Prefer_ERMS=0x1",
        1,
    );
    assert_ne!(doctored, held, "Prefer_ERMS moved; this fixture needs rewriting");
    doctored
}

#[test]
fn every_run_names_the_cpu_it_is_about_to_count_on() {
    let (code, said) = asked("name", None, "name", false);
    assert_eq!(code, 0, "naming a cpu never refuses: {said}");
    assert!(
        said.contains("family") && said.contains("model"),
        "and names it by family and model, which is what a job log needs to \
         make the next divergence one line instead of an afternoon: {said}"
    );
}

#[test]
fn the_recorded_silicon_answers_nought() {
    let (code, said) = asked("differs", Some(&this_hosts_block()), "same", false);
    assert_eq!(code, 0, "this host matches the block it was made from: {said}");
}

#[test]
fn other_silicon_answers_one_and_names_the_lines() {
    let (code, said) = asked("differs", Some(&one_line_off()), "other", false);
    assert_eq!(code, 1, "a differing block is answer 1: {said}");
    assert!(
        said.contains("Prefer_ERMS=0x1") && said.contains("Prefer_ERMS=0x0"),
        "and names the line that differs, both ways round: {said}"
    );
}

/// Answer 2 is the one that keeps the gates honest with nothing recorded: the
/// vein gates exactly as it always did rather than excusing a moved row on a
/// silicon nobody wrote down.
#[test]
fn nothing_recorded_answers_two_rather_than_excusing_anything() {
    let (code, said) = asked("differs", None, "absent", false);
    assert_eq!(code, 2, "no block is answer 2, never 0 or 1: {said}");

    let (empty_code, empty_said) =
        asked("differs", Some("# only a header, no block\n"), "empty", false);
    assert_eq!(
        empty_code, 2,
        "and a file of nothing but comments is the same case: {empty_said}"
    );
}

/// measured_on.sh's header records why: a container printed a diff once,
/// somebody pasted, and the container's numbers went into a golden over the
/// runner's. Handing this box its own block back holds that door open.
#[test]
fn the_pasteable_block_prints_in_ci_and_nowhere_else() {
    let off = one_line_off();
    let (_, here) = asked("differs", Some(&off), "nopaste", false);
    let (_, in_ci) = asked("differs", Some(&off), "paste", true);

    assert!(
        !here.contains("should a fresh sitting record it"),
        "off the runner it offers nothing to paste: {here}"
    );
    assert!(
        here.contains("prints only in CI"),
        "and says why, so a reader is not left guessing: {here}"
    );
    assert!(
        in_ci.contains("should a fresh sitting record it"),
        "on the runner that block is the whole point: {in_ci}"
    );
    assert!(
        in_ci.lines().filter(|l| l.starts_with("x86.cpu_features")).count() > 100,
        "and it prints the whole block: {in_ci}"
    );
}

/// A verb it does not know is a caller's mistake, and answering 0 or 1 to one
/// would have a gate act on an answer nobody gave.
#[test]
fn a_verb_it_does_not_know_is_neither_yes_nor_no() {
    let (code, said) = asked("refuse", None, "verb", false);
    assert_eq!(code, 2, "an unknown verb answers 2: {said}");
    assert!(said.contains("refuse"), "and names what it was asked: {said}");
}

/// A block may only be taken from a run that BOTH names its cpu and matches
/// every row — and a run that matches never reaches `differs`, so it would
/// never print one. That is a bootstrap with no first step, and this is the
/// step: while no block is recorded, CI prints the whole thing. It stops the
/// moment the file exists, because 123 lines on every green run is noise.
#[test]
fn ci_prints_a_whole_block_only_while_none_is_recorded() {
    let (_, bare) = asked("name", None, "boot-here", false);
    assert!(
        !bare.contains("x86.cpu_features"),
        "off the runner, naming a cpu prints a cpu and not a block: {bare}"
    );

    let (_, needed) = asked("name", None, "boot-ci", true);
    assert!(
        needed.lines().filter(|l| l.starts_with("x86.cpu_features")).count() > 100,
        "in CI with nothing recorded, the whole block, or no run ever \
         produces one: {needed}"
    );

    let (_, held) = asked("name", Some(&this_hosts_block()), "boot-done", true);
    assert!(
        !held.contains("x86.cpu_features"),
        "and once a block is recorded it stops, so green runs stay quiet: \
         {held}"
    );
    assert!(
        held.contains("family") && held.contains("model"),
        "the cpu is still named every run: {held}"
    );
}
