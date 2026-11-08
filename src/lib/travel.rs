// Arms that travel with a value (the 2026-10-12 gavel on #1894). A value
// carries the arms its type's owner wrote, so code that was handed one may
// call them bare without importing the owner. The check finds those calls
// (`check::check_file_shadow` hands them back) and they are renamed here into
// a traveling group, which the whole program fills with every owner's arms of
// that name. lib.rs reads this file with include!, so the items stay in the
// crate's root module. cargo fmt does not reach an included file, so CI's
// format job checks the files under src/'s subdirectories itself.

/// The traveling group of a name. The mark is a character the lexer never
/// makes part of a name, so no program can write the group's spelling, and a
/// call reaches it only by being renamed into it here.
fn traveling(name: &str) -> String {
    format!("~{name}")
}

/// Renames each call the check found travelling into its group, and hands
/// each declaration the shaded calls it holds. A file's spans are unique
/// within it, and the names are checked as well, so a local of the same name
/// elsewhere in the file keeps its own.
fn rename_travels(program: &mut ast::Program, travels: &[check::Travel]) {
    if travels.is_empty() {
        return;
    }
    let mut shaded: Vec<(usize, diag::Span, &Name)> =
        travels.iter().filter_map(|(name, span, at)| at.map(|i| (i, *span, name))).collect();
    shaded.sort_by_key(|(i, ..)| *i);
    for run in shaded.chunk_by(|a, b| a.0 == b.0) {
        program.fns[run[0].0].shades =
            Some(Box::new(run.iter().map(|&(_, span, name)| (span, name.clone())).collect()));
    }
    let at: crate::hash::Map<(u32, u32), &str> = travels
        .iter()
        .filter(|(.., at)| at.is_none())
        .map(|(name, span, _)| ((span.line, span.col), name.as_str()))
        .collect();
    if at.is_empty() {
        return;
    }
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            travel_stmt(stmt, &at);
        }
    }
}

fn travel_stmt(stmt: &mut ast::Stmt, at: &crate::hash::Map<(u32, u32), &str>) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => travel_expr(expr, at),
    }
}

fn travel_expr(e: &mut ast::Expr, at: &crate::hash::Map<(u32, u32), &str>) {
    match e {
        ast::Expr::App { head, args, .. } => {
            if let ast::Expr::Ident(name, span, _) = &mut **head {
                if at.get(&(span.line, span.col)) == Some(&name.as_str()) {
                    *name = Name::new(&traveling(name.as_str()));
                }
            } else {
                travel_expr(head, at);
            }
            for a in args {
                travel_expr(a, at);
            }
        }
        ast::Expr::Ident(..) | ast::Expr::Partial(..) => {}
        ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                travel_expr(k, at);
                travel_expr(v, at);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    travel_expr(inner, at);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                travel_expr(item, at);
            }
        }
        ast::Expr::Field { base, .. } => travel_expr(base, at),
        ast::Expr::Index { base, index, .. } => {
            travel_expr(base, at);
            travel_expr(index, at);
        }
        ast::Expr::Join { lhs, rhs, .. } | ast::Expr::BinOp { lhs, rhs, .. } => {
            travel_expr(lhs, at);
            travel_expr(rhs, at);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            travel_expr(body, at)
        }
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                travel_stmt(stmt, at);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            travel_expr(cond, at);
            travel_expr(early, at);
            for stmt in rest.iter_mut() {
                travel_stmt(stmt, at);
            }
        }
    }
}

/// A file of the program's own holds unqualified declarations; a module's
/// file can also hold an arm qualified for another group, `render/to_string`
/// among them, so a file is a module's only when it holds nothing bare, and
/// then the qualifier most of its declarations carry is the module's. Asked
/// only of the files named.
fn file_modules(
    fns: &[ast::FnDecl],
    files: &[&std::sync::Arc<str>],
) -> crate::hash::Map<std::sync::Arc<str>, Option<String>> {
    let mut counts: crate::hash::Map<std::sync::Arc<str>, crate::hash::Map<Option<&str>, usize>> =
        crate::hash::Map::default();
    for decl in fns.iter().filter(|d| !d.synthetic && !d.is_getter()) {
        if !files.iter().any(|f| std::sync::Arc::ptr_eq(f, &decl.file) || **f == decl.file) {
            continue;
        }
        let qual = ast::split_qual(&decl.name).map(|(q, _)| q);
        *counts.entry(decl.file.clone()).or_default().entry(qual).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(file, quals)| {
            let qual = match quals.contains_key(&None) {
                true => None,
                false => quals
                    .into_iter()
                    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                    .and_then(|(q, _)| q.map(str::to_string)),
            };
            (file, qual)
        })
        .collect()
}

