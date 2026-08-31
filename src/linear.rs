//! Which `push` call sites operate on a uniquely-owned list, so the runtime can
//! extend it in place instead of allocating a fresh header per element. This is
//! the uniqueness layer of Perceus: the accumulator threaded through the JSON
//! scanner is created once as `[]` and moved from `push` to `push`, never
//! duplicated, so its header refcount is always one and the copy is pure waste.
//!
//! Unsoundness here is memory corruption — mutating a list another reference
//! still sees — so the analysis is conservative. A push is marked in place only
//! when its list argument is provably unique: it traces back to a fresh `[]`
//! through a chain in which every step is *moved* (used exactly once) and never
//! aliased. Anything the analysis can't follow leaves the push allocating.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt, TemplatePart};
use crate::hash::{Map as HashMap, Set as HashSet};

/// Push call sites, keyed `(file, line, col)`, whose list argument is uniquely
/// owned and may be extended in place.
pub fn in_place_pushes(program: &Program) -> HashSet<(String, usize, usize)> {
    let analysis = Analysis::new(program);
    let mut out = HashSet::default();
    for decl in real_fns(program) {
        collect_pushes(&analysis, decl, &decl.body, &mut out);
    }
    out
}

/// The declarations an analysis may reason from.
///
/// Importing a qualified name enrolls a bare-named clone of the same
/// declaration so both spellings dispatch, and the clone carries the
/// original's span. Two names for one function is not two functions: the
/// clone is called by nobody, so every call-site test over it is vacuously
/// true, and a parameter the real declaration refuses is granted to its
/// twin. In-place sites are keyed by source position, which the twins share,
/// so that grant reaches the code the real name emits — an accumulator held
/// by two owners, written through one of them.
fn real_fns(program: &Program) -> impl Iterator<Item = &FnDecl> {
    program.fns.iter().filter(|d| !d.synthetic)
}

/// Every spelling the module graph gives std/list's `fold`: bare in its own
/// module, qualified once per import hop, and the enrolled clones of each.
/// The accumulator dispensation keys on this set because a fixed spelling
/// list cannot know how deeply an importer's importer qualified the name.
pub fn fold_spellings(program: &Program) -> HashSet<String> {
    program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list") && d.params.len() == 3)
        .filter(|d| crate::ast::split_qual(&d.name).map(|(_, s)| s).unwrap_or(&d.name) == "fold")
        .map(|d| d.name.clone())
        .collect()
}

struct Analysis<'a> {
    program: &'a Program,
    /// Declared type names, so a constructor call can be recognised.
    types: HashSet<String>,
    /// (function name, arity, param index) positions that receive a uniquely
    /// owned list at every call site and are moved (used at most once) in body.
    linear_params: HashSet<(String, usize, usize)>,
    /// (function name, arity) groups whose result is a freshly-built unique list.
    returns_unique: HashSet<(String, usize)>,
    /// The spellings that resolve to std/list's `fold` in this program.
    folds: HashSet<String>,
}

impl<'a> Analysis<'a> {
    fn new(program: &'a Program) -> Self {
        // Linearity is cyclically self-supporting (the accumulator's uniqueness
        // rests on the next hop's, which rests back on it), so this is a
        // *greatest* fixpoint: assume every parameter is linear and every group
        // returns a unique list, then remove any the code disproves. Removal is
        // monotone, so it converges, and a value stays "unique" only if nothing
        // ever aliases it — the sound direction for an in-place mutation.
        let types: HashSet<String> = program.types.iter().map(|t| t.name.clone()).collect();
        let folds = fold_spellings(program);
        let mut a = Analysis {
            program,
            types,
            linear_params: HashSet::default(),
            returns_unique: HashSet::default(),
            folds,
        };
        for decl in real_fns(program) {
            a.returns_unique.insert((decl.name.clone(), decl.params.len()));
            for i in 0..decl.params.len() {
                a.linear_params.insert((decl.name.clone(), decl.params.len(), i));
            }
        }
        a.fixpoint();
        a
    }

    fn fixpoint(&mut self) {
        loop {
            let mut changed = false;
            let drop_params: Vec<_> = self
                .linear_params
                .iter()
                .filter(|(name, arity, i)| !self.param_is_linear(name, *arity, *i))
                .cloned()
                .collect();
            for k in drop_params {
                self.linear_params.remove(&k);
                changed = true;
            }
            let drop_groups: Vec<_> = self
                .returns_unique
                .iter()
                .filter(|(name, arity)| {
                    !self.group(name, *arity).iter().all(
                        |d| matches!(d.body.last(), Some(Stmt::Expr(e)) if self.unique_list(e, d)),
                    )
                })
                .cloned()
                .collect();
            for k in drop_groups {
                self.returns_unique.remove(&k);
                changed = true;
            }
            if !changed {
                return;
            }
        }
    }

