//! A release build links with lld when clang can hand it the LTO link.
//!
//! lld takes a release build for 4.26% fewer instructions than GNU ld with
//! LLVM's plugin, and the program it links runs the same code. What a user can
//! see of the choice is the stamp the linker leaves in the binary's `.comment`
//! section: lld writes `Linker: ... LLD ...` and GNU ld writes nothing there.
//!
//! The spec asks the same question the build asks, independently: can this
//! clang link a one-line LTO program with `-fuse-ld=lld`. Where it can, the
//! release binary carries lld's stamp; where it cannot, it does not, because a
//! build that forced lld on a toolchain without one would fail outright.
//!
//! Watched red with `release_clang` passing `-fuse-ld=bfd` in its place: the
//! binary carried no stamp on a box whose clang links with lld.

#![cfg(target_os = "linux")]

use std::process::Command;

fn lld_can_link_lto(dir: &std::path::Path) -> bool {
    let ll = dir.join("probe.ll");
    std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").expect("the probe writes");
    Command::new("clang")
        .args(["-O1", "-flto", "-fuse-ld=lld", "-Wno-override-module"])
        .arg(&ll)
        .arg("-o")
        .arg(dir.join("probe"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn the_release_binary_names_its_linker() {
    let dir = std::env::temp_dir().join(format!("kanso_lld_spec_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("main.kso"), "print \"linked\"\n").expect("the program writes");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(dir.join("main")).output().expect("the binary runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "linked\n");

    let binary = std::fs::read(dir.join("main")).expect("the binary reads");
    let stamped =
        binary.windows(8).any(|w| w == b"Linker: ") && binary.windows(3).any(|w| w == b"LLD");
    let can = lld_can_link_lto(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        stamped, can,
        "lld can take the link: {can}; the binary carries its stamp: {stamped}"
    );
}
