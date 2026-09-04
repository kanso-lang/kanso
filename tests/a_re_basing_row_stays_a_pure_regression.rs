//! The re-basing row has to go red for ONE reason, and nothing in the ratchet
//! can tell it which.
//!
//! `a_re_basing_that_pays_for_a_regression` moves a golden and welfare's
//! baseline together — a definition change — beside a plain worsening, and the
//! trend gate must refuse the branch under the pure-regression rule: the
//! re-basing is not a win, so the worsening has nothing on the other side of
//! it. That is the whole claim of the row.
//!
//! The gate has a second refusal that fires on the same tree for a different
//! reason. A counter that moves without a sentence naming it and the value it
//! landed on is UNPRICED, and that exit is not the classification claim at
//! all. So the mutation writes a compiler-log paragraph naming both moves, and
//! the moment a value in that paragraph drifts from the value the `sed` line
//! writes, the row starts passing for the wrong reason — red either way, and
//! nobody looks at a green ratchet row again. The precedent is
//! `a_mutation_keeps_its_unclassified_counter`, where a classification sweep
//! quietly turned a third-state row into a pure-regression row.
//!
//! What this pins is the paragraph against the `sed` lines. What it does not
//! pin is the gate's output — the ratchet row runs the gate and requires red,
//! and this says the red is the one the row was written for.

/// Digits grouped in threes, the way the gate prints a counter's value.
fn commas(n: &str) -> String {
    let mut out = String::new();
    for (i, c) in n.chars().enumerate() {
        if i > 0 && (n.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Every value the mutation writes into a `bench/` golden, read out of the
/// `sed` replacement rather than named a second time here.
fn written(mutation: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = mutation.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("sed -i 's/") else { continue };
        // The file may sit on the sed line or on its continuation.
        let mut target = line.to_string();
        if rest.trim_end().ends_with('\\') || !line.contains("bench/") {
            if let Some(next) = lines.peek() {
                target.push_str(next);
            }
        }
        if !target.contains("bench/") || target.contains("welfare_floor.json") {
            continue;
        }
        let Some(shut) = rest.rfind("/'") else { continue };
        let Some(open) = rest[..shut].rfind('/') else { continue };
        let value: String = rest[open + 1..shut].chars().filter(|c| c.is_ascii_digit()).collect();
        if !value.is_empty() {
            out.push(value);
        }
    }
    out
}

#[test]
fn the_log_paragraph_states_every_value_the_mutation_writes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mutation = std::fs::read_to_string(
        root.join("scripts/ratchet/mutations/a_re_basing_that_pays_for_a_regression.sh"),
    )
    .expect("the mutation reads");

    let values = written(&mutation);
    assert_eq!(
        values.len(),
        2,
        "the mutation writes {} golden values, and this spec was written for the two \
         that make the row: the re-based counter and the plain worsening. Values read: {:?}",
        values.len(),
        values
    );

    let (_, entry) =
        mutation.split_once("<<'ENTRY'").expect("the mutation appends a compiler-log paragraph");
    for value in &values {
        let grouped = commas(value);
        assert!(
            entry.contains(&grouped),
            "the mutation writes {value} into a golden and its log paragraph does not \
             state {grouped}. The trend gate would then refuse the branch as UNPRICED — \
             a move with no sentence — rather than as the pure regression this row \
             exists to prove, and the row would be red for a reason nobody chose."
        );
    }
    for counter in ["compile_instructions", "work_jsonbench"] {
        assert!(
            entry.contains(counter),
            "the log paragraph does not name `{counter}`. The gate looks for the \
             counter's name as the gate spells it — `work_jsonbench` carries the \
             golden's prefix — and an unnamed move is UNPRICED."
        );
    }
    assert!(
        !mutation.contains("\"floor\":"),
        "the mutation adds a history entry to bench/welfare_floor.json, which is the \
         attribution that makes a pure regression PASS. The row would then be red only \
         if some other refusal fired."
    );
}
