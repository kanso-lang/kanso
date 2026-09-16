//! The compiler turns one mimalloc option off before its first allocation, and
//! it names that option by a number. mimalloc's Rust bindings stop naming
//! options well before `mi_option_arena_eager_commit`, so the number in
//! src/main.rs is copied out of the C library's own header — and a copied
//! number goes stale silently. A crate bump that inserts one option above it
//! would leave the compiler turning off whatever now sits at position four,
//! with the eager commit back on and nobody told.
//!
//! So this reads the header the build compiles, counts the enum, and asserts
//! the two agree. The behaviour that depends on it is pinned next door in
//! tests/bind_chain_depth.rs, which reads resident memory and goes red when
//! the arena comes back; this spec exists to say WHY when that happens.

use std::path::PathBuf;

/// The first `mi_option_t` position that the header gives `name`, counting
/// declarations in order from zero. Aliases (`mi_option_large_os_pages =
/// mi_option_allow_large_os_pages`) sit after `_mi_option_last` and are not
/// positions, so the walk stops there.
fn position_in_the_enum(header: &str, name: &str) -> Option<usize> {
    let body =
        header.split_once("typedef enum mi_option_e {").expect("the header declares mi_option_e").1;

    let mut at = 0;
    for line in body.lines() {
        let code = line.split("//").next().unwrap_or("").trim();
        let declared = code.trim_end_matches(',').trim();
        if declared.is_empty() || declared.contains('=') || declared.contains('}') {
            continue;
        }
        if declared == "_mi_option_last" {
            break;
        }
        if declared == name {
            return Some(at);
        }
        at += 1;
    }
    None
}

/// libmimalloc-sys vendors both v2 and v3 of the C library and its build script
/// picks v3 unless the `v2` feature is on, which this crate does not set.
fn the_header_the_build_compiles() -> PathBuf {
    let home = std::env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("HOME").expect("a home directory")).join(".cargo")
    });
    let registry = home.join("registry").join("src");
    let mut found = Vec::new();
    for index in std::fs::read_dir(&registry).expect("the registry has been fetched") {
        let index = index.expect("a registry index directory").path();
        for crate_dir in std::fs::read_dir(&index).into_iter().flatten().flatten() {
            let name = crate_dir.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("libmimalloc-sys-") {
                continue;
            }
            let header = crate_dir.path().join("c_src/mimalloc/v3/include/mimalloc.h");
            if header.is_file() {
                found.push(header);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| panic!("no libmimalloc-sys header under {}", registry.display()))
}

/// The number src/main.rs hands `mi_option_set`.
fn the_number_the_compiler_uses() -> usize {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("src/main.rs reads");
    let line = source
        .lines()
        .find(|l| l.trim_start().starts_with("const ARENA_EAGER_COMMIT:"))
        .expect("src/main.rs names the option");
    line.rsplit_once('=')
        .expect("the constant is assigned")
        .1
        .trim()
        .trim_end_matches(';')
        .parse()
        .expect("the constant is a number")
}

#[test]
fn the_number_is_where_the_header_declares_the_option() {
    let header = the_header_the_build_compiles();
    let text = std::fs::read_to_string(&header).expect("the header reads");
    let declared = position_in_the_enum(&text, "mi_option_arena_eager_commit")
        .expect("the header declares mi_option_arena_eager_commit");

    assert_eq!(
        the_number_the_compiler_uses(),
        declared,
        "src/main.rs turns off option {} and {} declares \
         mi_option_arena_eager_commit at {declared} — the compiler is \
         setting the wrong option",
        the_number_the_compiler_uses(),
        header.display()
    );
}
