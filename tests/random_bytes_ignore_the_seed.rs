// `KANSO_SEED` replays `math/random` exactly, which is right for a game and
// wrong for a secret, so `os/random_bytes` ignores it (#2298). The micro corpus
// pins the length and the kind of value on every engine; what it cannot pin is
// that the bytes change, because a golden holds one answer.
//
// So each engine runs one program twice under the same seed. The dice must
// come out the same both times, which shows the seed was in force, and the
// sixteen bytes must not. Two equal draws of sixteen secure bytes would happen
// once in 2^128 runs.
use std::process::Command;

fn twice(name: &str, engine: &[&str]) -> [(String, String); 2] {
    let dir =
        std::env::temp_dir().join(format!("kanso-random-bytes-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the directory is made");
    let file = dir.join("main.kso");
    let program = "import \"std/io\"\nimport \"std/math\"\nimport \"std/os\"\n\n\
                   math/random 1000000 .> (n -> io/write \"{n}\\n\")\n  \
                   .> (_ -> os/random_bytes 16) .> (b -> io/write \"{b}\\n\")\n";
    std::fs::write(&file, program).expect("the fixture writes");
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&file)
            .args(engine)
            .env("KANSO_SEED", "7")
            .output()
            .expect("kanso runs");
        assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
        let text = String::from_utf8(out.stdout).expect("the output is text");
        let (dice, bytes) = text.split_once('\n').expect("two lines");
        (dice.to_string(), bytes.to_string())
    };
    [run(), run()]
}

fn holds(name: &str, engine: &[&str]) {
    let [first, second] = twice(name, engine);
    assert_eq!(first.0, second.0, "KANSO_SEED did not replay the dice, so the run proves nothing");
    assert_eq!(first.1.matches(' ').count(), 15, "sixteen bytes were asked for: {}", first.1);
    assert_ne!(first.1, second.1, "the seed replayed the secure bytes");
}

#[test]
fn the_native_engine_ignores_the_seed() {
    holds("native", &[]);
}

#[test]
fn the_interpreter_ignores_the_seed() {
    holds("interp", &["--interp"]);
}
