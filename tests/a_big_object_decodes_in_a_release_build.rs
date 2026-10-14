//! A release build decodes a JSON object of any size in constant stack, and
//! keeps every tail call a jump however wide its arm.
//!
//! The decoder reads an object's keys in a loop of tail calls, one of which
//! enters `obj_key_end`, an arm of nine argument words. A release build used
//! to take `tailcc` away from every arm wider than a limit (twelve words on
//! x86-64 under `preserve_nonecc`) and turn each call into it into an ordinary
//! one. That left a frame on the stack for every hop, and an object of 300,000
//! keys overflowed it at eight. On x86-64 the release build now passes every
//! `musttail` the emitter wrote to clang as written.
//!
//! arm64 keeps every `musttail` too. An arm wider than its eight argument
//! registers is marked so that its backend makes no sibling call in it, which
//! it miscompiles (see `wide_tailcc_keeps_its_jumps` in src/main/tail_calls.rs).
//!
//! Watched red with the limit at 8 on x86-64: the binary died on SIGSEGV and
//! printed nothing.

#[test]
fn an_object_of_300_000_keys_decodes() {
    use std::fmt::Write as _;
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("kanso_big_object_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let mut json = String::from("{");
    for i in 0..300_000 {
        if i > 0 {
            json.push(',');
        }
        let _ = write!(json, "\"k{i}\":{i}");
    }
    json.push('}');
    std::fs::write(dir.join("big.json"), json).expect("the document writes");
    std::fs::write(
        dir.join("main.kso"),
        "import \"std/json\"\nimport \"std/os\"\n\nos/read_file \"big.json\" .> (raw -> print \"keys {length (json/decode raw)}\")\n",
    )
    .expect("the program writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let run = Command::new(dir.join("main")).current_dir(&dir).output().expect("the binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(run.status.success(), "the release binary failed: {:?}", run.status);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "keys 300000\n");
}

/// A loop that crosses into a twelve-word arm runs in one frame too.
///
/// The object above stopped being a witness once the release link dropped
/// `deadargelim` (2026-09-28): with its callers' signatures left whole, the
/// backend turns the narrowed call back into a sibling call, and 300,000 keys
/// decode in one frame even with arms narrowed at eight words. Here a
/// four-parameter function and a seven-parameter one call each other three
/// million times. Both take the cycle's flat twelve-word signature, so narrowing
/// at eight takes the tail call away from both, and the binary dies on SIGSEGV.
#[test]
fn a_cycle_through_a_twelve_word_arm_runs_in_one_frame() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("kanso_wide_cycle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("defs")).expect("a scratch directory");
    std::fs::write(
        dir.join("defs/defs.kso"),
        "import \"std/os\"\n\n\
         fn narrow 0 acc _ _\n  acc\n\n\
         fn narrow n acc s t\n  wide n acc s t \"x\" [n] { \"k\":n }\n\n\
         fn wide n acc s t u xs m\n  \
         k = length s + length t + length u + length xs + length m\n  \
         narrow (n - 1) (acc + k) s t\n\n\
         fn shown z\n  \"{narrow (z + 3000000) 0 \"ab\" \"c\"}\"\n\n\
         pub play = os/args .> (a -> print (shown (length a)))\n",
    )
    .expect("the module writes");
    std::fs::write(dir.join("main.kso"), "import \"./defs\"\n\ndefs/play\n")
        .expect("the entry writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let run = Command::new(dir.join("main")).current_dir(&dir).output().expect("the binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(run.status.success(), "the release binary failed: {:?}", run.status);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "18000000\n");
}

/// A self call through an arm of thirteen words is a jump too.
///
/// `walk` takes six boxed values and an int, thirteen words, one more than
/// `preserve_nonecc` passes in registers, so it keeps `tailcc`. Each hop builds
/// a record on the stack and hands its address to the runtime, which keeps
/// LLVM from turning the call into a sibling call on its own. With the
/// release build's limit at twelve the call lost its `musttail`, and the
/// binary ran out of stack. The regex port's Pike VM, whose position loop had
/// seven parameters, did the same on a 40,000-character line. The interpreter
/// and the dev build ran both.
#[test]
fn a_self_call_through_a_thirteen_word_arm_runs_in_one_frame() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("kanso_wide_self_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("w")).expect("a scratch directory");
    std::fs::write(
        dir.join("w/w.kso"),
        "type pair\n  left\n  right\n\n\
         pub fn walk p q r s t u n\n  \
         return length p + length q if n == 0\n  \
         x = pair p q\n  \
         walk x.right x.left r s t u (n - 1)\n",
    )
    .expect("the module writes");
    std::fs::write(
        dir.join("main.kso"),
        "import \"./w\"\n\nprint \"{w/walk \"a\" \"bb\" \"ccc\" [1] [2 3] [4 5 6] 1000000}\"\n",
    )
    .expect("the entry writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let run = Command::new(dir.join("main")).current_dir(&dir).output().expect("the binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "the release binary failed: {:?}\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "3\n");
}
