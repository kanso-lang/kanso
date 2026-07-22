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
use crate::infer::{self, Set, BOOL, BYTES, DESC, FAIL, FLOAT, FN, INT, LIST, REC, STR};
use std::collections::{HashMap, HashSet};

/// A function group: its name and arity.
pub type Group = (String, usize);

/// A cluster's members plus each member's carried argument positions.
type ClusterCarry = (Vec<Group>, HashMap<Group, Vec<usize>>);

const SCALAR: Set = INT | FLOAT | BOOL;
/// Sets an entry-threaded bare parameter may carry across a rewind. A value
/// that arrived at entry lives wholly below the mark — transitively, since
/// purity means a value never contains pointers to anything newer than
/// itself — so the rewind cannot touch it, provided nothing ever writes an
/// above-the-mark pointer into it afterward.
///
/// Closures, records, and descriptions qualify outright: the runtime writes
/// them only at construction. Strings and bytes are immutable payloads.
///
/// Lists qualify by a narrower argument. Pushing onto a below-mark list
/// writes only an integer (the shared buffer's used count) and an element
/// into below-mark spare capacity; the threaded header itself is never
/// mutated, and a pushed above-mark element is unreachable after the rewind
/// because only above-mark headers had a length covering its slot. The one
/// mutation that could re-point a below-mark header above the mark is the
/// in-place push (k_b_push_mut reallocates the buffer on growth), and it
/// cannot meet a threaded parameter inside the loop: a threaded slot accepts
/// only a bare parameter handed onward every iteration, so within a looping
/// arm the value has a second use and is never linear, and a push result is
/// an expression, which a threaded slot rejects. An exit arm may push
/// in-place — and no rewind follows it, because k_beat_pop keeps the region
/// alive for a heap result. The adversarial tests below pin each case.
///
/// Maps stay out: the first read caches a freshly allocated sorted view —
/// an above-the-mark pointer — into the below-mark header. Instant dangle.
const THREADED: Set = SCALAR | STR | BYTES | FN | REC | DESC | LIST;

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
    /// The loop rewinds, and the named positions are evacuated through the
    /// carry buffers each iteration — the fold accumulator's path.
    CarryBeat { positions: Vec<usize> },
    /// Another group tail-calls it: an entry the bracketing cannot see.
    OutsideTailCall,
    /// Its name is used as a function value: an unbracketed entry.
    UsedAsValue,
}

/// The analysis result codegen consumes: every beat group mapped to its
/// cluster id (a self-loop is a cluster of one), plus the tail-entry edges
/// that must be emitted as plain calls so the loop they enter can bracket.
pub struct Beats {
    pub ids: HashMap<Group, usize>,
    pub demoted: HashSet<(Group, Group)>,
    /// Carry-beat groups: the self-tail argument positions evacuated through
    /// the carry buffers at each rewind.
    pub carried: HashMap<Group, Vec<usize>>,
}

