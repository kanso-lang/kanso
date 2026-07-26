//! The enumerable's adapters are dispatched by three parallel sets of
//! hand-written arms — `fold`, `iter` and `next` — one per adapter shape.
//! Adding an adapter means adding an arm to each, and forgetting one fails
//! only when someone folds that particular shape. This pins the three sets
//! against each other so the omission fails here instead.
//!
//! Automating the arms away is what call-pattern specialization would buy;
//! until then, the parity is the guard.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn arms(source: &str, verb: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("pub fn ").or_else(|| line.strip_prefix("fn "))?;
            let rest = rest.strip_prefix(verb)?.strip_prefix(" (")?;
            let shape = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?;
            (!shape.is_empty()).then(|| shape.to_string())
        })
        .collect()
}

#[test]
fn every_adapter_shape_has_a_fold_an_iter_and_a_next() {
    let list = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/list/list.kso");
    let source = std::fs::read_to_string(&list).expect("the list module reads");

    let (fold, iter, next) = (arms(&source, "fold"), arms(&source, "iter"), arms(&source, "next"));

    assert!(!fold.is_empty(), "no fold arms found — the shape of list.kso changed");
    assert_eq!(fold, iter, "fold and iter disagree about which adapter shapes exist");
    assert_eq!(fold, next, "fold and next disagree about which adapter shapes exist");
}
