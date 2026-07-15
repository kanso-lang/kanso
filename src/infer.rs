use crate::ast::*;
use std::collections::HashMap;

/// Propagable type sets as tag bitsets — the single monotone inference
/// fixpoint (the story is told in about.html part 03), coarse to start:
/// one bit per runtime tag, records unrefined.
pub type Set = u16;

pub const INT: Set = 1 << 0;
pub const FLOAT: Set = 1 << 1;
pub const TRUE: Set = 1 << 2;
pub const FALSE: Set = 1 << 3;
pub const NONE: Set = 1 << 4;
pub const ERR: Set = 1 << 5;
pub const STR: Set = 1 << 6;
pub const REC: Set = 1 << 7;
pub const DESC: Set = 1 << 8;
pub const LIST: Set = 1 << 9;
pub const MAP: Set = 1 << 10;
pub const FN: Set = 1 << 11;
pub const BYTES: Set = 1 << 12;
pub const TOP: Set = (1 << 13) - 1;
pub const FAIL: Set = NONE | ERR;
pub const BOOL: Set = TRUE | FALSE;

pub struct Inference {
    /// per fn-decl index: joined argument sets seen at call sites
    pub params: Vec<Vec<Set>>,
    /// per fn-decl index: return set
    pub returns: Vec<Set>,
    /// per type index, per field: joined set seen at construction sites
    pub type_fields: Vec<Vec<Set>>,
}

struct Ctx<'a> {
    program: &'a Program,
    groups: HashMap<(&'a str, usize), Vec<usize>>,
    type_names: HashMap<&'a str, usize>,
    params: Vec<Vec<Set>>,
    returns: Vec<Set>,
    type_fields: Vec<Vec<Set>>,
    changed: bool,
}

pub fn infer(program: &Program) -> Inference {
    let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::new();
    for (i, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(i);
    }
    let type_names = program.types.iter().enumerate().map(|(i, t)| (t.name.as_str(), i)).collect();
    let mut ctx = Ctx {
        program,
        groups,
        type_names,
        params: program.fns.iter().map(|d| vec![0; d.params.len()]).collect(),
        returns: vec![0; program.fns.len()],
        type_fields: program.types.iter().map(|t| vec![0; t.fields.len()]).collect(),
        changed: true,
    };
    // seed: entry points (main, constants, tests) run with no arguments;
    // anything used as a function value gets TOP params.
    let mut rounds = 0;
    while ctx.changed && rounds < 200 {
        ctx.changed = false;
        rounds += 1;
        for i in 0..ctx.program.fns.len() {
            let decl = &ctx.program.fns[i];
            let mut env: HashMap<&str, Set> = HashMap::new();
            let param_sets = ctx.params[i].clone();
            for (pattern, joined) in decl.params.iter().zip(&param_sets) {
                bind_pattern(pattern, *joined, &ctx.type_fields, &ctx.type_names, &mut env);
            }
            let ret = eval_body(&mut ctx, &decl.body, &mut env);
            if ret | ctx.returns[i] != ctx.returns[i] {
                ctx.returns[i] |= ret;
                ctx.changed = true;
            }
        }
    }
    Inference { params: ctx.params, returns: ctx.returns, type_fields: ctx.type_fields }
}

fn bind_pattern<'a>(
    pattern: &'a Pattern,
    joined: Set,
    type_fields: &[Vec<Set>],
    type_names: &HashMap<&'a str, usize>,
    env: &mut HashMap<&'a str, Set>,
) {
    match pattern {
        // generics never bind failures
        Pattern::Var(name, _) => {
            env.insert(name, joined & !FAIL);
        }
        Pattern::Wildcard(_) | Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => {}
        Pattern::Annotated { name, ty, .. } => {
            let set = match ty.as_str() {
                "int" => INT,
                "float64" => FLOAT,
                "string" => STR,
                "bool" => BOOL,
                "err" => ERR,
                t if t.ends_with("[]") => LIST,
                t if t.contains('[') => MAP,
                _ => REC,
            };
            env.insert(name, set);
        }
        // destructuring a declared type refines each field to the join of what
        // construction sites stored there — so `_parsed p v` gives p its real
        // int-ness instead of TOP, which is what unblocks the scanner's hot path
        Pattern::Ctor { ty, fields } => {
            let field_sets = type_names.get(ty.as_str()).map(|i| &type_fields[*i]);
            for (fi, field) in fields.iter().enumerate() {
                let s = field_sets.and_then(|fs| fs.get(fi)).copied().unwrap_or(TOP & !FAIL);
                bind_pattern(field, s, type_fields, type_names, env);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for entry in entries {
                env.insert(&entry.bind_name, TOP & !FAIL);
            }
        }
    }
}

fn pattern_catches(pat: &Pattern) -> Set {
    match pat {
        Pattern::Nullary(name, _) if name == "none" => NONE,
        Pattern::Ctor { ty, .. } if ty == "err" => ERR,
        _ => 0,
    }
}

fn eval_body<'a>(ctx: &mut Ctx<'a>, body: &'a [Stmt], env: &mut HashMap<&'a str, Set>) -> Set {
    let mut result = NONE;
    for stmt in body {
        match stmt {
            Stmt::Bind { pattern, expr } => {
                let value = eval_expr(ctx, expr, env);
                match pattern {
                    Pattern::Var(name, _) => {
                        env.insert(name, value);
                    }
                    _ => bind_pattern(pattern, value, &ctx.type_fields, &ctx.type_names, env),
                }
            }
            Stmt::Expr(expr) => result = eval_expr(ctx, expr, env),
        }
    }
    result
}

