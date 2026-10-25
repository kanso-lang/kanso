use super::*;

/// What `Backend::group_indices` did before the index existed, kept as the
/// oracle.
fn scanned(program: &Program, name: &str, arity: usize) -> Vec<usize> {
    program
        .fns
        .iter()
        .enumerate()
        .filter(|(_, d)| d.name == name && d.params.len() == arity)
        .map(|(i, _)| i)
        .collect()
}

/// Every question the emitter can ask, asked both ways. `group_param_set`
/// and `group_return_set` FOLD over the answer, so a missing or extra
/// index is a wrong type set and a silently wrong ABI -- and the fold is
/// order-independent, which is why order is checked here anyway rather
/// than left to a future caller to discover.
fn agrees_over(program: &Program) {
    let by_name = group_indices_by_name(program);
    let mut arities: Vec<usize> = program.fns.iter().map(|d| d.params.len()).collect();
    arities.sort_unstable();
    arities.dedup();
    let mut names: Vec<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
    names.push("a name nothing declares");
    for name in names {
        for &arity in arities.iter().chain(std::iter::once(&99)) {
            let want = scanned(program, name, arity);
            let got: Vec<usize> = group_indices_in(&by_name, program, name, arity).collect();
            assert_eq!(
                want, got,
                "`{name}` at arity {arity}: the scan found {want:?} and the index found {got:?}"
            );
        }
    }
}

#[test]
fn over_the_library_the_compiler_carries() {
    let program = crate::compile_module(std::path::Path::new("lib/json"), false).expect("lib/json");
    agrees_over(&program);
}

#[test]
fn over_a_name_declared_at_two_arities() {
    // One name at two arities, and a second sharing neither: the case a
    // name-keyed index has to get right.
    let src = concat!(
        "fn f x\n  x\n\n",
        "fn f x y\n  x + y\n\n",
        "fn g x\n  x\n\n",
        "main = print \"{f 1} {f 1 2} {g 3}\"\n"
    );
    let program = crate::compile("test.kso", src, false).expect("the fixture compiles");
    agrees_over(&program);
}
