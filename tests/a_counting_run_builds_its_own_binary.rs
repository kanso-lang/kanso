//! `kanso run` keeps each program's binary in the temp directory, keyed by the
//! program's IR and the runtime's digest. A run under `KANSO_COUNTERS` links a
//! counting runtime and the IR is the same either way, so a key without the
//! counting bit handed a counting run the binary an ordinary run had built,
//! which printed no counters. The mem vein runs its fixtures under
//! `KANSO_COUNTERS`, so one of them run by hand first could not be regenerated.
//! Found on 2026-09-24 by exactly that.
use std::process::Command;

#[test]
fn a_counting_run_after_an_ordinary_one_still_counts() {
    // A program nobody else runs, so the first run below is the one that
    // builds its binary whatever this machine's cache already holds.
    let stamp = format!(
        "{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(format!("kanso-counting-{stamp}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("run.kso"), format!("print \"{stamp}\"\n"))
        .expect("the program writes");

    let run = |counting: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
        command.arg("run").arg("run.kso").current_dir(&dir);
        command.env_remove("KANSO_COUNTERS").env_remove("KANSO_COUNTERS_BUILD");
        if counting {
            command.env("KANSO_COUNTERS", "1");
        }
        command.output().expect("kanso runs")
    };
    let plain = run(false);
    let counted = run(true);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(String::from_utf8_lossy(&plain.stdout), format!("{stamp}\n"));
    assert_eq!(String::from_utf8_lossy(&counted.stdout), format!("{stamp}\n"));
    let said = String::from_utf8_lossy(&counted.stderr);
    assert!(
        said.lines().any(|line| line.starts_with("allocs=")),
        "a counting run printed no counters:\n{said}"
    );
}
