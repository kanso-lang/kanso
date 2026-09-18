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
///
/// Three readings so far, and each drop is a change telling this spec what it
/// cost per round:
///
///     18,001   kanso#1515, where the number was first pinned
///     16,201   kanso#1516, six a round less
///     15,601   kanso#1517, two a round less again
///     12,001   the frame memory, twelve a round less
///     10,801   the bound name's second copy, four a round less
///      9,001   the environment holding a `Name`, six a round less again
///      7,803   the interpreter reading the linearity analysis, four a round
///              less again
///      6,003   arm selection reusing one pair of candidate buffers, six a
///              round less again
///
/// The six are `eval_ident`: it used to build an `Rc<str>` every time it
/// resolved a name to a reference, and it remembers the answer now, so the six
/// names each round mentions allocate once for the whole run rather than once
/// per mention. The two are the tail hop: `grow` and `stack` each tail-call
/// once a round, and a hop used to clone the whole overload vector where it now
/// takes a refcount. The twelve are `frame_of`, which built a formatted trace
/// line and a package lookup -- two allocations -- on every entry into every
/// body, so six body entries a round cost twelve.
///
/// The last two are one change taken in two steps, and the split says what
/// each half reached. A binding used to allocate its name twice: `match_one`
/// pushed `name.as_str().to_owned()` into a `Bindings`, and `bind` then did
/// `name.to_string()` on top of it. Taking the name by value in `bind` drops
/// the second copy for the bindings that arrive through a `Bindings` — four a
/// round. Storing a `Name` rather than a `String` drops the first copy for
/// every binding, whichever route it came by — six a round. Six body entries
/// a round is the same six the frame-memory row above counted, and four of
/// them are the ones a pattern match binds.
///
/// Measured by building all three trees rather than subtracting one number
/// from another: main reads 12,001, the by-value commit alone 10,801, and the
/// two together 9,001.
///
/// The last row is this spec doing the job it was written for from the other
/// side. Every one before it removed an allocation the interpreter was making
/// for no reason; this one removes the CLONE, at the sites
/// `linear::in_place_pushes` proves nothing else will read. Four a round is
/// the two builders' four extending calls, each of which used to copy the
/// accumulator and now writes through it.
///
/// It is also the only pin this repository has that the optimisation FIRES.
/// The five specs that catch it firing where it should not --
/// `a_list_held_twice_is_not_pushed_into` and its two siblings, the
/// differential micro loop, and this file's own assertion -- all stay green
/// if the gate silently stops matching. This number does not.
///
/// Each time, the number was re-read rather than the assertion widened. A
/// change in what the ROUNDS cost is exactly what the subtraction exists to
/// see, so this spec going red on those branches was it working.
///
/// The six are arm selection. `match_params` built `score` and `binds` at
/// `Vec::with_capacity` on every overload candidate and gave up the moment a
/// pattern refused, so a candidate that failed on its first parameter had
/// already paid for two allocations -- and selection tries every arm in the
/// group. `grow` and `stack` each have two arms and each is called three times
/// a round, so six candidate pairs a round stopped being allocated.
///
/// The 19,201 the copying arm read is from before kanso#1516 and has not been
/// re-measured under either change. What this spec pins is unchanged either
/// way: the in-place path costs less per round than the copying one.
const PER_EXTRA_ROUND: u64 = 6_003;

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
