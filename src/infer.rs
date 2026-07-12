use crate::ast::*;
use std::collections::HashMap;

/// Propagable type sets as tag bitsets — the fixpoint of design/fixpoint.md,
/// coarse to start: one bit per runtime tag, records unrefined.
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
}

struct Ctx<'a> {
    program: &'a Program,
    groups: HashMap<(&'a str, usize), Vec<usize>>,
    type_names: HashMap<&'a str, usize>,
    params: Vec<Vec<Set>>,
    returns: Vec<Set>,
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
                bind_pattern(pattern, *joined, &mut env);
            }
            let ret = eval_body(&mut ctx, &decl.body, &mut env);
            if ret | ctx.returns[i] != ctx.returns[i] {
                ctx.returns[i] |= ret;
                ctx.changed = true;
            }
        }
    }
    Inference { params: ctx.params, returns: ctx.returns }
}

fn bind_pattern<'a>(pattern: &'a Pattern, joined: Set, env: &mut HashMap<&'a str, Set>) {
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
        Pattern::Ctor { fields, .. } => {
            for field in fields {
                bind_pattern(field, TOP & !FAIL, env);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for entry in entries {
                env.insert(&entry.bind_name, TOP & !FAIL);
            }
        }
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
                    _ => bind_pattern(pattern, value, env),
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
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    let _ = eval_expr(ctx, inner, env);
                }
            }
            STR
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
                _ => BOOL | fails,
            }
        }
        Expr::App { head, args, .. } => eval_call(ctx, head, args, env),
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
        _ => {
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

fn eval_call<'a>(ctx: &mut Ctx<'a>, head: &'a Expr, args: &'a [Expr], env: &mut HashMap<&'a str, Set>) -> Set {
    let arg_sets: Vec<Set> = args.iter().map(|a| eval_expr(ctx, a, env)).collect();
    let Expr::Ident(name, _) = head else {
        return TOP;
    };
    if env.contains_key(name.as_str()) {
        return TOP; // calling a local function value
    }
    if name == "if" {
        let cond_fail = arg_sets[0] & FAIL;
        return arg_sets[1] | arg_sets[2] | cond_fail;
    }
    if name == "err" {
        return ERR;
    }
    if name == "print" {
        return DESC | (arg_sets[0] & FAIL);
    }
    if ctx.type_names.contains_key(name.as_str()) || name == "entry" {
        let fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        return REC | fails;
    }
    if let Some(decls) = ctx.groups.get(&(name.as_str(), args.len())) {
        let decls = decls.clone();
        let mut out: Set = 0;
        // pass-through: failures flow when unhandled; coarse union keeps it monotone
        let arg_fails: Set = arg_sets.iter().fold(0, |acc, s| acc | (s & FAIL));
        out |= arg_fails;
        for i in decls {
            for (p, set) in arg_sets.iter().enumerate() {
                widen_param(ctx, i, p, *set);
            }
            out |= ctx.returns[i];
        }
        return out;
    }
    builtin_set(name, &arg_sets)
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
        _ => TOP,
    }
}
