//! A dev build calls the runtime's helpers; a release build inlines them.
//!
//! The helpers the emitter writes into every module -- tag tests, the fast
//! arms of append and index, the closure-call twins -- were all
//! `alwaysinline`. At `-O0` the always-inliner still honours that and copies
//! each one into every call site, and the instruction selector walks every
//! copy: on the codegen corpus `clang -cc1` read 359,109,516 instructions
//! with the attribute and 316,180,072 without. The dev tier is the one that
//! compiles fast, so its module leaves the helpers to be called, and the
//! release tier, where inlining them is the point, keeps the attribute.
//!
//! What a user can see is the module `kanso build` leaves beside the binary,
//! and what the program prints, which must not change with the tier.
//!
//! Watched red with `emit_ir_dev` asking for inlined helpers: the dev module
//! defined its helpers `alwaysinline` again.

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

fn inlined_helpers(module: &str) -> usize {
    module.lines().filter(|l| l.starts_with("define") && l.contains(" alwaysinline")).count()
}

#[test]
fn only_the_release_module_inlines_its_helpers() {
    let dir = std::env::temp_dir().join(format!("kanso_dev_helpers_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    // A closure call and an append, so a closure twin and the append helpers
    // are in the module.
    std::fs::write(
        dir.join("main.kso"),
        "import \"std/text\"\n\ntwice = (f x -> f (f x))\n\nprint (twice (s -> text/join [s \"!\"] \"\") \"hi\")\n",
    )
    .expect("the program writes");
    let (dev, dev_out) = build(&dir, false);
    let (release, release_out) = build(&dir, true);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(dev_out, "hi!!\n");
    assert_eq!(dev_out, release_out);
    assert_eq!(inlined_helpers(&dev), 0, "the dev module inlines its helpers");
    assert!(inlined_helpers(&release) > 0, "the release module inlines none of its helpers");
}
