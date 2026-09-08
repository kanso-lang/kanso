//! The compile term is measured on a NAMED workload, not an inherited one.
//!
//! Ruled 2026-09-08. Until then the three compile gates checked `lib/json`, so
//! the term moved with whatever lib/json happened to import that week:
//! kanso#1291 retired the escape fold, `std/list` went with it, and the compile
//! rows halved with the compiler byte-identical. By the objective's definition
//! that read as a four-point rise; by what the term is for it measured nothing.
//!
//! What could revert this quietly is a one-word edit — `compile_corpus` back to
//! `lib/json` in any of the three gates — and no other check in the tree would
//! notice, because every golden would simply be re-based to the new workload
//! and agree with itself. So the workload is asserted here by name.

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// The three gates the objective's compile terms come from, and the probe that
/// has to agree with them or it answers a different question.
const READERS: &[&str] = &[
    "scripts/gates/compile_allocs.sh",
    "scripts/gates/compile_memory.sh",
    "scripts/gates/compile_instructions.sh",
    "scripts/compile_row_probe.sh",
];

#[test]
fn every_compile_gate_checks_the_fixed_corpus() {
    for rel in READERS {
        let src = read(rel);
        assert!(
            src.contains("./kanso check compile_corpus"),
            "{rel} does not check bench/compile_corpus. The compile term is \
             measured on a named corpus since 2026-09-08, so that a row moves \
             by a compiler change or by an edit to the corpus and not by a \
             library changing its imports."
        );
        assert!(
            !src.contains("./kanso check lib/json"),
            "{rel} still checks lib/json. That is the workload the 2026-09-08 \
             ruling replaced: kanso#1291 dropped std/list from lib/json and the \
             compile rows halved with the compiler byte-identical."
        );
    }
}

#[test]
fn the_box_stages_the_corpus_it_is_asked_to_check() {
    let box_sh = read("scripts/gates/library_box.sh");
    assert!(
        box_sh.contains("cp -R bench/compile_corpus"),
        "scripts/gates/library_box.sh stages lib/ and nothing else, so a gate \
         asked to check compile_corpus would find no such package. The corpus \
         lives under bench/ rather than lib/ because a benchmark is not the \
         library, which is exactly why it needs a staging line of its own."
    );
}

#[test]
fn the_corpus_names_the_modules_the_ruling_named() {
    let src = read("bench/compile_corpus/compile_corpus.kso");
    for m in ["std/json", "std/list", "std/testing", "std/text"] {
        assert!(
            src.contains(&format!("import \"{m}\"")),
            "bench/compile_corpus does not import {m}. The 2026-09-08 ruling \
             names lib/json plus the std modules the benchmarks import, and an \
             import silently dropped here moves the compile rows the same way \
             the lib/json workload did."
        );
    }
}
