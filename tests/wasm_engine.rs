//! The third engine, inside `cargo test`.
//!
//! The differential law says all three engines agree, but every other test
//! binary runs two: native and `--interp`. The wasm engine was reached only
//! through headless Chrome, which cannot live in `cargo test`, so a wasm-only
//! divergence was invisible locally by construction — and that is how a
//! dispatch failure on a synthesised getter reached CI.
//!
//! This runs the same corpus against `docs/kanso.wasm` under an embedded
//! interpreter. Chrome remains the confirmation that a real browser agrees;
//! it stops being the only thing that would notice.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use wasmi::{Caller, Engine, Extern, Func, Linker, Module, Store, Table, Val};

/// The playground pins the dice so a program calling `random` compares two
/// streams rather than one; the native side gets the same value. The page
/// spells it in decimal and JavaScript wraps it into the i32 the export
/// takes, so the wrap is written out here rather than left to a coercion.
const SEED: i32 = 2685821657u32 as i32;
const SEED_TEXT: &str = "2685821657";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The program module's function table, published after it is instantiated.
/// The toolchain calls a kanso closure through `k_callback`, which has to
/// reach a table that does not exist when the toolchain is instantiated —
/// so the host holds the cell both sides meet in.
#[derive(Default, Clone)]
struct Dispatch(Rc<RefCell<Option<Table>>>);

/// `docs/kanso.wasm` is a build artifact that is also committed, so a stale
/// one would let this whole file pass while proving nothing about the source.
/// Compare it against everything it is built from rather than trusting it is
/// current. CI rebuilds it in this job for the same reason: a fresh clone
/// stamps every file with the checkout time, and an artifact that is merely
/// as old as the source is not evidence about the source.
fn freshness() -> Result<(), String> {
    let art = root().join("docs/kanso.wasm");
    let Ok(built) = art.metadata().and_then(|m| m.modified()) else {
        return Err("docs/kanso.wasm is missing".to_string());
    };
    let mut newer = Vec::new();
    for entry in std::fs::read_dir(root().join("src")).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let touched = path.metadata().and_then(|m| m.modified()).map_err(|e| e.to_string())?;
        if touched > built {
            newer.push(path.file_name().unwrap_or_default().to_string_lossy().to_string());
        }
    }
    match newer.is_empty() {
        true => Ok(()),
        false => Err(format!("docs/kanso.wasm predates {}", newer.join(", "))),
    }
}

struct Toolchain {
    store: Store<Dispatch>,
    instance: wasmi::Instance,
    engine: Engine,
}

