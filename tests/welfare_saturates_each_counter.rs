//! The curve reaches every counter, and no single one can carry a term.
//!
//! RULED (Clay, 2026-08-29): "you do the log or whatever function it is on
//! each term before averaging." Welfare used to average its counters' ratios
//! and saturate the mean, which let one runaway counter own a dimension — a
//! ratio enters a mean linearly and unbounded, so a benchmark 138 times
//! better than its baseline contributed 138 where one twice as good
//! contributed 2. The run-speed term became 68% the pretty-printing
//! benchmark while decode and encode, the rows the front page makes claims
//! about, were 1% between them.
//!
//! These pin the arithmetic at two points, and both numbers are properties
//! of the weights rather than of the compiler: the fixtures set every
//! counter's baseline to its own current value, so every ratio is exactly
//! one whatever the goldens say today, and the scores below do not move when
//! a benchmark gets faster.

use std::process::Command;

/// Score a staged `bench/` whose baseline has been rewritten so every
/// counter sits at `now`, except the ones `doctored` names, whose baselines
/// are multiplied by the factor given.
///
/// The now-values come from welfare's own report rather than from a second
/// reader of the goldens: a spec that re-parses what the tool parses is
/// asserting its own copy of the tool, and the copy is what goes stale.
///
/// `key` names the staging directory. Six separate tests in this repository
/// have staged into one path and torn each other down mid-run, so the key is
/// per-test and the tests pass distinct ones.
fn scored(key: &str, doctored: &[(&str, u128)]) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-welfare-sat-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bench")).expect("a staging directory");
    for entry in std::fs::read_dir(root.join("bench")).expect("bench is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_file() {
            let landing = stage.join("bench").join(path.file_name().expect("named"));
            std::fs::copy(&path, &landing).expect("the golden copies");
        }
    }

    // First run: the real baseline, read only for the `base -> now` rows.
    let told = run(root, &stage);
    let held = stage.join("bench/welfare_floor.json");
    let text = std::fs::read_to_string(&held).expect("the floor reads");

    let mut floor = String::from("{\"baseline\":{");
    let mut first = true;
    for name in names(&text) {
        let now = now_of(&told, &name).unwrap_or_else(|| {
            held_value(&text, &name).expect("a counter with no row and no baseline")
        });
        let factor = doctored.iter().find(|(n, _)| *n == name).map(|(_, f)| *f).unwrap_or(1);
        if !first {
            floor.push(',');
        }
        first = false;
        floor.push_str(&format!("\"{name}\":{}", now * factor));
    }
    // A floor of zero so the tool reports rather than refuses; what is under
    // test is the arithmetic, and the refusal has its own spec.
    //
    // The history carries one entry rather than none. The banner reads the
    // last ratchet's reason, and an empty list makes that read fail, so the
    // tool prints nothing and the spec fails on parsing rather than on the
    // number it came to check.
    floor.push_str("},\"floor\":0.0,\"history\":[{\"floor\":0.0,\"why\":\"a staged fixture\"}]}");
    std::fs::write(&held, floor).expect("the floor writes");

    let said = run(root, &stage);
    let _ = std::fs::remove_dir_all(&stage);
    said.lines()
        .next()
        .expect("welfare says something")
        .split("   ")
        .next()
        .expect("a score")
        .trim()
        .to_string()
}

fn run(root: &std::path::Path, stage: &std::path::Path) -> String {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .current_dir(stage)
        .output()
        .expect("welfare runs");
    String::from_utf8_lossy(&done.stdout).into_owned()
}

/// Every counter the floor's baseline names, in the order it names them.
fn names(floor: &str) -> Vec<String> {
    let open = floor.find("\"baseline\":{").expect("a baseline") + "\"baseline\":{".len();
    let shut = open + floor[open..].find('}').expect("the baseline ends");
    floor[open..shut]
        .split(',')
        .map(|pair| pair.split(':').next().expect("a key").trim_matches('"').to_string())
        .collect()
}

