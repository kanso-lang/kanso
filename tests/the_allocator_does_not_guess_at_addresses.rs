//! The allocator asks the OS for memory at an address it picks at random, and
//! the compile rows count what that costs.
//!
//! mimalloc v3 hands `mmap` a 4 MiB-aligned hint out of a 4 TiB window, and in
//! a release build the base of that window is drawn from per-process entropy:
//! v3/src/os.c, `_mi_os_get_aligned_hint`, under
//! `#if (MI_SECURE>=1 || defined(NDEBUG))`. The page map commits its entries by
//! address, so one allocation costs a different number of instructions
//! depending on where it lands. On kanso#1463 that showed as one binary
//! reading 131,884,793 and then 131,884,271 for the entry row inside a single
//! job, with all sixteen of the functions that moved on mimalloc's OS path.
//! Three runs on one container, one binary, one corpus and one environment
//! asked for 0x48e11400000, 0x52844800000 and 0x38240c00000.
//!
//! `.cargo/config.toml` defines `MI_NO_ALIGNED_HINT`, which is mimalloc's own
//! switch for this, and the library row does not move for it.
//!
//! THE DEFINE IS A STRING IN A FILE THE COMPILER NEVER READS, which is why
//! this spec exists. A crate bump that renames the switch, or a config edit
//! that drops it, leaves the randomisation back on and the vein flickering
//! again with nobody told. So: read mimalloc's own source, assert the switch
//! is still spelled the way the config spells it, and assert the config still
//! spells it.

use std::path::PathBuf;

/// libmimalloc-sys vendors both v2 and v3 of the C library and its build
/// script picks v3 unless the `v2` feature is on, which this crate does not
/// set.
fn the_os_source_the_build_compiles() -> PathBuf {
    let home = std::env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("HOME").expect("a home directory")).join(".cargo")
    });
    let registry = home.join("registry").join("src");
    let mut found = Vec::new();
    for index in std::fs::read_dir(&registry).expect("the registry has been fetched") {
        let index = index.expect("a registry index directory").path();
        for crate_dir in std::fs::read_dir(&index).into_iter().flatten().flatten() {
            let name = crate_dir.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("libmimalloc-sys-") {
                continue;
            }
            let source = crate_dir.path().join("c_src/mimalloc/v3/src/os.c");
            if source.is_file() {
                found.push(source);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| panic!("no libmimalloc-sys os.c under {}", registry.display()))
}

fn the_cargo_config() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"))
        .expect(".cargo/config.toml reads")
}

#[test]
fn the_switch_the_config_sets_is_the_one_the_source_reads() {
    let source = std::fs::read_to_string(the_os_source_the_build_compiles()).expect("os.c reads");
    assert!(
        source.contains("!defined(MI_NO_ALIGNED_HINT)"),
        "mimalloc's os.c no longer guards its hinted allocation on \
         MI_NO_ALIGNED_HINT. The define in .cargo/config.toml now buys nothing, \
         and the compile rows are back to counting where the allocator guessed."
    );
    assert!(
        the_cargo_config().contains("-DMI_NO_ALIGNED_HINT"),
        ".cargo/config.toml no longer defines MI_NO_ALIGNED_HINT, so the \
         allocator picks a random base address per process again and the three \
         compile rows stop reproducing."
    );
}

#[test]
fn the_randomisation_is_still_what_the_switch_turns_off() {
    let source = std::fs::read_to_string(the_os_source_the_build_compiles()).expect("os.c reads");
    let hint = source
        .split_once("void* _mi_os_get_aligned_hint(")
        .expect("os.c declares _mi_os_get_aligned_hint")
        .1;
    let body = hint.split_once("\n}\n").expect("the function has a body").0;
    // A release build defines NDEBUG, so this branch is the live one and the
    // seed is per process. If mimalloc ever stops randomising here, the define
    // is no longer buying determinism and this spec should be reread rather
    // than deleted -- the hint would still move the row, just predictably.
    assert!(
        body.contains("defined(NDEBUG)") && body.contains("_mi_theap_random_next"),
        "mimalloc's aligned hint no longer seeds itself from per-process \
         randomness in a release build. Re-read why .cargo/config.toml turns \
         the hint off before trusting the reason written there."
    );
}
