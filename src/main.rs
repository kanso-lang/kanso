use kanso::{ast, diag, eval};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

/// What the compiler itself costs, counted rather than timed. Peak resident
/// bytes is stable enough to compare between runs on one machine, but it is
/// the operating system's number and it moves with page granularity and with
/// whatever the allocator decided to keep; this is the compiler's own demand,
/// which is the same on every machine and every run.
///
/// Relaxed ordering throughout: these are a tally, not a synchronisation
/// point, and no reader depends on seeing them in any particular order.
struct Counting;

static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let n = layout.size() as u64;
        ALLOC_BYTES.fetch_add(n, Ordering::Relaxed);
        ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        let live = LIVE_BYTES.fetch_add(n, Ordering::Relaxed) + n;
        PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COMPILER_ALLOCATOR: Counting = Counting;

/// The compiler's own cost, in the same shape the runtime prints its own:
/// `KANSO_COUNTERS=1 kanso check <program>` writes them to stderr so a
/// recorder can read one and a reader can read the other.
fn compiler_counters() -> String {
    let (rounds, visits) = kanso::infer::work::taken();
    format!(
        "compile_alloc_bytes={}\ncompile_allocs={}\ncompile_peak_bytes={}\n\
         compile_passes={}\ncompile_rounds={}\ncompile_visits={}\n",
        ALLOC_BYTES.load(Ordering::Relaxed),
        ALLOC_CALLS.load(Ordering::Relaxed),
        PEAK_BYTES.load(Ordering::Relaxed),
        kanso::infer::work::passes(),
        rounds,
        visits,
    )
}

const VERBS: [&str; 8] = ["run", "check", "test", "build", "install", "list", "update", "repl"];

const USAGE: &str = "usage: kanso <verb> [arguments]

  run <file|dir> [--plan|--interp]   compile and run; --plan shows the effects
                                     it would perform, --interp uses the oracle
  play <file> [--interp]             run a little program: definitions and
                                     statements in one file, stdlib imports only
  check <file|dir>                   report what run would refuse, and stop
  test <file|dir>                    evaluate every `test_*` constant
  build <file|dir> [--release]       write a native binary here, named for
                                     the program
  repl                               evaluate expressions as you type them

  install <dir> [--from owner/repo@branch]
                                     resolve the imports, fetch them, write
                                     the lock; --from pins one to a branch
  list <dir>                         what the lock pins, and what has moved on
  update <dir> [owner/repo]          walk release pins forward, rewrite the lock

";

fn main() -> ExitCode {
    let code = driven();
    kanso::phase::report();
    code
}

