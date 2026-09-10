//! A group that dispatches on a byte read out of a byte string crosses the
//! call boundary as a raw i64, with 256 standing for the `none` a read past the
//! end answers, and the backend switches on that raw value directly.
//!
//! Which leaves one literal to be careful about. `fn kind 256` is an arm no
//! byte can ever reach, and a program may write it: the interpreter runs the
//! generic arm for a `none`, because 256 is not a byte and the arm is simply
//! dead. Fold the sentinel and the literal into one switch and that dead arm
//! catches every read past the end instead.
//!
//! Watched red against the first draft of the raw switch, which had no range
//! guard: native answered "a byte that cannot be" where the interpreter
//! answered "some other byte".

use std::process::Command;

fn ran(source: &str, interp: bool) -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kanso-byte-arm-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let file = dir.join("pkg.kso");
    std::fs::write(&file, source).expect("the program writes");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("play").arg(&file);
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    said
}

/// Two byte arms make the group switch-shaped; the third names a value no byte
/// holds. The third read is past the end of a two-byte string.
const PAST_THE_END: &str = r#"import "std/text"

fn kind 34
  "quote"

fn kind 91
  "bracket"

fn kind 256
  "a byte that cannot be"

fn kind _
  "some other byte"

fn look cs p
  kind cs[p]

bs = text/bytes "[\""

print (look bs 1)
>> print (look bs 2)
>> print (look bs 3)
"#;

#[test]
fn an_arm_naming_256_does_not_catch_the_end_of_the_input() {
    let native = ran(PAST_THE_END, false);
    let oracle = ran(PAST_THE_END, true);

    assert_eq!(
        native, oracle,
        "the engines disagree about an arm no byte can reach:\nnative:\n{native}\noracle:\n{oracle}"
    );
    assert_eq!(
        oracle, "bracket\nquote\nsome other byte\n",
        "a read past the end took an arm that names 256: {oracle}"
    );
}

/// The same group without the impossible literal, which is the shape the raw
/// switch is for. Kept beside it so a change that disabled the fast path
/// everywhere would not look like a pass.
const ORDINARY: &str = r#"import "std/text"

fn kind 34
  "quote"

fn kind 91
  "bracket"

fn kind none
  "the end"

fn kind _
  "some other byte"

fn look cs p
  kind cs[p]

bs = text/bytes "[\"a"

print (look bs 1)
>> print (look bs 3)
>> print (look bs 4)
"#;

#[test]
fn a_byte_group_still_answers_none_at_the_end() {
    let native = ran(ORDINARY, false);
    let oracle = ran(ORDINARY, true);

    assert_eq!(native, oracle, "the engines disagree:\nnative:\n{native}\noracle:\n{oracle}");
    assert_eq!(oracle, "bracket\nsome other byte\nthe end\n", "the arms did not answer: {oracle}");
}
