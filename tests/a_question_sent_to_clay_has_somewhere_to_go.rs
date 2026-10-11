//! A log paragraph that sends a measured decision to Clay names where it went.
//!
//! Until 2026-10-10 the only place a waiting decision could go was
//! `design/pending-gavels.md`. Since then it is a GitHub issue labelled
//! `needs-clay` (kanso#1852), and the ledger is retired. A send written before
//! the change names the ledger; one written after names the label. Either
//! answers it, and a send that names neither has gone to nobody, because a
//! question left in a log paragraph is one nobody sees.
//!
//! That happened twice in two days. On 2026-09-15 the entry "the per-call
//! floors, mapped after the inlines" closed by measuring a byte-position scan
//! for the JSON escape path — runbench 1,823,814,374 -> 1,801,576,724, a fall
//! of 1.2193 per cent — and said it "goes to Clay with this number and is not
//! built here". Nothing was filed, and it sat for a day. The next day the same
//! file turned up a second instance, in the 2026-09-15 count-from-main entry:
//! the `.rodata` page pin, measured on four probe binaries, closing "is
//! Clay's, and goes to him with these numbers rather than to the ledger" —
//! where the ledger was the only channel there was.
//!
//! A scan for the phrase alone would be noisy, and a list of exemptions would
//! go stale. Neither is needed.
//!
//! The noise is answered by what a send carries. A decision that goes to Clay
//! goes with its measurement, so a paragraph is read as a send only when it
//! carries one. That separates the two real sends above from a paragraph that
//! merely describes the phrase.
//!
//! The exemption list is answered by reading forward. The log is append-only,
//! so a 2026-09-15 paragraph cannot be edited to cite something filed on
//! 2026-09-16, and it does not have to be. A send is satisfied when its own
//! paragraph names a channel, or when a later paragraph names one and quotes
//! one of the send's own measurements, which is the shape a filing takes when
//! it quotes the number it files.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The places a waiting decision goes, spelled the way a citing paragraph
/// spells them: the ledger for a send written before 2026-10-10, and the issue
/// label for one written since.
const CHANNELS: [&str; 2] = ["design/pending-gavels.md", "needs-clay"];

fn names_a_channel(text: &str) -> bool {
    CHANNELS.iter().any(|channel| text.contains(channel))
}

/// THE THIRD STATE, and the reason a channel's name alone is not enough.
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
fn a_question_sent_to_clay_has_somewhere_to_go() {
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
        if names_a_channel(paragraph) {
            continue;
        }
        // A send filed by a later entry: that entry names a channel and
        // quotes one of this send's own measurements.
        let mine: Vec<String> = measurements(paragraph);
        let answered = paragraphs[index + 1..].iter().any(|later| {
            (names_a_channel(later) || later.to_ascii_lowercase().contains(BOUNCED))
                && mine.iter().any(|number| later.contains(number.as_str()))
        });
        if !answered {
            unanswered.push(paragraph.trim());
        }
    }

    assert!(
        unanswered.is_empty(),
        "the log and its archive send {} measured decision(s) to Clay that name \
         no `needs-clay` issue, and no later entry files them. A decision that \
         waits on him is a `needs-clay` issue carrying its search, its options \
         and a recommendation; the log's middle and a session's task list reach \
         nobody. Open one for each and name it where the send is written:\n\n{}",
        unanswered.len(),
        unanswered
            .iter()
            .map(|p| format!("  ---\n  {}\n", p.replace('\n', "\n  ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
