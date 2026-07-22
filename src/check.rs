use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use std::collections::HashSet;

pub const BUILTINS: [&str; 26] = [
    "args",
    "bytes",
    "char_code",
    "chars",
    "concat",
    "entries",
    "find2",
    "from_code",
    "if",
    "join",
    "length",
    "print",
    "push",
    "put",
    "random",
    "read_file",
    "render_value",
    "round",
    "sleep",
    "slice",
    "sqrt",
    "stdin",
    "to_float",
    "to_int",
    "utf8",
    "write_file",
];

/// The bare-name subset: what resolves without an import. Everything else
/// in BUILTINS is internal, reached only through std wrapper modules.
pub const AMBIENT: [&str; 6] = [
    "entries",
    "if",
    "length",
    "print",
    "push",
    "put",
];

pub fn check(program: &mut Program, require_main: bool) -> Vec<Diagnostic> {
    let markers = marker_names(program);
    let type_names = program.types.iter().map(|t| t.name.clone()).collect();
    let mut used = HashSet::new();
    let mut diags = resolve_markers(program, &markers);
    diags.extend(check_typesets(program, &type_names));
    diags.extend(check_file(program, &HashSet::new(), &mut used));
    if require_main {
        check_main(program, &mut diags);
    }
    check_unused_private(program, &used, &mut diags);
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

/// Zero-field type names: a bare mention is the marker value, and in
/// parameter position the bare name matches that marker.
pub fn marker_names(program: &Program) -> HashSet<String> {
    program
        .types
        .iter()
        .filter(|t| t.fields.is_empty())
        .map(|t| t.name.clone())
        .collect()
}

/// Rewrites bare marker names in parameter position into zero-field
/// constructor patterns, and rejects constructor calls that pass a marker
/// fields — a marker's bare mention is its value.
pub fn resolve_markers(program: &mut Program, markers: &HashSet<String>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for decl in &mut program.fns {
        for param in &mut decl.params {
            resolve_marker_pattern(param, markers, &mut diags);
        }
        for stmt in &mut decl.body {
            match stmt {
                Stmt::Bind { expr, .. } => check_marker_calls(expr, markers, &mut diags),
                Stmt::Expr(expr) => check_marker_calls(expr, markers, &mut diags),
            }
        }
    }
    diags
}

fn resolve_marker_pattern(
    pattern: &mut Pattern,
    markers: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    match pattern {
        Pattern::Var(name, _) if markers.contains(name.as_str()) => {
            *pattern = Pattern::Ctor { ty: name.clone(), fields: Vec::new() };
        }
        Pattern::Ctor { ty, fields } => {
            if markers.contains(ty.as_str()) && !fields.is_empty() {
                diags.push(Diagnostic::new(
                    "signature",
                    format!("`{ty}` takes no fields; its bare mention is its value"),
                    other_span(&fields[0]),
                ));
            }
            for field in fields {
                resolve_marker_pattern(field, markers, diags);
            }
        }
        _ => {}
    }
}

fn check_marker_calls(expr: &Expr, markers: &HashSet<String>, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) => {}
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                check_marker_calls(key, markers, diags);
                check_marker_calls(value, markers, diags);
            }
        }
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    check_marker_calls(inner, markers, diags);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                check_marker_calls(item, markers, diags);
            }
        }
        Expr::App { head, args, .. } => {
            if let Expr::Ident(name, span) = &**head {
                if markers.contains(name.as_str()) && !args.is_empty() {
                    diags.push(Diagnostic::new(
                        "signature",
                        format!("`{name}` takes no fields; its bare mention is its value"),
                        *span,
                    ));
                }
            }
            check_marker_calls(head, markers, diags);
            for arg in args {
                check_marker_calls(arg, markers, diags);
            }
        }
        Expr::Field { base, .. } => check_marker_calls(base, markers, diags),
        Expr::Index { base, index, .. } => {
            check_marker_calls(base, markers, diags);
            check_marker_calls(index, markers, diags);
        }
        Expr::Seq(lhs, rhs, _) => {
            check_marker_calls(lhs, markers, diags);
            check_marker_calls(rhs, markers, diags);
        }
        Expr::Lambda { body, .. } => check_marker_calls(body, markers, diags),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            check_marker_calls(lhs, markers, diags);
            check_marker_calls(rhs, markers, diags);
        }
    }
}

/// Builtin type words legal as typeset members alongside declared types.
const TYPESET_BUILTINS: [&str; 4] = ["bool", "float64", "int", "string"];

