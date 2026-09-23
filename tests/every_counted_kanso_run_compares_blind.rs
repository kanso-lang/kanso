//! Every gate that counts the kanso process preloads the address-blind
//! memcmp, on every `env -i` line it runs kanso under, and gets its path from
//! the script that proves it.
//!
//! libc's `__memcmp_avx2_movbe` takes a longer branch when either operand of a
//! short comparison lies within 32 bytes of a page end. On 2026-09-23 a runtime
//! change that added forty lines to `src/runtime.c`, which the compiler embeds,
//! moved 478 of the compile row's comparisons onto that branch with every call
//! count unchanged, and the row rose 2,688. Preloading
//! `scripts/gates/address_blind/compare.c` made the two trees read the same
//! number to the instruction. A gate that loses the preload goes back to
//! counting where the linker put its strings, and nothing else would say so:
//! its golden would simply be re-measured the next time it moved.
//!
//! The gates are derived from disk: every script that runs `./kanso` and
//! writes a callgrind profile. One is exempt, for the reason written beside it.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn gates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/gates")
}

fn gate_scripts() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(gates_dir()).expect("scripts/gates reads") {
        let path = entry.expect("a directory entry reads").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let body = std::fs::read_to_string(&path).expect("a gate script reads");
        out.insert(name, body);
    }
    out
}

/// `codegen_instructions.sh` counts the clang driver, `clang -cc1` and ld and
/// excludes kanso's own process. Clang's work genuinely changes when the
/// runtime it compiles changes, and preloading into it is a separate question
/// with its own measurement; kanso's half of a build is `emit_instructions.sh`,
/// which is governed here.
const EXEMPT: [&str; 1] = ["codegen_instructions.sh"];

const HELPER_CALL: &str = "blind=$(sh scripts/gates/address_blind.sh)";
const PRELOAD: &str = "LD_PRELOAD=\"$blind\"";

/// Scripts that run the kanso binary and count it with callgrind.
fn governed() -> BTreeMap<String, String> {
    gate_scripts()
        .into_iter()
        .filter(|(name, body)| {
            !EXEMPT.contains(&name.as_str())
                && body.contains("--tool=callgrind")
                && body.contains("./kanso ")
        })
        .collect()
}

/// The `env -i` lines of a script that launch something, with the text of any
/// continuation lines joined on, so a wrapped command reads as one.
fn env_lines(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = body.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with('#') || !line.contains("env -i ") {
            continue;
        }
        let mut joined = line.to_string();
        while joined.trim_end().ends_with('\\') {
            match lines.next() {
                Some(next) => joined.push_str(next),
                None => break,
            }
        }
        out.push(joined);
    }
    out
}

#[test]
fn the_derivation_finds_the_six_kanso_gates() {
    let names: Vec<String> = governed().into_keys().collect();
    for want in [
        "compile_instructions.sh",
        "emit_instructions.sh",
        "entry_instructions.sh",
        "interp_instructions.sh",
        "library_instructions.sh",
        "startup_instructions.sh",
    ] {
        assert!(
            names.iter().any(|n| n == want),
            "{want} counts the kanso process and the derivation did not find it; \
             it found {names:?}. A derivation that stops seeing a gate makes \
             every other test here pass over it."
        );
    }
}

#[test]
fn every_kanso_gate_resolves_the_blind_compare_before_it_runs_anything() {
    for (name, body) in governed() {
        let helper = body.find(HELPER_CALL).unwrap_or_else(|| {
            panic!(
                "{name} counts the kanso process and never runs \
                 scripts/gates/address_blind.sh, so `$blind` is empty and \
                 libc's memcmp is what it counts."
            )
        });
        let first_env = body
            .lines()
            .scan(0usize, |at, l| {
                let here = *at;
                *at += l.len() + 1;
                Some((here, l))
            })
            .find(|(_, l)| !l.trim_start().starts_with('#') && l.contains("env -i "))
            .map(|(at, _)| at)
            .unwrap_or_else(|| panic!("{name} has no env -i line"));
        assert!(
            helper < first_env,
            "{name} runs kanso before it resolves `$blind`, so the first run \
             preloads an empty path."
        );
    }
}

#[test]
fn every_kanso_run_in_a_gate_preloads_the_blind_compare() {
    for (name, body) in governed() {
        let lines = env_lines(&body);
        assert!(!lines.is_empty(), "{name} has no env -i line to check");
        for line in lines {
            assert!(
                line.contains(PRELOAD),
                "{name} runs `{}` without {PRELOAD}. Its count then depends on \
                 which page offsets the linker and the allocator gave the \
                 strings it compares.",
                line.trim()
            );
        }
    }
}

#[test]
fn the_exempt_gate_exists() {
    let all = gate_scripts();
    for name in EXEMPT {
        assert!(
            all.contains_key(name),
            "{name} is exempted and no such gate exists; an exemption for a \
             deleted gate hides the next gate that takes its name."
        );
    }
}
