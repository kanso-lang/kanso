use kanso::{ast, compile, diag, eval};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, file, plan) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!(
                "usage: kanso run <file.kso> [--plan] | kanso check <file.kso> | kanso \
                 test <file.kso>"
            );
            return ExitCode::from(2);
        }
    };
    let source = match std::fs::read_to_string(&file) {
        Ok(source) => source,
        Err(io) => {
            eprintln!("error: cannot read {file}: {io}");
            return ExitCode::from(2);
        }
    };
    let require_main = command == "run";
    let program = match compile(&file, &source, require_main) {
        Ok(program) => program,
        Err(rendered) => {
            eprint!("{rendered}");
            return ExitCode::from(2);
        }
    };
    if command == "check" {
        println!("{file}: ok");
        return ExitCode::SUCCESS;
    }
    if command == "test" {
        return run_tests(&program, &file, &source);
    }
    run(&program, &file, &source, plan)
}

fn parse_args(args: &[String]) -> Option<(String, String, bool)> {
    let command = args.first()?.clone();
    if command != "run" && command != "check" && command != "test" {
        return None;
    }
    let file = args.get(1)?.clone();
    let plan = args.iter().skip(2).any(|a| a == "--plan");
    let extra = args.iter().skip(2).any(|a| a != "--plan");
    match extra || (plan && command == "check") {
        true => None,
        false => Some((command, file, plan)),
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
        eprintln!("{file}: no tests found (a test is a zero-argument `fn test_*`)");
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
                let mut executor = eval::RealExecutor;
                eval::execute(&desc, &mut executor);
                ExitCode::SUCCESS
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
