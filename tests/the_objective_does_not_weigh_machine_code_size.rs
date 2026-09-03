//! `bench/text_golden.txt` is the one deterministic vein the welfare model does
//! not read, and that is deliberate.
//!
//! It looks exactly like the gap kanso#1215 closed: eleven benchmarks measured
//! by a gate, pinned in a golden, watched by the trend gate, and weighed at
//! nothing by the objective. The obvious repair is to give code size a term.
//!
//! kanso#1217 is why not. The index twin took encodebench's `.text` DOWN 144
//! bytes and its instruction count UP 67,116,000 — 0.966% — in the same
//! change, and inside `d_list/fold_3` the effect is starker: 4,083 bytes to
//! 3,682, four hundred and one bytes less code running fourteen per cent more
//! instructions, because a four-way specialisation was lost. A term that
//! rewarded smaller `.text` would have scored that regression as a gain,
//! twice.
//!
//! Code size is a diagnostic here, not a cost. It says a kernel arrived or
//! left — which is what `scripts/gates/machine_code.sh` exists for, and what
//! caught the bit twins landing on digestbench when every other row held. What
//! it does not do is stand in for what a program costs to run, and on this
//! corpus it has been measured pointing the wrong way.
//!
//! kanso#1137 is why this is a spec and not a comment: four claims in this
//! tree rested on prose and none of them held. If someone gives `.text` a
//! weight, this goes red and they read the paragraph above before deleting it.
//!
//! And on 2026-09-03 this file joined that list as the sixth. Both halves
//! below asked whether a file CONTAINED a string, so the gate half passed with
//! `scripts/gates/machine_code.sh` no longer reading the golden at all: strip
//! the host check and the diff, leave the sentence inside its `::error::`
//! message, and the spec certified as fine exactly the state its own failure
//! text describes. Both halves read operative lines now — comments dropped,
//! and lines whose whole job is to print a diagnostic dropped with them.
//!
//! Reading lines is still not running the gate, and the behavioural proof is
//! the ratchet's `machine_code` row, which applies a defect nightly and
//! refuses a gate that stayed green. This is the cheap per-pull-request half
//! of that same split.

use std::path::Path;

/// The lines of a script that DO something. A mention inside a comment or a
/// diagnostic is a mention, and this file exists because a mention was taken
/// for a use.
fn operative(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .filter(|line| !line.starts_with("echo") && !line.starts_with("printf"))
        .collect()
}

fn worked_on(text: &str, what: &str) -> bool {
    operative(text).iter().any(|line| line.contains(what))
}

fn welfare() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("scripts/welfare/welfare.kso")).expect("welfare reads")
}

fn machine_code_gate() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("scripts/gates/machine_code.sh"))
        .expect("the machine-code gate reads")
}

#[test]
fn welfare_does_not_read_the_machine_code_vein() {
    let src = welfare();
    assert!(
        !worked_on(&src, "text_golden"),
        "the welfare model now reads bench/text_golden.txt. Machine-code size is \
         deliberately outside the objective: kanso#1217 measured encodebench's \
         `.text` falling 144 bytes while its instruction count rose 67,116,000, \
         so a term rewarding smaller code would have scored that regression as a \
         gain. If the exclusion is being reversed on purpose, the reason above \
         is what has to be argued with — delete this spec in the same commit \
         that adds the term, and say why in design/compiler-log.md."
    );
}

/// The other half: the vein is still measured and still pinned. An exclusion
/// from the objective is not permission to stop counting the thing, and this
/// would go red if `.text` quietly left the tree instead of leaving the model.
#[test]
fn the_machine_code_vein_is_still_gated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = machine_code_gate();
    assert!(
        worked_on(&gate, "bench/text_golden.txt"),
        "scripts/gates/machine_code.sh no longer diffs bench/text_golden.txt, so \
         nothing counts what the compiler emits. The objective does not weigh it \
         and the gate is the only thing that watches it."
    );
    let golden = root.join("bench/text_golden.txt");
    assert!(golden.exists(), "bench/text_golden.txt is gone");
}

/// The counterexample that made this file's earlier shape untrue, kept so the
/// check above cannot quietly return to it. `scripts/gates/machine_code.sh`
/// names the golden three times: the host check, the diff, and one line of its
/// `::error::` message. Take the first two away and the gate reads nothing,
/// which is precisely what the assertion above claims to refuse.
#[test]
fn a_gate_that_only_names_the_golden_in_its_error_text_is_refused() {
    let gutted: String = machine_code_gate()
        .lines()
        .map(|line| {
            let bare = line.trim_start();
            let prints = bare.starts_with("echo") || bare.starts_with("printf");
            if line.contains("bench/text_golden.txt") && !prints {
                "# taken away"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        gutted.contains("bench/text_golden.txt"),
        "the error message still names it — that is the case this test is about"
    );
    assert!(
        !worked_on(&gutted, "bench/text_golden.txt"),
        "a gate naming the golden only in a diagnostic must not read as gating it"
    );
}
