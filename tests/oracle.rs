//! The interpreter is the semantics oracle: every golden case also runs
//! through the library interpreter and must match the same expectations the
//! native `kanso run` path is held to, byte for byte.

use kanso::eval::{render, trace_lines, Executor, Interp, Value};
use std::path::{Path, PathBuf};

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

/// A case reached through the entry that imports it prints qualified type
/// names and names its module in traces, so the cases that show either carry
/// an `.imported.*` golden beside the direct-run one.
fn expected(path: &Path, extension: &str) -> String {
    let imported = path.with_extension(format!("imported.{extension}"));
    if imported.exists() {
        return std::fs::read_to_string(&imported).expect("the imported golden reads");
    }
    let golden = path.with_extension(extension);
    std::fs::read_to_string(&golden).unwrap_or_else(|_| panic!("missing golden file {golden:?}"))
}

/// Captures raw printed lines the way the native binary writes stdout, and
/// reads the real filesystem so file-driven cases behave as `RealExecutor`.
struct CollectExecutor {
    program_args: Vec<String>,
    stdout: String,
    stderr: String,
    rng: kanso::eval::Rng,
}

impl Executor for CollectExecutor {
    fn print(&mut self, text: &str) {
        self.stdout.push_str(text);
        self.stdout.push('\n');
    }

