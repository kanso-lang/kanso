//! The tab's own route to native's layout: the emitter's IR, lowered to wasm by
//! `ir_wasm`, linked at instantiation against `runtime.c` built for wasm32.
//!
//! The runtime is built once with clang, as `tests/native_layout_on_wasm32.rs`
//! builds it, plus `wasm/hooks.c`, and linked as a module that exports its
//! memory, table, stack pointer and every symbol. Each program is built by
//! `kanso build`, and its module is retargeted and translated with no clang
//! involved. The host instantiates the runtime, sets aside the data and table
//! room the translated module asks for, instantiates the module against the
//! runtime's exports and calls the runtime's `_start`. Stdout, stderr and
//! exit are compared with the program's goldens.

use std::path::{Path, PathBuf};
use std::process::Command;
use wasmi::{
    Caller, Config, Engine, Extern, Func, Global, Linker, Module, Mutability, Ref, Store, Val,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Programs whose answer is a fact about the host they run on.
const HOST: [(&str, &str); 7] = [
    ("process_run", "runs a command"),
    ("a_command_that_answers_127", "runs a command"),
    ("make_dir_is_idempotent", "makes a directory"),
    ("dir_listing_is_sorted", "lists a directory"),
    ("a_file_that_is_not_text", "reads a file"),
    ("is_dir", "asks the filesystem"),
    ("write_err_stream", "orders two streams the host interleaves"),
];

const SYSROOT: &str = "/usr";
const WASI_LIB: &str = "/usr/lib/wasm32-wasi";

fn toolchain() -> bool {
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

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(out.status.success(), "{what}: {}", String::from_utf8_lossy(&out.stderr));
}

/// The runtime as a module of its own, built once into `dir`.
fn runtime(dir: &Path) -> Vec<u8> {
    let target = ["--target=wasm32-wasi", &format!("--sysroot={SYSROOT}"), "-O2", "-mtail-call"];
    let rt_ll = dir.join("runtime.ll");
    run(
        Command::new("clang")
            .args(target)
            .args(["-Xclang", "-target-abi", "-Xclang", "experimental-mv"])
            .args(["-D_WASI_EMULATED_SIGNAL", "-w", "-S", "-emit-llvm"])
            .arg("-I")
            .arg(root().join("wasm/include"))
            .arg(root().join("src/runtime.c"))
            .arg("-o")
            .arg(&rt_ll),
        "runtime.c lowers for wasm32",
    );
    let hooks_ll = dir.join("hooks.ll");
    run(
        Command::new("clang")
            .args(target)
            .args(["-Xclang", "-target-abi", "-Xclang", "experimental-mv"])
            .args(["-w", "-S", "-emit-llvm"])
            .arg(root().join("wasm/hooks.c"))
            .arg("-o")
            .arg(&hooks_ll),
        "the hooks lower for wasm32",
    );
    let mut objs = Vec::new();
    for (src, name) in
        [(rt_ll, "runtime.o"), (hooks_ll, "hooks.o"), (root().join("wasm/shim.c"), "shim.o")]
    {
        let o = dir.join(name);
        run(
            Command::new("clang").args(target).args(["-w", "-c"]).arg(&src).arg("-o").arg(&o),
            &format!("{name} builds for wasm32"),
        );
        objs.push(o);
    }
    let wasm = dir.join("runtime.wasm");
    run(
        Command::new("wasm-ld")
            .arg(Path::new(WASI_LIB).join("crt1-command.o"))
            .args(&objs)
            .arg(format!("-L{WASI_LIB}"))
            .args(["-lc", "-lwasi-emulated-signal", "-lwasi-emulated-getpid"])
            .args([
                "--export-all",
                "--export-table",
                "--growable-table",
                "--export=__stack_pointer",
            ])
            .args(["--export=malloc", "--stack-first", "-z", "stack-size=8388608", "-o"])
            .arg(&wasm),
        "the runtime links for wasm32",
    );
    std::fs::read(&wasm).expect("the runtime reads")
}

#[derive(Default)]
struct Host {
    out: Vec<u8>,
    err: Vec<u8>,
    exit: Option<i32>,
    memory: Option<wasmi::Memory>,
}

fn word(m: &wasmi::Memory, c: &Caller<'_, Host>, at: usize) -> usize {
    let mut b = [0u8; 4];
    m.read(c, at, &mut b).expect("in bounds");
    u32::from_le_bytes(b) as usize
}

/// One WASI call, answered as `tests/native_layout_on_wasm32.rs` answers it.
fn wasi(name: &str, c: &mut Caller<'_, Host>, args: &[Val]) -> Result<i32, wasmi::Error> {
    let i = |k: usize| args[k].i32().expect("an i32 argument") as usize;
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
        "args_sizes_get" | "environ_sizes_get" => {
            m.write(&mut *c, i(0), &0u32.to_le_bytes()).expect("in bounds");
            m.write(&mut *c, i(1), &0u32.to_le_bytes()).expect("in bounds");
            0
        }
        "args_get" | "environ_get" | "random_get" => 0,
        "fd_fdstat_get" => {
            m.write(&mut *c, i(1), &[2u8, 0, 0, 0, 0, 0, 0, 0]).expect("in bounds");
            0
        }
        "clock_time_get" => {
            let at: u64 = 1_767_225_600_000_000_000;
            m.write(&mut *c, i(2), &at.to_le_bytes()).expect("in bounds");
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

struct Ran {
    out: String,
    err: String,
    exit: i32,
    fuel: u64,
}

/// Instantiates the runtime, links the side module into it and runs `_start`.
fn execute(runtime: &[u8], side: &kanso::ir_wasm::Side) -> Result<Ran, String> {
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let rt = Module::new(&engine, runtime).map_err(|e| format!("runtime: {e}"))?;
    let prog =
        Module::new(&engine, &side.wasm[..]).map_err(|e| format!("the module validates: {e}"))?;
    let mut store = Store::new(&engine, Host::default());
    // A micro program spends well under a hundred million; a translation that
    // loops forever stops here and reads as a failure.
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
    let table =
        rti.get_table(&store, "__indirect_function_table").expect("the runtime exports its table");
    let malloc = rti.get_typed_func::<i32, i32>(&store, "malloc").expect("malloc");
    let data = malloc.call(&mut store, side.data as i32).map_err(|e| format!("malloc: {e}"))?;
    let tbase = table
        .grow(&mut store, side.table as u64, Val::FuncRef(Ref::Null))
        .map_err(|e| format!("table grow: {e}"))?;

    let mut plinker = Linker::<Host>::new(&engine);
    for import in prog.imports() {
        let name = import.name();
        let ext: Extern = match name {
            "memory" => memory.into(),
            "__indirect_function_table" => table.into(),
            "__memory_base" => Global::new(&mut store, Val::I32(data), Mutability::Const).into(),
            "__table_base" => {
                Global::new(&mut store, Val::I32(tbase as i32), Mutability::Const).into()
            }
            _ => rti.get_export(&store, name).ok_or(format!("the runtime has no {name}"))?,
        };
        plinker.define("env", name, ext).map_err(|e| format!("{name}: {e}"))?;
    }
    plinker
        .instantiate_and_start(&mut store, &prog)
        .map_err(|e| format!("the module links: {e}"))?;
    let start = rti.get_func(&store, "_start").expect("a command exports _start");
    let before = store.get_fuel().expect("fuel is on");
    let finished = start.call(&mut store, &[], &mut []);
    let fuel = before - store.get_fuel().expect("fuel is on");
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
    })
}

/// `program`, staged beside an entry that imports it, built natively; returns
/// the emitted module.
fn emitted(program: &Path, work: &Path) -> String {
    let name = program.file_stem().and_then(|s| s.to_str()).expect("named");
    let stage = work.join(name);
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the stage makes");
    std::fs::copy(program, stage.join(format!("{name}.kso"))).expect("the program copies");
    let entry = format!("run_{name}");
    std::fs::write(
        stage.join(format!("{entry}.kso")),
        format!("import \"./{name}\"\n\n{name}/play\n"),
    )
    .expect("the entry writes");
    run(
        Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("build")
            .arg(format!("{entry}.kso"))
            .current_dir(&stage),
        &format!("{name} builds"),
    );
    std::fs::read_to_string(stage.join(format!("{entry}.ll"))).expect("the ir reads")
}

fn golden(program: &Path, ext: &str) -> String {
    std::fs::read_to_string(program.with_extension(ext)).unwrap_or_default()
}

#[test]
fn the_micro_corpus_answers_the_same_through_the_translator() {
    if !toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = std::env::temp_dir().join(format!("kanso-irwasm-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("the work dir makes");
    let rt = runtime(&work);
    let micro = root().join("tests/golden/micro");
    let only = std::env::var("KANSO_IRWASM_ONLY").ok();
    let mut programs: Vec<PathBuf> = std::fs::read_dir(&micro)
        .expect("the corpus lists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "kso"))
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|s| s.contains("\npub play")))
        .filter(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            !HOST.iter().any(|(h, _)| *h == stem)
                && only.as_deref().is_none_or(|o| stem.contains(o))
        })
        .collect();
    programs.sort();
    assert!(only.is_some() || programs.len() > 250, "the corpus read {} programs", programs.len());
    let mut failures = Vec::new();
    for p in &programs {
        let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let ir = kanso::codegen::retarget_wasm32(&emitted(p, &work));
        let side = match kanso::ir_wasm::translate(&ir) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{name}: translate: {e}"));
                continue;
            }
        };
        std::fs::write(work.join(format!("{name}.wasm")), &side.wasm).expect("the module writes");
        match execute(&rt, &side) {
            Err(e) => failures.push(format!("{name}: {e}")),
            Ok(ran) => {
                let want_exit: i32 = golden(p, "exit").trim().parse().unwrap_or(0);
                if ran.out != golden(p, "out")
                    || ran.err != golden(p, "err")
                    || ran.exit != want_exit
                {
                    failures.push(format!(
                        "{name}: out {:?} err {:?} exit {} (fuel {})",
                        ran.out.chars().take(200).collect::<String>(),
                        ran.err.chars().take(200).collect::<String>(),
                        ran.exit,
                        ran.fuel
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} programs differ:\n{}",
        failures.len(),
        programs.len(),
        failures.join("\n")
    );
}

/// The interpreted row's corpus, the browser rows' workload, through the
/// translator. It prints its answer and the fuel it spent, the number the
/// tab's registry and the clang build of the same module are measured against.
#[test]
fn the_interp_corpus_answers_through_the_translator() {
    if !toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = std::env::temp_dir().join(format!("kanso-irwasm-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("the work dir makes");
    let rt = runtime(&work);
    let stage = work.join("corpus");
    std::fs::create_dir_all(stage.join("interp_corpus")).expect("the module dir makes");
    let from = root().join("bench/interp_corpus/interp_corpus");
    for f in std::fs::read_dir(&from).expect("the corpus lists") {
        let f = f.expect("an entry").path();
        std::fs::copy(&f, stage.join("interp_corpus").join(f.file_name().expect("named")))
            .expect("the file copies");
    }
    std::fs::write(stage.join("main.kso"), "import \"./interp_corpus\"\n\ninterp_corpus/play\n")
        .expect("the entry writes");
    run(
        Command::new(env!("CARGO_BIN_EXE_kanso")).arg("build").arg("main.kso").current_dir(&stage),
        "the corpus builds",
    );
    let native = Command::new(stage.join("main")).current_dir(&stage).output().expect("it runs");
    let ir = std::fs::read_to_string(stage.join("main.ll")).expect("the ir reads");
    let side =
        kanso::ir_wasm::translate(&kanso::codegen::retarget_wasm32(&ir)).expect("it translates");
    let ran = execute(&rt, &side).expect("it runs");
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(ran.out, String::from_utf8_lossy(&native.stdout), "the corpus's answer");
    assert_eq!(ran.exit, 0);
    println!(
        "translated, interp corpus: {} wasm instructions, module {} bytes, data {} bytes, {} table slots",
        ran.fuel,
        side.wasm.len(),
        side.data,
        side.table
    );
}
