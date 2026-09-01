//! A survivor that still points into a tenure block the beat pop freed.
//!
//! The evacuation promotes a node that has lived a lap into a tenure block,
//! and `k_survives_x` answers yes for a pointer into one. That answer is what
//! lets the copy prune: a survivor whose immediate interior survives is shared
//! rather than copied, and the walk stops there. For the arena the prune is
//! sound, because arena allocation is monotonic and a survivor can only point
//! at storage older than itself. Tenure storage is younger than the survivors
//! that come to hold it, so an arena record can carry a tenured string with no
//! arena pointer anywhere on the path to say so. `k_beat_pop` then freed the
//! block with that record still reachable.
//!
//! Watched red on 21d5c933: SIGSEGV in `k_copy_size` reading `s->data` for a
//! KStr in a munmap'd page, which the parent reports as "the program ran out of
//! stack" because native cannot see its own recursion. The oracle runs it.
//!
//! The shape it takes three things to reach, and nothing else in the corpus
//! has all three: an inner beat that builds a batch, an outer beat that
//! accumulates the batches so the batch nodes live a lap and are promoted, and
//! a SECOND pass over the accumulated list so a later evacuation walks the
//! promoted nodes after their block has gone. Drop any one and the program
//! finishes with the right answer on the broken runtime.

use std::process::Command;

const TWO_PASSES_OVER_A_CARRIED_LIST: &str = r#"import "std/io"
import "std/list"
import "std/text"

type item
  key
  n

pad = "0123456789012345678901234567890123456789012345678901234567890123"

fn made i
  item "{pad}{pad}-{i}" i

fn inner acc 0
  io/write "" . (_ -> acc)

fn inner acc j
  grown = text/concat acc [(made j)]
  io/write "" . (_ -> inner grown (j - 1))

fn onward found 0
  io/write "" . (_ -> found)

fn onward found i
  inner [] 30
    . (batch -> text/concat found batch)
    . (grown -> onward grown (i - 1))

fn key_of m
  m.key

fn n_of m
  m.n

fn shown ms
  total = list/sum (list/to_list (list/map ms n_of))
  all = text/join (list/to_list (list/map ms key_of)) "-"
  io/write "{length ms} {length all} {total}\n"

pub play = onward [] 280 . (ms -> onward ms 280) . (more -> shown more)
"#;

const ENTRY: &str = r#"import "./lib"

lib/play
"#;

#[test]
fn a_survivor_never_points_into_a_freed_tenure_block() {
    let dir = std::env::temp_dir().join("kanso-tenure-survivor");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("lib.kso"), TWO_PASSES_OVER_A_CARRIED_LIST)
        .expect("the library writes");
    std::fs::write(dir.join("main.kso"), ENTRY).expect("the entry writes");

    let native = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&dir)
        .output()
        .expect("kanso binary runs");

    // 280 laps of 30, twice: 16,800 records, 2,212,559 bytes of joined keys,
    // and 260,400 for the two triangular sums. Pinned, not banded.
    assert_eq!(
        String::from_utf8_lossy(&native.stdout),
        "16800 2212559 260400\n",
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );

    // The oracle is the reference. A divergence here is the same defect
    // reported from the other side.
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&dir)
        .arg("--interp")
        .output()
        .expect("kanso binary runs");
    assert_eq!(
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&native.stdout),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
}