    fn write(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    /// The oracle captures what a shell would: stderr after stdout, so a
    /// program writing to both compares against the other engines as-is.
    fn write_err(&mut self, text: &str) {
        self.stderr.push_str(text);
    }

    fn env(&mut self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn exists(&mut self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn is_dir(&mut self, path: &str) -> bool {
        std::path::Path::new(path).is_dir()
    }

    fn now(&mut self) -> i64 {
        kanso::eval::pinned_now().unwrap_or(0)
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let entries = std::fs::read_dir(path)
            .map_err(|_| format!("cannot list {path}: no such directory or unreadable"))?;
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    fn random(&mut self, n: u64) -> u64 {
        self.rng.below(n)
    }

    fn args(&mut self) -> Vec<String> {
        self.program_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Err("the oracle supplies no stdin".to_string())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        kanso::eval::read_file_text(path)
    }

    /// The same process the other engines start. The corpus only reaches for
    /// POSIX binaries every machine running this suite already has, so the
    /// comparison stays a comparison of engines rather than of installs.
    fn run(&mut self, cmd: &str, args: &[String]) -> Result<(i64, String, String), String> {
        let done = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|_| format!("cannot start {cmd}"))?;
        let status = done.status.code().map_or(128, i64::from);
        Ok((
            status,
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        ))
    }

    fn make_dir(&mut self, path: &str) -> Result<(), String> {
        Err(format!("the oracle does not touch the filesystem: cannot make {path}"))
    }

    fn write_file(&mut self, path: &str, _content: &str) -> Result<(), String> {
        Err(format!("the oracle does not write files: cannot write {path}"))
    }
}

struct Evaluation {
    status: i32,
    stderr: String,
    stdout: String,
    thunk_stats: String,
}

/// Mirrors `kanso_run` in src/wasm.rs — the canonical interpreter execution
/// shape — with printed text and diagnostics kept on separate streams so
/// each can be asserted against its golden file.
fn evaluate(program: &kanso::ast::Program, program_args: Vec<String>) -> Evaluation {
    // The interpreter's Rust stack grows with program recursion depth; pin a
    // deep stack explicitly instead of leaning on the test thread's margin.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(scope, || evaluate_on_stack(program, program_args))
            .expect("spawns")
            .join()
            .expect("evaluation thread completes")
    })
}

fn evaluate_on_stack(program: &kanso::ast::Program, program_args: Vec<String>) -> Evaluation {
    let interp = Interp::new(program);
    let value = match interp.run_main() {
        Ok(value) => value,
        Err(runtime) => {
            return Evaluation {
                status: 1,
                stderr: format!("error[runtime]: {}\n", runtime.message),
                stdout: String::new(),
                thunk_stats: interp.thunk_stats.render(),
            }
        }
    };
    // the corpus goldens pin the dice: same fixed seed as book_check
    let mut executor = CollectExecutor {
        program_args,
        rng: kanso::eval::Rng::from_seed(2_685_821_657_736_338_717),
        stdout: String::new(),
        stderr: String::new(),
    };
    let (reached, outcome) = match value {
        Value::Desc(desc) => ("the executor", interp.execute(&desc, &mut executor)),
        other => ("the entry", Ok(other)),
    };
    let thunk_stats = interp.thunk_stats.render();
    // what the program wrote to stderr precedes whatever the endpoint reports,
    // which is the order the native binary emits them in
    let wrote = executor.stderr;
    match outcome {
        Ok(Value::ErrV(info)) => Evaluation {
            status: 1,
            stderr: format!(
                "{wrote}error[endpoint]: unhandled err reached {reached}: {}\n{}",
                render(&info.reason, true),
                trace_lines(&info)
            ),
            stdout: executor.stdout,
            thunk_stats,
        },
        Ok(Value::NoneV) if reached == "the entry" => Evaluation {
            status: 1,
            stderr: format!("{wrote}error[endpoint]: unhandled none reached the entry\n"),
            stdout: executor.stdout,
            thunk_stats,
        },
        Ok(_) => Evaluation { status: 0, stderr: wrote, stdout: executor.stdout, thunk_stats },
        Err(runtime) => Evaluation {
            status: 1,
            stderr: format!("error[runtime]: {}\n", runtime.message),
            stdout: executor.stdout,
            thunk_stats,
        },
    }
}

#[test]
fn mem_corpus_interp_matches_the_semantic_counters() {
    // Evaluation counts are semantics: the .mem goldens' thunk_* lines are
    // produced by the native engine and asserted here against the interp,
    // byte for byte — the differential lattice extended to memory facts.
    for program in kso_files(&manifest_dir().join("tests/golden/mem")) {
        let compiled = compile_case(&program);
        let run = evaluate(&compiled, Vec::new());
        // allocs/forces/evals are evaluation semantics, engine-shared;
        // frees/escaped/live_exit are allocator behavior, native-only.
        let semantic: String = expected(&program, "mem")
            .lines()
            .filter(|line| {
                line.starts_with("thunk_allocs")
                    || line.starts_with("thunk_forces")
                    || line.starts_with("thunk_evals")
            })
            .map(|line| format!("{line}\n"))
            .collect();

        assert_eq!(run.stdout, expected(&program, "stdout"), "stdout mismatch for {program:?}");
        assert_eq!(run.thunk_stats, semantic, "semantic counters diverged for {program:?}");
        assert_eq!(run.status, 0, "expected success for {program:?}");
    }
}

/// A corpus case exports `play` and is a library, so what gets compiled is an
/// entry that imports it — the same wrapper `kanso run` is handed, since the
/// language knows no such name. Everything else compiles under its bare name,
/// as the CLI is invoked from the case's directory, so err origins in traces
/// name the file identically — through the play door when the file holds
/// definitions beside statements, which is the door its reader would use.
fn compile_case(program: &Path) -> kanso::ast::Program {
    let source = std::fs::read_to_string(program).expect("case source reads");
    let file =
        program.file_name().and_then(|name| name.to_str()).expect("kso files have utf-8 names");
    let compiled = match source.contains("\npub play") || source.starts_with("pub play") {
        true => {
            let entry = staged_entry(program);
            let wrapper = std::fs::read_to_string(&entry).expect("the entry reads");
            kanso::compile_entry(&entry.to_string_lossy(), &wrapper)
        }
        false if play_shaped(&source) => kanso::compile_play_file(file, &source),
        false => kanso::compile_entry(file, &source),
    };
    compiled.unwrap_or_else(|rendered| panic!("compile failed for {program:?}:\n{rendered}"))
}

/// A file holding declarations beside bare statements is a play file. A
/// binding is not a statement: a file of `fn`s and `test_` bindings is a test
/// file, and it has a verb of its own.
fn play_shaped(source: &str) -> bool {
    let tops: Vec<&str> = source.lines().filter(|l| !l.is_empty() && !l.starts_with(' ')).collect();
    let declares = tops.iter().any(|l| l.starts_with("fn ") || l.starts_with("type "));
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

/// Where a case's directory is staged. The CLI runs from that directory and
/// names the entry bare, so its traces carry bare file names; compiling by
/// absolute path here puts the staging prefix in front of them, and the
/// runtime cases take it back off before comparing.
fn stage_dir(program: &Path) -> PathBuf {
    let name = program.file_stem().and_then(|s| s.to_str()).expect("kso files have names");
    let source = program.parent().expect("cases live in a directory");
    std::env::temp_dir().join(format!(
        "kanso-oracle-{}-{name}",
        source.file_name().and_then(|s| s.to_str()).unwrap_or("dir")
    ))
}

/// The case's whole directory is staged, because cases read fixture files and
/// directories that sit beside them.
fn staged_entry(program: &Path) -> PathBuf {
    let name = program.file_stem().and_then(|s| s.to_str()).expect("kso files have names");
    let source = program.parent().expect("cases live in a directory");
    let stage = stage_dir(program);
    let _ = std::fs::remove_dir_all(&stage);
    stage_tree(source, &stage);
    let entry = stage.join(format!("run_{name}.kso"));
    std::fs::write(&entry, format!("import \"{name}\"\n\n{name}/play\n"))
        .expect("the entry file writes");
    entry
}

fn stage_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the staging directory is made");
    for entry in std::fs::read_dir(from).expect("the corpus is readable") {
        let path = entry.expect("directory entry").path();
        let landing = to.join(path.file_name().expect("entries have names"));
        match path.is_dir() {
            true => stage_tree(&path, &landing),
            false => {
                std::fs::copy(&path, &landing).expect("the file copies");
            }
        }
    }
}

#[test]
fn interpreter_prints_each_example_golden_stdout() {
    for program in kso_files(&manifest_dir().join("examples")) {
        let compiled = compile_case(&program);
        let golden = manifest_dir()
            .join("tests/golden/examples")
            .join(program.file_name().expect("kso files have names"));

        let run = evaluate(&compiled, Vec::new());

        assert_eq!(
            run.stdout,
            expected(&golden, "stdout"),
            "interpreter stdout mismatch for {program:?}"
        );
        assert_eq!(run.stderr, "", "interpreter diagnostics for {program:?}");
        assert_eq!(run.status, 0, "interpreter status for {program:?}");
    }
}

#[test]
fn interpreter_reports_each_runtime_endpoint_violation() {
    for program in kso_files(&manifest_dir().join("tests/golden/runtime")) {
        let compiled = compile_case(&program);

        let run = evaluate(&compiled, Vec::new());

        let said = run.stderr.replace(&format!("{}/", stage_dir(&program).display()), "");

        assert_eq!(
            said,
            expected(&program, "stderr"),
            "interpreter diagnostics mismatch for {program:?}"
        );
        assert_eq!(run.status, 1, "interpreter status for {program:?}");
    }
}

#[test]
fn interpreter_runs_the_trace_demo_module_with_default_and_explicit_args() {
    let entry = manifest_dir().join("examples/trace_demo/main.kso");
    let source = std::fs::read_to_string(&entry).expect("the entry reads");
    let program =
        kanso::compile_entry(&entry.to_string_lossy(), &source).expect("trace_demo compiles");
    for args in [Vec::new(), vec!["examples/trace_demo/VERSION".to_string()]] {
        let run = evaluate(&program, args.clone());

        assert_eq!(run.stdout, "== major version 12 detected ==\n", "stdout for args {args:?}");
        assert_eq!(run.stderr, "", "diagnostics for args {args:?}");
        assert_eq!(run.status, 0, "status for args {args:?}");
    }
}