    fn group(&self, name: &str, arity: usize) -> Vec<&'a FnDecl> {
        self.program.fns.iter().filter(|d| d.name == name && d.params.len() == arity).collect()
    }

    fn param_is_linear(&self, name: &str, arity: usize, i: usize) -> bool {
        // In every arm the parameter must be moved: a plain Var used at most
        // once, or a wildcard (dropped, used zero times). Anything else — a
        // literal discriminator, a destructure — can't be a linear accumulator.
        for decl in self.group(name, arity) {
            match decl.params.get(i) {
                Some(Pattern::Var(pname, _)) => {
                    if effective_uses(pname, &decl.body) > 1 {
                        return false;
                    }
                }
                Some(Pattern::Wildcard(_)) => {}
                _ => return false,
            }
        }
        // Every call site must pass a uniquely-owned list at position i.
        self.program.fns.iter().all(|caller| {
            caller.body.iter().all(|stmt| {
                let e = match stmt {
                    Stmt::Bind { expr, .. } => expr,
                    Stmt::Expr(e) => e,
                    Stmt::Set { value, .. } => value,
                };
                self.callsites_unique(caller, e, name, arity, i)
            })
        })
    }

    /// Every call site of this group hands over a uniquely-owned value at `i`.
    ///
    /// This is the caller half of `param_is_linear`, asked on its own. The
    /// other half — used at most once — is deliberately not asked, because a
    /// record read twice for its fields is used twice and is still finished
    /// afterwards. The reuse site replaces it with a stricter local test: every
    /// mention of the parameter anywhere in the arm is inside the one
    /// expression that consumes it. Nothing here touches `linear_params`, so
    /// what a push or a put is allowed to do is unchanged.
    fn callers_hand_over(&self, name: &str, arity: usize, i: usize) -> bool {
        // An operator is called by syntax, so its calls are `a * b` rather
        // than a named call this walk can find — the same blindness
        // `escapes_as_value` refuses, arrived at from the other direction.
        // With no call sites to look at the question answers yes for free,
        // and a parameter marked an accumulator on that answer reaches the
        // runtime as a plain string it was promised it could write into.
        if crate::is_operator(name) || self.escapes_as_value(name, arity) {
            return false;
        }
        self.program.fns.iter().all(|caller| {
            caller.body.iter().all(|stmt| {
                let e = match stmt {
                    Stmt::Bind { expr, .. } => expr,
                    Stmt::Expr(e) => e,
                    Stmt::Set { value, .. } => value,
                };
                self.callsites_unique(caller, e, name, arity, i)
            })
        })
    }

    /// Is this group ever mentioned as a value rather than called?
    ///
    /// Handing a group to a fold, or currying it, means it is called from
    /// somewhere no call site in this program describes — and a question about
    /// what every caller hands over then has no calls to look at and answers
    /// yes for free. The seed a fold carries in never reaches a conversion
    /// site, so a parameter marked an accumulator on that answer arrives at
    /// the runtime as a plain string it was promised it could write into.
    fn escapes_as_value(&self, name: &str, arity: usize) -> bool {
        self.program.fns.iter().any(|d| {
            d.body.iter().any(|s| {
                let e = match s {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                    Stmt::Set { value, .. } => value,
                };
                mentioned_as_value(e, name, arity)
            })
        })
    }

    /// True unless some call to `name`/`arity` in `e` passes a non-unique list
    /// at position `i`.
    fn callsites_unique(&self, ctx: &FnDecl, e: &Expr, name: &str, arity: usize, i: usize) -> bool {
        self.callsites_unique_in(ctx, e, &Handover { name, arity, i }, None, None)
    }

    /// `scoped` names a value that is unique within `e` because of how `e` was
    /// reached — the parameter of a folding lambda, which the fold hands a
    /// uniquely-owned accumulator on every call. It is set only where that has
    /// been checked, so a lambda that captures or reorders its accumulator
    /// never gets it, and calls inside such a lambda stay non-unique.
    fn callsites_unique_in(
        &self,
        ctx: &FnDecl,
        e: &Expr,
        q: &Handover,
        scoped: Option<&str>,
        lambda_bound: Option<&HashSet<String>>,
    ) -> bool {
        let Handover { name, arity, i } = *q;
        if let Expr::App { head, args, .. } = e {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == name)
                && args.len() == arity
                && (!self.unique_in(&args[i], ctx, scoped)
                    || !hands_over_fresh(&args[i], lambda_bound))
            {
                return false;
            }
            // a fold's own lambda is where an accumulator legitimately arrives
            // as a parameter; check the lambda in that light, and everything
            // else in this expression without it
            if matches!(head.as_ref(), Expr::Ident(n, _) if self.folds.contains(n.as_str()))
                && args.len() == 3
            {
                if let (Expr::Lambda { params, body, .. }, true) =
                    (&args[2], self.folder_is_unique(&args[2], ctx, scoped))
                {
                    let inner = params.first().map(|(n, _)| n.as_str());
                    // `scoped` says the ACCUMULATOR is unique, which is the
                    // fold's own contract. Everything else the folder captures
                    // is still handed over once per element, so the folder's
                    // body carries the same capture set any other consumer's
                    // callback would — fusion writes `map` as a reducer applied
                    // where it stands inside exactly this lambda, and without
                    // the set the interpolation bug walks straight back in.
                    let mut bound: HashSet<String> =
                        lambda_bound.cloned().unwrap_or_else(HashSet::default);
                    bound.extend(params.iter().map(|(n, _)| n.clone()));
                    collect_bound_names(body, &mut bound);
                    return self.callsites_unique_in(ctx, &args[0], q, scoped, lambda_bound)
                        && self.callsites_unique_in(ctx, &args[1], q, scoped, lambda_bound)
                        && self.callsites_unique_in(ctx, body, q, inner, Some(&bound));
                }
            }
        }
        // A lambda in HEAD position is applied where it stands — `a . (x -> b)`
        // parses as `App { head: the lambda, args: [a] }` — so it runs exactly
        // once per evaluation of this expression, which is what the use count
        // already measures. The streaming-loop idiom this language is written
        // in (`io/write "" . (_ -> go n acc)`) is all of these, and treating
        // them as consumers' callbacks costs pendbench its in-place push and
        // two per cent of its instructions for nothing.
        //
        // `lambda_bound` travels through unchanged rather than being cleared:
        // fusion writes a map's reducer as an immediately-applied lambda inside
        // a folder, and that folder IS a consumer's callback.
        if let Expr::App { head, args, .. } = e {
            if let Expr::Lambda { body, .. } = head.as_ref() {
                return self.callsites_unique_in(ctx, body, q, scoped, lambda_bound)
                    && args
                        .iter()
                        .all(|a| self.callsites_unique_in(ctx, a, q, scoped, lambda_bound));
            }
        }
        // Anywhere else a lambda body runs when its consumer decides, and how
        // many times is not a question this walk can answer. `list/map xs
        // (n -> kept s n)` mentions `s` once and hands it over once per
        // element, so the one-use count that licenses a hand-over is measuring
        // the wrong thing. `walk_for_builder` already refuses to license a join
        // site inside a lambda for the same reason.
        //
        // What survives is the call whose argument the lambda MAKES rather than
        // captures: `_ -> walk (built 500 []) 1` hands over a fresh `[]` every
        // time it runs, and the twin fixture in tests/golden/mem depends on
        // that push staying in place. So the names bound by the lambda travel
        // down, and only an argument built entirely from them still counts.
        if let Expr::Lambda { params, body, .. } = e {
            let mut bound: HashSet<String> = lambda_bound.cloned().unwrap_or_else(HashSet::default);
            bound.extend(params.iter().map(|(n, _)| n.clone()));
            collect_bound_names(body, &mut bound);
            return self.callsites_unique_in(ctx, body, q, scoped, Some(&bound));
        }
        child_exprs(e)
            .into_iter()
            .all(|c| self.callsites_unique_in(ctx, c, q, scoped, lambda_bound))
    }

    /// Does `e` evaluate to a freshly-owned list (refcount would be one)?
    fn unique_list(&self, e: &Expr, ctx: &FnDecl) -> bool {
        self.unique_in(e, ctx, None)
    }

    /// Does this folding function hand back a uniquely-owned value?
    ///
    /// Only the shape an accumulator loop written with `fold` actually takes:
    /// a lambda whose body is one call receiving the lambda's own first
    /// parameter first and mentioning it nowhere else. A lambda that captures
    /// an accumulator from outside, or passes it second, could alias — and
    /// mutating in place under an alias is corruption, not a wrong answer.
    fn folder_is_unique(&self, f: &Expr, ctx: &FnDecl, scoped: Option<&str>) -> bool {
        let _ = (ctx, scoped);
        let Expr::Lambda { params, body, .. } = f else { return false };
        let Some((acc, _)) = params.first() else { return false };
        self.body_is_folder(body.as_ref(), acc)
    }

    /// Does every path through a folder's body either hand the accumulator
    /// back untouched or apply an accumulator operation to it?
    ///
    /// Fusion writes chains in two shapes and this answers both. A `map`
    /// becomes the reducer applied where it stands — an immediately-applied
    /// lambda wearing a wrapper. A `select` cannot be composed that way,
    /// because it has to decide whether to write at all, so it becomes a
    /// conditional whose arms are the write and the accumulator unchanged.
    /// Neither is a named call, which is all the older test looked for.
    fn body_is_folder(&self, body: &Expr, acc: &str) -> bool {
        match body {
            // an arm that declines to write hands the accumulator back
            Expr::Ident(n, _) if n == acc => true,
            Expr::App { head, args, .. } => match head.as_ref() {
                // the wrapper: the same folder, applied where it stands
                Expr::Lambda { params, body: inner, .. } => {
                    let Some((inner_acc, _)) = params.first() else { return false };
                    takes_acc_first(args, acc) && self.body_is_folder(inner, inner_acc)
                }
                // a conditional writes on some paths and not others, and the
                // test it branches on must not be holding the accumulator
                Expr::Ident(n, _) if n == "if" && args.len() == 3 => {
                    count_in_expr(acc, &args[0]) == 0
                        && self.body_is_folder(&args[1], acc)
                        && self.body_is_folder(&args[2], acc)
                }
                Expr::Ident(callee, _) => {
                    // the accumulator builtins are the ones whose result is
                    // unique when their first argument is, which the folder's
                    // contract already gives; they are not declarations, so
                    // `returns_unique` never held them. They also force their
                    // arguments, so a sibling may read the accumulator.
                    let builtin = (matches!(callee.as_str(), "push" | "append" | "builtin_append")
                        && args.len() == 2)
                        || (callee == "put" && args.len() == 3);
                    if !takes_acc_first_with(args, acc, builtin) {
                        return false;
                    }
                    builtin || self.returns_unique.contains(&(callee.clone(), args.len()))
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn unique_in(&self, e: &Expr, ctx: &FnDecl, scoped: Option<&str>) -> bool {
        self.unique_in_with(e, ctx, scoped, &[])
    }

    /// The same question, told which sibling expressions do not count as
    /// aliases. A builtin's arguments are all evaluated before the builtin
    /// runs, so a read of the map inside one of them is finished by the time
    /// the write happens — `put m k (bump m[k])` mentions `m` twice and holds
    /// it once. Only the consuming call's own arguments may be exempted, and
    /// only for a builtin whose arguments are forced.
    fn unique_in_with(
        &self,
        e: &Expr,
        ctx: &FnDecl,
        scoped: Option<&str>,
        exempt: &[&Expr],
    ) -> bool {
        match e {
            // a literal is interned or freshly written and an interpolation
            // allocates its result, so nothing else holds the value; an
            // interned one carries no spare capacity, which is what keeps a
            // write-through off a shared constant
            Expr::List(..) | Expr::MapLit(..) | Expr::Str(..) => true,
            // A fold threads its seed through the folding function and returns
            // what the last call gave back, so the result is unique when the
            // seed is and the folder returns unique from a unique first
            // argument. This is an accumulator loop written with `fold`
            // instead of recursion, and without it the encode path collapses
            // at `escape_able`, where the byte builder passes through one.
            Expr::App { head, args, .. }
                if matches!(head.as_ref(), Expr::Ident(n, _) if self.folds.contains(n.as_str()))
                    && args.len() == 3 =>
            {
                self.unique_in_with(&args[1], ctx, scoped, exempt)
                    && self.folder_is_unique(&args[2], ctx, scoped)
            }
            // a conditional yields a unique value when every arm does; the
            // encode loop returns `if done acc (step acc)` and neither arm
            // aliases, but an unknown head would have read it as opaque
            Expr::App { head, args, .. }
                if matches!(head.as_ref(), Expr::Ident(n, _) if n == "if") && args.len() == 3 =>
            {
                self.unique_in_with(&args[1], ctx, scoped, exempt)
                    && self.unique_in_with(&args[2], ctx, scoped, exempt)
            }
            Expr::Guard { early, rest, .. } => {
                let tail = matches!(rest.last(), Some(Stmt::Expr(e))
                    if self.unique_in_with(e, ctx, scoped, exempt));
                self.unique_in_with(early, ctx, scoped, exempt) && tail
            }
            Expr::App { head, args, .. } => match head.as_ref() {
                Expr::Ident(n, _)
                    if matches!(n.as_str(), "push" | "append" | "builtin_append")
                        && args.len() == 2 =>
                {
                    self.unique_in(&args[0], ctx, scoped)
                }
                Expr::Ident(n, _) if n == "put" && args.len() == 3 => {
                    self.unique_in(&args[0], ctx, scoped)
                }
                // `concat` always allocates a fresh list (k_b_concat), so its
                // result is uniquely owned regardless of its arguments.
                Expr::Ident(n, _)
                    if matches!(n.as_str(), "concat" | "text/concat" | "builtin_concat")
                        && args.len() == 2 =>
                {
                    true
                }
                // and a byte builder is born fresh — it is the seed the whole
                // encode chain is threaded from, so without it every
                // accumulator downstream fails to prove unique at the root
                Expr::Ident(n, _)
                    if matches!(n.as_str(), "bytes" | "text/bytes" | "builtin_bytes")
                        && args.len() == 1 =>
                {
                    true
                }
                // a constructor allocates its record, so nobody else holds
                // the result — the same fact as a list or map literal, which
                // was missing only because a type is not a declaration and
                // `returns_unique` is built from declarations
                Expr::Ident(n, _) if self.types.contains(n.as_str()) => true,
                Expr::Ident(n, _) => self.returns_unique.contains(&(n.clone(), args.len())),
                _ => false,
            },
            Expr::Ident(name, _) => self.is_unique_source(name, ctx, scoped, exempt),
            _ => false,
        }
    }

    /// A variable holding a uniquely-owned list: a linear parameter, or a local
    /// bound to a unique list — in either case used at most once in the body, so
    /// no alias outlives the move.
    fn is_unique_source(
        &self,
        var: &str,
        ctx: &FnDecl,
        scoped: Option<&str>,
        exempt: &[&Expr],
    ) -> bool {
        // a folding lambda's parameter: the fold hands it a uniquely-owned
        // accumulator on every call, and the lambda was checked before this
        // name was put in scope
        if scoped == Some(var) {
            return true;
        }
        let discounted: usize = exempt.iter().map(|e| count_in_expr(var, e)).sum();
        if effective_uses(var, &ctx.body).saturating_sub(discounted) > 1 {
            return false;
        }
        if let Some(idx) =
            ctx.params.iter().position(|p| matches!(p, Pattern::Var(n, _) if n == var))
        {
            return self.linear_params.contains(&(ctx.name.clone(), ctx.params.len(), idx));
        }
        // a local binding `var = e`
        for stmt in &ctx.body {
            if let Stmt::Bind { pattern: Pattern::Var(n, _), expr } = stmt {
                if n == var {
                    return self.unique_in_with(expr, ctx, scoped, exempt);
                }
            }
        }
        false
    }
}

/// Collect in-place push sites in `body`: a `push` whose list argument is a
/// uniquely-owned source used exactly once here.
fn collect_pushes(
    a: &Analysis,
    decl: &FnDecl,
    body: &[Stmt],
    out: &mut HashSet<(String, usize, usize)>,
) {
    for stmt in body {
        let e = match stmt {
            Stmt::Bind { expr, .. } => expr,
            Stmt::Expr(e) => e,
            Stmt::Set { value, .. } => value,
        };
        walk_for_push(a, decl, e, out);
    }
}

/// Find the writes inside a folder body, following the same two shapes the
/// check accepts: the wrapper fusion leaves, and a conditional's arms.
fn mark_folder_body(
    a: &Analysis,
    decl: &FnDecl,
    body: &Expr,
    out: &mut HashSet<(String, usize, usize)>,
    acc: Option<&str>,
) {
    if let Expr::App { head, args, .. } = body {
        match head.as_ref() {
            Expr::Lambda { params, body: inner, .. } => {
                let inner_acc = params.first().map(|(n, _)| n.as_str());
                mark_folder_body(a, decl, inner, out, inner_acc);
                return;
            }
            Expr::Ident(n, _) if n == "if" && args.len() == 3 => {
                mark_folder_body(a, decl, &args[1], out, acc);
                mark_folder_body(a, decl, &args[2], out, acc);
                return;
            }
            _ => {}
        }
    }
    walk_for_push_in(a, decl, body, out, acc);
}

/// The accumulator arrives first and nowhere else, so the call consumes it
/// exactly once.
fn takes_acc_first(args: &[Expr], acc: &str) -> bool {
    takes_acc_first_with(args, acc, false)
}

/// The accumulator arrives first, and appears nowhere else — unless the callee
/// is a builtin, which forces every argument before it runs. Then a read in a
/// sibling argument of the very call that writes is finished before the write:
/// `tally` is `fold coll {} (m x -> put m x (bump m[x]))`, which mentions the
/// map twice and holds it once. That is the same discount `effective_uses`
/// already gives a bare call site, and without it a tally copied its map once
/// an element.
fn takes_acc_first_with(args: &[Expr], acc: &str, forced_args: bool) -> bool {
    if !matches!(args.first(), Some(Expr::Ident(n, _)) if n == acc) {
        return false;
    }
    forced_args || args[1..].iter().map(|a| count_in_expr(acc, a)).sum::<usize>() == 0
}

fn walk_for_push(a: &Analysis, decl: &FnDecl, e: &Expr, out: &mut HashSet<(String, usize, usize)>) {
    walk_for_push_in(a, decl, e, out, None)
}

fn walk_for_push_in(
    a: &Analysis,
    decl: &FnDecl,
    e: &Expr,
    out: &mut HashSet<(String, usize, usize)>,
    scoped: Option<&str>,
) {
    if let Expr::App { head, args, span, .. } = e {
        // push extends a list, append extends a byte builder, and each writes
        // into its own frontier when nobody else holds the value
        if matches!(head.as_ref(), Expr::Ident(n, _) if matches!(n.as_str(), "push" | "append" | "builtin_append"))
            && args.len() == 2
            && a.unique_in(&args[0], decl, scoped)
        {
            out.insert((decl.file.clone(), span.line as usize, span.col as usize));
        }
        // put extends a map the same way, one arity over
        if matches!(head.as_ref(), Expr::Ident(n, _) if n == "put")
            && args.len() == 3
            && a.unique_in_with(&args[0], decl, scoped, &[&args[1], &args[2]])
        {
            out.insert((decl.file.clone(), span.line as usize, span.col as usize));
        }
        // inside a validated folder the accumulator arrives as the lambda's
        // own parameter, and every push or append there is on a unique value
        if matches!(head.as_ref(), Expr::Ident(n, _) if a.folds.contains(n.as_str()))
            && args.len() == 3
            && a.folder_is_unique(&args[2], decl, scoped)
        {
            if let Expr::Lambda { params, body, .. } = &args[2] {
                let inner = params.first().map(|(n, _)| n.as_str());
                walk_for_push_in(a, decl, &args[0], out, scoped);
                walk_for_push_in(a, decl, &args[1], out, scoped);
                mark_folder_body(a, decl, body, out, inner);
                return;
            }
        }
    }
    for child in child_exprs(e) {
        // A lambda body is not walked here. Whether a write inside one is safe
        // depends on when the lambda runs, and only the shapes handled above
        // answer that: a folder is applied at once, per element, to the
        // accumulator the fold owns. Anything else may be a continuation that
        // escapes into a suspended effect and runs while the frame that
        // "used the value once" is still holding it — `xs . (p -> push acc p)`
        // reads as a single use of `acc` and is not one.
        if matches!(child, Expr::Lambda { .. }) {
            continue;
        }
        walk_for_push_in(a, decl, child, out, scoped);
    }
}

/// Count value occurrences of `var` in `body` (pattern bindings don't count).
/// Uses of `var` that could still hold the value when a write happens.
///
/// A builtin evaluates every argument before it runs, so a read of the map
/// inside a sibling argument of the very call that consumes it is finished
/// before the write: `put m k (bump m[k])` mentions `m` twice and holds it
/// once. Those mentions are discounted here, and only those — a use anywhere
/// else still counts, so a map read after the put, or passed elsewhere, keeps
/// the value shared and the write copying.
fn effective_uses(var: &str, body: &[Stmt]) -> usize {
    let total = count_uses(var, body);
    let mut discounted = 0;
    for stmt in body {
        let e = match stmt {
            Stmt::Bind { expr, .. } => expr,
            Stmt::Expr(e) => e,
            Stmt::Set { value, .. } => value,
        };
        discounted += consumed_sibling_uses(var, e);
    }
    total.saturating_sub(discounted)
}

/// Uses of `var` inside the sibling arguments of a call that consumes `var`
/// as its first argument.
fn consumed_sibling_uses(var: &str, e: &Expr) -> usize {
    let mut n = 0;
    if let Expr::App { head, args, .. } = e {
        let consuming = matches!(head.as_ref(), Expr::Ident(h, _)
            if matches!(h.as_str(), "put" | "push" | "append" | "builtin_append"));
        if consuming && matches!(args.first(), Some(Expr::Ident(first, _)) if first == var) {
            for sibling in &args[1..] {
                n += count_in_expr(var, sibling);
            }
        }
        for arg in args {
            n += consumed_sibling_uses(var, arg);
        }
        n += consumed_sibling_uses(var, head);
    } else {
        crate::for_each_child(e, |child| n += consumed_sibling_uses(var, child));
    }
    n
}

fn count_uses(var: &str, body: &[Stmt]) -> usize {
    let mut n = 0;
    for stmt in body {
        let e = match stmt {
            Stmt::Bind { expr, .. } => expr,
            Stmt::Expr(e) => e,
            Stmt::Set { value, .. } => value,
        };
        n += count_in_expr(var, e);
    }
    n
}

/// How many times `var` is used on the busiest path through `e`.
///
/// Summing every occurrence counts a conditional's branches as if both ran,
/// which reads a threaded accumulator as aliased and stops it being extended
/// in place. `encode_items` is the shape that matters — an `if` whose base
/// case returns the accumulator and whose step passes it on. `acc` appears
/// twice there and is used once — the base case returns it or the step
/// passes it on, never both. Taking the larger branch rather than their sum is
/// what a move actually is, and it is still conservative: no path uses the
/// value more often than the number this returns.
fn count_in_expr(var: &str, e: &Expr) -> usize {
    let here = matches!(e, Expr::Ident(n, _) if n == var) as usize;
    // `if c a b`: the condition always runs, exactly one arm follows it
    if let Expr::App { head, args, .. } = e {
        if matches!(head.as_ref(), Expr::Ident(n, _) if n == "if") && args.len() == 3 {
            let cond = count_in_expr(var, &args[0]);
            let taken = count_in_expr(var, &args[1]).max(count_in_expr(var, &args[2]));
            return here + cond + taken;
        }
    }
    // `return early if cond` — the guard fires or the rest runs
    if let Expr::Guard { cond, early, rest, .. } = e {
        let after: usize = rest
            .iter()
            .map(|st| match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    count_in_expr(var, expr)
                }
            })
            .sum();
        return here + count_in_expr(var, cond) + count_in_expr(var, early).max(after);
    }
    here + child_exprs(e).into_iter().map(|c| count_in_expr(var, c)).sum::<usize>()
}

fn child_exprs(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::Partial(..) => Vec::new(),
        Expr::Field { base, .. } => vec![base.as_ref()],
        Expr::Upcast { expr, .. } => vec![expr.as_ref()],
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts
            .iter()
            .map(|st| match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
            })
            .collect(),
        Expr::App { head, args, .. } => {
            let mut v: Vec<&Expr> = vec![head.as_ref()];
            v.extend(args.iter());
            v
        }
        Expr::Index { base, index, .. } => vec![base.as_ref(), index.as_ref()],
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            vec![lhs.as_ref(), rhs.as_ref()]
        }
        Expr::Guard { cond, early, rest, .. } => {
            let mut v: Vec<&Expr> = vec![cond.as_ref(), early.as_ref()];
            v.extend(rest.iter().map(|s| match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
            }));
            v
        }
        Expr::Seq(a, b, _) => vec![a.as_ref(), b.as_ref()],
        Expr::Lambda { body, .. } => vec![body.as_ref()],
        Expr::List(items, _) => items.iter().collect(),
        Expr::MapLit(pairs, _) => pairs.iter().flat_map(|(k, v)| [k, v]).collect(),
        Expr::Str(parts, _) => parts
            .iter()
            .filter_map(|p| match p {
                crate::ast::TemplatePart::Interp(x) => Some(x),
                crate::ast::TemplatePart::Lit(_) => None,
            })
            .collect(),
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => vec![],
    }
}

