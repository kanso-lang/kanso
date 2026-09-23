//! The same sources, from two directories, do NOT compile to the same program.
//!
//! This asserts what the compiler DOES, not what it should do — the same shape
//! `tests/sha256_peak.rs` beside it uses, and for the same reason: the defect
//! is real, the fix is not free, and a fact nothing pins is a fact that can be
//! lost.
//!
//! The beat analysis decides which loops may rewind their arena by asking
//! whether the declaration's `file` begins `std/` or `lib/`. `file` is the
//! field error origins are built from — a path meant for a diagnostic, read as
//! a semantic marker. So a package kept in a directory called `lib` compiles to
//! a program that never reclaims a block, and the same package one directory
//! over compiles to one that does. Five times the peak, from the name of
//! a folder.
//!
//! WHY IT IS STILL HERE. Removing the test was built and measured on
//! 2026-08-31 and turned the digest quadratic: at 128 KB the peak falls from
//! 1,262,485,520 bytes to 4,194,320 and the wall time rises from 1.3 seconds to
//! 68. The second 2026-08-31 entry in design/compiler-log.md has the curve. So
//! the fix is a real fix and its first draft was a bad trade, and the entry
//! this file exists to make red is a better one.
//!
//! WHAT THE FIRST DRAFTS GOT WRONG, so they are not re-derived. A nineteen-byte
//! message put both arms under the arena's 1 MiB first block, where every
//! program reads the same peak. `current_dir(at)` with a bare `.` argument
//! stamps `./main.kso` in both arms, so the directory name never reaches `file`
//! at all — the run happens from the grandparent and names `lib/app` and
//! `elsewhere/app` on the command line. And a `std/` import reads `std/` in
//! BOTH arms and so answers the same either way, which is a passing test that
//! proves nothing; the loop under test is the package's own.
//!
//! WHY THE LOOP IS FIVE LINES AND NOT THE DIGEST. Until 2026-09-23 the package
//! was a copy of lib/sha256, whose block loop carried a list of eight words
//! from one block to the next. kanso#1580 rewrote the digest to carry the
//! eight words as eight scalar arguments, and a loop that carries no list
//! needs no carry tier, so both arms read 1,048,576 and the copy stopped
//! showing the defect. The defect had not moved. `churn` below is the shape
//! the digest used to have and nothing else: a tail loop that builds a list
//! it drops and hands the next turn a list it keeps. Under `lib/` it reads
//! 5,242,880; anywhere else, one block.

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
    std::fs::write(
        pkg.join("main.kso"),
        "import \"./churn\"\n\nprint (churn/churn 20000 [0 0])\n",
    )
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

/// Both numbers are pinned exactly. The one under `lib/` is what the defect
/// costs; the one beside it is what the same sources cost anywhere else. When
/// the two agree, this assertion is the thing to delete.
#[test]
fn the_directory_a_package_sits_in_changes_its_memory() {
    let in_lib = peak_under("lib");
    let elsewhere = peak_under("elsewhere");

    assert_eq!(in_lib, 5_242_880, "the peak under lib/ moved");
    assert_eq!(elsewhere, 1_048_576, "the peak outside lib/ moved");
    assert_ne!(
        in_lib, elsewhere,
        "the directory stopped changing the program — delete this spec and say so"
    );
}
