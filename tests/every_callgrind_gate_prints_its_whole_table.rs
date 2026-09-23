//! Every gate that profiles a fixed file prints that profile's whole function
//! table, uncapped, on every run.
//!
//! kanso#1558 gave `compile_instructions.sh` this print after three
//! instructions could not be located from two job logs: the gate carried
//! `--threshold=90 | head -40`, which is forty rows of the hundred and
//! twenty-five that threshold has and fifteen functions once the headers are
//! dropped. Every one of the fifteen was equal across the two jobs, so the
//! comparison ended there. The whole table is about 1,115 rows and reaches
//! functions that retire a single instruction.
//!
//! That fix landed for one gate. Six others had the identical `--threshold=90
//! | head -40`, and one of them is the gate STATUS.md's standing row is about:
//! `interp_instructions` reads six apart across two CI jobs on one commit,
//! each stable across the gate's own second reading, and the instrument for
//! saying which frames moved did not exist. This spec is why the fix cannot
//! land for one gate again.
//!
//! Four properties, and the last two are the ones that are easy to get wrong:
//!
//!   * the annotate asks for the whole table (`--threshold=100`), exclusive
//!     rather than `--inclusive=yes`, which is a different reading;
//!   * nothing truncates it;
//!   * it runs BEFORE the row is compared, because a comparison of two jobs
//!     needs the agreeing side and the agreeing side is the one that never
//!     takes a failure path;
//!   * the table is STASHED, so the end-of-job step can re-emit it packed.
//!
//! That fourth one is kanso#1565's finding. Printing the table is not the
//! same as being able to read it: a log API returns the tail of a job and
//! caps it at 5,000 lines however long a tail is asked for, and the `cost
//! goldens` job runs 20,020 lines. Two tables of the twenty came back. A gate
//! that prints inline and never stashes is invisible to the packed step and
//! is back in that position, which is why this is a property and not a habit.
//!
//! The gates reach the annotate through `function_table.sh` rather than each
//! carrying their own copy, so every property below reads the gate with the
//! helper substituted where it is called. A gate is free to keep an inline
//! annotate; what it may not do is print a table nothing can read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn gates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates")
}

/// Every gate script, read off disk rather than listed here.
fn gate_scripts() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(gates_dir()).expect("scripts/gates reads") {
        let path = entry.expect("the entry reads").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let name =
            path.file_name().and_then(|n| n.to_str()).expect("the name is utf-8").to_string();
        out.insert(name, std::fs::read_to_string(&path).expect("the script reads"));
    }
    out
}

