//! No page that ships to readers carries a git conflict marker.
//!
//! On 2026-09-18 five branches each added a section to `docs/compiler.html`
//! and every merge from main conflicted in the same place, at the end of the
//! file just above the coda. One of those resolutions went wrong: the HTML was
//! left unmerged, a script rewrote the file for another reason, and the commit
//! went in with `<<<<<<< HEAD`, `=======` and `>>>>>>> origin/main` sitting in
//! the published page.
//!
//! All three page gates then ran on that tree and all three passed.
//! `golden_prose` reads the `data-golden` spans, `page_drift` counts log
//! entries against the page's git history, and `prose_check` looks for three
//! families of sentence. A conflict marker is none of those things, so the
//! sweep printed "the three page gates agree with what the tree says" over a
//! page with three markers in it.
//!
//! Nothing else would have caught it either. The marker sits in HTML, so a
//! browser renders it as text on the page rather than failing; the site builds;
//! the book checks pass. It was found by a grep run for an unrelated reason.
//!
//! So the cheapest possible check goes here, where it runs in the specs job on
//! every push. It reads the same two directories prose_check reads -- `docs`
//! and `docs/book` -- and it reads them off disk rather than from a list, so a
//! page added later is covered without anybody remembering this file.

use std::path::{Path, PathBuf};

/// The three markers `git merge` leaves. The middle one is anchored to the
/// start of a line AND to end-of-line, because a row of equals signs is
/// ordinary punctuation in a fenced code block or a rule; the other two are
/// distinctive enough on their own.
fn marker_on(line: &str) -> Option<&'static str> {
    if line.starts_with("<<<<<<< ") {
        Some("<<<<<<<")
    } else if line.starts_with(">>>>>>> ") {
        Some(">>>>>>>")
    } else if line == "=======" {
        Some("=======")
    } else {
        None
    }
}

fn pages() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for dir in ["docs", "docs/book"] {
        let d = root.join(dir);
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries {
            let p = e.expect("a directory entry").path();
            if p.extension().is_some_and(|x| x == "html" || x == "md") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn no_published_page_carries_a_conflict_marker() {
    let pages = pages();

    // A spec that matched nothing would pass forever. The tree had 29 pages
    // when this was written and the number is not pinned, because pages are
    // added; that there are many of them is the property worth asserting.
    assert!(
        pages.len() > 20,
        "only {} page(s) found under docs and docs/book, which means this spec \
         is reading the wrong place rather than that the pages went away",
        pages.len()
    );

    let mut found = Vec::new();
    for page in &pages {
        let Ok(body) = std::fs::read_to_string(page) else { continue };
        for (n, line) in body.lines().enumerate() {
            if let Some(marker) = marker_on(line) {
                let name = page.file_name().unwrap().to_string_lossy();
                found.push(format!("{name}:{}  {marker}", n + 1));
            }
        }
    }

    assert!(
        found.is_empty(),
        "a git conflict marker is published on {} line(s):\n  {}\n\n\
         This is an unfinished merge, not prose. Resolve it by hand -- the log \
         and the page both take both sides, main's first -- and check the \
         section numbers afterwards, because a merge that leaves a marker has \
         usually left a duplicated section number beside it.",
        found.len(),
        found.join("\n  ")
    );
}
