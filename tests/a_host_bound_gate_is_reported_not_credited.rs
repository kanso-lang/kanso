//! Which gates the ratchet may not prove on a foreign runner, and why the list
//! cannot quietly grow.
//!
//! `bench/instructions_golden.txt` pins exact retired-instruction counts and
//! the runner pool is not one machine. On 2026-09-03 the cost-goldens job and
//! the ratchet job, on ONE commit, read `digestbench 81252316` and `81252330`
//! — fourteen apart, two runners, one image and one glibc. A gate that diffs
//! those counts is red on any runner but the golden's, whatever the mutation
//! did, and no branch diff fixes it.
//!
//! So `scripts/ratchet/ratchet.kso` carries `host_bound`: gates whose red the
//! baseline REPORTS and does not fail on, and whose rows the proving pass then
//! SKIPS. Reported and skipped is the opposite of the blindness kanso#1228 was
//! built to catch, where a red gate was silently counted as proof.
//!
//! The danger is the list, not the mechanism: an entry that excuses a gate
//! which is not silicon-bound turns a real failure into a note. So the list is
//! pinned to a property of the gates themselves — a gate is host-bound exactly
//! when it counts instructions under callgrind — rather than to anyone's
//! judgement. A new callgrind gate that is not declared turns this red; a
//! declared gate that runs no callgrind turns it red too.
//!
//! Reading the table is not running it. The behavioural half is the nightly
//! `prove`, which applies every mutation and reads every gate on a real runner.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const TABLE: &str = include_str!("../scripts/ratchet/ratchet.kso");

/// The gates the table declares host-bound. This walks the `host_bound` LIST,
/// not the `bound` bindings: the first draft read the bindings, so removing an
/// entry from the list left the check green — the binding was still there and
/// the list it was absent from was never opened. Watched red the second time.
fn declared() -> Vec<String> {
    let listed = operative()
        .into_iter()
        .find_map(|l| l.strip_prefix("host_bound = [")?.strip_suffix(']').map(str::to_string))
        .expect("the table declares a host_bound list");
    let mut named: Vec<String> =
        listed.split_whitespace().map(|e| resolved(&gate_binding_of(e))).collect();
    named.sort();
    named
}

fn operative() -> Vec<&'static str> {
    TABLE.lines().map(str::trim_start).filter(|l| !l.starts_with('#')).collect()
}

/// `bound_a = bound work_gate why_exact` — the binding that names the gate.
fn gate_binding_of(entry: &str) -> String {
    for line in operative() {
        if let Some(rest) = line.strip_prefix(&format!("{entry} = bound ")) {
            return rest.split_whitespace().next().expect("a bound names a gate").to_string();
        }
    }
    panic!("{entry} is in host_bound and nothing binds it");
}

/// `work_gate = "sh scripts/gates/instructions.sh"` — what a binding holds.
fn resolved(binding: &str) -> String {
    for line in operative() {
        if let Some((name, rest)) = line.split_once(" = ") {
            if name == binding {
                return rest.trim().trim_matches('"').to_string();
            }
        }
    }
    panic!("{binding} is named as a host-bound gate and nothing binds it");
}

/// Every gate script that actually counts instructions under callgrind. A
/// mention in a comment is not a run; kanso#1137 found four checks resting on
/// prose and this file was written the day a fifth and a sixth turned up.
fn counts_instructions() -> Vec<String> {
    let mut found = Vec::new();
    let dir = root().join("scripts/gates");
    for entry in std::fs::read_dir(&dir).expect("the gates directory reads") {
        let path = entry.expect("an entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a gate reads");
        let runs = text.lines().map(str::trim_start).any(|line| {
            !line.starts_with('#') && line.contains("valgrind") && line.contains("callgrind")
        });
        if runs {
            let name = path.file_name().expect("a name").to_string_lossy();
            found.push(format!("sh scripts/gates/{name}"));
        }
    }
    found.sort();
    found
}

#[test]
fn the_host_bound_gates_are_exactly_the_ones_that_count_instructions() {
    assert_eq!(
        declared(),
        counts_instructions(),
        "the ratchet's host_bound list and the gates that run callgrind have come apart. \
         A callgrind gate left off the list makes a runner mismatch fail the whole job; \
         a gate on the list that runs no callgrind turns one of its real failures into a note."
    );
}

#[test]
fn a_host_bound_gate_is_reported_and_its_rows_are_not_credited() {
    // The wording is the contract: the baseline says the row went unproven
    // rather than claiming it red-before-mutation, and the proving pass drops
    // it instead of applying a mutation to a gate that is already red.
    let code: Vec<&str> =
        TABLE.lines().map(str::trim_start).filter(|l| !l.starts_with('#')).collect();
    assert!(
        code.iter().any(|l| l.contains("UNPROVEN THIS RUN")),
        "a host-bound gate's red must be reported under its own heading, in code — \
         this file's own header names the phrase, and a comment is not a pin"
    );
    assert!(
        code.iter().any(|l| l.starts_with("fn kept_provable")),
        "the rows sharing a host-bound gate must be dropped from the proving pass"
    );
}
