// Enumerable fusion: the rewrite that turns a consumer over an adapter chain
// into one fold, and the single-use inlining it runs first. lib.rs reads
// this file with include!, so the items stay in the crate root and the
// compiler builds to the same instructions it did when they sat in lib.rs
// (kanso#1867). cargo fmt does not reach an included file, so CI's format
// job checks the files under src/'s subdirectories itself.

/// Enumerable fusion: a consumer applied to an adapter chain rewrites to
/// one `fold` over the chain's root, the adapter steps composed into the
/// reducer. `fold`'s typed arms make the rewrite sound for any root — a
/// plain list takes the indexed loop, an iterator keeps the protocol — so
/// no per-element wrapper records exist for chains consumed in place.
pub fn fuse_enumerable(program: &mut ast::Program) {
    use ast::Stmt;
    if std::env::var_os("KANSO_NO_FUSE").is_some() {
        return;
    }
    let mut shorts: crate::hash::Map<Name, Name> = crate::hash::Map::default();
    // The set is asked `contains` and then dropped, so it borrows. Owning the
    // short names cost a `String` apiece for a table read once and thrown away.
    // `shorts` below still owns: it outlives the borrow the rewrite mutates
    // through, which is the same reason collapse_diamonds needs a keep mask.
    let std_names: crate::hash::Set<&str> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list"))
        .map(|d| ast::split_qual(&d.name).map(|(_, s)| s).unwrap_or(&d.name))
        .collect();
    for d in &program.fns {
        let short = ast::split_qual(&d.name).map(|(_, s)| s).unwrap_or(&d.name);
        if std_names.contains(short) {
            shorts.insert(Name::new(&d.name), Name::new(short));
        }
    }
    // the fold the rewrite names: a real decl in this program, whichever
    // qualified spelling the module graph produced
    let Some(fold_name) = program
        .fns
        .iter()
        .find(|d| {
            d.file.starts_with("std/list")
                && !d.synthetic
                && ast::split_qual(&d.name).map(|(_, s)| s).unwrap_or(&d.name) == "fold"
        })
        .map(|d| d.name.clone())
    else {
        return;
    };
    // private std/list helpers the generated reducers lean on (bump,
    // file_under), resolved to whatever qualified spelling the module
    // graph produced — privacy is a check-time property, and fusion runs
    // after the check
    let helpers: crate::hash::Map<Name, Name> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list") && !d.synthetic)
        .map(|d| {
            let short = ast::split_qual(&d.name).map_or(d.name.as_str(), |(_, s)| s);
            (Name::new(short), Name::new(&d.name))
        })
        .collect();
    let mut counter = 0usize;
    for decl in &mut program.fns {
        if decl.file.starts_with("std/") {
            continue;
        }
        inline_single_use_chains(&mut decl.body, &shorts);
        for stmt in &mut decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    fuse_expr(expr, &shorts, &fold_name, &helpers, &mut counter);
                }
            }
        }
    }
}

/// A width-forced split must not hide a chain: a binding whose value is an
/// adapter application and whose name is used exactly once — as the
/// collection argument of a later enumerable call — inlines back into the
/// chain before fusion looks. The binding was a rename, not an escape.
fn inline_single_use_chains(body: &mut Vec<ast::Stmt>, shorts: &crate::hash::Map<Name, Name>) {
    use ast::{Expr, Stmt};
    const ADAPTERS: [&str; 5] = ["drop", "map", "reject", "select", "take"];
    let mut idx = 0;
    while idx < body.len() {
        let Stmt::Bind { pattern: ast::Pattern::Var(name, _), expr } = &body[idx] else {
            idx += 1;
            continue;
        };
        let Expr::App { head, .. } = expr else {
            idx += 1;
            continue;
        };
        let Expr::Ident(aname, _, _) = head.as_ref() else {
            idx += 1;
            continue;
        };
        let is_adapter = shorts.get(aname.as_str()).is_some_and(|s| ADAPTERS.contains(&s.as_str()));
        if !is_adapter {
            idx += 1;
            continue;
        }
        let name = name.clone();
        let mut uses = 0usize;
        for later in body.iter().skip(idx + 1) {
            match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    count_ident_uses(expr, &name, &mut uses);
                }
            }
        }
        let sole_coll_use = uses == 1
            && body.iter().skip(idx + 1).any(|later| match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => coll_arg_use(expr, &name, shorts),
            });
        if !sole_coll_use {
            idx += 1;
            continue;
        }
        let Stmt::Bind { expr, .. } = body.remove(idx) else { unreachable!() };
        for later in body.iter_mut().skip(idx) {
            match later {
                Stmt::Bind { expr: e, .. } | Stmt::Expr(e) => {
                    substitute_ident(e, &name, &expr);
                }
            }
        }
    }
}

