//! `math/round` of a finite float answers the integer it rounds to, however
//! large, because a kanso int is arbitrary precision.
//!
//! The interpreter cast the rounded float with `as i64`, which saturates, so
//! 1e30 rounded to 9223372036854775807. The compiled engines called
//! `llround`, which answers LLONG_MIN for anything it cannot hold, so the same
//! call rounded to -9223372036854775808 there. Found by generated programs on
//! 2026-09-28, printing round of a large parsed float on each engine.
//!
//! The interpreter then answered the exact integer and a compiled build
//! refused anything past int64. Since the 2026-10-07 gavel a compiled build
//! answers the exact integer too, as a bignum, and the engines agree to the
//! byte on both sides of the edge. NaN and the infinities round to `none` on
//! every engine (ruled 2026-09-29); until then they answered 0 and the ends
//! of int64.
//!
//! The float is parsed from text built at run time, so no engine can fold
//! the call away before it runs.

use std::path::PathBuf;
use std::process::Command;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

fn staged(tag: &str, literal: &str) -> PathBuf {
    let stage = std::env::temp_dir().join(format!("kanso-round-past-int64-{tag}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("defs")).expect("a staging directory");
    std::fs::write(
        stage.join("defs/defs.kso"),
        format!(
            "import \"std/math\"\nimport \"std/os\"\nimport \"std/text\"\n\n\
             fn shown z\n  p = text/slice \"x\" 1 z\n  \
             f = text/to_float (text/join [\"{literal}\" p] \"\")\n  \
             \"{{math/round f}}\"\n\n\
             pub play = os/args .> (a -> print (shown (length a)))\n"
        ),
    )
    .expect("the module writes");
    std::fs::write(stage.join("main.kso"), "import \"./defs\"\n\ndefs/play\n")
        .expect("the entry writes");
    stage
}

fn interpreted(tag: &str, literal: &str) -> String {
    let stage = staged(&format!("i-{tag}"), literal);
    let out = Command::new(kanso())
        .args(["run", "main.kso", "--interp"])
        .current_dir(&stage)
        .output()
        .expect("kanso runs");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

fn compiled(tag: &str, literal: &str) -> String {
    let stage = staged(&format!("n-{tag}"), literal);
    let built = Command::new(kanso())
        .args(["build", "main.kso"])
        .current_dir(&stage)
        .output()
        .expect("kanso builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let out = Command::new(stage.join("main")).current_dir(&stage).output().expect("it runs");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn the_interpreter_rounds_past_int64_exactly() {
    assert_eq!(interpreted("big", "1e30"), "1000000000000000019884624838656\n");
    assert_eq!(interpreted("neg", "-9.3e18"), "-9300000000000000000\n");
}

#[test]
fn a_compiled_build_rounds_past_int64_exactly() {
    assert_eq!(compiled("big", "1e30"), "1000000000000000019884624838656\n");
    assert_eq!(compiled("neg", "-9.3e18"), "-9300000000000000000\n");
}

#[test]
fn inside_the_range_and_past_it_the_engines_agree() {
    for (tag, literal, want) in [
        ("edge", "9.2e18", "9200000000000000000\n"),
        ("half", "-2.5", "-3\n"),
        ("inf", "inf", "<none>\n"),
        ("ninf", "-inf", "<none>\n"),
        ("nan", "nan", "<none>\n"),
    ] {
        assert_eq!(interpreted(tag, literal), want, "interpreted round of {literal}");
        assert_eq!(compiled(tag, literal), want, "compiled round of {literal}");
    }
}
