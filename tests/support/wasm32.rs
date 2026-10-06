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
use wasmi::{Caller, Config, Engine, Extern, Func, Global, Linker, Module, Mutability, Ref, Store, Val};

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

pub struct Ran {
    pub out: String,
    pub err: String,
    pub exit: i32,
    pub fuel: u64,
    /// Bytes the runtime's linear memory grew by while the program ran: the
    /// memory a page holds for it beyond what instantiation set aside.
    pub grown: u64,
}

/// Instantiates the runtime, links `module` into it with `data` bytes and
/// `table` slots set aside, and runs `_start`. A micro program spends well
/// under a hundred million units of fuel; a translation that loops forever
/// stops at two billion and reads as a trap. `env` is the process
/// environment the runtime's `getenv` sees.
pub fn execute(
    runtime: &[u8],
    module: &[u8],
    data: u32,
    table: u32,
    env: &[(&str, &str)],
) -> Result<Ran, String> {
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let rt = Module::new(&engine, runtime).map_err(|e| format!("runtime: {e}"))?;
    let prog = Module::new(&engine, module).map_err(|e| format!("the module validates: {e}"))?;
    let env = env.iter().map(|(k, v)| format!("{k}={v}\0").into_bytes()).collect();
    let mut store = Store::new(&engine, Host { env, ..Host::default() });
    store.set_fuel(2_000_000_000).expect("fuel is on");
    let mut linker = Linker::<Host>::new(&engine);
    for import in rt.imports() {
        let wasmi::ExternType::Func(ty) = import.ty() else { continue };
        let name = import.name().to_string();
        let f = Func::new(&mut store, ty.clone(), move |mut c, args, res| {
            let r = wasi(&name, &mut c, args)?;
            if let Some(slot) = res.first_mut() {
                *slot = Val::I32(r);
            }
            Ok(())
        });
        linker.define(import.module(), import.name(), f).expect("the import defines");
    }
    let rti = linker.instantiate_and_start(&mut store, &rt).map_err(|e| format!("runtime: {e}"))?;
    let memory = rti.get_memory(&store, "memory").expect("the runtime exports its memory");
    store.data_mut().memory = Some(memory);
    let shared = rti.get_table(&store, "__indirect_function_table").expect("the runtime exports its table");
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
            "__table_base" => Global::new(&mut store, Val::I32(tbase as i32), Mutability::Const).into(),
            _ => rti.get_export(&store, name).ok_or(format!("the runtime has no {name}"))?,
        };
        plinker.define("env", name, ext).map_err(|e| format!("{name}: {e}"))?;
    }
    plinker.instantiate_and_start(&mut store, &prog).map_err(|e| format!("the module links: {e}"))?;
    let start = rti.get_func(&store, "_start").expect("a command exports _start");
    let before = store.get_fuel().expect("fuel is on");
    let size = memory.size(&store);
    let finished = start.call(&mut store, &[], &mut []);
    let fuel = before - store.get_fuel().expect("fuel is on");
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
        fuel,
        grown,
    })
}