fn eval_expr<'a>(ctx: &mut Ctx<'a>, expr: &'a Expr, env: &mut HashMap<&'a str, Set>) -> Set {
    match expr {
        Expr::Int(..) => INT,
        Expr::Float(..) => FLOAT,
        Expr::Str(parts, _) => {
            let mut fails: Set = 0;
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    fails |= eval_expr(ctx, inner, env) & FAIL;
                }
            }
            STR | fails
        }
        Expr::Ident(name, _) => ident_set(ctx, name, env),
        Expr::List(items, _) => {
            for item in items {
                let _ = eval_expr(ctx, item, env);
            }
            LIST
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                let _ = eval_expr(ctx, k, env);
                let _ = eval_expr(ctx, v, env);
            }
            MAP
        }
        Expr::Index { base, index, .. } => {
            let b = eval_expr(ctx, base, env);
            let k = eval_expr(ctx, index, env);
            let mut out = (b & FAIL) | (k & FAIL) | ERR; // strict: miss is err
            if b & BYTES != 0 {
                out |= INT;
            }
            if b & (LIST | MAP | STR) != 0 {
                out |= TOP & !FAIL;
            }
            out
        }
        Expr::Seq(l, r, _) => {
            let a = eval_expr(ctx, l, env);
            let b = eval_expr(ctx, r, env);
            DESC | (a & FAIL) | (b & FAIL)
        }
        Expr::Lambda { body, params, .. } => {
            let mut inner = env.clone();
            for (p, _) in params {
                inner.insert(p, TOP & !FAIL);
            }
            let _ = eval_expr(ctx, body, &mut inner);
            FN
        }
        Expr::BinOp { op, lhs, rhs, .. } => {
            let a = eval_expr(ctx, lhs, env);
            let b = eval_expr(ctx, rhs, env);
            let fails = (a & FAIL) | (b & FAIL);
            match *op {
                "+" | "-" | "*" => {
                    let mut out = fails;
                    if a & INT != 0 && b & INT != 0 {
                        out |= INT;
                    }
                    if a & FLOAT != 0 && b & FLOAT != 0 {
                        out |= FLOAT;
                    }
                    out
                }
                "/" => {
                    let mut out = fails | ERR;
                    if a & INT != 0 && b & INT != 0 {
                        out |= INT;
                    }
                    if a & FLOAT != 0 && b & FLOAT != 0 {
                        out |= FLOAT;
                    }
                    out
                }
                // the join yields a description, a lone propagated failure, or
                // an accumulated err merged from both sides
                "&" => DESC | fails | ERR,
                _ => BOOL | fails,
            }
        }
        Expr::App { head, args, piped, .. } => eval_call(ctx, head, args, env, *piped),
    }
}

fn ident_set<'a>(ctx: &mut Ctx<'a>, name: &'a str, env: &mut HashMap<&'a str, Set>) -> Set {
    if let Some(set) = env.get(name) {
        return *set;
    }
    match name {
        "true" => TRUE,
        "false" => FALSE,
        "none" => NONE,
        "args" | "stdin" => DESC,
        _ => {
            // a zero-field type's bare mention is its marker value
            if let Some(i) = ctx.type_names.get(name) {
                if ctx.program.types[*i].fields.is_empty() {
                    return REC;
                }
            }
            // constant mention evaluates; fn mention is a value (params go TOP)
            if let Some(decls) = ctx.groups.get(&(name, 0)) {
                let i = decls[0];
                return ctx.returns[i];
            }
            let arities: Vec<usize> = ctx
                .program
                .fns
                .iter()
                .filter(|d| d.name == name)
                .map(|d| d.params.len())
                .collect();
            for (i, decl) in ctx.program.fns.iter().enumerate() {
                if decl.name == name {
                    for p in 0..decl.params.len() {
                        widen_param(ctx, i, p, TOP);
                    }
                }
            }
            let _ = arities;
            FN
        }
    }
}

fn widen_param(ctx: &mut Ctx<'_>, decl: usize, param: usize, set: Set) {
    if ctx.params[decl][param] | set != ctx.params[decl][param] {
        ctx.params[decl][param] |= set;
        ctx.changed = true;
    }
}

