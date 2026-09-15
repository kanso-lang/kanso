//! The maps parse is external state, and the compile rows do not count it.
//! Ruled 2026-09-15.
//!
//! Rust's `std::rt::lang_start_internal` calls `pthread_getattr_np` once,
//! before `main`, to place the main thread's stack guard, and glibc answers by
//! parsing `/proc/self/maps` with `getline` and `sscanf`. What that costs
//! follows the number of lines in that file, which follows the binary's
//! section layout: a 64 KiB `.bss` addition that no execution reaches moved
//! the whole-process count by 2,128 with every instruction the compiler
//! retired identical. Clay: "this has nothing to do with compiler performance
//! and obviously shouldn't be part of what we measure."
//!
//! The normalization is the anchor. Each of the three compile gates reads its
//! row as `kanso::main` inclusive out of the callgrind profile, and the parse
//! sits entirely above that frame — measured 2026-09-15 on four binaries whose
//! `.bss` and `.text` differ and whose `kanso::main` frame reads 43,472,369 on
//! every one of them. The gate's own error text names the anchor as the answer
//! to the last reproduction failure this vein had, and `src/main.rs` carries
//! `#[inline(never)]` on `main` so a future compiler cannot fold the frame into
//! the shim that calls it and take the anchor with it.
//!
//! What this pins is that no gate goes back to the whole process. The cheap
//! regression is a gate reading callgrind's `summary:` line, which is one grep
//! shorter than the anchored read and counts the loader and the parse again.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The three instruction gates the compile term is read from.
const GATES: [&str; 3] = [
    "scripts/gates/compile_instructions.sh",
    "scripts/gates/entry_instructions.sh",
    "scripts/gates/library_instructions.sh",
];

/// The read every gate makes: the first `kanso::main` line of an inclusive
/// annotation, thousands separators stripped.
const ANCHOR: &str = "awk '/kanso::main/ && !seen { gsub(/,/, \"\", $1); print $1; seen = 1 }'";

#[test]
fn every_compile_gate_reads_its_row_at_the_compilers_main() {
    for gate in GATES {
        let body = std::fs::read_to_string(root().join(gate)).expect("a compile gate reads");
        assert!(
            body.contains(ANCHOR),
            "{gate} no longer reads its row as `kanso::main` inclusive. The row \
             is the compiler's own frame so that Rust's startup, and the \
             /proc/self/maps parse inside it, fall outside the count."
        );
    }
}

#[test]
fn no_compile_gate_reads_the_whole_process() {
    for gate in GATES {
        let body = std::fs::read_to_string(root().join(gate)).expect("a compile gate reads");
        let reads_summary = body
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .any(|line| line.contains("summary:"));
        assert!(
            !reads_summary,
            "{gate} reads callgrind's summary line, which is the whole process: \
             the loader, Rust's stack-guard placement and its parse of \
             /proc/self/maps. That parse is external state, ruled 2026-09-15, \
             and the row counts from `kanso::main` so it is not in the row."
        );
    }
}

#[test]
fn the_anchor_frame_cannot_be_inlined_away() {
    let body = std::fs::read_to_string(root().join("src/main.rs")).expect("src/main.rs reads");
    let lines: Vec<&str> = body.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("fn main()"))
        .expect("src/main.rs declares fn main");
    let attributes: Vec<&str> =
        lines[..at].iter().rev().take_while(|line| line.starts_with("#[")).copied().collect();
    assert!(
        attributes.contains(&"#[inline(never)]"),
        "fn main in src/main.rs carries {attributes:?} and not #[inline(never)]. \
         The three compile gates read their rows at the kanso::main frame; a \
         compiler that folded it into the shim would leave them nothing to read."
    );
}
