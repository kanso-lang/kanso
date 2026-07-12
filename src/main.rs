use kanso::{ast, compile, diag, eval};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("repl") {
        return repl();
    }
    let (command, file, plan, release) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!(
                "usage: kanso run <file.kso> [--plan] | kanso check <file.kso> | kanso \
                 test <file.kso> | kanso build <file.kso> [--release] | kanso repl"
            );
            return ExitCode::from(2);
        }
    };
    let require_main = command == "run";
    let path = std::path::Path::new(&file);
    let (program, source) = match path.is_dir() {
        true => match kanso::compile_module(path, require_main) {
            Ok(program) => (program, String::new()),
            Err(rendered) => {
                eprint!("{rendered}");
                return ExitCode::from(2);
            }
        },
        false => {
            let source = match std::fs::read_to_string(&file) {
                Ok(source) => source,
                Err(io) => {
                    eprintln!("error: cannot read {file}: {io}");
                    return ExitCode::from(2);
                }
            };
            match compile(&file, &source, require_main) {
                Ok(program) => (program, source),
                Err(rendered) => {
                    eprint!("{rendered}");
                    return ExitCode::from(2);
                }
            }
        }
    };
    if command == "check" {
        println!("{file}: ok");
        return ExitCode::SUCCESS;
    }
    if command == "test" {
        return run_tests(&program, &file, &source);
    }
    if command == "build" {
        return build(&program, &file, release);
    }
    run(&program, &file, &source, plan)
}

fn parse_args(args: &[String]) -> Option<(String, String, bool, bool)> {
    let command = args.first()?.clone();
    if command != "run" && command != "check" && command != "test" && command != "build" {
        return None;
    }
    let file = args.get(1)?.clone();
    let mut rest = args.iter().skip(2);
    let mut plan = false;
    let mut release = false;
    for arg in rest.by_ref() {
        match arg.as_str() {
            "--plan" => plan = true,
            "--release" => release = true,
            "--" => break,
            _ => return None,
        }
    }
    if plan && command != "run" {
        return None;
    }
    if release && command != "build" {
        return None;
    }
    Some((command, file, plan, release))
}

fn repl() -> ExitCode {
    use std::io::{BufRead, Write};
    println!("kanso repl — expressions evaluate, declarations persist, ctrl-d exits");
    let mut session = kanso::repl::Session::new();
    let mut executor = eval::RealExecutor { program_args: Vec::new() };
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    loop {
        let prompt = match buffer.is_empty() {
            true => "» ",
            false => "… ",
        };
        print!("{prompt}");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => return ExitCode::SUCCESS,
            Ok(_) => {}
        }
        let line = line.trim_end().to_string();
        let submit = match buffer.is_empty() {
            true if opens_block(&line) => {
                buffer = line;
                continue;
            }
            true => line,
            false if line.is_empty() => std::mem::take(&mut buffer),
            false => {
                buffer.push('\n');
                buffer.push_str(&line);
                continue;
            }
        };
        report(session.eval(&submit, &mut executor));
    }
}

/// Multi-line input: fn/type declarations and block-form constants read
/// until a blank line.
fn opens_block(line: &str) -> bool {
    line.starts_with("fn ") || line.starts_with("type ") || line.ends_with('=')
}

fn report(outcome: Result<kanso::repl::Outcome, String>) {
    match outcome {
        Ok(kanso::repl::Outcome::Defined(names)) => println!("defined {names}"),
        Ok(kanso::repl::Outcome::Value(rendered)) => match rendered.is_empty() {
            true => {}
            false => println!("{rendered}"),
        },
        Ok(kanso::repl::Outcome::Executed(rendered)) => match rendered.is_empty() {
            true => {}
            false => println!("{rendered}"),
        },
        Err(message) => eprint!("{message}"),
    }
}

/// Everything after `--` belongs to the program.
fn program_args() -> Vec<String> {
    let all: Vec<String> = std::env::args().collect();
    match all.iter().position(|a| a == "--") {
        Some(i) => all[i + 1..].to_vec(),
        None => Vec::new(),
    }
}

fn build(program: &ast::Program, file: &str, release: bool) -> ExitCode {
    let ir = match kanso::codegen::emit_ir(program) {
        Ok(ir) => ir,
        Err(unsupported) => {
            eprintln!("error: {unsupported}");
            return ExitCode::from(2);
        }
    };
    let stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out")
        .to_string();
    let ll_path = format!("{stem}.ll");
    if let Err(io) = std::fs::write(&ll_path, ir) {
        eprintln!("error: cannot write {ll_path}: {io}");
        return ExitCode::from(2);
    }
    let status = match release {
        true => release_clang(&stem, &ll_path),
        false => dev_clang(&stem, &ll_path),
    };
    match status {
        Ok(code) if code.success() => {
            println!("built ./{stem} (llvm ir at {ll_path})");
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("error: clang failed on {ll_path}");
            ExitCode::FAILURE
        }
        Err(io) => {
            eprintln!("error: cannot invoke clang: {io}");
            ExitCode::FAILURE
        }
    }
}

/// Release: whole-program LTO across the program and a freshly compiled
/// runtime — the slowest build and the fastest binary.
fn release_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    let runtime_path = std::env::temp_dir().join("kanso_runtime.c");
    std::fs::write(&runtime_path, include_str!("runtime.c"))?;
    std::process::Command::new("clang")
        .arg("-O3")
        .arg("-flto")
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&runtime_path)
        .arg("-lm")
        .status()
}

