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
                // A program that dies inside a closure body aborts, and an
                // abort is a trap. Expecting success here turned every such
                // diagnostic into a panic in the harness, so the engine looked
                // silent on the one path where a lambda reports anything.
                target.call(&mut caller, &[Val::I32(env), Val::I32(arg)], &mut out)?;
                Ok(out[0].i32().expect("a closure answers an i32"))
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
        // A program that exports `play` is a library: the engine is handed it
        // under the name an import will use, and compiles the entry that runs
        // it — the same two files the native engine is given, with no
        // filesystem under either of them.
        self.call("kanso_forget_sources", &[], &mut []).expect("the sources clear");
        let stem = name.strip_suffix(".kso").unwrap_or(name).to_string();
        let library = source.contains("\npub play") || source.starts_with("pub play");
        // definitions beside statements: the play door, wherever the file lives
        let tops: Vec<&str> =
            source.lines().filter(|l| !l.is_empty() && !l.starts_with(' ')).collect();
        let declares = tops.iter().any(|l| l.starts_with("fn ") || l.starts_with("type "));
        let states = tops.iter().any(|l| {
            let plain = !l.starts_with("import ")
                && !l.starts_with("fn ")
                && !l.starts_with("type ")
                && !l.starts_with('#');
            let binds = l.split_once(" = ").is_some_and(|(head, _)| !head.contains(' '));
            plain && !binds
        });
        let plays = !library && declares && states;
        let (compiled_name, compiled) = match library {
            true => (format!("run_{stem}.kso"), format!("import \"./{stem}\"\n\n{stem}/play\n")),
            false => (name.to_string(), source.to_string()),
        };
        if library {
            let (path_ptr, path_len) = self.write(&stem);
            let (file_ptr, file_len) = self.write(name);
            let (src_ptr, src_len) = self.write(source);
            self.call(
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
            )
            .expect("the library is accepted");
        }
        let (name_ptr, name_len) = self.write(&compiled_name);
        self.call("kanso_set_seed", &[Val::I32(SEED)], &mut []).expect("the seed is accepted");
        self.call("kanso_set_file", &[Val::I32(name_ptr), Val::I32(name_len)], &mut [])
            .expect("the file name is accepted");
        let (ptr, len) = self.write(&compiled);
        // tail calls on, as every browser this targets reports them: a
        // self-call that must not grow the stack is a different program
        // without them, and comparing that to the golden compares two
        // different programs
        let door = match plays {
            true => "kanso_play_wasm",
            false => "kanso_compile_wasm",
        };
        let status = self.i32_call(door, &[Val::I32(ptr), Val::I32(len), Val::I32(1)]);
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
        // The path is the QUOTED part, which is not always where the line
        // starts. `import t { slice:cut } "std/text"` names the stdlib and
        // reads as a local import to a check that only looks at the prefix —
        // which is how examples/imports.kso sat out the differential while
        // being a program the page can run perfectly well.
        line.starts_with("import ")
            && !imported_path(line).is_some_and(|path| path.starts_with("std/"))
    })
}

/// The quoted module path on an import line, whatever alias or selection
/// stands between the keyword and it.
fn imported_path(line: &str) -> Option<&str> {
    let open = line.find('"')? + 1;
    let rest = &line[open..];
    Some(&rest[..rest.find('"')?])
}

