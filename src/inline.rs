//! Inlining the wrappers that hide a builtin behind a name.
//!
//! `pub fn append acc x` whose whole body is `builtin_append acc x` is a
//! rename, not a function. Left alone it compiles to a real call, and every
//! analysis downstream sees a user function where the program meant a
//! primitive — the uniqueness analysis in particular, which decides whether an
//! append can extend its accumulator in place. It cannot decide that inside
//! the wrapper, because there the accumulator is a parameter whose ownership
//! belongs to a caller one frame away. On the encode board that hid forty-two
//! million appends behind three call sites.
//!
//! So the rename is undone before anything looks: a call to such a wrapper
//! becomes the call it stands for, at the site that made it.

use crate::ast::*;

/// Builtins that can produce an err, which therefore carry the origin of the
/// site that called them. Kept in step with codegen's origin-passing list.
const BIRTHS_ERR: [&str; 4] =
    ["builtin_to_int", "builtin_to_float", "builtin_utf8", "builtin_from_code"];
use crate::hash::Map as HashMap;

/// Wrapper name and arity, mapped to the builtin it stands for. An arm
/// qualifies when its body is exactly one call to a builtin, passing its own
/// parameters in their own order and nothing else — anything more and
/// treating it as the builtin would be a decision rather than a rename. A
/// rename of a rename resolves too: `fn wrapped x = text/bytes x` stands for
/// the builtin whichever module wrote the hop, so a literal argument is
/// refused at the same compile whether the chain is one link or three. The
/// type checker reads this map per arm; the call-site rewrite below adds the
/// stricter only-arm condition on top.
pub fn aliases(program: &Program) -> HashMap<(String, usize), String> {
    // The group sizes do not change while the fixpoint runs, so they are
    // counted once here rather than rebuilt on each pass.
    let mut counts: HashMap<(&str, usize), usize> = HashMap::default();
    for decl in &program.fns {
        *counts.entry((decl.name.as_str(), decl.params.len())).or_default() += 1;
    }
    let mut found = direct_aliases(program, &HashMap::default(), &counts);
    loop {
        let grown = direct_aliases(program, &found, &counts);
        if grown.len() == found.len() {
            return found;
        }
        found = grown;
    }
}

fn direct_aliases(
    program: &Program,
    known: &HashMap<(String, usize), String>,
    counts: &HashMap<(&str, usize), usize>,
) -> HashMap<(String, usize), String> {
    let mut found = HashMap::default();
    for decl in &program.fns {
        let [Stmt::Expr(Expr::App { head, args, piped: false, .. })] = decl.body.as_slice() else {
            continue;
        };
        let Expr::Ident(callee, _) = head.as_ref() else { continue };
        // A hop through a named wrapper is a rename only when that wrapper is
        // its own group's single arm — a second arm makes it a dispatch, and
        // the value could take the other door.
        //
        // Every value this map holds was written with the `builtin_` prefix on
        // it, and this arm only fires when the callee already carries one — so
        // stripping the prefix and putting it back was the identity, twice
        // allocated. The name is taken as it stands.
        let resolved: Option<&str> = match callee.starts_with("builtin_") {
            true => Some(callee.as_str()),
            false => known
                .get(&(callee.clone(), args.len()))
                .filter(|_| counts.get(&(callee.as_str(), args.len())) == Some(&1))
                .map(|v| v.as_str()),
        };
        let Some(target) = resolved else { continue };
        if args.len() != decl.params.len() {
            continue;
        }
        // A builtin that can give birth to an err takes the calling site's
        // origin, and inlining moves that site — an error that said it was
        // born in `text/to_int` would start saying it was born in whichever
        // function called it, which is worse for the reader and is a real
        // change in what the program reports. Those wrappers stay.
        if BIRTHS_ERR.iter().any(|b| target == *b) {
            continue;
        }
        let mut threads = true;
        for (param, arg) in decl.params.iter().zip(args) {
            let (Pattern::Var(name, _), Expr::Ident(passed, _)) = (param, arg) else {
                threads = false;
                break;
            };
            if name != passed {
                threads = false;
                break;
            }
        }
        if threads {
            found.insert((decl.name.clone(), decl.params.len()), target.to_string());
        }
    }
    found
}

/// Rewrite every call to a qualifying wrapper into the call it stands for.
/// Only a group whose forwarder is its ONLY arm is rewritten: a second arm
/// makes the group a dispatch (text/split's empty-separator err arm), and
/// sending its calls straight to the builtin would delete the dispatch that
/// reaches the other arm.
pub fn inline_builtin_wrappers(program: &mut Program) {
    let mut counts: HashMap<(&str, usize), usize> = HashMap::default();
    for decl in &program.fns {
        *counts.entry((decl.name.as_str(), decl.params.len())).or_default() += 1;
    }
    let mut alias = aliases(program);
    alias.retain(|(name, arity), _| counts.get(&(name.as_str(), *arity)) == Some(&1));
    if alias.is_empty() {
        return;
    }
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            rewrite(expr, &alias);
        }
    }
}

fn rewrite(expr: &mut Expr, alias: &HashMap<(String, usize), String>) {
    if let Expr::App { head, args, .. } = expr {
        if let Expr::Ident(name, span) = head.as_ref() {
            if let Some(builtin) = alias.get(&(name.clone(), args.len())) {
                **head = Expr::Ident(builtin.clone(), *span);
            }
        }
    }
    for child in children_mut(expr) {
        rewrite(child, alias);
    }
}

fn children_mut(expr: &mut Expr) -> Vec<&mut Expr> {
    match expr {
        Expr::App { head, args, .. } => {
            let mut out: Vec<&mut Expr> = vec![head.as_mut()];
            out.extend(args.iter_mut());
            out
        }
        Expr::BinOp { lhs, rhs, .. } | Expr::Seq(lhs, rhs, _) | Expr::Join { lhs, rhs, .. } => {
            vec![lhs.as_mut(), rhs.as_mut()]
        }
        Expr::Field { base, .. } | Expr::Upcast { expr: base, .. } => vec![base.as_mut()],
        Expr::Index { base, index, .. } => vec![base.as_mut(), index.as_mut()],
        Expr::Lambda { body, .. } => vec![body.as_mut()],
        Expr::List(items, _) => items.iter_mut().collect(),
        Expr::Guard { cond, early, rest, .. } => {
            let mut out: Vec<&mut Expr> = vec![cond.as_mut(), early.as_mut()];
            for stmt in rest.iter_mut() {
                out.push(match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                    Stmt::Set { value, .. } => value,
                });
            }
            out
        }
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts
            .iter_mut()
            .map(|s| match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            })
            .collect(),
        Expr::Str(parts, _) => parts
            .iter_mut()
            .filter_map(|p| match p {
                TemplatePart::Interp(e) => Some(e),
                TemplatePart::Lit(_) => None,
            })
            .collect(),
        Expr::MapLit(pairs, _) => pairs.iter_mut().flat_map(|(k, v)| [k, v]).collect(),
        _ => Vec::new(),
    }
}
