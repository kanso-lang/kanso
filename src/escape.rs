//! Escape analysis: which record types provably never escape, so a function
//! returning one can hand it back by value (in registers) instead of heap
//! allocating it. Unsoundness here is memory corruption, so the analysis is
//! deliberately conservative — a type is register-returnable only when *every*
//! syntactic occurrence of it is provably safe, and anything unrecognized
//! forces the type to "escapes".
//!
//! The safety argument leans on one structural fact: a register-returnable
//! type is never bound to a generic name. Its values exist only transiently as
//! expression results that flow construction -> return -> destructure, so there
//! is no variable a value could hide in and leak through. That lets the check
//! stay syntactic (no heap-flow dataflow) while remaining sound.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use std::collections::{HashMap, HashSet};

/// What codegen needs to hand register-returnable records back by value.
pub struct EscapeInfo {
    /// Register-returnable type name -> field count.
    pub field_count: HashMap<String, usize>,
    /// (function name, arity) groups whose result is a register-returnable
    /// type, mapped to that type's name.
    pub returns: HashMap<(String, usize), String>,
    /// (function name, arity, parameter index) positions that carry a
    /// register-returnable type (destructured as `(T ...)` by some arm),
    /// mapped to the type's name.
    pub carries: HashMap<(String, usize, usize), String>,
}

impl EscapeInfo {
    pub fn returns_ty(&self, name: &str, arity: usize) -> Option<&str> {
        self.returns.get(&(name.to_string(), arity)).map(String::as_str)
    }

    pub fn carries_ty(&self, name: &str, arity: usize, param: usize) -> Option<&str> {
        self.carries
            .get(&(name.to_string(), arity, param))
            .map(String::as_str)
    }
}

/// Full analysis result for codegen: the returnable types plus the groups that
/// return them and the parameter positions that carry them.
pub fn analyze(program: &Program) -> EscapeInfo {
    let returnable = register_returnable(program);
    let mut field_count = HashMap::new();
    let mut returns = HashMap::new();
    let mut carries = HashMap::new();
    for ty in &returnable {
        if let Some(decl) = program.types.iter().find(|t| &t.name == ty) {
            field_count.insert(ty.clone(), decl.fields.len());
        }
        let mut analysis = Analysis {
            program,
            returns_ty: HashSet::new(),
        };
        analysis.compute_returns_ty(ty);
        for key in analysis.returns_ty {
            returns.insert(key, ty.clone());
        }
        for f in &program.fns {
            for (i, p) in f.params.iter().enumerate() {
                if matches!(p, Pattern::Ctor { ty: pty, .. } if pty == ty) {
                    carries.insert((f.name.clone(), f.params.len(), i), ty.clone());
                }
            }
        }
    }
    EscapeInfo {
        field_count,
        returns,
        carries,
    }
}

/// Record type names that may be returned by value. A type qualifies when:
///  - it is never a field of another record type,
///  - it is only ever destructured via `(T ...)` patterns (never bound to a
///    Var/Wildcard/Annotated pattern), and
///  - every expression that produces a T value (a `T ...` construction, or a
///    call to a function whose group returns T) sits in a safe position: the
///    tail of a T-returning function, an `if` branch in tail position, or an
///    argument whose callee destructures that parameter as `(T ...)`.
pub fn register_returnable(program: &Program) -> HashSet<String> {
    let ctors: HashSet<&str> = program
        .types
        .iter()
        .filter(|t| !t.fields.is_empty())
        .map(|t| t.name.as_str())
        .collect();

    let analysis = Analysis {
        program,
        returns_ty: HashSet::new(),
    };

    ctors
        .iter()
        .filter(|ty| analysis.clone().returnable(ty))
        .map(|ty| ty.to_string())
        .collect()
}

#[derive(Clone)]
struct Analysis<'a> {
    program: &'a Program,
    /// (function name, arity) groups whose result is a `ty` value or a failure.
    returns_ty: HashSet<(String, usize)>,
}

impl<'a> Analysis<'a> {
    fn returnable(mut self, ty: &str) -> bool {
        // The packed convention shifts field 0's payload into the tag word,
        // which is only sound for an int: a pointer payload would lose its
        // tag and overflow the shift. The declared typeset makes this a
        // static check.
        let first_field_is_int = self
            .program
            .types
            .iter()
            .find(|t| t.name == ty)
            .and_then(|t| t.fields.first())
            .is_some_and(|(_, tys, _)| tys.len() == 1 && tys[0] == "int");
        if !first_field_is_int {
            return false;
        }
        // A type stored inside another record escapes through that record.
        for decl in &self.program.types {
            for (_, members, _) in &decl.fields {
                if members.iter().any(|m| m == ty) {
                    return false;
                }
            }
        }
        self.compute_returns_ty(ty);
        self.program.fns.iter().all(|f| self.body_is_safe(ty, &f.body))
    }

