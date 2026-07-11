use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use std::collections::HashSet;

pub const BUILTINS: [&str; 8] = ["at", "filter", "if", "length", "map", "print", "sort", "sum"];

pub fn check(program: &Program) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_type_order(program, &mut diags);
    check_fn_order(program, &mut diags);
    check_main(program, &mut diags);
    let globals = collect_globals(program, &mut diags);
    for decl in &program.fns {
        check_fn_body(decl, &globals, &mut diags);
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

fn collect_globals(program: &Program, diags: &mut Vec<Diagnostic>) -> HashSet<String> {
    let mut globals: HashSet<String> = BUILTINS.iter().map(|b| b.to_string()).collect();
    globals.insert("err".to_string());
    for nullary in NULLARY {
        globals.insert(nullary.to_string());
    }
    for ty in &program.types {
        if !globals.insert(ty.name.clone()) {
            diags.push(Diagnostic::new(
                "name",
                format!("the name `{}` is already taken", ty.name),
                ty.span,
            ));
        }
    }
    for decl in &program.fns {
        if BUILTINS.contains(&decl.name.as_str())
            || program.types.iter().any(|t| t.name == decl.name)
        {
            diags.push(Diagnostic::new(
                "name",
                format!("the name `{}` is already taken", decl.name),
                decl.span,
            ));
        }
        globals.insert(decl.name.clone());
    }
    globals
}

fn check_type_order(program: &Program, diags: &mut Vec<Diagnostic>) {
    if let Some(first_fn_line) = program.fns.iter().map(|f| f.span.line).min() {
        for ty in program.types.iter().filter(|t| t.span.line > first_fn_line) {
            diags.push(Diagnostic::new(
                "formatting",
                format!(
                    "canonical order places type declarations before functions; move `{}` up",
                    ty.name
                ),
                ty.span,
            ));
        }
    }
    for pair in program.types.windows(2) {
        if pair[0].name >= pair[1].name {
            diags.push(Diagnostic::new(
                "formatting",
                format!(
                    "type declarations appear in alphabetical order: `{}` before `{}`",
                    pair[1].name, pair[0].name
                ),
                pair[1].span,
            ));
        }
    }
    for ty in &program.types {
        for pair in ty.fields.windows(2) {
            let (prev_name, _, _) = &pair[0];
            let (next_name, _, next_span) = &pair[1];
            if prev_name >= next_name {
                diags.push(Diagnostic::new(
                    "formatting",
                    format!(
                        "fields appear in alphabetical order: `{next_name}` before `{prev_name}`"
                    ),
                    *next_span,
                ));
            }
        }
    }
}

fn check_fn_order(program: &Program, diags: &mut Vec<Diagnostic>) {
    let mut group_names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        match group_names.last() {
            Some(last) if *last == decl.name => {}
            _ => group_names.push(&decl.name),
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for name in &group_names {
        if !seen.insert(name) {
            let decl = program
                .fns
                .iter()
                .rev()
                .find(|d| d.name == *name)
                .expect("group name comes from decls");
            diags.push(Diagnostic::new(
                "formatting",
                format!("overloads of `{name}` must be adjacent"),
                decl.span,
            ));
        }
    }
    for pair in group_names.windows(2) {
        if pair[0] >= pair[1] {
            let decl = program
                .fns
                .iter()
                .find(|d| d.name == pair[1])
                .expect("group name comes from decls");
            diags.push(Diagnostic::new(
                "formatting",
                format!(
                    "function declarations appear in alphabetical order: `{}` before `{}`",
                    pair[1], pair[0]
                ),
                decl.span,
            ));
        }
    }
    check_overload_ranks(program, diags);
}

fn check_overload_ranks(program: &Program, diags: &mut Vec<Diagnostic>) {
    for pair in program.fns.windows(2) {
        if pair[0].name != pair[1].name || pair[0].params.len() != pair[1].params.len() {
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

fn same_shape(a: &[Pattern], b: &[Pattern]) -> bool {
    a.iter().zip(b.iter()).all(|(pa, pb)| match (pa, pb) {
        (Pattern::IntLit(x, _), Pattern::IntLit(y, _)) => x == y,
        (Pattern::StrLit(x, _), Pattern::StrLit(y, _)) => x == y,
        (Pattern::Nullary(x, _), Pattern::Nullary(y, _)) => x == y,
        (Pattern::Annotated { ty: x, .. }, Pattern::Annotated { ty: y, .. }) => x == y,
        (Pattern::Ctor { ty: x, .. }, Pattern::Ctor { ty: y, .. }) => x == y,
        (Pattern::Var(..) | Pattern::Wildcard, Pattern::Var(..) | Pattern::Wildcard) => true,
        _ => false,
    })
}

fn check_main(program: &Program, diags: &mut Vec<Diagnostic>) {
    match program.fns.iter().find(|d| d.name == "main") {
        Some(main) if !main.params.is_empty() => diags.push(Diagnostic::new(
            "signature",
            "`main` takes no parameters".to_string(),
            main.span,
        )),
        Some(_) => {}
        None => diags.push(Diagnostic::new(
            "name",
            "a program defines `main`".to_string(),
            Span { line: 1, col: 1 },
        )),
    }
}

struct Local {
    name: String,
    span: Span,
    used: bool,
}

struct Resolver<'a> {
    globals: &'a HashSet<String>,
    locals: Vec<Local>,
    diags: Vec<Diagnostic>,
}

fn check_fn_body(decl: &FnDecl, globals: &HashSet<String>, diags: &mut Vec<Diagnostic>) {
    let mut resolver = Resolver { globals, locals: Vec::new(), diags: Vec::new() };
    for param in &decl.params {
        resolver.bind_pattern(param);
    }
    let last = decl.body.len() - 1;
    for (i, stmt) in decl.body.iter().enumerate() {
        match stmt {
            Stmt::Bind { name, span, expr } => {
                resolver.resolve_expr(expr);
                if i == last {
                    resolver.diags.push(Diagnostic::new(
                        "unused",
                        "a body ends with its result expression, not a binding".to_string(),
                        *span,
                    ));
                }
                resolver.rebind(name, *span);
            }
            Stmt::Expr(expr) => {
                resolver.resolve_expr(expr);
                if i != last {
                    resolver.diags.push(Diagnostic::new(
                        "unused",
                        "unused expression: every non-final line binds a name (sequence \
                         effects with `>>`)"
                            .to_string(),
                        expr.span(),
                    ));
                }
            }
        }
    }
    resolver.flush_unused(0);
    diags.append(&mut resolver.diags);
}

impl Resolver<'_> {
    fn bind_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Var(name, span) => self.push_local(name, *span),
            Pattern::Annotated { name, span, .. } => self.push_local(name, *span),
            Pattern::Ctor { fields, .. } => {
                for field in fields {
                    self.bind_pattern(field);
                }
            }
            _ => {}
        }
    }

    fn push_local(&mut self, name: &str, span: Span) {
        self.locals.push(Local { name: name.to_string(), span, used: false });
    }

    fn rebind(&mut self, name: &str, span: Span) {
        if let Some(local) = self.locals.iter().rev().find(|l| l.name == name) {
            if !local.used {
                self.diags.push(Diagnostic::new(
                    "unused",
                    format!("unused binding: each version of `{name}` is used before the next"),
                    local.span,
                ));
            }
        }
        self.push_local(name, span);
    }

    fn flush_unused(&mut self, from: usize) {
        let mut shadowed: HashSet<String> = HashSet::new();
        for local in self.locals[from..].iter().rev() {
            if !local.used && !shadowed.contains(&local.name) {
                self.diags.push(Diagnostic::new(
                    "unused",
                    format!("unused binding `{}`", local.name),
                    local.span,
                ));
            }
            shadowed.insert(local.name.clone());
        }
        self.locals.truncate(from);
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(..) => {}
            Expr::Str(parts, _) => {
                for part in parts {
                    if let TemplatePart::Interp(inner) = part {
                        self.resolve_expr(inner);
                    }
                }
            }
            Expr::Ident(name, span) => self.resolve_name(name, *span),
            Expr::List(items, _) => {
                for item in items {
                    self.resolve_expr(item);
                }
            }
            Expr::App { head, args, .. } => {
                self.resolve_expr(head);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::Seq(lhs, rhs, _) => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::Lambda { params, body, .. } => {
                let base = self.locals.len();
                for (name, span) in params {
                    self.push_local(name, *span);
                }
                self.resolve_expr(body);
                self.flush_unused(base);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
        }
    }

    fn resolve_name(&mut self, name: &str, span: Span) {
        if let Some(local) = self.locals.iter_mut().rev().find(|l| l.name == name) {
            local.used = true;
            return;
        }
        if !self.globals.contains(name) {
            self.diags.push(Diagnostic::new("name", format!("unknown name `{name}`"), span));
        }
    }
}
