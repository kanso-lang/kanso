//! Joins the runtime's parts into the one text the compiler embeds.
//!
//! `src/runtime.c` lists the parts as `#include` lines, which is how clang
//! reads them when it compiles that file by path. The compiler cannot hand
//! clang a path: it embeds the runtime and writes it out when a build needs
//! it, so it embeds the parts joined, in the order the list gives them.
//! `kanso::RUNTIME_C` is that text.

use std::path::Path;

fn main() {
    let list = Path::new("src/runtime.c");
    println!("cargo:rerun-if-changed={}", list.display());
    let text = std::fs::read_to_string(list).expect("src/runtime.c reads");
    let mut joined = String::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let part = line
            .strip_prefix("#include \"")
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| {
                panic!("src/runtime.c holds only comments and its parts, and `{line}` is neither")
            });
        let path = Path::new("src").join(part);
        println!("cargo:rerun-if-changed={}", path.display());
        joined.push_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
        );
    }
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR for a build script");
    std::fs::write(Path::new(&out).join("runtime.c"), joined).expect("the joined runtime writes");

    // The compiler links with one writable segment. With RELRO, lld writes
    // two, and when the first ends on a page boundary the two are contiguous
    // in the file and in memory; valgrind 3.22 then reads no symbols from the
    // executable, and every gate that reads a frame of the compiler
    // (`kanso::main`, `run_interpreted_on_stack`, `emit_ir_for`) has nothing
    // to read. Where that boundary falls depends on the binary's size, so any
    // change can land on it; kanso#2004 did.
    // tests/the_compiler_links_one_writable_segment.rs holds this.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("linux") {
        println!("cargo:rustc-link-arg-bin=kanso=-Wl,-z,norelro");
    }
}
