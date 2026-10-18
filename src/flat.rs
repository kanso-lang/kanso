//! A span that only passes through is never built.
//!
//! The json decoder scans a string with `text/span_to`, hands the span to a
//! group that reads its fields and slices it, and drops it. Built as a record,
//! every string the decoder meets cost an allocation, and the slice of a span
//! reached none of the fused doors, because the emitter sees `text/slice run`
//! where it knows `slice cs a b`. Together they doubled the decode.
//!
//! So a record whose life is construction, a pass through some groups, and
//! field reads is carried as its fields instead. Three things are rewritten:
//!
//! - A *producer* is a one-arm group whose body is at least one guard
//!   answering `none` and then a construction, the shape of `text/span_to`.
//! - A *carrier* is a position of a group whose every arm there takes `none`,
//!   `_`, or a name for the record. Each name is read only by getters of the
//!   record's fields, or handed to another carrier. The group gains a twin
//!   taking the fields at that position, one parameter each; `none` arrives
//!   as `none` in the first and a zero in each of the others.
//! - A call handing a producer's answer straight to a carrier becomes a call
//!   to a twin of the producer, which takes the carrier's other arguments too
//!   and calls the carrier's twin where it answered `none` or the record.
//!
//! The twins keep the file and spans of the declaration they copy, and their
//! names carry `Flat{n}_` before the bare name, which `ast::spoken` takes
//! off, so a failure inside one names the function the author wrote at the
//! line they wrote it on. The producer's twin calls the carrier's from the
//! producer's file, so a carrier is entered that way only when its arms
//! cannot miss, and a miss never names the producer's line.
//!
//! The rewrite is a source-to-source one before any engine runs, so every
//! engine runs the same program and the differential law holds by
//! construction.
use crate::ast::{Expr, FnDecl, Pattern, Program, Resolution, Stmt};
use crate::hash::Map as HashMap;
use crate::name::Name;

/// The mark on a twin's bare name, before its number.
pub const FLAT_MARK: &str = "Flat";

/// The bare name a twin was copied from, when `bare` is a twin's.
pub fn twin_of(bare: &str) -> Option<&str> {
    let rest = bare.strip_prefix(FLAT_MARK)?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    rest[digits..].strip_prefix('_')
}

/// Whether the pass made the group: a twin's bare name carries the mark.
pub fn made(name: &str) -> bool {
    twin_of(crate::ast::split_qual(name).map_or(name, |(_, bare)| bare)).is_some()
}

struct Record {
    fields: Vec<String>,
}

struct Producer {
    ty: String,
    at: usize,
}

#[derive(Clone)]
struct Carrier {
    at: usize,
    ty: String,
}

type Group = (String, usize);

pub fn flatten(program: &mut Program) {
    let records = records(program);
    if records.is_empty() {
        return;
    }
    let producers = producers(program, &records);
    if producers.is_empty() {
        return;
    }
    let carriers = carriers(program, &records, &producers);
    if carriers.is_empty() {
        return;
    }
    // Twins are numbered in declaration order, so a build names them the
    // same way every time.
    let mut order: Vec<Group> = Vec::new();
    for f in &program.fns {
        let key = (f.name.clone(), f.params.len());
        if carriers.contains_key(&key) && !order.contains(&key) {
            order.push(key);
        }
    }
    let mut count = 0usize;
    let mut twins: HashMap<Group, String> = HashMap::default();
    for key in &order {
        count += 1;
        twins.insert(key.clone(), twin_name(&key.0, count));
    }
    let mut made = Vec::new();
    for key in &order {
        let carrier = &carriers[key];
        let fields = &records[&carrier.ty].fields;
        for f in program.fns.iter().filter(|f| f.name == key.0 && f.params.len() == key.1) {
            made.push(carrier_twin(f, &twins[key], carrier, fields, &carriers, &twins, &records));
        }
    }
    program.fns.extend(made);

    // A call handing a producer's answer to a carrier that cannot miss.
    let mut built: Vec<((String, usize, String), String)> = Vec::new();
    let mut extra = Vec::new();
    for i in 0..program.fns.len() {
        let mut body = std::mem::take(&mut program.fns[i].body);
        for stmt in &mut body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            entered(
                expr, program, &producers, &carriers, &twins, &records, &mut built, &mut extra,
                &mut count,
            );
        }
        program.fns[i].body = body;
    }
    program.fns.extend(extra);
}

