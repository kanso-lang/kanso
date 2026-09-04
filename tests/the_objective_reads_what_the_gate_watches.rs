//! `bench/objective_sources.txt` says which of the trend gate's golden rows
//! each of welfare's counters is made of, and nothing else in the tree knows
//! that. This replays it.
//!
//! The link cannot be derived. welfare renames on the way in — `work_jsonbench`
//! is the gate's spelling of the row welfare calls `decode_instructions` — and
//! for the memory terms `peak_of` sums the arena, held and perm peaks, so a
//! `*_peak_bytes` counter is a derived quantity with no row holding its value.
//! Joining on values was measured before the file was written and cannot work:
//! of the 27 counters, six match no row at all and two match several (2,097,152
//! is `arena_peak_bytes` in four goldens at once).
//!
//! So it is written down, and a written-down link is one a rename silently
//! breaks. This sums each counter's keys out of the goldens and asserts the
//! total is what `welfare --counters` prints — for every counter welfare has,
//! and no counter it does not. A benchmark joining the objective, a pool added
//! to `peak_of`, a golden's prefix changing, a row renamed on either side: each
//! turns this red rather than quietly unlinking a term from the trend gate's
//! re-basing check.
//!
//! WHAT IT CANNOT SEE, stated rather than left to be discovered: the check is
//! on the totals, so a pool that reads nought contributes nothing and dropping
//! it from a counter's rows costs the sum nothing. Every `held_peak_bytes` and
//! `perm_peak_bytes` in the goldens is nought today, so those twelve rows are
//! unfalsifiable right now. The direction that matters is covered — a pool
//! ADDED to `peak_of` and not to the file is nonzero by the time anyone cares,
//! and the sums disagree the moment it is — and a dropped zero row starts
//! failing the day the pool carries a byte.

use std::collections::HashMap;
use std::path::Path;

/// Every `[["bench/…" "prefix_"]]` binding in the gate, so the prefixes come
/// from the gate itself rather than a second copy here.
fn watched(gate: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in gate.lines() {
        let Some(open) = line.find("[[\"bench/") else { continue };
        let rest = &line[open + 3..];
        let mut parts = rest.split('"').filter(|p| !p.contains("[") && !p.contains("]"));
        let (Some(file), Some(_gap), Some(prefix)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        out.push((file.to_string(), prefix.to_string()));
    }
    out
}

/// Every numeric row of those goldens, keyed the way the gate keys it. Rows are
/// summed across samples, which is what the gate's own `totals` does.
fn rows(root: &Path, gate: &str) -> HashMap<String, i128> {
    let mut out: HashMap<String, i128> = HashMap::new();
    for (file, prefix) in watched(gate) {
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else { continue };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (name, value) = match line.split_once('=') {
                Some(pair) => pair,
                None => {
                    let mut words = line.split_whitespace();
                    match (words.next(), words.next(), words.next()) {
                        (Some(n), Some(v), None) => (n, v),
                        _ => continue,
                    }
                }
            };
            let Ok(n) = value.trim().parse::<i128>() else { continue };
            *out.entry(format!("{prefix}{}", name.trim())).or_default() += n;
        }
    }
    out
}

#[test]
fn every_objective_counter_sums_from_the_rows_the_gate_watches() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");
    let link = std::fs::read_to_string(root.join("bench/objective_sources.txt"))
        .expect("the link file reads");

    let mut made_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for line in link.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let (Some(counter), Some(key), None) = (words.next(), words.next(), words.next()) else {
            panic!("`{line}` is not `<objective counter> <gate key>`");
        };
        made_of.entry(counter).or_default().push(key);
    }

    let seen = rows(root, &gate);
    assert!(
        seen.len() > 200,
        "the goldens parsed to {} rows, so this spec is reading nothing",
        seen.len()
    );

    let said = std::process::Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "scripts/welfare", "--", "--counters"])
        .current_dir(root)
        .output()
        .expect("welfare runs");
    assert!(said.status.success(), "welfare --counters failed");
    let printed = String::from_utf8(said.stdout).expect("utf-8");

    let mut counted = 0;
    for line in printed.lines() {
        let Some((counter, value)) = line.split_once('=') else { continue };
        counted += 1;
        let want: i128 = value.trim().parse().expect("a counter is a number");
        let Some(keys) = made_of.get(counter) else {
            panic!(
                "welfare scores `{counter}` and bench/objective_sources.txt does not name it. \
                 The trend gate cannot tell a re-basing of that counter from a win until it \
                 does: add the golden rows it is made of."
            )
        };
        let mut got: i128 = 0;
        for key in keys {
            let Some(n) = seen.get(*key) else {
                panic!(
                    "bench/objective_sources.txt sends `{counter}` to `{key}`, and no golden \
                     the trend gate watches has that row. Either the row was renamed or its \
                     golden is missing from the gate's list."
                )
            };
            got += n;
        }
        assert_eq!(
            got,
            want,
            "`{counter}` reads {want} from welfare and {got} from {}. The link is wrong or \
             a pool joined the sum.",
            keys.join(" + ")
        );
    }
    assert!(counted > 0, "welfare --counters printed nothing");

    let extra: Vec<&str> = made_of
        .keys()
        .filter(|c| !printed.lines().any(|l| l.starts_with(&format!("{c}="))))
        .copied()
        .collect();
    assert!(
        extra.is_empty(),
        "bench/objective_sources.txt names {extra:?}, which welfare does not score. A link to \
         nothing is how the file stops being read as authoritative."
    );
}
