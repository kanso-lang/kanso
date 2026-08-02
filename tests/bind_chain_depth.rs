//! A program that sequences effects one after another holds one frame, not one
//! per effect. The interpreter is the oracle, so a shape native runs in flat
//! memory must not cost the interpreter memory proportional to its length.
//!
//! Peak memory is the claim rather than finishing, because a chain that nests
//! still finishes — it just costs about 1.6 KB a link, which is a gigabyte and
//! a half for a million of them before the host stack gives out. Only the
//! footprint separates a loop from a recursion.

use std::process::Command;

/// Peak resident bytes for a chain `links` long. The chain is right-nested
/// through `.`, which is the shape a loop over work takes: each step hands its
/// result to a closure that decides the next step.
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

    // BSD time reports a footprint in bytes under -l, GNU time a maximum
    // resident size in kilobytes under -v. Ask for both and read whichever
    // answered, so the same claim holds on either host.
    let reported = ["-l", "-v"].into_iter().find_map(|flag| {
        let done = Command::new("/usr/bin/time")
            .arg(flag)
            .arg(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg("chain.kso")
            .arg("--interp")
            .current_dir(&dir)
            .output()
            .expect("kanso runs");
        let said = String::from_utf8_lossy(&done.stderr).into_owned();
        // A rejected flag reports no peak, which is the signal to try the
        // other dialect rather than to judge the run.
        let peak = peak_bytes(&said)?;
        assert!(
            String::from_utf8_lossy(&done.stdout).contains("done"),
            "the chain did not finish:\n{said}"
        );
        Some(peak)
    });
    let _ = std::fs::remove_dir_all(&dir);

    reported.expect("neither time(1) dialect reported a peak")
}

/// Bytes from `peak memory footprint`, or kilobytes from `Maximum resident set
/// size`, whichever this host's time(1) wrote.
fn peak_bytes(said: &str) -> Option<u64> {
    said.lines().find_map(|line| {
        if line.contains("peak memory footprint") {
            return line.split_whitespace().next()?.parse().ok();
        }
        let (_, size) = line.split_once("Maximum resident set size (kbytes): ")?;
        Some(size.trim().parse::<u64>().ok()? * 1024)
    })
}

/// Ten times the work over the same live set, so a chain that runs as a loop
/// stays flat while one that nests grows with its length.
#[test]
fn a_longer_chain_of_effects_does_not_need_more_memory() {
    let short = peak_for(1_000);
    let long = peak_for(10_000);

    assert!(
        long < short * 2,
        "a chain ten times longer took {long} bytes against {short} — it is nesting, not looping"
    );
}
