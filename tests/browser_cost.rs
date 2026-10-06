//! What the browser engine costs, counted rather than timed.
//!
//! The playground compiles a program inside the tab with `docs/kanso.wasm`
//! the way `kanso build` compiles it, lowers the module the native emitter
//! writes to wasm (`src/ir_wasm.rs`), and runs it against `runtime.c` built
//! for wasm32 (`docs/kanso-runtime.wasm`). Ruled 2026-10-06: every environment
//! kanso runs in has terms in the objective, for CPU and for memory, and none
//! at zero. Wall time in a browser cannot be made to read the same number
//! twice, so neither stage is timed. wasmi runs both with fuel metering on,
//! and fuel is spent per executed wasm instruction: the same module on the
//! same input spends the same fuel on every machine. The compile's memory is
//! the toolchain's own allocator tally (`kanso_heap_*` in `src/main.rs`), in
//! requested bytes. The run's memory is what the runtime's linear memory grew
//! by while the program ran, in whole 64 KiB pages, which is what the page
//! holds for it. The same code answers both identically on every run.
//!
//! The workload is `bench/interp_corpus`, the program the interpreted row
//! runs, so the browser and the interpreter are priced on the same work. It
//! builds its own input and imports only std, so the tab needs no files.
//!
//! Four rows:
//!   browser_compile_instructions  fuel spent compiling the entry in the tab
//!   browser_compile_peak_bytes    the most the compile held above where it began
//!   browser_run_instructions      fuel spent running the program, its module
//!                                 and every runtime function it calls
//!   browser_run_peak_bytes        what the runtime's linear memory grew by
//!                                 while the program ran
//!
//! Regenerate with `KANSO_REGEN_BROWSER_GOLDEN=1 cargo test --release --test
//! browser_cost` after `sh scripts/build_wasm.sh`.
//!
//! WHICH RUSTC BUILT THE ARTIFACT DECIDES EVERY ROW. The wasm target is the
//! same on every host, so the machine does not move these numbers, and the
//! compiler that built `kanso.wasm` moves all four: 1.94.1 and 1.98.1 emit
//! different code for the same source. So the golden names its rustc the way
//! the compile veins do, and a host on another one prints its rows without
//! comparing them -- and, under CI, still fails, because CI's rows are the only
//! ones that may be recorded (`scripts/gates/host_gate.sh` draws the same line).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use wasmi::{Caller, Config, Engine, Extern, Func, Linker, Module, Store, Table, Val};

#[path = "support/wasm32.rs"]
mod wasm32;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The emitted program's function table, which a closure the toolchain calls
/// back into is found in.
#[derive(Default)]
struct Dispatch(Rc<RefCell<Option<Table>>>);

/// Enough fuel that no workload this size runs out; what is left afterwards is
/// what says how much was spent.
const TANK: u64 = u64::MAX / 4;

struct Toolchain {
    store: Store<Dispatch>,
    instance: wasmi::Instance,
}

impl Toolchain {
    fn load() -> Toolchain {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let bytes = std::fs::read(root().join("docs/kanso.wasm"))
            .expect("docs/kanso.wasm reads; run scripts/build_wasm.sh");
        let module = Module::new(&engine, &bytes[..]).expect("the artifact is a wasm module");
        let mut store = Store::new(&engine, Dispatch::default());
        store.set_fuel(TANK).expect("fuel is on");
        let mut linker = Linker::new(&engine);
        let callback = Func::wrap(
            &mut store,
            |mut caller: Caller<'_, Dispatch>,
             handle: i32,
             env: i32,
             arg: i32|
             -> Result<i32, wasmi::Error> {
                let table = caller.data().0.borrow().expect("a closure ran before main");
                let target = table
                    .get(&mut caller, handle as u64)
                    .expect("the handle is in the table")
                    .funcref()
                    .and_then(|f| f.val().copied().copied())
                    .expect("the table entry is a function");
                let mut out = [Val::I32(0)];
                target.call(&mut caller, &[Val::I32(env), Val::I32(arg)], &mut out)?;
                Ok(out[0].i32().expect("a closure answers an i32"))
            },
        );
        linker.define("env", "k_callback", callback).expect("k_callback is the one import");
        let instance =
            linker.instantiate_and_start(&mut store, &module).expect("the toolchain instantiates");
        Toolchain { store, instance }
    }

