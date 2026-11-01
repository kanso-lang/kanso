//! Where a string is built by joining onto itself: the third of the three
//! answers `for_the_emitter` gives, read off the shared `Analysis`.

use super::*;

/// Where a string is built by joining onto itself, and which parameter holds
/// the builder.
///
/// `text_build (n - 1) "{s}x"` reads `s` to make the new string and mentions
/// it nowhere else, so by the time the join runs `s` is finished. Asked the
/// way record reuse is asked: every caller hands the parameter over, and every
/// mention of it in the arm is inside this one expression. The shared fixpoint
/// is neither consulted nor changed.
///
/// Returns the join sites, and the (name, arity, index) of each accumulator so
/// the emitter can convert the seed where a caller hands one in from outside.
pub fn string_builders(program: &Program) -> (Sites, Slots, Sites) {
    string_builders_with(&Analysis::new(program), program)
}

/// The body of `string_builders`, reading an `Analysis` it was handed.
pub(super) fn string_builders_with(
    analysis: &Analysis,
    program: &Program,
) -> (Sites, Slots, Sites) {
    let mut sites = HashSet::default();
    let mut accs = Slots::default();
    for decl in real_fns(program) {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            walk_for_builder(analysis, decl, e, &mut sites, &mut accs);
        }
    }
    named_folder_joins(analysis, program, &mut sites);
    // A parameter that forwards the accumulator on is carrying one too, so it
    // joins the set whose callers convert a seed — the conversion moves out to
    // where the value enters the cycle, rather than happening on every hop.
    let (accs, carried) = carried_args(analysis, program, &sites, accs);
    (sites, accs, carried)
}

/// Where a call argument is already carrying the builder, so converting a seed
/// at that site would copy a string this component already owns.
///
/// `build_onto -> handed -> build_onto` is one accumulator passed round a
/// cycle, and only the self-call is recognised as staying inside it. Every
/// other hop looks like a caller from outside and re-seeds, which copies the
/// whole string once per iteration. A parameter a group forwards straight into
/// a carrying position is carrying one too, on the same terms the accumulator
/// itself was granted: every caller hands it over, and the arm mentions it
/// nowhere but that call.
fn carried_args(a: &Analysis, program: &Program, joins: &Sites, accs: Slots) -> (Slots, Sites) {
    let mut carrying = accs;
    loop {
        let before = carrying.len();
        for decl in real_fns(program) {
            let arity = decl.params.len();
            for (j, _) in param_names(decl) {
                if carrying.contains(&decl.name, arity, j) {
                    continue;
                }
                // Every arm of the group, not just this one: the set is keyed by
                // group, so an arm that aliases the parameter or answers it as a
                // plain string would inherit an exemption it cannot honour.
                let arms: Vec<&FnDecl> =
                    a.group(&decl.name, arity).filter(|d| !d.synthetic).collect();
                let all = arms.iter().all(|d| match d.params.get(j) {
                    Some(Pattern::Var(n, _)) => forwards_into(&carrying, d, n),
                    _ => false,
                });
                // A group nothing calls answers every call-site question yes for
                // free. An import enrols a bare-named clone that no call site
                // names, and these sites are keyed by source position, which the
                // twins share — so a grant made to the clone would reach the code
                // the real name emits.
                if all
                    && a.mentions.called_at(&decl.name, arity)
                    && a.callers_hand_over(&decl.name, arity, j)
                {
                    carrying.insert((decl.name.clone(), arity, j));
                }
            }
        }
        if carrying.len() == before {
            break;
        }
    }
    let mut out = HashSet::default();
    for decl in real_fns(program) {
        let built = built_locals(joins, decl);
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            collect_carried(&carrying, &built, decl, e, &mut out);
        }
    }
    (carrying, out)
}

