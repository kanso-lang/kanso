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
//! or a verdict. The fixture appends and pushes 300 times each; copying asks
//! 1,200 times more than extending in place, one ask per copy. A spec written
//! against the copy itself -- a counter on the clone, a probe of
//! `Rc::strong_count` -- would go green the moment the decomposition moved,
//! which is exactly when it needed to speak.
//!
//! THE TWO BYTE COUNTERS ARE EXCLUDED, and the exclusion is measured rather
//! than assumed. `interp_alloc_bytes` and `interp_peak_bytes` track the length
//! of the path the run was handed: the same fixture staged at `/tmp/chain`
//! reads 10,524,425 and 148,058, and staged at a name 34 characters longer
//! reads 10,578,250 and 148,485. `interp_allocs` reads 21,173 at both. A
//! spec that stages under `std::env::temp_dir()` runs on a path whose length
//! is the host's, so pinning either byte counter would pin macOS's
//! `/var/folders/...` against Linux's `/tmp`. That is the 2026-09-15 rule:
//! what cannot be normalized is left out, and the exclusion is named where a
//! reader will find it.

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

pub play = print "{length (grow (text/bytes "") 300)} {length (stack [] 300)}"
"#;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

/// Run the real binary on the staged fixture and answer what it printed and
/// what the interpreter's allocator counted.
fn ran() -> (String, Vec<(String, u64)>) {
    let stage = std::env::temp_dir().join("kanso-unique-container");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    std::fs::write(stage.join("builders.kso"), LIBRARY).expect("the library writes");
    let entry = stage.join("run_builders.kso");
    std::fs::write(&entry, "import \"./builders\"\n\nbuilders/play\n").expect("the entry writes");

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
    let _ = std::fs::remove_dir_all(&stage);
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
    let (printed, _) = ran();
    assert_eq!(printed, "1200 600\n", "600 appends of two bytes, and 600 pushes");
}

/// Exact, not a band. The copying interpreter asks 22,362 times for the same
/// answer; a tolerance wide enough to survive 1,200 extra allocations is wide
/// enough to survive the fix being removed.
#[test]
fn a_unique_container_is_extended_in_place() {
    let (_, counted) = ran();
    assert_eq!(
        counter(&counted, "interp_allocs"),
        21_162,
        "the interpreter copied a container no other reference points at. \
         Copying asks the allocator 22,362 times over this fixture, one ask \
         per copy; extending in place asks 21,162. `push`, `put` and \
         `append` in src/eval.rs take their container by value and hand it \
         to `taken`, which is `Rc::try_unwrap` with the clone as its other \
         arm. Counted: {counted:?}"
    );
}