    fn func(&self, name: &str) -> Func {
        self.instance
            .get_func(&self.store, name)
            .unwrap_or_else(|| panic!("the toolchain exports {name}"))
    }

    fn call(&mut self, name: &str, args: &[Val], results: &mut [Val]) {
        let f = self.func(name);
        f.call(&mut self.store, args, results).unwrap_or_else(|e| panic!("{name}: {e}"));
    }

    fn i32_call(&mut self, name: &str, args: &[Val]) -> i32 {
        let mut out = [Val::I32(0)];
        self.call(name, args, &mut out);
        out[0].i32().unwrap_or_else(|| panic!("{name} answers an i32"))
    }

    fn i64_call(&mut self, name: &str) -> u64 {
        let mut out = [Val::I64(0)];
        self.call(name, &[], &mut out);
        out[0].i64().unwrap_or_else(|| panic!("{name} answers an i64")) as u64
    }

    fn memory(&self) -> wasmi::Memory {
        match self.instance.get_export(&self.store, "memory") {
            Some(Extern::Memory(m)) => m,
            _ => panic!("the toolchain exports its memory"),
        }
    }

    fn write(&mut self, text: &str) -> (i32, i32) {
        let len = text.len() as i32;
        let ptr = self.i32_call("kanso_alloc", &[Val::I32(len)]);
        let memory = self.memory();
        memory
            .write(&mut self.store, ptr as u32 as usize, text.as_bytes())
            .expect("the allocation is writable");
        (ptr, len)
    }

    fn read(&mut self, ptr_fn: &str, len_fn: &str) -> Vec<u8> {
        let ptr = self.i32_call(ptr_fn, &[]) as u32 as usize;
        let len = self.i32_call(len_fn, &[]) as u32 as usize;
        let mut bytes = vec![0u8; len];
        self.memory().read(&self.store, ptr, &mut bytes).expect("the buffer reads");
        bytes
    }

    fn fuel(&self) -> u64 {
        self.store.get_fuel().expect("fuel is on")
    }

    /// The fuel and the peak a stage spends, the peak measured above the live
    /// bytes the stage began with.
    fn begin(&mut self) -> (u64, u64) {
        self.call("kanso_heap_reset", &[], &mut []);
        let live = self.i64_call("kanso_heap_live");
        (self.fuel(), live)
    }

    fn end(&mut self, (fuel, live): (u64, u64)) -> (u64, u64) {
        let spent = fuel - self.fuel();
        let peak = self.i64_call("kanso_heap_peak") - live;
        (spent, peak)
    }
}

struct Sitting {
    compile: (u64, u64),
    run: (u64, u64),
    output: String,
}

/// The corpus through the native route: compiled by the toolchain the way
/// `kanso build` compiles it and lowered by `ir_wasm` (`kanso_compile_native`),
/// then run against `runtime.c` built for wasm32. The run's memory is what
/// the runtime's linear memory grew by while the program ran: the page holds
/// that much for it, in whole 64 KiB pages.
fn sitting(rt: &[u8]) -> Sitting {
    let mut t = Toolchain::load();
    let source =
        std::fs::read_to_string(root().join("bench/interp_corpus/interp_corpus/interp_corpus.kso"))
            .expect("the corpus reads");
    t.call("kanso_forget_sources", &[], &mut []);
    let (path_ptr, path_len) = t.write("interp_corpus");
    let (file_ptr, file_len) = t.write("interp_corpus.kso");
    let (src_ptr, src_len) = t.write(&source);
    t.call(
        "kanso_hand_source",
        &[
            Val::I32(path_ptr),
            Val::I32(path_len),
            Val::I32(file_ptr),
            Val::I32(file_len),
            Val::I32(src_ptr),
            Val::I32(src_len),
        ],
        &mut [],
    );
    let entry = "import \"./interp_corpus\"\n\ninterp_corpus/play\n";
    let (name_ptr, name_len) = t.write("main.kso");
    t.call("kanso_set_file", &[Val::I32(name_ptr), Val::I32(name_len)], &mut []);
    let (ptr, len) = t.write(entry);
    let start = t.begin();
    let status = t.i32_call("kanso_compile_native", &[Val::I32(ptr), Val::I32(len)]);
    let compile = t.end(start);
    assert_eq!(
        status,
        0,
        "the corpus compiles on the native route: {:?}",
        String::from_utf8_lossy(&t.read("kanso_out_ptr", "kanso_out_len"))
    );
    let side = t.read("kanso_wasm_ptr", "kanso_wasm_len");
    let data = t.i32_call("kanso_side_data", &[]) as u32;
    let table = t.i32_call("kanso_side_table", &[]) as u32;
    let ran = wasm32::execute(rt, &side, data, table, &[("KANSO_SEED", "1")]).expect("it runs");
    assert_eq!(ran.exit, 0, "the corpus runs to completion: {}", ran.err);
    Sitting { compile, run: (ran.fuel, ran.grown), output: ran.out }
}

