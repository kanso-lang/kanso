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

/// COMPILE SPEED'S two counters, one of them a thousand times better than its
/// baseline and the other at parity. Saturating each counter first bounds what
/// the runaway can contribute at one, so the term is (1024/1024.5 + 2/3) / 2 *
/// 0.32 and the score is 53.33. Saturating the MEAN instead answers 58.63 on
/// the same fixture, which is the shape the 2026-08-29 ruling closed.
///
/// It used to be asserted on the run side, where eleven counters shared one
/// term and the number moved every time a benchmark joined. Clay's gavel of
/// 2026-09-06 put the run side on one consolidated program, so each run term
/// has exactly one counter and a runaway there IS the term -- there is nothing
/// left for the rule to bound. Compile speed still has two, and the arithmetic
/// under test never depended on which term carried it.
///
/// The count is what moves this number, and it is pinned rather than derived
/// on purpose: a spec that recomputes what the tool computes is asserting its
/// own copy of the tool. If a counter joins compile speed, recompute the
/// fraction above from the new count.
#[test]
fn one_counter_running_away_cannot_carry_its_term() {
    assert_eq!(scored("runaway", &[("compile_instructions", 1024)]), "welfare 53.33");
}

/// WEIGHT SAYS HOW MUCH A DIMENSION MATTERS; SATIATION SAYS HOW LONG IT KEEPS
/// MATTERING. The same doubling is worth more on the run side than on the
/// compile side, and the gap is the whole content of the two satiations: run
/// terms satiate at 2.0, where a doubling moves satisfaction from 1/3 to 1/2,
/// and compile terms at 0.5, where it moves from 2/3 to 4/5.
///
/// This replaces the advertised/guards pair, which asserted that half the
/// run-speed weight belonged to decode and encode and half to the shape
/// guards between them. The 2026-09-06 gavel retired that split: the shapes
/// are phases inside one program now, at measured proportions, so there are
/// no halves to compare. The asymmetry this asserts is the one the model
/// still has, and it is a property of the weights rather than of the compiler.
///
/// 53.00 against 50.13: a doubling buys the run side 0.05 of the index and the
/// compile side 0.0213, and compile speed carries the LARGER weight of the two.
#[test]
fn a_doubling_is_worth_more_on_the_run_side_than_the_compile_side() {
    let run = scored("doubled-run", &[("run_instructions", 2)]);
    let compile = scored("doubled-compile", &[("compile_instructions", 2)]);
    assert_eq!(run, "welfare 53.00", "a doubling of the run program's work");
    assert_eq!(compile, "welfare 50.13", "the same doubling of what compiling costs");
    assert!(
        run > compile,
        "the run side satiates later, so it keeps paying: {run} against {compile}"
    );
}
