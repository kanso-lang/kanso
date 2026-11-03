//! `kanso lsp` against a recorded editor session: each `tests/golden/lsp/*.in`
//! is one JSON-RPC message per line, sent framed the way an editor frames it,
//! and the server's replies, one body per line, must equal the `.out` beside
//! it byte for byte.
//!
//! Two placeholders keep a transcript readable and independent of where the
//! checkout lives: `{{ROOT}}` is the fixture directory, in the input and the
//! output alike, and `{{TEXT:name.kso}}` is that fixture's text as a JSON
//! string. `KANSO_REGEN_LSP_GOLDEN=1` writes the `.out` files instead.

use kanso::lsp::json::Json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/lsp")
}

/// The transcript's messages, placeholders filled.
fn messages(case: &str) -> Vec<String> {
    let root = dir();
    let text =
        std::fs::read_to_string(root.join(format!("{case}.in"))).expect("the transcript reads");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut line = line.replace("{{ROOT}}", &root.to_string_lossy());
            while let Some(at) = line.find("{{TEXT:") {
                let end = at + line[at..].find("}}").expect("a closed placeholder");
                let name = &line[at + 7..end];
                let source = std::fs::read_to_string(root.join(name)).expect("the fixture reads");
                line.replace_range(at..end + 2, &Json::Str(source).text());
            }
            line
        })
        .collect()
}

/// Run the server on the framed messages and return each reply's body, after
/// checking that its header states its length in bytes.
fn replies(case: &str) -> (String, Option<i32>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso lsp starts");
    let mut framed = Vec::new();
    for m in messages(case) {
        write!(framed, "Content-Length: {}\r\n\r\n{m}", m.len()).unwrap();
    }
    child.stdin.take().unwrap().write_all(&framed).unwrap();
    let done = child.wait_with_output().expect("kanso lsp ends");
    assert_eq!(String::from_utf8_lossy(&done.stderr), "", "the server writes nothing to stderr");
    let mut rest: &[u8] = &done.stdout;
    let mut out = String::new();
    while !rest.is_empty() {
        let split = rest.windows(4).position(|w| w == b"\r\n\r\n").expect("a header ends");
        let header = std::str::from_utf8(&rest[..split]).unwrap();
        let length: usize = header
            .strip_prefix("Content-Length: ")
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("a header names the length: {header:?}"));
        let body = &rest[split + 4..split + 4 + length];
        let body = std::str::from_utf8(body).expect("a reply is UTF-8");
        assert!(kanso::lsp::json::parse(body).is_some(), "a reply is JSON: {body}");
        out.push_str(body);
        out.push('\n');
        rest = &rest[split + 4 + length..];
    }
    (out.replace(&*dir().to_string_lossy(), "{{ROOT}}"), done.status.code())
}

fn transcript(case: &str) {
    let (got, code) = replies(case);
    let golden = dir().join(format!("{case}.out"));
    if std::env::var_os("KANSO_REGEN_LSP_GOLDEN").is_some() {
        std::fs::write(&golden, &got).unwrap();
    }
    let want = std::fs::read_to_string(&golden).expect("the golden reads");
    assert_eq!(got, want, "{case}: the replies differ from {}", golden.display());
    assert_eq!(code, Some(0), "a server told to shut down before exit exits 0");
}

/// Clay's example: `foo x` hands `x` to `bar`, which has an arm for a string
/// and an arm for an int, so hovering `x` says `int | string`. `twice y`
/// hands `y` to `foo`, and the answer follows it there.
#[test]
fn a_parameter_reads_as_every_arm_its_uses_reach() {
    transcript("overloads");
}

/// The words `kanso check` prints arrive as diagnostics, on the token they
/// point at; an edit that fixes them clears them, and so does closing.
#[test]
fn the_check_s_diagnostics_reach_the_editor() {
    transcript("diagnostics");
}

#[test]
fn a_name_leads_to_where_it_was_declared() {
    transcript("definition");
}

/// A stream that ends without `shutdown` is a crashed client, and the
/// protocol has the server exit 1.
#[test]
fn an_exit_without_shutdown_is_a_failure() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("kanso lsp starts");
    let exit = r#"{"jsonrpc":"2.0","method":"exit"}"#;
    write!(child.stdin.take().unwrap(), "Content-Length: {}\r\n\r\n{exit}", exit.len()).unwrap();
    let done = child.wait_with_output().unwrap();
    assert_eq!(done.stdout, b"");
    assert_eq!(done.status.code(), Some(1));
}
