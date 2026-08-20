// A field write in a play constant's block reached the statement grouper,
// whose set-arm was an unreachable! — sixteen fuzz children, one site.
// The shape is check-only-reachable (staged runs resolve names first), so
// the pin lives here. Watched red as a panic before the parser learned the
// refusal a fn body already gives.
use std::process::Command;

#[test]
fn a_set_in_a_play_body_is_refused_not_panicked() {
    let dir = std::env::temp_dir().join("kanso-set-play");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("set_play.kso");
    std::fs::write(&file, "pub play =\n  a.peers = []\n  admin\n  name\n").expect("fixture writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("kanso runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("writes a field, and only a `build` block may do that"),
        "expected the build refusal, got: {err}"
    );
    assert!(!err.contains("panicked"), "the compiler must diagnose, not panic: {err}");
}