/// Dev (the default): the program compiles unoptimized and links against a
/// cached optimized runtime object, so the runtime's cost is paid once per
/// runtime version, not per build.
fn dev_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    let runtime_obj = cached_runtime_object()?;
    std::process::Command::new("clang")
        .arg("-O0")
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&runtime_obj)
        .arg("-lm")
        .status()
}

fn cached_runtime_object() -> std::io::Result<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};
    let source = include_str!("runtime.c");
    let mut hasher = std::hash::DefaultHasher::new();
    source.hash(&mut hasher);
    let key = hasher.finish();
    let object = std::env::temp_dir().join(format!("kanso_runtime_{key:016x}.o"));
    if object.exists() {
        return Ok(object);
    }
    let c_path = std::env::temp_dir().join(format!("kanso_runtime_{key:016x}.c"));
    std::fs::write(&c_path, source)?;
    let staging = std::env::temp_dir().join(format!("kanso_runtime_{key:016x}_{}.o", std::process::id()));
    let status = std::process::Command::new("clang")
        .arg("-O2")
        .arg("-c")
        .arg(&c_path)
        .arg("-o")
        .arg(&staging)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other("clang failed on the runtime"));
    }
    std::fs::rename(&staging, &object)?;
    Ok(object)
}

fn run_tests(program: &ast::Program, file: &str, source: &str) -> ExitCode {
    let interp = eval::Interp::new(program);
    let mut names: Vec<&str> = program
        .fns
        .iter()
        .filter(|d| d.name.starts_with("test_") && d.params.is_empty())
        .map(|d| d.name.as_str())
        .collect();
    names.dedup();
    if names.is_empty() {
        eprintln!("{file}: no tests found (a test is a constant named `test_*`)");
        return ExitCode::from(2);
    }
    let mut failed = 0;
    for name in &names {
        let outcome = interp.run_named(name).expect("filtered on zero-arg fns");
        match outcome {
            Ok(eval::Value::True) => println!("{name} ... ok"),
            Ok(other) => {
                failed += 1;
                println!("{name} ... FAILED (returned {})", eval::render(&other, true));
            }
            Err(runtime) => {
                failed += 1;
                let d = diag::Diagnostic::new("runtime", runtime.message, runtime.span);
                println!("{name} ... FAILED");
                eprint!("{}", diag::render(&[d], file, source));
            }
        }
    }
    println!("{} passed, {failed} failed", names.len() - failed);
    match failed {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

/// `run` builds and executes: a dev-mode native binary, cached by IR hash so
/// an unchanged program re-runs with no clang at all. `--plan` stays on the
/// interpreter — it renders the effect DAG instead of executing it.
fn run(program: &ast::Program, file: &str, source: &str, plan: bool) -> ExitCode {
    if plan {
        return run_plan(program, file, source);
    }
    let ir = match kanso::codegen::emit_ir(program) {
        Ok(ir) => ir,
        Err(unsupported) => {
            eprintln!("error: {unsupported}");
            return ExitCode::from(2);
        }
    };
    let binary = match cached_program_binary(&ir) {
        Ok(binary) => binary,
        Err(io) => {
            eprintln!("error: cannot build: {io}");
            return ExitCode::FAILURE;
        }
    };
    let status = std::process::Command::new(&binary).args(program_args()).status();
    match status {
        Ok(code) => match code.code() {
            Some(n) => ExitCode::from(n.clamp(0, 255) as u8),
            None => ExitCode::FAILURE,
        },
        Err(io) => {
            eprintln!("error: cannot execute {}: {io}", binary.display());
            ExitCode::FAILURE
        }
    }
}

fn cached_program_binary(ir: &str) -> std::io::Result<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    ir.hash(&mut hasher);
    include_str!("runtime.c").hash(&mut hasher);
    let key = hasher.finish();
    let binary = std::env::temp_dir().join(format!("kanso_run_{key:016x}"));
    if binary.exists() {
        return Ok(binary);
    }
    let ll_path = std::env::temp_dir().join(format!("kanso_run_{key:016x}.ll"));
    std::fs::write(&ll_path, ir)?;
    let staging = std::env::temp_dir().join(format!("kanso_run_{key:016x}_{}", std::process::id()));
    let ll = ll_path.to_string_lossy().into_owned();
    let out = staging.to_string_lossy().into_owned();
    let status = dev_clang(&out, &ll)?;
    if !status.success() {
        return Err(std::io::Error::other("clang failed"));
    }
    std::fs::rename(&staging, &binary)?;
    Ok(binary)
}

fn run_plan(program: &ast::Program, file: &str, source: &str) -> ExitCode {
    let interp = eval::Interp::new(program);
    let result = match interp.run_main() {
        Ok(value) => value,
        Err(runtime) => {
            let d = diag::Diagnostic::new("runtime", runtime.message, runtime.span);
            eprint!("{}", diag::render(&[d], file, source));
            return ExitCode::FAILURE;
        }
    };
    match result {
        eval::Value::Desc(desc) => {
            let mut out = String::from("plan:\n");
            eval::render_plan(&desc, &mut out);
            print!("{out}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("error: main is not a description; there is no plan to show");
            ExitCode::FAILURE
        }
    }
}
