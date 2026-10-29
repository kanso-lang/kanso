// The late rewrites: a field read becomes its getter applied, a merge its
// constructor call, and `!=` the negation of `==`. lib.rs reads this file with
// include!, so the items stay in the crate root and the compiler builds to the
// same instructions it did when they sat in lib.rs (kanso#1867). cargo fmt
// does not reach an included file, so CI's format job checks the files under
// src/'s subdirectories itself.

/// `x.name` is the getter applied. The rewrite runs late, after the checks
/// that read field syntax to say something about the field — a type conflict
/// names the read site, and an application would have nothing to point at.
pub fn desugar_field_reads(program: &mut ast::Program) {
    // Inequality rides the same hook: it has to see every module the merge
    // produced, which is exactly what this pass already runs after.
    desugar_inequality(program);
    desugar_merges(program);
    synthesize_reader_groups(program);
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            desugar_stmt(stmt);
        }
    }
}

/// The builtin that copies a record with named fields replaced (#1886).
pub const MERGE: &str = "merge";

/// The field a `merge` key names, when it is a plain string literal.
pub fn merge_key(key: &ast::Expr) -> Option<&str> {
    match key {
        ast::Expr::Str(parts, _) => match parts.as_slice() {
            [ast::TemplatePart::Lit(text)] => Some(text.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// A record this compilation unit declares, as the checker sees it: an
/// imported record is a qualified name or the short clone the loader makes.
pub fn merge_owned(ty: &ast::TypeDecl) -> bool {
    !ty.synthetic && !ast::has_slash(&ty.name) && !ty.fields.is_empty()
}

/// `merge u { "b":2 }` becomes a call to a group with one arm per record the
/// calling module declares that has every field named:
///
/// ```text
/// fn Merge_1b (t Kept0 _ Kept2) Given0 = t Kept0 Given0 Kept2
/// ```
///
/// so every engine runs a dispatch and a construction it already has. One
/// group per module and set of names, shared by every call that names them.
/// A value of a type with no arm meets the dispatch's own refusal.
fn desugar_merges(program: &mut ast::Program) {
    let records: Vec<(String, Vec<String>, crate::diag::Span)> = program
        .types
        .iter()
        .filter(|t| !t.synthetic && !t.fields.is_empty())
        .map(|t| (t.name.clone(), t.fields.iter().map(|(f, _, _)| f.clone()).collect(), t.span))
        .collect();
    let mut made: crate::hash::Set<String> = crate::hash::Set::default();
    let mut arms = Vec::new();
    for decl in &mut program.fns {
        let owner = ast::split_qual(&decl.name).map(|(o, _)| o.to_string()).unwrap_or_default();
        let mut at = |e: &mut ast::Expr| merge_expr(e, &owner, &records, &mut made, &mut arms);
        for stmt in &mut decl.body {
            match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => at(expr),
            }
        }
    }
    program.fns.extend(arms);
}

fn merge_expr(
    e: &mut ast::Expr,
    owner: &str,
    records: &[(String, Vec<String>, crate::diag::Span)],
    made: &mut crate::hash::Set<String>,
    arms: &mut Vec<ast::FnDecl>,
) {
    for_each_child_mut(e, &mut |child| merge_expr(child, owner, records, made, arms));
    let ast::Expr::App { head, args, span, .. } = e else { return };
    let ast::Expr::Ident(name, _, _) = head.as_ref() else { return };
    if name.as_str() != MERGE || args.len() != 2 {
        return;
    }
    let ast::Expr::MapLit(entries, _) = &args[1] else { return };
    let Some(keys) =
        entries.iter().map(|(k, _)| merge_key(k).map(str::to_string)).collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let span = *span;
    // Each name carries its length, so two sets of names never spell the same
    // group, and the result is a word every engine takes as a symbol.
    let bare: String =
        keys.iter().fold("Merge".to_string(), |acc, k| format!("{acc}_{}{k}", k.len()));
    let group = match owner.is_empty() {
        true => bare,
        false => format!("{owner}/{bare}"),
    };
    if made.insert(group.clone()) {
        for (ty, fields, declared) in records {
            let ty_owner = ast::split_qual(ty).map(|(o, _)| o).unwrap_or("");
            if ty_owner != owner || !keys.iter().all(|k| fields.contains(k)) {
                continue;
            }
            let given = |k: &str| keys.iter().position(|g| g == k);
            let pattern = fields
                .iter()
                .enumerate()
                .map(|(at, f)| match given(f) {
                    Some(_) => ast::Pattern::Wildcard(span),
                    None => ast::Pattern::Var(Name::new(&format!("Kept{at}")), span),
                })
                .collect();
            let mut params =
                vec![ast::Pattern::Ctor { ty: Name::new(ty), fields: pattern, whole: None }];
            params.extend(
                (0..keys.len()).map(|at| ast::Pattern::Var(Name::new(&format!("Given{at}")), span)),
            );
            let built = fields
                .iter()
                .enumerate()
                .map(|(at, f)| {
                    let local = match given(f) {
                        Some(g) => format!("Given{g}"),
                        None => format!("Kept{at}"),
                    };
                    ast::Expr::Ident(Name::new(&local), span, crate::ast::Resolution::default())
                })
                .collect();
            let body = ast::Expr::App {
                head: Box::new(ast::Expr::Ident(
                    Name::new(ty),
                    span,
                    crate::ast::Resolution::default(),
                )),
                args: built,
                span,
                piped: false,
            };
            // Each arm carries its type's span: arms of one group that share a
            // span read as one declaration made twice when a module is merged.
            arms.push(ast::FnDecl {
                name: group.clone(),
                is_pub: true,
                written_pub: false,
                span: *declared,
                params,
                body: vec![ast::Stmt::Expr(body)],
                file: crate::ast::unstamped(),
                synthetic: false,
                shades: None,
            });
        }
    }
    let ast::Expr::App { args, .. } = e else { return };
    let map = args.pop().expect("two arguments");
    let ast::Expr::MapLit(entries, _) = map else { return };
    args.extend(entries.into_iter().map(|(_, v)| v));
    *head_of(e) = ast::Expr::Ident(Name::new(&group), span, crate::ast::Resolution::default());
}

fn head_of(e: &mut ast::Expr) -> &mut ast::Expr {
    match e {
        ast::Expr::App { head, .. } => head.as_mut(),
        _ => unreachable!("a merge is an application"),
    }
}

/// An err answers `.reason`, `.cause` and `.origin` whatever the program
/// declares, and a read desugars to the getter's name, so the group has to
/// exist when no record declares the field. This arm makes it exist. The
/// reader hole at every dispatcher's entry answers an err before any arm is
/// tried, so the arm is never reached by one; a record reaching it matches
/// nothing and gets the field error every other getter gives.
///
/// Once, over the merged program: synthesised per module, every module
/// carried an identical arm and the merge refused them as overlapping.
fn synthesize_reader_groups(program: &mut ast::Program) {
    for field in ast::ERR_READERS {
        let name = ast::getter_name(field);
        if program.fns.iter().any(|f| f.name == name) {
            continue;
        }
        let span = crate::diag::Span::at(0, 0);
        program.fns.push(ast::FnDecl {
            name,
            is_pub: true,
            written_pub: false,
            span,
            params: vec![ast::Pattern::Ctor {
                ty: Name::new("err"),
                fields: vec![ast::Pattern::Var(Name::new(ast::GETTER_BINDER), span)],
                whole: None,
            }],
            body: vec![ast::Stmt::Expr(ast::Expr::Ident(
                Name::new(ast::GETTER_BINDER),
                span,
                crate::ast::Resolution::default(),
            ))],
            file: crate::ast::unstamped(),
            synthetic: false,
            shades: None,
        });
    }
}

fn desugar_stmt(stmt: &mut ast::Stmt) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => desugar_expr(expr),
    }
}

/// A type says equality once. Where a module arms `==`, `a != b` becomes the
/// denial of that arm rather than a second builtin, so no type can call two
/// values equal and unequal at the same time. Gated on the arm existing,
/// because a program with no arm should keep paying nothing for the question.
pub fn desugar_inequality(program: &mut ast::Program) {
    let armed = program.fns.iter().any(|d| d.name == "==" && d.params.len() == 2);
    if !armed {
        return;
    }
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            deny_stmt(stmt);
        }
    }
}