impl Toolchain {
    fn load() -> Toolchain {
        let engine = Engine::default();
        let bytes = std::fs::read(root().join("docs/kanso.wasm")).expect("the wasm artifact reads");
        let module = Module::new(&engine, &bytes[..]).expect("the artifact is a wasm module");
        let mut store = Store::new(&engine, Dispatch::default());
        let mut linker = Linker::new(&engine);
        let callback = Func::wrap(
            &mut store,
            |mut caller: Caller<'_, Dispatch>, handle: i32, env: i32, arg: i32| -> i32 {
                let table = caller.data().0.borrow().expect("a closure ran before main");
                let target = table
                    .get(&mut caller, handle as u64)
                    .expect("the handle is in the table")
                    .funcref()
                    .and_then(|f| f.val().copied().copied())
                    .expect("the table entry is a function");
                let mut out = [Val::I32(0)];
                target
                    .call(&mut caller, &[Val::I32(env), Val::I32(arg)], &mut out)
                    .expect("the closure body runs");
                out[0].i32().expect("a closure answers an i32")
            },
        );
        linker.define("env", "k_callback", callback).expect("k_callback is the one import");
        let instance =
            linker.instantiate_and_start(&mut store, &module).expect("the toolchain instantiates");
        Toolchain { store, instance, engine }
    }

    fn call(&mut self, name: &str, args: &[Val], results: &mut [Val]) -> Result<(), wasmi::Error> {
        let func = self
            .instance
            .get_func(&self.store, name)
            .unwrap_or_else(|| panic!("the toolchain exports {name}"));
        func.call(&mut self.store, args, results)
    }

    fn i32_call(&mut self, name: &str, args: &[Val]) -> i32 {
        let mut out = [Val::I32(0)];
        self.call(name, args, &mut out).unwrap_or_else(|e| panic!("{name}: {e}"));
        out[0].i32().unwrap_or_else(|| panic!("{name} answers an i32"))
    }

    fn memory(&self) -> wasmi::Memory {
        match self.instance.get_export(&self.store, "memory") {
            Some(Extern::Memory(m)) => m,
            _ => panic!("the toolchain exports its memory"),
        }
    }

    /// Copy a string into the toolchain's heap, the way the page does.
    fn write(&mut self, text: &str) -> (i32, i32) {
        let len = text.len() as i32;
        let ptr = self.i32_call("kanso_alloc", &[Val::I32(len)]);
        let memory = self.memory();
        memory
            .write(&mut self.store, ptr as usize, text.as_bytes())
            .expect("the allocation is writable");
        (ptr, len)
    }

    fn output(&mut self) -> String {
        let ptr = self.i32_call("kanso_out_ptr", &[]) as usize;
        let len = self.i32_call("kanso_out_len", &[]) as usize;
        let mut bytes = vec![0u8; len];
        self.memory().read(&self.store, ptr, &mut bytes).expect("the output buffer reads");
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Compile and run one program, answering what a user would see.
    fn run(&mut self, name: &str, source: &str) -> Answer {
        let (name_ptr, name_len) = self.write(name);
        self.call("kanso_set_seed", &[Val::I32(SEED)], &mut []).expect("the seed is accepted");
        self.call("kanso_set_file", &[Val::I32(name_ptr), Val::I32(name_len)], &mut [])
            .expect("the file name is accepted");
        let (ptr, len) = self.write(source);
        // tail calls on, as every browser this targets reports them: a
        // self-call that must not grow the stack is a different program
        // without them, and comparing that to the golden compares two
        // different programs
        let status =
            self.i32_call("kanso_compile_wasm", &[Val::I32(ptr), Val::I32(len), Val::I32(1)]);
        if status == 2 {
            return Answer::CompileError(self.output());
        }
        if status == 1 {
            return Answer::Declined(self.output());
        }
        let ptr = self.i32_call("kanso_wasm_ptr", &[]) as usize;
        let len = self.i32_call("kanso_wasm_len", &[]) as usize;
        let mut emitted = vec![0u8; len];
        self.memory().read(&self.store, ptr, &mut emitted).expect("the emitted module reads");

        let module = Module::new(&self.engine, &emitted[..]).expect("the emitted module parses");
        let mut linker = Linker::new(&self.engine);
        // every rt_* the toolchain exports is an import the program wants
        let wanted: Vec<String> =
            module.imports().map(|import| import.name().to_string()).collect();
        for name in wanted {
            let Some(Extern::Func(f)) = self.instance.get_export(&self.store, &name) else {
                panic!("the program imports `{name}`, which the toolchain does not export");
            };
            linker.define("env", &name, f).expect("the shim defines");
        }
        let program = linker
            .instantiate_and_start(&mut self.store, &module)
            .expect("the program instantiates");
        match program.get_export(&self.store, "table") {
            Some(Extern::Table(t)) => *self.store.data().0.borrow_mut() = Some(t),
            _ => panic!("the program exports no function table"),
        }

        let main = program.get_func(&self.store, "main").expect("the program exports main");
        let mut handle = [Val::I32(0)];
        if main.call(&mut self.store, &[], &mut handle).is_err() {
            let _ = self.call("kanso_take_rt_error", &[], &mut []);
            return Answer::Ran(1, self.output());
        }
        let mut code = [Val::I32(0)];
        let exec = self.instance.get_func(&self.store, "kanso_exec_main").expect("kanso_exec_main");
        if exec.call(&mut self.store, &handle, &mut code).is_err() {
            let _ = self.call("kanso_take_rt_error", &[], &mut []);
            return Answer::Ran(1, self.output());
        }
        Answer::Ran(code[0].i32().unwrap_or(1), self.output())
    }
}

#[derive(Debug)]
enum Answer {
    Ran(i32, String),
    /// The backend refuses the program up front and says why; the playground
    /// falls back to the interpreter, so this is a stated gap, not a failure.
    Declined(String),
    CompileError(String),
}

