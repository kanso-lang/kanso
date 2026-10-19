//! Where a stack overflow is reported (#1991, under the 2026-10-17 gavel on
//! #1987).
//!
//! A recursion deeper than the stack stops the program on every engine, at a
//! depth that differs between them: the interpreter counts calls and the
//! compiled program runs out of machine stack. So the report names a place no
//! depth decides: the outermost cycle still on the stack, by the first call
//! written inside it, to a member of it, outside tail position. A recursion
//! entered inside another one leaves the outer one named on every engine,
//! whichever runs out first.
//!
//! Each engine keeps one record. A call that enters a cycle that grows the
//! stack from outside it saves the record, sets it to the cycle's place when
//! it is empty, and puts it back when the call returns, so a cycle entered
//! inside another never replaces it and a cycle that has returned leaves
//! nothing behind. A call from inside the cycle records nothing, so a
//! recursion's steps cost nothing. A call that would enter in tail position
//! is made as an ordinary call instead, which keeps its frame for the put-back:
//! the caller is not on the cycle, so it adds one frame, not one a step. A
//! group called as a value records the same way wherever it is called from.

use crate::ast::{Expr, FnDecl, Program, Stmt};
use crate::diag::Span;
use crate::hash::Map;

/// The cycles that grow the stack, and the place an overflow in each names.
pub struct Deep {
    /// Each group on such a cycle, by name, with its arity and its cycle.
    /// Keyed by the name alone so a call site asks with the name it holds.
    cycle_of: Map<String, Vec<(usize, usize)>>,
    /// Each cycle's place: the declaration the call was written in, and the
    /// call's span.
    places: Vec<(usize, Span)>,
}

impl Deep {
    pub fn of(program: &Program) -> Deep {
        let mut groups: Map<(&str, usize), usize> = Map::default();
        let mut group_of = Vec::with_capacity(program.fns.len());
        let mut names: Vec<(&str, usize)> = Vec::new();
        for d in &program.fns {
            let key = (d.name.as_str(), d.params.len());
            let next = groups.len();
            let g = *groups.entry(key).or_insert_with(|| {
                names.push(key);
                next
            });
            group_of.push(g);
        }
        // every call by name to a group: (calling declaration, callee group,
        // span, whether it is in tail position), in program order
        let mut calls: Vec<(usize, usize, Span, bool)> = Vec::new();
        for (i, d) in program.fns.iter().enumerate() {
            walk_body(&d.body, true, &mut |callee, arity, span, tail| {
                if let Some(&g) = groups.get(&(callee, arity)) {
                    calls.push((i, g, span, tail));
                }
            });
        }
        let mut adj = vec![Vec::new(); groups.len()];
        for &(from, to, _, _) in &calls {
            adj[group_of[from]].push(to);
        }
        let mut cycle = vec![usize::MAX; groups.len()];
        for (id, scc) in crate::beat::sccs_of(&adj).into_iter().enumerate() {
            if scc.len() > 1 || adj[scc[0]].contains(&scc[0]) {
                for g in scc {
                    cycle[g] = id;
                }
            }
        }
        // a cycle grows the stack when a call inside it, to a member of it,
        // is not in tail position; the first such call is its place
        let mut place: Map<usize, (usize, Span)> = Map::default();
        for &(from, to, span, tail) in &calls {
            let c = cycle[group_of[from]];
            if !tail && c != usize::MAX && cycle[to] == c {
                place.entry(c).or_insert((from, span));
            }
        }
        let mut places = Vec::new();
        let mut index: Map<usize, usize> = Map::default();
        let mut cycle_of: Map<String, Vec<(usize, usize)>> = Map::default();
        for (g, &(name, arity)) in names.iter().enumerate() {
            if let Some(&p) = place.get(&cycle[g]) {
                let id = *index.entry(cycle[g]).or_insert_with(|| {
                    places.push(p);
                    places.len() - 1
                });
                cycle_of.entry(name.to_string()).or_default().push((arity, id));
            }
        }
        Deep { cycle_of, places }
    }

