//! `bench/runbench_phases.txt` carries a share per phase, and the authority for
//! those shares is the table in the run program's own header, which was measured
//! phase by phase rather than chosen. Two copies of a measurement drift, and the
//! one that goes stale here is the one nobody reads: the map is consulted only
//! when history is reconstructed, so a repetition count could change, the header
//! be re-measured, and the map keep last month's mix without anything saying so.
//!
//! This replays one against the other. It does not check the shares are right --
//! the header's own table says how they were derived -- only that the file and
//! the program agree on what they are.
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

/// The header's table is comment lines of the shape
/// `#   decode  98 rounds of lib/json over large.json  1,045,772,072  34.54%`.
/// A row is a comment whose last field ends in `%` and whose first word after
/// the hash is the phase.
fn shares_in_the_header(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix('#') else { continue };
        let fields: Vec<&str> = rest.split_whitespace().collect();
        let [phase, .., last] = fields.as_slice() else { continue };
        let Some(share) = last.strip_suffix('%') else { continue };
        if share.parse::<f64>().is_err() {
            continue;
        }
        out.push((phase.to_string(), share.to_string()));
    }
    out
}

fn shares_in_the_map(map: &str) -> Vec<(String, String)> {
    map.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let fields: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(
                fields.len(),
                3,
                "a phase line is `<phase> <history counter> <share>`, found `{l}`"
            );
            (fields[0].to_string(), fields[2].to_string())
        })
        .collect()
}

#[test]
fn every_phase_share_is_the_one_the_run_program_measured() {
    let header = shares_in_the_header(&read("bench/runbench/runbench/runbench.kso"));
    let map = shares_in_the_map(&read("bench/runbench_phases.txt"));

    assert!(
        !header.is_empty(),
        "no share table found in the run program's header -- if its shape changed, \
         this spec has to follow it rather than be deleted"
    );
    assert_eq!(
        map, header,
        "bench/runbench_phases.txt and the run program's header disagree about the \
         mix. The header is the measurement; re-measure it when a repetition count \
         changes and move the map to match."
    );
}

/// The map is the reconstruction's whole link to history, so a phase that names
/// a counter no perf-history row ever carried would silently contribute nothing
/// and the reconstruction would quietly weigh less than it claims.
#[test]
fn every_phase_names_a_counter_the_instructions_golden_knows() {
    let map = read("bench/runbench_phases.txt");
    let golden = read("bench/instructions_golden.txt");
    let benches: Vec<&str> = golden
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    for line in map.lines().map(str::trim).filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (phase, counter) = (fields[0], fields[1]);
        let stem = counter.strip_suffix("_instructions").unwrap_or_else(|| {
            panic!("`{counter}` is not a `*_instructions` counter, so no row carries it")
        });
        // The counter's stem and the benchmark's name usually differ by the
        // `bench` suffix, and twice by more: the decode counter is jsonbench's
        // row, and the pend phase's counter has always been spelled `pending`.
        // Both spellings are the history's, so they are matched rather than
        // corrected.
        let aliases = [("decode", "jsonbench"), ("pending", "pendbench")];
        let known = benches.iter().any(|b| {
            *b == format!("{stem}bench")
                || *b == stem
                || aliases.iter().any(|(from, to)| stem == *from && b == to)
        });
        assert!(
            known,
            "phase `{phase}` names `{counter}`, and no benchmark in \
             bench/instructions_golden.txt answers to `{stem}`. Either the phase \
             maps to a different benchmark or that benchmark has been retired."
        );
    }
}
