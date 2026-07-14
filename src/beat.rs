//! Which self-recursive groups are beat loops — heartbeat rung 2's analysis.
//!
//! A beat loop may rewind the arena to its entry mark between iterations
//! (`k_beat_iter` before each self-tail-call), because the analysis proves the
//! only values crossing an iteration boundary are *entry-threaded* (the very
//! value that arrived at entry, below the mark) or *non-heap scalars* (live in
//! registers, no storage to free). Everything else an iteration allocated is
//! garbage the moment the next iteration begins — the sed experiment's insight,
//! emitted by the compiler instead of by hand.
//!
//! Soundness runs one direction: anything the analysis cannot see is
//! ineligible, and an ineligible loop simply keeps today's grow-only arena.
//!
//! A group `(name, arity)` is a beat loop iff:
//!
//! 1. some arm ends in a self-tail-call (it is a loop);
//! 2. every self-tail-call argument is either
//!    a) a bare own-parameter (top-level `Var` pattern) whose group set —
//!    failures aside, since the dispatcher propagates those before any arm
//!    body (and so before any boundary) runs — is int/float/bool, string, or
//!    bytes: immutable payloads with no lazily-allocated internals (maps
//!    memoize a sorted view *above* the mark into a header *below* it; lists
//!    grow their shared buffer; both stay ineligible), or
//!    b) any expression, when the callee's parameter set at that position is —
//!    failures aside — pure non-heap scalars;
//! 3. no other group tail-calls it (every outside entry is a plain call, so
//!    codegen brackets each one with `k_beat_push`/`k_beat_pop`); and
//! 4. its name is never used as a function value (an `Ident` outside call-head
//!    position) — a value call would be an unbracketed entry.

use crate::ast::{Expr, Pattern, Program, Stmt, TemplatePart};
use crate::infer::{self, Set, BOOL, BYTES, FAIL, FLOAT, INT, STR};
use std::collections::HashSet;

const SCALAR: Set = INT | FLOAT | BOOL;
const THREADED: Set = SCALAR | STR | BYTES;

pub fn beat_loops(program: &Program, inference: &infer::Inference) -> HashSet<(String, usize)> {
    let mut out = HashSet::new();
    let mut groups: HashSet<(String, usize)> = HashSet::new();
    for d in &program.fns {
        groups.insert((d.name.clone(), d.params.len()));
    }
    'group: for (name, arity) in &groups {
        let mut has_self_tail = false;
        for (di, decl) in program.fns.iter().enumerate() {
            let in_group = decl.name == *name && decl.params.len() == *arity;
            for tail in tail_exprs(decl.body.last()) {
                let Expr::App { head, args, piped: false, .. } = tail else { continue };
                let Expr::Ident(callee, _) = head.as_ref() else { continue };
                if callee != name || args.len() != *arity {
                    continue;
                }
                if !in_group {
                    // an outside group tail-calls this one: entry without a push
                    continue 'group;
                }
                has_self_tail = true;
                for (i, arg) in args.iter().enumerate() {
                    if !arg_ok(program, inference, decl, di, name, *arity, i, arg) {
                        continue 'group;
                    }
                }
            }
        }
        if !has_self_tail {
            continue;
        }
        if used_as_value(program, name) {
            continue;
        }
        // Profitability: rewinding is worth the push/iter/pop bracketing only
        // when an iteration actually allocates. A pure byte scanner (compare,
        // add, recurse through pure helpers) is eligible but pointless —
        // bracketing it would tax the hottest loops for nothing.
        if !alloc_groups(program).contains(name.as_str()) {
            continue;
        }
        out.insert((name.clone(), *arity));
    }
    out
}

/// Names of groups whose evaluation may allocate, transitively: seeded by
/// arms containing a primitive allocation, propagated across calls to a least
/// fixpoint. Purity through helpers is thus visible — a scanner that only
/// compares, adds, and recurses through pure predicates stays out.
fn alloc_groups(program: &Program) -> HashSet<&str> {
    let mut allocating: HashSet<&str> = HashSet::new();
    for d in &program.fns {
        if d.body.iter().any(|s| stmt_allocates(s, &allocating, true)) {
            allocating.insert(d.name.as_str());
        }
    }
    loop {
        let mut changed = false;
        for d in &program.fns {
            if !allocating.contains(d.name.as_str())
                && d.body.iter().any(|s| stmt_allocates(s, &allocating, false))
            {
                allocating.insert(d.name.as_str());
                changed = true;
            }
        }
        if !changed {
            return allocating;
        }
    }
}