/// `{owner}/Flat{n}_{bare}`.
fn twin_name(name: &str, n: usize) -> String {
    match crate::ast::last_slash(name) {
        Some(at) => format!("{}/{FLAT_MARK}{n}_{}", &name[..at], &name[at + 1..]),
        None => format!("{FLAT_MARK}{n}_{name}"),
    }
}

/// Every record type with fields, keyed by each of its spellings: an
/// enrollment clone constructs and matches as its origin, so both name the
/// one record and both resolve to the origin's name.
fn records(program: &Program) -> HashMap<String, Record> {
    let mut out = HashMap::default();
    for t in &program.types {
        if t.fields.is_empty() || t.origin.is_some() {
            continue;
        }
        out.insert(
            t.name.clone(),
            Record { fields: t.fields.iter().map(|f| f.0.clone()).collect() },
        );
    }
    out
}

/// The record a type's spelling names, by its origin's name.
fn canonical<'a>(program: &'a Program, ty: &'a str) -> &'a str {
    program.types.iter().find(|t| t.name == ty).and_then(|t| t.origin.as_deref()).unwrap_or(ty)
}

/// One-arm groups of plain names whose body is one or more guards answering
/// `none` and then one construction of a record, every field given.
fn producers(program: &Program, records: &HashMap<String, Record>) -> HashMap<Group, Producer> {
    let mut arms: HashMap<Group, usize> = HashMap::default();
    for f in &program.fns {
        *arms.entry((f.name.clone(), f.params.len())).or_default() += 1;
    }
    let mut out = HashMap::default();
    for (at, f) in program.fns.iter().enumerate() {
        let key = (f.name.clone(), f.params.len());
        if arms[&key] != 1 || f.synthetic || f.shades.is_some() || f.is_getter() {
            continue;
        }
        if !f.params.iter().all(|p| matches!(p, Pattern::Var(..))) {
            continue;
        }
        let [Stmt::Expr(body)] = f.body.as_slice() else { continue };
        // A bare construction is left to the escape analysis, which hands a
        // record back by value where it can, and to the arena's rewinds,
        // which a loop half carried as fields and half as records defeats.
        if !matches!(body, Expr::Guard { .. }) {
            continue;
        }
        let Some(ty) = built_after_guards(program, body, records) else { continue };
        out.insert(key, Producer { ty, at });
    }
    out
}

/// The record a producer's body builds, when the body has that shape.
fn built_after_guards(
    program: &Program,
    e: &Expr,
    records: &HashMap<String, Record>,
) -> Option<String> {
    match e {
        Expr::Guard { early, rest, .. } => {
            if !is_none(early) {
                return None;
            }
            let [Stmt::Expr(next)] = rest.as_slice() else { return None };
            built_after_guards(program, next, records)
        }
        Expr::App { head, args, piped: false, .. } => {
            let Expr::Ident(ty, _, _) = head.as_ref() else { return None };
            let ty = canonical(program, ty.as_str());
            let record = records.get(ty)?;
            (record.fields.len() == args.len()).then(|| ty.to_string())
        }
        _ => None,
    }
}

fn is_none(e: &Expr) -> bool {
    matches!(e, Expr::Ident(n, _, _) if n.as_str() == "none")
}

/// A call to a producer: its name and arity.
fn producer_call<'a>(
    e: &'a Expr,
    producers: &HashMap<Group, Producer>,
) -> Option<(&'a str, &'a [Expr])> {
    let Expr::App { head, args, piped: false, .. } = e else { return None };
    let Expr::Ident(name, _, _) = head.as_ref() else { return None };
    producers
        .contains_key(&(name.to_string(), args.len()))
        .then_some((name.as_str(), args.as_slice()))
}

/// The groups a call can name: declared, and not one the compiler treats
/// as more than its arms.
fn eligible(program: &Program, name: &str, arity: usize) -> bool {
    let mut any = false;
    for f in program.fns.iter().filter(|f| f.name == name && f.params.len() == arity) {
        if f.synthetic || f.shades.is_some() || f.is_getter() || crate::is_operator(&f.name) {
            return false;
        }
        any = true;
    }
    any && !name.contains(crate::ast::BARE_MARK)
}

