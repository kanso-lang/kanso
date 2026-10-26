//! The run program the objective weighs carries five of its stress shapes
//! from the benchmarks that diagnose them, and the benchmark survives as a
//! diagnostic golden. Two copies of one workload drift silently: `runbench`
//! would go on reporting a shape that `escapebench` had stopped exercising.
//!
//! So the run program holds no copy. Each carried module is a symbolic link to
//! the benchmark's file, which keeps the module where the run program imports
//! it and its functions' names unchanged, and this says each one is a link.
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(the link under bench/runbench, the benchmark module it points at)`.
///
/// A shape added to the run program without a line here is a shape nothing
/// pins, which is why the count is asserted at the end rather than left to
/// whatever the list happens to hold.
const CARRIED: &[(&str, &str)] = &[
    ("bench/runbench/runbench/deep/bench.kso", "bench/deepbench/deepbench/bench.kso"),
    ("bench/runbench/runbench/pend/bench.kso", "bench/pendbench/pendbench/bench.kso"),
    ("bench/runbench/runbench/escape/mod.kso", "bench/escapebench/escapebench/mod.kso"),
    ("bench/runbench/runbench/index/indexbench.kso", "bench/indexbench/indexbench/indexbench.kso"),
    ("bench/runbench/runbench/split/scanbench.kso", "bench/scanbench/scanbench/scanbench.kso"),
];

/// Phases that joined the run program on 2026-10-10 from a port's reduced
/// fixture rather than from a benchmark, measured on main before the fix each
/// one answers. No benchmark carries them, so there is no second copy to
/// drift from; the mem fixture each fix adds says where a move came from.
/// `restate` joined on 2026-10-11, measured on the compiler before kanso#1840,
/// and `insist` on 2026-10-19, measured on main before kanso#1943's fix.
/// `guarded`, `handed`, `overwrite` and `reentered` joined on 2026-10-25 from
/// kanso#2089, measured on main before their fixes, and `branched` and
/// `padded` later that day, measured on main before theirs. `relayed` joined
/// on 2026-10-26, measured on main before its fix.
const BORN_HERE: &[&str] = &[
    "bench/runbench/runbench/admitted/admitted.kso",
    "bench/runbench/runbench/appended/appended.kso",
    "bench/runbench/runbench/bag/bag.kso",
    "bench/runbench/runbench/branched/branched.kso",
    "bench/runbench/runbench/bucketed/bucketed.kso",
    "bench/runbench/runbench/bump/bump.kso",
    "bench/runbench/runbench/counted/counted.kso",
    "bench/runbench/runbench/deduped/deduped.kso",
    "bench/runbench/runbench/drained/drained.kso",
    "bench/runbench/runbench/fold/fold.kso",
    "bench/runbench/runbench/guarded/guarded.kso",
    "bench/runbench/runbench/handed/handed.kso",
    "bench/runbench/runbench/insist/insist.kso",
    "bench/runbench/runbench/lasted/lasted.kso",
    "bench/runbench/runbench/machine/machine.kso",
    "bench/runbench/runbench/nested/nested.kso",
    "bench/runbench/runbench/overwrite/overwrite.kso",
    "bench/runbench/runbench/padded/padded.kso",
    "bench/runbench/runbench/reentered/reentered.kso",
    "bench/runbench/runbench/relayed/relayed.kso",
    "bench/runbench/runbench/restate/restate.kso",
    "bench/runbench/runbench/rested/rested.kso",
    "bench/runbench/runbench/rewind/rewind.kso",
    "bench/runbench/runbench/scatter/scatter.kso",
    "bench/runbench/runbench/stack/stack.kso",
];

fn read(rel: &str) -> String {
    let path: PathBuf = manifest_dir().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

#[test]
fn the_run_program_carries_the_shapes_unchanged() {
    for (link, source) in CARRIED {
        let path = manifest_dir().join(link);
        let meta = std::fs::symlink_metadata(&path).unwrap_or_else(|e| panic!("{link}: {e}"));
        assert!(
            meta.file_type().is_symlink(),
            "{link} is a file of its own, so it can drift from {source}. The run \
             program's shape and the benchmark that diagnoses it have to be the \
             same program; make it a link to {source}."
        );
        let target = std::fs::canonicalize(&path).unwrap_or_else(|e| panic!("{link}: {e}"));
        let want = std::fs::canonicalize(manifest_dir().join(source))
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(target, want, "{link} points somewhere other than {source}");
    }
    assert_eq!(CARRIED.len(), 5, "a shape joined or left the run program without a line here");
}

/// Every directory under `bench/runbench/runbench` that is not the driver
/// holds exactly one carried module, and that module is named above. A module
/// added and not listed would otherwise pass the loop by not being in it.
#[test]
fn every_carried_module_is_listed() {
    let root = manifest_dir().join("bench/runbench/runbench");
    let mut found: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the run program's directory reads") {
        let entry = entry.expect("its entries read");
        if !entry.file_type().expect("a file type").is_dir() {
            continue;
        }
        for file in std::fs::read_dir(entry.path()).expect("a shape's directory reads") {
            let file = file.expect("its entries read");
            // A size is the program's own and not part of the shape: the escape
            // phase runs a shorter list than escapebench, whose length is chosen
            // so the bracket's benefit lands inside it (2026-09-30).
            if file.file_name() == "size.kso" {
                continue;
            }
            let rel = file
                .path()
                .strip_prefix(manifest_dir())
                .expect("under the manifest")
                .to_string_lossy()
                .into_owned();
            found.push(rel);
        }
    }
    found.sort();
    let mut listed: Vec<String> = CARRIED.iter().map(|(copy, _)| copy.to_string()).collect();
    listed.extend(BORN_HERE.iter().map(|copy| copy.to_string()));
    listed.sort();
    assert_eq!(found, listed, "a module under bench/runbench/runbench is not pinned above");
}

/// The mix table in the driver's header is the program's own account of what
/// it weighs, and a repetition count edited without re-measuring makes that
/// account a fiction. This does not re-measure -- it asserts the table names
/// every constant the driver declares, so a phase cannot be resized or added
/// while the header goes on describing the old program.
#[test]
fn the_header_accounts_for_every_repetition_count() {
    let driver = read("bench/runbench/runbench/runbench.kso");
    let header: String =
        driver.lines().take_while(|l| l.starts_with('#')).collect::<Vec<_>>().join("\n");
    let counts: Vec<&str> = driver
        .lines()
        // Top-level only. A binding INSIDE a body is indented, and `tally`
        // has two of them -- the first run of this read `  bulk` as a phase
        // and went red naming it, which is the right shape of failure for the
        // wrong reason.
        .filter(|l| !l.starts_with('#') && !l.starts_with(char::is_whitespace))
        .filter_map(|l| l.split_once(" = "))
        .map(|(name, _)| name)
        .collect();
    assert!(!counts.is_empty(), "the driver declares no repetition counts");
    for name in &counts {
        let phase = name.split('_').next().expect("a name has a first word");
        assert!(
            header.contains(phase),
            "the header's mix table says nothing about `{name}`, so the shares \
             it states are not this program's"
        );
    }
}
