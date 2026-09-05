//! A ratchet row is only a row if the harness reaches it.
//!
//! `scripts/ratchet/ratchet.kso` writes each row as a binding and then
//! assembles them into `rows` through a chain of `text/concat`s. Writing the
//! binding is the visible half and joining the chain is the half nobody looks
//! at, so a row can be added, reviewed and merged while the harness never runs
//! it. Two were found that way on 2026-09-05, while a third row was being
//! retired: `live_counters`, written the same day livebench joined the
//! objective, and `compile_ir_pair`, which had watched a compile-row refusal
//! since the day the pair was ruled. Neither ever ran. `compile_ir_pair` was
//! deleted under the one-row-one-value ruling and would have gone to its grave
//! having proved nothing.
//!
//! That is the same shape this repo keeps finding: a check that cannot fail,
//! green and read as evidence. kanso#1199 found two rows blind, kanso#1229
//! found three more and four checks that could not fail, and every one of them
//! was at least running. A row off the chain is the cheaper version of the
//! same defect and nothing could see it.
//!
//! So the chain is walked from `rows` rather than eyeballed.

use std::collections::{BTreeMap, BTreeSet};

const RATCHET: &str = include_str!("../scripts/ratchet/ratchet.kso");

/// Every `name = row` binding in the file.
fn rows_defined(src: &str) -> BTreeSet<String> {
    src.lines()
        .filter_map(|line| line.strip_suffix(" = row"))
        .filter(|name| !name.starts_with(char::is_whitespace))
        .map(str::to_string)
        .collect()
}

/// Every one-line binding whose body is a list or a concatenation of them,
/// keyed by the name it binds. These are the links of the chain.
fn list_bindings(src: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in src.lines() {
        if line.starts_with(char::is_whitespace) || line.starts_with('#') {
            continue;
        }
        let Some((name, body)) = line.split_once(" = ") else { continue };
        if name.contains(char::is_whitespace) {
            continue;
        }
        if body == "row" {
            continue;
        }
        if body.starts_with('[') || body.starts_with("text/concat") {
            out.insert(name.to_string(), body.to_string());
        }
    }
    out
}

/// The names a binding's body mentions, whatever brackets they sit inside.
fn words(body: &str) -> Vec<String> {
    body.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Every row `rows` reaches, following the concat chain from its root.
fn rows_reached(src: &str) -> BTreeSet<String> {
    let defined = rows_defined(src);
    let lists = list_bindings(src);
    let mut reached = BTreeSet::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut queue = vec!["rows".to_string()];
    while let Some(name) = queue.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(body) = lists.get(&name) else { continue };
        for word in words(body) {
            if defined.contains(&word) {
                reached.insert(word);
            } else if lists.contains_key(&word) {
                queue.push(word);
            }
        }
    }
    reached
}

#[test]
fn the_root_binding_is_there_to_walk_from() {
    let lists = list_bindings(RATCHET);
    assert!(
        lists.contains_key("rows"),
        "scripts/ratchet/ratchet.kso binds no `rows` list, so this file walks \
         nothing and would pass over a ratchet that runs nothing at all"
    );
}

#[test]
fn the_file_defines_rows_at_all() {
    let defined = rows_defined(RATCHET);
    assert!(
        defined.len() > 50,
        "found only {} row bindings in scripts/ratchet/ratchet.kso; the shape \
         moved and this file is now reading the wrong thing",
        defined.len()
    );
}

#[test]
fn every_row_written_is_a_row_the_harness_runs() {
    let defined = rows_defined(RATCHET);
    let reached = rows_reached(RATCHET);
    let orphans: Vec<_> = defined.difference(&reached).cloned().collect();
    assert!(
        orphans.is_empty(),
        "these rows are written in scripts/ratchet/ratchet.kso and never join \
         the chain that `rows` assembles, so the harness never applies their \
         mutation and they prove nothing: {orphans:?}"
    );
}
