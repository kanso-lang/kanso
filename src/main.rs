use kanso::{ast, compile, diag, eval};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("repl") {
        return repl();
    }
    let (command, file, plan, release, interp) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!(
                "usage: kanso run <file.kso> [--plan|--interp] | kanso check <file.kso> | kanso \
                 test <file.kso> | kanso build <file.kso> [--release] | kanso repl"
            );
            return ExitCode::from(2);
        }
    };
    let require_main = command == "run" || command == "play";
    let path = std::path::Path::new(&file);
    let (program, source) = match path.is_dir() {
        true => match kanso::compile_module(path, require_main) {
            Ok(program) => (program, String::new()),
            Err(rendered) => {
                eprint!("{}", diag::paint(&rendered));
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
            let has_play = source.contains("pub play");
            let has_defs = source
                .lines()
                .any(|l| l.starts_with("fn ") || l.starts_with("type ") || l.starts_with("pub "));
            let library_verb = command == "test";
            match match (command.as_str(), has_play, has_defs) {
                ("play", _, _) => kanso::compile_play(&file, &source),
                (_, true, _) if !library_verb => kanso::compile_play(&file, &source),
                ("check", false, true) => compile(&file, &source, false),
                (_, false, true) if !library_verb => Err(format!(
                    "error: `{file}` is a library — nothing to run. give the \
                     module a main.kso entry, or define `pub play` and use \
                     `kanso play`\n"
                )),
                _ if library_verb => compile(&file, &source, false),
                _ => kanso::compile_entry(&file, &source),
            } {
                Ok(program) => (program, source),
                Err(rendered) => {
                    eprint!("{}", diag::paint(&rendered));
                    return ExitCode::from(2);
                }
            }
        }
    };
    if std::env::var("KANSO_BEAT_REPORT").is_ok() {
        let inference = kanso::infer::infer(&program);
        for line in kanso::beat::report(&program, &inference) {
            eprintln!("beat: {line}");
        }
    }
    if command == "check" {
        for advisory in kanso::advisory::door_advisories(&program) {
            eprintln!("{advisory}");
        }
        println!("{file}: ok");
        return ExitCode::SUCCESS;
    }
    if command == "test" {
        return run_tests(&program, &file, &source);
    }
    if command == "build" {
        return build(&program, &file, release);
    }
    if interp {
        return run_interpreted(&program);
    }
    run(&program, &file, &source, plan)
}

fn parse_args(args: &[String]) -> Option<(String, String, bool, bool, bool)> {
    let command = args.first()?.clone();
    if command != "run" && command != "check" && command != "test" && command != "build" && command != "play" {
        return None;
    }
    let file = args.get(1)?.clone();
    let mut rest = args.iter().skip(2);
    let mut plan = false;
    let mut release = false;
    let mut interp = false;
    for arg in rest.by_ref() {
        match arg.as_str() {
            "--plan" => plan = true,
            "--release" => release = true,
            "--interp" => interp = true,
            "--" => break,
            _ => return None,
        }
    }
    if (plan || interp) && command != "run" && command != "play" {
        return None;
    }
    if release && command != "build" {
        return None;
    }
    Some((command, file, plan, release, interp))
}

/// Execute `main` on the reference interpreter — the semantics oracle. `run`
/// compiles native; this path is for effects the backend doesn't lower yet
/// (the cooperative scheduler, `sleep`, `random`), so the concurrency model
/// can be seen before it is ported to the native and wasm engines.
fn run_interpreted(program: &ast::Program) -> ExitCode {
    let interp = eval::Interp::new(program);
    let desc = match interp.run_main() {
        Ok(eval::Value::Desc(d)) => d,
        Ok(_) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e.message);
            return ExitCode::FAILURE;
        }
    };
    let mut executor = eval::RealExecutor { program_args: program_args(), rng: eval::Rng::seeded() };
    match interp.execute(&desc, &mut executor) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e.message);
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn repl() -> ExitCode {
    ExitCode::FAILURE
}

