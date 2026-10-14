// hoist_repeated_strings: an interpolation a body spells more than once is
// computed once into a binding. lib.rs reads this file with include!, so the
// items stay in the crate root and the compiler builds to the same
// instructions it did when they sat in lib.rs (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under
// src/'s subdirectories itself.

/// Evaluate a repeated pure interpolation once.
///
/// Reading a key and then writing it is the ordinary shape of map code, and
/// spelled directly it builds the key twice: `put m "k{i}" (bump m["k{i}"])`
/// allocates two identical strings an iteration. Binding it costs one.
///
/// The licence is deliberately small, because hoisting is only safe when
/// evaluating once instead of twice cannot be observed. An interpolation
/// qualifies when every piece of it is a name, a number or arithmetic over
/// those — no calls, no indexing, nothing that can fail or reach an effect.
/// Lambda bodies are left alone entirely: a folder's shape is what the
/// linearity analysis matches on, and a binding in front of it would hide the
/// write that makes a fold write in place.
pub fn hoist_repeated_strings(program: &mut ast::Program) {
    let mut counter = 0usize;
    for decl in &mut program.fns {
        if decl.synthetic {
            continue;
        }
        hoist_in_body(&mut decl.body, &mut counter);
    }
}

/// Arithmetic over names and numbers, and nothing else.
fn purely_computed(e: &ast::Expr) -> bool {
    use ast::Expr;
    match e {
        Expr::Ident(..) | Expr::Int(..) | Expr::Float(..) => true,
        Expr::BinOp { op, lhs, rhs, .. } => {
            matches!(*op, "+" | "-" | "*" | "/" | "%")
                && purely_computed(lhs)
                && purely_computed(rhs)
        }
        Expr::Str(parts, _) => parts.iter().all(|p| match p {
            ast::TemplatePart::Lit(_) => true,
            ast::TemplatePart::Interp(inner) => purely_computed(inner),
        }),
        _ => false,
    }
}

/// A shape key that ignores where the expression was written.
fn shape_of(e: &ast::Expr) -> String {
    let text = format!("{e:?}");
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find("Span {") {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find('}') {
            Some(j) => &rest[i + j + 1..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn collect_hoistable(e: &ast::Expr, found: &mut Vec<(String, ast::Expr)>) {
    use ast::Expr;
    if let Expr::Lambda { .. } = e {
        return;
    }
    if let Expr::Str(parts, _) = e {
        let interpolated = parts.iter().any(|p| matches!(p, ast::TemplatePart::Interp(_)));
        if interpolated && purely_computed(e) {
            found.push((shape_of(e), e.clone()));
            return;
        }
    }
    for_each_child(e, |child| collect_hoistable(child, found));
}

fn replace_shape(e: &mut ast::Expr, shape: &str, name: &str) {
    use ast::Expr;
    if let Expr::Lambda { .. } = e {
        return;
    }
    if shape_of(e) == shape {
        let span = e.span();
        *e = Expr::Ident(Name::new(name), span, crate::ast::Resolution::default());
        return;
    }
    for_each_child_mut(e, &mut |c| replace_shape(c, shape, name));
}

fn stmt_expr_mut(s: &mut ast::Stmt) -> &mut ast::Expr {
    match s {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr,
    }
}

fn hoist_in_body(body: &mut Vec<ast::Stmt>, counter: &mut usize) {
    let mut i = 0;
    while i < body.len() {
        let mut found = Vec::new();
        match &body[i] {
            ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                collect_hoistable(expr, &mut found)
            }
        }
        let mut seen: crate::hash::Map<String, usize> = crate::hash::Map::default();
        for (shape, _) in &found {
            *seen.entry(shape.clone()).or_default() += 1;
        }
        let mut repeated: Vec<(String, ast::Expr)> = Vec::new();
        let mut taken: crate::hash::Set<String> = crate::hash::Set::default();
        for (shape, expr) in found {
            if seen.get(&shape).copied().unwrap_or(0) > 1 && taken.insert(shape.clone()) {
                repeated.push((shape, expr));
            }
        }
        if repeated.is_empty() {
            i += 1;
            continue;
        }
        let mut inserted = 0;
        for (shape, expr) in repeated {
            let name = format!("once{counter}");
            *counter += 1;
            let span = expr.span();
            match &mut body[i + inserted] {
                ast::Stmt::Bind { expr: e, .. } | ast::Stmt::Expr(e) => {
                    replace_shape(e, &shape, &name)
                }
            }
            body.insert(
                i + inserted,
                ast::Stmt::Bind { pattern: ast::Pattern::Var(Name::new(&name), span), expr },
            );
            inserted += 1;
        }
        i += inserted + 1;
    }
}