/// Names this arm binds to the join itself — the builder made right here.
/// A guard holds the statements after it, so a join bound below a
/// `return acc if …` sits inside the guard rather than in the body's own list,
/// and a bound branch holds its own block; both are walked. Missing them
/// re-seeded the builder at the call that handed it on, a copy of the whole
/// string on every lap.
fn built_locals(joins: &Sites, decl: &FnDecl) -> HashSet<String> {
    fn in_stmts(joins: &Sites, decl: &FnDecl, stmts: &[Stmt], out: &mut HashSet<String>) {
        for s in stmts {
            let e = match s {
                Stmt::Bind { pattern: Pattern::Var(name, _), expr: Expr::Str(_, span) } => {
                    if joins.contains(&(decl.file.clone(), span.line as usize, span.col as usize)) {
                        out.insert(name.to_string());
                    }
                    continue;
                }
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            in_expr(joins, decl, e, out);
        }
    }
    fn in_expr(joins: &Sites, decl: &FnDecl, e: &Expr, out: &mut HashSet<String>) {
        match e {
            Expr::Guard { rest, .. } => in_stmts(joins, decl, rest, out),
            Expr::Block(stmts, _) => in_stmts(joins, decl, stmts, out),
            Expr::Lambda { .. } => {}
            _ => {
                for c in child_exprs(e) {
                    in_expr(joins, decl, c, out);
                }
            }
        }
    }
    let mut out = HashSet::default();
    in_stmts(joins, decl, &decl.body, &mut out);
    out
}

fn param_names(decl: &FnDecl) -> Vec<(usize, String)> {
    decl.params
        .iter()
        .enumerate()
        .filter_map(|(j, p)| match p {
            Pattern::Var(n, _) => Some((j, n.to_string())),
            _ => None,
        })
        .collect()
}

/// Does this arm hand `name` on into a carrying position, and mention it
/// nowhere else?
fn forwards_into(carrying: &Slots, decl: &FnDecl, name: &str) -> bool {
    let everywhere: usize = decl
        .body
        .iter()
        .map(|s| match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => count_in_expr(name, expr),
        })
        .sum();
    let mut found = false;
    for stmt in &decl.body {
        let e = match stmt {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
        };
        walk_forwards(carrying, e, name, &mut found);
    }
    found && everywhere == 1
}

fn walk_forwards(carrying: &Slots, e: &Expr, name: &str, found: &mut bool) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(callee, _, _) = head.as_ref() {
            for (i, arg) in args.iter().enumerate() {
                if matches!(arg, Expr::Ident(n, _, _) if n == name)
                    && carrying.contains(callee, args.len(), i)
                {
                    *found = true;
                }
            }
        }
    }
    for child in child_exprs(e) {
        walk_forwards(carrying, child, name, found);
    }
}

fn collect_carried(
    carrying: &Slots,
    built: &HashSet<String>,
    decl: &FnDecl,
    e: &Expr,
    out: &mut Sites,
) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(callee, _, _) = head.as_ref() {
            for (i, arg) in args.iter().enumerate() {
                if !carrying.contains(callee, args.len(), i) {
                    continue;
                }
                if let Expr::Ident(n, span, _) = arg {
                    let holds = built.contains(n.as_str())
                        || param_names(decl).into_iter().any(|(j, p)| {
                            &p == n && carrying.contains(&decl.name, decl.params.len(), j)
                        });
                    if holds {
                        out.insert((decl.file.clone(), span.line as usize, span.col as usize));
                    }
                }
            }
        }
    }
    for child in child_exprs(e) {
        collect_carried(carrying, built, decl, child, out);
    }
}

