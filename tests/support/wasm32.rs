//! The wasm32 runtime and a host for programs linked against it, shared by the
//! specs that run the translator's output (`#[path]`-included, so it is not a
//! test target of its own).
//!
//! The runtime is `runtime.c` built by `scripts/build_runtime_wasm.sh`, the
//! same script the playground's build runs. The host instantiates it, sets
//! aside the data and table room a translated module asks for, instantiates the
//! module against the runtime's exports, and calls the runtime's `_start`
//! under wasmi's fuel meter with a WASI layer of a dozen calls.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use wasmi::{
    Caller, Config, Engine, Extern, Func, Global, Linker, Module, Mutability, Ref, Store, Val,
};

const WASI_LIB: &str = "/usr/lib/wasm32-wasi";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Whether this host can build the runtime at all. The Linux specs job sets
/// `KANSO_WASM32_REQUIRED` after installing the toolchain, and there a missing
/// piece is a failure rather than a skip.
pub fn toolchain() -> bool {
    let clang = Command::new("clang").arg("--version").output().is_ok();
    let ld = Command::new("wasm-ld").arg("--version").output().is_ok();
    let libc = Path::new(WASI_LIB).join("libc.a").exists();
    let ok = clang && ld && libc;
    assert!(
        ok || std::env::var("KANSO_WASM32_REQUIRED").is_err(),
        "this job installs the wasm32 toolchain: clang {clang}, wasm-ld {ld}, wasi-libc {libc}"
    );
    ok
}

/// The runtime module, built into `dir` by the playground's own script.
pub fn runtime(dir: &Path) -> Vec<u8> {
    let out = dir.join("kanso-runtime.wasm");
    let done = Command::new("sh")
        .arg(root().join("scripts/build_runtime_wasm.sh"))
        .arg(&out)
        .output()
        .expect("the runtime build runs");
    assert!(done.status.success(), "the runtime builds: {}", String::from_utf8_lossy(&done.stderr));
    std::fs::read(&out).expect("the runtime reads")
}

#[derive(Default)]
struct Host {
    out: Vec<u8>,
    err: Vec<u8>,
    exit: Option<i32>,
    memory: Option<wasmi::Memory>,
    /// `KEY=value` entries, each NUL-terminated as WASI hands them over.
    env: Vec<Vec<u8>>,
    /// Nanoseconds the program has slept, which is the only thing that moves
    /// its clock.
    slept: u64,
}

fn word(m: &wasmi::Memory, c: &Caller<'_, Host>, at: usize) -> usize {
    let mut b = [0u8; 4];
    m.read(c, at, &mut b).expect("in bounds");
    u32::from_le_bytes(b) as usize
}

