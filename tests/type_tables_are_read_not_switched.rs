//! A module answers a type's name and fields out of constant tables.
//!
//! Every module defines `k_type_name`, `k_type_shown`, `k_type_field_count`
//! and `k_type_field_name` for the runtime, which prints records and reads
//! fields by name through them. They were switches over the type id, one arm
//! per type and a nested switch per type's fields. clang's fast selector at
//! -O0 does not lower a switch, and those functions sat in the text every
//! pass over the module body reads. As arrays behind a bounds check, written
//! beside the interned strings, they took the dev codegen row from 367,410,698
//! to 360,741,989, the release row from 1,742,456,776 to 1,723,410,294, and
//! start-up from 798,960 to 750,547 on this container.
//!
//! What a user can see is what a program prints when it shows records and
//! reads their fields by name, on both tiers, and the module `kanso build`
//! leaves beside the binary.
//!
//! Watched red against the switch tables: the module switched on the id.

use std::process::Command;

fn build(dir: &std::path::Path, release: bool) -> (String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg("main.kso").current_dir(dir);
    if release {
        cmd.arg("--release");
    }
    let built = cmd.output().expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(dir.join("main")).output().expect("the binary runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    (module, String::from_utf8_lossy(&ran.stdout).into_owned())
}

#[test]
fn records_print_and_read_by_name_without_a_switch_on_the_id() {
    let dir = std::env::temp_dir().join(format!("kanso_type_tables_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(
        dir.join("shapes.kso"),
        "type point\n  x\n  y\n  z\n\ntype pair\n  a\n  b\n\nfn norm p\n  { x y } = p\n  \
         x * x + y * y\n\npub fn shown n\n  q = pair n 2\n  \
         \"{point n 4 0} {q} {norm (point n 4 9)} {q.b}\"\n",
    )
    .expect("the library writes");
    std::fs::write(dir.join("main.kso"), "import \"./shapes\"\n\nprint (shapes/shown 3)\n")
        .expect("the program writes");
    for release in [false, true] {
        let (module, out) = build(&dir, release);
        assert_eq!(out, "shapes/point 3 4 0 shapes/pair 3 2 25 2\n", "release={release}");
        assert!(
            !module.contains("switch i64 %id"),
            "release={release}: the module switches on a type id"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