fn driven() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("repl") {
        return repl();
    }
    // Asking for help is not a mistake, so it answers on stdout and exits
    // clean — `kanso --help | less` showed nothing when it did neither.
    if matches!(args.first().map(String::as_str), Some("help" | "--help" | "-h")) {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let (command, file, plan, release, interp) = match parse_args(&args) {
        Some(parsed) => parsed,
        None => {
            // What was wrong comes before what to do about it: a bare usage
            // dump makes the reader diff it against what they typed.
            match args.first() {
                None => eprintln!("kanso: no verb given"),
                Some(verb) if VERBS.contains(&verb.as_str()) => {
                    eprintln!("kanso: `{verb}` wants a file or directory")
                }
                Some(verb) => eprintln!("kanso: `{verb}` is not a verb"),
            }
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    // hako's three verbs are kanso programs. They run interpreted because a
    // verb of the toolchain cannot wait on a C compiler, and everything after
    // the directory is the verb's own: the hakos to pin for `install`, the one
    // to walk for `update`.
    if matches!(command.as_str(), "install" | "list" | "update") {
        let mut argv = vec![command.clone(), file.clone()];
        match command.as_str() {
            "install" => argv.extend(hako_overrides()),
            "update" => argv.extend(hako_named()),
            _ => {}
        }
        return run_hako(argv);
    }

    if command == "play" {
        let source = match std::fs::read_to_string(&file) {
            Ok(source) => source,
            Err(io) => {
                eprintln!("error: cannot read {file}: {io}");
                return ExitCode::from(2);
            }
        };
        let program = match kanso::compile_play_file(&file, &source) {
            Ok(program) => program,
            Err(rendered) => {
                eprint!("{}", diag::paint(&rendered));
                return ExitCode::from(2);
            }
        };
        if interp {
            return run_interpreted(&program, program_args());
        }
        return run(&program, &file, &source, false);
    }
    let require_entry = command == "run";
    // Targeting a directory means its entry: `kanso run foo` is
    // `kanso run foo/main.kso` (the module-shape gavel), and checking the
    // directory checks the same program. A directory without an entry is a
    // library, compiled as the module it is.
    let entry_inside = std::path::Path::new(&file).join("main.kso");
    let rerouted_dir =
        matches!(command.as_str(), "run" | "check" | "build") && entry_inside.is_file();
    // A build rerouted through a directory keeps the directory's name: the
    // program is `bench/jsonbench`, and `main` names nothing.
    // `.` and `..` address a directory without naming it, so the name comes
    // from where the path lands rather than from how it was spelled.
    let built_as = match rerouted_dir {
        true => std::fs::canonicalize(&file)
            .ok()
            .as_deref()
            .and_then(std::path::Path::file_name)
            .map(|n| n.to_string_lossy().into_owned()),
        false => None,
    };
    let file = match rerouted_dir {
        true => entry_inside.to_string_lossy().into_owned(),
        false => file,
    };
    // Testing a program directory tests its module: the entry holds
    // statements, the tests live in the library beside it. The descent only
    // happens when it is unambiguous — no root library files and exactly
    // one module directory.
    let file = match command == "test" && entry_inside.is_file() {
        true => {
            let dir = std::path::Path::new(&file);
            let root_libs = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| {
                    e.path().extension().is_some_and(|x| x == "kso") && e.file_name() != "main.kso"
                })
                .count();
            let subdirs: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && std::fs::read_dir(p)
                            .into_iter()
                            .flatten()
                            .flatten()
                            .any(|e| e.path().extension().is_some_and(|x| x == "kso"))
                })
                .collect();
            match (root_libs, subdirs.as_slice()) {
                (0, [only]) => only.to_string_lossy().into_owned(),
                _ => file,
            }
        }
        false => file,
    };
    // A test file is a member of its module, and a module's files share their
    // declarations — so testing one compiles the module it belongs to, or the
    // file cannot see the very functions it is testing.
    let among_siblings = command == "test"
        && std::path::Path::new(&file).is_file()
        && std::path::Path::new(&file).parent().is_some_and(|dir| {
            std::fs::read_dir(dir).into_iter().flatten().flatten().any(|e| {
                let path = e.path();
                path.extension().is_some_and(|x| x == "kso")
                    && path.file_name() != std::path::Path::new(&file).file_name()
                    && e.file_name() != "main.kso"
            })
        });
    let only_from = match among_siblings {
        true => std::path::Path::new(&file).file_name().map(|n| n.to_string_lossy().into_owned()),
        false => None,
    };
    let file = match among_siblings {
        true => std::path::Path::new(&file)
            .parent()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or(file),
        false => file,
    };
    let path = std::path::Path::new(&file);
    let (program, source) = match path.is_dir() {
        true => match kanso::compile_module(path, require_entry) {
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
            // "It was never advisory" — Clay, 2026-08-25. Gavel 24 is dispatch
            // semantics now, so an arm written for its own hako's err is dead
            // code, and dead code that looks like error handling is worth
            // refusing rather than mentioning.
            let refusals = kanso::provenance::violations(&program, &prov, &inference.returns);
            if !refusals.is_empty() {
                for refusal in &refusals {
                    eprintln!("{refusal}");
                }
                return ExitCode::from(1);
            }
        }
        if std::env::var_os("KANSO_COUNTERS").is_some() {
            eprint!("{}", compiler_counters());
        }
        println!("{file}: ok");
        return ExitCode::SUCCESS;
    }
    if command == "test" {
        return run_tests(&program, &file, &source, only_from.as_deref());
    }
    if command == "build" {
        return build(&program, &file, release, built_as);
    }
    if interp {
        return run_interpreted(&program, program_args());
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
        && command != "install"
        && command != "list"
        && command != "update"
        && command != "play"
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
    if plan && command != "run" {
        return None;
    }
    if interp && command != "run" && command != "play" {
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
fn run_interpreted(program: &ast::Program, args: Vec<String>) -> ExitCode {
    // Interp eval depth scales with program recursion (and force-time
    // evaluation of deferred binds); pin a deep stack rather than lean on
    // the main thread's default, mirroring the oracle harness.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(scope, || run_interpreted_on_stack(program, args))
            .expect("spawns")
            .join()
            .expect("interpreter thread completes")
    })
}

