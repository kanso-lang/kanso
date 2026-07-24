//! Whole-program demand analysis for the lazy v1 fragment.
//!
//! design/lazy-v1-plan.md pins the surface: a binding is conditionally
//! demanded — and compiles to a thunk — when every use passes it,
//! unscrutinized, into a dispatch argument position where at least one arm
//! of the callee group discards that parameter. Dispatch is kanso's only
//! branch, so "some arm ignores it" is exactly "demand depends on which
//! arm wins." Every other binding stays strict: any misclassification here
//! errs toward strict, which is today's behavior.

use crate::ast::{Expr, Pattern, Program, Stmt, TemplatePart};
use std::collections::{HashMap, HashSet};

pub struct DemandInfo {
    /// (owning fn name, owning fn arity, statement index) of each lazy bind.
    lazy_binds: HashSet<(String, usize, usize)>,
    /// The subset whose cell provably dies with its frame: every use targets
    /// a safe callee position, so the frame epilogue may release it (guarded
    /// by the returned-thunk alias check at runtime).
    releasable: HashSet<(String, usize, usize)>,
}

impl DemandInfo {
    pub fn is_lazy_bind(&self, fn_name: &str, arity: usize, stmt_index: usize) -> bool {
        self.lazy_binds
            .contains(&(fn_name.to_string(), arity, stmt_index))
    }

    pub fn is_releasable(&self, fn_name: &str, arity: usize, stmt_index: usize) -> bool {
        self.releasable
            .contains(&(fn_name.to_string(), arity, stmt_index))
    }

    pub fn lazy_bind_count(&self) -> usize {
        self.lazy_binds.len()
    }
}

/// Occurrences of a bound parameter that cannot leak its cell past the
/// callee's frame: forced positions (operands, index bases and keys,
/// interpolations — codegen forces each when a thunk may arrive) and the
/// bare-result return. Anything else — stored into a literal, captured,
/// passed onward — may outlive the frame, so the position is unsafe. One
/// level deep by design: passing the parameter to another call is unsafe
/// even if that callee would qualify, erring sound.
fn param_stays_local(body: &[Stmt], name: &str) -> bool {
    fn operand_is(e: &Expr, name: &str) -> bool {
        matches!(e, Expr::Ident(id, _) if id == name)
    }
    fn expr_safe(e: &Expr, name: &str, is_result: bool) -> bool {
        match e {
            Expr::Ident(id, _) if id == name => is_result,
            Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => true,
            Expr::BinOp { lhs, rhs, .. } => {
                (operand_is(lhs, name) || expr_safe(lhs, name, false))
                    && (operand_is(rhs, name) || expr_safe(rhs, name, false))
            }
            Expr::Index { base, index, .. } => {
                (operand_is(base, name) || expr_safe(base, name, false))
                    && (operand_is(index, name) || expr_safe(index, name, false))
            }
            Expr::Str(parts, _) => parts.iter().all(|p| match p {
                TemplatePart::Lit(_) => true,
                TemplatePart::Interp(inner) => {
                    operand_is(inner, name) || expr_safe(inner, name, false)
                }
            }),
            Expr::Block(stmts, _) => param_stays_local_at(stmts, name, is_result),
            Expr::App { head, args, .. } => {
                expr_safe(head, name, false)
                    && args.iter().all(|a| expr_safe(a, name, false))
            }
            Expr::List(items, _) => items.iter().all(|a| expr_safe(a, name, false)),
            Expr::MapLit(entries, _) => entries
                .iter()
                .all(|(k, v)| expr_safe(k, name, false) && expr_safe(v, name, false)),
            Expr::Field { base, .. } => expr_safe(base, name, false),
            Expr::Upcast { expr, .. } => expr_safe(expr, name, false),
            Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
                expr_safe(a, name, false) && expr_safe(b, name, false)
            }
            Expr::Lambda { body, .. } => expr_safe(body, name, false),
        }
    }
    fn param_stays_local_at(body: &[Stmt], name: &str, result_ok: bool) -> bool {
        let last = body.len().saturating_sub(1);
        body.iter().enumerate().all(|(i, stmt)| match stmt {
            Stmt::Bind { expr, .. } => expr_safe(expr, name, false),
            Stmt::Expr(expr) => expr_safe(expr, name, result_ok && i == last),
        })
    }
    param_stays_local_at(body, name, true)
}

/// A callee position is safe when every arm either never binds the value
/// (wildcard or literal pattern — the dispatcher forces literals to compare)
/// or binds it and keeps it frame-local.
fn safe_position(program: &Program, group: &str, arity: usize, position: usize) -> bool {
    let mut any = false;
    for f in &program.fns {
        if f.name != group || f.params.len() != arity {
            continue;
        }
        any = true;
        match &f.params[position] {
            Pattern::Wildcard(_)
            | Pattern::IntLit(..)
            | Pattern::StrLit(..)
            | Pattern::Nullary(..) => {}
            Pattern::Var(name, _) => {
                if !param_stays_local(&f.body, name) {
                    return false;
                }
            }
            Pattern::Annotated { .. } | Pattern::Ctor { .. } | Pattern::Keyed { .. } => {
                return false;
            }
        }
    }
    any
}

