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

/// Small blocks on wasm32, kept on a free list per size and handed back out
/// before dlmalloc is asked. `std::alloc::System` on wasm32 is dlmalloc, and
/// its malloc and free were the costliest thing the tab's compile did that the
/// native compiler does not: mimalloc already keeps blocks this way. A request
/// of 512 bytes or fewer is rounded up to a multiple of eight, so a block goes
/// back to the list it came from and dlmalloc only ever sees the rounded size.
/// On `bench/interp_corpus` this took `browser_compile_instructions` from
/// 514,922,959 to 453,713,751. A ceiling of 256 bytes cost 0.8% more than
/// 512, and one of 1024 saved 0.3% while keeping twice as much on the lists.
/// The tally above counts requested bytes before any of this, so the peak row
/// reads what the compiler asked for and not what the lists hold. The engine
/// runs on one thread.
#[cfg(target_arch = "wasm32")]
mod small {
    pub const MAX: usize = 512;
    pub static mut FREE: [*mut u8; MAX / 8] = [std::ptr::null_mut(); MAX / 8];

    pub fn class(size: usize, align: usize) -> Option<usize> {
        (size != 0 && size <= MAX && align <= 8).then(|| (size - 1) / 8)
    }

    pub unsafe fn take(c: usize) -> *mut u8 {
        let head = unsafe { std::ptr::addr_of_mut!(FREE[c]) };
        let p = unsafe { *head };
        if p.is_null() {
            let layout = unsafe { std::alloc::Layout::from_size_align_unchecked((c + 1) * 8, 8) };
            return unsafe { std::alloc::GlobalAlloc::alloc(&super::UNDER, layout) };
        }
        unsafe { *head = *(p as *mut *mut u8) };
        p
    }

    pub unsafe fn give(c: usize, p: *mut u8) {
        let head = unsafe { std::ptr::addr_of_mut!(FREE[c]) };
        unsafe { *(p as *mut *mut u8) = *head };
        unsafe { *head = p };
    }
}

/// The tab's `memcmp`, eight bytes at a time (`kanso::compare`). The one
/// wasm32 links by default is compiler-builtins' loop of one byte per pass.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe { kanso::compare::bytes(a, b, n) }
}

/// Equality alone, which is what `==` on two slices asks for.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe { kanso::compare::differ(a, b, n) }
}

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

/// mimalloc's `mi_option_use_numa_nodes`, seventeenth in the same enum and
/// read from the same header by the same spec.
#[cfg(not(target_arch = "wasm32"))]
const USE_NUMA_NODES: i32 = 16;