fn run_interpreted_on_stack(program: &ast::Program, args: Vec<String>) -> ExitCode {
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
            let mut executor = eval::RealExecutor { program_args: args, rng: eval::Rng::seeded() };
            match interp.execute(&desc, &mut executor) {
                Ok(eval::Value::ErrV(info)) if deliberate_exit(&info.reason).is_some() => {
                    ExitCode::from(deliberate_exit(&info.reason).unwrap_or(1))
                }
                Ok(eval::Value::ErrV(info)) => {
                    eprint!(
                        "error[endpoint]: unhandled err reached the executor: {}\n{}",
                        eval::render(&interp, &info.reason, true),
                        eval::trace_lines(&interp, &info)
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
                "error[endpoint]: unhandled err reached the entry: {}\n{}",
                eval::render(&interp, &info.reason, true),
                eval::trace_lines(&interp, &info)
            );
            ExitCode::FAILURE
        }
        eval::Value::NoneV => {
            eprintln!("error[endpoint]: unhandled none reached the entry");
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

/// A deliberate exit is an err whose reason is `os/exit_status`. The endpoint
/// reads its code rather than reporting it, because the program did not fail
/// to say what it meant — it said it.
fn deliberate_exit(reason: &eval::Value) -> Option<u8> {
    let eval::Value::Record { ty, fields } = reason else { return None };
    // the type spells its module chain at whatever depth the import graph
    // qualified it: os/exit_status directly, hako/os/exit_status one hop in
    if !(ty.as_ref() == "os/exit_status" || ty.ends_with("/os/exit_status")) {
        return None;
    }
    match fields.borrow().first() {
        Some(eval::Value::Int(code)) => Some(u8::try_from(code.clone()).unwrap_or(1)),
        // an exit_status carrying something that is not a status is not a
        // program saying what it meant — it is one that went wrong computing
        // the code, and the reader is owed that rather than a silent 1
        _ => None,
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

/// hako's `list`, carried in the binary as source and run on the spot.
fn run_hako(args: Vec<String>) -> ExitCode {
    let program = match kanso::compile_hako() {
        Ok(program) => program,
        Err(rendered) => {
            eprint!("{}", diag::paint(&rendered));
            return ExitCode::from(2);
        }
    };
    run_interpreted(&program, args)
}

/// Everything after `--` belongs to the program.
fn program_args() -> Vec<String> {
    let all: Vec<String> = std::env::args().collect();
    match all.iter().position(|a| a == "--") {
        Some(i) => all[i + 1..].to_vec(),
        None => Vec::new(),
    }
}

fn build(program: &ast::Program, file: &str, release: bool, built_as: Option<String>) -> ExitCode {
    let ir = match kanso::codegen::emit_ir(program) {
        Ok(ir) => ir,
        Err(unsupported) => {
            eprintln!("error: {unsupported}");
            return ExitCode::from(2);
        }
    };
    let named = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out")
        .to_string();
    let stem = built_as.unwrap_or(named);
    // A build is named for its program, so `kanso build myapp` from the
    // directory above `myapp/` wants to write a file where the directory
    // already is. The linker's own words for that are `cannot open output
    // file myapp: Is a directory` followed by `clang failed`, which names
    // neither the cause nor the way out.
    if std::path::Path::new(&stem).is_dir() {
        eprintln!(
            "error: this build is named `{stem}`, and a directory of that name is here — \
             build it from inside (`cd {stem} && kanso build .`), or build it from \
             somewhere the name is free"
        );
        return ExitCode::from(2);
    }
    let ll_path = format!("{stem}.ll");
    let ir = match release {
        true => narrow_tailcc(ir),
        false => ir,
    };
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

/// arm64's argument registers, x0 through x7.
const ARGUMENT_REGISTERS: usize = 8;

/// How many argument registers a parameter list wants: a %KValue and a %parsed
/// are each two i64s, everything else is one.
fn arg_registers(params: &str) -> usize {
    params
        .split(", ")
        .filter(|p| !p.trim().is_empty())
        .map(|p| match p.starts_with("%KValue") || p.starts_with("%parsed") {
            true => 2,
            false => 1,
        })
        .sum()
}

fn defined_symbol(line: &str) -> Option<(String, usize)> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let name = line[..open].rsplit('@').next()?.trim().trim_matches('"').to_string();
    Some((name, arg_registers(&line[open + 1..close])))
}

fn called_symbol(line: &str) -> Option<String> {
    let at = line.find(" @")?;
    let rest = &line[at + 2..];
    let end = rest.find('(')?;
    Some(rest[..end].trim().trim_matches('"').to_string())
}

/// tailcc kept wherever the arguments fit the argument registers.
///
/// `tailcc` is what the beat machinery's `musttail` needs, and at -O1 and above
/// on arm64 the convention is miscompiled for an arm whose arguments spill past
/// x7 — the binary jumps to an address that was a value. The boundary is exactly
/// the register file: the micro corpus is clean at eight and one sample
/// segfaults at nine, growing to ten samples by twelve. So a wide arm keeps the
/// C convention and every call into it becomes ordinary, and everything narrow
/// enough keeps the guarantee the optimized build used to give up.
///
/// What a wide arm gives up is the jump: it spends a frame per hop, and a deep
/// recursion through one overflows the stack, which is loud.
fn narrow_tailcc(ir: String) -> String {
    let wide: std::collections::HashSet<String> = ir
        .lines()
        .filter(|l| l.starts_with("define tailcc ") || l.starts_with("declare tailcc "))
        .filter_map(defined_symbol)
        .filter(|(_, regs)| *regs > ARGUMENT_REGISTERS)
        .map(|(name, _)| name)
        .collect();
    let mut here = String::new();
    let mut out = String::new();
    for line in ir.lines() {
        let mut line = line.to_string();
        if line.starts_with("define ") || line.starts_with("declare ") {
            here = defined_symbol(&line).map(|(n, _)| n).unwrap_or_default();
            if wide.contains(&here) {
                line = line.replacen("tailcc ", "", 1);
            }
        } else if line.contains(" call ") {
            let callee = called_symbol(&line).unwrap_or_default();
            if wide.contains(&here) {
                line = line.replace("musttail ", "");
            }
            if wide.contains(&callee) {
                line = line.replace("musttail ", "").replace("tailcc ", "");
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
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

/// `only_from` names one of the module's files: `kanso test lib/list/list_test.kso`
/// compiles the module and runs that file's tests, where the bare directory
/// runs them all.
fn run_tests(
    program: &ast::Program,
    file: &str,
    source: &str,
    only_from: Option<&str>,
) -> ExitCode {
    let interp = eval::Interp::new(program);
    let mut names: Vec<&str> = program
        .fns
        .iter()
        .filter(|d| d.name.starts_with("test_") && d.params.is_empty())
        .filter(|d| only_from.is_none_or(|want| d.file.ends_with(want)))
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
                println!("{name} ... FAILED (returned {})", eval::render(&interp, &other, true));
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
                eprintln!("{}", ended_by_signal(&code, Some(program)));
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
fn ended_by_signal(status: &std::process::ExitStatus, program: Option<&ast::Program>) -> String {
    use std::os::unix::process::ExitStatusExt;
    const SIGSEGV: i32 = 11;
    match status.signal() {
        Some(SIGSEGV) => kanso::stack_exhausted(program),
        Some(other) => format!("error[runtime]: the program was ended by signal {other}"),
        None => "error[runtime]: the program ended without an exit code".to_string(),
    }
}

#[cfg(not(unix))]
fn ended_by_signal(_status: &std::process::ExitStatus, _program: Option<&ast::Program>) -> String {
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
    // The process writing the IR owns the file it hands clang. Two runs of
    // the same program share a key, so a shared path let one truncate and
    // rewrite what the other's clang was already reading — which surfaces as
    // a segmentation fault inside LLVM's assembly lexer, blamed on the
    // program rather than on the race.
    let ll_path =
        std::env::temp_dir().join(format!("kanso_run_{key:016x}_{}.ll", std::process::id()));
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
            let force = |v: &eval::Value| match interp.demand(v) {
                Ok(eval::Value::Desc(d)) => Some(d),
                _ => None,
            };
            eval::render_plan(&desc, &mut out, &force);
            print!("{out}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("error: main is not an io; there is no plan to show");
            ExitCode::FAILURE
        }
    }
}
