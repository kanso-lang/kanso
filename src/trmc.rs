//! Accumulating recursion becomes a loop when reassociation is exact.
//!
//! `1 + count (n - 1)` keeps a frame per call: the addition still owes work
//! after the recursion returns. Over arbitrary-precision integers the sum
//! is the same in any association, so the group rewrites to a tail-calling
//! helper threading an accumulator — the loop the programmer would have
//! written by hand — and the wrapper keeps the group's surface. The license
//! is deliberately narrow: every arm is a single expression, the operator
//! is one of `+`/`*` across all recursive arms, the non-recursive operand
//! and every base body are integer literals, and the recursive call is a
//! direct call to the group's own name and arity. Anything else — floats,
//! guards, double recursion like fib — keeps its frames.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use num_bigint::BigInt;
use std::collections::HashMap;

enum Arm<'a> {
    Base(BigInt),
    Rec { op: &'static str, operand: BigInt, self_args: &'a [Expr] },
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
    let (lit, call) = match (lhs.as_ref(), rhs.as_ref()) {
        (Expr::Int(k, _), call) => (k, call),
        (call, Expr::Int(k, _)) => (k, call),
        _ => return None,
    };
    let Expr::App { head, args, .. } = call else { return None };
    let Expr::Ident(callee, _) = head.as_ref() else { return None };
    if callee != name || args.len() != arity {
        return None;
    }
    if args.iter().any(|a| mentions(a, name)) {
        return None;
    }
    Some(Arm::Rec { op, operand: lit.clone(), self_args: args })
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
    let mut groups: HashMap<(String, usize), Vec<usize>> = HashMap::new();
    for (i, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.clone(), decl.params.len())).or_default().push(i);
    }
    let mut new_fns: Vec<FnDecl> = Vec::new();
    let mut replaced: HashMap<usize, FnDecl> = HashMap::new();
    for ((name, arity), idxs) in &groups {
        if name.contains('/') || *arity == 0 {
            continue;
        }
        let decls: Vec<&FnDecl> = idxs.iter().map(|i| &program.fns[*i]).collect();
        if decls.iter().any(|d| d.synthetic) {
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
                        rhs: Box::new(Expr::Int(operand.clone(), span)),
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
        let wrapper_params: Vec<Pattern> =
            (0..*arity).map(|i| Pattern::Var(format!("trmcp{i}"), span)).collect();
        let mut wrapper_args: Vec<Expr> =
            (0..*arity).map(|i| Expr::Ident(format!("trmcp{i}"), span)).collect();
        wrapper_args.push(Expr::Int(identity, span));
        let wrapper = FnDecl {
            name: name.clone(),
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
        };
        let mut idxs = idxs.clone();
        let first = idxs.remove(0);
        replaced.insert(first, wrapper);
        for i in idxs {
            replaced.insert(i, FnDecl { name: String::new(), ..program.fns[i].clone() });
        }
    }
    if replaced.is_empty() {
        return;
    }
    let mut rebuilt: Vec<FnDecl> = Vec::new();
    for (i, decl) in program.fns.drain(..).enumerate() {
        match replaced.remove(&i) {
            Some(r) if r.name.is_empty() => {}
            Some(r) => rebuilt.push(r),
            None => rebuilt.push(decl),
        }
    }
    rebuilt.extend(new_fns);
    program.fns = rebuilt;
}
