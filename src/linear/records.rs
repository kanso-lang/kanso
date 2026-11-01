//! Constructor sites that may build into a record that is finished: the second
//! of the three answers `for_the_emitter` gives, read off the shared `Analysis`.

use super::*;

/// Constructor sites that may build into a record that is finished.
///
/// `shift (n - 1) (point (p.x + 1) p.y)` reads every field it needs before the
/// constructor runs, so by then `p` is done with. When every caller hands the
/// parameter over uniquely, and the only mentions of it in the arm are those
/// reads, its storage is free and the new record can go there.
///
/// The runtime keeps the other half: it writes through only into a record of
/// the same width, so a mismatch — or the shared zero-field marker — falls
/// back to allocating.
pub fn reusable_records(program: &Program) -> HashMap<(std::sync::Arc<str>, usize, usize), String> {
    reusable_records_with(&Analysis::new(program), program)
}

/// The body of `reusable_records`, reading an `Analysis` it was handed.
pub(super) fn reusable_records_with(
    analysis: &Analysis,
    program: &Program,
) -> HashMap<(std::sync::Arc<str>, usize, usize), String> {
    let types: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    let mut out = HashMap::default();
    for decl in real_fns(program) {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            walk_for_reuse(analysis, decl, e, &types, &mut out);
        }
    }
    out
}

fn walk_for_reuse(
    a: &Analysis,
    decl: &FnDecl,
    e: &Expr,
    types: &HashSet<&str>,
    out: &mut HashMap<(std::sync::Arc<str>, usize, usize), String>,
) {
    // a lambda runs when somebody else decides, so what is finished inside one
    // is not knowable from here
    if matches!(e, Expr::Lambda { .. }) {
        return;
    }
    if let Expr::App { head, args, span, .. } = e {
        if let Expr::Ident(name, _, _) = head.as_ref() {
            if types.contains(name.as_str()) && !args.is_empty() {
                if let Some(victim) = sole_finished_record(a, decl, args) {
                    out.insert((decl.file.clone(), span.line as usize, span.col as usize), victim);
                }
            }
        }
    }
    for child in child_exprs(e) {
        walk_for_reuse(a, decl, child, types, out);
    }
}

/// The one parameter these arguments read, when it is read nowhere else in the
/// arm and every caller hands it over.
fn sole_finished_record(a: &Analysis, decl: &FnDecl, args: &[Expr]) -> Option<String> {
    let arity = decl.params.len();
    let mut candidate: Option<String> = None;
    for (i, pattern) in decl.params.iter().enumerate() {
        let Pattern::Var(name, _) = pattern else { continue };
        let here: usize = args.iter().map(|arg| count_in_expr(name, arg)).sum();
        if here == 0 {
            continue;
        }
        // Finished means read from, and a field read is the only read that
        // leaves nothing behind. A bare mention stores the record itself --
        // `node n i` puts the previous node in the new node's first field --
        // and building into its storage would make the new node point at
        // its own slot: a chain of records came out cyclic on native and
        // correct on the oracle until 2026-09-10.
        let through_fields: usize = args.iter().map(|arg| field_reads_in(name, arg)).sum();
        if here != through_fields {
            continue;
        }
        let everywhere: usize = decl
            .body
            .iter()
            .map(|s| match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => count_in_expr(name, expr),
            })
            .sum();
        // read here and nowhere else, or somebody outlives the constructor
        if here != everywhere || !a.callers_hand_over(&decl.name, arity, i) {
            continue;
        }
        if candidate.is_some() {
            return None;
        }
        candidate = Some(name.to_string());
    }
    candidate
}
