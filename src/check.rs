use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use std::collections::{HashMap, HashSet};

pub const BUILTINS: [&str; 46] = [
    "append",
    "args",
    "bytes",
    "char_code",
    "chars",
    "concat",
    "entries",
    "find2",
    "find2_below",
    "from_code",
    "list_dir",
    "now",
    "if",
    "is_desc",
    "join",
    "length",
    "print",
    "push",
    "put",
    "random",
    "read_file",
    "render_value",
    "run",
    "round",
    "sleep",
    "slice",
    "split",
    "sqrt",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "bit_shl",
    "bit_shr",
    "env",
    "exists",
    "is_dir",
    "stdin",
    "to_float",
    "to_int",
    "utf8",
    "wrap_err",
    "write",
    "write_err",
    "make_dir",
    "write_file",
];

/// The bare-name subset: what resolves without an import. Everything else
/// in BUILTINS is internal, reached only through std wrapper modules.
pub const AMBIENT: [&str; 6] = ["entries", "if", "length", "print", "push", "put"];

/// The harness's own vocabulary. A package may not turn its own failure into
/// a value — that is the two-universe rule — and a test has to, or a library
/// can never assert that its errors happen. The way out is not an exemption:
/// the harness is a foreign party, so a *builtin* doing the reading is a
/// stranger rescuing, which the rule already permits. Attribution proves it —
/// an err is only ever rescued where a pattern names it, and a builtin has no
/// pattern for a package to be blamed for.
///
/// It is in scope in a `_test.kso` file and nowhere else — gated by the file
/// that declares the caller, the way `builtin_` names are gated to std. A
/// production file cannot reach it, so the rule stands undiminished rather
/// than carrying a hole a program could reach for. Gating on the *file*
/// rather than on the verb also keeps `kanso check` honest: it compiles test
/// files too, and a name that existed under one verb and not another would be
/// a worse thing to explain than where it lives.
pub const HARNESS: [&str; 1] = ["failed?"];

fn harness_file(file: &str) -> bool {
    file.ends_with("_test.kso")
}

pub fn check(program: &mut Program, require_entry: bool) -> Vec<Diagnostic> {
    let markers = marker_names(program);
    let type_names = program.types.iter().map(|t| t.name.clone()).collect();
    let mut used = HashSet::new();
    let mut diags = resolve_markers(program, &markers);
    diags.extend(check_typesets(program, &type_names));
    diags.extend(check_file(program, &HashSet::new(), &mut used));
    if require_entry {
        check_entry(program, &mut diags);
    }
    check_unused_private(program, &used, &mut diags);
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

/// An effect is a description, and a description that nobody forces never
/// happens. The language already refuses the neighbouring mistake — a body
/// that does io must hand the io back, or it "would abandon the effects above
/// it" — but that reads a function's own body, so an effect abandoned inside
/// somebody else's parameter walked straight past it.
///
/// Only the unambiguous case is refused: every arm of the group discards the
/// position, so there is no reading of the program where the effect happens.
/// A group where one arm reads the parameter and another does not is a real
/// question and a different one.
/// `err` names a raise, and a raise records where it happened. Handed over as
/// a value the record has nowhere to come from: the interpreter attributes it
/// to whatever line applies it, deep inside a callee the author never wrote,
/// and a compiled wrapper has only its own definition to point at. A lambda
/// puts the mention back in the program — `(reason -> err reason)` reads the
/// same and carries a line a reader can open.
/// `flag == true` asks a question that already has its answer in `flag`, and
/// `flag == false` is `not flag` written the long way. Neither can be anything
/// but noise, and kanso has no linter layer for noise to live in, so what a
/// linter elsewhere would warn about is refused here.
fn check_boolean_equality(program: &Program, diags: &mut Vec<Diagnostic>) {
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            boolean_equality_walk(expr, diags);
        }
    }
}

fn boolean_equality_walk(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    if let Expr::BinOp { op, lhs, rhs, span } = expr {
        if matches!(*op, "==" | "!=") {
            if let Some(word) = boolean_literal(lhs).or_else(|| boolean_literal(rhs)) {
                diags.push(Diagnostic::new("name", said_plainly(op, word), *span));
            }
        }
    }
    for child in crate::expr_children(expr) {
        boolean_equality_walk(child, diags);
    }
}

fn boolean_literal(expr: &Expr) -> Option<&'static str> {
    let Expr::Ident(name, _) = expr else {
        return None;
    };
    match name.as_str() {
        "true" => Some("true"),
        "false" => Some("false"),
        _ => None,
    }
}

/// Which of the four spellings this is decides what to say instead, and all
/// four have a shorter answer.
fn said_plainly(op: &str, word: &str) -> String {
    let plain = match (op, word) {
        ("==", "true") | ("!=", "false") => "the value itself",
        _ => "`not` the value",
    };
    format!("comparing to `{word}` asks a question the value already answers — write {plain}")
}

/// `[plotted "x" "y" "z"]` is four elements: the function and three strings.
/// A list element is one atom — the language already says so for operators,
/// where `[1 + 2]` is a syntax error rather than three elements — and this
/// says it for application, where the parser was silently obliging instead.
///
/// There is no guessing involved, because both readings have a spelling: `(f
/// x)` calls, and `&f` holds. Refusing the bare form costs a sigil and takes
/// away a shape whose two meanings looked identical.
fn check_call_shaped_list(program: &Program, diags: &mut Vec<Diagnostic>) {
    let mut arities: std::collections::HashMap<&str, usize> = Default::default();
    for decl in &program.fns {
        let widest = arities.entry(decl.name.as_str()).or_insert(0);
        *widest = (*widest).max(decl.params.len());
    }
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        // A name bound here is this scope's, whatever a function elsewhere is
        // called: sha256 binds `first` and `list/first` is not what it means.
        let mut bound: std::collections::HashSet<&str> = Default::default();
        for p in &decl.params {
            collect_pattern_names(p, &mut bound);
        }
        for stmt in &decl.body {
            if let Stmt::Bind { pattern, .. } = stmt {
                collect_pattern_names(pattern, &mut bound);
            }
        }
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            call_shaped_walk(expr, &arities, &bound, diags);
        }
    }
}

fn collect_pattern_names<'a>(p: &'a Pattern, out: &mut std::collections::HashSet<&'a str>) {
    match p {
        Pattern::Var(name, _) => {
            out.insert(name.as_str());
        }
        Pattern::Annotated { name, .. } => {
            out.insert(name.as_str());
        }
        Pattern::Ctor { fields, .. } => {
            for f in fields {
                collect_pattern_names(f, out);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for e in entries {
                out.insert(e.bind_name.as_str());
            }
        }
        _ => {}
    }
}

fn call_shaped_walk(
    expr: &Expr,
    arities: &std::collections::HashMap<&str, usize>,
    bound: &std::collections::HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Expr::List(items, _) = expr {
        for (at, item) in items.iter().enumerate() {
            let Expr::Ident(name, span) = item else {
                continue;
            };
            let following = items.len() - at - 1;
            if bound.contains(name.as_str()) {
                continue;
            }
            let takes = arities.get(name.as_str()).copied().unwrap_or(0);
            if takes > 0 && following > 0 {
                diags.push(Diagnostic::new(
                    "syntax",
                    format!(
                        "`{name}` takes {takes} argument(s), and a list element is one \
                         atom — this reads as {} separate elements. Write `({name} …)` \
                         to call it, or `&{name}` to hold it",
                        items.len()
                    ),
                    *span,
                ));
            }
        }
    }
    for child in crate::expr_children(expr) {
        call_shaped_walk(child, arities, bound, diags);
    }
}

fn check_err_as_value(program: &Program, diags: &mut Vec<Diagnostic>) {
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    err_value_walk(expr, diags)
                }
            }
        }
    }
}

fn err_value_walk(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    let mut found = Vec::new();
    err_value_scan(expr, &mut found);
    for span in found {
        diags.push(Diagnostic::new(
            "name",
            "`err` raises, so it is a call and not a value — an err records the line it \
             was raised on, and a bare mention has no line to record. Write \
             `(reason -> err reason)`"
                .to_string(),
            span,
        ));
    }
}

