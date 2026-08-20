//! Who checks the IR kanso writes.
//!
//! Nothing in this project reads the emitted .ll. `kanso build` writes it and
//! hands it to clang, so the only verifier a program ever meets is the one on
//! the machine that built it — and that verdict has already differed by host:
//! a thunk released in a block its creation did not dominate was refused on
//! macOS and compiled and run on linux, from identical source.
//!
//! The reason is recorded here because it decides what the project can rely
//! on: **clang only runs LLVM's verifier when it was built with assertions.**
//! Apple's clang is; the linux runner's is not, so on that host `clang -c` on
//! a .ll parses the module and codegens it without ever checking it. A tool
//! built with assertions — `opt`, `llvm-as` — checks on any host.
//!
//! So this asks each candidate in turn and needs one of them to refuse a
//! five-line dominance violation. A host with none of them says so instead of
//! going quietly green, and the emitter's own output is then put through
//! whichever answered.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Tools that read a .ll and answer, most-preferred first. The versioned
/// spellings are what the linux images ship; the bare ones are what a
/// developer's path has.
const CANDIDATES: [&str; 12] = [
    "opt",
    "llvm-as",
    "/opt/homebrew/opt/llvm/bin/opt",
    "opt-19",
    "llvm-as-19",
    "opt-18",
    "llvm-as-18",
    "opt-17",
    "llvm-as-17",
    "opt-16",
    "llvm-as-16",
    // last, because it answers only where it was built with assertions
    "clang",
];

fn read_ir(tool: &str, path: &Path) -> Option<Output> {
    let out = std::env::temp_dir().join("kanso_ir_check.out");
    let mut command = Command::new(tool);
    match tool {
        "clang" => command.arg("-c").arg("-o").arg(&out),
        _ if tool.contains("opt") => command.arg("-passes=verify").arg("-disable-output"),
        _ => command.arg("-o").arg(&out),
    };
    command.arg(path).output().ok()
}

/// A register used in a block its definition does not reach. The exact shape
/// LLVM refused in the thunk-release defect, reduced to five lines.
fn does_not_dominate() -> PathBuf {
    let bad = std::env::temp_dir().join("kanso_does_not_dominate.ll");
    std::fs::write(
        &bad,
        "define i64 @f(i64 %n) {\n\
         entry:\n  %c = icmp eq i64 %n, 0\n  br i1 %c, label %made, label %uses\n\n\
         made:\n  %v = add i64 %n, 1\n  br label %done\n\n\
         uses:\n  %w = add i64 %v, 2\n  br label %done\n\n\
         done:\n  %r = phi i64 [ 0, %made ], [ %w, %uses ]\n  ret i64 %r\n}\n",
    )
    .expect("the fixture writes");
    bad
}

/// The first tool on this host that refuses invalid IR. Every test here needs
/// one, and which one it is belongs in the failure message.
fn verifier() -> Option<&'static str> {
    let bad = does_not_dominate();
    CANDIDATES.into_iter().find(|tool| match read_ir(tool, &bad) {
        Some(answer) => {
            !answer.status.success()
                && String::from_utf8_lossy(&answer.stderr).contains("does not dominate all uses")
        }
        None => false,
    })
}

#[test]
fn this_host_has_something_that_reads_ir() {
    assert!(
        verifier().is_some(),
        "no tool on this host refuses IR that does not dominate its uses. \
         Tried {CANDIDATES:?}. clang is not one — it verifies only when built \
         with assertions, which the linux images are not, so a build here \
         checks nothing about the IR kanso wrote."
    );
}

