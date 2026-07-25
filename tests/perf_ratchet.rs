//! Performance specs that read the compiled representation instead of a
//! clock. Wall time is noisy and machine-bound; these facts are exact: if a
//! change un-inlines the hot predicates, drops tail-call elimination, or
//! un-wires in-place list reuse, a spec here fails as a diff — the ratchet's
//! other half is the cost golden (bench/cost_golden.txt) diffed in CI.

fn ir_for(source: &str) -> String {
    let program = kanso::compile("spec.kso", source, true).expect("spec program compiles");
    kanso::codegen::emit_ir(&program).expect("spec program lowers to IR")
}

const RECURSIVE: &str = "fn build 0 acc
  acc

fn build n acc
  build (n - 1) (push acc n)

main = print \"{length (build 5 [])}\"
";

#[test]
fn hot_predicates_are_inline_definitions_not_declares() {
    let ir = ir_for(RECURSIVE);

    let define_line = |name: &str| {
        ir.lines()
            .find(|l| l.starts_with("define internal") && l.contains(&format!("@{name}(")))
            .unwrap_or_else(|| {
                panic!(
                    "{name} lost its IR twin — LTO does not inline across the \
                     .ll/.o boundary, so this regresses the hottest path"
                )
            })
            .to_string()
    };
    let all = [
        "k_truthy",
        "k_not_failure",
        "k_check_tag",
        "k_check_int",
        "k_check_bool",
        "k_int",
        "k_float",
        "k_bool",
        "k_none",
    ];
    for name in all {
        let line = define_line(name);

        assert!(
            line.contains("alwaysinline"),
            "{name}'s IR twin dropped alwaysinline — the define exists but no \
             longer folds into call sites: {line}"
        );
        assert!(
            !ir.contains(&format!("declare i64 @{name}(")),
            "{name} is declared as an external call again — the inline twin is bypassed"
        );
    }
}

#[test]
fn self_recursion_compiles_to_musttail() {
    let ir = ir_for(RECURSIVE);

    assert!(
        ir.contains("musttail call"),
        "recursive dispatch lost musttail — the constant-stack guarantee is gone"
    );
}

#[test]
fn linear_list_accumulator_pushes_in_place() {
    let ir = ir_for(RECURSIVE);

    assert!(
        ir.contains("@k_b_push_mut("),
        "the linearly-threaded accumulator fell back to copying k_b_push — \
         in-place reuse is un-wired"
    );
}

/// The failure predicate is written twice: once in C, once as the LLVM twin
/// the emitter inlines. They must test the same tags, and nothing else in the
/// suite notices when they drift — a program only sees the difference where a
/// none meets a path that inlines. Pin the twin to err alone.
#[test]
fn the_failure_twin_tests_err_and_nothing_else() {
    let ir = ir_for(RECURSIVE);
    let body: String = ir
        .lines()
        .skip_while(|l| !l.contains("define internal i64 @k_not_failure("))
        .take_while(|l| !l.starts_with('}'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        body.contains("icmp ne i64 %tag, 5"),
        "the twin no longer tests the err tag: {body}"
    );
    assert!(
        !body.contains(", 4"),
        "the twin tests the none tag again — a none is a value, not a failure: {body}"
    );
}

const ANY_FIELD: &str = "type box
  v:any int

main = print \"{box 7}\"
";

/// `any` accepts every value except the absence channel, so its check is a
/// call that tests the tag — not the constant true it was before `none`
/// became a value the set had to exclude.
#[test]
fn any_tests_the_tag_rather_than_passing_everything() {
    let ir = ir_for(ANY_FIELD);
    assert!(
        ir.contains("call i64 @k_check_any("),
        "an `any` member no longer calls k_check_any, so it either passes a \
         none or stopped being checked at all"
    );
}

const STRICT_INDEX: &str = "main =
  xs = [1 2]
  print \"{xs[1]!}\"
";

/// The strict index is a different call from the lenient one: it errs where
/// the lenient form answers none, which is what lets a guarded lookup say so.
#[test]
fn the_strict_index_errs_instead_of_answering_none() {
    let ir = ir_for(STRICT_INDEX);
    assert!(
        ir.contains("@k_index("),
        "`xs[i]!` stopped emitting the erring index, so a miss would answer \
         none where the author said it cannot happen"
    );
}

const GUARD: &str = "main = print (pick 3)

fn pick n
  return \"low\" if (n < 10)
  \"high\"
";

/// A guard is a branch, not a call into a runtime helper — the tail it skips
/// is the untaken arm of a conditional.
#[test]
fn a_guard_compiles_to_a_branch() {
    let ir = ir_for(GUARD);
    let picks: Vec<&str> = ir
        .lines()
        .skip_while(|l| !(l.starts_with("define") && l.contains("@d_pick_1")))
        .take_while(|l| !l.starts_with('}'))
        .collect();
    let body = picks.join("\n");
    assert!(
        body.contains("br i1 "),
        "a guard stopped compiling to a conditional branch: {body}"
    );
}