/// The (group, arity, position) targets a binding's uses flow into. The
/// demand rule already guarantees every use is a direct argument; this
/// records where.
fn use_targets(
    expr: &Expr,
    name: &str,
    out: &mut Vec<(String, usize, usize)>,
) {
    match expr {
        Expr::App { head, args, .. } => {
            if !matches!(head.as_ref(), Expr::Ident(..)) {
                use_targets(head, name, out);
            }
            for (i, arg) in args.iter().enumerate() {
                match arg {
                    Expr::Ident(id, _) if id == name => {
                        if let Expr::Ident(callee, _) = head.as_ref() {
                            out.push((callee.clone(), args.len(), i));
                        }
                    }
                    _ => use_targets(arg, name, out),
                }
            }
        }
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => use_targets(expr, name, out),
                }
            }
        }
        Expr::MapLit(entries, _) => {
            for (k, v) in entries {
                use_targets(k, name, out);
                use_targets(v, name, out);
            }
        }
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(e) = part {
                    use_targets(e, name, out);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                use_targets(item, name, out);
            }
        }
        Expr::Field { base, .. } => use_targets(base, name, out),
        Expr::Upcast { expr, .. } => use_targets(expr, name, out),
        Expr::Index { base, index, .. } => {
            use_targets(base, name, out);
            use_targets(index, name, out);
        }
        Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
            use_targets(a, name, out);
            use_targets(b, name, out);
        }
        Expr::Lambda { body, .. } => use_targets(body, name, out),
        Expr::BinOp { lhs, rhs, .. } => {
            use_targets(lhs, name, out);
            use_targets(rhs, name, out);
        }
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => {}
    }
}

/// For each (group name, arity), which argument positions have at least one
/// arm that discards the parameter outright.
fn discard_positions(program: &Program) -> HashMap<(String, usize), Vec<bool>> {
    let mut positions: HashMap<(String, usize), Vec<bool>> = HashMap::new();
    for f in &program.fns {
        let slots = positions
            .entry((f.name.clone(), f.params.len()))
            .or_insert_with(|| vec![false; f.params.len()]);
        for (i, param) in f.params.iter().enumerate() {
            if matches!(param, Pattern::Wildcard(_)) {
                slots[i] = true;
            }
        }
    }
    positions
}

#[derive(Default)]
struct Uses {
    /// Appearances as a direct argument at a discard-capable position.
    deferrable: usize,
    /// Any other appearance: scrutiny, arithmetic, interpolation, capture,
    /// or an argument position every arm binds.
    demanding: usize,
}

fn collect_uses(
    expr: &Expr,
    name: &str,
    discard: &HashMap<(String, usize), Vec<bool>>,
    uses: &mut Uses,
) {
    match expr {
        Expr::Ident(id, _) if id == name => uses.demanding += 1,
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => {}
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                        collect_uses(expr, name, discard, uses)
                    }
                }
            }
        }
        Expr::App { head, args, .. } => {
            let discard_slots = match head.as_ref() {
                Expr::Ident(callee, _) => discard.get(&(callee.clone(), args.len())),
                _ => None,
            };
            if !matches!(head.as_ref(), Expr::Ident(..)) {
                collect_uses(head, name, discard, uses);
            }
            for (i, arg) in args.iter().enumerate() {
                match arg {
                    Expr::Ident(id, _) if id == name => {
                        let deferrable =
                            discard_slots.is_some_and(|slots| slots.get(i).copied() == Some(true));
                        match deferrable {
                            true => uses.deferrable += 1,
                            false => uses.demanding += 1,
                        }
                    }
                    _ => collect_uses(arg, name, discard, uses),
                }
            }
        }
        Expr::MapLit(entries, _) => {
            for (k, v) in entries {
                collect_uses(k, name, discard, uses);
                collect_uses(v, name, discard, uses);
            }
        }
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(e) = part {
                    collect_uses(e, name, discard, uses);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                collect_uses(item, name, discard, uses);
            }
        }
        Expr::Field { base, .. } => collect_uses(base, name, discard, uses),
        Expr::Upcast { expr, .. } => collect_uses(expr, name, discard, uses),
        Expr::Index { base, index, .. } => {
            collect_uses(base, name, discard, uses);
            collect_uses(index, name, discard, uses);
        }
        Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
            collect_uses(a, name, discard, uses);
            collect_uses(b, name, discard, uses);
        }
        Expr::Lambda { body, .. } => collect_uses(body, name, discard, uses),
        Expr::BinOp { lhs, rhs, .. } => {
            collect_uses(lhs, name, discard, uses);
            collect_uses(rhs, name, discard, uses);
        }
    }
}

