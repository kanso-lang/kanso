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
//!
//! AND EACH TEST STAGES ITS OWN TREE, which the `staged` helper below explains
//! at length because it cost two red CI rounds on branches that had touched
//! neither the interpreter nor this file. The directory got seven characters
//! longer when the tag went into its name; the difference this file pins did
//! not move, which is the subtraction doing what the paragraph above says it
//! is for.

use std::path::{Path, PathBuf};
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
///      5,403   a call's parameters bound in one environment frame, two a
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
/// The two are the environment. `bind` pushed an `Rc<Env>` node per BINDING,
/// so a call with two parameters made two nodes; a whole call's parameters go
/// into one frame now. `grow acc n` and `stack xs n` each take two and each is
/// called once a round, so two nodes a round stopped being allocated. The
/// vector the frame holds is the one arm selection already filled, so the
/// frame itself costs nothing beyond the node.
///
/// The 19,201 the copying arm read is from before kanso#1516 and has not been
/// re-measured under either change. What this spec pins is unchanged either
/// way: the in-place path costs less per round than the copying one.
///
/// 5,403 -> 4,803 on the branch that keeps the dispatcher's SCORE buffer
/// across dispatches instead of allocating one per dispatch. Six hundred
/// fewer allocations over three hundred extra rounds is TWO A ROUND, and the
/// buffer is the cause: the A/B behind that change reads `__rust_alloc`
/// 1,362,891 -> 1,243,349 and `__rust_dealloc` by the same 119,542, with the
/// growth path untouched.
///
/// WHICH two of the round's dispatches stopped allocating is NOT established.
/// The paragraphs above decompose their own deltas by counting calls, and the
/// same arithmetic does not obviously land on two here: a score buffer was
/// allocated per dispatch-loop ITERATION, and how many of a round's
/// iterations carry a parameter list long enough to allocate one is not
/// something this file measures. Two a round is the measurement; the
/// decomposition is left open rather than guessed, because a wrong
/// decomposition written here is what the next reader would check their
/// change against.
///
/// The sibling test is the reason this re-read is safe: the builders answer
/// 1200 600 and 2400 1200 exactly as before, so nothing about the in-place
/// path changed, and the number moved DOWN, which is the wrong direction for
/// a container that stopped being extended in place.
const PER_EXTRA_ROUND: u64 = 4_803;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

/// Each caller of `ran` owns its own staging directory, named for the tag it
/// passes. Sharing one was a real defect rather than untidiness: cargo runs
/// the tests in a binary on parallel threads, `std::fs::write` truncates
/// before it writes, and a `kanso` started by one test read the library while
/// the other test's `File::create` had it at zero bytes. kanso#1502's Linux
/// job died on it with the library present and empty --
///
///     the interpreted run failed:
///     error[name]: unknown name `builders/run`
///       --> /tmp/kanso-unique-container/run_300.kso:3:1
///
/// -- and kanso#1529's macOS job the same way, on two branches whose diffs
/// touched neither the interpreter nor this file. That is the 2026-09-15 rule
/// read the other way round: the state a measurement reads has to be the
/// measurement's own, and a directory two threads write is nobody's.
///
/// The tags are all the same length, so two runs staged under different tags
/// are handed paths that cost the same.
fn staged(tag: &str) -> PathBuf {
    let stage = std::env::temp_dir().join(format!("kanso-unique-container-{tag}"));
    std::fs::create_dir_all(&stage).expect("a staging directory");
    std::fs::write(stage.join("builders.kso"), LIBRARY).expect("the library writes");
    stage
}

/// The two measuring tests' tags, and the pair the crossing spec stages.
const ANSWER: &str = "answer";
const ALLOCS: &str = "allocs";
const CROSS_A: &str = "crossa";
const CROSS_B: &str = "crossb";

/// Run the real binary on an ALREADY-STAGED tree at one size and answer what
/// it printed and what the interpreter's allocator counted.
///
/// It stages nothing itself. A helper that re-wrote the library on every call
/// would heal the very window this file's crossing spec holds open, and did:
/// the spec passed against the shared directory until the staging moved out.
///
/// The two entry names are the same length on purpose: a run's allocations
/// track the length of the path it was handed, and the difference the tests
/// below take is only a cancellation if the two paths cost the same. Every
/// tag is the same length for the same reason, so a reading taken under one
/// is comparable with a reading taken under another.
fn ran(stage: &Path, rounds: u32) -> (String, Vec<(String, u64)>) {
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
    let stage = staged(ANSWER);
    assert_eq!(ran(&stage, 300).0, "1200 600\n", "600 appends of two bytes, and 600 pushes");
    assert_eq!(ran(&stage, 600).0, "2400 1200\n", "twice the rounds, twice the answer");
}

/// Exact, not a band, and a difference rather than a count.
#[test]
fn a_unique_container_is_extended_in_place() {
    let stage = staged(ALLOCS);
    let small = counter(&ran(&stage, 300).1, "interp_allocs");
    let large = counter(&ran(&stage, 600).1, "interp_allocs");
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

/// One staging tree's library is not another's.
///
/// This is the spec for the defect above, and it makes the race deterministic
/// rather than waiting for it. `std::fs::write` truncates and then writes, so
/// a run that reads the library inside that window reads an empty file and
/// dies naming the function it cannot find. Truncating one tree's library and
/// leaving it truncated is that window held open.
///
/// Watched red by making `staged` ignore its tag, which is exactly the
/// directory this file used to share:
///
///     the interpreted run failed:
///     error[name]: unknown name `builders/run`
///
/// the same words CI reported on two branches that had touched neither the
/// interpreter nor this file.
#[test]
fn one_trees_truncated_library_is_not_another_trees() {
    let victim = staged(CROSS_A);
    let other = staged(CROSS_B);

    // `File::create` with nothing written after it: what every `fs::write`
    // here passes through, stopped at the point the other thread can see.
    std::fs::File::create(victim.join("builders.kso")).expect("the library truncates");
    assert_eq!(
        std::fs::metadata(victim.join("builders.kso")).expect("it is still there").len(),
        0,
        "the window is an empty file rather than a missing one, which is why \
         the run gets as far as resolving a name and fails on the name"
    );

    assert_eq!(
        ran(&other, 300).0,
        "1200 600\n",
        "a run under one tag reads its own library; truncating another tag's \
         may not reach it. Sharing one directory is how kanso#1502 and \
         kanso#1529 went red on branches that changed neither the interpreter \
         nor this file."
    );
}
