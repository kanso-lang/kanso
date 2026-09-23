//! Every numbered section on the compiler page has its own number, and the
//! numbers ascend down the page.
//!
//! Branches here claim section numbers ahead of landing, and several are open at
//! once. On 2026-09-23 two of them — kanso#1565 and kanso#1568 — both numbered
//! their section 122: the second was given its number after checking the other
//! open branches' claims and missing one. Each branch was green on its own, and
//! nothing in the tree would have objected when both landed. The page would have
//! published two sections called 122.
//!
//! The conflict audits this repo already does look for duplicated PARAGRAPHS,
//! lost sections and conflict markers. A duplicated NUMBER is none of those: the
//! two sections are different text, so every structural check passes over it.
//! This is the check for that.
//!
//! Two sections carry names rather than numbers — `techniques` and `coda` — and
//! they are left alone. Gaps are allowed, because a branch holding a number that
//! has not landed yet leaves one.

use std::path::Path;

/// The section numbers in page order, read out of `<span class="sec-num">N</span>`.
fn numbers() -> Vec<u32> {
    let page =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/compiler.html"))
            .expect("the compiler page reads");
    let open = r#"<span class="sec-num">"#;
    page.match_indices(open)
        .filter_map(|(at, m)| {
            let rest = &page[at + m.len()..];
            rest[..rest.find('<')?].trim().parse().ok()
        })
        .collect()
}

#[test]
fn the_page_has_numbered_sections_to_check() {
    // A reader that finds nothing makes both tests below pass vacuously, so
    // establish first that it finds the page's sections.
    let n = numbers();
    assert!(
        n.len() >= 100,
        "found only {} numbered sections on docs/compiler.html; the markup this \
         reads has probably changed, and the tests below would pass on nothing",
        n.len()
    );
}

#[test]
fn no_two_sections_share_a_number() {
    let n = numbers();
    let mut seen = std::collections::BTreeMap::new();
    for (i, x) in n.iter().enumerate() {
        if let Some(first) = seen.insert(*x, i) {
            panic!(
                "section {x} appears twice on docs/compiler.html, at positions \
                 {} and {} among its numbered sections. Two open branches \
                 claimed the same number and both landed; renumber the later one.",
                first + 1,
                i + 1
            );
        }
    }
}

#[test]
fn the_numbers_ascend_down_the_page() {
    let n = numbers();
    for w in n.windows(2) {
        assert!(
            w[1] > w[0],
            "section {} follows section {} on docs/compiler.html. A merge that \
             kept both sides of a conflict put them in the wrong order.",
            w[1],
            w[0]
        );
    }
}
