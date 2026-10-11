// The checks on `list/tie`: where a maker's `ref` may go, and which links a
// literal graph may name. check.rs reads this file with include!, so the
// items stay in the check module and the compiler builds to the same
// instructions it did when they sat in check.rs (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under
// src/'s subdirectories itself.

/// Ruled 2026-10-04: a reference `list/tie` hands its maker is stored in a
/// constructor's field and nowhere else, because the node it names may not be
/// made yet. So every call of the maker's `ref` is an argument of a
/// constructor, or an element of a list or map literal that is one: a field
/// may hold a collection, ruled 2026-10-10, and a literal that holds
/// references reads none of them. `ref` itself may go to a function the program declares, and
/// that function's parameter is then held to the same rule, which is how a
/// read inside a helper the maker calls is found. A `ref` handed anywhere else
/// is refused, since what happens to it there is out of this check's sight.
///
/// Where the ids are a list of literals and a link is a literal, the whole
/// graph is visible, and a link to an id the list does not hold is refused
/// here rather than answered as a broken link at run time.
fn check_tie_references(program: &Program, diags: &mut Vec<Diagnostic>) {
    // The calls first, and the tables the check reads only when there is one:
    // a program that never ties pays for one walk of its own functions. std's
    // functions are left out, because no std module calls `list/tie`.
    let mut sites: Vec<(&Expr, &std::sync::Arc<str>)> = Vec::new();
    for d in program.fns.iter().filter(|d| !d.file.starts_with("std/")) {
        for st in &d.body {
            tie_calls(stmt_expr(st), &d.file, &mut sites);
        }
    }
    if sites.is_empty() {
        return;
    }
    let ctors: HashSet<&str> =
        program.types.iter().filter(|t| t.members.is_empty()).map(|t| t.name.as_str()).collect();
    let mut groups: HashMap<&str, Vec<&FnDecl>> = HashMap::default();
    for d in &program.fns {
        groups.entry(d.name.as_str()).or_default().push(d);
    }
    let mut tie = TieCheck { ctors: &ctors, groups: &groups, held: Vec::new(), diags: Vec::new() };
    for (call, file) in sites {
        let first = tie.diags.len();
        tie.check_tie(call);
        if !file.is_empty() {
            place_in(&mut tie.diags[first..], file);
        }
    }
    let mut seen: HashSet<(&str, usize)> = HashSet::default();
    while let Some((name, at)) = tie.held.pop() {
        if !seen.insert((name, at)) {
            continue;
        }
        let Some(arms) = groups.get(name) else { continue };
        for arm in arms {
            let Some(Pattern::Var(r, _)) = arm.params.get(at) else { continue };
            let first = tie.diags.len();
            for st in &arm.body {
                tie.reads(stmt_expr(st), r.as_str(), None, false);
            }
            if !arm.file.is_empty() {
                place_in(&mut tie.diags[first..], &arm.file);
            }
        }
    }
    diags.append(&mut tie.diags);
}

/// Every `list/tie` and `list/tie!` call under `e`, a maker's own included.
fn tie_calls<'a>(
    e: &'a Expr,
    file: &'a std::sync::Arc<str>,
    sites: &mut Vec<(&'a Expr, &'a std::sync::Arc<str>)>,
) {
    if let Expr::App { head, args, .. } = e {
        let tied =
            matches!(head.as_ref(), Expr::Ident(n, _, _) if n == "list/tie" || n == "list/tie!");
        if tied && args.len() == 2 {
            sites.push((e, file));
        }
    }
    crate::for_each_child(e, |c| tie_calls(c, file, sites));
}

fn stmt_expr(st: &Stmt) -> &Expr {
    match st {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
    }
}

struct TieCheck<'a> {
    ctors: &'a HashSet<&'a str>,
    groups: &'a HashMap<&'a str, Vec<&'a FnDecl>>,
    /// A function and the position of a parameter that receives `ref`.
    held: Vec<(&'a str, usize)>,
    diags: Vec<Diagnostic>,
}

/// A literal id or link, spelled so an int and a string never collide.
fn tie_literal(e: &Expr) -> Option<String> {
    match e {
        Expr::Int(n, _) => Some(format!("int {n}")),
        Expr::Str(parts, _) => match parts.as_slice() {
            [] => Some("str ".to_string()),
            [TemplatePart::Lit(s)] => Some(format!("str {s}")),
            _ => None,
        },
        _ => None,
    }
}