fn walk_for_builder(
    a: &Analysis,
    decl: &FnDecl,
    e: &Expr,
    sites: &mut HashSet<(std::sync::Arc<str>, usize, usize)>,
    accs: &mut Slots,
) {
    // a lambda runs when somebody else decides
    if matches!(e, Expr::Lambda { .. }) {
        return;
    }
    // except a fold's own folder, which runs once per element on the
    // accumulator the fold owns: a join onto that accumulator writes into it
    if let Expr::App { head, args, .. } = e {
        if matches!(head.as_ref(), Expr::Ident(n, _, _) if a.folds.contains(n.as_str()))
            && args.len() == 3
            && a.fold_owns_accumulator(args, decl, None)
        {
            if let Expr::Lambda { params, body, .. } = &args[2] {
                if let Some((acc, _)) = params.first() {
                    mark_folder_joins(decl, body, acc, sites);
                }
            }
        }
    }
    if let Expr::Str(parts, span) = e {
        if parts.len() > 1 {
            if let Some(TemplatePart::Interp(Expr::Ident(name, _, _))) = parts.first() {
                if let Some(i) = builder_param(a, decl, name, e) {
                    sites.insert((decl.file.clone(), span.line as usize, span.col as usize));
                    accs.insert((decl.name.clone(), decl.params.len(), i));
                }
            }
            // `"{st.body}x"`, where `st` is a record handed over whole whose
            // `body` nothing else holds, every mention of `st` reads a field,
            // and `body` is read here alone: the string is finished with by
            // the time the join runs, as a builder parameter's is.
            if let Some(TemplatePart::Interp(first)) = parts.first() {
                if let Some((p, f)) = field_read(first) {
                    if a.field_moves(p, f, decl) {
                        sites.insert((decl.file.clone(), span.line as usize, span.col as usize));
                    }
                }
            }
        }
    }
    for child in child_exprs(e) {
        walk_for_builder(a, decl, child, sites, accs);
    }
}

/// Whether every mention of `name` in an arm is a piece of a template. A
/// template reads its pieces into a string of its own and keeps none of them,
/// so an arm like `seed n acc = seed (n - 1) "{acc}{acc}"` reads the value
/// twice and leaves nothing holding it.
pub(super) fn only_rendered(name: &str, body: &[Stmt]) -> bool {
    fn ok(name: &str, e: &Expr) -> bool {
        match e {
            Expr::Ident(n, _, _) => n != name,
            Expr::Str(parts, _) => parts.iter().all(|part| match part {
                TemplatePart::Interp(Expr::Ident(_, _, _)) => true,
                TemplatePart::Interp(inner) => ok(name, inner),
                TemplatePart::Lit(_) => true,
            }),
            _ => child_exprs(e).all(|c| ok(name, c)),
        }
    }
    body.iter().all(|s| match s {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) => ok(name, expr),
    })
}

/// Is `e` a template that starts with `acc` and mentions it nowhere else?
pub(super) fn joins_onto(e: &Expr, acc: &str) -> bool {
    let Expr::Str(parts, _) = e else { return false };
    parts.len() > 1
        && matches!(parts.first(), Some(TemplatePart::Interp(Expr::Ident(n, _, _))) if n == acc)
        && count_in_expr(acc, e) == 1
}

/// The joins in a validated folder's body that extend its accumulator, along
/// the paths `body_is_folder` accepts: the body itself, either arm of a
/// conditional, and a wrapper applied where it stands.
fn mark_folder_joins(
    decl: &FnDecl,
    body: &Expr,
    acc: &str,
    sites: &mut HashSet<(std::sync::Arc<str>, usize, usize)>,
) {
    match body {
        Expr::Str(_, span) if joins_onto(body, acc) => {
            sites.insert((decl.file.clone(), span.line as usize, span.col as usize));
        }
        Expr::App { head, args, .. } => match head.as_ref() {
            Expr::Lambda { params, body: inner, .. } => {
                if let Some((inner_acc, _)) = params.first() {
                    mark_folder_joins(decl, inner, inner_acc, sites);
                }
            }
            Expr::Ident(n, _, _) if n == "if" && args.len() == 3 => {
                mark_folder_joins(decl, &args[1], acc, sites);
                mark_folder_joins(decl, &args[2], acc, sites);
            }
            _ => {}
        },
        _ => {}
    }
}

