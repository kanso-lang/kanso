//! Each case in tests/golden/derive_here is an app run as a user runs one,
//! from its own directory: `kanso run main.kso` there. A module file beside
//! the entry is then named bare, `m.kso`, which tests/golden/derive, run from
//! the repository's root, never shows. Each case prints its `.stdout` on both
//! engines.

use std::process::Command;

#[test]
fn every_case_run_from_its_own_directory_says_what_its_golden_says() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/derive_here");
    let mut cases: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
        .expect("the derive_here corpus reads")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "the derive_here corpus is empty");
    for case in cases {
        let want = std::fs::read_to_string(case.with_extension("stdout")).expect("a golden");
        for engine in [&[][..], &["--interp"][..]] {
            let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .args(["run", "main.kso"])
                .args(engine)
                .current_dir(&case)
                .output()
                .expect("kanso runs");
            let err = String::from_utf8_lossy(&done.stderr);
            assert_eq!(err, "", "{} {engine:?} was refused", case.display());
            assert_eq!(
                String::from_utf8_lossy(&done.stdout),
                want,
                "{} {engine:?}",
                case.display()
            );
        }
    }
}