/// One WASI call. Writes go to the stream they name, the clock reads one fixed
/// instant (2026-01-01, so a program asking whether time has passed the epoch
/// is answered) and randomness is zeros, so a run is a function of its
/// program; everything that would reach a filesystem answers ENOSYS.
fn wasi(name: &str, c: &mut Caller<'_, Host>, args: &[Val]) -> Result<i32, wasmi::Error> {
    let i = |k: usize| args[k].i32().expect("an i32 argument") as u32 as usize;
    let m = c.data().memory.expect("the runtime's memory");
    Ok(match name {
        "fd_write" => {
            let mut total = 0u32;
            for k in 0..i(2) {
                let p = word(&m, c, i(1) + k * 8);
                let n = word(&m, c, i(1) + k * 8 + 4);
                let mut bytes = vec![0u8; n];
                m.read(&*c, p, &mut bytes).expect("in bounds");
                match i(0) {
                    2 => c.data_mut().err.extend_from_slice(&bytes),
                    _ => c.data_mut().out.extend_from_slice(&bytes),
                }
                total += n as u32;
            }
            m.write(&mut *c, i(3), &total.to_le_bytes()).expect("in bounds");
            0
        }
        "args_sizes_get" => {
            m.write(&mut *c, i(0), &0u32.to_le_bytes()).expect("in bounds");
            m.write(&mut *c, i(1), &0u32.to_le_bytes()).expect("in bounds");
            0
        }
        "environ_sizes_get" => {
            let count = c.data().env.len() as u32;
            let bytes: u32 = c.data().env.iter().map(|e| e.len() as u32).sum();
            m.write(&mut *c, i(0), &count.to_le_bytes()).expect("in bounds");
            m.write(&mut *c, i(1), &bytes.to_le_bytes()).expect("in bounds");
            0
        }
        "environ_get" => {
            let (mut at, mut buf) = (i(0), i(1));
            for entry in c.data().env.clone() {
                m.write(&mut *c, at, &(buf as u32).to_le_bytes()).expect("in bounds");
                m.write(&mut *c, buf, &entry).expect("in bounds");
                at += 4;
                buf += entry.len();
            }
            0
        }
        "args_get" | "random_get" => 0,
        // Filetype 0, unknown: no stream here is a terminal, so `isatty` says
        // no and the runtime writes its diagnostics without colour, as it
        // does when native's stderr is a pipe.
        "fd_fdstat_get" => {
            m.write(&mut *c, i(1), &[0u8; 8]).expect("in bounds");
            0
        }
        "clock_time_get" => {
            let at: u64 = 1_767_225_600_000_000_000 + c.data().slept;
            m.write(&mut *c, i(2), &at.to_le_bytes()).expect("in bounds");
            0
        }
        // A sleep returns at once and moves the clock by what it asked for, so
        // the scheduler's deadlines arrive in the order native reaches them.
        // A page cannot block, and a run is a function of its program.
        "poll_oneoff" => {
            let (subs, events, n) = (i(0), i(1), i(2));
            for k in 0..n {
                let sub = subs + k * 48;
                let mut b = [0u8; 48];
                m.read(&*c, sub, &mut b).expect("in bounds");
                if b[8] == 0 {
                    let timeout = u64::from_le_bytes(b[24..32].try_into().expect("eight bytes"));
                    c.data_mut().slept += timeout;
                }
                let mut ev = [0u8; 32];
                ev[..8].copy_from_slice(&b[..8]);
                ev[10] = b[8];
                m.write(&mut *c, events + k * 32, &ev).expect("in bounds");
            }
            m.write(&mut *c, i(3), &(n as u32).to_le_bytes()).expect("in bounds");
            0
        }
        "fd_prestat_get" => 8,
        "proc_exit" => {
            c.data_mut().exit = Some(i(0) as i32);
            return Err(wasmi::Error::new("proc_exit"));
        }
        _ => 52,
    })
}

/// The export `counted` adds: an i64 global holding how many wasm instructions
/// the module has executed.
pub const COUNTER: &str = "__kanso_executed";