/// A program the wasm engine cannot survive long enough to be compared with.
/// It has no blackhole, so an unguarded knot recurses until the stack ends it
/// — in Chrome that is a diagnostic and lands in wasm_gaps.txt, but this
/// runner shares the test process's stack and aborts it before any comparison
/// can happen. The gap list cannot help: it is consulted after the run.
///
/// Named one at a time rather than pattern-matched, so closing the guard
/// empties this list visibly.
fn outruns_the_runners_stack(listed: &str) -> bool {
    listed == "tests/golden/runtime/a_guarded_shape_that_is_not_a_knot.kso"
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

/// A corpus program that exports `play` is a library, so the native engine
/// is handed the entry that imports it — staged once per corpus directory,
/// because programs read fixtures that sit beside them.
fn native_entry(path: &Path) -> (PathBuf, String, &'static str) {
    let dir = path.parent().expect("a program has a directory").to_path_buf();
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let source = std::fs::read_to_string(path).expect("the program reads");
    if !source.contains("\npub play") && !source.starts_with("pub play") {
        let verb = match play_shaped(&source) {
            true => "play",
            false => "run",
        };
        return (dir, name, verb);
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let stage =
        std::env::temp_dir().join("kanso-wasm-native").join(dir.file_name().unwrap_or_default());
    static STAGED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> =
        std::sync::OnceLock::new();
    let mut done = STAGED.get_or_init(Default::default).lock().expect("the staging lock");
    if done.insert(stage.clone()) {
        let _ = std::fs::remove_dir_all(&stage);
        stage_tree(&dir, &stage);
    }
    drop(done);
    // The directory is copied once, for the fixtures beside the program; the
    // program itself is copied every time, because a caller that writes one
    // probe after another to the same name would otherwise be answered by the
    // first one forever.
    std::fs::copy(path, stage.join(&name)).expect("the program copies");
    let entry = format!("run_{stem}.kso");
    std::fs::write(stage.join(&entry), format!("import \"./{stem}\"\n\n{stem}/play\n"))
        .expect("the entry file writes");
    (stage, entry, "run")
}

/// Declarations beside bare statements: the play door, wherever the file
/// lives. A binding is not a statement — a file of `fn`s and `test_` bindings
/// is a test file, which has a verb of its own.
fn play_shaped(source: &str) -> bool {
    let tops: Vec<&str> = source.lines().filter(|l| !l.is_empty() && !l.starts_with(' ')).collect();
    let declares = tops.iter().any(|l| l.starts_with("fn ") || l.starts_with("type "));
    let states = tops.iter().any(|l| {
        let plain = !l.starts_with("import ")
            && !l.starts_with("fn ")
            && !l.starts_with("type ")
            && !l.starts_with('#');
        let binds = l.split_once(" = ").is_some_and(|(head, _)| !head.contains(' '));
        plain && !binds
    });
    declares && states
}

fn stage_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the staging directory is made");
    for entry in std::fs::read_dir(from).expect("the corpus is readable") {
        let path = entry.expect("directory entry").path();
        let landing = to.join(path.file_name().expect("entries have names"));
        match path.is_dir() {
            true => stage_tree(&path, &landing),
            false => {
                std::fs::copy(&path, &landing).expect("the file copies");
            }
        }
    }
}

/// What the native engine does with a program: the merged stream and the
/// exit code, which is the comparison the Chrome harness makes. The `.out`
/// goldens pin stdout alone and the wasm engine has one output area, so
/// comparing against them would compare two different things.
fn natively(path: &Path) -> (i32, String) {
    // from the program's own directory, under its bare name: an err stamps
    // the path it was given, and the wasm side is only ever given a basename
    let (dir, entry, verb) = native_entry(path);
    let done = std::process::Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args([verb, &entry])
        .current_dir(&dir)
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
    let mut skipped: Vec<(String, &str)> = Vec::new();
    for path in corpus() {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let listed = path.strip_prefix(root()).unwrap_or(&path).to_string_lossy().to_string();
        let source = std::fs::read_to_string(&path).expect("the program reads");
        // Both reasons used to print as "relative import", so the one program
        // skipped for outrunning the stack was described as something else
        // entirely. A skip is a hole in the differential; the line that
        // records it has to say which hole.
        let why = match (wants_a_filesystem(&source), outruns_the_runners_stack(&listed)) {
            (true, _) => Some("a local import, and neither host has a filesystem"),
            (_, true) => Some("it outruns the runner's stack before it can be compared"),
            _ => None,
        };
        if let Some(why) = why {
            skipped.push((listed.clone(), why));
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
                // the wasm backend translates a getter's internal name too, and
                // it is the one engine no other test can watch doing it
                assert!(
                    !text.contains(&kanso::ast::getter_name("")),
                    "{name} showed a getter its internal name on wasm: {text}"
                );
                let (native_code, native_text) = natively(&path);
                assert_eq!(text, native_text, "wasm and native disagree on {name}");
                assert_eq!(code, native_code, "wasm and native exit differently on {name}");
                ran += 1;
            }
            // A refusal is the weaker outcome and needs the same watching as a
            // wrong answer: the differential law allows an engine to speak
            // fewer features only where it rejects them plainly, which makes
            // the rejection the thing that has to be written down.
            (Answer::Declined(reason), Some((_, answer))) => {
                assert!(
                    reason.contains(answer),
                    "{listed} is a known gap answering `{answer}`, and the backend now \
                     declines with `{}` — close it or restate it in \
                     tests/golden/wasm_gaps.txt",
                    reason.trim()
                );
                met += 1;
            }
            (Answer::Declined(reason), None) => panic!(
                "{listed} is refused by the wasm backend with `{}`, and no gap says so. \
                 Add it to tests/golden/wasm_gaps.txt or close it — a refusal nothing \
                 records is a gap the corpus cannot see.",
                reason.trim()
            ),
            (Answer::CompileError(text), _) => panic!("{name} fails to compile on wasm: {text}"),
        }
    }
    // `ran > 0` stood here, which one surviving program would satisfy. A walk
    // is worth what it covers, so what is asserted is the ACCOUNTING: every
    // program in the corpus was run, met as a listed gap, or skipped for a
    // reason named on this list — and nothing fell off it quietly.
    assert_eq!(
        ran + met + skipped.len(),
        corpus().len(),
        "the walk lost programs: {ran} ran, {met} gaps, {} skipped, {} in the corpus",
        skipped.len(),
        corpus().len()
    );

    // The skip list is pinned by NAME rather than by count, because the way
    // this goes wrong is a predicate quietly widening. `wants_a_filesystem`
    // read the start of an import line instead of its quoted path, so
    // `import t { slice:cut } "std/text"` looked local and examples/imports.kso
    // sat out the differential — a program the page runs correctly.
    let expected_skips = [(
        "tests/golden/runtime/a_guarded_shape_that_is_not_a_knot.kso",
        "it outruns the runner's stack before it can be compared",
    )];
    let seen: Vec<(&str, &str)> = skipped.iter().map(|(n, w)| (n.as_str(), *w)).collect();
    assert_eq!(
        seen,
        expected_skips.to_vec(),
        "the set of programs held out of the wasm differential changed"
    );

    // A directory that stops contributing is the other way coverage collapses,
    // and the total above cannot see it: `corpus()` would shrink and the
    // accounting would still balance. Each walked directory answers for itself.
    for dir in ["examples", "tests/golden/runtime", "tests/golden/micro"] {
        let from_here = corpus()
            .iter()
            .filter(|p| p.strip_prefix(root()).unwrap_or(p).to_string_lossy().starts_with(dir))
            .count();
        assert!(from_here > 0, "{dir} contributed nothing to the wasm walk");
    }
    // A gap this runner cannot execute is still a gap — Chrome runs it and
    // holds it to the listed answer. Counting it here would demand a run that
    // ends the test process.
    let unrunnable = gaps.iter().filter(|(listed, _)| outruns_the_runners_stack(listed)).count();
    assert_eq!(
        met + unrunnable,
        gaps.len(),
        "a program in tests/golden/wasm_gaps.txt was never reached"
    );
    println!("wasm: {ran} agree, {met} known gaps, {} held out", skipped.len());
    for (name, why) in &skipped {
        println!("  skipped {name} ({why})");
    }
}

