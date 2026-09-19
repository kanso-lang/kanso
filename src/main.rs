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
    let ir = match kanso::codegen::emit_ir(program, closure_convention()) {
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
    match *ANSWER.get_or_init(preserve_none_probe) {
        true => ClosureConvention::PreserveNone,
        false => ClosureConvention::Absent,
    }
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
    let runtime_obj = cached_runtime_object("release", &["-O3", "-flto"])?;
    std::process::Command::new("clang")
        .arg("-O3")
        .arg("-flto")
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
    let preserve = closure_convention() == kanso::codegen::ClosureConvention::PreserveNone;
    preserve.hash(&mut hasher);
    // The same reason, for the same kind of reason: whether the runtime carries
    // its twenty-seven counter gates is a `-D` the caller decides, not a fact
    // in the source, so an object built one way must not be handed to a build
    // that wanted the other. A shipped binary linked against a counting runtime
    // pays for gates it can never reach; a counting binary linked against a
    // gate-free one reports zeros and the goldens all move at once.
    let counting = kanso::codegen::counters_wanted();
    counting.hash(&mut hasher);
    let key = hasher.finish();
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
    let ir = match kanso::codegen::emit_ir(program, closure_convention()) {
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
    // The same swap, for the same reason: this key names the runtime the
    // binary would be linked against, and that runtime is a constant.
    kanso::hash::RUNTIME_DIGEST.hash(&mut hasher);
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
    let ll_path = std::env::temp_dir().join(format!("kanso_run_{key:016x}_{}.ll", pid_tag()));
    std::fs::write(&ll_path, ir)?;
    let staging = std::env::temp_dir().join(format!("kanso_run_{key:016x}_{}", pid_tag()));
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
