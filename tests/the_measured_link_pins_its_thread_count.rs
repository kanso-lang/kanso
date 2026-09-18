//! The gate that counts the linker pins how many threads it may use, and the
//! compiler honours the ask.
//!
//! `ld` splits LTO codegen across threads. callgrind counts every thread, and
//! how the work lands is the scheduler's to decide rather than the input's, so
//! a row that counts a parallel link counts the scheduler with it. Two links
//! of byte-identical bitcode on one container read 20,565,047,254 and
//! 20,565,047,243 on one pair and 20,565,047,241 and 20,565,049,584 on the
//! next; with `jobs=1` the same pair read 20,574,502,681 twice.
//!
//! That is the 2026-09-15 rule: the state is put into a known one before the
//! measurement rather than explained after it. The property lives in two files
//! -- the gate has to ask and the compiler has to pass the ask on -- and a
//! property split across two files is one nothing checks unless something
//! reads both. kanso#1487 certified this row's child tree deterministic and it
//! drew two faces again within the day, because the term that was left had no
//! check of its own.
//!
//! This is deliberately NOT a default. A user's release build has no row to
//! keep and every reason to use its cores, so the variable is unset there and
//! `release_clang` adds nothing.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The gate's own build invocations run under `env -i`, so every variable the
/// measurement depends on is named on that line and nowhere else.
#[test]
fn the_codegen_gate_asks_for_one_thread_on_every_invocation() {
    let p = root().join("scripts/gates/codegen_instructions.sh");
    let src = std::fs::read_to_string(&p).expect("the codegen gate reads");

    // The LINES that set up an environment, not every mention of the name --
    // the paragraph above them in the gate explains the pinning and says the
    // variable out loud, and a substring count would let that paragraph stand
    // in for the thing it describes.
    let envs: Vec<&str> = src.lines().filter(|l| l.contains("env -i ")).collect();
    assert!(
        !envs.is_empty(),
        "the codegen gate no longer runs under `env -i`, so this test is \
         looking in the wrong place -- find where the measurement's \
         environment is set and check there instead"
    );
    let bare: Vec<&&str> = envs.iter().filter(|l| !l.contains("KANSO_LTO_JOBS=1")).collect();
    assert!(
        bare.is_empty(),
        "these lines build under `env -i` without pinning the LTO thread \
         count, so they count the scheduler along with the linker: {bare:#?}"
    );
}

/// The ask is worth nothing if the compiler drops it on the floor.
#[test]
fn release_clang_passes_the_ask_to_the_linker() {
    let p = root().join("src/main.rs");
    let src = std::fs::read_to_string(&p).expect("src/main.rs reads");

    let release = src.split_once("fn release_clang").expect("release_clang is there").1;
    let dev_at = release.find("fn dev_clang").unwrap_or(release.len());
    let release = &release[..dev_at];

    assert!(
        release.contains("KANSO_LTO_JOBS"),
        "release_clang does not read KANSO_LTO_JOBS, so the codegen gate's ask \
         reaches nothing and the row goes back to counting the scheduler"
    );
    assert!(
        release.contains("-Wl,-plugin-opt=jobs="),
        "release_clang reads the variable but does not turn it into the option \
         the linker understands. `--lto-obj-path` and `--thinlto-jobs` are \
         lld's spellings and this ld rejects them; the GNU plugin takes \
         `-Wl,-plugin-opt=jobs=N`."
    );

    // The dev tier links at -O0 with no LTO, so it has nothing to pin and
    // pinning it would be a flag the linker ignores.
    let dev = &src[src.find("fn dev_clang").expect("dev_clang is there")..];
    let dev = &dev[..dev.find("fn cached_runtime_object").unwrap_or(dev.len())];
    assert!(
        !dev.contains("KANSO_LTO_JOBS"),
        "dev_clang pins an LTO thread count, and the dev tier does not run LTO"
    );
}
