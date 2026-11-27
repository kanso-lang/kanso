//! `std/p256` against OpenSSL, on a corpus OpenSSL judged.
//!
//! scripts/p256_cases makes keys with `openssl ecparam`, signs random
//! messages with `openssl dgst -sign`, and makes most cases wrong in one way:
//! a message or a signature with a bit flipped, another key, a key off the
//! curve or with a coordinate at or past p, r or s of zero or of n or past
//! it, the other s (n - s, which ECDSA accepts), a DER integer with a
//! needless zero, a byte after the sequence, or a random r and s.
//! `openssl pkeyutl -verify` judges each; the script refuses a coordinate at
//! or past p itself, which SEC 1 asks and OpenSSL forgives by reducing it.
//! tests/p256/cases.txt holds 240 of those cases and tests/p256/verdicts.txt
//! OpenSSL's verdicts.
//!
//! The judge (tests/p256/judge) imports `std/p256` as any program does, so
//! the library's own text answers, never a copy of it. The corpus runs
//! natively, and its first twelve cases run on the interpreter too, where a
//! verification takes over a second.
//!
//! WATCHED RED by comparing the point's y with r in place of its x: the
//! native run prints 0 for every case, and the first disagreement is the
//! corpus's first valid signature.

use std::process::Command;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn judged(flags: &[&str], cases: &std::path::Path) -> String {
    let mut args = vec!["run", "tests/p256/judge"];
    args.extend_from_slice(flags);
    let cases = cases.to_str().expect("the corpus's path is text");
    args.extend_from_slice(&["--", cases]);
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(&args)
        .current_dir(root())
        .output()
        .expect("kanso runs");
    assert!(out.status.success(), "the judge failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("verdicts are text")
}

fn first_disagreement(want: &str, got: &str, cases: &str) -> Option<String> {
    let lines: Vec<&str> = cases.lines().collect();
    want.lines().zip(got.lines()).enumerate().find(|(_, (w, g))| w != g).map(|(i, (w, g))| {
        format!("case {} wants {w}, got {g}: {}", i + 1, lines.get(i).copied().unwrap_or(""))
    })
}

#[test]
fn std_p256_agrees_with_openssl_natively() {
    let cases_path = root().join("tests/p256/cases.txt");
    let cases = std::fs::read_to_string(&cases_path).expect("the corpus reads");
    let want =
        std::fs::read_to_string(root().join("tests/p256/verdicts.txt")).expect("verdicts read");
    let got = judged(&[], &cases_path);
    assert_eq!(got.lines().count(), 240, "one verdict a case");
    assert_eq!(
        want.lines().filter(|l| *l == "1").count(),
        119,
        "the corpus holds 119 good signatures"
    );
    if let Some(d) = first_disagreement(&want, &got, &cases) {
        panic!("std/p256 and OpenSSL disagree: {d}");
    }
}

#[test]
fn std_p256_agrees_with_openssl_on_the_interpreter() {
    let cases =
        std::fs::read_to_string(root().join("tests/p256/cases.txt")).expect("the corpus reads");
    let want =
        std::fs::read_to_string(root().join("tests/p256/verdicts.txt")).expect("verdicts read");
    let first: String = cases.lines().take(12).map(|l| format!("{l}\n")).collect();
    let wanted: String = want.lines().take(12).map(|l| format!("{l}\n")).collect();
    let dir = std::env::temp_dir().join(format!("p256-interp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let path = dir.join("cases.txt");
    std::fs::write(&path, &first).expect("the cases write");
    let got = judged(&["--interp"], &path);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        wanted.lines().any(|l| l == "1") && wanted.lines().any(|l| l == "0"),
        "both verdicts are asked"
    );
    if let Some(d) = first_disagreement(&wanted, &got, &first) {
        panic!("std/p256 on the interpreter and OpenSSL disagree: {d}");
    }
}
