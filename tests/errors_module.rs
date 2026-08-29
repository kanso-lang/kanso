//! Diagnostics raised inside a DEPENDENCY, not in the program the user named.
//!
//! tests/golden/errors holds a fixture per mistake and every one of them is a
//! single file, so the whole corpus asks the same question: does this
//! program's own mistake get reported. Nothing asked whether a mistake in a
//! library that a program merely imports gets reported, and that gap was
//! found the way gaps usually are — by needing the answer. Removing the
//! per-dependency `check_merged` pass turned on exactly this shape, and there
//! was no fixture to run it against.
//!
//! So this corpus is directory-shaped: each case is a module tree with a
//! library at fault and an entry that imports it, checked whole, with its
//! stderr pinned byte for byte the way the flat corpus pins its own.
//!
//! `kanso check` is run from the manifest directory against a RELATIVE path,
//! because the diagnostics name the module they came from and an absolute
//! path would pin the clone rather than the compiler.

use std::process::Command;

fn cases() -> Vec<std::path::PathBuf> {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/errors_module");
    let mut found: Vec<_> = std::fs::read_dir(&root)
        .expect("the module-error corpus reads")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    found.sort();
    found
}

#[test]
fn a_library_at_fault_is_reported_through_the_program_that_imports_it() {
    let cases = cases();
    assert!(!cases.is_empty(), "the module-error corpus is empty");
    for case in cases {
        let name = case.file_name().expect("a case is named").to_string_lossy().to_string();
        let relative = format!("tests/golden/errors_module/{name}");
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("check")
            .arg(&relative)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso binary runs");

        let golden = case.with_extension("stderr");
        let expected = std::fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("the golden reads for {name}"));

        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected,
            "diagnostics mismatch for {name}"
        );
        assert_eq!(output.status.code(), Some(2), "compile errors exit 2 for {name}");
        assert!(output.stdout.is_empty(), "no stdout on compile error for {name}");
    }
}

/// A golden can be regenerated into agreement with a bug, and this one was.
///
/// `resolve_import` carried `../` into the resolved path, so a module reached
/// as `../deep` from `mid/` named itself `mid/../deep` in every diagnostic it
/// raised. Both goldens above PINNED that spelling: the corpus had been
/// regenerated from whatever the compiler printed, and nobody read the path.
/// A golden is only as good as the last person who looked at it.
///
/// So this asserts a property instead. Import syntax is syntax — `./` and
/// `../` say where to look, not what the module is called — and no resolved
/// path may carry it. Regenerating the goldens cannot satisfy this, because it
/// does not compare against a file; the only way to make it pass is to resolve
/// the path properly. It covers every case in the corpus, including ones
/// written after it.
#[test]
fn no_diagnostic_names_the_path_a_module_was_reached_through() {
    let cases = cases();
    assert!(!cases.is_empty(), "the module-error corpus is empty");
    let mut checked = 0;
    for case in cases {
        let name = case.file_name().expect("a case is named").to_string_lossy().to_string();
        let relative = format!("tests/golden/errors_module/{name}");
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("check")
            .arg(&relative)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso binary runs");
        let said = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(
            !said.contains("/../") && !said.contains("/./"),
            "{name} names the path it was reached through rather than the module: {said}"
        );
        checked += 1;
    }
    // A loop that reached nothing would make the assertion above vacuous.
    assert!(checked >= 3, "only {checked} cases were checked");
}