/// The cost gate (demand x cost x slack): a thunk cell only pays for itself
/// when the deferred work is real. A user-function call can recurse
/// arbitrarily; a bare builtin chain (push, at, arithmetic) is cheaper than
/// the cell, so it compiles strict.
fn expensive(expr: &Expr, fns: &HashSet<&str>) -> bool {
    match expr {
        Expr::Block(stmts, _) => stmts.iter().any(|st| match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expensive(expr, fns),
        }),
        Expr::App { head, args, .. } => {
            if let Expr::Ident(callee, _) = head.as_ref() {
                if fns.contains(callee.as_str()) {
                    return true;
                }
            }
            args.iter().any(|a| expensive(a, fns))
        }
        Expr::BinOp { lhs, rhs, .. } | Expr::Seq(lhs, rhs, _) | Expr::Join { lhs, rhs, .. } => {
            expensive(lhs, fns) || expensive(rhs, fns)
        }
        Expr::List(items, _) => items.iter().any(|a| expensive(a, fns)),
        Expr::MapLit(entries, _) => entries.iter().any(|(k, v)| expensive(k, fns) || expensive(v, fns)),
        Expr::Str(parts, _) => parts.iter().any(|p| match p {
            TemplatePart::Interp(e) => expensive(e, fns),
            TemplatePart::Lit(_) => false,
        }),
        Expr::Field { base, .. } => expensive(base, fns),
        Expr::Upcast { expr, .. } => expensive(expr, fns),
        Expr::Index { base, index, .. } => expensive(base, fns) || expensive(index, fns),
        Expr::Lambda { .. } | Expr::Ident(..) | Expr::Int(..) | Expr::Float(..) => false,
    }
}

pub fn analyze(program: &Program) -> DemandInfo {
    // The worst-case measurement mode: force everything by thunking nothing.
    // A measurement tool, not a semantics switch — forcing runs what laziness
    // would skip (design/compiler-log.md, strict-mode thread).
    if std::env::var_os("KANSO_STRICT").is_some() {
        return DemandInfo { lazy_binds: HashSet::new(), releasable: HashSet::new() };
    }
    let discard = discard_positions(program);
    let fn_names: HashSet<&str> = program.fns.iter().map(|f| f.name.as_str()).collect();
    let mut lazy_binds = HashSet::new();
    for f in &program.fns {
        for (i, stmt) in f.body.iter().enumerate() {
            let Stmt::Bind { pattern: Pattern::Var(name, _), .. } = stmt else {
                continue;
            };
            // A later rebind of the same name would make use-attribution
            // ambiguous; treat the whole name as strict in that case.
            let rebound = f.body[i + 1..].iter().any(|later| {
                matches!(later, Stmt::Bind { pattern: Pattern::Var(n, _), .. } if n == name)
            });
            let mut uses = Uses::default();
            for later in &f.body[i + 1..] {
                let e = match later {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                };
                collect_uses(e, name, &discard, &mut uses);
            }
            let Stmt::Bind { expr, .. } = stmt else { unreachable!() };
            if !rebound && uses.deferrable > 0 && uses.demanding == 0 && expensive(expr, &fn_names) {
                lazy_binds.insert((f.name.clone(), f.params.len(), i));
            }
        }
    }
    // A (group, arity, index) key can name a bind in more than one arm of
    // the group; the release verdict must hold for every arm it names.
    let mut verdicts: HashMap<(String, usize, usize), bool> = HashMap::new();
    for decl in &program.fns {
        for (i, stmt) in decl.body.iter().enumerate() {
            let key = (decl.name.clone(), decl.params.len(), i);
            if !lazy_binds.contains(&key) {
                continue;
            }
            let Stmt::Bind { pattern: Pattern::Var(name, _), .. } = stmt else {
                let slot = verdicts.entry(key).or_insert(true);
                *slot = false;
                continue;
            };
            let mut targets = Vec::new();
            for later in &decl.body[i + 1..] {
                let e = match later {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                };
                use_targets(e, name, &mut targets);
            }
            let ok = targets
                .iter()
                .all(|(g, a, p)| safe_position(program, g, *a, *p));
            let slot = verdicts.entry(key).or_insert(true);
            *slot = *slot && ok;
        }
    }
    let releasable = verdicts
        .into_iter()
        .filter_map(|(key, ok)| ok.then_some(key))
        .collect();
    DemandInfo { lazy_binds, releasable }
}

#[cfg(test)]
mod tests {
    use super::analyze;

    fn program(source: &str) -> crate::ast::Program {
        let lexed = crate::lexer::lex(source).expect("lexes");
        crate::parser::parse(&lexed).expect("parses")
    }

    #[test]
    fn discard_capable_argument_is_lazy() {
        let p = program(
            "fn burn 0 acc\n  acc\n\nfn burn n acc\n  burn (n - 1) (acc + n)\n\n\
             fn pick false _\n  0\n\nfn pick true chosen\n  chosen\n\n\
             pub play =\n  expensive = burn 2000 0\n  print \"picked: {pick false expensive}\"\n",
        );
        let info = analyze(&p);
        assert!(info.is_lazy_bind("play", 0, 0), "expensive flows only to pick's discard-capable slot");
    }

    #[test]
    fn scrutinized_binding_stays_strict() {
        let p = program(
            "fn double n\n  n + n\n\npub play =\n  shared = double 21\n  print \"sum: {shared + shared}\"\n",
        );
        let info = analyze(&p);
        assert_eq!(info.lazy_bind_count(), 0, "arithmetic scrutiny demands the value");
    }
}
