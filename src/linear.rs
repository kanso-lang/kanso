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

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use std::collections::HashSet;

/// Push call sites, keyed `(file, line, col)`, whose list argument is uniquely
/// owned and may be extended in place.
pub fn in_place_pushes(program: &Program) -> HashSet<(String, usize, usize)> {
    let analysis = Analysis::new(program);
    let mut out = HashSet::new();
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

struct Analysis<'a> {
    program: &'a Program,
    /// (function name, arity, param index) positions that receive a uniquely
    /// owned list at every call site and are moved (used at most once) in body.
    linear_params: HashSet<(String, usize, usize)>,
    /// (function name, arity) groups whose result is a freshly-built unique list.
    returns_unique: HashSet<(String, usize)>,
}

impl<'a> Analysis<'a> {
    fn new(program: &'a Program) -> Self {
        // Linearity is cyclically self-supporting (the accumulator's uniqueness
        // rests on the next hop's, which rests back on it), so this is a
        // *greatest* fixpoint: assume every parameter is linear and every group
        // returns a unique list, then remove any the code disproves. Removal is
        // monotone, so it converges, and a value stays "unique" only if nothing
        // ever aliases it — the sound direction for an in-place mutation.
        let mut a =
            Analysis { program, linear_params: HashSet::new(), returns_unique: HashSet::new() };
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

    /// True unless some call to `name`/`arity` in `e` passes a non-unique list
    /// at position `i`.
    fn callsites_unique(&self, ctx: &FnDecl, e: &Expr, name: &str, arity: usize, i: usize) -> bool {
        self.callsites_unique_in(ctx, e, name, arity, i, None)
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
        name: &str,
        arity: usize,
        i: usize,
        scoped: Option<&str>,
    ) -> bool {
        if let Expr::App { head, args, .. } = e {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == name)
                && args.len() == arity
                && !self.unique_in(&args[i], ctx, scoped)
            {
                return false;
            }
            // a fold's own lambda is where an accumulator legitimately arrives
            // as a parameter; check the lambda in that light, and everything
            // else in this expression without it
            if matches!(head.as_ref(), Expr::Ident(n, _) if matches!(n.as_str(), "fold" | "list/fold"))
                && args.len() == 3
            {
                if let (Expr::Lambda { params, body, .. }, true) =
                    (&args[2], self.folder_is_unique(&args[2], ctx, scoped))
                {
                    let inner = params.first().map(|(n, _)| n.as_str());
                    return self.callsites_unique_in(ctx, &args[0], name, arity, i, scoped)
                        && self.callsites_unique_in(ctx, &args[1], name, arity, i, scoped)
                        && self.callsites_unique_in(ctx, body, name, arity, i, inner);
                }
            }
        }
        child_exprs(e).into_iter().all(|c| self.callsites_unique_in(ctx, c, name, arity, i, scoped))
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
        let Expr::Lambda { params, body, .. } = f else { return false };
        let Some((acc, _)) = params.first() else { return false };
        let Expr::App { head, args, .. } = body.as_ref() else { return false };
        if !matches!(args.first(), Some(Expr::Ident(n, _)) if n == acc) {
            return false;
        }
        if args[1..].iter().map(|a| count_in_expr(acc, a)).sum::<usize>() > 0 {
            return false;
        }
        // Fusion composes a chain by applying the reducer where it stands, so
        // the folder it leaves behind is `(a v -> (a v -> push a v) a (f v))`.
        // An immediately-applied lambda handed the accumulator as its first
        // argument is the same folder wearing a wrapper: ask the question of
        // what it applies.
        if let Expr::Lambda { .. } = head.as_ref() {
            return self.folder_is_unique(head.as_ref(), ctx, scoped);
        }
        let Expr::Ident(callee, _) = head.as_ref() else { return false };
        let _ = (ctx, scoped);
        // The accumulator builtins are the ones whose result is unique when
        // their first argument is, which the folder's contract already gives.
        // They are not declarations, so `returns_unique` never held them.
        if matches!(callee.as_str(), "push" | "append" | "builtin_append") && args.len() == 2 {
            return true;
        }
        if callee == "put" && args.len() == 3 {
            return true;
        }
        self.returns_unique.contains(&(callee.clone(), args.len()))
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
            Expr::List(..) | Expr::MapLit(..) => true,
            // A fold threads its seed through the folding function and returns
            // what the last call gave back, so the result is unique when the
            // seed is and the folder returns unique from a unique first
            // argument. This is an accumulator loop written with `fold`
            // instead of recursion, and without it the encode path collapses
            // at `escape_able`, where the byte builder passes through one.
            Expr::App { head, args, .. }
                if matches!(head.as_ref(), Expr::Ident(n, _) if matches!(n.as_str(), "fold" | "list/fold"))
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
            out.insert((decl.file.clone(), span.line, span.col));
        }
        // put extends a map the same way, one arity over
        if matches!(head.as_ref(), Expr::Ident(n, _) if n == "put")
            && args.len() == 3
            && a.unique_in_with(&args[0], decl, scoped, &[&args[1], &args[2]])
        {
            out.insert((decl.file.clone(), span.line, span.col));
        }
        // inside a validated folder the accumulator arrives as the lambda's
        // own parameter, and every push or append there is on a unique value
        if matches!(head.as_ref(), Expr::Ident(n, _) if matches!(n.as_str(), "fold" | "list/fold"))
            && args.len() == 3
            && a.folder_is_unique(&args[2], decl, scoped)
        {
            if let Expr::Lambda { params, body, .. } = &args[2] {
                let mut inner = params.first().map(|(n, _)| n.as_str());
                walk_for_push_in(a, decl, &args[0], out, scoped);
                walk_for_push_in(a, decl, &args[1], out, scoped);
                // peel the wrapper fusion leaves: the applied lambda is the
                // folder, and its own first parameter is what holds the
                // accumulator inside it
                let mut target = body.as_ref();
                while let Expr::App { head, args: inner_args, .. } = target {
                    let Expr::Lambda { params: ps, body: b, .. } = head.as_ref() else { break };
                    if !matches!(inner_args.first(), Some(Expr::Ident(n, _)) if Some(n.as_str()) == inner)
                    {
                        break;
                    }
                    inner = ps.first().map(|(n, _)| n.as_str());
                    target = b.as_ref();
                }
                walk_for_push_in(a, decl, target, out, inner);
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
        for child in crate::expr_children(e) {
            n += consumed_sibling_uses(var, child);
        }
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
        let program = crate::compile("test.kso", src, true).unwrap();
        assert!(in_place_pushes(&program).is_empty(), "aliased pushes must not be marked in-place");
    }
}
