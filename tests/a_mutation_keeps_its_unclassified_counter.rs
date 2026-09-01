//! The mutation that proves the trend gate's third state needs a counter with
//! no direction, and a classification sweep can take that away without
//! touching the mutation.
//!
//! `a_worsening_paid_for_by_an_unclassified_counter` raises one classified
//! counter and one unclassified one, and the gate has to refuse the first
//! while letting the second count toward neither side. It used `evac_allocs`
//! until 2026-09-01, when that counter was given a direction. The row went on
//! turning the gate red — two classified worsenings are a pure regression, so
//! any rule refuses them — while testing nothing it was written to test. A
//! ratchet row that passes for the wrong reason is worse than one that fails,
//! because nothing looks at it again.
//!
//! So this holds the two ends together: the counter the mutation raises in the
//! digest golden must be absent from both direction tables.

/// Every counter name the gate's `lower_*` and `higher_*` bindings list.
fn classified(gate: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in gate.lines() {
        let named = line.starts_with("lower_") || line.starts_with("higher_");
        if !named {
            continue;
        }
        let (Some(open), Some(shut)) = (line.find('['), line.find(']')) else { continue };
        for word in line[open + 1..shut].split('"') {
            let word = word.trim();
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
    }
    out
}

/// The counter the mutation rewrites in `bench/cost_golden_digest.txt`, read
/// out of its own `sed` line rather than named twice.
fn raised(mutation: &str) -> String {
    for line in mutation.lines() {
        let Some(rest) = line.strip_prefix("sed -i 's/^") else { continue };
        if !line.contains("cost_golden_digest.txt") {
            continue;
        }
        let Some(cut) = rest.find('=') else { continue };
        return rest[..cut].to_string();
    }
    panic!("the mutation no longer rewrites a named row of the digest golden");
}

#[test]
fn the_counter_the_mutation_leans_on_is_still_unclassified() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mutation = std::fs::read_to_string(
        root.join("scripts/ratchet/mutations/a_worsening_paid_for_by_an_unclassified_counter.sh"),
    )
    .expect("the mutation reads");
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");

    let named = classified(&gate);
    assert!(
        named.len() > 40,
        "the direction tables parsed to {} names, so this spec is reading nothing",
        named.len()
    );

    let counter = raised(&mutation);
    assert!(
        !named.iter().any(|n| *n == counter),
        "the mutation raises `{counter}` to stand for a counter with no direction, and a \
         direction table now names it. Both halves of the mutation are classified \
         worsenings, so it proves a pure-regression rule rather than the third state. \
         Point the mutation at a counter that stays unclassified on purpose."
    );
}
