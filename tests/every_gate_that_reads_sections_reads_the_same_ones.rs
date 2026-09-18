//! A gate that prints a binary's section sizes prints the SAME sections as
//! every other one, and `.rodata` is among them.
//!
//! WHY `.rodata` AND NOT JUST text/data/bss. The section line exists so a
//! reader who sees a row move can tell a code change from a data change
//! without rebuilding anything. `compile_instructions.sh`'s own calibration
//! table is the argument: across seven binaries differing only in code or
//! data nothing reaches, `+64 KiB .bss` and `+64 KiB .rodata` leave the
//! anchored frame identical to the instruction where `+400 dead fns` moves
//! it, so constant data is one of the two cases where the split is total.
//! The gates printed `.bss` and not `.rodata`, which is half of that pair.
//!
//! WHY A SPEC AND NOT A SWEEP. STATUS.md's normalization row asked for this
//! on the interp gate alone, on the belief that `compile_instructions.sh`
//! already printed `.rodata`. It did not: the only `.rodata` in any gate was
//! in that calibration table, inside a comment. A list of gates written down
//! by hand goes stale the same way -- see the count in CLAUDE.md that was
//! wrong for as long as it was written down -- so this reads the gates off
//! disk.

use std::fs;

const SECTIONS: &str = "text|rodata|data|bss";

#[test]
fn every_gate_that_reads_sections_reads_the_same_ones() {
    let mut printing = Vec::new();
    let mut wrong = Vec::new();

    for entry in fs::read_dir("scripts/gates").expect("scripts/gates") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let body = fs::read_to_string(&path).expect("gate body");
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        for line in body.lines() {
            // The section line is an awk program anchored on a `.`-prefixed
            // section name. Comments are skipped: the calibration table that
            // misled STATUS.md lives in one.
            let code = line.trim_start();
            if code.starts_with('#') {
                continue;
            }
            if !code.contains(r"/^\.(") {
                continue;
            }
            printing.push((name.clone(), code.to_string()));
            if !code.contains(SECTIONS) {
                wrong.push((name.clone(), code.to_string()));
            }
        }
    }

    // Pinned, not bounded. A floor would stay green through exactly the
    // change this is here to catch -- a gate that stops printing its
    // sections. Nine today, across six files; move the number when a gate
    // is added or removed and say so.
    assert_eq!(
        printing.len(),
        9,
        "the gates hold {} section lines, not nine:\n{}",
        printing.len(),
        printing
            .iter()
            .map(|(f, l)| format!("  {f}\n    {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        wrong.is_empty(),
        "these section lines do not name `{SECTIONS}`:\n{}",
        wrong
            .iter()
            .map(|(f, l)| format!("  {f}\n    {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
