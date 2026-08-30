use crate::ast::*;
use crate::hash::Map as HashMap;

/// Propagable type sets as tag bitsets — the single monotone inference
/// fixpoint (the story is told in about.html part 03), coarse to start:
/// one bit per runtime tag, records unrefined.
pub type Set = u16;

pub const INT: Set = 1 << 0;
pub const FLOAT: Set = 1 << 1;
pub const TRUE: Set = 1 << 2;
pub const FALSE: Set = 1 << 3;
pub const NONE: Set = 1 << 4;
pub const ERR: Set = 1 << 5;
pub const STR: Set = 1 << 6;
pub const REC: Set = 1 << 7;
pub const DESC: Set = 1 << 8;
pub const LIST: Set = 1 << 9;
pub const MAP: Set = 1 << 10;
pub const FN: Set = 1 << 11;
pub const BYTES: Set = 1 << 12;
/// Lazy v1: the value may be an unforced thunk; force sites are emitted
/// only where this bit is present, so strict code pays nothing.
pub const THUNK: Set = 1 << 13;
pub const TOP: Set = (1 << 14) - 1;
/// What propagates on its own. A none is a value and stays where it is put;
/// only an err abandons the computation that produced it.
pub const FAIL: Set = ERR;
pub const BOOL: Set = TRUE | FALSE;

pub struct Inference {
    /// Every declaration's argument sets, end to end, with `param_starts`
    /// saying where each one begins. A `Vec<Set>` apiece was one heap block
    /// per declaration for one to three `u16`, and `Set` is a `u16`. Read it
    /// through `param` rather than by index.
    params: Vec<Set>,
    /// One more entry than there are declarations, so a declaration's slice is
    /// always `params[starts[i]..starts[i + 1]]`.
    param_starts: Vec<u32>,
    /// per fn-decl index: return set
    pub returns: Vec<Set>,
    /// per type index, per field: joined set seen at construction sites
    pub type_fields: Vec<Vec<Set>>,
}

impl Inference {
    /// The set inferred for one parameter of one declaration.
    pub fn param(&self, decl: usize, at: usize) -> Set {
        self.params[self.param_starts[decl] as usize + at]
    }
}