/// A copy of `module` that counts the wasm instructions it executes.
///
/// wasmi's fuel cannot be the count. It charges a whole frame (a function
/// body, a loop body, an if arm) when control enters it, and a `block` is not
/// a frame of its own, so a loop around a `br_table` dispatch pays for every
/// arm on every pass. On 2026-10-23 the tab's compile spent 433,668,787 fuel
/// and executed 293,067,500 instructions.
///
/// So the copy splits each function body into runs that control can enter only
/// at their first instruction (a run ends after any instruction that branches,
/// opens or closes a block, or traps) and adds each run's length to one global
/// as the run begins. Every branch lands after a `block`'s `end`, after a
/// `loop`, or after an `if` or `else`, which is where runs begin, so the global
/// ends up holding the number of instructions control reached.
pub fn counted(module: &[u8]) -> Vec<u8> {
    use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
    use wasm_encoder::{ConstExpr, Instruction, RawSection};
    use wasmparser::{Operator, Parser, Payload};

    let mut globals = 0u32;
    for payload in Parser::new(0).parse_all(module) {
        match payload.expect("the module parses") {
            Payload::ImportSection(r) => {
                for import in r.into_imports() {
                    if let wasmparser::TypeRef::Global(_) = import.expect("an import parses").ty {
                        globals += 1;
                    }
                }
            }
            Payload::GlobalSection(r) => globals += r.count(),
            _ => {}
        }
    }
    let counter = globals;
    let ty = wasm_encoder::GlobalType {
        val_type: wasm_encoder::ValType::I64,
        mutable: true,
        shared: false,
    };
    // Sections come in a fixed order, which is not the order of their ids, and
    // a module may have no global or export section of its own; the counter's
    // go where those would have.
    let rank = |id: u8| match id {
        13 => 5,
        6 => 6,
        7 => 7,
        8 => 8,
        9 => 9,
        12 => 10,
        10 => 11,
        11 => 12,
        _ => 0,
    };
    let mut re = RoundtripReencoder;
    let mut out = wasm_encoder::Module::new();
    let mut global_done = false;
    let mut export_done = false;
    let mut code = wasm_encoder::CodeSection::new();
    let mut in_code = false;
    for payload in Parser::new(0).parse_all(module) {
        let payload = payload.expect("the module parses");
        let id = match &payload {
            Payload::CodeSectionStart { .. } => Some(10),
            p => p.as_section().map(|(id, _)| id),
        };
        if let Some(id) = id {
            if !global_done && rank(id) > rank(6) {
                let mut g = wasm_encoder::GlobalSection::new();
                g.global(ty, &ConstExpr::i64_const(0));
                out.section(&g);
                global_done = true;
            }
            if !export_done && rank(id) > rank(7) {
                let mut e = wasm_encoder::ExportSection::new();
                e.export(COUNTER, wasm_encoder::ExportKind::Global, counter);
                out.section(&e);
                export_done = true;
            }
        }
        match payload {
            Payload::GlobalSection(r) => {
                let mut g = wasm_encoder::GlobalSection::new();
                re.parse_global_section(&mut g, r).expect("the globals copy");
                g.global(ty, &ConstExpr::i64_const(0));
                out.section(&g);
                global_done = true;
            }
            Payload::ExportSection(r) => {
                let mut e = wasm_encoder::ExportSection::new();
                re.parse_export_section(&mut e, r).expect("the exports copy");
                e.export(COUNTER, wasm_encoder::ExportKind::Global, counter);
                out.section(&e);
                export_done = true;
            }
            Payload::CodeSectionStart { .. } => in_code = true,
            Payload::CodeSectionEntry(body) => {
                let mut f = re.new_function_with_parsed_locals(&body).expect("the locals copy");
                let ops: Vec<Operator> = body
                    .get_operators_reader()
                    .expect("the body reads")
                    .into_iter()
                    .map(|op| op.expect("an instruction parses"))
                    .collect();
                let mut at = 0;
                while at < ops.len() {
                    let mut end = at;
                    while end < ops.len() {
                        end += 1;
                        if matches!(
                            ops[end - 1],
                            Operator::Block { .. }
                                | Operator::Loop { .. }
                                | Operator::If { .. }
                                | Operator::Else
                                | Operator::End
                                | Operator::Br { .. }
                                | Operator::BrIf { .. }
                                | Operator::BrTable { .. }
                                | Operator::Return
                                | Operator::Unreachable
                        ) {
                            break;
                        }
                    }
                    f.instruction(&Instruction::GlobalGet(counter));
                    f.instruction(&Instruction::I64Const((end - at) as i64));
                    f.instruction(&Instruction::I64Add);
                    f.instruction(&Instruction::GlobalSet(counter));
                    for op in &ops[at..end] {
                        f.instruction(&re.instruction(op.clone()).expect("an instruction copies"));
                    }
                    at = end;
                }
                code.function(&f);
            }
            payload => {
                if in_code {
                    out.section(&code);
                    in_code = false;
                }
                if let Some((id, range)) = payload.as_section() {
                    out.section(&RawSection { id, data: &module[range] });
                }
            }
        }
    }
    if in_code {
        out.section(&code);
    }
    out.finish()
}