    /// Fixpoint: a group returns `ty` when every tail leaf of every member is a
    /// `ty` construction, a failure, or a call to another group that returns
    /// `ty`.
    fn compute_returns_ty(&mut self, ty: &str) {
        loop {
            let mut changed = false;
            for f in &self.program.fns {
                let key = (f.name.clone(), f.params.len());
                if self.returns_ty.contains(&key) {
                    continue;
                }
                if self.tail_returns_ty(ty, &f.body) {
                    self.returns_ty.insert(key);
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// A group returns `ty` when some arm's tail produces an actual `ty` value
    /// (a construction or a call to another `ty`-returning group) — not merely a
    /// failure. `err(...)`/`none` arms are propagation, not `ty` production, so
    /// they must not pull a function that returns json-or-failure into the set.
    fn tail_returns_ty(&self, ty: &str, body: &[Stmt]) -> bool {
        match body.last() {
            Some(Stmt::Expr(e)) => self.produces_ty(ty, e),
            _ => false,
        }
    }

    fn body_is_safe(&self, ty: &str, body: &[Stmt]) -> bool {
        let Some((last, rest)) = body.split_last() else {
            return true;
        };
        // Non-tail statements never mention ty (no construction, no binding).
        for stmt in rest {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    if self.pattern_binds_ty(ty, pattern) || self.expr_mentions_ty(ty, expr) {
                        return false;
                    }
                }
                Stmt::Expr(e) => {
                    if self.expr_mentions_ty(ty, e) {
                        return false;
                    }
                }
            }
        }
        match last {
            Stmt::Bind { .. } => false,
            Stmt::Expr(e) => self.tail_position_safe(ty, e),
        }
    }

    /// A tail expression may itself be a ty construction/return, or an `if`
    /// whose branches are each tail-safe. Anywhere else, ty must not appear
    /// except as an argument the callee immediately destructures.
    fn tail_position_safe(&self, ty: &str, e: &Expr) -> bool {
        if let Expr::App { head, args, .. } = e {
            if let Expr::Ident(name, _) = head.as_ref() {
                if name == "if" && args.len() == 3 {
                    return !self.expr_mentions_ty(ty, &args[0])
                        && self.tail_position_safe(ty, &args[1])
                        && self.tail_position_safe(ty, &args[2]);
                }
                if name == ty {
                    // a ty construction in tail position: its field args must
                    // not themselves smuggle a ty value anywhere unsafe.
                    return args.iter().all(|a| self.arg_safe(ty, a));
                }
            }
        }
        // Any other tail expression: safe as long as ty only appears in
        // immediately-destructured argument positions.
        self.expr_safe_calls(ty, e)
    }

    /// `e` is a call argument. It may produce a ty value only if the enclosing
    /// call destructures it; that is validated by the caller of this via
    /// `call_safe`. Here we ensure any *nested* ty usage inside `e` is safe.
    fn arg_safe(&self, ty: &str, e: &Expr) -> bool {
        self.expr_safe_calls(ty, e)
    }

    /// Walk `e` verifying every call that passes a ty-producing argument is
    /// received by a parameter destructured as `(ty ...)`, and that ty never
    /// appears in a container, index, binop, lambda, or template.
    fn expr_safe_calls(&self, ty: &str, e: &Expr) -> bool {
        match e {
            Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => true,
            Expr::Field { base, .. } => {
                !self.produces_ty(ty, base) && self.expr_safe_calls(ty, base)
            }
            Expr::Str(parts, _) => parts.iter().all(|p| match p {
                crate::ast::TemplatePart::Lit(_) => true,
                crate::ast::TemplatePart::Interp(x) => !self.produces_ty(ty, x) && self.expr_safe_calls(ty, x)
            }),
            Expr::List(items, _) => items
                .iter()
                .all(|x| !self.produces_ty(ty, x) && self.expr_safe_calls(ty, x)),
            Expr::MapLit(pairs, _) => pairs.iter().all(|(k, v)| {
                !self.produces_ty(ty, k)
                    && !self.produces_ty(ty, v)
                    && self.expr_safe_calls(ty, k)
                    && self.expr_safe_calls(ty, v)
            }),
            Expr::Index { base, index, .. } => {
                !self.produces_ty(ty, base)
                    && !self.produces_ty(ty, index)
                    && self.expr_safe_calls(ty, base)
                    && self.expr_safe_calls(ty, index)
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                !self.produces_ty(ty, lhs)
                    && !self.produces_ty(ty, rhs)
                    && self.expr_safe_calls(ty, lhs)
                    && self.expr_safe_calls(ty, rhs)
            }
            Expr::Seq(a, b, _) => {
                !self.produces_ty(ty, a)
                    && self.expr_safe_calls(ty, a)
                    && self.expr_safe_calls(ty, b)
            }
            Expr::Lambda { body, .. } => !self.produces_ty(ty, body) && self.expr_safe_calls(ty, body),
            Expr::App { head, args, .. } => {
                let Expr::Ident(name, _) = head.as_ref() else {
                    // higher-order head: any ty inside is unsafe
                    return !self.produces_ty(ty, head)
                        && args.iter().all(|a| !self.produces_ty(ty, a) && self.expr_safe_calls(ty, a));
                };
                if name == "if" && args.len() == 3 {
                    return !self.produces_ty(ty, &args[0])
                        && self.expr_safe_calls(ty, &args[0])
                        && args[1..]
                            .iter()
                            .all(|a| !self.produces_ty(ty, a) && self.expr_safe_calls(ty, a));
                }
                for (i, a) in args.iter().enumerate() {
                    if self.produces_ty(ty, a) {
                        if !self.callee_destructures(ty, name, args.len(), i) {
                            return false;
                        }
                        // the produced value is consumed here; still check its
                        // own subexpressions.
                        if !self.expr_safe_calls(ty, a) {
                            return false;
                        }
                    } else if !self.expr_safe_calls(ty, a) {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Does `e` evaluate to a ty value (a construction or a ty-returning call)?
    fn produces_ty(&self, ty: &str, e: &Expr) -> bool {
        match e {
            Expr::App { head, args, .. } => match head.as_ref() {
                Expr::Ident(name, _) => {
                    name == ty
                        || (name == "if"
                            && args.len() == 3
                            && (self.produces_ty(ty, &args[1]) || self.produces_ty(ty, &args[2])))
                        || self.returns_ty.contains(&(name.clone(), args.len()))
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// No member of the callee group binds parameter `i` to a *generic* name.
    /// A ty value flowing here is therefore either destructured by a `(ty ...)`
    /// arm or simply not matched by an arm meant for some other shape (a failure
    /// arm, a literal, a different constructor) — never captured as a variable
    /// it could leak through. Var/Annotated/Keyed bind generically and escape.
    fn callee_destructures(&self, _ty: &str, name: &str, arity: usize, i: usize) -> bool {
        let members: Vec<&FnDecl> = self
            .program
            .fns
            .iter()
            .filter(|f| f.name == name && f.params.len() == arity)
            .collect();
        !members.is_empty()
            && members.iter().all(|f| match f.params.get(i) {
                Some(Pattern::Var(..))
                | Some(Pattern::Annotated { .. })
                | Some(Pattern::Keyed { .. })
                | None => false,
                Some(_) => true,
            })
    }

    fn pattern_binds_ty(&self, ty: &str, p: &Pattern) -> bool {
        // A ty destructure is fine; anything that could bind a ty value to a
        // generic name is not. Since ty is only ever produced by our tracked
        // expressions, a Var/Annotated is only dangerous if it *could* hold ty
        // — which we forbid by treating any Var-bound ty flow as escape upstream.
        matches!(p, Pattern::Annotated { ty: pty, .. } if pty == ty)
    }

    fn expr_mentions_ty(&self, ty: &str, e: &Expr) -> bool {
        // Conservative: ty appears anywhere in this (non-tail) expression.
        match e {
            Expr::Ident(name, _) => name == ty,
            Expr::Field { base, .. } => self.expr_mentions_ty(ty, base),
            Expr::App { head, args, .. } => {
                self.expr_mentions_ty(ty, head) || args.iter().any(|a| self.expr_mentions_ty(ty, a))
            }
            Expr::Index { base, index, .. } => {
                self.expr_mentions_ty(ty, base) || self.expr_mentions_ty(ty, index)
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                self.expr_mentions_ty(ty, lhs) || self.expr_mentions_ty(ty, rhs)
            }
            Expr::Seq(a, b, _) => self.expr_mentions_ty(ty, a) || self.expr_mentions_ty(ty, b),
            Expr::Lambda { body, .. } => self.expr_mentions_ty(ty, body),
            Expr::List(items, _) => items.iter().any(|x| self.expr_mentions_ty(ty, x)),
            Expr::MapLit(pairs, _) => pairs
                .iter()
                .any(|(k, v)| self.expr_mentions_ty(ty, k) || self.expr_mentions_ty(ty, v)),
            Expr::Str(parts, _) => parts.iter().any(|p| match p {
                crate::ast::TemplatePart::Interp(x) => self.expr_mentions_ty(ty, x),
                crate::ast::TemplatePart::Lit(_) => false,
            }),
            Expr::Int(..) | Expr::Float(..) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::register_returnable;
    use std::path::Path;

    #[test]
    fn json_parsed_returnable_error_types_not() {
        let program = crate::compile_module(Path::new("lib/json"), false).unwrap();
        let r = register_returnable(&program);
        assert!(r.contains("parsed"), "_parsed should be register-returnable, got {r:?}");
        assert!(!r.contains("defect"), "defect escapes via err-wrapping, got {r:?}");
        assert!(!r.contains("parse_failure"), "parse_failure escapes via err-wrapping, got {r:?}");
    }

    #[test]
    fn record_in_a_list_escapes() {
        let src = "type pt\n  x:int\n  y:int\n\nmain = print \"{length [(mk 1 2) (mk 3 4)]}\"\n\nfn mk a b\n  pt a b\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        assert!(!register_returnable(&program).contains("pt"), "pt escapes into a list");
    }

    #[test]
    fn construct_then_destructure_is_returnable() {
        let src = "type pair\n  a:int\n  b:int\n\nfn add (pair x y)\n  x + y\n\nmain = print \"{add (mk 5)}\"\n\nfn mk n\n  pair n n\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        assert!(register_returnable(&program).contains("pair"), "pair is construct-then-destructure");
    }
}
