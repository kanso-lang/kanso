//! `measured_on.sh` documents the two-line form in its own header:
//!
//!     # measured-on glibc=2.39-0ubuntu8.8
//!     # measured-on clang=18.1.3
//!
//! Nothing in the tree had ever written it. Every golden named exactly one
//! fact until 2026-09-06, when the instructions vein started depending on the
//! host's clang as well as its glibc and gained a second line. The gate
//! collects the lines with `sed -p`, so `want` came back newline-joined, while
//! `have` is built by a loop that joins with spaces. The two are compared as
//! one string, so a host that matches fact for fact refused itself, and the
//! error printed the two strings looking identical.
//!
//! The second defect is in the same expression. The pattern is a plain prefix
//! strip, so ANY comment line beginning `# measured-on ` becomes fact data --
//! and these goldens carry dated notes in prose. A note opening "measured-on
//! moves to clang 19.1.1 with these rows" made the gate exit 2 on a fact
//! called `moves`. Both reds are host questions answered wrong, on rows that
//! were byte-identical to the golden.

use std::path::{Path, PathBuf};
use std::process::Command;

struct Answer {
    code: i32,
    said: String,
}

fn ask(name: &str, golden_body: &str) -> Answer {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-measured-on-{name}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let golden_at = stage.join("golden.txt");
    std::fs::write(&golden_at, golden_body).expect("the golden writes");

    let out = Command::new("sh")
        .arg(root.join("scripts/gates/measured_on.sh"))
        .arg(&golden_at)
        .current_dir(root)
        .output()
        .expect("the script runs");
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Answer { code: out.status.code().expect("the script exits"), said }
}

/// What this host answers for the two facts, read off the gate's own report
/// rather than derived a second time here. A spec that recomputed the glibc
/// revision would be pinning its own parse instead of the gate's.
fn this_host() -> (String, String) {
    let answer = ask("probe", "# measured-on glibc=0 clang=0\nsome_counter=1\n");
    let tail = answer
        .said
        .lines()
        .find_map(|l| l.split_once("; here "))
        .unwrap_or_else(|| panic!("the gate reports what it read: {}", answer.said))
        .1
        .to_string();
    let mut facts = tail.split_whitespace();
    let glibc = facts.next().expect("a glibc fact").to_string();
    let clang = facts.next().expect("a clang fact").to_string();
    (glibc, clang)
}

/// The form the gate's header documents, on the host that matches it.
#[test]
fn two_measured_on_lines_are_two_facts_not_one_string() {
    let (glibc, clang) = this_host();
    let answer =
        ask("two-lines", &format!("# measured-on {glibc}\n# measured-on {clang}\nsome_counter=1\n"));
    assert_eq!(
        answer.code, 0,
        "this host names both facts, so it compares. The gate's own header \
         documents one fact a line: {}",
        answer.said
    );
}

/// One line or two is a matter of layout, so the two spellings of the same
/// pair have to answer alike.
#[test]
fn one_line_and_two_lines_say_the_same_thing() {
    let (glibc, clang) = this_host();
    let split =
        ask("split", &format!("# measured-on {glibc}\n# measured-on {clang}\nsome_counter=1\n"));
    let joined = ask("joined", &format!("# measured-on {glibc} {clang}\nsome_counter=1\n"));
    assert_eq!(
        split.code, joined.code,
        "the same pair of facts, written two ways, got two answers.\n  split: {}\n  joined: {}",
        split.said, joined.said
    );
}

/// These goldens carry dated notes, and a note may say the word. Prose is not
/// a fact list, and reading it as one refuses a host that matches.
#[test]
fn a_note_that_opens_with_the_phrase_is_prose_not_facts() {
    let (glibc, clang) = this_host();
    let answer = ask(
        "prose",
        &format!(
            "# measured-on {glibc} {clang}\n\
             # measured-on moves to clang 19.1.1 with these rows. A clang-18\n\
             # host writes the fallback and cannot regenerate them.\n\
             some_counter=1\n"
        ),
    );
    assert_eq!(
        answer.code, 0,
        "the note is prose. Reading it as facts asks this host about one \
         called `moves`: {}",
        answer.said
    );
}

/// The tightening must not buy silence. A fact the gate cannot read is still
/// an error, because a typo in a real fact list would otherwise be dropped
/// and the golden compared against fewer facts than it names.
#[test]
fn a_misspelt_fact_is_still_refused() {
    let answer = ask("typo", "# measured-on clnag=19.1.1\nsome_counter=1\n");
    assert_eq!(
        answer.code, 2,
        "`clnag=19.1.1` is shaped like a fact and names nothing this gate \
         reads: {}",
        answer.said
    );
}

/// And a golden whose only `measured-on` line is prose names no host at all,
/// which is the one thing worse than naming the wrong one.
#[test]
fn prose_alone_leaves_the_golden_naming_no_host() {
    let answer = ask("prose-only", "# measured-on moves to clang 19.1.1.\nsome_counter=1\n");
    assert_eq!(answer.code, 1, "no facts were named: {}", answer.said);
    assert!(
        answer.said.contains("names no host"),
        "and the gate says so rather than comparing nothing: {}",
        answer.said
    );
}
