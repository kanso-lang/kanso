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
use crate::infer::{self, Set, BOOL, BYTES, DESC, FAIL, FLOAT, FN, INT, LIST, MAP, REC, STR};
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

pub fn beat_loops(program: &Program, inference: &infer::Inference, mut_sites: &MutSites) -> Beats {
    let chains = chain_groups(program, mut_sites);
    let mut ids = HashMap::new();
    let mut carried = HashMap::new();
    let mut next = 0;
    for (name, arity, v) in classify_all(program, inference, mut_sites, &chains) {
        if v == Verdict::Beat {
            ids.insert((name, arity), next);
            next += 1;
        }
    }
    for (cluster, cluster_carried) in eligible_clusters(program, inference, mut_sites, &chains) {
        for member in cluster {
            ids.insert(member, next);
        }
        next += 1;
        for (group, positions) in cluster_carried {
            carried.insert(group, positions);
        }
    }
    let mut demoted = HashSet::new();
    for (callee, callers, positions) in demotable_entries(program, inference, mut_sites, &chains) {
        ids.insert(callee.clone(), next);
        next += 1;
        for caller in callers {
            demoted.insert((caller, callee.clone()));
        }
        if !positions.is_empty() {
            carried.insert(callee, positions);
        }
    }
    for (name, arity, v) in classify_all(program, inference, mut_sites, &chains) {
        if let Verdict::CarryBeat { positions } = v {
            let g = (name, arity);
            if !ids.contains_key(&g) {
                ids.insert(g.clone(), next);
                next += 1;
                carried.insert(g, positions);
            }
        }
    }
    // A carried beat evacuates its slots every iteration, and a shared
    // library driver threads its caller's invariant source through the loop
    // — carrying that copies an unbounded value per iteration, so imported
    // groups stay out of the carry tier. A plain beat carries nothing: its
    // rewind is a pointer reset, and a module loop that earned one keeps
    // it. Groups with a synthetic arm stay out of everything — a bare clone
    // is a second spelling the analyses cannot see through.
    let has_synthetic: std::collections::HashSet<&str> =
        program.fns.iter().filter(|d| d.synthetic).map(|d| d.name.as_str()).collect();
    let imported: std::collections::HashSet<&str> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/") || d.file.starts_with("lib/"))
        .map(|d| d.name.as_str())
        .collect();
    ids.retain(|(name, _), _| !has_synthetic.contains(name.as_str()));
    let carried_needed: std::collections::HashSet<(String, usize)> =
        carried.keys().cloned().collect();
    carried.retain(|(name, _), _| {
        !has_synthetic.contains(name.as_str()) && !imported.contains(name.as_str())
    });
    // an id whose carry was just stripped must not stay armed as a carry
    // beat with nothing staged: drop imported ids that needed their carry
    ids.retain(|g, _| !imported.contains(g.0.as_str()) || !carried_needed.contains(g));

    // A demoted pair lives or dies with its target loop, never with the
    // caller's name: a user loop entered through a group that shares its
    // name with a clone still needs the entry demoted, or the loop's
    // rewinds run against a mark nobody pushed.
    demoted.retain(|(_, callee)| ids.contains_key(callee));
    Beats { ids, demoted, carried }
}

