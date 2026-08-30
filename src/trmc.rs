//! Accumulating recursion becomes a loop when reassociation is exact.
//!
//! `1 + count (n - 1)` keeps a frame per call: the addition still owes work
//! after the recursion returns. Over arbitrary-precision integers the sum
//! is the same in any association, so the group gains a tail-calling helper
//! threading an accumulator, entered through an int-ascribed wrapper arm.
//! The original arms all stay: specificity sends integer arguments to the
//! wrapper (a concrete type outranks a bare variable) while literal-pattern
//! arms still answer their literals directly, and any non-integer argument
//! falls through to the original arms and behaves exactly as it always
//! did — including the frame-per-call descent. The license is deliberately
//! narrow: every arm is a single expression, the operator is one of `+`/`*`
//! across all recursive arms, every base body is an integer literal, the
//! recursive call is a direct call to the group's own name and arity, and
//! some parameter position dispatches on an integer literal (the counter the
//! descent consumes). Anything else — floats, guards, double recursion like
//! fib — is left alone entirely.
//!
//! The leftover operand may be a literal or an expression the group's own
//! shape proves is an integer: `n * fact (n - 1)` reassociates as exactly as
//! `1 + count (n - 1)` does. What proves it is an induction the wrapper
//! starts. The wrapper ascribes every counter position `int`, so a
//! non-integer argument never enters the loop at all; each recursive call
//! must then hand those positions arithmetic over counters, which is integer
//! again. An operand built from counters and literals is therefore an integer
//! at every depth, and it is pure — a name, a literal, and `+`/`-`/`*` reach
//! no call and no effect, so computing it before the descent instead of after
//! moves nothing a program can observe. Any operand outside that grammar, or
//! any recursive call that hands a counter position something the grammar
//! cannot read, leaves the whole group alone.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::hash::Map as HashMap;
use num_bigint::BigInt;

enum Arm<'a> {
    Base(BigInt),
    Rec { op: &'static str, operand: &'a Expr, self_args: &'a [Expr] },
}

/// The one shape an arm may take, or nothing.
fn classify<'a>(decl: &'a FnDecl, name: &str, arity: usize) -> Option<Arm<'a>> {
    let [Stmt::Expr(expr)] = decl.body.as_slice() else { return None };
    if let Expr::Int(k, _) = expr {
        return Some(Arm::Base(k.clone()));
    }
    let Expr::BinOp { op, lhs, rhs, .. } = expr else { return None };
    let op = match *op {
        "+" => "+",
        "*" => "*",
        _ => return None,
    };
    // One side descends and the other is the work left over. Both sides
    // descending is double recursion, which reassociates into nothing.
    let (operand, call) = match (is_self_call(lhs, name, arity), is_self_call(rhs, name, arity)) {
        (false, true) => (lhs.as_ref(), rhs.as_ref()),
        (true, false) => (rhs.as_ref(), lhs.as_ref()),
        _ => return None,
    };
    let Expr::App { args, .. } = call else { return None };
    if args.iter().any(|a| mentions(a, name)) || mentions(operand, name) {
        return None;
    }
    Some(Arm::Rec { op, operand, self_args: args })
}

/// A direct call to the group's own name at its own arity.
fn is_self_call(expr: &Expr, name: &str, arity: usize) -> bool {
    let Expr::App { head, args, .. } = expr else { return false };
    let Expr::Ident(callee, _) = head.as_ref() else { return false };
    callee == name && args.len() == arity
}

/// Arithmetic over `ints` and integer literals, which is an integer and has
/// no way to fail, allocate, or perform an effect.
fn int_arithmetic(expr: &Expr, ints: &[String]) -> bool {
    match expr {
        Expr::Int(..) => true,
        Expr::Ident(n, _) => ints.iter().any(|known| known == n),
        Expr::BinOp { op, lhs, rhs, .. } => {
            matches!(*op, "+" | "-" | "*") && int_arithmetic(lhs, ints) && int_arithmetic(rhs, ints)
        }
        _ => false,
    }
}

