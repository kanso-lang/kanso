// The checks on a group's arms: arms that overlap, a typeset each member of
// which some arm answers, overload ranks, and a bare call two arms could
// both take. check.rs reads this file with include!, so the items stay in
// the check module and the compiler builds to the same instructions it did
// when they sat in check.rs (kanso#1867). cargo fmt does not reach an
// included file, so CI's format job checks the files under src/'s
// subdirectories itself.

/// Two arms of one group with the same shape: the second can never be
/// reached, whichever file it is in. The ranking check below compares
/// neighbours, which is exactly right for arms written together and blind to a
/// module — a directory of files sharing one namespace, where the arms of a
/// group need not sit next to each other or even in the same file. So the
/// unreachable-arm half runs here, over the whole module at once.
///
/// The declarations are sorted once so that each group's arms sit together,
/// in source order, and an arm is compared only with its own group. Looking
/// back over every declaration for every arm compared each pair in the module,
/// which the entry corpus paid for in millions of instructions. What is
/// reported is the later arm, and the reports are put back in source order.
fn check_overlapping_arms(program: &Program, diags: &mut Vec<Diagnostic>) {
    let mut real: Vec<(usize, &FnDecl)> =
        program.fns.iter().filter(|d| !d.synthetic).enumerate().collect();
    real.sort_unstable_by(|(i, a), (j, b)| {
        (a.name.as_str(), a.params.len(), i).cmp(&(b.name.as_str(), b.params.len(), j))
    });
    let mut flat = None;
    let mut found: Vec<(usize, Diagnostic)> = Vec::new();
    let groups =
        real.chunk_by(|(_, a), (_, b)| a.name == b.name && a.params.len() == b.params.len());
    // A field's reader overlaps itself only when a record names the field
    // twice, which the type check reports in the reader's own words.
    for group in groups.filter(|g| g.len() > 1 && getter_field(&g[0].1.name).is_none()) {
        for (g, &(at, later)) in group.iter().enumerate() {
            let earlier = || group[..g].iter().map(|&(_, d)| d);
            if earlier().any(|e| same_shape(&e.params, &later.params)) {
                let why = format!("overlapping overloads of `{}` are illegal", later.name);
                found.push((at, Diagnostic::new("dispatch", why, later.span).about(&later.file)));
                continue;
            }
            let covered = earlier()
                .find_map(|e| typeset_covers(program, &mut flat, &e.params, &later.params));
            if let Some((narrow, wide, same)) = covered {
                let why = match same {
                    true => format!(
                        "`{narrow}` and `{wide}` hold the same types, and the arm for `{wide}` \
                         comes first, so this arm can never run"
                    ),
                    false => format!(
                        "every member of `{narrow}` is a member of `{wide}`, and the arm for \
                         `{wide}` comes first, so this arm can never run; put it above that one"
                    ),
                };
                found.push((at, Diagnostic::new("dispatch", why, later.span).about(&later.file)));
                continue;
            }
            if let Some(ty) = members_each_answered(program, &mut flat, group, g) {
                let why = format!(
                    "every value a `{ty}` can be has an arm that takes it before this one, so \
                     this arm can never run"
                );
                found.push((at, Diagnostic::new("dispatch", why, later.span).about(&later.file)));
                continue;
            }
            let shadowed = earlier()
                .any(|e| e.params.iter().zip(&later.params).all(|(p, q)| covers(p, q, true)));
            if shadowed {
                let why = "an arm of this group written above takes every value this one does, \
                           so this arm can never run; put it above that one"
                    .to_string();
                found.push((at, Diagnostic::new("dispatch", why, later.span).about(&later.file)));
            }
        }
    }
    found.sort_by_key(|&(at, _)| at);
    diags.extend(found.into_iter().map(|(_, d)| d));
}

