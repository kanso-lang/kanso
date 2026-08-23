//! A carry that gets repaired, and the arena invariant it depends on.
//!
//! When a rewind would reclaim what a surviving value points at, the runtime
//! copies those bytes below the mark and raises the mark over them. Raising it
//! moved the mark's pointer and left its remaining count behind, so the mark
//! described a position that did not exist and the arena went on to hand out
//! memory past the end of a block. glibc caught the damage on linux, in an
//! unrelated allocation; macOS never noticed.
//!
//! Nothing else in the corpus reaches that path. Every .kso under tests/ was
//! run against an assertion of the invariant and none of them breaks it, and
//! no mem-vein golden has carry_dedup above zero. This is the smallest program
//! that does: a loop that reads a list of files and gathers the bodies, where
//! the list survives the rewind and the bodies do not.

use std::process::Command;

const A_READ_LOOP: &str = r#"import "std/os"
import "std/text"

fn gathered paths at acc
  read_next paths at acc (at > length paths)

fn read_next _ _ acc true
  print "gathered {length acc}"

fn read_next paths at acc false
  on = (body -> gathered paths (at + 1) (text/concat acc [body]))
  os/read_file paths[at]! . on

wanted = ["README.md" "STATUS.md" "CLAUDE.md" "Cargo.toml"]

gathered wanted 1 []
"#;

/// Watched red on the runtime before the mark carried its own count:
/// "a beat mark and the arena disagree about the room that is left".
#[test]
fn a_repaired_carry_leaves_the_arena_consistent() {
    let dir = std::env::temp_dir().join("kanso-carry-repair");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let program = dir.join("gather.kso");
    std::fs::write(&program, A_READ_LOOP).expect("the program writes");

    // The paths it reads are repo-relative, so it runs from the repo.
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg(&program)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "gathered 4\n",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
