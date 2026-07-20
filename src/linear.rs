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
    for decl in &program.fns {
        collect_pushes(&analysis, decl, &decl.body, &mut out);
    }
    out
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
        let mut a = Analysis {
            program,
            linear_params: HashSet::new(),
            returns_unique: HashSet::new(),
        };
        for decl in &program.fns {
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
                    !self.group(name, *arity).iter().all(|d| {
                        matches!(d.body.last(), Some(Stmt::Expr(e)) if self.unique_list(e, d))
                    })
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
        self.program
            .fns
            .iter()
            .filter(|d| d.name == name && d.params.len() == arity)
            .collect()
    }

    fn param_is_linear(&self, name: &str, arity: usize, i: usize) -> bool {
        // In every arm the parameter must be moved: a plain Var used at most
        // once, or a wildcard (dropped, used zero times). Anything else — a
        // literal discriminator, a destructure — can't be a linear accumulator.
        for decl in self.group(name, arity) {
            match decl.params.get(i) {
                Some(Pattern::Var(pname, _)) => {
                    if count_uses(pname, &decl.body) > 1 {
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
                };
                self.callsites_unique(caller, e, name, arity, i)
            })
        })
    }

    /// True unless some call to `name`/`arity` in `e` passes a non-unique list
    /// at position `i`.
    fn callsites_unique(
        &self,
        ctx: &FnDecl,
        e: &Expr,
        name: &str,
        arity: usize,
        i: usize,
    ) -> bool {
        if let Expr::App { head, args, .. } = e {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == name)
                && args.len() == arity
                && !self.unique_list(&args[i], ctx)
            {
                return false;
            }
        }
        child_exprs(e)
            .into_iter()
            .all(|c| self.callsites_unique(ctx, c, name, arity, i))
    }

    /// Does `e` evaluate to a freshly-owned list (refcount would be one)?
    fn unique_list(&self, e: &Expr, ctx: &FnDecl) -> bool {
        match e {
            Expr::List(..) => true,
            Expr::App { head, args, .. } => match head.as_ref() {
                Expr::Ident(n, _) if n == "push" && args.len() == 2 => {
                    self.unique_list(&args[0], ctx)
                }
                // `concat` always allocates a fresh list (k_b_concat), so its
                // result is uniquely owned regardless of its arguments.
                Expr::Ident(n, _) if n == "concat" && args.len() == 2 => true,
                Expr::Ident(n, _) => self.returns_unique.contains(&(n.clone(), args.len())),
                _ => false,
            },
            Expr::Ident(name, _) => self.is_unique_source(name, ctx),
            _ => false,
        }
    }

    /// A variable holding a uniquely-owned list: a linear parameter, or a local
    /// bound to a unique list — in either case used at most once in the body, so
    /// no alias outlives the move.
    fn is_unique_source(&self, var: &str, ctx: &FnDecl) -> bool {
        if count_uses(var, &ctx.body) > 1 {
            return false;
        }
        if let Some(idx) = ctx.params.iter().position(
            |p| matches!(p, Pattern::Var(n, _) if n == var),
        ) {
            return self
                .linear_params
                .contains(&(ctx.name.clone(), ctx.params.len(), idx));
        }
        // a local binding `var = e`
        for stmt in &ctx.body {
            if let Stmt::Bind { pattern: Pattern::Var(n, _), expr } = stmt {
                if n == var {
                    return self.unique_list(expr, ctx);
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
        };
        walk_for_push(a, decl, e, out);
    }
}

fn walk_for_push(
    a: &Analysis,
    decl: &FnDecl,
    e: &Expr,
    out: &mut HashSet<(String, usize, usize)>,
) {
    if let Expr::App { head, args, span, .. } = e {
        if matches!(head.as_ref(), Expr::Ident(n, _) if n == "push")
            && args.len() == 2
            && a.unique_list(&args[0], decl)
        {
            out.insert((decl.file.clone(), span.line, span.col));
        }
    }
    for child in child_exprs(e) {
        walk_for_push(a, decl, child, out);
    }
}

/// Count value occurrences of `var` in `body` (pattern bindings don't count).
fn count_uses(var: &str, body: &[Stmt]) -> usize {
    let mut n = 0;
    for stmt in body {
        let e = match stmt {
            Stmt::Bind { expr, .. } => expr,
            Stmt::Expr(e) => e,
        };
        n += count_in_expr(var, e);
    }
    n
}

fn count_in_expr(var: &str, e: &Expr) -> usize {
    let here = matches!(e, Expr::Ident(n, _) if n == var) as usize;
    here + child_exprs(e).into_iter().map(|c| count_in_expr(var, c)).sum::<usize>()
}

fn child_exprs(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::Field { base, .. } => vec![base.as_ref()],
        Expr::App { head, args, .. } => {
            let mut v: Vec<&Expr> = vec![head.as_ref()];
            v.extend(args.iter());
            v
        }
        Expr::Index { base, index, .. } => vec![base.as_ref(), index.as_ref()],
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            vec![lhs.as_ref(), rhs.as_ref()]
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
        let src = "fn dup xs\n  a = push xs 1\n  b = push xs 2\n  concat a b\n\nmain = print \"{length (dup [1 2 3])}\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        assert!(
            in_place_pushes(&program).is_empty(),
            "aliased pushes must not be marked in-place"
        );
    }
}