/// What the counter in a module `counted` wrote reads now.
pub fn executed<T>(store: &Store<T>, instance: &wasmi::Instance) -> u64 {
    let global = instance.get_global(store, COUNTER).expect("the module is counted");
    global.get(store).i64().expect("the counter is an i64") as u64
}

pub struct Ran {
    pub out: String,
    pub err: String,
    pub exit: i32,
    /// Wasm instructions the runtime and the module executed (`counted`).
    pub executed: u64,
    /// Bytes the runtime's linear memory grew by while the program ran: the
    /// memory a page holds for it beyond what instantiation set aside.
    pub grown: u64,
}

/// A linker that answers every function `module` imports with `wasi`.
fn wasi_linker(engine: &Engine, store: &mut Store<Host>, module: &Module) -> Linker<Host> {
    let mut linker = Linker::<Host>::new(engine);
    for import in module.imports() {
        let wasmi::ExternType::Func(ty) = import.ty() else { continue };
        let name = import.name().to_string();
        let f = Func::new(&mut *store, ty.clone(), move |mut c, args, res| {
            let r = wasi(&name, &mut c, args)?;
            if let Some(slot) = res.first_mut() {
                *slot = Val::I32(r);
            }
            Ok(())
        });
        linker.define(import.module(), import.name(), f).expect("the import defines");
    }
    linker
}

/// Runs a WASI command that clang and wasm-ld linked whole, runtime and
/// program in one module, under the host `execute` gives the pair. A trap
/// that is not the command exiting is an error.
pub fn command(module: &[u8], env: &[(&str, &str)]) -> Result<Ran, String> {
    let mut config = Config::default();
    config.consume_fuel(true);
    // the same frame budget `execute` gives, for the same reason
    config.set_max_recursion_depth(10_000);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &counted(module)[..])
        .map_err(|e| format!("the module validates: {e}"))?;
    let env = env.iter().map(|(k, v)| format!("{k}={v}\0").into_bytes()).collect();
    let mut store = Store::new(&engine, Host { env, ..Host::default() });
    store.set_fuel(10_000_000_000).expect("fuel is on");
    let linker = wasi_linker(&engine, &mut store, &module);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| format!("it instantiates: {e}"))?;
    let memory = instance.get_memory(&store, "memory").expect("a command exports its memory");
    store.data_mut().memory = Some(memory);
    let start = instance.get_func(&store, "_start").expect("a command exports _start");
    let size = memory.size(&store);
    let finished = start.call(&mut store, &[], &mut []);
    let executed = executed(&store, &instance);
    let grown = (memory.size(&store) - size) * 65536;
    let host = store.data();
    let exit = match (finished, host.exit) {
        (_, Some(code)) => code,
        (Ok(()), None) => 0,
        (Err(e), None) => return Err(format!("trapped: {e}")),
    };
    Ok(Ran {
        out: String::from_utf8_lossy(&host.out).into_owned(),
        err: String::from_utf8_lossy(&host.err).into_owned(),
        exit,
        executed,
        grown,
    })
}

