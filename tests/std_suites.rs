//! The standard library's own tests, run by `cargo test`.
//!
//! They existed and nothing ran them. `kanso test lib/list/list_test.kso`
//! answered `unknown name find` for however long, because the only `kanso test`
//! CI ever ran was the kanso-json mirror's — so a suite could rot in place and
//! the whole board stayed green.
//!
//! Both shapes are covered here, because they take different paths through the
//! verb: the directory runs every test file beside the module, and a file runs
//! its own against the module it imports.

use std::process::Command;

fn tested(target: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["test", target])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

/// Every directory under lib/ that holds a `*_test.kso`, as the path the verb
/// is given (`lib/list`, `lib/net/http`). The walk goes all the way down: it
/// used to read one level, and lib/net/http's suite never ran.
fn suites() -> Vec<String> {
    fn walk(dir: &std::path::Path, rel: &str, found: &mut Vec<String>) {
        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .expect("the library reads")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entries.sort();
        if entries.iter().any(|p| p.to_string_lossy().ends_with("_test.kso")) {
            found.push(rel.to_string());
        }
        for sub in entries.iter().filter(|p| p.is_dir()) {
            let name = sub.file_name().expect("a directory has a name").to_string_lossy();
            walk(sub, &format!("{rel}/{name}"), found);
        }
    }
    let mut found = Vec::new();
    walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib"), "lib", &mut found);
    found
}

#[test]
fn every_standard_module_with_tests_passes_them() {
    let suites = suites();
    assert_eq!(suites.len(), 7, "the library's suites moved: {suites:?}");
    let mut refused = Vec::new();
    for name in &suites {
        let output = tested(name);
        let said = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        if !output.status.success() || !said.contains("0 failed") {
            refused.push(format!("  {name}: {}", said.trim()));
        }
    }
    assert!(refused.is_empty(), "standard suites that do not pass:\n{}", refused.join("\n"));
}

/// The file form takes a different path — it has to import the module the
/// file sits beside, or the test cannot see what it is testing.
#[test]
fn every_standard_suite_passes_named_as_a_file() {
    let mut refused = Vec::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in suites() {
        for entry in
            std::fs::read_dir(root.join(&name)).expect("the module reads").filter_map(Result::ok)
        {
            let file = entry.file_name().to_string_lossy().into_owned();
            if !file.ends_with("_test.kso") {
                continue;
            }
            let output = tested(&format!("{name}/{file}"));
            let said = String::from_utf8_lossy(&output.stdout).into_owned()
                + &String::from_utf8_lossy(&output.stderr);
            if !output.status.success() || !said.contains("0 failed") {
                refused.push(format!("  {name}/{file}: {}", said.trim()));
            }
        }
    }
    assert!(refused.is_empty(), "suites that do not pass by file:\n{}", refused.join("\n"));
}
