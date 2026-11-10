//! A fiber waiting on a socket is waited for, on both engines.
//!
//! A read that found nothing arrived, or an accept that found nobody
//! calling, went back in the queue to be asked again a millisecond later.
//! A query to a database cost a millisecond for that, and a server with
//! one fiber waiting on a socket woke a thousand times a second (#2405).
//! Both schedulers now wait on the sockets themselves, for as long as the
//! earliest fiber on the clock allows.
//!
//! The fixture is the mem corpus's: a reader with nothing to read beside a
//! writer that sleeps fifty milliseconds before it writes. There are two
//! things to wait for, the writer's clock and then the reader's socket.
//! `tests/golden/mem/a_reader_waits_on_its_socket.mem` pins the native
//! count among the allocator's; this holds the interpreter to the same
//! number, which no golden of its own carries.
//!
//! Watched red with the wait taken out of each engine in turn: the count
//! is then not reported at all, because the scheduler naps instead.

use std::path::PathBuf;
use std::process::Command;

fn waits(engine: &[&str]) -> Option<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-sock-waits-{}", engine.len()));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a directory to run in");
    let name = "a_reader_waits_on_its_socket";
    std::fs::copy(
        root.join("tests/golden/mem").join(format!("{name}.kso")),
        stage.join(format!("{name}.kso")),
    )
    .expect("the fixture copies");
    std::fs::write(stage.join("run.kso"), format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(stage.join("run.kso"))
        .args(engine)
        .env("KANSO_COUNTERS", "1")
        .output()
        .expect("kanso runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "the reader heard: late\n");
    let counters = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&stage);
    counters.lines().find_map(|l| l.strip_prefix("sock_waits=")).map(str::to_string)
}

#[test]
fn the_scheduler_waits_twice_for_a_clock_and_a_socket() {
    assert_eq!(waits(&[]).as_deref(), Some("2"), "natively");
    assert_eq!(waits(&["--interp"]).as_deref(), Some("2"), "interpreted");
}
