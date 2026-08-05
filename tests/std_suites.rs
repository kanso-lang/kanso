//! The standard library's own tests, run by `cargo test`.
//!
//! They existed and nothing ran them. `kanso test lib/list/list_test.kso`
//! answered `unknown name find` for however long, because the only `kanso test`
//! CI ever ran was the kanso-json mirror's — so a suite could rot in place and
//! the whole board stayed green.
//!
//! Both shapes are covered here, because they take different paths through the
//! verb: the directory runs a module's tests, and a file runs its own inside
//! the module it belongs to.

use std::process::Command;

fn tested(target: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["test", target])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

fn suites() -> Vec<std::path::PathBuf> {
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(&lib)
        .expect("the library is there")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|dir| {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .any(|e| e.file_name().to_string_lossy().ends_with("_test.kso"))
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_standard_module_with_tests_passes_them() {
    let suites = suites();
    assert!(suites.len() >= 4, "the library's suites moved: found {}", suites.len());
    let mut refused = Vec::new();
    for dir in &suites {
        let name = dir.file_name().expect("a directory has a name").to_string_lossy();
        let output = tested(&format!("lib/{name}"));
        let said = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        if !output.status.success() || !said.contains("0 failed") {
            refused.push(format!("  lib/{name}: {}", said.trim()));
        }
    }
    assert!(refused.is_empty(), "standard suites that do not pass:\n{}", refused.join("\n"));
}

/// The file form takes a different path — it has to compile the module the
/// file belongs to, or the test cannot see what it is testing.
#[test]
fn every_standard_suite_passes_named_as_a_file() {
    let mut refused = Vec::new();
    for dir in suites() {
        let name = dir.file_name().expect("a directory has a name").to_string_lossy().into_owned();
        for entry in std::fs::read_dir(&dir).expect("the module reads").filter_map(Result::ok) {
            let file = entry.file_name().to_string_lossy().into_owned();
            if !file.ends_with("_test.kso") {
                continue;
            }
            let output = tested(&format!("lib/{name}/{file}"));
            let said = String::from_utf8_lossy(&output.stdout).into_owned()
                + &String::from_utf8_lossy(&output.stderr);
            if !output.status.success() || !said.contains("0 failed") {
                refused.push(format!("  lib/{name}/{file}: {}", said.trim()));
            }
        }
    }
    assert!(refused.is_empty(), "suites that do not pass by file:\n{}", refused.join("\n"));
}
