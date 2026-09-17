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

/// Every ratio exactly one, so each side is its weights and nothing else, and
/// the meta is the saturating combination of the two.
///
/// PRODUCTION: run speed and run memory saturate at 1/(1+2.0) carrying 0.45 and
/// 0.40, release build at 1/(1+0.5) carrying 0.15.
/// 0.15 + 0.13333 + 0.10 = 0.38333.
///
/// DEVELOPMENT: compile speed, compile memory, dev build and start-up all
/// saturate at 1/(1+0.5) and carry 0.30, 0.08, 0.22 and 0.25; the two
/// interpreter terms at 1/(1+1.0) carry 0.11 and 0.04.
/// 0.85 * 0.66667 + 0.15 * 0.5 = 0.64167.
///
/// META: f(x) = x/(x+1), and the sum is divided by f(1) = 0.5 to put the
/// ceiling back at a hundred.
/// 100 * (0.70 * 0.27711 + 0.30 * 0.39086) / 0.5 = 62.25.
///
/// It read 48.00 between the 2026-09-06 consolidation and the 2026-09-16
/// split, and 46.67 before Clay's 2026-09-02 gavel.
#[test]
fn every_counter_at_parity_scores_the_weights_alone() {
    assert_eq!(scored("parity", &[]), "welfare 62.25");
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
    assert_eq!(scored("runaway", &[("compile_instructions", 1024)]), "welfare 63.33");
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
/// SINCE THE 2026-09-16 SPLIT THE GAP HAS TWO CAUSES, and saying it is all
/// satiation would be wrong. Run speed satiates later AND sits on the
/// production side, which the meta weighs 0.70 against development's 0.30. The
/// assertion below still holds and is still worth pinning, but a reader who
/// wants satiation ALONE has to compare two terms on the same side -- compile
/// speed at 0.5 against interpreter speed at 1.0 -- because a cross-side
/// comparison carries the meta's weights with it.
///
/// 67.45 against 62.69. Doubling run instructions takes production from
/// 0.38333 to 0.45833; doubling compile instructions takes development from
/// 0.64167 to 0.66167, since compile speed averages its two counters and only
/// one of them moved.
#[test]
fn a_doubling_is_worth_more_on_the_run_side_than_the_compile_side() {
    let run = scored("doubled-run", &[("run_instructions", 2)]);
    let compile = scored("doubled-compile", &[("compile_instructions", 2)]);
    assert_eq!(run, "welfare 67.45", "a doubling of the run program's work");
    assert_eq!(compile, "welfare 62.69", "the same doubling of what compiling costs");
    assert!(
        run > compile,
        "the run side satiates later, so it keeps paying: {run} against {compile}"
    );
}
