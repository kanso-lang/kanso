//! Who checks the IR kanso writes.
//!
//! Nothing in this project reads the emitted .ll. `kanso build` writes it and
//! hands it to clang, so the only verifier a program ever meets is the one on
//! the machine that built it — and that verdict has already differed by host:
//! a thunk released in a block its creation did not dominate was refused on
//! macOS and compiled and run on linux, from identical source.
//!
//! Two things have to hold for "clang will catch it" to be a defence. The
//! toolchain has to verify at all, and kanso's own output has to pass. This
//! file asserts both, on whatever host it runs, so a host where the first is
//! false says so instead of going quietly green.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn clang_on(path: &std::path::Path) -> std::process::Output {
    Command::new("clang")
        .arg("-c")
        .arg("-o")
        .arg(std::env::temp_dir().join("kanso_ir_check.o"))
        .arg(path)
        .output()
        .expect("clang runs")
}

/// A register used in a block its definition does not reach. This is the exact
/// shape LLVM refused in the thunk-release defect, reduced to five lines, and
/// the message is the one that came back then.
#[test]
fn the_toolchain_refuses_ir_that_does_not_dominate_its_uses() {
    let bad = std::env::temp_dir().join("kanso_does_not_dominate.ll");
    std::fs::write(
        &bad,
        "define i64 @f(i64 %n) {\n\
         entry:\n  %c = icmp eq i64 %n, 0\n  br i1 %c, label %made, label %uses\n\n\
         made:\n  %v = add i64 %n, 1\n  br label %done\n\n\
         uses:\n  %w = add i64 %v, 2\n  br label %done\n\n\
         done:\n  %r = phi i64 [ 0, %made ], [ %w, %uses ]\n  ret i64 %r\n}\n",
    )
    .expect("the fixture writes");

    let refused = clang_on(&bad);
    let said = String::from_utf8_lossy(&refused.stderr).into_owned();

    assert!(
        !refused.status.success() && said.contains("does not dominate all uses"),
        "this host's clang accepted IR that does not dominate its uses, so it \
         is not the verifier this project has been relying on. It said: {said}"
    );
}

/// And the emitter's own output, on a program that reaches the paths the
/// defect lived on: a bound thunk in a function whose arguments can fail.
#[test]
fn the_ir_kanso_writes_passes_that_verifier() {
    let dir = std::env::temp_dir().join("kanso_ir_written");
    std::fs::create_dir_all(&dir).expect("the directory is made");
    let source = dir.join("written.kso");
    std::fs::copy(manifest_dir().join("tests/golden/micro/bound_thunk_across_arms.kso"), &source)
        .expect("the fixture copies");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg("written.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));

    let checked = clang_on(&dir.join("written.ll"));

    assert!(
        checked.status.success(),
        "kanso wrote IR its own toolchain refuses: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
