//! The browser backend: compiles a kanso program to a WebAssembly module in
//! which every value is an i32 handle into the host-side registry. The module
//! has no memory and no data — literals are pre-registered at compile time
//! and baked in as handle constants; dispatch, calls, and recursion are wasm.
use crate::ast::*;
use crate::wasm_encode::{Body, Import, Module};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Lit {
    Int(num_bigint::BigInt),
    Float(f64),
    Str(String),
    True,
    False,
    NoneV,
}

pub struct Compiled {
    pub bytes: Vec<u8>,
    pub lits: Vec<Lit>,
    /// type id -> (name, field names); id 0 is the builtin `entry`.
    pub types: Vec<(String, Vec<String>)>,
}

#[derive(PartialEq, Eq, Hash)]
enum LitKey {
    Int(num_bigint::BigInt),
    FloatBits(u64),
    Str(String),
    True,
    False,
    NoneV,
}

const RT_IS_FAILURE: u32 = 0;
const RT_EQ_LIT: u32 = 1;
const RT_CHECK_TYPE: u32 = 2;
const RT_CHECK_REC: u32 = 3;
const RT_CHECK_ERR: u32 = 4;
const RT_FIELD: u32 = 5;
const RT_ERR_INNER: u32 = 6;
const RT_KEYED_CHECK: u32 = 7;
const RT_KEYED_FIELD: u32 = 8;
const RT_MKERR: u32 = 9;
const RT_ARG: u32 = 10;
const RT_MKLIST: u32 = 11;
const RT_MKMAP: u32 = 12;
const RT_MKREC: u32 = 13;
const RT_TEMPLATE: u32 = 14;
const RT_BINOP: u32 = 15;
const RT_INDEX: u32 = 16;
const RT_TRUTHY: u32 = 17;
const RT_BUILTIN: u32 = 18;
const RT_SEQ: u32 = 19;
const RT_MAYBE_BIND: u32 = 20;
const RT_MKCLOSURE: u32 = 21;
const RT_CALL: u32 = 22;
const RT_ENVGET: u32 = 23;
const RT_DIE: u32 = 24;
const RT_LIST_LEN: u32 = 25;

fn imports() -> Vec<Import> {
    vec![
        Import { name: "rt_is_failure", params: 1, returns: true },
        Import { name: "rt_eq_lit", params: 2, returns: true },
        Import { name: "rt_check_type", params: 2, returns: true },
        Import { name: "rt_check_rec", params: 3, returns: true },
        Import { name: "rt_check_err", params: 1, returns: true },
        Import { name: "rt_field", params: 2, returns: true },
        Import { name: "rt_err_inner", params: 1, returns: true },
        Import { name: "rt_keyed_check", params: 2, returns: true },
        Import { name: "rt_keyed_field", params: 2, returns: true },
        Import { name: "rt_mkerr", params: 1, returns: true },
        Import { name: "rt_arg", params: 1, returns: false },
        Import { name: "rt_mklist", params: 1, returns: true },
        Import { name: "rt_mkmap", params: 1, returns: true },
        Import { name: "rt_mkrec", params: 2, returns: true },
        Import { name: "rt_template", params: 1, returns: true },
        Import { name: "rt_binop", params: 3, returns: true },
        Import { name: "rt_index", params: 2, returns: true },
        Import { name: "rt_truthy", params: 1, returns: true },
        Import { name: "rt_builtin", params: 2, returns: true },
        Import { name: "rt_seq", params: 2, returns: true },
        Import { name: "rt_maybe_bind", params: 2, returns: true },
        Import { name: "rt_mkclosure", params: 2, returns: true },
        Import { name: "rt_call", params: 2, returns: true },
        Import { name: "rt_envget", params: 2, returns: true },
        Import { name: "rt_die", params: 1, returns: false },
        Import { name: "rt_list_len", params: 1, returns: true },
    ]
}

struct Ctx {
    body: Body,
    scope: HashMap<String, u32>,
}

