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

/// Run a sample the way a user runs a library: through a generated entry
/// file that imports it and names its exported lambda. RULED: `play` is an
/// ordinary exported name the language knows nothing about, so the
/// convention of running one lives here, in the harness, not in the
/// compiler. A sample with no `pub play` is already an entry file and runs
/// directly.
///
/// The sample's whole directory is staged, because samples read fixture
/// files and directories that sit beside them.
fn run_kanso_as_library(program: &Path, extra: &[&str], envs: &[(&str, &str)]) -> Output {
    let text = std::fs::read_to_string(program).expect("the sample reads");
    if !text.contains("\npub play") && !text.starts_with("pub play") {
        return run_kanso_env(program, extra, envs);
    }

    let name =
        program.file_stem().and_then(|s| s.to_str()).expect("kso files have names").to_string();
    let source = program.parent().expect("samples live in a directory");
    let stage = std::env::temp_dir().join(format!(
        "kanso-entry-{}-{name}",
        source.file_name().and_then(|s| s.to_str()).unwrap_or("dir")
    ));
    let _ = std::fs::remove_dir_all(&stage);
    stage_tree(source, &stage);
    let entry = stage.join(format!("run_{name}.kso"));
    std::fs::write(&entry, format!("import \"{name}\"\n\n{name}/play\n"))
        .expect("the entry file writes");

    let out = run_kanso_env(&entry, extra, envs);
    let _ = std::fs::remove_dir_all(&stage);
    out
}

fn stage_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the staging directory is made");
    for entry in std::fs::read_dir(from).expect("the corpus is readable") {
        let path = entry.expect("directory entry").path();
        let landing = to.join(path.file_name().expect("entries have names"));
        if path.is_dir() {
            stage_tree(&path, &landing);
        } else {
            std::fs::copy(&path, &landing).expect("the file copies");
        }
    }
}

/// A file holding declarations beside bare statements is a play file — the
/// relaxed single-file form. The verb follows the file's shape, because a
/// reader running one of these by hand has to pick the same door.
fn play_file(program: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(program) else { return false };
    if text.starts_with("pub play") || text.contains("\npub play") {
        return false;
    }
    let tops: Vec<&str> = text.lines().filter(|l| !l.is_empty() && !l.starts_with(' ')).collect();
    let declares = tops.iter().any(|l| l.starts_with("fn ") || l.starts_with("type "));
    // A binding is not a statement: a file of `fn`s and `test_` bindings is a
    // test file, and it has a verb of its own.
    let states = tops.iter().any(|l| {
        let plain = !l.starts_with("import ")
            && !l.starts_with("fn ")
            && !l.starts_with("type ")
            && !l.starts_with('#');
        let binds = l.split_once(" = ").is_some_and(|(head, _)| !head.contains(' '));
        plain && !binds
    });
    declares && states
}