fn held_value(floor: &str, name: &str) -> Option<u128> {
    let at = floor.find(&format!("\"{name}\":"))? + name.len() + 3;
    let rest = &floor[at..];
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

/// `  decode_instructions    3,266,896,510 -> 2,858,845,253    +14.3%`
fn now_of(told: &str, name: &str) -> Option<u128> {
    let line = told.lines().find(|l| l.trim_start().starts_with(name))?;
    let after = line.split("->").nth(1)?;
    after.split_whitespace().next()?.replace(',', "").parse().ok()
}

/// Every ratio exactly one, so the score is the weights and nothing else.
/// The three run terms saturate at 1/(1+2.0) and carry 0.15, 0.15 and 0.26;
/// the two compile terms at 1/(1+0.5) carry 0.32 and 0.12.
/// 100 * (0.05 + 0.05 + 0.08667 + 0.21333 + 0.08).
///
/// It read 46.67 under the weights before Clay's 2026-09-02 gavel, when run
/// speed was one term of 0.30, run memory 0.30 and compile speed 0.28.
#[test]
fn every_counter_at_parity_scores_the_weights_alone() {
    assert_eq!(scored("parity", &[]), "welfare 48.00");
}

/// One of the ten GUARD counters a thousand times better than its baseline,
/// the other nine and both advertised rows at parity. Saturating each
/// counter first bounds what the runaway can contribute at one, so the guard
/// term is (9/3 + 1024/1026) / 10 * 0.15 and the score is 49.00.
///
/// Saturating the MEAN instead answers well above this on the same fixture,
/// which is the shape the 2026-08-29 ruling closed and what this number is
/// here to catch: one benchmark would take its term almost to the ceiling
/// while every other sat at parity.
///
/// THE COUNT IS WHAT MOVES THIS NUMBER, and it usually moves LATE. It read
/// 49.16 over eight counters until kanso#1221, four hours after kanso#1215
/// minted `scan_instructions`, `escape_instructions` and `index_instructions`
/// — a minted counter enters the floor's baseline at the next ratchet rather
/// than at the merge that mints it, and this fixture takes its names from that
/// baseline. So a pull request that adds a run-speed counter usually leaves
/// this spec green and the NEXT `--set` turns it red. Recompute the fraction
/// above from the new count when that happens; the number is pinned rather
/// than derived on purpose, because a spec that recomputes what the tool
/// computes is asserting its own copy of the tool.
///
/// 2026-09-04 is the case where it did NOT move late: `read_instructions`
/// joined the guards and the same pull request ran `--set`, so the mint and
/// the ratchet were one change and this spec went red inside it. Nine guards
/// became ten and 49.11 became 49.00. Nothing about the rule changed — the
/// delay was never a property of the spec, only of the usual order — and the
/// two neighbours held: parity stays 48.00 because it does not depend on the
/// count, and the advertised runaway stays 52.99 because that half still has
/// two rows.
#[test]
fn one_counter_running_away_cannot_carry_its_term() {
    assert_eq!(scored("runaway", &[("wide_instructions", 1024)]), "welfare 49.00");
}

/// THE HALVES ARE NOT INTERCHANGEABLE. The same thousandfold win is worth
/// more on an advertised row than on a guard, and that is the whole content
/// of Clay's 2026-09-02 split: half the run-speed weight belongs to decode
/// and encode — the rows the front page makes claims about — and half to the
/// nine shape guards between them.
///
/// Before the split all eleven counters sat in one term and the two fixtures
/// below scored the SAME number, 48.48, because a counter was a counter. A
/// shape win scored as if a real workload had got faster. These two numbers
/// differing is the property; their order is the direction.
///
/// The gap is large because the halves are unequal in count as well as in
/// kind: a runaway is one of two advertised rows and one of ten guards, so
/// it moves its half by 1/2 rather than by 1/10. The gap widens every time a
/// guard is added, which is the right direction — a corpus with more shapes in
/// it makes any one shape worth less — and it is why this pair is asserted as
/// an ORDER as well as two numbers.
#[test]
fn a_win_on_an_advertised_row_outscores_the_same_win_on_a_guard() {
    let advertised = scored("advertised", &[("decode_instructions", 1024)]);
    let guard = scored("guard", &[("wide_instructions", 1024)]);
    assert_eq!(advertised, "welfare 52.99", "an advertised runaway");
    assert_eq!(guard, "welfare 49.00", "the same runaway on a guard");
    assert!(
        advertised > guard,
        "the advertised half is worth more per counter: {advertised} against {guard}"
    );
}