struct Ctx<'a> {
    /// Whether any constant in this program mentions itself. Such a mention
    /// is emitted as a thunk in a storing position, so every container read
    /// has to admit one; no other program does, and none pays for it.
    defers_into_containers: bool,
    program: &'a Program,
    demand: crate::demand::DemandInfo<'a>,
    /// (name, arity) of the decl currently being walked, for lazy-bind lookup.
    current: (&'a str, usize),
    /// Which function is being evaluated, so a read of somebody's return set
    /// can record who wanted it.
    current_index: usize,
    /// For each function, the functions that have read its return set. A
    /// return that widens only ever changes the answer of a function that
    /// asked for it.
    /// Who to wake when a declaration's answer changes, as a bitset: one row
    /// of `reader_words` u64 per declaration, the row's bit `r` set when `r`
    /// reads it. A `HashSet<usize>` apiece was 1,072 of the front end's
    /// allocation blocks, and waking cloned one to release the borrow.
    readers: Vec<u64>,
    reader_words: usize,
    /// One row, copied out so the wake loop can take `ctx` mutably. Kept
    /// between wakes, so it allocates once.
    reader_scratch: Vec<u64>,
    /// Functions to visit next round. A field of a declared type widening can
    /// reach any function through pattern binding, so that one blankets.
    dirty_next: Vec<bool>,
    /// The current round's work list. It lives here rather than in the loop
    /// so a change can wake a reader the walk has not reached yet, and that
    /// reader runs in this round instead of costing a whole extra one.
    dirty: Vec<bool>,
    /// Per declared type, the functions whose patterns destructure it — see
    /// `field_readers`. A field's set growing wakes these and nothing else.
    field_readers: Vec<Vec<usize>>,
    /// The arms a (name, arity) dispatches over, as a half-open range into
    /// `group_members`. A range is two words and copies, so a call site can
    /// take one and leave `ctx` free to be borrowed mutably while it walks
    /// the arms — where holding the group itself meant cloning its `Vec` on
    /// every call the pass inferred, 2,693 heap blocks on `lib/json`.
    groups: HashMap<(&'a str, usize), (u32, u32)>,
    group_members: Vec<usize>,
    /// What a desc-valued local would yield to a bind, tracked through one
    /// binding level so `x = os/read_file p` then `x . f` gives f the STR.
    yields: HashMap<&'a str, Set>,
    type_names: HashMap<&'a str, usize>,
    params: Vec<Set>,
    param_starts: Vec<u32>,
    returns: Vec<Set>,
    type_fields: Vec<Vec<Set>>,
    changed: bool,
}

/// What compiling actually did, as opposed to what it wrote. Emitted text
/// measures the product; these count the process — the fixpoint rounds and
/// the expression visits that produce it.
pub mod work {
    use std::cell::Cell;
    thread_local! {
        static ROUNDS: Cell<u64> = const { Cell::new(0) };
        static VISITS: Cell<u64> = const { Cell::new(0) };
        static PASSES: Cell<u64> = const { Cell::new(0) };
    }
    pub fn reset() {
        ROUNDS.with(|c| c.set(0));
        VISITS.with(|c| c.set(0));
        PASSES.with(|c| c.set(0));
    }
    /// One whole-program inference. Rounds and visits say what a pass costs;
    /// this says how many the front end asks for, which is the number a new
    /// diagnostic can raise without either of the others moving.
    pub fn pass() {
        PASSES.with(|c| c.set(c.get() + 1));
    }
    pub fn passes() -> u64 {
        PASSES.with(Cell::get)
    }
    pub fn round() {
        ROUNDS.with(|c| c.set(c.get() + 1));
    }
    pub fn visit() {
        VISITS.with(|c| c.set(c.get() + 1));
    }
    /// (fixpoint rounds, expression visits)
    pub fn taken() -> (u64, u64) {
        (ROUNDS.with(Cell::get), VISITS.with(Cell::get))
    }
}

/// Which functions can be affected when a declared type's field set grows.
///
/// Only a `Pattern::Ctor` reads `type_fields` — destructuring `(cell v)` is
/// what turns a field's accumulated set into a binding — so the answer is
/// static: every function whose patterns, in its head or anywhere in its body,
/// destructure that type. Before this index existed a field growing marked
/// EVERY function dirty, and lib/json spent three extra full sweeps of 407
/// functions to let seven of them move.
fn ctor_types(pat: &Pattern, type_names: &HashMap<&str, usize>, out: &mut Vec<usize>) {
    if let Pattern::Ctor { ty, fields, .. } = pat {
        if let Some(&i) = type_names.get(ty.as_str()) {
            out.push(i);
        }
        for field in fields {
            ctor_types(field, type_names, out);
        }
    }
}

fn stmt_ctor_types(stmt: &Stmt, type_names: &HashMap<&str, usize>, out: &mut Vec<usize>) {
    match stmt {
        Stmt::Bind { pattern, expr } => {
            ctor_types(pattern, type_names, out);
            expr_ctor_types(expr, type_names, out);
        }
        Stmt::Expr(expr) => expr_ctor_types(expr, type_names, out),
        Stmt::Set { value, .. } => expr_ctor_types(value, type_names, out),
    }
}

fn expr_ctor_types(expr: &Expr, type_names: &HashMap<&str, usize>, out: &mut Vec<usize>) {
    if let Expr::Block(stmts, _) | Expr::Build(stmts, _) = expr {
        for stmt in stmts {
            stmt_ctor_types(stmt, type_names, out);
        }
    }
    crate::for_each_child(expr, |child| expr_ctor_types(child, type_names, out));
}

fn field_readers(program: &Program, type_names: &HashMap<&str, usize>) -> Vec<Vec<usize>> {
    let mut readers = vec![Vec::new(); program.types.len()];
    for (i, decl) in program.fns.iter().enumerate() {
        let mut seen = Vec::new();
        for pattern in &decl.params {
            ctor_types(pattern, type_names, &mut seen);
        }
        for stmt in &decl.body {
            stmt_ctor_types(stmt, type_names, &mut seen);
        }
        seen.sort_unstable();
        seen.dedup();
        for ty in seen {
            readers[ty].push(i);
        }
    }
    readers
}

pub fn infer(program: &Program) -> Inference {
    work::pass();
    let mut by_group: HashMap<(&str, usize), Vec<usize>> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for (i, decl) in program.fns.iter().enumerate() {
        by_group.entry((decl.name.as_str(), decl.params.len())).or_default().push(i);
    }
    let mut group_members: Vec<usize> = Vec::with_capacity(program.fns.len());
    let groups: HashMap<(&str, usize), (u32, u32)> = by_group
        .into_iter()
        .map(|(key, arms)| {
            let start = group_members.len() as u32;
            group_members.extend(arms);
            (key, (start, group_members.len() as u32))
        })
        .collect();
    let type_names: HashMap<&str, usize> =
        program.types.iter().enumerate().map(|(i, t)| (t.name.as_str(), i)).collect();
    let field_readers = field_readers(program, &type_names);
    let defers_into_containers = program.fns.iter().any(|d| {
        fn mentions(expr: &Expr, name: &str) -> bool {
            if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
                if n == name {
                    return true;
                }
            }
            crate::any_child(expr, |c| mentions(c, name))
        }
        d.params.is_empty()
            && d.body.iter().any(|stmt| match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => mentions(expr, &d.name),
                Stmt::Set { value, .. } => mentions(value, &d.name),
            })
    });
    // A row of bits per declaration, rounded up to whole words.
    let reader_words = program.fns.len().div_ceil(64).max(1);
    // Where each declaration's argument sets begin in the flat vector below,
    // with a final entry so the last declaration's slice reads the same way as
    // every other one's.
    let mut param_starts: Vec<u32> = Vec::with_capacity(program.fns.len() + 1);
    let mut at = 0u32;
    for d in &program.fns {
        param_starts.push(at);
        at += d.params.len() as u32;
    }
    param_starts.push(at);
    let mut ctx = Ctx {
        defers_into_containers,
        program,
        demand: crate::phase::watched("infer/demand", || crate::demand::analyze(program)),
        current: ("", 0),
        current_index: 0,
        readers: vec![0u64; program.fns.len() * reader_words],
        reader_words,
        reader_scratch: Vec::with_capacity(reader_words),
        dirty_next: vec![false; program.fns.len()],
        dirty: Vec::new(),
        field_readers,
        groups,
        group_members,
        yields: HashMap::default(),
        type_names,
        params: vec![0; program.fns.iter().map(|d| d.params.len()).sum()],
        param_starts,
        returns: vec![0; program.fns.len()],
        type_fields: program.types.iter().map(|t| vec![0; t.fields.len()]).collect(),
        changed: true,
    };
    // seed: entry points (main, constants, tests) run with no arguments;
    // anything used as a function value gets TOP params.
    let mut rounds = 0;
    // The program outlives the context, so an arm's name is borrowed rather
    // than cloned: this loop runs once per function per round, and a round
    // count in the twenties turns a clone here into thousands of allocations
    // that only ever serve as a lookup key.
    let fns = &program.fns;
    let mut env = Env::default();
    let mut param_sets: Vec<Set> = Vec::new();
    // Every function is visited the first round; after that only the ones a
    // change can reach. Four fifths of the visits in a settled fixpoint find
    // nothing, and a visit costs a walk of the whole body.
    ctx.dirty = vec![true; fns.len()];
    // A round reads whatever earlier visits in the same round already wrote,
    // and the two flows want opposite orders: returns travel callee-to-caller,
    // params caller-to-callee. The first sweep runs callee-first so leaf
    // returns land before any caller asks; after that the direction
    // alternates, and each sweep carries one flow the whole way instead of
    // one hop per round.
    let order = callee_first(program);
    while ctx.changed && rounds < 200 {
        ctx.changed = false;
        rounds += 1;
        work::round();
        let mut moved = 0usize;
        ctx.dirty_next = vec![false; fns.len()];
        let mut visited = 0usize;
        let walk: Box<dyn Iterator<Item = &usize>> = match rounds % 2 == 1 {
            true => Box::new(order.iter()),
            false => Box::new(order.iter().rev()),
        };
        for &i in walk {
            let decl = &fns[i];
            if !ctx.dirty[i] {
                continue;
            }
            visited += 1;
            ctx.current_index = i;
            ctx.current = (decl.name.as_str(), decl.params.len());
            ctx.yields.clear();
            env.clear();
            param_sets.clear();
            param_sets.extend_from_slice(
                &ctx.params[ctx.param_starts[i] as usize..ctx.param_starts[i + 1] as usize],
            );
            for (pattern, joined) in decl.params.iter().zip(&param_sets) {
                bind_pattern(pattern, *joined, &ctx.type_fields, &ctx.type_names, &mut env);
            }
            let ret = eval_body(&mut ctx, &decl.body, &mut env);
            if ret | ctx.returns[i] != ctx.returns[i] {
                ctx.returns[i] |= ret;
                ctx.changed = true;
                moved += 1;
                let w = ctx.reader_words;
                let mut scratch = std::mem::take(&mut ctx.reader_scratch);
                scratch.clear();
                scratch.extend_from_slice(&ctx.readers[i * w..(i + 1) * w]);
                for (word, &row) in scratch.iter().enumerate() {
                    let mut bits = row;
                    while bits != 0 {
                        let r = word * 64 + bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        // This round as well as the next. A reader the sweep
                        // has not reached yet takes the new answer now instead
                        // of costing a whole round to hear about it, and one
                        // already behind the cursor is simply not walked again.
                        ctx.dirty[r] = true;
                        ctx.dirty_next[r] = true;
                    }
                }
                ctx.reader_scratch = scratch;
            }
        }
        if std::env::var_os("KANSO_PHASES").is_some() {
            eprintln!("round {rounds}: {moved} moved of {visited} visited");
        }
        ctx.dirty = std::mem::take(&mut ctx.dirty_next);
    }
    Inference {
        params: ctx.params,
        param_starts: ctx.param_starts,
        returns: ctx.returns,
        type_fields: ctx.type_fields,
    }
}

