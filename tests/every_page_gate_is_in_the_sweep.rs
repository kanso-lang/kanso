//! The page sweep names every gate that reads the published pages.
//!
//! There are three of them and they run in three different CI jobs:
//! `golden_prose` under the welfare job, `page_drift` under the docs job,
//! `prose_check` under its own. Nothing named them together, so a session
//! editing docs/compiler.html ran the two it remembered. On kanso#1328 the
//! page's `front_end_visits` span still carried the number its golden had
//! moved past; page_drift and prose_check both pass on that tree, because
//! neither reads a `data-golden` span. golden_prose turned two jobs red — its
//! own, and welfare, which runs it last and so reported ALREADY RED and could
//! prove nothing about the three rows sharing that gate.
//!
//! `scripts/gates/all_pages.sh` runs the three. A sweep that named two of
//! three would be the same failure with a shorter command, so the table is
//! pinned to a property of the tree: every program under scripts/ that holds
//! a literal `docs` path is either a row in the sweep or in its `elsewhere`
//! list with a reason. A page gate added later cannot go unswept the way
//! golden_prose did.
//!
//! Reading the table is not running it. The behavioural half is the script.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const SWEEP: &str = include_str!("../scripts/gates/all_pages.sh");

/// The rows of one `name="..."` table in the sweep, split on the FIRST colon.
/// A row's middle field is prose and holds colons of its own, so splitting on
/// every colon would read the description as fields.
fn table(name: &str) -> Vec<(String, String)> {
    let body = SWEEP
        .split_once(&format!("{name}=\""))
        .unwrap_or_else(|| panic!("the sweep declares a {name} table"))
        .1
        .split_once('"')
        .unwrap_or_else(|| panic!("the {name} table is closed"))
        .0;
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (gate, rest) = l
                .split_once(':')
                .unwrap_or_else(|| panic!("a {name} row is <program>:<rest>, got {l:?}"));
            (gate.to_string(), rest.to_string())
        })
        .collect()
}

/// Every `scripts/<name>/<name>.kso` whose source holds a literal `docs` path.
///
/// The quote matters. Without it this also matches the word in a comment, and
/// a check that passes because it matched prose is the shape of mistake this
/// whole file is about.
fn programs_reading_docs() -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(root().join("scripts")).expect("scripts reads") {
        let entry = entry.expect("a directory entry reads");
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let program = entry.path().join(format!("{name}.kso"));
        let Ok(src) = std::fs::read_to_string(&program) else {
            continue;
        };
        if src.contains("\"docs/") || src.contains("\"docs\"") {
            found.insert(name, src);
        }
    }
    found
}

#[test]
fn every_program_that_reads_the_pages_is_swept_or_excused() {
    let on_disk = programs_reading_docs();
    assert!(
        on_disk.len() >= 3,
        "at least the three page gates hold a docs path; found {:?}",
        on_disk.keys().collect::<Vec<_>>()
    );
    let named: BTreeSet<String> =
        table("gates").into_iter().chain(table("elsewhere")).map(|(gate, _)| gate).collect();
    let unaccounted: Vec<_> = on_disk.keys().filter(|k| !named.contains(*k)).collect();
    assert!(
        unaccounted.is_empty(),
        "scripts/gates/all_pages.sh accounts for neither sweeping nor excusing \
         {unaccounted:?} — a sweep that names all but one looks like coverage \
         and is not"
    );
    let phantom: Vec<_> = named.iter().filter(|k| !on_disk.contains_key(*k)).collect();
    assert!(
        phantom.is_empty(),
        "the sweep names {phantom:?}, which is not a program under scripts/ \
         that reads a docs path"
    );
}

/// A row claims whether its gate takes `--write`, and the sweep passes the
/// flag on that claim alone. A row saying `yes` for a gate with no write path
/// would hand it an argument it does not understand.
#[test]
fn a_row_claiming_write_names_a_gate_that_has_one() {
    for (gate, rest) in table("gates") {
        let takes = rest.rsplit(':').next().expect("a row ends in its write field");
        assert!(
            takes == "yes" || takes == "no",
            "{gate}'s write field is {takes:?}, which is neither yes nor no"
        );
        let src = std::fs::read_to_string(root().join(format!("scripts/{gate}/{gate}.kso")))
            .expect("the gate's source reads");
        assert_eq!(
            takes == "yes",
            src.contains("--write"),
            "{gate} claims write={takes} and its source says otherwise"
        );
    }
}

/// An excuse pointing at another sweep has to be true of that sweep.
///
/// `book_panels` and `book_quotes` read docs/book and are left out here
/// because `scripts/book_check.sh` already runs both. That is a claim about a
/// file, so it is read rather than trusted: an excuse naming a script that
/// stops running the gate turns this red instead of quietly leaving the gate
/// unswept everywhere.
#[test]
fn an_excuse_naming_another_sweep_is_true_of_it() {
    let mut delegated = 0;
    for (gate, reason) in table("elsewhere") {
        assert!(!reason.trim().is_empty(), "{gate} is excused with no reason");
        let Some(script) = reason.split_whitespace().find(|w| w.starts_with("scripts/")) else {
            continue;
        };
        let text = std::fs::read_to_string(root().join(script))
            .unwrap_or_else(|_| panic!("{gate} is excused to {script}, which is not in the tree"));
        assert!(text.contains(&gate), "{gate} is excused to {script}, which does not run it");
        delegated += 1;
    }
    assert!(delegated > 0, "at least one excuse delegates to another sweep");
}

/// The sweep runs something that reads a `data-golden` span.
///
/// This is the property the missed round turned on. Pinning the NAME
/// golden_prose would go green on a rename while the coverage was gone, so
/// the assertion is that some program the sweep runs reads the attribute.
#[test]
fn the_sweep_reads_the_golden_quoting_spans() {
    let reads = table("gates").into_iter().any(|(gate, _)| {
        std::fs::read_to_string(root().join(format!("scripts/{gate}/{gate}.kso")))
            .is_ok_and(|src| src.contains("data-golden"))
    });
    assert!(
        reads,
        "scripts/gates/all_pages.sh runs nothing that reads a data-golden span — \
         that is the one page property page_drift and prose_check cannot see, \
         and running the two of them and pushing is what this sweep is for"
    );
}
