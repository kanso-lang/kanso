//! The local sweep covers every counter golden, and the lazy tier beside them.
//!
//! `scripts/gates/all_counters.sh` exists because each counter vein is a gate
//! of its own, and following CLAUDE.md's "counters changed, regenerate every
//! vein in the same PR" meant remembering each filename and which golden it
//! writes. kanso#1249 remembered the .mem vein, the four code goldens and the
//! wasm blob and missed nine cost goldens; CI found them a round late.
//!
//! The sweep reads its veins from the gates themselves, so it cannot name one
//! gate and run another (kanso#2291). What can still go wrong is a cost golden
//! on disk that no gate reads, and a lazy tier the sweep never runs; the two
//! tests here watch those.

use std::collections::BTreeSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const SWEEP: &str = include_str!("../scripts/gates/all_counters.sh");

/// Every cost golden on disk is one the sweep says it covers, and every golden
/// it says it covers is on disk.
///
/// This used to read a count word out of CLAUDE.md, because the instructions
/// had said four cost goldens when there were ten and a branch that believed
/// them regenerated one of ten (kanso#1846). The count is written nowhere, so
/// the check runs the script the way a session would, with `--list`, and
/// compares what it prints with the directory.
#[test]
fn the_sweep_lists_every_cost_golden() {
    let out = std::process::Command::new("sh")
        .arg("scripts/gates/all_counters.sh")
        .arg("--list")
        .current_dir(root())
        .output()
        .expect("the sweep runs");
    assert!(out.status.success(), "all_counters.sh --list failed: {out:?}");
    let listed: BTreeSet<String> = String::from_utf8(out.stdout)
        .expect("the list is text")
        .lines()
        .map(|l| {
            let (_, golden) = l.split_once(' ').expect("a line is `vein golden`");
            golden.trim_start_matches("bench/").to_string()
        })
        .collect();
    let on_disk: BTreeSet<String> = std::fs::read_dir(root().join("bench"))
        .expect("the bench directory reads")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("cost_golden") && n.ends_with(".txt"))
        .collect();
    assert!(!on_disk.is_empty(), "there is at least one cost golden");
    let missing: Vec<_> = on_disk.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "all_counters.sh --list does not name {missing:?}, so the sweep never reads them"
    );
    let extra: Vec<_> = listed.difference(&on_disk).collect();
    assert!(extra.is_empty(), "all_counters.sh --list names {extra:?}, which is not on disk");
}

/// The sweep runs something that reads the lazy tier.
///
/// `tests/golden/mem/*.mem` is the vein CLAUDE.md names FIRST in the list a
/// counter change must regenerate, and it is read by `tests/golden.rs` rather
/// than by a `*_counters.sh` gate, so the sweep's loop over the gates cannot
/// see it, and until 2026-09-06 neither could the sweep. The compile
/// sweep had the same hole twice in the same week: a golden read only by a cargo
/// test, invisible to both the script's list and the spec replaying it.
///
/// This asserts the property rather than the line: the sweep's own text, plus
/// the source of every `--test` target it runs, has to mention the vein
/// somewhere.
#[test]
fn the_sweep_reads_the_lazy_tier() {
    let mut body = SWEEP.to_string();
    for rest in SWEEP.split("--test ").skip(1) {
        let target = rest.split_whitespace().next().unwrap_or_default();
        let path = root().join(format!("tests/{target}.rs"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            body.push_str(&text);
        }
    }
    assert!(
        body.contains("golden/mem"),
        "scripts/gates/all_counters.sh runs nothing that reads tests/golden/mem — \
         the lazy tier is the first vein CLAUDE.md tells a counter change to \
         regenerate, and a vein with no reader in the sweep moves and says so \
         only in CI"
    );
}