/// Instantiates the runtime, links `module` into it with `data` bytes and
/// `table` slots set aside, and runs `_start`, both modules as `counted`
/// copies. Fuel is only the stop: the heaviest micro program executes 927
/// million instructions, which its counted copy spends 2.33 billion units of
/// fuel on, and a translation that loops forever stops at ten billion and
/// reads as a trap. `env` is the process environment the runtime's `getenv`
/// sees.
pub fn execute(
    runtime: &[u8],
    module: &[u8],
    data: u32,
    table: u32,
    env: &[(&str, &str)],
) -> Result<Ran, String> {
    let mut config = Config::default();
    config.consume_fuel(true);
    // wasmi stops at a thousand frames, where a browser gives a page several
    // times that and native runs on an 8 MB stack: three hundred nested ties
    // reach past a thousand frames on every engine that runs them.
    config.set_max_recursion_depth(10_000);
    let engine = Engine::new(&config);
    let rt = Module::new(&engine, &counted(runtime)[..]).map_err(|e| format!("runtime: {e}"))?;
    // validated as written, then run as the counted copy
    Module::validate(&engine, module).map_err(|e| format!("the module validates: {e}"))?;
    let prog = Module::new(&engine, &counted(module)[..])
        .map_err(|e| format!("the module validates: {e}"))?;
    let env = env.iter().map(|(k, v)| format!("{k}={v}\0").into_bytes()).collect();
    let mut store = Store::new(&engine, Host { env, ..Host::default() });
    store.set_fuel(10_000_000_000).expect("fuel is on");
    let linker = wasi_linker(&engine, &mut store, &rt);
    let rti = linker.instantiate_and_start(&mut store, &rt).map_err(|e| format!("runtime: {e}"))?;
    let memory = rti.get_memory(&store, "memory").expect("the runtime exports its memory");
    store.data_mut().memory = Some(memory);
    let shared =
        rti.get_table(&store, "__indirect_function_table").expect("the runtime exports its table");
    let malloc = rti.get_typed_func::<i32, i32>(&store, "malloc").expect("malloc");
    let base = malloc.call(&mut store, data as i32).map_err(|e| format!("malloc: {e}"))?;
    let tbase = shared
        .grow(&mut store, table as u64, Val::FuncRef(Ref::Null))
        .map_err(|e| format!("table grow: {e}"))?;

    let mut plinker = Linker::<Host>::new(&engine);
    for import in prog.imports() {
        let name = import.name();
        let ext: Extern = match name {
            "memory" => memory.into(),
            "__indirect_function_table" => shared.into(),
            "__memory_base" => Global::new(&mut store, Val::I32(base), Mutability::Const).into(),
            "__table_base" => {
                Global::new(&mut store, Val::I32(tbase as i32), Mutability::Const).into()
            }
            _ => rti.get_export(&store, name).ok_or(format!("the runtime has no {name}"))?,
        };
        plinker.define("env", name, ext).map_err(|e| format!("{name}: {e}"))?;
    }
    let pi = plinker
        .instantiate_and_start(&mut store, &prog)
        .map_err(|e| format!("the module links: {e}"))?;
    let start = rti.get_func(&store, "_start").expect("a command exports _start");
    let before = executed(&store, &rti) + executed(&store, &pi);
    let size = memory.size(&store);
    let finished = start.call(&mut store, &[], &mut []);
    // A trap leaves the instance usable, so the page's host asks the runtime
    // where the overflow began, as docs/kanso-engine.js does.
    let place = rti.get_typed_func::<(), i32>(&store, "k_deep_place").expect("k_deep_place");
    let place = match place.call(&mut store, ()).expect("k_deep_place answers") as u32 as usize {
        0 => None,
        at => {
            let bytes = &memory.data(&store)[at..];
            let end = bytes.iter().position(|&b| b == 0).expect("the place ends");
            Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
        }
    };
    let executed = executed(&store, &rti) + executed(&store, &pi) - before;
    let grown = (memory.size(&store) - size) * 65536;
    let host = store.data();
    let mut err = String::from_utf8_lossy(&host.err).into_owned();
    let exit = match (finished, host.exit) {
        (_, Some(code)) => code,
        (Ok(()), None) => 0,
        // Native catches a stack overflow and says so; wasm's call stack
        // ends in a trap, and the page answers it with the same lines.
        (Err(e), None) if e.to_string().contains("call stack exhausted") => {
            err += &format!("{}\n", kanso::stack_exhausted(place.as_deref()));
            1
        }
        (Err(e), None) => return Err(format!("trapped: {e}")),
    };
    Ok(Ran { out: String::from_utf8_lossy(&host.out).into_owned(), err, exit, executed, grown })
}
