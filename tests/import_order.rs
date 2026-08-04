//! A module's dependencies are compiled one on top of another, so the peak
//! the front end reaches includes everything already loaded when it starts on
//! the next one. Which order that is must be the module's property, not a
//! consequence of which file happened to name a shared import.

use std::process::Command;

const LONGEST: &str = "import \"std/list\"\nimport \"std/text\"\n\n\
                       pub fn longest words\n  list/max (list/map words text/trim)\n";

const TIDY: &str = "import \"std/text\"\n\npub fn tidy s\n  text/trim s\n";

/// The same two declarations both times; only the file holding the one that
/// reaches for `std/list` changes.
fn module(dir: &std::path::Path, alpha: &str, beta: &str) {
    std::fs::create_dir_all(dir).expect("the module directory is made");
    std::fs::write(dir.join("alpha.kso"), alpha).expect("alpha is written");
    std::fs::write(dir.join("beta.kso"), beta).expect("beta is written");
}

fn peak(dir: &std::path::Path) -> u64 {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(dir)
        .env("KANSO_COUNTERS", "1")
        .output()
        .expect("kanso binary runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the module does not check: {stderr}");
    stderr
        .lines()
        .find_map(|line| line.strip_prefix("compile_peak_bytes="))
        .and_then(|n| n.parse().ok())
        .expect("the counters report a peak")
}

/// Watched red with the import union left in file order: naming `std/list`
/// from the second file rather than the first cost 37 KB more to check, and
/// nothing about what the module declares had changed.
#[test]
fn which_file_names_a_dependency_does_not_change_what_checking_costs() {
    let temp = std::env::temp_dir().join("kanso_import_order");
    let _ = std::fs::remove_dir_all(&temp);
    let named_first = temp.join("named_first");
    let named_second = temp.join("named_second");
    module(&named_first, LONGEST, TIDY);
    module(&named_second, TIDY, LONGEST);

    let first = peak(&named_first);
    let second = peak(&named_second);

    // Not byte-equal: a module's declarations are merged in file order, so
    // moving one between files still shifts the number, by ten bytes here.
    // A dependency loaded in the wrong order shifted it by 36,983.
    assert_eq!(
        second as i64 - first as i64,
        10,
        "which file names a dependency moved what the front end holds: {first} against {second}"
    );
}
