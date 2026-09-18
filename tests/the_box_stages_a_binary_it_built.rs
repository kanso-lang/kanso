//! The staging script builds the compiler before it copies it.
//!
//! `scripts/gates/library_box.sh` stages the corpora and the compiler at a
//! fixed path, and every instruction gate that reads `/tmp/kanso-compile-ir`
//! reads whatever binary it put there. Until 2026-09-18 it copied
//! `./target/release/kanso` without building it, so the binary measured was
//! whatever was last built in the worktree the script ran from.
//!
//! That is not a hypothetical. On 2026-09-18 four runs staged out of a stale
//! target directory read the interpreted anchor 3,100,448 apart with
//! `sip::Hasher::write` live at 6.96% of the run, on a tree where no file in
//! `src/` declares a randomly-seeded container. The spread and the SipHash
//! both belonged to a compiler from before the 2026-09-16 fixed-seed fix.
//! Four runs of a release build of the same commit read one value, with no
//! SipHash frame at all. The wrong number reached a log entry and an open
//! pull request before anything caught it.
//!
//! `all_counters.sh` opens with `cargo build --release` and `all_compile.sh`
//! reaches the same build through `build_benchmarks.sh`, both for this
//! reason. This spec puts the third one under the same rule.
//!
//! THE COMMENTS COME OFF FIRST. A spec that greps the file as written passes
//! on a paragraph that merely mentions the build -- which is how
//! `the_measured_link_names_its_object.rs`'s first shape went green with a
//! site that had lost its variable.

use std::path::Path;

fn script() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates/library_box.sh");
    std::fs::read_to_string(p).expect("scripts/gates/library_box.sh is readable")
}

/// Every line with its comment removed, so only what the shell runs is read.
fn code(text: &str) -> Vec<&str> {
    text.lines().map(|l| l.trim()).filter(|l| !l.starts_with('#') && !l.is_empty()).collect()
}

#[test]
fn the_box_stages_a_binary_it_built() {
    let text = script();
    let lines = code(&text);

    let built = lines
        .iter()
        .position(|l| l.starts_with("cargo build --release"))
        .expect(
            "library_box.sh builds the compiler before it stages it. Without \
             that line the script copies whatever ./target/release/kanso \
             happens to hold, which on 2026-09-18 was a compiler four weeks \
             old, and every gate reading /tmp/kanso-compile-ir measured it.",
        );

    let staged = lines
        .iter()
        .position(|l| l.contains("cp ./target/release/kanso"))
        .expect("library_box.sh stages the compiler into the box");

    assert!(
        built < staged,
        "the build has to come BEFORE the copy: line {built} builds and line \
         {staged} copies, counting only lines the shell runs. A build after \
         the copy stages the previous binary and refreshes the target \
         directory for whoever measures next, which is the same defect one \
         run later."
    );
}

/// The copy reads exactly one path, and the spec above names it.
///
/// An allowlist that names `./target/release/kanso` is worth nothing if the
/// script later stages `$CARGO_TARGET_DIR/release/kanso` instead: the
/// position check would still pass and would be checking a line that no
/// longer stages the compiler.
#[test]
fn the_staged_path_is_the_one_the_build_writes() {
    let text = script();
    let staging: Vec<&str> =
        code(&text).into_iter().filter(|l| l.contains("/kanso\"") && l.starts_with("cp ")).collect();

    assert_eq!(
        staging.len(),
        1,
        "exactly one line copies a compiler into the box, and it is the one \
         the build above has to precede. Found: {staging:?}"
    );
    assert!(
        staging[0].contains("./target/release/kanso"),
        "the staged binary is the one `cargo build --release` writes, at \
         ./target/release/kanso. This line stages {:?} instead, so the build \
         line above no longer covers it.",
        staging[0]
    );
}