/// Constructor sites that may build into a record that is finished.
///
/// `shift (n - 1) (point (p.x + 1) p.y)` reads every field it needs before the
/// constructor runs, so by then `p` is done with. When every caller hands the
/// parameter over uniquely, and the only mentions of it in the arm are those
/// reads, its storage is free and the new record can go there.
///
/// The runtime keeps the other half: it writes through only into a record of
/// the same width, so a mismatch — or the shared zero-field marker — falls
/// back to allocating.
pub fn reusable_records(program: &Program) -> HashMap<(String, usize, usize), String> {
    let analysis = Analysis::new(program);
    let types: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    let mut out = HashMap::default();
    for decl in real_fns(program) {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            walk_for_reuse(&analysis, decl, e, &types, &mut out);
        }
    }
    out
}

fn walk_for_reuse(
    a: &Analysis,
    decl: &FnDecl,
    e: &Expr,
    types: &HashSet<&str>,
    out: &mut HashMap<(String, usize, usize), String>,
) {
    // a lambda runs when somebody else decides, so what is finished inside one
    // is not knowable from here
    if matches!(e, Expr::Lambda { .. }) {
        return;
    }
    if let Expr::App { head, args, span, .. } = e {
        if let Expr::Ident(name, _) = head.as_ref() {
            if types.contains(name.as_str()) && !args.is_empty() {
                if let Some(victim) = sole_finished_record(a, decl, args) {
                    out.insert((decl.file.clone(), span.line as usize, span.col as usize), victim);
                }
            }
        }
    }
    for child in child_exprs(e) {
        walk_for_reuse(a, decl, child, types, out);
    }
}