/// Every exported std function in the shipped library, with how many
/// arguments it takes. Derived from `lib/` the same way
/// `scripts/diagnostic_differential.py` derives it, because the rule is the
/// same rule and the source is the same source — a shared list would go stale
/// against the library it describes.
fn std_surface() -> Vec<(String, String, usize)> {
    let mut found = Vec::new();
    for module in std::fs::read_dir(root().join("lib")).expect("the library reads") {
        let module = module.expect("a module entry").path();
        if !module.is_dir() {
            continue;
        }
        let name = module.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&module)
            .expect("a module's files")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "kso"))
            .collect();
        files.sort();
        for file in files {
            if file.to_string_lossy().ends_with("_test.kso") {
                continue;
            }
            for line in std::fs::read_to_string(&file).expect("a module file reads").lines() {
                let Some(rest) = line.strip_prefix("pub fn ") else { continue };
                let mut words = rest.split_whitespace();
                let Some(fname) = words.next() else { continue };
                let params: Vec<&str> = words.collect();
                // a destructuring parameter means the arm takes a shape rather
                // than a value; another arm of the group takes the value
                if params.iter().any(|p| p.contains('(')) {
                    continue;
                }
                found.push((name.clone(), fname.to_string(), params.len()));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The diagnostics differential on the third engine. The script holds native
/// against the interpreter; this holds wasm against native, so all three say
/// the same thing when a program asks a std function for the wrong thing.
#[test]
fn the_wasm_engine_complains_the_way_the_others_do() {
    if let Err(stale) = freshness() {
        panic!("{stale} — run scripts/build_wasm.sh before this can prove anything");
    }
    let work = std::env::temp_dir().join("kanso-wasm-diagnostics");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("a directory of its own");
    let mut toolchain = Toolchain::load();
    let (mut asked, mut declined) = (0, 0);
    for (module, name, arity) in std_surface() {
        if arity == 0 {
            continue;
        }
        let args = vec!["bad"; arity].join(" ");
        let source = format!(
            "import \"std/{module}\"\n\ntype wrong\n  a\n  b\n\npub play =\n  \
             bad = wrong 1 2\n  print \"{{{module}/{name} {args}}}\"\n"
        );
        let probe = work.join("probe.kso");
        std::fs::write(&probe, &source).expect("the probe writes");
        match toolchain.run("probe.kso", &source) {
            Answer::Ran(_, text) => {
                let (_, native_text) = natively(&probe);
                assert_eq!(
                    text, native_text,
                    "wasm and native complain differently about {module}/{name}"
                );
                asked += 1;
            }
            // a backend that refuses the program up front has said so, which
            // is the exemption the differential law grants an engine that
            // declines rather than diverges
            Answer::Declined(_) | Answer::CompileError(_) => declined += 1,
        }
    }
    assert!(asked > 0, "no std function was asked for the wrong thing");
    println!("wasm: {asked} std complaints match native, {declined} declined by the backend");
}

impl Toolchain {
    /// One line at the playground's prompt, the way the page sends it.
    fn prompt(&mut self, line: &str) -> (i32, String) {
        let (ptr, len) = self.write(line);
        let code = self.i32_call("kanso_repl_eval", &[Val::I32(ptr), Val::I32(len)]);
        (code, self.output())
    }
}

/// The page is the copy of the repl most people meet, and nothing drove it.
/// The session compiles the way a file does now, so an import at the prompt
/// has to reach the shipped library here too — a browser has no filesystem,
/// and the modules are carried in the binary for exactly this.
#[test]
fn the_playground_prompt_reaches_the_library() {
    let mut toolchain = Toolchain::load();

    let (code, said) = toolchain.prompt("import \"std/list\"");
    assert_eq!(code, 0, "the import was refused: {said}");
    assert_eq!(said.trim(), "imported list", "{said}");

    let (code, answer) = toolchain.prompt("list/sum [1 2 3]");
    assert_eq!(code, 0, "the module was unreachable: {answer}");
    assert_eq!(answer.trim(), "6", "{answer}");
}

/// A path naming no module leaves the page's session as it was.
#[test]
fn the_playground_prompt_refuses_a_module_that_is_not_there() {
    let mut toolchain = Toolchain::load();

    let (code, said) = toolchain.prompt("import \"std/nope\"");
    assert_eq!(code, 1, "a missing module was accepted: {said}");
    assert!(said.contains("not in the shipped library"), "{said}");

    let (code, said) = toolchain.prompt("import \"std/math\"");
    assert_eq!(code, 0, "the session did not survive: {said}");
    assert_eq!(said.trim(), "imported math", "{said}");
}

/// The page's echo for an ordinary declaration, which is where the doubling
/// shows if there is one.
#[test]
fn the_playground_echoes_a_declaration_once() {
    let mut toolchain = Toolchain::load();
    let (code, said) = toolchain.prompt("fn doubled n\n  n * 2");
    assert_eq!(code, 0, "{said}");
    assert_eq!(said.trim(), "defined doubled", "{said}");
}

/// The page gets the directive too, because directives live on the session.
#[test]
fn the_playground_prompt_can_start_over() {
    let mut toolchain = Toolchain::load();
    toolchain.prompt("fn doubled n\n  n * 2");

    let (code, said) = toolchain.prompt(":reset");
    assert_eq!(code, 0, "{said}");
    assert_eq!(said.trim(), "session cleared", "{said}");

    let (code, gone) = toolchain.prompt("doubled 4");
    assert_eq!(code, 1, "the declaration survived the reset: {gone}");
}

/// A program that dies leaves the module instance usable. `with_interp` used
/// to hold the INTERP borrow across evaluation, so an abort inside it left the
/// cell borrowed forever and the NEXT program's `load` could not take it — the
/// failure surfaced as a compiler trap on a program that is fine by itself.
/// Both mention the same knotted description, which is what reaches the abort.
#[test]
fn a_program_that_dies_leaves_the_engine_usable() {
    let knot = "type box\n  v\n\nfn use b\n  print \"y {b}\"\n\nd = print \"x\" >> use (box d)\n\n";
    let blackhole = "error[runtime]: a lazy binding demands its own value\n";
    let mut toolchain = Toolchain::load();

    toolchain.run("dies.kso", &format!("{knot}fn go x\n  x\n\npub play = go d\n"));
    let after = toolchain.run("after.kso", &format!("{knot}pub play = d\n"));

    assert!(
        matches!(&after, Answer::Ran(1, text) if text == blackhole),
        "a program run after one that died answered {after:?}"
    );
}

/// A deliberate exit is the one err an endpoint reads rather than reports:
/// `os/exit 3` yields an err carrying `os/exit_status 3`, and the program
/// said what it meant. Three of the four endpoints already knew this —
/// `k_exit_status` in the emitted C runtime, and `eval::deliberate_exit`
/// from the driver — but `exec_main` here did not, so a page that called
/// `os/exit` printed `unhandled err reached the executor` at its reader and
/// answered 1 whatever code the program named.
///
/// The corpus walk holds the zero case
/// (`tests/golden/micro/a_deliberate_exit_says_nothing.kso`) against native.
/// Neither corpus can carry a NONZERO one — micro asserts every program in
/// it exits 0 and the runtime corpus asserts every program in it exits 1 —
/// so the code passing through is pinned here for this engine and in
/// tests/a_deliberate_exit_carries_its_code.rs for the other two.
#[test]
fn a_deliberate_exit_carries_its_code_out_of_the_page() {
    let mut toolchain = Toolchain::load();
    let source = "import \"std/io\"\nimport \"std/os\"\n\n\
                  pub play = io/write \"before\" >> os/exit 3\n";

    let answer = toolchain.run("a_deliberate_exit.kso", source);

    assert!(
        matches!(&answer, Answer::Ran(3, text) if text == "before"),
        "the page lost the code the program named: {answer:?}"
    );
}
