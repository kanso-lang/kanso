use kanso::{ast, diag, eval};
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
                 test <file.kso> | kanso build <file.kso> [--release] | kanso install <dir> [--from owner/repo@branch] | kanso list|update <dir> | \
                 kanso repl"
            );
            return ExitCode::from(2);
        }
    };
    if matches!(command.as_str(), "install" | "list" | "update") {
        let cache = std::env::var("KANSO_HAKO")
            .map(std::path::PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".hako")))
            .unwrap_or_default();
        let root = std::path::Path::new(&file);
        let done = match command.as_str() {
            "install" => kanso::hako::install(root, &cache, &hako_overrides()),
            "list" => kanso::hako::list(root),
            _ => kanso::hako::update(root, &cache, hako_named().as_deref()),
        };
        return match done {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(reason) => {
                eprintln!("error: {reason}");
                ExitCode::from(2)
            }
        };
    }
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
            match kanso::compile_source(&command, &file, &source) {
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
        let mut_sites = kanso::linear::in_place_pushes(&program);
        for line in kanso::beat::report(&program, &inference, &mut_sites) {
            eprintln!("beat: {line}");
        }
    }
    if command == "check" {
        for advisory in kanso::advisory::door_advisories(&program) {
            eprintln!("{advisory}");
        }
        let inference = kanso::infer::infer(&program);
        if std::env::var_os("KANSO_NO_PROV").is_none() {
            let prov = kanso::provenance::analyze(&program);
            for advisory in kanso::provenance::violations(&program, &prov, &inference.returns) {
                eprintln!("{advisory}");
            }
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

/// `kanso update <dir> [hako]` — the one verb that takes a name after its
/// directory, so the flag loop must not judge it.
fn hako_named() -> Option<String> {
    std::env::args().nth(3).filter(|a| !a.starts_with("--"))
}

/// `kanso install <dir> --from owner/repo@branch`, repeatable.
fn hako_overrides() -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2).filter(|pair| pair[0] == "--from").map(|pair| pair[1].clone()).collect()
}

fn parse_args(args: &[String]) -> Option<(String, String, bool, bool, bool)> {
    let command = args.first()?.clone();
    if command != "run"
        && command != "check"
        && command != "test"
        && command != "build"
        && command != "play"
        && command != "install"
        && command != "list"
        && command != "update"
    {
        return None;
    }
    let file = args.get(1)?.clone();
    // `update` names a hako after its directory; every other verb takes flags
    let mut rest = args.iter().skip(2 + usize::from(command == "update"));
    let mut plan = false;
    let mut release = false;
    let mut interp = false;
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--plan" => plan = true,
            "--release" => release = true,
            "--interp" => interp = true,
            // an interim pin's spelling; hako_overrides reads the values
            "--from" if command == "install" => {
                rest.next()?;
            }
            // worst-case measurement: thunk nothing, force everything. The
            // env var carries it to every stage (demand runs in infer,
            // codegen, and the interp) and into the spawned native binary.
            "--strict" => std::env::set_var("KANSO_STRICT", "1"),
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
    // Interp eval depth scales with program recursion (and force-time
    // evaluation of deferred binds); pin a deep stack rather than lean on
    // the main thread's default, mirroring the oracle harness.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(scope, || run_interpreted_on_stack(program))
            .expect("spawns")
            .join()
            .expect("interpreter thread completes")
    })
}