/// The one parameter these arguments read, when it is read nowhere else in the
/// arm and every caller hands it over.
fn sole_finished_record(a: &Analysis, decl: &FnDecl, args: &[Expr]) -> Option<String> {
    let arity = decl.params.len();
    let mut candidate: Option<String> = None;
    for (i, pattern) in decl.params.iter().enumerate() {
        let Pattern::Var(name, _) = pattern else { continue };
        let here: usize = args.iter().map(|arg| count_in_expr(name, arg)).sum();
        if here == 0 {
            continue;
        }
        let everywhere: usize = decl
            .body
            .iter()
            .map(|s| match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => count_in_expr(name, expr),
                Stmt::Set { value, .. } => count_in_expr(name, value),
            })
            .sum();
        // read here and nowhere else, or somebody outlives the constructor
        if here != everywhere || !a.callers_hand_over(&decl.name, arity, i) {
            continue;
        }
        if candidate.is_some() {
            return None;
        }
        candidate = Some(name.clone());
    }
    candidate
}

/// Every mention of `name` in `e` that is not the head of an application of
/// exactly `arity` arguments — a bare reference, or a partial application.
fn mentioned_as_value(e: &Expr, name: &str, arity: usize) -> bool {
    if let Expr::Ident(n, _) = e {
        return n == name;
    }
    if let Expr::App { head, args, .. } = e {
        let called = matches!(head.as_ref(), Expr::Ident(n, _) if n == name) && args.len() == arity;
        let escaping_head = match called {
            true => false,
            false => mentioned_as_value(head, name, arity),
        };
        return escaping_head || args.iter().any(|a| mentioned_as_value(a, name, arity));
    }
    child_exprs(e).into_iter().any(|c| mentioned_as_value(c, name, arity))
}