/// Post-order over the call graph: every function lands after the functions
/// it mentions, so within a round the shared params and returns are already
/// fresh where the graph is acyclic, and a cycle costs rounds only for its
/// own knot.
fn callee_first(program: &Program) -> Vec<usize> {
    // The declarations sharing a name, as one flat vector and a range apiece.
    // `by_name.entry(..).or_default().push(i)` allocated a `Vec` per DISTINCT
    // name — 937 blocks for a table that is built once and only ever read.
    // Sorting the pairs groups the arms of a name together, and within a name
    // the indices stay ascending, which is the order the push loop gave them.
    let mut by_name: HashMap<&str, (u32, u32)> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for decl in &program.fns {
        by_name.entry(decl.name.as_str()).or_insert((0, 0)).1 += 1;
    }
    let mut at = 0;
    for slot in by_name.values_mut() {
        let count = slot.1;
        *slot = (at, at);
        at += count;
    }
    let mut members: Vec<usize> = vec![0; program.fns.len()];
    for (i, decl) in program.fns.iter().enumerate() {
        let slot = by_name.get_mut(decl.name.as_str()).expect("every name was counted");
        members[slot.1 as usize] = i;
        slot.1 += 1;
    }
    fn gather<'a>(expr: &'a Expr, names: &mut Vec<&'a str>) {
        if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
            names.push(n.as_str());
        }
        crate::for_each_child(expr, |child| gather(child, names));
    }
    // One flat vector and a start per declaration, rather than a `Vec<usize>`
    // apiece. The loop below fills each declaration's slice completely before
    // moving to the next, so appending to the flat vector and recording where
    // each one began is the same walk with one allocation instead of four
    // hundred. #1140 did this to the dispatch table for the same reason.
    let mut flat: Vec<usize> = Vec::new();
    let mut starts: Vec<u32> = Vec::with_capacity(program.fns.len() + 1);
    // Outside the loop and cleared, rather than built inside it. The vector
    // holds borrowed names and dies at the end of each declaration, so a fresh
    // one per declaration was 1,367 allocation blocks for a buffer that is the
    // same size and shape every time round. Reused, it grows once to the
    // largest declaration and stays there.
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        names.clear();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => gather(expr, &mut names),
                Stmt::Set { value, .. } => gather(value, &mut names),
            }
        }
        names.sort_unstable();
        names.dedup();
        starts.push(flat.len() as u32);
        for name in &names {
            if let Some(&(start, end)) = by_name.get(name) {
                flat.extend_from_slice(&members[start as usize..end as usize]);
            }
        }
    }
    starts.push(flat.len() as u32);
    let calls = |i: usize| &flat[starts[i] as usize..starts[i + 1] as usize];
    let n = program.fns.len();
    let mut order = Vec::with_capacity(n);
    let mut state = vec![0u8; n];
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack = vec![(root, 0usize)];
        while let Some(frame) = stack.last_mut() {
            let (node, next) = *frame;
            if next < calls(node).len() {
                frame.1 += 1;
                let child = calls(node)[next];
                if state[child] == 0 {
                    state[child] = 1;
                    stack.push((child, 0));
                }
            } else {
                state[node] = 2;
                order.push(node);
                stack.pop();
            }
        }
    }
    order
}

