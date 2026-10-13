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

/// Every counter name the gate's direction tables list: the `lower_*`
/// bindings, one to a line, and `higher`, which spans lines between
/// `higher = [` and the `]` that closes it.
fn classified(gate: &str) -> (Vec<String>, Vec<String>) {
    let mut lower = Vec::new();
    for line in gate.lines().filter(|l| l.starts_with("lower_")) {
        let (Some(open), Some(shut)) = (line.find('['), line.find(']')) else { continue };
        lower.extend(line[open + 1..shut].split('"').skip(1).step_by(2).map(str::to_string));
    }
    let body = gate.lines().skip_while(|l| *l != "higher = [").skip(1);
    let higher = body
        .take_while(|l| *l != "]")
        .filter(|l| !l.trim().starts_with('#'))
        .flat_map(|l| l.split('"').skip(1).step_by(2).map(str::to_string).collect::<Vec<_>>())
        .collect();
    (lower, higher)
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

    let (lower, higher) = classified(&gate);
    assert!(
        lower.len() > 40 && higher.len() > 10,
        "the direction tables parsed to {} and {} names, so this spec is reading nothing",
        lower.len(),
        higher.len()
    );
    let named: Vec<String> = lower.into_iter().chain(higher).collect();

    let counter = raised(&mutation);
    assert!(
        !named.contains(&counter),
        "the mutation raises `{counter}` to stand for a counter with no direction, and a \
         direction table now names it. Both halves of the mutation are classified \
         worsenings, so it proves a pure-regression rule rather than the third state. \
         Point the mutation at a counter that stays unclassified on purpose."
    );
}