impl Beats {
    pub fn same_cluster(&self, a: &Group, b: &Group) -> bool {
        match (self.ids.get(a), self.ids.get(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }
}

pub fn beat_loops(program: &Program, inference: &infer::Inference) -> Beats {
    let mut ids = HashMap::new();
    let mut carried = HashMap::new();
    let mut next = 0;
    for (name, arity, v) in classify_all(program, inference) {
        if v == Verdict::Beat {
            ids.insert((name, arity), next);
            next += 1;
        }
    }
    for (cluster, cluster_carried) in eligible_clusters(program, inference) {
        for member in cluster {
            ids.insert(member, next);
        }
        next += 1;
        for (group, positions) in cluster_carried {
            carried.insert(group, positions);
        }
    }
    let mut demoted = HashSet::new();
    for (callee, callers, positions) in demotable_entries(program, inference) {
        ids.insert(callee.clone(), next);
        next += 1;
        for caller in callers {
            demoted.insert((caller, callee.clone()));
        }
        if !positions.is_empty() {
            carried.insert(callee, positions);
        }
    }
    for (name, arity, v) in classify_all(program, inference) {
        if let Verdict::CarryBeat { positions } = v {
            let g = (name, arity);
            if !ids.contains_key(&g) {
                ids.insert(g.clone(), next);
                next += 1;
                carried.insert(g, positions);
            }
        }
    }
    // Imported library functions stay beat-ineligible: a shared driver like
    // list/fold_at threads its caller's invariant source through the loop,
    // and carrying it would copy per iteration. Beats belong to the
    // user-code loops that own their data. Provenance is the file stamp —
    // bare-enrolled clones of imported decls are still imported code.
    let imported: std::collections::HashSet<&str> = program
        .fns
        .iter()
        .filter(|d| d.synthetic || d.file.starts_with("std/") || d.name.contains('/'))
        .map(|d| d.name.as_str())
        .collect();
    ids.retain(|(name, _), _| !imported.contains(name.as_str()));
    carried.retain(|(name, _), _| !imported.contains(name.as_str()));
    demoted.retain(|(caller, _)| !imported.contains(caller.0.as_str()));
    Beats { ids, demoted, carried }
}

/// Self-loops whose only defect is a tail entry, where every entering group
/// is acyclic in the tail-call graph. Demoting those entries to plain calls
/// costs each caller one bounded stack frame and lets the loop bracket.
fn demotable_entries(
    program: &Program,
    inference: &infer::Inference,
) -> Vec<(Group, Vec<Group>, Vec<usize>)> {
    let allocating = alloc_groups(program);
    let mut cyclic: HashSet<Group> = HashSet::new();
    // a group is cyclic when any tail path returns to it (self-edge or SCC)
    let mut tail_edges: Vec<(Group, Group)> = Vec::new();
    for decl in &program.fns {
        let from = (decl.name.clone(), decl.params.len());
        for tail in tail_exprs(decl.body.last()) {
            let Expr::App { head, args, piped: false, .. } = tail else { continue };
            let Expr::Ident(callee, _) = head.as_ref() else { continue };
            let to = (callee.clone(), args.len());
            if from == to {
                cyclic.insert(from.clone());
            }
            tail_edges.push((from.clone(), to));
        }
    }
    for cluster in tail_cycles(&tail_edges) {
        cyclic.extend(cluster);
    }
    let mut out = Vec::new();
    for (name, arity, v) in classify_all(program, inference) {
        if v != Verdict::OutsideTailCall {
            continue;
        }
        let group = (name.clone(), arity);
        // beat-worthy apart from the entry? crossing args become carried
        let crossing = crossing_positions(program, inference, &name, arity);
        if crossing.len() > K_CARRY_MAX
            || crossing
                .iter()
                .any(|&p| accumulator_grows(program, &name, arity, p))
            || used_as_value(program, &name)
            || !allocating.contains(name.as_str())
        {
            continue;
        }
        let callers: HashSet<Group> = tail_edges
            .iter()
            .filter(|(from, to)| *to == group && *from != group)
            .map(|(from, _)| from.clone())
            .collect();
        if !callers.is_empty() && callers.iter().all(|c| !cyclic.contains(c)) {
            let mut list: Vec<_> = callers.into_iter().collect();
            list.sort();
            out.push((group, list, crossing));
        }
    }
    out.sort();
    out
}

/// Mirrors the runtime's K_CARRY_MAX: how many crossing positions a carry
/// beat may evacuate per iteration.
const K_CARRY_MAX: usize = 8;

/// A carried position whose next value extends its own previous value —
/// `push acc x`, `concat acc more`, `put acc k v` feeding the same slot —
/// grows with the iteration count, and copying it every rewind costs
/// quadratic bytes where grow-only costs linear. Those accumulators stay on
/// the grow-only path. Growth hidden behind a closure call is not detected;
/// the cost-bound frontier owns that case.
fn accumulator_grows(program: &Program, name: &str, arity: usize, position: usize) -> bool {
    const EXTENDING: [&str; 3] = ["concat", "push", "put"];
    for decl in program.fns.iter() {
        if decl.name != name || decl.params.len() != arity {
            continue;
        }
        let own = decl.params.get(position).and_then(|p| match p {
            Pattern::Var(n, _) => Some(n.as_str()),
            _ => None,
        });
        for tail in tail_exprs(decl.body.last()) {
            let Expr::App { head, args, piped: false, .. } = tail else { continue };
            let Expr::Ident(callee, _) = head.as_ref() else { continue };
            if callee != name || args.len() != arity {
                continue;
            }
            if let Expr::App { head: ah, args: aargs, .. } = &args[position] {
                if let Expr::Ident(op, _) = ah.as_ref() {
                    let extends_self = EXTENDING.contains(&op.as_str())
                        && aargs.first().is_some_and(|a| {
                            matches!(a, Expr::Ident(n, _) if Some(n.as_str()) == own)
                        });
                    if extends_self {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// The self-tail argument positions the boundary rule rejects — the ones a
/// carry beat must evacuate. Sorted and deduplicated.
fn crossing_positions(
    program: &Program,
    inference: &infer::Inference,
    name: &str,
    arity: usize,
) -> Vec<usize> {
    let mut out = Vec::new();
    for (di, decl) in program.fns.iter().enumerate() {
        if decl.name != name || decl.params.len() != arity {
            continue;
        }
        for tail in tail_exprs(decl.body.last()) {
            let Expr::App { head, args, piped: false, .. } = tail else { continue };
            let Expr::Ident(callee, _) = head.as_ref() else { continue };
            if callee != name || args.len() != arity {
                continue;
            }
            for (i, arg) in args.iter().enumerate() {
                if !arg_ok(program, inference, decl, di, name, arity, i, arg) && !out.contains(&i) {
                    out.push(i);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// Groups belonging to any multi-group tail cycle.
fn tail_cycles(
    edges: &[(Group, Group)],
) -> Vec<Vec<Group>> {
    let nodes: Vec<Group> = {
        let mut set = HashSet::new();
        for (a, b) in edges {
            set.insert(a.clone());
            set.insert(b.clone());
        }
        let mut v: Vec<_> = set.into_iter().collect();
        v.sort();
        v
    };
    let index: HashMap<&Group, usize> =
        nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();
    let mut adj = vec![Vec::new(); nodes.len()];
    for (a, b) in edges {
        adj[index[a]].push(index[b]);
    }
    sccs_of(&adj)
        .into_iter()
        .filter(|scc| scc.len() >= 2)
        .map(|scc| scc.into_iter().map(|i| nodes[i].clone()).collect())
        .collect()
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
) -> Vec<ClusterCarry> {
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
        if let Some(carried) =
            cluster_edges_ok(program, inference, &groups, &members, &edges)
        {
            out.push((scc.iter().map(|&g| groups[g].clone()).collect(), carried));
        }
    }
    out
}

/// The threaded-slot fixpoint plus the per-edge argument check. Crossing
/// slots become carried; growth in a carried slot disqualifies the cluster.
fn cluster_edges_ok(
    program: &Program,
    inference: &infer::Inference,
    groups: &[(String, usize)],
    members: &HashSet<usize>,
    edges: &[(usize, usize, usize, &Vec<Expr>)],
) -> Option<HashMap<Group, Vec<usize>>> {
    let inner: Vec<&(usize, usize, usize, &Vec<Expr>)> = edges
        .iter()
        .filter(|(from, to, _, _)| members.contains(from) && members.contains(to))
        .collect();
    let slot_set = |g: usize, i: usize| {
        let (name, arity) = &groups[g];
        group_param_set(program, inference, name, *arity, i)
    };
    // start: every slot whose values are all immutable-payload is a candidate
    // an EMPTY slot set means inference saw no direct call site (the group
    // is only ever entered through a lambda) — unknown, never assumed safe
    let mut threaded: HashSet<(usize, usize)> = HashSet::new();
    for &g in members {
        for i in 0..groups[g].1 {
            let s = slot_set(g, i);
            if s != 0 && s & !FAIL & !THREADED == 0 {
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
    // every remaining edge argument becomes a carried slot on its callee;
    // growth in a carried slot disqualifies the cluster
    let mut carried: HashMap<Group, Vec<usize>> = HashMap::new();
    for (_, to, di, args) in &inner {
        let decl = &program.fns[*di];
        for (i, arg) in args.iter().enumerate() {
            let s = slot_set(*to, i);
            if (s != 0 && s & !FAIL & !SCALAR == 0) || threaded.contains(&(*to, i)) {
                continue;
            }
            if let Expr::App { head: ah, args: aargs, .. } = arg {
                if let Expr::Ident(op, _) = ah.as_ref() {
                    let own = decl.params.get(i).and_then(|p| match p {
                        Pattern::Var(n, _) => Some(n.as_str()),
                        _ => None,
                    });
                    let extends_self = ["concat", "push", "put"].contains(&op.as_str())
                        && aargs.first().is_some_and(|a| {
                            matches!(a, Expr::Ident(n, _) if Some(n.as_str()) == own)
                        });
                    if extends_self {
                        return None;
                    }
                }
            }
            let slots = carried.entry(groups[*to].clone()).or_default();
            if !slots.contains(&i) {
                slots.push(i);
            }
        }
    }
    for slots in carried.values_mut() {
        slots.sort_unstable();
        if slots.len() > K_CARRY_MAX {
            return None;
        }
    }
    Some(carried)
}

/// Strongly connected components of the tail-call graph, returned only for
/// real cycles of two or more groups.
fn tail_sccs(n: usize, edges: &[(usize, usize, usize, &Vec<Expr>)]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    for &(from, to, _, _) in edges {
        adj[from].push(to);
    }
    let mut out = sccs_of(&adj);
    out.retain(|scc| scc.len() >= 2);
    out
}

/// Iterative Tarjan over an adjacency list.
fn sccs_of(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
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
    out
}

/// Every self-recursive group's verdict, one line each, sorted — printed by
/// the toolchain under KANSO_BEAT_REPORT so a real workload can be measured
/// before the next rung is built.
pub fn report(program: &Program, inference: &infer::Inference) -> Vec<String> {
    let demoted: HashSet<Group> = demotable_entries(program, inference)
        .into_iter()
        .map(|(callee, _, _)| callee)
        .collect();
    let clustered: HashSet<Group> = eligible_clusters(program, inference)
        .into_iter()
        .flat_map(|(members, _)| members)
        .collect();
    let mut rows: Vec<(String, usize, Verdict)> = classify_all(program, inference)
        .into_iter()
        .filter(|(name, arity, _)| {
            let g = (name.clone(), *arity);
            !clustered.contains(&g) && !demoted.contains(&g)
        })
        .collect();
    for (name, arity) in clustered.iter().chain(demoted.iter()) {
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
                Verdict::CarryBeat { positions } => {
                    let list: Vec<String> =
                        positions.iter().map(|p| (p + 1).to_string()).collect();
                    format!(
                        "carry beat: rewinds every iteration, evacuating argument {}",
                        list.join(", ")
                    )
                }
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
    for decl in program.fns.iter() {
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
        }
    }
    if !has_self_tail {
        return None;
    }
    if outside_tail {
        return Some(Verdict::OutsideTailCall);
    }
    if used_as_value(program, name) {
        return Some(Verdict::UsedAsValue);
    }
    if !allocating.contains(name) {
        return Some(Verdict::PureLoop);
    }
    let crossing = crossing_positions(program, inference, name, arity);
    if !crossing.is_empty() {
        if let Some(&position) = crossing
            .iter()
            .find(|&&p| accumulator_grows(program, name, arity, p))
        {
            return Some(Verdict::ArgCrosses { position });
        }
        if crossing.len() <= K_CARRY_MAX {
            return Some(Verdict::CarryBeat { positions: crossing });
        }
        return Some(Verdict::ArgCrosses { position: crossing[0] });
    }
    Some(Verdict::Beat)
}

/// Names of groups whose evaluation may allocate, transitively: seeded by
/// arms containing a primitive allocation, propagated across calls to a least
/// fixpoint. Purity through helpers is thus visible — a scanner that only
/// compares, adds, and recurses through pure predicates stays out.
fn alloc_groups(program: &Program) -> HashSet<&str> {
    let fn_names: HashSet<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
    let mut allocating: HashSet<&str> = HashSet::new();
    for d in &program.fns {
        if d.body.iter().any(|s| stmt_allocates(s, &fn_names, &allocating, true)) {
            allocating.insert(d.name.as_str());
        }
    }
    loop {
        let mut changed = false;
        for d in &program.fns {
            if !allocating.contains(d.name.as_str())
                && d.body.iter().any(|s| stmt_allocates(s, &fn_names, &allocating, false))
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

fn stmt_allocates(stmt: &Stmt, fn_names: &HashSet<&str>, allocating: &HashSet<&str>, seed_pass: bool) -> bool {
    let e = match stmt {
        Stmt::Bind { expr, .. } => expr,
        Stmt::Expr(e) => e,
    };
    expr_allocates(e, fn_names, allocating, seed_pass)
}

/// Does evaluating `e` allocate? On the seed pass only primitive allocations
/// count (builders, interpolation, allocating builtins, constructors — any
/// call that is neither a known-pure builtin nor a user group). On fixpoint
/// passes a call to an already-allocating group counts too.
fn expr_allocates(e: &Expr, fn_names: &HashSet<&str>, allocating: &HashSet<&str>, seed_pass: bool) -> bool {
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
                    // a name that is neither a builtin nor a program function
                    // is a closure value: its body is unknowable, so it may
                    // allocate
                    ALLOCATING.contains(&n.as_str())
                        || (!PURE.contains(&n.as_str())
                            && !fn_names.contains(n.as_str())
                            && n != "if")
                        || (!PURE.contains(&n.as_str())
                            && !seed_pass
                            && allocating.contains(n.as_str()))
                }
                other => expr_allocates(other, fn_names, allocating, seed_pass),
            };
            head_allocates || args.iter().any(|a| expr_allocates(a, fn_names, allocating, seed_pass))
        }
        Expr::Field { base, .. } => expr_allocates(base, fn_names, allocating, seed_pass),
        Expr::Index { base, index, .. } => {
            expr_allocates(base, fn_names, allocating, seed_pass)
                || expr_allocates(index, fn_names, allocating, seed_pass)
        }
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            expr_allocates(lhs, fn_names, allocating, seed_pass)
                || expr_allocates(rhs, fn_names, allocating, seed_pass)
        }
        Expr::Seq(a, b, _) => {
            expr_allocates(a, fn_names, allocating, seed_pass)
                || expr_allocates(b, fn_names, allocating, seed_pass)
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
    if let Expr::App { head, args, piped, .. } = e {
        if !piped && matches!(head.as_ref(), Expr::Ident(n, _) if n == "if") && args.len() == 3 {
            expand_tail(&args[1], out);
            expand_tail(&args[2], out);
            return;
        }
        // a tail pipe into a literal lambda inlines (codegen emits the
        // lambda body as the caller's own tail), so its tails are ours
        if *piped && args.len() == 1 {
            if let Expr::Lambda { params, body, .. } = head.as_ref() {
                if params.len() == 1 {
                    expand_tail(body, out);
                    return;
                }
            }
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
        Expr::Field { base, .. } => value_use(base, name),
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

    fn compiled(src: &str) -> (crate::ast::Program, infer::Inference) {
        let program = crate::compile("test.kso", src, true).unwrap();
        let inference = infer::infer(&program);
        (program, inference)
    }

    fn loops_of(src: &str) -> std::collections::HashSet<(String, usize)> {
        // the tests assert membership; the cluster ids are irrelevant here
        let program = crate::compile("test.kso", src, true).unwrap();
        let inference = infer::infer(&program);
        beat_loops(&program, &inference).ids.into_keys().collect()
    }

    #[test]
    fn scalar_and_threaded_loop_is_eligible() {
        // the jsonbench shape: a threaded string, a counter, an accumulator
        let src = "fn crunch _ 0 acc\n  acc\n\nfn crunch cs n acc\n  crunch cs (n - 1) (acc + length \"beat {n}\")\n\nmain =\n  s = \"abc\"\n  print \"{crunch s 3 0}\"\n";
        assert!(loops_of(src).contains(&("crunch".to_string(), 3)));
    }

    #[test]
    fn growing_accumulator_stays_grow_only() {
        // push acc n extends its own previous value: carrying it would copy
        // quadratic bytes where grow-only allocates linear. The gate keeps
        // it off the carry path.
        let src = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (push acc n)\n\nmain = print \"{collect 3 []}\"\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert!(!beats.ids.contains_key(&("collect".to_string(), 2)));
    }

    #[test]
    fn bounded_accumulator_carries() {
        // a fixed-shape rebuild does not grow with the iteration count; the
        // carry evacuates it each rewind.
        let src = "main = print \"{spin 10 [0 1]}\"\n\nfn spin 0 acc\n  length acc\n\nfn spin n acc\n  a = acc[1]\n  b = acc[2]\n  spin (n - 1) [b (a + b)]\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert_eq!(beats.carried.get(&("spin".to_string(), 2)), Some(&vec![1]));
    }#[test]
    fn tail_entry_from_acyclic_caller_is_demoted() {
        // go tail-calls into spin's loop, but go is acyclic: the entry is
        // demoted to a plain call (one bounded frame) and spin brackets.
        let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert!(beats.ids.contains_key(&("spin".to_string(), 2)));
        assert!(beats
            .demoted
            .contains(&(("go".to_string(), 1), ("spin".to_string(), 2))));
    }

    #[test]
    fn closure_threaded_loop_with_demoted_entry_is_a_beat() {
        // f is a closure handed through unchanged: immutable internals,
        // wholly below the entry mark, safe to carry across the rewind.
        let src = "fn go f n\n  spin f n 0\n\nmain =\n  salt = (x -> x * 2)\n  print \"{go salt 5}\"\n\nfn spin f 0 acc\n  f acc\n\nfn spin f n acc\n  step = \"seen {n}\"\n  spin f (n - 1) (acc + length step)\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
    }

    #[test]
    fn list_threaded_loop_is_a_beat() {
        // the list is handed onward unchanged every iteration: below the
        // mark, header never mutated, safe across the rewind.
        let src = "fn go xs n\n  spin xs n 0\n\nmain =\n  base = [10 20 30]\n  print \"{go base 5}\"\n\nfn spin xs 0 acc\n  acc + length xs\n\nfn spin xs n acc\n  garbage = \"iteration {n}\"\n  spin xs (n - 1) (acc + length garbage)\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
    }

    #[test]
    fn map_threaded_loop_carries_the_map() {
        // a map may never thread (its first read caches an above-mark sorted
        // view into the below-mark header), so the carry evacuates it — the
        // copy resets the cache, which keeps the rewind sound.
        let src = "fn go m n\n  spin m n 0\n\nmain =\n  prices = [\"a\": 1 \"b\": 2]\n  print \"{go prices 3}\"\n\nfn spin m 0 acc\n  acc + length m\n\nfn spin m n acc\n  step = \"seen {n}\"\n  spin m (n - 1) (acc + length step)\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert_eq!(beats.carried.get(&("spin".to_string(), 3)), Some(&vec![0]));
    }

    #[test]
    fn tail_entry_from_cyclic_caller_stays_ineligible() {
        // ping and pong form a tail cycle; pong's entry into spin can never
        // be demoted — a plain call inside a musttail cycle would grow the
        // stack without bound.
        let src = "main = print \"{ping 3}\"\n\nfn ping n\n  pong n\n\nfn pong 0\n  spin 2 0\n\nfn pong n\n  ping (n - 1)\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        let (program, inference) = compiled(src);
        let beats = super::beat_loops(&program, &inference);

        assert!(!beats.ids.contains_key(&("spin".to_string(), 2)));
        assert!(beats.demoted.is_empty());
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
            loops.ids.is_empty() && loops.demoted.is_empty(),
            "json's folds thread lists/maps and must stay ineligible, got {:?}", loops.ids
        );
    }
}
