//! A row that pins two values has to say which binary read both of them.
//!
//! `scripts/gates/compile_ir_row.sh` defines the pair in its own words: "a key
//! and the TWO values one chip has been seen to read ON ONE BINARY". The
//! qualifier is the whole content of the claim, because the alternative
//! explanation for two numbers under one key is that they were counted on two
//! different compilers — which is what this vein exists to detect.
//!
//! Every pair written before 2026-09-05 cites its binary. The file carries
//! "ONE BINARY, ONE CHIP, TWO VALUES. Both shas are printed by the gate itself"
//! and, for each pair since, a line of the form "Same chip 0x19/0x11, same
//! binary sha 0e081d4c2c96: 41,845,704 ...". The convention was there; nothing
//! checked it.
//!
//! kanso#1252 then wrote `family0x19-model0x11 41379503 41380022` with a block
//! that cites no sha, because there was none to cite: 41379503 was measured on
//! the pre-#1251 binary and 41380022 on the post-#1251 one. #1251 changed the
//! compiler and re-sat only two of the four rows, saying so in its own commit
//! message; I read the leftover stale value as a second mode. The golden's bare
//! line follows the first row's first value, so the objective spent a day
//! reading a compile_instructions no chip had counted on this binary.
//!
//! So the citation is pinned rather than trusted: a paired row's two values
//! must both appear in one dated entry that also names a binary sha. It was
//! watched red against the defect itself on main rather than a mutation, and
//! the fix that greened it removed the last pair from the table — so the first
//! test now runs over no rows and passes vacuously. That is the resting state
//! this check is for: it has nothing to say until somebody writes a pair, and
//! then it asks for the sha before the pair can land.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const TABLE: &str = include_str!("../bench/compile_instructions_by_cpu.txt");

/// Rows are the non-comment, non-blank lines: key then one or two values.
fn paired_rows() -> Vec<(String, String, String)> {
    TABLE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let key = it.next()?.to_string();
            let one = it.next()?.to_string();
            let two = it.next()?.to_string();
            Some((key, one, two))
        })
        .collect()
}

/// The file is one long comment header and then the rows, so "a run of comment
/// lines" is the whole header and any check scoped to it is vacuous — the first
/// draft of this test used that and passed on the defect, because a sha from
/// July sat in the same run as a pair from September.
///
/// The file's real unit is the DATED ENTRY: every sitting is written as
/// `# 2026-09-05 ...` and runs until the next one. A pair's justification is
/// the entry that recorded it, so that is what has to carry the sha.
fn dated_entries() -> Vec<String> {
    let starts_entry = |l: &str| {
        let t = l.trim_start_matches(['#', ' ']);
        t.len() >= 10
            && t.as_bytes()[..10].iter().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    *b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            })
    };
    let mut out: Vec<String> = Vec::new();
    for line in TABLE.lines().filter(|l| l.trim_start().starts_with('#')) {
        if starts_entry(line) || out.is_empty() {
            out.push(String::new());
        }
        let last = out.last_mut().expect("an entry is open");
        last.push_str(line);
        last.push('\n');
    }
    out
}

/// The prose writes numbers with thousands separators and the rows do not.
fn grouped(n: &str) -> String {
    let digits: Vec<char> = n.chars().collect();
    let mut out = String::new();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

/// A sha, and not a value pretending to be one. The first draft asked only for
/// eight hex characters and PASSED on the defect, because "41379503" is eight
/// hex characters — the check answered yes using the very number it was meant
/// to demand a citation for. So the block must say the word, and the token has
/// to carry a letter no decimal count can.
fn names_a_sha(block: &str) -> bool {
    block.contains("sha")
        && block.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
            w.len() >= 8
                && w.chars().all(|c| c.is_ascii_hexdigit())
                && w.chars().any(|c| c.is_ascii_alphabetic())
        })
}

#[test]
fn every_paired_row_cites_the_one_binary_that_read_both_values() {
    for (key, one, two) in paired_rows() {
        let cited = dated_entries().into_iter().any(|b| {
            let has = |v: &str| b.contains(v) || b.contains(&grouped(v));
            has(&one) && has(&two) && names_a_sha(&b)
        });
        assert!(
            cited,
            "{key} pins {one} and {two} and no comment block in \
             bench/compile_instructions_by_cpu.txt names both values beside a \
             binary sha. A pair is TWO VALUES ONE CHIP READ ON ONE BINARY; \
             without the sha it is indistinguishable from a stale row carried \
             across a compiler change, which is what kanso#1252 wrote here"
        );
    }
}

/// And the bare golden follows the first row's first value. compile_ir_row.sh
/// checks this on every run, but only on a host that gets as far as measuring —
/// three of the compile gates refuse on a container, so a disagreement can sit
/// unread locally until CI. Two files and no measurement, so a spec can ask.
///
/// An EMPTY table is a state the design has, and this spec said "the table has
/// a row" when it was written a day ago. A change to the compiler's own bytes
/// removes every row at once — a value counted against the old binary is worse
/// than no value — and the table stays empty until CI draws a chip, refuses,
/// and prints the row to add. `compile_ir_row.sh` handles that case in its
/// first branch and says so. There is no first row for the golden to follow
/// then, so this asks nothing rather than failing with a sentence about a row
/// that is absent on purpose.
#[test]
fn the_golden_follows_the_first_row() {
    let Some(first) = TABLE.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))
    else {
        return;
    };
    let want = first.split_whitespace().nth(1).expect("the first row pins a value");
    let golden = std::fs::read_to_string(root().join("bench/compile_instructions_golden.txt"))
        .expect("the golden reads");
    let got = golden
        .lines()
        .find_map(|l| l.strip_prefix("compile_instructions="))
        .expect("the golden names compile_instructions");
    assert_eq!(
        got, want,
        "the golden welfare and the trend gate read says {got} and the table's \
         first row says {want}"
    );
}
