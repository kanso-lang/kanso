//! The run program the objective weighs carries a copy of each stress shape,
//! and the benchmark it came from survives as a diagnostic golden. Two copies
//! of one workload drift, and the drift is silent: `runbench` would go on
//! reporting a shape that `escapebench` had stopped exercising, and the
//! objective would be weighing something nobody is watching.
//!
//! So the copies are byte-identical to their sources, and this says so.
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(the copy under bench/runbench, the benchmark module it came from)`.
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

fn read(rel: &str) -> String {
    let path: PathBuf = manifest_dir().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

#[test]
fn the_run_program_carries_the_shapes_unchanged() {
    for (copy, source) in CARRIED {
        assert_eq!(
            read(copy),
            read(source),
            "{copy} has drifted from {source}. The run program's shape and the \
             benchmark that diagnoses it have to be the same program, or the \
             objective weighs one workload and the golden watches another."
        );
    }
    assert_eq!(CARRIED.len(), 5, "a shape joined or left the run program without a line here");
}

/// Every directory under `bench/runbench/runbench` that is not the driver
/// holds exactly one carried module, and that module is named above. A copy
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
