//! The objective can name the counter set it scored.
//!
//! The chart replay ruled on 2026-08-31 could not be computed, and the reason
//! was data rather than design. `perf_record` writes one history row per merged
//! commit from a hand-picked list of counters, and on 2026-09-03 the newest row
//! — commit a100f4f — held 12 of the 24 counters the formula reads.
//! `compile_instructions` was among the twelve missing. So "the replayed series
//! begins at the first commit for which every counter in the current formula
//! exists" named no commit at all, and the replayed line would have been empty.
//!
//! `welfare --counters` prints the set `score` was given, so a row can carry
//! exactly what the formula reads. Printing it here rather than assembling it
//! again in perf_record is the whole point: two lists drift the first time a
//! counter joins the model, and a chart replaying today's formula over
//! yesterday's counter set is wrong without saying so.

use std::process::Command;

fn counters() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .arg("--")
        .arg("--counters")
        .current_dir(root)
        .output()
        .expect("welfare runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Every counter the model weighs, and nothing else. The list is written out
/// rather than derived, because a spec that recomputes what the tool computes
/// is asserting its own copy of the tool — and this list IS the thing the
/// replay depends on, so it is pinned where a reader can see it move.
#[test]
fn the_counter_set_is_the_one_the_formula_reads() {
    let said = counters();
    let want = [
        "decode_instructions",
        "encode_instructions",
        "oneshot_instructions",
        "basket_instructions",
        "wide_instructions",
        "deep_instructions",
        "pending_instructions",
        "digest_instructions",
        "scan_instructions",
        "escape_instructions",
        "index_instructions",
        "decode_peak_bytes",
        "decode_arena_blocks",
        "encode_peak_bytes",
        "encode_arena_blocks",
        "oneshot_peak_bytes",
        "basket_peak_bytes",
        "scan_arena_blocks",
        "scan_peak_bytes",
        "digest_peak_bytes",
        "digest_arena_blocks",
        "compile_instructions",
        "compile_allocs",
        "compile_peak_bytes",
    ];
    let named: Vec<&str> =
        said.lines().filter_map(|l| l.split('=').next()).filter(|l| !l.is_empty()).collect();
    assert_eq!(named.len(), want.len(), "one line per counter the model weighs: {said}");
    for c in want {
        assert!(named.contains(&c), "the set is missing {c}: {said}");
    }
}

/// Every line is `name=value` with a value that parses, because a row assembled
/// from this is read back as numbers. A counter printed with no value would
/// reach the history as a null and take the replayed line out silently.
#[test]
fn every_counter_carries_a_number() {
    for line in counters().lines() {
        let (name, value) = line.split_once('=').unwrap_or_else(|| panic!("name=value: {line}"));
        assert!(!name.is_empty(), "a named counter: {line}");
        value.parse::<u128>().unwrap_or_else(|_| panic!("a number for {name}: {line}"));
    }
}

/// The flag reports and does not ratchet. `--counters` must not be a path that
/// can move the floor, which is the one thing this file must never become a
/// second door to.
#[test]
fn asking_what_was_scored_does_not_move_the_floor() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let floor = root.join("bench/welfare_floor.json");
    let before = std::fs::read(&floor).expect("the floor reads");
    let _ = counters();
    let after = std::fs::read(&floor).expect("the floor reads");
    assert_eq!(before, after, "--counters is a report, not a ratchet");
}

