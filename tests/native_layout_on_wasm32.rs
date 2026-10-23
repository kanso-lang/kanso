//! Native's layout, built for wasm32, answers what native answers.
//!
//! The playground runs a program with every value held in a registry the
//! toolchain keeps, which cost the interpreter corpus 2.39 billion units of
//! wasmi fuel where native spends 13.8 million x86 instructions. The route to
//! closing that is the one the 2026-10-06 prompt names: values in linear
//! memory in native's layout, with `runtime.c` compiled to wasm32. This is the
//! first step of it, and it holds the step to the goldens before anything in
//! the tab depends on it.
//!
//! Each program in the micro corpus is built by `kanso build`, its module is
//! passed through `codegen::retarget_wasm32`, and clang builds the result for
//! wasm32 against `runtime.c` built the same way, with wasi-libc under both.
//! The module runs under the WASI host in `tests/support/wasm32.rs`, and its
//! stdout, stderr and exit code are compared with the program's goldens: the
//! same bytes the interpreter, native and the page are held to.
//!
//! WHAT THE BUILD NEEDS, and why each piece is what it is:
//!
//! - `-Xclang -target-abi -Xclang experimental-mv` when `runtime.c` is lowered
//!   to IR, and NOT when that IR is lowered to an object. The emitted module
//!   passes a `KValue` as two `i64` words, which is x86's C ABI for it; the
//!   wasm32 C ABI passes a struct through a pointer. The multivalue ABI
//!   expands it into the two words, so the two sides agree. Kept off the
//!   object step, the backend keeps the standard ABI for the 128-bit helpers
//!   wasi-libc calls, which is the ABI wasi-libc was built with.
//! - `wasm/include`, whose headers declare the process and socket calls the
//!   effect executor makes, and `wasm/shim.c`, which answers each as a
//!   failure and supplies the three 128-bit helpers no wasm32 compiler-rt on
//!   this toolchain has.
//! - `-mtail-call`, because the emitted module's `musttail` calls stay tail
//!   calls once their convention is dropped -- all but the ones returning a
//!   `KValue`. Without the multivalue ABI the backend returns that struct
//!   through a slot in the caller's frame, and a call that has to write into
//!   its caller's frame cannot replace it, so the backend makes an ordinary
//!   call and no error says so. A pair of groups that call each other two
//!   hundred thousand times then runs in one frame only because LLVM inlines
//!   one into the other and the pair becomes a loop.
//! - `-inline-threshold=2000` on the module, the threshold native's release
//!   link uses, for the same reason. At clang's default of 225 the inlining
//!   that turns such a pair into a loop held until ints could widen
//!   (2026-10-07): each step's int then carries a test for a bignum and a
//!   way to the general group, and `a_record_rebuilt_at_depth`'s step costs
//!   260 against 225 and ran out of stack. The multivalue ABI would make the
//!   calls real tail calls, and it broke the 128-bit helpers' products when
//!   tried on every object, so this harness keeps the standard ABI and the
//!   threshold release already uses.
//! - An 8 MB stack placed first. wasm-ld's default is 64 KB growing down into
//!   the data segment, and a twenty-step `.>` chain overran it and wrote over
//!   libc's `stdout`; native runs with 8 MB.
//!
//! Programs that need a process or a filesystem are listed in `HOST` with the
//! reason. The page has neither, and a WASI host that pretended to would be
//! testing the host.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[path = "support/wasm32.rs"]
mod wasm32;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Programs that need a call returning a `KValue` to be a real tail call.
/// This harness keeps the standard ABI on the emitted module, so such a call
/// is an ordinary one (see the header), and a loop that goes through a
/// function pointer cannot be inlined into one. The translator the page uses
/// makes the call a tail call, and `ir_to_wasm` runs this program.
const DEPTH: [(&str, &str); 1] = [(
    "a_lambda_called_in_tail_position_costs_no_stack",
    "jumps through a closure pointer 100,000 times",
)];

/// Programs whose answer is a fact about the host they run on.
const HOST: [(&str, &str); 8] = [
    ("process_run", "runs a command"),
    ("a_command_that_answers_127", "runs a command"),
    ("make_dir_is_idempotent", "makes a directory"),
    ("dir_listing_is_sorted", "lists a directory"),
    ("a_file_that_is_not_text", "reads a file"),
    ("is_dir", "asks the filesystem"),
    ("modified_reads_when_a_file_changed", "asks the filesystem"),
    ("write_err_stream", "orders two streams the host interleaves"),
];

const SYSROOT: &str = "/usr";
const WASI_LIB: &str = "/usr/lib/wasm32-wasi";

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(out.status.success(), "{what}: {}", String::from_utf8_lossy(&out.stderr));
}

/// `runtime.c` and the shim, built once into `dir`.
fn runtime(dir: &Path) -> (PathBuf, PathBuf) {
    let rt_ll = dir.join("runtime.ll");
    let rt_o = dir.join("runtime.o");
    let shim_o = dir.join("shim.o");
    let target = ["--target=wasm32-wasi", &format!("--sysroot={SYSROOT}"), "-O2", "-mtail-call"];
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
    run(
        Command::new("clang").args(target).args(["-w", "-c"]).arg(&rt_ll).arg("-o").arg(&rt_o),
        "the runtime builds for wasm32",
    );
    run(
        Command::new("clang")
            .args(target)
            .args(["-w", "-c"])
            .arg(root().join("wasm/shim.c"))
            .arg("-o")
            .arg(&shim_o),
        "the shim builds for wasm32",
    );
    (rt_o, shim_o)
}

