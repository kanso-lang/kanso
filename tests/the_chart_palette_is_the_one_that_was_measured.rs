//! The trend chart's seven colours are a measured set, and this pins them.
//!
//! Colour on a categorical chart is computable, so it was computed rather than
//! chosen. The seven hues below are the only ordering, of the 5,040 the seven
//! hues admit, that was kept: it clears the adjacent-pair floors under
//! simulated protanopia, deuteranopia and tritanopia in both modes AND clears
//! them again among the three compile lines taken all-pairs, which is the
//! comparison a reader of this chart actually makes. Worst margin 9.2 either
//! way (OKLab delta-E x100, floor 8); worst normal-vision margin 24.6 within a
//! group against a floor of 15. 216 of the 5,040 orderings clear both gates.
//!
//! ORDER IS THE MECHANISM, NOT DECORATION. The check measures ADJACENT pairs,
//! so moving one line past another can break a pair that was never touched,
//! and recolouring one line re-opens the whole set. That is why this spec pins
//! the sequence and not just the membership: a palette is a set plus an order.
//!
//! What went wrong, on 2026-09-09. The page shipped ONE palette drawn on two
//! surfaces, and it had been picked for the dark one. On the light page
//! (#fcfbf7) every one of the seven lines sat under 3:1 -- yellow at 1.48 --
//! and three of them (#38bdf8, #94a3b8, #a78bfa) crowded the blue band, the
//! worst adjacent pair separating by 5.2 under deuteranopia against a floor of
//! 8. The grey chosen for binary size failed the chroma floor outright: a line
//! with no colour in it reads as part of the grid. Each mode is now stepped
//! for the surface it is drawn on and validated against that surface.
//!
//! Three light-mode steps still sit under 3:1 (#1baf7a 2.72, #eda100 2.09,
//! #e87ba4 2.60). That is a documented relief rather than a failure, and the
//! relief is the panel under the chart: every line up there has a row down
//! there carrying its number. `missing_panel_rows`, in scripts/site_smoke,
//! holds up that half -- off the rendered page in a real browser, since what
//! matters is what a reader can look up -- and the two checks stand or fall
//! together. Weakening either one is what makes the other's relief a fiction.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn page() -> String {
    std::fs::read_to_string(root().join("docs/numbers.html")).unwrap()
}

/// The measured set, in the measured order: token, light step, dark step.
const PALETTE: [(&str, &str, &str); 7] = [
    ("--series-run-instructions", "#eda100", "#c98500"),
    ("--series-run-memory", "#2a78d6", "#3987e5"),
    ("--series-compile-instructions", "#eb6834", "#d95926"),
    ("--series-compile-allocations", "#1baf7a", "#199e70"),
    ("--series-compile-memory", "#4a3aa7", "#9085e9"),
    ("--series-binary-size", "#e87ba4", "#d55181"),
    // Green is the same step in both modes: it clears 3:1 against each surface
    // as it stands, so the score's own line does not change colour with the
    // reader's theme.
    ("--series-welfare", "#008300", "#008300"),
];

/// The `--series-*: #hex;` declarations in source order, from the slice of the
/// page between `open` and the next `}`.
fn declarations(page: &str, open: &str) -> Vec<(String, String)> {
    let at = page.find(open).unwrap_or_else(|| panic!("docs/numbers.html has no `{open}` block"));
    let rest = &page[at + open.len()..];
    let end = rest.find('}').expect("the block is never closed");
    rest[..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("--series-"))
        .filter_map(|l| l.strip_suffix(';'))
        .filter_map(|l| l.split_once(": "))
        .map(|(name, hex)| (format!("--series-{name}"), hex.to_string()))
        .collect()
}

#[test]
fn the_light_steps_are_the_ones_that_were_measured() {
    let want: Vec<_> = PALETTE.iter().map(|(t, l, _)| (t.to_string(), l.to_string())).collect();
    assert_eq!(
        declarations(&page(), ":root {"),
        want,
        "docs/numbers.html's light series steps are not the measured set, in the measured \
         order. Colour here is computed, never chosen: re-run the validator over the whole \
         palette -- adjacent pairs in BOTH modes and all pairs within the compile group -- \
         and move this list only with the numbers that came back."
    );
}

#[test]
fn the_dark_steps_are_the_ones_that_were_measured() {
    let want: Vec<_> = PALETTE.iter().map(|(t, _, d)| (t.to_string(), d.to_string())).collect();
    assert_eq!(
        declarations(&page(), "@media (prefers-color-scheme: dark) {\n  :root {"),
        want,
        "docs/numbers.html's dark series steps are not the measured set, in the measured \
         order. The dark column is the same hues stepped for the dark surface, validated \
         against it as a set -- not the light column reused, which is the defect this \
         replaced."
    );
}

/// The chart reads the tokens and never a literal colour, so the two modes
/// cannot drift apart at the point of use. A raw hex in TREND would be a
/// colour that is right in one mode and unmeasured in the other, which is
/// exactly how the palette this replaced came to be wrong.
#[test]
fn every_line_wears_a_token_and_no_line_wears_a_hex() {
    let page = page();
    let open = "const TREND = [";
    let at = page.find(open).expect("docs/numbers.html has no TREND array");
    let rest = &page[at + open.len()..];
    let block = &rest[..rest.find("\n  ];").expect("the TREND array is never closed")];

    let worn: Vec<_> = block
        .match_indices("hue: 'var(")
        .map(|(i, m)| {
            let tail = &block[i + m.len()..];
            tail[..tail.find(')').expect("an unclosed var()")].to_string()
        })
        .collect();
    let want: Vec<_> = PALETTE.iter().map(|(t, _, _)| t.to_string()).collect();
    assert_eq!(worn, want, "the chart's lines do not wear the measured tokens in order");

    let hex: Vec<_> = block
        .match_indices('#')
        .map(|(i, _)| block[i..].chars().take(7).collect::<String>())
        .filter(|s| s.len() == 7 && s[1..].chars().all(|c| c.is_ascii_hexdigit()))
        .collect();
    assert!(
        hex.is_empty(),
        "TREND names the colours {hex:?} directly; a line wears a --series-* token so that \
         each mode gets the step measured against its own surface"
    );
}
