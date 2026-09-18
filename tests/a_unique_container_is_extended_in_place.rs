//! A container nothing else points at is extended where it stands.
//!
//! `push`, `put` and `append` each answer with a new container, and the
//! interpreter built that answer by copying the old one every time. kanso#1497
//! profiled what that costs on the interpreted corpus: `__memcpy_avx_unaligned
//! _erms` is the largest single frame in the run, and 180,081,360 bytes of it
//! are `append` rebuilding an accumulator one byte at a time.
//!
//! That PR tried `Rc::try_unwrap` on `utf8`, measured a fall of 298,420, took
//! `append`'s refcount histogram -- unique on 1,326 of 36,966 calls -- and
//! declined the same fix for `append` without running it. Run, on the three
//! container builtins together, it is 14,194,625 instructions, 0.64% of the
//! interpreted row.
//!
//! WHICH SHAPE PAYS, because the histogram was not wrong about the shape it
//! measured. An accumulator threaded through a name the caller still holds --
//! `stack (push xs n) (n - 1)` -- is pointed at by that frame as well, so it
//! is not unique and nothing changes. An accumulator that arrives as another
//! call's answer -- `push (push xs n) n` -- is pointed at by nothing else, and
//! that is the shape this fixture is built out of. `lib/list`'s own
//! `put acc k (push (bucket acc[k]) x)` is the same shape, which is why the
//! corpus moved.
//!
//! WHAT IS PINNED is how many times the run asked the allocator, not a frame
//! or a verdict. A spec written against the copy itself -- a counter on the
//! clone, a probe of `Rc::strong_count` -- would go green the moment the
//! decomposition moved, which is exactly when it needed to speak.
//!
//! AND IT IS PINNED AS A DIFFERENCE, because an absolute count is the host's.
//! `interp_alloc_bytes` and `interp_peak_bytes` track the length of the path
//! the run was handed: the same fixture staged at `/tmp/chain` reads
//! 10,524,425 and 148,058, and staged at a name 34 characters longer reads
//! 10,578,250 and 148,485. `interp_allocs` held at 21,173 across that pair on
//! Linux, and pinning it anyway was wrong: macOS stages under
//! `/var/folders/...` rather than `/tmp` and the first CI round on the other
//! host went red on exactly this file.
//!
//! So the fixture runs the SAME program at two sizes, from entry files of the
//! same name length in the same directory, and pins what the second costs
//! over the first. Every fixed allocation — the loader, the path, the
//! library, the entry — is identical in both runs and cancels exactly. What
//! is left is what the extra 300 rounds cost, which is the thing the change
//! is about. That is the 2026-09-15 rule: a term that cannot be normalized is
//! not measured, and the way to normalize this one is to subtract it.
//!
//! Copying reads 19,201 for those rounds and extending in place 18,001 -- the
//! 1,200 copies the two builders would have made, one allocation each. The
//! absolute counts moved with the entry's name between two revisions of this
//! very file, 21,162 to 21,180, while the difference did not.

use std::path::PathBuf;
use std::process::Command;

/// Both builders hand the previous answer straight into the next call, so the
/// value being extended is pointed at by the argument and nothing else. The
/// two are here together because `append` copies bytes and `push` copies
/// values, and the fix has to reach both.
const LIBRARY: &str = r#"import "std/text"

fn grow acc 0
  acc

fn grow acc n
  grow (text/append (text/append acc "ab") "cd") (n - 1)

fn stack xs 0
  xs

fn stack xs n
  stack (push (push xs n) n) (n - 1)

pub fn run rounds
  print "{length (grow (text/bytes "") rounds)} {length (stack [] rounds)}"
"#;

/// What 300 extra rounds of the two builders cost, with every fixed
/// allocation cancelled by the subtraction.
const PER_EXTRA_ROUND: u64 = 18_001;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

/// Run the real binary on the staged fixture at one size and answer what it
/// printed and what the interpreter's allocator counted.
///
/// The two entry names are the same length on purpose: a run's allocations
/// track the length of the path it was handed, and the difference the tests
/// below take is only a cancellation if the two paths cost the same.
fn ran(rounds: u32) -> (String, Vec<(String, u64)>) {
    let stage = std::env::temp_dir().join("kanso-unique-container");
    std::fs::create_dir_all(&stage).expect("a staging directory");
    std::fs::write(stage.join("builders.kso"), LIBRARY).expect("the library writes");
    let entry = stage.join(format!("run_{rounds}.kso"));
    std::fs::write(&entry, format!("import \"./builders\"\n\nbuilders/run {rounds}\n"))
        .expect("the entry writes");

    let out = Command::new(kanso())
        .arg("run")
        .arg(&entry)
        .arg("--interp")
        .env("KANSO_COUNTERS", "1")
        .output()
        .expect("kanso runs");
    assert!(
        out.status.success(),
        "the interpreted run failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let counted: Vec<(String, u64)> = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.starts_with("interp_"))
        .filter_map(|(key, value)| Some((key.to_string(), value.trim().parse().ok()?)))
        .collect();
    let printed = String::from_utf8_lossy(&out.stdout).to_string();
    (printed, counted)
}

fn counter(counted: &[(String, u64)], name: &str) -> u64 {
    counted
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| *value)
        .unwrap_or_else(|| panic!("the run counts {name}; it counted {counted:?}"))
}

/// The answer first: extending in place may not change what the program says.
#[test]
fn the_builders_answer_what_they_answered_before() {
    assert_eq!(ran(300).0, "1200 600\n", "600 appends of two bytes, and 600 pushes");
    assert_eq!(ran(600).0, "2400 1200\n", "twice the rounds, twice the answer");
}

/// Exact, not a band, and a difference rather than a count.
#[test]
fn a_unique_container_is_extended_in_place() {
    let small = counter(&ran(300).1, "interp_allocs");
    let large = counter(&ran(600).1, "interp_allocs");
    assert_eq!(
        large - small,
        PER_EXTRA_ROUND,
        "300 more rounds of the two builders cost {} allocations; extending \
         a unique container in place costs {PER_EXTRA_ROUND}. Every fixed \
         allocation is the same in both runs and cancels, so what is left is \
         what the rounds cost. `push`, `put` and `append` in src/eval.rs take \
         their container by value and hand it to `taken`, which is \
         `Rc::try_unwrap` with the clone as its other arm. Read {small} at \
         300 rounds and {large} at 600.",
        large - small
    );
}