/// A multi-member field typeset enumerates concrete types: each member must
/// name a declared type or a builtin type word.
pub fn check_typesets(program: &Program, type_names: &HashSet<String>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for ty in &program.types {
        for (field, tys, span) in &ty.fields {
            if tys.len() < 2 {
                continue;
            }
            for member in tys {
                let known = TYPESET_BUILTINS.contains(&member.as_str())
                    || type_names.contains(member.as_str());
                if !known {
                    diags.push(Diagnostic::new(
                        "name",
                        format!(
                            "`{member}` in the typeset of field `{field}` names no \
                             declared or builtin type"
                        ),
                        *span,
                    ));
                }
            }
        }
    }
    diags
}

/// Names a file declares, for the module-wide first pass.
pub fn declared_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    for ty in &program.types {
        names.insert(ty.name.clone());
    }
    for decl in &program.fns {
        names.insert(decl.name.clone());
    }
    names
}

/// Per-file checks: canonical order plus name resolution against this file's
/// globals extended with the rest of the module. Records which module-level
/// names the file uses, for the unused-private check.
pub fn check_file(
    program: &Program,
    extern_globals: &HashSet<String>,
    used_globals: &mut HashSet<String>,
) -> Vec<Diagnostic> {
    check_file_shadow(program, extern_globals, used_globals, &HashSet::new())
}

