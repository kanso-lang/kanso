//! STATUS.md's count of the gavel ledger is read off the ledger, not recalled.
//!
//! `design/pending-gavels.md` is the single ledger of decisions awaiting Clay,
//! and STATUS.md indexes it with a sentence that names how many questions wait
//! and how they split between blocking and not. That sentence is maintained by
//! hand, and it has now gone stale twice in four days.
//!
//! On 2026-09-05 it said nineteen while the ledger held three: fifteen had been
//! gaveled and had left the file as the lifecycle requires, and the count was
//! carried forward instead of recounted. It was recounted that day, and by
//! 2026-09-08 it said six and "none blocking" while the ledger held three
//! entries under a heading named Blocking and one under Open — four of the six
//! it named having been ruled or, in the assert hako's case, built.
//!
//! A session reading a stale index reports the queue wrongly to the person the
//! queue is for, which is what happened on 2026-09-08. So the sentence is
//! pinned to the file it describes: the ledger's `###` headings are counted
//! under each `##` section, and this test fails when the two disagree. The
//! ledger stays the source of truth; STATUS.md only has to agree with it.
//!
//! The second test guards the hole that made the first one incomplete. On
//! 2026-09-08 the ledger's Parked section — by its own heading "on the record,
//! no action" — carried a full entry appended under it with no `###` heading of
//! its own: a measurement offered to Clay, closing with "this entry is where
//! that goes". Nothing could reach it. Sessions cite entries by heading, and
//! STATUS.md indexes by heading, so an unheaded entry is filed and invisible at
//! once, and the count above stays right while the ledger is wrong. Parked is a
//! list of one-liners; anything longer belongs under a heading in one of the
//! two live sections, or in the log.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const SPELLED: [&str; 21] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
];

/// How many `### ` headings stand under the `## ` section whose heading starts
/// with `prefix`. Counting `###` across the whole file would also sweep in the
/// Stale and Parked sections, which are the record of what is NOT waiting.
fn entries_under(ledger: &str, prefix: &str) -> usize {
    let mut inside = false;
    let mut seen = 0;
    for line in ledger.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            inside = heading.starts_with(prefix);
            continue;
        }
        if inside && line.starts_with("### ") {
            seen += 1;
        }
    }
    seen
}

/// The word standing immediately before a phrase, lowercased and stripped of
/// the bold markers STATUS.md writes its lead-ins in.
fn word_before(text: &str, phrase: &str) -> String {
    text.split_once(phrase)
        .unwrap_or_else(|| panic!("STATUS.md says {phrase:?}"))
        .0
        .split_whitespace()
        .next_back()
        .expect("a word stands before it")
        .trim_matches('*')
        .to_ascii_lowercase()
}

/// Parked is bullets. Prose appended there is an entry nothing can cite.
#[test]
fn the_parked_section_holds_only_its_list() {
    let ledger =
        std::fs::read_to_string(root().join("design/pending-gavels.md")).expect("the ledger reads");
    let parked = ledger.split_once("## Parked").expect("the ledger has a Parked section").1;
    let stray: Vec<&str> = parked
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| {
            let t = l.trim();
            // a bullet's wrapped continuation is indented; an appended entry
            // starts at column zero, which is the shape this catches
            !t.is_empty() && !t.starts_with("- ") && !l.starts_with(' ')
        })
        .collect();
    assert!(
        stray.is_empty(),
        "design/pending-gavels.md's Parked section holds prose that is not part of \
         its list: {stray:?}. Parked is one line per parked item; an entry appended \
         here has no `###` heading, so no session citing by heading and no index \
         can reach it — give it a heading under Blocking or Open, or move it to the \
         log"
    );
}

#[test]
fn the_status_index_counts_the_ledger() {
    let ledger =
        std::fs::read_to_string(root().join("design/pending-gavels.md")).expect("the ledger reads");
    let status = std::fs::read_to_string(root().join("STATUS.md")).expect("STATUS.md reads");

    let blocking = entries_under(&ledger, "Blocking");
    let open = entries_under(&ledger, "Open, not blocking");
    let total = blocking + open;

    // The sentence wraps, so the split is read from the whitespace-normalised
    // paragraph rather than from one line.
    let flat = status.split_whitespace().collect::<Vec<_>>().join(" ");

    let claimed_total = word_before(&flat, "questions wait in `design/pending-gavels.md`");
    let claimed_blocking = word_before(&flat, "blocking,");
    let claimed_open = word_before(&flat, "open — each with a recommendation");

    for (claimed, counted, what) in [
        (claimed_total, total, "questions in total"),
        (claimed_blocking, blocking, "blocking"),
        (claimed_open, open, "open, not blocking"),
    ] {
        let want = SPELLED.get(counted).copied().unwrap_or("");
        assert!(
            claimed == want || claimed == counted.to_string(),
            "design/pending-gavels.md holds {counted} {what} and STATUS.md calls them \
             {claimed:?} — the index is maintained by hand and has gone stale twice, \
             so it is pinned to the ledger rather than trusted"
        );
    }
}
