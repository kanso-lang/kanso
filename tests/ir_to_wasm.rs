//! The tab's own route to native's layout: the emitter's IR, lowered to wasm by
//! `ir_wasm`, linked at instantiation against `runtime.c` built for wasm32.
//!
//! The runtime is built once by `scripts/build_runtime_wasm.sh`, the script the
//! playground's build runs, as a module that exports its memory, table, stack
//! pointer and every symbol. Each program is built by
//! `kanso build`, and its module is retargeted and translated with no clang
//! involved. The host instantiates the runtime, sets aside the data and table
//! room the translated module asks for, instantiates the module against the
//! runtime's exports and calls the runtime's `_start`. Stdout, stderr and
//! exit are compared with the program's goldens.

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "support/wasm32.rs"]
mod wasm32;
use wasm32::{execute, runtime, toolchain};

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

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(out.status.success(), "{what}: {}", String::from_utf8_lossy(&out.stderr));
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
        match execute(&rt, &side.wasm, side.data, side.table, &[]) {
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
    let ran = execute(&rt, &side.wasm, side.data, side.table, &[]).expect("it runs");
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

/// Translates the module at `KANSO_IRWASM_PROFILE` once, for a profiler to
/// watch: `cargo test --release --test ir_to_wasm translate_one -- --ignored`.
#[test]
#[ignore]
fn translate_one() {
    let Ok(path) = std::env::var("KANSO_IRWASM_PROFILE") else { return };
    let ir = std::fs::read_to_string(path).expect("the ir reads");
    let side = kanso::ir_wasm::translate(&kanso::codegen::retarget_wasm32(&ir)).expect("it translates");
    println!("{} bytes", side.wasm.len());
}