/// Every mention of `err` outside call position, collected rather than
/// reported so the walk stays a plain traversal.
fn err_value_scan(expr: &Expr, found: &mut Vec<Span>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) => {}
        Expr::Ident(name, span) => {
            if name == "err" {
                found.push(*span);
            }
        }
        Expr::Partial(name, span) => {
            if name == "err" {
                found.push(*span);
            }
        }
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        err_value_scan(expr, found)
                    }
                }
            }
        }
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                err_value_scan(key, found);
                err_value_scan(value, found);
            }
        }
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    err_value_scan(inner, found);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                err_value_scan(item, found);
            }
        }
        Expr::App { head, args, .. } => {
            // the head of a call with arguments IS the raise, which is the one
            // place the mention names a line of its own
            match (&**head, args.is_empty()) {
                (Expr::Ident(name, _), false) if name == "err" => {}
                _ => err_value_scan(head, found),
            }
            for arg in args {
                err_value_scan(arg, found);
            }
        }
        Expr::Field { base, .. } => err_value_scan(base, found),
        Expr::Upcast { expr, .. } => err_value_scan(expr, found),
        Expr::Index { base, index, .. } => {
            err_value_scan(base, found);
            err_value_scan(index, found);
        }
        Expr::Seq(lhs, rhs, _) => {
            err_value_scan(lhs, found);
            err_value_scan(rhs, found);
        }
        Expr::Lambda { body, .. } => err_value_scan(body, found),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            err_value_scan(lhs, found);
            err_value_scan(rhs, found);
        }
        Expr::Guard { cond, early, rest, .. } => {
            err_value_scan(cond, found);
            err_value_scan(early, found);
            for stmt in rest {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        err_value_scan(expr, found)
                    }
                }
            }
        }
    }
}

fn check_effect_discarded(
    program: &Program,
    inference: &crate::infer::Inference,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::infer::DESC;
    // inference is handed in: one pass serves every check that reads it

    let mut returns: std::collections::HashMap<(&str, usize), crate::infer::Set> =
        Default::default();
    // a position every arm throws away
    let mut discarded: std::collections::HashMap<(&str, usize, usize), bool> = Default::default();
    for (i, d) in program.fns.iter().enumerate() {
        let key = (d.name.as_str(), d.params.len());
        *returns.entry(key).or_insert(0) |= inference.returns[i];
        for (pos, param) in d.params.iter().enumerate() {
            let wildcard = matches!(param, Pattern::Wildcard(_));
            let slot = discarded.entry((d.name.as_str(), d.params.len(), pos)).or_insert(true);
            *slot &= wildcard;
        }
    }

    let describes = |e: &Expr| -> bool {
        let Expr::App { head, args, piped: false, .. } = e else { return false };
        let Expr::Ident(name, _) = head.as_ref() else { return false };
        returns.get(&(name.as_str(), args.len())).is_some_and(|s| s & DESC != 0)
    };

    let walk = |e: &Expr, diags: &mut Vec<Diagnostic>| {
        let Expr::App { head, args, piped: false, .. } = e else { return };
        let Expr::Ident(name, _) = head.as_ref() else { return };
        if !returns.contains_key(&(name.as_str(), args.len())) {
            return;
        }
        for (pos, arg) in args.iter().enumerate() {
            if !describes(arg) {
                continue;
            }
            if !discarded.get(&(name.as_str(), args.len(), pos)).copied().unwrap_or(false) {
                continue;
            }
            diags.push(Diagnostic::new(
                "effect",
                format!(
                    "`{name}` reads none of what it is handed here, so this effect \
                     never happens — hand it back, or give `{name}` a name for it"
                ),
                arg.span(),
            ));
        }
    };

    for decl in &program.fns {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
            };
            let mut stack = vec![e];
            while let Some(cur) = stack.pop() {
                walk(cur, diags);
                stack.extend(crate::expr_children(cur));
            }
        }
    }
}

/// The gavel makes a receiver responsible for a none it can be handed, and
/// lets the caller discharge that by resolving first. The report belongs at
/// the argument, because that is the line an author edits.
fn check_none_exhaustive(
    program: &Program,
    inference: &crate::infer::Inference,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::infer::NONE;
    // inference is handed in: one pass serves every check that reads it

    // group -> joined return set, and whether any arm names none at a position
    let mut returns: std::collections::HashMap<(&str, usize), crate::infer::Set> =
        Default::default();
    let mut handles: std::collections::HashMap<(&str, usize, usize), bool> = Default::default();
    for (i, d) in program.fns.iter().enumerate() {
        let key = (d.name.as_str(), d.params.len());
        *returns.entry(key).or_insert(0) |= inference.returns[i];
        for (pos, param) in d.params.iter().enumerate() {
            let names_none = match param {
                Pattern::Nullary(n, _) => n == "none",
                Pattern::Annotated { ty, .. } => ty == "none",
                _ => false,
            };
            *handles.entry((d.name.as_str(), d.params.len(), pos)).or_insert(false) |= names_none;
        }
    }

    // only what is provable: a lenient read, a literal none, or a call whose
    // group's return set carries one
    let yields_none = |e: &Expr| -> bool {
        match e {
            Expr::Index { strict: false, .. } => true,
            Expr::Ident(name, _) => name == "none",
            Expr::App { head, args, piped: false, .. } => match head.as_ref() {
                Expr::Ident(name, _) => {
                    returns.get(&(name.as_str(), args.len())).is_some_and(|s| s & NONE != 0)
                }
                _ => false,
            },
            _ => false,
        }
    };

    let walk = |e: &Expr, diags: &mut Vec<Diagnostic>, owner: &str| {
        let Expr::App { head, args, piped: false, .. } = e else { return };
        let Expr::Ident(name, _) = head.as_ref() else { return };
        if !returns.contains_key(&(name.as_str(), args.len())) {
            return;
        }
        for (pos, arg) in args.iter().enumerate() {
            if !yields_none(arg) {
                continue;
            }
            if *handles.get(&(name.as_str(), args.len(), pos)).unwrap_or(&false) {
                continue;
            }
            // an absolute path built from the binary's location tells a
            // reader nothing; the module name is what they searched for
            let short = owner.rsplit_once("/lib/").map(|(_, m)| m).unwrap_or(owner);
            diags.push(Diagnostic::new(
                "exhaustive",
                format!(
                    "this can be a none and `{name}` has no arm for it — resolve it \
                     here, or give `{name}` a `none` arm (in {short})"
                ),
                arg.span(),
            ));
        }
    };

    for decl in &program.fns {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
            };
            let mut stack = vec![e];
            while let Some(cur) = stack.pop() {
                walk(cur, diags, &decl.file);
                stack.extend(crate::expr_children(cur));
            }
        }
    }
}

/// A lookup answers "not found" with a none, so a collection that could
/// hold one would make every lenient read ambiguous. A record field is
/// different: it is known to exist, so a none there means the value is
/// nothing and nothing else.
fn check_none_in_collections(program: &Program, diags: &mut Vec<Diagnostic>) {
    fn is_none_lit(e: &Expr) -> bool {
        matches!(e, Expr::Ident(name, _) if name == "none")
    }
    fn walk(e: &Expr, diags: &mut Vec<Diagnostic>) {
        match e {
            Expr::List(items, _) => {
                for item in items.iter().filter(|i| is_none_lit(i)) {
                    diags.push(Diagnostic::new(
                        "none",
                        "a list cannot hold a none: a lookup answers \"not found\" \
                         with one, so an element would be indistinguishable"
                            .to_string(),
                        item.span(),
                    ));
                }
            }
            Expr::MapLit(pairs, _) => {
                for (_, value) in pairs.iter().filter(|(_, v)| is_none_lit(v)) {
                    diags.push(Diagnostic::new(
                        "none",
                        "a map cannot hold a none: a lookup answers \"not found\" \
                         with one, so a value would be indistinguishable"
                            .to_string(),
                        value.span(),
                    ));
                }
            }
            _ => {}
        }
        for child in crate::expr_children(e) {
            walk(child, diags);
        }
    }
    for decl in &program.fns {
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
            };
            walk(e, diags);
        }
    }
}

