//! A cycle of an empty list ends at once, on every engine.
//!
//! `list/cycle []` has no element to repeat. Its `next` wrapped back to the
//! first position and asked again, forever: the interpreter and a dev build
//! spun until they were stopped, and a release build ran out of memory
//! first. Found by a generated program on 2026-09-28 that took four elements
//! of an empty cycle.
//!
//! The program runs under a deadline, so the old behaviour fails this spec
//! instead of hanging it. The empty list is built at run time, so no engine
//! can fold the call away.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

fn staged(tag: &str) -> PathBuf {
    let stage = std::env::temp_dir().join(format!("kanso-empty-cycle-{tag}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("defs")).expect("a staging directory");
    std::fs::write(
        stage.join("defs/defs.kso"),
        "import \"std/list\"\nimport \"std/os\"\n\n\
         fn shown z\n  \
         none_left = list/to_list (list/take (list/cycle (list/take [1 2] z)) 4)\n  \
         \"{none_left}\"\n\n\
         pub play = os/args .> (a -> print (shown (length a)))\n",
    )
    .expect("the module writes");
    std::fs::write(stage.join("main.kso"), "import \"./defs\"\n\ndefs/play\n")
        .expect("the entry writes");
    stage
}

/// Runs `program` in `dir` and answers its stdout, or None when it has not
/// finished in ten seconds.
fn within_deadline(program: &Path, args: &[&str], dir: &Path) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("it starts");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if child.try_wait().expect("it can be waited on").is_some() {
            let out = child.wait_with_output().expect("its output reads");
            return Some(String::from_utf8_lossy(&out.stdout).into_owned());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    None
}

#[test]
fn the_interpreter_ends_an_empty_cycle() {
    let stage = staged("i");
    let said = within_deadline(&kanso(), &["run", "main.kso", "--interp"], &stage);
    assert_eq!(said.as_deref(), Some("[]\n"), "the interpreter did not end an empty cycle");
}

#[test]
fn a_compiled_build_ends_an_empty_cycle() {
    let stage = staged("n");
    let built = Command::new(kanso())
        .args(["build", "main.kso"])
        .current_dir(&stage)
        .output()
        .expect("kanso builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let said = within_deadline(&stage.join("main"), &[], &stage);
    assert_eq!(said.as_deref(), Some("[]\n"), "a compiled build did not end an empty cycle");
}
