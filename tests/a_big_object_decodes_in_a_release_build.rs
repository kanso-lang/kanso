//! A release build decodes a JSON object of any size in constant stack.
//!
//! The decoder reads an object's keys in a loop of tail calls, one of which
//! enters `obj_key_end`, an arm of nine argument words. A release build used
//! to take `tailcc` away from every arm wider than eight words, the arm64
//! register file, and turn each call into it into an ordinary one. That left
//! a frame on the stack for every key, and an object of 300,000 keys
//! overflowed it. x86-64 has no miscompile to work around and keeps the tail
//! call for an arm of nine words, so the loop runs in one frame. arm64 keeps
//! the narrower limit, which is why this spec runs on x86-64 alone.
//!
//! Watched red with `TAILCC_WIDEST` at 8 on x86-64: the binary died on
//! SIGSEGV and printed nothing.

#[cfg(target_arch = "x86_64")]
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