/// Whatever `primitive` a literal expression is, by the same rule the
/// constructor check uses.
fn literal_type(e: &Expr) -> Option<&'static str> {
    match e {
        Expr::Int(..) => Some("int"),
        Expr::Float(..) => Some("float64"),
        Expr::Str(parts, _) => match parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))) {
            true => Some("string"),
            false => None,
        },
        Expr::List(..) => Some("list"),
        Expr::MapLit(..) => Some("map"),
        Expr::Ident(name, _) if name == "true" || name == "false" => Some("bool"),
        _ => None,
    }
}

/// The constructor check keeps a field's promise where the value is written
/// out; assignment is the other place a field is written, and only a `build`
/// block may do it. Without this the promise holds until the knot is tied and
/// then stops holding, which is the worst of both.
/// "an int", "a string" — a diagnostic that fumbles its own grammar reads as
/// carelessness about everything else in it.
fn article(word: &str) -> String {
    match word.starts_with(['a', 'e', 'i', 'o', 'u']) {
        true => format!("an {word}"),
        false => format!("a {word}"),
    }
}

/// What every construction site puts into each field of each type, as the set
/// of literal kinds seen there. This is the supply side of a field's type,
/// and with no annotation it is all the compiler is told.
fn field_supply(program: &Program) -> HashMap<(&str, &str), Vec<(&'static str, Span)>> {
    let mut supply: HashMap<(&str, &str), Vec<(&'static str, Span)>> = HashMap::new();
    for decl in &program.fns {
        let mut stack: Vec<&Expr> = Vec::new();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            stack.push(expr);
        }
        while let Some(cur) = stack.pop() {
            if let Expr::App { head, args, .. } = cur {
                if let Expr::Ident(name, _) = head.as_ref() {
                    if let Some(ty) = program.types.iter().find(|t| t.name == *name) {
                        if ty.fields.len() == args.len() {
                            for ((field, _, _), arg) in ty.fields.iter().zip(args) {
                                if let Some(kind) = literal_type(arg) {
                                    supply
                                        .entry((ty.name.as_str(), field.as_str()))
                                        .or_default()
                                        .push((kind, arg.span()));
                                }
                            }
                        }
                    }
                }
            }
            stack.extend(crate::expr_children(cur));
        }
    }
    supply
}

/// A field's type is not only what construction sites put in — it is also what
/// its uses require. Reading `p.name` into a parameter declared `string` says
/// the field holds strings, and a construction site that put an int there is
/// not a missing annotation but a disagreement between two things the program
/// already states. Reporting it names both, because neither line is wrong on
/// its own and the author is the one who knows which they meant.
fn check_field_conflicts(program: &Program, diags: &mut Vec<Diagnostic>) {
    let supply = field_supply(program);

    // a local bound straight to a constructor is where a field read's owning
    // type is knowable without knowing anything else, the same footing the
    // assignment check stands on
    for decl in &program.fns {
        let mut built: HashMap<&str, &str> = HashMap::new();
        for stmt in &decl.body {
            if let Stmt::Bind { pattern: Pattern::Var(name, _), expr: Expr::App { head, .. } } =
                stmt
            {
                if let Expr::Ident(ty, _) = head.as_ref() {
                    if program.types.iter().any(|t| t.name == *ty) {
                        built.insert(name.as_str(), ty.as_str());
                    }
                }
            }
        }
        if built.is_empty() {
            continue;
        }
        let mut stack: Vec<&Expr> = Vec::new();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            stack.push(expr);
        }
        while let Some(cur) = stack.pop() {
            if let Expr::App { head, args, .. } = cur {
                if let Expr::Ident(callee, _) = head.as_ref() {
                    demand_conflicts(program, &supply, &built, callee, args, diags);
                }
            }
            stack.extend(crate::expr_children(cur));
        }
    }
}

/// One call: for each argument that reads a field off a local whose type is
/// known, compare what the callee's parameter demands against what the field
/// is actually given anywhere in the program.
fn demand_conflicts(
    program: &Program,
    supply: &HashMap<(&str, &str), Vec<(&'static str, Span)>>,
    built: &HashMap<&str, &str>,
    callee: &str,
    args: &[Expr],
    diags: &mut Vec<Diagnostic>,
) {
    let Some(target) =
        program.fns.iter().find(|d| d.name == callee && d.params.len() == args.len())
    else {
        return;
    };
    for (param, arg) in target.params.iter().zip(args) {
        let Pattern::Annotated { ty: demanded, .. } = param else { continue };
        let Expr::Field { base, name: field, span } = arg else { continue };
        let Expr::Ident(local, _) = base.as_ref() else { continue };
        let Some(owner) = built.get(local.as_str()) else { continue };
        let Some(given) = supply.get(&(*owner, field.as_str())) else { continue };
        for (kind, where_) in given {
            if kind != demanded {
                diags.push(Diagnostic::new(
                    "type",
                    format!(
                        "`{owner}`'s `{field}` is {} here, and `{callee}` takes {} — these \
                         two cannot both hold",
                        article(kind),
                        article(demanded)
                    ),
                    *where_,
                ));
                diags.push(Diagnostic::new(
                    "type",
                    format!("...and this is where it is read as {}", article(demanded)),
                    *span,
                ));
            }
        }
    }
}

/// The engines carry `none` as a tag rather than a declared type, so nothing
/// can derive from it yet. Rejecting here keeps all three engines identical
/// instead of one erroring and another silently dropping the subtype.
fn check_sub_parents(program: &Program, diags: &mut Vec<Diagnostic>) {
    for ty in &program.types {
        if ty.parent.as_deref() == Some("none") {
            diags.push(Diagnostic::new(
                "type",
                format!(
                    "`{}` cannot derive from `none` yet — name the members you \
                     mean, or use a zero-field marker type",
                    ty.name
                ),
                ty.span,
            ));
        }
    }
}

/// The gaveled tie-rejection: with subtypes, two arms of one group can
/// overlap without either dominating — a value both accept, where each
/// arm is the more specific one somewhere. Dispatch would need tiebreak
/// lore; kanso outlaws the shape instead. The fix is always the same:
/// write the doubly-specific arm.
pub fn check_arm_ties(program: &Program, diags: &mut Vec<Diagnostic>) {
    let parents: std::collections::HashMap<&str, &str> = program
        .types
        .iter()
        .filter_map(|t| t.parent.as_deref().map(|p| (t.name.as_str(), p)))
        .collect();
    if parents.is_empty() {
        return;
    }
    let relation = |a: &str, b: &str| -> Option<i64> {
        if a == b {
            return Some(0);
        }
        let mut cur = a;
        let mut d = 0i64;
        while let Some(p) = parents.get(cur) {
            d += 1;
            cur = p;
            if cur == b {
                return Some(d);
            }
        }
        let mut cur = b;
        let mut d = 0i64;
        while let Some(p) = parents.get(cur) {
            d += 1;
            cur = p;
            if cur == a {
                return Some(-d);
            }
        }
        None
    };
    let compare = |pa: &Pattern, pb: &Pattern| -> Option<i64> {
        let rank = |p: &Pattern| p.rank() as i64;
        match (pa, pb) {
            (Pattern::Annotated { ty: ta, .. }, Pattern::Annotated { ty: tb, .. }) => {
                relation(ta, tb)
            }
            _ => match rank(pa) == rank(pb) {
                true => Some(0),
                false => Some(rank(pb) - rank(pa)),
            },
        }
    };
    let mut groups: std::collections::HashMap<(&str, usize), Vec<&FnDecl>> =
        std::collections::HashMap::new();
    for d in &program.fns {
        groups.entry((d.name.as_str(), d.params.len())).or_default().push(d);
    }
    for ((name, _), decls) in groups {
        for i in 0..decls.len() {
            for j in i + 1..decls.len() {
                let a = decls[i];
                let b = decls[j];
                let mut a_stricter = false;
                let mut b_stricter = false;
                let mut overlap = true;
                for (pa, pb) in a.params.iter().zip(&b.params) {
                    match compare(pa, pb) {
                        None => {
                            overlap = false;
                            break;
                        }
                        Some(c) if c > 0 => a_stricter = true,
                        Some(c) if c < 0 => b_stricter = true,
                        _ => {}
                    }
                }
                if overlap && a_stricter && b_stricter {
                    diags.push(Diagnostic::new(
                        "dispatch",
                        format!(
                            "these `{name}` arms tie: each is the more specific \
                             one somewhere, and a call could match both — write \
                             the arm that is most specific in every position"
                        ),
                        b.span,
                    ));
                }
            }
        }
    }
}

/// Zero-field type names: a bare mention is the marker value, and in
/// parameter position the bare name matches that marker.
pub fn marker_names(program: &Program) -> HashSet<String> {
    program
        .types
        .iter()
        .filter(|t| t.fields.is_empty() && t.parent.is_none() && t.members.is_empty())
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
                Stmt::Set { value, .. } => check_marker_calls(value, markers, &mut diags),
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
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) | Expr::Partial(..) => {}
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        check_marker_calls(expr, markers, diags)
                    }
                }
            }
        }
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
        Expr::Upcast { expr, .. } => check_marker_calls(expr, markers, diags),
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
        Expr::Guard { cond, early, rest, .. } => {
            check_marker_calls(cond, markers, diags);
            check_marker_calls(early, markers, diags);
            for stmt in rest {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        check_marker_calls(expr, markers, diags)
                    }
                }
            }
        }
    }
}

