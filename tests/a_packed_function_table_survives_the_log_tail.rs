//! The packed function table, and the recipe printed beside it, recover the
//! table byte for byte out of a job log.
//!
//! A log API returns the TAIL of a job and caps it — 5,000 lines, whatever
//! tail length is asked for. On 2026-09-22 the `cost goldens` job ran 20,020
//! lines and handed back two function tables of the twenty it printed; the
//! eighteen others were written down and unreadable, and the frame that
//! carries the standing row's three instructions is in one of them. Packing
//! each table gzip+base64 turns 1,726 rows into 190 lines, which is what puts
//! them all inside the tail.
//!
//! That only helps if a reader can get the rows back. So this spec enters
//! where the reader enters: it runs the real script, prefixes every line with
//! a timestamp the way a job log does, and then runs THE RECIPE THE SCRIPT
//! ITSELF PRINTS, lifted out of its header rather than copied here. A recipe
//! that has drifted from the format fails this, which is the whole reason it
//! is lifted and not retyped.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn tail_script() -> PathBuf {
    repo().join("scripts/gates/function_tables_tail.sh")
}

/// A table with the shape callgrind_annotate gives: the count FIRST, then one
/// space, then a name that may itself contain spaces. The space inside a name
/// is the hazard — splitting a row on every whitespace run rather than on the
/// first space produced a phantom ten-million-instruction mover on
/// 2026-09-22 — so a row of that shape is in here deliberately.
fn a_table() -> String {
    let mut out = String::new();
    out.push_str("946,933,024 PROGRAM TOTALS\n");
    out.push_str("20,848,747 ./string/../sysdeps/x86_64/multiarch/memcmp-avx2-movbe.S:__memcmp_avx2_movbe [/usr/lib/x86_64-linux-gnu/libc.so.6]\n");
    out.push_str("1,518 ./nptl/./nptl/pthread_getattr_np.c:pthread_getattr_np@@GLIBC_2.32 [/usr/lib/x86_64-linux-gnu/libc.so.6]\n");
    out.push_str("14 ???:0x0000000000144ab0 [/tmp/kanso-compile-ir/kanso]\n");
    out.push_str("3 ???:some frame with spaces in its name [/tmp/kanso-compile-ir/kanso]\n");
    // Enough rows that packing is doing real work rather than padding.
    for i in 0..2000 {
        out.push_str(&format!("{i} ???:0x{i:016x} [/tmp/kanso-compile-ir/kanso]\n"));
    }
    out
}

/// The script's output with a timestamp on every line, which is what a job log
/// hands a reader and what the recipe has to cope with.
fn as_a_job_log(out: &str) -> String {
    out.lines().map(|l| format!("2026-09-23T00:10:01.1234567Z {l}\n")).collect()
}

fn run_tail(stash: &Path, budget: Option<&str>) -> String {
    let mut cmd = Command::new("sh");
    cmd.arg(tail_script()).env("KANSO_FUNCTION_TABLES", stash).current_dir(repo());
    if let Some(b) = budget {
        cmd.env("KANSO_FUNCTION_TABLES_BUDGET", b);
    }
    let out = cmd.output().expect("the packing script runs");
    assert!(out.status.success(), "the packing script failed: {out:?}");
    String::from_utf8(out.stdout).expect("its output is utf-8")
}

/// The recipe out of the script's own header, as a runnable pipeline.
///
/// It is lifted rather than retyped so that a recipe which has gone stale
/// fails here. The header writes it as a comment over three continued lines,
/// against a log called `job.log` and a table called `interp`.
fn the_documented_recipe(log: &Path, out: &Path, name: &str) -> String {
    let body = std::fs::read_to_string(tail_script()).expect("the script reads");
    let after = body.split("TO READ ONE BACK").nth(1).unwrap_or_else(|| {
        panic!(
            "scripts/gates/function_tables_tail.sh no longer prints a recipe \
             for reading a packed table back. A packed table with no recipe is \
             a table nobody decodes."
        )
    });
    let mut recipe = String::new();
    // `.skip(1)`: the split lands mid-line, and what is left of the heading
    // line is prose rather than a comment.
    for line in after.lines().skip(1) {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix('#') else { break };
        let rest = rest.trim();
        if rest.is_empty() {
            if recipe.is_empty() {
                continue; // the blank line between the heading and the recipe
            }
            break; // the recipe has ended
        }
        recipe.push_str(rest);
        recipe.push('\n');
    }
    assert!(
        recipe.contains("base64 -d") && recipe.contains("gunzip"),
        "the recipe lifted out of the script does not unpack anything, so this \
         spec would be testing a pipeline of its own invention:\n{recipe}"
    );
    recipe
        .replace("job.log", &log.display().to_string())
        .replace("interp.txt", &out.display().to_string())
        .replace("interp", name)
}

/// A scratch directory of this spec's own, named for the caller so two tests
/// running at once do not pack each other's tables.
fn scratch(what: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kanso-packed-tables-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    dir
}

fn stash_with(what: &str, tables: &[(&str, String)]) -> PathBuf {
    let dir = scratch(what);
    for (name, body) in tables {
        std::fs::write(dir.join(format!("{name}.txt")), body).expect("the table writes");
    }
    dir
}