pub struct WasmBackend<'a> {
    program: &'a Program,
    module: Module,
    lits: Vec<Lit>,
    lit_map: HashMap<LitKey, u32>,
    type_ids: HashMap<&'a str, i64>,
    dispatchers: HashMap<(String, usize), u32>,
    wrappers: HashMap<String, u32>,
    tailcalls: bool,
}

pub fn compile(program: &Program, tailcalls: bool) -> Result<Compiled, String> {
    let mut type_ids = HashMap::new();
    type_ids.insert("entry", 0i64);
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    let mut backend = WasmBackend {
        program,
        module: Module::new(imports()),
        lits: Vec::new(),
        lit_map: HashMap::new(),
        type_ids,
        dispatchers: HashMap::new(),
        wrappers: HashMap::new(),
        tailcalls,
    };
    backend.run()
}

impl<'a> WasmBackend<'a> {
    fn run(&mut self) -> Result<Compiled, String> {
        let mut groups: Vec<(String, usize, Vec<&'a FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            let key = (decl.name.clone(), decl.params.len());
            match groups.iter_mut().find(|(n, a, _)| (*n == key.0) && *a == key.1) {
                Some((_, _, decls)) => decls.push(decl),
                None => groups.push((key.0, key.1, vec![decl])),
            }
        }
        for (name, arity, _) in &groups {
            let idx = self.module.declare(*arity as u32);
            self.dispatchers.insert((name.clone(), *arity), idx);
        }
        let Some(main_idx) = self.dispatchers.get(&("main".to_string(), 0)).copied() else {
            return Err("no main".to_string());
        };
        self.module.set_main(main_idx);
        for (name, arity, decls) in &groups {
            let idx = self.dispatchers[&(name.clone(), *arity)];
            let body = self.emit_dispatcher(name, *arity, decls)?;
            self.module.define(idx, body);
        }
        let mut types = vec![("entry".to_string(), vec!["key".to_string(), "value".to_string()])];
        for ty in &self.program.types {
            let fields = ty.fields.iter().map(|(name, _, _)| name.clone()).collect();
            types.push((ty.name.clone(), fields));
        }
        let module = std::mem::replace(&mut self.module, Module::new(Vec::new()));
        Ok(Compiled { bytes: module.assemble(), lits: std::mem::take(&mut self.lits), types })
    }

    fn lit(&mut self, key: LitKey, make: impl FnOnce() -> Lit) -> u32 {
        if let Some(idx) = self.lit_map.get(&key) {
            return *idx;
        }
        let idx = self.lits.len() as u32;
        self.lits.push(make());
        self.lit_map.insert(key, idx);
        idx
    }

    fn str_lit(&mut self, text: &str) -> u32 {
        self.lit(LitKey::Str(text.to_string()), || Lit::Str(text.to_string()))
    }