/// Builtin type words legal as typeset members alongside declared types.
const TYPESET_BUILTINS: [&str; 5] = ["bool", "float64", "int", "none", "string"];

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
    check_constant_cycles(program, &mut diags);
    check_retired_any(program, &mut diags);
    check_field_annotations(program, &mut diags);
    check_field_conflicts(program, &mut diags);
    check_annotation_names(program, &mut diags);
    let mut globals = collect_globals(program, &mut diags);
    globals.extend(extern_globals.iter().cloned());
    let mut fn_arities: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for decl in &program.fns {
        let arities = fn_arities.entry(decl.name.clone()).or_default();
        if !arities.contains(&decl.params.len()) {
            arities.push(decl.params.len());
        }
    }
    // A construction is positional and complete: `point 1` where `point`
    // declares two fields is the same mistake as calling a two-argument
    // function with one, and the compiler knows both counts. A typeset never
    // constructs and a subtype takes the one value it wraps, so neither is
    // entered here.
    let type_arity: std::collections::HashMap<String, usize> = program
        .types
        .iter()
        .filter(|t| t.members.is_empty() && t.parent.is_none() && !t.fields.is_empty())
        .map(|t| (t.name.clone(), t.fields.len()))
        .collect();
    for decl in &program.fns {
        check_fn_body_shadow(
            decl,
            &globals,
            used_globals,
            &mut diags,
            shadowable,
            &fn_arities,
            &type_arity,
        );
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
/// The `?` sigil is enforced through the return-set lattice, both ways: a
/// `?` name must answer with true/false (err allowed — a predicate over
/// fallible work stays a predicate), and a group answering only in booleans
/// must carry the `?`. Mixed sets are exempt in the unmarked direction.
fn check_predicates(
    program: &Program,
    inference: &crate::infer::Inference,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::infer::{ERR, FALSE, TRUE};
    // inference is handed in: one pass serves every check that reads it
    let mut groups: std::collections::HashMap<&str, (crate::infer::Set, crate::diag::Span)> =
        std::collections::HashMap::new();
    for (i, decl) in program.fns.iter().enumerate() {
        // test functions are assertions the test verb consumes — their own
        // convention, not questions
        let short = decl.name.rsplit_once('/').map(|(_, s)| s).unwrap_or(&decl.name);
        // a getter takes its field's name, so this rule would reach past the
        // function and rename the field — a language question (should a
        // boolean field be `drained?`) that is not this check's to answer
        if decl.synthetic
            || decl.is_getter()
            || decl.name == crate::ast::ENTRY
            || short.starts_with("test_")
            // an arm extending an operator is named for the operator, and
            // `<` cannot take a `?` — this rule is about identifiers
            || matches!(short, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
        {
            continue;
        }
        let entry = groups.entry(decl.name.as_str()).or_insert((0, decl.span));
        entry.0 |= inference.returns[i];
    }
    // a HashMap hands these back in a different order every run, which makes
    // a multi-diagnostic file's output unstable; report in source order
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(name, (_, span))| (span.line, span.col, *name));
    for (name, (set, span)) in groups {
        let short = name.rsplit_once('/').map(|(_, s)| s).unwrap_or(name);
        let is_question = short.ends_with('?');
        // only err rides along (a predicate over fallible work is still a
        // predicate). none is not an answer a question may give — when the
        // static absence checker lands, a none-bearing `?` set becomes a
        // compile error; until then a none-polluted set is simply not
        // provably boolean and the unmarked direction stays quiet.
        let boolish = set != 0 && set & !(TRUE | FALSE | ERR) == 0 && set & (TRUE | FALSE) != 0;
        // the marked direction fires only on a provable lie: a `?` group
        // whose set cannot contain a boolean at all. generic drivers widen
        // honest predicates to TOP, and TOP still holds bool.
        if is_question && set != 0 && set & (TRUE | FALSE) == 0 {
            diags.push(Diagnostic::new(
                "naming",
                format!(
                    "`{short}` asks a question: a `?` function answers \
                     true or false (err may ride along)"
                ),
                span,
            ));
        }
        if !is_question && boolish {
            diags.push(Diagnostic::new(
                "naming",
                format!("`{short}` answers only true or false: name it `{short}?`"),
                span,
            ));
        }
    }
}

/// `x.name` where no record type anywhere declares `name` cannot succeed at
/// any runtime, so it is a compile error rather than a failure the program
/// has to reach. Reading a field a *particular* record lacks stays a runtime
/// error, since only the value knows its type.
fn check_field_exists(program: &Program, diags: &mut Vec<Diagnostic>) {
    let declared: HashSet<&str> =
        program.types.iter().flat_map(|t| t.fields.iter().map(|(f, _, _)| f.as_str())).collect();
    for decl in &program.fns {
        for stmt in &decl.body {
            field_reads(stmt, &declared, diags);
        }
    }
}

fn field_reads(stmt: &Stmt, declared: &HashSet<&str>, diags: &mut Vec<Diagnostic>) {
    let expr = match stmt {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
        Stmt::Set { value, .. } => value,
    };
    field_reads_expr(expr, declared, diags);
}

fn field_reads_expr(e: &Expr, declared: &HashSet<&str>, diags: &mut Vec<Diagnostic>) {
    if let Expr::Field { base, name, span } = e {
        if !declared.contains(name.as_str()) {
            diags.push(Diagnostic::new(
                "name",
                format!("no record type has a field `{name}`"),
                *span,
            ));
        }
        field_reads_expr(base, declared, diags);
        return;
    }
    for child in crate::expr_children(e) {
        field_reads_expr(child, declared, diags);
    }
}

pub fn check_merged(program: &Program, require_entry: bool) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Three checks read what inference knows, and inference over a whole
    // program is the most expensive thing the front end does. One pass,
    // handed round.
    let inference = crate::phase::watched("infer", || crate::infer::infer(program));
    check_constants(program, &mut diags);
    check_constant_cycles(program, &mut diags);
    check_predicates(program, &inference, &mut diags);
    check_arm_ties(program, &mut diags);
    check_build_blocks(program, &mut diags);
    check_sub_parents(program, &mut diags);
    check_none_in_collections(program, &mut diags);
    check_bare_ambiguity(program, &mut diags);
    check_call_arities(program, &mut diags);
    check_binding_patterns(program, &mut diags);
    check_overlapping_arms(program, &mut diags);
    check_field_exists(program, &mut diags);
    check_literal_arguments(program, &mut diags);
    check_effect_discarded(program, &inference, &mut diags);
    check_err_as_value(program, &mut diags);
    check_call_shaped_list(program, &mut diags);
    check_boolean_equality(program, &mut diags);
    if std::env::var("KANSO_EXHAUSTIVE").is_ok() {
        check_none_exhaustive(program, &inference, &mut diags);
    }
    if require_entry {
        check_entry(program, &mut diags);
    }
    diags
}

