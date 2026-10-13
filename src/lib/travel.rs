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

/// Every traveling group a whole program's calls name, each with the first
/// call that named it: the declaration's file and the call's span.
fn traveling_calls(program: &ast::Program) -> Vec<(String, diag::Span, std::sync::Arc<str>)> {
    let mut found: Vec<(String, diag::Span, std::sync::Arc<str>)> = Vec::new();
    let mut seen = crate::hash::Set::default();
    for decl in &program.fns {
        let mut names = Vec::new();
        for stmt in &decl.body {
            stmt_travels(stmt, &mut names);
        }
        for (name, span) in names {
            if seen.insert(name.clone()) {
                found.push((name, span, decl.file.clone()));
            }
        }
    }
    found
}

fn stmt_travels(stmt: &ast::Stmt, out: &mut Vec<(String, diag::Span)>) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr_travels(expr, out),
    }
}

fn expr_travels(e: &ast::Expr, out: &mut Vec<(String, diag::Span)>) {
    match e {
        ast::Expr::Ident(name, span, _) => {
            if name.as_str().starts_with('~') && !ast::has_slash(name.as_str()) {
                out.push((name.as_str().to_string(), *span));
            }
        }
        ast::Expr::Partial(..) | ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                expr_travels(k, out);
                expr_travels(v, out);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    expr_travels(inner, out);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                expr_travels(item, out);
            }
        }
        ast::Expr::App { head, args, .. } => {
            expr_travels(head, out);
            for a in args {
                expr_travels(a, out);
            }
        }
        ast::Expr::Field { base, .. } => expr_travels(base, out),
        ast::Expr::Index { base, index, .. } => {
            expr_travels(base, out);
            expr_travels(index, out);
        }
        ast::Expr::Join { lhs, rhs, .. } | ast::Expr::BinOp { lhs, rhs, .. } => {
            expr_travels(lhs, out);
            expr_travels(rhs, out);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            expr_travels(body, out)
        }
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                stmt_travels(stmt, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            expr_travels(cond, out);
            expr_travels(early, out);
            for stmt in rest.iter() {
                stmt_travels(stmt, out);
            }
        }
    }
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
/// (rule 4 of the gavel, so a value's arms come from one owner). Answers the
/// calls whose group stayed empty, rendered for the file that wrote each.
fn enroll_travelers(program: &mut ast::Program) -> Result<(), String> {
    let calls = traveling_calls(program);
    if calls.is_empty() {
        return Ok(());
    }
    let wanted: crate::hash::Set<&str> = calls.iter().map(|(n, ..)| &n[1..]).collect();
    let owned: crate::hash::Set<&str> =
        program.types.iter().filter(|t| !t.synthetic).map(|t| t.name.as_str()).collect();
    let mut arms = Vec::new();
    for decl in &program.fns {
        if decl.synthetic || !decl.is_pub {
            continue;
        }
        let Some((qual, short)) = ast::split_qual(&decl.name) else { continue };
        if !wanted.contains(short) {
            continue;
        }
        if decl.params.iter().any(|p| owned_parameter(p, qual, &owned)) {
            let mut arm = decl.clone();
            arm.name = traveling(short);
            arm.synthetic = true;
            arms.push(arm);
        }
    }
    let filled: crate::hash::Set<String> = arms.iter().map(|a| a.name.clone()).collect();
    drop(wanted);
    drop(owned);
    program.fns.extend(arms);
    let mut rendered = String::new();
    for (name, span, file) in &calls {
        if filled.contains(name) {
            continue;
        }
        let bare = &name[1..];
        let d = diag::Diagnostic::new(
            "name",
            format!(
                "unknown name `{bare}` — no module in this program writes `{bare}` for a \
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