fn count_ident_uses(e: &ast::Expr, name: &str, uses: &mut usize) {
    if let ast::Expr::Ident(n, _, _) = e {
        if n == name {
            *uses += 1;
        }
    }
    for_each_child(e, |child| count_ident_uses(child, name, uses));
}

/// Is the sole use of `name` the collection argument of an enumerable call?
fn coll_arg_use(e: &ast::Expr, name: &str, shorts: &crate::hash::Map<Name, Name>) -> bool {
    if let ast::Expr::App { head, args, .. } = e {
        if let ast::Expr::Ident(h, _, _) = head.as_ref() {
            if shorts.contains_key(h.as_str()) {
                if let Some(ast::Expr::Ident(first, _, _)) = args.first() {
                    if first == name {
                        return true;
                    }
                }
            }
        }
    }
    any_child(e, |c| coll_arg_use(c, name, shorts))
}

/// Every read of `name` becomes `replacement`, wherever it sits. The walk is
/// `walk_children_mut` rather than a list of forms written out here, because
/// the list this used to be had no arm for a guard: `inline_single_use_chains`
/// counts uses with `for_each_child`, which does descend into one, so a chain
/// whose only reader sat after a `return ... if ...` was removed and the reader
/// left naming nothing. The interpreter answered "unknown name" and the native
/// backend refused the program.
fn substitute_ident(e: &mut ast::Expr, name: &str, replacement: &ast::Expr) {
    if let ast::Expr::Ident(n, _, _) = e {
        if n == name {
            *e = replacement.clone();
            return;
        }
    }
    walk_children_mut(e, &mut |child| substitute_ident(child, name, replacement));
}

fn fuse_expr(
    e: &mut ast::Expr,
    shorts: &crate::hash::Map<Name, Name>,
    fold_name: &str,
    helpers: &crate::hash::Map<Name, Name>,
    counter: &mut usize,
) {
    use ast::Expr;
    match e {
        Expr::App { head, args, .. } => {
            fuse_expr(head, shorts, fold_name, helpers, counter);
            for a in args.iter_mut() {
                fuse_expr(a, shorts, fold_name, helpers, counter);
            }
        }
        Expr::Lambda { body, .. } => fuse_expr(body, shorts, fold_name, helpers, counter),
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                        fuse_expr(expr, shorts, fold_name, helpers, counter)
                    }
                }
            }
        }
        Expr::Join { lhs: a, rhs: b, .. } => {
            fuse_expr(a, shorts, fold_name, helpers, counter);
            fuse_expr(b, shorts, fold_name, helpers, counter);
        }
        Expr::List(items, _) => {
            for i in items {
                fuse_expr(i, shorts, fold_name, helpers, counter);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                fuse_expr(k, shorts, fold_name, helpers, counter);
                fuse_expr(v, shorts, fold_name, helpers, counter);
            }
        }
        Expr::Index { base, index, .. } => {
            fuse_expr(base, shorts, fold_name, helpers, counter);
            fuse_expr(index, shorts, fold_name, helpers, counter);
        }
        Expr::Field { base, .. } => fuse_expr(base, shorts, fold_name, helpers, counter),
        Expr::Upcast { expr, .. } => fuse_expr(expr, shorts, fold_name, helpers, counter),
        Expr::BinOp { lhs, rhs, .. } => {
            fuse_expr(lhs, shorts, fold_name, helpers, counter);
            fuse_expr(rhs, shorts, fold_name, helpers, counter);
        }
        Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    fuse_expr(inner, shorts, fold_name, helpers, counter);
                }
            }
        }
        _ => {}
    }
    if let Some(rewritten) = try_fuse(e, shorts, fold_name, helpers, counter) {
        *e = rewritten;
    } else if let Some(rewritten) = try_fuse_piped(e, shorts, fold_name, helpers, counter) {
        *e = rewritten;
    }
}

