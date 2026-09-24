//! An integer literal means the same number however many digits it has.
//!
//! The lexer sums a literal of eighteen digits or fewer into a `u64` where it
//! lies, and hands a longer one to `BigInt`'s parser. Both paths have to give
//! the number that was written, including at the width where the path
//! changes and at `u64`'s own limit. This builds literals either side of
//! both and reads them back through arithmetic, on the interpreter and the
//! native engine.
//!
//! Past 64 bits only the interpreter reads the literal, and the native engine
//! refuses it by name, which the differential law allows.
//!
//! Watched red with the fast path taking twenty digits: the interpreter read
//! the twenty-digit literal wrapped, and the difference came out wrong.

use std::process::Command;

fn run(dir: &std::path::Path, interp: bool) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("play").arg("main.kso").current_dir(dir);
    if interp {
        cmd.arg("--interp");
    }
    cmd.output().expect("kanso runs")
}

fn play(dir: &std::path::Path, interp: bool) -> String {
    let ran = run(dir, interp);
    assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
    String::from_utf8_lossy(&ran.stdout).into_owned()
}

#[test]
fn every_width_reads_back_as_written() {
    let dir = std::env::temp_dir().join(format!("kanso_int_widths_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(
        dir.join("main.kso"),
        "print \"{0} {7} {007}\"\n\
         print \"{999999999999999999 + 1}\"\n\
         print \"{1000000000000000000 - 1}\"\n\
         print \"{9223372036854775807 - 9223372036854775806}\"\n",
    )
    .expect("the program writes");
    let want = "0 7 7\n1000000000000000000\n999999999999999999\n1\n";
    let interp = play(&dir, true);
    let native = play(&dir, false);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(interp, want);
    assert_eq!(native, want);
}

#[test]
fn a_literal_past_64_bits_reads_on_the_interpreter_and_is_refused_natively() {
    let dir = std::env::temp_dir().join(format!("kanso_int_wide_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(
        dir.join("main.kso"),
        "print \"{99999999999999999999 - 99999999999999999998}\"\n",
    )
    .expect("the program writes");
    let interp = play(&dir, true);
    let native = run(&dir, false);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(interp, "1\n");
    assert!(!native.status.success(), "the native engine ran a literal it cannot hold");
    assert!(
        String::from_utf8_lossy(&native.stderr).contains("does not fit this build's 64-bit int"),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
}