fn run_interpreted_on_stack(program: &ast::Program) -> ExitCode {
    let interp = eval::Interp::new(program);
    // Mirror the native runtime's KANSO_COUNTERS convention: semantic thunk
    // counters print to stderr at exit, byte-identical across engines.
    struct Stats<'i, 'p>(&'i eval::Interp<'p>);
    impl Drop for Stats<'_, '_> {
        fn drop(&mut self) {
            if std::env::var_os("KANSO_COUNTERS").is_some() {
                eprint!("{}", self.0.thunk_stats.render());
            }
        }
    }
    let _stats = Stats(&interp);
    let value = match interp.run_main() {
        Ok(value) => value,
        Err(e) => {
            eprintln!("error[runtime]: {}", e.message);
            return ExitCode::FAILURE;
        }
    };
    match value {
        eval::Value::Desc(desc) => {
            let mut executor =
                eval::RealExecutor { program_args: program_args(), rng: eval::Rng::seeded() };
            match interp.execute(&desc, &mut executor) {
                Ok(eval::Value::ErrV(info)) if deliberate_exit(&info.reason).is_some() => {
                    ExitCode::from(deliberate_exit(&info.reason).unwrap_or(1))
                }
                Ok(eval::Value::ErrV(info)) => {
                    eprint!(
                        "error[endpoint]: unhandled err reached the executor: {}\n{}",
                        eval::render(&info.reason, true),
                        eval::trace_lines(&info)
                    );
                    ExitCode::FAILURE
                }
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error[runtime]: {}", e.message);
                    ExitCode::FAILURE
                }
            }
        }
        eval::Value::ErrV(info) if deliberate_exit(&info.reason).is_some() => {
            ExitCode::from(deliberate_exit(&info.reason).unwrap_or(1))
        }
        eval::Value::ErrV(info) => {
            eprint!(
                "error[endpoint]: unhandled err reached main: {}\n{}",
                eval::render(&info.reason, true),
                eval::trace_lines(&info)
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
            report(session.directive(&line));
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

/// A deliberate exit is an err whose reason is `io/exit_status`. The endpoint
/// reads its code rather than reporting it, because the program did not fail
/// to say what it meant — it said it.
fn deliberate_exit(reason: &eval::Value) -> Option<u8> {
    let eval::Value::Record { ty, fields } = reason else { return None };
    if &**ty != "io/exit_status" {
        return None;
    }
    match fields.borrow().first() {
        Some(eval::Value::Int(code)) => Some(u8::try_from(code.clone()).unwrap_or(1)),
        _ => Some(1),
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
    let runtime_obj = cached_runtime_object("release", &["-O3", "-flto"])?;
    std::process::Command::new("clang")
        .arg("-O3")
        .arg("-flto")
        .args(if cfg!(target_arch = "x86_64") { &["-mssse3"][..] } else { &[][..] })
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&runtime_obj)
        .arg("-lm")
        .status()
}

/// Dev (the default): the program compiles unoptimized and links against a
/// cached optimized runtime object, so the runtime's cost is paid once per
/// runtime version, not per build.
fn dev_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    let runtime_obj = cached_runtime_object("dev", &["-O2"])?;
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

fn cached_runtime_object(profile: &str, opt: &[&str]) -> std::io::Result<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};
    let source = include_str!("runtime.c");
    let mut hasher = std::hash::DefaultHasher::new();
    source.hash(&mut hasher);
    profile.hash(&mut hasher);
    let key = hasher.finish();
    let object = std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key:016x}.o"));
    if object.exists() {
        return Ok(object);
    }
    let c_path = std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key:016x}.c"));
    std::fs::write(&c_path, source)?;
    let staging = std::env::temp_dir()
        .join(format!("kanso_runtime_{profile}_{key:016x}_{}.o", std::process::id()));
    let status = std::process::Command::new("clang")
        .args(opt)
        .args(if cfg!(target_arch = "x86_64") { &["-mssse3"][..] } else { &[][..] })
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
            None => {
                eprintln!("{}", ended_by_signal(&code));
                ExitCode::FAILURE
            }
        },
        Err(io) => {
            eprintln!("error: cannot execute {}: {io}", binary.display());
            ExitCode::FAILURE
        }
    }
}

/// A program the operating system killed has no exit code to report, and
/// saying nothing leaves the reader with a bare failure and no cause.
#[cfg(unix)]
fn ended_by_signal(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    const SIGSEGV: i32 = 11;
    match status.signal() {
        Some(SIGSEGV) => {
            "error[runtime]: the program ran out of stack: recursion went deeper than the stack holds"
                .to_string()
        }
        Some(other) => format!("error[runtime]: the program was ended by signal {other}"),
        None => "error[runtime]: the program ended without an exit code".to_string(),
    }
}

#[cfg(not(unix))]
fn ended_by_signal(_status: &std::process::ExitStatus) -> String {
    "error[runtime]: the program ended without an exit code".to_string()
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
            eprintln!("error: main is not an io; there is no plan to show");
            ExitCode::FAILURE
        }
    }
}
