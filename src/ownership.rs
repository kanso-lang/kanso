//! Whole-program borrow/consume inference — the printable per-function
//! ownership signature under the no-runtime-memory-management direction
//! (compiler.html §10). For each function group `(name, arity)` it computes,
//! per parameter, whether that parameter is only *borrowed* (read, its storage
//! never incorporated into the result) or *consumed* (moved/stored into the
//! output, so its storage is a reuse-or-free candidate).
//!
//! The virtual design committee's convergent demand (Fowler, Metz) was that
//! ownership must not be an ambient whole-program judgement with no artifact:
//! it must bottom out in a per-function *signature* you can pin, diff, and
//! blame — the same shape as kanso's inferred types, surfaced rather than
//! hidden. This pass computes exactly that signature. It is the discovery
//! foundation only: nothing consumes its output for codegen, so a conservative
//! v1 is safe.
//!
//! Soundness runs one direction: a parameter is called `Borrow` only when every
//! occurrence is provably a read; anything the analysis cannot follow (an
//! unknown callee, a record constructor, a rebinding, a closure capture) counts
//! as `Consume`. Over-claiming consumption loses a reuse opportunity; it never
//! wrongly frees live data.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt, TemplatePart};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Borrow,
    Consume,
}

/// Argument modes of a builtin: which slots read-and-retain-nothing (`Borrow`)
/// versus fold the argument's storage into the result (`Consume`). A builtin
/// absent here is unknown and treated as all-`Consume` at the call site.
fn builtin_modes(name: &str, arity: usize) -> Option<Vec<Mode>> {
    use Mode::{Borrow as B, Consume as C};
    let modes: &[Mode] = match (name, arity) {
        // read the container / scalars, produce a fresh or scalar result
        ("at", 2) => &[B, B],
        ("find2", 4) => &[B, B, B, B],
        ("length", 1) | ("sum", 1) | ("to_int", 1) | ("to_float", 1) => &[B],
        ("char_code", 1) | ("bytes", 1) | ("utf8", 1) | ("chars", 1) | ("from_code", 1) => &[B],
        ("slice", 3) => &[B, B, B],
        ("read_file", 1) => &[B],
        ("write_file", 2) => &[B, B],
        // fold argument storage into the result (reuse-or-free candidates)
        ("push", 2) => &[C, C],
        ("put", 3) => &[C, C, C],
        ("concat", 2) | ("join", 2) => &[C, C],
        ("sort", 1) | ("entries", 1) => &[C],
        // collection consumed; the function argument is only invoked, not stored
        ("map", 2) | ("filter", 2) => &[C, B],
        _ => return None,
    };
    Some(modes.to_vec())
}

/// The inferred ownership signature of every function group, keyed
/// `(name, arity)`, one `Mode` per parameter position.
pub fn signatures(program: &Program) -> HashMap<(String, usize), Vec<Mode>> {
    // group arities present in the program
    let mut arities: HashSet<(String, usize)> = HashSet::new();
    for d in &program.fns {
        arities.insert((d.name.clone(), d.params.len()));
    }
    // least fixpoint: assume every parameter borrowed, promote to consume as the
    // code proves consumption; promotion is monotone, so this converges.
    let mut sig: HashMap<(String, usize), Vec<Mode>> = arities
        .iter()
        .map(|(n, a)| ((n.clone(), *a), vec![Mode::Borrow; *a]))
        .collect();

    loop {
        let mut changed = false;
        for (name, arity) in &arities {
            let consumed = consumed_params(program, &sig, name, *arity);
            let entry = sig.get_mut(&(name.clone(), *arity)).unwrap();
            for (i, slot) in entry.iter_mut().enumerate() {
                if consumed.contains(&i) && *slot == Mode::Borrow {
                    *slot = Mode::Consume;
                    changed = true;
                }
            }
        }
        if !changed {
            return sig;
        }
    }
}

/// Which parameter indices of `(name, arity)` are used in a consuming position
/// in some arm, given the current signature estimate.
fn consumed_params(
    program: &Program,
    sig: &HashMap<(String, usize), Vec<Mode>>,
    name: &str,
    arity: usize,
) -> HashSet<usize> {
    let mut consumed = HashSet::new();
    for decl in program.fns.iter().filter(|d| d.name == name && d.params.len() == arity) {
        let mut names_consumed: HashSet<String> = HashSet::new();
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } => expr,
                Stmt::Expr(e) => e,
            };
            // every statement's value flows toward the result; the callee slot
            // modes still pull genuine reads back to Borrow inside it.
            walk(sig, e, Mode::Consume, &mut names_consumed);
        }
        for (i, pat) in decl.params.iter().enumerate() {
            if let Pattern::Var(pname, _) = pat {
                if names_consumed.contains(pname) {
                    consumed.insert(i);
                }
            }
        }
    }
    consumed
}

/// Argument modes to apply at a call to `fname`/`arity`: the builtin table, else
/// the current user signature, else conservative all-`Consume`.
fn call_modes(sig: &HashMap<(String, usize), Vec<Mode>>, fname: &str, arity: usize) -> Vec<Mode> {
    builtin_modes(fname, arity)
        .or_else(|| sig.get(&(fname.to_string(), arity)).cloned())
        .unwrap_or_else(|| vec![Mode::Consume; arity])
}