/// What this arm calls the arguments in counter positions. The wrapper
/// ascribes those `int`, so a plain name in one is an integer; a name
/// ascribed anything else is not, and a literal pattern binds nothing.
fn counter_names(decl: &FnDecl, counter: &[bool]) -> Vec<String> {
    counter
        .iter()
        .enumerate()
        .filter(|(_, is_counter)| **is_counter)
        .filter_map(|(i, _)| match decl.params.get(i) {
            Some(Pattern::Var(n, _)) => Some(n.clone()),
            Some(Pattern::Annotated { name, ty, .. }) if ty == "int" => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// Does the expression call or reference the group anywhere inside?
fn mentions(expr: &Expr, name: &str) -> bool {
    let mut found = false;
    walk(expr, &mut |e| {
        if let Expr::Ident(id, _) | Expr::Partial(id, _) = e {
            if id == name {
                found = true;
            }
        }
    });
    found
}

fn walk(expr: &Expr, visit: &mut dyn FnMut(&Expr)) {
    visit(expr);
    match expr {
        Expr::App { head, args, .. } => {
            walk(head, visit);
            for a in args {
                walk(a, visit);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            walk(lhs, visit);
            walk(rhs, visit);
        }
        Expr::List(items, _) => {
            for i in items {
                walk(i, visit);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                walk(k, visit);
                walk(v, visit);
            }
        }
        Expr::Field { base, .. } => walk(base, visit),
        Expr::Index { base, index, .. } => {
            walk(base, visit);
            walk(index, visit);
        }
        Expr::Seq(a, b, _) => {
            walk(a, visit);
            walk(b, visit);
        }
        _ => {}
    }
}

/// Names bound by a pattern, so the accumulator's name cannot shadow one.
fn bound_names(p: &Pattern, out: &mut Vec<String>) {
    match p {
        Pattern::Var(n, _) => out.push(n.clone()),
        Pattern::Annotated { name, .. } => out.push(name.clone()),
        Pattern::Ctor { fields, .. } => {
            for f in fields {
                bound_names(f, out);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for e in entries {
                out.push(e.bind_name.clone());
            }
        }
        _ => {}
    }
}

pub fn rewrite(program: &mut Program) {
    // Keys borrowed from the program, which already holds every one of these
    // names. `program` is `&mut`, but nothing here writes to it: the arms are
    // accumulated in `new_fns` and extended on at the end, which is what makes
    // the borrow hold for the whole walk. Cloning a name per declaration to
    // look one up was 990 blocks of the front end's allocations.
    let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::default();
    for (i, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(i);
    }
    let mut new_fns: Vec<FnDecl> = Vec::new();
    for ((name, arity), idxs) in &groups {
        if name.contains('/') || *arity == 0 {
            continue;
        }
        let decls: Vec<&FnDecl> = idxs.iter().map(|i| &program.fns[*i]).collect();
        if decls.iter().any(|d| d.synthetic) {
            continue;
        }
        // the counter positions: where some arm dispatches on an integer
        // literal. The wrapper ascribes those, so only integer arguments
        // ever take the loop; without one, nothing bounds the descent and
        // the group is left alone.
        let counter: Vec<bool> = (0..*arity)
            .map(|i| decls.iter().any(|d| matches!(d.params.get(i), Some(Pattern::IntLit(..)))))
            .collect();
        if !counter.iter().any(|c| *c) {
            continue;
        }
        let arms: Option<Vec<Arm>> = decls.iter().map(|d| classify(d, name, *arity)).collect();
        let Some(arms) = arms else { continue };
        let mut ops = arms.iter().filter_map(|a| match a {
            Arm::Rec { op, .. } => Some(*op),
            Arm::Base(_) => None,
        });
        let Some(op) = ops.next() else { continue };
        if ops.any(|o| o != op) || !arms.iter().any(|a| matches!(a, Arm::Base(_))) {
            continue;
        }
        // The induction that makes a named operand an integer: counters go in
        // as integers and come out of every recursive call as arithmetic over
        // integers. An operand that is only ever a literal needs none of it,
        // and asking anyway would drop groups the narrow license already
        // rewrites.
        let proven = decls.iter().zip(&arms).all(|(decl, arm)| {
            let Arm::Rec { operand, self_args, .. } = arm else { return true };
            let ints = counter_names(decl, &counter);
            if matches!(operand, Expr::Int(..)) {
                return true;
            }
            int_arithmetic(operand, &ints)
                && counter.iter().enumerate().filter(|(_, is_counter)| **is_counter).all(
                    |(i, _)| match self_args.get(i) {
                        Some(arg) => int_arithmetic(arg, &ints),
                        None => false,
                    },
                )
        });
        if !proven {
            continue;
        }
        let helper = format!("trmc/{name}");
        let mut taken: Vec<String> = Vec::new();
        for d in &decls {
            for p in &d.params {
                bound_names(p, &mut taken);
            }
        }
        let mut acc = "acc".to_string();
        let mut i = 0;
        while taken.contains(&acc) {
            i += 1;
            acc = format!("acc{i}");
        }
        for (decl, arm) in decls.iter().zip(&arms) {
            let span = decl.span;
            let mut params = decl.params.clone();
            params.push(Pattern::Var(acc.clone(), span));
            let body = match arm {
                Arm::Base(k) => vec![Stmt::Expr(Expr::BinOp {
                    op,
                    lhs: Box::new(Expr::Ident(acc.clone(), span)),
                    rhs: Box::new(Expr::Int(k.clone(), span)),
                    span,
                })],
                Arm::Rec { operand, self_args, .. } => {
                    let mut args: Vec<Expr> = self_args.to_vec();
                    args.push(Expr::BinOp {
                        op,
                        lhs: Box::new(Expr::Ident(acc.clone(), span)),
                        rhs: Box::new((*operand).clone()),
                        span,
                    });
                    vec![Stmt::Expr(Expr::App {
                        head: Box::new(Expr::Ident(helper.clone(), span)),
                        args,
                        span,
                        piped: false,
                    })]
                }
            };
            new_fns.push(FnDecl {
                name: helper.clone(),
                is_pub: false,
                span,
                params,
                body,
                file: decl.file.clone(),
                synthetic: true,
            });
        }
        let span = decls[0].span;
        let identity = match op {
            "*" => BigInt::from(1),
            _ => BigInt::from(0),
        };
        let wrapper_params: Vec<Pattern> = (0..*arity)
            .map(|i| match counter[i] {
                true => {
                    Pattern::Annotated { name: format!("trmcp{i}"), ty: "int".to_string(), span }
                }
                false => Pattern::Var(format!("trmcp{i}"), span),
            })
            .collect();
        let mut wrapper_args: Vec<Expr> =
            (0..*arity).map(|i| Expr::Ident(format!("trmcp{i}"), span)).collect();
        wrapper_args.push(Expr::Int(identity, span));
        new_fns.push(FnDecl {
            name: name.to_string(),
            is_pub: decls.iter().any(|d| d.is_pub),
            span,
            params: wrapper_params,
            body: vec![Stmt::Expr(Expr::App {
                head: Box::new(Expr::Ident(helper, span)),
                args: wrapper_args,
                span,
                piped: false,
            })],
            file: decls[0].file.clone(),
            synthetic: true,
        });
    }
    program.fns.extend(new_fns);
}