fn deny_stmt(stmt: &mut ast::Stmt) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => deny_expr(expr),
    }
}

fn deny_expr(e: &mut ast::Expr) {
    for_each_child_mut(e, &mut deny_expr);
    let ast::Expr::BinOp { op: "!=", lhs, rhs, span } = e else {
        return;
    };
    let (span, zero) = (*span, ast::Expr::Int(0.into(), *span));
    let lhs = Box::new(std::mem::replace(lhs.as_mut(), zero.clone()));
    let rhs = Box::new(std::mem::replace(rhs.as_mut(), zero));
    let same = ast::Expr::BinOp { op: "==", lhs, rhs, span };
    *e = ast::Expr::App {
        head: Box::new(ast::Expr::Ident(Name::new("if"), span, crate::ast::Resolution::default())),
        args: vec![
            same,
            ast::Expr::Ident(Name::new("false"), span, crate::ast::Resolution::default()),
            ast::Expr::Ident(Name::new("true"), span, crate::ast::Resolution::default()),
        ],
        span,
        piped: false,
    };
}

fn desugar_expr(e: &mut ast::Expr) {
    if let ast::Expr::Field { base, name, span } = e {
        desugar_expr(base);
        let head = ast::Expr::Ident(
            Name::new(&ast::getter_name(name)),
            *span,
            crate::ast::Resolution::default(),
        );
        let base = std::mem::replace(base.as_mut(), ast::Expr::Int(0.into(), *span));
        *e = ast::Expr::App { head: Box::new(head), args: vec![base], span: *span, piped: false };
        return;
    }
    for_each_child_mut(e, &mut desugar_expr);
}
