//! Derivations (#2186): a framework's conventions, run while the program
//! compiles.
//!
//! An entry that writes `derive conventions/routes` has the compiler build a
//! `derive/program` describing the entry and the local modules it imports, call
//! `conventions/routes` with it on the interpreter, and read back a list of
//! declarations. A `derive/constant` becomes a constant in the entry, written
//! out as the literal its value spells, and a `derive/refusal` becomes a
//! compile error at the item it names. Nothing here is checked: the constants
//! join the entry before the merged check, which reads them as it reads code
//! someone wrote.
//!
//! The derivation's module was compiled on its own as one of the entry's
//! dependencies, so it cannot read the program it describes except through
//! the description. No executor is supplied, so a derivation that answers an
//! effect is refused rather than run.

use crate::ast::{
    Derive, Expr, FnDecl, Pattern, Program, Resolution, Stmt, TemplatePart, TypeDecl,
};
use crate::diag::{Diagnostic, Span};
use crate::eval::{Entries, Interp, MapKey, Value};
use crate::name::Name;
use std::cell::RefCell;

/// Each qualified name the entry's imports bring in, and whether it is pub.
pub type Exports = crate::hash::Map<String, bool>;
use std::rc::Rc;

thread_local! {
    /// What `kanso derived` prints: each constant as `name = value`, under
    /// the derivation that declared it. Empty unless that verb asked.
    static SHOWN: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Asks the next compile to keep what its derivations declare, for printing.
pub fn show() {
    SHOWN.with(|s| *s.borrow_mut() = Some(Vec::new()));
}

/// What the compile since `show` declared, one line per constant.
pub fn shown() -> Vec<String> {
    SHOWN.with(|s| s.borrow_mut().take().unwrap_or_default())
}

/// The module every derivation's vocabulary lives in. The compiler imports it
/// on the entry's behalf whenever the entry derives anything.
pub const MODULE: &str = "std/derive";

/// What the derivations an entry adds: its constants and record types, and
/// the arms they defined, each as the import path of the module it belongs
/// to and its source.
#[derive(Default)]
pub struct Derived {
    pub constants: Vec<FnDecl>,
    pub types: Vec<TypeDecl>,
    pub definitions: Vec<(String, String)>,
}

/// One declaration a derivation listed, as the entry will hold it.
enum Made {
    Constant(FnDecl),
    Type(TypeDecl),
    Definition(String, String),
}

/// What the derivations an entry names add to it, or the diagnostics a
/// refusal or a misbehaving derivation produced.
pub fn derive(
    entry: &Program,
    root: &str,
    deps: &Program,
    exports: &Exports,
) -> Result<Derived, Vec<Diagnostic>> {
    let mut diags = reads_around(entry, deps);
    if !diags.is_empty() {
        return Err(diags);
    }
    let interp = Interp::copying(deps);
    let description = describe(entry, root, deps, exports, &interp)?;
    let mut made = Derived::default();
    for d in &entry.derives {
        match run(&interp, deps, d, &description) {
            Ok(decls) => {
                for decl in decls {
                    match declared(&interp, entry, root, deps, d, &decl) {
                        Ok(Some(Made::Constant(c))) => made.constants.push(c),
                        Ok(Some(Made::Type(t))) => made.types.push(t),
                        Ok(Some(Made::Definition(path, source))) => {
                            made.definitions.push((path, source))
                        }
                        Ok(None) => {}
                        Err(diag) => diags.push(diag),
                    }
                }
            }
            Err(diag) => diags.push(diag),
        }
    }
    match diags.is_empty() {
        true => Ok(made),
        false => Err(diags),
    }
}

/// The entry, then the app's own modules it imports, each with its pub
/// functions, its pub constants and its record types. The modules a
/// derivation lives in are left out: they hold the convention, not the
/// program it describes.
fn describe(
    entry: &Program,
    root: &str,
    deps: &Program,
    exports: &Exports,
    interp: &Interp,
) -> Result<Value, Vec<Diagnostic>> {
    let mut modules = vec![entry_module(entry, root)];
    for name in described(entry) {
        modules.push(module(deps, &name, exports, interp).map_err(|d| vec![d])?);
    }
    Ok(record("derive/program", vec![list(modules)]))
}

/// The entry as a module, under its file's stem. It holds statements only and
/// declares nothing, so it lists only its calls, each from the caller "", and
/// a convention that checks calls checks the ones written in the entry too
/// (#2198).
fn entry_module(entry: &Program, root: &str) -> Value {
    let mut calls = Vec::new();
    for decl in entry.fns.iter().filter(|f| f.name == crate::ast::ENTRY) {
        calls_of(decl, root, "", &mut calls);
    }
    let fields = vec![
        text(root),
        list(Vec::new()),
        Value::Map(Rc::new(Entries::new())),
        list(Vec::new()),
        list(calls),
    ];
    record("derive/module", fields)
}

/// The modules a derivation is shown, by the qualifier the entry reaches each
/// under: the app's own, imported from beside or below the entry. A library
/// reached through `../` belongs to something else and is left out. The entry
/// reads these through its derivations, so their imports are in use whether
/// or not a statement names them.
pub fn described(entry: &Program) -> Vec<String> {
    let deriving: Vec<&str> =
        entry.derives.iter().filter_map(|d| d.name.split_once('/').map(|(m, _)| m)).collect();
    entry
        .imports
        .iter()
        .filter(|i| i.path.starts_with("./"))
        .map(|i| match &i.alias {
            Some(alias) => alias.to_string(),
            None => i.path.rsplit('/').next().unwrap_or_default().to_string(),
        })
        .filter(|name| !deriving.contains(&name.as_str()))
        .collect()
}

/// A derivation that names a module it describes, in its own module's code.
/// The description is the whole of what a derivation may know about the app,
/// so that its declarations follow from what `kanso derived` can show; a
/// convention that imported a model or a controller would read the app around
/// it. Each such name is refused where it is written.
fn reads_around(entry: &Program, deps: &Program) -> Vec<Diagnostic> {
    let described: Vec<String> = described(entry).iter().map(|m| format!("{m}/")).collect();
    let mut diags = Vec::new();
    for d in &entry.derives {
        let Some((home, _)) = d.name.split_once('/') else { continue };
        let own = format!("{home}/");
        let files: Vec<&std::sync::Arc<str>> = deps
            .fns
            .iter()
            .filter(|f| !f.synthetic && f.name.starts_with(&own))
            .map(|f| &f.file)
            .collect();
        for f in deps.fns.iter().filter(|f| !f.synthetic && files.contains(&&f.file)) {
            let mut named = Vec::new();
            for stmt in &f.body {
                let e = match stmt {
                    Stmt::Bind { expr, .. } => expr,
                    Stmt::Expr(e) => e,
                };
                names_in(e, &mut named);
            }
            for (name, span) in named {
                let Some(module) = described.iter().find(|m| name.starts_with(m.as_str())) else {
                    continue;
                };
                let msg = format!(
                    "`{name}` is in `{}`, which `{}` describes; a derivation \
                     reads the app only through the program it is handed",
                    module.trim_end_matches('/'),
                    d.name
                );
                diags.push(Diagnostic::new("derive", msg, span).about(&f.file));
            }
        }
    }
    diags
}

fn names_in(e: &Expr, named: &mut Vec<(String, Span)>) {
    if let Expr::Ident(name, span, _) = e {
        named.push((name.to_string(), *span));
    }
    crate::for_each_child(e, |child| names_in(child, named));
}

fn module(
    deps: &Program,
    name: &str,
    exports: &Exports,
    interp: &Interp,
) -> Result<Value, Diagnostic> {
    // Whether a name reaches the entry as pub. A declaration's own flag is
    // demoted when it arrives through another module that does not re-export
    // it, and that route can load first, so the entry's exports decide.
    let open = |n: &str| exports.get(n).copied().unwrap_or(false);
    let prefix = format!("{name}/");
    let mut seen: Vec<&str> = Vec::new();
    let mut functions = Vec::new();
    let mut constants = Entries::new();
    for decl in &deps.fns {
        let Some(bare) = decl.name.strip_prefix(&prefix) else { continue };
        if !open(&decl.name) || decl.synthetic || bare.contains('/') || seen.contains(&bare) {
            continue;
        }
        if crate::ast::getter_field(bare).is_some() || bare.starts_with(char::is_uppercase) {
            continue;
        }
        seen.push(bare);
        // A constant is handed over as its value, which is how a derivation
        // reads data the app keeps, such as a list of migrations.
        if decl.params.is_empty() {
            let reference = Value::FnRef(Rc::from(decl.name.as_str()));
            let value = interp
                .call_value(reference, Vec::new(), decl.span)
                .and_then(|v| interp.demand(&v))
                .map_err(|e| {
                    let msg = format!("`{}` could not be read: {}", decl.name, e.message);
                    Diagnostic::new("derive", msg, decl.span).about(&decl.file)
                })?;
            constants.insert(MapKey::Str(bare.to_string().into()), value);
            continue;
        }
        let params = decl.params.iter().map(param).collect();
        let reference = Value::FnRef(Rc::from(decl.name.as_str()));
        functions.push(record("derive/function", vec![text(bare), list(params), reference]));
    }
    let records = deps
        .types
        .iter()
        .filter(|t| open(&t.name) && !t.synthetic && t.parent.is_none() && t.members.is_empty())
        .filter_map(|t| t.name.strip_prefix(&prefix).map(|bare| (bare, t)))
        .filter(|(bare, _)| !bare.contains('/'))
        .map(|(bare, t)| {
            let fields = t.fields.iter().map(|(f, _, _)| text(f)).collect();
            record("derive/record", vec![text(bare), list(fields)])
        })
        .collect();
    let constants = Value::Map(Rc::new(constants));
    let calls = calls_in(deps, name);
    let fields = vec![text(name), list(functions), constants, list(records), list(calls)];
    Ok(record("derive/module", fields))
}

/// Every call a function of module `name` makes to a named function, as
/// `derive/call` records, in the order the module declares and writes them.
fn calls_in(deps: &Program, name: &str) -> Vec<Value> {
    let prefix = format!("{name}/");
    let mut calls = Vec::new();
    for decl in deps.fns.iter().filter(|f| !f.synthetic) {
        let Some(caller) = decl.name.strip_prefix(&prefix) else { continue };
        if caller.contains('/') || crate::ast::getter_field(caller).is_some() {
            continue;
        }
        calls_of(decl, name, caller, &mut calls);
    }
    calls
}

fn calls_of(decl: &FnDecl, module: &str, caller: &str, calls: &mut Vec<Value>) {
    for stmt in &decl.body {
        let e = match stmt {
            Stmt::Bind { expr, .. } => expr,
            Stmt::Expr(e) => e,
        };
        calls_from(e, module, caller, calls);
    }
}

fn calls_from(e: &Expr, module: &str, caller: &str, calls: &mut Vec<Value>) {
    if let Expr::App { head, args, span, .. } = e {
        if let Expr::Ident(callee, _, _) = &**head {
            let named = |a: &Expr| match a {
                Expr::Ident(n, _, _) => text(n.as_str()),
                _ => text(""),
            };
            // A string written whole, with nothing interpolated, is a name a
            // convention can check: `path_to "posts/show" id` (#2198).
            let written = |a: &Expr| match a {
                Expr::Str(parts, _) => parts
                    .iter()
                    .map(|p| match p {
                        crate::ast::TemplatePart::Lit(s) => Some(s.as_str()),
                        crate::ast::TemplatePart::Interp(_) => None,
                    })
                    .collect::<Option<String>>()
                    .map_or_else(|| text(""), |s| text(&s)),
                _ => text(""),
            };
            let fields = vec![
                text(module),
                text(caller),
                text(callee.as_str()),
                list(args.iter().map(named).collect()),
                list(args.iter().map(written).collect()),
                Value::Int((span.line as i64).into()),
                Value::Int((span.col as i64).into()),
            ];
            calls.push(record("derive/call", fields));
        }
    }
    crate::for_each_child(e, |child| calls_from(child, module, caller, calls));
}

fn param(p: &Pattern) -> Value {
    let (name, annotation) = match p {
        Pattern::Var(n, _) => (n.as_str(), ""),
        Pattern::Annotated { name, ty, .. } => (name.as_str(), ty.as_str()),
        _ => ("_", ""),
    };
    record("derive/param", vec![text(name), text(annotation)])
}

fn record(ty: &str, fields: Vec<Value>) -> Value {
    Value::Record { ty: Rc::from(ty), fields: Rc::new(RefCell::new(fields)) }
}

fn list(items: Vec<Value>) -> Value {
    Value::List(Rc::new(items))
}

fn text(s: &str) -> Value {
    Value::Str(s.to_string().into())
}

/// Calls the derivation and answers the declarations it listed.
fn run(
    interp: &Interp,
    deps: &Program,
    d: &Derive,
    description: &Value,
) -> Result<Vec<Value>, Diagnostic> {
    let known = deps.fns.iter().any(|f| f.name == d.name && f.params.len() == 1);
    if !known {
        let msg = format!(
            "`derive {}` names no function an import declares with one parameter, \
             the program it is handed",
            d.name
        );
        return Err(Diagnostic::new("derive", msg, d.span));
    }
    let callee = Value::FnRef(Rc::from(d.name.as_str()));
    let answered = interp
        .call_value(callee, vec![description.clone()], d.span)
        .and_then(|v| interp.demand(&v))
        .map_err(|e| failed(d, &e.message))?;
    let items = match answered {
        Value::List(items) => items,
        Value::Desc(_) => {
            let msg = format!(
                "`{}` answered an effect; a derivation runs while the program \
                 compiles, so it may only compute",
                d.name
            );
            return Err(Diagnostic::new("derive", msg, d.span));
        }
        Value::ErrV(e) | Value::Unboxed(e) => {
            return Err(failed(d, &crate::eval::render(interp, &e.reason, false)));
        }
        other => {
            let msg = format!(
                "`{}` answered {}; a derivation answers a list of declarations \
                 (make a lazy sequence a list with `list/to_list`)",
                d.name,
                crate::eval::render(interp, &other, true)
            );
            return Err(Diagnostic::new("derive", msg, d.span));
        }
    };
    // An err in the list is the derivation failing partway, and says so with
    // its reason rather than as a declaration it does not recognise.
    let answer = |v: &Value| match interp.demand(v) {
        Ok(Value::ErrV(e) | Value::Unboxed(e)) => {
            Err(failed(d, &crate::eval::render(interp, &e.reason, false)))
        }
        Ok(v) => Ok(v),
        Err(e) => Err(failed(d, &e.message)),
    };
    items.iter().map(answer).collect()
}

fn failed(d: &Derive, reason: &str) -> Diagnostic {
    Diagnostic::new("derive", format!("`{}` failed: {reason}", d.name), d.span)
}

/// One declaration: a constant or a record type for the entry, nothing for a
/// refusal that became a diagnostic, or a diagnostic for anything else.
fn declared(
    interp: &Interp,
    entry: &Program,
    root: &str,
    deps: &Program,
    d: &Derive,
    decl: &Value,
) -> Result<Option<Made>, Diagnostic> {
    let kinds = [
        "derive/constant",
        "derive/type_declaration",
        "derive/definition",
        "derive/refusal",
        "derive/call_refusal",
    ];
    let fields = match decl {
        Value::Record { ty, fields } if kinds.contains(&&**ty) => fields.borrow().clone(),
        other => {
            let msg = format!(
                "`{}` listed {}, which is not a declaration; a derivation lists \
                 `derive/constant`, `derive/type_declaration` and `derive/refusal` \
                 records",
                d.name,
                crate::eval::render(interp, other, true)
            );
            return Err(Diagnostic::new("derive", msg, d.span));
        }
    };
    let origin = plain_text(interp, &fields[0]);
    if let Value::Record { ty, .. } = decl {
        if &**ty == "derive/refusal" {
            let message = plain_text(interp, &fields[1]);
            return Err(refused_at(deps, &origin, message, d.span));
        }
        if &**ty == "derive/call_refusal" {
            return Err(refused_at_call(interp, entry, root, deps, d, &fields));
        }
        if &**ty == "derive/type_declaration" {
            return declared_type(interp, d, &fields).map(|t| Some(Made::Type(t)));
        }
        if &**ty == "derive/definition" {
            return defined(interp, entry, d, &origin, &fields).map(Some);
        }
    }
    let name = plain_text(interp, &fields[1]);
    SHOWN.with(|s| {
        if let Some(lines) = s.borrow_mut().as_mut() {
            lines.push(format!("{name} = {}    # {}", spelled(interp, &fields[2]), d.name));
        }
    });
    let value = written(interp, &fields[2], d.span).map_err(|what| {
        let msg = format!(
            "`{}` declared `{name}` holding {what}, which a program cannot write \
             down as a literal",
            d.name
        );
        Diagnostic::new("derive", msg, d.span)
    })?;
    Ok(Some(Made::Constant(FnDecl {
        name,
        params: Vec::new(),
        body: vec![Stmt::Expr(value)],
        span: d.span,
        is_pub: false,
        written_pub: false,
        file: crate::ast::unstamped(),
        synthetic: false,
        shades: None,
    })))
}

/// An arm a derivation defined, for the module its origin names. The module
/// has to be one the derivation was shown, because it compiles the arm as one
/// of its own files, and an arm there may claim the module's own types.
fn defined(
    interp: &Interp,
    entry: &Program,
    d: &Derive,
    origin: &str,
    fields: &[Value],
) -> Result<Made, Diagnostic> {
    let module = origin.split_once('/').map_or(origin, |(m, _)| m);
    let import = entry.imports.iter().filter(|i| i.path.starts_with("./")).find(|i| {
        let qual = match &i.alias {
            Some(alias) => alias.as_str(),
            None => i.path.rsplit('/').next().unwrap_or_default(),
        };
        qual == module
    });
    let Some(import) = import.filter(|_| described(entry).iter().any(|m| m == module)) else {
        let msg = format!(
            "`{}` defined an arm for `{origin}`, and `{module}` is not one of the \
             modules it describes",
            d.name
        );
        return Err(Diagnostic::new("derive", msg, d.span));
    };
    let source = plain_text(interp, &fields[1]);
    SHOWN.with(|s| {
        if let Some(lines) = s.borrow_mut().as_mut() {
            lines.push(format!("# in {module}, by {}", d.name));
            lines.extend(source.lines().map(str::to_string));
        }
    });
    Ok(Made::Definition(import.path.clone(), source))
}

/// A refusal at one call a description listed, in the file of the function
/// that makes it.
fn refused_at_call(
    interp: &Interp,
    entry: &Program,
    root: &str,
    deps: &Program,
    d: &Derive,
    fields: &[Value],
) -> Diagnostic {
    let message = plain_text(interp, &fields[1]);
    let call = match interp.demand(&fields[0]) {
        Ok(Value::Record { ty, fields }) if &*ty == "derive/call" => fields.borrow().clone(),
        _ => return Diagnostic::new("derive", message, d.span),
    };
    let number = |v: &Value| match interp.demand(v) {
        Ok(Value::Int(n)) => n.to_string().parse::<u32>().ok(),
        _ => None,
    };
    let (module, caller) = (plain_text(interp, &call[0]), plain_text(interp, &call[1]));
    let at = number(&call[5]).zip(number(&call[6]));
    let made = match module == root && caller.is_empty() {
        true => entry.fns.iter().find(|f| f.name == crate::ast::ENTRY),
        false => deps.fns.iter().find(|f| f.name == format!("{module}/{caller}")),
    };
    match (made, at) {
        (Some(f), Some((line, col))) => {
            Diagnostic::new("derive", message, Span { line, col }).about(&f.file)
        }
        _ => Diagnostic::new("derive", message, d.span),
    }
}

/// A record type the entry declares, named and with fields as a programmer
/// could have written them.
fn declared_type(interp: &Interp, d: &Derive, fields: &[Value]) -> Result<TypeDecl, Diagnostic> {
    let name = plain_text(interp, &fields[1]);
    let listed = match interp.demand(&fields[2]) {
        Ok(Value::List(items)) => items,
        _ => {
            let msg =
                format!("`{}` declared type `{name}` with fields that are not a list", d.name);
            return Err(Diagnostic::new("derive", msg, d.span));
        }
    };
    let names: Vec<String> = listed.iter().map(|f| plain_text(interp, f)).collect();
    if let Some(bad) = std::iter::once(&name).chain(&names).find(|n| !a_name(n)) {
        let msg = format!(
            "`{}` declared type `{name}` naming `{bad}`, which a program could not \
             write as a name",
            d.name
        );
        return Err(Diagnostic::new("derive", msg, d.span));
    }
    SHOWN.with(|s| {
        if let Some(lines) = s.borrow_mut().as_mut() {
            lines.push(format!("type {name} {}    # {}", names.join(" "), d.name));
        }
    });
    Ok(TypeDecl {
        name,
        is_pub: false,
        span: d.span,
        synthetic: false,
        origin: None,
        parent: None,
        members: Vec::new(),
        fields: names.into_iter().map(|n| (n, Vec::new(), d.span)).collect(),
        file: crate::ast::unstamped(),
    })
}

/// Whether `n` is spelled as a declared name: a lowercase letter or an
/// underscore, then lowercase letters, digits and underscores.
fn a_name(n: &str) -> bool {
    let mut chars = n.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn plain_text(interp: &Interp, v: &Value) -> String {
    match v {
        Value::Str(t) => t.to_string(),
        other => crate::eval::render(interp, other, false),
    }
}

/// A refusal lands on the function or record its origin names, in the file
/// that declares it; one that names nothing lands on the `derive` line.
fn refused_at(deps: &Program, origin: &str, message: String, fallback: Span) -> Diagnostic {
    if let Some(f) = deps.fns.iter().find(|f| f.name == origin) {
        return Diagnostic::new("derive", message, f.span).about(&f.file);
    }
    if let Some(t) = deps.types.iter().find(|t| t.name == origin) {
        return Diagnostic::new("derive", message, t.span).about(&t.file);
    }
    Diagnostic::new("derive", message, fallback)
}

/// A value as the literal that builds it, or the name of what has no literal.
fn written(interp: &Interp, v: &Value, span: Span) -> Result<Expr, String> {
    let ident = |n: &str| Expr::Ident(Name::from(n), span, Resolution::default());
    let v = interp.demand(v).map_err(|e| e.message)?;
    Ok(match &v {
        Value::Int(n) => Expr::Int(n.big().into_owned(), span),
        Value::Float(f) => Expr::Float(*f, span),
        Value::Str(t) => Expr::Str(vec![TemplatePart::Lit(t.to_string())], span),
        Value::True => ident("true"),
        Value::False => ident("false"),
        Value::NoneV => ident("none"),
        Value::Done => ident("done"),
        Value::FnRef(name) => ident(name),
        Value::List(items) => Expr::List(
            items.iter().map(|x| written(interp, x, span)).collect::<Result<_, _>>()?,
            span,
        ),
        Value::Map(entries) => Expr::MapLit(pairs(interp, entries, span)?, span),
        Value::Record { ty, fields } => Expr::App {
            piped: false,
            head: Box::new(ident(ty)),
            args: fields
                .borrow()
                .iter()
                .map(|x| written(interp, x, span))
                .collect::<Result<_, _>>()?,
            span,
        },
        Value::Sub { ty, inner } => Expr::App {
            piped: false,
            head: Box::new(ident(ty)),
            args: vec![written(interp, inner, span)?],
            span,
        },
        other => return Err(crate::eval::render(interp, other, true)),
    })
}

fn pairs(interp: &Interp, entries: &Entries, span: Span) -> Result<Vec<(Expr, Expr)>, String> {
    let key = |k: &MapKey| match k {
        MapKey::Int(n) => Expr::Int(n.big().into_owned(), span),
        MapKey::Str(t) => Expr::Str(vec![TemplatePart::Lit(t.to_string())], span),
    };
    let each: Vec<(&MapKey, &Value)> = match entries {
        Entries::Few(few) => few.iter().map(|(k, v)| (k, v)).collect(),
        Entries::Many(many) => many.iter().collect(),
    };
    each.into_iter().map(|(k, v)| Ok((key(k), written(interp, v, span)?))).collect()
}

/// A declared value as `kanso derived` prints it: as the interpreter renders
/// it, except that a function is spelled by its name, which is the point of
/// a route table.
fn spelled(interp: &Interp, v: &Value) -> String {
    let v = interp.demand(v).unwrap_or_else(|_| v.clone());
    match &v {
        Value::FnRef(name) => name.to_string(),
        Value::List(items) => {
            let each: Vec<String> = items.iter().map(|x| spelled(interp, x)).collect();
            format!("[{}]", each.join(" "))
        }
        Value::Map(entries) => {
            let each: Vec<(&MapKey, &Value)> = match &**entries {
                Entries::Few(few) => few.iter().map(|(k, v)| (k, v)).collect(),
                Entries::Many(many) => many.iter().collect(),
            };
            let each: Vec<String> = each
                .into_iter()
                .map(|(k, x)| {
                    let key = match k {
                        MapKey::Int(n) => n.to_string(),
                        MapKey::Str(t) => format!("{:?}", t.to_string()),
                    };
                    format!("{key}:{}", spelled(interp, x))
                })
                .collect();
            format!("{{ {} }}", each.join(" "))
        }
        Value::Record { ty, fields } => {
            let each: Vec<String> = fields.borrow().iter().map(|x| spelled(interp, x)).collect();
            format!("({ty} {})", each.join(" "))
        }
        other => crate::eval::render(interp, other, true),
    }
}
