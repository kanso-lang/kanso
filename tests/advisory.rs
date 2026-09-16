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

/// The case a reason-type proxy cannot see: the err was raised here, and
/// only its reason was borrowed from elsewhere. Provenance is the raiser.
#[test]
fn laundering_an_own_err_through_a_foreign_reason_is_refused() {
    // GAVEL 1b subsumes the advisory this used to assert. Forging a foreign
    // reason so your own err can be rescued under a borrowed name is no longer
    // advised against — it does not compile, because only a reason's owner
    // builds one. What retires is the forged variant, and this pins the
    // refusal that replaced it.
    let dir = std::path::Path::new("tests/golden/advisory/laundered");
    let said = kanso::compile_module(dir, false).expect_err("the forged reason is refused");

    assert_eq!(
        said,
        "error[opacity]: `json/parse_failure` is foreign — only `json` builds a \
         `parse_failure`; ask it for one through a pub function (module \
         tests/golden/advisory/laundered)\n"
    );
}

/// The fixpoint reaches an answer several hops from where the type is built,
/// and this pins that it still does.
///
/// `return_type_names` used to be round-robin: it asked every declaration in
/// order, over and over, until a whole pass changed nothing. It is a
/// dependency-driven worklist now -- a declaration is re-asked only when one
/// of the answers its body read has grown -- and 92.5% of the visits the
/// round-robin made produced nothing.
///
/// What a worklist can get wrong is stopping early, so the fixture is built to
/// catch exactly that. `relay` is four hops from the declaration that names
/// `json/parse_failure`, and the hops are declared caller-first, which is the
/// worst order for the round-robin: one pass carried the type one hop, so the
/// advisory on `relay` only became true on the fifth. A worklist that failed to
/// re-queue a reader would leave `relay` with an empty answer and this would
/// assert an empty vector.
///
/// The assertion is the advisory a user reads, not the round count or the
/// visit count -- those are the decomposition, and the decomposition is what
/// just moved.
#[test]
fn an_answer_four_hops_from_where_the_type_is_built_still_arrives() {
    let dir = std::path::Path::new("tests/golden/advisory/relayed");
    let program = kanso::compile_module(dir, false).expect("relayed compiles");

    let advisories = door_advisories(&program);

    assert_eq!(
        advisories,
        vec!["advisory[door]: `relay` returns `json/parse_failure` and the \
             surface offers nothing that accepts it — re-export what callers \
             need, or wrap it"
            .to_string()]
    );
}
