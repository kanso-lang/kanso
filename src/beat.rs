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
use std::collections::{HashMap, HashSet};

const SCALAR: Set = INT | FLOAT | BOOL;
const THREADED: Set = SCALAR | STR | BYTES;

/// One self-recursive group's fate under the analysis. `Beat` is the only
/// verdict codegen acts on; the others exist so `report` can say why a loop
/// keeps the grow-only arena — the data that decides what the survivor
/// machinery is worth on a real program.
#[derive(PartialEq)]
pub enum Verdict {
    /// Rewinds between iterations.
    Beat,
    /// Eligible, but no iteration allocates — bracketing would tax a hot
    /// loop for nothing.
    PureLoop,
    /// Argument `position` may carry a heap value across the iteration
    /// boundary — the case the three-way escape split would reclaim.
    ArgCrosses { position: usize },
    /// Another group tail-calls it: an entry the bracketing cannot see.
    OutsideTailCall,
    /// Its name is used as a function value: an unbracketed entry.
    UsedAsValue,
}

/// Every beat group, mapped to its cluster id. A self-loop is a cluster of
/// one; a mutual-recursion cluster shares one id across its members, and
/// codegen rewinds only on tail calls that stay inside one cluster.
pub fn beat_loops(program: &Program, inference: &infer::Inference) -> HashMap<(String, usize), usize> {
    let mut out = HashMap::new();
    let mut next = 0;
    for (name, arity, v) in classify_all(program, inference) {
        if v == Verdict::Beat {
            out.insert((name, arity), next);
            next += 1;
        }
    }
    for cluster in eligible_clusters(program, inference) {
        for member in cluster {
            out.insert(member, next);
        }
        next += 1;
    }
    out
}

/// Multi-group tail-call cycles that may rewind: every entry from outside is
/// a plain call, no member is used as a value, some member allocates, and at
/// every tail edge inside the cluster each argument is a pure scalar in the
/// callee's slot or a bare parameter threaded hand-to-hand from the cluster's
/// entry. A parameter allocated mid-cycle is not entry-threaded — rewinding
/// would free it under a live register — so threading is a fixpoint: a slot
/// keeps its threaded standing only while every edge feeding it passes a
/// bare parameter from a slot that kept its own.
fn eligible_clusters(
    program: &Program,
    inference: &infer::Inference,
) -> Vec<Vec<(String, usize)>> {
    let groups: Vec<(String, usize)> = {
        let set: HashSet<(String, usize)> = program
            .fns
            .iter()
            .map(|d| (d.name.clone(), d.params.len()))
            .collect();
        let mut v: Vec<_> = set.into_iter().collect();
        v.sort();
        v
    };
    let index: HashMap<&(String, usize), usize> =
        groups.iter().enumerate().map(|(i, g)| (g, i)).collect();
    // tail edges: (caller group, callee group, decl index, args)
    let mut edges: Vec<(usize, usize, usize, &Vec<Expr>)> = Vec::new();
    for (di, decl) in program.fns.iter().enumerate() {
        let from = index[&(decl.name.clone(), decl.params.len())];
        for tail in tail_exprs(decl.body.last()) {
            let Expr::App { head, args, piped: false, .. } = tail else { continue };
            let Expr::Ident(callee, _) = head.as_ref() else { continue };
            if let Some(&to) = index.get(&(callee.clone(), args.len())) {
                edges.push((from, to, di, args));
            }
        }
    }
    let sccs = tail_sccs(groups.len(), &edges);
    let allocating = alloc_groups(program);
    let mut out = Vec::new();
    for scc in sccs {
        if scc.len() < 2 {
            continue; // self-loops stay on the proven path
        }
        let members: HashSet<usize> = scc.iter().copied().collect();
        let tail_entry = edges
            .iter()
            .any(|(from, to, _, _)| members.contains(to) && !members.contains(from));
        if tail_entry {
            continue;
        }
        if scc.iter().any(|&g| used_as_value(program, &groups[g].0)) {
            continue;
        }
        if !scc.iter().any(|&g| allocating.contains(groups[g].0.as_str())) {
            continue;
        }
        if cluster_edges_ok(program, inference, &groups, &members, &edges) {
            out.push(scc.iter().map(|&g| groups[g].clone()).collect());
        }
    }
    out
}

