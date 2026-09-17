//! The compiler's allocator skips mimalloc's aligned path, and the bound it
//! skips below is mimalloc's own guarantee rather than a guess.
//!
//! The mimalloc crate's `GlobalAlloc` hands every allocation to
//! `mi_malloc_aligned`, whatever alignment it asked for. That wrapper checks
//! the alignment is a power of two, builds a mask, takes a candidate block off
//! the free list and tests whether it is aligned, before handing back the
//! block `mi_malloc` would have handed back on its own. `kanso check
//! compile_corpus` spent 1,608,924 instructions in it; skipping it where it
//! has nothing to do took the row from 36,956,079 to 36,241,230.
//!
//! THE BOUND IS EIGHT AND NOT SIXTEEN, and mimalloc says why in its own
//! assertion, in v3/src/alloc.c:
//!
//! ```c
//! mi_assert_internal(page->block_size < MI_MAX_ALIGN_SIZE ||
//!                    _mi_is_aligned(block, MI_MAX_ALIGN_SIZE));
//! ```
//!
//! A block is sixteen-aligned *unless it is smaller than sixteen bytes*.
//! `Layout` carries size and alignment independently, so `align 16, size 8` is
//! spellable, and a bound of sixteen would hand exactly that to `mi_malloc`
//! and could get back an address the caller may not use. Eight holds for every
//! size.
//!
//! A run-time check cannot see that: with padding on, this build's smallest
//! bin is already sixteen bytes wide, so `align 16, size 8` comes back
//! sixteen-aligned whichever branch it took, and a spec written against it
//! passes on the wrong bound. The assumption lives in mimalloc's source, so
//! that is where it is pinned — the same treatment
//! `the_allocator_does_not_guess_at_addresses.rs` gives the aligned hint.

use std::path::PathBuf;

/// mimalloc's vendored v3 tree, under whichever registry checkout built it.
/// Same walk as `the_allocator_does_not_guess_at_addresses.rs`, which is the
/// other spec that pins an assumption to the allocator's own source: one level
/// into each registry index, then the crate directory by name. Walking blindly
/// does not work — the index directory carries a hyphen in its name and a
/// filter that skips those never descends, which is how the first draft of
/// this spec passed while reading nothing at all.
fn mimalloc_source(rel: &str) -> Option<String> {
    let home = std::env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("HOME").expect("a home directory")).join(".cargo")
    });
    let registry = home.join("registry").join("src");
    let mut found = Vec::new();
    for index in std::fs::read_dir(&registry).into_iter().flatten().flatten() {
        for crate_dir in std::fs::read_dir(index.path()).into_iter().flatten().flatten() {
            if !crate_dir.file_name().to_string_lossy().starts_with("libmimalloc-sys-") {
                continue;
            }
            let source = crate_dir.path().join("c_src/mimalloc/v3").join(rel);
            if source.is_file() {
                found.push(source);
            }
        }
    }
    found.sort();
    found.pop().map(|p| std::fs::read_to_string(p).expect("a mimalloc source reads"))
}

/// The guarantee the bound rests on, still written where it was read.
#[test]
fn mimalloc_still_only_promises_sixteen_to_a_block_that_big() {
    let text = mimalloc_source("src/alloc.c")
        .expect("libmimalloc-sys's vendored v3/src/alloc.c, which the build compiled");
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(
            "mi_assert_internal(page->block_size < MI_MAX_ALIGN_SIZE || \
             _mi_is_aligned(block, MI_MAX_ALIGN_SIZE));"
        ),
        "mimalloc no longer states, in v3/src/alloc.c, that a block is only \
         MI_MAX_ALIGN_SIZE-aligned when it is at least that big. src/main.rs \
         bypasses the aligned path below alignment 8 BECAUSE of that sentence. \
         Read what replaced it before changing the bound, and do not assume the \
         guarantee got stronger."
    );
    let types = mimalloc_source("include/mimalloc/types.h")
        .expect("libmimalloc-sys's vendored v3/include/mimalloc/types.h");
    assert!(
        types.contains("#define MI_MAX_ALIGN_SIZE  16"),
        "MI_MAX_ALIGN_SIZE is no longer 16, and the bound in src/main.rs was \
         chosen against 16."
    );
}

/// And the bound `src/main.rs` actually uses is the one that guarantee
/// allows. Read from the source because it cannot be read any other way: the
/// `#[global_allocator]` lives in the BINARY crate, and an integration test
/// links the library, so an allocation made from here goes through the test
/// harness's allocator and says nothing whatever about the compiler's. Two
/// run-time tests were written first, one asking for alignment 4096 in blocks
/// of 8 bytes, and both passed with the bound raised to 8192 — because
/// neither was ever reaching the code it was written for.
#[test]
fn the_bypass_bound_is_the_one_the_guarantee_allows() {
    let main =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
            .expect("src/main.rs reads");
    let line = main
        .lines()
        .find(|l| l.contains("layout.align() <="))
        .expect(
            "src/main.rs no longer bypasses mimalloc's aligned path on an alignment bound.              If the bypass is gone the compile rows are back up by about 715,000              instructions and this spec has nothing to guard; delete it in the same              commit rather than leaving it passing on a file it no longer describes.",
        );
    let bound: u32 = line
        .split("layout.align() <=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.trim_end_matches('{').trim().parse().ok())
        .unwrap_or_else(|| panic!("the bound is not a number: {line}"));
    assert_eq!(
        bound, 8,
        "src/main.rs bypasses mimalloc's aligned path below alignment {bound}. mimalloc          guarantees MI_MAX_ALIGN_SIZE (16) alignment only for a block at least that big,          so an allocation of `align 16, size 8` — which `Layout` can spell — may come          back 8-aligned from `mi_malloc`. Eight is the bound that holds for every size.          Raising it needs a reason stronger than \"it seemed to work\", because it          cannot be tested from here: this crate's tests do not link the binary that          carries the #[global_allocator], so a run-time check of it passes whatever          the bound is."
    );
}
