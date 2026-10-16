//! Where a stack overflow is reported (ruled on #1975).
//!
//! A recursion deeper than the stack stops the program on every engine, at a
//! depth that differs between them: the interpreter counts calls and the
//! compiled program runs out of machine stack. Whichever frame was entering
//! when it ran out is an accident of that depth, and in a cycle of two groups
//! the two engines would name different ones. So the report names the cycle
//! instead, by one fixed place: the first call written inside the cycle, to a
//! member of it, outside tail position, in program order.
//!
//! A group carries that place when one of its arms makes such a call, since
//! only a call that is not in tail position grows the stack. Each engine
//! records the place on entering the group, the compiled program in the
//! group's function and the interpreter when an arm's body starts, and the
//! overflow prints the last one recorded. A group whose recursion is all in
//! tail position runs in constant stack on both engines and records nothing.

use crate::ast::{Expr, FnDecl, Program, Stmt};
use crate::diag::Span;
use crate::hash::Map;

/// For each declaration, in `program.fns` order, the place an overflow in its
/// group's cycle names: the declaration the call was written in, and the
/// call's span. `None` for a declaration whose group is on no cycle it can
/// grow the stack through.
pub fn deep_sites(program: &Program) -> Vec<Option<(usize, Span)>> {
    let mut groups: Map<(&str, usize), usize> = Map::default();
    let mut group_of = Vec::with_capacity(program.fns.len());
    for d in &program.fns {
        let next = groups.len();
        let g = *groups.entry((d.name.as_str(), d.params.len())).or_insert(next);
        group_of.push(g);
    }
    // every call by name to a group: (calling declaration, callee group,
    // span, whether it is in tail position), in program order
    let mut calls: Vec<(usize, usize, Span, bool)> = Vec::new();
    for (i, d) in program.fns.iter().enumerate() {
        let last = d.body.len().saturating_sub(1);
        for (k, stmt) in d.body.iter().enumerate() {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            let tail = k == last && matches!(stmt, Stmt::Expr(_));
            walk(e, tail, &mut |callee, arity, span, tail| {
                if let Some(&g) = groups.get(&(callee, arity)) {
                    calls.push((i, g, span, tail));
                }
            });
        }
    }
    let mut adj = vec![Vec::new(); groups.len()];
    for &(from, to, _, _) in &calls {
        adj[group_of[from]].push(to);
    }
    let mut cycle_of = vec![usize::MAX; groups.len()];
    for (id, scc) in crate::beat::sccs_of(&adj).into_iter().enumerate() {
        let on_cycle = scc.len() > 1 || adj[scc[0]].contains(&scc[0]);
        if on_cycle {
            for g in scc {
                cycle_of[g] = id;
            }
        }
    }
    // the cycle's place, and which groups grow the stack through it
    let mut place: Map<usize, (usize, Span)> = Map::default();
    let mut grows = vec![false; groups.len()];
    for &(from, to, span, tail) in &calls {
        let c = cycle_of[group_of[from]];
        if tail || c == usize::MAX || cycle_of[to] != c {
            continue;
        }
        place.entry(c).or_insert((from, span));
        grows[group_of[from]] = true;
    }
    group_of
        .iter()
        .map(|&g| match grows[g] {
            true => place.get(&cycle_of[g]).copied(),
            false => None,
        })
        .collect()
}

/// The place as an err's origin spells it, "{fn} at {file}:{line}".
pub fn spelled(program: &Program, (decl, span): (usize, Span)) -> String {
    let d: &FnDecl = &program.fns[decl];
    format!("{} at {}:{}", crate::ast::frame_name(&d.name), d.file, span.line)
}

/// Every call by name under `e`, with whether it is in tail position. A
/// branch of an `if` in tail position is in tail position; a lambda's body
/// runs somewhere else, so nothing in it is.
fn walk<'a>(e: &'a Expr, tail: bool, found: &mut impl FnMut(&'a str, usize, Span, bool)) {
    if let Expr::App { head, args, piped: false, span } = e {
        if let Expr::Ident(name, _, _) = head.as_ref() {
            if name.as_str() == "if" && args.len() == 3 {
                walk(&args[0], false, found);
                walk(&args[1], tail, found);
                walk(&args[2], tail, found);
                return;
            }
            found(name.as_str(), args.len(), *span, tail);
            for a in args {
                walk(a, false, found);
            }
            return;
        }
    }
    crate::for_each_child(e, |c| walk(c, false, found));
}
