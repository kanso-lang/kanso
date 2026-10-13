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

/// Renames each call the check found travelling into its group. A file's
/// spans are unique within it, and the names are checked as well, so a local
/// of the same name elsewhere in the file keeps its own.
fn rename_travels(program: &mut ast::Program, travels: &[(String, diag::Span)]) {
    if travels.is_empty() {
        return;
    }
    let at: crate::hash::Map<(u32, u32), &str> =
        travels.iter().map(|(name, span)| ((span.line, span.col), name.as_str())).collect();
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
/// then the qualifier most of its declarations carry is the module's.
fn file_modules(fns: &[ast::FnDecl]) -> crate::hash::Map<std::sync::Arc<str>, Option<String>> {
    let mut counts: crate::hash::Map<std::sync::Arc<str>, crate::hash::Map<Option<&str>, usize>> =
        crate::hash::Map::default();
    for decl in fns.iter().filter(|d| !d.synthetic && !d.is_getter()) {
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

/// Where a shaded call stood: its file and position, the name the check gave
/// it and the module's own group it was put back on.
type Stood = Vec<(std::sync::Arc<str>, (u32, u32), String, String)>;

/// Puts each module's call of its own function back on the module's own
/// group, and answers where each stood. A program no root enrolls keeps them
/// there. A module a root imports is checked in this shape and renamed again
/// by `reshade` for the root: a shaded group is empty until the root fills
/// it, so a function only shaded calls reach would read as never called.
/// A traveling call of a name nobody declares stays as it is, for the
/// program that will hand the value over.
fn unshade(fns: &mut [ast::FnDecl]) -> Stood {
    let declares: crate::hash::Set<String> = fns.iter().map(|f| f.name.clone()).collect();
    let calls = traveling_calls(fns, &declares);
    let to: crate::hash::Map<String, (String, String)> = calls
        .into_iter()
        .filter(|(_, _, local, ..)| !local.is_empty())
        .map(|(group, short, local, ..)| (group, (local, traveling(&short))))
        .collect();
    let mut stood = Vec::new();
    if to.is_empty() {
        return stood;
    }
    each_name(fns, &mut |file, name, span| {
        if let Some((local, written)) = to.get(name.as_str()) {
            stood.push((file.clone(), (span.line, span.col), written.clone(), local.clone()));
            *name = Name::new(local);
        }
    });
    stood
}

/// Renames each call `unshade` put back as the check found it, for the root
/// that enrolls the program to find again.
fn reshade(fns: &mut [ast::FnDecl], stood: &Stood) {
    if stood.is_empty() {
        return;
    }
    let at: crate::hash::Map<(&str, (u32, u32)), usize> =
        stood.iter().enumerate().map(|(i, (file, at, ..))| ((&**file, *at), i)).collect();
    each_name(fns, &mut |file, name, span| {
        if let Some(&i) = at.get(&(&**file, (span.line, span.col))) {
            let (_, _, written, local) = &stood[i];
            if name.as_str() == local {
                *name = Name::new(written);
            }
        }
    });
}

/// The wrapper table without the groups a put-back call names. Those calls
/// travel again once the check is done, and the root may yet fill their group
/// with an owner's arm, so none of them is rewritten into the builtin here.
fn kept_traveling<V>(
    mut wrappers: crate::hash::Map<String, V>,
    stood: &Stood,
) -> crate::hash::Map<String, V> {
    for (_, _, _, local) in stood {
        wrappers.remove(local);
    }
    wrappers
}

/// The group a module's own function of that name belongs to.
fn local_group(qual: Option<&str>, short: &str) -> String {
    match qual {
        Some(q) => ast::qualified(q, short),
        None => short.to_string(),
    }
}

/// The group a traveling call joins when its module declares the name too:
/// the module's arms and the owners' arms of other shapes, kept apart from
/// every other module's.
fn shaded_group(qual: Option<&str>, short: &str) -> String {
    format!("~{}~{short}", qual.unwrap_or(""))
}

/// Renames each traveling call whose module declares the name into that
/// module's shaded group, and answers every group the program's calls name:
/// the group, the short name, the module's own group (empty when the module
/// declares none) and the first call's span and file.
fn traveling_calls(
    fns: &mut [ast::FnDecl],
    declares: &crate::hash::Set<String>,
) -> Vec<(String, String, String, diag::Span, std::sync::Arc<str>)> {
    let modules = file_modules(fns);
    let mut found = Vec::new();
    let mut seen = crate::hash::Set::default();
    for decl in fns.iter_mut() {
        let qual = modules.get(&decl.file).cloned().flatten();
        let mut names = Vec::new();
        for stmt in &mut decl.body {
            stmt_travels(stmt, qual.as_deref(), declares, &mut names);
        }
        for (group, short, local, span) in names {
            if seen.insert(group.clone()) {
                found.push((group, short, local, span, decl.file.clone()));
            }
        }
    }
    found
}

type Named = Vec<(String, String, String, diag::Span)>;

fn stmt_travels(
    stmt: &mut ast::Stmt,
    qual: Option<&str>,
    declares: &crate::hash::Set<String>,
    out: &mut Named,
) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
            expr_travels(expr, qual, declares, out)
        }
    }
}