/// Whether a pattern takes every value another does, where the two rank alike
/// and so the one written first wins. Two constructor patterns of one type rank
/// alike, and so do an annotation and a constructor pattern of its type, so
/// `(pt n _)` above `(pt 1 _)` leaves the second nothing. Inside a constructor
/// a binder takes any field, since a field never holds an err, and an
/// annotation takes a literal of its type. At the top a binder ranks below
/// every type and a literal above it, so neither is compared there.
fn covers(e: &Pattern, l: &Pattern, top: bool) -> bool {
    if same_shape(std::slice::from_ref(e), std::slice::from_ref(l)) {
        return true;
    }
    match (e, l) {
        (Pattern::Var(..) | Pattern::Wildcard(..), _) => !top,
        (Pattern::Annotated { ty, .. }, Pattern::Ctor { ty: t, .. }) => ty == t,
        (Pattern::Annotated { ty, .. }, Pattern::IntLit(..)) => !top && ty == "int",
        (Pattern::Annotated { ty, .. }, Pattern::StrLit(..)) => !top && ty == "string",
        (Pattern::Annotated { ty, .. }, Pattern::Nullary(n, _)) => {
            !top && ty == "bool" && (n == "true" || n == "false")
        }
        // A bare reason ranks below every named one, so `(err _)` written
        // first still leaves `(err (woe a))` its errs.
        (Pattern::Ctor { ty: a, fields: fa, .. }, Pattern::Ctor { .. })
            if a == "err"
                && matches!(fa.as_slice(), [Pattern::Var(..) | Pattern::Wildcard(..)]) =>
        {
            false
        }
        (Pattern::Ctor { ty: a, fields: fa, .. }, Pattern::Ctor { ty: b, fields: fb, .. }) => {
            a == b && fa.len() == fb.len() && fa.iter().zip(fb).all(|(x, y)| covers(x, y, false))
        }
        _ => false,
    }
}

/// Whether each value a later arm's `bool` or typeset parameter admits is
/// taken by another arm first, the other parameters being the same shape. A
/// literal and a member type rank above a typeset or `bool`, so an arm for each
/// of them takes its values whatever the order; an earlier typeset arm takes the
/// members it holds because it comes first. `group` is the arms of one group in
/// source order and `at` is the later arm's place in it. Answers the type left
/// with nothing.
fn members_each_answered<'a>(
    program: &Program,
    flat: &mut Option<crate::hash::Map<String, Vec<String>>>,
    group: &[(usize, &'a FnDecl)],
    at: usize,
) -> Option<&'a str> {
    let later = group[at].1;
    let siblings = || group.iter().enumerate().filter(move |&(j, _)| j != at);
    let is_set = |ty: &str| program.types.iter().any(|t| t.name == ty && !t.members.is_empty());
    for (p, param) in later.params.iter().enumerate() {
        let Pattern::Annotated { ty, .. } = param else { continue };
        let rest_alike = |a: &FnDecl| {
            a.params.iter().zip(&later.params).enumerate().all(|(q, (x, y))| {
                q == p || same_shape(std::slice::from_ref(x), std::slice::from_ref(y))
            })
        };
        let answered = if ty == "bool" {
            ["true", "false"].iter().all(|lit| {
                siblings().any(|(_, &(_, a))| {
                    rest_alike(a) && matches!(&a.params[p], Pattern::Nullary(n, _) if n == lit)
                })
            })
        } else if is_set(ty) {
            let sets = flat.get_or_insert_with(|| program.flat_typesets());
            let Some(members) = sets.get(ty.as_str()) else { continue };
            members.iter().all(|m| {
                siblings().any(|(j, &(_, a))| {
                    rest_alike(a)
                        && match &a.params[p] {
                            Pattern::Annotated { ty: t, .. } => {
                                t == m
                                    || (j < at
                                        && sets.get(t.as_str()).is_some_and(|s| s.contains(m)))
                            }
                            Pattern::Ctor { ty: t, fields, .. } => {
                                t == m
                                    && fields.iter().all(|f| {
                                        matches!(f, Pattern::Var(..) | Pattern::Wildcard(..))
                                    })
                            }
                            Pattern::Nullary(n, _) => n == m,
                            _ => false,
                        }
                })
            })
        } else {
            continue;
        };
        if answered {
            return Some(ty.as_str());
        }
    }
    None
}