/// The hand-over being asked about: which group, at which arity, at which
/// parameter. Fixed for the whole walk, so it travels as one value rather than
/// three that every recursive call has to thread by hand.
struct Handover<'a> {
    name: &'a str,
    arity: usize,
    i: usize,
}

/// Every name a lambda body binds for itself: the locals of its blocks and
/// guards, added to the parameters the caller already put in the set.
fn collect_bound_names(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                if let Stmt::Bind { pattern: Pattern::Var(n, _), .. } = stmt {
                    out.insert(n.clone());
                }
            }
        }
        Expr::Lambda { params, .. } => out.extend(params.iter().map(|(n, _)| n.clone())),
        _ => {}
    }
    for child in child_exprs(e) {
        collect_bound_names(child, out);
    }
}

/// Outside a lambda, every hand-over is the one the use count measured, so this
/// answers yes. Inside one, only a value the lambda builds from what it binds
/// itself is fresh on each run — a captured name is handed over once per
/// invocation and the count says nothing about how many that is.
fn hands_over_fresh(arg: &Expr, lambda_bound: Option<&HashSet<String>>) -> bool {
    let Some(bound) = lambda_bound else { return true };
    let mut names = Vec::new();
    collect_idents_here(arg, &mut names);
    names.iter().all(|n| bound.contains(n))
}

