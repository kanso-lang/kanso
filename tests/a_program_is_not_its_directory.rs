//! The same sources, from two directories, compile to the same program.
//!
//! The beat analysis keeps an imported library's loops out of the carry tier,
//! and until 2026-10-10 it recognised a library by asking whether the
//! declaration's `file` began `std/` or `lib/`. Shipped modules are filed as
//! `std/<module>/<file>`, so the `lib/` arm matched nothing shipped and every
//! user directory called `lib`: a package kept there compiled to a program
//! that never reclaimed a block, five times the peak of the same package one
//! directory over. This file pinned that defect, as a fact nothing else
//! pinned, with the instruction to delete the assertion when the two agreed.
//!
//! WHY THE `lib/` ARM WAS THERE. The beat unit test for the json library
//! compiled `lib/json` as a root, and the arm made that root behave like an
//! installed module. The unit test now reads the library through
//! `import "std/json"`, which is how a program meets it. Removing the whole
//! exclusion was measured on 2026-08-31 and turned the digest quadratic; only
//! the `lib/` arm goes here, and the `std/` exclusion stays.
//!
//! The run happens from the grandparent and names `lib/app` and
//! `elsewhere/app` on the command line, because `current_dir(at)` with a bare
//! `.` stamps `./main.kso` in both arms and the directory never reaches
//! `file`. `churn` is a tail loop that builds a list it drops and hands the
//! next turn a list it keeps, which is the shape that needs the carry tier.
//!
//! Watched red with `|| d.file.starts_with("lib/")` put back in `beat.rs`:
//! the peak under `lib/` reads 5,242,880.

use std::process::Command;

/// Peak arena bytes for the same package built under `where_it_sits`.
fn peak_under(where_it_sits: &str) -> u64 {
    let root = std::env::temp_dir().join(format!("kanso-dirflag-{where_it_sits}"));
    let _ = std::fs::remove_dir_all(&root);
    let pkg = root.join(where_it_sits).join("app");
    std::fs::create_dir_all(pkg.join("churn")).expect("a package to build");
    std::fs::write(
        pkg.join("churn").join("churn.kso"),
        "pub fn churn 0 acc\n  acc\n\n\
         pub fn churn n acc\n  \
         junk = [n (n + 1) (n + 2) (n + 3) (n + 4) (n + 5) (n + 6) (n + 7)]\n  \
         churn (n - 1) [(acc[1] + junk[2]) (acc[2] + junk[7])]\n",
    )
    .expect("the loop writes");
    std::fs::write(pkg.join("main.kso"), "import \"./churn\"\n\nprint (churn/churn 20000 [0 0])\n")
        .expect("the entry writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("{where_it_sits}/app"))
        .env("KANSO_COUNTERS", "1")
        .current_dir(&root)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&root);

    said.lines()
        .find_map(|l| l.trim().strip_prefix("arena_peak_bytes=")?.parse().ok())
        .unwrap_or_else(|| panic!("no arena_peak_bytes for {where_it_sits}/app in:\n{said}"))
}

/// Both numbers are pinned, and they are the same number.
#[test]
fn the_directory_a_package_sits_in_does_not_change_its_memory() {
    let in_lib = peak_under("lib");
    let elsewhere = peak_under("elsewhere");

    assert_eq!(in_lib, 1_048_576, "the peak under lib/ moved");
    assert_eq!(elsewhere, 1_048_576, "the peak outside lib/ moved");
}
