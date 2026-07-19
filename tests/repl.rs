use kanso::eval::ScriptedExecutor;
use kanso::repl::{Outcome, Session};

fn value(session: &mut Session, input: &str) -> String {
    let mut executor = ScriptedExecutor::default();
    match session.eval(input, &mut executor) {
        Ok(Outcome::Value(rendered)) => rendered,
        Ok(Outcome::Defined(echo)) => echo,
        Ok(Outcome::Executed(rendered)) => format!("executed {rendered}"),
        Err(message) => format!("error {message}"),
    }
}

#[test]
fn expressions_evaluate() {
    let mut session = Session::new();

    assert_eq!(value(&mut session, "1 + 2"), "3");
}

#[test]
fn constants_persist() {
    let mut session = Session::new();
    let _ = value(&mut session, "x = 10");

    assert_eq!(value(&mut session, "x * x"), "100");
}

#[test]
fn fns_persist_and_dispatch() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn double n\n  n * 2");

    assert_eq!(value(&mut session, "double 21"), "42");
}

#[test]
fn it_history_binds_prior_results() {
    let mut session = Session::new();
    let _ = value(&mut session, "1 + 2");
    let _ = value(&mut session, "10");

    assert_eq!(value(&mut session, "it0 + it1"), "13");
}

#[test]
fn redefining_a_name_replaces_all_arms() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn greet name\n  \"hi {name}\"");
    let _ = value(&mut session, "fn greet name\n  \"yo {name}\"");

    assert_eq!(value(&mut session, "greet \"ada\""), "\"yo ada\"");
}

#[test]
fn failed_input_rolls_back() {
    let mut session = Session::new();
    let _ = value(&mut session, "boom boom");

    assert_eq!(value(&mut session, "5"), "5");
}

#[test]
fn descriptions_execute_through_the_executor() {
    let mut session = Session::new();
    let mut executor = ScriptedExecutor::default();
    let outcome = session.eval("print \"hello\"", &mut executor);

    assert!(matches!(outcome, Ok(Outcome::Executed(_))));
    assert_eq!(executor.transcript, vec!["print \"hello\"".to_string()]);
}

#[test]
fn multi_arm_input_keeps_all_arms() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn sign 0\n  \"zero\"\n\nfn sign n\n  \"other\"");

    assert_eq!(value(&mut session, "sign 0"), "\"zero\"");
}

#[test]
fn redefinition_echoes_redefined() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn greet name\n  \"hi {name}\"");

    assert_eq!(value(&mut session, "fn greet name\n  \"yo {name}\""), "redefined greet");
}

#[test]
fn delete_removes_an_unused_declaration() {
    let mut session = Session::new();
    let _ = value(&mut session, "x = 10");

    assert_eq!(session.delete("x"), Ok("deleted x".to_string()));
    assert_eq!(value(&mut session, "x = 7"), "defined x");
}

#[test]
fn delete_refuses_while_something_depends_on_it() {
    let mut session = Session::new();
    let _ = value(&mut session, "x = 10");
    let _ = value(&mut session, "fn double_x _\n  x * 2");

    let refusal = session.delete("x").unwrap_err();

    assert!(refusal.starts_with("cannot delete `x`"), "got: {refusal}");
    assert_eq!(value(&mut session, "double_x 0"), "20");
}

#[test]
fn show_renders_one_declaration_without_running_it() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn shout word\n  \"{word}!\"");

    assert_eq!(session.show(Some("shout")), Ok("fn shout word\n  \"{word}!\"".to_string()));
}

#[test]
fn show_renders_the_session_as_a_canonical_file() {
    let mut session = Session::new();
    let _ = value(&mut session, "fn ada _\n  1");
    let _ = value(&mut session, "type user\n  name: string");

    let file = session.show(None).unwrap();

    let user_at = file.find("type user").unwrap();
    let ada_at = file.find("fn ada").unwrap();
    assert!(user_at < ada_at, "types come before functions:\n{file}");
}