impl<'a> TieCheck<'a> {
    /// One call `tie_calls` found: its maker's `ref` is held to the rule.
    fn check_tie(&mut self, call: &'a Expr) {
        let Expr::App { args, .. } = call else { return };
        let [ids, maker] = args.as_slice() else { return };
        let literal: Option<HashSet<String>> = match ids {
            Expr::List(items, _) => items.iter().map(tie_literal).collect(),
            _ => None,
        };
        match maker {
            Expr::Lambda { params, body, .. } if params.len() == 2 => {
                self.reads(body, params[1].0.as_str(), literal.as_ref(), false);
            }
            Expr::Ident(f, _, _) | Expr::Partial(f, _) => {
                if let Some((name, _)) = self.groups.get_key_value(f.as_str()) {
                    self.held.push((name, 1));
                }
            }
            _ => {}
        }
    }

    /// Walks `e`, where `r` is a `ref`. `in_field` says `e` is itself an
    /// argument of a constructor.
    fn reads(&mut self, e: &'a Expr, r: &str, literal: Option<&HashSet<String>>, in_field: bool) {
        match e {
            Expr::App { head, args, span, .. } => {
                let callee = match head.as_ref() {
                    Expr::Ident(n, _, _) => Some(n.as_str()),
                    _ => None,
                };
                if callee == Some(r) {
                    if !in_field {
                        self.diags.push(Diagnostic::new(
                            "tie",
                            format!(
                                "`{r}` answers a reference to a node that may not be made yet, \
                                 so it goes straight into a constructor's field, alone or in a \
                                 list or map written there, and is never read or kept anywhere \
                                 else"
                            ),
                            *span,
                        ));
                    }
                    if let (Some(ids), [link]) = (literal, args.as_slice()) {
                        if let Some(key) = tie_literal(link).filter(|k| !ids.contains(k)) {
                            let shown = key.split_once(' ').map_or(key.as_str(), |(_, v)| v);
                            self.diags.push(Diagnostic::new(
                                "tie",
                                format!(
                                    "`{r}` links to `{shown}`, and no id in the list is \
                                     `{shown}`, so this tie can only break"
                                ),
                                *span,
                            ));
                        }
                    }
                    for a in args {
                        self.reads(a, r, literal, false);
                    }
                    return;
                }
                let builds = callee.is_some_and(|n| self.ctors.contains(n));
                for (at, a) in args.iter().enumerate() {
                    if matches!(a, Expr::Ident(n, _, _) if n == r) {
                        self.handed(callee, at, r, a.span());
                        continue;
                    }
                    self.reads(a, r, literal, builds);
                }
                if !matches!(head.as_ref(), Expr::Ident(..)) {
                    self.reads(head, r, literal, false);
                }
            }
            Expr::Lambda { params, body, .. } => {
                if params.iter().all(|(p, _)| p != r) {
                    self.reads(body, r, literal, false);
                }
            }
            Expr::Ident(n, span, _) if n == r => self.handed(None, 0, r, *span),
            // A list or map literal written as a field is storage, and so is
            // each element in it, ruled 2026-10-10. A map's key is read to
            // place the pair, so only its values may hold a reference.
            Expr::List(items, _) if in_field => {
                for item in items {
                    self.reads(item, r, literal, true);
                }
            }
            Expr::MapLit(pairs, _) if in_field => {
                for (key, value) in pairs {
                    self.reads(key, r, literal, false);
                    self.reads(value, r, literal, true);
                }
            }
            _ => crate::for_each_child(e, |c| self.reads(c, r, literal, false)),
        }
    }

    /// `ref` itself, handed to `callee` at position `at`.
    fn handed(&mut self, callee: Option<&str>, at: usize, r: &str, span: Span) {
        let declared = callee.and_then(|n| self.groups.get_key_value(n));
        if let Some((name, arms)) = declared.filter(|(_, arms)| !arms[0].file.starts_with("std/")) {
            if arms.iter().any(|d| d.params.len() > at) {
                self.held.push((name, at));
                return;
            }
        }
        let to = callee.map_or("a value".to_string(), |n| format!("`{n}`"));
        self.diags.push(Diagnostic::new(
            "tie",
            format!(
                "`{r}` is handed to {to}, and what happens to it there is out of sight; \
                 call `{r}` where its answer goes straight into a constructor's field, \
                 or hand it to a function of this program"
            ),
            span,
        ));
    }
}
