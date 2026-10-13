// The shadowing check: the scopes a function body declares and the resolver
// that walks them, refusing a name bound twice in one body. check.rs reads
// this file with include!, so the items stay in the check module and the
// compiler builds to the same instructions it did when they sat in check.rs
// (kanso#1867). cargo fmt does not reach an included file, so CI's format job
// checks the files under src/'s subdirectories itself.

struct Local<'a> {
    /// Borrowed from the pattern that bound it. The declaration this walk
    /// reads outlives the walk, so owning the name meant a `String` per
    /// binding for a vector that is truncated at the end of every scope.
    name: &'a str,
    span: Span,
    used: bool,
    /// Bound to something the function was handed: a parameter, a field of
    /// one, an element of a list passed in, or what a traveling arm answered
    /// for one. A bare name this module cannot see may still be called on
    /// such a value, because the value carries its owner's arms (2026-10-12).
    arrived: bool,
}

/// What the program declares, gathered once and asked about everywhere. The
/// three used to travel as three parameters; a fourth would have been a
/// fourth, and they are one question about one program.
struct Declared<'a> {
    /// Arities of this module's own fn groups; an application of a local
    /// group must match one (arity 0 opts out: a constant's value may be
    /// callable, which only the runtime can arbitrate).
    fn_arities: &'a Arities<'a>,
    /// The functions the module's other files declare. With this file's own
    /// they are the arms a traveling call of the same name is offered beside.
    sibling_fns: &'a HashSet<&'a str>,
    /// How many fields each record type declares. Construction is positional,
    /// so an application of a type name has to hand over all of them.
    type_arity: &'a crate::hash::Map<&'a str, (usize, Option<&'a str>)>,
    /// Every declared type name, records and subtypes and typesets alike.
    /// An upcast's target is a name this walk already reaches, so asking
    /// whether a type answers to it costs the lookup and no traversal.
    types: TypeNames<'a>,
}