/// Bare-enrolled imports (synthetic clones) are shadowable: a local binding
/// named like one is the local's to keep — the enrollment must never make
/// every stdlib export a forbidden binding name.
pub fn check_file_shadow(
    program: &Program,
    extern_globals: &HashSet<String>,
    used_globals: &mut HashSet<String>,
    shadowable: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_type_order(program, &mut diags);
    check_fn_order(program, &mut diags);
    check_constants(program, &mut diags);
    let mut globals = collect_globals(program, &mut diags);
    globals.extend(extern_globals.iter().cloned());
    for decl in &program.fns {
        check_fn_body_shadow(decl, &globals, used_globals, &mut diags, shadowable);
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

/// A `_`-prefixed declaration is module-private and must be used somewhere
/// in the module; public names are API surface and exempt.
pub fn check_unused_private(
    program: &Program,
    used_globals: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    let mut reported: HashSet<&str> = HashSet::new();
    for decl in &program.fns {
        if decl.name.starts_with('_')
            && !used_globals.contains(&decl.name)
            && reported.insert(&decl.name)
        {
            diags.push(Diagnostic::new(
                "unused",
                format!("private `{}` is never used in its module", decl.name),
                decl.span,
            ));
        }
    }
    for ty in &program.types {
        if ty.name.starts_with('_') && !used_globals.contains(&ty.name) {
            diags.push(Diagnostic::new(
                "unused",
                format!("private type `{}` is never used in its module", ty.name),
                ty.span,
            ));
        }
    }
}

/// Merged-namespace coherence for a directory module.
pub fn check_merged(program: &Program, require_main: bool) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_constants(program, &mut diags);
    if require_main {
        check_main(program, &mut diags);
    }
    diags
}

fn collect_globals(program: &Program, diags: &mut Vec<Diagnostic>) -> HashSet<String> {
    let mut globals: HashSet<String> = AMBIENT.iter().map(|b| b.to_string()).collect();
    globals.insert("entry".to_string());
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
        if AMBIENT.contains(&decl.name.as_str())
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
        for (_, tys, span) in &ty.fields {
            for pair in tys.windows(2) {
                if pair[0] >= pair[1] {
                    diags.push(Diagnostic::new(
                        "formatting",
                        "typeset members appear in alphabetical order, without duplicates"
                            .to_string(),
                        *span,
                    ));
                }
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

fn check_constants(program: &Program, diags: &mut Vec<Diagnostic>) {
    let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
    for decl in &program.fns {
        match groups.last_mut() {
            Some((name, decls)) if *name == decl.name => decls.push(decl),
            _ => groups.push((&decl.name, vec![decl])),
        }
    }
    for (name, decls) in groups {
        let has_constant = decls.iter().any(|d| d.params.is_empty());
        if has_constant && decls.len() > 1 {
            diags.push(Diagnostic::new(
                "dispatch",
                format!("`{name}` is a constant (arity 0); a constant admits no overloads"),
                decls[1].span,
            ));
        }
    }
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
        (Pattern::Var(..) | Pattern::Wildcard(..), Pattern::Var(..) | Pattern::Wildcard(..)) => true,
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

fn other_span(pattern: &Pattern) -> Span {
    match pattern {
        Pattern::IntLit(_, s) | Pattern::StrLit(_, s) | Pattern::Nullary(_, s) => *s,
        Pattern::Annotated { span, .. } | Pattern::Keyed { span, .. } => *span,
        Pattern::Var(_, s) => *s,
        Pattern::Wildcard(s) => *s,
        Pattern::Ctor { .. } => Span { line: 0, col: 0 },
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
    used_globals: &'a mut HashSet<String>,
    diags: Vec<Diagnostic>,
    shadowable: &'a HashSet<String>,
    /// std-origin files (stamped `std/...` by the loader) may name internal
    /// builtins through the builtin_ prefix; nothing else may.
    std_origin: bool,
}

fn check_fn_body_shadow(
    decl: &FnDecl,
    globals: &HashSet<String>,
    used_globals: &mut HashSet<String>,
    diags: &mut Vec<Diagnostic>,
    shadowable: &HashSet<String>,
) {
    let mut resolver = Resolver {
        globals,
        locals: Vec::new(),
        used_globals,
        diags: Vec::new(),
        std_origin: decl.file.starts_with("std/"),
        shadowable,
    };
    for param in &decl.params {
        resolver.bind_pattern(param);
    }
    let last = decl.body.len() - 1;
    for (i, stmt) in decl.body.iter().enumerate() {
        match stmt {
            Stmt::Bind { pattern, expr } => {
                resolver.resolve_expr(expr);
                if i == last {
                    resolver.diags.push(Diagnostic::new(
                        "unused",
                        "a body ends with its result expression, not a binding".to_string(),
                        expr.span(),
                    ));
                }
                resolver.bind_target(pattern);
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

    fn bind_target(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Var(name, span) => self.rebind(name, *span),
            Pattern::Ctor { fields, .. } => {
                for field in fields {
                    self.bind_target_field(field);
                }
            }
            Pattern::Keyed { entries, span } => {
                for pair in entries.windows(2) {
                    if pair[0].field >= pair[1].field {
                        self.diags.push(Diagnostic::new(
                            "formatting",
                            format!(
                                "keyed reads list fields in alphabetical order: `{}` before \
                                 `{}`",
                                pair[1].field, pair[0].field
                            ),
                            pair[1].span,
                        ));
                    }
                }
                let _ = span;
                for entry in entries {
                    self.rebind(&entry.bind_name, entry.span);
                }
            }
            _ => {}
        }
    }

    fn bind_target_field(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Var(name, span) => self.rebind(name, *span),
            Pattern::Ctor { fields, .. } => {
                for field in fields {
                    self.bind_target_field(field);
                }
            }
            Pattern::Wildcard(span) => self.diags.push(Diagnostic::new(
                "syntax",
                "`_` does not appear in binding patterns; omit fields with a keyed read"
                    .to_string(),
                *span,
            )),
            other => self.diags.push(Diagnostic::new(
                "syntax",
                "binding patterns are irrefutable: names and nested constructor patterns \
                 only"
                    .to_string(),
                other_span(other),
            )),
        }
    }

    fn push_local(&mut self, name: &str, span: Span) {
        if self.globals.contains(name) && !self.shadowable.contains(name) {
            self.diags.push(Diagnostic::new(
                "name",
                format!("`{name}` is already a declaration; rename the binding"),
                span,
            ));
        }
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
            Expr::Int(..) | Expr::Float(..) => {}
            Expr::Field { base, .. } => self.resolve_expr(base),
            Expr::MapLit(pairs, _) => {
                for (key, value) in pairs {
                    self.resolve_expr(key);
                    self.resolve_expr(value);
                }
            }
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
            Expr::Index { base, index, .. } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::Seq(lhs, rhs, _) => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::Lambda { params, body, .. } => {
                let base = self.locals.len();
                for (name, span) in params {
                    if name != "_" {
                        self.push_local(name, *span);
                    }
                }
                self.resolve_expr(body);
                self.flush_unused(base);
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
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
        if let Some(stripped) = name.strip_prefix("builtin_") {
            match self.std_origin && BUILTINS.contains(&stripped) {
                true => return,
                false => {
                    self.diags.push(Diagnostic::new(
                        "name",
                        format!("`{name}` is internal to the standard library — import its module"),
                        span,
                    ));
                    return;
                }
            }
        }
        match self.globals.contains(name) {
            true => {
                self.used_globals.insert(name.to_string());
            }
            false => {
                self.diags.push(Diagnostic::new("name", format!("unknown name `{name}`"), span));
            }
        }
    }
}
