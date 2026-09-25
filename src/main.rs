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

/// The allocator under the tally. mimalloc on every target that can build it:
/// glibc's malloc and free were 15.17% of `kanso check compile_corpus` and the
/// compiler's demand — every counter above — does not move by a byte.
#[cfg(not(target_arch = "wasm32"))]
const UNDER: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[cfg(target_arch = "wasm32")]
const UNDER: std::alloc::System = std::alloc::System;

/// mimalloc's `mi_option_arena_eager_commit`, by its position in the
/// `mi_option_t` enum. The Rust bindings stop naming options well before this
/// one, so the number is written here rather than imported: libmimalloc-sys
/// 0.1.49 builds v3 of the C library, whose header declares the option fifth,
/// after `show_errors`, `show_stats`, `verbose` and `deprecated_eager_commit`.
/// `tests/the_allocator_option_is_the_one_the_header_names.rs` reads that
/// header and goes red if the position ever moves.
#[cfg(not(target_arch = "wasm32"))]
const ARENA_EAGER_COMMIT: i32 = 4;

/// mimalloc's `mi_option_purge_delay`, sixteenth in the same enum and read
/// from the same header by the same spec.
#[cfg(not(target_arch = "wasm32"))]
const PURGE_DELAY: i32 = 15;

/// Two options, both set before the first allocation, both for a reason a
/// counter could not see.
///
/// **Reserve the first arena without committing it.**
///
/// Left alone, mimalloc commits that arena up front, and the six megabytes it
/// costs are resident in every process the compiler starts. Nothing in the
/// objective can see them: `compile_peak_bytes` is the compiler's own demand,
/// counted in the tally above, not the operating system's number.
/// `tests/bind_chain_depth.rs` reads resident memory, and it is what caught
/// this — a constant six megabytes under both of its readings squeezed a
/// ten-fold depth ratio from 3.16 to 1.97, and the shape that nests stopped
/// looking like one.
///
/// Turning the option off costs 24,330 instructions against the 3.84 million
/// the allocator saves. It has to happen here rather than at the top of
/// `main`, because by then the arena is already committed: Rust's runtime
/// allocates before it hands over. A constructor runs ahead of all of it.
///
/// **Never purge, so a timer stops deciding what the vein counts.** mimalloc
/// returns free pages to the operating system on a clock. Each arena carries
/// a deadline, and a pass over one asks `_mi_clock_now` — glibc's
/// `clock_gettime`, through the vDSO — whether that deadline has gone by. How
/// many of those asks a process makes depends on how long it has been
/// running, and how long it has been running is wall time. On
/// `kanso check compile_corpus` it asks 163 times; on the entry and library
/// corpora, which take about three and a half times the work, the whole purge
/// machinery costs five times as much. That is the signature of a term keyed
/// to elapsed time. A counter golden holding it is a number the next host,
/// or the next busy afternoon, is free to disagree with.
///
/// `-1` disables purging, which takes `_mi_prim_clock_now` from 163 calls to
/// 3 and removes 8,288 instructions from the module row, 44,608 from the
/// entry row and 48,487 from the library row. The three that remain are
/// `_mi_clock_start`'s calibration: it reads the clock twice to measure what
/// a read costs, then a third time for the process's start stamp, behind a
/// `mi_clock_diff == 0` guard that lets the whole thing happen once. The
/// compiler is a short-lived process that exits and gives everything back at
/// once, so never purging removes work.
///
/// This was found while hunting a reproduction failure — the three compile
/// rows came back 13 instructions apart on two CI runs of one commit, and the
/// 2026-09-05 ruling is one row, one value, so a disagreement halts the vein
/// and is neither keyed nor averaged. The timer does not explain that 13: the
/// same gap on all three rows points at something that happens once per
/// process, and purge asks scale with the run instead. So this removes a
/// wall-clock dependence that was real and would have bitten later, and the
/// original disagreement is still open. If it returns, those three reads are
/// the next place to look: their count cannot vary, but their cost is the
/// host's vDSO — 33 instructions here, 11 a call — so a clocksource priced
/// differently moves all three rows by the same small amount, which is the
/// shape that was seen. Three does not divide 13, so that is a suspect
/// rather than an answer.
#[cfg(not(target_arch = "wasm32"))]
extern "C" fn set_the_allocator_before_it_runs() {
    unsafe { libmimalloc_sys::mi_option_set(ARENA_EAGER_COMMIT, 0) };
    unsafe { libmimalloc_sys::mi_option_set(PURGE_DELAY, -1) };
}

