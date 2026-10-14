//! What `kanso play` accepts and refuses.
//!
//! A play file is the relaxed form: functions and types defined right
//! beside the statements that use them, no `pub play` ceremony. The verb
//! is built to stay small — it runs and never builds, its imports are the
//! stdlib and nothing else, and nothing can import a play file — so the
//! form cannot leak into real programs.

use std::path::PathBuf;
use std::process::Command;

fn play(name: &str, engine: &[&str]) -> std::process::Output {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play").join(name);
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg(&fixture)
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
        .output()
        .expect("kanso runs")
}

/// The relaxed form runs, and both engines print the same bytes.
#[test]
fn declarations_and_statements_share_the_file() {
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/greeting.stdout");
    let want = std::fs::read_to_string(&golden).expect("the golden reads");
    for engine in [&[][..], &["--interp"][..]] {
        let run = play("greeting.kso", engine);

        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            want,
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.status.success(), "engine {engine:?}");
    }
}

/// A type the file declares is the file's name, even where a builtin shares
/// it. `run` is the builtin std/os wraps, and this file imports nothing; the
/// checker refused `run 1 2 3` with the builtin's count and native code did
/// the same at the build.
#[test]
fn a_type_named_like_a_builtin_is_the_files_own() {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/play/a_type_named_like_a_builtin.stdout");
    let want = std::fs::read_to_string(&golden).expect("the golden reads");
    for engine in [&[][..], &["--interp"][..]] {
        let run = play("a_type_named_like_a_builtin.kso", engine);

        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            want,
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.status.success(), "engine {engine:?}");
    }
}

/// A function the file declares wins over a type an import enrols under the
/// same bare name. std/list exports a record `counting`; the interpreter built
/// one from this file's two-argument `counting` and stopped on its field count,
/// where native code called the function.
#[test]
fn a_function_named_like_an_imported_type_is_the_files_own() {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/play/a_function_named_like_an_imported_type.stdout");
    let want = std::fs::read_to_string(&golden).expect("the golden reads");
    for engine in [&[][..], &["--interp"][..]] {
        let run = play("a_function_named_like_an_imported_type.kso", engine);

        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            want,
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.status.success(), "engine {engine:?}");
    }
}

/// Nothing can import a play file, so `pub` inside one claims an audience
/// that cannot exist.
#[test]
fn pub_is_refused_in_a_play_file() {
    let run = play("no_exports.kso", &[]);

    let err = String::from_utf8_lossy(&run.stderr);
    assert!(err.contains("a play file exports nothing"), "wrong diagnostic: {err}");
    assert_eq!(run.status.code(), Some(2));
}

/// The stdlib and nothing else — the same contract the web playground has.
#[test]
fn a_local_import_is_refused_in_a_play_file() {
    let run = play("no_local_imports.kso", &[]);

    let err = String::from_utf8_lossy(&run.stderr);
    assert!(
        err.contains("a play file imports the stdlib and nothing else"),
        "wrong diagnostic: {err}"
    );
    assert_eq!(run.status.code(), Some(2));
}

/// The verb runs; it does not build. A binary would be the first step of a
/// real program, and real programs use `run` and library files.
#[test]
fn play_files_do_not_build() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/greeting.kso");
    let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(&fixture)
        .output()
        .expect("kanso runs");

    assert_ne!(run.status.code(), Some(0), "build accepted a play file");
}

/// The two verbs must not send a reader in a circle.
///
/// A file holding `pub play` is refused by both: `kanso run` because the
/// module has no entry, `kanso play` because the form takes bare statements.
/// That is correct twice over, and for a while each refusal named the OTHER
/// verb as the fix — so a reader who followed either sentence landed on the
/// other's refusal with nothing else to try.
///
/// The assertion is on what a user reads, not on which site wrote it: neither
/// message may prescribe a verb that answers this same file with an error.
#[test]
fn the_two_verbs_do_not_point_at_each_other() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/wrong_verb.kso");

    let played = play("wrong_verb.kso", &[]);
    let ran = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&fixture)
        .output()
        .expect("kanso runs");

    // Both refuse, which is the premise rather than the finding.
    assert_eq!(played.status.code(), Some(2), "play refuses a pub-play file");
    assert_ne!(ran.status.code(), Some(0), "run refuses a library");

    let play_says = String::from_utf8_lossy(&played.stderr).to_string();
    let run_says = String::from_utf8_lossy(&ran.stderr).to_string();

    assert!(
        !play_says.contains("`kanso run` runs this file"),
        "play prescribes a verb that refuses the same file: {play_says}"
    );
    assert!(
        !run_says.contains("with `kanso play`"),
        "run prescribes a verb that refuses the same file: {run_says}"
    );

    // The route both now name, pinned rather than merely non-circular: a
    // message that stopped saying the wrong thing could stop saying anything.
    assert!(
        play_says.contains(
            "`pub play` is a library's export — import this module from an \
             entry file and name its `play`; `kanso play` takes bare statements"
        ),
        "play does not name the route that works: {play_says}"
    );
    assert!(
        run_says.contains(
            "is a library — nothing to run. it exports `play`: import the \
             module from an entry file, or give the module a main.kso entry"
        ),
        "run does not name the route that works: {run_says}"
    );

    // And the route is real. The same shape the micro corpus uses.
    let stage = std::env::temp_dir().join("kanso-two-verbs");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the staging directory is made");
    std::fs::copy(&fixture, stage.join("wrong_verb.kso")).expect("the library copies");
    let entry = stage.join("run_wrong_verb.kso");
    std::fs::write(&entry, "import \"./wrong_verb\"\n\nwrong_verb/play\n")
        .expect("the entry file writes");

    let imported = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&entry)
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(&stage);

    assert_eq!(
        String::from_utf8_lossy(&imported.stdout),
        "hi\n",
        "the advised route does not run the module: {}",
        String::from_utf8_lossy(&imported.stderr)
    );
    assert!(imported.status.success());
}