/// The locals a declaration has bound, newest last.
///
/// This was a `HashMap<&str, Set>`, and it is the wrong shape for what it
/// holds: a declaration binds a handful of names, and hashing a five-byte
/// string costs more than walking a handful of them. Every child scope —
/// a block, a lambda, a guard's untaken side — took a clone of it, which
/// allocated a table and rehashed every entry into it; the clone is a
/// memcpy now.
///
/// A bind pushes rather than replaces, so a name written twice appears
/// twice and the search reads back to front. Lookup answers the same as
/// the map did, and the vector cannot grow past the bind sites written in
/// the declaration.
#[derive(Clone, Default)]
struct Env<'a>(Vec<(&'a str, Set)>);

impl<'a> Env<'a> {
    fn clear(&mut self) {
        self.0.clear();
    }

    fn insert(&mut self, name: &'a str, set: Set) {
        self.0.push((name, set));
    }

    fn get(&self, name: &str) -> Option<Set> {
        self.0.iter().rev().find(|(n, _)| *n == name).map(|&(_, s)| s)
    }

    fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// A child scope: these bindings, with room for the ones it will add.
    /// A plain clone sizes the vector to what it copied, so the child's
    /// first bind reallocated it — 216 allocations over a compile.
    fn child(&self, extra: usize) -> Env<'a> {
        let mut out = Vec::with_capacity(self.0.len() + extra);
        out.extend_from_slice(&self.0);
        Env(out)
    }
}

