//! The digest of the per-process floor reads the format the floor actually
//! prints.
//!
//! `per_process_floor.sh` writes its listing with one `printf` format, and the
//! workflow step that digests it picks the lines apart with one `sed`. They are
//! in different files and nothing connects them, so the failure available here
//! is the quiet one: the printf gains a column, the sed matches nothing, and
//! the digest goes out as thirty-two zeroes on every sitting. A zero that looks
//! like agreement is the shape this project has been burned by before -- the
//! sweep that ran eleven gates and said nothing about the twelfth.
//!
//! So the two are held together by running them against each other: take the
//! script's own printf, render a line, feed it to the workflow's own sed, and
//! require a cost and a name to come back out.

use std::path::Path;
use std::process::Command;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The `printf` the floor listing is written with, as the script spells it.
fn listing_format() -> String {
    let s = read("scripts/gates/per_process_floor.sh");
    let line = s
        .lines()
        .find(|l| l.contains("printf '") && l.contains("\"$cost\"") && l.contains("\"$name\""))
        .unwrap_or_else(|| panic!("per_process_floor.sh prints no listing line"));
    let start = line.find('\'').expect("the format is single-quoted");
    let rest = &line[start + 1..];
    let end = rest.find('\'').expect("the format closes its quote");
    rest[..end].to_string()
}

/// The `sed` expression the workflow digests the listing with.
fn digest_sed() -> String {
    let s = read(".github/workflows/ci.yml");
    let line = s
        .lines()
        // `sed -n 's/^per_process_floor=//p'` reads the same file two steps
        // earlier, so the digest's own expression is picked by the character
        // class it splits the cost off with rather than by coming first.
        .find(|l| l.contains("sed -n") && l.contains("/tmp/floor.txt") && l.contains("[0-9]"))
        .unwrap_or_else(|| panic!("no step digests /tmp/floor.txt into cost and name"));
    let start = line.find('\'').expect("the sed script is single-quoted");
    let rest = &line[start + 1..];
    let end = rest.find('\'').expect("the sed script closes its quote");
    // YAML doubles the backslashes so the shell sees one; undo that here so
    // this test feeds sed exactly what the shell will.
    rest[..end].replace("\\\\", "\\")
}

#[test]
fn the_sed_matches_a_line_the_printf_produced() {
    let fmt = listing_format();
    let sed = digest_sed();
    let rendered = Command::new("sh")
        .arg("-c")
        .arg(format!("printf '{fmt}' 1234567 kanso::some::frame_name"))
        .output()
        .expect("sh runs");
    let line = String::from_utf8_lossy(&rendered.stdout).to_string();
    assert!(
        line.contains("1234567") && line.contains("frame_name"),
        "the printf did not render the sample: {line:?}"
    );

    let out = Command::new("sed")
        .arg("-n")
        .arg(&sed)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(line.as_bytes())?;
            c.wait_with_output()
        })
        .expect("sed runs");
    let got = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        got.contains('\t'),
        "the workflow's sed ({sed}) read nothing out of the line \
         per_process_floor.sh prints ({line:?}), so the digest would go out as \
         thirty-two zeroes and read as agreement"
    );
    let (cost, name) = got.trim_end().split_once('\t').expect("a cost and a name");
    assert_eq!(cost, "1234567", "the digest would weigh the wrong number");
    assert_eq!(
        name, "kanso::some::frame_name",
        "the digest buckets by name length, so a name read with its padding \
         still on lands in the wrong bucket"
    );
}

#[test]
fn the_header_and_the_totals_are_not_read_as_frames() {
    let sed = digest_sed();
    let noise = "=== the cost every process pays, whatever it compiles\n\
                 per_process_floor_frames=605\n\
                 per_process_floor=558620\n";
    let out = Command::new("sed")
        .arg("-n")
        .arg(&sed)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(noise.as_bytes())?;
            c.wait_with_output()
        })
        .expect("sed runs");
    assert!(
        out.stdout.is_empty(),
        "the digest read the listing's own header or totals as a frame: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}