/// The carriers: positions found from the calls handed a producer's answer
/// and, through the names arms bind there, from the calls those names are
/// handed to. Then every position whose arms do anything else with the
/// record is dropped, until none is.
fn carriers(
    program: &Program,
    records: &HashMap<String, Record>,
    producers: &HashMap<Group, Producer>,
) -> HashMap<Group, Carrier> {
    // A group with two positions found, or one found under two records, is
    // left alone: a twin flattens one position.
    let mut found: HashMap<Group, Option<Carrier>> = HashMap::default();
    let mut work: Vec<Group> = Vec::new();
    for f in &program.fns {
        each_stmt_expr(&f.body, &mut |e| {
            visit(e, &mut |e| {
                let Expr::App { head, args, piped: false, .. } = e else { return };
                let Expr::Ident(name, _, _) = head.as_ref() else { return };
                for (at, a) in args.iter().enumerate() {
                    if let Some((p, pargs)) = producer_call(a, producers) {
                        let ty = producers[&(p.to_string(), pargs.len())].ty.clone();
                        if eligible(program, name, args.len()) {
                            note(
                                (name.to_string(), args.len()),
                                Carrier { at, ty },
                                &mut found,
                                &mut work,
                            );
                        }
                    }
                }
            })
        });
    }
    while let Some(key) = work.pop() {
        let Some(Some(carrier)) = found.get(&key).cloned() else { continue };
        for f in program.fns.iter().filter(|f| f.name == key.0 && f.params.len() == key.1) {
            let Some(v) = f.params.get(carrier.at).and_then(binder) else { continue };
            each_stmt_expr(&f.body, &mut |e| {
                visit(e, &mut |e| {
                    let Expr::App { head, args, piped: false, .. } = e else { return };
                    let Expr::Ident(name, _, _) = head.as_ref() else { return };
                    for (at, a) in args.iter().enumerate() {
                        if is_named(a, v) && eligible(program, name, args.len()) {
                            let c = Carrier { at, ty: carrier.ty.clone() };
                            note((name.to_string(), args.len()), c, &mut found, &mut work);
                        }
                    }
                })
            });
        }
    }
    let mut kept: HashMap<Group, Carrier> =
        found.into_iter().filter_map(|(k, c)| c.map(|c| (k, c))).collect();
    loop {
        let dropped: Vec<Group> = kept
            .iter()
            .filter(|(key, carrier)| !carries(program, key, carrier, records, &kept))
            .map(|(key, _)| key.clone())
            .collect();
        if dropped.is_empty() {
            return kept;
        }
        for key in dropped {
            kept.remove(&key);
        }
    }
}

/// Records a position found, or marks the group left alone when it was
/// found before at another position or under another record.
fn note(
    key: Group,
    carrier: Carrier,
    found: &mut HashMap<Group, Option<Carrier>>,
    work: &mut Vec<Group>,
) {
    match found.get(&key) {
        None => {
            found.insert(key.clone(), Some(carrier));
            work.push(key);
        }
        Some(Some(seen)) if seen.at != carrier.at || seen.ty != carrier.ty => {
            found.insert(key, None);
        }
        _ => {}
    }
}