/// The gaps the wasm engine has, and what it answers instead. Shared with
/// the Chrome harness so a gap is written down once; a listed program that
/// starts passing is a failure here, because a closed gap left on the list
/// is a lie about the engine.
fn known_gaps() -> Vec<(String, String)> {
    let text =
        std::fs::read_to_string(root().join("tests/golden/wasm_gaps.txt")).expect("the gap list");
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, answer) = line.split_once('\t').expect("a gap is path<tab>answer");
            (name.to_string(), answer.to_string())
        })
        .collect()
}

/// A relative import wants a filesystem, which neither host has; `std/`
/// resolves because the toolchain embeds it. Mirrored from the Chrome
/// harness, where it is the same three lines.
fn wants_a_filesystem(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("import ") && !line.starts_with("import \"std/")
    })
}

/// The same three directories the Chrome harness walks, so the two engines
/// are held to one corpus rather than to two that drift apart.
fn corpus() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in ["examples", "tests/golden/runtime", "tests/golden/micro"] {
        let dir = root().join(dir);
        let mut here: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "kso"))
            .collect();
        here.sort();
        found.extend(here);
    }
    found
}

/// What the native engine does with a program: the merged stream and the
/// exit code, which is the comparison the Chrome harness makes. The `.out`
/// goldens pin stdout alone and the wasm engine has one output area, so
/// comparing against them would compare two different things.
fn natively(path: &Path) -> (i32, String) {
    // from the program's own directory, under its bare name: an err stamps
    // the path it was given, and the wasm side is only ever given a basename
    let done = std::process::Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", &path.file_name().unwrap_or_default().to_string_lossy()])
        .current_dir(path.parent().expect("a program has a directory"))
        .env("KANSO_SEED", SEED_TEXT)
        .output()
        .expect("the native engine runs");
    let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&done.stderr));
    (done.status.code().unwrap_or(1), text)
}

/// The micro corpus is one construct per program, so a divergence names the
/// construct. Every program the wasm backend accepts must answer exactly what
/// the native engine answers — the differential law, inside cargo test.
#[test]
fn the_wasm_engine_agrees_with_the_golden_corpus() {
    if let Err(stale) = freshness() {
        panic!("{stale} — run scripts/build_wasm.sh before this can prove anything");
    }
    let gaps = known_gaps();
    let mut toolchain = Toolchain::load();
    let (mut ran, mut met) = (0, 0);
    let (mut declined, mut skipped) = (Vec::new(), Vec::new());
    for path in corpus() {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let listed = path.strip_prefix(root()).unwrap_or(&path).to_string_lossy().to_string();
        let source = std::fs::read_to_string(&path).expect("the program reads");
        if wants_a_filesystem(&source) {
            skipped.push(listed.clone());
            continue;
        }
        let gap = gaps.iter().find(|(listed_name, _)| *listed_name == listed);
        match (toolchain.run(&name, &source), gap) {
            (Answer::Ran(_, text), Some((_, answer))) => {
                assert!(
                    text.contains(answer),
                    "{listed} is a known gap answering `{answer}`, and it now answers `{}` \
                     — close it or restate it in tests/golden/wasm_gaps.txt",
                    text.trim()
                );
                met += 1;
            }
            (Answer::Ran(code, text), None) => {
                let (native_code, native_text) = natively(&path);
                assert_eq!(text, native_text, "wasm and native disagree on {name}");
                assert_eq!(code, native_code, "wasm and native exit differently on {name}");
                ran += 1;
            }
            (Answer::Declined(reason), _) => declined.push(format!("{name}: {}", reason.trim())),
            (Answer::CompileError(text), _) => panic!("{name} fails to compile on wasm: {text}"),
        }
    }
    assert!(ran > 0, "nothing in the corpus ran on wasm");
    assert_eq!(met, gaps.len(), "a program in tests/golden/wasm_gaps.txt was never reached");
    println!(
        "wasm: {ran} agree, {met} known gaps, {} declined, {} need a filesystem",
        declined.len(),
        skipped.len()
    );
    for gap in &declined {
        println!("  declined {gap}");
    }
    for name in &skipped {
        println!("  skipped {name} (relative import — neither host has a filesystem)");
    }
}