fn bind_pattern<'a>(
    pattern: &'a Pattern,
    joined: Set,
    type_fields: &[Vec<Set>],
    type_names: &HashMap<&'a str, usize>,
    env: &mut Env<'a>,
) {
    match pattern {
        // generics never bind failures
        Pattern::Var(name, _) => {
            env.insert(name, joined & !FAIL);
        }
        Pattern::Wildcard(_) | Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => {
        }
        Pattern::Annotated { name, ty, .. } => {
            let set = match ty.as_str() {
                "int" => INT,
                "float64" => FLOAT,
                "string" => STR,
                "bool" => BOOL,
                "err" => ERR,
                t if t.ends_with("[]") => LIST,
                t if t.contains('[') => MAP,
                _ => REC,
            };
            env.insert(name, set);
        }
        // destructuring a declared type refines each field to the join of what
        // construction sites stored there — so `_parsed p v` gives p its real
        // int-ness instead of TOP, which is what unblocks the scanner's hot path
        Pattern::Ctor { ty, fields, .. } => {
            let field_sets = type_names.get(ty.as_str()).map(|i| &type_fields[*i]);
            for (fi, field) in fields.iter().enumerate() {
                let s = field_sets.and_then(|fs| fs.get(fi)).copied().unwrap_or(TOP & !FAIL);
                bind_pattern(field, s, type_fields, type_names, env);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for entry in entries {
                env.insert(&entry.bind_name, TOP & !FAIL);
            }
        }
    }
}

fn pattern_catches(pat: &Pattern) -> Set {
    match pat {
        Pattern::Nullary(name, _) if name == "none" => NONE,
        Pattern::Ctor { ty, .. } if ty == "err" => ERR,
        _ => 0,
    }
}

fn eval_body<'a>(ctx: &mut Ctx<'a>, body: &'a [Stmt], env: &mut Env<'a>) -> Set {
    let mut result = NONE;
    for (index, stmt) in body.iter().enumerate() {
        match stmt {
            Stmt::Bind { pattern, expr } => {
                let mut value = eval_expr(ctx, expr, env);
                if ctx.demand.is_lazy_bind(ctx.current.0, ctx.current.1, index) {
                    // The binding holds a thunk; forcing yields the expr's set.
                    value |= THUNK;
                }
                match pattern {
                    Pattern::Var(name, _) => {
                        if value & DESC != 0 {
                            let y = desc_yield_of(ctx, expr);
                            ctx.yields.insert(name, y);
                        }
                        env.insert(name, value);
                    }
                    _ => bind_pattern(pattern, value, &ctx.type_fields, &ctx.type_names, env),
                }
            }
            Stmt::Expr(expr) => result = eval_expr(ctx, expr, env),
            Stmt::Set { value, .. } => {
                eval_expr(ctx, value, env);
            }
        }
    }
    result
}

fn eval_expr<'a>(ctx: &mut Ctx<'a>, expr: &'a Expr, env: &mut Env<'a>) -> Set {
    work::visit();
    match expr {
        Expr::Int(..) => INT,
        Expr::Partial(..) => TOP,
        Expr::Upcast { expr: inner, .. } => eval_expr(ctx, inner, env),
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            // a child scope: block binds stay local to the branch
            let mut env = env.child(stmts.len());
            let mut result = NONE;
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { pattern, expr } => {
                        let value = eval_expr(ctx, expr, &mut env);
                        match pattern {
                            Pattern::Var(name, _) => {
                                env.insert(name, value);
                            }
                            _ => bind_pattern(
                                pattern,
                                value,
                                &ctx.type_fields,
                                &ctx.type_names,
                                &mut env,
                            ),
                        }
                    }
                    Stmt::Expr(expr) => result = eval_expr(ctx, expr, &mut env),
                    Stmt::Set { value, .. } => {
                        eval_expr(ctx, value, &mut env);
                    }
                }
            }
            result
        }
        Expr::Field { base, .. } => {
            let _ = eval_expr(ctx, base, env);
            TOP
        }
        Expr::Float(..) => FLOAT,
        Expr::Str(parts, _) => {
            let mut fails: Set = 0;
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    fails |= eval_expr(ctx, inner, env) & FAIL;
                }
            }
            STR | fails
        }
        Expr::Ident(name, _) => ident_set(ctx, name, env),
        Expr::List(items, _) => {
            for item in items {
                let _ = eval_expr(ctx, item, env);
            }
            LIST
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                let _ = eval_expr(ctx, k, env);
                let _ = eval_expr(ctx, v, env);
            }
            MAP
        }
        Expr::Index { base, index, strict, .. } => {
            let b = eval_expr(ctx, base, env);
            let k = eval_expr(ctx, index, env);
            // a miss errs under the sigil (xs[i]!) and nones under the plain
            // lenient form (xs[i])
            let miss = match strict {
                true => ERR,
                false => NONE,
            };
            let mut out = (b & FAIL) | (k & FAIL) | miss;
            if b & BYTES != 0 {
                out |= INT;
            }
            if b & (LIST | MAP | STR) != 0 {
                // A container holds a thunk only where a constant defers a
                // mention of itself into one, so a program with no such
                // constant keeps the mask and pays no force at any read.
                let deferred = match ctx.defers_into_containers {
                    true => THUNK,
                    false => 0,
                };
                out |= (TOP & !FAIL & !THUNK) | deferred;
            }
            out
        }
        Expr::Seq(l, r, _) => {
            let a = eval_expr(ctx, l, env);
            let b = eval_expr(ctx, r, env);
            DESC | (a & FAIL) | (b & FAIL)
        }
        Expr::Lambda { body, params, .. } => {
            let mut inner = env.child(params.len());
            for (p, _) in params {
                inner.insert(p, TOP & !FAIL);
            }
            let _ = eval_expr(ctx, body, &mut inner);
            FN
        }
        Expr::BinOp { op, lhs, rhs, .. } => {
            let a = eval_expr(ctx, lhs, env);
            let b = eval_expr(ctx, rhs, env);
            let fails = (a & FAIL) | (b & FAIL);
            match *op {
                // int op int stays int; any float operand widens the other,
                // so the result is float
                "+" | "-" | "*" => fails | numeric_result(a, b),
                "/" | "%" => fails | ERR | numeric_result(a, b),
                // the bitwise three answer a whole number; every remaining
                // operator compares, and a comparison answers true or false
                "&" | "|" | "^" => fails | INT,
                _ => BOOL | fails,
            }
        }
        Expr::Guard { cond, early, rest, .. } => {
            let c = eval_expr(ctx, cond, env);
            let mut benv = env.child(rest.len());
            let taken = eval_expr(ctx, early, env);
            let not_taken = eval_body(ctx, rest, &mut benv);
            (c & FAIL) | taken | not_taken
        }
        // the join yields a description, a lone propagated failure, or an
        // accumulated err merged from both sides
        Expr::Join { lhs, rhs, .. } => {
            let a = eval_expr(ctx, lhs, env);
            let b = eval_expr(ctx, rhs, env);
            DESC | ((a | b) & FAIL) | ERR
        }
        Expr::App { head, args, piped, .. } => eval_call(ctx, head, args, env, *piped),
    }
}

