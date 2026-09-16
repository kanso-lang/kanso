//! Nothing the compile rows count may hash with a per-process random seed.
//!
//! `std`'s `HashMap` and `HashSet` default to `RandomState`, which draws a new
//! key from the OS on every process. Two runs of the same binary over the same
//! input then probe their tables in different orders and do a different amount
//! of work getting the same answer. `src/hash.rs` says that iteration order
//! changing is harmless because nothing observable depends on it, and that is
//! true of what the compiler WRITES. It is not true of what the compiler
//! COSTS, and three goldens count exactly that:
//! `bench/compile_instructions_golden.txt`, `bench/entry_instructions_golden.txt`
//! and `bench/library_instructions_golden.txt` hold one exact value each.
//!
//! kanso#1449 declared `proven` as `std::collections::HashSet<Span>` in two
//! places in `src/infer.rs` and one in `src/check.rs`, and the three rows
//! stopped reproducing. Three CI rounds on identical source returned three
//! distinct values on every row; eight local runs of the library gate's own
//! command varied across a spread of 313 instructions on ONE machine, and five
//! more under `setarch -R` varied too, so it was not the chip and not ASLR. A
//! callgrind diff of two of those profiles named the whole delta in one frame,
//! `hashbrown::map::HashMap<K,V,S,A>::insert`, +119 with every other function
//! identical to the instruction. Spelling the three as `crate::hash::Set` put
//! six consecutive runs on one value.
//!
//! The hunt cost three CI rounds, two published corrections and an escalation
//! to Clay that had to be withdrawn, all for a container the author did not
//! notice was spelled differently from its neighbours. So the spelling is a
//! property of the tree now rather than a habit. Every exception is named
//! below with the reason it is off the counted path.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A file:line the compile rows do not reach, and why.
///
/// `kanso check` is the whole of what the three compile gates run. A container
/// only these paths build is never constructed while that command is counted.
const OFF_THE_COUNTED_PATH: &[(&str, &str)] = &[
    (
        "src/eval.rs",
        "the interpreter. No compile golden runs a program, and the interpreter's \
         own cost is not counted by any exact vein.",
    ),
    (
        "src/wasm_rt.rs",
        "the wasm runtime shim, compiled into the wasm blob rather than into a \
         path `kanso check` walks.",
    ),
    (
        "src/main.rs",
        "narrow_tailcc, which rewrites emitted LLVM IR. The compile rows stop \
         before codegen; only `kanso build` reaches it.",
    ),
];

#[test]
fn the_compile_path_hashes_with_a_fixed_seed() {
    let src = root().join("src");
    let mut loose: Vec<String> = Vec::new();

    let mut files: Vec<_> = std::fs::read_dir(&src)
        .expect("src/ is readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    assert!(files.len() > 10, "src/ holds the compiler's modules");

    for path in &files {
        let rel = path
            .strip_prefix(root())
            .expect("the file is under the manifest")
            .to_string_lossy()
            .into_owned();

        // src/hash.rs is where the fixed-seed aliases are DEFINED, so it names
        // std's types on purpose.
        if rel == "src/hash.rs" {
            continue;
        }
        let excused = OFF_THE_COUNTED_PATH.iter().any(|(f, _)| *f == rel);

        let text = std::fs::read_to_string(path).expect("the module is readable");
        for (n, line) in text.lines().enumerate() {
            let names_std = line.contains("std::collections::HashMap")
                || line.contains("std::collections::HashSet")
                || line.contains("use std::collections::{BTreeMap, HashMap}")
                || line.contains("use std::collections::HashMap")
                || line.contains("use std::collections::HashSet");
            if names_std && !excused {
                loose.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }

    assert!(
        loose.is_empty(),
        "these hash with std's per-process random seed, on a path the three \
         compile instruction goldens count:\n\n{}\n\nEach golden holds ONE exact \
         value, and a randomly-seeded table makes the number differ between two \
         runs of the same binary -- which reads as the gate's case (2), a \
         reproduction failure, and halts the vein. Spell them \
         `crate::hash::Map` / `crate::hash::Set`, which hash with a fixed seed. \
         If the container really is off the counted path, add it to \
         OFF_THE_COUNTED_PATH above with the reason.",
        loose.join("\n")
    );
}

/// Every excuse names a file that exists and still needs excusing.
///
/// An allowlist nobody prunes is how the first list in this repo went stale.
#[test]
fn no_excuse_outlives_its_file() {
    for (file, reason) in OFF_THE_COUNTED_PATH {
        let path = root().join(file);
        assert!(path.is_file(), "{file} is excused and does not exist");
        assert!(reason.len() > 40, "{file}'s excuse is too short to be a reason");
        let text = std::fs::read_to_string(&path).expect("the module is readable");
        assert!(
            text.contains("std::collections::HashMap")
                || text.contains("std::collections::HashSet"),
            "{file} no longer names a std-hashed container, so its excuse is dead \
             and comes out of OFF_THE_COUNTED_PATH"
        );
    }
}