fn eval_call<'a>(
    ctx: &mut Ctx<'a>,
    head: &'a Expr,
    args: &'a [Expr],
    env: &mut HashMap<&'a str, Set>,
    piped: bool,
) -> Set {
    let mut arg_sets: Vec<Set> = args.iter().map(|a| eval_expr(ctx, a, env)).collect();
    let mut piped_fail: Set = 0;
    if piped && !arg_sets.is_empty() && arg_sets[0] & DESC != 0 {
        // a description piped into a continuation: the executor runs it and
        // hands the continuation its YIELD, never the description itself —
        // and never a failure, which the bind skips before the call. the
        // piped value's own failure bits short-circuit at the call site, so
        // they reach the result directly.
        piped_fail = arg_sets[0] & FAIL;
        arg_sets[0] = (arg_sets[0] & !DESC & !FAIL) | desc_yield(&args[0]);
    }
    let piped_fail = piped_fail;
    let Expr::Ident(name, _) = head else {
        return TOP | piped_fail;
    };
    if env.contains_key(name.as_str()) {
        return TOP | piped_fail; // calling a local function value
    }
    if name == "if" {
        let cond_fail = arg_sets[0] & FAIL;
        return arg_sets[1] | arg_sets[2] | cond_fail | piped_fail;
    }
    if name == "err" {
        return ERR | piped_fail;
    }
    if name == "print" {
        return DESC | (arg_sets[0] & FAIL) | piped_fail;
    }
    if let Some(&idx) = ctx.type_names.get(name.as_str()) {
        // constructing a declared type: grow each field's set by this arg's,
        // dropping failures (a failing arg makes construction propagate, so the
        // field itself only ever holds the successful value's type)
        for (fi, argset) in arg_sets.iter().enumerate() {
            if let Some(slot) = ctx.type_fields[idx].get_mut(fi) {
                let refined = *slot | (*argset & !FAIL);
                if refined != *slot {
                    *slot = refined;
                    ctx.changed = true;
                }
            }
        }
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return REC | fails | piped_fail;
    }
    if name == "entry" {
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return REC | fails | piped_fail;
    }
    if let Some(decls) = ctx.groups.get(&(name.as_str(), args.len())) {
        let decls = decls.clone();
        let mut out: Set = 0;
        // pass-through: a failure in arg `pos` reaches the result only when no arm
        // catches it there. an arm whose pattern is `none`/`(err _)` handles that
        // failure (e.g. `_is_ws none -> false`), so it must not contaminate the
        // result — that spurious `none` is what kept scanner positions off `int`.
        for (pos, arg) in arg_sets.iter().enumerate() {
            let caught = decls.iter().fold(0, |acc, &i| {
                acc | ctx.program.fns[i].params.get(pos).map_or(0, pattern_catches)
            });
            out |= (arg & FAIL) & !caught;
        }
        for i in decls {
            for (p, set) in arg_sets.iter().enumerate() {
                widen_param(ctx, i, p, *set);
            }
            out |= ctx.returns[i];
        }
        return out | piped_fail;
    }
    builtin_set(name, &arg_sets) | piped_fail
}

/// What a description's execution hands a bound continuation, syntactically:
/// the yield of the lexical description expression, failures stripped (the
/// bind skips them before the continuation runs). Anything unrecognized is
/// conservatively any-non-failure.
fn desc_yield(e: &Expr) -> Set {
    match e {
        Expr::App { head, piped: false, .. } => match head.as_ref() {
            Expr::Ident(n, _) if n == "read_file" || n == "stdin" => STR,
            Expr::Ident(n, _) if n == "args" => LIST,
            Expr::Ident(n, _) if n == "print" || n == "write_file" => 0,
            _ => TOP & !FAIL,
        },
        // `a >> b` yields what its right side yields
        Expr::Seq(_, b, _) => desc_yield(b),
        // a join yields nothing a continuation would see
        Expr::BinOp { op: "&", .. } => 0,
        _ => TOP & !FAIL,
    }
}

pub fn builtin_set(name: &str, args: &[Set]) -> Set {
    let fails: Set = args.iter().fold(0, |acc, s| acc | (s & FAIL));
    match name {
        "at" => {
            let mut out = fails | NONE;
            if args[0] & BYTES != 0 {
                out |= INT;
            }
            if args[0] & (LIST | MAP) != 0 {
                out |= TOP & !FAIL;
            }
            if args[0] & STR != 0 {
                out |= STR;
            }
            out
        }
        "bytes" => BYTES | fails,
        "find2" => INT | fails,
        "slice" => (args[0] & (BYTES | LIST | STR)) | fails,
        "utf8" => STR | ERR | fails,
        "length" => INT | fails,
        "push" | "concat" | "chars" | "entries" | "sort" | "filter" => LIST | fails,
        "map" => LIST | fails,
        "put" => MAP | fails,
        "join" => STR | fails,
        "to_int" => INT | ERR | fails,
        "to_float" => FLOAT | ERR | fails,
        "from_code" => STR | ERR | fails,
        "char_code" => INT | fails,
        "sum" => INT | fails,
        "read_file" | "write_file" => DESC | fails,
        _ => TOP,
    }
}
