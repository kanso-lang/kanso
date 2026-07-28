use kanso::advisory::door_advisories;

#[test]
fn a_pub_fn_returning_a_foreign_type_with_no_accepting_op_is_advised() {
    let dir = std::path::Path::new("tests/golden/advisory/leaky");
    let program = kanso::compile_module(dir, false).expect("leaky compiles");

    let advisories = door_advisories(&program);

    assert_eq!(
        advisories,
        vec!["advisory[door]: `parse` returns `json/parse_failure` and the \
             surface offers nothing that accepts it — re-export what callers \
             need, or wrap it"
            .to_string()]
    );
}

#[test]
fn an_accepting_op_on_the_surface_silences_the_advisory() {
    let dir = std::path::Path::new("tests/golden/advisory/doored");
    let program = kanso::compile_module(dir, false).expect("doored compiles");

    let advisories = door_advisories(&program);

    assert_eq!(advisories, Vec::<String>::new());
}

#[test]
fn an_arm_converting_its_own_err_is_advised() {
    let dir = std::path::Path::new("tests/golden/advisory/own_err");
    let program = kanso::compile_module(dir, false).expect("own_err compiles");
    let inference = kanso::infer::infer(&program);

    let advisories = kanso::advisory::license_advisories(&program, &inference.returns);

    assert_eq!(
        advisories,
        vec!["advisory[license]: `position` turns its own `woe` into a value — \
             an err is handled by the package that did not raise it; re-raise, \
             or let callers name the reason themselves"
            .to_string()]
    );
}

#[test]
fn an_arm_that_re_raises_its_own_err_is_silent() {
    let dir = std::path::Path::new("tests/golden/advisory/reraises");
    let program = kanso::compile_module(dir, false).expect("reraises compiles");
    let inference = kanso::infer::infer(&program);

    assert!(kanso::advisory::license_advisories(&program, &inference.returns).is_empty());
}
