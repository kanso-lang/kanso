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

fn expected(path: &Path, extension: &str) -> String {
    let golden = path.with_extension(extension);
    std::fs::read_to_string(&golden).unwrap_or_else(|_| panic!("missing golden file {golden:?}"))
}

/// Captures raw printed lines the way the native binary writes stdout, and
/// reads the real filesystem so file-driven cases behave as `RealExecutor`.
struct CollectExecutor {
    program_args: Vec<String>,
    stdout: String,
}

impl Executor for CollectExecutor {
    fn print(&mut self, text: &str) {
        self.stdout.push_str(text);
        self.stdout.push('\n');
    }

    fn args(&mut self) -> Vec<String> {
        self.program_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Err("the oracle supplies no stdin".to_string())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
    }

    fn write_file(&mut self, path: &str, _content: &str) -> Result<(), String> {
        Err(format!("the oracle does not write files: cannot write {path}"))
    }
}

struct Evaluation {
    status: i32,
    stderr: String,
    stdout: String,
}

/// Mirrors `kanso_run` in src/wasm.rs — the canonical interpreter execution
/// shape — with printed text and diagnostics kept on separate streams so
/// each can be asserted against its golden file.
fn evaluate(program: &kanso::ast::Program, program_args: Vec<String>) -> Evaluation {
    let interp = Interp::new(program);
    let value = match interp.run_main() {
        Ok(value) => value,
        Err(runtime) => {
            return Evaluation {
                status: 1,
                stderr: format!("error[runtime]: {}\n", runtime.message),
                stdout: String::new(),
            }
        }
    };
    let mut executor = CollectExecutor { program_args, stdout: String::new() };
    let (reached, outcome) = match value {
        Value::Desc(desc) => ("the executor", interp.execute(&desc, &mut executor)),
        other => ("main", Ok(other)),
    };
    match outcome {
        Ok(Value::ErrV(info)) => Evaluation {
            status: 1,
            stderr: format!(
                "error[endpoint]: unhandled err reached {reached}: {}\n{}",
                render(&info.reason, true),
                trace_lines(&info)
            ),
            stdout: executor.stdout,
        },
        Ok(Value::NoneV) if reached == "main" => Evaluation {
            status: 1,
            stderr: "error[endpoint]: unhandled none reached main\n".to_string(),
            stdout: executor.stdout,
        },
        Ok(_) => Evaluation { status: 0, stderr: String::new(), stdout: executor.stdout },
        Err(runtime) => Evaluation {
            status: 1,
            stderr: format!("error[runtime]: {}\n", runtime.message),
            stdout: executor.stdout,
        },
    }
}

/// Compiles under the bare file name, as the CLI is invoked from the case's
/// directory, so err origins in traces name the file identically.
fn compile_case(program: &Path) -> kanso::ast::Program {
    let file = program
        .file_name()
        .and_then(|name| name.to_str())
        .expect("kso files have utf-8 names");
    let source = std::fs::read_to_string(program).expect("case source reads");
    kanso::compile(file, &source, true)
        .unwrap_or_else(|rendered| panic!("compile failed for {program:?}:\n{rendered}"))
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

        assert_eq!(
            run.stderr,
            expected(&program, "stderr"),
            "interpreter diagnostics mismatch for {program:?}"
        );
        assert_eq!(run.status, 1, "interpreter status for {program:?}");
    }
}

#[test]
fn interpreter_runs_the_trace_demo_module_with_default_and_explicit_args() {
    let dir = manifest_dir().join("examples/trace_demo");
    let program = kanso::compile_module(&dir, true).expect("trace_demo compiles");
    for args in [Vec::new(), vec!["examples/trace_demo/VERSION".to_string()]] {
        let run = evaluate(&program, args.clone());

        assert_eq!(run.stdout, "== major version 12 detected ==\n", "stdout for args {args:?}");
        assert_eq!(run.stderr, "", "diagnostics for args {args:?}");
        assert_eq!(run.status, 0, "status for args {args:?}");
    }
}
