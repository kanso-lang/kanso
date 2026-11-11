//! Every line on the trend chart wears a `--series-*` token, and both colour
//! modes declare every token.
//!
//! The colours themselves live in the CSS of docs/numbers.html and nowhere
//! else. This file once held a second copy of all of them and asserted the
//! page equalled it; the chart draws one panel per counter, so colour tells
//! no two lines apart, and the copy protected nothing a reader sees
//! (kanso#2447). How the set was measured is in the log's entries of
//! 2026-09-09 and 2026-09-17.
//!
//! What can still go wrong is a line that names a colour directly, which is
//! right in one mode and unchosen in the other, or a token one mode declares
//! and the other does not.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn page() -> String {
    std::fs::read_to_string(root().join("docs/numbers.html")).unwrap()
}

/// The `--series-*` tokens declared in source order, from the slice of the
/// page between `open` and the next `}`.
fn declared(page: &str, open: &str) -> Vec<String> {
    let at = page.find(open).unwrap_or_else(|| panic!("docs/numbers.html has no `{open}` block"));
    let rest = &page[at + open.len()..];
    let end = rest.find('}').expect("the block is never closed");
    rest[..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("--series-"))
        .filter_map(|l| l.strip_suffix(';'))
        .filter_map(|l| l.split_once(": "))
        .map(|(name, _)| format!("--series-{name}"))
        .collect()
}

/// The text of the page's `TREND` array.
fn trend(page: &str) -> &str {
    let open = "const TREND = [";
    let at = page.find(open).expect("docs/numbers.html has no TREND array");
    let rest = &page[at + open.len()..];
    &rest[..rest.find("\n  ];").expect("the TREND array is never closed")]
}

#[test]
fn both_modes_declare_the_same_tokens() {
    let page = page();
    let light = declared(&page, ":root {");
    let dark = declared(&page, "@media (prefers-color-scheme: dark) {\n  :root {");
    assert!(!light.is_empty(), "the light block declares no --series-* token");
    assert_eq!(light, dark, "the two colour modes do not declare the same tokens in order");
}

#[test]
fn every_line_wears_a_declared_token_and_no_line_wears_a_hex() {
    let page = page();
    let block = trend(&page);
    let tokens = declared(&page, ":root {");

    let worn: Vec<_> = block
        .match_indices("hue: 'var(")
        .map(|(i, m)| {
            let tail = &block[i + m.len()..];
            tail[..tail.find(')').expect("an unclosed var()")].to_string()
        })
        .collect();
    assert!(!worn.is_empty(), "no line in TREND wears a token");
    let undeclared: Vec<_> = worn.iter().filter(|t| !tokens.contains(t)).collect();
    assert!(undeclared.is_empty(), "TREND wears tokens the page never declares: {undeclared:?}");

    let hex: Vec<_> = block
        .match_indices('#')
        .map(|(i, _)| block[i..].chars().take(7).collect::<String>())
        .filter(|s| s.len() == 7 && s[1..].chars().all(|c| c.is_ascii_hexdigit()))
        .collect();
    assert!(
        hex.is_empty(),
        "TREND names the colours {hex:?} directly; a line wears a --series-* token so that \
         each mode gets its own step"
    );
}
