//! The same program compiled by two processes at once.
//!
//! A run caches its binary under a key hashed from the IR, so two runs of one
//! program share that key — and shared the path the IR was written to. One
//! process truncated and rewrote that file while another's clang was reading
//! it, and a half-read module makes LLVM's assembly lexer walk off the end of
//! its buffer. What the reader saw was a segmentation fault inside clang and
//! an invitation to file a bug against LLVM.
//!
//! Two tests in `tests/hako.rs` run the same program, which is how a suite
//! that had never done this began doing it several times a run.

use std::process::{Command, Stdio};

fn a_program_in(root: &std::path::Path, mark: u32) {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).expect("a directory of its own");
    // The binary is cached under a hash of the IR, so a program this test has
    // built before is never compiled again. A literal no other run produces
    // keeps every sitting a cold one.
    std::fs::write(root.join("main.kso"), format!("print \"{{{mark} + 1}}\"\n"))
        .expect("the program writes");
}

fn run(root: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("run")
        .arg(root)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// The defect itself, which is deterministic even though the corruption it
/// causes is not: two processes compiling one program must not be handed the
/// same file to write.
///
/// WHY THIS WATCHES INSTEAD OF COUNTING WHAT IS LEFT. It used to read the
/// leftovers after both runs exited, which worked only because the IR file was
/// never cleaned up -- a leak of 42 KB per cache MISS that reached 112,000
/// files, and a guard resting on a bug fails the moment the bug is fixed. The
/// file now goes as soon as clang has read it, so the property is observed
/// while it holds: the IR is written before clang starts and removed after it
/// returns, and a cold run of this program is ~133 ms with ~100 ms of that
/// window. The poll below is a millisecond, so it sees the file with about a
/// hundredfold margin.
///
/// It cannot pass vacuously. Seeing nothing at all is a failure with its own
/// sentence, because "the race never happened" and "the race happened and was
/// safe" must not look alike.
///
/// AND IT IS WHY THE SIBLING BELOW IS NOT ENOUGH ON ITS OWN. Under the pid
/// stripped back out of the path, this caught the defect in 10 sittings of 10
/// and `many_builds_of_one_program_all_answer` in 9 -- it passed once with the
/// bug in place, because whether two processes actually overlap on the file is
/// the race and the race is not owed to anyone. One of these two is a
/// corruption that may or may not happen; this one is the decision that lets
/// it.
#[test]
fn two_builds_of_one_program_do_not_share_a_file() {
    let root = std::env::temp_dir().join("kanso-concurrent-paths");
    let mark = std::process::id();
    a_program_in(&root, mark);

    let temp = std::env::temp_dir();
    let before = ir_files(&temp);
    let mut first = run(&root).spawn().expect("kanso starts");
    let mut second = run(&root).spawn().expect("kanso starts");

    // Everything either process was seen holding at any instant, unioned. The
    // second run may find the binary already cached and write no IR at all,
    // which is why this asks for at least one rather than exactly two.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        seen.extend(ir_files(&temp).difference(&before).cloned());
        let done = |c: &mut std::process::Child| matches!(c.try_wait(), Ok(Some(_)));
        if done(&mut first) && done(&mut second) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    for mut r in [first, second] {
        r.wait().expect("kanso finishes");
    }

    let watched: Vec<String> = seen.into_iter().collect();
    assert!(
        !watched.is_empty(),
        "no IR file was seen at all, so this proved nothing: either the \
         binary was already cached for both runs -- the literal is built from \
         this process's own id to stop that -- or the poll is too slow for the \
         compile it is watching"
    );
    assert!(
        watched.iter().all(|f| owner(f).is_some()),
        "an IR file named for the program alone is one a second process \
         truncates while the first is reading it: {watched:?}"
    );
}

/// Every process gets the whole program, which is the point of the above.
#[test]
fn many_builds_of_one_program_all_answer() {
    let root = std::env::temp_dir().join("kanso-concurrent-answers");
    let mark = std::process::id() + 1;
    a_program_in(&root, mark);

    let racers: Vec<_> = (0..8).map(|_| run(&root).spawn().expect("kanso starts")).collect();
    let answers: Vec<String> = racers
        .into_iter()
        .map(|r| {
            let done = r.wait_with_output().expect("kanso finishes");
            match done.status.success() {
                true => String::from_utf8_lossy(&done.stdout).into_owned(),
                false => String::from_utf8_lossy(&done.stderr).into_owned(),
            }
        })
        .collect();

    assert!(
        answers.iter().all(|a| *a == format!("{}\n", mark + 1)),
        "a concurrent build did not produce the program: {answers:#?}"
    );
}

fn ir_files(dir: &std::path::Path) -> std::collections::HashSet<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return std::collections::HashSet::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("kanso_run_") && n.ends_with(".ll"))
        .collect()
}

/// The process a `kanso_run_<key>_<pid>.ll` belongs to, and nothing when the
/// name carries no process — which is the shape this pins.
fn owner(name: &str) -> Option<u32> {
    let stem = name.trim_start_matches("kanso_run_").trim_end_matches(".ll");
    stem.split_once('_').and_then(|(_, pid)| pid.parse().ok())
}
