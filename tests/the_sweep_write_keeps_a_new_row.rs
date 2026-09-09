//! `all_counters.sh --write` rewrites a golden's rows from the measured file
//! while keeping the header. Until 2026-09-09 it replaced rows line for line
//! and stopped at the golden's last row, so a counter the runtime had just
//! gained — one more row than the golden held — was dropped on the floor. The
//! sweep printed `rewrote bench/cost_golden.txt`, the file did not change,
//! and CI read `46a47 > sh_bytes=...` on every one of the twelve a round
//! later (kanso#1354).
//!
//! The rewrite lives in scripts/gates/keep_header.sh so it can be run here
//! on a golden the size of a postcard rather than through the 204-second
//! sweep.
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rewrite(name: &str, golden: &str, got: &str) -> String {
    // one directory per test: the two run in one process, side by side
    let dir = std::env::temp_dir().join(format!("keep_header_{}_{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let golden_path = dir.join("golden.txt");
    let got_path = dir.join("got.txt");
    std::fs::write(&golden_path, golden).unwrap();
    std::fs::write(&got_path, got).unwrap();
    let out = Command::new("sh")
        .arg(manifest_dir().join("scripts/gates/keep_header.sh"))
        .arg(&golden_path)
        .arg(&got_path)
        .output()
        .expect("sh runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn a_row_the_measured_file_gained_lands_after_the_goldens_last() {
    let golden = "# a header line\n# measured-on host=x\n\nallocs=1\nsh_map=2\n";
    let got = "allocs=3\nsh_map=4\nsh_bytes=5\n";
    assert_eq!(
        rewrite("gained", golden, got),
        "# a header line\n# measured-on host=x\n\nallocs=3\nsh_map=4\nsh_bytes=5\n"
    );
}

#[test]
fn the_header_and_its_blank_line_survive_a_rewrite_that_adds_nothing() {
    let golden = "# kept\n\nallocs=1\nsh_map=2\n";
    let got = "allocs=9\nsh_map=8\n";
    assert_eq!(rewrite("kept", golden, got), "# kept\n\nallocs=9\nsh_map=8\n");
}

#[test]
fn the_sweep_rewrites_through_the_helper() {
    let sweep =
        std::fs::read_to_string(manifest_dir().join("scripts/gates/all_counters.sh")).unwrap();
    assert!(
        sweep.contains("sh scripts/gates/keep_header.sh"),
        "all_counters.sh --write must rewrite through keep_header.sh; an inline \
         awk is what dropped the new row"
    );
}