/// Whether an earlier arm takes every value a later one does because a typeset
/// it names holds every member of the later arm's typeset, the other parameters
/// being the same shape. Two typeset arms rank alike, so the one written first
/// wins wherever both match. An arm naming a member type is not caught here: it
/// ranks above any typeset and runs whatever the order. Answers the later and
/// earlier typeset names, and whether the two hold the same members.
fn typeset_covers<'a>(
    program: &Program,
    flat: &mut Option<crate::hash::Map<String, Vec<String>>>,
    earlier: &'a [Pattern],
    later: &'a [Pattern],
) -> Option<(&'a str, &'a str, bool)> {
    let is_set = |ty: &str| program.types.iter().any(|t| t.name == ty && !t.members.is_empty());
    let mut found = None;
    for (e, l) in earlier.iter().zip(later) {
        if same_shape(std::slice::from_ref(e), std::slice::from_ref(l)) {
            continue;
        }
        let (Pattern::Annotated { ty: wide, .. }, Pattern::Annotated { ty: narrow, .. }) = (e, l)
        else {
            return None;
        };
        if found.is_some() || !is_set(wide) || !is_set(narrow) {
            return None;
        }
        let sets = flat.get_or_insert_with(|| program.flat_typesets());
        let (Some(w), Some(n)) = (sets.get(wide.as_str()), sets.get(narrow.as_str())) else {
            return None;
        };
        if !n.iter().all(|m| w.contains(m)) {
            return None;
        }
        found = Some((narrow.as_str(), wide.as_str(), w.iter().all(|m| n.contains(m))));
    }
    found
}

fn check_overload_ranks(program: &Program, diags: &mut Vec<Diagnostic>) {
    for pair in program.fns.windows(2) {
        if pair[0].name != pair[1].name
            || pair[0].params.len() != pair[1].params.len()
            || getter_field(&pair[0].name).is_some()
        {
            continue;
        }
        let prev: Vec<u8> = pair[0].params.iter().map(Pattern::rank).collect();
        let next: Vec<u8> = pair[1].params.iter().map(Pattern::rank).collect();
        if prev > next {
            diags.push(Diagnostic::new(
                "formatting",
                format!(
                    "overloads of `{}` appear most-specific first: literal, then concrete \
                     type, then generic",
                    pair[1].name
                ),
                pair[1].span,
            ));
        }
        if prev == next && same_shape(&pair[0].params, &pair[1].params) {
            diags.push(Diagnostic::new(
                "dispatch",
                format!("overlapping overloads of `{}` are illegal", pair[1].name),
                pair[1].span,
            ));
        }
    }
}