/// The numeric result of `+`/`-`/`*`/`/`: int only when both are int; float
/// whenever a float meets any number (the int widens).
fn numeric_result(a: Set, b: Set) -> Set {
    let mut out = 0;
    if a & INT != 0 && b & INT != 0 {
        out |= INT;
    }
    let anum = a & (INT | FLOAT);
    let bnum = b & (INT | FLOAT);
    if (a & FLOAT != 0 && bnum != 0) || (b & FLOAT != 0 && anum != 0) {
        out |= FLOAT;
    }
    out
}

fn ident_set<'a>(ctx: &mut Ctx<'a>, name: &'a str, env: &mut Env<'a>) -> Set {
    if let Some(set) = env.get(name) {
        return set;
    }
    match name.strip_prefix("builtin_").unwrap_or(name) {
        "true" => TRUE,
        "false" => FALSE,
        "none" => NONE,
        "args" | "stdin" | "now" => DESC,
        _ => {
            // a zero-field type's bare mention is its marker value
            if let Some(i) = ctx.type_names.get(name) {
                if ctx.program.types[*i].fields.is_empty() {
                    return REC;
                }
            }
            // constant mention evaluates; fn mention is a value (params go TOP)
            if let Some(&(start, _)) = ctx.groups.get(&(name, 0)) {
                let i = ctx.group_members[start as usize];
                mark_reader(ctx, i);
                // A constant naming itself inside its own body hands over its
                // cell rather than a value — there is nothing else to hand
                // over yet. Everything that mention flows into may therefore
                // hold a thunk, which is what tells a later read to force
                // rather than to report a value no arm knows.
                let deferred = match ctx.current_index == i {
                    true => THUNK,
                    false => 0,
                };
                return ctx.returns[i] | deferred;
            }
            let arities: Vec<usize> =
                ctx.program.fns.iter().filter(|d| d.name == name).map(|d| d.params.len()).collect();
            for (i, decl) in ctx.program.fns.iter().enumerate() {
                if decl.name == name {
                    for p in 0..decl.params.len() {
                        widen_param(ctx, i, p, TOP);
                    }
                }
            }
            let _ = arities;
            FN
        }
    }
}

/// `decl`'s answer feeds the declaration being walked, so a change to it must
/// wake that one.
fn mark_reader(ctx: &mut Ctx<'_>, decl: usize) {
    let at = decl * ctx.reader_words + ctx.current_index / 64;
    ctx.readers[at] |= 1u64 << (ctx.current_index % 64);
}

fn widen_param(ctx: &mut Ctx<'_>, decl: usize, param: usize, set: Set) {
    let at = ctx.param_starts[decl] as usize + param;
    if ctx.params[at] | set != ctx.params[at] {
        ctx.params[at] |= set;
        ctx.changed = true;
        ctx.dirty_next[decl] = true;
    }
}

