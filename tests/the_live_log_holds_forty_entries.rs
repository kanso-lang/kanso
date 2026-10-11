//! design/compiler-log.md holds the last forty entries, and every older one is
//! in design/log/compiler-log-archive.md, unedited and in order.
//!
//! The live file is read at its tail and appended on every change, so it grows
//! without limit unless something moves the older end out. Nothing did. On
//! 2026-09-17 it held 95 entries and was moved by hand; by 2026-10-10 it held
//! 526 entries and 25,301 lines and was moved by hand again, by the pull
//! request fixing kanso#1850. kanso#1854 asked for this spec.
//!
//! The count is pinned rather than capped. A branch that appends the
//! forty-first entry runs `scripts/archive_log.sh`, which moves the oldest one
//! to the end of the archive, in the same commit, so the file holds forty
//! after every merge.

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

fn headings(text: &str) -> Vec<&str> {
    text.lines().filter(|l| l.starts_with("## ")).collect()
}

#[test]
fn the_live_log_holds_forty_entries() {
    let live = read("design/compiler-log.md");
    let held = headings(&live);
    assert_eq!(
        held.len(),
        40,
        "design/compiler-log.md holds {} entries. It holds the last forty: run \
         `sh scripts/archive_log.sh`, which moves the oldest {} entr{}, unedited, \
         from the top of the live log to the end of \
         design/log/compiler-log-archive.md, and commit it with the entry. The \
         first to move is {:?}.",
        held.len(),
        held.len().saturating_sub(40),
        if held.len() == 41 { "y" } else { "ies" },
        held.first().copied().unwrap_or_default(),
    );
}

/// The 2026-09-18 re-merge had to check this by hand: a resolution that kept
/// both sides left entries in both files.
#[test]
fn no_entry_is_in_both_the_live_log_and_the_archive() {
    let live = read("design/compiler-log.md");
    let archive = read("design/log/compiler-log-archive.md");
    let archived: std::collections::BTreeSet<&str> = headings(&archive).into_iter().collect();
    let both: Vec<&str> = headings(&live).into_iter().filter(|h| archived.contains(h)).collect();
    assert!(
        both.is_empty(),
        "these entries are in both design/compiler-log.md and the archive: {both:?}. \
         An entry lives in exactly one of them."
    );
}

/// The oldest live entry is no older than the newest archived one, so the
/// two files read as one chronology split at a point.
#[test]
fn the_archive_ends_where_the_live_log_begins() {
    let live = read("design/compiler-log.md");
    let archive = read("design/log/compiler-log-archive.md");
    let date = |h: &str| h.trim_start_matches("## ").get(..10).unwrap_or_default().to_string();
    let first_live = headings(&live).first().map(|h| date(h)).unwrap_or_default();
    let last_archived = headings(&archive).last().map(|h| date(h)).unwrap_or_default();
    assert!(
        last_archived <= first_live,
        "the archive's last entry is dated {last_archived} and the live log's first \
         {first_live}; entries move from the top of the live log to the end of the \
         archive, so the archive never runs ahead of it"
    );
}