/// INTERIM committee ruling, awaiting Clay's tie gavel: a bare call that two
/// imports answer with the very same shape is refused at the call site,
/// because the silent alternative resolves the tie by import order — and
/// imports are formatted alphabetical, so the winner would be whichever
/// module's directory sorts first. Rejecting is the conservative reading: a
/// refused program can be given meaning later; a silently-resolved one is a
/// commitment. Qualified calls stay legal, and a local arm of the same shape
/// still shadows the imports, per the ruled precedence.
fn check_bare_ambiguity(program: &Program, diags: &mut Vec<Diagnostic>) {
    use crate::hash::Map as HashMap;
    let origin_of = |d: &FnDecl| -> Option<String> {
        program
            .fns
            .iter()
            .find(|t| {
                !t.synthetic
                    && t.file == d.file
                    && t.span.line == d.span.line
                    && t.span.col == d.span.col
                    && t.params.len() == d.params.len()
                    && t.name.ends_with(&format!("/{}", d.name))
            })
            .map(|t| t.name.clone())
    };
    let mut torn: HashMap<(&str, usize), Vec<String>> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for (i, a) in program.fns.iter().enumerate() {
        if !a.synthetic || crate::ast::has_slash(&a.name) {
            continue;
        }
        for b in program.fns.iter().skip(i + 1) {
            if !b.synthetic
                || b.name != a.name
                || b.params.len() != a.params.len()
                || !same_shape(&a.params, &b.params)
            {
                continue;
            }
            let (Some(oa), Some(ob)) = (origin_of(a), origin_of(b)) else { continue };
            if oa == ob {
                continue;
            }
            let shadowed = program.fns.iter().any(|l| {
                !l.synthetic
                    && l.name == a.name
                    && l.params.len() == a.params.len()
                    && same_shape(&l.params, &a.params)
            });
            if shadowed {
                continue;
            }
            let entry = torn.entry((a.name.as_str(), a.params.len())).or_default();
            for o in [oa, ob] {
                if !entry.contains(&o) {
                    entry.push(o);
                }
            }
        }
    }
    if torn.is_empty() {
        return;
    }
    fn walk(e: &Expr, torn: &HashMap<(&str, usize), Vec<String>>, diags: &mut Vec<Diagnostic>) {
        if let Expr::App { head, args, .. } = e {
            if let Expr::Ident(name, span, _) = head.as_ref() {
                if let Some(origins) = torn.get(&(name.as_str(), args.len())) {
                    let spellings = origins.join("` and `");
                    diags.push(Diagnostic::new(
                        "dispatch",
                        format!(
                            "a bare `{name}` reaches `{spellings}` alike — say which \
                             with a qualifier"
                        ),
                        *span,
                    ));
                }
            }
        }
        crate::for_each_child(e, |child| walk(child, torn, diags));
    }
    for d in &program.fns {
        if d.synthetic || crate::ast::has_slash(&d.name) {
            continue;
        }
        let bound: crate::hash::Set<&str> = d
            .params
            .iter()
            .filter_map(|p| match p {
                Pattern::Var(n, _) | Pattern::Annotated { name: n, .. } => Some(n.as_str()),
                _ => None,
            })
            .collect();
        let live: HashMap<(&str, usize), Vec<String>> = torn
            .iter()
            .filter(|((n, _), _)| !bound.contains(*n))
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        if live.is_empty() {
            continue;
        }
        for stmt in &d.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => walk(expr, &live, diags),
            }
        }
    }
}

/// What an arm can learn about a value from an annotation. Every effect is one
/// kind, and a list or a map is asked only whether it is one, whatever the
/// annotation says it holds, so `_:[]int` and `_:[]string` take the same
/// values. Any other annotation names what it tests.
fn tested_as(ty: &str) -> &str {
    if crate::ast::is_effect_type(ty) {
        "an effect"
    } else if ty.ends_with("[]") {
        "a list"
    } else if ty.contains('[') {
        "a map"
    } else {
        ty
    }
}

fn same_shape(a: &[Pattern], b: &[Pattern]) -> bool {
    a.iter().zip(b.iter()).all(|(pa, pb)| match (pa, pb) {
        (Pattern::IntLit(x, _), Pattern::IntLit(y, _)) => x == y,
        (Pattern::StrLit(x, _), Pattern::StrLit(y, _)) => x == y,
        (Pattern::Nullary(x, _), Pattern::Nullary(y, _)) => x == y,
        (Pattern::Annotated { ty: x, .. }, Pattern::Annotated { ty: y, .. }) => {
            x == y || tested_as(x) == tested_as(y)
        }
        (Pattern::Ctor { ty: x, fields: fa, .. }, Pattern::Ctor { ty: y, fields: fb, .. }) => {
            x == y && fa.len() == fb.len() && same_shape(fa, fb)
        }
        // A constructor pattern that names every field and tests none takes
        // what the annotation takes: `(pt a b)` is every `pt`, and so is
        // `_:pt`. The engines choose between the two by rank or by order, and
        // whichever they choose, the other arm never runs.
        (Pattern::Annotated { ty: x, .. }, Pattern::Ctor { ty: y, fields, .. })
        | (Pattern::Ctor { ty: y, fields, .. }, Pattern::Annotated { ty: x, .. }) => {
            x == y && fields.iter().all(|f| matches!(f, Pattern::Var(..) | Pattern::Wildcard(..)))
        }
        // `none` and `done` are each the one value of their type, so the
        // literal and the annotation take the same thing.
        (Pattern::Nullary(x, _), Pattern::Annotated { ty: y, .. })
        | (Pattern::Annotated { ty: y, .. }, Pattern::Nullary(x, _)) => x == y,
        (Pattern::Var(..) | Pattern::Wildcard(..), Pattern::Var(..) | Pattern::Wildcard(..)) => {
            true
        }
        _ => false,
    })
}