/// A call's argument count against the arms that could answer it. The
/// per-file pass sees only that file's declarations, so a wrong-arity call
/// to an *imported* group reached codegen unchecked and surfaced as an
/// undefined symbol from the assembler — compiler internals, where a
/// diagnostic belongs. This runs on the merged program, where the
/// dependency's arms are in scope.
/// A binding's pattern naming a type nobody declared. `poynt a = 1` passed
/// check and reached the emitter, which answered `native backend: unknown
/// type` — the backend's own words, for a name the reader typed. The
/// interpreter said something better and different, so the two engines
/// disagreed about a program neither should have accepted.
///
/// Parameter patterns were already covered: an arm taking an undeclared type
/// never matches, and arm selection says so.
fn check_binding_patterns(program: &Program, diags: &mut Vec<Diagnostic>) {
    let declared: HashSet<&str> = program.types.iter().map(|ty| ty.name.as_str()).collect();
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        for stmt in &decl.body {
            let Stmt::Bind { pattern: Pattern::Ctor { ty, fields }, .. } = stmt else {
                continue;
            };
            if declared.contains(ty.as_str()) || fields.is_empty() {
                continue;
            }
            // `let x = 1` reads as destructuring a `let`, which is how every
            // language that spells a binding with a keyword arrives here.
            let bound = param_names(&fields[0]).first().copied().unwrap_or_default();
            let instead = match ty.as_str() {
                "let" | "var" | "const" | "val" => {
                    format!(" — a binding is `{bound} = …`, with no keyword")
                }
                _ => String::new(),
            };
            diags.push(Diagnostic::new(
                "name",
                format!("`{ty}` is not a type, so a binding cannot destructure with it{instead}"),
                other_span(&fields[0]),
            ));
        }
    }
}

fn check_call_arities(program: &Program, diags: &mut Vec<Diagnostic>) {
    // Construction is positional and complete, and the same seam hid it: a
    // type declared in one file of a module and built in another was checked
    // by nobody, and a foreign type never was. A typeset does not construct
    // and a subtype takes the one value it wraps, so neither is entered.
    let fields: HashMap<&str, usize> = program
        .types
        .iter()
        .filter(|ty| ty.members.is_empty() && ty.parent.is_none() && !ty.fields.is_empty())
        .map(|ty| (ty.name.as_str(), ty.fields.len()))
        .collect();
    let mut arities: HashMap<&str, Vec<usize>> = HashMap::new();
    for decl in &program.fns {
        let slot = arities.entry(decl.name.as_str()).or_default();
        if !slot.contains(&decl.params.len()) {
            slot.push(decl.params.len());
        }
    }
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        let mut bound: HashSet<&str> = decl.params.iter().flat_map(param_names).collect();
        for stmt in &decl.body {
            bound_in_stmt(stmt, &mut bound);
        }
        for stmt in &decl.body {
            arity_walk_stmt(stmt, &arities, &fields, &bound, diags);
        }
    }
}

fn param_names(p: &Pattern) -> Vec<&str> {
    match p {
        Pattern::Var(n, _) => vec![n.as_str()],
        Pattern::Annotated { name, .. } => vec![name.as_str()],
        Pattern::Ctor { fields, .. } => fields.iter().flat_map(param_names).collect(),
        Pattern::Keyed { entries, .. } => entries.iter().map(|e| e.bind_name.as_str()).collect(),
        _ => Vec::new(),
    }
}

/// Every name this declaration binds anywhere in its body — its parameters,
/// its bindings, and the parameters of every lambda inside it. A binding may
/// not shadow a *function* declaration, so the call half never needed this;
/// a type's name is not covered by that rule, and `grown = (mark -> ...)`
/// beside std/list's two-field `grown` is a legal thing to write.
///
/// The whole body is gathered before the walk rather than as it goes: a name
/// bound after the line that uses it is still a name this code owns, and a
/// check that rejects programs should err towards saying nothing.
fn bound_in_stmt<'a>(stmt: &'a Stmt, out: &mut HashSet<&'a str>) {
    match stmt {
        Stmt::Bind { pattern, expr } => {
            out.extend(param_names(pattern));
            bound_in_expr(expr, out);
        }
        Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => bound_in_expr(expr, out),
    }
}

fn bound_in_expr<'a>(e: &'a Expr, out: &mut HashSet<&'a str>) {
    if let Expr::Lambda { params, .. } = e {
        out.extend(params.iter().map(|(n, _)| n.as_str()));
    }
    for child in crate::expr_children(e) {
        bound_in_expr(child, out);
    }
}

fn arity_walk_stmt(
    stmt: &Stmt,
    arities: &HashMap<&str, Vec<usize>>,
    fields: &HashMap<&str, usize>,
    bound: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
            arity_walk_expr(expr, arities, fields, bound, diags)
        }
    }
}

fn arity_walk_expr(
    e: &Expr,
    arities: &HashMap<&str, Vec<usize>>,
    fields: &HashMap<&str, usize>,
    bound: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(name, span) = &**head {
            // Bare names count too. No binding may shadow a declaration —
            // `adder = ...` beside `fn adder` is a name error — so a bare
            // name matching a declared group is that group, and the per-file
            // pass sees only one file of a module, which leaves every call
            // to a sibling file unchecked.
            let known = arities.get(name.as_str());
            if !bound.contains(name.as_str()) {
                if let Some(count) = fields.get(name.as_str()) {
                    if args.len() != *count {
                        diags.push(Diagnostic::new(
                            "arity",
                            format!(
                                "`{name}` has {count} field(s), got {} \
                                 (construction is positional, fields alphabetical)",
                                args.len()
                            ),
                            *span,
                        ));
                    }
                }
                if let Some(known) = known {
                    if !known.contains(&args.len()) && !known.contains(&0) {
                        let mut takes: Vec<String> = known.iter().map(|a| a.to_string()).collect();
                        takes.sort();
                        diags.push(Diagnostic::new(
                            "arity",
                            format!(
                                "no {}-argument arm of `{}` (arms take {})",
                                args.len(),
                                name,
                                takes.join(", ")
                            ),
                            *span,
                        ));
                    }
                }
            }
        }
    }
    for child in crate::expr_children(e) {
        arity_walk_expr(child, arities, fields, bound, diags);
    }
}

/// A literal argument no arm could ever take.
///
/// The general question — can this expression's type reach any arm — wants
/// the inference sets and their joins, and a join is a superset, so a wrong
/// call can hide behind a right one at another site. A literal needs none of
/// that: its type is on the page. So this asks the narrow question exactly,
/// and says nothing where an argument is anything else. What it catches is
/// the call that could never have worked, reported where the author wrote it
/// instead of when the value arrives.
fn check_literal_arguments(program: &Program, diags: &mut Vec<Diagnostic>) {
    let mut groups: HashMap<(&str, usize), Vec<&FnDecl>> = HashMap::new();
    for decl in &program.fns {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(decl);
    }
    let types: HashMap<&str, &TypeDecl> =
        program.types.iter().map(|t| (t.name.as_str(), t)).collect();
    let builtins = crate::inline::aliases(program);
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        let bound: HashSet<&str> = decl.params.iter().flat_map(param_names).collect();
        for stmt in &decl.body {
            literal_walk_stmt(stmt, &groups, &types, &bound, &builtins, diags);
        }
    }
}

/// What each builtin will accept in each position, verified by running the
/// wrong thing at each one and reading the diagnostic it gives. Absent
/// entries are unconstrained: this says nothing it has not checked.
fn builtin_demand(name: &str, index: usize) -> Option<&'static [LitKind]> {
    use LitKind::*;
    let table: &[(&str, &[&[LitKind]])] = &[
        ("length", &[&[Str, List, Map]]),
        ("entries", &[&[Map]]),
        ("bytes", &[&[Str]]),
        ("char_code", &[&[Str]]),
        ("chars", &[&[Str]]),
        ("split", &[&[Str], &[Str]]),
        ("sqrt", &[&[Int, Float]]),
        ("bit_and", &[&[Int], &[Int]]),
        ("bit_or", &[&[Int], &[Int]]),
        ("bit_xor", &[&[Int], &[Int]]),
        ("bit_not", &[&[Int]]),
        ("bit_shl", &[&[Int], &[Int]]),
        ("bit_shr", &[&[Int], &[Int]]),
        ("round", &[&[Int, Float]]),
        ("print", &[&[Str]]),
        ("write", &[&[Str]]),
        ("write_err", &[&[Str]]),
        ("env", &[&[Str]]),
        ("exists", &[&[Str]]),
        ("is_dir", &[&[Str]]),
        ("list_dir", &[&[Str]]),
        ("read_file", &[&[Str]]),
        ("run", &[&[Str], &[List]]),
        ("make_dir", &[&[Str]]),
        ("write_file", &[&[Str], &[Str]]),
        ("push", &[&[List]]),
        ("put", &[&[Map]]),
        ("sleep", &[&[Int]]),
        ("random", &[&[Int]]),
        ("from_code", &[&[Int]]),
    ];
    table.iter().find(|(n, _)| *n == name).and_then(|(_, positions)| positions.get(index)).copied()
}

