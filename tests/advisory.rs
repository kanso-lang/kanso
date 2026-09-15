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

fn licenses(dir: &str) -> Vec<String> {
    let dir = std::path::Path::new(dir);
    let program = kanso::compile_module(dir, false).expect("module compiles");
    let inference = kanso::infer::infer(&program);
    let prov = kanso::provenance::analyze(&program);
    kanso::provenance::violations(&program, &prov, &inference.returns)
}

/// Gavel 24 made the two-universe rule dispatch semantics, so an arm written
/// for an err its own hako raised is dead rather than merely unlicensed, and
/// the message says so. The flow is proved by a written call site: the pub
/// self-seed that used to stand in for callers out of view retired with the
/// gavel, because under it every pub bare-err arm was a violation — including
/// `std/testing`'s `when_failed`, which the whole testing design turns on.
#[test]
fn an_arm_for_this_packages_own_err_can_never_match() {
    assert_eq!(
        licenses("tests/golden/advisory/own_err"),
        vec!["error[license]: `position` has an arm for an err raised in `own_err`, \
             and that arm can never match — a failure does not enter an arm its own \
             hako raised, so it passes as though the arm were not written. Return an \
             err, or let a caller in another package name the reason"
            .to_string()]
    );
}

/// Provenance keys its fixpoint on (name, arity). Nothing pinned the name half
/// of that: collapsing every group's name to `""` left all six advisories here
/// green and lib/json's three unchanged, so the pass could have been gutted
/// without a spec noticing.
///
/// `recover` is fed by a written call site; `quiet` is never called, so nothing
/// reaches it. They differ only in their name, and if the key stops telling
/// them apart `quiet` inherits what `recover` was fed and is reported for a
/// rescue it never made.
#[test]
fn a_group_is_told_apart_by_its_name_and_not_only_its_arity() {
    assert_eq!(
        licenses("tests/golden/advisory/group_identity"),
        vec!["error[license]: `recover` has an arm for an err raised in \
             `group_identity`, and that arm can never match — a failure does not \
             enter an arm its own hako raised, so it passes as though the arm were \
             not written. Return an err, or let a caller in another package name \
             the reason"
            .to_string()]
    );
}

#[test]
fn re_raising_ones_own_err_is_silent() {
    assert!(licenses("tests/golden/advisory/reraises").is_empty());
}

/// The case a reason-type proxy cannot see: the err was raised here, and
/// only its reason was borrowed from elsewhere. Provenance is the raiser.
#[test]
fn laundering_an_own_err_through_a_foreign_reason_is_refused() {
    // GAVEL 1b subsumes the advisory this used to assert. Forging a foreign
    // reason so your own err can be rescued under a borrowed name is no longer
    // advised against — it does not compile, because only a reason's owner
    // builds one. The advisory itself stays live where the reason is genuinely
    // foreign, which own_err and doored cover; what retires is the forged
    // variant, and this pins the refusal that replaced it.
    let dir = std::path::Path::new("tests/golden/advisory/laundered");
    let said = kanso::compile_module(dir, false).expect_err("the forged reason is refused");

    assert_eq!(
        said,
        "error[opacity]: `json/parse_failure` is foreign — only `json` builds a \
         `parse_failure`; ask it for one through a pub function (module \
         tests/golden/advisory/laundered)\n"
    );
}

/// The other direction, which must stay legal: the failure really is
/// somebody else's, so this program is the party that may answer it.
#[test]
fn rescuing_a_genuinely_foreign_err_is_silent() {
    assert!(licenses("tests/golden/advisory/foreign_rescue").is_empty());
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