/// Drops what each declaration recorded of its shaded calls. A program no
/// root enrolls keeps every such call on the module's own arms, and the
/// wrapper rewrite may then reach it like any other call.
fn forget_shades(fns: &mut [ast::FnDecl]) {
    for decl in fns.iter_mut() {
        decl.shades = None;
    }
}

/// The group a module's own function of that name belongs to.
fn local_group(qual: Option<&str>, short: &str) -> String {
    match qual {
        Some(q) => ast::qualified(q, short),
        None => short.to_string(),
    }
}

/// The group a shaded call joins when an owner's arm would: the module's arms
/// and the owners' arms of other shapes, kept apart from every other module's.
fn shaded_group(qual: Option<&str>, short: &str) -> String {
    format!("~{}~{short}", qual.unwrap_or(""))
}

/// Every traveling group a whole program's calls name, each with the first
/// call that named it: the call's span and its declaration's file.
fn traveling_calls(fns: &[ast::FnDecl]) -> Vec<(String, diag::Span, std::sync::Arc<str>)> {
    let mut found = Vec::new();
    let mut seen: crate::hash::Set<&str> = crate::hash::Set::default();
    for decl in fns {
        for stmt in &decl.body {
            stmt_travels(stmt, &mut |short, span| {
                if seen.insert(short) {
                    found.push((traveling(short), span, decl.file.clone()));
                }
            });
        }
    }
    found
}

/// Hands each traveling call a body spells to `f`: its short name and span.
/// A name already shaded, or spelled with a module, is not one.
fn stmt_travels<'a>(stmt: &'a ast::Stmt, f: &mut impl FnMut(&'a str, diag::Span)) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr_travels(expr, f),
    }
}

fn expr_travels<'a>(e: &'a ast::Expr, f: &mut impl FnMut(&'a str, diag::Span)) {
    match e {
        ast::Expr::Ident(name, span, _) => {
            let Some(short) = name.as_str().strip_prefix('~') else { return };
            if !short.contains('~') && !ast::has_slash(short) {
                f(short, *span);
            }
        }
        ast::Expr::Partial(..) | ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                expr_travels(k, f);
                expr_travels(v, f);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    expr_travels(inner, f);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                expr_travels(item, f);
            }
        }
        ast::Expr::App { head, args, .. } => {
            expr_travels(head, f);
            for a in args {
                expr_travels(a, f);
            }
        }
        ast::Expr::Field { base, .. } => expr_travels(base, f),
        ast::Expr::Index { base, index, .. } => {
            expr_travels(base, f);
            expr_travels(index, f);
        }
        ast::Expr::Join { lhs, rhs, .. } | ast::Expr::BinOp { lhs, rhs, .. } => {
            expr_travels(lhs, f);
            expr_travels(rhs, f);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            expr_travels(body, f)
        }
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                stmt_travels(stmt, f);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            expr_travels(cond, f);
            expr_travels(early, f);
            for stmt in rest.iter() {
                stmt_travels(stmt, f);
            }
        }
    }
}

/// The type a parameter matches on, if it names one: a record, subtype or
/// nullary value, read through a bare clone to the declaration it copies.
fn parameter_type(param: &ast::Pattern, origins: &crate::hash::Map<&str, &str>) -> Option<String> {
    let ty = match param {
        ast::Pattern::Ctor { ty, .. } => ty.as_str(),
        ast::Pattern::Annotated { ty, .. } => ty.as_str(),
        ast::Pattern::Nullary(ty, _) => ty.as_str(),
        _ => return None,
    };
    Some(origins.get(ty).copied().unwrap_or(ty).to_string())
}

/// An arm's shape: how many parameters it takes and the type each matches.
/// A local arm shadows an owner's arm of the same shape (rule 4).
fn shape(decl: &ast::FnDecl, origins: &crate::hash::Map<&str, &str>) -> Vec<Option<String>> {
    decl.params.iter().map(|p| parameter_type(p, origins)).collect()
}

