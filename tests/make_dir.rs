//! A program that writes a file has to be able to make the directory it goes
//! in. `io/write_file` refuses a path whose parent is missing, so without this
//! every generator script shells out to `mkdir -p` — which is what
//! bench/make_jsonbench.kso still does, and what this exists to retire.
//!
//! Making a directory that is already there succeeds, the way `mkdir -p` does.
//! A generator run twice is the ordinary case, not an error.

use std::process::Command;

fn ran(program: &str, dir: &std::path::Path) -> (String, String) {
    let file = dir.join("run.kso");
    std::fs::write(&file, program).expect("the program writes");
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("run.kso")
        .current_dir(dir)
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

#[test]
fn a_directory_is_made_with_its_parents() {
    let dir = std::env::temp_dir().join("kanso-make-dir");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");

    let program = "import \"std/io\"\n\n\
                   pub play = made\n\n\
                   made = io/make_dir \"deep/nested\" . (_ -> wrote)\n\n\
                   wrote = io/write_file \"deep/nested/f.txt\" \"hello\" . (_ -> told)\n\n\
                   told = io/is_dir \"deep/nested\" . said\n\n\
                   fn said yes\n  print \"{yes}\"\n";
    let (out, err) = ran(program, &dir);
    let content = std::fs::read_to_string(dir.join("deep/nested/f.txt")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(out, "true\n", "make_dir did not make the directory: {err}");
    assert_eq!(content, "hello", "the file its parent was made for is missing");
}

/// `mkdir -p` succeeds on a directory that exists, and a generator that runs
/// twice depends on that.
#[test]
fn making_a_directory_that_exists_is_not_an_error() {
    let dir = std::env::temp_dir().join("kanso-make-dir-twice");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");

    let program = "import \"std/io\"\n\n\
                   pub play = once\n\n\
                   once = io/make_dir \"here\" . (_ -> twice)\n\n\
                   twice = io/make_dir \"here\" . (_ -> print \"ok\")\n";
    let (out, err) = ran(program, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(out, "ok\n", "the second make_dir refused: {err}");
}
