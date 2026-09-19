//! The measured link gives its LTO object a name the run chooses.
//!
//! clang writes it to `/tmp/<stem>-XXXXXX.o` with fresh hex every run. `ld`'s
//! LLVM plugin puts that path into a `StringMap`, and how far the probe walks
//! depends on the string. Measured on one binary and one corpus with every
//! other input held fixed, twenty-two object names:
//!
//! ```text
//! twenty names            5,163,341,031
//! `4b8c1a` and `fedcba`   5,163,341,042
//! ```
//!
//! Eleven apart, deterministic per name, about one name in eleven. That is the
//! eleven `codegen_instructions_release` has been disagreeing with itself by
//! since kanso#1507 pinned the plugin's thread count, and kanso#1502's own job
//! drew both buckets: 6,820,866,344 and then 6,820,866,355. Three rounds were
//! spent on it before the name was found, and one was spent asserting the name
//! was NOT it on five samples of a one-in-eleven effect.
//!
//! `-save-temps=obj` derives the object's name from the input, so the string is
//! the same every run. The flag rides `KANSO_FIXED_TEMPS`, which only the gate
//! sets, for the reason `KANSO_LTO_JOBS` is not the default either: it leaves a
//! file in the user's directory and a user's build has no row to keep.
//!
//! The object it leaves is then state the next build would find, which is
//! exactly what kanso#1512 cleared for the binary and the IR. So `clear_output`
//! removes all three, and this spec says so.

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{rel} is on disk at {path:?}"))
}

fn uncommented(text: &str) -> Vec<&str> {
    text.lines().map(str::trim).filter(|l| !l.starts_with('#') && !l.starts_with("//")).collect()
}

#[test]
fn the_release_link_passes_save_temps_only_when_the_variable_is_set() {
    let main = read("src/main.rs");
    let body = main
        .split_once("fn release_clang")
        .expect("main.rs defines release_clang")
        .1
        .split_once("\nfn ")
        .expect("release_clang is followed by another item")
        .0;
    // The comments in that body name both the flag and the variable, and an
    // ordering check that reads them is checking the prose. This asks the code.
    let code: String =
        body.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    assert!(
        code.contains("KANSO_FIXED_TEMPS"),
        "release_clang does not read KANSO_FIXED_TEMPS outside its comments"
    );
    assert!(
        code.contains("-save-temps=obj"),
        "release_clang does not pass -save-temps=obj outside its comments"
    );
    let flag = code.find("-save-temps=obj").expect("checked above");
    let var = code.find("KANSO_FIXED_TEMPS").expect("checked above");
    assert!(var < flag, "the flag is passed before the variable is read, so it is unconditional");
}

#[test]
fn every_measured_build_sets_the_variable() {
    let gate = read("scripts/gates/codegen_instructions.sh");
    let builds: Vec<&str> = uncommented(&gate)
        .into_iter()
        .filter(|l| l.contains("./kanso build pkg/codegen_corpus"))
        .collect();
    assert_eq!(builds.len(), 3, "expected three corpus builds, found {builds:#?}");

    // THE PINS TRAVEL TOGETHER. Counting the sites and asserting three was the
    // first shape of this, and it did not fail when one site lost the variable
    // -- a comment naming the pair made the count four, so dropping one left
    // three and the assertion passed. The invariant is per line instead: every
    // executable `env -i` that pins the thread count pins the object name too,
    // and the reverse.
    for line in uncommented(&gate) {
        if !line.contains("env -i") {
            continue;
        }
        assert_eq!(
            line.contains("KANSO_LTO_JOBS"),
            line.contains("KANSO_FIXED_TEMPS"),
            "one pin without the other on this line, so the measurement is \
             loose in one of its two places:\n  {line}"
        );
    }
}

#[test]
fn the_clear_removes_the_saved_object_too() {
    let gate = read("scripts/gates/codegen_instructions.sh");
    let body = gate
        .split_once("clear_output() {")
        .expect("the gate defines clear_output")
        .1
        .split_once('}')
        .expect("clear_output has a body")
        .0;
    for written in ["codegen_corpus", "codegen_corpus.ll", "codegen_corpus.o"] {
        assert!(
            body.contains(written),
            "clear_output does not remove {written}, which a measured build writes:\n{body}"
        );
    }
}
