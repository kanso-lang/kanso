//! A loop that forks each pass's work beside the rest of the loop must not
//! hold more memory the longer it runs, on either engine (#2263).
//!
//! This is a server's shape: accept, then run the handler and the next accept
//! as a group. Each pass is a group the next pass is a member of, so the
//! groups nest as deep as the loop has run. A group whose handler finished
//! cleanly has nothing left to do but pass the loop's outcome up, and holding
//! it held a server's memory in proportion to the requests it had answered.
//!
//! tests/golden/mem/a_server_loop_holds_what_one_request_holds.mem pins the
//! native counters at one size. This file compares two sizes on both engines,
//! which a single golden cannot do for the interpreter: its peak includes the
//! front end, so the number that says something is the difference.

use std::process::Command;

fn program(n: u64) -> String {
    format!(
        "import \"std/text\"\n\
         import \"std/time\"\n\n\
         fn handled n\n  \
         page = text/join [\"<p>\" \"request {{n}}\" \"</p>\"] \"\"\n  \
         time/sleep 0 .> (_ -> effect (length page))\n\n\
         fn served 0\n  print \"served them all\"\n\n\
         fn served n\n  handled n\n  \
         time/sleep 0 .> (_ -> served (n - 1))\n\n\
         served {n}\n"
    )
}

/// The peak a run of `n` passes reports under the counter named `counter`.
fn peak(n: u64, engine: &[&str], counter: &str) -> u64 {
    let dir = std::env::temp_dir().join(format!("kanso-server-loop-{n}-{}", engine.len()));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("run.kso"), program(n)).expect("the program writes");
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("run.kso")
        .args(engine)
        .env("KANSO_COUNTERS", "1")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(String::from_utf8_lossy(&done.stdout), "served them all\n");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    said.lines()
        .find_map(|l| l.trim().strip_prefix(&format!("{counter}="))?.parse().ok())
        .unwrap_or_else(|| panic!("no {counter} in:\n{said}"))
}

/// Natively the arena is the whole story: one block at either size. Before
/// the scheduler let finished groups go and rewound what passes left behind,
/// 4,000 passes held 4 blocks and 16,000 held 13.
#[test]
fn natively_a_server_loop_holds_one_arena_block_at_any_length() {
    assert_eq!(peak(4_000, &[], "arena_peak_bytes"), 1_048_576);
    assert_eq!(peak(16_000, &[], "arena_peak_bytes"), 1_048_576);
}

/// The interpreter frees what nothing holds, so what it held was the groups
/// and one table entry for each pass. Its peak includes the front end, which
/// is the same at both sizes, so the two peaks are compared with each other.
/// Four times the passes costs one byte, the extra digit in the program's
/// text. Before, 4,000 passes peaked at 1,420,281 bytes and 16,000 at
/// 5,581,819.
#[test]
fn interpreted_a_server_loop_holds_no_more_the_longer_it_runs() {
    let short = peak(4_000, &["--interp"], "interp_peak_bytes");
    let long = peak(16_000, &["--interp"], "interp_peak_bytes");
    assert_eq!(
        long as i64 - short as i64,
        1,
        "four times the passes held more memory: {short} bytes at 4,000 \
         against {long} at 16,000"
    );
}
