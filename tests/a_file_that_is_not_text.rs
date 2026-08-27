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

fn fixture() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("kanso-not-text");
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let blob = dir.join("three.bin");
    std::fs::write(&blob, BYTES).expect("the fixture writes");
    let program = format!(
        "import \"std/io\"\nimport \"std/os\"\n\nos/read_file \"{}\" . io/write\n",
        blob.display()
    );
    std::fs::write(dir.join("run.kso"), program).expect("the program writes");
    dir
}

fn run(interp: bool) -> (String, Vec<u8>) {
    let dir = fixture();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("run").arg(dir.join("run.kso"));
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.stdout)
}

/// Native hands the bytes back untouched, the invalid one included.
#[test]
fn native_reads_a_file_that_is_not_text() {
    let (said, out) = run(false);
    assert_eq!(out, BYTES, "native changed the bytes; it said: {said}");
}

/// The interpreter refuses, and says why. Before the reason was kept this read
/// `no such file or unreadable` — about a file three bytes long that is
/// sitting right there.
#[test]
fn the_interpreter_refuses_and_names_the_reason() {
    let (said, _) = run(true);
    assert!(said.contains("the bytes are not text"), "the refusal did not name the reason: {said}");
    assert!(!said.contains("no such file"), "the refusal still claims the file is absent: {said}");
}
