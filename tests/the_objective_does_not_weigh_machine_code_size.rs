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

use std::path::Path;

fn welfare() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("scripts/welfare/welfare.kso")).expect("welfare reads")
}

#[test]
fn welfare_does_not_read_the_machine_code_vein() {
    let src = welfare();
    assert!(
        !src.contains("text_golden"),
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
    let gate = std::fs::read_to_string(root.join("scripts/gates/machine_code.sh"))
        .expect("the machine-code gate reads");
    assert!(
        gate.contains("bench/text_golden.txt"),
        "scripts/gates/machine_code.sh no longer diffs bench/text_golden.txt, so \
         nothing counts what the compiler emits. The objective does not weigh it \
         and the gate is the only thing that watches it."
    );
    let golden = root.join("bench/text_golden.txt");
    assert!(golden.exists(), "bench/text_golden.txt is gone");
}
