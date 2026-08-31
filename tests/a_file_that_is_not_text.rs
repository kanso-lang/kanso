//! Three bytes — `a`, `0xFF`, `b` — and the two engines answer differently.
//!
//! Native reads any file. `runtime.c` opens it `"rb"`, takes the bytes, and
//! hands them back; the round trip is exact, and a program that reads this
//! file and writes it out again produces `61 ff 62`. The interpreter cannot
//! follow it there: it reads into a Rust `String`, which cannot hold bytes
//! that are not utf-8, so it refuses.
//!
//! The differential law allows an engine to speak less than another only when
//! the quieter one REFUSES with a clear diagnostic. Until this spec was
//! written the refusal said `no such file or unreadable` about a file that is
//! present, readable, and 3 bytes long — the reason had been thrown away by a
//! `map_err(|_| ...)`, so the one thing the message needed to say was the one
//! thing it could not.
//!
//! WHAT IS PINNED AND WHAT IS NOT. That each engine gives its own answer, and
//! that the interpreter's answer names the real cause. NOT that they should
//! differ: whether `read_file` is byte-transparent on every engine is a design
//! question, and it is filed rather than settled here. When it is settled one
//! of these two assertions is what goes red.
use std::process::Command;

const BYTES: [u8; 3] = [b'a', 0xFF, b'b'];

/// One directory per test. They share nothing but the bytes: both write
/// `run.kso` and cargo runs them at the same time, so a shared directory let
/// one test read the other's half-written file — `an entry file needs at least
/// one statement`, at random, on a file that has one. The same collision
/// kanso#1169 fixed for the two playground tests.
fn fixture(who: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-not-text-{who}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("three.bin"), BYTES).expect("the fixture writes");
    // The path is RELATIVE and the program is run from the directory holding
    // it. An absolute one was interpolated here first, and it made the source
    // line's length a property of the host's temp directory: `/tmp/...` on
    // linux fits, and macOS's
    // `/var/folders/df/djsxfhc17x95674wsm_g8s980000gn/T/...` took the line to
    // 99 characters, where kanso allows 80. So the spec failed on the other
    // host with a formatting refusal and never reached what it meant to test.
    let program = "import \"std/io\"\nimport \"std/os\"\n\nos/read_file \"three.bin\" . io/write\n";
    std::fs::write(dir.join("run.kso"), program).expect("the program writes");
    dir
}

fn run(who: &str, interp: bool) -> (String, Vec<u8>) {
    let dir = fixture(who);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("run").arg("run.kso").current_dir(&dir);
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.stdout)
}

/// Native hands the bytes back untouched, the invalid one included.
#[test]
fn native_reads_a_file_that_is_not_text() {
    let (said, out) = run("native", false);
    assert_eq!(out, BYTES, "native changed the bytes; it said: {said}");
}

/// The interpreter refuses, and says why. Before the reason was kept this read
/// `no such file or unreadable` — about a file three bytes long that is
/// sitting right there.
#[test]
fn the_interpreter_refuses_and_names_the_reason() {
    let (said, _) = run("interp", true);
    assert!(said.contains("the bytes are not text"), "the refusal did not name the reason: {said}");
    assert!(!said.contains("no such file"), "the refusal still claims the file is absent: {said}");
}