fn eval_call<'a>(
    ctx: &mut Ctx<'a>,
    head: &'a Expr,
    args: &'a [Expr],
    env: &mut Env<'a>,
    piped: bool,
) -> Set {
    // A `Set` is a u16 and a call's arity is almost always small, so the
    // eight that fit here never reach the allocator. dhat put this line at
    // 8,309 blocks — 13.4% of every allocation the front end makes — for
    // vectors holding one to three sixteen-bit values.
    let mut inline = [0 as Set; 8];
    let mut spill: Vec<Set> = Vec::new();
    let arg_sets: &mut [Set] = match args.len() <= inline.len() {
        true => {
            for (slot, a) in inline.iter_mut().zip(args) {
                *slot = eval_expr(ctx, a, env);
            }
            &mut inline[..args.len()]
        }
        false => {
            spill.extend(args.iter().map(|a| eval_expr(ctx, a, env)));
            &mut spill
        }
    };
    let mut piped_bits: Set = 0;
    if piped && !arg_sets.is_empty() && arg_sets[0] & DESC != 0 {
        // a description piped into a continuation: the executor runs it and
        // hands the continuation its YIELD, never the description itself —
        // and never a failure, which the bind skips before the call. the
        // piped value's own failure bits short-circuit at the call site, so
        // they reach the result directly. and the pipe expression's own value
        // is then a description (the bind chain the executor later runs), so
        // DESC rides along too — dropping it once let a downstream pipe
        // direct-apply a real description as if it were the yield.
        piped_bits = (arg_sets[0] & FAIL) | DESC;
        arg_sets[0] = (arg_sets[0] & !DESC & !FAIL) | desc_yield_of(ctx, &args[0]);
    }
    let piped_bits = piped_bits;
    // an applied lambda is a binding in disguise — the fusion pass emits
    // these — and skipping its body hid every call inside it from the param
    // fixpoint, which is how a list-taking callee once "proved" int-only and
    // crossed the ABI as a smuggled pointer. Step the beta: bind the
    // argument sets and walk the body.
    if let Expr::Lambda { params, body, .. } = head {
        let mut inner = env.child(params.len());
        for ((p, _), set) in params.iter().zip(arg_sets.iter()) {
            inner.insert(p, *set & !FAIL);
        }
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return eval_expr(ctx, body, &mut inner) | fails | piped_bits;
    }
    let Expr::Ident(name, _) = head else {
        let _ = eval_expr(ctx, head, env);
        return TOP | piped_bits;
    };
    if env.contains_key(name.as_str()) {
        return TOP | piped_bits; // calling a local function value
    }
    if name == "if" {
        let cond_fail = arg_sets[0] & FAIL;
        return arg_sets[1] | arg_sets[2] | cond_fail | piped_bits;
    }
    if name == "err" {
        return ERR | piped_bits;
    }
    if name == "print" {
        return DESC | (arg_sets[0] & FAIL) | piped_bits;
    }
    if let Some(&idx) = ctx.type_names.get(name.as_str()) {
        // constructing a declared type: grow each field's set by this arg's,
        // dropping failures (a failing arg makes construction propagate, so the
        // field itself only ever holds the successful value's type)
        for (fi, argset) in arg_sets.iter().enumerate() {
            if let Some(slot) = ctx.type_fields[idx].get_mut(fi) {
                let refined = *slot | (*argset & !FAIL);
                if refined != *slot {
                    *slot = refined;
                    ctx.changed = true;
                    for &reader in &ctx.field_readers[idx] {
                        ctx.dirty[reader] = true;
                        ctx.dirty_next[reader] = true;
                    }
                }
            }
        }
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return REC | fails | piped_bits;
    }
    if name == "entry" {
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return REC | fails | piped_bits;
    }
    if let Some(&(start, end)) = ctx.groups.get(&(name.as_str(), args.len())) {
        let (start, end) = (start as usize, end as usize);
        let mut out: Set = 0;
        // pass-through: a failure in arg `pos` reaches the result only when no arm
        // catches it there. an arm whose pattern is `none`/`(err _)` handles that
        // failure (e.g. `_is_ws none -> false`), so it must not contaminate the
        // result — that spurious `none` is what kept scanner positions off `int`.
        for (pos, arg) in arg_sets.iter().enumerate() {
            let caught = ctx.group_members[start..end].iter().fold(0, |acc, &i| {
                acc | ctx.program.fns[i].params.get(pos).map_or(0, pattern_catches)
            });
            out |= (arg & FAIL) & !caught;
        }
        for k in start..end {
            let i = ctx.group_members[k];
            for (p, set) in arg_sets.iter().enumerate() {
                widen_param(ctx, i, p, *set);
            }
            mark_reader(ctx, i);
            out |= ctx.returns[i];
        }
        return out | piped_bits;
    }
    builtin_set(name, arg_sets) | piped_bits
}

/// What a description's execution hands a bound continuation, syntactically:
/// the yield of the lexical description expression, failures stripped (the
/// bind skips them before the continuation runs). Anything unrecognized is
/// conservatively any-non-failure.
/// desc_yield with one level of binding lookthrough: an ident that names a
/// tracked desc-valued local answers with that local's recorded yield.
fn desc_yield_of(ctx: &Ctx, e: &Expr) -> Set {
    if let Expr::Ident(name, _) = e {
        if let Some(y) = ctx.yields.get(name.as_str()) {
            return *y;
        }
    }
    desc_yield(e)
}