/// Record each identifier occurrence used in `Consume` position. `mode` is the
/// mode in which `e`'s own value is used; call arguments take their mode from
/// the callee, not from `mode`.
fn walk(
    sig: &HashMap<(String, usize), Vec<Mode>>,
    e: &Expr,
    mode: Mode,
    out: &mut HashSet<String>,
) {
    match e {
        Expr::Ident(name, _) => {
            if mode == Mode::Consume {
                out.insert(name.clone());
            }
        }
        Expr::App { head, args, .. } => {
            if let Expr::Ident(fname, _) = head.as_ref() {
                let modes = call_modes(sig, fname, args.len());
                for (arg, m) in args.iter().zip(modes) {
                    walk(sig, arg, m, out);
                }
            } else {
                // computed callee: read the head, treat args conservatively
                walk(sig, head, Mode::Borrow, out);
                for arg in args {
                    walk(sig, arg, Mode::Consume, out);
                }
            }
        }
        // literals fold their elements into a fresh collection: consume
        Expr::List(items, _) => {
            for item in items {
                walk(sig, item, Mode::Consume, out);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                walk(sig, k, Mode::Consume, out);
                walk(sig, v, Mode::Consume, out);
            }
        }
        // arithmetic and comparison read their operands
        Expr::BinOp { lhs, rhs, .. } => {
            walk(sig, lhs, Mode::Borrow, out);
            walk(sig, rhs, Mode::Borrow, out);
        }
        // indexing reads the container and the key (result aliasing is v2)
        Expr::Index { base, index, .. } => {
            walk(sig, base, Mode::Borrow, out);
            walk(sig, index, Mode::Borrow, out);
        }
        // sequencing: the left runs for effect, the right is the value
        Expr::Seq(a, b, _) => {
            walk(sig, a, Mode::Consume, out);
            walk(sig, b, mode, out);
        }
        // a closure captures by consuming what it closes over
        Expr::Lambda { body, .. } => walk(sig, body, Mode::Consume, out),
        // interpolation reads the value to render it
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    walk(sig, inner, Mode::Borrow, out);
                }
            }
        }
        Expr::Int(..) | Expr::Float(..) => {}
    }
}

/// Render a signature like `(cs: borrow, p: borrow, acc: consume)` for a group,
/// using the parameter names from its first declaration.
pub fn render_signature(decl: &FnDecl, modes: &[Mode]) -> String {
    let parts: Vec<String> = decl
        .params
        .iter()
        .zip(modes)
        .map(|(pat, m)| {
            let n = match pat {
                Pattern::Var(name, _) => name.clone(),
                _ => "_".to_string(),
            };
            let m = match m {
                Mode::Borrow => "borrow",
                Mode::Consume => "consume",
            };
            format!("{n}: {m}")
        })
        .collect();
    format!("{}({})", decl.name, parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::{signatures, Mode};
    use std::path::Path;

    #[test]
    fn read_only_param_is_borrow() {
        let src = "fn f x\n  length x\n\nmain = print \"{f [1 2 3]}\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        let sig = signatures(&program);
        assert_eq!(sig[&("f".to_string(), 1)][0], Mode::Borrow);
    }

    #[test]
    fn stored_param_is_consume() {
        let src = "fn g x\n  push x 1\n\nmain = print \"{length (g [1])}\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        let sig = signatures(&program);
        assert_eq!(sig[&("g".to_string(), 1)][0], Mode::Consume);
    }

    #[test]
    fn returned_param_is_consume() {
        let src = "fn h x\n  x\n\nmain = print \"{h 5}\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        let sig = signatures(&program);
        assert_eq!(sig[&("h".to_string(), 1)][0], Mode::Consume);
    }

    #[test]
    fn borrow_propagates_through_calls() {
        // x is only ever forwarded to a read-only borrow; the fixpoint must
        // carry that across the call boundary, not give up at it.
        let src = "fn a x\n  length x\n\nfn b x\n  a x\n\nmain = print \"{b [1 2]}\"\n";
        let program = crate::compile("test.kso", src, true).unwrap();
        let sig = signatures(&program);
        assert_eq!(sig[&("b".to_string(), 1)][0], Mode::Borrow);
        assert_eq!(sig[&("a".to_string(), 1)][0], Mode::Borrow);
    }

    #[test]
    fn json_scanner_input_is_borrow_accumulator_is_consume() {
        // the whole-program payoff: across the recursive-descent parser the byte
        // input `cs` is proven never consumed (pure borrow, never copied), while
        // a string accumulator threaded through `push` is consume.
        let program = crate::compile_module(Path::new("lib/json"), false).unwrap();
        let sig = signatures(&program);
        let str_char = &sig[&("_str_char".to_string(), 4)];
        assert_eq!(str_char[0], Mode::Borrow, "cs (byte input) should be borrow");
        assert_eq!(str_char[3], Mode::Consume, "acc (accumulator) should be consume");
    }
}
