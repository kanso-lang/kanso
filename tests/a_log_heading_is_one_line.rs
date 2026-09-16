//! `scripts/page_drift` counts log entries by the lines a commit adds that
//! start with `## `, and fails past a budget of three. So a heading wrapped
//! across two lines is counted twice, and an entry that should have spent one
//! of the three spends two. kanso#1448 and kanso#1451 each landed a title that
//! had been hard-wrapped at the column limit:
//!
//!     ## 2026-09-16 — A folder handed to `fold` is never called by name, and a
//!     ## walk that found no call site answered yes (DONE)
//!
//! Markdown renders that as two h2s, the second one a sentence fragment, and
//! the gate reads it as two entries. The wrap is the whole defect: the words
//! are right, the line break is not.
//!
//! The property is not "a heading is short". Non-dated `## ` sections are a
//! real convention here — an entry carries `## the measurement`, `## CI's
//! sitting`, `## CORRECTION: ...` as its own sub-sections. What separates
//! those from a wrap is that a real heading is preceded by a blank line and a
//! wrap is preceded by the line it wrapped from. So: no `## ` line is
//! immediately preceded by another `## ` line.
//!
//! Measured when this was written: two violations in the live log, zero
//! across the archive's 1,272 entries, so the convention was already
//! universal and only today's two entries broke it.

use std::path::Path;

/// Every `## ` line that follows another `## ` line, as (file, line number,
/// the pair of lines), which is the wrap and nothing else.
fn wrapped(path: &Path) -> Vec<(usize, String, String)> {
    let text = std::fs::read_to_string(path).expect("the log reads");
    let lines: Vec<&str> = text.lines().collect();
    lines
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0].starts_with("## ") && pair[1].starts_with("## "))
        .map(|(i, pair)| (i + 2, pair[0].to_string(), pair[1].to_string()))
        .collect()
}

#[test]
fn a_log_heading_is_one_line() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in ["design/compiler-log.md", "design/log/compiler-log-archive.md"] {
        let found = wrapped(&root.join(rel));
        assert!(
            found.is_empty(),
            "{rel}: {} heading(s) wrapped onto a second `## ` line, which \
             page_drift counts as a second entry. Join each pair onto one \
             line:\n{}",
            found.len(),
            found
                .iter()
                .map(|(n, a, b)| format!("  line {n}:\n    {a}\n    {b}\n"))
                .collect::<String>()
        );
    }
}
