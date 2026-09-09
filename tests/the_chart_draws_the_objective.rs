//! The trend chart on docs/numbers.html draws the counters the objective reads.
//!
//! The page's own subject line is "what a run costs, what compiling costs, and
//! the welfare score they roll up into", and the chart under it is the long
//! view of that score. A line reading a counter the objective has retired is
//! shape nobody is optimising, and a term the objective reads with no line is
//! the part of the score a reader cannot see move.
//!
//! Both happened, and for a year of commits nothing said so. On 2026-09-09 the
//! chart drew six lines and five of them were retired counters:
//! `instructions + encode_instructions` and `oneshot_arena_peak_bytes` for the
//! run side, retired by the 2026-09-06 gavel that made the objective's runtime
//! one consolidated program, and `compile_rounds + compile_visits +
//! emitted_lines` for the compile side, retired by the 2026-09-03 rebuild
//! before it. `run_instructions` and `run_peak_bytes`, which every row has
//! carried since 2026-09-06, were drawn nowhere. The retired arena peak had
//! two distinct values across every row holding it, so that line was flat.
//!
//! THE ROW SIDE NEEDS NO CHECK HERE. `scripts/perf_record` writes the row's
//! objective counters straight out of `welfare --counters`, unfiltered, so a
//! counter that enters the model enters the row in the same commit. What can
//! drift is this page, which names its counters by hand.
//!
//! `bench/objective_sources.txt` is the list, replayed against `welfare
//! --counters` by tests/the_objective_reads_what_the_gate_watches.rs. This
//! reads its first column and asks the chart about it in both directions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every counter the objective weighs: the first field of each `<counter>
/// <gate key>` pair, with the comments and blank lines dropped. A counter that
/// is a sum names several gate keys and so appears on several lines; the set
/// collapses those.
fn objective_counters(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The `const TREND = [ ... ];` array, by brace-free bounds: the array is the
/// only thing between that opening line and the first `];` at the same indent.
fn trend_block(page: &str) -> &str {
    let open = "const TREND = [";
    let at = page.find(open).expect("docs/numbers.html has no TREND array");
    let rest = &page[at + open.len()..];
    let end = rest.find("\n  ];").expect("the TREND array is never closed");
    &rest[..end]
}

/// Every row key the chart's reader lambdas look up, as `r.<name>`. The
/// readers are the whole of what a line draws, so this is the chart's side of
/// the question in full.
fn keys_read(block: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (i, _) in block.match_indices("r.") {
        let before = block[..i].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let rest = &block[i + 2..];
        let end =
            rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        if end > 0 {
            out.insert(rest[..end].to_string());
        }
    }
    out
}

/// The two lines that are deliberately not objective counters. Each is here
/// with its reason, so a third one cannot arrive without somebody writing one.
///
/// `welfare` is the score itself rather than a counter it reads. `text_bytes`
/// is the decoder's machine code, ruled out of the objective on 2026-09-05 --
/// no machine-code-size term in welfare, and `.text` keeps its own exact vein
/// -- and the page says as much beside the line, which is why it is drawn grey.
const NOT_COUNTERS: [&str; 2] = ["welfare", "text_bytes"];

#[test]
fn every_objective_counter_has_a_line() {
    let page = std::fs::read_to_string(root().join("docs/numbers.html")).unwrap();
    let sources = std::fs::read_to_string(root().join("bench/objective_sources.txt")).unwrap();

    let drawn = keys_read(trend_block(&page));
    let missing: Vec<_> =
        objective_counters(&sources).into_iter().filter(|c| !drawn.contains(c)).collect();

    assert!(
        missing.is_empty(),
        "the objective reads {missing:?} and the chart draws no line for them; \
         docs/numbers.html's TREND draws {drawn:?}",
    );
}

#[test]
fn every_line_is_an_objective_counter_or_says_why_not() {
    let page = std::fs::read_to_string(root().join("docs/numbers.html")).unwrap();
    let sources = std::fs::read_to_string(root().join("bench/objective_sources.txt")).unwrap();

    let counters = objective_counters(&sources);
    let stale: Vec<_> = keys_read(trend_block(&page))
        .into_iter()
        .filter(|k| !counters.contains(k) && !NOT_COUNTERS.contains(&k.as_str()))
        .collect();

    assert!(
        stale.is_empty(),
        "the chart draws {stale:?}, which the objective does not read; either \
         give the line a counter the objective weighs or name it in \
         NOT_COUNTERS with the reason it stays",
    );
}