/// Self-loops whose only defect is a tail entry, where every entering group
/// is acyclic in the tail-call graph. Demoting those entries to plain calls
/// costs each caller one bounded stack frame and lets the loop bracket.
fn demotable_entries(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
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
    for (name, arity, v) in classify_all(program, inference, mut_sites, chains) {
        if v != Verdict::OutsideTailCall {
            continue;
        }
        let group = (name.clone(), arity);
        // beat-worthy apart from the entry? crossing args become carried
        let crossing = crossing_positions(program, inference, mut_sites, chains, &name, arity);
        if crossing.len() > K_CARRY_MAX
            || crossing.iter().any(|&p| {
                let set = group_param_set(program, inference, &name, arity, p);
                accumulator_grows(program, &name, arity, p) || set == 0 || set & BYTES != 0
            })
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
                        && aargs.first().is_some_and(
                            |a| matches!(a, Expr::Ident(n, _) if Some(n.as_str()) == own),
                        );
                    if extends_self {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Sites where a push/append/put was proven unique, keyed by source
/// position — the linearity analysis's output, threaded in so the chain
/// test below can insist on pointer identity rather than merely on type.
pub type MutSites = std::collections::HashSet<(String, usize, usize)>;

/// Groups whose every arm returns the very object that arrived as its first
/// parameter — pointer identity through mut appends, folds, conditionals and
/// calls to other chaining groups. A greatest fixpoint: assume every group
/// with a named first parameter chains it, then remove any an arm disproves.
///
/// Identity is what makes a bytes accumulator safe to thread across a
/// rewind. The header arrived at the loop's entry, so it lives below the
/// mark; a mut append returns its argument, so the pointer never changes;
/// and growth allocates outside the arena, so the payload is never above the
/// mark either. A fresh value at the same type has none of those properties,
/// which is why the test is identity and not the type set.
fn chain_groups(program: &Program, mut_sites: &MutSites) -> HashSet<Group> {
    let folds = crate::linear::fold_spellings(program);
    let mut chains: HashSet<Group> = program
        .fns
        .iter()
        .filter(|d| !d.params.is_empty())
        .map(|d| (d.name.clone(), d.params.len()))
        .collect();
    loop {
        let mut changed = false;
        let drop: Vec<Group> = chains
            .iter()
            .filter(|(name, arity)| {
                !program.fns.iter().filter(|d| d.name == *name && d.params.len() == *arity).all(
                    |d| match d.params.first() {
                        Some(Pattern::Var(own, _)) => {
                            let locals = local_binds(d);
                            tail_exprs(d.body.last())
                                .iter()
                                .all(|t| is_chain(t, own, d, &locals, mut_sites, &chains, &folds))
                        }
                        _ => false,
                    },
                )
            })
            .cloned()
            .collect();
        for k in drop {
            if std::env::var("KANSO_CHAIN_REPORT").is_ok() {
                eprintln!("chain: dropped {}/{}", k.0, k.1);
            }
            chains.remove(&k);
            changed = true;
        }
        if !changed {
            return chains;
        }
    }
}

/// The single-assignment locals of an arm, so a chain can pass through a
/// named intermediate — a binding is pure naming, which preserves identity.
fn local_binds(decl: &crate::ast::FnDecl) -> HashMap<&str, &Expr> {
    decl.body
        .iter()
        .filter_map(|st| match st {
            Stmt::Bind { pattern: Pattern::Var(n, _), expr } => Some((n.as_str(), expr)),
            _ => None,
        })
        .collect()
}

fn is_chain(
    e: &Expr,
    own: &str,
    decl: &crate::ast::FnDecl,
    locals: &HashMap<&str, &Expr>,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
    folds: &HashSet<String>,
) -> bool {
    match e {
        Expr::Ident(p, _) => {
            p == own
                || locals
                    .get(p.as_str())
                    .is_some_and(|e2| is_chain(e2, own, decl, locals, mut_sites, chains, folds))
        }
        Expr::App { head, args, span, .. } => match head.as_ref() {
            Expr::Ident(n, _)
                if matches!(n.as_str(), "append" | "builtin_append")
                    && args.len() == 2
                    && mut_sites.contains(&(decl.file.clone(), span.line, span.col)) =>
            {
                is_chain(&args[0], own, decl, locals, mut_sites, chains, folds)
            }
            Expr::Ident(n, _) if n == "if" && args.len() == 3 => {
                is_chain(&args[1], own, decl, locals, mut_sites, chains, folds)
                    && is_chain(&args[2], own, decl, locals, mut_sites, chains, folds)
            }
            Expr::Ident(n, _) if folds.contains(n.as_str()) && args.len() == 3 => {
                let folder_chains = match &args[2] {
                    Expr::Lambda { params, body, .. } => params.first().is_some_and(|(p, _)| {
                        is_chain(body, p, decl, locals, mut_sites, chains, folds)
                    }),
                    _ => false,
                };
                folder_chains && is_chain(&args[1], own, decl, locals, mut_sites, chains, folds)
            }
            Expr::Ident(f, _) if chains.contains(&(f.clone(), args.len())) => args
                .first()
                .is_some_and(|a| is_chain(a, own, decl, locals, mut_sites, chains, folds)),
            _ => false,
        },
        Expr::Guard { early, rest, .. } => {
            is_chain(early, own, decl, locals, mut_sites, chains, folds)
                && matches!(rest.last(), Some(Stmt::Expr(t)) if is_chain(t, own, decl, locals, mut_sites, chains, folds))
        }
        _ => false,
    }
}

/// The self-tail argument positions the boundary rule rejects — the ones a
/// carry beat must evacuate. Sorted and deduplicated.
fn crossing_positions(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
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
                if !arg_ok(program, inference, mut_sites, chains, decl, di, name, arity, i, arg)
                    && !out.contains(&i)
                {
                    out.push(i);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// Groups belonging to any multi-group tail cycle.
fn tail_cycles(edges: &[(Group, Group)]) -> Vec<Vec<Group>> {
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
    let index: HashMap<&Group, usize> = nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();
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
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
) -> Vec<ClusterCarry> {
    let groups: Vec<(String, usize)> = {
        let set: HashSet<(String, usize)> =
            program.fns.iter().map(|d| (d.name.clone(), d.params.len())).collect();
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
        let tail_entry =
            edges.iter().any(|(from, to, _, _)| members.contains(to) && !members.contains(from));
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
            cluster_edges_ok(program, inference, mut_sites, chains, &groups, &members, &edges)
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
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
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
    // A bytes slot may also cross by pointer identity: every inner edge
    // feeds it a mut-append chain rooted at one of the caller's own
    // chain-threaded slots, so the value is the accumulator that entered
    // the cluster — header below the entry mark, growth outside the arena,
    // no pointers inside. The same license as a self-tail, read around a
    // cycle: greatest fixpoint, assume every bytes slot qualifies, knock
    // out any an edge disproves.
    let folds = crate::linear::fold_spellings(program);
    let mut chain_threaded: HashSet<(usize, usize)> = HashSet::new();
    for &g in members {
        for i in 0..groups[g].1 {
            let s = slot_set(g, i);
            if s != 0 && s & !FAIL & !BYTES == 0 {
                chain_threaded.insert((g, i));
            }
        }
    }
    loop {
        let mut changed = false;
        for &&(from, to, di, args) in &inner {
            let decl = &program.fns[di];
            let locals = local_binds(decl);
            for (i, arg) in args.iter().enumerate() {
                if !chain_threaded.contains(&(to, i)) {
                    continue;
                }
                let fed_by_chain = decl.params.iter().enumerate().any(|(j, pat)| {
                    let Pattern::Var(own, _) = pat else { return false };
                    chain_threaded.contains(&(from, j))
                        && is_chain(arg, own, decl, &locals, mut_sites, chains, &folds)
                });
                if !fed_by_chain && chain_threaded.remove(&(to, i)) {
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
            if (s != 0 && s & !FAIL & !SCALAR == 0)
                || threaded.contains(&(*to, i))
                || chain_threaded.contains(&(*to, i))
            {
                continue;
            }
            // a byte builder rebuilt each iteration would deep-copy its
            // whole buffer at every rewind: growth wearing a carry — and a
            // slot inference can't type may hide the same shape
            if s & BYTES != 0 || s == 0 {
                return None;
            }
            if let Expr::App { head: ah, args: aargs, .. } = arg {
                if let Expr::Ident(op, _) = ah.as_ref() {
                    let own = decl.params.get(i).and_then(|p| match p {
                        Pattern::Var(n, _) => Some(n.as_str()),
                        _ => None,
                    });
                    let extends_self = ["concat", "push", "put"].contains(&op.as_str())
                        && aargs.first().is_some_and(
                            |a| matches!(a, Expr::Ident(n, _) if Some(n.as_str()) == own),
                        );
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
pub fn report(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
) -> Vec<String> {
    let chains = chain_groups(program, mut_sites);
    let demoted: HashSet<Group> = demotable_entries(program, inference, mut_sites, &chains)
        .into_iter()
        .map(|(callee, _, _)| callee)
        .collect();
    let clustered: HashSet<Group> = eligible_clusters(program, inference, mut_sites, &chains)
        .into_iter()
        .flat_map(|(members, _)| members)
        .collect();
    let mut rows: Vec<(String, usize, Verdict)> =
        classify_all(program, inference, mut_sites, &chains)
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
                Verdict::PureLoop => {
                    "pure loop: no iteration allocates, nothing to rewind".to_string()
                }
                Verdict::ArgCrosses { position } => format!(
                    "grow-only: argument {} may carry heap across the iteration",
                    position + 1
                ),
                Verdict::CarryBeat { positions } => {
                    let list: Vec<String> = positions.iter().map(|p| (p + 1).to_string()).collect();
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
            // classify stops at the first blocker; say what else is waiting,
            // so a fix aimed at one reason is not a surprise when it lands
            let allocating = alloc_groups(program);
            let also: Vec<String> =
                blockers(program, inference, mut_sites, &chains, &allocating, name, *arity)
                    .into_iter()
                    .filter(|b| b != v)
                    .map(|b| match b {
                        Verdict::ArgCrosses { position } => {
                            format!("argument {} also carries heap", position + 1)
                        }
                        Verdict::OutsideTailCall => "also an unbracketed entry".to_string(),
                        Verdict::UsedAsValue => "also used as a function value".to_string(),
                        _ => String::new(),
                    })
                    .filter(|line| !line.is_empty())
                    .collect();
            match also.is_empty() {
                true => format!("{name}/{arity}: {fate}"),
                false => format!("{name}/{arity}: {fate} ({})", also.join("; ")),
            }
        })
        .collect()
}

/// Every reason a group declines, not only the first one found.
///
/// `classify` stops at the first blocker because codegen asks one question —
/// is this `Beat`? — and any other answer means the same thing to it. The
/// report wants the whole picture, because a loop unblocked for one reason
/// may still decline for another, and knowing that before building the fix
/// is the difference between an optimisation that pays and one that does not.
fn blockers(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
    allocating: &HashSet<&str>,
    name: &str,
    arity: usize,
) -> Vec<Verdict> {
    let mut found = Vec::new();
    if outside_tails(program, name, arity) {
        found.push(Verdict::OutsideTailCall);
    }
    if used_as_value(program, name) {
        found.push(Verdict::UsedAsValue);
    }
    // a loop that allocates nothing has nothing the others could cost it
    if !allocating.contains(name) {
        return found;
    }
    let crossing = crossing_positions(program, inference, mut_sites, chains, name, arity);
    if let Some(&position) = crossing.iter().find(|&&p| {
        let set = group_param_set(program, inference, name, arity, p);
        accumulator_grows(program, name, arity, p) || set == 0 || set & BYTES != 0
    }) {
        found.push(Verdict::ArgCrosses { position });
    }
    found
}

fn classify_all(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
) -> Vec<(String, usize, Verdict)> {
    let allocating = alloc_groups(program);
    let mut groups: Vec<(String, usize)> = {
        let set: HashSet<(String, usize)> =
            program.fns.iter().map(|d| (d.name.clone(), d.params.len())).collect();
        set.into_iter().collect()
    };
    groups.sort();
    groups
        .into_iter()
        .filter_map(|(name, arity)| {
            classify(program, inference, mut_sites, chains, &allocating, &name, arity)
                .map(|v| (name, arity, v))
        })
        .collect()
}

/// Does any arm of this group tail-call the group itself?
fn has_self_tail(program: &Program, name: &str, arity: usize) -> bool {
    tail_calls_to(program, name, arity).any(|in_group| in_group)
}

/// Does anything *outside* the group tail-call it? Such an entry never passes
/// through the loop's bracket, so the loop cannot rewind.
fn outside_tails(program: &Program, name: &str, arity: usize) -> bool {
    tail_calls_to(program, name, arity).any(|in_group| !in_group)
}

/// Every tail call to `name`/`arity`, paired with whether the caller is the
/// group itself.
fn tail_calls_to<'a>(
    program: &'a Program,
    name: &'a str,
    arity: usize,
) -> impl Iterator<Item = bool> + 'a {
    program.fns.iter().flat_map(move |decl| {
        let in_group = decl.name == name && decl.params.len() == arity;
        tail_exprs(decl.body.last()).into_iter().filter_map(move |tail| {
            let Expr::App { head, args, piped: false, .. } = tail else { return None };
            let Expr::Ident(callee, _) = head.as_ref() else { return None };
            (callee == name && args.len() == arity).then_some(in_group)
        })
    })
}

/// The verdict for one group, or None when it has no self-tail-call (not a
/// loop, nothing to say).
fn classify(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
    allocating: &HashSet<&str>,
    name: &str,
    arity: usize,
) -> Option<Verdict> {
    if !has_self_tail(program, name, arity) {
        return None;
    }
    if outside_tails(program, name, arity) {
        return Some(crate::beat::Verdict::OutsideTailCall);
    }
    if used_as_value(program, name) {
        return Some(crate::beat::Verdict::UsedAsValue);
    }
    if !allocating.contains(name) {
        return Some(crate::beat::Verdict::PureLoop);
    }
    let crossing = crossing_positions(program, inference, mut_sites, chains, name, arity);
    if !crossing.is_empty() {
        // a slot inference can't type may hide a growing accumulator
        // behind a helper call, and a byte builder rebuilt each iteration
        // would deep-copy its whole buffer at every rewind — neither is
        // ever assumed cheap to carry
        if let Some(&position) = crossing.iter().find(|&&p| {
            let set = group_param_set(program, inference, name, arity, p);
            accumulator_grows(program, name, arity, p) || set == 0 || set & BYTES != 0
        }) {
            return Some(crate::beat::Verdict::ArgCrosses { position });
        }
        if crossing.len() <= K_CARRY_MAX {
            return Some(crate::beat::Verdict::CarryBeat { positions: crossing });
        }
        return Some(crate::beat::Verdict::ArgCrosses { position: crossing[0] });
    }
    Some(crate::beat::Verdict::Beat)
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

fn stmt_allocates(
    stmt: &Stmt,
    fn_names: &HashSet<&str>,
    allocating: &HashSet<&str>,
    seed_pass: bool,
) -> bool {
    let e = match stmt {
        Stmt::Bind { expr, .. } => expr,
        Stmt::Expr(e) => e,
        Stmt::Set { value, .. } => value,
    };
    expr_allocates(e, fn_names, allocating, seed_pass)
}

/// Does evaluating `e` allocate? On the seed pass only primitive allocations
/// count (builders, interpolation, allocating builtins, constructors — any
/// call that is neither a known-pure builtin nor a user group). On fixpoint
/// passes a call to an already-allocating group counts too.
fn expr_allocates(
    e: &Expr,
    fn_names: &HashSet<&str>,
    allocating: &HashSet<&str>,
    seed_pass: bool,
) -> bool {
    const ALLOCATING: &[&str] = &[
        "chars",
        "concat",
        "entries",
        "err",
        "filter",
        "from_code",
        "join",
        "map",
        "push",
        "put",
        "slice",
        "sort",
        "utf8",
    ];
    const PURE: &[&str] = &[
        "at",
        "bytes",
        "char_code",
        "find2",
        "if",
        "length",
        "sum",
        "to_float",
        "to_int",
        "print",
    ];
    match e {
        Expr::List(..) | Expr::MapLit(..) | Expr::Lambda { .. } | Expr::Partial(..) => true,
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts.iter().any(|st| match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                expr_allocates(expr, fn_names, allocating, seed_pass)
            }
        }),
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
            head_allocates
                || args.iter().any(|a| expr_allocates(a, fn_names, allocating, seed_pass))
        }
        Expr::Field { base, .. } => expr_allocates(base, fn_names, allocating, seed_pass),
        Expr::Upcast { expr, .. } => expr_allocates(expr, fn_names, allocating, seed_pass),
        Expr::Index { base, index, .. } => {
            expr_allocates(base, fn_names, allocating, seed_pass)
                || expr_allocates(index, fn_names, allocating, seed_pass)
        }
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            expr_allocates(lhs, fn_names, allocating, seed_pass)
                || expr_allocates(rhs, fn_names, allocating, seed_pass)
        }
        Expr::Guard { cond, early, rest, .. } => {
            expr_allocates(cond, fn_names, allocating, seed_pass)
                || expr_allocates(early, fn_names, allocating, seed_pass)
                || rest
                    .iter()
                    .any(|s| expr_allocates(guard_stmt_expr(s), fn_names, allocating, seed_pass))
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

fn guard_stmt_expr(s: &Stmt) -> &Expr {
    match s {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
    }
}

fn expand_tail<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
    if let Expr::Guard { early, rest, .. } = e {
        expand_tail(early, out);
        if let Some(Stmt::Expr(last)) = rest.last() {
            expand_tail(last, out);
        }
        return;
    }
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

/// Does this expression yield a value that holds no pointer? A list carries
/// KValues, so licensing one to cross a rewind is only sound when nothing in
/// it can point at storage the rewind frees. Answering syntactically is
/// deliberately narrow: a literal number, a boolean, or a builtin whose whole
/// return set is scalar. Anything it cannot see through it refuses, which
/// costs a bracket and never correctness.
fn scalar_elem(
    e: &Expr,
    own: &str,
    decl: &crate::ast::FnDecl,
    inference: &infer::Inference,
    decl_index: usize,
) -> bool {
    match e {
        Expr::Int(_, _) | Expr::Float(_, _) => true,
        // Arithmetic and comparison over operands that are themselves
        // pointer-free: `+` on two numbers is a number, and every comparison
        // answers a boolean. An operand this cannot see through — a string,
        // a record — fails below and takes the whole expression with it, so
        // the recursion is what keeps the rule honest rather than the list of
        // operators.
        Expr::BinOp { lhs, rhs, .. } => {
            scalar_elem(lhs, own, decl, inference, decl_index)
                && scalar_elem(rhs, own, decl, inference, decl_index)
        }
        // Reading the accumulator being licensed. This is the same
        // co-inductive step the linearity fixpoint already takes: assume the
        // licence holds, and then every value in there is pointer-free, so
        // one read back out is too. A put that stores anything else fails its
        // own clause and denies the licence, which is what makes the
        // assumption safe to make.
        Expr::Index { base, .. } => matches!(base.as_ref(), Expr::Ident(b, _) if b == own),
        // The ordinary shape is `push xs n`, where n is the loop's own
        // counter: a name rather than a literal, and a scalar all the same.
        // Reading its parameter set lets the common case through without
        // letting a name of unknown type through with it.
        Expr::Ident(n, _) => {
            matches!(n.as_str(), "true" | "false")
                || decl
                    .params
                    .iter()
                    .position(|pat| matches!(pat, Pattern::Var(v, _) if v == n))
                    .is_some_and(|j| {
                        let set = inference.params[decl_index][j];
                        set != 0 && set & !FAIL & !SCALAR == 0
                    })
        }
        Expr::App { head, args, .. } => match head.as_ref() {
            Expr::Ident(n, _) => {
                let set = infer::builtin_set(n, &vec![infer::TOP; args.len()]);
                set != 0 && set & !FAIL & !SCALAR == 0
            }
            _ => false,
        },
        _ => false,
    }
}

/// A map accumulator threaded through in-place puts, every one of which writes
/// a pointer-free value under a key that holds no arena pointer.
///
/// The map's sorted view is what makes this different from a list, and the
/// difference is smaller than it looks. The view's own storage is malloc'd, so
/// a rewind cannot free it — what a rewind CAN invalidate is the values inside
/// it, and those are exactly what this rule already constrains. A literal key
/// is built once into permanent storage; an interpolated one is assembled in
/// the arena each time round and is refused for that reason.
///
/// The entry point insists on seeing a put. A bare name is a chain in the
/// recursion, because that is where the accumulator arrived from — but on its
/// own it means a map merely passed through and read, which is the case the
/// carry exists to evacuate and must not be licensed out of it.
fn is_scalar_map_chain(
    e: &Expr,
    own: &str,
    decl: &crate::ast::FnDecl,
    inference: &infer::Inference,
    decl_index: usize,
    locals: &HashMap<&str, &Expr>,
    mut_sites: &MutSites,
) -> bool {
    let Expr::App { head, .. } = e else { return false };
    let Expr::Ident(n, _) = head.as_ref() else { return false };
    matches!(n.as_str(), "put" | "builtin_put")
        && map_chain_rest(e, own, decl, inference, decl_index, locals, mut_sites)
}

fn map_chain_rest(
    e: &Expr,
    own: &str,
    decl: &crate::ast::FnDecl,
    inference: &infer::Inference,
    decl_index: usize,
    locals: &HashMap<&str, &Expr>,
    mut_sites: &MutSites,
) -> bool {
    match e {
        Expr::Ident(p, _) => {
            p == own
                || locals.get(p.as_str()).is_some_and(|e2| {
                    map_chain_rest(e2, own, decl, inference, decl_index, locals, mut_sites)
                })
        }
        Expr::App { head, args, span, .. } => match head.as_ref() {
            Expr::Ident(n, _)
                if matches!(n.as_str(), "put" | "builtin_put")
                    && args.len() == 3
                    && mut_sites.contains(&(decl.file.clone(), span.line, span.col)) =>
            {
                literal_key(&args[1])
                    && scalar_elem(&args[2], own, decl, inference, decl_index)
                    && map_chain_rest(&args[0], own, decl, inference, decl_index, locals, mut_sites)
            }
            _ => false,
        },
        _ => false,
    }
}

/// A key whose storage outlives any rewind: a number, or a string with nothing
/// interpolated into it, which the emitter builds once into a permanent slot
/// rather than in the arena.
fn literal_key(e: &Expr) -> bool {
    match e {
        Expr::Int(_, _) => true,
        Expr::Str(parts, _) => parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))),
        _ => false,
    }
}

/// A list accumulator threaded through in-place pushes, every one of which
/// pushes a scalar. The bytes license below rests on raw bytes holding no
/// pointers; this rests on the same fact reached a different way, so the two
/// are the same rule and not a widening of it.
fn is_scalar_list_chain(
    e: &Expr,
    own: &str,
    decl: &crate::ast::FnDecl,
    inference: &infer::Inference,
    decl_index: usize,
    locals: &HashMap<&str, &Expr>,
    mut_sites: &MutSites,
) -> bool {
    match e {
        Expr::Ident(p, _) => {
            p == own
                || locals.get(p.as_str()).is_some_and(|e2| {
                    is_scalar_list_chain(e2, own, decl, inference, decl_index, locals, mut_sites)
                })
        }
        Expr::App { head, args, span, .. } => match head.as_ref() {
            Expr::Ident(n, _)
                if matches!(n.as_str(), "push" | "builtin_push")
                    && args.len() == 2
                    && mut_sites.contains(&(decl.file.clone(), span.line, span.col)) =>
            {
                scalar_elem(&args[1], own, decl, inference, decl_index)
                    && is_scalar_list_chain(
                        &args[0], own, decl, inference, decl_index, locals, mut_sites,
                    )
            }
            _ => false,
        },
        _ => false,
    }
}

/// May `arg` cross an iteration boundary? Either an entry-threaded bare
/// parameter of an immutable-payload set, or a value the callee's parameter
/// set proves is a non-heap scalar (failures never reach a boundary: the
/// dispatcher propagates them before any arm body runs).
#[allow(clippy::too_many_arguments)]
fn arg_ok(
    program: &Program,
    inference: &infer::Inference,
    mut_sites: &MutSites,
    chains: &HashSet<Group>,
    decl: &crate::ast::FnDecl,
    decl_index: usize,
    name: &str,
    arity: usize,
    position: usize,
    arg: &Expr,
) -> bool {
    if let Expr::Ident(p, _) = arg {
        let own = decl.params.iter().position(|pat| matches!(pat, Pattern::Var(n, _) if n == p));
        if let Some(j) = own {
            let set = inference.params[decl_index][j];
            if set & !FAIL & !THREADED == 0 {
                return true;
            }
        }
    }
    // a bytes accumulator crossing by pointer identity: the value is the one
    // that arrived at this arm's entry, threaded through mut appends, so its
    // header is below the mark and its growth is outside the arena. Raw
    // bytes hold no pointers, so nothing in it can dangle across a rewind —
    // which is why this license reads BYTES and no other heap set.
    if let Some(Pattern::Var(own, _)) = decl.params.first() {
        let set0 = inference.params[decl_index][0];
        let locals = local_binds(decl);
        let folds = crate::linear::fold_spellings(program);
        if set0 != 0
            && set0 & !FAIL & !BYTES == 0
            && is_chain(arg, own, decl, &locals, mut_sites, chains, &folds)
        {
            return true;
        }
    }
    // The list accumulator is read at the position under test rather than at
    // the first parameter: a fold written `go n xs` carries its counter first
    // and the list it is building second, which is the ordinary shape.
    if let Some(Pattern::Var(own, _)) = decl.params.get(position) {
        let set = inference.params[decl_index][position];
        let locals = local_binds(decl);
        if set != 0
            && set & !FAIL & !LIST == 0
            && is_scalar_list_chain(arg, own, decl, inference, decl_index, &locals, mut_sites)
        {
            return true;
        }
        if set != 0
            && set & !FAIL & !MAP == 0
            && is_scalar_map_chain(arg, own, decl, inference, decl_index, &locals, mut_sites)
        {
            return true;
        }
    }
    // an empty set means inference saw no resolved call site — unknown,
    // never assumed safe
    let callee_set = group_param_set(program, inference, name, arity, position);
    callee_set != 0 && callee_set & !FAIL & !SCALAR == 0
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
                Stmt::Set { value, .. } => value,
            };
            value_use(e, name)
        })
    })
}

fn value_use(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident(n, _) | Expr::Partial(n, _) => n == name,
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts.iter().any(|st| match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                value_use(expr, name)
            }
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
        Expr::Seq(a, b, _) => value_use(a, name) || value_use(b, name),
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

#[cfg(test)]
mod tests {
    use super::beat_loops;
    use crate::infer;

    fn compiled(src: &str) -> (crate::ast::Program, infer::Inference) {
        let program = crate::compile("test.kso", src, false).unwrap();
        let inference = infer::infer(&program);
        (program, inference)
    }

    fn loops_of(src: &str) -> std::collections::HashSet<(String, usize)> {
        // the tests assert membership; the cluster ids are irrelevant here
        let program = crate::compile("test.kso", src, false).unwrap();
        let inference = infer::infer(&program);
        beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program))
            .ids
            .into_keys()
            .collect()
    }

    /// The report exists to say why a loop keeps the grow-only arena, and a
    /// loop can decline for more than one reason at once. Reporting only the
    /// first sends the next optimisation after a blocker that was never the
    /// whole story.
    ///
    /// The element pushed is a string on purpose. A list of scalars is
    /// licensed to cross a boundary now, so pushing `1` here would leave one
    /// blocker and the example could not say what it exists to say. A string
    /// element is a pointer into the arena, which is the boundary of that
    /// license and still a reason to decline.
    #[test]
    fn a_second_blocker_is_reported_not_masked() {
        let src = "fn feed acc\n  step acc\n\nfn step acc\n  step (push acc \"x\")\n\nmain = print \"{feed []}\"\n";
        let (program, inference) = compiled(src);
        let lines = super::report(&program, &inference, &crate::linear::in_place_pushes(&program));
        let step = lines.iter().find(|l| l.starts_with("step/1")).expect("step is reported");

        assert!(
            step.contains("unbracketed entry") && step.contains("also carries heap"),
            "the report named one blocker and hid the other: {step}"
        );
    }

    /// A list accumulator holds KValues, so licensing one to cross a rewind
    /// turns on what those values are. Integers point at nothing and are as
    /// safe to carry as the raw bytes of a string builder; a string element
    /// is a pointer into the arena the rewind is about to reclaim. Both
    /// halves are asserted here because a license that admitted either would
    /// pass an example that only tested the first.
    #[test]
    fn a_list_of_scalars_may_cross_a_boundary_and_a_list_of_strings_may_not() {
        let ints = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs 2)\n\nmain = print \"{length (go 3 [])}\"\n";
        assert!(loops_of(ints).contains(&("go".to_string(), 2)), "a list of integers is refused");

        let strs = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs \"a\")\n\nmain = print \"{length (go 3 [])}\"\n";
        assert!(
            !loops_of(strs).contains(&("go".to_string(), 2)),
            "a list of strings was licensed, and its elements can dangle"
        );
    }

    /// `push xs (n + 1)` is the same loop as `push xs n` with arithmetic in
    /// it, and arithmetic over pointer-free operands is pointer-free. The
    /// element test used to see through a literal and a builtin call and stop
    /// at a binary operator, which is an arbitrary place for a rule about
    /// what a value can point at to end.
    ///
    /// The string case is the boundary and is here for the same reason the
    /// other examples pair their halves: `+` over two strings answers a
    /// string, which is a pointer, so the operator alone proves nothing and
    /// the operands are what the rule reads.
    #[test]
    fn arithmetic_in_a_pushed_element_is_still_pointer_free() {
        let sums = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs (n + 1))\n\nmain = print \"{length (go 3 [])}\"\n";
        assert!(
            loops_of(sums).contains(&("go".to_string(), 2)),
            "a pushed sum of two numbers is refused"
        );

        let joins = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs (\"a\" + \"b\"))\n\nmain = print \"{length (go 3 [])}\"\n";
        assert!(
            !loops_of(joins).contains(&("go".to_string(), 2)),
            "a pushed join of two strings was licensed, and a string is a pointer"
        );
    }

    #[test]
    fn scalar_and_threaded_loop_is_eligible() {
        // the jsonbench shape: a threaded string, a counter, an accumulator
        let src = "fn crunch _ 0 acc\n  acc\n\nfn crunch cs n acc\n  crunch cs (n - 1) (acc + length \"beat {n}\")\n\nmain =\n  s = \"abc\"\n  print \"{crunch s 3 0}\"\n";
        assert!(loops_of(src).contains(&("crunch".to_string(), 3)));
    }

    /// `push acc n` extends its own previous value, and this used to keep the
    /// loop off the carry path: copying a growing accumulator every rewind
    /// cost quadratic bytes, and the ch10 teaching program went 33 KB to
    /// 16 MB of traffic when it was carried.
    ///
    /// A growing list of scalars carries now, because that copy no longer
    /// happens: its storage is malloc'd rather than arena, so a rewind has
    /// nothing to copy. The same ch10 program is byte-identical at 1,000 and
    /// 10,000 elements and holds a quarter of the arena at 100,000.
    ///
    /// The gate still stands for everything else it was written for — a
    /// growing map, a growing byte builder, a list whose elements are
    /// pointers — which is what the second half here insists on.
    #[test]
    fn a_growing_collection_of_scalars_carries_unless_its_key_lives_in_the_arena() {
        let ints = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (push acc n)\n\nmain = print \"{collect 3 []}\"\n";
        let (program, inference) = compiled(ints);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));
        assert!(
            beats.ids.contains_key(&("collect".to_string(), 2)),
            "a growing list of scalars is still refused the carry"
        );

        // A growing map carries too now, under a literal key: its pairs moved
        // out of the arena the same way a list's items did, and its sorted
        // view was always malloc'd, so the rewind can reach neither.
        let maps = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (put acc \"k\" n)\n\nmain = print \"{collect 3 {:}}\"\n";
        let (mp, mi) = compiled(maps);
        let mbeats = super::beat_loops(&mp, &mi, &crate::linear::in_place_pushes(&mp));
        assert!(
            mbeats.ids.contains_key(&("collect".to_string(), 2)),
            "a growing map under a literal key is still refused"
        );

        // And an interpolated key is not, because that string is assembled in
        // the arena each time round and the view would hold it after the
        // rewind freed it. This is the boundary the licence stops at.
        let keyed = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (put acc \"k{n}\" n)\n\nmain = print \"{collect 3 {:}}\"\n";
        let (kp, ki) = compiled(keyed);
        let kbeats = super::beat_loops(&kp, &ki, &crate::linear::in_place_pushes(&kp));
        assert!(
            !kbeats.ids.contains_key(&("collect".to_string(), 2)),
            "a map keyed by an interpolation was licensed, and that key lives in the arena"
        );
    }

    #[test]
    fn bounded_accumulator_carries() {
        // a fixed-shape rebuild does not grow with the iteration count; the
        // carry evacuates it each rewind.
        let src = "main = print \"{spin 10 [0 1]}\"\n\nfn spin 0 acc\n  length acc\n\nfn spin n acc\n  a = acc[1]\n  b = acc[2]\n  spin (n - 1) [b (a + b)]\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert_eq!(beats.carried.get(&("spin".to_string(), 2)), Some(&vec![1]));
    }
    #[test]
    fn tail_entry_from_acyclic_caller_is_demoted() {
        // go tail-calls into spin's loop, but go is acyclic: the entry is
        // demoted to a plain call (one bounded frame) and spin brackets.
        let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert!(beats.ids.contains_key(&("spin".to_string(), 2)));
        assert!(beats.demoted.contains(&(("go".to_string(), 1), ("spin".to_string(), 2))));
    }

    #[test]
    fn closure_threaded_loop_with_demoted_entry_is_a_beat() {
        // f is a closure handed through unchanged: immutable internals,
        // wholly below the entry mark, safe to carry across the rewind.
        let src = "fn go f n\n  spin f n 0\n\nmain =\n  salt = (x -> x * 2)\n  print \"{go salt 5}\"\n\nfn spin f 0 acc\n  f acc\n\nfn spin f n acc\n  step = \"seen {n}\"\n  spin f (n - 1) (acc + length step)\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
    }

    #[test]
    fn list_threaded_loop_is_a_beat() {
        // the list is handed onward unchanged every iteration: below the
        // mark, header never mutated, safe across the rewind.
        let src = "fn go xs n\n  spin xs n 0\n\nmain =\n  base = [10 20 30]\n  print \"{go base 5}\"\n\nfn spin xs 0 acc\n  acc + length xs\n\nfn spin xs n acc\n  garbage = \"iteration {n}\"\n  spin xs (n - 1) (acc + length garbage)\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
    }

    #[test]
    fn map_threaded_loop_carries_the_map() {
        // a map may never thread (its first read caches an above-mark sorted
        // view into the below-mark header), so the carry evacuates it — the
        // copy resets the cache, which keeps the rewind sound.
        let src = "fn go m n\n  spin m n 0\n\nmain =\n  prices = { \"a\":1 \"b\":2 }\n  print \"{go prices 3}\"\n\nfn spin m 0 acc\n  acc + length m\n\nfn spin m n acc\n  step = \"seen {n}\"\n  spin m (n - 1) (acc + length step)\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert_eq!(beats.carried.get(&("spin".to_string(), 3)), Some(&vec![0]));
    }

    #[test]
    fn demoted_entry_survives_a_clone_sharing_the_callers_name() {
        // a bare-enrolled clone named `go` joins the local go's dispatch
        // group; spin stays an eligible user loop, so go's entry must stay
        // demoted — losing the bracket while spin keeps its rewinds
        // corrupts live memory (the vse fold/fold_at miscompile).
        let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        let (mut program, _) = compiled(src);
        let mut clone = program.fns.iter().find(|d| d.name == "go").unwrap().clone();
        clone.synthetic = true;
        clone.file = "std/list".to_string();
        program.fns.push(clone);
        let inference = infer::infer(&program);
        let beats = beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert!(beats.demoted.contains(&(("go".to_string(), 1), ("spin".to_string(), 2))));
    }

    #[test]
    fn tail_entry_from_cyclic_caller_stays_ineligible() {
        // ping and pong form a tail cycle; pong's entry into spin can never
        // be demoted — a plain call inside a musttail cycle would grow the
        // stack without bound.
        let src = "main = print \"{ping 3}\"\n\nfn ping n\n  pong n\n\nfn pong 0\n  spin 2 0\n\nfn pong n\n  ping (n - 1)\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        let (program, inference) = compiled(src);
        let beats =
            super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

        assert!(!beats.ids.contains_key(&("spin".to_string(), 2)));
        assert!(beats.demoted.is_empty());
    }

    #[test]
    fn non_tail_outside_call_is_fine() {
        let src = "main = print \"{1 + spin 3 0}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
        assert!(loops_of(src).contains(&("spin".to_string(), 2)));
    }

    fn module_fixture(name: &str, lib: &str, entry: &str) -> crate::ast::Program {
        let dir = std::env::temp_dir().join(format!("kanso-beat-{name}"));
        std::fs::create_dir_all(dir.join("work")).expect("fixture dir");
        std::fs::write(dir.join("work/work.kso"), lib).expect("fixture writes");
        let main = dir.join("main.kso");
        std::fs::write(&main, entry).expect("fixture writes");
        crate::compile_entry(&main.to_string_lossy(), entry).unwrap()
    }

    #[test]
    fn a_module_loop_is_one_group_with_one_spelling() {
        // enrollment gives an imported function a bare twin, and before the
        // alias canonicalization the twin made every module-internal
        // recursion read as a call between two groups — no loop inside an
        // imported module could ever beat. One spelling, one group, and the
        // chain license reaches it.
        let lib = "import \"std/text\"\n\npub fn stamp acc 0\n  acc\n\npub fn stamp acc n\n  stamp (text/append acc \"x{n}\") (n - 1)\n\npub fn start _\n  text/bytes \"\"\n\npub fn finish acc\n  length (text/utf8 acc)\n";
        let entry = "import \"work\"\n\nprint \"{work/finish (work/stamp (work/start 0) 9)}\"\n";
        let program = module_fixture("canon", lib, entry);
        let inference = infer::infer(&program);
        let muts = crate::linear::in_place_pushes(&program);
        let loops = beat_loops(&program, &inference, &muts);

        assert!(loops.ids.contains_key(&("work/stamp".to_string(), 2)), "got {:?}", loops.ids);
    }

    #[test]
    fn a_locally_bound_name_is_never_rewritten() {
        // `first` here is a local binding, and std/list also exports a
        // `first`. Rewriting the local to the function value fed a closure
        // to append at runtime — kq's pretty-printer found it. A name that
        // is ever locally bound keeps its clones and its old dispatch.
        let dir = std::env::temp_dir().join("kanso-beat-shadow");
        std::fs::create_dir_all(&dir).expect("fixture dir");
        let _ = std::fs::remove_dir_all(dir.join("work"));
        std::fs::write(
            dir.join("helpers.kso"),
            "import \"std/list\"\n\npub fn label xs\n  first = list/first xs\n  \"head {first}\"\n",
        )
        .expect("fixture writes");
        std::fs::write(dir.join("main.kso"), "print \"{label [7 8 9]}\"\n")
            .expect("fixture writes");
        let program = crate::compile_module(&dir, false).unwrap();
        let label = program.fns.iter().find(|d| d.name == "label").expect("label compiles");
        let mut idents = std::collections::HashSet::new();
        for stmt in &label.body {
            collect_idents(stmt, &mut idents);
        }

        assert!(idents.contains("first"), "the local was rewritten: {idents:?}");
    }

    fn collect_idents(stmt: &crate::ast::Stmt, out: &mut std::collections::HashSet<String>) {
        let e = match stmt {
            crate::ast::Stmt::Bind { expr, .. }
            | crate::ast::Stmt::Expr(expr)
            | crate::ast::Stmt::Set { value: expr, .. } => expr,
        };
        idents_in(e, out);
    }

    fn idents_in(e: &crate::ast::Expr, out: &mut std::collections::HashSet<String>) {
        if let crate::ast::Expr::Ident(n, _) = e {
            out.insert(n.clone());
        }
        if let crate::ast::Expr::Str(parts, _) = e {
            for p in parts {
                if let crate::ast::TemplatePart::Interp(inner) = p {
                    idents_in(inner, out);
                }
            }
        }
        if let crate::ast::Expr::App { head, args, .. } = e {
            idents_in(head, out);
            for a in args {
                idents_in(a, out);
            }
        }
    }

    #[test]
    fn json_decode_loops_stay_conservative() {
        // kanso-json's scanners are mutually recursive and thread record and
        // list accumulators — those stay out. The two encoders thread a byte
        // builder by pointer identity, which is exactly what the chain
        // license admits: raw bytes hold no pointers, so nothing in the
        // accumulator can dangle across a rewind.
        let program = crate::compile_module(std::path::Path::new("lib/json"), false).unwrap();
        let inference = infer::infer(&program);
        let loops = beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));
        let mut licensed: Vec<(String, usize)> = loops.ids.into_keys().collect();
        licensed.sort();
        assert_eq!(
            licensed,
            vec![("encode_items".to_string(), 3), ("encode_pairs".to_string(), 3)],
            "only the byte-builder encoders may rewind; scanners threading \
             records or lists stay on the grow-only arena"
        );
    }
}
