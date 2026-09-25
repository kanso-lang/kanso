//! No temp path this compiler builds interpolates a bare process id.
//!
//! A path's LENGTH is a term in what the process costs — the bytes are copied,
//! walked and handed to `open` — and a pid runs from one digit to seven. Two
//! runs of one binary on one box therefore wrote paths of different lengths and
//! counted different instructions for the same work. On 2026-09-17 that was all
//! that the codegen row's second reading still disagreed by once the warm-up was
//! fixed: 120 out of 1,071,604,729 on the dev tier, 582 out of 7,307,728,731 on
//! release, one binary, one job.
//!
//! `pid_tag()` pads it. The in-file spec beside that function pins the width;
//! this one pins that every site goes through it, so a path added later cannot
//! quietly put the variance back.

use std::path::Path;

const SOURCE: &str = "src/main.rs";

#[test]
fn no_format_string_interpolates_a_bare_process_id() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE);
    let text = std::fs::read_to_string(&p).expect("src/main.rs reads");

    let mut bare = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if !line.contains("std::process::id()") {
            continue;
        }
        // The one legitimate use is inside `pid_tag`, which is what pads it.
        if line.contains("pid_tag_of(std::process::id())") {
            continue;
        }
        bare.push(format!("{}:{}: {}", SOURCE, n + 1, line.trim()));
    }
    assert!(
        bare.is_empty(),
        "a temp path interpolates the pid without padding it, so its length \
         moves run to run:\n{}",
        bare.join("\n")
    );
}

/// And the padding is actually reached: every temp path names `pid_tag`.
#[test]
fn every_temp_path_that_wants_uniqueness_asks_pid_tag() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE);
    let text = std::fs::read_to_string(&p).expect("src/main.rs reads");
    let uses = text.matches("pid_tag()").count();
    assert!(
        uses >= 5,
        "only {uses} temp paths ask pid_tag(); there were five when this spec \
         was written, and a path that stopped asking is the variance coming back"
    );
}

/// No name this compiler builds pads a number through `format!`.
///
/// `{:016x}` and `{:07}` write a value's own digits and then one `write_char`
/// per missing digit, so the cost of naming a file moved with the number in
/// it. Most of those numbers hash a tool's modification time and change from
/// one runner image to the next, and on 2026-09-25 the start-up row read 52
/// instructions apart on two runners for one binary. `hex16` and `pid_tag_of`
/// write every digit the same way whatever the value.
#[test]
fn no_name_pads_a_number_through_format() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE);
    let text = std::fs::read_to_string(&p).expect("src/main.rs reads");

    let padded: Vec<String> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| line.contains(":016x}") || line.contains(":07}"))
        // the unit spec compares hex16 with the format it replaced
        .filter(|(_, line)| !line.contains("super::hex16"))
        .map(|(n, line)| format!("{}:{}: {}", SOURCE, n + 1, line.trim()))
        .collect();
    assert!(
        padded.is_empty(),
        "a name pads a number through format!, so what naming it costs moves \
         with the number:\n{}",
        padded.join("\n")
    );
}
