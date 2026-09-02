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

/// Every ratio exactly one, so the score is the weights and nothing else:
/// run speed and run memory saturate at 1/(1+2.0) and carry 0.30 each,
/// compile speed and compile memory at 1/(1+0.5) carrying 0.28 and 0.12.
/// 100 * (0.10 + 0.10 + 0.18667 + 0.08).
#[test]
fn every_counter_at_parity_scores_the_weights_alone() {
    assert_eq!(scored("parity", &[]), "welfare 46.67");
}

/// One of the eleven run-speed counters a thousand times better than its
/// baseline, the other ten at parity. Saturating each counter first bounds
/// what the runaway can contribute at one, so the term is
/// (10/3 + 1024/1026) / 11 * 0.30 and the score is 48.48.
///
/// Saturating the MEAN instead answers well above this on the same fixture:
/// the mean ratio is (10 + 1024)/11 = 94.0 and its satisfaction is 0.9791, so
/// one benchmark takes the run-speed term almost to its ceiling while ten
/// others sit at parity. That is the shape the ruling closed, and it is what
/// this number is here to catch.
///
/// THE COUNT IS WHAT MOVES THIS NUMBER, and it moves LATE. It read 49.16 over
/// eight counters until kanso#1221, four hours after kanso#1215 minted
/// `scan_instructions`, `escape_instructions` and `index_instructions` — a
/// minted counter enters the floor's baseline at the next ratchet rather than
/// at the merge that mints it, and this fixture takes its names from that
/// baseline. So a pull request that adds a run-speed counter leaves this spec
/// green and the NEXT `--set` turns it red. Recompute the fraction above from
/// the new count when that happens; the number is pinned rather than derived
/// on purpose, because a spec that recomputes what the tool computes is
/// asserting its own copy of the tool.
#[test]
fn one_counter_running_away_cannot_carry_its_term() {
    assert_eq!(scored("runaway", &[("wide_instructions", 1024)]), "welfare 48.48");
}
