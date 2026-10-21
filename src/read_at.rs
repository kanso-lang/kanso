//! A position a caller has just read at.
//!
//! The JSON decoder hands a byte on together with the position it was read
//! from: `array_delim cs cs[p + 1] (p + 1) acc`. Inside an arm the byte
//! reaches as a byte rather than as `none`, that read succeeded, so the
//! position is an index of `cs` and lies inside its length. `p + 1` in such
//! an arm cannot leave the machine word, and the emitter says so to LLVM,
//! which then drops the overflow test the addition would otherwise carry.
//!
//! The fact rests on every call. A group qualifies at a triple of its
//! parameters only when each call passes some `c`, `c[x]` and `x` there, the
//! collection and the index written alike in both places, and when nothing
//! names the group except as the head of such a call: a partial, a piped
//! call or a bare mention lets a caller pass anything.
use crate::ast::{Expr, Program, Stmt};
use crate::hash::{Map as HashMap, Set as HashSet};

/// Three parameters of a group: a collection, a value read out of it, and
/// the position it was read at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub coll: usize,
    pub byte: usize,
    pub pos: usize,
}

pub type Links = HashMap<(String, usize), Vec<Link>>;

/// Every group whose every call passes a collection, a read out of it and
/// the read's position, with the parameters each call passes them at.
pub fn links(program: &Program) -> Links {
    let groups: HashSet<(&str, usize)> =
        program.fns.iter().map(|f| (f.name.as_str(), f.params.len())).collect();
    let mut found: HashMap<(String, usize), Vec<Link>> = HashMap::default();
    let mut refused: HashSet<String> = HashSet::default();
    for f in &program.fns {
        for st in &f.body {
            match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    walk(expr, &groups, &mut found, &mut refused)
                }
            }
        }
    }
    found.retain(|(name, _), links| !refused.contains(name) && !links.is_empty());
    found
}

fn walk(
    e: &Expr,
    groups: &HashSet<(&str, usize)>,
    found: &mut HashMap<(String, usize), Vec<Link>>,
    refused: &mut HashSet<String>,
) {
    match e {
        Expr::App { piped: false, head, args, .. } if matches!(head.as_ref(), Expr::Ident(..)) => {
            let Expr::Ident(name, _, _) = head.as_ref() else { unreachable!() };
            let key = (name.as_str(), args.len());
            if groups.contains(&key) {
                let here = at_call(args);
                found
                    .entry((name.as_str().to_string(), args.len()))
                    .and_modify(|links| links.retain(|l| here.contains(l)))
                    .or_insert(here);
            }
            for a in args {
                walk(a, groups, found, refused);
            }
        }
        Expr::Ident(name, _, _) | Expr::Partial(name, _) => {
            refused.insert(name.as_str().to_string());
        }
        _ => crate::for_each_child(e, |c| walk(c, groups, found, refused)),
    }
}

/// The triples one call's arguments form.
fn at_call(args: &[Expr]) -> Vec<Link> {
    let mut out = Vec::new();
    for (byte, arg) in args.iter().enumerate() {
        let Expr::Index { base, index, .. } = arg else { continue };
        for coll in 0..args.len() {
            if !same(&args[coll], base) {
                continue;
            }
            for (pos, at) in args.iter().enumerate() {
                if same(at, index) {
                    out.push(Link { coll, byte, pos });
                }
            }
        }
    }
    out
}

/// Two expressions that are written alike and compute without effects, so
/// one call evaluates both to the same value.
fn same(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Ident(x, _, _), Expr::Ident(y, _, _)) => x.as_str() == y.as_str(),
        (Expr::Int(x, _), Expr::Int(y, _)) => x == y,
        (
            Expr::BinOp { op: o1, lhs: l1, rhs: r1, .. },
            Expr::BinOp { op: o2, lhs: l2, rhs: r2, .. },
        ) => o1 == o2 && matches!(*o1, "+" | "-") && same(l1, l2) && same(r1, r2),
        _ => false,
    }
}