#[cfg(not(target_arch = "wasm32"))]
fn repl() -> ExitCode {
    use rustyline::error::ReadlineError;
    println!(
        "kanso repl — expressions evaluate, declarations persist, :help for \
         directives, ctrl-d exits"
    );
    let mut editor = match rustyline::DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("error: cannot open the terminal: {e}");
            return ExitCode::FAILURE;
        }
    };
    let history = std::env::home_dir().map(|h| h.join(".kanso_repl_history"));
    if let Some(path) = &history {
        let _ = editor.load_history(path);
    }
    let mut session = kanso::repl::Session::new();
    let mut executor = eval::RealExecutor { program_args: Vec::new(), rng: eval::Rng::seeded() };
    let mut buffer = String::new();
    loop {
        let prompt = match buffer.is_empty() {
            true => "» ",
            false => "… ",
        };
        // inside a block, the next line almost always sits at indent 2 —
        // pre-fill it so the user never types the indentation
        let read = match buffer.is_empty() {
            true => editor.readline(prompt),
            false => editor.readline_with_initial(prompt, ("  ", "")),
        };
        let line = match read {
            Ok(line) => line.trim_end().to_string(),
            // ctrl-c abandons the block in progress (or the empty prompt)
            Err(ReadlineError::Interrupted) => {
                buffer.clear();
                continue;
            }
            Err(_) => break,
        };
        if buffer.is_empty() && line.starts_with(':') {
            let _ = editor.add_history_entry(&line);
            directive(&line, &mut session);
            continue;
        }
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
        if !submit.trim().is_empty() {
            let _ = editor.add_history_entry(&submit);
        }
        report(session.eval(&submit, &mut executor));
    }
    if let Some(path) = &history {
        let _ = editor.save_history(path);
    }
    ExitCode::SUCCESS
}

/// Multi-line input: fn/type declarations and block-form constants read
/// until a blank line.
#[cfg(not(target_arch = "wasm32"))]
fn opens_block(line: &str) -> bool {
    let head = line.strip_prefix("pub ").unwrap_or(line);
    head.starts_with("fn ") || head.starts_with("type ") || line.ends_with('=')
}

/// `:`-directives talk to the session itself, outside the language's grammar.
#[cfg(not(target_arch = "wasm32"))]
fn directive(line: &str, session: &mut kanso::repl::Session) {
    let mut words = line.split_whitespace();
    let verb = words.next().unwrap_or("");
    let arg = words.next();
    match (verb, arg) {
        (":show", name) => match session.show(name) {
            Ok(text) => println!("{text}"),
            Err(message) => eprint!("{}", diag::paint(&message)),
        },
        (":delete", Some(name)) => match session.delete(name) {
            Ok(echo) => println!("{echo}"),
            Err(message) => eprint!("{}", diag::paint(&message)),
        },
        (":delete", None) => eprintln!("usage: :delete name"),
        (":help", _) => {
            println!(":show          the whole session, as the file it is");
            println!(":show foo      foo's definition, without running it");
            println!(":delete foo    remove foo (refused while something still uses it)");
            println!(":help          this list");
            println!("repeating a declaration replaces it; ctrl-c abandons a block");
        }
        _ => eprintln!("unknown directive {verb} — try :help"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn report(outcome: Result<kanso::repl::Outcome, String>) {
    match outcome {
        Ok(kanso::repl::Outcome::Defined(echo)) => println!("{echo}"),
        Ok(kanso::repl::Outcome::Value(rendered)) => match rendered.is_empty() {
            true => {}
            false => println!("{rendered}"),
        },
        Ok(kanso::repl::Outcome::Executed(rendered)) => match rendered.is_empty() {
            true => {}
            false => println!("{rendered}"),
        },
        Err(message) => eprint!("{}", diag::paint(&message)),
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
                eprint!("{}", diag::paint(&diag::render(&[d], file, source)));
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
            eprint!("{}", diag::paint(&diag::render(&[d], file, source)));
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
