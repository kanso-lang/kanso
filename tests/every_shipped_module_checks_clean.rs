//! An import of a shipped module does not ask that module's merged check: the
//! module and everything it imports are the shipped library, fixed when the
//! binary is built, so the check answers the same empty list in every program.
//! It was asked once per import in every compile, which was a quarter of the
//! entry path's work. This is where the answer is asked instead, for every
//! module the loader serves, so a library change that breaks one is still a
//! red build rather than an error no compile reports.

use std::path::{Path, PathBuf};
use std::process::Command;

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
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib/imports.rs"))
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

/// The directories under lib/ that hold a module's source, as import paths.
fn modules(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let mut holds_source = false;
    for entry in std::fs::read_dir(dir).expect("lib/ reads").flatten() {
        let path = entry.path();
        if path.is_dir() {
            modules(&path, root, out);
        } else if path.extension().is_some_and(|x| x == "kso")
            && !path.to_string_lossy().ends_with("_test.kso")
        {
            holds_source = true;
        }
    }
    if holds_source {
        let rel = dir.strip_prefix(root).expect("under lib/").to_string_lossy().into_owned();
        out.push(rel);
    }
}

/// The table is everything under lib/. A module there and not in the table is
/// one an installed binary, with no lib/ beside it, and the browser, with no
/// filesystem, cannot import: `std/expect`, which chapter 7 teaches for test
/// failures, said "`std/expect` is not in the shipped library" outside a
/// checkout. Each module is imported from a directory with `KANSO_STD`
/// pointing at an empty one, so only the embedded copies can answer. The import
/// is unused and the compile is refused for that, which shows it was found.
#[test]
fn every_module_under_lib_resolves_without_a_lib_beside_the_binary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");
    let mut found = Vec::new();
    modules(&root, &root, &mut found);
    found.sort();
    assert!(found.len() > 10, "lib/ holds the shipped modules: {found:?}");

    let stage = std::env::temp_dir().join("kanso-every-shipped-module");
    let _ = std::fs::remove_dir_all(&stage);
    let empty = stage.join("no-std");
    std::fs::create_dir_all(&empty).expect("a directory to stage in");

    let mut missing = Vec::new();
    for module in &found {
        let file = stage.join("main.kso");
        std::fs::write(&file, format!("import \"std/{module}\"\n\nprint \"ok\"\n"))
            .expect("the program writes");
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(["check", "main.kso"])
            .env("KANSO_STD", &empty)
            .current_dir(&stage)
            .output()
            .expect("kanso runs");
        if String::from_utf8_lossy(&done.stderr).contains("is not in the shipped library") {
            missing.push(module.clone());
        }
    }
    assert_eq!(missing, Vec::<String>::new(), "modules an installed binary cannot import");
}
