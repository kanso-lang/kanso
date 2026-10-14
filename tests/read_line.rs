//! `io/read_line` reads standard input, which the micro corpus does not feed:
//! it runs every program with nothing on stdin, so there it pins only the end
//! of the input. Each program in `tests/golden/stdin` carries the input it is
//! fed (`.in`) beside what it must print (`.out`) and, when it fails, what it
//! must say (`.err`). Both engines that read a real stdin are held to the same
//! bytes: the native build and the interpreter.
//!
//! The corpus pipes the whole input in before the program starts, so it
//! cannot see whether output was flushed before a read waited. The last test
//! here plays a person at a terminal: it waits for each prompt before it
//! types the answer, so a prompt left in a buffer stalls it.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus() -> PathBuf {
    root().join("tests/golden/stdin")
}

fn programs() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(corpus())
        .expect("the stdin corpus lists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "kso"))
        .collect();
    found.sort();
    found
}

/// A directory holding the program and an entry that imports it, the way the
/// micro corpus runs a library. Named per engine so two tests never share one.
fn staged(program: &Path, engine: &str) -> (PathBuf, String) {
    let name = program.file_stem().and_then(|s| s.to_str()).expect("named").to_string();
    let stage = std::env::temp_dir().join(format!("kanso-stdin-{engine}-{name}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the stage makes");
    std::fs::copy(program, stage.join(format!("{name}.kso"))).expect("the program copies");
    let entry = format!("run_{name}.kso");
    std::fs::write(stage.join(&entry), format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry writes");
    (stage, entry)
}

fn golden(program: &Path, ext: &str) -> Option<Vec<u8>> {
    std::fs::read(program.with_extension(ext)).ok()
}

#[test]
fn every_engine_reads_the_lines_it_is_fed() {
    let found = programs();
    assert!(found.len() >= 6, "the stdin corpus holds {} programs", found.len());
    for program in found {
        let input = golden(&program, "in").expect("a stdin program carries its input");
        let out = golden(&program, "out").expect("a stdin program carries its output");
        let err = golden(&program, "err");
        for (engine, extra) in [("native", &[][..]), ("interp", &["--interp"][..])] {
            let (stage, entry) = staged(&program, engine);
            let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .arg("run")
                .arg(&entry)
                .args(extra)
                .current_dir(&stage)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("kanso starts");
            child.stdin.take().expect("stdin").write_all(&input).expect("the input goes in");
            let done = child.wait_with_output().expect("kanso finishes");
            let _ = std::fs::remove_dir_all(&stage);
            let name = program.display();
            assert_eq!(
                String::from_utf8_lossy(&done.stdout),
                String::from_utf8_lossy(&out),
                "{name} prints differently on {engine}"
            );
            assert_eq!(
                String::from_utf8_lossy(&done.stderr),
                String::from_utf8_lossy(err.as_deref().unwrap_or_default()),
                "{name} says something different on stderr on {engine}"
            );
            let code = if err.is_some() { 1 } else { 0 };
            assert_eq!(done.status.code(), Some(code), "{name} exits differently on {engine}");
        }
    }
}

/// Reads from `rx` until what has arrived ends with `want`, or fails after
/// `wait`. A prompt still sitting in the program's buffer never arrives.
fn expect_tail(rx: &mpsc::Receiver<u8>, seen: &mut Vec<u8>, want: &str, wait: Duration) {
    while !seen.ends_with(want.as_bytes()) {
        match rx.recv_timeout(wait) {
            Ok(byte) => seen.push(byte),
            Err(_) => panic!(
                "waited for {want:?} and the program had written only {:?}",
                String::from_utf8_lossy(seen)
            ),
        }
    }
}

/// Ends the program when a wait fails, so a stalled prompt fails the test
/// without leaving the program waiting on a read nobody will answer.
struct Killed(std::process::Child);

impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_prompt_is_written_before_the_read_waits() {
    let program = corpus().join("a_prompt_between_reads_arrives_in_order.kso");
    for (engine, extra) in [("native-live", &[][..]), ("interp-live", &["--interp"][..])] {
        let (stage, entry) = staged(&program, engine);
        let mut child = Killed(
            Command::new(env!("CARGO_BIN_EXE_kanso"))
                .arg("run")
                .arg(&entry)
                .args(extra)
                .current_dir(&stage)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("kanso starts"),
        );
        let mut stdout = child.0.stdout.take().expect("stdout");
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            while let Ok(1) = stdout.read(&mut byte) {
                if tx.send(byte[0]).is_err() {
                    break;
                }
            }
        });
        let mut typing = child.0.stdin.take().expect("stdin");
        let mut seen = Vec::new();
        // the first prompt waits on a compile as well as on the flush
        expect_tail(&rx, &mut seen, "> ", Duration::from_secs(120));
        typing.write_all(b"alpha\n").expect("the first answer goes in");
        expect_tail(&rx, &mut seen, "echo: alpha\n> ", Duration::from_secs(20));
        typing.write_all(b"beta\n").expect("the second answer goes in");
        expect_tail(&rx, &mut seen, "echo: beta\n> ", Duration::from_secs(20));
        drop(typing);
        expect_tail(&rx, &mut seen, "bye\n", Duration::from_secs(20));
        let status = child.0.wait().expect("kanso finishes");
        reader.join().expect("the reader ends");
        let _ = std::fs::remove_dir_all(&stage);
        assert!(status.success(), "{engine} exits {status}");
        assert_eq!(
            String::from_utf8_lossy(&seen),
            "> echo: alpha\n> echo: beta\n> bye\n",
            "{engine} interleaved the conversation differently"
        );
    }
}
