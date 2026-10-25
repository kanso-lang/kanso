//! The index `used_as_value` reads, held to the walk it replaced.

use super::*;

/// What `used_as_value` did before the index existed, kept as the oracle.
/// `value_use` beside it is the original walk, untouched.
fn scanned(program: &Program, name: &str) -> bool {
    program.fns.iter().any(|d| {
        d.body.iter().any(|stmt| {
            let (Stmt::Bind { expr, .. } | Stmt::Expr(expr)) = stmt;
            value_use(expr, name)
        })
    })
}

/// Both ways, over every name the program writes anywhere plus a name it
/// does not. Every identifier is asked, not just the group names the four
/// callers happen to reach, because a name in call position is the case
/// the index has to get RIGHT BY EXCLUDING -- asking only names that are
/// used as values could not tell the two rules apart.
fn agrees_over(src: &str) {
    let program = crate::compile("test.kso", src, false).expect("the sample compiles");
    let index = ValueUses::of(&program);
    let mut every: crate::hash::Set<&str> = crate::hash::Set::default();
    for decl in &program.fns {
        every.insert(decl.name.as_str());
        for stmt in &decl.body {
            let (Stmt::Bind { expr, .. } | Stmt::Expr(expr)) = stmt;
            collect_names_borrowed(expr, &mut every);
        }
    }
    every.insert("a name nothing writes");
    let mut asked = 0;
    for name in &every {
        assert_eq!(
            index.has(name),
            scanned(&program, name),
            "`{name}`: the index and the scan disagree"
        );
        asked += 1;
    }
    assert!(asked > 3, "the sample asked only {asked} names");
}

/// Every identifier anywhere, head position included -- deliberately
/// wider than what the index collects, so the queries reach names the
/// index must answer NO for.
fn collect_names_borrowed<'a>(e: &'a Expr, out: &mut crate::hash::Set<&'a str>) {
    if let Expr::App { head, .. } = e {
        if let Expr::Ident(n, _, _) = head.as_ref() {
            out.insert(n.as_str());
        }
    }
    collect_value_uses(e, out);
    match e {
        Expr::App { head, args, .. } => {
            collect_names_borrowed(head, out);
            for a in args {
                collect_names_borrowed(a, out);
            }
        }
        Expr::Block(stmts, _) => {
            for st in stmts {
                let (Stmt::Bind { expr, .. } | Stmt::Expr(expr)) = st;
                collect_names_borrowed(expr, out);
            }
        }
        _ => {}
    }
}

#[test]
fn a_name_in_call_position_is_not_a_value_use() {
    // `twice` is only ever called, `step` is handed over as a value, and
    // `both` is both. A collector that took every head would say yes to
    // `twice`, and the scan says no.
    agrees_over(concat!(
        "fn step x\n  x + 1\n\n",
        "fn twice f x\n  f (f x)\n\n",
        "fn both g\n  g 1\n\n",
        "main = print \"{twice step 1} {both step}\"\n"
    ));
}

#[test]
fn over_the_shapes_the_walk_has_arms_for() {
    // A list, a map literal, an interpolation, an index, a field, a
    // lambda body and a binop, each carrying a name as a value.
    agrees_over(concat!(
        "fn use_it a b\n  a + b\n\n",
        "one = 1\n\n",
        "two = 2\n\n",
        "xs = [one two]\n\n",
        "m = { 1:one 2:two }\n\n",
        "main = print \"{one} {xs[two]} {use_it one two} {m[one]}\"\n"
    ));
}

#[test]
fn over_the_library_the_compiler_carries() {
    let program = crate::compile_module(std::path::Path::new("lib/json"), false).expect("lib/json");
    let index = ValueUses::of(&program);
    let mut asked = 0;
    for decl in &program.fns {
        let name = decl.name.as_str();
        assert_eq!(
            index.has(name),
            scanned(&program, name),
            "`{name}`: the index and the scan disagree"
        );
        asked += 1;
    }
    assert!(asked > 100, "lib/json declared only {asked} names");
}

/// The original walk, kept as the oracle for the index.
fn value_use(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident(n, _, _) | Expr::Partial(n, _) => n == name,
        Expr::Block(stmts, _) => stmts.iter().any(|st| match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => value_use(expr, name),
        }),
        Expr::App { head, args, .. } => {
            let head_is_plain_name = matches!(head.as_ref(), Expr::Ident(..));
            (!head_is_plain_name && value_use(head, name))
                || args.iter().any(|a| value_use(a, name))
        }
        Expr::Field { base, .. } => value_use(base, name),
        Expr::Upcast { expr, .. } => value_use(expr, name),
        Expr::Index { base, index, .. } => value_use(base, name) || value_use(index, name),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            value_use(lhs, name) || value_use(rhs, name)
        }
        Expr::Guard { cond, early, rest, .. } => {
            value_use(cond, name)
                || value_use(early, name)
                || rest.iter().any(|s| value_use(guard_stmt_expr(s), name))
        }
        Expr::Lambda { body, .. } => value_use(body, name),
        Expr::List(items, _) => items.iter().any(|i| value_use(i, name)),
        Expr::MapLit(pairs, _) => {
            pairs.iter().any(|(k, v)| value_use(k, name) || value_use(v, name))
        }
        Expr::Str(parts, _) => parts.iter().any(|p| match p {
            TemplatePart::Interp(inner) => value_use(inner, name),
            TemplatePart::Lit(_) => false,
        }),
        Expr::Int(..) | Expr::Float(..) => false,
    }
}