    fn emit_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&'a FnDecl],
    ) -> Result<Body, String> {
        let mut ctx = Ctx { body: Body::new(arity as u32), scope: HashMap::new() };
        for decl in decls {
            ctx.scope.clear();
            ctx.body.block_void();
            for (i, pattern) in decl.params.iter().enumerate() {
                self.emit_pattern(&mut ctx, i as u32, pattern)?;
            }
            self.emit_body(&mut ctx, &decl.body, true)?;
            ctx.body.ret();
            ctx.body.end();
        }
        for i in 0..arity as u32 {
            ctx.body.local_get(i);
            ctx.body.call(RT_IS_FAILURE);
            ctx.body.if_void();
            ctx.body.local_get(i);
            ctx.body.ret();
            ctx.body.end();
        }
        let msg = self.str_lit(&format!("no overload of `{name}` matches these arguments"));
        ctx.body.i32_const(msg as i64);
        ctx.body.call(RT_DIE);
        ctx.body.unreachable();
        Ok(ctx.body)
    }

    /// Emits the checks for one dispatch-arm pattern; a mismatch branches to
    /// the enclosing arm block (depth 0 — checks stay flat).
    fn emit_pattern(&mut self, ctx: &mut Ctx, value_local: u32, pattern: &Pattern) -> Result<(), String> {
        match pattern {
            Pattern::IntLit(n, _) => {
                let lit = self.lit(LitKey::Int(n.clone()), || Lit::Int(n.clone()));
                ctx.body.local_get(value_local);
                ctx.body.i32_const(lit as i64);
                ctx.body.call(RT_EQ_LIT);
                ctx.body.eqz();
                ctx.body.br_if(0);
            }
            Pattern::StrLit(s, _) => {
                let lit = self.str_lit(s);
                ctx.body.local_get(value_local);
                ctx.body.i32_const(lit as i64);
                ctx.body.call(RT_EQ_LIT);
                ctx.body.eqz();
                ctx.body.br_if(0);
            }
            Pattern::Nullary(name, _) => {
                let lit = self.nullary_lit(name);
                ctx.body.local_get(value_local);
                ctx.body.i32_const(lit as i64);
                ctx.body.call(RT_EQ_LIT);
                ctx.body.eqz();
                ctx.body.br_if(0);
            }
            Pattern::Wildcard(_) => {
                ctx.body.local_get(value_local);
                ctx.body.call(RT_IS_FAILURE);
                ctx.body.br_if(0);
            }
            Pattern::Var(name, _) => {
                ctx.body.local_get(value_local);
                ctx.body.call(RT_IS_FAILURE);
                ctx.body.br_if(0);
                ctx.scope.insert(name.clone(), value_local);
            }
            Pattern::Annotated { name, ty, .. } => {
                let code = self.type_code(ty)?;
                ctx.body.local_get(value_local);
                ctx.body.i32_const(code);
                ctx.body.call(RT_CHECK_TYPE);
                ctx.body.eqz();
                ctx.body.br_if(0);
                ctx.scope.insert(name.clone(), value_local);
            }
            Pattern::Ctor { ty, fields } if ty == "err" => {
                ctx.body.local_get(value_local);
                ctx.body.call(RT_CHECK_ERR);
                ctx.body.eqz();
                ctx.body.br_if(0);
                let inner = ctx.body.local();
                ctx.body.local_get(value_local);
                ctx.body.call(RT_ERR_INNER);
                ctx.body.local_set(inner);
                self.emit_pattern(ctx, inner, &fields[0])?;
            }
            Pattern::Ctor { ty, fields } => {
                let tid = *self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("unknown type `{ty}`"))?;
                ctx.body.local_get(value_local);
                ctx.body.i32_const(tid);
                ctx.body.i32_const(fields.len() as i64);
                ctx.body.call(RT_CHECK_REC);
                ctx.body.eqz();
                ctx.body.br_if(0);
                for (i, field) in fields.iter().enumerate() {
                    let fv = ctx.body.local();
                    ctx.body.local_get(value_local);
                    ctx.body.i32_const(i as i64);
                    ctx.body.call(RT_FIELD);
                    ctx.body.local_set(fv);
                    self.emit_pattern(ctx, fv, field)?;
                }
            }
            Pattern::Keyed { .. } => {
                // keyed patterns never match in dispatch (bindings only)
                ctx.body.op_idx(0x0c, 0);
            }
        }
        Ok(())
    }

    fn nullary_lit(&mut self, name: &str) -> u32 {
        match name {
            "true" => self.lit(LitKey::True, || Lit::True),
            "false" => self.lit(LitKey::False, || Lit::False),
            _ => self.lit(LitKey::NoneV, || Lit::NoneV),
        }
    }

    fn type_code(&self, ty: &str) -> Result<i64, String> {
        if ty.ends_with("[]") {
            return Ok(4);
        }
        if ty.contains('[') {
            return Ok(5);
        }
        Ok(match ty {
            "int" => 0,
            "float64" => 1,
            "string" => 2,
            "bool" => 3,
            "err" => 6,
            _ => {
                let tid = self
                    .type_ids
                    .get(ty)
                    .ok_or_else(|| format!("unknown type `{ty}`"))?;
                100 + tid
            }
        })
    }

    fn emit_body(&mut self, ctx: &mut Ctx, body: &[Stmt], tail: bool) -> Result<(), String> {
        let last = body.len() - 1;
        for (i, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    self.emit_expr(ctx, expr, false)?;
                    self.emit_binding(ctx, pattern)?;
                }
                Stmt::Expr(expr) => {
                    self.emit_expr(ctx, expr, tail && i == last)?;
                    if i != last {
                        ctx.body.drop_();
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_binding(&mut self, ctx: &mut Ctx, pattern: &Pattern) -> Result<(), String> {
        match pattern {
            Pattern::Var(name, _) => {
                let local = ctx.body.local();
                ctx.body.local_set(local);
                ctx.scope.insert(name.clone(), local);
            }
            Pattern::Ctor { ty, fields } => {
                let tid = *self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("unknown type `{ty}`"))?;
                let v = ctx.body.local();
                ctx.body.local_tee(v);
                ctx.body.i32_const(tid);
                ctx.body.i32_const(fields.len() as i64);
                ctx.body.call(RT_CHECK_REC);
                ctx.body.eqz();
                ctx.body.if_void();
                let msg = self.str_lit(&format!("cannot destructure value as `{ty}`"));
                ctx.body.i32_const(msg as i64);
                ctx.body.call(RT_DIE);
                ctx.body.unreachable();
                ctx.body.end();
                for (i, field) in fields.iter().enumerate() {
                    if let Pattern::Var(name, _) = field {
                        let local = ctx.body.local();
                        ctx.body.local_get(v);
                        ctx.body.i32_const(i as i64);
                        ctx.body.call(RT_FIELD);
                        ctx.body.local_set(local);
                        ctx.scope.insert(name.clone(), local);
                    }
                }
            }
            Pattern::Keyed { entries, .. } => {
                let v = ctx.body.local();
                ctx.body.local_tee(v);
                ctx.body.i32_const(entries.len() as i64);
                ctx.body.call(RT_KEYED_CHECK);
                ctx.body.drop_();
                for entry in entries {
                    let name_lit = self.str_lit(&entry.field);
                    let local = ctx.body.local();
                    ctx.body.local_get(v);
                    ctx.body.i32_const(name_lit as i64);
                    ctx.body.call(RT_KEYED_FIELD);
                    ctx.body.local_set(local);
                    ctx.scope.insert(entry.bind_name.clone(), local);
                }
            }
            _ => return Err("unsupported binding pattern".to_string()),
        }
        Ok(())
    }

    fn emit_expr(&mut self, ctx: &mut Ctx, expr: &Expr, tail: bool) -> Result<(), String> {
        match expr {
            Expr::Int(n, _) => {
                let lit = self.lit(LitKey::Int(n.clone()), || Lit::Int(n.clone()));
                ctx.body.i32_const(lit as i64);
            }
            Expr::Float(x, _) => {
                let lit = self.lit(LitKey::FloatBits(x.to_bits()), || Lit::Float(*x));
                ctx.body.i32_const(lit as i64);
            }
            Expr::Str(parts, _) => self.emit_template(ctx, parts)?,
            Expr::Ident(name, _) => self.emit_ident(ctx, name, tail)?,
            Expr::List(items, _) => {
                for item in items {
                    self.emit_expr(ctx, item, false)?;
                    ctx.body.call(RT_ARG);
                }
                ctx.body.i32_const(items.len() as i64);
                ctx.body.call(RT_MKLIST);
            }
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    self.emit_expr(ctx, k, false)?;
                    ctx.body.call(RT_ARG);
                    self.emit_expr(ctx, v, false)?;
                    ctx.body.call(RT_ARG);
                }
                ctx.body.i32_const(pairs.len() as i64);
                ctx.body.call(RT_MKMAP);
            }
            Expr::Index { base, index, .. } => {
                self.emit_expr(ctx, base, false)?;
                self.emit_expr(ctx, index, false)?;
                ctx.body.call(RT_INDEX);
            }
            Expr::Seq(l, r, _) => {
                self.emit_expr(ctx, l, false)?;
                self.emit_expr(ctx, r, false)?;
                ctx.body.call(RT_SEQ);
            }
            Expr::Lambda { .. } => self.emit_lambda(ctx, expr)?,
            Expr::BinOp { op, lhs, rhs, .. } => {
                let code = match *op {
                    "+" => 0,
                    "-" => 1,
                    "*" => 2,
                    "/" => 3,
                    "==" => 10,
                    "!=" => 11,
                    "<" => 12,
                    ">" => 13,
                    "<=" => 14,
                    ">=" => 15,
                    other => return Err(format!("unsupported operator `{other}`")),
                };
                ctx.body.i32_const(code);
                self.emit_expr(ctx, lhs, false)?;
                self.emit_expr(ctx, rhs, false)?;
                ctx.body.call(RT_BINOP);
            }
            Expr::App { head, args, piped, .. } => self.emit_app(ctx, head, args, *piped, tail)?,
        }
        Ok(())
    }

    fn emit_template(&mut self, ctx: &mut Ctx, parts: &[TemplatePart]) -> Result<(), String> {
        if let [TemplatePart::Lit(s)] = parts {
            let lit = self.str_lit(s);
            ctx.body.i32_const(lit as i64);
            return Ok(());
        }
        if parts.is_empty() {
            let lit = self.str_lit("");
            ctx.body.i32_const(lit as i64);
            return Ok(());
        }
        for part in parts {
            match part {
                TemplatePart::Lit(s) => {
                    let lit = self.str_lit(s);
                    ctx.body.i32_const(lit as i64);
                }
                TemplatePart::Interp(inner) => self.emit_expr(ctx, inner, false)?,
            }
            ctx.body.call(RT_ARG);
        }
        ctx.body.i32_const(parts.len() as i64);
        ctx.body.call(RT_TEMPLATE);
        Ok(())
    }

    fn emit_ident(&mut self, ctx: &mut Ctx, name: &str, tail: bool) -> Result<(), String> {
        if let Some(local) = ctx.scope.get(name) {
            ctx.body.local_get(*local);
            return Ok(());
        }
        if self.program.types.iter().any(|t| t.name == name && t.fields.is_empty()) {
            let tid = self.type_ids[name];
            ctx.body.i32_const(tid);
            ctx.body.i32_const(0);
            ctx.body.call(RT_MKREC);
            return Ok(());
        }
        if let Some(idx) = self.dispatchers.get(&(name.to_string(), 0)).copied() {
            match tail && self.tailcalls {
                true => ctx.body.return_call(idx),
                false => ctx.body.call(idx),
            }
            return Ok(());
        }
        match name {
            "true" | "false" | "none" => {
                let lit = self.nullary_lit(name);
                ctx.body.i32_const(lit as i64);
            }
            "args" | "stdin" => {
                let lit = self.str_lit(name);
                ctx.body.i32_const(lit as i64);
                ctx.body.i32_const(0);
                ctx.body.call(RT_BUILTIN);
            }
            _ if self.program.fns.iter().any(|d| d.name == name) => {
                let widx = self.fn_wrapper(name)?;
                ctx.body.i32_const(widx as i64);
                ctx.body.i32_const(0);
                ctx.body.call(RT_MKCLOSURE);
            }
            _ => return Err(format!("unsupported name `{name}`")),
        }
        Ok(())
    }

    /// A named function used as a value becomes a table wrapper dispatching
    /// on the argument count.
    fn fn_wrapper(&mut self, name: &str) -> Result<u32, String> {
        if let Some(widx) = self.wrappers.get(name) {
            return Ok(*widx);
        }
        let arities: Vec<usize> = self
            .program
            .fns
            .iter()
            .filter(|d| d.name == name)
            .map(|d| d.params.len())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let fn_idx = self.module.declare(2);
        self.module.table.push(fn_idx);
        let widx = (self.module.table.len() - 1) as u32;
        self.wrappers.insert(name.to_string(), widx);
        let mut body = Body::new(2);
        let len = body.local();
        body.local_get(1);
        body.call(RT_LIST_LEN);
        body.local_set(len);
        for arity in arities {
            let target = self.dispatchers[&(name.to_string(), arity)];
            body.local_get(len);
            body.i32_const(arity as i64);
            body.op(0x46); // i32.eq
            body.if_void();
            for i in 0..arity {
                body.local_get(1);
                body.i32_const(i as i64);
                body.call(RT_ENVGET);
            }
            body.call(target);
            body.ret();
            body.end();
        }
        let msg = self.str_lit(&format!("no overload of `{name}` matches these arguments"));
        body.i32_const(msg as i64);
        body.call(RT_DIE);
        body.unreachable();
        self.module.define(fn_idx, body);
        Ok(widx)
    }

    /// Lambda lifting: the closure body becomes a table function taking
    /// (env, args); captures ride in env, both read via rt_envget.
    fn emit_lambda(&mut self, ctx: &mut Ctx, expr: &Expr) -> Result<(), String> {
        let Expr::Lambda { params, body, .. } = expr else {
            return Err("not a lambda".to_string());
        };
        let param_names: Vec<&str> = params.iter().map(|(p, _)| p.as_str()).collect();
        let mut captures: Vec<String> = Vec::new();
        free_idents(body, &mut |name| {
            if ctx.scope.contains_key(name)
                && !param_names.contains(&name)
                && !captures.iter().any(|c| c == name)
            {
                captures.push(name.to_string());
            }
        });
        let fn_idx = self.module.declare(2);
        self.module.table.push(fn_idx);
        let tidx = (self.module.table.len() - 1) as u32;
        let mut inner = Ctx { body: Body::new(2), scope: HashMap::new() };
        for (i, p) in param_names.iter().enumerate() {
            let local = inner.body.local();
            inner.body.local_get(1);
            inner.body.i32_const(i as i64);
            inner.body.call(RT_ENVGET);
            inner.body.local_set(local);
            inner.scope.insert(p.to_string(), local);
        }
        for (i, c) in captures.iter().enumerate() {
            let local = inner.body.local();
            inner.body.local_get(0);
            inner.body.i32_const(i as i64);
            inner.body.call(RT_ENVGET);
            inner.body.local_set(local);
            inner.scope.insert(c.clone(), local);
        }
        self.emit_expr(&mut inner, body, true)?;
        self.module.define(fn_idx, inner.body);
        for c in &captures {
            ctx.body.local_get(ctx.scope[c]);
            ctx.body.call(RT_ARG);
        }
        ctx.body.i32_const(tidx as i64);
        ctx.body.i32_const(captures.len() as i64);
        ctx.body.call(RT_MKCLOSURE);
        Ok(())
    }

    fn emit_app(
        &mut self,
        ctx: &mut Ctx,
        head: &Expr,
        args: &[Expr],
        piped: bool,
        tail: bool,
    ) -> Result<(), String> {
        if piped {
            return self.emit_piped(ctx, head, args);
        }
        let Expr::Ident(name, _) = head else {
            return Err("unsupported call head".to_string());
        };
        if ctx.scope.contains_key(name.as_str()) {
            for arg in args {
                self.emit_expr(ctx, arg, false)?;
                ctx.body.call(RT_ARG);
            }
            ctx.body.local_get(ctx.scope[name.as_str()]);
            ctx.body.i32_const(args.len() as i64);
            ctx.body.call(RT_CALL);
            return Ok(());
        }
        if name == "if" {
            let cond = ctx.body.local();
            self.emit_expr(ctx, &args[0], false)?;
            ctx.body.local_tee(cond);
            ctx.body.call(RT_IS_FAILURE);
            ctx.body.if_i32();
            ctx.body.local_get(cond);
            ctx.body.else_();
            ctx.body.local_get(cond);
            ctx.body.call(RT_TRUTHY);
            ctx.body.if_i32();
            self.emit_expr(ctx, &args[1], false)?;
            ctx.body.else_();
            self.emit_expr(ctx, &args[2], false)?;
            ctx.body.end();
            ctx.body.end();
            return Ok(());
        }
        if name == "err" {
            self.emit_expr(ctx, &args[0], false)?;
            ctx.body.call(RT_MKERR);
            return Ok(());
        }
        if let Some(tid) = self.type_ids.get(name.as_str()).copied() {
            let fields = self
                .program
                .types
                .iter()
                .find(|t| t.name == *name)
                .map(|t| t.fields.clone())
                .unwrap_or_default();
            for (i, arg) in args.iter().enumerate() {
                self.emit_expr(ctx, arg, false)?;
                match fields.get(i).filter(|(_, tys, _)| tys.len() >= 2) {
                    Some((field, tys, _)) => {
                        let value = ctx.body.local();
                        ctx.body.local_tee(value);
                        ctx.body.call(RT_ARG);
                        self.emit_typeset_check(ctx, value, name, field, tys)?;
                    }
                    None => ctx.body.call(RT_ARG),
                }
            }
            ctx.body.i32_const(tid);
            ctx.body.i32_const(args.len() as i64);
            ctx.body.call(RT_MKREC);
            return Ok(());
        }
        if let Some(idx) = self.dispatchers.get(&(name.clone(), args.len())).copied() {
            for arg in args {
                self.emit_expr(ctx, arg, false)?;
            }
            match tail && self.tailcalls {
                true => ctx.body.return_call(idx),
                false => ctx.body.call(idx),
            }
            return Ok(());
        }
        if crate::check::BUILTINS.contains(&name.as_str()) {
            for arg in args {
                self.emit_expr(ctx, arg, false)?;
                ctx.body.call(RT_ARG);
            }
            let lit = self.str_lit(name);
            ctx.body.i32_const(lit as i64);
            ctx.body.i32_const(args.len() as i64);
            ctx.body.call(RT_BUILTIN);
            return Ok(());
        }
        Err(format!("unsupported call to `{name}`"))
    }

    /// Constructor enforcement for a multi-member field typeset: a field value
    /// matching no member is a defect (failures skip the check and propagate
    /// through `rt_mkrec`).
    fn emit_typeset_check(
        &mut self,
        ctx: &mut Ctx,
        value: u32,
        ty_name: &str,
        field: &str,
        tys: &[String],
    ) -> Result<(), String> {
        ctx.body.local_get(value);
        ctx.body.call(RT_IS_FAILURE);
        for member in tys {
            let code = self.type_code(member)?;
            ctx.body.local_get(value);
            ctx.body.i32_const(code);
            ctx.body.call(RT_CHECK_TYPE);
            ctx.body.op(0x72); // i32.or
        }
        ctx.body.eqz();
        ctx.body.if_void();
        let msg =
            self.str_lit(&format!("field `{field}` of `{ty_name}` takes {}", tys.join(" ")));
        ctx.body.i32_const(msg as i64);
        ctx.body.call(RT_DIE);
        ctx.body.unreachable();
        ctx.body.end();
        Ok(())
    }

    /// A piped application binds when the piped value is a description:
    /// the rest of the call becomes a continuation closure over the already
    /// evaluated arguments, mirroring the native emitter.
    fn emit_piped(&mut self, ctx: &mut Ctx, head: &Expr, args: &[Expr]) -> Result<(), String> {
        let piped_local = ctx.body.local();
        self.emit_expr(ctx, &args[0], false)?;
        ctx.body.local_set(piped_local);
        let closure: Result<(), String> = match head {
            Expr::Ident(name, _)
                if self.dispatchers.contains_key(&(name.clone(), args.len())) =>
            {
                let target = self.dispatchers[&(name.clone(), args.len())];
                let rest = args.len() - 1;
                let fn_idx = self.module.declare(2);
                self.module.table.push(fn_idx);
                let tidx = (self.module.table.len() - 1) as u32;
                let mut inner = Body::new(2);
                inner.local_get(1);
                inner.i32_const(0);
                inner.call(RT_ENVGET);
                for i in 0..rest {
                    inner.local_get(0);
                    inner.i32_const(i as i64);
                    inner.call(RT_ENVGET);
                }
                inner.call(target);
                self.module.define(fn_idx, inner);
                for arg in &args[1..] {
                    self.emit_expr(ctx, arg, false)?;
                    ctx.body.call(RT_ARG);
                }
                ctx.body.i32_const(tidx as i64);
                ctx.body.i32_const(rest as i64);
                ctx.body.call(RT_MKCLOSURE);
                Ok(())
            }
            Expr::Ident(name, _) if crate::check::BUILTINS.contains(&name.as_str()) => {
                let rest = args.len() - 1;
                let name_lit = self.str_lit(name);
                let fn_idx = self.module.declare(2);
                self.module.table.push(fn_idx);
                let tidx = (self.module.table.len() - 1) as u32;
                let mut inner = Body::new(2);
                inner.local_get(1);
                inner.i32_const(0);
                inner.call(RT_ENVGET);
                inner.call(RT_ARG);
                for i in 0..rest {
                    inner.local_get(0);
                    inner.i32_const(i as i64);
                    inner.call(RT_ENVGET);
                    inner.call(RT_ARG);
                }
                inner.i32_const(name_lit as i64);
                inner.i32_const(args.len() as i64);
                inner.call(RT_BUILTIN);
                self.module.define(fn_idx, inner);
                for arg in &args[1..] {
                    self.emit_expr(ctx, arg, false)?;
                    ctx.body.call(RT_ARG);
                }
                ctx.body.i32_const(tidx as i64);
                ctx.body.i32_const(rest as i64);
                ctx.body.call(RT_MKCLOSURE);
                Ok(())
            }
            Expr::Lambda { .. } if args.len() == 1 => self.emit_lambda(ctx, head),
            Expr::Ident(name, _) if ctx.scope.contains_key(name.as_str()) && args.len() == 1 => {
                ctx.body.local_get(ctx.scope[name.as_str()]);
                Ok(())
            }
            _ => Err("unsupported pipe target".to_string()),
        };
        closure?;
        let c = ctx.body.local();
        ctx.body.local_set(c);
        ctx.body.local_get(piped_local);
        ctx.body.local_get(c);
        ctx.body.call(RT_MAYBE_BIND);
        Ok(())
    }
}

fn free_idents(expr: &Expr, visit: &mut dyn FnMut(&str)) {
    match expr {
        Expr::Ident(name, _) => visit(name),
        Expr::Int(..) | Expr::Float(..) => {}
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    free_idents(inner, visit);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                free_idents(item, visit);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                free_idents(k, visit);
                free_idents(v, visit);
            }
        }
        Expr::Index { base, index, .. } => {
            free_idents(base, visit);
            free_idents(index, visit);
        }
        Expr::Seq(l, r, _) => {
            free_idents(l, visit);
            free_idents(r, visit);
        }
        Expr::Lambda { params, body, .. } => {
            let mask: Vec<&str> = params.iter().map(|(p, _)| p.as_str()).collect();
            free_idents(body, &mut |name| {
                if !mask.contains(&name) {
                    visit(name);
                }
            });
        }
        Expr::BinOp { lhs, rhs, .. } => {
            free_idents(lhs, visit);
            free_idents(rhs, visit);
        }
        Expr::App { head, args, .. } => {
            free_idents(head, visit);
            for arg in args {
                free_idents(arg, visit);
            }
        }
    }
}