/// A file of definitions and statements that imports a neighbour fits
/// neither verb. `kanso run` refuses it because it declares something, and
/// `kanso play` because play files import the stdlib and nothing else. Each
/// refusal used to name the other verb, so a reader following either one
/// was sent straight back (#1893).
///
/// What runs it is a module: the statements in a main.kso, the definitions
/// in a library beside it. Both refusals must say that, and the test builds
/// the module they describe to show the route is real.
#[test]
fn definitions_beside_a_local_import_are_sent_to_a_module() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/beside/sum.kso");
    let played = play("beside/sum.kso", &[]);
    let ran = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&fixture)
        .output()
        .expect("kanso runs");

    assert_eq!(played.status.code(), Some(2), "play refuses a local import");
    assert_ne!(ran.status.code(), Some(0), "run refuses a file that declares");

    let play_says = String::from_utf8_lossy(&played.stderr).to_string();
    let run_says = String::from_utf8_lossy(&ran.stderr).to_string();

    assert!(!play_says.contains("(`kanso run`)"), "play sends the reader to run: {play_says}");
    assert!(!run_says.contains("with `kanso play`"), "run sends the reader to play: {run_says}");
    let route = "move the statements into a main.kso beside it, mark the \
                 definitions they use `pub`, and run the directory with `kanso run`";
    assert!(play_says.contains(route), "play does not name the module: {play_says}");
    assert!(run_says.contains(route), "run does not name the module: {run_says}");

    let stage = std::env::temp_dir().join("kanso-beside");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the staging directory is made");
    std::fs::copy(fixture.with_file_name("calc.kso"), stage.join("calc.kso"))
        .expect("the neighbour copies");
    std::fs::write(stage.join("sum.kso"), "pub fn three _\n  3\n").expect("the library writes");
    std::fs::write(
        stage.join("main.kso"),
        "import \"./calc\"\nimport \"./sum\"\n\nprint \"{calc/twice (sum/three 0)}\"\n",
    )
    .expect("the entry writes");
    let module = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&stage)
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(&stage);

    assert_eq!(
        String::from_utf8_lossy(&module.stdout),
        "6\n",
        "the module the refusals describe does not run: {}",
        String::from_utf8_lossy(&module.stderr)
    );
    assert!(module.status.success());
}

/// Every fixture under tests/golden/play that carries a golden is read
/// against it, byte for byte: a `.stdout` on both engines, a `.stderr` as the
/// refusal a reader sees, a `.run.stderr` as what `kanso run` says of the same
/// file. The `.stderr` files sat unread for weeks, and one went on describing
/// a refusal its fixture no longer met.
#[test]
fn every_play_golden_is_read() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/golden/play");
    let mut fixtures: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the play corpus reads") {
        let path = entry.expect("an entry reads").path();
        match path.is_dir() {
            true => fixtures.extend(
                std::fs::read_dir(&path)
                    .expect("a fixture directory reads")
                    .map(|e| e.expect("an entry reads").path()),
            ),
            false => fixtures.push(path),
        }
    }
    fixtures.retain(|p| p.extension().is_some_and(|e| e == "kso"));
    fixtures.sort();

    let mut read = 0;
    for fixture in &fixtures {
        let shown = fixture.strip_prefix(&root).expect("under the manifest");
        let stdout = fixture.with_extension("stdout");
        let stderr = fixture.with_extension("stderr");
        if stdout.exists() {
            let want = std::fs::read_to_string(&stdout).expect("the golden reads");
            for engine in [&[][..], &["--interp"][..]] {
                let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
                    .current_dir(&root)
                    .arg("play")
                    .arg(shown)
                    .args(engine)
                    .env("KANSO_SEED", "2685821657736338717")
                    .output()
                    .expect("kanso runs");
                assert_eq!(
                    String::from_utf8_lossy(&run.stdout),
                    want,
                    "{} on {engine:?}: {}",
                    shown.display(),
                    String::from_utf8_lossy(&run.stderr)
                );
                assert!(run.status.success(), "{} on {engine:?}", shown.display());
            }
            read += 1;
        }
        if stderr.exists() {
            let want = std::fs::read_to_string(&stderr).expect("the golden reads");
            let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .current_dir(&root)
                .arg("play")
                .arg(shown)
                .output()
                .expect("kanso runs");
            assert_eq!(String::from_utf8_lossy(&run.stderr), want, "{}", shown.display());
            assert_eq!(run.status.code(), Some(2), "{} is refused", shown.display());
            read += 1;
        }
        // What `kanso run` says of the same file, where the two verbs must
        // not send a reader to each other.
        let run_stderr = fixture.with_extension("run.stderr");
        if run_stderr.exists() {
            let want = std::fs::read_to_string(&run_stderr).expect("the golden reads");
            let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .current_dir(&root)
                .arg("run")
                .arg(shown)
                .output()
                .expect("kanso runs");
            assert_eq!(String::from_utf8_lossy(&run.stderr), want, "run {}", shown.display());
            assert_eq!(run.status.code(), Some(2), "run refuses {}", shown.display());
            read += 1;
        }
    }
    // Ten goldens today: four `.stdout`, five `.stderr` and one `.run.stderr`.
    // A walk that found none would pass on an empty corpus.
    assert_eq!(read, 10, "the walk read {read} goldens");
}