/// The page replays the score in javascript, because it is static and the
/// 2026-08-31 directive says the chart shows the current formula over the
/// stored rows. That is a second statement of the rule, and a second statement
/// drifts from the first silently — the chart would go on drawing a line, just
/// the wrong one.
///
/// So run the page's own two functions, lifted out of the html rather than
/// copied here, over the model and counters welfare emits, and require the
/// answer welfare gives. A copy would agree with itself forever.
#[test]
fn the_page_replays_the_score_the_tool_computes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/numbers.html")).expect("the page");

    // From `function <name>` to the brace that closes it. Counting braces is
    // enough here because neither function holds a brace in a string or a
    // comment, and the extraction failing loudly beats it silently taking the
    // wrong text.
    let lift = |name: &str| -> String {
        let head = format!("function {name}(");
        let at = page.find(&head).unwrap_or_else(|| panic!("the page defines {name}"));
        let open = page[at..].find('{').expect("a body") + at;
        let mut depth = 0;
        for (i, c) in page[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return page[at..open + i + 1].to_string();
                    }
                }
                _ => {}
            }
        }
        panic!("{name} is never closed");
    };

    let ask = |flag: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(root.join("scripts/welfare"))
            .args(if flag.is_empty() { vec![] } else { vec!["--", flag] })
            .current_dir(root)
            .output()
            .expect("welfare runs");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // The row the page would read: perf_record writes the welfare counter set
    // into the history row under welfare's own names, so `--counters` is that
    // half of a row exactly.
    let row: String = ask("--counters")
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| format!("{k:?}:{v},"))
        .collect();

    let dir = root.join("target/page-replay");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let model_at = dir.join("model.txt");
    std::fs::write(&model_at, ask("--model")).expect("the model");
    let script_at = dir.join("replay.js");
    std::fs::write(
        &script_at,
        format!(
            "{}\n{}\nconst m = parseModel(require('fs').readFileSync({:?}, 'utf8'));\n\
             if (!m) throw new Error('the model did not parse');\n\
             console.log(replayScore({{{row}}}, m));\n",
            lift("parseModel"),
            lift("replayScore"),
            model_at.to_str().expect("a path"),
        ),
    )
    .expect("the script");

    let out = Command::new("node").arg(&script_at).output().expect("node runs");
    assert!(out.status.success(), "node: {}", String::from_utf8_lossy(&out.stderr));
    let theirs: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().expect("a score");

    let said = ask("");
    let banner: f64 =
        said.split_whitespace().nth(1).expect("welfare says a score").parse().expect("a number");

    assert!(
        (theirs - banner).abs() < 0.005,
        "the page's replay gives {theirs:.4}, welfare says {banner:.2}"
    );
}

/// A row from before a counter joined the model cannot be scored, and the
/// chart must leave it out rather than invent a reading. Clay, 2026-09-03:
/// "No backfill: start the replayed line at the first commit with the full
/// current counter set."
#[test]
fn a_row_missing_a_counter_is_not_scored() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/numbers.html")).expect("the page");
    let lift = |name: &str| -> String {
        let head = format!("function {name}(");
        let at = page.find(&head).unwrap_or_else(|| panic!("the page defines {name}"));
        let open = page[at..].find('{').expect("a body") + at;
        let mut depth = 0;
        for (i, c) in page[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return page[at..open + i + 1].to_string();
                    }
                }
                _ => {}
            }
        }
        panic!("{name} is never closed");
    };

    let script = format!(
        "{}\n{}\n\
         const m = parseModel('term t|1.0|2.0|a,b\\nbase a=10\\nbase b=20\\n');\n\
         console.log(JSON.stringify([\n\
         replayScore({{a: 10, b: 20}}, m),\n\
         replayScore({{a: 10}}, m),\n\
         replayScore({{}}, m),\n\
         ]));\n",
        lift("parseModel"),
        lift("replayScore"),
    );
    let dir = root.join("target/page-replay");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let at = dir.join("partial.js");
    std::fs::write(&at, script).expect("the script");

    let out = Command::new("node").arg(&at).output().expect("node runs");
    assert!(out.status.success(), "node: {}", String::from_utf8_lossy(&out.stderr));
    let said = String::from_utf8_lossy(&out.stdout);

    // At parity every ratio is 1, so each counter scores 1/(1+2) and the term
    // scores a third of its whole weight: 100 * 1/3.
    assert_eq!(
        said.trim(),
        "[33.33333333333333,null,null]",
        "a complete row scores and a partial one does not: {said}"
    );
}
