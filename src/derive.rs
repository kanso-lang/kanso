//! Derivations (#2186): a framework's conventions, run while the program
//! compiles.
//!
//! An entry that writes `derive conventions/routes` has the compiler build a
//! `derive/program` describing the local modules the entry imports, call
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

use crate::ast::{Derive, Expr, FnDecl, Pattern, Program, Resolution, Stmt, TemplatePart};
use crate::diag::{Diagnostic, Span};
use crate::eval::{Entries, Interp, MapKey, Value};
use crate::name::Name;
use std::cell::RefCell;
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

/// What the derivations an entry names add to it: constants for the entry,
/// or the diagnostics a refusal or a misbehaving derivation produced.
pub fn derive(entry: &Program, deps: &Program) -> Result<Vec<FnDecl>, Vec<Diagnostic>> {
    let description = describe(entry, deps);
    let interp = Interp::new(deps);
    let mut made = Vec::new();
    let mut diags = Vec::new();
    for d in &entry.derives {
        match run(&interp, deps, d, &description) {
            Ok(decls) => {
                for decl in decls {
                    match declared(&interp, deps, d, &decl) {
                        Ok(Some(constant)) => made.push(constant),
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

/// The app's own modules the entry imports, each with its pub functions and its
/// record types. The modules a derivation lives in are left out: they hold
/// the convention, not the program it describes.
fn describe(entry: &Program, deps: &Program) -> Value {
    let modules = described(entry).iter().map(|name| module(deps, name)).collect();
    record("derive/program", vec![list(modules)])
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

fn module(deps: &Program, name: &str) -> Value {
    let prefix = format!("{name}/");
    let mut seen: Vec<&str> = Vec::new();
    let mut functions = Vec::new();
    for decl in &deps.fns {
        let Some(bare) = decl.name.strip_prefix(&prefix) else { continue };
        if !decl.is_pub || decl.synthetic || bare.contains('/') || seen.contains(&bare) {
            continue;
        }
        if crate::ast::getter_field(bare).is_some() || bare.starts_with(char::is_uppercase) {
            continue;
        }
        seen.push(bare);
        let params = decl.params.iter().map(param).collect();
        let reference = Value::FnRef(Rc::from(decl.name.as_str()));
        functions.push(record("derive/function", vec![text(bare), list(params), reference]));
    }
    let records = deps
        .types
        .iter()
        .filter(|t| t.is_pub && !t.synthetic && t.parent.is_none() && t.members.is_empty())
        .filter_map(|t| t.name.strip_prefix(&prefix).map(|bare| (bare, t)))
        .filter(|(bare, _)| !bare.contains('/'))
        .map(|(bare, t)| {
            let fields = t.fields.iter().map(|(f, _, _)| text(f)).collect();
            record("derive/record", vec![text(bare), list(fields)])
        })
        .collect();
    record("derive/module", vec![text(name), list(functions), list(records)])
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
    items.iter().map(|v| interp.demand(v).map_err(|e| failed(d, &e.message))).collect()
}

fn failed(d: &Derive, reason: &str) -> Diagnostic {
    Diagnostic::new("derive", format!("`{}` failed: {reason}", d.name), d.span)
}

/// One declaration: a constant for the entry, nothing for a refusal that
/// became a diagnostic, or a diagnostic for anything else.
fn declared(
    interp: &Interp,
    deps: &Program,
    d: &Derive,
    decl: &Value,
) -> Result<Option<FnDecl>, Diagnostic> {
    let fields = match decl {
        Value::Record { ty, fields } if matches!(&**ty, "derive/constant" | "derive/refusal") => {
            fields.borrow().clone()
        }
        other => {
            let msg = format!(
                "`{}` listed {}, which is not a declaration; a derivation lists \
                 `derive/constant` and `derive/refusal` records",
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
    Ok(Some(FnDecl {
        name,
        params: Vec::new(),
        body: vec![Stmt::Expr(value)],
        span: d.span,
        is_pub: false,
        written_pub: false,
        file: crate::ast::unstamped(),
        synthetic: false,
        shades: None,
    }))
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
