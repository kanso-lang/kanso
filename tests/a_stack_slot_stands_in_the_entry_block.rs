//! Where the emitted module claims its stack.
//!
//! An `alloca` standing in a block other than the function's first one is a
//! dynamic stack object to LLVM, whatever its size. The function then keeps a
//! frame pointer it would otherwise omit, restores `rsp` through it on every
//! return path, and claims the slot again on each pass rather than once. The
//! decoder carried sixty-eight such slots out of seventy-three and paid 8.78%
//! of its instructions for them — 2,098,864,058 to 1,914,624,003 on identical
//! source, with every allocation counter byte-identical.
//!
//! The emitted line count cannot see this and neither can any cost golden: the
//! same lines are written either way, and only their order changes. So the
//! property is asserted where it lives, on the `.ll` the build hands clang.
//!
//! The program below is the smallest that discriminates. A record built in
//! each arm of an `if` puts two argument arrays in blocks the entry does not
//! contain; before the fix this file reported two of its three slots adrift,
//! and a straight-line body reports none.

use std::path::PathBuf;
use std::process::Command;

const ENTRY: &str = "import \"./slot\"\nimport \"std/io\"\n\nio/write \"{pick 1}\\n\"\n";

const LIBRARY: &str = "pub type pair\n  a\n  b\n\npub fn pick n\n  \
                       chosen = if (n > 0) (pair n n) (pair 0 0)\n  chosen.a\n";

/// Every `alloca` in the module, paired with the block it stands in. A
/// function's first block carries no label of its own, so the walk names it
/// `entry` and renames on each label it passes.
fn slots_adrift(ir: &str) -> Vec<String> {
    let mut adrift = Vec::new();
    let mut function = String::new();
    let mut block = String::new();
    for line in ir.lines() {
        if line.starts_with("define") {
            function = line.to_string();
            block = "entry".to_string();
        } else if let Some(label) = line.strip_suffix(':') {
            if !label.starts_with(char::is_whitespace) && !label.is_empty() {
                block = label.to_string();
            }
        } else if line.contains(" = alloca ") && block != "entry" {
            adrift.push(format!("{}\n    in block {block} of {function}", line.trim()));
        }
    }
    adrift
}

#[test]
fn no_stack_slot_stands_outside_its_function_s_entry_block() {
    let dir = std::env::temp_dir().join("kanso-entry-block-slots");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("slot")).expect("the package directory");
    std::fs::write(dir.join("main.kso"), ENTRY).expect("the entry writes");
    std::fs::write(dir.join("slot/slot.kso"), LIBRARY).expect("the library writes");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", ".", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(
        built.status.success(),
        "the build refused the fixture: {}{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let name = PathBuf::from(dir.file_name().expect("the directory is named"));
    let ll = dir.join(name.with_extension("ll"));
    let ir = std::fs::read_to_string(&ll).expect("the build writes the ir beside the binary");
    let total = ir.matches(" = alloca ").count();
    assert!(total >= 3, "the fixture stopped claiming stack at all: {total} slots in {ll:?}");

    let adrift = slots_adrift(&ir);
    assert!(
        adrift.is_empty(),
        "{} of {total} stack slots stand outside their function's entry block, \
         so each of those functions keeps a frame pointer and restores rsp \
         through it on every return:\n  {}",
        adrift.len(),
        adrift.join("\n  ")
    );
}
