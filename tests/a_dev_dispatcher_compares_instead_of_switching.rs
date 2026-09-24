//! A dev module's dispatchers ask their cases with compares, not a switch.
//!
//! A group whose arms discriminate on int literals, or on a value's tag,
//! compiles to a branch on that value. A release module writes it as a
//! `switch`, which its optimiser turns into a jump table. clang's fast
//! instruction selector at -O0 does not lower a switch at all, so the whole
//! block went to the slow selector. A dev module compares each case in turn,
//! which took the codegen corpus's dev row from 360,741,989 to 359,341,981.
//!
//! What a user can see is what the program prints, on both tiers, and the
//! module `kanso build` leaves beside the binary.
//!
//! Watched red with the dev tier writing the switch: the dev module switched.

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

/// The switches `module` writes outside the thunk dispatcher, whose arms go
/// to the slow selector whatever branches to them.
fn switches(module: &str) -> usize {
    let mut n = 0;
    let mut in_thunks = false;
    for line in module.lines() {
        if line.starts_with("define") {
            in_thunks = line.contains("@d_thunk_eval(");
        }
        if !in_thunks && line.trim_start().starts_with("switch i64") {
            n += 1;
        }
    }
    n
}

#[test]
fn the_dev_module_compares_and_the_release_module_switches() {
    let dir = std::env::temp_dir().join(format!("kanso_dev_compares_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(
        dir.join("words.kso"),
        "fn word 1\n  \"one\"\n\nfn word 2\n  \"two\"\n\nfn word 3\n  \"three\"\n\nfn word _\n  \"many\"\n\n\
         pub fn said n\n  \"{word n} {word (n + 1)} {word (n + 2)} {word (n + 3)}\"\n",
    )
    .expect("the library writes");
    std::fs::write(dir.join("main.kso"), "import \"./words\"\n\nprint (words/said 1)\n")
        .expect("the program writes");
    let (dev, dev_out) = build(&dir, false);
    let (release, release_out) = build(&dir, true);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(dev_out, "one two three many\n");
    assert_eq!(release_out, dev_out);
    assert_eq!(switches(&dev), 0, "the dev module switches");
    assert!(switches(&release) > 0, "the release module lost its switch");
}