    /// The place a call from `caller` to `callee` records: `callee`'s cycle's,
    /// when `callee` is on a cycle that grows the stack and `caller` is not on
    /// that cycle. `caller` is `None` for a call through a value, which
    /// records wherever it comes from.
    pub fn entering(&self, caller: Option<(&str, usize)>, callee: (&str, usize)) -> Option<(usize, Span)> {
        let to = self.cycle(callee)?;
        let from = caller.and_then(|c| self.cycle(c));
        match from == Some(to) {
            true => None,
            false => Some(self.places[to]),
        }
    }

    /// For each declaration in `program.fns`, one more than the number of
    /// its cycle, or 0 when it is on none that grows the stack. The
    /// interpreter asks by declaration, so a call costs no lookup by name.
    pub fn by_decl(&self, program: &Program) -> Vec<u32> {
        program
            .fns
            .iter()
            .map(|d| self.cycle((d.name.as_str(), d.params.len())).map_or(0, |c| c as u32 + 1))
            .collect()
    }

    /// The place of every cycle, by number.
    pub fn places(&self) -> &[(usize, Span)] {
        &self.places
    }

    fn cycle(&self, (name, arity): (&str, usize)) -> Option<usize> {
        self.cycle_of.get(name)?.iter().find(|(a, _)| *a == arity).map(|&(_, c)| c)
    }
}

/// The place as an err's origin spells it, "{fn} at {file}:{line}".
pub fn spelled(program: &Program, (decl, span): (usize, Span)) -> String {
    let d: &FnDecl = &program.fns[decl];
    format!("{} at {}:{}", crate::ast::frame_name(&d.name), d.file, span.line)
}

/// Every call by name in a body, with whether it is in tail position: the
/// body's last statement is, when the body is.
fn walk_body<'a>(body: &'a [Stmt], tail: bool, found: &mut impl FnMut(&'a str, usize, Span, bool)) {
    let last = body.len().saturating_sub(1);
    for (k, stmt) in body.iter().enumerate() {
        let e = match stmt {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
        };
        walk(e, tail && k == last && matches!(stmt, Stmt::Expr(_)), found);
    }
}

/// Every call by name under `e`, with whether it is in tail position, read
/// the way the emitter's `emit_tail` reads it: a branch of an `if`, a
/// guard's early answer and the body after it, and the body of a one-name
/// lambda a value is piped into all keep tail position. A call of an
/// insisting form never does, since it is called so that its answer can be
/// pointed at the line. A lambda's body anywhere else runs somewhere else,
/// so nothing in it is.
fn walk<'a>(e: &'a Expr, tail: bool, found: &mut impl FnMut(&'a str, usize, Span, bool)) {
    match e {
        Expr::Guard { cond, early, rest, .. } => {
            walk(cond, false, found);
            walk(early, tail, found);
            walk_body(rest, tail, found);
        }
        Expr::App { head, args, piped: true, .. } if args.len() == 1 => match head.as_ref() {
            Expr::Lambda { params, body, .. } if params.len() == 1 => {
                walk(&args[0], false, found);
                walk(body, tail, found);
            }
            _ => crate::for_each_child(e, |c| walk(c, false, found)),
        },
        Expr::App { head, args, piped: false, span } => match head.as_ref() {
            Expr::Ident(name, _, _) if name.as_str() == "if" && args.len() == 3 => {
                walk(&args[0], false, found);
                walk(&args[1], tail, found);
                walk(&args[2], tail, found);
            }
            Expr::Ident(name, _, _) => {
                found(name.as_str(), args.len(), *span, tail && !name.ends_with('!'));
                for a in args {
                    walk(a, false, found);
                }
            }
            _ => crate::for_each_child(e, |c| walk(c, false, found)),
        },
        _ => crate::for_each_child(e, |c| walk(c, false, found)),
    }
}