#[cfg(not(target_arch = "wasm32"))]
#[used]
#[cfg_attr(target_vendor = "apple", link_section = "__DATA,__mod_init_func")]
#[cfg_attr(not(target_vendor = "apple"), link_section = ".init_array")]
static BEFORE_THE_FIRST_ALLOCATION: extern "C" fn() = set_the_allocator_before_it_runs;

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
        // The mimalloc crate's `GlobalAlloc` hands every allocation to
        // `mi_malloc_aligned`, whatever alignment it asked for. That wrapper
        // checks the alignment is a power of two, builds a mask from it, takes
        // a candidate block off the small-page free list and tests whether the
        // block is aligned, before it can hand back the block `mi_malloc`
        // would have handed back on its own. Every block mimalloc gives out is
        // at least eight-aligned, so an allocation that asks for no more than
        // eight has nothing to check.
        //
        // EIGHT AND NOT SIXTEEN. `MI_MAX_ALIGN_SIZE` is 16 and mimalloc
        // guarantees that for a block big enough to hold it, but a block
        // SMALLER than sixteen bytes can sit at an eight-aligned offset inside
        // a page whose first block is sixteen-aligned. `Layout` carries a size
        // and an alignment independently, so `align 16, size 8` is spellable
        // and would be wrong here. Eight is the bound that holds for every
        // size, and it is where the allocations are: a `Vec<u8>`, a `String`,
        // and any record whose widest field is a pointer or a u64.
        //
        // Only where mimalloc is the allocator under the tally. On wasm32
        // `UNDER` is `std::alloc::System` and `libmimalloc_sys` is not linked
        // at all, so the bypass is not merely pointless there, it does not
        // compile — and `src/main.rs` IS in the wasm build, which is how six
        // jobs went red on the first round of this change.
        #[cfg(not(target_arch = "wasm32"))]
        if layout.align() <= 8 {
            return unsafe { libmimalloc_sys::mi_malloc(layout.size()) }.cast();
        }
        unsafe { UNDER.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        unsafe { UNDER.dealloc(ptr, layout) }
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

/// What an interpreted run holds, read off the same counting allocator the
/// compiler's own row reads.
///
/// The interpreted engine is a deployment, not a stage of one: the program is
/// parsed, inferred and then EXECUTED in this process, and nothing is emitted.
/// So the number this prints covers the front end and the interpreter
/// together, which is what an interpreted run actually costs. Clay's gavel of
/// 2026-09-16 puts it on the development side, below start-up and speed:
/// "start-time is vastly more important than speed which is more important
/// than memory usage."
///
/// Printed at the interpreter's exit rather than at the process's, beside the
/// thunk counters, because that is where the run has finished holding
/// everything it is going to hold.
fn interpreter_counters() -> String {
    format!(
        "interp_alloc_bytes={}\ninterp_allocs={}\ninterp_peak_bytes={}\n",
        ALLOC_BYTES.load(Ordering::Relaxed),
        ALLOC_CALLS.load(Ordering::Relaxed),
        PEAK_BYTES.load(Ordering::Relaxed),
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

/// The compile row is read out of this frame, so it has to have one.
///
/// `scripts/gates/compile_instructions.sh` counts the compiler's own work by
/// taking `kanso::main` inclusive out of a callgrind profile, which drops the
/// loader and the stack guard above it. It anchored on
/// `std::rt::lang_start::{{closure}}` for one round and CI refused: that name
/// is the standard library's, and the toolchain the runners carry does not
/// emit it. This one is ours. `inline(never)` is what stops a future compiler
/// folding it into the shim that calls it and taking the anchor with it.
#[inline(never)]
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
        return play(&program, &file, &source);
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
    let mut counters = false;
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
            // The allocation counters cost the emitted fast paths a load and
            // a branch at every inlined append, push and insert -- 25,968,820
            // instructions on the run program, 1.2128%, measured by folding
            // the eight gates to a constant and relinking the shipped recipe.
            // A binary nobody is going to count is built without them, and
            // the counter gates ask for them by name.
            "--counters" => counters = true,
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
    if counters && command != "build" {
        return None;
    }
    // Carried by an env var for the same reason `--strict` is: codegen sits
    // three stages from this parse, and threading a sixth bool through every
    // caller of this tuple to reach it would be the whole diff.
    if counters {
        std::env::set_var("KANSO_COUNTERS_BUILD", "1");
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
                eprint!("{}", interpreter_counters());
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
                Ok(eval::Value::ErrV(info)) if eval::deliberate_exit(&info.reason).is_some() => {
                    ExitCode::from(eval::deliberate_exit(&info.reason).unwrap_or(1))
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
        eval::Value::ErrV(info) if eval::deliberate_exit(&info.reason).is_some() => {
            ExitCode::from(eval::deliberate_exit(&info.reason).unwrap_or(1))
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
    let emitted = match release {
        true => kanso::codegen::emit_ir(program, closure_convention()),
        false => kanso::codegen::emit_ir_dev(program, closure_convention()),
    };
    let ir = match emitted {
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
    // x86-64 alone: the arm64 limit `narrow_tailcc` keeps is a miscompile of
    // wide tail calls, and this convention has not been measured there.
    let flatten = release
        && cfg!(target_arch = "x86_64")
        && closure_convention() == kanso::codegen::ClosureConvention::PreserveNone;
    let ir = match (release, flatten) {
        (true, true) => preserve_none_calls(
            preserve_none_tails(narrow_tailcc(ir, PRESERVE_NONE_REGISTERS)),
            include_str!("runtime.c"),
        ),
        (true, false) => narrow_tailcc(ir, TAILCC_WIDEST),
        (false, _) => ir,
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

/// The widest arm, in argument words, that keeps `tailcc` in a release build.
///
/// On arm64 it is the register file, x0 through x7, past which the
/// convention is miscompiled (see `narrow_tailcc`). x86-64 lowers a `tailcc`
/// call whose arguments spill onto the stack correctly, so there the limit is
/// a cost and was measured. At eight it narrowed the JSON decoder's
/// `obj_key_end`, nine words, and a release build decoding an object of
/// 300,000 keys spent a frame per key and overflowed its stack. At nine the
/// decoder keeps its jumps and runbench reads 0.25% less. At ten and above the
/// wider std/regexp arms keep theirs too, and scanbench reads 1.9% more: a
/// tail call copies its stack arguments into the caller's, and past three of
/// them that costs more than the frame it saves.
const TAILCC_WIDEST: usize = match cfg!(target_arch = "aarch64") {
    true => 8,
    false => 9,
};

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

/// tailcc kept on every arm no wider than `widest` argument words:
/// `TAILCC_WIDEST`, or twelve where `preserve_none_tails` follows, since that
/// convention passes twelve words in registers and an arm that fits keeps its
/// jump. On scanbench the twelve reads 1.52% less than the nine.
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
fn narrow_tailcc(ir: String, widest: usize) -> String {
    let wide: std::collections::HashSet<String> = ir
        .lines()
        .filter(|l| l.starts_with("define tailcc ") || l.starts_with("declare tailcc "))
        .filter_map(defined_symbol)
        .filter(|(_, regs)| *regs > widest)
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

/// How many argument registers `preserve_nonecc` has on x86-64: r12 to r15,
/// rdi, rsi, rdx, rcx, r8, r9, r11 and rax.
const PRESERVE_NONE_REGISTERS: usize = 12;

/// The words a parameter of this type takes in a flattened signature, or
/// `None` for a type the rewrite does not carry.
fn flat_words(ty: &str) -> Option<usize> {
    match ty {
        "%KValue" | "%parsed" => Some(2),
        "i64" | "ptr" | "double" => Some(1),
        _ => None,
    }
}

/// A parameter or argument split into its type and the rest: `%KValue %x0`,
/// `i64 7`, `%KValue { i64 2, i64 0 }`.
fn typed(item: &str) -> Option<(&str, &str)> {
    let item = item.trim();
    let space = item.find(' ')?;
    let (ty, value) = (&item[..space], item[space + 1..].trim());
    flat_words(ty).map(|_| (ty, value))
}

/// Splits an argument list at the commas outside any brace, bracket or
/// parenthesis, so a constant `%KValue { i64 2, i64 0 }` stays one item.
fn top_level_items(list: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in list.char_indices() {
        match c {
            '{' | '[' | '(' | '<' => depth += 1,
            '}' | ']' | ')' | '>' => depth -= 1,
            ',' if depth == 0 => {
                items.push(list[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if !list[start..].trim().is_empty() {
        items.push(list[start..].trim());
    }
    items
}

/// The index just past the parenthesis that closes the one before `open`.
fn closing(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in line[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The symbol an `@` at `at` names, quotes removed, and where it ends.
fn symbol_at(line: &str, at: usize) -> (String, usize) {
    let rest = &line[at + 1..];
    match rest.strip_prefix('"') {
        Some(quoted) => {
            let end = quoted.find('"').unwrap_or(quoted.len());
            (quoted[..end].to_string(), at + 1 + end + 2)
        }
        None => {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || "_.$/-".contains(c)))
                .unwrap_or(rest.len());
            (rest[..end].to_string(), at + 1 + end)
        }
    }
}

/// A `define tailcc` header: the return type, the symbol, the parameters and
/// whatever follows the closing parenthesis.
struct TailDefine {
    ret: String,
    params: Vec<(String, String)>,
    words: usize,
}

fn tail_define(line: &str) -> Option<(String, TailDefine)> {
    let rest = line.strip_prefix("define tailcc ")?;
    let at = line.len() - rest.len() + rest.find(" @")? + 1;
    let ret = line["define tailcc ".len()..at].trim().to_string();
    let (name, after) = symbol_at(line, at);
    let close = closing(line, after)?;
    let mut params = Vec::new();
    let mut words = 0;
    for item in top_level_items(&line[after + 1..close - 1]) {
        let (ty, value) = typed(item)?;
        if !value.starts_with('%') || value.contains(' ') {
            return None;
        }
        words += flat_words(ty)?;
        params.push((ty.to_string(), value.to_string()));
    }
    Some((name, TailDefine { ret, params, words }))
}

/// Where a line calls a `tailcc` function: the offset of the convention
/// keyword, of the callee's `@`, and the symbol.
fn tail_call(line: &str) -> Option<(usize, usize, String)> {
    let keyword = line.find("call tailcc ")? + "call ".len();
    let at = keyword + line[keyword..].find(" @")? + 1;
    Some((keyword, at, symbol_at(line, at).0))
}

/// Every `tailcc` function joined to the others it `musttail`s into or is
/// `musttail`ed from takes `preserve_nonecc` and one flat signature.
///
/// The JSON decoder is a cycle of tail calls, `parse_value` into
/// `string_scan` into `obj_key_start` into `obj_delim` and round again, and
/// under `tailcc` every arm saved the callee-saved registers it used on entry
/// and restored them before each jump out. shrink-wrapping could not move the
/// saves, because an arm has several exits and each `musttail` is one. On
/// runbench those pushes and pops were 55,000,000 instructions in the four
/// arms named. `preserve_nonecc` has no callee-saved registers, so an arm
/// that jumps to the next owes nothing to restore.
///
/// A `musttail` call may cross an arity or a type only under `tailcc`. Under
/// any other convention the caller and callee prototypes must match, so each
/// set of functions a `musttail` joins takes one signature: every parameter
/// flattened to `i64` words, padded with `poison` to the widest member. The
/// widest decoder arm is nine words and the convention passes twelve in
/// registers, so nothing goes on the stack. A set that is wider than that, or
/// holds a parameter of a type not flattened here, or a member whose address
/// is taken, keeps `tailcc`.
fn preserve_none_tails(ir: String) -> String {
    let defines: std::collections::HashMap<String, TailDefine> =
        ir.lines().filter_map(tail_define).collect();
    let names: Vec<&String> = defines.keys().collect();
    let index: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let mut parent: Vec<usize> = (0..names.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    // A set is spoiled by any use of a member that is not its header or a
    // call, and by a `define tailcc` this does not parse.
    let mut spoiled = vec![false; names.len()];
    let mut header_spoils = false;
    let mut here: Option<usize> = None;
    for line in ir.lines() {
        if line.starts_with("define ") || line.starts_with("declare ") {
            here = line
                .find(" @")
                .map(|at| symbol_at(line, at + 1).0)
                .and_then(|n| index.get(n.as_str()).copied());
            if line.starts_with("define tailcc ") && here.is_none() {
                header_spoils = true;
            }
            if line.starts_with("declare tailcc ") {
                header_spoils = true;
            }
        }
        let call = tail_call(line);
        if let (Some((_, _, callee)), Some(from)) = (&call, here) {
            if line.contains("musttail call tailcc ") {
                match index.get(callee.as_str()) {
                    Some(&to) => {
                        let (a, b) = (root(&mut parent, from), root(&mut parent, to));
                        parent[a] = b;
                    }
                    None => spoiled[from] = true,
                }
            }
        }
        let header_at = match line.starts_with("define ") {
            true => line.find(" @").map(|at| at + 1),
            false => None,
        };
        let call_at = call.as_ref().map(|(_, at, _)| *at);
        for (at, _) in line.match_indices('@') {
            if Some(at) == header_at || Some(at) == call_at {
                continue;
            }
            if let Some(&i) = index.get(symbol_at(line, at).0.as_str()) {
                spoiled[i] = true;
            }
        }
    }
    if header_spoils || names.is_empty() {
        return ir;
    }
    // Each set's width, its return type, and whether it may be rewritten.
    let mut width = vec![0usize; names.len()];
    let mut ret: Vec<Option<&str>> = vec![None; names.len()];
    let mut ok = vec![true; names.len()];
    for (i, name) in names.iter().enumerate() {
        let r = root(&mut parent, i);
        let define = &defines[name.as_str()];
        width[r] = width[r].max(define.words);
        ok[r] &= !spoiled[i];
        match ret[r] {
            None => ret[r] = Some(&define.ret),
            Some(seen) => ok[r] &= seen == define.ret,
        }
    }
    let flat: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            let r = root(&mut parent, i);
            (ok[r] && width[r] <= PRESERVE_NONE_REGISTERS).then_some((n.as_str(), width[r]))
        })
        .collect();
    if flat.is_empty() {
        return ir;
    }
    let mut out = String::with_capacity(ir.len() + ir.len() / 8);
    let mut fresh = 0usize;
    let mut unpack: Option<String> = None;
    for line in ir.lines() {
        if let Some(lines) = unpack.take() {
            // after the entry label when there is one
            match line.trim_end().ends_with(':') {
                true => {
                    out.push_str(line);
                    out.push('\n');
                    out.push_str(&lines);
                    continue;
                }
                false => out.push_str(&lines),
            }
        }
        if let Some((name, define)) =
            tail_define(line).filter(|(n, _)| flat.contains_key(n.as_str()))
        {
            let words = flat[name.as_str()];
            let mut signature = Vec::with_capacity(words);
            let mut lines = String::new();
            for (ty, value) in &define.params {
                match ty.as_str() {
                    "i64" => signature.push(format!("i64 {value}")),
                    "ptr" => {
                        signature.push(format!("i64 {value}.w"));
                        lines.push_str(&format!("  {value} = inttoptr i64 {value}.w to ptr\n"));
                    }
                    "double" => {
                        signature.push(format!("i64 {value}.w"));
                        lines.push_str(&format!("  {value} = bitcast i64 {value}.w to double\n"));
                    }
                    _ => {
                        signature.push(format!("i64 {value}.w0"));
                        signature.push(format!("i64 {value}.w1"));
                        lines.push_str(&format!(
                            "  {value}.half = insertvalue {ty} poison, i64 {value}.w0, 0\n  \
                             {value} = insertvalue {ty} {value}.half, i64 {value}.w1, 1\n"
                        ));
                    }
                }
            }
            for pad in signature.len()..words {
                signature.push(format!("i64 %pad{pad}"));
            }
            let at = line.find(" @").unwrap() + 1;
            let (_, after) = symbol_at(line, at);
            let close = closing(line, after).unwrap();
            out.push_str("define preserve_nonecc ");
            out.push_str(&line["define tailcc ".len()..after]);
            out.push('(');
            out.push_str(&signature.join(", "));
            out.push(')');
            out.push_str(&line[close..]);
            out.push('\n');
            unpack = Some(lines);
            continue;
        }
        if let Some((keyword, at, callee)) = tail_call(line) {
            if let Some(&words) = flat.get(callee.as_str()) {
                let (_, after) = symbol_at(line, at);
                let close = closing(line, after).unwrap();
                let indent = &line[..line.len() - line.trim_start().len()];
                let mut args = Vec::with_capacity(words);
                for item in top_level_items(&line[after + 1..close - 1]) {
                    let (ty, value) = typed(item).expect("an argument of a flattened callee");
                    match ty {
                        "i64" => args.push(format!("i64 {value}")),
                        "ptr" | "double" => {
                            fresh += 1;
                            let cast = if ty == "ptr" { "ptrtoint" } else { "bitcast" };
                            out.push_str(&format!(
                                "{indent}%pn{fresh} = {cast} {ty} {value} to i64\n"
                            ));
                            args.push(format!("i64 %pn{fresh}"));
                        }
                        _ => {
                            for half in 0..2 {
                                fresh += 1;
                                out.push_str(&format!(
                                    "{indent}%pn{fresh} = extractvalue {ty} {value}, {half}\n"
                                ));
                                args.push(format!("i64 %pn{fresh}"));
                            }
                        }
                    }
                }
                while args.len() < words {
                    args.push("i64 poison".to_string());
                }
                out.push_str(&line[..keyword]);
                out.push_str("preserve_nonecc ");
                out.push_str(&line[keyword + "tailcc ".len()..after]);
                out.push('(');
                out.push_str(&args.join(", "));
                out.push(')');
                out.push_str(&line[close..]);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The functions the runtime calls by name, which keep the C convention its
/// own calls use: every `extern` it declares.
fn runtime_externs(runtime: &str) -> std::collections::HashSet<String> {
    runtime
        .lines()
        .filter(|l| l.starts_with("extern ") && l.contains('('))
        .filter_map(|l| {
            let head = &l[..l.find('(')?];
            let name = head.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).next()?;
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// Every other function the program defines and calls only directly takes
/// `preserve_nonecc` too.
///
/// A function the program calls in the ordinary way saved the callee-saved
/// registers it used, whether or not its caller had anything live in them.
/// Under this convention the caller saves what it keeps live across the call
/// and the callee saves nothing. Nothing outside the module can call these:
/// what the runtime calls by name it declares `extern`, and those keep the C
/// convention, as does `main`, anything whose address is taken, and the
/// module's own copies of runtime helpers, whose names begin `k_`.
fn preserve_none_calls(ir: String, runtime: &str) -> String {
    let externs = runtime_externs(runtime);
    let plain = |line: &str| -> Option<String> {
        let rest = line.strip_prefix("define ")?;
        if rest.contains("preserve_nonecc") || rest.contains("tailcc ") {
            return None;
        }
        let at = line.find(" @")? + 1;
        let name = symbol_at(line, at).0;
        let kept = name == "main" || name.starts_with("k_") || externs.contains(&name);
        (!kept).then_some(name)
    };
    let mut chosen: std::collections::HashSet<String> = ir.lines().filter_map(plain).collect();
    if chosen.is_empty() {
        return ir;
    }
    // A direct call reaches its callee through `call <type> @name(`; any other
    // mention of a chosen name is its address, and it keeps its convention.
    let callee_at = |line: &str| -> Option<usize> {
        let call = line.find("call ")? + "call ".len();
        let at = call + line[call..].find(" @")? + 1;
        // one word between: the return type, and no convention before it
        (line[call..at].trim().split(' ').count() == 1).then_some(at)
    };
    let mut taken = Vec::new();
    for line in ir.lines() {
        let header = match line.starts_with("define ") {
            true => line.find(" @").map(|at| at + 1),
            false => None,
        };
        let call = callee_at(line);
        for (at, _) in line.match_indices('@') {
            if Some(at) == header || Some(at) == call {
                continue;
            }
            let name = symbol_at(line, at).0;
            if chosen.contains(&name) {
                taken.push(name);
            }
        }
    }
    for name in taken {
        chosen.remove(&name);
    }
    let mut out = String::with_capacity(ir.len() + ir.len() / 64);
    for line in ir.lines() {
        if plain(line).is_some_and(|n| chosen.contains(&n)) {
            // the convention follows the linkage, when there is one
            let rest = &line["define ".len()..];
            let linkage = ["internal ", "private "].into_iter().find(|l| rest.starts_with(l));
            let (linkage, rest) = match linkage {
                Some(l) => (l, &rest[l.len()..]),
                None => ("", rest),
            };
            out.push_str("define ");
            out.push_str(linkage);
            out.push_str("preserve_nonecc ");
            out.push_str(rest);
        } else if let Some(at) =
            callee_at(line).filter(|at| chosen.contains(&symbol_at(line, *at).0))
        {
            let call = line[..at].rfind("call ").unwrap() + "call ".len();
            out.push_str(&line[..call]);
            out.push_str("preserve_nonecc ");
            out.push_str(&line[call..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Release: whole-program LTO across the program and a freshly compiled
/// runtime — the slowest build and the fastest binary.
/// Whether this host's clang can take `preserve_none` on a closure body.
///
/// PROBED WITH THE .ll FORM, and deliberately not with the C attribute: the
/// two fail in opposite ways. clang 18 rejects `preserve_nonecc` at the
/// parser and merely WARNS about `__attribute__((preserve_none))`, so a probe
/// built on the C half answers yes on a toolchain that would then refuse the
/// module the emitter writes. The C side carries
/// `-Werror=unknown-attributes` for the same reason, so a wrong answer breaks
/// the build at the first define rather than producing a binary whose two
/// halves disagree about registers.
///
/// Asked once per process. The emitted IR and the runtime object must agree,
/// so `cached_runtime_object` keys on this too.
fn closure_convention() -> kanso::codegen::ClosureConvention {
    use kanso::codegen::ClosureConvention;
    static ANSWER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    match *ANSWER.get_or_init(remembered_probe) {
        true => ClosureConvention::PreserveNone,
        false => ClosureConvention::Absent,
    }
}

/// The probe's answer, kept on disk beside the runtime objects.
///
/// The answer belongs to the clang binary, not to the process asking, and the
/// probe is a whole clang run: 32,201,483 instructions, 6.4% of a dev build
/// of the codegen corpus, paid by every build to learn what the last one
/// learned. It is keyed by the clang the PATH resolves to, followed through
/// its symlinks, with that file's size and modification time, so installing
/// another clang asks again. A key that cannot be formed, or a file that holds
/// anything but the one byte this writes, means asking.
fn remembered_probe() -> bool {
    let Some(identity) = clang_identity() else { return preserve_none_probe() };
    let key = identity
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let path = std::env::temp_dir().join(format!("kanso_pn_answer_{key:016x}"));
    match std::fs::read(&path).ok().as_deref() {
        Some(b"1") => return true,
        Some(b"0") => return false,
        _ => {}
    }
    let answer = preserve_none_probe();
    // Written under a name of its own and renamed into place, so a build
    // running beside this one reads the whole byte or nothing.
    let staged = std::env::temp_dir().join(format!("kanso_pn_answer_{key:016x}_{}", pid_tag()));
    if std::fs::write(&staged, if answer { b"1" } else { b"0" }).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
    answer
}

/// The clang this process would run, as a path followed through its links,
/// with its size and modification time.
fn clang_identity() -> Option<String> {
    tool_identity("clang")
}

/// Whether clang can hand an LTO link to lld, asked once per pair of tools.
///
/// lld links a release build for 4.26% fewer instructions than GNU ld with
/// LLVM's plugin, measured on the codegen corpus on this container:
/// 1,750,778,100 against 1,676,140,277. Nearly all of the difference is the
/// linker's own work -- the plugin process spent 128,765,719 in libc and
/// 52,810,020 in libbfd around LLVM's 926,724,119 -- and the program it
/// produces runs within 281 instructions of GNU ld's on runbench.
///
/// It is asked rather than assumed because an lld from another LLVM release
/// than clang's cannot read clang's bitcode, and a runner can carry one: the
/// probe links a one-line LTO program, and anything short of success keeps
/// GNU ld. The answer is remembered under clang's identity and that of the
/// `ld.lld` on PATH, if there is one.
fn lld_links_lto() -> bool {
    // clang finds lld in its own LLVM directory as well as on PATH, so what
    // is on PATH is part of the key and not a condition: a runner with lld
    // only beside clang answered the spec's probe yes while this said no
    // without asking.
    let Some(clang) = tool_identity("clang") else {
        return false;
    };
    let lld = tool_identity("ld.lld").unwrap_or_default();
    let key = format!("{clang}|{lld}")
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let path = std::env::temp_dir().join(format!("kanso_lld_answer_{key:016x}"));
    match std::fs::read(&path).ok().as_deref() {
        Some(b"1") => return true,
        Some(b"0") => return false,
        _ => {}
    }
    let answer = lld_probe();
    let staged = std::env::temp_dir().join(format!("kanso_lld_answer_{key:016x}_{}", pid_tag()));
    if std::fs::write(&staged, if answer { b"1" } else { b"0" }).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
    answer
}

/// The arguments that put a link in lld, or none.
///
/// lld links on every hardware thread by default, which a user's build wants
/// and a measurement cannot have: under callgrind the thread pool's
/// scheduling lands in the count, and the codegen rows read 434,345,526 and
/// 434,337,763 on two CI runs of one tree, and 1,677,317,287 and 1,677,792,392
/// on release. Three dev links here read 434,957,073, 434,928,291 and
/// 434,926,291 on the default and 434,600,869 three times with `--threads=1`.
/// `KANSO_LTO_JOBS` is already how a measurement asks for one LTO job, so it
/// sets lld's thread count as well.
fn lld_args() -> Vec<String> {
    if !(cfg!(target_os = "linux") && lld_links_lto()) {
        return Vec::new();
    }
    let mut args = vec!["-fuse-ld=lld".to_string()];
    if let Ok(n) = std::env::var("KANSO_LTO_JOBS") {
        if !n.is_empty() {
            args.push(format!("-Wl,--threads={n}"));
        }
    }
    args
}

fn lld_probe() -> bool {
    let dir = std::env::temp_dir();
    let ll = dir.join(format!("kanso_lld_probe_{}.ll", pid_tag()));
    let out = dir.join(format!("kanso_lld_probe_{}", pid_tag()));
    if std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").is_err() {
        return false;
    }
    // Output thrown away rather than read, for the reason `preserve_none_probe`
    // gives.
    let ok = std::process::Command::new("clang")
        .args(["-O1", "-flto", "-fuse-ld=lld", "-Wl,-plugin-opt=O3", "-Wno-override-module"])
        .arg(&ll)
        .arg("-o")
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&out);
    ok
}

/// A tool on PATH, as a path followed through its links, with its size and
/// modification time.
fn tool_identity(name: &str) -> Option<String> {
    let dirs = std::env::var_os("PATH")?;
    let found = std::env::split_paths(&dirs).map(|d| d.join(name)).find(|p| p.is_file())?;
    let real = std::fs::canonicalize(&found).ok()?;
    let meta = std::fs::metadata(&real).ok()?;
    let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(format!("{}:{}:{}", real.display(), meta.len(), modified.as_nanos()))
}

/// The process id, always the same number of characters.
///
/// EVERY TEMP PATH THIS FILE BUILDS CARRIES ONE, and a path's LENGTH is a term
/// in what the process costs: the bytes are copied, walked and handed to
/// `open`. A pid is anywhere from one digit to seven, so two runs of one binary
/// on one box wrote paths of different lengths and counted different
/// instructions for doing the same thing. On 2026-09-17 that was the whole of
/// what the codegen row's second reading still disagreed by once the warm-up
/// was fixed: 120 instructions out of 1,071,604,729 on the dev tier and 582 out
/// of 7,307,728,731 on release, on one binary in one job.
///
/// Seven digits covers every pid Linux hands out under the default
/// `pid_max` of 4,194,304. A larger `pid_max` widens the field rather than
/// truncating it, so uniqueness is never at risk; the length guarantee is,
/// and `tests/a_temp_path_is_the_same_length_every_run.rs` says so in the
/// same breath as it pins the width.
fn pid_tag() -> String {
    pid_tag_of(std::process::id())
}

/// Split out from `pid_tag` so the width can be asked about a pid this process
/// does not have.
fn pid_tag_of(pid: u32) -> String {
    format!("{pid:07}")
}

/// Compile a two-define module: one carrying the convention, one calling
/// through it. Both halves are there because a toolchain that parsed the
/// define and refused the call site would still refuse what the emitter
/// writes, and a probe testing only the define would not know.
fn preserve_none_probe() -> bool {
    let dir = std::env::temp_dir();
    let ll = dir.join(format!("kanso_pn_probe_{}.ll", pid_tag()));
    let obj = dir.join(format!("kanso_pn_probe_{}.o", pid_tag()));
    let module = "define preserve_nonecc i64 @p(i64 %x) { ret i64 %x }\n\
                  define i64 @q(i64 %x) {\n\
                  \x20 %r = call preserve_nonecc i64 @p(i64 %x)\n\
                  \x20 ret i64 %r\n\
                  }\n";
    if std::fs::write(&ll, module).is_err() {
        return false;
    }
    // `status()` WITH THE CHILD'S OUTPUT THROWN AWAY, not `output()`, and the
    // difference is measurable. Only `status.success()` was ever read here, but
    // `output()` opens pipes and drains them, and how many `poll` and `read`
    // calls that takes depends on when the child's bytes arrive rather than on
    // anything the compiler did. Measured 2026-09-17: two runs of one binary on
    // one box, same corpus, same environment, counted 1,016,046,470 and
    // 1,016,048,745 for `kanso build`, and eight frames of ten thousand six
    // hundred and forty-two accounted for every instruction of the difference:
    // `FileDesc::read_to_end` +903, `small_probe_read` +591, `read_output`
    // +253, `read` +220, `__memcpy_avx` +176, `poll` +88, `__errno_location`
    // +44. All of it is the pipe-draining loop. With no pipe there is no loop.
    let ok = std::process::Command::new("clang")
        .args(["-Wno-override-module", "-c"])
        .arg(&ll)
        .arg("-o")
        .arg(&obj)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&obj);
    ok
}

fn release_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    // THE RUNTIME IS LINKED AS MACHINE CODE, NOT AS BITCODE. It was `-O3
    // -flto`, which put the whole runtime through the LTO link beside the
    // program on every release build, and the runtime is three quarters of
    // what that link generates code for: on the codegen corpus 59,082 bytes
    // of runtime `.text` against 19,135 of program. Built native, the release
    // row reads 2,848,583,206 against 6,598,715,476, -56.8%, and the run
    // program pays +3.38%, almost all of it three helpers the link used to
    // inline -- `k_b_find2_raw`, `k_b_find2_below_raw` and `k_beat_iter`.
    // The objective weighs the two at about +1.6 and -0.26. ThinLTO was the
    // other way to shrink this link and was declined at -3.47% for +3.03%;
    // design/compiler-log.md has both.
    //
    // The runtime inlines at the program's threshold. The LTO link applied
    // `-inline-threshold=2000` to the runtime as well while the runtime was
    // bitcode, and compiled on its own it fell back to clang's 225: helpers
    // such as `k_b_push_grow` stopped being inlined into their callers inside
    // the runtime. On runbench the ladder read 1,811,839,413 at 225,
    // 1,809,106,361 at 1000, 1,806,069,074 at 2000 and 1,805,972,054 at 4000.
    // The object is compiled once and cached, so the build pays it once.
    let runtime_obj = cached_runtime_object(
        "release",
        &["-O3", "-mllvm", "-inline-threshold=2000", "-DK_HOT_ELSEWHERE"],
    )?;
    let hot_obj = cached_object("release_hot", &hot_source(), &["-O3", "-flto"])?;
    std::process::Command::new("clang")
        // THE PROGRAM IS OPTIMIZED ONCE, AT THE LINK. `-O3` here ran the full
        // pipeline on the program's IR as clang -cc1 turned it into bitcode,
        // and the LTO link then ran it again over program and hot helpers
        // together. `-O1` before the link and `plugin-opt=O3` at it, on this
        // container: the release codegen row 2,903,108,801 -> 1,751,444,900,
        // -39.7%, and the run program 1,806,069,074 -> 1,895,750,256, +4.97%.
        // `-O2` read 2,536,822,676 and 1,815,479,975; no pre-link passes at
        // all read 1,380,698,213 and 2,053,358,047, +13.7%, because the link's
        // pipeline expects its input already simplified. Scored by the
        // objective against main's goldens, -O1 is +0.19 and -O2 +0.11.
        //
        // On Linux only. `-plugin-opt` is the gold plugin's spelling and
        // Apple's ld64 refuses it ("ld: unknown options: -plugin-opt=O3"),
        // so elsewhere both steps stay at -O3 as they were.
        .args(if cfg!(target_os = "linux") {
            &["-O1", "-Wl,-plugin-opt=O3"][..]
        } else {
            &["-O3"][..]
        })
        .arg("-flto")
        // lld where it can take the LTO link: `lld_links_lto` says why.
        .args(lld_args())
        // Eight times clang's default of 250. The run program spends one
        // instruction in ten on `push`, `pop` and `ret` -- 215,229,225 of
        // 2,185,625,151 in the binary's own code -- and the functions paying
        // most are small and hot rather than looping: k_map_sorted is 49.6%
        // frame, string_at and entry_onto 20.0% each. Inlining is what
        // removes a frame, so the threshold is the lever.
        //
        // THE LADDER IS NOT MONOTONE, which is why the value here was found
        // by measuring rather than by reasoning. On runbench.ll against 1000:
        // 1250 reads -0.9473%, 1500 reads -0.8295% -- WORSE than 1250 while
        // costing 10,336 more bytes -- 2000 reads -2.0948% and 3000 reads
        // -2.3746%. A wider threshold admits a different SET of inlinings,
        // and a superset of decisions is not a better program, so no value
        // between two measured ones may be assumed to lie between them.
        //
        // 2000 is taken because 3000 costs 40.5% of .text for a further
        // 0.28%, where 2000 costs 13.2%. Those are this container's ladder,
        // taken by linking runbench.ll and counting; CI's own work row read
        // -1.2281% for the same step, 59% of what the ladder projected, so
        // the ladder orders the rungs and does not size them.
        //
        // Machine-code size has no welfare term -- Clay ruled that on
        // 2026-09-05 -- but `.text` keeps its own exact vein, so the growth
        // is watched even though it is not scored.
        //
        // WHICH MEANS THE .TEXT REASON ABOVE IS NOT WHY 2000 STANDS, and on
        // 2026-09-19 the scored reason was measured. The ladder carries two
        // more rungs, taken the same way, on a runbench.ll byte-identical
        // across every arm:
        //
        //     225    1,938,999,983   343,128 bytes   22,317,482 calls
        //     2000   1,840,276,313   424,088         18,812,341
        //     4000   1,823,291,354   510,152
        //     8000   1,821,134,592   567,496
        //
        // 4000 is -0.923% of the run and 8000 a further -0.118%. What stops
        // 4000 is `codegen_instructions_release`, which is a PRODUCTION
        // welfare term at weight 0.15: the same box, two passes, each arm
        // byte-identical, reads 6,827,333,184 against 7,075,918,942, a rise
        // of 3.64%. The dev tier does not move at all (595,943,218 both
        // arms), because `dev_clang` passes -O0 and no -mllvm.
        //
        // Scored against the goldens by ratio, the run gain alone is 77.27 ->
        // 77.33 and the pair together is 77.26: one hundredth BELOW the
        // floor. The break-even, worked before the codegen arm finished, was
        // a release build 3.2% dearer, and it came in at 3.64%. And the
        // paragraph above records that CI's work row moved 59% of what this
        // container's ladder projected for 1000 -> 2000; at that travel the
        // run gain is nearer 0.54% and the trade is not close.
        //
        // So 2000 stands on the objective rather than on `.text`, and 4000 is
        // measured-and-declined. design/compiler-log.md carries the frame
        // measurement this came out of.
        .arg("-mllvm")
        .arg("-inline-threshold=2000")
        .args(if cfg!(target_arch = "x86_64") { &["-mssse3"][..] } else { &[][..] })
        // HOW MANY THREADS THE LINKER'S LTO MAY USE, when a measurement asks.
        // Unset -- which is every build but a gate's -- the plugin picks, and a
        // release build keeps every core it can get.
        //
        // The gate asks for one, because a row cannot be pinned to a number the
        // scheduler helps choose. `ld` splits LTO codegen across threads,
        // callgrind counts every thread, and how the work lands is not a
        // property of the input: two links of byte-identical bitcode on this
        // container read 20,565,047,254 and 20,565,047,243 on one pair and
        // 20,565,047,241 and 20,565,049,584 on the next, while both `clang`
        // children came back byte for byte every time. With `jobs=1` the same
        // pair reads 20,574,502,681 twice.
        //
        // That is the 2026-09-15 rule applied where it belongs: the state is
        // put into a known one for the measurement rather than explained
        // afterwards. It is NOT made the default, because a user's release
        // build has no row to keep and every reason to use the cores.
        .args(match std::env::var("KANSO_LTO_JOBS") {
            Ok(n) if !n.is_empty() => vec![format!("-Wl,-plugin-opt=jobs={n}")],
            _ => vec![],
        })
        // AND THE LTO OBJECT GETS A NAME THE RUN CHOOSES, for the measurement
        // only. clang writes it to `/tmp/<stem>-XXXXXX.o` with fresh hex every
        // run; `ld`'s LLVM plugin puts that path into a `StringMap`, and the
        // probe length depends on the string. Twenty-two names were measured on
        // one binary and one corpus with every other input held: twenty read
        // 5,163,341,031 and two -- `4b8c1a` and `fedcba` -- read 5,163,341,042.
        // Eleven apart, deterministic per name, about one name in eleven.
        //
        // That is the eleven `codegen_instructions_release` has been
        // disagreeing with itself by since kanso#1507 pinned the thread count.
        // kanso#1502's own job drew both buckets: 6,820,866,344 and then
        // 6,820,866,355. About one job in eleven goes red on that row for
        // nothing a diff can explain, and three rounds were spent on it.
        //
        // `-save-temps=obj` writes the object beside the output under a name
        // derived from the input, so the string is the same every run. It is
        // the 2026-09-15 rule again: the state is put into a known one rather
        // than explained afterwards. NOT the default, for the same reason
        // `KANSO_LTO_JOBS` is not -- it leaves a file in a user's directory and
        // a user's build has no row to keep.
        .args(match std::env::var("KANSO_FIXED_TEMPS") {
            Ok(v) if !v.is_empty() => vec!["-save-temps=obj"],
            _ => vec![],
        })
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&hot_obj)
        .arg(&runtime_obj)
        .arg("-lm")
        .without_driver()
}

/// Dev (the default): the program compiles unoptimized and links against a
/// cached optimized runtime object, so the runtime's cost is paid once per
/// runtime version, not per build.
fn dev_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    let runtime_obj = cached_runtime_object("dev", &["-O2"])?;
    std::process::Command::new("clang")
        .arg("-O0")
        // The dev link has no LTO in it, and lld still halves it: on the
        // codegen corpus GNU ld spent 85,738,887 instructions and lld
        // 45,154,514. The same probe decides, since an lld that can take an
        // LTO link can take a plain one.
        .args(lld_args())
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&runtime_obj)
        .arg("-lm")
        .without_driver()
}

/// A clang command that runs the jobs its driver would have run, without the
/// driver.
///
/// Every LLVM process spends most of its start-up relocating libLLVM: the
/// driver of a dev build is 31,702,963 instructions, 83% of them in the
/// dynamic loader, and it does nothing a build needs beyond deciding the two
/// commands it then spawns -- `clang -cc1` and the linker. Those commands
/// depend on the toolchain, the flags and the file names, and on nothing a
/// program's IR says. So the driver is asked once, with `-###`, for a build
/// whose files carry placeholder names in a directory of their own; the
/// answer is kept under a key naming the tools, the flags and the objects;
/// and every later build runs the two commands with its own names put back.
///
/// Anything unexpected -- a driver that prints other than two jobs, a key
/// that cannot be formed, a job whose program is missing -- runs the driver
/// as before. `KANSO_CLANG_DRIVER` forces the driver, which is how the spec
/// compares the two.
trait WithoutDriver {
    fn without_driver(&mut self) -> std::io::Result<std::process::ExitStatus>;
}

impl WithoutDriver for std::process::Command {
    fn without_driver(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let args: Vec<String> = self.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        if cfg!(target_os = "linux") && std::env::var_os("KANSO_CLANG_DRIVER").is_none() {
            if let Some(status) = replayed(&args) {
                return status;
            }
        }
        self.status()
    }
}

const IN_MARK: &str = "kanso_replay_in";
const OUT_MARK: &str = "kanso_replay_out";

/// The two jobs for `args`, run directly. None when the replay cannot be
/// trusted, and the caller runs the driver.
fn replayed(args: &[String]) -> Option<std::io::Result<std::process::ExitStatus>> {
    // `-save-temps=obj` is only ever asked for to give the LTO object a name
    // that is the same every run, because ld's plugin hashes that path and a
    // random one moved the release row by eleven instructions one run in
    // eleven. The replay's names are fixed already, so it leaves the option to
    // the driver, which still needs it.
    let args: Vec<String> = args.iter().filter(|a| *a != "-save-temps=obj").cloned().collect();
    // Exactly one `-o`, and the input is the argument after it: that is the
    // only shape `dev_clang` and `release_clang` hand over, and anything else
    // goes to the driver.
    let o = args.iter().position(|a| a == "-o")?;
    if args.iter().filter(|a| *a == "-o").count() != 1
        || args.iter().any(|a| a.starts_with("-save-temps"))
    {
        return None;
    }
    let out = std::path::Path::new(args.get(o + 1)?);
    let ll = std::path::Path::new(args.get(o + 2)?);
    let ll_name = ll.file_stem()?.to_str()?;
    let out_name = out.file_name()?.to_str()?;
    let cwd = std::env::current_dir().ok()?;
    let ll_abs = cwd.join(ll);
    let out_abs = cwd.join(out);
    let mut shape: Vec<String> = args.to_vec();
    shape[o + 1] = OUT_MARK.to_string();
    shape[o + 2] = format!("{IN_MARK}.ll");

    let identity = format!(
        "{}|{}|{}|{}|{}",
        tool_identity("clang")?,
        tool_identity("ld.lld").unwrap_or_default(),
        shape.join("\u{0}"),
        std::env::var("LIBRARY_PATH").unwrap_or_default(),
        std::env::var("COMPILER_PATH").unwrap_or_default(),
    );
    let key = kanso::hash::digest_of(identity.as_bytes());
    let cache = std::env::temp_dir().join(format!("kanso_jobs_{:016x}{:016x}", key.0, key.1));

    // Named for the output rather than the process. The jobs run inside it,
    // and lld's LTO reads the directory it runs in into what it hashes: with a
    // stage named for the pid, three runs of one release build read
    // 1,644,357,816 and 1,644,922,716 apart. Two builds writing the same
    // output were already racing for it, so sharing a stage costs nothing new.
    let named = kanso::hash::digest_of(out_abs.to_string_lossy().as_bytes());
    let stage = std::env::temp_dir().join(format!("kanso_stage_{:016x}", named.0));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).ok()?;
    let stage_str = stage.to_str()?.to_string();
    let jobs = match std::fs::read_to_string(&cache).ok().and_then(|t| jobs_of(&t)) {
        Some(jobs) => jobs,
        None => {
            let jobs = asked(&shape, &stage, &stage_str)?;
            let staged = std::env::temp_dir().join(format!(
                "kanso_jobs_{:016x}{:016x}_{}",
                key.0,
                key.1,
                pid_tag()
            ));
            let text: Vec<String> = jobs.iter().map(|j| j.join("\u{0}")).collect();
            if std::fs::write(&staged, text.join("\n")).is_ok() {
                let _ = std::fs::rename(&staged, &cache);
            }
            jobs
        }
    };
    let _ = std::fs::remove_file(stage.join(format!("{IN_MARK}.ll")));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&ll_abs, stage.join(format!("{ll_name}.ll"))).ok()?;
    let put_back = |a: &str| a.replace(IN_MARK, ll_name).replace(OUT_MARK, out_name);
    let mut status = None;
    for (n, job) in jobs.iter().enumerate() {
        let mut argv: Vec<String> = job.iter().map(|a| put_back(a)).collect();
        // The link writes where the build asked, not into the stage.
        if n + 1 == jobs.len() {
            let at = argv.iter().position(|a| a == "-o")?;
            argv[at + 1] = out_abs.to_str()?.to_string();
        }
        let ran =
            std::process::Command::new(&argv[0]).args(&argv[1..]).current_dir(&stage).status();
        let failed = !matches!(&ran, Ok(s) if s.success());
        status = Some(ran);
        if failed {
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&stage);
    status
}

/// The driver's jobs for `shape`, with the stage and the driver's own temporary
/// object replaced by marks. None unless there are exactly two and both name a
/// program that exists.
fn asked(shape: &[String], stage: &std::path::Path, stage_str: &str) -> Option<Vec<Vec<String>>> {
    // The driver refuses an input that is not there, even when only asked.
    std::fs::write(stage.join(format!("{IN_MARK}.ll")), "").ok()?;
    // The driver prints its jobs on stderr. They go to a file rather than a
    // pipe, so no read loop that scheduling can lengthen lands in the row.
    let listing = stage.join("jobs");
    let said = std::process::Command::new("clang")
        .arg("-###")
        .args(shape)
        .current_dir(stage)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&listing).ok()?)
        .status()
        .ok()?;
    if !said.success() {
        return None;
    }
    let text = std::fs::read_to_string(&listing).ok()?;
    let mut jobs: Vec<Vec<String>> =
        text.lines().filter(|l| l.starts_with(" \"")).map(quoted_args).collect::<Option<_>>()?;
    if jobs.len() != 2
        || jobs.iter().any(|j| j.is_empty() || !std::path::Path::new(&j[0]).is_file())
    {
        return None;
    }
    // The object cc1 writes and the linker reads: the driver names it in its
    // temp directory with a random suffix, and the replay names it beside the
    // input, relative to the stage both jobs run in. NO PATH OF THE STAGE'S
    // REACHES A JOB. The stage is named for the process, and ld's LLVM plugin
    // hashes the object's path, so a path carrying the pid would put back the
    // eleven-instruction wobble `-save-temps=obj` exists to remove. The two
    // compilation directories are the stage's too, and are only written into
    // debug information, which these builds do not ask for; they are `.`.
    let at = jobs[0].iter().position(|a| a == "-o")?;
    let temp_obj = jobs[0].get(at + 1)?.clone();
    let ours = format!("{IN_MARK}.o");
    for job in &mut jobs {
        for a in job.iter_mut() {
            *a = a.replace(&temp_obj, &ours);
            for dir in ["-fdebug-compilation-dir=", "-fcoverage-compilation-dir="] {
                if a.starts_with(dir) {
                    *a = format!("{dir}.");
                }
            }
        }
    }
    if jobs.iter().flatten().any(|a| a.contains(stage_str)) {
        return None;
    }
    Some(jobs)
}

/// One `-###` line: arguments in double quotes, with `\` escaping the next
/// character.
fn quoted_args(line: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut chars = line.chars();
    loop {
        match chars.next() {
            None => return Some(args),
            Some(' ') => continue,
            Some('"') => {
                let mut arg = String::new();
                loop {
                    match chars.next()? {
                        '\\' => arg.push(chars.next()?),
                        '"' => break,
                        c => arg.push(c),
                    }
                }
                args.push(arg);
            }
            Some(_) => return None,
        }
    }
}

fn jobs_of(text: &str) -> Option<Vec<Vec<String>>> {
    let jobs: Vec<Vec<String>> =
        text.split('\n').map(|j| j.split('\u{0}').map(str::to_string).collect()).collect();
    match jobs.len() == 2 && jobs.iter().all(|j| std::path::Path::new(&j[0]).is_file()) {
        true => Some(jobs),
        false => None,
    }
}

/// What names a cached runtime object: everything that decides what clang
/// makes of the one source. Two builds that differ in any of these must not
/// share an object.
fn runtime_key(profile: &str, opt: &[&str], preserve: bool, counting: bool) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    // THE SOURCE'S DIGEST, NOT THE SOURCE. `runtime.c` is a constant of this
    // binary, so what it hashes to was settled by the compiler that built it;
    // walking its 450,100 bytes here cost a quarter of `kanso play`'s
    // start-up and told every process the same thing. See `hash::digest_of`.
    kanso::hash::RUNTIME_DIGEST.hash(&mut hasher);
    profile.hash(&mut hasher);
    // THE CONVENTION IS NOT IN THE SOURCE -- it is a `-D` the probe decides,
    // so a machine that gains or loses clang 19 would otherwise reuse an
    // object built under the other answer and link it against IR built under
    // the new one. The two halves disagreeing about registers is a
    // miscompile, so it goes in the key.
    preserve.hash(&mut hasher);
    // The same reason, for the same kind of reason: whether the runtime carries
    // its twenty-seven counter gates is a `-D` the caller decides, not a fact
    // in the source, so an object built one way must not be handed to a build
    // that wanted the other. A shipped binary linked against a counting runtime
    // pays for gates it can never reach; a counting binary linked against a
    // gate-free one reports zeros and the goldens all move at once.
    counting.hash(&mut hasher);
    // And the flags the object is compiled with. They were not in the key, so
    // changing a profile's flags left every machine linking the object the
    // old flags built: a bitcode runtime cached before the release build went
    // native would have been linked as bitcode for as long as it stayed in
    // the temp directory, and nothing would have said so.
    opt.hash(&mut hasher);
    hasher.finish()
}

fn cached_runtime_object(profile: &str, opt: &[&str]) -> std::io::Result<std::path::PathBuf> {
    cached_object(profile, include_str!("runtime.c"), opt)
}

/// The helpers a release build keeps in its LTO link, as a translation unit
/// of their own. The runtime itself is machine code there, and the link can
/// only inline what it holds as bitcode: `k_b_find2_raw`,
/// `k_b_find2_below_raw` and `k_beat_iter` were 161,703,385 instructions of
/// the run program's calls when the whole runtime went native. So the runtime
/// is compiled `-DK_HOT_ELSEWHERE`, which leaves those three out, and this unit
/// defines them.
///
/// The text is taken from `runtime.c` itself rather than kept in a second
/// file, because a dozen specs read these functions out of that file and a
/// copy would drift from what they check. The typedefs come the same way.
/// The declarations below are the globals and functions the helpers reach,
/// which `runtime.c` defines with external linkage for this unit's sake.
fn hot_source() -> String {
    hot_source_of(include_str!("runtime.c"))
}

fn hot_source_of(runtime: &str) -> String {
    let mut out = String::from(concat!(
        "#include <stdint.h>\n",
        "#include <stddef.h>\n",
        "#if defined(__aarch64__)\n#include <arm_neon.h>\n",
        "#elif defined(__x86_64__)\n#include <tmmintrin.h>\n#endif\n",
        "#ifdef KANSO_COUNTERS_BUILD\n#define K_COUNTING 1\n#else\n#define K_COUNTING 0\n#endif\n",
    ));
    for (start, end) in [
        ("typedef struct { char* data; int len; int cap; } KStr;", "\n"),
        ("typedef struct KBlock {", "\n"),
        ("typedef struct { KBlock* block;", " KMark;\n"),
        ("#define K_BEAT_MAX ", "\n"),
        ("typedef struct { char* data; size_t cap; size_t used; } KCarryBuf;", "} KCarry;\n"),
    ] {
        out.push_str(hot_text(runtime, start, end));
    }
    out.push_str(concat!(
        "extern KBlock* k_blocks;\n",
        "extern KStr* k_seek_str;\n",
        "extern char* k_arena;\n",
        "extern size_t k_arena_left;\n",
        "extern long long k_stat_beat_iters;\n",
        "extern long long k_stat_find2_calls;\n",
        "extern KMark k_beat_stack[K_BEAT_MAX];\n",
        "extern int k_beat_depth;\n",
        "extern KMark* k_beat_top;\n",
        "extern KMark* k_seek_under;\n",
        "extern int k_buf_dirty;\n",
        "extern KCarry k_carries[K_BEAT_MAX];\n",
        "extern long long k_live_block_bytes;\n",
        "void k_beat_push_deep(void);\n",
        "void k_beat_rewind_slow(KMark* m);\n",
        "__attribute__((noreturn, noinline)) void k_die(const char* msg);\n",
    ));
    // Each definition is found by its name and taken from the start of the
    // line that names it, attribute and all. The two scanners are matched on
    // the signature rather than on `always_inline`, because the ratchet row
    // that puts them out of line removes that attribute, and a lift keyed on
    // it found nothing and the tree stopped building: the row read UNBUILT
    // on kanso#1585.
    for start in [
        "static inline int k_tail_window(",
        "static inline void k_beat_rewind(KMark* m) {",
        "__attribute__((always_inline)) void k_beat_iter(void) {",
        "__attribute__((always_inline)) void k_beat_push(void) {",
        "long long k_b_find2_raw(const unsigned char* d,",
        "long long k_b_find2_below_raw(const unsigned char* d,",
    ] {
        out.push_str(hot_line_text(runtime, start, "\n}\n"));
    }
    out
}

/// `hot_text` from the start of the line holding `start`.
fn hot_line_text<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let at = source.find(start).unwrap_or_else(|| panic!("runtime.c no longer holds `{start}`"));
    let line = source[..at].rfind('\n').map_or(0, |n| n + 1);
    hot_text(source, &source[line..at + start.len()], end)
}

/// From `start` through the first `end` after it, inclusive. A function ends
/// at the first closing brace in column zero, which is how every definition in
/// `runtime.c` is written.
fn hot_text<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let from = source.find(start).unwrap_or_else(|| panic!("runtime.c no longer holds `{start}`"));
    let to = from
        + source[from..].find(end).unwrap_or_else(|| panic!("`{start}` has no end in runtime.c"))
        + end.len();
    &source[from..to]
}

fn cached_object(profile: &str, source: &str, opt: &[&str]) -> std::io::Result<std::path::PathBuf> {
    let preserve = closure_convention() == kanso::codegen::ClosureConvention::PreserveNone;
    let counting = kanso::codegen::counters_wanted();
    let key = runtime_key(profile, opt, preserve, counting);
    let object = std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key:016x}.o"));
    if object.exists() {
        return Ok(object);
    }
    let c_path = std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key:016x}.c"));
    std::fs::write(&c_path, source)?;
    let staging =
        std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key:016x}_{}.o", pid_tag()));
    // `-Werror=unknown-attributes` is the belt to the probe's braces: clang 18
    // only WARNS about `preserve_none` and would silently compile the runtime
    // on the C convention while the emitted IR used the other one.
    let convention: &[&str] = match preserve {
        true => &["-DKANSO_PRESERVE_NONE", "-Werror=unknown-attributes"],
        false => &[],
    };
    let counters: &[&str] = match counting {
        true => &["-DKANSO_COUNTERS_BUILD"],
        false => &[],
    };
    let status = std::process::Command::new("clang")
        .args(opt)
        .args(convention)
        .args(counters)
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
    match built_binary(program) {
        Ok(binary) => execute(&binary, program),
        Err(code) => code,
    }
}

/// `kanso play` of a file whose modules all came from this binary. The file's
/// text decides its program once the compiler that read it is fixed, so a
/// second play of the same text looks its binary up by that text and does not
/// emit at all. Emitting was two thirds of a warm `kanso play print "x"`:
/// 404,031 of the 615,803 instructions under `kanso::main`, spent writing IR
/// whose only use was to be hashed into the name of a binary already built.
///
/// The first play emits, builds through the IR's key as `run` does, and gives
/// the binary a second name under the text's key. A file that loaded a module
/// from disk has no text key, since that module can change under it, and goes
/// through `run`.
fn play(program: &ast::Program, file: &str, source: &str) -> ExitCode {
    let Some(key) = played_key(file, source) else {
        return run(program, file, source, false);
    };
    let played = std::env::temp_dir().join(format!("kanso_play_{key}"));
    if played.exists() {
        return execute(&played, program);
    }
    match built_binary(program) {
        Ok(binary) => {
            // Two plays racing to name one binary both find the name taken or
            // take it, and either way the file under it is the same build.
            let _ = std::fs::hard_link(&binary, &played);
            execute(&binary, program)
        }
        Err(code) => code,
    }
}

/// What decides a play file's binary, hashed. The IR is a function of the
/// file's name and text, the compiler, the runtime it links, the closure
/// convention the installed clang takes, whether counters are compiled in, and
/// the `KANSO_` settings the compiler reads, so each of those goes into the
/// key. The compiler is named by its own file's path, length and modification
/// time: a rebuild writes a new file, and the key moves with it.
fn played_key(file: &str, source: &str) -> Option<String> {
    if !kanso::play_file_is_self_contained() {
        return None;
    }
    let compiler = std::env::current_exe().ok()?;
    let meta = std::fs::metadata(&compiler).ok()?;
    let written = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    let mut text: Vec<u8> = Vec::with_capacity(file.len() + source.len() + 256);
    let mut part = |bytes: &[u8]| {
        text.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        text.extend_from_slice(bytes);
    };
    part(file.as_bytes());
    part(source.as_bytes());
    part(compiler.as_os_str().as_encoded_bytes());
    part(&meta.len().to_le_bytes());
    part(&written.as_nanos().to_le_bytes());
    let convention = closure_convention();
    part(&[matches!(convention, kanso::codegen::ClosureConvention::PreserveNone) as u8]);
    for (name, value) in std::env::vars_os() {
        if name.as_encoded_bytes().starts_with(b"KANSO_") {
            part(name.as_encoded_bytes());
            part(value.as_encoded_bytes());
        }
    }
    let (ka, kb) = kanso::hash::key_of(&text);
    let (ra, rb) = kanso::hash::RUNTIME_DIGEST;
    let counting = if kanso::codegen::counters_wanted() { "c" } else { "" };
    Some(format!("{:016x}{:016x}{counting}", ka ^ ra, kb ^ rb.rotate_left(17)))
}

/// The dev binary for a program: its IR emitted and looked up by the IR's key,
/// built on a miss.
fn built_binary(program: &ast::Program) -> Result<std::path::PathBuf, ExitCode> {
    let ir = match kanso::codegen::emit_ir_dev(program, closure_convention()) {
        Ok(ir) => ir,
        Err(unsupported) => {
            eprintln!("error: {unsupported}");
            return Err(ExitCode::from(2));
        }
    };
    match cached_program_binary(&ir) {
        Ok(binary) => Ok(binary),
        Err(io) => {
            eprintln!("error: cannot build: {io}");
            Err(ExitCode::FAILURE)
        }
    }
}

fn execute(binary: &std::path::Path, program: &ast::Program) -> ExitCode {
    let status = std::process::Command::new(binary).args(program_args()).status();
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
    // 128 bits of the IR, at a third of SipHash's cost: see `hash::key_of`.
    // The runtime the binary would be linked against is a constant, so its
    // digest is folded in rather than walked. Whether it counts is not: a
    // counting run and an ordinary one emit the same IR and link different
    // runtimes, so without the mark the second of them ran the first one's
    // binary, and a counting run printed no counters.
    let (ka, kb) = kanso::hash::key_of(ir.as_bytes());
    let (ra, rb) = kanso::hash::RUNTIME_DIGEST;
    let counting = if kanso::codegen::counters_wanted() { "c" } else { "" };
    let key = format!("{:016x}{:016x}{counting}", ka ^ ra, kb ^ rb.rotate_left(17));
    let binary = std::env::temp_dir().join(format!("kanso_run_{key}"));
    if binary.exists() {
        return Ok(binary);
    }
    // The process writing the IR owns the file it hands clang. Two runs of
    // the same program share a key, so a shared path let one truncate and
    // rewrite what the other's clang was already reading — which surfaces as
    // a segmentation fault inside LLVM's assembly lexer, blamed on the
    // program rather than on the race.
    let ll_path = std::env::temp_dir().join(format!("kanso_run_{key}_{}.ll", pid_tag()));
    std::fs::write(&ll_path, ir)?;
    let staging = std::env::temp_dir().join(format!("kanso_run_{key}_{}", pid_tag()));
    let ll = ll_path.to_string_lossy().into_owned();
    let out = staging.to_string_lossy().into_owned();
    let status = dev_clang(&out, &ll)?;
    if !status.success() {
        return Err(std::io::Error::other("clang failed"));
    }
    std::fs::rename(&staging, &binary)?;
    // clang has read it, and nothing reads it again: the binary beside it is
    // the cache, keyed by the same hash. Left behind, one accumulates per
    // cache MISS -- 42 KB each, and a long-lived container reached 112,000 of
    // them, which is the whole of a session's disk allowance. The staging
    // path above needs no such line because `rename` consumes it.
    let _ = std::fs::remove_file(&ll_path);
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

// THE TEST MODULE GOES LAST, and clippy insists: `items_after_test_module`
// fires on anything declared after one, and CI lints with warnings denied.
#[cfg(test)]
mod a_temp_path_is_the_same_length_every_run {
    use super::pid_tag_of;

    /// Every pid Linux hands out under the default `pid_max` renders to the
    /// same number of characters, so two runs of one binary build paths of
    /// one length.
    #[test]
    fn every_pid_under_the_default_ceiling_is_seven_characters() {
        for pid in [1u32, 2, 9, 10, 99, 100, 999, 1000, 65_535, 999_999, 4_194_303, 4_194_304] {
            let tag = pid_tag_of(pid);
            assert_eq!(
                tag.len(),
                7,
                "pid {pid} rendered {tag:?}, {} characters. A path whose length \
                 moves with the pid makes two runs of one binary count different \
                 instructions for the same work.",
                tag.len()
            );
        }
    }

    /// And it is still a pid: padding may not collide two of them.
    #[test]
    fn padding_keeps_every_pid_distinct() {
        let pids = [1u32, 10, 100, 1000, 10_000, 100_000, 1_000_000, 4_194_303];
        let tags: std::collections::BTreeSet<String> =
            pids.iter().map(|p| pid_tag_of(*p)).collect();
        assert_eq!(tags.len(), pids.len(), "padding collided two pids: {tags:?}");
    }

    /// Past the default ceiling the field widens rather than truncating. A
    /// host with a larger `pid_max` loses the length guarantee and keeps
    /// uniqueness, which is the right way round.
    #[test]
    fn a_wider_pid_widens_the_field() {
        assert_eq!(pid_tag_of(12_345_678), "12345678");
        assert_ne!(pid_tag_of(12_345_678), pid_tag_of(2_345_678));
    }
}

#[cfg(test)]
mod a_cached_runtime_is_named_by_what_built_it {
    use super::runtime_key;

    /// A runtime object built with one set of flags is not the object another
    /// set asks for. The release profile went from bitcode to machine code by
    /// changing its flags alone, and with the flags outside the key every
    /// machine that had built the bitcode object kept linking it.
    #[test]
    fn the_runtime_key_names_the_flags() {
        assert_ne!(
            runtime_key("release", &["-O3", "-flto"], false, false),
            runtime_key("release", &["-O3"], false, false),
            "two flag sets share a cached runtime object"
        );
        assert_eq!(
            runtime_key("release", &["-O3"], false, false),
            runtime_key("release", &["-O3"], false, false),
        );
    }
}

#[cfg(test)]
mod the_hot_unit_is_taken_from_the_runtime {
    use super::hot_source;

    /// Every piece the release build's bitcode unit is assembled from is
    /// still where the extraction looks for it. A signature edited in
    /// `runtime.c` without the list in `hot_source` panics here rather than
    /// in a user's release build.
    #[test]
    fn every_hot_definition_is_found() {
        let unit = hot_source();
        for name in [
            "k_tail_window",
            "k_beat_rewind",
            "k_beat_iter",
            "k_beat_push",
            "k_b_find2_raw",
            "k_b_find2_below_raw",
        ] {
            assert!(unit.contains(&format!("{name}(")), "the hot unit lacks {name}");
        }
    }

    /// The ratchet row for the two scanners removes their `always_inline`,
    /// and the release build must still find them: a lift keyed on the
    /// attribute stopped the mutated tree building, and the row read UNBUILT
    /// on kanso#1585. Watched red with the scanners' start strings keyed on
    /// the attribute again.
    #[test]
    fn the_scanners_are_found_without_their_attribute() {
        let bare = include_str!("runtime.c")
            .replace("__attribute__((always_inline)) long long k_b_find2", "long long k_b_find2");
        assert_ne!(bare, include_str!("runtime.c"), "the attribute is no longer there to remove");
        let unit = super::hot_source_of(&bare);
        for name in ["k_b_find2_raw", "k_b_find2_below_raw"] {
            assert!(
                unit.contains(&format!("\nlong long {name}(")),
                "the bare {name} was not lifted"
            );
        }
    }
}

#[cfg(test)]
mod a_tail_cycle_takes_one_flat_signature {
    use super::preserve_none_tails;

    /// Two arms of a cycle with different arities, the shape of the JSON
    /// decoder's, and an entry that calls into it the ordinary way.
    const CYCLE: &str = "define tailcc %parsed @\"d/a_2\"(%KValue %x0, i64 %x1r) {\nentry:\n  %t1 = musttail call tailcc %parsed @\"d/b_3\"(%KValue %x0, i64 %x1r, %KValue { i64 2, i64 0 })\n  ret %parsed %t1\n}\ndefine tailcc %parsed @\"d/b_3\"(%KValue %x0, i64 %x1r, %KValue %x2) {\nentry:\n  %t1 = musttail call tailcc %parsed @\"d/a_2\"(%KValue %x2, i64 %x1r)\n  ret %parsed %t1\n}\ndefine %KValue @main_entry(%KValue %v) {\nentry:\n  %p = call tailcc %parsed @\"d/a_2\"(%KValue %v, i64 1)\n  ret %KValue %v\n}\n";

    #[test]
    fn every_member_takes_the_widest_signature() {
        let out = preserve_none_tails(CYCLE.to_string());
        assert!(!out.contains("tailcc"), "{out}");
        let headers: Vec<&str> =
            out.lines().filter(|l| l.starts_with("define preserve_nonecc")).collect();
        assert_eq!(
            headers,
            [
                "define preserve_nonecc %parsed @\"d/a_2\"(i64 %x0.w0, i64 %x0.w1, i64 %x1r, i64 %pad3, i64 %pad4) {",
                "define preserve_nonecc %parsed @\"d/b_3\"(i64 %x0.w0, i64 %x0.w1, i64 %x1r, i64 %x2.w0, i64 %x2.w1) {",
            ]
        );
        // the narrow arm's call is padded to the wide one's five words
        assert!(out.contains("musttail call preserve_nonecc %parsed @\"d/a_2\"(i64 %pn"), "{out}");
        assert!(out.contains(", i64 poison, i64 poison)"), "{out}");
        // and the entry, which is not in the cycle, calls in the same way
        assert!(out.contains("%p = call preserve_nonecc %parsed @\"d/a_2\"("), "{out}");
        // the parameters are put back together before the body reads them
        assert!(out.contains("entry:\n  %x0.half = insertvalue %KValue poison, i64 %x0.w0, 0\n  %x0 = insertvalue %KValue %x0.half, i64 %x0.w1, 1\n"), "{out}");
    }

    /// A member whose address is taken may be reached by an indirect call
    /// that still passes the old signature, so its whole cycle is left alone.
    #[test]
    fn an_address_taken_member_keeps_its_cycle_as_it_was() {
        let taken = format!("{CYCLE}@table = constant ptr @\"d/b_3\"\n");
        assert_eq!(preserve_none_tails(taken.clone()), taken);
    }

    /// A parameter of a type the rewrite does not flatten leaves the cycle
    /// alone rather than guessing at its words.
    #[test]
    fn an_unflattened_parameter_keeps_its_cycle_as_it_was() {
        let odd = CYCLE.replace("(%KValue %x0, i64 %x1r) {", "(%KValue %x0, i32 %x1r) {");
        assert_eq!(preserve_none_tails(odd.clone()), odd);
    }

    /// Wider than the convention's twelve registers, the arms would pass on
    /// the stack, and they keep `tailcc`.
    #[test]
    fn a_cycle_wider_than_the_registers_keeps_tailcc() {
        let wide = "define tailcc %KValue @w(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e, %KValue %f, i64 %g) {\nentry:\n  %r = musttail call tailcc %KValue @w(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e, %KValue %f, i64 %g)\n  ret %KValue %r\n}\n";
        assert_eq!(preserve_none_tails(wide.to_string()), wide);
    }
}

#[cfg(test)]
mod a_direct_call_saves_what_its_caller_keeps {
    use super::{preserve_none_calls, runtime_externs};

    const RUNTIME: &str = "extern KValue d_thunk_eval(long long site, KValue* args);\n\
                           extern const char* k_type_field_name(long long type_id, long long i);\n";

    const MODULE: &str = "define %KValue @\"d_m/f_1\"(%KValue %x0) {\nentry:\n  %r = call %KValue @\"d_m/g_1\"(%KValue %x0)\n  ret %KValue %r\n}\ndefine internal %KValue @\"d_m/g_1\"(%KValue %x0) {\nentry:\n  ret %KValue %x0\n}\ndefine %KValue @d_thunk_eval(i64 %site, ptr %args) {\nentry:\n  %r = call %KValue @\"d_m/f_1\"(%KValue zeroinitializer)\n  ret %KValue %r\n}\ndefine %KValue @k_user_main() {\nentry:\n  %r = call %KValue @\"d_m/f_1\"(%KValue zeroinitializer)\n  ret %KValue %r\n}\n";

    #[test]
    fn the_runtime_s_externs_are_read_from_its_source() {
        let names = runtime_externs(RUNTIME);
        assert!(names.contains("d_thunk_eval") && names.contains("k_type_field_name"), "{names:?}");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn a_function_called_only_directly_takes_the_convention() {
        let out = preserve_none_calls(MODULE.to_string(), RUNTIME);
        assert!(out.contains("define preserve_nonecc %KValue @\"d_m/f_1\"("), "{out}");
        // after the linkage, which is where the parser wants it
        assert!(out.contains("define internal preserve_nonecc %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("%r = call preserve_nonecc %KValue @\"d_m/g_1\"("), "{out}");
        assert_eq!(out.matches("call preserve_nonecc %KValue @\"d_m/f_1\"(").count(), 2, "{out}");
        // what the runtime calls by name keeps the C convention
        assert!(out.contains("define %KValue @d_thunk_eval("), "{out}");
        assert!(out.contains("define %KValue @k_user_main("), "{out}");
    }

    #[test]
    fn a_function_whose_address_is_taken_keeps_its_convention() {
        let taken = format!("{MODULE}@table = constant ptr @\"d_m/g_1\"\n");
        let out = preserve_none_calls(taken, RUNTIME);
        assert!(out.contains("define internal %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("%r = call %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("define preserve_nonecc %KValue @\"d_m/f_1\"("), "{out}");
    }
}
