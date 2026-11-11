//! A job's package install reads a mirror whose name resolves.
//!
//! On 2026-11-11 the first mirror in a hosted runner's list stopped
//! resolving on the runners for hours, and every job that installs a
//! package died in the install, in three repositories, before any test ran.
//! `scripts/apt_ready.sh` runs before each install and rewrites the list
//! when the first name does not resolve.
//!
//! The script is run here against a list and a resolver of this test's
//! own, so what is pinned is the list it leaves behind.

use std::path::{Path, PathBuf};
use std::process::Command;

const LIST: &str = "http://azure.archive.ubuntu.com/ubuntu/\tpriority:1\n\
                    https://archive.ubuntu.com/ubuntu/\tpriority:2\n\
                    https://security.ubuntu.com/ubuntu/\tpriority:3\n";

/// Runs the script with a resolver that answers only for `known`, and
/// returns the list it left and what it printed.
fn after(case: &str, known: &[&str]) -> (String, String) {
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-apt-ready-{case}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a temp directory");
    let list = stage.join("apt-mirrors.txt");
    std::fs::write(&list, LIST).expect("the list is written");
    let lookup = stage.join("lookup.sh");
    let arms: String = known.iter().map(|host| format!("  {host}) exit 0 ;;\n")).collect();
    std::fs::write(&lookup, format!("#!/bin/sh\ncase \"$1\" in\n{arms}  *) exit 2 ;;\nesac\n"))
        .expect("the resolver is written");
    let out = Command::new("sh")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/apt_ready.sh"))
        .env("KANSO_APT_MIRRORS", &list)
        .env("KANSO_APT_LOOKUP", format!("sh {}", lookup.display()))
        .env("KANSO_APT_SUDO", "")
        .env("KANSO_APT_PAUSE", "0")
        .output()
        .expect("the script runs");
    assert!(out.status.success(), "the script failed: {out:?}");
    (
        std::fs::read_to_string(&list).expect("the list reads"),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

#[test]
fn a_first_mirror_that_resolves_is_left_alone() {
    let (list, said) = after("fine", &["azure.archive.ubuntu.com", "archive.ubuntu.com"]);
    assert_eq!(list, LIST);
    assert_eq!(said, "");
}

#[test]
fn a_first_mirror_that_does_not_resolve_gives_way_to_one_that_does() {
    let (list, said) = after("away", &["archive.ubuntu.com", "mirrors.kernel.org"]);
    assert_eq!(list, "http://archive.ubuntu.com/ubuntu/\tpriority:1\n");
    assert!(said.contains("azure.archive.ubuntu.com does not resolve"), "{said}");
}

#[test]
fn the_second_fallback_is_taken_when_the_first_is_away_too() {
    let (list, _) = after("further", &["mirrors.kernel.org"]);
    assert_eq!(list, "http://mirrors.kernel.org/ubuntu/\tpriority:1\n");
}

#[test]
fn with_no_mirror_resolving_the_list_is_left_for_the_install_to_report() {
    let (list, said) = after("none", &[]);
    assert_eq!(list, LIST);
    assert!(said.contains("no other mirror resolves"), "{said}");
}
