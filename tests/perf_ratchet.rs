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