/// What a literal is, coarsely enough to compare against a pattern.
#[derive(Clone, Copy, PartialEq)]
enum LitKind {
    Int,
    Float,
    Str,
    List,
    Map,
}

fn literal_kind(e: &Expr) -> Option<LitKind> {
    match e {
        Expr::Int(..) => Some(LitKind::Int),
        Expr::Float(..) => Some(LitKind::Float),
        Expr::Str(parts, _) => match parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))) {
            // an interpolation can render anything; only a plain string is a
            // literal for this purpose
            true => Some(LitKind::Str),
            false => None,
        },
        Expr::List(..) => Some(LitKind::List),
        Expr::MapLit(..) => Some(LitKind::Map),
        _ => None,
    }
}

/// Could this pattern accept a literal of this kind? Unsure counts as yes:
/// the check only fires where every arm is a definite no.
fn pattern_admits(p: &Pattern, kind: LitKind, types: &HashMap<&str, &TypeDecl>) -> bool {
    match p {
        Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => true,
        Pattern::IntLit(..) => kind == LitKind::Int,
        Pattern::StrLit(..) => kind == LitKind::Str,
        Pattern::Nullary(..) => false,
        Pattern::Ctor { .. } => false,
        Pattern::Annotated { ty, .. } => type_admits(ty, kind, types),
    }
}

fn type_admits(ty: &str, kind: LitKind, types: &HashMap<&str, &TypeDecl>) -> bool {
    if ty.ends_with("[]") {
        return kind == LitKind::List;
    }
    if ty.contains('[') {
        return kind == LitKind::Map;
    }
    match ty {
        "int" => kind == LitKind::Int,
        // dispatch does not widen: an int literal reaches no float64 arm,
        // whatever arithmetic does with the two together
        "float64" => kind == LitKind::Float,
        "string" => kind == LitKind::Str,
        "bool" | "none" | "err" => false,
        "some" | "any" => true,
        other => match types.get(other) {
            // a typeset admits what any member admits; a subtype admits what
            // its parent does; a record admits no literal at all
            Some(t) if !t.members.is_empty() => {
                t.members.iter().any(|m| type_admits(m, kind, types))
            }
            Some(t) => match &t.parent {
                Some(parent) => type_admits(parent, kind, types),
                None => false,
            },
            // an unknown name is not something to be confident about
            None => true,
        },
    }
}

fn literal_walk_stmt(
    stmt: &Stmt,
    groups: &HashMap<(&str, usize), Vec<&FnDecl>>,
    types: &HashMap<&str, &TypeDecl>,
    bound: &HashSet<&str>,
    builtins: &HashMap<(String, usize), String>,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
            literal_walk_expr(expr, groups, types, bound, builtins, diags)
        }
    }
}

fn literal_walk_expr(
    e: &Expr,
    groups: &HashMap<(&str, usize), Vec<&FnDecl>>,
    types: &HashMap<&str, &TypeDecl>,
    bound: &HashSet<&str>,
    builtins: &HashMap<(String, usize), String>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(name, _) = &**head {
            if !bound.contains(name.as_str()) {
                // a std wrapper is a rename over a builtin, so the builtin's
                // demand is the one that will actually be met
                let bare =
                    builtins.get(&(name.clone(), args.len())).cloned().unwrap_or_else(|| {
                        name.rsplit_once('/')
                            .map(|(_, n)| n.to_string())
                            .unwrap_or_else(|| name.clone())
                    });
                let bare = bare.strip_prefix("builtin_").unwrap_or(&bare).to_string();
                // A program that declares its own `run` calls its own `run`,
                // and the arms below say what those take. Without this, every
                // function sharing a bare name with a builtin would inherit
                // the builtin's demands.
                //
                // A std wrapper is a declaration too, and it forwards to the
                // builtin rather than replacing it — so treating it as a
                // shadow silenced the demand for every builtin somebody had
                // written a wrapper over. That is how `print 5` came to be
                // refused at compile time naming its argument while
                // `text/chars 5` answered a bare runtime string: not a
                // decision, an accident of which names have wrappers.
                let forwards = builtins.contains_key(&(name.clone(), args.len()));
                let shadowed = !forwards && groups.contains_key(&(name.as_str(), args.len()));
                for (i, arg) in args.iter().enumerate() {
                    let Some(kind) = literal_kind(arg) else { continue };
                    let Some(allowed) = builtin_demand(&bare, i).filter(|_| !shadowed) else {
                        continue;
                    };
                    if !allowed.contains(&kind) {
                        let want: Vec<String> =
                            allowed.iter().map(|k| describe_literal(*k).to_string()).collect();
                        diags.push(Diagnostic::new(
                            "type",
                            format!(
                                "`{bare}` takes {} here, not {}",
                                or_join(want),
                                describe_literal(kind)
                            ),
                            arg.span(),
                        ));
                    }
                }
                if let Some(arms) = groups.get(&(name.as_str(), args.len())) {
                    for (i, arg) in args.iter().enumerate() {
                        let Some(kind) = literal_kind(arg) else { continue };
                        let admitted = arms.iter().any(|a| {
                            a.params.get(i).is_some_and(|p| pattern_admits(p, kind, types))
                        });
                        if !admitted {
                            let wanted: Vec<String> = arms
                                .iter()
                                .filter_map(|a| a.params.get(i))
                                .map(describe_pattern)
                                .collect();
                            diags.push(Diagnostic::new(
                                "type",
                                format!(
                                    "no arm of `{name}` takes {} here (arms take {})",
                                    describe_literal(kind),
                                    dedup_join(wanted)
                                ),
                                arg.span(),
                            ));
                        }
                    }
                }
            }
        }
    }
    for child in crate::expr_children(e) {
        literal_walk_expr(child, groups, types, bound, builtins, diags);
    }
}

fn describe_literal(kind: LitKind) -> &'static str {
    match kind {
        LitKind::Int => "an int",
        LitKind::Float => "a float",
        LitKind::Str => "a string",
        LitKind::List => "a list",
        LitKind::Map => "a map",
    }
}

fn describe_pattern(p: &Pattern) -> String {
    match p {
        Pattern::IntLit(n, _) => n.to_string(),
        Pattern::StrLit(s, _) => format!("\"{s}\""),
        Pattern::Nullary(n, _) => n.clone(),
        Pattern::Ctor { ty, .. } => ty.clone(),
        Pattern::Annotated { ty, .. } => ty.clone(),
        Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => "any".to_string(),
    }
}

/// "a list, a map, or a string" — a demand reads as a choice, not a list.
fn or_join(mut names: Vec<String>) -> String {
    names.sort();
    names.dedup();
    match names.len() {
        0 | 1 => names.join(""),
        2 => names.join(" or "),
        n => format!("{}, or {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn dedup_join(mut names: Vec<String>) -> String {
    names.sort();
    names.dedup();
    names.join(", ")
}

/// Values born before the block stay immutable, which is what keeps every
/// cycle inside one birth cohort.
fn check_build_blocks(program: &Program, diags: &mut Vec<Diagnostic>) {
    let type_names: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    for decl in &program.fns {
        for stmt in &decl.body {
            build_walk_stmt(stmt, &type_names, diags);
        }
    }
}

fn build_walk_stmt(stmt: &Stmt, type_names: &HashSet<&str>, diags: &mut Vec<Diagnostic>) {
    // a `Set` reached here is one no `build` block enclosed: writing a field is
    // the one mutation the language has, and it lives in a build block only
    if let Stmt::Set { target, field, span, .. } = stmt {
        diags.push(Diagnostic::new(
            "build",
            format!(
                "`{target}.{field} = ...` writes a field, and only a `build` block may do that"
            ),
            *span,
        ));
    }
    match stmt {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
            build_walk_expr(expr, type_names, diags);
        }
    }
}

fn build_walk_expr(expr: &Expr, type_names: &HashSet<&str>, diags: &mut Vec<Diagnostic>) {
    if let Expr::Build(stmts, _) = expr {
        let mut born: HashSet<&str> = HashSet::new();
        for stmt in stmts {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    if let (Pattern::Var(name, _), true) = (pattern, constructs(expr, type_names)) {
                        born.insert(name);
                    }
                }
                Stmt::Set { target, field, span, .. } if !born.contains(target.as_str()) => {
                    diags.push(Diagnostic::new(
                        "build",
                        format!(
                            "`{target}.{field} = ...` writes only block-born values: \
                             `{target}` is not a construction made in this `build` \
                             block, so it stays immutable"
                        ),
                        *span,
                    ));
                }
                _ => {}
            }
        }
    }
    for child in crate::expr_children(expr) {
        build_walk_expr(child, type_names, diags);
    }
}

