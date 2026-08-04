//! A wrong-argument-count call to a function declared in a sibling file.
//!
//! The per-file pass sees one file, and a module is a directory of files
//! sharing one namespace, so every call across that boundary went unchecked.
//! The merged pass that would have caught it looked only at qualified names,
//! on the reasoning that a bare name might be a local holding a function
//! value — but no binding may shadow a declaration, so it never can be.
//!
//! What the reader met instead was the assembler. Codegen emitted
//! `d_adder_2` for a group that declares only `d_adder_1`, and an undefined
//! symbol surfaced as a clang error quoting LLVM IR from a temporary file.
//!
//! Both engines run every case, because the diagnostic has to be the same
//! diagnostic and neither may build the program.

use std::process::Command;

fn kanso(verb: &str, fixture: &str, engine: &[&str]) -> (String, String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg(verb)
        .arg(format!("tests/golden/arity/{fixture}"))
        .args(engine)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

const ENGINES: [&[&str]; 2] = [&[], &["--interp"]];

/// The call lives in a library file, the declaration in its sibling.
#[test]
fn a_sibling_files_arity_is_checked() {
    let (_, err, code) = kanso("check", "sibling", &[]);
    assert_eq!(
        err.lines().next(),
        Some("error[arity]: no 2-argument arm of `adder` (arms take 1) (module tests/golden/arity/sibling/sibling)"),
        "check did not report the arity: {err}"
    );
    assert_eq!(code, Some(2), "a compile error exits 2");
}

/// The same call from the entry file, against the module it imports.
#[test]
fn an_entry_files_arity_is_checked() {
    let (_, err, code) = kanso("check", "entry", &[]);
    assert_eq!(
        err.lines().next(),
        Some("error[arity]: no 2-argument arm of `lib/adder` (arms take 1)"),
        "check did not report the arity: {err}"
    );
    assert_eq!(code, Some(2), "a compile error exits 2");
}

/// Neither engine builds it, and neither shows the reader an assembler.
#[test]
fn no_engine_hands_the_reader_llvm() {
    for fixture in ["sibling", "entry"] {
        for engine in ENGINES {
            let (out, err, code) = kanso("run", fixture, engine);
            let named = match fixture {
                "entry" => "error[arity]: no 2-argument arm of `lib/adder`",
                _ => "error[arity]: no 2-argument arm of `adder`",
            };
            assert!(
                err.starts_with(named),
                "{fixture} {engine:?} did not report the arity: out={out:?} err={err:?}"
            );
            assert!(
                !err.contains("clang") && !err.contains(".ll:") && !err.contains("%KValue"),
                "{fixture} {engine:?} leaked compiler internals: {err}"
            );
            assert_eq!(code, Some(2), "{fixture} {engine:?} exits 2");
        }
    }
}

/// The same rule the ranking pass enforces between neighbours, applied across
/// a module. Two arms with the same shape mean the second can never be
/// reached, and a module is a directory of files whose arms need not sit next
/// to each other — so the neighbour comparison could not see it, and two
/// files each declaring `twice _` compiled and silently picked one.
#[test]
fn an_arm_no_call_can_reach_is_refused_across_files() {
    let (_, err, code) = kanso("check", "overlap", &[]);
    assert_eq!(
        err.lines().next(),
        Some(
            "error[dispatch]: overlapping overloads of `twice` are illegal \
             (module tests/golden/arity/overlap/overlap)"
        ),
        "check did not report the unreachable arm: {err}"
    );
    assert_eq!(code, Some(2), "a compile error exits 2");
}

/// Construction is positional and complete, and the same seam hid it as the
/// call arity: a type declared in one file of a module and built in another
/// was checked by nobody, and a foreign type never was. Native built the
/// over-wide record and printed it; the interpreter refused.
#[test]
fn a_construction_across_files_counts_its_fields() {
    let (_, err, code) = kanso("check", "construction", &[]);
    assert_eq!(
        err.lines().next(),
        Some(
            "error[arity]: `local` has 2 field(s), got 3 (construction is positional, \
             fields alphabetical) (module tests/golden/arity/construction/construction)"
        ),
        "check did not count the fields: {err}"
    );
    assert_eq!(code, Some(2), "a compile error exits 2");
}

/// A module's entry is synthesized from main.kso's statements, so a module
/// that declares its own `main` collides with it. Both halves of what is
/// wrong get said: a constant admits no overloads, and the entry takes none.
#[test]
fn a_module_with_no_entry_file_names_the_file() {
    let (_, err, code) = kanso("run", "no_entry", &[]);

    assert!(err.contains("a directory runs through `main.kso`, and this module has none"), "{err}");
    assert_eq!(code, Some(2));
}

#[test]
fn main_is_a_name_a_module_may_use() {
    // The entry a directory is run through is compiled in under a name no
    // program can spell, so `main` is free: `main.kso` names the file, and
    // nothing about that reserves the word inside it.
    for engine in [&[][..], &["--interp"][..]] {
        let (out, err, code) = kanso("run", "entry_main", engine);
        assert_eq!(err, "", "{engine:?}");
        assert_eq!(out, "42\n", "{engine:?}");
        assert_eq!(code, Some(0), "{engine:?}");
    }
}
