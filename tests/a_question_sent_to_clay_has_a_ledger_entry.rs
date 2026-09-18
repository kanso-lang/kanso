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

/// THE THIRD STATE, and the reason the ledger's name alone is not enough.
///
/// A send has three ends, not two. It is filed and open; or filed and ruled;
/// or BOUNCED -- sent out of the ledger unruled, because the question turned
/// out to have no surface area a program could see, and by the 2026-08-29
/// ruling such a question is the implementer's rather than Clay's. A bounce
/// has no ledger entry BY CONSTRUCTION, so a rule that demands the ledger's
/// name can never be satisfied by one.
///
/// Gavel #159 is the worked example: the `lex_word` send asks whether a
/// `String` should exist at all, and the inline-name entry bounced it on
/// 2026-08-29. Demanding a ledger entry for it would ask a session to file a
/// question that was deliberately taken off the docket.
///
/// So a later paragraph answers a send when it quotes one of the send's own
/// measurements AND either names the ledger or records the bounce. The
/// measurement is what ties the answer to the send; without it "bounced"
/// anywhere in the log would excuse everything.
const BOUNCED: &str = "bounced";

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

/// What ties an answer to the send it answers.
///
/// This has to accept exactly what `carries_a_measurement` accepts, and for a
/// while it did not. That function counts a bare percentage as a measurement,
/// so a send whose only number is `27.6%` is a send; but the tie-back read
/// comma-grouped integers alone, so `mine` came back empty and the "filed by a
/// later entry" escape could never fire for it. Such a send could only ever be
/// satisfied by naming the ledger in its OWN paragraph -- which is the one
/// thing a send written before the rule existed cannot go back and do.
///
/// The escapebench send is the worked example: measured at 27.6%, filed in the
/// ledger under its own heading, answered by a paragraph naming the ledger and
/// quoting 27.6%, and still reported unanswered.
fn measurements(text: &str) -> Vec<String> {
    let mut out: Vec<String> = grouped_numbers(text).collect();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && chars[i] == '%' {
                let run: String = chars[start..=i].iter().collect();
                out.push(run);
            }
        } else {
            i += 1;
        }
    }
    out
}

#[test]
fn a_question_sent_to_clay_has_a_ledger_entry() {
    // BOTH FILES, ARCHIVE FIRST. The log holds the last forty entries and
    // the rest moves to design/log/compiler-log-archive.md unedited, so a
    // spec reading only the live file stops checking a send the moment the
    // trim walks past it -- and it silently stops being able to fail, which
    // is the worst way for a spec to go quiet. The archive move that came
    // with this branch is what surfaced it: it carried the paragraph the
    // ratchet's own mutation anchors on out of the live file, and the
    // mutation went stale rather than red.
    //
    // Archive first because the "filed by a later entry" rule reads forward,
    // and the archive is by construction older than everything live. The two
    // are joined with a blank line so no paragraph straddles the seam.
    let archive = std::fs::read_to_string(root().join("design/log/compiler-log-archive.md"))
        .expect("the archive is there");
    let live = std::fs::read_to_string(root().join("design/compiler-log.md"))
        .expect("the live log is there");
    let log = format!("{}\n\n{}", archive.trim_end(), live);
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
        let mine: Vec<String> = measurements(paragraph);
        let answered = paragraphs[index + 1..].iter().any(|later| {
            (later.contains(LEDGER) || later.to_ascii_lowercase().contains(BOUNCED))
                && mine.iter().any(|number| later.contains(number.as_str()))
        });
        if !answered {
            unanswered.push(paragraph.trim());
        }
    }

    assert!(
        unanswered.is_empty(),
        "the log and its archive send {} measured decision(s) to Clay with no \
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
