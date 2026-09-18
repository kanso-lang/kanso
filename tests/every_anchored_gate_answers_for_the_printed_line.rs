//! Every gate that anchors an inclusive frame either subtracts the printed
//! line or says in the script why it does not.
//!
//! `std::io::stdio::_print`'s subtree ends in `memrchr` over the formatted
//! bytes, and what that frame costs moves with the binary's LAYOUT rather than
//! with anything the program under test does. kanso#1487 measured it: five CI
//! builds on 2026-09-17, across trees whose compiler source was identical,
//! drew two faces thirteen apart on the module, entry and library rows, while
//! start-up -- whose gate prints nothing -- gave one value on all five. The
//! 2026-09-15 rule says a term that cannot be normalized is excluded and the
//! exclusion is named.
//!
//! That fix landed on three gates and missed the fourth. `interp_instructions`
//! kept drawing the same two faces for a day: kanso#1486's two sittings read
//! 2,182,526,878 and 2,182,526,865 on trees whose only difference was three
//! goldens, a page and a log entry. Nothing could see it, because the property
//! lived in three scripts and in no check.
//!
//! So the property is read off disk here rather than remembered. A gate that
//! anchors a frame and prints nothing is fine -- it says so, in the words
//! below, and the sentence is the thing this spec makes it write down.

use std::path::PathBuf;

/// The marker a gate writes when its program prints nothing to stdout, so the
/// exemption is a sentence somebody chose rather than a silence.
const EXEMPT: &str = "PRINTS NOTHING TO STDOUT";

/// What subtracting the line looks like: the frame is read out of the profile.
const SUBTRACTS: &str = ":std::io::stdio::_print \\[";

/// A gate anchors when it reads one frame's inclusive cost as the whole row.
const ANCHORS: &str = "--inclusive=yes --threshold=100";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn a_gate_that_anchors_a_frame_answers_for_the_printed_line() {
    let dir = root().join("scripts/gates");
    let mut anchored = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the gates directory is there") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("a gate script reads");
        if !src.contains(ANCHORS) {
            continue;
        }
        let name = path.file_name().expect("a file name").to_string_lossy().into_owned();
        anchored.push((name, src));
    }

    assert!(
        anchored.len() >= 5,
        "only {} gate(s) anchor an inclusive frame, which is fewer than the five \
         this property was written against -- if a gate stopped anchoring, say so \
         here; if this test stopped finding them, it is the test that is broken",
        anchored.len()
    );

    let mut silent = Vec::new();
    for (name, src) in &anchored {
        if !src.contains(SUBTRACTS) && !src.contains(EXEMPT) {
            silent.push(name.clone());
        }
    }

    assert!(
        silent.is_empty(),
        "these gates anchor an inclusive frame and neither subtract \
         `std::io::stdio::_print` nor say why they need not: {silent:?}. \
         A row that counts a printed line counts the binary's layout with it. \
         Either take the line off the way compile_instructions.sh does, or \
         write `{EXEMPT}` into the script beside the reason."
    );
}