/// The threaded-slot fixpoint plus the per-edge argument check.
fn cluster_edges_ok(
    program: &Program,
    inference: &infer::Inference,
    groups: &[(String, usize)],
    members: &HashSet<usize>,
    edges: &[(usize, usize, usize, &Vec<Expr>)],
) -> bool {
    let inner: Vec<&(usize, usize, usize, &Vec<Expr>)> = edges
        .iter()
        .filter(|(from, to, _, _)| members.contains(from) && members.contains(to))
        .collect();
    let slot_set = |g: usize, i: usize| {
        let (name, arity) = &groups[g];
        group_param_set(program, inference, name, *arity, i)
    };
    // start: every slot whose values are all immutable-payload is a candidate
    let mut threaded: HashSet<(usize, usize)> = HashSet::new();
    for &g in members {
        for i in 0..groups[g].1 {
            if slot_set(g, i) & !FAIL & !THREADED == 0 {
                threaded.insert((g, i));
            }
        }
    }
    // knock out any slot fed by something other than a still-threaded bare param
    loop {
        let mut changed = false;
        for &&(from, to, di, args) in &inner {
            let decl = &program.fns[di];
            for (i, arg) in args.iter().enumerate() {
                if !threaded.contains(&(to, i)) {
                    continue;
                }
                let fed_by_threaded = match arg {
                    Expr::Ident(p, _) => decl
                        .params
                        .iter()
                        .position(|pat| matches!(pat, Pattern::Var(n, _) if n == p))
                        .is_some_and(|j| threaded.contains(&(from, j))),
                    _ => false,
                };
                if !fed_by_threaded && threaded.remove(&(to, i)) {
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // every edge argument lands in a scalar slot or a surviving threaded slot
    inner.iter().all(|(_, to, _, args)| {
        (0..args.len()).all(|i| {
            slot_set(*to, i) & !FAIL & !SCALAR == 0 || threaded.contains(&(*to, i))
        })
    })
}

/// Strongly connected components of the tail-call graph (iterative Tarjan),
/// returned only for real cycles of two or more groups.
fn tail_sccs(n: usize, edges: &[(usize, usize, usize, &Vec<Expr>)]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    for &(from, to, _, _) in edges {
        adj[from].push(to);
    }
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();
    let mut counter = 0;
    let mut out = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        // (node, next child position)
        let mut work = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut ci)) = work.last_mut() {
            if *ci == 0 {
                index[v] = counter;
                low[v] = counter;
                counter += 1;
                stack.push(v);
                on_stack[v] = true;
            }
            if *ci < adj[v].len() {
                let w = adj[v][*ci];
                *ci += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut scc = Vec::new();
                    loop {
                        let w = stack.pop().expect("tarjan stack");
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    out.push(scc);
                }
            }
        }
    }
    out.retain(|scc| scc.len() >= 2);
    out
}

/// Every self-recursive group's verdict, one line each, sorted — printed by
/// the toolchain under KANSO_BEAT_REPORT so a real workload can be measured
/// before the next rung is built.
pub fn report(program: &Program, inference: &infer::Inference) -> Vec<String> {
    let clustered: HashSet<(String, usize)> =
        eligible_clusters(program, inference).into_iter().flatten().collect();
    let mut rows: Vec<(String, usize, Verdict)> = classify_all(program, inference)
        .into_iter()
        .filter(|(name, arity, _)| !clustered.contains(&(name.clone(), *arity)))
        .collect();
    for (name, arity) in &clustered {
        rows.push((name.clone(), *arity, Verdict::Beat));
    }
    rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    rows.iter()
        .map(|(name, arity, v)| {
            let fate = match v {
                Verdict::Beat => "beat: rewinds every iteration".to_string(),
                Verdict::PureLoop => "pure loop: no iteration allocates, nothing to rewind".to_string(),
                Verdict::ArgCrosses { position } => format!(
                    "grow-only: argument {} may carry heap across the iteration",
                    position + 1
                ),
                Verdict::OutsideTailCall => {
                    "grow-only: another group tail-calls it (unbracketed entry)".to_string()
                }
                Verdict::UsedAsValue => {
                    "grow-only: used as a function value (unbracketed entry)".to_string()
                }
            };
            format!("{name}/{arity}: {fate}")
        })
        .collect()
}

fn classify_all(program: &Program, inference: &infer::Inference) -> Vec<(String, usize, Verdict)> {
    let allocating = alloc_groups(program);
    let mut groups: Vec<(String, usize)> = {
        let set: HashSet<(String, usize)> = program
            .fns
            .iter()
            .map(|d| (d.name.clone(), d.params.len()))
            .collect();
        set.into_iter().collect()
    };
    groups.sort();
    groups
        .into_iter()
        .filter_map(|(name, arity)| {
            classify(program, inference, &allocating, &name, arity)
                .map(|v| (name, arity, v))
        })
        .collect()
}

/// The verdict for one group, or None when it has no self-tail-call (not a
/// loop, nothing to say).
fn classify(
    program: &Program,
    inference: &infer::Inference,
    allocating: &HashSet<&str>,
    name: &str,
    arity: usize,
) -> Option<Verdict> {
    let mut has_self_tail = false;
    let mut outside_tail = false;
    let mut arg_crosses: Option<usize> = None;
    for (di, decl) in program.fns.iter().enumerate() {
        let in_group = decl.name == name && decl.params.len() == arity;
        for tail in tail_exprs(decl.body.last()) {
            let Expr::App { head, args, piped: false, .. } = tail else { continue };
            let Expr::Ident(callee, _) = head.as_ref() else { continue };
            if callee != name || args.len() != arity {
                continue;
            }
            if !in_group {
                outside_tail = true;
                continue;
            }
            has_self_tail = true;
            for (i, arg) in args.iter().enumerate() {
                if arg_crosses.is_none()
                    && !arg_ok(program, inference, decl, di, name, arity, i, arg)
                {
                    arg_crosses = Some(i);
                }
            }
        }
    }
    if !has_self_tail {
        return None;
    }
    if outside_tail {
        return Some(Verdict::OutsideTailCall);
    }
    if let Some(position) = arg_crosses {
        return Some(Verdict::ArgCrosses { position });
    }
    if used_as_value(program, name) {
        return Some(Verdict::UsedAsValue);
    }
    if !allocating.contains(name) {
        return Some(Verdict::PureLoop);
    }
    Some(Verdict::Beat)
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
                    ALLOCATING.contains(&n.as_str())
                        || (!PURE.contains(&n.as_str())
                            && !seed_pass
                            && allocating.contains(n.as_str()))
                }
                other => expr_allocates(other, allocating, seed_pass),
            };
            head_allocates || args.iter().any(|a| expr_allocates(a, allocating, seed_pass))
        }
        Expr::Index { base, index, .. } => {
            expr_allocates(base, allocating, seed_pass)
                || expr_allocates(index, allocating, seed_pass)
        }
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
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
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            value_use(lhs, name) || value_use(rhs, name)
        }
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
        // the tests assert membership; the cluster ids are irrelevant here
        let program = crate::compile("test.kso", src, true).unwrap();
        let inference = infer::infer(&program);
        beat_loops(&program, &inference).into_keys().collect()
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
