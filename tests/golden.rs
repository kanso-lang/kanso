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
    let source = std::fs::read_to_string(program).unwrap_or_default();
    let verb = match source.contains("pub play") {
        true => "play",
        false => "run",
    };
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
