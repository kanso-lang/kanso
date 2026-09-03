//! The ratchet's box holds everything any ci.yml job installs.
//!
//! Every other job installs what its own gates need. The ratchet runs all of
//! their gates, and so needs the union — which nothing checked. On 2026-09-03
//! the baseline pass reported `ALREADY RED cost goldens ... gate: sh
//! scripts/gates/instructions.sh` on a branch that had touched src/runtime.c:
//! that gate runs callgrind, valgrind is installed only inside the cost
//! goldens job, and neither ratchet job had it. The gate was red before any
//! mutation was applied, and the ratchet reads a red gate as proof. Two rows
//! had proved nothing from the day they were written.
//!
//! The check reads INSTALL LINES on both sides, never file text. The first
//! draft of it asked whether scripts/ratchet/toolchain.sh contained the string
//! `valgrind` anywhere, and the paragraph above — which names valgrind four
//! times — satisfied it with the install removed. A comment is not a pin;
//! kanso#1137 found four checks resting on prose and this would have been a
//! fifth on the day it was written.
//!
//! A tool a gate needs and no ci.yml job installs is out of this file's reach:
//! that is the base image's business.

use std::collections::BTreeSet;

const CI: &str = include_str!("../.github/workflows/ci.yml");
const NIGHTLY: &str = include_str!("../.github/workflows/ratchet.yml");
const TOOLCHAIN: &str = include_str!("../scripts/ratchet/toolchain.sh");

/// What one line installs: the packages of an `apt-get install`, the targets
/// of a `rustup target add`. A comment installs nothing, whatever it mentions.
fn installed_by(line: &str) -> Vec<String> {
    let bare = line.trim_start();
    if bare.starts_with('#') {
        return Vec::new();
    }
    if let Some((_, tail)) = bare.split_once("rustup target add") {
        return tail.split_whitespace().map(str::to_string).collect();
    }
    let Some((_, tail)) = bare.split_once("apt-get install") else { return Vec::new() };
    let mut named = Vec::new();
    for word in tail.split_whitespace() {
        if word.starts_with('-') {
            continue;
        }
        if word.starts_with('>') || word.starts_with('&') || word.starts_with('|') {
            break;
        }
        named.push(word.to_string());
    }
    named
}

fn installs(text: &str) -> BTreeSet<String> {
    text.lines().flat_map(installed_by).collect()
}

#[test]
fn every_tool_a_ci_job_installs_is_on_the_ratchet_box() {
    let carried = installs(TOOLCHAIN);
    let missing: Vec<String> = installs(CI).into_iter().filter(|t| !carried.contains(t)).collect();
    assert!(
        missing.is_empty(),
        "ci.yml installs {missing:?} for a job whose gates the ratchet runs, and \
         scripts/ratchet/toolchain.sh does not. A row whose gate needs one of these \
         is red on the ratchet's box before any mutation is applied, and a red gate \
         reads as proof."
    );
}

#[test]
fn a_comment_naming_a_tool_does_not_install_it() {
    // The failure the first draft of this file walked into. The script's own
    // header explains why valgrind is there, so a check reading file text
    // passes with the install gone.
    let commented_out = TOOLCHAIN.replace("apt-get install -y -qq valgrind", "# apt-get valgrind");
    assert!(
        commented_out.contains("valgrind"),
        "the script still mentions valgrind — this is the case the check must not be fooled by"
    );
    assert!(
        !installs(&commented_out).contains("valgrind"),
        "a commented-out install must not count as installing anything"
    );
}

#[test]
fn both_workflows_that_run_the_ratchet_run_the_toolchain_script() {
    for (name, text) in [("ci.yml", CI), ("ratchet.yml", NIGHTLY)] {
        let runs = text.lines().any(|l| {
            !l.trim_start().starts_with('#') && l.contains("scripts/ratchet/toolchain.sh")
        });
        assert!(
            runs,
            "{name} runs ratchet rows, so it has to set the box up with \
             scripts/ratchet/toolchain.sh first"
        );
    }
}
