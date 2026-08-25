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

#[test]
fn rescuing_an_err_this_package_raised_is_advised() {
    assert_eq!(
        licenses("tests/golden/advisory/own_err"),
        vec!["advisory[license]: `position` rescues an err raised in this program \
             — a failure is handled by a package that did not raise it; return an \
             err, or let a caller elsewhere name the reason"
            .to_string()]
    );
}

/// Provenance keys its fixpoint on (name, arity). Nothing pinned the name half
/// of that: collapsing every group's name to `""` left all six advisories here
/// green and lib/json's three unchanged, so the pass could have been gutted
/// without a spec noticing.
///
/// `rescue` is pub, so provenance seeds it — a published err parameter is
/// assumed to see its own package's failures, because its callers are not all
/// in view. `quiet` is private and uncalled, so nothing feeds it. They differ
/// only in their name, and if the key stops telling them apart `quiet`
/// inherits what `rescue` was fed and is advised for a rescue it never made.
#[test]
fn a_group_is_told_apart_by_its_name_and_not_only_its_arity() {
    assert_eq!(
        licenses("tests/golden/advisory/group_identity"),
        vec!["advisory[license]: `rescue` rescues an err raised in this program \
             — a failure is handled by a package that did not raise it; return an \
             err, or let a caller elsewhere name the reason"
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
