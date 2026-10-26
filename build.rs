//! Joins the runtime's parts into the one text the compiler embeds.
//!
//! `src/runtime.c` lists the parts as `#include` lines, which is how clang
//! reads them when it compiles that file by path. The compiler cannot hand
//! clang a path: it embeds the runtime and writes it out when a build needs
//! it, so it embeds the parts joined, in the order the list gives them.
//! `kanso::RUNTIME_C` is that text.
//!
//! The emitter writes some of the runtime's numbers and names into the IR, so
//! they are read from the joined text here, never copied: the tag enum becomes
//! `K_INT` through `K_BIG`, and the functions defined with `K_DOORCC` become
//! `PRESERVE_NONE_DOORS`. `src/codegen.rs` includes what this writes as
//! `runtime_abi.rs`.

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
    std::fs::write(Path::new(&out).join("runtime.c"), &joined).expect("the joined runtime writes");
    std::fs::write(Path::new(&out).join("runtime_abi.rs"), runtime_abi(&joined))
        .expect("the runtime's numbers write");

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

/// The tags and doors of the runtime, as Rust.
///
/// A door is a function the program calls with `preserve_nonecc`; calling one
/// on the C convention passes its arguments in the wrong registers, and the
/// program reads garbage rather than failing to build. So every line that
/// opens with `K_DOORCC` must be `K_DOORCC KValue name(` and end in `{`, a
/// definition, or `;`, a forward declaration, and the build stops on any
/// other: a door this reader passed over would be called on the wrong
/// convention without a word.
fn runtime_abi(runtime: &str) -> String {
    let line = runtime
        .lines()
        .find(|l| l.starts_with("enum { K_INT,"))
        .expect("the runtime declares its tag enum on one line, opening with K_INT");
    let mut out =
        String::from("// Written by build.rs from the runtime's tag enum and its doors.\n");
    let tags = line.trim_start_matches("enum {").trim_end().trim_end_matches("};");
    for (at, name) in tags.split(',').map(str::trim).enumerate() {
        out.push_str(&format!("pub const {name}: i64 = {at};\n"));
    }
    let mut doors = Vec::new();
    for line in runtime.lines().filter(|l| l.starts_with("K_DOORCC")) {
        let door = line
            .strip_prefix("K_DOORCC KValue ")
            .and_then(|rest| rest.split_once('('))
            .map(|(name, _)| name)
            .filter(|name| name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric()));
        match (door, line.trim_end().chars().last()) {
            (Some(name), Some('{')) => doors.push(name),
            (Some(_), Some(';')) => {}
            _ => panic!(
                "a door is `K_DOORCC KValue name(` on one line, ending in `{{` or `;`: `{line}`"
            ),
        }
    }
    out.push_str(&format!("pub const PRESERVE_NONE_DOORS: [&str; {}] = {doors:?};\n", doors.len()));
    out
}