/// A program's module, retargeted, built and linked into a WASI command.
fn link(stage: &Path, entry: &str, rt: &(PathBuf, PathBuf)) -> PathBuf {
    let native = std::fs::read_to_string(stage.join(format!("{entry}.ll"))).expect("the ir reads");
    let ll = stage.join(format!("{entry}.wasm32.ll"));
    std::fs::write(&ll, kanso::codegen::retarget_wasm32(&native)).expect("the ir writes");
    let obj = stage.join(format!("{entry}.wasm32.o"));
    run(
        Command::new("clang")
            .args(["--target=wasm32-wasi", &format!("--sysroot={SYSROOT}"), "-O2", "-mtail-call"])
            .args(["-mllvm", "-inline-threshold=2000"])
            .args(["-w", "-c"])
            .arg(&ll)
            .arg("-o")
            .arg(&obj),
        &format!("{entry} builds for wasm32"),
    );
    let wasm = stage.join(format!("{entry}.wasm"));
    run(
        Command::new("wasm-ld")
            .arg(Path::new(WASI_LIB).join("crt1-command.o"))
            .arg(&obj)
            .arg(&rt.0)
            .arg(&rt.1)
            .arg(format!("-L{WASI_LIB}"))
            .args(["-lc", "-lwasi-emulated-signal", "-lwasi-emulated-getpid"])
            .args(["--strip-all", "--stack-first", "-z", "stack-size=8388608", "-o"])
            .arg(&wasm),
        &format!("{entry} links for wasm32"),
    );
    wasm
}

/// A linked command, run by the shared WASI host.
fn execute(wasm: &Path) -> wasm32::Ran {
    let bytes = std::fs::read(wasm).expect("the module reads");
    wasm32::command(&bytes, &[]).unwrap_or_else(|e| panic!("{}: {e}", wasm.display()))
}

/// `program`, staged beside its siblings with an entry that imports it, built
/// natively and then for wasm32.
fn stage_and_link(program: &Path, rt: &(PathBuf, PathBuf), work: &Path) -> PathBuf {
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
    link(&stage, &entry, rt)
}

#[test]
fn the_micro_corpus_answers_the_same_on_native_layout_in_wasm32() {
    if !wasm32::toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = std::env::temp_dir().join(format!("kanso-wasm32-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("the work dir makes");
    let rt = runtime(&work);
    let micro = root().join("tests/golden/micro");
    let mut programs: Vec<PathBuf> = std::fs::read_dir(&micro)
        .expect("the corpus lists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "kso"))
        .filter(|p| std::fs::read_to_string(p).expect("the program reads").contains("\npub play"))
        .filter(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            !HOST.iter().chain(&DEPTH).any(|(name, _)| *name == stem)
        })
        .collect();
    programs.sort();
    assert!(programs.len() > 250, "the corpus read {} programs", programs.len());

    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(program) = programs.get(k) else { break };
                let wasm = stage_and_link(program, &rt, &work);
                let ran = execute(&wasm);
                let out = std::fs::read_to_string(program.with_extension("out"))
                    .expect("a played program has an .out golden");
                let err =
                    std::fs::read_to_string(program.with_extension("err")).unwrap_or_default();
                if ran.out != out || ran.err != err || ran.exit != 0 {
                    failures.lock().expect("the list locks").push(format!(
                        "{}: exit {}\n  stdout {:?}\n  want   {:?}\n  stderr {:?}\n  want   {:?}",
                        program.display(),
                        ran.exit,
                        ran.out,
                        out,
                        ran.err,
                        err
                    ));
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&work);
    let failures = failures.into_inner().expect("the list unlocks");
    assert!(
        failures.is_empty(),
        "{} of {} programs answer differently on native's layout in wasm32:\n{}",
        failures.len(),
        programs.len(),
        failures.join("\n")
    );
}

/// The interpreted row's corpus, the browser rows' workload, on native's
/// layout. It prints its answer and the wasm instructions it executed.
#[test]
fn the_interp_corpus_answers_on_native_layout_in_wasm32() {
    if !wasm32::toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = std::env::temp_dir().join(format!("kanso-wasm32-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("the work dir makes");
    let rt = runtime(&work);
    let stage = work.join("corpus");
    std::fs::create_dir_all(&stage).expect("the stage makes");
    let from = root().join("bench/interp_corpus/interp_corpus");
    std::fs::create_dir_all(stage.join("interp_corpus")).expect("the module dir makes");
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
    let ran = execute(&link(&stage, "main", &rt));
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(ran.out, String::from_utf8_lossy(&native.stdout), "the corpus's answer");
    assert_eq!(ran.exit, 0);
    println!("native layout on wasm32, interp corpus: {} wasm instructions", ran.executed);
}

/// Every line `WASM32_LINES` rewrites is one the emitter writes, so a helper
/// whose text changes turns this red rather than slipping past the rewrite.
#[test]
fn every_wasm32_line_is_one_the_emitter_writes() {
    // The table is in codegen/wasm32.rs, the body emitter in codegen.rs and
    // emit.rs, the call twins in module.rs, and the runtime's lines in
    // declares.rs.
    let read = |p: &str| std::fs::read_to_string(root().join(p)).expect("codegen reads");
    let emitter = read("src/codegen/wasm32.rs")
        + &read("src/codegen.rs")
        + &read("src/codegen/module.rs")
        + &read("src/codegen/emit.rs")
        + &read("src/codegen/declares.rs");
    for (native, _) in kanso::codegen::WASM32_LINES {
        let hits = emitter.matches(native).count();
        // once in the table, at least once where it is written
        assert!(hits >= 2, "`{native}` is no longer a line the emitter writes");
    }
}
