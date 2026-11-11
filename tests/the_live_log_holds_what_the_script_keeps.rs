//! design/compiler-log.md holds the most recent entries, as many as
//! `scripts/archive_log.sh` keeps, and every older one is in
//! design/log/compiler-log-archive.md, unedited and in order.
//!
//! The live file is read at its tail and appended on every change, so it grows
//! without limit unless something moves the older end out. Nothing did. On
//! 2026-09-17 it held 95 entries and was moved by hand; by 2026-10-10 it held
//! 526 entries and 25,301 lines and was moved by hand again, by the pull
//! request fixing kanso#1850. kanso#1854 asked for this spec.
//!
//! The count is pinned rather than capped. A branch whose entry takes the
//! file past the limit runs the script, which moves the oldest one by date
//! to the end of the archive, in the same commit, so the file holds the
//! limit after every merge. The limit is the script's `keep=` line and is
//! read from there.

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

fn headings(text: &str) -> Vec<&str> {
    text.lines().filter(|l| l.starts_with("## ")).collect()
}

/// How many entries the script keeps: its `keep=` line.
fn kept() -> usize {
    let script = read("scripts/archive_log.sh");
    let line = script
        .lines()
        .find_map(|l| l.strip_prefix("keep="))
        .expect("scripts/archive_log.sh has a keep= line");
    line.trim().parse().expect("keep= is a number")
}

#[test]
fn the_live_log_holds_what_the_script_keeps() {
    let live = read("design/compiler-log.md");
    let held = headings(&live);
    let keep = kept();
    assert_eq!(
        held.len(),
        keep,
        "design/compiler-log.md holds {} entries and scripts/archive_log.sh \
         keeps {keep}: run `sh scripts/archive_log.sh`, which moves the oldest \
         {} by date, unedited, to the end of \
         design/log/compiler-log-archive.md, and commit it with the entry. The \
         first to move is {:?}.",
        held.len(),
        held.len().saturating_sub(keep),
        held.iter().min_by_key(|h| h.get(3..13).unwrap_or_default()).copied().unwrap_or_default(),
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

/// `scripts/archive_log.sh` moves the oldest entries by date, wherever they
/// sit, because entries land in merge order (kanso#1966). On #1964 the live
/// log opened with a 2026-10-13 entry above 2026-10-12 ones, the script moved
/// the top one, and the spec above refused the archive it left.
#[test]
fn the_archive_script_moves_the_oldest_dated_entries() {
    let dir = std::env::temp_dir().join(format!("kanso_archive_log_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("design/log")).expect("a scratch directory");

    // Two entries more than the script keeps. The first two are dated a day
    // after the third, so the two oldest are the third and the fourth.
    let keep = kept();
    let mut live = String::from("# compiler log\n\nThe live end.\n\n");
    for i in 1..=keep + 2 {
        let day = match i {
            1 | 2 => 13,
            3 | 4 => 11,
            _ => 12,
        };
        live.push_str(&format!("## 2026-10-{day} — entry {i}\n\nbody {i}\n\n"));
    }
    std::fs::write(dir.join("design/compiler-log.md"), &live).expect("the live log writes");
    let archive = "# archive\n\n## 2026-10-10 — older\n\nolder body\n";
    std::fs::write(dir.join("design/log/compiler-log-archive.md"), archive)
        .expect("the archive writes");

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/archive_log.sh");
    let status = std::process::Command::new("sh")
        .arg(&script)
        .current_dir(&dir)
        .status()
        .expect("sh runs the script");
    assert!(status.success(), "the script exits {status}");

    let live_after = std::fs::read_to_string(dir.join("design/compiler-log.md")).unwrap();
    let archive_after =
        std::fs::read_to_string(dir.join("design/log/compiler-log-archive.md")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    let stayed = headings(&live_after);
    assert_eq!(stayed.len(), keep, "{live_after}");
    assert_eq!(stayed[0], "## 2026-10-13 — entry 1", "{live_after}");
    assert!(!stayed.iter().any(|h| h.ends_with("entry 3") || h.ends_with("entry 4")));
    assert!(live_after.starts_with("# compiler log\n\nThe live end.\n\n## 2026-10-13 — entry 1\n"));
    assert_eq!(
        archive_after,
        "# archive\n\n## 2026-10-10 — older\n\nolder body\n\n\
         ## 2026-10-11 — entry 3\n\nbody 3\n\n## 2026-10-11 — entry 4\n\nbody 4\n"
    );
}
