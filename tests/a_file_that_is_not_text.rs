//! Three bytes — `a`, `0xFF`, `b` — and two readers, on both engines.
//!
//! `read_file` is text. Native used to read any file: `runtime.c` opened it
//! `"rb"`, took the bytes and handed them back, and the round trip was exact.
//! The interpreter could not follow it there, because it reads into a Rust
//! `String`, so the two engines answered this file differently, and the first
//! version of this spec pinned each engine's own answer while filing the
//! question of whether they should agree. Clay ruled it on 2026-08-29
//! (archive, "gavel: read_file is text, read_bytes is bytes, per precedent"):
//! `read_file` refuses bytes that are not text on every engine, with the same
//! words, and `read_bytes` is the reader that hands bytes back.
//!
//! The refusal names the real cause. Before the reason was kept it said
//! `no such file or unreadable` about a file that is present, readable, and
//! three bytes long — the reason had been thrown away by a `map_err(|_| ...)`.
use std::path::PathBuf;
use std::process::Command;

const BYTES: [u8; 3] = [b'a', 0xFF, b'b'];

const AS_TEXT: &str =
    "import \"std/io\"\nimport \"std/os\"\n\nos/read_file \"three.bin\" .> io/write\n";

/// The bytes go out through `write_file`, which the ruling says takes bytes;
/// the copy is what the spec reads back.
const AS_BYTES: &str =
    "import \"std/os\"\n\nos/read_bytes \"three.bin\" .> (b -> os/write_file \"copy.bin\" b)\n";

/// One directory per test. They share nothing but the bytes: each writes
/// `run.kso` and cargo runs them at the same time, so a shared directory let
/// one test read another's half-written file — `an entry file needs at least
/// one statement`, at random, on a file that has one. The same collision
/// kanso#1169 fixed for the two playground tests.
///
/// The path is RELATIVE and the program is run from the directory holding
/// it. An absolute one was interpolated here first, and it made the source
/// line's length a property of the host's temp directory: `/tmp/...` on
/// linux fits, and macOS's
/// `/var/folders/df/djsxfhc17x95674wsm_g8s980000gn/T/...` took the line to
/// 99 characters, where kanso allows 80. So the spec failed on the other
/// host with a formatting refusal and never reached what it meant to test.
fn fixture(who: &str, program: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-not-text-{who}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let _ = std::fs::remove_file(dir.join("copy.bin"));
    std::fs::write(dir.join("three.bin"), BYTES).expect("the fixture writes");
    std::fs::write(dir.join("run.kso"), program).expect("the program writes");
    dir
}

fn run(who: &str, program: &str, interp: bool) -> (String, Vec<u8>, PathBuf) {
    let dir = fixture(who, program);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("run").arg("run.kso").current_dir(&dir);
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.stdout, dir)
}

fn refuses_as_text(who: &str, interp: bool) {
    let (said, out, _) = run(who, AS_TEXT, interp);
    assert!(out.is_empty(), "{who} wrote bytes through a text reader: {out:?}");
    assert!(
        said.contains("cannot read three.bin: the bytes are not text"),
        "{who} did not refuse with the ruled sentence: {said}"
    );
    assert!(!said.contains("no such file"), "{who} still claims the file is absent: {said}");
}

/// Native refuses, with the ruled sentence. Until 2026-09-09 it handed the
/// three bytes through as a string, and this assertion is the one the old
/// spec said would go red when the question was settled.
#[test]
fn native_refuses_a_file_that_is_not_text() {
    refuses_as_text("native", false);
}

/// The interpreter refuses with the same words.
#[test]
fn the_interpreter_refuses_and_names_the_reason() {
    refuses_as_text("interp", true);
}

/// The other reader hands the bytes back untouched, the invalid one included,
/// on both engines, and `write_file` takes bytes: the copy is the original.
#[test]
fn read_bytes_hands_the_bytes_back_on_both_engines() {
    for (who, interp) in [("native-bytes", false), ("interp-bytes", true)] {
        let (said, _, dir) = run(who, AS_BYTES, interp);
        let copy = std::fs::read(dir.join("copy.bin")).unwrap_or_default();
        assert_eq!(copy, BYTES, "{who} changed the bytes; it said: {said}");
    }
}