/// A call that merely returns a record may hand back something older.
fn constructs(expr: &Expr, type_names: &HashSet<&str>) -> bool {
    matches!(expr, Expr::App { head, .. }
        if matches!(&**head, Expr::Ident(name, _) if type_names.contains(name.as_str())))
}

fn collect_globals(program: &Program, diags: &mut Vec<Diagnostic>) -> HashSet<String> {
    let mut globals: HashSet<String> = AMBIENT.iter().map(|b| b.to_string()).collect();
    globals.extend(HARNESS.iter().map(|b| b.to_string()));
    globals.insert("entry".to_string());
    globals.insert("err".to_string());
    globals.insert("wrap_err".to_string());
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
    if let Some(first_fn_line) =
        program.fns.iter().filter(|f| !f.is_getter()).map(|f| f.span.line).min()
    {
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
    for ty in &program.types {
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

/// Declaration order is the author's. What survives here is the ranking rule,
/// which is about which arm dispatch picks rather than about where the arms sit.
fn check_fn_order(program: &Program, diags: &mut Vec<Diagnostic>) {
    check_overload_ranks(program, diags);
}

/// The constants this expression demands *while computing itself*, which is a
/// smaller set than the ones it mentions.
///
/// A constructor stores its arguments; it never looks at them. So a mention
/// inside one is not a demand — the value is put in a field and read later,
/// by which time the constant it names has a value. A call that scrutinizes,
/// an operator, a guard: those force what they are handed, and a mention
/// there is a demand.
///
/// That difference is the whole of the cycle rule. `x = x` asks for its own
/// answer to compute its own answer. `x = { "a":(node x) }` asks for a place
/// to put a name, which is a question the definition can answer.
fn demanded_refs<'a>(
    expr: &'a Expr,
    known: &HashSet<&str>,
    types: &HashSet<&str>,
    out: &mut Vec<&'a str>,
) {
    if let Expr::Ident(name, _) | Expr::Partial(name, _) = expr {
        if known.contains(name.as_str()) {
            out.push(name);
        }
    }
    // a list or map literal stores every element, so nothing inside is demanded
    if matches!(expr, Expr::List(..) | Expr::MapLit(..)) {
        return;
    }
    if let Expr::App { head, args, .. } = expr {
        if let Expr::Ident(name, _) = head.as_ref() {
            if types.contains(name.as_str()) {
                // a constructor's arguments are stored, so only the head is a
                // demand, and the head is a type rather than a constant
                let _ = args;
                return;
            }
        }
    }
    for child in crate::expr_children(expr) {
        demanded_refs(child, known, types, out);
    }
}

/// A constant defined in terms of itself has no value to compute: the
/// definition asks for the answer it is meant to produce. Undetected, it
/// recurses until the stack ends, which reports the machine's limit rather
/// than the program's mistake.
/// `any` was the old name for the type that accepts every value except
/// `none`, which is what it never said. An unannotated field or parameter is
/// the unconstrained one.
/// A field's type is what construction sites put in it and uses ask of it,
/// and the compiler works that out for itself — so writing it down again adds
/// a second copy that can disagree with the first. The editor shows the
/// inferred answer on hover, which is where a reader should learn it.
fn check_field_annotations(program: &Program, diags: &mut Vec<Diagnostic>) {
    for ty in &program.types {
        for (field, declared, span) in &ty.fields {
            if declared.is_empty() {
                continue;
            }
            diags.push(Diagnostic::new(
                "type",
                format!(
                    "a record field carries no type — write `{field}` and let the compiler \
                     infer what it holds"
                ),
                *span,
            ));
        }
    }
}

fn check_retired_any(program: &Program, diags: &mut Vec<Diagnostic>) {
    let retired = |ty: &str, span, diags: &mut Vec<Diagnostic>| {
        if ty == "any" {
            diags.push(Diagnostic::new(
                "type",
                "there is no `any` type; leave the annotation off for a field or \
                 parameter that accepts anything"
                    .to_string(),
                span,
            ));
        }
    };
    for ty in &program.types {
        for (_, declared, span) in &ty.fields {
            for name in declared {
                retired(name, *span, diags);
            }
        }
    }
    for decl in &program.fns {
        for param in &decl.params {
            if let Pattern::Annotated { ty, span, .. } = param {
                retired(ty, *span, diags);
            }
        }
    }
}

/// An annotation names a type, and a name nothing declares is a mistake the
/// reader cannot see: the arm simply never matches. The native backend already
/// refuses it, so the program did fail — but at build time, in a message about
/// a backend, rather than here in a message about the program.
///
/// A bracketed form is checked through its shape, not around it. Inference
/// reads only the shape — `ends_with("[]")` is a list, `contains('[')` is a map
/// — so the name inside was decoration nothing verified, and `[]banana`
/// compiled and ran while a bare `banana` was refused. A typo in an element
/// name should be as loud as a typo anywhere else.
fn check_annotation_names(program: &Program, diags: &mut Vec<Diagnostic>) {
    const BUILT_IN: [&str; 8] = ["int", "float64", "string", "bool", "none", "err", "some", "any"];
    let declared: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    for decl in &program.fns {
        for param in &decl.params {
            let Pattern::Annotated { ty, span, .. } = param else { continue };
            for name in annotation_names(ty) {
                // a qualified name is the import resolver's to answer for
                if name.contains('/') || BUILT_IN.contains(&name) || declared.contains(name) {
                    continue;
                }
                diags.push(Diagnostic::new(
                    "type",
                    format!("no type is called `{name}`, so this arm can never match"),
                    *span,
                ));
            }
        }
    }
}

/// The names an annotation mentions, whatever shape holds them. The parser
/// folds `[]T` to `T[]` and `map[K V]` to `map[K V]`, so both are read back
/// here rather than at each comparison — one place that knows the spelling.
fn annotation_names(ty: &str) -> Vec<&str> {
    if let Some(rest) = ty.strip_prefix("map[").and_then(|r| r.strip_suffix(']')) {
        return rest.split_whitespace().filter(|n| *n != "map").collect();
    }
    let inner = ty.trim_end_matches("[]");
    if inner.is_empty() {
        return Vec::new();
    }
    // `T[K]` is the older map spelling the lexer still folds to; both names
    // in it are types and both are checked.
    if let Some(open) = inner.find('[') {
        let (base, rest) = inner.split_at(open);
        let key = rest.trim_start_matches('[').trim_end_matches(']');
        return vec![base, key];
    }
    vec![inner]
}

fn check_constant_cycles(program: &Program, diags: &mut Vec<Diagnostic>) {
    let constants: Vec<&FnDecl> = program.fns.iter().filter(|d| d.params.is_empty()).collect();
    let names: HashSet<&str> = constants.iter().map(|d| d.name.as_str()).collect();
    let types: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    let mut refs: HashMap<&str, Vec<&str>> = HashMap::new();
    for decl in &constants {
        let out = refs.entry(decl.name.as_str()).or_default();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                Stmt::Set { value, .. } => value,
            };
            demanded_refs(expr, &names, &types, out);
        }
    }
    for decl in &constants {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack: Vec<&str> = refs.get(decl.name.as_str()).cloned().unwrap_or_default();
        while let Some(cur) = stack.pop() {
            if cur == decl.name {
                diags.push(Diagnostic::new(
                    "name",
                    format!("`{}` is defined in terms of itself, so it has no value", decl.name),
                    decl.span,
                ));
                break;
            }
            if seen.insert(cur) {
                stack.extend(refs.get(cur).into_iter().flatten());
            }
        }
    }
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

/// Two arms of one group with the same shape: the second can never be
/// reached, whichever file it is in. The ranking check below compares
/// neighbours, which is exactly right for arms written together and blind to a
/// module — a directory of files sharing one namespace, where the arms of a
/// group need not sit next to each other or even in the same file. So the
/// unreachable-arm half runs here, over the whole module at once.
///
/// Declarations are walked in order and each looks back, so what is reported
/// is the later arm and the order of the reports is the order of the source.
fn check_overlapping_arms(program: &Program, diags: &mut Vec<Diagnostic>) {
    let real: Vec<&FnDecl> = program.fns.iter().filter(|d| !d.synthetic).collect();
    for (i, later) in real.iter().enumerate() {
        let clashes = real[..i].iter().any(|earlier| {
            earlier.name == later.name
                && earlier.params.len() == later.params.len()
                && same_shape(&earlier.params, &later.params)
        });
        if clashes {
            diags.push(Diagnostic::new(
                "dispatch",
                format!("overlapping overloads of `{}` are illegal", later.name),
                later.span,
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

/// INTERIM committee ruling, awaiting Clay's tie gavel: a bare call that two
/// imports answer with the very same shape is refused at the call site,
/// because the silent alternative resolves the tie by import order — and
/// imports are formatted alphabetical, so the winner would be whichever
/// module's directory sorts first. Rejecting is the conservative reading: a
/// refused program can be given meaning later; a silently-resolved one is a
/// commitment. Qualified calls stay legal, and a local arm of the same shape
/// still shadows the imports, per the ruled precedence.
fn check_bare_ambiguity(program: &Program, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
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
    let mut torn: HashMap<(&str, usize), Vec<String>> = HashMap::new();
    for (i, a) in program.fns.iter().enumerate() {
        if !a.synthetic || a.name.contains('/') {
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
            if let Expr::Ident(name, span) = head.as_ref() {
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
        for child in crate::expr_children(e) {
            walk(child, torn, diags);
        }
    }
    for d in &program.fns {
        if d.synthetic || d.name.contains('/') {
            continue;
        }
        let bound: std::collections::HashSet<&str> = d
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
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    walk(expr, &live, diags)
                }
            }
        }
    }
}

fn same_shape(a: &[Pattern], b: &[Pattern]) -> bool {
    a.iter().zip(b.iter()).all(|(pa, pb)| match (pa, pb) {
        (Pattern::IntLit(x, _), Pattern::IntLit(y, _)) => x == y,
        (Pattern::StrLit(x, _), Pattern::StrLit(y, _)) => x == y,
        (Pattern::Nullary(x, _), Pattern::Nullary(y, _)) => x == y,
        (Pattern::Annotated { ty: x, .. }, Pattern::Annotated { ty: y, .. }) => x == y,
        (Pattern::Ctor { ty: x, fields: fa }, Pattern::Ctor { ty: y, fields: fb }) => {
            x == y && fa.len() == fb.len() && same_shape(fa, fb)
        }
        (Pattern::Var(..) | Pattern::Wildcard(..), Pattern::Var(..) | Pattern::Wildcard(..)) => {
            true
        }
        _ => false,
    })
}

/// A directory is run through the file `main.kso`, whose statements are
/// compiled in under a name no program can spell. What is missing here is
/// that file — never a declaration the reader was supposed to write called
/// `main`.
fn check_entry(program: &Program, diags: &mut Vec<Diagnostic>) {
    if !program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
        diags.push(Diagnostic::new(
            "name",
            "a directory runs through `main.kso`, and this module has none".to_string(),
            Span { line: 1, col: 1 },
        ));
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
    /// Arities of this module's own fn groups; an application of a local
    /// group must match one (arity 0 opts out: a constant's value may be
    /// callable, which only the runtime can arbitrate).
    fn_arities: &'a std::collections::HashMap<String, Vec<usize>>,
    /// How many fields each record type declares. Construction is positional,
    /// so an application of a type name has to hand over all of them.
    type_arity: &'a std::collections::HashMap<String, usize>,
    /// std-origin files (stamped `std/...` by the loader) may name internal
    /// builtins through the builtin_ prefix; nothing else may.
    std_origin: bool,
    /// A `_test.kso` file may name the harness surface; nothing else may.
    harness: bool,
}

fn check_fn_body_shadow(
    decl: &FnDecl,
    globals: &HashSet<String>,
    used_globals: &mut HashSet<String>,
    diags: &mut Vec<Diagnostic>,
    shadowable: &HashSet<String>,
    fn_arities: &std::collections::HashMap<String, Vec<usize>>,
    type_arity: &std::collections::HashMap<String, usize>,
) {
    let mut resolver = Resolver {
        globals,
        locals: Vec::new(),
        used_globals,
        diags: Vec::new(),
        std_origin: decl.file.starts_with("std/"),
        harness: harness_file(&decl.file),
        shadowable,
        fn_arities,
        type_arity,
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
            Stmt::Set { target, value, span, .. } => {
                resolver.resolve_name(target, *span);
                resolver.resolve_expr(value);
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
            // `_:type` ascribes without binding: there is no name to use
            if local.name == "_" {
                continue;
            }
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
            // `&f` reads f exactly as a bare mention does
            Expr::Partial(name, span) => self.resolve_name(name, *span),
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
                let from = self.locals.len();
                for stmt in stmts {
                    match stmt {
                        Stmt::Bind { pattern, expr } => {
                            self.resolve_expr(expr);
                            self.bind_pattern(pattern);
                        }
                        Stmt::Expr(expr) => self.resolve_expr(expr),
                        Stmt::Set { target, value, span, .. } => {
                            self.resolve_name(target, *span);
                            self.resolve_expr(value);
                        }
                    }
                }
                self.locals.truncate(from);
            }
            Expr::Field { base, .. } => self.resolve_expr(base),
            Expr::Upcast { expr, .. } => self.resolve_expr(expr),
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
                if let Expr::Ident(name, span) = &**head {
                    let local = self.locals.iter().any(|l| &l.name == name);
                    if !local {
                        if let Some(fields) = self.type_arity.get(name.as_str()) {
                            if args.len() != *fields {
                                self.diags.push(Diagnostic::new(
                                    "arity",
                                    format!(
                                        "`{name}` has {fields} field(s), got {} \
                                         (construction is positional, fields alphabetical)",
                                        args.len()
                                    ),
                                    *span,
                                ));
                            }
                        }
                        if let Some(arities) = self.fn_arities.get(name.as_str()) {
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
            Expr::Guard { cond, early, rest, .. } => {
                self.resolve_expr(cond);
                self.resolve_expr(early);
                let base = self.locals.len();
                for stmt in rest {
                    match stmt {
                        Stmt::Bind { pattern, expr } => {
                            self.resolve_expr(expr);
                            self.bind_pattern(pattern);
                        }
                        Stmt::Expr(expr) => self.resolve_expr(expr),
                        Stmt::Set { target, value, span, .. } => {
                            self.resolve_name(target, *span);
                            self.resolve_expr(value);
                        }
                    }
                }
                self.flush_unused(base);
            }
        }
    }

    fn resolve_name(&mut self, name: &str, span: Span) {
        if let Some(local) = self.locals.iter_mut().rev().find(|l| l.name == name) {
            local.used = true;
            return;
        }
        if HARNESS.contains(&name) && !self.harness {
            self.diags.push(Diagnostic::new(
                "name",
                format!(
                    "`{name}` reads a failure, which only a test may do — it belongs in a \
                     `_test.kso` file, because a package may not turn its own err into a value"
                ),
                span,
            ));
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
            false if name == "set" => {
                self.diags.push(Diagnostic::new(
                    "name",
                    "a field is written by assignment inside a `build` block: `target.field = value`"
                        .to_string(),
                    span,
                ));
            }
            false => {
                self.diags.push(Diagnostic::new("name", format!("unknown name `{name}`"), span));
            }
        }
    }
}
