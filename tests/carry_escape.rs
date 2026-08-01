//! A walk that reads a directory tree and keeps a record per file.
//!
//! Without the fix the native engine reads a string header out of a carry
//! buffer a deeper chain has already retired, and dies inside the copy walk.
//! `main.rs` reports any SIGSEGV as stack exhaustion, so the failure reads as
//! runaway recursion and is nothing of the kind. The interpreter is the oracle
//! and answers 11 either way.
//!
//! The input is generated rather than committed, and three things about it are
//! load-bearing. The file sizes: ten of sixteen bytes and one of a quarter
//! megabyte, where uniform sizes never reproduce it. The layout: four sibling
//! directories and one nested, where a flat directory of the same count and
//! total does not. And the length of the path each record ends up holding —
//! the fault appears at a root name of eighteen characters and holds from
//! there, which is why the root below is a fixed twenty-four and why the walk
//! runs from the scratch directory rather than naming an absolute path whose
//! length belongs to whichever machine is running.
//!
//! That last one cost an attempt. A first version of this test named the tree
//! relatively as `tree`, four characters, and passed with the bug present.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Twenty-four characters, comfortably past the eighteen where the fault
/// starts, and the same on every machine because it is relative.
const ROOT: &str = "a_tree_of_sample_outputs";

const PROGRAM: &str = r#"import "std/io"
import "std/list"
import "std/text"

type source_file
  path
  body

fn walk_all path
  io/list_dir path . (names -> walk_names path names 1 [])

fn walk_names path names at acc
  walking_names path names at acc (at > length names)

fn walking_names _ _ _ acc true
  acc

fn walking_names path names at acc false
  kid = "{path}/{names[at]!}"
  io/is_dir kid . (d -> walk_kid path names at acc kid d)

fn walk_kid path names at acc kid true
  on = (deep -> walk_names path names (at + 1) (joined acc deep))
  walk_all kid . on

fn walk_kid path names at acc kid false
  on = (body -> read_one path names at acc kid body)
  io/read_file kid . on

fn read_one path names at acc kid body
  walk_names path names (at + 1) (push acc (source_file kid body))

fn joined a b
  list/to_list (text/concat a b)

pub play = walk_all "a_tree_of_sample_outputs" . (fs -> print (length fs))
"#;

const TREE: [(&str, usize); 11] = [
    ("ch05/pipes.out", 16),
    ("ch05/quiet.kso", 16),
    ("ch05/quiet.out", 16),
    ("ch05/quiet_plan.out", 16),
    ("ch05/render.kso", 16),
    ("ch05/render.out", 16),
    ("ch05/running.kso", 16),
    ("ch05/running.out", 16),
    ("ch08/using.ll", 259_888),
    ("ch09/squeeze/methods.kso", 16),
    ("ch10/classify.out", 16),
];

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join("kanso_carry_escape");
    let _ = std::fs::remove_dir_all(&dir);
    for (name, size) in TREE {
        let path = dir.join(ROOT).join(name);
        std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("mkdir");
        std::fs::write(&path, "x".repeat(size)).expect("write");
    }
    std::fs::write(dir.join("walk.kso"), PROGRAM).expect("write program");
    dir
}

fn answer(dir: &Path, extra: &[&str]) -> (String, String) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("walk.kso")
        .args(extra)
        .current_dir(dir)
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

#[test]
fn a_record_built_from_a_carried_value_outlives_the_buffer_it_points_into() {
    let dir = scratch();

    let (oracle, complaint) = answer(&dir, &["--interp"]);
    assert_eq!(complaint, "");
    assert_eq!(oracle, "11\n", "the oracle's answer changed");

    let (native, complaint) = answer(&dir, &[]);
    assert_eq!(complaint, "", "native refused the walk");
    assert_eq!(native, oracle, "native disagrees with the oracle");

    let _ = std::fs::remove_dir_all(&dir);
}
