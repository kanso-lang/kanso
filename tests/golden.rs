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
    let source = std::fs::read_to_string(program).unwrap_or_default();
    let verb = match source.contains("pub play") {
        true => "play",
        false => "run",
    };
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg(verb)
        .arg(program.file_name().expect("kso files have names"))
        .args(extra)
        .current_dir(program.parent().expect("programs live in a directory"))
        .output()
        .expect("kanso binary runs")
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
