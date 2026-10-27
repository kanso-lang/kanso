// Arms no value in the program can reach, found and dropped before anything
// is emitted. codegen.rs reads this file with include! where these items
// stood, so they stay in the codegen module.

/// A group's name and arity, and a position in it.
type UnreadPositions = Vec<(String, usize, usize)>;

/// Arms no value in the program can reach, dropped before anything is emitted.
///
/// An arm whose parameter pattern names a record type matches only a value of
/// that type, and the only way a value of a declared type comes to exist is an
/// expression that names the type: a construction, a partial of it, a bare
/// constructor handed on as a function, or an upcast to it. So a type that no
/// expression anywhere in the program names never has a value, and neither does
/// a subtype of it that is never built. `entries` is the one builder outside
/// the program's text, and it makes `entry`, id 0. An arm matching an unbuilt
/// type, at any depth of its pattern, can never be chosen, and what only it
/// calls is dead with it -- which the prune after emission then removes.
///
/// std/list's `next` dispatches over every lazy adapter the library declares,
/// so a program that maps once emitted every adapter's arm and everything each
/// arm calls: 1,560 of the codegen corpus's 4,628 lines of IR were adapters it
/// never builds and what they call. A group keeps all its arms if
/// every one of them would go, so a call that reaches it still fails the way
/// it did. The interpreter is untouched and the differential corpus is the
/// check that the two still agree.
fn without_unbuilt_arms(program: &Program) -> Option<(Vec<bool>, UnreadPositions)> {
    use crate::ast::{Expr, Pattern, Stmt, TemplatePart};
    let mut ids: HashMap<&str, i64> = HashMap::default();
    ids.insert("entry", 0);
    for (i, ty) in program.types.iter().enumerate() {
        ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    for ty in &program.types {
        if let Some(id) = ty.origin.as_deref().and_then(|o| ids.get(o).copied()) {
            ids.insert(ty.name.as_str(), id);
        }
    }
    let mut built: crate::hash::Set<i64> = crate::hash::Set::default();
    built.insert(0);
    fn names_in_stmt<'a>(s: &'a Stmt, out: &mut Vec<&'a str>) {
        match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => names_in(expr, out),
        }
    }
    fn names_in<'a>(e: &'a Expr, out: &mut Vec<&'a str>) {
        match e {
            Expr::Int(..) | Expr::Float(..) => {}
            Expr::Ident(n, _, _) | Expr::Partial(n, _) => out.push(n.as_str()),
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    names_in(k, out);
                    names_in(v, out);
                }
            }
            Expr::Str(parts, _) => {
                for p in parts {
                    if let TemplatePart::Interp(inner) = p {
                        names_in(inner, out);
                    }
                }
            }
            Expr::List(items, _) => items.iter().for_each(|x| names_in(x, out)),
            Expr::App { head, args, .. } => {
                names_in(head, out);
                args.iter().for_each(|x| names_in(x, out));
            }
            Expr::Field { base, .. } => names_in(base, out),
            Expr::Index { base, index, .. } => {
                names_in(base, out);
                names_in(index, out);
            }
            Expr::BinOp { lhs: a, rhs: b, .. } | Expr::Join { lhs: a, rhs: b, .. } => {
                names_in(a, out);
                names_in(b, out);
            }
            Expr::Lambda { body, .. } => names_in(body, out),
            Expr::Block(stmts, _) => stmts.iter().for_each(|s| names_in_stmt(s, out)),
            Expr::Upcast { expr, ty, .. } => {
                names_in(expr, out);
                out.push(ty.as_str());
            }
            Expr::Guard { cond, early, rest, .. } => {
                names_in(cond, out);
                names_in(early, out);
                rest.iter().for_each(|s| names_in_stmt(s, out));
            }
        }
    }
    fn unbuilt(p: &Pattern, ids: &HashMap<&str, i64>, built: &crate::hash::Set<i64>) -> bool {
        match p {
            Pattern::Ctor { ty, fields, .. } => {
                ids.get(ty.as_str()).is_some_and(|id| !built.contains(id))
                    || fields.iter().any(|f| unbuilt(f, ids, built))
            }
            _ => false,
        }
    }
    // Only a body that can run builds anything. std/list declares a builder
    // for every adapter, and counting names over every declaration found
    // every adapter built, whether or not the program ever called the
    // builder. So reachability and construction are one fixpoint: a group is
    // reached when a live arm names it, an arm is live when its group is
    // reached and no type in its pattern is unbuilt, and a live arm's body
    // builds what it names. The roots are the entry, every constant, and the
    // groups the emitter calls without the source naming them -- the
    // renderer and the user operators.
    let root = |d: &crate::ast::FnDecl| {
        d.name == crate::ast::ENTRY
            || d.params.is_empty()
            || d.name == RENDER_GROUP
            || !d.name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '/' | '~'))
    };
    if !program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
        return None;
    }
    let mut reached: crate::hash::Set<&str> = crate::hash::Set::default();
    for d in &program.fns {
        if root(d) {
            reached.insert(d.name.as_str());
        }
    }
    let mut read = vec![false; program.fns.len()];
    loop {
        let mut moved = false;
        for (at, d) in program.fns.iter().enumerate() {
            if read[at] || !reached.contains(d.name.as_str()) {
                continue;
            }
            if d.params.iter().any(|p| unbuilt(p, &ids, &built)) {
                continue;
            }
            read[at] = true;
            moved = true;
            let mut named: Vec<&str> = Vec::new();
            d.body.iter().for_each(|s| names_in_stmt(s, &mut named));
            for name in named {
                if let Some(id) = ids.get(name) {
                    built.insert(*id);
                }
                reached.insert(name);
            }
            // A subtype's value matches its parent's patterns, so building one
            // builds every ancestor it flows as.
            loop {
                let before = built.len();
                for ty in &program.types {
                    let (Some(child), Some(parent)) =
                        (ids.get(ty.name.as_str()), ty.parent.as_deref())
                    else {
                        continue;
                    };
                    if built.contains(child) {
                        if let Some(p) = ids.get(parent) {
                            built.insert(*p);
                        }
                    }
                }
                if built.len() == before {
                    break;
                }
            }
        }
        if !moved {
            break;
        }
    }
    let dead: Vec<bool> =
        program.fns.iter().map(|d| d.params.iter().any(|p| unbuilt(p, &ids, &built))).collect();
    if !dead.iter().any(|d| *d) {
        return None;
    }
    // A group every one of whose arms would go keeps them all. Beside each
    // group's count of live arms sits the positions those arms read, so the
    // positions only a dropped arm read fall out of the same walk.
    let reads = |d: &crate::ast::FnDecl| {
        d.params.iter().enumerate().fold(0u64, |m, (i, p)| {
            if i < 64 && !matches!(p, Pattern::Var(..) | Pattern::Wildcard(_)) {
                m | 1 << i
            } else {
                m
            }
        })
    };
    let mut live_arms: HashMap<(&str, usize), (usize, u64)> = HashMap::default();
    for (d, gone) in program.fns.iter().zip(&dead) {
        let n = live_arms.entry((d.name.as_str(), d.params.len())).or_insert((0, 0));
        if !gone {
            n.0 += 1;
            n.1 |= reads(d);
        }
    }
    let mut unread: UnreadPositions = Vec::new();
    for (d, gone) in program.fns.iter().zip(&dead) {
        let (live, kept) = live_arms[&(d.name.as_str(), d.params.len())];
        if !gone || live == 0 {
            continue;
        }
        let lost = reads(d) & !kept;
        for i in (0..d.params.len().min(64)).filter(|i| lost & 1 << i != 0) {
            unread.push((d.name.clone(), d.params.len(), i));
        }
    }
    unread.sort();
    unread.dedup();
    let keep = program
        .fns
        .iter()
        .zip(&dead)
        .map(|(d, gone)| !gone || live_arms[&(d.name.as_str(), d.params.len())].0 == 0)
        .collect();
    Some((keep, unread))
}

/// A program the emitter was handed: lent, when the caller goes on to use it,
/// or given, when it does not.
enum Given<'a> {
    Lent(&'a Program),
    Owned(Program),
}

impl std::ops::Deref for Given<'_> {
    type Target = Program;
    fn deref(&self) -> &Program {
        match self {
            Given::Lent(p) => p,
            Given::Owned(p) => p,
        }
    }
}

/// The program `without_unbuilt_arms` describes. One the caller owns loses its
/// dropped arms where it stands; one it lends is copied, arm by kept arm, since
/// the caller goes on holding the original.
fn kept_arms(program: Given<'_>, keep: &[bool]) -> Program {
    match program {
        Given::Owned(mut owned) => {
            let mut at = keep.iter();
            owned.fns.retain(|_| *at.next().unwrap_or(&true));
            owned
        }
        Given::Lent(lent) => Program {
            fns: lent.fns.iter().zip(keep).filter(|(_, k)| **k).map(|(d, _)| d.clone()).collect(),
            types: lent.types.clone(),
            imports: lent.imports.clone(),
            reexports: lent.reexports.clone(),
            root: lent.root.clone(),
        },
    }
}