/// And the emitter's own output, over the micro corpus — every construct the
/// emitter has a distinct path for, which is what that corpus is. Both
/// invalid-IR defects this project has had were found by accident on a laptop
/// whose clang happens to verify, so a check that reads one program is a check
/// that would have found neither.
#[test]
fn the_ir_kanso_writes_passes_that_verifier() {
    let Some(tool) = verifier() else {
        return; // the test above is the one that reports this
    };
    let dir = std::env::temp_dir().join("kanso_ir_written");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the directory is made");

    let corpus = manifest_dir().join("tests/golden/micro");
    let mut programs: Vec<PathBuf> = std::fs::read_dir(&corpus)
        .expect("the corpus is there")
        .filter_map(|entry| {
            let path = entry.expect("the entry reads").path();
            (path.extension()? == "kso").then_some(path)
        })
        .collect();
    programs.sort();
    assert!(programs.len() > 30, "the corpus moved: only {} programs", programs.len());

    let refused: Vec<String> = programs
        .iter()
        .filter_map(|program| {
            let name = program.file_stem().expect("kso files have names");
            let source = dir.join(program.file_name().expect("kso files have names"));
            std::fs::copy(program, &source).expect("the program copies");
            // Most corpus samples export `play` and are libraries, so what
            // gets built is the entry that imports one — the same wrapper the
            // runner writes, since the language knows no such name. A sample
            // that is already an entry file builds as it stands.
            let stem = name.to_str().expect("kso files have utf-8 names");
            let text = std::fs::read_to_string(program).expect("the sample reads");
            let exports_play = text.contains("\npub play") || text.starts_with("pub play");
            let built_stem = match exports_play {
                true => format!("built_{stem}"),
                false => stem.to_string(),
            };
            let entry = dir.join(format!("{built_stem}.kso"));
            if exports_play {
                std::fs::write(&entry, format!("import \"./{stem}\"\n\n{stem}/play\n"))
                    .expect("the entry file writes");
            }

            let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .arg("build")
                .arg(entry.file_name().expect("kso files have names"))
                .current_dir(&dir)
                .output()
                .expect("kanso runs");
            if !built.status.success() {
                let said = String::from_utf8_lossy(&built.stderr);
                return Some(format!("{name:?} does not build: {said}"));
            }

            let written = dir.join(&built_stem).with_extension("ll");
            let checked = read_ir(tool, &written).expect("the verifier runs");
            match checked.status.success() {
                true => None,
                false => {
                    let said = String::from_utf8_lossy(&checked.stderr);
                    Some(format!("{name:?}: {said}"))
                }
            }
        })
        .collect();

    assert!(
        refused.is_empty(),
        "{tool} refuses the IR kanso wrote for {} of {} programs:\n{}",
        refused.len(),
        programs.len(),
        refused.join("\n")
    );
}

/// Samples written to exercise the register-return convention, where a record
/// small enough travels as two words rather than a boxed value.
///
/// They sit inside a trap. Escape analysis decides the convention per GROUP,
/// so ONE escaping use boxes every use. A sample that grows a consumer the
/// analysis treats as escaping stops exercising the convention entirely,
/// keeps passing, and pins nothing — which nearly happened: a nine-consumer
/// sample written for this ran correctly on a compiler that had none of the
/// fixes, because every one of its consumers was boxed.
///
/// So each of these must still emit a `%parsed`-returning definition. When one
/// stops, the sample has quietly become a different test and wants splitting
/// rather than the assertion relaxed.
const CARRIED: &[&str] = &[
    "a_constant_naming_a_record_constant",
    "a_closure_captures_a_record",
    "a_field_read_of_a_carried_record",
    "a_carried_record_answered_by_name",
];

#[test]
fn the_carried_samples_still_carry_in_registers() {
    let micro = manifest_dir().join("tests/golden/micro");
    let dir = std::env::temp_dir().join("kanso_carried_samples");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the staging directory is made");

    let boxed: Vec<String> = CARRIED
        .iter()
        .filter_map(|stem| {
            let sample = micro.join(format!("{stem}.kso"));
            assert!(sample.exists(), "{stem} is listed here but not in the corpus");
            std::fs::copy(&sample, dir.join(format!("{stem}.kso"))).expect("the sample copies");
            let entry = dir.join(format!("built_{stem}.kso"));
            std::fs::write(&entry, format!("import \"./{stem}\"\n\n{stem}/play\n"))
                .expect("the entry file writes");

            let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .arg("build")
                .arg(entry.file_name().expect("kso files have names"))
                .current_dir(&dir)
                .output()
                .expect("kanso runs");
            assert!(built.status.success(), "{stem} does not build");

            let ir = std::fs::read_to_string(dir.join(format!("built_{stem}.ll")))
                .expect("the emitted ir reads");
            match ir.contains("define %parsed") {
                true => None,
                false => Some((*stem).to_string()),
            }
        })
        .collect();

    assert!(
        boxed.is_empty(),
        "these are written for the register convention and no longer reach it — every use \
         of their group is boxed, so they exercise nothing they were written for: {boxed:?}"
    );
}
