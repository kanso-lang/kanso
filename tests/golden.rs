use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kso_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("missing directory {dir:?}"))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "kso"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no .kso files in {dir:?}");
    files
}

fn run_kanso(program: &Path, extra: &[&str]) -> Output {
    run_kanso_env(program, extra, &[])
}

fn run_kanso_env(program: &Path, extra: &[&str], envs: &[(&str, &str)]) -> Output {
    // `run` is the verb; a file's `pub play` is what `run` finds, not a
    // different way to start it. Choosing a verb from the source also meant
    // the corpus never exercised the path `run` takes for a play file.
    let verb = "run";
    let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
    // goldens pin the dice; a bare run seeds from entropy
    command.env("KANSO_SEED", "2685821657736338717");
    command
        .arg(verb)
        .arg(program.file_name().expect("kso files have names"))
        .args(extra)
        .current_dir(program.parent().expect("programs live in a directory"));
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("kanso binary runs")
}

fn expected(path: &Path, extension: &str) -> String {
    let golden = path.with_extension(extension);
    std::fs::read_to_string(&golden).unwrap_or_else(|_| panic!("missing golden file {golden:?}"))
}

#[test]
fn examples_print_their_golden_stdout() {
    for program in kso_files(&manifest_dir().join("examples")) {
        let golden = manifest_dir()
            .join("tests/golden/examples")
            .join(program.file_name().expect("kso files have names"));
        let output = run_kanso(&program, &[]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected(&golden, "stdout"),
            "stdout mismatch for {program:?}"
        );
        assert!(output.status.success(), "expected success for {program:?}");
    }
}

#[test]
fn plan_prints_the_description_without_executing_it() {
    let program = manifest_dir().join("examples/effects.kso");
    let golden = manifest_dir().join("tests/golden/examples/effects.plan");
    let output = run_kanso(&program, &["--plan"]);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        std::fs::read_to_string(&golden).expect("effects.plan exists"),
        "plan mismatch"
    );
}

#[test]
fn error_corpus_reports_each_golden_diagnostic() {
    for program in kso_files(&manifest_dir().join("tests/golden/errors")) {
        let output = run_kanso(&program, &[]);

        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected(&program, "stderr"),
            "diagnostics mismatch for {program:?}"
        );
        assert_eq!(output.status.code(), Some(2), "compile errors exit 2 for {program:?}");
        assert!(output.stdout.is_empty(), "no stdout on compile error for {program:?}");
    }
}

#[test]
fn mem_corpus_pins_native_allocator_counters() {
    // The memory-goldens vein: each program's .mem file pins the native
    // runtime's deterministic allocator counters, the same ratchet idea as
    // bench/cost_golden.txt but per-program. The lazy fragment will extend
    // these with engine-shared semantic counters (forces, evaluations,
    // cells live at exit) asserted on both engines.
    for program in kso_files(&manifest_dir().join("tests/golden/mem")) {
        let output = run_kanso_env(&program, &[], &[("KANSO_COUNTERS", "1")]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected(&program, "stdout"),
            "stdout mismatch for {program:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected(&program, "mem"),
            "allocator counters drifted for {program:?}"
        );
        assert!(output.status.success(), "expected success for {program:?}");
    }
}

#[test]
fn strict_mode_thunks_nothing_with_identical_output() {
    // The worst-case measurement mode: --strict compiles every binding
    // eager. Output must match the lazy build; the counters prove no cell
    // was ever created.
    let program = manifest_dir().join("tests/golden/mem/skip_unused.kso");
    let strict = run_kanso_env(&program, &["--strict"], &[("KANSO_COUNTERS", "1")]);

    assert_eq!(
        String::from_utf8_lossy(&strict.stdout),
        expected(&program, "stdout"),
        "strict output diverged"
    );
    assert!(
        String::from_utf8_lossy(&strict.stderr).contains("thunk_allocs=0\n"),
        "strict mode still allocated thunks"
    );
}

/// One construct per program, so a failure names the construct rather than
/// telling you that something in a fifteen-feature example broke. Both engines
/// run each one and must agree: these are the differential law at its smallest
/// useful size.
#[test]
fn micro_corpus_agrees_across_engines() {
    for program in kso_files(&manifest_dir().join("tests/golden/micro")) {
        for extra in [&[][..], &["--interp"][..]] {
            let output = run_kanso(&program, extra);

            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                expected(&program, "out"),
                "stdout mismatch for {program:?} (extra {extra:?})"
            );
            assert_eq!(output.status.code(), Some(0), "{program:?} (extra {extra:?}) exits 0");
        }
    }
}

#[test]
fn runtime_corpus_reports_endpoint_violations() {
    for program in kso_files(&manifest_dir().join("tests/golden/runtime")) {
        // Both engines must report the endpoint violation identically: native
        // (the compiled binary) and the interpreter oracle.
        for extra in [&[][..], &["--interp"][..]] {
            let output = run_kanso(&program, extra);

            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                expected(&program, "stderr"),
                "diagnostics mismatch for {program:?} (extra {extra:?})"
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "endpoint violations exit 1 for {program:?} (extra {extra:?})"
            );
        }
    }
}

