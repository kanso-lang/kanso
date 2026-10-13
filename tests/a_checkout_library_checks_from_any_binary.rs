//! `kanso check lib/json` from a source checkout reads the library's own
//! builtins whichever binary runs it. The import loader already finds `std/`
//! in `lib/` beside the working directory when the toolchain's copy is not
//! there; the check that lets a library file name `builtin_*` looked only
//! beside the binary, so a compiler built into a target directory elsewhere
//! (the ratchet links one in from /tmp) refused the library it was checking.

use std::path::Path;
use std::process::Command;

#[test]
fn a_binary_outside_the_checkout_checks_its_library() {
    let dir = std::env::temp_dir()
        .join(format!("kanso_elsewhere_{}", std::process::id()))
        .join("release");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the directory is made");
    let binary = dir.join("kanso");
    std::fs::copy(env!("CARGO_BIN_EXE_kanso"), &binary).expect("the binary copies");
    let run = Command::new(&binary)
        .args(["check", "lib/json"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .env_remove("KANSO_STD")
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(dir.parent().expect("a parent"));
    assert_eq!(String::from_utf8_lossy(&run.stderr), "");
    assert!(run.status.success());
}
