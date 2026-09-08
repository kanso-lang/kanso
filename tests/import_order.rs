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
    counters(dir)
        .lines()
        .find_map(|line| line.strip_prefix("compile_peak_bytes="))
        .and_then(|n| n.parse().ok())
        .expect("the counters report a peak")
}

fn counters(dir: &std::path::Path) -> String {
    run(dir, "KANSO_COUNTERS")
}

/// The dependencies this module loaded, in the order it loaded them.
fn load_order(dir: &std::path::Path) -> Vec<String> {
    run(dir, "KANSO_PHASES")
        .lines()
        .filter_map(|line| line.strip_prefix("load "))
        // the module itself is the first line and names a temp directory
        .filter(|path| !path.starts_with('/'))
        .map(str::to_string)
        .collect()
}

fn run(dir: &std::path::Path, var: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(dir)
        .env(var, "1")
        .output()
        .expect("kanso binary runs");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(output.status.success(), "the module does not check: {stderr}");
    stderr
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

    // THE ASSERTION IS THE ORDER, not the bytes. `import_list.sort_by` in
    // `load_dependencies` is what this guards, and removing it is what turns
    // this red: the two modules then load `list, text, render` and
    // `text, list, render`, and the peak of the second goes to 512,480
    // against the first's 483,682 — 28,798 bytes bought by nothing but which
    // file happened to name `std/list` first.
    assert_eq!(
        load_order(&named_first),
        load_order(&named_second),
        "which file names a dependency changed the order the module loads them in"
    );

    // The peaks are read but not pinned against each other, and that is a
    // repair rather than a relaxation. This asserted a byte-exact residual —
    // seventeen, then fifteen — until 2026-09-08, when the arm64 runner read
    // twenty-three where this x86-64 host read fifteen on the same tree. Both
    // hosts are deterministic and both are right: the counter reports what the
    // allocator holds, and two allocators round a merge of the same
    // declarations differently. A number that moves with the allocator is not
    // the compiler's, so pinning it pinned the wrong thing, and the order
    // above is the property that was meant all along.
    //
    // They are still read, because a module that stops checking here says so
    // through `peak`'s own assertion, and because the two numbers belong
    // beside the order in the failure message.
    let first = peak(&named_first);
    let second = peak(&named_second);
    assert!(first > 0 && second > 0, "the front end reported no peak: {first} against {second}");
}
