//! A compile vein whose row is in no direction table is invisible to the gate
//! that watches it, the same way digestbench was invisible to the work vein.
//!
//! `tests/every_benchmark_in_the_work_vein_has_a_direction.rs` asserts this for
//! `bench/instructions_golden.txt` and stops there, so it could not see that
//! BOTH newer compile veins had joined the tree without joining the table:
//! `entry_instructions` since kanso#1330 and `library_instructions` since
//! kanso#1337. Running the gate on kanso#1338's own diff is what surfaced it —
//! a fall of 1,282,921 printed as UNCLASSIFIED drift, and a rise of the same
//! size would have printed the same way and counted toward neither side of the
//! pure-regression rule.
//!
//! The invariant is the one the work-vein spec argues for its own file. Each of
//! these goldens holds retired instructions for one compile, and fewer is
//! better in all of them, so every row has a direction. Nothing here is a
//! presence counter whose direction means nothing alone.
//!
//! DERIVED FROM DISK, NOT FROM A LIST. The veins are every
//! `bench/*instructions_golden.txt` other than the work vein's, so a fourth
//! compile path opening a vein under a fourth name is covered on the day the
//! file lands. A hardcoded list is the shape that went stale here twice.

/// The counter names the trend gate's `lower_*` bindings list. Read out of the
/// gate's source because the gate is written in kanso and there is no other way
/// in; a restructuring of those bindings breaks this loudly.
fn classified_lower(gate: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in gate.lines() {
        let Some(rest) = line.strip_prefix("lower_") else { continue };
        let Some(open) = rest.find('[') else { continue };
        let Some(shut) = rest.find(']') else { continue };
        for word in rest[open + 1..shut].split('"') {
            let word = word.trim();
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
    }
    out
}

/// Every `bench/*instructions_golden.txt` except the work vein's, whose rows
/// are benchmark names rather than `counter=value` and which has its own spec.
fn compile_veins(bench: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(bench).expect("bench/ reads") {
        let path = entry.expect("a bench/ entry reads").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if name.ends_with("instructions_golden.txt") && name != "instructions_golden.txt" {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[test]
fn every_row_of_every_compile_vein_is_named_by_a_direction_table() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");
    let named = classified_lower(&gate);
    assert!(
        named.len() > 20,
        "the lower_* bindings parsed to {} names, which means the shape moved \
         and this spec is reading nothing",
        named.len()
    );

    let veins = compile_veins(&root.join("bench"));
    assert!(
        veins.len() >= 3,
        "found {} compile instruction veins in bench/, and there have been at \
         least three since kanso#1337, so this spec is reading nothing",
        veins.len()
    );

    let mut adrift = Vec::new();
    let mut rows = 0;
    for vein in &veins {
        let text = std::fs::read_to_string(vein).expect("a compile vein reads");
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let name = line.split('=').next().expect("a row names a counter").trim();
            rows += 1;
            if !named.iter().any(|n| n == name) {
                adrift.push(name.to_string());
            }
        }
    }
    assert_eq!(
        rows,
        veins.len(),
        "one row per compile vein is the shape these goldens have; {rows} rows \
         over {} files means the shape moved",
        veins.len()
    );
    assert!(
        adrift.is_empty(),
        "these counters are in a bench/*instructions_golden.txt and in no \
         direction table, so a rise in any of them reads as UNCLASSIFIED drift \
         and the trend gate exits green: {adrift:?}"
    );
}
