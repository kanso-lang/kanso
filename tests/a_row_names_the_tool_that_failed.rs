//! The row says which tool failed, rather than which index it was reading.
//!
//! `perf_record` asks welfare for the score and takes the second field of its
//! first line. When welfare itself dies it prints no such line, and the row
//! builder used to report that as `missing index 2` born in `score_in` — a
//! message naming the reader instead of the tool it read, and pointing at a
//! file nothing was wrong with. On 2026-09-04 three CI heads failed exactly
//! that way while a golden row was being harvested, and the message sent a
//! reader to `perf_record.kso` when the fault was two processes away.
//!
//! THE STATUS IS NOT THE QUESTION, and a spec keyed to it would be worse than
//! none. welfare exits 1 on a fall and on a rise nobody ratcheted, printing a
//! score in both — those are the commits this row exists to record, and a
//! refusal that read the status would go silent on precisely them. What a
//! crash leaves behind is a first line with no second field. That is what is
//! asked, and the status rides along in the message.

use std::process::Command;

/// A tree perf_record can run in: the goldens copied so a test may edit them,
/// the compiler, the scripts and the library borrowed from the checkout, and a
/// git repository because the row carries the commit it describes.
///
/// `key` names the directory, because separate tests staging into one path
/// have torn each other down mid-run in this repository before.
fn staged(key: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-row-names-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bench")).expect("a staging directory");
    for entry in std::fs::read_dir(root.join("bench")).expect("bench is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_file() {
            let landing = stage.join("bench").join(path.file_name().expect("named"));
            std::fs::copy(&path, &landing).expect("the golden copies");
        }
    }
    for borrowed in ["target", "scripts", "lib"] {
        std::os::unix::fs::symlink(root.join(borrowed), stage.join(borrowed))
            .expect("the checkout lends its directory");
    }
    for argv in [
        vec!["init", "--quiet", "."],
        vec!["-c", "user.email=spec@kanso.invalid", "-c", "user.name=spec", "commit",
             "--quiet", "--allow-empty", "-m", "the stage"],
    ] {
        let done =
            Command::new("git").args(argv).current_dir(&stage).output().expect("git runs");
        assert!(done.status.success(), "the stage wants a repository");
    }
    stage
}

fn recorded(stage: &std::path::Path) -> (String, String) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "scripts/perf_record"])
        .current_dir(stage)
        .output()
        .expect("perf_record runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

/// The refusal names welfare and carries what welfare said, so a reader lands
/// on the tool that failed rather than on the line that read it.
///
/// The break is the real one: a counter the model weighs whose instruction row
/// is missing, which is the state a branch is in between minting a benchmark
/// and harvesting its row from CI.
#[test]
fn a_welfare_that_prints_no_score_is_named_rather_than_indexed() {
    let stage = staged("crash");
    let golden = stage.join("bench/instructions_golden.txt");
    let text = std::fs::read_to_string(&golden).expect("the golden reads");
    let cut: String =
        text.lines().filter(|l| !l.starts_with("readbench ")).collect::<Vec<_>>().join("\n");
    assert_ne!(cut, text, "the golden had no readbench row to remove");
    std::fs::write(&golden, cut + "\n").expect("the golden writes");

    let (_, said) = recorded(&stage);

    assert!(
        said.contains("welfare printed no score"),
        "the refusal should name welfare, and said:\n{said}"
    );
    assert!(
        said.contains("readbench"),
        "it should carry what welfare said, which names the counter:\n{said}"
    );
    assert!(
        !said.contains("missing index 2"),
        "`missing index 2` is the message this spec exists to retire:\n{said}"
    );
}

/// And the score is still read when there is one. A refusal that fired on a
/// healthy run would cost the history every row it has.
#[test]
fn a_welfare_that_prints_a_score_is_read_as_before() {
    let (row, said) = recorded(&staged("healthy"));
    assert!(said.is_empty(), "a healthy run says nothing on stderr:\n{said}");
    assert!(
        row.contains("\"welfare\":"),
        "the row carries the score it read:\n{row}"
    );
}
