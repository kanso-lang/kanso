//! The same sources, from two directories, compiled to the same program.
//!
//! On 2026-08-31 they were not. The beat analysis decided which loops could
//! rewind their arena by asking whether the declaration's `file` began `std/`
//! or `lib/`, and `file` is the field error origins are built from — a path
//! for a diagnostic, read as a semantic marker. So a package kept in a
//! directory called `lib` compiled to a program that never reclaimed a block,
//! and the same package one directory over compiled to one that did. Eighty-six
//! times the peak, from the name of a folder.
//!
//! Both arms build a digest, because that is the shape the difference is
//! largest on and the shape the loss is unbounded on: sha256 walks a message
//! sixty-four bytes at a time and everything a block builds is dead when the
//! next one starts, so a compiler that reclaims holds one block and a compiler
//! that does not holds the message.
//!
//! The digest is COPIED into the package rather than imported from `std/`.
//! That is what isolates the folder: a `std/sha256` import reads `std/` in
//! both arms and so answers the same in both, defect or no defect, which is a
//! passing test that proves nothing. `tests/sha256_peak.rs` is the spec for
//! the import; this one is the spec for the directory, and it needs the loops
//! under test to be the package's own.
//!
//! WHAT THE FIRST DRAFTS GOT WRONG, so they are not re-derived. A nineteen-byte
//! message put both arms under the arena's 1 MiB first block, where every
//! program reads the same peak. `current_dir(at)` with a bare `.` argument
//! stamps `./main.kso` in both arms, so the directory name never reaches
//! `file` at all — the run happens from the grandparent and names `lib/app`
//! and `elsewhere/app` on the command line. And a `std/sha256` import hid the
//! difference a third time, which is the paragraph above.

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

/// The equality is the spec; the constant beside it is what says which way the
/// equality was reached. Two arms that agree at 30 MB would satisfy the first
/// assertion and none of the point.
#[test]
fn a_package_compiles_the_same_wherever_it_is_kept() {
    let in_lib = peak_under("lib");
    let elsewhere = peak_under("elsewhere");

    assert_eq!(
        in_lib, elsewhere,
        "the directory changed the program: {in_lib} bytes under lib/, \
         {elsewhere} bytes under elsewhere/"
    );
    assert_eq!(in_lib, 1_048_576, "the digest stopped holding one block");
}
