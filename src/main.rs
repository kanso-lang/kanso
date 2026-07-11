use kanso::{ast, compile, diag, eval};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, file, plan) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!("usage: kanso run <file.kso> [--plan] | kanso check <file.kso>");
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
    let program = match compile(&file, &source) {
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
    run(&program, &file, &source, plan)
}

fn parse_args(args: &[String]) -> Option<(String, String, bool)> {
    let command = args.first()?.clone();
    if command != "run" && command != "check" {
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
                eval::render_plan(&desc, 1, &mut out);
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
