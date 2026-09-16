//! A log paragraph that sends a measured decision to Clay names the ledger it
//! went to.
//!
//! `design/pending-gavels.md` is the single ledger of decisions awaiting Clay.
//! A session cites entries by heading, never by a task id, because task numbers
//! resolve nowhere outside the session that made them — so a decision that is
//! only in the log's middle and in one session's head has gone to nobody.
//!
//! That happened twice in two days. On 2026-09-15 the entry "the per-call
//! floors, mapped after the inlines" closed by measuring a byte-position scan
//! for the JSON escape path — runbench 1,823,814,374 -> 1,801,576,724, a fall
//! of 1.2193 per cent — and said it "goes to Clay with this number and is not
//! built here". No entry was written, and it sat unfiled for a day. The entry
//! that found that miss wrote down the gap it did not close: nothing checked
//! that a paragraph saying a question goes to Clay had an entry to go to. The
//! next day the same file turned up a second instance, in the 2026-09-15
//! count-from-main entry: the `.rodata` page pin, measured on four probe
//! binaries, closing "is Clay's, and goes to him with these numbers rather
//! than to the ledger" — where the ledger is the only channel there is.
//!
//! Two instances a day apart is a defect in the process, not a slip, so the
//! check is built here.
//!
//! The gap paragraph's own objection was that a scan for the phrase "would
//! pass over every historical entry that has since been ruled, so it would
//! either be noisy or would need a list of exemptions that goes stale the way
//! the counts did". Both halves are answered rather than ignored.
//!
//! The noise is answered by what a send carries. A decision that goes to Clay
//! goes with its measurement — the ledger's own filing rule says an entry
//! carries the numbers behind it — so a paragraph is read as a send only when
//! it carries one. That is what separates the two real sends above from the
//! paragraph that merely describes the phrase; the latter names no number and
//! is not a send.
//!
//! The exemption list is answered by not having one. The live log is
//! append-only, so a 2026-09-15 paragraph cannot be edited to cite an entry
//! filed on 2026-09-16 — and it does not have to be. A send is satisfied when
//! the paragraph names the ledger file itself, OR when a later paragraph names
//! it and shares one of the send's own measurements, which is exactly the
//! shape a filing entry takes when it quotes the number it is filing. So a
//! historical send answered later reads as answered, with no list to keep.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The ledger, named the way a citing paragraph spells it.
const LEDGER: &str = "design/pending-gavels.md";

/// The ways the log says a decision is Clay's. Read against a paragraph with
/// its newlines flattened, since the log wraps at eighty columns and a phrase
/// straddles the wrap as often as not.
const SENDS: [&str; 6] =
    ["goes to clay", "go to clay", "goes to him", "is clay's", "are clay's", "to clay with"];

/// A measurement, in the log's own spelling: a grouped number of five figures
/// or more, or a percentage. Anything smaller is a line number or a year.
fn carries_a_measurement(text: &str) -> bool {
    grouped_numbers(text).next().is_some() || text.contains("per cent") || text.contains('%')
}

/// Every grouped number in the text — `1,823,814,374` and the like. These are
/// what a filing paragraph quotes back when it files the send.
fn grouped_numbers(text: &str) -> impl Iterator<Item = String> + '_ {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == ',') {
                i += 1;
            }
            let run: String = bytes[start..i].iter().collect();
            let run = run.trim_end_matches(',').to_string();
            if run.contains(',') && run.chars().filter(char::is_ascii_digit).count() >= 5 {
                out.push(run);
            }
        } else {
            i += 1;
        }
    }
    out.into_iter()
}

fn flattened(paragraph: &str) -> String {
    paragraph.replace('\n', " ").to_ascii_lowercase()
}

#[test]
fn a_question_sent_to_clay_has_a_ledger_entry() {
    let log = std::fs::read_to_string(root().join("design/compiler-log.md"))
        .expect("the live log is there");
    let paragraphs: Vec<&str> = log.split("\n\n").collect();

    let mut unanswered = Vec::new();
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let flat = flattened(paragraph);
        if !SENDS.iter().any(|send| flat.contains(send)) {
            continue;
        }
        if !carries_a_measurement(paragraph) {
            // Prose about the phrase, not a decision travelling with its
            // numbers. The doc comment says why this is the dividing line.
            continue;
        }
        if paragraph.contains(LEDGER) {
            continue;
        }
        // A send filed by a later entry: that entry names the ledger and
        // quotes one of this send's own measurements.
        let mine: Vec<String> = grouped_numbers(paragraph).collect();
        let answered = paragraphs[index + 1..].iter().any(|later| {
            later.contains(LEDGER) && mine.iter().any(|number| later.contains(number.as_str()))
        });
        if !answered {
            unanswered.push(paragraph.trim());
        }
    }

    assert!(
        unanswered.is_empty(),
        "design/compiler-log.md sends {} measured decision(s) to Clay with no \
         entry in {LEDGER} to go to, and no later entry filing them. A decision \
         that waits on him lives in the ledger; the log's middle and a session's \
         task list reach nobody. File each one, with its search and a \
         recommendation, and name the ledger where the send is written:\n\n{}",
        unanswered.len(),
        unanswered
            .iter()
            .map(|p| format!("  ---\n  {}\n", p.replace('\n', "\n  ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
