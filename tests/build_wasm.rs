//! `kanso build <entry> --wasm`: the program as a wasm side module the browser
//! links against the runtime built for wasm32, the way the playground runs a
//! program, written to a file a page can load.
//!
//! The module carries the two numbers its host needs before it can link it,
//! the bytes of data it asks for and the table slots, in a custom section
//! named `kanso.link`, as two little-endian u32s. This builds a program, reads
//! the section back, links the module against the runtime and runs it.

use std::path::PathBuf;
use std::process::Command;

#[path = "support/wasm32.rs"]
mod wasm32;
use wasm32::{execute, link, runtime, toolchain};

#[test]
fn a_program_built_for_the_browser_runs_linked_against_the_runtime() {
    if !toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work: PathBuf =
        std::env::temp_dir().join(format!("kanso-build-wasm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("the work dir makes");
    std::fs::write(work.join("greet.kso"), "pub fn greeting name\n  \"hello, {name}\"\n")
        .expect("the module writes");
    std::fs::write(work.join("page.kso"), "import \"./greet\"\n\nprint (greet/greeting \"tab\")\n")
        .expect("the entry writes");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "page.kso", "--wasm"])
        .current_dir(&work)
        .output()
        .expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    assert_eq!(String::from_utf8_lossy(&built.stdout), "built ./page.wasm (llvm ir at page.ll)\n");

    let module = std::fs::read(work.join("page.wasm")).expect("the module was written");
    let (data, table) =
        link(&module).expect("the module says how to link it: data bytes, then table slots");
    let ran =
        execute(&runtime(&work), &module, data, table, &[]).expect("the module links and runs");
    assert_eq!((ran.out.as_str(), ran.err.as_str(), ran.exit), ("hello, tab\n", "", 0));
    let _ = std::fs::remove_dir_all(&work);
}