/// The pipe spelling of a chain fuses too, one runtime branch late. A piped
/// chain's subject may be a description — the pipe is a bind there, and no
/// rewrite may touch it — so the chain becomes: bind the subject once, and if
/// it is not a description, run the nested spelling of the same chain, which
/// try_fuse has already flattened into one fold. A description takes the
/// original pipes; everything else takes the loop. The cost of the honesty
/// is one tag test per chain and a second copy of the chain's code.
fn try_fuse_piped(
    e: &ast::Expr,
    shorts: &crate::hash::Map<Name, Name>,
    fold_name: &str,
    helpers: &crate::hash::Map<Name, Name>,
    counter: &mut usize,
) -> Option<ast::Expr> {
    use ast::{Expr, Pattern, Stmt};
    // collect the piped stages outside-in: consumer first, subject last
    let mut stages: Vec<(&str, &[Expr], crate::diag::Span)> = Vec::new();
    let mut cur = e;
    #[allow(clippy::while_let_loop)]
    loop {
        let Expr::App { head, args, span, piped: true } = cur else { break };
        let Expr::Ident(name, _, _) = head.as_ref() else { break };
        if !shorts.contains_key(name.as_str()) || args.is_empty() {
            break;
        }
        stages.push((name.as_str(), args.as_slice(), *span));
        cur = &args[0];
    }
    if stages.len() < 2 {
        return None;
    }
    let root = cur.clone();
    let span = stages[0].2;
    *counter += 1;
    let tmp = format!("froot{counter}");
    let tmp_ident =
        || Expr::Ident(Name::new(&tmp.clone()), span, crate::ast::Resolution::default());
    // the nested spelling with the bound subject as its innermost source
    let mut nested = tmp_ident();
    for (name, args, sspan) in stages.iter().rev() {
        let mut all = vec![nested];
        all.extend(args[1..].iter().cloned());
        nested = Expr::App {
            head: Box::new(Expr::Ident(Name::new(name), *sspan, crate::ast::Resolution::default())),
            args: all,
            span: *sspan,
            piped: false,
        };
    }
    let fused = try_fuse(&nested, shorts, fold_name, helpers, counter)?;
    // the original pipes, re-rooted at the binding
    let mut original = tmp_ident();
    for (name, args, sspan) in stages.iter().rev() {
        let mut all = vec![original];
        all.extend(args[1..].iter().cloned());
        original = Expr::App {
            head: Box::new(Expr::Ident(Name::new(name), *sspan, crate::ast::Resolution::default())),
            args: all,
            span: *sspan,
            piped: true,
        };
    }
    let test = Expr::App {
        head: Box::new(Expr::Ident(Name::new("is_desc"), span, crate::ast::Resolution::default())),
        args: vec![tmp_ident()],
        span,
        piped: false,
    };
    let picked = Expr::App {
        head: Box::new(Expr::Ident(Name::new("if"), span, crate::ast::Resolution::default())),
        args: vec![test, original, fused],
        span,
        piped: false,
    };
    Some(Expr::Block(
        vec![
            Stmt::Bind { pattern: Pattern::Var(Name::new(&tmp), span), expr: root },
            Stmt::Expr(picked),
        ],
        span,
    ))
}

