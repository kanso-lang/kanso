use kanso::{ast, compile, diag, eval};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, file, plan) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!(
                "usage: kanso run <file.kso> [--plan] | kanso check <file.kso> | kanso \
                 test <file.kso> | kanso build <file.kso>"
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
        return build(&program, &file);
    }
    run(&program, &file, &source, plan)
}

fn parse_args(args: &[String]) -> Option<(String, String, bool)> {
    let command = args.first()?.clone();
    if command != "run" && command != "check" && command != "test" && command != "build" {
        return None;
    }
    let file = args.get(1)?.clone();
    let mut rest = args.iter().skip(2);
    let mut plan = false;
    for arg in rest.by_ref() {
        match arg.as_str() {
            "--plan" => plan = true,
            "--" => break,
            _ => return None,
        }
    }
    match plan && command != "run" {
        true => None,
        false => Some((command, file, plan)),
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

fn build(program: &ast::Program, file: &str) -> ExitCode {
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
    let runtime_path = std::env::temp_dir().join("kanso_runtime.c");
    if let Err(io) = std::fs::write(&runtime_path, include_str!("runtime.c")) {
        eprintln!("error: cannot write runtime: {io}");
        return ExitCode::from(2);
    }
    let status = std::process::Command::new("clang")
        .arg("-O3")
        .arg("-flto")
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(&stem)
        .arg(&ll_path)
        .arg(&runtime_path)
        .arg("-lm")
        .status();
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

fn run(program: &ast::Program, file: &str, source: &str, plan: bool) -> ExitCode {
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
        eval::Value::Desc(desc) => match plan {
            true => {
                let mut out = String::from("plan:\n");
                eval::render_plan(&desc, &mut out);
                print!("{out}");
                ExitCode::SUCCESS
            }
            false => {
                let mut executor = eval::RealExecutor { program_args: program_args() };
                match interp.execute(&desc, &mut executor) {
                    Ok(eval::Value::ErrV(reason)) => {
                        eprintln!(
                            "error[endpoint]: unhandled err reached the executor: {}",
                            eval::render(&reason, true)
                        );
                        ExitCode::FAILURE
                    }
                    Ok(_) => ExitCode::SUCCESS,
                    Err(runtime) => {
                        let d = diag::Diagnostic::new("runtime", runtime.message, runtime.span);
                        eprint!("{}", diag::render(&[d], file, source));
                        ExitCode::FAILURE
                    }
                }
            }
        },
        eval::Value::ErrV(reason) => {
            eprintln!(
                "error[endpoint]: unhandled err reached main: {}",
                eval::render(&reason, true)
            );
            ExitCode::FAILURE
        }
        eval::Value::NoneV => {
            eprintln!("error[endpoint]: unhandled none reached main");
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}
