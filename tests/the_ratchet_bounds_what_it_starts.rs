//! A command the ratchet starts may allocate what `scripts/bounded.sh` allows.
//!
//! A row's mutation makes the compiler wrong on purpose, and a wrong compiler
//! can build a program that allocates without end. Tests run under the limit
//! because cargo starts them through that script. A gate that is a shell
//! script or the compiler itself has no cargo in front of it, so the ratchet
//! sets the limit on everything it runs in a worktree.
//!
//! The fixture stages one row whose gate is green only under the limit, and
//! runs the real `prove` over it. The ratchet is started with the limit
//! lifted, because this test is itself running under it.
#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(
        done.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn discarded(tree: &PathBuf) {
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(tree)
        .current_dir(root())
        .output();
    let _ = std::fs::remove_dir_all(tree);
}

const PROBE: &str = "\
bound_probe = row
  \"a gate started with no limit on what it may allocate\"
  false
  \"sh probe_gate.sh\"
  \"bound-probe (a staged row)\"
  (breaking \"probe_breaks\")
  no_setup

";

/// A worktree of HEAD carrying this checkout's ratchet and limit script, with
/// one more row at the end of the table. Its gate prints the limit it runs
/// under and passes when that is 4 GiB and the mutation's file is absent.
fn staged() -> PathBuf {
    let tree = std::env::temp_dir().join("kanso-ratchet-bound");
    discarded(&tree);
    for stale in ["kanso-ratchet-base", "kanso-ratchet-1"] {
        discarded(&std::env::temp_dir().join(stale));
    }
    git(
        root(),
        &["worktree", "add", "--detach", "--force", tree.to_str().expect("a path"), "HEAD"],
    );

    let module = "scripts/ratchet/ratchet";
    for entry in std::fs::read_dir(root().join(module)).expect("the ratchet module reads") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().expect("named");
        std::fs::copy(&path, tree.join(module).join(name)).expect("a ratchet file copies");
    }
    std::fs::copy(root().join("scripts/bounded.sh"), tree.join("scripts/bounded.sh"))
        .expect("the limit script copies");

    let rows = tree.join(module).join("rows_d.kso");
    let table = std::fs::read_to_string(&rows).expect("the last rows file reads");
    assert_eq!(table.matches("[itself]").count(), 1, "the table's last row moved");
    let closing = table.rfind("\nrows_d = ").expect("the table closes in rows_d") + 1;
    let joined = format!("{}{PROBE}{}", &table[..closing], &table[closing..])
        .replace("[itself]", "[itself bound_probe]");
    std::fs::write(&rows, joined).expect("the staged table writes");

    let gate = "echo \"limit $(ulimit -d)\"\n\
                test \"$(ulimit -d)\" = 4194304 && test ! -e probe_broken\n";
    std::fs::write(tree.join("probe_gate.sh"), gate).expect("the gate writes");
    std::fs::write(tree.join("scripts/ratchet/mutations/probe_breaks.sh"), "touch probe_broken\n")
        .expect("the mutation writes");

    git(&tree, &["add", "scripts", "probe_gate.sh"]);
    git(
        &tree,
        &[
            "-c",
            "user.email=spec@kanso.invalid",
            "-c",
            "user.name=spec",
            "commit",
            "--quiet",
            "-m",
            "a staged row",
        ],
    );
    tree
}

#[test]
fn a_gate_the_ratchet_starts_runs_under_the_limit() {
    let tree = staged();
    let done = Command::new("sh")
        .args(["-c", "ulimit -S -d unlimited && exec \"$@\"", "sh"])
        .arg(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "scripts/ratchet", "--", "prove", "bound-probe"])
        .current_dir(&tree)
        .output()
        .expect("the ratchet runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    discarded(&tree);
    assert!(
        said.contains("every row turned its gate red"),
        "the staged gate is green only under the limit:\n{said}"
    );
    assert!(done.status.success(), "the staged row proves:\n{said}");
}