/// A set variable is machine-dependent, so it is asserted here with the
/// environment controlled rather than pinned in the corpus. The unset case
/// reads as `none` on every machine and lives in the micro corpus.
#[test]
fn a_set_environment_variable_reads_through() {
    let program = manifest_dir().join("tests/golden/micro/env_unset_is_none.kso");
    let source = std::fs::read_to_string(&program).expect("the program reads");
    let named = source.replace("KANSO_DEFINITELY_UNSET_XYZ", "KANSO_ENV_PROBE");
    let staged = std::env::temp_dir().join("kanso-env-probe");
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).expect("temp work dir");
    let file = staged.join("env_probe.kso");
    std::fs::write(&file, named).expect("program writes");

    for extra in [&[][..], &["--interp"][..]] {
        let output = run_kanso_env(&file, extra, &[("KANSO_ENV_PROBE", "supplied")]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "unset reads as supplied\n",
            "engine {extra:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// A timestamp cannot live in the corpus, so the clock is asserted with
/// KANSO_NOW pinned — the same instrument KANSO_SEED is for the dice, and the
/// reason a run that timestamps can be replayed at all. Both engines.
#[test]
fn a_pinned_clock_reads_the_same_in_both_engines() {
    let staged = std::env::temp_dir().join("kanso-clock-probe");
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).expect("temp work dir");
    let program = staged.join("clock_probe.kso");
    std::fs::write(
        &program,
        "import \"std/time\"\n\npub play = time/now . show\n\npub fn show t\n  print \"{t}\"\n",
    )
    .expect("program writes");

    for extra in [&[][..], &["--interp"][..]] {
        let output = run_kanso_env(&program, extra, &[("KANSO_NOW", "1700000000000")]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "1700000000000\n",
            "engine {extra:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Copy a tree, so a sample that reads a directory beside it still finds one.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the staging directory is made");
    for entry in std::fs::read_dir(from).expect("the corpus is readable") {
        let path = entry.expect("directory entry").path();
        let landing = to.join(path.file_name().expect("entries have names"));
        if path.is_dir() {
            copy_tree(&path, &landing);
        } else {
            std::fs::copy(&path, &landing).expect("the file copies");
        }
    }
}

/// Every micro sample run a second way: as a LIBRARY, reached through a
/// generated entry file that imports it and names its exported lambda.
///
/// A sample run directly is one module, and a whole layer of the compiler —
/// qualification, export enrollment, ambient groups — never runs at all. That
/// layer had four separate bugs in it, each of which made a construct that
/// works in a file stop working the moment somebody put it in a library, and
/// none of them could fail a corpus that only ever ran files. This is the
/// cheapest way to run the same programs through the other path.
///
/// Samples with no `pub play` are entry files already — a bare statement is
/// how they start, so there is nothing to import and nothing to name.
#[test]
fn micro_corpus_agrees_when_it_is_imported() {
    let source = manifest_dir().join("tests/golden/micro");
    let stage = std::env::temp_dir().join("kanso-micro-imported");
    let _ = std::fs::remove_dir_all(&stage);
    copy_tree(&source, &stage);

    let mut covered = 0;
    for program in kso_files(&source) {
        let name =
            program.file_stem().and_then(|s| s.to_str()).expect("kso files have names").to_string();
        let text = std::fs::read_to_string(&program).expect("the sample reads");
        if !text.contains("\npub play") {
            continue;
        }

        // RULED: an imported record prints its QUALIFIED type name, so a
        // sample that prints records legitimately answers differently as a
        // library — `sample/point 3 4` where the direct run says `point 3 4`.
        // Those samples carry a second golden for this path; everything else
        // must match its ordinary one byte for byte.
        let imported_golden = program.with_extension("imported.out");
        let expected_out = if imported_golden.exists() {
            std::fs::read_to_string(&imported_golden).expect("the imported golden reads")
        } else {
            expected(&program, "out")
        };

        let entry = stage.join(format!("run_{name}.kso"));
        std::fs::write(&entry, format!("import \"{name}\"\n\n{name}/play\n"))
            .expect("the entry file writes");
        covered += 1;

        for extra in [&[][..], &["--interp"][..]] {
            let output = run_kanso(&entry, extra);

            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                expected_out,
                "{name} answers differently as a library (extra {extra:?})"
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "{name} as a library exits 0 (extra {extra:?})"
            );
        }
    }

    // A staging step that silently copied nothing would make every assertion
    // above vacuous, and the test would go on passing.
    assert!(covered > 53, "only {covered} samples were reached through an import");
    let _ = std::fs::remove_dir_all(&stage);
}
