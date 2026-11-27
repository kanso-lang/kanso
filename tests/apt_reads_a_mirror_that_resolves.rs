//! A job's package install reads a mirror whose name resolves.
//!
//! On 2026-11-11 the first mirror in a hosted runner's list stopped
//! resolving on the runners for hours, and every job that installs a
//! package died in the install, in three repositories, before any test ran.
//! `scripts/apt_ready.sh` runs before each install and rewrites the list
//! when the first name does not resolve, to every other mirror that does.
//!
//! The script is run here against a list and a resolver of this test's
//! own, so what is pinned is the list it leaves behind.

use std::path::{Path, PathBuf};
use std::process::Command;

const LIST: &str = "http://azure.archive.ubuntu.com/ubuntu/\tpriority:1\n\
                    https://archive.ubuntu.com/ubuntu/\tpriority:2\n\
                    https://security.ubuntu.com/ubuntu/\tpriority:3\n";

/// Runs the script with a resolver that answers only for `known` and an
/// empty directory of sources, and returns the list it left and what it
/// printed.
fn after(case: &str, known: &[&str]) -> (String, String) {
    let (list, said, _) = after_sources(case, known, &[]);
    (list, said)
}

/// As `after`, with these files in the script's sources directory, and
/// also returns the names of the files it left there, sorted.
fn after_sources(
    case: &str,
    known: &[&str],
    sources: &[(&str, &str)],
) -> (String, String, Vec<String>) {
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-apt-ready-{case}"));
    let _ = std::fs::remove_dir_all(&stage);
    let sources_dir = stage.join("sources.list.d");
    std::fs::create_dir_all(&sources_dir).expect("a temp directory");
    for (name, body) in sources {
        std::fs::write(sources_dir.join(name), body).expect("a source is written");
    }
    let list = stage.join("apt-mirrors.txt");
    std::fs::write(&list, LIST).expect("the list is written");
    let lookup = stage.join("lookup.sh");
    let arms: String = known.iter().map(|host| format!("  {host}) exit 0 ;;\n")).collect();
    std::fs::write(&lookup, format!("#!/bin/sh\ncase \"$1\" in\n{arms}  *) exit 2 ;;\nesac\n"))
        .expect("the resolver is written");
    let out = Command::new("sh")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/apt_ready.sh"))
        .env("KANSO_APT_MIRRORS", &list)
        .env("KANSO_APT_RETRIES", stage.join("fetch-again"))
        .env("KANSO_APT_SOURCES", &sources_dir)
        .env("KANSO_APT_LOOKUP", format!("sh {}", lookup.display()))
        .env("KANSO_APT_SUDO", "")
        .env("KANSO_APT_PAUSE", "0")
        .output()
        .expect("the script runs");
    assert!(out.status.success(), "the script failed: {out:?}");
    let mut left: Vec<String> = std::fs::read_dir(&sources_dir)
        .expect("the sources read")
        .map(|e| e.expect("an entry").file_name().to_string_lossy().to_string())
        .collect();
    left.sort();
    (
        std::fs::read_to_string(&list).expect("the list reads"),
        String::from_utf8_lossy(&out.stdout).to_string(),
        left,
    )
}

/// What the runners answered on 2026-11-27: Microsoft's archive refused
/// with 403, and `apt-get update` failed the install of Ubuntu's clang 19.
/// The runner's source for it goes, and every other source stays.
#[test]
fn microsofts_archive_is_not_read() {
    let microsoft = "deb [arch=amd64,armhf,arm64] https://packages.microsoft.com/ubuntu/24.04/prod noble main\n";
    let ubuntu = "Types: deb\nURIs: http://archive.ubuntu.com/ubuntu/\nSuites: noble\n";
    let sources = [("microsoft-prod.list", microsoft), ("ubuntu.sources", ubuntu)];
    let known = ["azure.archive.ubuntu.com"];
    let (list, said, left) = after_sources("microsoft", &known, &sources);
    assert_eq!(left, vec!["ubuntu.sources".to_string()]);
    assert!(said.ends_with("microsoft-prod.list\n"), "{said}");
    assert!(said.starts_with("apt: not reading "), "{said}");
    assert_eq!(list, LIST);
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
    assert_eq!(
        list,
        "http://archive.ubuntu.com/ubuntu/\tpriority:1\n\
         http://mirrors.kernel.org/ubuntu/\tpriority:2\n"
    );
    assert!(said.contains("azure.archive.ubuntu.com does not resolve"), "{said}");
    let again = std::env::temp_dir().join("kanso-apt-ready-away/fetch-again");
    assert_eq!(
        std::fs::read_to_string(again).expect("apt is told to fetch again"),
        "Acquire::Retries \"4\";\n"
    );
}

/// What the runners answered on 2026-11-11: neither of Ubuntu's own names,
/// and the others. One mirror alone then failed to connect for one package,
/// so every mirror that resolves is listed.
#[test]
fn every_other_mirror_that_resolves_is_listed_in_order() {
    let known = ["mirrors.kernel.org", "mirrors.mit.edu", "mirrors.ocf.berkeley.edu"];
    let (list, _) = after("further", &known);
    assert_eq!(
        list,
        "http://mirrors.kernel.org/ubuntu/\tpriority:1\n\
         http://mirrors.mit.edu/ubuntu/\tpriority:2\n\
         http://mirrors.ocf.berkeley.edu/ubuntu/\tpriority:3\n"
    );
}

#[test]
fn with_no_mirror_resolving_the_list_is_left_for_the_install_to_report() {
    let (list, said) = after("none", &[]);
    assert_eq!(list, LIST);
    assert!(said.contains("no other mirror resolves"), "{said}");
    assert!(!std::env::temp_dir().join("kanso-apt-ready-none/fetch-again").exists());
}
