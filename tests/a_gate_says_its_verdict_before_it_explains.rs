//! A gate that reaches its verdict and then buries it has not reported it.
//!
//! GitHub keeps at most FIFTY annotations per check run. Each of the four
//! instruction gates emits a dozen `::error::` lines when its row parts, and
//! the cross-run thirteen parts all three compile rows at once. On kanso#1477
//! — whose whole diff is one golden fixture and a log entry, so it cannot
//! reach the compiler at all — the job could be read as far as `a move of -13`
//! and the two lines saying WHICH of the two cases it was fell past the cap.
//! The gate had already taken its second reading. Nobody could see what it
//! said.
//!
//! So the verdict is pinned to the head of what each gate prints: after the
//! three lines naming the row and the size of the move, and before the long
//! explanation of the two cases.

use std::path::Path;

/// The gates that take a second reading and report which case it is.
const GATES: [&str; 4] = [
    "scripts/gates/compile_instructions.sh",
    "scripts/gates/entry_instructions.sh",
    "scripts/gates/library_instructions.sh",
    "scripts/gates/codegen_instructions.sh",
];

fn read(name: &str) -> Option<String> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read_to_string(p).ok()
}

#[test]
fn the_verdict_comes_before_the_explanation() {
    let mut checked = 0;
    for gate in GATES {
        let Some(text) = read(gate) else { continue };
        let verdict = text.find("VERDICT (1)").unwrap_or_else(|| {
            panic!("{gate} takes a second reading but prints no `VERDICT (1)` line")
        });
        let alternative = text
            .find("VERDICT (2)")
            .unwrap_or_else(|| panic!("{gate} prints `VERDICT (1)` and no `VERDICT (2)`"));
        let explanation = text
            .find("(1) THE CHANGE UNDER TEST MOVED IT")
            .unwrap_or_else(|| panic!("{gate} has no two-case explanation to come after"));
        assert!(
            verdict < explanation && alternative < explanation,
            "{gate} explains the two cases at byte {explanation} before it says which one it \
             is at {verdict}/{alternative}. Three rows failing at a dozen lines each puts the \
             verdict past GitHub's fifty-annotation cap, which is how kanso#1477 reported a \
             second reading nobody could read."
        );
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} gates were found to check");
}

#[test]
fn the_second_reading_is_a_notice_not_only_stdout() {
    // stdout reaches the job log and the artifact, and both of those have to
    // be fetched. An annotation comes back over the ordinary API, which is the
    // one path a reader always has.
    let mut checked = 0;
    for gate in GATES {
        let Some(text) = read(gate) else { continue };
        if !text.contains("VERDICT (1)") {
            continue;
        }
        assert!(
            text.contains("::notice::") && text.contains("_again="),
            "{gate} prints its second reading to stdout alone; it needs a `::notice::` \
             carrying `_again=` so the number survives as an annotation"
        );
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} gates were found to check");
}
