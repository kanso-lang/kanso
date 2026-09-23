//! A call into another module that answers a number gives back what it built.
//!
//! The construction cohort brackets a call whose arguments are immutable: the
//! arena is marked before it and rewound after it when the answer is a scalar,
//! or the answer is copied out when it is not. Until 2026-09-23 the license
//! asked whether the callee's module name extended the caller's by a segment,
//! the spelling modules had before identity became the canonical path. After
//! that change the test held only for a caller in the root module, so a
//! package's own modules calling each other were never bracketed. The archive
//! entry that found it (2026-08-18, "the qualified door") says the predicate
//! had to ask the real relation instead, and this spec pins it.
//!
//! The package below is that shape and nothing else. `app` calls
//! `phase/churn` twice, and each call builds about a megabyte of doubled
//! strings and answers their length. With the bracket the second call reuses
//! what the first gave back, and the peak is one call's. Without it the peak
//! holds both.
//!
//! `app/run` is itself called from the root module, which the old test did
//! admit, so the run program's one cohort pop happens either way. The counter
//! below is two with the license and one without.

use std::process::Command;

fn counters() -> String {
    let root = std::env::temp_dir().join("kanso-a-phase-gives-its-garbage-back");
    let _ = std::fs::remove_dir_all(&root);
    let pkg = root.join("prog");
    std::fs::create_dir_all(pkg.join("app").join("phase")).expect("a package to build");
    std::fs::write(
        pkg.join("app").join("phase").join("phase.kso"),
        "import \"std/text\"\n\n\
         fn doubled s n\n  wider s n (length s < n)\n\n\
         fn wider s _ false\n  s\n\n\
         fn wider s n true\n  doubled (text/join [s s] \"\") n\n\n\
         pub fn churn n\n  length (doubled \"ab\" n)\n",
    )
    .expect("the phase writes");
    std::fs::write(
        pkg.join("app").join("app.kso"),
        "import \"./phase\"\n\n\
         pub fn run n\n  first = phase/churn n\n  second = phase/churn n\n  first + second\n",
    )
    .expect("the app writes");
    std::fs::write(pkg.join("main.kso"), "import \"./app\"\n\nprint (app/run 600000)\n")
        .expect("the entry writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(".")
        .env("KANSO_COUNTERS", "1")
        .current_dir(&pkg)
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(String::from_utf8_lossy(&done.stdout), "2097152\n");
    String::from_utf8_lossy(&done.stderr).into_owned()
}

fn read(said: &str, key: &str) -> u64 {
    said.lines()
        .find_map(|l| l.trim().strip_prefix(key)?.strip_prefix('=')?.parse().ok())
        .unwrap_or_else(|| panic!("no {key} in:\n{said}"))
}

#[test]
fn a_call_between_a_packages_modules_is_a_cohort() {
    let said = counters();
    assert_eq!(read(&said, "cohort_frees"), 2, "the call into phase was not bracketed");
    assert_eq!(read(&said, "arena_peak_bytes"), 3_145_744, "the peak moved");
}