fn rows(s: &Sitting) -> String {
    format!(
        "browser_compile_instructions={}\nbrowser_compile_peak_bytes={}\n\
         browser_run_instructions={}\nbrowser_run_peak_bytes={}\n",
        s.compile.0, s.compile.1, s.run.0, s.run.1
    )
}

/// The rustc on this host, in the spelling `scripts/gates/measured_on.sh`
/// writes: upstream version only.
fn rustc_here() -> String {
    let out = std::process::Command::new("rustc").arg("--version").output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let v = text.split_whitespace().nth(1).unwrap_or("unknown");
    format!("rustc={v}")
}

const HEADER: &str = "\
# What the browser engine costs: bench/interp_corpus compiled by
# docs/kanso.wasm the way kanso build compiles it, its module lowered to wasm
# in the toolchain, and run against runtime.c built for wasm32, all under wasmi
# with fuel metering. Fuel is spent per executed wasm instruction, the compile
# peak is the toolchain allocator's requested bytes and the run's memory is
# what the runtime's linear memory grew by, so every row is the same on every
# run of the same code. See tests/browser_cost.rs.
";

#[test]
fn the_browser_engine_costs_what_the_golden_says() {
    // the runtime is part of what the page ships, so a host that cannot build
    // it cannot price the page; under CI that is a failure (`toolchain`)
    if !wasm32::toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = std::env::temp_dir().join(format!("kanso-browser-cost-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("the work dir makes");
    let rt = wasm32::runtime(&work);
    let first = sitting(&rt);
    let again = sitting(&rt);
    let _ = std::fs::remove_dir_all(&work);
    // the interpreter's own answer for this corpus, so a module that ran and
    // printed nothing is not priced as a cheap one
    assert!(
        first.output.starts_with("interp "),
        "the tab printed the corpus's answer, not {:?}",
        first.output
    );
    let got = rows(&first);
    assert_eq!(
        got,
        rows(&again),
        "two sittings of one artifact must read the same, or the rows are not a measurement"
    );
    let path = root().join("bench/browser_golden.txt");
    let stored = std::fs::read_to_string(&path).unwrap_or_default();
    let here = rustc_here();
    let want_host =
        stored.lines().find_map(|l| l.strip_prefix("# measured-on ")).map(str::to_string);
    if let Some(want_host) = &want_host {
        if *want_host != here && std::env::var("KANSO_REGEN_BROWSER_GOLDEN").is_err() {
            print!("{got}");
            assert!(
                std::env::var("GITHUB_ACTIONS").is_err(),
                "bench/browser_golden.txt was measured on {want_host} and this host is {here}; \
                 the rows above are this host's and nothing was compared"
            );
            eprintln!("measured on {want_host}, here {here}: not compared");
            return;
        }
    }
    if std::env::var("KANSO_REGEN_BROWSER_GOLDEN").is_ok() {
        let kept: String = stored
            .lines()
            .filter(|l| l.starts_with('#') && !l.starts_with("# measured-on "))
            .map(|l| format!("{l}\n"))
            .collect();
        let head = if kept.is_empty() { HEADER.to_string() } else { kept };
        let head = format!("{head}# measured-on {here}\n");
        std::fs::write(&path, format!("{head}{got}")).expect("the golden writes");
        return;
    }
    let want: String = std::fs::read_to_string(&path)
        .expect("bench/browser_golden.txt reads")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(got, want, "the browser rows moved; say which way in design/compiler-log.md");
}
