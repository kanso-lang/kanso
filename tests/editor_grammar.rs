//! The editor grammar in `editors/kanso` names kanso's words in regular
//! expressions, which cannot read the compiler. This reads them back and
//! asks the compiler about each: the builtins are the names in scope without
//! an import, and each word the grammar colours as an operator or a keyword
//! is one the lexer reads as such (#2293).

use kanso::lexer::{lex, Tok};

fn grammar() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/editors/kanso/syntaxes/kanso.tmLanguage.json");
    std::fs::read_to_string(path).expect("the grammar reads")
}

/// The words of the alternation in the `match` written just before the
/// pattern's `"name": "<scope>"`.
fn words(grammar: &str, scope: &str) -> Vec<String> {
    let named =
        grammar.find(&format!("\"name\": \"{scope}\"")).expect("the scope is in the grammar");
    let start = grammar[..named].rfind("\"match\": ").expect("the pattern has a match");
    let rest = &grammar[start..named];
    let open = rest.find("\\\\b(").expect("the match is an alternation") + 4;
    let close = open + rest[open..].find(")\\\\b").expect("the alternation closes");
    rest[open..close].split('|').map(str::to_string).collect()
}

/// The tokens one line of source lexes to.
fn lexed(line: &str) -> Vec<Tok> {
    let source = format!("{line}\n");
    let lexed = lex(&source).unwrap_or_else(|d| panic!("{line:?} lexes: {d:?}"));
    lexed.lines.into_iter().flat_map(|l| l.tokens).map(|(t, _, _)| t).collect()
}

#[test]
fn the_grammar_colours_every_name_in_scope_without_an_import() {
    let mut want: Vec<String> =
        kanso::check::AMBIENT.iter().filter(|w| **w != "if").map(|w| w.to_string()).collect();
    want.push("err".to_string());
    want.sort();
    assert_eq!(words(&grammar(), "support.function.builtin.kanso"), want);
}

#[test]
fn the_grammar_colours_as_operators_the_words_the_lexer_reads_as_operators() {
    let spelled = words(&grammar(), "keyword.operator.logical.kanso");
    assert_eq!(spelled, ["and", "or", "not"]);
    for word in spelled {
        let tokens = lexed(&format!("x = a {word} b"));
        assert!(
            matches!(&tokens[3], Tok::Op(op) if *op == word),
            "{word:?} lexes as {:?}",
            tokens[3]
        );
    }
}

#[test]
fn the_grammar_colours_return_which_the_lexer_reads_as_a_keyword() {
    assert!(grammar().contains("\"match\": \"\\\\breturn\\\\b\""));
    assert!(lexed("return x").contains(&Tok::KwReturn));
}