fn desc_yield(e: &Expr) -> Set {
    fn base(n: &str) -> &str {
        let n = n.strip_prefix("builtin_").unwrap_or(n);
        crate::ast::bare_name(n)
    }
    match e {
        // the io constants referenced bare: stdin yields the input string,
        // args the argument list
        Expr::Ident(n, _) if base(n) == "stdin" => STR,
        Expr::Ident(n, _) if base(n) == "args" => LIST,
        Expr::Ident(n, _) if base(n) == "now" => INT,
        // an `if` yields whichever branch runs
        Expr::App { head, args, piped: false, .. }
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == "if") && args.len() == 3 =>
        {
            desc_yield(&args[1]) | desc_yield(&args[2])
        }
        Expr::App { head, piped: false, .. } => match head.as_ref() {
            Expr::Ident(n, _) if matches!(base(n), "read_file" | "stdin") => STR,
            // status, stdout, stderr — the std wrapper reads them into a record
            Expr::Ident(n, _) if base(n) == "run" => LIST,
            // the handle a later kill names
            Expr::Ident(n, _) if base(n) == "start" => INT,
            Expr::Ident(n, _) if base(n) == "args" => LIST,
            Expr::Ident(n, _) if base(n) == "random" => INT,
            // an unset variable yields none, which is a value the consumer
            // dispatches on rather than a failure it has to trap
            Expr::Ident(n, _) if base(n) == "env" => STR | NONE,
            Expr::Ident(n, _) if matches!(base(n), "exists" | "is_dir") => TRUE | FALSE,
            Expr::Ident(n, _) if base(n) == "list_dir" => LIST,
            Expr::Ident(n, _) if base(n) == "now" => INT,
            // an unset variable yields none, which is a value the consumer
            // dispatches on rather than a failure it has to trap
            Expr::Ident(n, _)
                if matches!(
                    base(n),
                    "print"
                        | "write"
                        | "write_err"
                        | "write_file"
                        | "make_dir"
                        | "sleep"
                        | "net_write"
                        | "net_close"
                ) =>
            {
                0
            }
            _ => TOP & !FAIL,
        },
        // `a >> b` yields what its right side yields
        Expr::Seq(_, b, _) => desc_yield(b),
        // a join yields nothing a continuation would see
        Expr::Join { .. } => 0,
        Expr::Guard { early, rest, .. } => {
            let rest_yield = match rest.last() {
                Some(Stmt::Expr(e)) => desc_yield(e),
                _ => TOP & !FAIL,
            };
            desc_yield(early) | rest_yield
        }
        _ => TOP & !FAIL,
    }
}

pub fn builtin_set(name: &str, args: &[Set]) -> Set {
    let name = name.strip_prefix("builtin_").unwrap_or(name);
    let fails: Set = args.iter().fold(0, |acc, s| acc | (s & FAIL));
    match name {
        "at" => {
            let mut out = fails | NONE;
            if args[0] & BYTES != 0 {
                out |= INT;
            }
            if args[0] & (LIST | MAP) != 0 {
                out |= TOP & !FAIL;
            }
            if args[0] & STR != 0 {
                out |= STR;
            }
            out
        }
        "append" => BYTES | fails,
        "is_desc" => BOOL | fails,
        "bytes" => BYTES | fails,
        "to_bytes" => BYTES | ERR | fails,
        "find2" => INT | fails,
        "find2_below" => INT | fails,
        "slice" => (args[0] & (BYTES | LIST | STR)) | fails,
        "utf8" => STR | ERR | fails,
        "render_value" => STR | fails,
        "length" => INT | fails,
        "push" | "concat" | "chars" | "split" | "entries" | "sort" | "filter" => LIST | fails,
        "map" => LIST | fails,
        "put" => MAP | fails,
        "join" => STR | fails,
        "to_int" => INT | ERR | fails,
        "to_float" => FLOAT | ERR | fails,
        "from_code" => STR | ERR | fails,
        "char_code" => INT | fails,
        "sum" => INT | fails,
        "bit_and" | "bit_or" | "bit_xor" | "bit_not" | "bit_shl" | "bit_shr" => INT | fails,
        "sqrt" => FLOAT | fails,
        "round" => INT | fails,
        // a worded chain step answers a description when its subject is one,
        // and whatever its callback answers when the subject has settled
        "bind" | "rescue" | "annotate" => TOP,
        "read_file" | "write" | "write_err" | "write_file" | "make_dir" | "sleep" | "random"
        | "env" | "exists" | "is_dir" | "list_dir" | "now" | "run" | "start" | "kill"
        | "listen" | "net_port" | "accept" | "net_read" | "net_write" | "net_close" => DESC | fails,
        _ => TOP,
    }
}