/// The name an arm binds at a position, when it binds one.
fn binder(p: &Pattern) -> Option<&str> {
    match p {
        Pattern::Var(n, _) => Some(n.as_str()),
        Pattern::Annotated { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

fn is_named(e: &Expr, v: &str) -> bool {
    matches!(e, Expr::Ident(n, _, _) if n.as_str() == v)
}

/// Whether every arm of the group takes the record at the position and does
/// nothing with it but read its fields and hand it to another carrier.
fn carries(
    program: &Program,
    key: &Group,
    carrier: &Carrier,
    records: &HashMap<String, Record>,
    kept: &HashMap<Group, Carrier>,
) -> bool {
    let fields = &records[&carrier.ty].fields;
    program.fns.iter().filter(|f| f.name == key.0 && f.params.len() == key.1).all(|f| {
        match &f.params[carrier.at] {
            Pattern::Nullary(n, _) if n.as_str() == "none" => true,
            Pattern::Wildcard(_) => true,
            Pattern::Var(v, _) => only_read(f, v.as_str(), fields, &carrier.ty, kept),
            Pattern::Annotated { name, ty, .. } => {
                canonical(program, ty.as_str()) == carrier.ty
                    && only_read(f, name.as_str(), fields, &carrier.ty, kept)
            }
            _ => false,
        }
    })
}

/// Whether `v` is read in `f` only by its fields' getters and as an argument
/// to a carrier of the same record, and is bound nowhere else in the arm.
fn only_read(
    f: &FnDecl,
    v: &str,
    fields: &[String],
    ty: &str,
    kept: &HashMap<Group, Carrier>,
) -> bool {
    let mut ok = true;
    // another parameter of the same name would make `v` ambiguous
    let mut seen = 0;
    for p in &f.params {
        p.each_name(&mut |n| seen += usize::from(n == v));
    }
    if seen != 1 || rebinds(&f.body, v) {
        return false;
    }
    each_stmt_expr(&f.body, &mut |e| ok &= reads_only(e, v, fields, ty, kept));
    ok
}

fn reads_only(
    e: &Expr,
    v: &str,
    fields: &[String],
    ty: &str,
    kept: &HashMap<Group, Carrier>,
) -> bool {
    match e {
        Expr::Ident(n, _, _) | Expr::Partial(n, _) => n.as_str() != v,
        Expr::App { head, args, piped: false, .. } => {
            if let Expr::Ident(name, _, _) = head.as_ref() {
                if let ([arg], Some(field)) = (args.as_slice(), crate::ast::getter_field(name)) {
                    if is_named(arg, v) && fields.iter().any(|f| f == field) {
                        return true;
                    }
                }
                let handed = args.iter().filter(|a| is_named(a, v)).count();
                if handed > 0 {
                    let key = (name.to_string(), args.len());
                    let to_carrier = handed == 1
                        && kept.get(&key).is_some_and(|c| c.ty == ty && is_named(&args[c.at], v));
                    if !to_carrier {
                        return false;
                    }
                    return args
                        .iter()
                        .filter(|a| !is_named(a, v))
                        .all(|a| reads_only(a, v, fields, ty, kept));
                }
            }
            let mut ok = reads_only(head, v, fields, ty, kept);
            for a in args {
                ok &= reads_only(a, v, fields, ty, kept);
            }
            ok
        }
        Expr::Lambda { params, body, .. } => {
            !params.iter().any(|(p, _)| p == v) && reads_only(body, v, fields, ty, kept)
        }
        _ => !crate::any_child(e, |c| !reads_only(c, v, fields, ty, kept)),
    }
}

/// Whether a statement anywhere in the body binds `v` again.
fn rebinds(stmts: &[Stmt], v: &str) -> bool {
    stmts.iter().any(|st| match st {
        Stmt::Bind { pattern, expr } => {
            let mut hit = false;
            pattern.each_name(&mut |n| hit |= n == v);
            hit || rebinds_in(expr, v)
        }
        Stmt::Expr(expr) => rebinds_in(expr, v),
    })
}

fn rebinds_in(e: &Expr, v: &str) -> bool {
    match e {
        Expr::Block(stmts, _) => rebinds(stmts, v),
        Expr::Guard { cond, early, rest, .. } => {
            rebinds_in(cond, v) || rebinds_in(early, v) || rebinds(rest.as_slice(), v)
        }
        _ => crate::any_child(e, |c| rebinds_in(c, v)),
    }
}

/// Every top-level expression of a body, the bound ones included.
fn each_stmt_expr<'a>(stmts: &'a [Stmt], f: &mut impl FnMut(&'a Expr)) {
    for st in stmts {
        match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => f(expr),
        }
    }
}

/// `e` and everything below it.
fn visit<'a>(e: &'a Expr, f: &mut impl FnMut(&'a Expr)) {
    f(e);
    crate::for_each_child(e, |c| visit(c, f));
}

/// The name a field of `v` is carried under in a twin. Source names are
/// lowercase, so no program can spell it.
fn field_name(v: &str, field: &str) -> String {
    format!("Flat_{v}_{field}")
}

fn ident(name: &str, span: crate::diag::Span) -> Expr {
    Expr::Ident(Name::new(name), span, Resolution::default())
}

/// A carrier's arm, taking the record's fields where it took the record.
fn carrier_twin(
    f: &FnDecl,
    name: &str,
    carrier: &Carrier,
    fields: &[String],
    carriers: &HashMap<Group, Carrier>,
    twins: &HashMap<Group, String>,
    records: &HashMap<String, Record>,
) -> FnDecl {
    let mut twin = f.clone();
    twin.name = name.to_string();
    let at = carrier.at;
    let taken = twin.params.remove(at);
    let spread: Vec<Pattern> = match &taken {
        Pattern::Nullary(_, span) => {
            let mut out = vec![taken.clone()];
            out.extend((1..fields.len()).map(|_| Pattern::Wildcard(*span)));
            out
        }
        Pattern::Wildcard(span) => (0..fields.len()).map(|_| Pattern::Wildcard(*span)).collect(),
        Pattern::Var(v, span) | Pattern::Annotated { name: v, span, .. } => {
            let v = v.to_string();
            for st in &mut twin.body {
                let expr = match st {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                };
                carried(expr, &v, carriers, twins, records);
            }
            fields
                .iter()
                .map(|field| Pattern::Var(Name::new(&field_name(&v, field)), *span))
                .collect()
        }
        _ => unreachable!("a carrier takes none, `_` or a name there"),
    };
    for (i, p) in spread.into_iter().enumerate() {
        twin.params.insert(at + i, p);
    }
    twin
}

/// Inside a twin's arm: a field read of `v` becomes the field's name, and
/// `v` handed to a carrier becomes its fields handed to that carrier's twin.
fn carried(
    e: &mut Expr,
    v: &str,
    carriers: &HashMap<Group, Carrier>,
    twins: &HashMap<Group, String>,
    records: &HashMap<String, Record>,
) {
    if let Expr::App { head, args, span, piped: false } = e {
        if let Expr::Ident(name, _, _) = head.as_ref() {
            if let ([arg], Some(field)) = (args.as_slice(), crate::ast::getter_field(name)) {
                if is_named(arg, v) {
                    *e = ident(&field_name(v, field), *span);
                    return;
                }
            }
            let key = (name.to_string(), args.len());
            if let (Some(c), Some(twin)) = (carriers.get(&key), twins.get(&key)) {
                if is_named(&args[c.at], v) {
                    let at_span = match &args[c.at] {
                        Expr::Ident(_, s, _) => *s,
                        _ => *span,
                    };
                    let spread: Vec<Expr> = records[&c.ty]
                        .fields
                        .iter()
                        .map(|f| ident(&field_name(v, f), at_span))
                        .collect();
                    args.splice(c.at..=c.at, spread);
                    **head = ident(twin, head_span(head));
                }
            }
        }
    }
    crate::for_each_child_mut(e, &mut |c| carried(c, v, carriers, twins, records));
}

fn head_span(head: &Expr) -> crate::diag::Span {
    match head {
        Expr::Ident(_, s, _) => *s,
        _ => crate::diag::Span::at(0, 0),
    }
}

/// A call `g .. (p ..) ..` where `p` is a producer and `g` a carrier at that
/// position whose arms cannot miss becomes a call to the producer's twin for
/// that carrier, handed the producer's arguments and then `g`'s others.
#[allow(clippy::too_many_arguments)]
fn entered(
    e: &mut Expr,
    program: &Program,
    producers: &HashMap<Group, Producer>,
    carriers: &HashMap<Group, Carrier>,
    twins: &HashMap<Group, String>,
    records: &HashMap<String, Record>,
    built: &mut Vec<((String, usize, String), String)>,
    extra: &mut Vec<FnDecl>,
    count: &mut usize,
) {
    crate::for_each_child_mut(e, &mut |c| {
        entered(c, program, producers, carriers, twins, records, built, extra, count)
    });
    let Expr::App { head, args, piped: false, .. } = e else { return };
    let Expr::Ident(name, head_at, _) = head.as_ref() else { return };
    let key = (name.to_string(), args.len());
    let (Some(carrier), Some(twin)) = (carriers.get(&key), twins.get(&key)) else { return };
    let Some((p, pargs)) = producer_call(&args[carrier.at], producers) else { return };
    let producer = &producers[&(p.to_string(), pargs.len())];
    if producer.ty != carrier.ty || !total(program, &key, carrier.at) {
        return;
    }
    let pkey = (p.to_string(), pargs.len(), twin.clone());
    let entry = match built.iter().find(|(k, _)| *k == pkey) {
        Some((_, n)) => n.clone(),
        None => {
            *count += 1;
            let decl = &program.fns[producer.at];
            let n = twin_name(&decl.name, *count);
            let fields = records[&producer.ty].fields.len();
            extra.push(producer_twin(decl, &n, twin, carrier.at, key.1, fields));
            built.push((pkey, n.clone()));
            n
        }
    };
    let head_at = *head_at;
    let Expr::App { args: inner, .. } = args.remove(carrier.at) else { unreachable!() };
    let mut handed = inner;
    handed.append(args);
    *args = handed;
    **head = ident(&entry, head_at);
}

/// Whether a carrier always reaches an arm when handed the record or
/// `none`: every other position takes anything, and some arm takes each.
fn total(program: &Program, key: &Group, at: usize) -> bool {
    let arms: Vec<&FnDecl> =
        program.fns.iter().filter(|f| f.name == key.0 && f.params.len() == key.1).collect();
    let anything = |p: &Pattern| matches!(p, Pattern::Var(..) | Pattern::Wildcard(_));
    let others =
        arms.iter().all(|f| f.params.iter().enumerate().all(|(i, p)| i == at || anything(p)));
    let takes_none = arms.iter().any(|f| {
        anything(&f.params[at])
            || matches!(&f.params[at], Pattern::Nullary(n, _) if n.as_str() == "none")
    });
    let takes_record = arms.iter().any(|f| {
        matches!(&f.params[at], Pattern::Var(..) | Pattern::Wildcard(_) | Pattern::Annotated { .. })
    });
    others && takes_none && takes_record
}

/// The producer, copied to answer by calling a carrier's twin: its own
/// parameters and then the carrier's other arguments, with `none` handed on
/// in the first field and the record as its fields.
fn producer_twin(
    decl: &FnDecl,
    name: &str,
    carrier: &str,
    at: usize,
    arity: usize,
    fields: usize,
) -> FnDecl {
    let mut twin = decl.clone();
    twin.name = name.to_string();
    twin.is_pub = false;
    twin.written_pub = false;
    let others: Vec<String> = (0..arity - 1).map(|i| format!("Flat_arg{i}")).collect();
    for o in &others {
        twin.params.push(Pattern::Var(Name::new(o), decl.span));
    }
    let call = |mut spread: Vec<Expr>, span: crate::diag::Span| {
        let mut args: Vec<Expr> = others.iter().map(|o| ident(o, span)).collect();
        let tail = args.split_off(at);
        args.append(&mut spread);
        args.extend(tail);
        Expr::App { piped: false, head: Box::new(ident(carrier, span)), args, span }
    };
    if let [Stmt::Expr(body)] = twin.body.as_mut_slice() {
        answered(body, &call, fields);
    }
    twin
}

/// A guard becomes an `if`: the emitter and the escape analysis read a
/// conditional in tail position as two tails, and a carrier answering a
/// by-value record through the twin keeps that record by value.
fn answered(e: &mut Expr, call: &impl Fn(Vec<Expr>, crate::diag::Span) -> Expr, fields: usize) {
    match e {
        Expr::Guard { cond, early, rest, span } => {
            let at = match early.as_ref() {
                Expr::Ident(_, s, _) => *s,
                _ => *span,
            };
            // `none` rides in the first field and a zero in each of the
            // rest: the none arm reads only the first, and a field that only
            // ever holds an int is one the emitter can pass unboxed.
            let mut spread = vec![ident("none", at)];
            spread.extend((1..fields).map(|_| Expr::Int(0.into(), at)));
            let taken = call(spread, at);
            let [Stmt::Expr(next)] = rest.as_mut_slice() else {
                unreachable!("a producer's guard is followed by one expression")
            };
            answered(next, call, fields);
            let next = std::mem::replace(next, ident("none", at));
            let cond = std::mem::replace(cond.as_mut(), ident("none", at));
            *e = Expr::App {
                piped: false,
                head: Box::new(ident("if", *span)),
                args: vec![cond, taken, next],
                span: *span,
            };
        }
        Expr::App { args, span, .. } => {
            let spread = std::mem::take(args);
            *e = call(spread, *span);
        }
        _ => unreachable!("a producer's body is guards and then a construction"),
    }
}
