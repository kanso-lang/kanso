//! Every Linux job runs on a named runner image, never on `ubuntu-latest`.
//!
//! The image brings the glibc, clang and linker the cost goldens were measured
//! under, and their measured-on lines name them. `ubuntu-latest` moved from
//! ubuntu-24.04 to ubuntu-26.04 by 2026-11-18. That brought glibc 2.43 and
//! clang 19.1.7, every glibc- and clang-keyed gate refused, and the cost
//! goldens job went red on a pull request that changed one page of prose. A
//! counter reads the code under test and nothing else, so the image is fixed
//! the way rustc is (`the_toolchain_is_pinned_to_the_rows.rs`), and a new one
//! arrives in a pull request that re-sits the rows.

use std::fs;
use std::path::Path;

const IMAGE: &str = "ubuntu-24.04";

#[test]
fn every_linux_job_names_the_image_the_rows_were_measured_on() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .expect("the workflows directory")
        .map(|e| e.expect("a workflow entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .collect();
    files.sort();
    let mut jobs = 0;
    let mut floating = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).expect("a readable workflow");
        for (n, line) in text.lines().enumerate() {
            // A job names its runner on `runs-on:`, or in a matrix row's `os:`.
            let key = line.trim().trim_start_matches("- ");
            let Some(runner) = key.strip_prefix("runs-on:").or_else(|| key.strip_prefix("os:"))
            else {
                continue;
            };
            let runner = runner.trim();
            if !runner.starts_with("ubuntu") {
                continue;
            }
            jobs += 1;
            if runner != IMAGE {
                floating.push(format!("{}:{}: {runner}", path.display(), n + 1));
            }
        }
    }
    assert!(jobs > 0, "no workflow names a Linux runner, so this spec reads nothing");
    assert!(
        floating.is_empty(),
        "these jobs run on an image other than {IMAGE}, so the rows they measure \
         move with whatever that label resolves to:\n{}",
        floating.join("\n")
    );
}