#[test]
fn the_recipe_the_script_prints_recovers_the_table_byte_for_byte() {
    let table = a_table();
    let stash = stash_with("recipe", &[("interp", table.clone())]);
    let log_dir = scratch("recipe-log");
    let log = log_dir.join("job.log");
    std::fs::write(&log, as_a_job_log(&run_tail(&stash, None))).expect("the log writes");

    let got = log_dir.join("recovered.txt");
    let recipe = the_documented_recipe(&log, &got, "interp");
    let out = Command::new("sh").arg("-c").arg(&recipe).output().expect("the recipe runs");
    assert!(out.status.success(), "the recipe failed:\n{recipe}\n{out:?}");
    // A pipeline's status is its LAST stage's, and `gunzip` here is happy with
    // trailing rubbish after a complete stream. So an earlier stage can fail
    // while the bytes still come out right: on 2026-09-23 `base64` said
    // `invalid input` and the recovered table matched anyway, because the only
    // bad byte was past the end of the gzip stream. Reading stderr is what
    // sees it.
    let noise = String::from_utf8_lossy(&out.stderr);
    assert!(
        noise.trim().is_empty(),
        "the recipe recovered the table and complained while doing it, which \
         means the packed format and the recipe have drifted apart in a way \
         the bytes happen to survive:\n{recipe}\nstderr: {noise}"
    );

    let recovered = std::fs::read_to_string(&got).expect("the recovered table reads");
    assert_eq!(
        recovered, table,
        "the recipe printed beside the packed table does not recover it. It is \
         the only way a reader gets the rows back, so a recipe that has drifted \
         from the format leaves the table unreadable in a different way."
    );
}

#[test]
fn the_markers_a_reader_selects_on_are_each_on_a_line_of_their_own() {
    // `fold` ends its last chunk without a newline, so a block written
    // `cat "$body"; echo "#end $name"` puts the end marker on the tail of the
    // final base64 line. A reader's range then runs to the end of the log
    // instead of to the end of the block. Every table but the last one in a
    // job is misread that way, and the last one survives by luck.
    //
    // Several table sizes, because the defect hides whenever the packed bytes
    // happen to land on a fold boundary — which is how it got past this spec
    // the first time.
    for rows in [1usize, 7, 199, 500, 2000] {
        let table: String = (0..rows)
            .map(|i| format!("{i} ???:0x{i:016x} [/tmp/kanso-compile-ir/kanso]\n"))
            .collect();
        let stash = stash_with(&format!("markers-{rows}"), &[("interp", table)]);
        let printed = run_tail(&stash, None);
        for marker in ["#table interp ", "#end interp"] {
            assert!(
                printed.lines().any(|l| l == marker.trim_end() || l.starts_with(marker)),
                "with {rows} rows, `{marker}` is not on a line of its own:\n{}",
                printed
                    .lines()
                    .filter(|l| l.contains("#end") || l.contains("#table"))
                    .map(|l| format!("  [{}]", &l[l.len().saturating_sub(40)..]))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

#[test]
fn the_packed_block_carries_the_digest_of_what_it_unpacks_to() {
    // A reader who got a short tail needs to tell a whole block from a cut
    // one, and the rows themselves will not say.
    let table = a_table();
    let stash = stash_with("digest", &[("interp", table.clone())]);
    let printed = run_tail(&stash, None);
    let sha = printed
        .lines()
        .find(|l| l.starts_with("#table interp "))
        .and_then(|l| l.split("sha256=").nth(1))
        .expect("the block carries a sha256")
        .to_string();

    let real =
        Command::new("sha256sum").arg(stash.join("interp.txt")).output().expect("sha256sum runs");
    let real = String::from_utf8(real.stdout).expect("utf-8");
    let real = real.split_whitespace().next().expect("a digest").to_string();
    assert_eq!(sha, real, "the digest on the #table line is not of the table it packs");
}

#[test]
fn every_stashed_table_is_named_in_the_index_at_the_end() {
    // The index is unpacked and last, so a reader whose tail cut off the
    // blocks still learns which tables the job carried and can say which ones
    // the cut took. Without it a missing table and a table that never ran read
    // the same.
    let stash = stash_with(
        "index",
        &[
            ("interp", a_table()),
            ("compile", a_table()),
            ("jsonbench", "7 ???:one row\n".to_string()),
        ],
    );
    let printed = run_tail(&stash, None);
    let index = printed
        .lines()
        .rfind(|l| l.starts_with("#tables"))
        .unwrap_or_else(|| panic!("the packed output carries no `#tables` index:\n{printed}"));
    for name in ["interp", "compile", "jsonbench"] {
        assert!(index.contains(name), "the index `{index}` does not name {name}");
    }
    assert_eq!(
        printed.lines().next_back(),
        Some(index),
        "the index is not the last line printed, so a reader who got only the \
         final handful of lines misses it"
    );
}

#[test]
fn a_stash_too_big_for_the_tail_says_so() {
    // The budget is the thing this whole change is about, so exceeding it is
    // reported rather than left to be discovered by a reader who asks for a
    // table and finds the tail starts halfway through it.
    let stash = stash_with("budget", &[("interp", a_table()), ("compile", a_table())]);
    let printed = run_tail(&stash, Some("10"));
    assert!(
        printed.contains("::warning::") && printed.contains("budget of 10"),
        "the packed tables ran past their budget and the job said nothing:\n{}",
        printed.lines().take(8).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn an_empty_stash_is_said_out_loud_rather_than_printed_as_nothing() {
    // Silence here reads exactly like a job whose tables packed fine.
    let stash = scratch("empty");
    let printed = run_tail(&stash, None);
    assert!(
        printed.contains("no function tables were stashed"),
        "an empty stash printed nothing at all, which a reader cannot tell \
         from a job that packed its tables: {printed:?}"
    );
}
