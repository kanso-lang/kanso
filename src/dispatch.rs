//! Which switch-dispatched parameters can be carried and matched as a raw i64
//! byte discriminator instead of a boxed KValue. This is what lets the scanner's
//! per-token dispatch become a raw `switch` with the current byte in a register,
//! the way serde's inner loop works, rather than a boxed value threaded through
//! a `musttail` call.
//!
//! Soundness turns on one point: the none case is encoded as the sentinel 256,
//! so the discriminator must be provably in the range 0 to 255 or none. Two
//! local facts together guarantee it, with no whole-value range tracking. First,
//! every call site passes `at _ _` at the discriminator position. Second, the
//! group's inferred discriminator set is exactly `int | none`, which `at`
//! produces only on a bytes container — a list, map, or string widens the set
//! past `int | none`. Together they force every container to be bytes, hence
//! every value to be a byte or none. Anything less certain leaves it boxed.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::infer::{Inference, Set, INT, NONE};
use std::collections::HashSet;

/// (function name, arity, discriminator index) triples whose discriminator can
/// be passed and switched as a raw i64 (byte value, or 256 for none).
pub fn byte_dispatched(program: &Program, inference: &Inference) -> HashSet<(String, usize, usize)> {
    let mut out = HashSet::new();
    let mut seen: HashSet<(&str, usize)> = HashSet::new();
    for decl in &program.fns {
        let key = (decl.name.as_str(), decl.params.len());
        if !seen.insert(key) {
            continue;
        }
        let group: Vec<&FnDecl> = program
            .fns
            .iter()
            .filter(|d| d.name == decl.name && d.params.len() == decl.params.len())
            .collect();
        let Some(disc) = switch_disc(&group) else {
            continue;
        };
        if group_param_set(program, inference, &decl.name, decl.params.len(), disc) != (INT | NONE) {
            continue;
        }
        if all_calls_feed_at(program, &decl.name, decl.params.len(), disc) {
            out.insert((decl.name.clone(), decl.params.len(), disc));
        }
    }
    out
}

/// The discriminator position of a switch-shaped group: the single parameter
/// carrying int-literal/nullary patterns (all others generic), with at least two
/// int arms. Mirrors the backend's own `switch_shape`.
fn switch_disc(decls: &[&FnDecl]) -> Option<usize> {
    if decls[0].params.is_empty() {
        return None;
    }
    let mut disc: Option<usize> = None;
    let mut int_arms = 0;
    for decl in decls {
        for (i, pattern) in decl.params.iter().enumerate() {
            match pattern {
                Pattern::Var(..) | Pattern::Wildcard(..) => {}
                Pattern::IntLit(..) | Pattern::Nullary(..) => {
                    if disc.is_some_and(|d| d != i) {
                        return None;
                    }
                    disc = Some(i);
                    if matches!(pattern, Pattern::IntLit(..)) {
                        int_arms += 1;
                    }
                }
                _ => return None,
            }
        }
    }
    match (disc, int_arms >= 2) {
        (Some(d), true) => Some(d),
        _ => None,
    }
}

fn group_param_set(
    program: &Program,
    inference: &Inference,
    name: &str,
    arity: usize,
    param: usize,
) -> Set {
    program
        .fns
        .iter()
        .enumerate()
        .filter(|(_, d)| d.name == name && d.params.len() == arity)
        .fold(0, |acc, (i, _)| acc | inference.params[i][param])
}

/// Every call to `name`/`arity` in the whole program passes an `at _ _` at the
/// discriminator position (and never through a pipe, which would move it).
fn all_calls_feed_at(program: &Program, name: &str, arity: usize, disc: usize) -> bool {
    let mut ok = true;
    for decl in &program.fns {
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } => walk(expr, name, arity, disc, &mut ok),
                Stmt::Expr(e) => walk(e, name, arity, disc, &mut ok),
                Stmt::Set { value, .. } => walk(value, name, arity, disc, &mut ok),
            }
        }
    }
    ok
}

fn walk(e: &Expr, name: &str, arity: usize, disc: usize, ok: &mut bool) {
    if !*ok {
        return;
    }
    if let Expr::App { head, args, piped, .. } = e {
        if let Expr::Ident(callee, _) = head.as_ref() {
            if callee == name && args.len() == arity && (*piped || !is_at_call(&args[disc])) {
                *ok = false;
                return;
            }
        }
    }
    for child in children(e) {
        walk(child, name, arity, disc, ok);
    }
}

fn is_at_call(e: &Expr) -> bool {
    // lenient indexing (`cs[p]`) is the byte-discriminator shape
    matches!(e, Expr::Index { strict: false, .. })
}

fn children(e: &Expr) -> Vec<&Expr> {
    match e {
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
    use super::byte_dispatched;
    use std::path::Path;

    #[test]
    fn scanner_byte_discriminators_identified() {
        let program = crate::compile_module(Path::new("lib/json"), false).unwrap();
        let inference = crate::infer::infer(&program);
        let d = byte_dispatched(&program, &inference);
        for expected in [("value_for", 3, 0), ("str_char", 4, 1), ("array_delim", 4, 1)] {
            let key = (expected.0.to_string(), expected.1, expected.2);
            assert!(d.contains(&key), "{expected:?} should be byte-dispatched, got {d:?}");
        }
    }

    #[test]
    fn int_switch_not_fed_by_at_is_rejected() {
        // `pick` dispatches on int literals but its discriminator comes from
        // `length`, not `at` on bytes, so the sentinel would be unsound.
        let src = "main = print \"{pick (length [1 2 3])}\"\n\nfn pick 1\n  \"one\"\n\nfn pick 2\n  \"two\"\n\nfn pick _\n  \"other\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        let inference = crate::infer::infer(&program);
        let d = byte_dispatched(&program, &inference);
        assert!(!d.contains(&("pick".to_string(), 1, 0)), "pick is not byte-dispatched, got {d:?}");
    }
}