fn collect_idents_here(e: &Expr, out: &mut Vec<String>) {
    if let Expr::Ident(name, _) = e {
        out.push(name.clone());
    }
    for child in child_exprs(e) {
        collect_idents_here(child, out);
    }
}

/// A set of source positions, or of (group, arity, index) triples — the two
/// happen to have the same shape.
pub type Sites = HashSet<(String, usize, usize)>;

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
pub fn string_builders(program: &Program) -> (Sites, Sites, Sites) {
    let analysis = Analysis::new(program);
    let mut sites = HashSet::default();
    let mut accs = HashSet::default();
    for decl in real_fns(program) {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            walk_for_builder(&analysis, decl, e, &mut sites, &mut accs);
        }
    }
    // A parameter that forwards the accumulator on is carrying one too, so it
    // joins the set whose callers convert a seed — the conversion moves out to
    // where the value enters the cycle, rather than happening on every hop.
    let (accs, carried) = carried_args(&analysis, program, &sites, accs);
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
fn carried_args(a: &Analysis, program: &Program, joins: &Sites, accs: Sites) -> (Sites, Sites) {
    let mut carrying = accs;
    loop {
        let before = carrying.len();
        for decl in real_fns(program) {
            let arity = decl.params.len();
            for (j, _) in param_names(decl) {
                if carrying.contains(&(decl.name.clone(), arity, j)) {
                    continue;
                }
                // Every arm of the group, not just this one: the set is keyed by
                // group, so an arm that aliases the parameter or answers it as a
                // plain string would inherit an exemption it cannot honour.
                let arms: Vec<&FnDecl> = real_fns(program)
                    .filter(|d| d.name == decl.name && d.params.len() == arity)
                    .collect();
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
                    && called_somewhere(program, &decl.name, arity)
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
                Stmt::Set { value, .. } => value,
            };
            collect_carried(&carrying, &built, decl, e, &mut out);
        }
    }
    (carrying, out)
}