/// Whether a parameter names a type the declaring module owns: one of its
/// own records, subtypes or nullary values, spelled `qual/name` once the
/// module is qualified.
fn owned_parameter(param: &ast::Pattern, qual: &str, owned: &crate::hash::Set<&str>) -> bool {
    let ty = match param {
        ast::Pattern::Ctor { ty, .. } => ty.as_str(),
        ast::Pattern::Annotated { ty, .. } => ty.as_str(),
        ast::Pattern::Nullary(ty, _) => ty.as_str(),
        _ => return false,
    };
    owned.contains(ty) && ast::split_qual(ty).is_some_and(|(q, _)| q == qual)
}

/// Fills each traveling group a whole program calls with every arm of that
/// name an owner wrote: a pub arm with a parameter of a type its module owns
/// (rule 4 of the gavel, so a value's arms come from one owner). A shaded
/// call, of a function its module declares too, goes to a group of its own
/// when an owner's arm of a shape none of the module's has would join it, and
/// otherwise stays where it is. Answers the calls whose group stayed empty,
/// rendered for the file that wrote each.
fn enroll_travelers(program: &mut ast::Program) -> Result<(), String> {
    let calls = traveling_calls(&program.fns);
    let shading = program.fns.iter().any(|d| d.shades.is_some());
    if calls.is_empty() && !shading {
        return Ok(());
    }
    // Decided over the program as it stands, by index: each group filled
    // with the declarations it clones, and each shaded call sent to a group.
    let (fills, sent) = {
        let fns = &program.fns;
        let mut wanted: crate::hash::Set<&str> = calls.iter().map(|(g, ..)| &g[1..]).collect();
        wanted.extend(fns.iter().flat_map(|d| d.shaded().iter().map(|(_, n)| n.as_str())));
        let owned: crate::hash::Set<&str> =
            program.types.iter().filter(|t| !t.synthetic).map(|t| t.name.as_str()).collect();
        let mut owners: crate::hash::Map<&str, Vec<usize>> = crate::hash::Map::default();
        for (i, decl) in fns.iter().enumerate() {
            if decl.synthetic || !decl.written_pub {
                continue;
            }
            let Some((qual, short)) = ast::split_qual(&decl.name) else { continue };
            if wanted.contains(short)
                && decl.params.iter().any(|p| owned_parameter(p, qual, &owned))
            {
                owners.entry(short).or_default().push(i);
            }
        }
        let mut fills: Vec<(String, Vec<usize>)> = calls
            .iter()
            .map(|(group, ..)| {
                (group.clone(), owners.get(&group[1..]).cloned().unwrap_or_default())
            })
            .collect();
        let mut sent: Vec<(usize, diag::Span, Name)> = Vec::new();
        // Most shaded calls name a function no owner writes, and they are
        // settled here without asking which module made them.
        let shaded: Vec<(usize, diag::Span, &str)> = fns
            .iter()
            .enumerate()
            .flat_map(|(i, d)| d.shaded().iter().map(move |(span, n)| (i, *span, n.as_str())))
            .filter(|(_, _, short)| owners.contains_key(short))
            .collect();
        if !shaded.is_empty() {
            let origins: crate::hash::Map<&str, &str> = program
                .types
                .iter()
                .filter_map(|t| t.origin.as_deref().map(|o| (t.name.as_str(), o)))
                .collect();
            let mut files: Vec<&std::sync::Arc<str>> = Vec::new();
            for &(i, ..) in &shaded {
                if !files.iter().any(|f| std::sync::Arc::ptr_eq(f, &fns[i].file)) {
                    files.push(&fns[i].file);
                }
            }
            let modules = file_modules(fns, &files);
            let located: Vec<(usize, diag::Span, Option<&str>, &str, String)> = shaded
                .into_iter()
                .map(|(i, span, short)| {
                    let qual = modules.get(&fns[i].file).and_then(|q| q.as_deref());
                    (i, span, qual, short, local_group(qual, short))
                })
                .collect();
            // the module's own arms of each name a call shades, found in one pass
            let mut mine: crate::hash::Map<&str, Vec<usize>> =
                located.iter().map(|l| (l.4.as_str(), Vec::new())).collect();
            let mut shorts: Vec<&str> = located.iter().map(|l| l.3).collect();
            shorts.sort_unstable();
            shorts.dedup();
            for (j, decl) in fns.iter().enumerate() {
                let short = ast::split_qual(&decl.name).map_or(decl.name.as_str(), |(_, s)| s);
                if !shorts.contains(&short) {
                    continue;
                }
                if let Some(arms) = mine.get_mut(decl.name.as_str()) {
                    arms.push(j);
                }
            }
            let mut decided: crate::hash::Map<&str, Option<Name>> = crate::hash::Map::default();
            for (i, span, qual, short, local) in &located {
                let group = decided.entry(local.as_str()).or_insert_with(|| {
                    let own = &mine[local.as_str()];
                    let taken: Vec<Vec<Option<String>>> =
                        own.iter().map(|&j| shape(&fns[j], &origins)).collect();
                    let joining: Vec<usize> = owners[short]
                        .iter()
                        .copied()
                        .filter(|&j| !taken.contains(&shape(&fns[j], &origins)))
                        .collect();
                    if own.is_empty() || joining.is_empty() {
                        return None;
                    }
                    let group = shaded_group(*qual, short);
                    fills.push((group.clone(), own.iter().copied().chain(joining).collect()));
                    Some(Name::new(&group))
                });
                if let Some(group) = group {
                    sent.push((*i, *span, group.clone()));
                }
            }
        }
        (fills, sent)
    };
    // The calls move before any arm is cloned, so a clone of a declaration
    // that holds one calls the group too.
    send_shaded(&mut program.fns, sent);
    let mut arms = Vec::new();
    let mut filled = crate::hash::Set::default();
    for (group, from) in &fills {
        for &j in from {
            let mut arm = program.fns[j].clone();
            arm.name = group.clone();
            arm.synthetic = true;
            arms.push(arm);
        }
        if !from.is_empty() {
            filled.insert(group.as_str());
        }
    }
    program.fns.extend(arms);
    forget_shades(&mut program.fns);
    let mut rendered = String::new();
    for (group, span, file) in &calls {
        if filled.contains(group.as_str()) {
            continue;
        }
        let short = &group[1..];
        let d = diag::Diagnostic::new(
            "name",
            format!(
                "unknown name `{short}` — no module in this program writes `{short}` for a \
                 type it owns, so no value carries it"
            ),
            *span,
        );
        let source = std::fs::read_to_string(&**file).unwrap_or_default();
        rendered.push_str(&diag::render(&[d], file, &source));
    }
    match rendered.is_empty() {
        true => Ok(()),
        false => Err(rendered),
    }
}