fn expr_travels(
    e: &mut ast::Expr,
    qual: Option<&str>,
    declares: &crate::hash::Set<String>,
    out: &mut Named,
) {
    let each = |e: &mut ast::Expr, out: &mut Named| expr_travels(e, qual, declares, out);
    match e {
        ast::Expr::Ident(name, span, _) => {
            let Some(short) = name.as_str().strip_prefix('~') else { return };
            if short.contains('~') || ast::has_slash(short) {
                return;
            }
            let short = short.to_string();
            let local = local_group(qual, &short);
            if declares.contains(&local) {
                let group = shaded_group(qual, &short);
                *name = Name::new(&group);
                out.push((group, short, local, *span));
            } else {
                out.push((name.as_str().to_string(), short, String::new(), *span));
            }
        }
        ast::Expr::Partial(..) | ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                each(k, out);
                each(v, out);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    each(inner, out);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                each(item, out);
            }
        }
        ast::Expr::App { head, args, .. } => {
            each(head, out);
            for a in args {
                each(a, out);
            }
        }
        ast::Expr::Field { base, .. } => each(base, out),
        ast::Expr::Index { base, index, .. } => {
            each(base, out);
            each(index, out);
        }
        ast::Expr::Join { lhs, rhs, .. } | ast::Expr::BinOp { lhs, rhs, .. } => {
            each(lhs, out);
            each(rhs, out);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => each(body, out),
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                stmt_travels(stmt, qual, declares, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            each(cond, out);
            each(early, out);
            for stmt in rest.iter_mut() {
                stmt_travels(stmt, qual, declares, out);
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
/// (rule 4 of the gavel, so a value's arms come from one owner). A call in a
/// module that declares the name too is offered the module's own arms, and an
/// owner's arm of the same shape as one of them is left out. Answers the
/// calls whose group stayed empty, rendered for the file that wrote each.
fn enroll_travelers(program: &mut ast::Program) -> Result<(), String> {
    let declares: crate::hash::Set<String> = program.fns.iter().map(|f| f.name.clone()).collect();
    let calls = traveling_calls(&mut program.fns, &declares);
    if calls.is_empty() {
        return Ok(());
    }
    let wanted: crate::hash::Set<&str> =
        calls.iter().map(|(_, short, ..)| short.as_str()).collect();
    let owned: crate::hash::Set<&str> =
        program.types.iter().filter(|t| !t.synthetic).map(|t| t.name.as_str()).collect();
    let origins: crate::hash::Map<&str, &str> = program
        .types
        .iter()
        .filter_map(|t| t.origin.as_deref().map(|o| (t.name.as_str(), o)))
        .collect();
    let mut owners: crate::hash::Map<&str, Vec<&ast::FnDecl>> = crate::hash::Map::default();
    for decl in &program.fns {
        if decl.synthetic || !decl.written_pub {
            continue;
        }
        let Some((qual, short)) = ast::split_qual(&decl.name) else { continue };
        if wanted.contains(short) && decl.params.iter().any(|p| owned_parameter(p, qual, &owned)) {
            owners.entry(short).or_default().push(decl);
        }
    }
    let mut arms = Vec::new();
    // A module's call that no owner's arm would join goes back to the
    // module's own group, where qualification would have sent it. Most calls
    // a module makes of its own functions on a parameter are of this kind,
    // and a clone of the group for each would be a second copy of the code.
    let mut restored: crate::hash::Map<String, String> = crate::hash::Map::default();
    for (group, short, local, ..) in &calls {
        let mine: Vec<&ast::FnDecl> = match local.is_empty() {
            true => Vec::new(),
            false => program.fns.iter().filter(|f| &f.name == local).collect(),
        };
        let taken: Vec<Vec<Option<String>>> = mine.iter().map(|d| shape(d, &origins)).collect();
        let theirs = owners.get(short.as_str()).into_iter().flatten();
        let joining: Vec<&ast::FnDecl> =
            theirs.copied().filter(|d| !taken.contains(&shape(d, &origins))).collect();
        if !mine.is_empty() && joining.is_empty() {
            restored.insert(group.clone(), local.clone());
            continue;
        }
        for decl in mine.iter().copied().chain(joining) {
            let mut arm = decl.clone();
            arm.name = group.clone();
            arm.synthetic = true;
            arms.push(arm);
        }
    }
    let filled: crate::hash::Set<String> = arms.iter().map(|a| a.name.clone()).collect();
    drop(wanted);
    drop(owned);
    drop(origins);
    drop(owners);
    program.fns.extend(arms);
    if !restored.is_empty() {
        restore(&mut program.fns, &restored);
    }
    let mut rendered = String::new();
    for (group, short, _, span, file) in &calls {
        if filled.contains(group) || restored.contains_key(group) {
            continue;
        }
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

/// Hands every name the bodies of a program spell to `f`, with the file the
/// body is in.
fn each_name(
    fns: &mut [ast::FnDecl],
    f: &mut impl FnMut(&std::sync::Arc<str>, &mut Name, diag::Span),
) {
    for decl in fns.iter_mut() {
        let file = decl.file.clone();
        for stmt in &mut decl.body {
            name_stmt(stmt, &file, f);
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

/// Puts each call of a group in `to` back on the module's own group.
fn restore(fns: &mut [ast::FnDecl], to: &crate::hash::Map<String, String>) {
    each_name(fns, &mut |_, name, _| {
        if let Some(local) = to.get(name.as_str()) {
            *name = Name::new(local);
        }
    });
}