fn stmt_allocates(stmt: &Stmt, allocating: &HashSet<&str>, seed_pass: bool) -> bool {
    let e = match stmt {
        Stmt::Bind { expr, .. } => expr,
        Stmt::Expr(e) => e,
    };
    expr_allocates(e, allocating, seed_pass)
}

/// Does evaluating `e` allocate? On the seed pass only primitive allocations
/// count (builders, interpolation, allocating builtins, constructors — any
/// call that is neither a known-pure builtin nor a user group). On fixpoint
/// passes a call to an already-allocating group counts too.
fn expr_allocates(e: &Expr, allocating: &HashSet<&str>, seed_pass: bool) -> bool {
    const ALLOCATING: &[&str] = &[
        "chars", "concat", "entries", "err", "filter", "from_code", "join", "map",
        "push", "put", "slice", "sort", "utf8",
    ];
    const PURE: &[&str] = &[
        "at", "bytes", "char_code", "find2", "if", "length", "sum", "to_float",
        "to_int", "print",
    ];
    match e {
        Expr::List(..) | Expr::MapLit(..) | Expr::Lambda { .. } => true,
        Expr::Str(parts, _) => parts.iter().any(|p| matches!(p, TemplatePart::Interp(_))),
        Expr::App { head, args, .. } => {
            let head_allocates = match head.as_ref() {
                Expr::Ident(n, _) => {
                    if ALLOCATING.contains(&n.as_str()) {
                        true
                    } else if PURE.contains(&n.as_str()) {
                        false
                    } else if seed_pass {
                        false
                    } else {
                        allocating.contains(n.as_str())
                    }
                }
                other => expr_allocates(other, allocating, seed_pass),
            };
            head_allocates || args.iter().any(|a| expr_allocates(a, allocating, seed_pass))
        }
        Expr::Index { base, index, .. } => {
            expr_allocates(base, allocating, seed_pass)
                || expr_allocates(index, allocating, seed_pass)
        }
        Expr::BinOp { lhs, rhs, .. } => {
            expr_allocates(lhs, allocating, seed_pass)
                || expr_allocates(rhs, allocating, seed_pass)
        }
        Expr::Seq(a, b, _) => {
            expr_allocates(a, allocating, seed_pass)
                || expr_allocates(b, allocating, seed_pass)
        }
        Expr::Ident(..) | Expr::Int(..) | Expr::Float(..) => false,
    }
}

/// The tail expressions of an arm body: the final statement's expression,
/// with lazy `if` expanding into both branches — mirroring `emit_tail`, which
/// emits `musttail` exactly there. Piped applications are not tail calls.
fn tail_exprs(last: Option<&Stmt>) -> Vec<&Expr> {
    let Some(Stmt::Expr(e)) = last else { return Vec::new() };
    let mut out = Vec::new();
    expand_tail(e, &mut out);
    out
}

fn expand_tail<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
    if let Expr::App { head, args, piped: false, .. } = e {
        if matches!(head.as_ref(), Expr::Ident(n, _) if n == "if") && args.len() == 3 {
            expand_tail(&args[1], out);
            expand_tail(&args[2], out);
            return;
        }
    }
    out.push(e);
}

/// May `arg` cross an iteration boundary? Either an entry-threaded bare
/// parameter of an immutable-payload set, or a value the callee's parameter
/// set proves is a non-heap scalar (failures never reach a boundary: the
/// dispatcher propagates them before any arm body runs).
#[allow(clippy::too_many_arguments)]
fn arg_ok(
    program: &Program,
    inference: &infer::Inference,
    decl: &crate::ast::FnDecl,
    decl_index: usize,
    name: &str,
    arity: usize,
    position: usize,
    arg: &Expr,
) -> bool {
    if let Expr::Ident(p, _) = arg {
        let own = decl
            .params
            .iter()
            .position(|pat| matches!(pat, Pattern::Var(n, _) if n == p));
        if let Some(j) = own {
            let set = inference.params[decl_index][j];
            if set & !FAIL & !THREADED == 0 {
                return true;
            }
        }
    }
    let callee_set = group_param_set(program, inference, name, arity, position);
    callee_set & !FAIL & !SCALAR == 0
}