/// Three options, all set before the first allocation, each for a reason a
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
///
/// **One NUMA node, so the host's topology is not read.** Asked how many nodes
/// it may spread over, mimalloc counts them itself: it formats
/// `/sys/devices/system/node/node1`, asks `access` whether that exists, and
/// goes on to `node2` until one does not. The first ask comes from the first
/// allocation of the interpreter's own thread, inside the frame
/// `interp_instructions` counts, and costs about 750 instructions on a
/// one-node machine and that again for every further node. On a machine with
/// more than one, every thread then asks the kernel which node it is running
/// on, and places its memory by the answer. None of that is the compiler's
/// work, and a runner with two nodes would read a different row. Telling
/// mimalloc there is one node answers the question without the probe.
#[cfg(not(target_arch = "wasm32"))]
extern "C" fn set_the_allocator_before_it_runs() {
    unsafe { libmimalloc_sys::mi_option_set(ARENA_EAGER_COMMIT, 0) };
    unsafe { libmimalloc_sys::mi_option_set(PURGE_DELAY, -1) };
    unsafe { libmimalloc_sys::mi_option_set(USE_NUMA_NODES, 1) };
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
        // `compiler_counters` and `interpreter_counters` read these two, and
        // only the command line prints them; the tab reads the live bytes and
        // the peak, so its compile keeps no tally it never reads.
        #[cfg(not(target_arch = "wasm32"))]
        ALLOC_BYTES.fetch_add(n, Ordering::Relaxed);
        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        if let Some(c) = small::class(layout.size(), layout.align()) {
            return unsafe { small::take(c) };
        }
        unsafe { UNDER.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        if let Some(c) = small::class(layout.size(), layout.align()) {
            return unsafe { small::give(c, ptr) };
        }
        unsafe { UNDER.dealloc(ptr, layout) }
    }
    /// Counted as the allocation, copy and free that `GlobalAlloc`'s own
    /// `realloc` performs, so the tally reads the same either way. A small
    /// block that stays in its class keeps its place; one that crosses moves
    /// between the lists and dlmalloc; two large blocks are dlmalloc's, which
    /// can grow in place where the default could only copy.
    #[cfg(target_arch = "wasm32")]
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        let (old, new) = (layout.size() as u64, new_size as u64);
        let live = LIVE_BYTES.load(Ordering::Relaxed);
        PEAK_BYTES.fetch_max(live + new, Ordering::Relaxed);
        LIVE_BYTES.store(live + new - old, Ordering::Relaxed);
        let was = small::class(layout.size(), layout.align());
        let now = small::class(new_size, layout.align());
        if was.is_none() && now.is_none() {
            return unsafe { UNDER.realloc(ptr, layout, new_size) };
        }
        if was.is_some() && was == now {
            return ptr;
        }
        let moved = match now {
            Some(c) => unsafe { small::take(c) },
            None => unsafe {
                UNDER.alloc(std::alloc::Layout::from_size_align_unchecked(new_size, layout.align()))
            },
        };
        if !moved.is_null() {
            unsafe { std::ptr::copy_nonoverlapping(ptr, moved, layout.size().min(new_size)) };
            match was {
                Some(c) => unsafe { small::give(c, ptr) },
                None => unsafe { UNDER.dealloc(ptr, layout) },
            }
        }
        moved
    }
}

#[global_allocator]
static COMPILER_ALLOCATOR: Counting = Counting;

/// The playground's memory terms read the same tally the compile row reads:
/// the peak is moved down to what is live before a stage, and read after it,
/// so a compile and a run each answer the most their own stage held. Requested
/// bytes rather than pages, because a page is 64 KiB and a term that moves in
/// steps that size cannot see most changes.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_heap_reset() {
    PEAK_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_heap_peak() -> u64 {
    PEAK_BYTES.load(Ordering::Relaxed)
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_heap_live() -> u64 {
    LIVE_BYTES.load(Ordering::Relaxed)
}

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

const VERBS: [&str; 11] = [
    "run", "play", "check", "derived", "test", "build", "install", "list", "update", "repl", "lsp",
];

const USAGE: &str = "usage: kanso <verb> [arguments]

  run <file|dir> [--plan|--interp]   compile and run; --plan shows the effects
                                     it would perform, --interp uses the oracle
  play <file> [--plan|--interp]      run a little program: definitions and
                                     statements in one file, stdlib imports only
  check <file|dir>                   report what run would refuse, and stop
  derived <file|dir>                 print what the entry's `derive` lines
                                     declared, then stop
  test <file|dir>                    run the `test_*` constants in each
                                     `*_test.kso` file, which sees only pub names
  build <file|dir> [--release|--counters|--wasm]
                                     write a native binary here, named for
                                     the program; --counters builds one that
                                     reports its counters under KANSO_COUNTERS=1;
                                     --wasm writes a module a page can run
  repl                               evaluate expressions as you type them
  lsp                                serve an editor the language server
                                     protocol on stdin and stdout

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
    reader_may_close();
    let code = driven();
    kanso::phase::report();
    code
}