/// Joins in a function handed to a fold by name, `fold xs "" piece`, where
/// `piece acc i` answers `"{acc}..."`. The fold applies it once per element to
/// the accumulator it owns, as it would a lambda, so the join may write into
/// that accumulator on the same terms: every arm's last expression joins onto
/// its first parameter and mentions it nowhere else, every direct call hands
/// that argument over, and every place the name is used as a value is the
/// folder of a fold whose seed nothing else holds.
fn named_folder_joins(
    a: &Analysis,
    program: &Program,
    sites: &mut HashSet<(std::sync::Arc<str>, usize, usize)>,
) {
    for decl in real_fns(program) {
        if decl.params.len() != 2 || !a.escapes_as_value(&decl.name, 2) {
            continue;
        }
        let arms: Vec<&FnDecl> =
            real_fns(program).filter(|d| d.name == decl.name && d.params.len() == 2).collect();
        // each group once, from its first arm
        if !std::ptr::eq(arms[0], decl) {
            continue;
        }
        let joins: Option<Vec<&Expr>> = arms
            .iter()
            .map(|d| {
                let Some(Pattern::Var(acc, _)) = d.params.first() else { return None };
                let [Stmt::Expr(last)] = d.body.as_slice() else { return None };
                joins_onto(last, acc).then_some(last)
            })
            .collect();
        let Some(joins) = joins else { continue };
        if !a.call_sites_hand_over(&decl.name, 2, 0) || !only_folded(a, program, &decl.name) {
            continue;
        }
        for (d, join) in arms.iter().zip(joins) {
            if let Expr::Str(_, span) = join {
                sites.insert((d.file.clone(), span.line as usize, span.col as usize));
            }
        }
    }
}

/// Is every mention of `name` that is not the head of a call the folder of
/// a fold whose seed nothing else holds?
fn only_folded(a: &Analysis, program: &Program, name: &str) -> bool {
    fn walk(a: &Analysis, ctx: &FnDecl, e: &Expr, name: &str, ok: &mut bool) {
        if !*ok {
            return;
        }
        match e {
            Expr::Ident(n, _, _) if n == name => *ok = false,
            Expr::App { head, args, .. } => {
                let folded = matches!(head.as_ref(), Expr::Ident(f, _, _) if a.folds.contains(f.as_str()))
                    && args.len() == 3
                    && matches!(&args[2], Expr::Ident(n, _, _) if n == name);
                if folded {
                    if !a.unique_in(&args[1], ctx, None) {
                        *ok = false;
                    }
                    walk(a, ctx, &args[0], name, ok);
                    walk(a, ctx, &args[1], name, ok);
                    return;
                }
                match head.as_ref() {
                    // a partial application keeps what it was given
                    Expr::Ident(n, _, _) if n == name && args.len() != 2 => *ok = false,
                    Expr::Ident(n, _, _) if n == name => {}
                    _ => walk(a, ctx, head, name, ok),
                }
                for arg in args {
                    walk(a, ctx, arg, name, ok);
                }
            }
            _ => crate::for_each_child(e, |c| walk(a, ctx, c, name, ok)),
        }
    }
    let mut ok = true;
    for &d in a.mentions.mentioning(name) {
        let decl = &program.fns[d];
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            walk(a, decl, e, name, &mut ok);
        }
    }
    ok
}

/// The parameter index of `name`, when every caller hands it over and this
/// expression holds every mention of it in the arm.
fn builder_param(a: &Analysis, decl: &FnDecl, name: &str, here: &Expr) -> Option<usize> {
    let arity = decl.params.len();
    let i = decl.params.iter().position(|p| matches!(p, Pattern::Var(n, _) if n == name))?;
    let inside = count_in_expr(name, here);
    let everywhere: usize = decl
        .body
        .iter()
        .map(|s| match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => count_in_expr(name, expr),
        })
        .sum();
    match inside == everywhere && a.callers_hand_over(&decl.name, arity, i) {
        true => Some(i),
        false => None,
    }
}