/// Names this arm binds to the join itself — the builder made right here.
fn built_locals(joins: &Sites, decl: &FnDecl) -> HashSet<String> {
    decl.body
        .iter()
        .filter_map(|s| match s {
            Stmt::Bind { pattern: Pattern::Var(name, _), expr: Expr::Str(_, span) } => joins
                .contains(&(decl.file.clone(), span.line as usize, span.col as usize))
                .then(|| name.clone()),
            _ => None,
        })
        .collect()
}

/// Does any call site in the program name this group?
fn called_somewhere(program: &Program, name: &str, arity: usize) -> bool {
    fn in_expr(e: &Expr, name: &str, arity: usize) -> bool {
        if let Expr::App { head, args, .. } = e {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == name) && args.len() == arity {
                return true;
            }
        }
        child_exprs(e).into_iter().any(|c| in_expr(c, name, arity))
    }
    program.fns.iter().any(|d| {
        d.body.iter().any(|s| {
            let e = match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            in_expr(e, name, arity)
        })
    })
}

fn param_names(decl: &FnDecl) -> Vec<(usize, String)> {
    decl.params
        .iter()
        .enumerate()
        .filter_map(|(j, p)| match p {
            Pattern::Var(n, _) => Some((j, n.clone())),
            _ => None,
        })
        .collect()
}