fn driven() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("repl") {
        return repl();
    }
    // The language server speaks to an editor over stdin and stdout, and
    // takes no file: the editor names each buffer as it opens it.
    if args.first().map(String::as_str) == Some("lsp") {
        return ExitCode::from(kanso::lsp::serve_stdio() as u8);
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
                // A known verb with its file given was refused for a flag, and
                // the flag is what the reader has to change.
                Some(verb) if VERBS.contains(&verb.as_str()) => {
                    match args.iter().skip(2).find(|a| a.starts_with("--")) {
                        Some(flag) if args.len() > 1 => {
                            eprintln!("kanso: `{verb}` does not take `{flag}`")
                        }
                        _ => eprintln!("kanso: `{verb}` wants a file or directory"),
                    }
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
        if !interp && !plan {
            if let Some(played) = played_before(&file, &source) {
                return execute(&played, ended_by_signal);
            }
        }
        let program = match kanso::compile_play_file(&file, &source) {
            Ok(program) => program,
            Err(rendered) => {
                eprint!("{}", diag::paint(&rendered));
                return ExitCode::from(2);
            }
        };
        if plan {
            return run_plan(&program, &file, &source);
        }
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
        matches!(command.as_str(), "run" | "check" | "derived" | "build") && entry_inside.is_file();
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
    // The path as typed, kept only when it was a directory taken to its entry:
    // a message offering `--interp` should quote what the user wrote.
    let (file, typed) = match rerouted_dir {
        true => (entry_inside.to_string_lossy().into_owned(), Some(file)),
        false => (file, None),
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
    // A test file is an outside consumer of the module beside it (the
    // 2026-10-12 gavel): each one compiles on its own as an importer of that
    // module, and testing the directory runs every one of them.
    if command == "test" {
        return test(&file);
    }
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
            if command == "derived" {
                kanso::derive::show();
            }
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
    if command == "derived" {
        for line in kanso::derive::shown() {
            println!("{line}");
        }
        return ExitCode::SUCCESS;
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
    if command == "build" {
        return build(program, &file, release, built_as);
    }
    if interp {
        return run_interpreted(&program, program_args());
    }
    run(&program, &file, typed.as_deref().unwrap_or(&file), &source, plan)
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
        && command != "derived"
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
            // The program as a wasm side module for a page to link against
            // the runtime built for wasm32, as the playground links one. It
            // reaches `build` the way `--strict` reaches the stages it
            // changes, so the tuple stays as it is.
            "--wasm" if command == "build" => std::env::set_var("KANSO_BUILD_WASM", "1"),
            "--" => break,
            _ => return None,
        }
    }
    if plan && command != "run" && command != "play" {
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

// The interpreter gates anchor their count on this frame, so it stays out of
// line: built as one codegen unit, the compiler inlined it into its thread's
// closure and the gate read a drop glue that carries its name.
#[inline(never)]
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
                eprintln!("interp_callee_lookups={}", self.0.callee_lookups.get());
                eprintln!("interp_frames_made={}", self.0.frames_made.get());
                // Reported when it is not nought, as the native runtime
                // reports it, so no golden without a socket carries it.
                if self.0.sock_waits.get() > 0 {
                    eprintln!("sock_waits={}", self.0.sock_waits.get());
                }
            }
        }
    }
    let _stats = Stats(&interp);
    let value = match interp.run_main() {
        Ok(value) => value,
        Err(e) => {
            eprint!("error[runtime]: {}\n{}", e.message, e.place());
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
                    eprint!("error[runtime]: {}\n{}", e.message, e.place());
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

fn build(program: ast::Program, file: &str, release: bool, built_as: Option<String>) -> ExitCode {
    let emitted = match release {
        true => kanso::codegen::emit_ir_owned(program, closure_convention()),
        false => kanso::codegen::emit_ir_dev_owned(program, closure_convention()),
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
    if std::env::var_os("KANSO_BUILD_WASM").is_some() {
        return built_for_the_browser(&stem, &ll_path, ir);
    }
    // x86-64 alone: this convention has not been measured on arm64. On both
    // every `musttail` the emitter wrote reaches clang as written, so a tail
    // call consumes no stack. An arm64 release build also guards each arm
    // wider than the register file against two miscompiles (`arm64_tails`).
    let flatten = release
        && cfg!(target_arch = "x86_64")
        && closure_convention() == kanso::codegen::ClosureConvention::PreserveNone;
    let ir = match (flatten, release && cfg!(target_arch = "aarch64")) {
        (true, _) => preserve_none_calls(preserve_none_tails(ir), kanso::RUNTIME_C),
        (false, true) => arm64_tails(ir),
        (false, false) => ir,
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
        Err(io) if clang_is_missing(&io) => {
            eprintln!(
                "error: cannot build: clang is not on PATH, and `kanso build` needs it to \
                 write a native binary. install clang and build again"
            );
            ExitCode::FAILURE
        }
        Err(io) => {
            eprintln!("error: cannot invoke clang: {io}");
            ExitCode::FAILURE
        }
    }
}

/// `kanso build --wasm`: the emitted module translated to a wasm side
/// module (`ir_wasm`), written as `<stem>.wasm` with the IR beside it. A
/// host links it as the playground does: against the runtime built for
/// wasm32, after setting aside the data bytes and table slots it asks for.
/// Those two numbers ride in the module itself, in a custom section named
/// `kanso.link`, as little-endian u32s, so the file is all a page needs.
fn built_for_the_browser(stem: &str, ll_path: &str, ir: String) -> ExitCode {
    if let Err(io) = std::fs::write(ll_path, &ir) {
        eprintln!("error: cannot write {ll_path}: {io}");
        return ExitCode::from(2);
    }
    let side = match kanso::ir_wasm::translate_owned(kanso::codegen::retarget_wasm32(&ir)) {
        Ok(side) => side,
        Err(why) => {
            eprintln!("error: cannot lower {ll_path} to wasm: {why}");
            return ExitCode::FAILURE;
        }
    };
    let mut module = side.wasm;
    let mut payload = Vec::new();
    payload.extend_from_slice(&side.data.to_le_bytes());
    payload.extend_from_slice(&side.table.to_le_bytes());
    custom_section(&mut module, "kanso.link", &payload);
    let path = format!("{stem}.wasm");
    if let Err(io) = std::fs::write(&path, module) {
        eprintln!("error: cannot write {path}: {io}");
        return ExitCode::from(2);
    }
    println!("built ./{path} (llvm ir at {ll_path})");
    ExitCode::SUCCESS
}

/// Appends a custom section to a wasm module: id 0, its size, then the name
/// and the payload, each length as unsigned LEB128.
fn custom_section(module: &mut Vec<u8>, name: &str, payload: &[u8]) {
    let mut body = Vec::new();
    leb128(&mut body, name.len());
    body.extend_from_slice(name.as_bytes());
    body.extend_from_slice(payload);
    module.push(0);
    leb128(module, body.len());
    module.extend_from_slice(&body);
}

fn leb128(out: &mut Vec<u8>, mut n: usize) {
    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

include!("main/tail_calls.rs");

include!("main/toolchain.rs");

include!("main/clang.rs");

/// `kanso test <target>`: a test file, or a directory whose test files all
/// run. Each test file is compiled alone, as an importer of the module it
/// sits beside, so one test file never sees another's helpers.
fn test(target: &str) -> ExitCode {
    let path = std::path::Path::new(target);
    let files: Vec<String> = match path.is_dir() {
        true => {
            let mut found: Vec<String> = std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| kanso::is_test_file(p))
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            found.sort();
            found
        }
        false => vec![target.to_string()],
    };
    if files.is_empty() {
        eprintln!("{target}: no test files (a test file is named `*_test.kso`)");
        return ExitCode::from(2);
    }
    let mut suites = Vec::new();
    for file in files {
        let source = match std::fs::read_to_string(&file) {
            Ok(source) => source,
            Err(io) => {
                eprintln!("error: cannot read {file}: {io}");
                return ExitCode::from(2);
            }
        };
        // A file of bare statements, tested, used to be refused as a library
        // with a statement in it, which named the file's shape and not the
        // missing tests.
        let declares = source
            .lines()
            .any(|l| l.starts_with("fn ") || l.starts_with("type ") || l.starts_with("pub "));
        if !declares && !kanso::is_test_file(std::path::Path::new(&file)) {
            eprintln!(
                "error: `{file}` holds statements and no tests. `kanso test` runs the \
                 `test_*` constants a library declares, and a `*_test.kso` file beside a \
                 module tests that module. `kanso run {file}` runs this one"
            );
            return ExitCode::from(2);
        }
        let compiled = match kanso::is_test_file(std::path::Path::new(&file)) {
            true => kanso::compile_test_file(&file, &source),
            false => kanso::compile_source("test", &file, &source),
        };
        match compiled {
            Ok(program) => suites.push((program, file, source)),
            Err(rendered) => {
                eprint!("{}", diag::paint(&rendered));
                return ExitCode::from(2);
            }
        }
    }
    run_tests(target, &suites)
}

/// The tests run on the interpreter's deep stack, as `run --interp` does. On
/// the main thread's stack a test that recursed too far overflowed Rust's
/// stack before the interpreter's depth limit could report it, and the abort
/// took every other test's result with it.
fn run_tests(target: &str, suites: &[(ast::Program, String, String)]) -> ExitCode {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(scope, || run_tests_on_stack(target, suites))
            .expect("spawns")
            .join()
            .expect("test thread completes")
    })
}

fn run_tests_on_stack(target: &str, suites: &[(ast::Program, String, String)]) -> ExitCode {
    let mut ran = 0;
    let mut failed = 0;
    for (program, file, source) in suites {
        let interp = eval::Interp::new(program);
        // the module's own pub names come along with the import, and a
        // `test_*` among them is the module's, not this file's
        let own = std::path::Path::new(file).file_name().map(|n| n.to_string_lossy().into_owned());
        let mut names: Vec<&str> = program
            .fns
            .iter()
            .filter(|d| d.name.starts_with("test_") && d.params.is_empty())
            .filter(|d| own.as_deref().is_none_or(|want| d.file.ends_with(want)))
            .map(|d| d.name.as_str())
            .collect();
        names.dedup();
        for name in &names {
            ran += 1;
            let outcome = interp.run_named(name).expect("filtered on zero-arg fns");
            // A test whose value is an effect has not said true or false
            // yet: it is performed, and what it yields is judged below as
            // any test's value is (#2458).
            let (outcome, performed) = match outcome {
                Ok(eval::Value::Desc(desc)) => {
                    let mut executor =
                        eval::RealExecutor { program_args: Vec::new(), rng: eval::Rng::seeded() };
                    (interp.execute(&desc, &mut executor), true)
                }
                other => (other, false),
            };
            match outcome {
                Ok(eval::Value::True) => println!("{name} ... ok"),
                // An effect that ended in an err says where the err was
                // born, as an entry's unhandled err does under `run`.
                Ok(eval::Value::ErrV(info)) if performed => {
                    failed += 1;
                    println!("{name} ... FAILED");
                    eprint!(
                        "error[endpoint]: unhandled err reached the executor: {}\n{}",
                        eval::render(&interp, &info.reason, true),
                        eval::trace_lines(&interp, &info)
                    );
                }
                Ok(other) => {
                    failed += 1;
                    println!(
                        "{name} ... FAILED (returned {})",
                        eval::render(&interp, &other, true)
                    );
                }
                // A failure that knows its place names it as `run` does. Its
                // span belongs to that place's file, which need not be this
                // one: a failure inside std/text printed std/text's line
                // number under the test file's path.
                Err(runtime) if runtime.at.is_some() => {
                    failed += 1;
                    println!("{name} ... FAILED");
                    eprint!("error[runtime]: {}\n{}", runtime.message, runtime.place());
                }
                Err(runtime) => {
                    failed += 1;
                    let d = diag::Diagnostic::new("runtime", runtime.message, runtime.span);
                    println!("{name} ... FAILED");
                    eprint!("{}", diag::paint(&diag::render(&[d], file, source)));
                }
            }
        }
    }
    if ran == 0 {
        eprintln!("{target}: no tests found (a test is a constant named `test_*`)");
        return ExitCode::from(2);
    }
    println!("{} passed, {failed} failed", ran - failed);
    match failed {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

include!("main/execute.rs");
