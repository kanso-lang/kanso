//! What counts as using an import.
//!
//! Two walks answer it. One marks every module qualifier a name mentions;
//! the other marks bare names, because the bare overload space makes spelling
//! optional and not the dependency. Both read `Expr::Ident` and stopped there,
//! so three ways of naming an imported thing marked nothing — and a file whose
//! only use was one of them could not be written at all: drop the import and
//! the name does not resolve, keep it and the check refuses the file.

use std::path::PathBuf;
use std::process::Command;

const SHAPES: &str = "type num int\n\npub fn make x\n  num x\n";

fn check(dir: &PathBuf) -> (String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(dir)
        .output()
        .expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.status.code())
}

fn staged(name: &str, entry: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-import-use-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("staging");
    std::fs::write(dir.join("shapes.kso"), SHAPES).expect("the module writes");
    std::fs::write(dir.join("main.kso"), entry).expect("the entry writes");
    dir
}

/// `(n):shapes/num` names `shapes` the way an annotation does.
#[test]
fn a_widening_target_uses_the_import() {
    let dir = staged("upcast", "import \"./shapes\"\n\nn = 3\n\nprint \"{(n):shapes/num}\"\n");
    let (err, code) = check(&dir);
    assert!(!err.contains("unused import"), "the widening target marked nothing: {err}");
    assert_eq!(code, Some(0), "{err}");
}

/// `&shapes/make` names `shapes` the way a call does; the sigil holds the
/// name rather than changing it.
#[test]
fn a_held_function_uses_the_import() {
    let dir = staged("partial", "import \"./shapes\"\n\nf = &shapes/make\n\nprint \"{f 3}\"\n");
    let (err, code) = check(&dir);
    assert!(!err.contains("unused import"), "the held name marked nothing: {err}");
    assert_eq!(code, Some(0), "{err}");
}

/// The bare overload space makes spelling optional, not the dependency: a
/// name any import exports can be written without its qualifier. `&make` is
/// that name held rather than called, and the walk that collects bare uses
/// read `Expr::Ident` and nothing else.
#[test]
fn a_held_name_uses_the_import_without_its_qualifier() {
    let dir = staged("bare", "import \"./shapes\"\n\nf = &make\n\nprint \"{f 3}\"\n");
    let (err, code) = check(&dir);
    assert!(!err.contains("unused import"), "the bare held name marked nothing: {err}");
    assert_eq!(code, Some(0), "{err}");
}

/// The control, so widening the walk cannot pass by marking every import used.
#[test]
fn an_import_nothing_names_is_still_refused() {
    let dir = staged("idle", "import \"./shapes\"\n\nprint \"nothing\"\n");
    let (err, code) = check(&dir);
    assert!(err.contains("unused import"), "an idle import passed: {err}");
    assert_eq!(code, Some(2));
}