/// The profile paths a script writes, as written.
fn profiles_written(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (_, rest) in
        body.match_indices("--callgrind-out-file=").map(|(i, m)| (i, &body[i + m.len()..]))
    {
        let path: String = rest.chars().take_while(|c| !c.is_whitespace() && *c != '\\').collect();
        let path = path.trim_matches('"').to_string();
        if !path.is_empty() && !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// A gate is exempt only for a reason written down here, and the list is
/// asserted to be exactly these three — another gate cannot join it quietly.
///
/// `codegen_instructions.sh` names its profiles `/tmp/cg.codegen.$tier.%p`:
/// one file per process across the clang driver, the convention probe,
/// `clang -cc1` and ld, at 6.8 billion instructions on the release tier.
/// It also already diffs its own two readings per frame and prints what moved,
/// which the single-profile gates do not do. Printing every process's whole
/// table on every run is a different change from this one and wants its own
/// measurement of what it costs the job log.
///
/// `path_independence.sh` writes `/tmp/cg.pi` and never reads it: the count
/// comes off valgrind's stderr, and the same path is overwritten by every
/// measurement in its loop, so there is no profile left to print by the time
/// the gate has a verdict. Making that gate name which frames moved means
/// keeping both profiles first.
///
/// `address_blind.sh` profiles a probe of about thirty-five instructions,
/// twice, to prove the memcmp every counted run preloads costs the same at
/// two page offsets. Its verdict is the pair of counts, and a refusal prints
/// both; there is no compiler frame in that profile to locate.
const EXEMPT: [&str; 3] = ["address_blind.sh", "codegen_instructions.sh", "path_independence.sh"];

/// The scripts this spec governs: every gate that writes a callgrind profile
/// and is not exempt above.
///
/// This filter began as "a path with no `%p` and no shell expansion", which
/// read the shape of the write rather than the shape of the profile, and it
/// dropped two of the six gates the change covers: `emit_instructions.sh`
/// writes `"$out"` and `instructions.sh` writes `/tmp/cg.$b` in a loop over
/// fourteen named benchmarks, and each of those is one profile per run with a
/// name the script knows. What actually makes codegen different is many
/// processes under one reading, which is a reason and not a spelling, so it is
/// named in EXEMPT rather than filtered for here.
fn governed() -> BTreeMap<String, String> {
    gate_scripts()
        .into_iter()
        .filter(|(name, body)| {
            !EXEMPT.contains(&name.as_str()) && !profiles_written(body).is_empty()
        })
        .collect()
}

/// The shared script that annotates one profile, prints it collapsed, and
/// stashes a copy for the packed end-of-job step.
const HELPER: &str = "function_table.sh";

/// The directory the packed step reads. A table that does not land here is
/// printed and unreadable, which is the position kanso#1565 was in.
const STASH: &str = "KANSO_FUNCTION_TABLES";

/// A gate with the helper's body substituted where it calls it, so a gate that
/// delegates and a gate that inlines read the same to every test here. The
/// call sits exactly where the print used to, so relative order is preserved
/// and the before-the-comparison test still means what it meant.
fn resolved(body: &str) -> String {
    if !body.contains(HELPER) {
        return body.to_string();
    }
    let helper = std::fs::read_to_string(gates_dir().join(HELPER))
        .expect("scripts/gates/function_table.sh reads");
    let mut out = String::new();
    for line in body.lines() {
        if line.contains(HELPER) {
            out.push_str(&helper);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// The uncapped, exclusive annotate of a profile, and the line it sits on.
fn whole_table_lines(body: &str) -> Vec<(usize, &str)> {
    body.lines()
        .enumerate()
        .filter(|(_, l)| {
            let l = l.trim_start();
            l.starts_with("callgrind_annotate")
                && l.contains("--threshold=100")
                && !l.contains("--inclusive")
        })
        .collect()
}

#[test]
fn the_exempt_list_names_only_gates_that_exist() {
    let all = gate_scripts();
    for name in EXEMPT {
        assert!(
            all.contains_key(name),
            "{name} is exempted from printing its whole table and no such gate \
             exists. An exemption for a deleted gate hides the next gate that \
             takes its name."
        );
    }
}

#[test]
fn every_profiling_gate_is_governed_or_exempt() {
    // The point of deriving the list: a gate added with a fixed profile path
    // and no whole-table print fails here rather than a round later.
    let profiling: Vec<String> = gate_scripts()
        .into_iter()
        .filter(|(_, body)| !profiles_written(body).is_empty())
        .map(|(name, _)| name)
        .collect();
    let governed: Vec<String> = governed().into_keys().collect();
    let unaccounted: Vec<&String> = profiling
        .iter()
        .filter(|n| !governed.contains(n) && !EXEMPT.contains(&n.as_str()))
        .collect();
    assert!(
        unaccounted.is_empty(),
        "these gates write a callgrind profile and are neither governed by this \
         spec nor exempt with a reason: {unaccounted:?}"
    );
    assert!(
        governed.len() >= 6,
        "only {} gates are governed, which is fewer than the six kanso#1558's \
         finding covers — the derivation has stopped seeing them: {governed:?}",
        governed.len()
    );
}

#[test]
fn every_profiling_gate_annotates_its_whole_profile() {
    for (name, body) in governed() {
        let body = resolved(&body);
        assert!(
            !whole_table_lines(&body).is_empty(),
            "scripts/gates/{name} profiles a fixed file and prints no whole \
             function table. It needs an exclusive `callgrind_annotate \
             --threshold=100`, on every run, or a move of a few instructions \
             cannot be located from two job logs. That is what kanso#1558 cost \
             on compile_instructions and what STATUS.md's standing row wants on \
             interp_instructions."
        );
    }
}

#[test]
fn the_whole_table_is_not_truncated() {
    for (name, body) in governed() {
        let body = resolved(&body);
        let lines: Vec<&str> = body.lines().collect();
        for (at, _) in whole_table_lines(&body) {
            // The command and whatever it is piped into, to the end of the
            // pipeline. The cap that hid the three was on a continuation line.
            let mut pipeline = String::new();
            let mut i = at;
            loop {
                pipeline.push_str(lines[i]);
                pipeline.push('\n');
                if !lines[i].trim_end().ends_with('\\') || i + 1 >= lines.len() {
                    break;
                }
                i += 1;
            }
            for cap in ["head ", "head -", "| tail", "sed -n '1,"] {
                assert!(
                    !pipeline.contains(cap),
                    "scripts/gates/{name} caps its whole-table annotate with \
                     `{cap}`, which is exactly what hid the three instructions \
                     on 2026-09-22:\n{pipeline}"
                );
            }
        }
    }
}

#[test]
fn the_whole_table_is_printed_before_the_comparison() {
    for (name, body) in governed() {
        let body = resolved(&body);
        let Some(compare) = body.find("if [ \"$got\" = \"$want\" ]; then") else {
            // Not every governed gate compares one row that way;
            // `instructions.sh` diffs a whole file. Those have nothing to be
            // before, and the truncation and presence tests still hold.
            continue;
        };
        let first = whole_table_lines(&body)
            .first()
            .map(|(at, _)| body.lines().take(*at).map(|l| l.len() + 1).sum::<usize>())
            .expect("the presence test runs first");
        assert!(
            first < compare,
            "scripts/gates/{name} prints its whole function table after the row \
             is compared, so a job whose row AGREED never emits one. A \
             comparison of two jobs needs both sides, and the agreeing side is \
             the one that is always missing."
        );
    }
}

#[test]
fn every_profiling_gate_stashes_its_table_where_the_packed_step_reads() {
    for (name, body) in governed() {
        let body = resolved(&body);
        assert!(
            body.contains(STASH),
            "scripts/gates/{name} prints a whole function table and never \
             stashes it under ${STASH}, so the packed end-of-job step cannot \
             re-emit it. A printed table that falls outside the last 5,000 \
             lines of the job log is written down and unreadable, which is \
             the position eighteen of twenty tables were in on 2026-09-22."
        );
    }
}

#[test]
fn the_packed_step_runs_last_and_runs_on_a_red_job() {
    let ci = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml"),
    )
    .expect("the workflow reads");
    let at = ci.find("function_tables_tail.sh").unwrap_or_else(|| {
        panic!(
            "no CI step runs scripts/gates/function_tables_tail.sh, so the \
             tables are stashed and never emitted. Stashing without emitting \
             is worse than the inline print it replaced."
        )
    });
    // The step's own block, back to the `- name:` that opens it.
    let head = ci[..at].rfind("      - name:").expect("the step has a name");
    let step = &ci[head..at];
    assert!(
        step.contains("if: always()"),
        "the packed-tables step does not carry `if: always()`, so the run that \
         most wants the tables — the one where a row went red and the verdict \
         step failed — is the run that does not print them:\n{step}"
    );
    // Nothing that prints a table may come after it, or the tail fills with
    // rows again and the packing bought nothing.
    let after = &ci[at..];
    for later in ["instructions.sh", "function_table.sh"] {
        assert!(
            !after.contains(later),
            "the cost-goldens job runs `{later}` AFTER the packed tables, so \
             its output pushes them back out of the log tail."
        );
    }
}

#[test]
fn the_walk_listing_runs_on_a_red_job_and_before_the_packed_tables() {
    // `compile_instructions` drifts by three between hosts and the frame that
    // carries it is a directory walk. Deciding between the two remedies the
    // 2026-09-15 ruling allows — normalize the state, or exclude the term and
    // name the exclusion — needs to know whether the ENTRY SET differs between
    // hosts or only its layout. Nothing recorded that, so a step prints it.
    //
    // It has to survive a red job, because the jobs worth comparing are the
    // ones where the row disagreed.
    let ci = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml"),
    )
    .expect("the workflow reads");
    let at = ci.find("#walk-entry ").unwrap_or_else(|| {
        panic!(
            "no CI step lists what the compile row's directory walk sees. The \
             three instructions are in that walk and the remedy turns on what \
             the walk is handed."
        )
    });
    let head = ci[..at].rfind("      - name:").expect("the step has a name");
    assert!(
        ci[head..at].contains("if: always()"),
        "the walk listing does not carry `if: always()`, so the jobs where the \
         row disagreed — the only ones worth comparing — do not print it"
    );
    let packed = ci.find("function_tables_tail.sh").expect("the packed step exists");
    assert!(
        at < packed,
        "the walk listing comes after the packed tables, which are meant to be \
         the last thing in the job"
    );
}
