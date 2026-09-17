//! What every kanso process pays whatever it is compiling, read off its own
//! profiles.
//!
//! Three compile rows -- module, entry, library -- read thirteen instructions
//! apart from their goldens across CI sittings of one commit, ALL THREE by
//! exactly thirteen. A 36.9-million-instruction compile and a 132.0-million-
//! instruction compile cannot both lose precisely thirteen instructions of
//! their shared work if that work is the compiling, so the thirteen is a cost
//! the process pays to exist. `scripts/gates/per_process_floor.sh` prints that
//! cost, derived from the profiles rather than from a list somebody wrote
//! down, and two jobs whose floors differ name the frame between them.
//!
//! The trap this spec exists for is a floor computed from the frames a
//! thresholded listing prints. The frames that matter here are the small ones
//! -- thirteen instructions in a hundred and thirty-two million -- and an
//! instrument that drops them agrees with itself run after run while saying
//! nothing at all.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One file, three functions, a total. `alpha` is the workload and moves;
/// `beta` is the per-process term and does not; `gamma` is in some profiles
/// and not others, which is what a frame reached by one route only looks like.
fn profile(alpha: u64, beta: u64, gamma: Option<u64>) -> String {
    let mut s = format!(
        "version: 1\ncreator: callgrind-3.22.0\ncmd: ./probe\npart: 1\n\n\
         positions: line\nevents: Ir\n\n\
         fl=probe.c\nfn=alpha\n1 {alpha}\n\nfn=beta\n2 {beta}\n\n"
    );
    if let Some(g) = gamma {
        s.push_str(&format!("fn=gamma\n3 {g}\n\n"));
    }
    s.push_str(&format!("totals: {}\n", alpha + beta + gamma.unwrap_or(0)));
    s
}

fn stage(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a staging directory");
    dir
}

/// Runs the instrument over a job's worth of profiles and hands back what it
/// said.
fn floor_of(dir: &Path, job: &str, betas: u64) -> String {
    // Three workloads an order of magnitude apart, the way the three compile
    // gates are, and one per-process term shared by all of them.
    let shapes = [(100_000_000_u64, Some(70_u64)), (360_000_000, Some(70)), (1_320_000_000, None)];
    let mut paths = Vec::new();
    for (i, (alpha, gamma)) in shapes.iter().enumerate() {
        let p = dir.join(format!("cg.{job}.{i}"));
        std::fs::write(&p, profile(*alpha, betas, *gamma)).expect("a profile");
        paths.push(p);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("sh")
        .arg(root.join("scripts/gates/per_process_floor.sh"))
        .args(&paths)
        .output()
        .expect("per_process_floor runs");
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "the instrument exited {:?}: {said}", out.status.code());
    said
}

/// A frame whose cost held across every workload is the floor. One that moved
/// with the workload is not, and neither is one that only some profiles carry.
#[test]
fn only_the_frame_that_held_across_every_workload_is_floor() {
    let dir = stage("kanso-floor-shared");
    let said = floor_of(&dir, "one", 41_000);

    assert!(said.contains("probe.c:beta"), "beta held in all three and must be named:\n{said}");
    assert!(
        !said.contains("probe.c:alpha"),
        "alpha moved with the workload and is not part of the floor:\n{said}"
    );
    assert!(
        !said.contains("probe.c:gamma"),
        "gamma is missing from one profile, so nothing is known about it:\n{said}"
    );
    assert!(
        said.contains("per_process_floor_frames=1"),
        "exactly one frame held, and the count says so:\n{said}"
    );
    assert!(
        said.contains("per_process_floor=41000"),
        "the floor is beta's cost and nothing else:\n{said}"
    );
}

/// The reason the instrument exists: two jobs that differ in nothing but a
/// thirteen-instruction per-process term report floors thirteen apart, and the
/// listing names the frame. This is the read that a thresholded instrument
/// cannot make -- thirteen against a floor of forty-one thousand inside a
/// profile of over a billion.
#[test]
fn two_jobs_thirteen_apart_name_the_frame_between_them() {
    let dir = stage("kanso-floor-thirteen");
    let low = floor_of(&dir, "low", 41_000);
    let high = floor_of(&dir, "high", 41_013);

    let read = |said: &str| -> u64 {
        said.lines()
            .find_map(|l| l.strip_prefix("per_process_floor="))
            .unwrap_or_else(|| panic!("the instrument prints a floor, and said:\n{said}"))
            .trim()
            .parse()
            .expect("the floor is a number")
    };
    let (a, b) = (read(&low), read(&high));
    assert_eq!(b - a, 13, "the floors must be thirteen apart, and read {a} and {b}");
    assert!(low.contains("41000  probe.c:beta"), "the low job names beta at its cost:\n{low}");
    assert!(high.contains("41013  probe.c:beta"), "the high job names beta at its cost:\n{high}");
}
