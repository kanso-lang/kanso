//! A literal's words are written into the module, not read off it.
//!
//! `n - 1` needs the payload of the literal `1`, and the emitter used to read
//! it with `extractvalue %KValue { i64 0, i64 1 }, 1`. That is the same number
//! with an instruction around it, and it is one clang's fast selector at -O0
//! does not lower: the rest of the block went to the slow selector. The
//! codegen corpus had 48 of them, and writing the words instead took the dev
//! row from 372,274,647 instructions to 367,410,698.
//!
//! What a user can see is the module `kanso build` leaves beside the binary,
//! and what the program prints.
//!
//! Watched red with `literal_word` answering nothing: both modules read the
//! literal's payload with `extractvalue` again.

use std::process::Command;

fn build(dir: &std::path::Path, release: bool) -> (String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg("main.kso").current_dir(dir);
    if release {
        cmd.arg("--release");
    }
    let built = cmd.output().expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(dir.join("main")).output().expect("the binary runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    (module, String::from_utf8_lossy(&ran.stdout).into_owned())
}

#[test]
fn neither_tier_reads_a_word_off_a_literal() {
    let dir = std::env::temp_dir().join(format!("kanso_literal_words_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(
        dir.join("count.kso"),
        "fn down 0 acc\n  acc\n\nfn down n acc\n  down (n - 1) (acc + n)\n\npub fn total n\n  down n 0\n",
    )
    .expect("the library writes");
    std::fs::write(dir.join("main.kso"), "import \"./count\"\n\nprint (count/total 10)\n")
        .expect("the program writes");
    for release in [false, true] {
        let (module, out) = build(&dir, release);
        assert_eq!(out, "55\n");
        let read: Vec<&str> =
            module.lines().filter(|l| l.contains("= extractvalue %KValue { ")).collect();
        assert!(read.is_empty(), "release={release}: the module reads a literal's words: {read:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