fn group_param_set(
    program: &Program,
    inference: &infer::Inference,
    name: &str,
    arity: usize,
    position: usize,
) -> Set {
    program
        .fns
        .iter()
        .enumerate()
        .filter(|(_, d)| d.name == name && d.params.len() == arity)
        .fold(0, |acc, (i, _)| acc | inference.params[i][position])
}

/// Does `name` appear as a function value — an identifier outside call-head
/// position — anywhere in the program?
fn used_as_value(program: &Program, name: &str) -> bool {
    program.fns.iter().any(|d| {
        d.body.iter().any(|stmt| {
            let e = match stmt {
                Stmt::Bind { expr, .. } => expr,
                Stmt::Expr(e) => e,
            };
            value_use(e, name)
        })
    })
}

fn value_use(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident(n, _) => n == name,
        Expr::App { head, args, .. } => {
            let head_is_plain_name = matches!(head.as_ref(), Expr::Ident(..));
            (!head_is_plain_name && value_use(head, name))
                || args.iter().any(|a| value_use(a, name))
        }
        Expr::Index { base, index, .. } => value_use(base, name) || value_use(index, name),
        Expr::BinOp { lhs, rhs, .. } => value_use(lhs, name) || value_use(rhs, name),
        Expr::Seq(a, b, _) => value_use(a, name) || value_use(b, name),
        Expr::Lambda { body, .. } => value_use(body, name),
        Expr::List(items, _) => items.iter().any(|i| value_use(i, name)),
        Expr::MapLit(pairs, _) => pairs.iter().any(|(k, v)| value_use(k, name) || value_use(v, name)),
        Expr::Str(parts, _) => parts.iter().any(|p| match p {
            TemplatePart::Interp(inner) => value_use(inner, name),
            TemplatePart::Lit(_) => false,
        }),
        Expr::Int(..) | Expr::Float(..) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::beat_loops;
    use crate::infer;

    fn loops_of(src: &str) -> std::collections::HashSet<(String, usize)> {
        let program = crate::compile("test.kso", src, true).unwrap();
        let inference = infer::infer(&program);
        beat_loops(&program, &inference)
    }

    #[test]
    fn scalar_and_threaded_loop_is_eligible() {
        // the jsonbench shape: a threaded string, a counter, an accumulator
        let src = "fn crunch _ 0 acc\n  acc\n\nfn crunch cs n acc\n  crunch cs (n - 1) (acc + length \"beat {n}\")\n\nmain =\n  s = \"abc\"\n  print \"{crunch s 3 0}\"\n";
        assert!(loops_of(src).contains(&("crunch".to_string(), 3)));
    }

    #[test]
    fn list_accumulator_loop_is_not_eligible() {
        // the accumulator's storage is rebuilt every iteration — a rewind
        // would free the list the next iteration receives.
        let src = "fn collect _ 0 acc\n  acc\n\nfn collect cs n acc\n  collect cs (n - 1) (push acc n)\n\nmain =\n  s = \"x\"\n  print \"{length (collect s 3 [])}\"\n";
        assert!(loops_of(src).is_empty());
    }

    #[test]
    fn tail_called_from_outside_is_not_eligible() {
        // go's tail call enters the loop without a push; the loop's iter
        // would rewind to some unrelated outer mark.
        let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        assert!(!loops_of(src).contains(&("spin".to_string(), 2)));
    }

    #[test]
    fn non_tail_outside_call_is_fine() {
        let src = "main = print \"{1 + spin 3 0}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        assert!(loops_of(src).contains(&("spin".to_string(), 2)));
    }

    #[test]
    fn json_decode_loops_stay_conservative() {
        // kanso-json's scanners are mutually recursive and thread record
        // accumulators — v1 must leave all of them alone.
        let program =
            crate::compile_module(std::path::Path::new("lib/json"), false).unwrap();
        let inference = infer::infer(&program);
        let loops = beat_loops(&program, &inference);
        assert!(
            loops.is_empty(),
            "json's folds thread lists/maps and must stay ineligible, got {loops:?}"
        );
    }
}
