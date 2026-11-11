//! The measured link is handed its runtime objects under names no digest
//! decides.
//!
//! A cached runtime object's name carries a digest: of the runtime's text,
//! and for the hot unit of `src/main.rs`'s own bytes as well. The linker
//! keeps its inputs' names in a string map, and how far a probe walks
//! depends on the string, which `the_measured_link_names_its_object.rs`
//! found for the LTO object. On 2026-11-11 a change that added two lines to
//! `main.rs`, and changed nothing a build emits, renamed the hot unit from
//! `..._hot_6ab23eac654b1fdf.o` to `..._hot_1f12ca05e76f21b9.o`. clang
//! counted the same 173,563,803 instructions and the linker counted 17
//! more, here and in CI alike, and `codegen_instructions_release` moved
//! with them.
//!
//! A measured build sets `KANSO_FIXED_TEMPS`, and under it the link is
//! handed a copy of each cached object under a plain name. Any other build
//! links the cached file itself, since it has no row to keep.
//!
//! The spec puts a `clang` first on the path that writes down its
//! arguments and runs the real one, and reads the link's line.
//!
//! A plain name on the link was not the whole of it. An object records the
//! name of the file it was compiled from, as bitcode records its module's,
//! and the runtime's source was staged under a name with the same digest.
//! With the link's names already plain, a comment added to `main.rs` still
//! moved the linker by 64. The source now reaches clang on standard input,
//! and the last spec here reads the cached objects for any name of the
//! kind. After both, the same edit left both tiers' counts where they were.

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Builds a one-line program through clang's driver and returns every
/// argument list clang was run with.
fn clang_lines(case: &str, release: bool, measured: bool) -> Vec<String> {
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-link-names-{case}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bin")).expect("a temp directory");
    std::fs::write(stage.join("main.kso"), "print \"linked\"\n").expect("the program is written");
    let real = Command::new("sh").args(["-c", "command -v clang"]).output().expect("sh runs");
    let real = String::from_utf8_lossy(&real.stdout).trim().to_string();
    assert!(!real.is_empty(), "this spec needs clang on the path");
    let seen = stage.join("seen.txt");
    let wrapper = stage.join("bin/clang");
    std::fs::write(
        &wrapper,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{real}' \"$@\"\n", seen.display()),
    )
    .expect("the wrapper is written");
    let mode = Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod runs");
    assert!(mode.success());
    let path = format!("{}:{}", stage.join("bin").display(), std::env::var("PATH").unwrap());
    let mut build = Command::new(env!("CARGO_BIN_EXE_kanso"));
    build.arg("build").arg("main.kso").current_dir(&stage);
    build.env("PATH", path).env("KANSO_CLANG_DRIVER", "1").env_remove("KANSO_FIXED_TEMPS");
    if release {
        build.arg("--release");
    }
    if measured {
        build.env("KANSO_FIXED_TEMPS", "1");
    }
    let out = build.output().expect("kanso runs");
    assert!(out.status.success(), "the build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let ran = Command::new(stage.join("main")).output().expect("the binary runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "linked\n");
    std::fs::read_to_string(&seen).expect("clang was run").lines().map(str::to_string).collect()
}

/// The one line that links the program: it names the program's IR.
fn link_line(lines: &[String]) -> &str {
    let links: Vec<&String> = lines.iter().filter(|l| l.contains("main.ll")).collect();
    assert_eq!(links.len(), 1, "expected one clang run over main.ll, saw {lines:#?}");
    links[0]
}

fn names(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .filter(|word| word.contains("runtime") && word.ends_with(".o"))
        .map(|word| Path::new(word).file_name().and_then(|n| n.to_str()).unwrap_or(word))
        .collect()
}

#[test]
fn a_measured_release_link_names_its_runtime_plainly() {
    let lines = clang_lines("release-measured", true, true);
    assert_eq!(
        names(link_line(&lines)),
        ["kanso_linked_runtime_release_hot.o", "kanso_linked_runtime_release.o"]
    );
}

#[test]
fn a_measured_dev_link_names_its_runtime_plainly() {
    let lines = clang_lines("dev-measured", false, true);
    assert_eq!(names(link_line(&lines)), ["kanso_linked_runtime_dev.o"]);
}

/// A build nobody measures links the cached files, whose names carry the
/// digests that keep one build's object from another's.
#[test]
fn an_unmeasured_link_names_the_cached_objects() {
    let lines = clang_lines("release-plain", true, false);
    let named = names(link_line(&lines));
    assert_eq!(named.len(), 2, "{named:?}");
    assert!(named[0].starts_with("kanso_runtime_release_hot_"), "{named:?}");
    assert!(named[1].starts_with("kanso_runtime_release_"), "{named:?}");
}

/// Built into a temp directory of its own, so the runtime is compiled here
/// and the objects read are the ones this compiler made.
#[test]
fn a_cached_runtime_object_records_no_name_a_digest_decides() {
    let stage: PathBuf = std::env::temp_dir().join("kanso-link-names-objects");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("tmp")).expect("a temp directory");
    std::fs::write(stage.join("main.kso"), "print \"linked\"\n").expect("the program is written");
    for tier in [&["build", "main.kso"][..], &["build", "main.kso", "--release"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(tier)
            .current_dir(&stage)
            .env("TMPDIR", stage.join("tmp"))
            .env("KANSO_COUNTERS_BUILD", "1")
            .output()
            .expect("kanso runs");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    let mut read = 0;
    for entry in std::fs::read_dir(stage.join("tmp")).expect("the temp directory lists") {
        let path = entry.expect("an entry reads").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if !(name.starts_with("kanso_runtime_") && name.ends_with(".o")) {
            continue;
        }
        let bytes = std::fs::read(&path).expect("the object reads");
        let needle = b"kanso_runtime_";
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle),
            "{name} holds a name beginning `kanso_runtime_`, which is where the digest is"
        );
        read += 1;
    }
    assert_eq!(read, 3, "expected the dev runtime, the release runtime and its hot unit");
}