fn run_kanso_env(program: &Path, extra: &[&str], envs: &[(&str, &str)]) -> Output {
    let verb = match play_file(program) {
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

/// Every example is a little program a reader runs by hand, so the harness
/// runs it the way they would rather than through a generated entry.
#[test]
fn examples_print_their_golden_stdout() {
    for program in kso_files(&manifest_dir().join("examples")) {
        let golden = manifest_dir()
            .join("tests/golden/examples")
            .join(program.file_name().expect("kso files have names"));
        let expected_out = expected(&golden, "stdout");
        let output = run_kanso_as_library(&program, &[], &[]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected_out,
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

/// Every fixture runs as a LIBRARY through the harness-generated entry, per
/// the play migration — this corpus no longer touches the compiler's entry
/// synthesis. Most diagnostics are byte-identical either way; the 23 that
/// gain the loader's ` (module …)` suffix carry a second golden for this
/// path, the same shape as the micro corpus's `.imported.out`.
#[test]
fn error_corpus_reports_each_golden_diagnostic() {
    for program in kso_files(&manifest_dir().join("tests/golden/errors")) {
        let imported_golden = program.with_extension("imported.stderr");
        let expected_err = if imported_golden.exists() {
            std::fs::read_to_string(&imported_golden).expect("the imported golden reads")
        } else {
            expected(&program, "stderr")
        };
        let output = run_kanso_as_library(&program, &[], &[]);

        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected_err,
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
    // bench/cost_golden.txt but per-program. Every fixture runs as a LIBRARY
    // through the harness-generated entry, and the counters must match the
    // direct-run goldens byte for byte — an imported program pays the same
    // allocation shape as a direct one. The one exception is qualified
    // record rendering, whose longer type names cost string bytes; those
    // fixtures carry `.imported.*` goldens. The lazy fragment will extend
    // these with engine-shared semantic counters (forces, evaluations,
    // cells live at exit) asserted on both engines.
    for program in kso_files(&manifest_dir().join("tests/golden/mem")) {
        let imported_out = program.with_extension("imported.stdout");
        let expected_out = if imported_out.exists() {
            std::fs::read_to_string(&imported_out).expect("the imported golden reads")
        } else {
            expected(&program, "stdout")
        };
        let imported_mem = program.with_extension("imported.mem");
        let expected_mem = if imported_mem.exists() {
            std::fs::read_to_string(&imported_mem).expect("the imported golden reads")
        } else {
            expected(&program, "mem")
        };
        let output = run_kanso_as_library(&program, &[], &[("KANSO_COUNTERS", "1")]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected_out,
            "stdout mismatch for {program:?}"
        );
        // A new counter is additive and moves every file in this vein at
        // once; regenerating by hand is how one gets missed.
        if std::env::var_os("KANSO_REGEN_MEM_GOLDEN").is_some() {
            let at =
                if imported_mem.exists() { imported_mem } else { program.with_extension("mem") };
            std::fs::write(&at, &output.stderr).expect("the mem golden writes");
            continue;
        }
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected_mem,
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
    let strict = run_kanso_as_library(&program, &["--strict"], &[("KANSO_COUNTERS", "1")]);

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
        let imported_golden = program.with_extension("imported.stderr");
        let expected_err = if imported_golden.exists() {
            std::fs::read_to_string(&imported_golden).expect("the imported golden reads")
        } else {
            expected(&program, "stderr")
        };
        // Both engines must report the endpoint violation identically: native
        // (the compiled binary) and the interpreter oracle. Every fixture
        // runs as a LIBRARY through the harness-generated entry; the 13 whose
        // messages render a value or a trace carry `.imported.stderr`
        // goldens, because names spell QUALIFIED through an import.
        for extra in [&[][..], &["--interp"][..]] {
            let output = run_kanso_as_library(&program, extra, &[]);

            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                expected_err,
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
        let output = run_kanso_as_library(&file, extra, &[("KANSO_ENV_PROBE", "supplied")]);

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
        let output = run_kanso_as_library(&program, extra, &[("KANSO_NOW", "1700000000000")]);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "1700000000000\n",
            "engine {extra:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// One construct per program, so a failure names the construct rather than
/// telling you that something in a fifteen-feature example broke. Both engines
/// run each one and must agree: the differential law at its smallest useful
/// size.
///
/// Every sample runs as a LIBRARY, through the harness-generated entry that
/// imports it and names its exported lambda — this corpus no longer touches
/// the compiler's entry synthesis, which is the migration's point. The
/// library path is also where four separate qualification bugs lived, none of
/// which could fail a corpus that only ran files. A sample with no `pub play`
/// is an entry file already and runs directly through the runner's fallback.
#[test]
fn micro_corpus_agrees_across_engines() {
    let source = manifest_dir().join("tests/golden/micro");

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
        covered += 1;

        for extra in [&[][..], &["--interp"][..]] {
            let output = run_kanso_as_library(&program, extra, &[]);

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

    // A loop that silently skipped everything would make every assertion
    // above vacuous, and the test would go on passing.
    assert!(covered > 53, "only {covered} samples were reached through an import");
}

/// The same corpus, release-built and executed.
///
/// `--release` is `-O3 -flto` where the default build is `-O0` against a
/// cached runtime, and the two disagreed for months: a tailcc arm whose
/// arguments overflow the eight argument registers was miscompiled under the
/// optimizer, so any program importing std/regexp segfaulted with no
/// diagnostic. Nothing caught it because nothing here release-built a program
/// and ran it — CI release-builds only the benchmarks, none of which import
/// regexp, and the IR verifier builds this corpus without the flag.
///
/// The profile is not an engine, so this asserts against the same goldens the
/// other runs do: what a program prints cannot depend on how hard it was
/// optimized.
#[test]
fn micro_corpus_survives_a_release_build() {
    let source = manifest_dir().join("tests/golden/micro");

    let mut covered = 0;
    for program in kso_files(&source) {
        let name =
            program.file_stem().and_then(|s| s.to_str()).expect("kso files have names").to_string();
        let text = std::fs::read_to_string(&program).expect("the sample reads");
        if !text.contains("\npub play") {
            continue;
        }
        let imported_golden = program.with_extension("imported.out");
        let expected_out = match imported_golden.exists() {
            true => std::fs::read_to_string(&imported_golden).expect("the imported golden reads"),
            false => expected(&program, "out"),
        };
        covered += 1;

        let stage = std::env::temp_dir().join(format!("kanso-release-{name}"));
        let _ = std::fs::remove_dir_all(&stage);
        stage_tree(program.parent().expect("samples live in a directory"), &stage);
        let _ = std::fs::remove_dir_all(stage.join(&name));
        let entry = format!("run_{name}");
        std::fs::write(
            stage.join(format!("{entry}.kso")),
            format!("import \"{name}\"\n\n{name}/play\n"),
        )
        .expect("the entry file writes");

        let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("build")
            .arg(format!("{entry}.kso"))
            .arg("--release")
            .current_dir(&stage)
            .output()
            .expect("kanso runs");
        assert!(
            built.status.success(),
            "{name} does not release-build: {}",
            String::from_utf8_lossy(&built.stderr)
        );

        let ran = Command::new(stage.join(&entry)).current_dir(&stage).output().expect("it runs");
        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            expected_out,
            "{name} answers differently when release-built"
        );
        assert_eq!(ran.status.code(), Some(0), "{name} exits 0 when release-built");
        let _ = std::fs::remove_dir_all(&stage);
    }

    assert!(covered > 53, "only {covered} samples were release-built");
}
