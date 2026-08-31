// `os/exit 3` is a program saying what it meant. The endpoint reads the
// `os/exit_status` an err carries and exits with that code, in silence.
//
// The corpora cannot express this. `tests/golden/runtime` asserts every
// program in it exits 1, because it is the endpoint-violations corpus;
// `tests/golden/micro` asserts every program in it exits 0. A deliberate
// exit of THREE is neither, so the code passing through is pinned here.
// The micro corpus holds the zero case, which is what the three engines'
// differential walk compares.
use std::process::Command;

// One directory per test, named by the caller. Both tests staged the same
// path and cargo runs them on separate threads, so one could be inside
// `fs::write` — which truncates before it writes — while the other read the
// file. The reader then compiled an empty or half-written program, and the
// deliberate exit of 3 came back as the compiler's 2. It failed 2 runs in 20
// measured back to back, which is often enough to redden an unrelated pull request and rare
// enough to look like something else. kanso#1169 fixed the same shape in the
// playground tests.
fn exits(name: &str, engine: &[&str]) -> (Option<i32>, String) {
    let dir = std::env::temp_dir().join(format!("kanso-deliberate-exit-{name}"));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("main.kso");
    let program = "import \"std/io\"\nimport \"std/os\"\n\nio/write \"before\" >> os/exit 3\n";
    std::fs::write(&file, program).expect("fixture writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&file)
        .args(engine)
        .output()
        .expect("kanso runs");
    (out.status.code(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn the_native_engine_carries_the_code_out() {
    let (code, err) = exits("native", &[]);
    assert_eq!(code, Some(3), "the code the program named did not reach the shell");
    assert_eq!(err, "", "a deliberate exit reports nothing");
}

#[test]
fn the_interpreter_agrees() {
    let (code, err) = exits("interp", &["--interp"]);
    assert_eq!(code, Some(3), "the oracle lost the code the program named");
    assert_eq!(err, "", "a deliberate exit reports nothing");
}
