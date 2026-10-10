//! The third engine, inside `cargo test`.
//!
//! The differential law says all three engines agree, but every other test
//! binary runs two: native and `--interp`. The wasm engine was reached only
//! through headless Chrome, which cannot live in `cargo test`, so a wasm-only
//! divergence was invisible locally by construction — and that is how a
//! dispatch failure on a synthesised getter reached CI.
//!
//! This runs the same corpus against `docs/kanso.wasm` under an embedded
//! interpreter, the way the page does: the toolchain compiles a program with
//! the native emitter, lowers the module to wasm, and the program runs on
//! `runtime.c` built for wasm32. Chrome remains the confirmation that a real
//! browser agrees; it stops being the only thing that would notice.
//!
//! A host without the wasm32 toolchain (clang, wasm-ld and wasi-libc) cannot
//! build the runtime, so a spec that runs a program skips there. The Linux
//! specs job installs the toolchain and refuses the skip.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use wasmi::{Engine, Extern, Linker, Module, Store, Val};

#[path = "support/wasm32.rs"]
mod wasm32;

/// The playground pins the dice so a program calling `random` compares two
/// streams rather than one; the native side gets the same value. The page
/// spells it in decimal and JavaScript wraps it into the i32 the export
/// takes, so the wrap is written out here rather than left to a coercion.
const SEED: i32 = 2685821657u32 as i32;
const SEED_TEXT: &str = "2685821657";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `docs/kanso.wasm` is a build artifact, and every spec in this file runs it.
/// It is no longer committed, so a tree that has not built one has no blob at
/// all, and a blob built before the last edit to `src/` is older than the
/// compiler it stands for. Both are refusals: neither file is evidence about
/// this source. The check lives in `Toolchain::load` rather than at the call
/// sites because it was written at two of them and there are eleven.
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
    if newer.is_empty() {
        return Ok(());
    }
    // read_dir answers in whatever order the filesystem holds, so the refusal
    // named the same files in a different sequence on each run and two reports
    // of one failure did not compare.
    newer.sort();
    Err(format!("docs/kanso.wasm predates {}", newer.join(", ")))
}

struct Toolchain {
    store: Store<()>,
    instance: wasmi::Instance,
}

/// The runtime a compiled program links against, built once for the whole
/// binary by the playground's own script. None on a host that cannot build it.
fn runtime() -> Option<&'static [u8]> {
    static BUILT: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            if !wasm32::toolchain() {
                return None;
            }
            let dir = std::env::temp_dir().join(format!("kanso-page-rt-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("the runtime's directory makes");
            Some(wasm32::runtime(&dir))
        })
        .as_deref()
}

/// The toolchain, on a host that can run what it compiles.
fn page() -> Option<Toolchain> {
    match runtime() {
        Some(_) => Some(Toolchain::load()),
        None => {
            eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
            None
        }
    }
}