fn try_fuse(
    e: &ast::Expr,
    shorts: &crate::hash::Map<Name, Name>,
    fold_name: &str,
    helpers: &crate::hash::Map<Name, Name>,
    counter: &mut usize,
) -> Option<ast::Expr> {
    use ast::Expr;
    let Expr::App { head, args, span, piped: false } = e else { return None };
    let Expr::Ident(cname, _, _) = head.as_ref() else { return None };
    let consumer = shorts.get(cname.as_str())?.clone();
    let span = *span;
    let lam = |params: Vec<&String>, body: Expr| Expr::Lambda {
        params: params.iter().map(|p| ((*p).clone(), span)).collect(),
        body: Box::new(body),
        span,
    };
    let ident = |n: &str| Expr::Ident(Name::new(n), span, crate::ast::Resolution::default());
    let call = |h: Expr, a: Vec<Expr>| Expr::App { head: Box::new(h), args: a, span, piped: false };
    *counter += 1;
    let acc = format!("facc{counter}");
    let x = format!("felem{counter}");
    let (init, reducer) = match (consumer.as_str(), args.len()) {
        ("fold", 3) => (args[1].clone(), args[2].clone()),
        ("tally", 1) => {
            let bump = helpers.get("bump")?;
            let seen = Expr::Index {
                base: Box::new(ident(&acc)),
                index: Box::new(ident(&x)),
                strict: false,
                span,
            };
            let bumped = call(ident(bump), vec![seen]);
            (
                Expr::MapLit(Vec::new(), span),
                lam(vec![&acc, &x], call(ident("put"), vec![ident(&acc), ident(&x), bumped])),
            )
        }
        ("group_by", 2) => {
            let file_under = helpers.get("file_under")?;
            let keyed = call(args[1].clone(), vec![ident(&x)]);
            (
                Expr::MapLit(Vec::new(), span),
                lam(vec![&acc, &x], call(ident(file_under), vec![ident(&acc), keyed, ident(&x)])),
            )
        }
        ("sum", 1) => (
            Expr::Int(0u32.into(), span),
            lam(
                vec![&acc, &x],
                Expr::BinOp { op: "+", lhs: Box::new(ident(&acc)), rhs: Box::new(ident(&x)), span },
            ),
        ),
        ("to_list", 1) => (
            Expr::List(Vec::new(), span),
            lam(vec![&acc, &x], call(ident("push"), vec![ident(&acc), ident(&x)])),
        ),
        ("count", 2) => (
            Expr::Int(0u32.into(), span),
            lam(
                vec![&acc, &x],
                call(
                    ident("if"),
                    vec![
                        call(args[1].clone(), vec![ident(&x)]),
                        Expr::BinOp {
                            op: "+",
                            lhs: Box::new(ident(&acc)),
                            rhs: Box::new(Expr::Int(1u32.into(), span)),
                            span,
                        },
                        ident(&acc),
                    ],
                ),
            ),
        ),
        _ => return None,
    };
    let mut source = args[0].clone();
    let mut reducer = reducer;
    let mut fused_any = false;
    #[allow(clippy::while_let_loop)]
    loop {
        let Expr::App { head: ahead, args: aargs, piped: false, .. } = &source else { break };
        let Expr::Ident(aname, _, _) = ahead.as_ref() else { break };
        let Some(adapter) = shorts.get(aname.as_str()).cloned() else { break };
        if aargs.len() != 2 {
            break;
        }
        *counter += 1;
        let a2 = format!("facc{counter}");
        let x2 = format!("felem{counter}");
        let step = aargs[1].clone();
        reducer = match adapter.as_str() {
            "map" => lam(
                vec![&a2, &x2],
                call(reducer.clone(), vec![ident(&a2), call(step, vec![ident(&x2)])]),
            ),
            "select" => lam(
                vec![&a2, &x2],
                call(
                    ident("if"),
                    vec![
                        call(step, vec![ident(&x2)]),
                        call(reducer.clone(), vec![ident(&a2), ident(&x2)]),
                        ident(&a2),
                    ],
                ),
            ),
            "reject" => lam(
                vec![&a2, &x2],
                call(
                    ident("if"),
                    vec![
                        call(step, vec![ident(&x2)]),
                        ident(&a2),
                        call(reducer.clone(), vec![ident(&a2), ident(&x2)]),
                    ],
                ),
            ),
            _ => break,
        };
        let Expr::App { args: aargs, .. } = source else { unreachable!() };
        source = aargs.into_iter().next().expect("adapters carry a source");
        fused_any = true;
    }
    if !fused_any {
        return None;
    }
    Some(call(ident(fold_name), vec![source, init, reducer]))
}
