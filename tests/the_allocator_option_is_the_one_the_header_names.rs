//! The compiler sets two mimalloc options before its first allocation, and it
//! names each by a number. mimalloc's Rust bindings stop naming options well
//! before either, so the numbers in src/main.rs are copied out of the C
//! library's own header — and a copied number goes stale silently. A crate
//! bump that inserts one option above them would leave the compiler setting
//! whatever now sits at those positions, with the eager commit back on, the
//! purge timer back on, and nobody told.
//!
//! So this reads the header the build compiles, counts the enum, and asserts
//! every number agrees. The behaviour each one buys is pinned elsewhere:
//! tests/bind_chain_depth.rs reads resident memory and goes red when the arena
//! comes back, and the three compile instruction goldens go red when the purge
//! timer returns, because its `clock_gettime` makes the row a property of the
//! host's clocksource. This spec exists to say WHY when either happens.

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

/// A number src/main.rs hands `mi_option_set`, read off its constant.
fn the_number_the_compiler_uses(constant: &str) -> usize {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("src/main.rs reads");
    let want = format!("const {constant}:");
    let line = source
        .lines()
        .find(|l| l.trim_start().starts_with(&want))
        .unwrap_or_else(|| panic!("src/main.rs declares {constant}"));
    line.rsplit_once('=')
        .expect("the constant is assigned")
        .1
        .trim()
        .trim_end_matches(';')
        .parse()
        .expect("the constant is a number")
}

/// Every option the compiler sets, as (its constant in src/main.rs, the name
/// the header declares). Adding an option to the constructor and not to this
/// list leaves the new number unpinned, which is the whole failure this spec
/// exists to prevent — so `the_constructor_sets_only_options_this_spec_pins`
/// reads the constructor back and asserts the list is complete.
const OPTIONS: &[(&str, &str)] = &[
    ("ARENA_EAGER_COMMIT", "mi_option_arena_eager_commit"),
    ("PURGE_DELAY", "mi_option_purge_delay"),
];

#[test]
fn every_number_is_where_the_header_declares_its_option() {
    let header = the_header_the_build_compiles();
    let text = std::fs::read_to_string(&header).expect("the header reads");

    for (constant, option) in OPTIONS {
        let declared = position_in_the_enum(&text, option)
            .unwrap_or_else(|| panic!("the header declares {option}"));
        let used = the_number_the_compiler_uses(constant);
        assert_eq!(
            used,
            declared,
            "src/main.rs sets option {used} as {constant} and {} declares \
             {option} at {declared} — the compiler is setting the wrong option",
            header.display()
        );
    }
}

/// The list above has to be complete or the numbers it does not carry are
/// unpinned. The constructor is one function and it calls `mi_option_set` once
/// per option, so reading its body back names every constant in play.
#[test]
fn the_constructor_sets_only_options_this_spec_pins() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("src/main.rs reads");
    let body = source
        .split_once("fn set_the_allocator_before_it_runs()")
        .expect("src/main.rs declares the constructor")
        .1
        .split_once("\n}")
        .expect("the constructor has a body")
        .0;

    let mut set: Vec<&str> = Vec::new();
    for call in body.split("mi_option_set(").skip(1) {
        let arg = call.split(',').next().expect("mi_option_set takes an option").trim();
        set.push(arg);
    }

    assert!(!set.is_empty(), "the constructor sets no option at all");
    for constant in &set {
        assert!(
            OPTIONS.iter().any(|(named, _)| named == constant),
            "the constructor sets {constant} and OPTIONS does not name it, so \
             that number is copied out of the header and pinned by nothing"
        );
    }
    assert_eq!(
        set.len(),
        OPTIONS.len(),
        "OPTIONS names {} options and the constructor sets {}: {set:?}",
        OPTIONS.len(),
        set.len()
    );
}