/// Does this arm hand `name` on into a carrying position, and mention it
/// nowhere else?
fn forwards_into(carrying: &Sites, decl: &FnDecl, name: &str) -> bool {
    let everywhere: usize = decl
        .body
        .iter()
        .map(|s| match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => count_in_expr(name, expr),
            Stmt::Set { value, .. } => count_in_expr(name, value),
        })
        .sum();
    let mut found = false;
    for stmt in &decl.body {
        let e = match stmt {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            Stmt::Set { value, .. } => value,
        };
        walk_forwards(carrying, e, name, &mut found);
    }
    found && everywhere == 1
}

fn walk_forwards(carrying: &Sites, e: &Expr, name: &str, found: &mut bool) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(callee, _) = head.as_ref() {
            for (i, arg) in args.iter().enumerate() {
                if matches!(arg, Expr::Ident(n, _) if n == name)
                    && carrying.contains(&(callee.clone(), args.len(), i))
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
    carrying: &Sites,
    built: &HashSet<String>,
    decl: &FnDecl,
    e: &Expr,
    out: &mut Sites,
) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(callee, _) = head.as_ref() {
            for (i, arg) in args.iter().enumerate() {
                if !carrying.contains(&(callee.clone(), args.len(), i)) {
                    continue;
                }
                if let Expr::Ident(n, span) = arg {
                    let holds = built.contains(n)
                        || param_names(decl).into_iter().any(|(j, p)| {
                            &p == n && carrying.contains(&(decl.name.clone(), decl.params.len(), j))
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
    sites: &mut HashSet<(String, usize, usize)>,
    accs: &mut HashSet<(String, usize, usize)>,
) {
    // a lambda runs when somebody else decides
    if matches!(e, Expr::Lambda { .. }) {
        return;
    }
    if let Expr::Str(parts, span) = e {
        if parts.len() > 1 {
            if let Some(TemplatePart::Interp(Expr::Ident(name, _))) = parts.first() {
                if let Some(i) = builder_param(a, decl, name, e) {
                    sites.insert((decl.file.clone(), span.line as usize, span.col as usize));
                    accs.insert((decl.name.clone(), decl.params.len(), i));
                }
            }
        }
    }
    for child in child_exprs(e) {
        walk_for_builder(a, decl, child, sites, accs);
    }
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
            Stmt::Set { value, .. } => count_in_expr(name, value),
        })
        .sum();
    match inside == everywhere && a.callers_hand_over(&decl.name, arity, i) {
        true => Some(i),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::in_place_pushes;
    use std::path::Path;

    #[test]
    fn json_accumulator_pushes_flagged() {
        let program = crate::compile_module(Path::new("lib/json"), false).unwrap();
        let p = in_place_pushes(&program);
        assert!(
            p.iter().any(|(f, _, _)| f.ends_with("value.kso")),
            "array accumulator push should be in-place, got {p:?}"
        );
        assert!(
            p.iter().any(|(f, _, _)| f.ends_with("text.kso")),
            "string accumulator pushes should be in-place, got {p:?}"
        );
    }

    #[test]
    fn aliased_list_not_flagged() {
        // xs is pushed twice, so neither push uniquely owns it — mutating in
        // place would corrupt the other reference. Must stay allocating.
        let src = "fn dup xs\n  a = push xs 1\n  b = push xs 2\n  push a b\n\nmain = print \"{length (dup [1 2 3])}\"\n";
        let program = crate::compile("test.kso", src, false).unwrap();
        assert!(in_place_pushes(&program).is_empty(), "aliased pushes must not be marked in-place");
    }
}
