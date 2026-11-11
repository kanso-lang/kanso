//! A program pays for the part of the shipped library it reaches (kanso#1952).
//! `std/text` holds two dozen pub functions; a program that calls one of them
//! should not lex, parse, infer and check the rest. Watched red on the
//! compiler before the prune: `text/pad_start` was in the compiled program,
//! which held 48 declarations, and inference visited 627 expressions.

#[test]
fn an_import_reads_only_the_declarations_the_program_names() {
    let source = "import \"std/text\"\n\nprint (text/uppercase \"kanso\")\n";
    kanso::infer::work::reset();
    let program = kanso::compile_entry("main.kso", source).expect("the program compiles");
    let (_, visits) = kanso::infer::work::taken();
    let names: Vec<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
    assert!(names.iter().any(|n| n.ends_with("uppercase")), "{names:?}");
    assert!(!names.iter().any(|n| n.ends_with("pad_start")), "{names:?}");
    // 223 since a loop counter's floor proves std/text's reads: a read that
    // cannot miss carries no none, and inference settles a visit sooner.
    assert_eq!(visits, 223, "expression visits");
}

fn declared(source: &str) -> (Vec<String>, u64) {
    kanso::infer::work::reset();
    let program = kanso::compile_entry("main.kso", source).expect("the program compiles");
    let (_, visits) = kanso::infer::work::taken();
    (program.fns.iter().map(|d| d.name.clone()).collect(), visits)
}

/// A comment names nothing (kanso#2434). The reach read every word a file
/// spells, a comment's among them, so a program whose comment said
/// `pad_start` in passing compiled `text/pad_start` and everything it calls.
/// Watched red with the comment test taken out of the reach: the program held
/// `text/pad_start`, `text/padding` and `text/filled`.
#[test]
fn a_comment_keeps_no_declaration() {
    let above = "# pad_start would line these up\nimport \"std/text\"\n\n\
                 print (text/uppercase \"kanso\")\n";
    let (names, visits) = declared(above);
    assert!(!names.iter().any(|n| n.ends_with("pad_start")), "{names:?}");
    assert_eq!(visits, 223, "the same visits as the program with no comment");

    let beside = "import \"std/text\"\n\nprint (text/uppercase \"kanso\") # or pad_start\n";
    let (names, _) = declared(beside);
    assert!(!names.iter().any(|n| n.ends_with("pad_start")), "{names:?}");
}

/// A `#` inside a string starts no comment, so what the string goes on to
/// name is still reached: the call in the interpolation here is the only
/// place the program says `trim`.
#[test]
fn a_hash_in_a_string_hides_nothing_after_it() {
    let source = "import \"std/text\"\n\nprint \"#{text/trim \" kanso \"}\"\n";
    let (names, _) = declared(source);
    assert!(names.iter().any(|n| n.ends_with("trim")), "{names:?}");
}

/// A string's text names nothing either (kanso#2434). An err whose message
/// said `nothing to find` kept every `find` in the library: the issue's
/// 1.67M instructions were that one word, measured at 1,153,085 of the
/// 1,306,636 the arm holding it cost. Watched red with the string read as
/// code: the program held `text/pad_start`.
#[test]
fn a_string_keeps_no_declaration() {
    let source = "import \"std/text\"\n\n\
                  print (text/uppercase \"no pad_start here\")\n";
    let (names, visits) = declared(source);
    assert!(!names.iter().any(|n| n.ends_with("pad_start")), "{names:?}");
    assert_eq!(visits, 223, "the same visits as the program whose string says less");
}

/// What a string interpolates is code, at any depth: the only `trim` here is
/// inside a string that is itself inside an interpolation, and the only
/// `lowercase` follows an escaped brace.
#[test]
fn an_interpolation_is_still_read() {
    let nested = "import \"std/text\"\n\n\
                  print \"a {text/uppercase \"b {text/trim \" c \"}\"} d\"\n";
    let (names, _) = declared(nested);
    assert!(names.iter().any(|n| n.ends_with("trim")), "{names:?}");

    let escaped = "import \"std/text\"\n\nprint \"\\{ {text/lowercase \"K\"}\"\n";
    let (names, _) = declared(escaped);
    assert!(names.iter().any(|n| n.ends_with("lowercase")), "{names:?}");
}
