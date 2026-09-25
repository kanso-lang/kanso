//! The interpreter keeps an int in a machine word while it fits one and in a
//! `BigInt` once it does not. Nothing a program can see may tell the two
//! apart, and above all a number that comes back into range must come back
//! into the word: equality, ordering and map keys compare the two forms as
//! different values otherwise, so `(max + 1) - 1` would not equal `max`.
//!
//! Every program here crosses the word on the interpreter, which is the only
//! engine that goes past it (the native build refuses; see numeric_parity).
use std::process::Command;

fn interpreted(program: &str) -> String {
    interpreted_beside(program, None)
}

/// Runs `program` as the entry, with `library` beside it as `w.kso` when given.
fn interpreted_beside(program: &str, library: Option<&str>) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    (program, library).hash(&mut hasher);
    let dir = std::env::temp_dir().join(format!("kanso-word-{:016x}", hasher.finish()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp work dir");
    std::fs::write(dir.join("main.kso"), program).expect("the entry writes");
    if let Some(library) = library {
        std::fs::write(dir.join("w.kso"), library).expect("the library writes");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", ".", "--interp"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn each_operator_carries_past_the_word() {
    let program = "\
import \"std/list\"

max = 9223372036854775807
least = 0 - max - 1
print \"{max + 1} {least - 1} {3037000500 * 3037000500} {least / (0 - 1)}\"
print \"{list/sum [max max 2]} {(max + 1) % 10} {max * 4 / 2}\"
";
    assert_eq!(
        interpreted(program),
        "9223372036854775808 -9223372036854775809 9223372037000250000 9223372036854775808\n\
         18446744073709551616 8 18446744073709551614\n"
    );
}

#[test]
fn a_number_back_in_range_is_the_same_number() {
    let program = "\
max = 9223372036854775807
back = max + 1 - 1
halved = max * 2 / 2
keys = put {} back \"in\"
wide = 9223372036854775807.0
print \"{back == max} {halved == max} {back < max} {back > max - 1}\"
print \"{keys[max]} {length (put keys max \"again\")} {back == wide}\"
";
    assert_eq!(interpreted(program), "true true false true\nin 1 false\n");
}

/// A literal and a pattern are read out of the source's `BigInt` into a word
/// by a direct digit read, and so is every arithmetic result. The edges of
/// that read are the two ends of the word: the largest positive literal, the
/// first one past it, and a result that lands exactly on the most negative
/// word from outside it.
#[test]
fn the_ends_of_the_word_read_back_as_words() {
    let library = "\
pub fn name 9223372036854775807
  \"max\"

pub fn name 9223372036854775808
  \"past\"

pub fn name _
  \"other\"
";
    let program = "\
import \"./w\"

max = 9223372036854775807
least = 0 - max - 1
under = least - 1
print \"{w/name max} {w/name (max + 1)} {w/name (max - 1)}\"
print \"{w/name 9223372036854775808} {9223372036854775808 - 1 == max}\"
print \"{under + 1 == least} {under + 1 < least + 1}\"
print \"{length (put (put {} least 1) (under + 1) 2)}\"
";
    assert_eq!(
        interpreted_beside(program, Some(library)),
        "max past other\npast true\ntrue true\n1\n"
    );
}
