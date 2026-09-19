//! The same sources, from two directories, compile to the same program.
//!
//! They did not until 2026-09-19. The beat analysis decided which loops may
//! rewind their arena by asking whether the declaration's `file` begins `std/`
//! or `lib/`. `file` is the field error origins are built from -- a path meant
//! for a diagnostic, read as a semantic marker. So a package kept in a
//! directory called `lib` compiled to a program that never reclaimed a block,
//! and the same package one directory over compiled to one that did: at 131,072
//! bytes hashed, 32,505,888 against 7,340,064; on this file's own 4,096-byte
//! message, 2,097,152 against 1,048,576. Twice the peak here and four times it
//! there, from the name of a folder.
//!
//! THE PREFIX IS GONE and the rule reads the loop's own shape instead: an
//! imported group keeps a carry of at most one position. The two numbers agree
//! now, so this file asserts the property rather than the defect, and goes red
//! if a path ever decides a program's memory again.
//!
//! WHY THE FIRST FIX WAS NOT THIS ONE. Removing the prefix outright was built
//! and measured on 2026-08-31 and turned the digest quadratic: at 128 KB the
//! peak fell from 1,262,485,520 bytes to 4,194,320 and the wall time rose from
//! 1.3 seconds to 68. The second 2026-08-31 entry in design/compiler-log.md has
//! the curve. What separates the two is width: `sha256/compress` and
//! `sha256/turned` carry two positions each and are the whole of that cost;
//! `sha256/blocked` and `sha256/digested` carry one and are the whole of the
//! saving. Measured 2026-09-19, one group at a time.
//!
//! WHAT THE FIRST DRAFTS OF THIS FILE GOT WRONG, so they are not re-derived. A
//! nineteen-byte message put both arms under the arena's 1 MiB first block,
//! where every program reads the same peak. `current_dir(at)` with a bare `.`
//! argument stamps `./main.kso` in both arms, so the directory name never
//! reached `file` at all -- the run happens from the grandparent and names
//! `lib/app` and `elsewhere/app` on the command line. And a `std/sha256` import
//! reads `std/` in BOTH arms and so answered the same either way, which is a
//! passing test that proves nothing; the digest is copied into the package
//! instead, so the loops under test are the package's own.

use std::process::Command;

/// Peak arena bytes for the same package built under `where_it_sits`.
fn peak_under(where_it_sits: &str) -> u64 {
    let root = std::env::temp_dir().join(format!("kanso-dirflag-{where_it_sits}"));
    let _ = std::fs::remove_dir_all(&root);
    let pkg = root.join(where_it_sits).join("app");
    std::fs::create_dir_all(pkg.join("digest")).expect("a package to build");
    std::fs::create_dir_all(pkg.join("walk")).expect("a package to build");

    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib/sha256/sha256.kso");
    std::fs::copy(&source, pkg.join("digest").join("digest.kso")).expect("the digest copies");
    std::fs::write(
        pkg.join("walk").join("walk.kso"),
        "pub fn bytes 0 acc\n  acc\n\n\
         pub fn bytes n acc\n  bytes (n - 1) (push acc (n % 251))\n",
    )
    .expect("the byte builder writes");
    std::fs::write(
        pkg.join("main.kso"),
        "import \"./digest\"\nimport \"./walk\"\n\n\
         print (digest/hex (walk/bytes 4096 []))\n",
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

/// Both numbers are pinned exactly, and they are the same number. The pin is
/// what a package costs wherever it sits; the equality is what this file is
/// for.
#[test]
fn the_directory_a_package_sits_in_does_not_change_its_memory() {
    let in_lib = peak_under("lib");
    let elsewhere = peak_under("elsewhere");

    assert_eq!(in_lib, 1_048_576, "the peak under lib/ moved");
    assert_eq!(elsewhere, 1_048_576, "the peak outside lib/ moved");
    assert_eq!(in_lib, elsewhere, "a directory name is deciding a program's memory again");
}
