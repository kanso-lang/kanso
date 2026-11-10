//! A lazy binding a callback holds is freed once the callback has run, so a
//! loop that makes one each pass holds no more of them the longer it runs
//! (#2408).
//!
//! This is a server's shape: each pass binds a value its handler may never
//! read and hands it to a callback that runs later. The frame that made the
//! binding cannot free its cell, because the callback still holds it, and
//! until the scheduler's beat took such cells they lived until the program
//! exited. A senro app held 184 bytes more for every request it answered.
//!
//! tests/golden/mem/a_callback_lets_go_of_the_lazy_binding_it_read.mem pins
//! the native counters at one length. This file compares two lengths, which
//! is the claim: the cells left at exit do not grow with the passes.

use std::process::Command;

fn program(n: u64) -> String {
    format!(
        "import \"std/time\"\n\n\
         fn built n\n  \"request {{n}}\"\n\n\
         fn sized 0 _\n  effect 0\n\n\
         fn sized _ page\n  effect (length page)\n\n\
         fn handled n\n  page = built n\n  \
         time/sleep 0 .> (_ -> sized n page)\n\n\
         fn served 0\n  print \"served them all\"\n\n\
         fn served n\n  handled n\n  \
         time/sleep 0 .> (_ -> served (n - 1))\n\n\
         served {n}\n"
    )
}

/// What a native run of `n` passes reports under the counter named `counter`.
fn counted(n: u64, counter: &str) -> u64 {
    let dir = std::env::temp_dir().join(format!("kanso-held-cells-{n}-{counter}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("run.kso"), program(n)).expect("the program writes");
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("run.kso")
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

/// Every pass makes one cell, and the only one live at exit is the last
/// pass's, which no rewind came after. Before, each length left as many live
/// as it had made: 4,000 and 16,000.
#[test]
fn the_cells_left_at_exit_do_not_grow_with_the_passes() {
    assert_eq!(counted(4_000, "thunk_allocs"), 4_000);
    assert_eq!(counted(4_000, "thunk_live_exit"), 1);
    assert_eq!(counted(16_000, "thunk_allocs"), 16_000);
    assert_eq!(counted(16_000, "thunk_live_exit"), 1);
}