impl Toolchain {
    fn load() -> Toolchain {
        if let Err(stale) = freshness() {
            panic!("{stale} — run scripts/build_wasm.sh before this can prove anything");
        }
        let engine = Engine::default();
        let bytes = std::fs::read(root().join("docs/kanso.wasm")).expect("the wasm artifact reads");
        let module = Module::new(&engine, &bytes[..]).expect("the artifact is a wasm module");
        let mut store = Store::new(&engine, ());
        let instance = Linker::new(&engine)
            .instantiate_and_start(&mut store, &module)
            .expect("the toolchain instantiates");
        Toolchain { store, instance }
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

    /// Copy a string into the toolchain's heap, the way the page does. Every
    /// pointer and length the engine answers is an i32 on the wire and an
    /// unsigned address in memory, so each is read through `u32`.
    fn write(&mut self, text: &str) -> (i32, i32) {
        let len = text.len() as i32;
        let ptr = self.i32_call("kanso_alloc", &[Val::I32(len)]);
        let memory = self.memory();
        memory
            .write(&mut self.store, ptr as u32 as usize, text.as_bytes())
            .expect("the allocation is writable");
        (ptr, len)
    }

    fn output(&mut self) -> String {
        let ptr = self.i32_call("kanso_out_ptr", &[]) as u32 as usize;
        let len = self.i32_call("kanso_out_len", &[]) as u32 as usize;
        let mut bytes = vec![0u8; len];
        self.memory().read(&self.store, ptr, &mut bytes).expect("the output buffer reads");
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Hand the toolchain one program the way the page does, answering whether
    /// it takes the play door and where the source to compile sits.
    fn prepare(&mut self, name: &str, source: &str) -> (bool, i32, i32) {
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
        (plays, ptr, len)
    }
}

impl Toolchain {
    /// Compile one program the way `kanso build` does, lower its module in
    /// the toolchain (`kanso_compile_native`), and run it against the wasm32
    /// runtime: the route the playground takes, answering what a user would
    /// see.
    fn run(&mut self, name: &str, source: &str) -> Answer {
        let (plays, ptr, len) = self.prepare(name, source);
        let door = match plays {
            true => "kanso_play_native",
            false => "kanso_compile_native",
        };
        let status = self.i32_call(door, &[Val::I32(ptr), Val::I32(len)]);
        if status == 2 {
            return Answer::CompileError(self.output());
        }
        if status == 1 {
            return Answer::Declined(self.output());
        }
        let ptr = self.i32_call("kanso_wasm_ptr", &[]) as u32 as usize;
        let len = self.i32_call("kanso_wasm_len", &[]) as u32 as usize;
        let mut side = vec![0u8; len];
        self.memory().read(&self.store, ptr, &mut side).expect("the side module reads");
        let data = self.i32_call("kanso_side_data", &[]) as u32;
        let table = self.i32_call("kanso_side_table", &[]) as u32;
        let rt = runtime().expect("a spec that runs a program asks for the page first");
        match wasm32::execute(rt, &side, data, table, &[("KANSO_SEED", SEED_TEXT)]) {
            Ok(ran) => Answer::Ran(ran.exit, ran.out + &ran.err),
            // Native catches a stack overflow and says so; wasm's call stack
            // ends in a trap, and the page answers it with the same sentence.
            Err(e) if e.contains("call stack exhausted") => {
                Answer::Ran(1, format!("{}\n", kanso::stack_exhausted()))
            }
            Err(e) => Answer::Ran(-1, format!("the host stopped it: {e}")),
        }
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
    let work = std::env::temp_dir().join("kanso-wasm-diagnostics");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("a directory of its own");
    let Some(mut toolchain) = page() else { return };
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

/// A pointer the toolchain hands back is a wasm i32, and past two gibibytes
/// its top bit is set. Read as signed, a buffer that is fine lands at a
/// negative offset and the page reads nothing. The random programs found it
/// when the older engine kept every value a run made until the next run.
/// Growing the toolchain's memory first puts the compiled module above the
/// line, and the sixteen-megabyte answer then comes back through the runtime.
#[test]
fn an_answer_above_two_gibibytes_reads_back() {
    let source = "import \"std/text\"\n\nfn doubled s 0\n  s\n\nfn doubled s n\n  \
                  doubled (text/join [s s] \"\") (n - 1)\n\npub play = print (doubled \"ab\" 23)\n";
    let Some(mut toolchain) = page() else { return };
    let pages = toolchain.memory().size(&toolchain.store);
    let above = (1u64 << 31) / 65536 + 16;
    toolchain.memory().grow(&mut toolchain.store, above - pages).expect("the memory grows");
    let Answer::Ran(code, text) = toolchain.run("above.kso", source) else {
        panic!("the program did not run on wasm");
    };
    assert_eq!(code, 0, "{}", &text[..text.len().min(200)]);
    assert_eq!(text.len(), 2 * (1 << 23) + 1, "the answer came back cut");
    assert!(text.ends_with("abab\n"), "the answer came back wrong");
}

/// An accumulator the linearity analysis proves nobody else reads is extended
/// where it stands. The older engine copied the list on every `push` and
/// filled its four gibibytes at thirty-two thousand; a map built by `put` to
/// the same size answered 1 and printed nothing. The page now runs runtime.c,
/// which extends both in place as native does.
#[test]
fn an_accumulator_grows_where_it_stands() {
    let pushes = "fn fill acc 0\n  acc\n\nfn fill acc n\n  fill (push acc n) (n - 1)\n\n\
                  pub play = print (length (fill [] 32000))\n";
    let puts = "fn fill m 0\n  m\n\nfn fill m n\n  fill (put m n n) (n - 1)\n\n\
                pub play = print (length (entries (fill {} 32000)))\n";
    for (name, source) in [("pushes.kso", pushes), ("puts.kso", puts)] {
        let Some(mut toolchain) = page() else { return };
        let answer = toolchain.run(name, source);
        assert!(
            matches!(&answer, Answer::Ran(0, text) if text == "32000\n"),
            "{name} answered {answer:?}"
        );
    }
}

/// A string built by joining onto itself grows where it stands, and a seed is
/// not written through: the second program seeds three loops with the same
/// `"ab"`. The older engine kept every string such a loop made, about 2.4 GB
/// for seventy thousand one-letter joins.
#[test]
fn a_string_built_onto_itself_grows_where_it_stands() {
    let long = "fn build s 0\n  s\n\nfn build s n\n  build \"{s}x\" (n - 1)\n\n\
                pub play = print (length (build \"\" 70000))\n";
    let seeded = "fn build s 0\n  s\n\nfn build s n\n  build \"{s}x\" (n - 1)\n\n\
                  pub play = print \"{build \"ab\" 2} {build \"ab\" 3} {build \"ab\" 1}\"\n";
    for (name, source, want) in
        [("long.kso", long, "70000\n"), ("seeded.kso", seeded, "abxx abxxx abx\n")]
    {
        let Some(mut toolchain) = page() else { return };
        let answer = toolchain.run(name, source);
        assert!(
            matches!(&answer, Answer::Ran(0, text) if text == want),
            "{name} answered {answer:?}"
        );
    }
}

/// A loop seeded with a list its caller reads again copies the seed once, and
/// the caller's `seed` still reads `[0]`. A map written with `put` goes the
/// same way, and a loop that returns its seed untouched hands back a list the
/// next push copies rather than writes. The older engine copied on every lap
/// and filled its memory at seventy thousand pushes.
#[test]
fn a_loop_seeded_with_a_list_its_caller_reads_copies_it_once() {
    let source = "fn fill acc 0\n  acc\n\nfn fill acc n\n  fill (push acc n) (n - 1)\n\n\
                  fn keyed m 0\n  m\n\nfn keyed m n\n  keyed (put m \"k{n}\" n) (n - 1)\n\n\
                  seed = [0]\n\nbase = { \"a\":1 }\n\nkept = fill seed 0\n\n\
                  print \"{length (fill seed 70000)} {seed}\"\n\
                  print \"{length (keyed base 30000)} {base}\"\n\
                  print \"{push kept 9} {seed}\"\n";
    let Some(mut toolchain) = page() else { return };
    let answer = toolchain.run("seeded.kso", source);
    let want = "70001 [0]\n30001 { \"a\":1 }\n[0 9] [0]\n";
    assert!(matches!(&answer, Answer::Ran(0, text) if text == want), "{answer:?}");
}

/// A binding the demand analysis made lazy runs when something reads it, and
/// not before. `v = nope 2` fails if it runs, and its one use hands it to a
/// parameter the second arm of `f4` ignores, so every engine prints 7. When
/// the first arm returns the binding instead, the print reads it, and every
/// engine fails on it. The older browser engine ran every binding where it
/// stood and died on `nope` in the first case.
#[test]
fn a_lazy_binding_nobody_reads_never_runs() {
    let defs = "fn nope 1\n  1\n\nfn f4 0 x\n  x\n\nfn f4 _ _\n  7\n\n\
                fn shown z\n  v = nope 2\n  f4 z v\n\n";
    for (name, call, want) in [
        ("ignored.kso", "print (shown 1)\n", Answer::Ran(0, "7\n".to_string())),
        (
            "read.kso",
            "print (shown 0)\n",
            Answer::Ran(
                1,
                "error[runtime]: no overload of `nope` matches these arguments\n".to_string(),
            ),
        ),
    ] {
        let Some(mut toolchain) = page() else { return };
        let answer = toolchain.run(name, &format!("{defs}{call}"));
        assert_eq!(format!("{answer:?}"), format!("{want:?}"), "{name}");
    }
}

/// Two errs meeting in one operation merge, and a merged err was born
/// nowhere, so its report has no `born in` line. An err with a single birth
/// keeps the site it was born at. The older browser engine stamped the
/// operation's site on the merge.
#[test]
fn a_merged_err_reports_no_birth_site() {
    let defs = "import \"std/text\"\n\nfn shown z\n  v = text/to_float \"{z}x\"\n";
    let reason = "\"\"0x\" is not a number\"";
    for (name, body, want) in [
        (
            "merged.kso",
            "  v / v\n",
            format!("error[endpoint]: unhandled err reached the entry: [{reason} {reason}]\n"),
        ),
        (
            "single.kso",
            "  v + 1\n",
            format!(
                "error[endpoint]: unhandled err reached the entry: {reason}\n  \
                 born in text/to_float at std/text/text.kso:89\n"
            ),
        ),
    ] {
        let Some(mut toolchain) = page() else { return };
        let answer = toolchain.run(name, &format!("{defs}{body}\nprint (shown 0)\n"));
        assert_eq!(format!("{answer:?}"), format!("{:?}", Answer::Ran(1, want)), "{name}");
    }
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

/// A program that dies leaves the toolchain usable for the next one. Both
/// programs mention the same knotted binding; the second must still compile
/// and answer the blackhole's sentence.
#[test]
fn a_program_that_dies_leaves_the_engine_usable() {
    let knot = "type box\n  v\n\nd = (box d).v\n\n";
    let blackhole = "error[runtime]: a lazy binding demands its own value\n";
    let Some(mut toolchain) = page() else { return };

    toolchain.run("dies.kso", &format!("{knot}fn go x\n  x\n\npub play = print \"{{go d}}\"\n"));
    let after = toolchain.run("after.kso", &format!("{knot}pub play = print \"{{d}}\"\n"));

    assert!(
        matches!(&after, Answer::Ran(1, text) if text == blackhole),
        "a program run after one that died answered {after:?}"
    );
}

/// A program that runs out of stack says so, and the program after it runs.
/// The wasm call stack ends in a trap, and the page answers it with the
/// sentence native prints.
#[test]
fn a_program_that_runs_out_of_stack_leaves_the_engine_usable() {
    let deep = std::fs::read_to_string(root().join("tests/golden/runtime/deep_recursion.kso"))
        .expect("the deep recursion fixture reads");
    let Some(mut toolchain) = page() else { return };

    let fell = toolchain.run("deep_recursion.kso", &deep);
    let after = toolchain.run("after.kso", "print \"{1 + 2}\"\n");

    assert!(
        matches!(&fell, Answer::Ran(1, text) if text.contains("ran out of stack")),
        "the recursion answered {fell:?}"
    );
    assert!(
        matches!(&after, Answer::Ran(0, text) if text == "3\n"),
        "a program run after one that ran out of stack answered {after:?}"
    );
}

/// A deliberate exit is the one err an endpoint reads rather than reports:
/// `os/exit 3` yields an err carrying `os/exit_status 3`, and the program
/// said what it meant. The page runs `k_exit_status` from runtime.c, the
/// endpoint native runs, and the runtime's `proc_exit` carries the code out.
/// The older browser engine printed `unhandled err reached the executor` and
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
    let Some(mut toolchain) = page() else { return };
    let source = "import \"std/io\"\nimport \"std/os\"\n\n\
                  pub play = io/write \"before\" .> (_ -> os/exit 3)\n";

    let answer = toolchain.run("a_deliberate_exit.kso", source);

    assert!(
        matches!(&answer, Answer::Ran(3, text) if text == "before"),
        "the page lost the code the program named: {answer:?}"
    );
}

/// A count is the front end's answer, so the page gives it too.
///
/// This is the half no corpus reaches. The browser differential reads
/// `examples`, `tests/golden/runtime` and `tests/golden/micro` — programs
/// that RUN — and a program refused at compile time is in none of them, so
/// the sentence the page says for a wrong builtin count was a thing nobody
/// had read. Before this it ran the program: the count was tested where the
/// call happened, and the call never happened.
#[test]
fn the_page_refuses_a_wrong_builtin_count() {
    let mut toolchain = Toolchain::load();
    let said = match toolchain.run(
        "a_builtin_called_with_the_wrong_count.kso",
        "fn sized xs\n  length xs xs\n\npub play = print \"alive\"\n",
    ) {
        Answer::CompileError(said) => said,
        Answer::Ran(code, out) => panic!("the page ran it, {code}: {out}"),
        Answer::Declined(said) => panic!("the page declined instead of refusing: {said}"),
    };
    assert!(
        said.contains("`length` takes 1 argument(s), got 2"),
        "the page said something else: {said}"
    );
}

/// The whole error corpus, on the engine no corpus reached.
///
/// The browser differential reads `examples`, `tests/golden/runtime` and
/// `tests/golden/micro` — programs that RUN — so a program refused at compile
/// time was held to a golden on two engines and to nothing on the third. The
/// differential law does not have a clause for that: three engines agree or
/// the quiet ones refuse in words somebody has read.
///
/// Every fixture must reach the same sentence the other two do, byte for
/// byte. There is no gap list, because there are no gaps: the front end is
/// shared, and a diagnostic that differs here is one of the engines having
/// its own copy of something. The count is asserted so a corpus that stops
/// being walked cannot pass by walking none of it.
#[test]
fn the_page_refuses_the_error_corpus_the_way_the_others_do() {
    let dir = root().join("tests/golden/errors");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the error corpus reads")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "kso"))
        .collect();
    files.sort();
    assert!(files.len() > 150, "the error corpus shrank to {}", files.len());

    let mut toolchain = Toolchain::load();
    let mut wrong = Vec::new();
    let mut read = 0;
    for path in &files {
        let name = path.file_name().expect("fixtures have names").to_string_lossy().to_string();
        let source = std::fs::read_to_string(path).expect("a fixture reads");
        // the `.imported.stderr` twin, where one exists, is what a fixture
        // says when it is reached as a module — which is how both this and
        // the native corpus runner stage it
        let imported = path.with_extension("imported.stderr");
        let golden = match imported.exists() {
            true => std::fs::read_to_string(&imported),
            false => std::fs::read_to_string(path.with_extension("stderr")),
        }
        .expect("every fixture carries a golden");
        read += 1;
        let said = match toolchain.run(&name, &source) {
            Answer::CompileError(said) => said,
            Answer::Ran(code, out) => format!("<ran, {code}> {out}"),
            Answer::Declined(said) => format!("<declined> {said}"),
        };
        if said != golden {
            wrong.push(format!("{name}\n  page:   {said:?}\n  golden: {golden:?}"));
        }
    }
    assert_eq!(read, files.len(), "a fixture was skipped without saying so");
    assert!(wrong.is_empty(), "{} of {read} differ:\n{}", wrong.len(), wrong.join("\n"));
}

/// The most witnesses a construct can have and still be named. Three
/// independent programs survive losing one and still leave two behind, so
/// naming carriers past that buys nothing and would move this golden every
/// time an ordinary fixture lands.
const WITNESSES: usize = 3;

/// Which construct each `Expr` variant is, for the census below. Written out
/// rather than derived, so a new variant does not compile until somebody
/// decides what to call it and whether a program carries it.
fn shape(e: &kanso::ast::Expr) -> &'static str {
    use kanso::ast::Expr::*;
    match e {
        Int(..) => "Int",
        Float(..) => "Float",
        MapLit(..) => "MapLit",
        Str(..) => "Str",
        Ident(..) => "Ident",
        Partial(..) => "Partial",
        List(..) => "List",
        App { .. } => "App",
        Field { .. } => "Field",
        Index { .. } => "Index",
        Lambda { .. } => "Lambda",
        BinOp { .. } => "BinOp",
        Join { .. } => "Join",
        Block(..) => "Block",
        Upcast { .. } => "Upcast",
        Guard { .. } => "Guard",
    }
}

fn pattern_shape(p: &kanso::ast::Pattern) -> &'static str {
    use kanso::ast::Pattern::*;
    match p {
        IntLit(..) => "p:IntLit",
        StrLit(..) => "p:StrLit",
        Nullary(..) => "p:Nullary",
        Var(..) => "p:Var",
        Wildcard(..) => "p:Wildcard",
        Annotated { .. } => "p:Annotated",
        Ctor { .. } => "p:Ctor",
        Keyed { .. } => "p:Keyed",
    }
}

type Census = std::collections::BTreeMap<&'static str, std::collections::BTreeSet<String>>;

fn carries(census: &mut Census, what: &'static str, program: &str) {
    census.entry(what).or_default().insert(program.to_string());
}

/// Statements, walked as statements, so a census can say which statement
/// shapes the corpus carries as well as which expressions.
fn census_stmts(list: &[kanso::ast::Stmt], census: &mut Census, program: &str) {
    for st in list {
        match st {
            kanso::ast::Stmt::Bind { pattern, expr } => {
                carries(census, "s:Bind", program);
                carries(census, pattern_shape(pattern), program);
                census_expr(expr, census, program);
            }
            kanso::ast::Stmt::Expr(e) => {
                carries(census, "s:Expr", program);
                census_expr(e, census, program);
            }
        }
    }
}

fn census_expr(e: &kanso::ast::Expr, census: &mut Census, program: &str) {
    carries(census, shape(e), program);
    match e {
        kanso::ast::Expr::Block(list, _) => census_stmts(list, census, program),
        kanso::ast::Expr::Guard { cond, early, rest, .. } => {
            census_expr(cond, census, program);
            census_expr(early, census, program);
            census_stmts(rest, census, program);
        }
        _ => kanso::for_each_child(e, |c| census_expr(c, census, program)),
    }
}

/// Every construct the grammar has, carried by a program the page runs.
///
/// The corpus above is what holds the three engines to one answer, so a
/// construct no program in it uses is a construct the page is never asked
/// about. Nothing in the tree could see that. The widening upcast
/// `(expr):type` rides on a single program, and deleting that program — or
/// moving it to the gap list — would retire the construct from the
/// differential without turning anything red.
///
/// A construct also has to be carried by a program reached AS A MODULE, and
/// that is a second thing this asserts. Importing a module renames every name
/// it owns, and a construct holding a type or a name that the rename pass
/// forgets is broken only on that path. `Upcast` was carried by one program,
/// which runs definitions beside statements and is therefore never imported,
/// and the rename pass had never qualified an upcast's target: widening to a
/// type a module declares reported at runtime that the value was not one, on
/// the interpreter, and refused the module outright on both backends.
///
/// Two doors, because a corpus program is either a library exporting `play`
/// or declarations beside bare statements, and `parse` alone refuses the
/// second kind. A program that neither door reads fails here rather than
/// being skipped: seventeen were, silently, when this census was a scratch
/// file, and the hole is what made it report two constructs as carried by
/// nothing.
#[test]
fn every_construct_is_carried_by_a_program_the_page_runs() {
    let gapped: std::collections::BTreeSet<String> =
        route_gaps().into_iter().map(|(path, _)| path).collect();

    let mut census = Census::new();
    // the same census over the programs an import reaches, and therefore the
    // programs the rename pass walks
    let mut renamed = Census::new();
    let (mut read, mut skipped) = (0, 0);
    let mut unread = Vec::new();
    for path in corpus() {
        let listed = path
            .strip_prefix(root())
            .expect("the corpus lives under the root")
            .to_string_lossy()
            .to_string();
        if gapped.contains(&listed) {
            skipped += 1;
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("a corpus program reads");
        let Ok(lexed) = kanso::lexer::lex(&source) else {
            unread.push(format!("{listed}: does not lex"));
            continue;
        };
        let program = match kanso::parser::parse(&lexed) {
            Ok(p) => p,
            Err(_) => match kanso::parser::parse_play(&lexed) {
                Ok(p) => p,
                Err(diags) => {
                    unread.push(format!("{listed}: {}", diags[0].message));
                    continue;
                }
            },
        };
        read += 1;
        // the harness stages a program exporting `play` behind a generated
        // entry that imports it, which is the only way a corpus program meets
        // the rename pass
        let imported = source.contains("\npub play") || source.starts_with("pub play");
        for decl in &program.fns {
            for param in &decl.params {
                carries(&mut census, pattern_shape(param), &listed);
                if imported {
                    carries(&mut renamed, pattern_shape(param), &listed);
                }
            }
            census_stmts(&decl.body, &mut census, &listed);
            if imported {
                census_stmts(&decl.body, &mut renamed, &listed);
            }
        }
    }
    assert!(unread.is_empty(), "the census could not read:\n  {}", unread.join("\n  "));
    assert_eq!(read + skipped, corpus().len(), "the walk lost programs");

    let order = [
        "Int",
        "Float",
        "MapLit",
        "Str",
        "Ident",
        "Partial",
        "List",
        "App",
        "Field",
        "Index",
        "Lambda",
        "BinOp",
        "Join",
        "Block",
        "Upcast",
        "Guard",
        "p:IntLit",
        "p:StrLit",
        "p:Nullary",
        "p:Var",
        "p:Wildcard",
        "p:Annotated",
        "p:Ctor",
        "p:Keyed",
        "s:Bind",
        "s:Expr",
    ];
    let never_imported: Vec<&str> =
        order.iter().copied().filter(|what| !renamed.contains_key(what)).collect();
    assert!(
        never_imported.is_empty(),
        "carried only by programs nothing imports, so the rename an import performs \
         has never walked {never_imported:?} — add a fixture exporting `play` that uses it"
    );

    let mut here = String::new();
    for what in order {
        let carriers = census.remove(what).unwrap_or_default();
        let named = match carriers.len() {
            0 => "NOBODY".to_string(),
            n if n <= WITNESSES => carriers.into_iter().collect::<Vec<_>>().join(" "),
            _ => "many".to_string(),
        };
        here.push_str(&format!("{what}\t{named}\n"));
    }
    assert!(census.is_empty(), "a construct the order forgot: {:?}", census.keys());

    let golden = root().join("tests/golden/shapes.txt");
    if std::env::var("KANSO_REGEN_SHAPES").is_ok() {
        let head = std::fs::read_to_string(&golden)
            .map(|t| {
                t.lines()
                    .take_while(|l| l.starts_with('#') || l.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .expect("the golden's header stays");
        std::fs::write(&golden, format!("{head}\n{here}")).expect("the golden writes");
    }
    let want: String = std::fs::read_to_string(&golden)
        .expect("tests/golden/shapes.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(
        want, here,
        "the constructs the page-runnable corpus carries moved.\n\
         NOBODY means the last program using a construct left the corpus, and the page \
         stopped being asked about it — add a program rather than regenerating.\n\
         Otherwise regenerate with KANSO_REGEN_SHAPES=1 and say in the pull request which \
         construct gained or lost a carrier.\n\
         {read} programs read, {skipped} gapped"
    );
}

/// The page's half of the partial contract. `tests/partial.rs` holds the
/// interpreter and native; this is the third engine running the program the
/// two of them agree on (RULED 2026-08-29, "the backends build the partial
/// over a value"). Until then the page declined it as a limit of its own, and
/// this spec pinned the refusal.
#[test]
fn the_page_builds_a_partial_over_a_value() {
    let Some(mut toolchain) = page() else { return };
    let program =
        "fn add a b c\n  a + b + c\n\nfn foo f\n  &f 2\n\npub play = print \"{(foo add) 5 7}\"\n";
    match toolchain.run("of_param.kso", program) {
        Answer::Ran(code, out) => assert_eq!((code, out.as_str()), (0, "14\n")),
        Answer::Declined(said) => panic!("the page still declines it: {said}"),
        Answer::CompileError(said) => panic!("the front door refused it: {said}"),
    }
}

/// The native route's gaps: path and the text it answers instead, from
/// tests/golden/native_route_gaps.txt.
fn route_gaps() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(root().join("tests/golden/native_route_gaps.txt"))
        .expect("the route's gap list");
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, answer) = line.split_once('\t').expect("a gap is path<tab>answer");
            (name.to_string(), answer.to_string())
        })
        .collect()
}

/// The playground's route to native's layout, held to the corpus the current
/// engine is held to: every program compiled the way `kanso build` compiles
/// it, its module lowered by `ir_wasm` inside the toolchain, and run against
/// `runtime.c` built for wasm32. Each answer is compared with the native
/// binary's, stream and exit code, except where tests/golden/native_route_gaps.txt
/// says what the route answers instead.
#[test]
fn the_page_agrees_with_the_golden_corpus() {
    let Some(mut toolchain) = page() else { return };
    let gaps = route_gaps();
    let only = std::env::var("KANSO_ROUTE_ONLY").ok();
    let (mut ran, mut met, mut held, mut wrong) = (0, 0, Vec::new(), Vec::new());
    for path in corpus() {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let listed = path.strip_prefix(root()).unwrap_or(&path).to_string_lossy().to_string();
        if only.as_deref().is_some_and(|o| !listed.contains(o)) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("the program reads");
        // a local import wants a filesystem, which neither host has
        if wants_a_filesystem(&source) {
            held.push(listed);
            continue;
        }
        let (native_code, native_text) = natively(&path);
        let gap = gaps.iter().find(|(g, _)| *g == listed);
        let answer = match toolchain.run(&name, &source) {
            Answer::Ran(code, text) => (code, text),
            Answer::Declined(why) => (-2, format!("declined: {}", why.trim())),
            Answer::CompileError(text) => (native_code.max(1), text),
        };
        let agrees = answer.0 == native_code && answer.1 == native_text;
        match (agrees, gap) {
            (true, None) => ran += 1,
            (false, Some((_, said))) if answer.1.contains(said.as_str()) => met += 1,
            (true, Some(_)) => wrong.push(format!(
                "{listed} agrees with native now: take it off tests/golden/native_route_gaps.txt"
            )),
            (false, Some((_, said))) => wrong.push(format!(
                "{listed} is listed as answering `{said}` and answers {:?}",
                answer.1.chars().take(300).collect::<String>()
            )),
            (false, None) => wrong.push(format!(
                "{listed}: exit {} against {native_code}\n  page   {:?}\n  native {:?}",
                answer.0,
                answer.1.chars().take(300).collect::<String>(),
                native_text.chars().take(300).collect::<String>()
            )),
        }
    }
    println!("the page: {ran} agree, {met} known gaps, {} held out", held.len());
    assert!(wrong.is_empty(), "{} programs are wrong:\n{}", wrong.len(), wrong.join("\n"));
    if only.is_none() {
        assert_eq!(
            ran + met + held.len(),
            corpus().len(),
            "the walk lost programs: {ran} ran, {met} gaps, {} held out, {} in the corpus",
            held.len(),
            corpus().len()
        );
        assert_eq!(
            met,
            gaps.len(),
            "a program in tests/golden/native_route_gaps.txt was never reached"
        );
        // Held out by name rather than by count, because the way this goes
        // wrong is the filesystem test quietly widening: it once read the
        // start of an import line instead of its quoted path, and
        // examples/imports.kso sat out the differential while being a program
        // the page runs correctly. Nothing in the corpus imports a local
        // module today, so the list is empty.
        assert_eq!(held, Vec::<String>::new(), "the programs held out of the walk changed");
        // A directory that stops contributing is the other way coverage
        // collapses, and the total above cannot see it.
        for dir in ["examples", "tests/golden/runtime", "tests/golden/micro"] {
            let from_here = corpus()
                .iter()
                .filter(|p| p.strip_prefix(root()).unwrap_or(p).to_string_lossy().starts_with(dir))
                .count();
            assert!(from_here > 0, "{dir} contributed nothing to the walk");
        }
    }
}
