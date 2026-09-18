//! `ld` reads whatever is already at `-o`, so the codegen gate clears it.
//!
//! Measured on the gate's own corpus, one binary, one machine, the same `ld`
//! command each time, varying only the state of the output path:
//!
//! ```text
//! absent            5,163,341,031   (twice, to the instruction)
//! an empty file     5,163,341,036   (twice, +5)
//! 100 bytes         5,163,343,385
//! 5 MB              5,163,343,385
//! the real binary   5,163,343,385   (+2,354 over absent)
//! ```
//!
//! Three groups, each internally identical to the instruction, and size does
//! not matter once the file is non-empty. Three different object-file names
//! with the output in the same state read identically, so the variable is the
//! prior contents of the output path alone.
//!
//! The gate read the third group by accident: `stage_and_warm` wipes the box
//! and warms both tiers, so the counted build found the warm-up's binary at
//! `-o`. Correct by luck. Dropping or reordering a warm-up would move the row
//! by 2,354 with nothing in the diff to say why, which is the shape of failure
//! the 2026-09-15 normalization ruling exists to stop.
//!
//! This spec reads the script off disk rather than running it, because running
//! it takes minutes under callgrind and what is being asserted is structural:
//! every build the gate performs is preceded by a clear. A spec that ran the
//! gate would assert the number, which the golden already does, and would say
//! nothing about the build the number came from.

fn gate() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/gates/codegen_instructions.sh");
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("the gate reads at {path:?}"))
}

/// Lines that invoke the corpus build, with comments and blanks dropped. Each
/// one is a build whose `-o` path `ld` will inspect.
fn build_lines(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i, l.trim().to_string()))
        .filter(|(_, l)| !l.starts_with('#'))
        .filter(|(_, l)| l.contains("./kanso build pkg/codegen_corpus"))
        .collect()
}

fn clear_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().starts_with('#'))
        .filter(|(_, l)| l.trim() == "clear_output")
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn the_gate_still_builds_the_corpus_in_three_places() {
    let text = gate();
    let builds = build_lines(&text);
    assert_eq!(
        builds.len(),
        3,
        "expected the warm-up, the counted build and the second counted build; found {builds:#?}"
    );
}

#[test]
fn every_corpus_build_is_preceded_by_a_clear() {
    let text = gate();
    let clears = clear_lines(&text);
    assert!(!clears.is_empty(), "the gate defines no clear_output call");
    for (line, source) in build_lines(&text) {
        let nearest = clears.iter().filter(|c| **c < line).max().copied();
        let Some(nearest) = nearest else {
            panic!("the build at line {} has no clear_output before it:\n  {source}", line + 1);
        };
        assert!(
            line - nearest <= 6,
            "the build at line {} is {} lines after the nearest clear_output at line {}, \
             which is far enough that something may run between them:\n  {source}",
            line + 1,
            line - nearest,
            nearest + 1
        );
    }
}

#[test]
fn the_clear_removes_both_of_what_a_build_writes() {
    let text = gate();
    let body = text
        .split_once("clear_output() {")
        .expect("the gate defines clear_output")
        .1
        .split_once('}')
        .expect("clear_output has a body")
        .0;
    for written in ["codegen_corpus", "codegen_corpus.ll"] {
        assert!(
            body.contains(written),
            "clear_output does not remove {written}, which a build writes:\n{body}"
        );
    }
}
