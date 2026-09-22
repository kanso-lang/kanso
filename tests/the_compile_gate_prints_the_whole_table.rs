//! The compile row's gate prints its WHOLE function table, uncapped.
//!
//! On 2026-09-22 `compile_instructions` read 35,551,167 on main and
//! 35,551,170 on a branch whose diff was `design/compiler-log.md` and
//! `docs/compiler.html` — files no part of the compiler reads, since only
//! `lib/*.kso` is `include_str!`'d into it. Localising those three meant
//! comparing the two jobs' profiles, and the job log carried
//! `--threshold=90 | head -40`: forty rows of the hundred and twenty-five that
//! threshold has, and fifteen functions once the header lines were dropped.
//! Every one of the fifteen was equal to the instruction, so the comparison
//! ended there.
//!
//! The whole table is about 1,115 rows and reaches functions that retire a
//! SINGLE instruction, which is the resolution a three-instruction move needs.
//! Such a move can sit in a function too small to make any threshold, so
//! nothing short of the whole table catches one.
//!
//! Four jobs have now read this row on trees that cannot reach the compiler,
//! two at 35,551,167 and two at 35,551,170, and one CPU model produced both
//! values — so the machine is not the variable and the table is what would say
//! what is.
//!
//! It has to be printed on EVERY run rather than on the failing one, because a
//! comparison needs both sides and only one side is ever the job that failed.
//!
//! And it has to be in the JOB LOG rather than in the artifact that already
//! holds the raw profile: some sessions cannot fetch the artifact at all, their
//! egress policy refusing the blob host with `gateway answered 403 to CONNECT`,
//! which no credential and no retry gets past.

const GATE: &str = include_str!("../scripts/gates/compile_instructions.sh");

/// The gate's uncapped, exclusive annotate of the compile profile.
///
/// `--inclusive=yes` is a different reading and the gate makes two of those;
/// this looks for the exclusive one, which is the per-function table.
fn whole_table_command() -> &'static str {
    GATE.lines()
        .find(|l| {
            let l = l.trim_start();
            l.starts_with("callgrind_annotate")
                && l.contains("--threshold=100")
                && l.contains("/tmp/cg.compile")
                && !l.contains("--inclusive")
        })
        .unwrap_or_else(|| {
            panic!(
                "scripts/gates/compile_instructions.sh prints no whole function \
                 table. It needs an exclusive `callgrind_annotate \
                 --threshold=100 /tmp/cg.compile`, on every run, or a move of a \
                 few instructions cannot be located from two job logs."
            )
        })
}

#[test]
fn the_compile_gate_annotates_the_whole_profile() {
    let cmd = whole_table_command();
    assert!(
        cmd.contains("--threshold=100"),
        "the whole-table annotate must ask for the whole table: {cmd}"
    );
}

/// The cap is the thing that hid the three, so the pipeline may not truncate.
#[test]
fn the_whole_table_is_not_truncated() {
    // The command and whatever it is piped into, up to the end of the pipeline.
    let start = GATE.find(whole_table_command()).expect("the command is in the script");
    let tail = &GATE[start..];
    let pipeline: String = {
        let mut out = String::new();
        for line in tail.lines() {
            out.push_str(line);
            out.push('\n');
            if !line.trim_end().ends_with('\\') {
                break;
            }
        }
        out
    };
    for cap in ["head ", "head -", "| tail", "sed -n '1,"] {
        assert!(
            !pipeline.contains(cap),
            "the whole-table annotate is capped with `{cap}`, which is exactly \
             what hid the three instructions on 2026-09-22:\n{pipeline}"
        );
    }
}

/// And it runs on every run, not only when the row has already parted.
///
/// The gate's failure path starts at the second callgrind pass, which is taken
/// only after the comparison fails. A table printed after that point is a table
/// the green side never emits, and a comparison needs the green side.
#[test]
fn the_whole_table_is_printed_before_the_comparison() {
    let table = GATE.find(whole_table_command()).expect("the command is in the script");
    let compare = GATE
        .find("if [ \"$got\" = \"$want\" ]; then")
        .expect("the gate compares the row against the golden");
    assert!(
        table < compare,
        "the whole function table is printed after the row is compared, so a \
         job whose row AGREED never emits one. A comparison of two jobs needs \
         both sides, and the agreeing side is the one that is always missing."
    );
}