/// Whether `file` is one of the shipped library's own files, read from disk.
/// Imported, a std module's files are stamped `std/...`; tested from the
/// checkout, as `kanso test lib/list` does, the same files are stamped
/// `lib/...`, and they may reach the builtins they wrap like any other copy of
/// themselves. Asked only on the arm that would refuse a builtin, which a
/// correct program never reaches, so its filesystem calls cost nothing there.
fn in_shipped_library(file: &str) -> bool {
    // The file is resolved, not only its directory, so a module built of links
    // to the library's files (the book's ch08 sample is one) reads as the
    // library, which is the text it is.
    let Ok(file) = std::fs::canonicalize(file) else { return false };
    let Some(module) = file.parent() else { return false };
    // A unit test runs from target/<profile>/deps, one directory deeper than
    // the compiler, so the second root misses the checkout there; the third
    // names it, and exists only in that build.
    let roots = [
        std::env::var("KANSO_STD").ok().map(std::path::PathBuf::from),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../../lib"))),
        #[cfg(test)]
        Some(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/lib"))),
    ];
    roots
        .into_iter()
        .flatten()
        .filter_map(|r| std::fs::canonicalize(r).ok())
        .any(|root| module.parent() == Some(root.as_path()))
}

struct Resolver<'a> {
    globals: &'a HashSet<&'a str>,
    locals: Vec<Local<'a>>,
    diags: Vec<Diagnostic>,
    shadowable: &'a HashSet<String>,
    declared: &'a Declared<'a>,
    /// std-origin files (stamped `std/...` by the loader) may name internal
    /// builtins through the builtin_ prefix; nothing else may.
    std_origin: bool,
    /// The file, for the one case `std_origin` misses: a std module tested
    /// from the checkout, which the loader stamps `lib/...`.
    file: &'a str,
    /// Whether the names being bound now arrived with the function: set while
    /// its parameters bind, and while a binding takes an arrived value.
    arriving: bool,
    /// Set just before a lambda handed to a call that also carries an arrived
    /// value, so the lambda's parameters arrive too: `list/map xs (x -> area
    /// x)` reaches each element of a list passed in.
    lambda_arrives: bool,
    /// The calls of a name this module cannot see whose arguments carry an
    /// arrived value. Each is renamed into the traveling group of that name,
    /// which the whole program fills with its owners' arms. Beside them, and
    /// marked, the calls of a function the module declares made on one.
    travels: Vec<(&'a str, Span, bool)>,
}

fn check_fn_body_shadow<'a>(
    (index, decl): (usize, &'a FnDecl),
    globals: &'a HashSet<&'a str>,
    diags: &mut Vec<Diagnostic>,
    shadowable: &'a HashSet<String>,
    declared: &'a Declared<'a>,
    locals: &mut Vec<Local<'a>>,
    travels: &mut Vec<Travel>,
) {
    let mut resolver = Resolver {
        globals,
        locals: std::mem::take(locals),
        diags: Vec::new(),
        std_origin: decl.file.starts_with("std/"),
        file: &decl.file,
        shadowable,
        declared,
        arriving: true,
        lambda_arrives: false,
        travels: Vec::new(),
    };
    for param in &decl.params {
        resolver.bind_pattern(param);
    }
    resolver.arriving = false;
    let last = decl.body.len() - 1;
    for (i, stmt) in decl.body.iter().enumerate() {
        match stmt {
            Stmt::Bind { pattern, expr } => {
                resolver.resolve_expr(expr);
                resolver.arriving = resolver.arrived(expr);
                if i == last {
                    resolver.diags.push(Diagnostic::new(
                        "unused",
                        "a body ends with its result expression, not a binding".to_string(),
                        expr.span(),
                    ));
                }
                resolver.bind_target(pattern);
                resolver.arriving = false;
            }
            Stmt::Expr(expr) => {
                resolver.resolve_expr(expr);
                if i != last {
                    resolver.diags.push(Diagnostic::new(
                        "unused",
                        "unused expression: every non-final line binds a name (sequence \
                         effects with `.>`)"
                            .to_string(),
                        expr.span(),
                    ));
                }
            }
        }
    }
    resolver.flush_unused(0);
    diags.append(&mut resolver.diags);
    travels.extend(resolver.travels.iter().map(|&(name, span, shaded)| {
        (crate::name::Name::new(name), span, shaded.then_some(index))
    }));
    *locals = resolver.locals;
}

impl<'a> Resolver<'a> {
    fn bind_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::Var(name, span) => self.push_local(name, *span),
            Pattern::Annotated { name, span, .. } => self.push_local(name, *span),
            Pattern::Ctor { fields, whole, .. } => {
                if let Some(named) = whole {
                    self.push_local(&named.0, named.1);
                }
                for field in fields {
                    self.bind_pattern(field);
                }
            }
            _ => {}
        }
    }

    fn bind_target(&mut self, pattern: &'a Pattern) {
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

    fn bind_target_field(&mut self, pattern: &'a Pattern) {
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

    fn push_local(&mut self, name: &'a str, span: Span) {
        if self.globals.contains(name) && !self.shadowable.contains(name) {
            self.diags.push(Diagnostic::new(
                "name",
                format!("`{name}` is already a declaration; rename the binding"),
                span,
            ));
        }
        self.locals.push(Local { name, span, used: false, arrived: self.arriving });
    }

    fn rebind(&mut self, name: &'a str, span: Span) {
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
        // The set borrows from `self.locals`, which is why the two fields are
        // taken apart here: the loop reads one and writes the other, and
        // reaching them through `self` would be one borrow doing both. Owning
        // the names instead was a `String` per binding in every scope the
        // checker left.
        //
        // A binding is shadowed when a later one in the same scope has its
        // name. A scope holds a handful of bindings, so the later ones are
        // scanned rather than gathered into a set, which was a table built for
        // every scope the checker closed.
        {
            let locals = &self.locals[from..];
            let diags = &mut self.diags;
            for (at, local) in locals.iter().enumerate().rev() {
                // `_:type` ascribes without binding: there is no name to use
                if local.name == "_" {
                    continue;
                }
                let shadowed = locals[at + 1..].iter().any(|l| l.name == local.name);
                if !local.used && !shadowed {
                    diags.push(Diagnostic::new(
                        "unused",
                        format!("unused binding `{}`", local.name),
                        local.span,
                    ));
                }
            }
        }
        self.locals.truncate(from);
    }

    fn resolve_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Int(..) | Expr::Float(..) => {}
            // `&f` reads f exactly as a bare mention does
            Expr::Partial(name, span) => self.resolve_name(name, *span),
            // A block's names are its own and go when it ends.
            Expr::Block(stmts, _) => {
                let from = self.locals.len();
                let last = stmts.len().saturating_sub(1);
                for (i, stmt) in stmts.iter().enumerate() {
                    match stmt {
                        Stmt::Bind { pattern, expr } => {
                            self.resolve_expr(expr);
                            self.arriving = self.arrived(expr);
                            self.bind_pattern(pattern);
                            self.arriving = false;
                        }
                        // The same rule the outer walk applies, applied here
                        // too. A non-final line whose value goes nowhere is a
                        // description nobody joined.
                        Stmt::Expr(expr) => {
                            self.resolve_expr(expr);
                            if i != last {
                                self.diags.push(Diagnostic::new(
                                    "unused",
                                    "unused expression: every non-final line binds a name \
                                     (sequence effects with `.>`)"
                                        .to_string(),
                                    expr.span(),
                                ));
                            }
                        }
                    }
                }
                if matches!(expr, Expr::Block(..)) {
                    self.locals.truncate(from);
                }
            }
            Expr::Field { base, .. } => self.resolve_expr(base),
            // `(x):type` names a type the way an annotation does, and until
            // this arm asked, nothing before the backends did. `kanso check`
            // answered ok and each engine then spoke for itself: the two
            // backends refused the whole program with `unknown type`, and the
            // interpreter said at run time that the value was not one.
            Expr::Upcast { expr, ty, span } => {
                for name in undeclared_in(ty, self.declared.types) {
                    self.diags.push(Diagnostic::new(
                        "type",
                        format!("no type is called `{name}`, so there is nothing to widen to"),
                        *span,
                    ));
                }
                self.resolve_expr(expr);
            }
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
            Expr::Ident(name, span, _) => self.resolve_name(name, *span),
            Expr::List(items, _) => {
                for item in items {
                    self.resolve_expr(item);
                }
            }
            Expr::App { head, args, .. } => {
                // Asked only of a call that could travel or that hands a lambda
                // on, so a call to a known name pays nothing for the rule.
                let travels = matches!(&**head, Expr::Ident(name, ..) if self.travels_to(name));
                let shades = matches!(&**head, Expr::Ident(name, ..) if self.shades(name));
                let lambdas = args.iter().any(|a| matches!(a, Expr::Lambda { .. }));
                let carries =
                    (travels || shades || lambdas) && args.iter().any(|a| self.arrived(a));
                match &**head {
                    Expr::Ident(name, span, _) if carries && travels => {
                        self.travels.push((name.as_str(), *span, false));
                    }
                    Expr::Ident(name, span, _) if carries && shades => {
                        self.travels.push((name.as_str(), *span, true));
                        self.resolve_expr(head);
                    }
                    _ => self.resolve_expr(head),
                }
                for arg in args {
                    self.lambda_arrives = carries && matches!(arg, Expr::Lambda { .. });
                    self.resolve_expr(arg);
                    self.lambda_arrives = false;
                }
                if let Expr::Ident(name, span, _) = &**head {
                    let local = self.locals.iter().any(|l| l.name == name.as_str());
                    if !local {
                        if let Some(&(fields, wraps)) = self.declared.type_arity.get(name.as_str())
                        {
                            if let Some(message) =
                                construction_arity(name, fields, wraps, args.len())
                            {
                                self.diags.push(Diagnostic::new("arity", message, *span));
                            }
                        }
                        if let Some(arities) = self.declared.fn_arities.get(name) {
                            if !arities.contains(&args.len()) && !arities.contains(&0) {
                                let mut known: Vec<String> =
                                    arities.iter().map(|a| a.to_string()).collect();
                                known.sort();
                                self.diags.push(Diagnostic::new(
                                    "arity",
                                    format!(
                                        "no {}-argument arm of `{}` (arms take {})",
                                        args.len(),
                                        name,
                                        known.join(", ")
                                    ),
                                    *span,
                                ));
                            }
                        }
                    }
                }
            }
            Expr::Index { base, index, .. } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::Lambda { params, body, .. } => {
                let base = self.locals.len();
                let outer = std::mem::replace(&mut self.arriving, self.lambda_arrives);
                self.lambda_arrives = false;
                for (name, span) in params {
                    if name != "_" {
                        self.push_local(name, *span);
                    }
                }
                self.arriving = outer;
                self.resolve_expr(body);
                self.flush_unused(base);
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::Guard { cond, early, rest, .. } => {
                self.resolve_expr(cond);
                self.resolve_expr(early);
                let base = self.locals.len();
                for stmt in rest.iter() {
                    match stmt {
                        Stmt::Bind { pattern, expr } => {
                            self.resolve_expr(expr);
                            self.arriving = self.arrived(expr);
                            self.bind_pattern(pattern);
                            self.arriving = false;
                        }
                        Stmt::Expr(expr) => self.resolve_expr(expr),
                    }
                }
                self.flush_unused(base);
            }
        }
    }

    /// Whether a value came in with the function rather than being made in it.
    fn arrived(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(name, ..) => self
                .locals
                .iter()
                .rev()
                .find(|l| l.name == name.as_str())
                .is_some_and(|l| l.arrived),
            Expr::Field { base, .. } | Expr::Index { base, .. } => self.arrived(base),
            Expr::Upcast { expr, .. } => self.arrived(expr),
            // What a traveling arm answers for an arrived value arrives too.
            Expr::App { head, args, .. } => match &**head {
                Expr::Ident(name, ..) => {
                    self.travels_to(name) && args.iter().any(|a| self.arrived(a))
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// A bare name nothing here declares: no local, no global this file can
    /// see, no builtin. Called on an arrived value, it names an arm the
    /// value's owner wrote, and the whole program finds it.
    fn travels_to(&self, name: &str) -> bool {
        !name.contains('/')
            && !self.globals.contains(name)
            && !name.starts_with("builtin_")
            && !self.locals.iter().any(|l| l.name == name)
    }

    /// A function the module declares, called on an arrived value: the call
    /// still reaches the module's own arms, and the arms the value's owner
    /// wrote join them unless one of the module's has the same shape.
    fn shades(&self, name: &str) -> bool {
        (self.declared.fn_arities.declares(name) || self.declared.sibling_fns.contains(name))
            && !self.locals.iter().any(|l| l.name == name)
    }

    fn resolve_name(&mut self, name: &str, span: Span) {
        if let Some(local) = self.locals.iter_mut().rev().find(|l| l.name == name) {
            local.used = true;
            return;
        }
        if let Some(stripped) = name.strip_prefix("builtin_") {
            // Bare only. `builtin_` is a prefix on a name a program writes for
            // itself, and a QUALIFIED name is a declaration in another module —
            // whose own name may perfectly well start with those bytes. Matching
            // the head of `builtin_shapes/circle` read it as a builtin nobody
            // has, so a module named for what it holds could be imported and
            // never used: every reference to it was refused.
            //
            // The slash is asked on the REFUSING arm rather than beside the
            // prefix, because that arm is the one a correct program never
            // reaches. Asked in front it scanned every builtin reference the
            // standard library makes, and the compile corpus read 6,347
            // instructions more for an answer only the refusal needs.
            match self.std_origin && BUILTINS.contains(&stripped) {
                true => return,
                false if BUILTINS.contains(&stripped) && in_shipped_library(self.file) => return,
                false if !name.contains('/') => {
                    self.diags.push(Diagnostic::new(
                        "name",
                        format!("`{name}` is internal to the standard library — import its module"),
                        span,
                    ));
                    return;
                }
                false => {}
            }
        }
        match self.globals.contains(name) {
            // The name resolves; nothing more to say about it here. A set of
            // which globals a file used was recorded at this line for an
            // unused-private check that has since moved to `lib::private_uses`,
            // and nothing read it after the move — a `String` per mention, for
            // a set that was dropped.
            true => {}
            false => {
                self.diags.push(Diagnostic::new(
                    "name",
                    format!("unknown name `{name}`{}", moved_to_os(name)),
                    span,
                ));
            }
        }
    }
}
