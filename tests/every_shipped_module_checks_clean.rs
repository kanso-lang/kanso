//! An import of a shipped module does not ask that module's merged check: the
//! module and everything it imports are the shipped library, fixed when the
//! binary is built, so the check answers the same empty list in every program.
//! It was asked once per import in every compile, which was a quarter of the
//! entry path's work. This is where the answer is asked instead, for every
//! module the loader serves, so a library change that breaks one is still a
//! red build rather than an error no compile reports.

use std::path::Path;

#[test]
fn every_shipped_module_checks_clean() {
    for path in kanso::SHIPPED_MODULES {
        if let Err(e) = kanso::check_shipped(path) {
            panic!("{path} does not check clean as the loader compiles it:\n{e}");
        }
    }
}

/// The list the check walks is the loader's table, read out of the source: a
/// module added to the table and not to the list would be skipped by every
/// import and checked by nothing.
#[test]
fn the_list_is_the_loaders_table() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("the loader's source reads");
    let mut table: Vec<&str> = source
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("\"std/"))
        .filter_map(|rest| rest.split_once("\" => "))
        .map(|(name, _)| name)
        .collect();
    table.sort_unstable();
    let mut listed: Vec<&str> =
        kanso::SHIPPED_MODULES.iter().map(|p| p.strip_prefix("std/").expect("std/")).collect();
    listed.sort_unstable();
    assert_eq!(table, listed);
}
