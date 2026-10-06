//! An integer written as text past int64 is read whole by every engine.
//!
//! The interpreter reads `text/to_int "12345678901234567890"` as the whole
//! integer, because a kanso int is arbitrary precision. A compiled build
//! answered an err value, `"..." overflows this engine's integers`, which
//! std/json turned into `parse_failure "invalid number"` for a well-formed
//! document. Found by generated programs on 2026-09-28; compiled builds then
//! refused the number outright, and since the 2026-10-07 gavel they read it
//! into a bignum.
//!
//! The text is built at run time, so no engine can fold the call away.

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

fn staged(tag: &str, call: &str, literal: &str) -> PathBuf {
    let stage = std::env::temp_dir().join(format!("kanso-int-past-int64-{tag}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("defs")).expect("a staging directory");
    let json = match call.starts_with("json/") {
        true => "import \"std/json\"\n",
        false => "",
    };
    std::fs::write(
        stage.join("defs/defs.kso"),
        format!(
            "{json}import \"std/os\"\nimport \"std/text\"\n\n\
             fn shown z\n  p = text/slice \"x\" 1 z\n  \
             s = text/join [\"{literal}\" p] \"\"\n  \
             \"{{{call} s}}\"\n\n\
             pub play = os/args .> (a -> print (shown (length a)))\n"
        ),
    )
    .expect("the module writes");
    std::fs::write(stage.join("main.kso"), "import \"./defs\"\n\ndefs/play\n")
        .expect("the entry writes");
    stage
}

fn interpreted(tag: &str, call: &str, literal: &str) -> String {
    let stage = staged(&format!("i-{tag}"), call, literal);
    let out = Command::new(kanso())
        .args(["run", "main.kso", "--interp"])
        .current_dir(&stage)
        .output()
        .expect("kanso runs");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

fn compiled(tag: &str, call: &str, literal: &str) -> String {
    let stage = staged(&format!("n-{tag}"), call, literal);
    let built = Command::new(kanso())
        .args(["build", "main.kso"])
        .current_dir(&stage)
        .output()
        .expect("kanso builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let out = Command::new(stage.join("main")).current_dir(&stage).output().expect("it runs");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

const PAST: [(&str, &str, &str, &str); 4] = [
    ("int-big", "text/to_int", "12345678901234567890", "12345678901234567890\n"),
    ("int-neg", "text/to_int", "-9223372036854775809", "-9223372036854775809\n"),
    ("json-big", "json/decode", "12345678901234567890", "12345678901234567890\n"),
    ("json-list", "json/decode", "[9223372036854775808, 1]", "[9223372036854775808 1]\n"),
];

#[test]
fn the_interpreter_reads_the_whole_integer() {
    for (tag, call, literal, want) in PAST {
        assert_eq!(interpreted(tag, call, literal), want, "interpreted {call} {literal}");
    }
}

#[test]
fn a_compiled_build_reads_the_whole_integer() {
    for (tag, call, literal, want) in PAST {
        assert_eq!(compiled(tag, call, literal), want, "compiled {call} {literal}");
    }
}

#[test]
fn at_the_ends_of_the_range_the_engines_agree() {
    for (tag, call, literal, want) in [
        ("int-max", "text/to_int", "9223372036854775807", "9223372036854775807\n"),
        ("int-min", "text/to_int", "-9223372036854775808", "-9223372036854775808\n"),
        ("json-max", "json/decode", "[9223372036854775807]", "[9223372036854775807]\n"),
    ] {
        assert_eq!(interpreted(tag, call, literal), want, "interpreted {call} {literal}");
        assert_eq!(compiled(tag, call, literal), want, "compiled {call} {literal}");
    }
}