/// Renames each shaded call sent to a group, visiting only the declarations
/// that hold one.
fn send_shaded(fns: &mut [ast::FnDecl], mut sent: Vec<(usize, diag::Span, Name)>) {
    sent.sort_by_key(|(i, ..)| *i);
    for run in sent.chunk_by(|a, b| a.0 == b.0) {
        let decl = &mut fns[run[0].0];
        let file = decl.file.clone();
        for stmt in &mut decl.body {
            name_stmt(stmt, &file, &mut |_, name, span| {
                if let Some((.., group)) = run.iter().find(|(_, at, _)| *at == span) {
                    *name = group.clone();
                }
            });
        }
    }
}

fn name_stmt(
    stmt: &mut ast::Stmt,
    file: &std::sync::Arc<str>,
    f: &mut impl FnMut(&std::sync::Arc<str>, &mut Name, diag::Span),
) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => name_expr(expr, file, f),
    }
}

fn name_expr(
    e: &mut ast::Expr,
    file: &std::sync::Arc<str>,
    f: &mut impl FnMut(&std::sync::Arc<str>, &mut Name, diag::Span),
) {
    match e {
        ast::Expr::Ident(name, span, _) => f(file, name, *span),
        ast::Expr::Partial(..) | ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                name_expr(k, file, f);
                name_expr(v, file, f);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    name_expr(inner, file, f);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                name_expr(item, file, f);
            }
        }
        ast::Expr::App { head, args, .. } => {
            name_expr(head, file, f);
            for a in args {
                name_expr(a, file, f);
            }
        }
        ast::Expr::Field { base, .. } => name_expr(base, file, f),
        ast::Expr::Index { base, index, .. } => {
            name_expr(base, file, f);
            name_expr(index, file, f);
        }
        ast::Expr::Join { lhs, rhs, .. } | ast::Expr::BinOp { lhs, rhs, .. } => {
            name_expr(lhs, file, f);
            name_expr(rhs, file, f);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            name_expr(body, file, f)
        }
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                name_stmt(stmt, file, f);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            name_expr(cond, file, f);
            name_expr(early, file, f);
            for stmt in rest.iter_mut() {
                name_stmt(stmt, file, f);
            }
        }
    }
}
