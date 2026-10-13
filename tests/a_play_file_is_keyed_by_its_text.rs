//! A second `kanso play` of an unchanged file finds its binary by the file's
//! text and does not emit. What that key has to hold is what these check: a
//! file whose text changed between plays runs the new text, and a file that
//! read a module from disk is not keyed by its own text at all, since the
//! module can change under it while the file stays the same.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso_played_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the directory is made");
    dir
}

fn play(file: &Path, std_dir: Option<&Path>) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
    command.arg("play").arg(file);
    if let Some(dir) = std_dir {
        command.env("KANSO_STD", dir);
    }
    let run = command.output().expect("kanso runs");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// The same name, new text: the second play runs what the file says now.
#[test]
fn an_edited_play_file_runs_what_it_now_says() {
    let dir = fresh("edited");
    let file = dir.join("main.kso");
    std::fs::write(&file, "print \"first\"\n").unwrap();
    assert_eq!(play(&file, None), "first\n");
    std::fs::write(&file, "print \"second\"\n").unwrap();
    assert_eq!(play(&file, None), "second\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `std/expect` is the one library module the compiler does not carry, so a
/// play file that imports it reads it from disk. The play file does not
/// change here and the module does, and the second play runs the new module.
#[test]
fn a_module_read_from_disk_changes_what_the_same_play_file_runs() {
    let dir = fresh("from_disk");
    let module = dir.join("std/expect");
    std::fs::create_dir_all(&module).unwrap();
    std::fs::write(module.join("expect.kso"), "pub fn shout s\n  \"{s}!\"\n").unwrap();
    let file = dir.join("main.kso");
    std::fs::write(&file, "import \"std/expect\"\n\nprint (expect/shout \"a\")\n").unwrap();
    let std_dir = dir.join("std");
    assert_eq!(play(&file, Some(&std_dir)), "a!\n");
    std::fs::write(module.join("expect.kso"), "pub fn shout s\n  \"{s}?\"\n").unwrap();
    assert_eq!(play(&file, Some(&std_dir)), "a?\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A warm play runs the binary without compiling the file, and a death by
/// signal is worded from the compiled program. The second play of a file that
/// runs out of stack compiles it then, and says what ended it as the first did.
#[test]
fn a_warm_play_that_runs_out_of_stack_still_says_so() {
    let dir = fresh("out_of_stack");
    let file = dir.join("main.kso");
    std::fs::write(
        &file,
        "fn total n\n  return 0 if n < 1\n  n + total (n - 1)\n\nprint \"{total 2000000}\"\n",
    )
    .unwrap();
    for round in ["first", "second"] {
        let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("play")
            .arg(&file)
            .output()
            .expect("kanso runs");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(err.contains("ran out of stack"), "{round} play said: {err}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A binary that runs out of stack now says so itself, so the play's own
/// wording is reached only by a death the program cannot explain: here, a
/// signal sent from outside while it waits on stdin. Both plays name it.
#[cfg(target_os = "linux")]
#[test]
fn a_warm_play_ended_by_a_signal_names_it() {
    use std::io::Read;
    use std::process::Stdio;
    let dir = fresh("signalled");
    let file = dir.join("main.kso");
    std::fs::write(&file, "import \"std/io\"\n\nio/stdin .> (text -> print \"{length text}\")\n")
        .unwrap();
    for round in ["first", "second"] {
        let mut run = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("play")
            .arg(&file)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("kanso runs");
        let program = program_under(run.id());
        assert_eq!(unsafe { kill(program, 15) }, 0, "{round} play's program takes the signal");
        let status = run.wait().expect("the play ends");
        let mut err = String::new();
        run.stderr.take().unwrap().read_to_string(&mut err).unwrap();
        assert!(!status.success(), "{round} play succeeded");
        assert!(err.contains("ended by signal 15"), "{round} play said: {err:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "linux")]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// The compiled program `kanso play` started, found among its children by the
/// names the two caches give their binaries: `kanso_run_` on a cold play,
/// `kanso_play_` on a warm one. Compiling starts other children first.
#[cfg(target_os = "linux")]
fn program_under(play: u32) -> i32 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        let children = std::fs::read_to_string(format!("/proc/{play}/task/{play}/children"))
            .unwrap_or_default();
        for child in children.split_whitespace() {
            let comm = std::fs::read_to_string(format!("/proc/{child}/comm")).unwrap_or_default();
            if comm.starts_with("kanso_play_") || comm.starts_with("kanso_run_") {
                return child.parse().unwrap();
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("kanso play started no program within two minutes");
}
