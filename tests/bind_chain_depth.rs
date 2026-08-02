//! A program that sequences effects one after another holds one frame, not one
//! per effect. The interpreter is the oracle, so a shape native runs in flat
//! memory must not cost the interpreter memory proportional to its length.
//!
//! Peak memory is the claim rather than a counter, because the failure this
//! guards is a Rust frame per link — invisible to every counter the runtime
//! keeps, and fatal only once the host stack runs out.

use std::process::Command;

/// Runs a chain `links` long and answers its peak resident bytes. The chain is
/// right-nested through `.`, which is the shape a loop over work takes: each
/// step hands its result to a closure that decides the next step.
fn peak_for(links: u64) -> u64 {
    let dir = std::env::temp_dir().join(format!("kanso-chain-{links}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let program = dir.join("chain.kso");
    std::fs::write(
        &program,
        format!(
            "import \"std/io\"\n\n\
             pub play = step 1\n\n\
             fn step n\n  going n (n > {links})\n\n\
             fn going _ true\n  io/write \"done\\n\"\n\n\
             fn going n false\n  io/write \"\" . (_ -> step (n + 1))\n"
        ),
    )
    .expect("the program writes");

    let done = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("chain.kso")
        .arg("--interp")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let out = String::from_utf8_lossy(&done.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(out.contains("done"), "the chain finished:\n{out}\n{said}");
    said.lines()
        .find_map(|l| {
            let (n, _) = l.trim().split_once(' ')?;
            l.contains("peak memory footprint").then(|| n.parse().ok())?
        })
        .unwrap_or_else(|| panic!("no peak memory in:\n{said}"))
}

/// Ten times the work on the same live set, so a chain that runs as a loop
/// stays flat and one that runs as a recursion grows about tenfold.
#[test]
fn a_longer_chain_of_effects_does_not_need_more_memory() {
    let short = peak_for(1_000);
    let long = peak_for(10_000);

    assert!(
        long < short * 2,
        "a chain ten times longer took {long} bytes against {short} — it is nesting, not looping"
    );
}
