//! The browser backend: compiles a kanso program to a WebAssembly module in
//! which every value is an i32 handle into the host-side registry. The module
//! has no memory and no data — literals are pre-registered at compile time
//! and baked in as handle constants; dispatch, calls, and recursion are wasm.
use crate::ast::*;
use crate::hash::Map as HashMap;
use crate::wasm_encode::{Body, Import, Module};

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
/// The arity that marks a cell rather than a function. Paired with `DEFERRED`
/// in wasm_rt, which is what reads it.
const DEFERRED_ARITY: i64 = -2;
const RT_CALL: u32 = 22;
const RT_ENVGET: u32 = 23;
const RT_DIE: u32 = 24;
const RT_LIST_LEN: u32 = 25;
const RT_ERR_HOP: u32 = 26;
const RT_ERR_STAMP: u32 = 27;
const RT_AT: u32 = 28;
const RT_MKSUB: u32 = 29;
const RT_UPCAST: u32 = 30;
const RT_SETFIELD: u32 = 31;
const RT_FIELD_BY_NAME: u32 = 32;
const RT_ROUTES_TO_ARMS: u32 = 33;
const RT_JOIN: u32 = 34;
const RT_NO_FIELD: u32 = 35;
const RT_DEFER: u32 = 36;
const RT_CONST: u32 = 37;
const RT_FORCE: u32 = 38;
/// Whether a match may proceed: false only for an err this arm's own hako
/// raised (gavel 24, clause 1, as dispatch semantics).
const RT_NOT_OWN_ERR: u32 = 39;
/// A positional destructuring bind whose value is the wrong shape. It takes
/// the VALUE as well as the type name, because the sentence names what the
/// reader bound and only the runtime knows it.
const RT_DIE_DESTRUCTURE: u32 = 40;
/// The three worded chain steps. Appended at the end of the import list so
/// every index above stays where it was.
const RT_BIND: u32 = 41;
const RT_RESCUE: u32 = 42;
const RT_ANNOTATE: u32 = 43;

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
        Import { name: "rt_mkerr", params: 2, returns: true },
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
        Import { name: "rt_mkclosure", params: 3, returns: true },
        Import { name: "rt_call", params: 2, returns: true },
        Import { name: "rt_envget", params: 2, returns: true },
        Import { name: "rt_die", params: 1, returns: false },
        Import { name: "rt_list_len", params: 1, returns: true },
        Import { name: "rt_err_hop", params: 2, returns: true },
        Import { name: "rt_err_stamp", params: 2, returns: true },
        Import { name: "rt_at", params: 2, returns: true },
        Import { name: "rt_mksub", params: 2, returns: true },
        Import { name: "rt_upcast", params: 2, returns: true },
        Import { name: "rt_setfield", params: 3, returns: true },
        Import { name: "rt_field_by_name", params: 2, returns: true },
        Import { name: "rt_routes_to_arms", params: 1, returns: true },
        Import { name: "rt_join", params: 2, returns: true },
        Import { name: "rt_no_field", params: 2, returns: true },
        Import { name: "rt_defer", params: 1, returns: true },
        Import { name: "rt_const", params: 1, returns: true },
        Import { name: "rt_force", params: 1, returns: true },
        Import { name: "rt_not_own_err", params: 2, returns: true },
        Import { name: "rt_die_destructure", params: 2, returns: false },
        Import { name: "rt_bind", params: 2, returns: true },
        Import { name: "rt_rescue", params: 2, returns: true },
        Import { name: "rt_annotate", params: 3, returns: true },
    ]
}

struct Ctx {
    body: Body,
    scope: HashMap<String, u32>,
    /// Err-origin prefix "{fn} at {file}" for the declaration being emitted.
    prefix: String,
    /// The hako that declaration belongs to. An arm cannot see an err its own
    /// hako raised, and this is the side of the comparison the compiler knows.
    hako: String,
    /// The constant this body computes, where it is a constant. A list
    /// element mentioning it has to wait rather than read a cell that is
    /// still being filled.
    group: String,
}

pub struct WasmBackend<'a> {
    program: &'a Program,
    module: Module,
    lits: Vec<Lit>,
    lit_map: HashMap<LitKey, u32>,
    type_ids: HashMap<&'a str, i64>,
    dispatchers: HashMap<(String, usize), u32>,
    wrappers: HashMap<String, u32>,
    /// Zero-arity names that reach themselves through other constants.
    knotted: crate::hash::Set<String>,
    tailcalls: bool,
}

/// A partial application, as the lambda it is equivalent to: `&add 2` becomes
/// `(x -> add 2 x)`. Shared in shape with the native backend, and refusing the
/// same case for the same reason — with two arities live, only the arriving
/// arguments decide which one a partial is waiting for, and a lambda has to
/// commit to a parameter count before they arrive.
fn partial_lambda(
    program: &Program,
    name: &str,
    supplied: &[Expr],
    span: crate::diag::Span,
) -> Result<Expr, String> {
    let mut arities: Vec<usize> = program
        .fns
        .iter()
        .filter(|d| d.name == name && d.params.len() >= supplied.len())
        .map(|d| d.params.len())
        .collect();
    arities.sort_unstable();
    arities.dedup();
    // `&` supplies without running, so an arm's last argument is a partial
    // like any other and the value waits to be called. Only more arguments
    // than any arm accepts is unfinishable.
    if arities.is_empty() {
        return Err(format!(
            "browser backend: `&{name}` holds {} argument(s), and no `{name}` takes more",
            supplied.len()
        ));
    }
    let Some(&arity) = arities.first() else { unreachable!("checked non-empty") };
    if arities.len() > 1 {
        return Err(format!(
            "browser backend: `&{name}` escapes as a value while {} arms could still finish it",
            arities.len()
        ));
    }
    let params: Vec<(String, crate::diag::Span)> =
        (0..arity - supplied.len()).map(|i| (format!("k#partial{i}"), span)).collect();
    let mut args = supplied.to_vec();
    args.extend(params.iter().map(|(n, s)| Expr::Ident(n.clone(), *s)));
    let head = Expr::Ident(name.to_string(), span);
    let body = Expr::App { head: Box::new(head), args, piped: false, span };
    Ok(Expr::Lambda { params, body: Box::new(body), span })
}

pub fn compile(program: &Program, tailcalls: bool) -> Result<Compiled, String> {
    let mut type_ids = HashMap::default();
    type_ids.insert("entry", 0i64);
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    // an enrollment clone is an alias: one identity per type
    let clone_ids: Vec<(&str, i64)> = program
        .types
        .iter()
        .filter_map(|t| {
            t.origin.as_deref().and_then(|o| type_ids.get(o).map(|id| (t.name.as_str(), *id)))
        })
        .collect();
    for (name, id) in clone_ids {
        type_ids.insert(name, id);
    }
    let mut backend = WasmBackend {
        program,
        module: Module::new(imports()),
        lits: Vec::new(),
        lit_map: HashMap::default(),
        type_ids,
        dispatchers: HashMap::default(),
        wrappers: HashMap::default(),
        knotted: crate::codegen::knotted_constants(program),
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
        // a subtype annotation outranks its ancestors: arms sort deepest
        // first, mirroring the native dispatcher and the interp's scores
        let parents: crate::hash::Map<&str, &str> = self
            .program
            .types
            .iter()
            .filter_map(|t| t.parent.as_deref().map(|p| (t.name.as_str(), p)))
            .collect();
        let depth_of = |ty: &str| -> i64 {
            let mut d = 0i64;
            let mut cur = ty;
            while let Some(p) = parents.get(cur) {
                d += 1;
                cur = p;
            }
            d
        };
        let typeset_names: crate::hash::Set<&str> = self
            .program
            .types
            .iter()
            .filter(|t| !t.members.is_empty())
            .map(|t| t.name.as_str())
            .collect();
        for (_, _, decls) in &mut groups {
            decls.sort_by_key(|d| {
                let total: i64 = d
                    .params
                    .iter()
                    .map(|p| match p {
                        Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 3000,
                        Pattern::Annotated { ty, .. } => {
                            match typeset_names.contains(ty.as_str()) {
                                true => 1000,
                                false => 2000 + depth_of(ty),
                            }
                        }
                        // an err arm ranks as its reason pattern does —
                        // mirrors the native sort and the interp's scores
                        Pattern::Ctor { ty, fields, .. } if ty == "err" && fields.len() == 1 => {
                            match &fields[0] {
                                Pattern::Annotated { ty: rty, .. } => {
                                    match typeset_names.contains(rty.as_str()) {
                                        true => 1000,
                                        false => 2000 + depth_of(rty),
                                    }
                                }
                                Pattern::Var(..) | Pattern::Wildcard(..) => 1,
                                _ => 2000,
                            }
                        }
                        // a constructor pattern ranks by the same chain the
                        // annotations use: naming the subtype is nearer than
                        // naming what it wraps
                        Pattern::Ctor { ty, .. } => 2000 + depth_of(ty),
                        Pattern::Keyed { .. } => 2000,
                        Pattern::Var(..) | Pattern::Wildcard(..) => 0,
                    })
                    .sum();
                (std::cmp::Reverse(total), d.synthetic)
            });
        }
        for (name, arity, _) in &groups {
            let idx = self.module.declare(*arity as u32);
            self.dispatchers.insert((name.clone(), *arity), idx);
        }
        let Some(main_idx) = self.dispatchers.get(&(crate::ast::ENTRY.to_string(), 0)).copied()
        else {
            return Err("no entry".to_string());
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

    /// Can a value matching this annotation be an err? Only then is the
    /// own-origin guard emitted — every other pattern refuses failures
    /// already, so the check would cost a call per match to learn nothing.
    fn admits_err(&self, ty: &str) -> bool {
        if ty == "err" {
            return true;
        }
        self.program
            .types
            .iter()
            .find(|t| t.name == ty && !t.members.is_empty())
            .is_some_and(|t| t.members.iter().any(|m| m != ty && self.admits_err(m)))
    }

    /// Refuse the match when the value is an err this arm's own hako raised.
    fn own_origin_guard(&mut self, ctx: &mut Ctx, value_local: u32) {
        let arm = self.str_lit(&ctx.hako.clone());
        ctx.body.local_get(value_local);
        ctx.body.i32_const(arm as i64);
        ctx.body.call(RT_NOT_OWN_ERR);
        ctx.body.eqz();
        ctx.body.br_if(0);
    }

    fn str_lit(&mut self, text: &str) -> u32 {
        self.lit(LitKey::Str(text.to_string()), || Lit::Str(text.to_string()))
    }

    /// The origin literal for an err construction site.
    /// The raise site's literal: the package that raises here, a NUL, then the
    /// trace line. `rt_mkerr` splits it — the match rule wants the first half
    /// and the report wants the second, and one literal keeps the two from
    /// drifting apart. Native's `origin_arg` builds the same shape.
    fn origin_lit(&mut self, prefix: &str, hako: &str, span: crate::diag::Span) -> u32 {
        let origin = format!("{hako}\0{prefix}:{}", span.line);
        self.str_lit(&origin)
    }

    /// The table entry a knotted constant is reached through. Native seeds
    /// such a cell before main and freezes it once; a wasm module has no start
    /// section, so the cell is minted on the first mention and the runtime
    /// memoises it from there.
    fn const_cell(&mut self, name: &str) -> Option<u32> {
        if !self.knotted.contains(name) {
            return None;
        }
        self.fn_wrapper(name).ok()
    }

    fn emit_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&'a FnDecl],
    ) -> Result<Body, String> {
        let mut ctx = Ctx {
            body: Body::new(arity as u32),
            scope: HashMap::default(),
            prefix: String::new(),
            hako: String::new(),
            group: match arity {
                0 => name.to_string(),
                _ => String::new(),
            },
        };
        for decl in decls {
            ctx.scope.clear();
            ctx.prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            ctx.hako = crate::provenance::package_of(&decl.file).to_string();
            ctx.body.block_void();
            for (i, pattern) in decl.params.iter().enumerate() {
                self.emit_pattern(&mut ctx, i as u32, pattern)?;
            }
            self.emit_body(&mut ctx, &decl.body, true)?;
            ctx.body.ret();
            ctx.body.end();
        }
        let hop_name = self.str_lit(name);
        for i in 0..arity as u32 {
            ctx.body.local_get(i);
            ctx.body.call(RT_IS_FAILURE);
            ctx.body.if_void();
            ctx.body.local_get(i);
            ctx.body.i32_const(hop_name as i64);
            ctx.body.call(RT_ERR_HOP);
            ctx.body.ret();
            ctx.body.end();
        }
        // a getter that matched nothing is a field error to the reader, and
        // only the runtime can name the value it was handed
        match crate::ast::getter_field(name) {
            Some(field) if arity == 1 => {
                let lit = self.str_lit(field);
                ctx.body.local_get(0);
                ctx.body.i32_const(lit as i64);
                ctx.body.call(RT_NO_FIELD);
            }
            _ => {
                let msg = self.str_lit(&format!("no overload of `{name}` matches these arguments"));
                ctx.body.i32_const(msg as i64);
                ctx.body.call(RT_DIE);
            }
        }
        ctx.body.unreachable();
        Ok(ctx.body)
    }

    /// Emits the checks for one dispatch-arm pattern; a mismatch branches to
    /// the enclosing arm block (depth 0 — checks stay flat).
    fn emit_pattern(
        &mut self,
        ctx: &mut Ctx,
        value_local: u32,
        pattern: &Pattern,
    ) -> Result<(), String> {
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
                if self.admits_err(ty) {
                    self.own_origin_guard(ctx, value_local);
                }
                let members: Vec<String> = self
                    .program
                    .types
                    .iter()
                    .find(|t| t.name == *ty && !t.members.is_empty())
                    .map(|t| t.members.clone())
                    .unwrap_or_else(|| vec![ty.clone()]);
                // one member is the plain check; several OR together
                let ok = ctx.body.local();
                ctx.body.i32_const(0);
                ctx.body.local_set(ok);
                for member in &members {
                    let code = self.type_code(member)?;
                    ctx.body.local_get(value_local);
                    ctx.body.i32_const(code);
                    ctx.body.call(RT_CHECK_TYPE);
                    ctx.body.local_get(ok);
                    ctx.body.i32_or();
                    ctx.body.local_set(ok);
                }
                ctx.body.local_get(ok);
                ctx.body.eqz();
                ctx.body.br_if(0);
                ctx.scope.insert(name.clone(), value_local);
            }
            Pattern::Ctor { ty, fields, whole } if ty == "err" => {
                self.own_origin_guard(ctx, value_local);
                ctx.body.local_get(value_local);
                ctx.body.call(RT_CHECK_ERR);
                ctx.body.eqz();
                ctx.body.br_if(0);
                let inner = ctx.body.local();
                ctx.body.local_get(value_local);
                ctx.body.call(RT_ERR_INNER);
                ctx.body.local_set(inner);
                self.emit_pattern(ctx, inner, &fields[0])?;
                if let Some(named) = whole {
                    ctx.scope.insert(named.0.clone(), value_local);
                }
            }
            Pattern::Ctor { ty, fields, whole } => {
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
                if let Some(named) = whole {
                    ctx.scope.insert(named.0.clone(), value_local);
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
            "some" => 8,
            "err" => 6,
            "none" => 7,
            _ => {
                let tid = self.type_ids.get(ty).ok_or_else(|| format!("unknown type `{ty}`"))?;
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
                Stmt::Set { target, field, value, .. } => {
                    let Some(&local) = ctx.scope.get(target) else {
                        return Err(format!("`set` target `{target}` is not in scope"));
                    };
                    let name_lit = self.str_lit(field);
                    ctx.body.local_get(local);
                    ctx.body.i32_const(name_lit as i64);
                    self.emit_expr(ctx, value, false)?;
                    ctx.body.call(RT_SETFIELD);
                    ctx.body.drop_();
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
            Pattern::Ctor { ty, fields, .. } => {
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
                // The value goes to the runtime rather than a baked sentence,
                // matching codegen.rs and the interpreter. `v` still holds it:
                // the local_tee above kept it past the check.
                let msg = self.str_lit(ty);
                ctx.body.local_get(v);
                ctx.body.i32_const(msg as i64);
                ctx.body.call(RT_DIE_DESTRUCTURE);
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
            Expr::Partial(name, span) => {
                let lambda = partial_lambda(self.program, name, &[], *span)?;
                return self.emit_expr(ctx, &lambda, false);
            }
            Expr::Upcast { expr: inner, ty, .. } => {
                self.emit_expr(ctx, inner, false)?;
                let code = self.type_code(ty)?;
                ctx.body.i32_const(code);
                ctx.body.call(RT_UPCAST);
            }
            Expr::Block(stmts, _) => {
                self.emit_body(ctx, stmts, tail)?;
            }
            // A `build` answers nothing — its last statement is as likely to be
            // a field write as anything, and the names it bound are already in
            // this scope. It still has to leave a word behind, because every
            // expression site expects one, and the statement above it drops it.
            Expr::Build(stmts, _) => {
                for stmt in stmts {
                    self.emit_body(ctx, std::slice::from_ref(stmt), false)?;
                    if matches!(stmt, Stmt::Bind { .. } | Stmt::Set { .. }) {
                        continue;
                    }
                    ctx.body.drop_();
                }
                let lit = self.nullary_lit("none");
                ctx.body.i32_const(lit as i64);
            }
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
                    self.emit_element(ctx, item)?;
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
            Expr::Field { base, name, .. } => {
                self.emit_expr(ctx, base, false)?;
                let name_lit = self.str_lit(name);
                ctx.body.i32_const(name_lit as i64);
                ctx.body.call(RT_FIELD_BY_NAME);
            }
            Expr::Index { base, index, strict, span } => {
                self.emit_expr(ctx, base, false)?;
                self.emit_expr(ctx, index, false)?;
                match strict {
                    true => {
                        ctx.body.call(RT_INDEX);
                        let origin = self.origin_lit(&ctx.prefix, &ctx.hako, *span);
                        ctx.body.i32_const(origin as i64);
                        ctx.body.call(RT_ERR_STAMP);
                    }
                    false => ctx.body.call(RT_AT),
                }
            }
            Expr::Seq(l, r, _) => {
                self.emit_expr(ctx, l, false)?;
                // GAVEL 15: the wall defers its right side, so what follows
                // becomes a cell over the names it reads and the executor
                // builds it once the left has run.
                self.emit_cell(ctx, r)?;
                ctx.body.call(RT_SEQ);
            }
            Expr::Lambda { .. } => self.emit_lambda(ctx, expr)?,
            Expr::Join { lhs, rhs, .. } => {
                self.emit_expr(ctx, lhs, false)?;
                self.emit_expr(ctx, rhs, false)?;
                ctx.body.call(RT_JOIN);
            }
            Expr::Guard { cond, early, rest, .. } => {
                // a fired guard makes the tail unreachable, which is exactly
                // the untaken branch of a conditional
                let c = ctx.body.local();
                self.emit_expr(ctx, cond, false)?;
                ctx.body.local_tee(c);
                ctx.body.call(RT_IS_FAILURE);
                ctx.body.if_i32();
                ctx.body.local_get(c);
                ctx.body.else_();
                ctx.body.local_get(c);
                ctx.body.call(RT_TRUTHY);
                ctx.body.if_i32();
                self.emit_expr(ctx, early, false)?;
                ctx.body.else_();
                self.emit_body(ctx, rest, false)?;
                ctx.body.end();
                ctx.body.end();
            }
            Expr::BinOp { op, lhs, rhs, span } => {
                let armable =
                    matches!(*op, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
                        && self.program.fns.iter().any(|d| d.name == *op && d.params.len() == 2);
                if let Some(idx) =
                    armable.then(|| self.dispatchers.get(&(op.to_string(), 2)).copied()).flatten()
                {
                    let a = ctx.body.local();
                    let b = ctx.body.local();
                    self.emit_expr(ctx, lhs, false)?;
                    ctx.body.local_set(a);
                    self.emit_expr(ctx, rhs, false)?;
                    ctx.body.local_set(b);
                    ctx.body.local_get(a);
                    ctx.body.call(RT_ROUTES_TO_ARMS);
                    ctx.body.local_get(b);
                    ctx.body.call(RT_ROUTES_TO_ARMS);
                    ctx.body.i32_or();
                    ctx.body.if_i32();
                    ctx.body.local_get(a);
                    ctx.body.local_get(b);
                    ctx.body.call(idx);
                    ctx.body.else_();
                    ctx.body.i32_const(self.binop_code(op)?);
                    ctx.body.local_get(a);
                    ctx.body.local_get(b);
                    ctx.body.call(RT_BINOP);
                    if *op == "/" {
                        let origin = self.origin_lit(&ctx.prefix, &ctx.hako, *span);
                        ctx.body.i32_const(origin as i64);
                        ctx.body.call(RT_ERR_STAMP);
                    }
                    ctx.body.end();
                    return Ok(());
                }
                let code = self.binop_code(op)?;
                ctx.body.i32_const(code);
                self.emit_expr(ctx, lhs, false)?;
                self.emit_expr(ctx, rhs, false)?;
                ctx.body.call(RT_BINOP);
                if *op == "/" {
                    let origin = self.origin_lit(&ctx.prefix, &ctx.hako, *span);
                    ctx.body.i32_const(origin as i64);
                    ctx.body.call(RT_ERR_STAMP);
                }
            }
            // `(&roll 4) 5` is one application seen whole, so it lowers to the
            // call it means and dispatch happens on the total count
            Expr::App { head, args, piped, span } if matches!(head.as_ref(), Expr::App { head: inner, .. } if matches!(inner.as_ref(), Expr::Partial(..))) =>
            {
                let Expr::App { head: inner, args: held, .. } = head.as_ref() else {
                    unreachable!()
                };
                let Expr::Partial(name, nspan) = inner.as_ref() else { unreachable!() };
                let mut all = held.clone();
                all.extend(args.iter().cloned());
                let callee = Expr::Ident(name.clone(), *nspan);
                let call =
                    Expr::App { head: Box::new(callee), args: all, piped: *piped, span: *span };
                return self.emit_expr(ctx, &call, false);
            }
            Expr::App { head, args, span, .. } if matches!(head.as_ref(), Expr::Partial(..)) => {
                let Expr::Partial(name, _) = head.as_ref() else { unreachable!() };
                let lambda = partial_lambda(self.program, name, args, *span)?;
                return self.emit_expr(ctx, &lambda, false);
            }
            Expr::App { head, args, piped, span } => {
                self.emit_app(ctx, head, args, *piped, tail, *span)?
            }
        }
        Ok(())
    }

    fn binop_code(&self, op: &str) -> Result<i64, String> {
        match op {
            "+" => Ok(0),
            "-" => Ok(1),
            "*" => Ok(2),
            "/" => Ok(3),
            "%" => Ok(4),
            "==" => Ok(10),
            "!=" => Ok(11),
            "<" => Ok(12),
            ">" => Ok(13),
            "<=" => Ok(14),
            ">=" => Ok(15),
            "&" => Ok(20),
            "|" => Ok(21),
            "^" => Ok(22),
            other => Err(format!("unsupported operator `{other}`")),
        }
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
        if self
            .program
            .types
            .iter()
            .any(|t| t.name == name && t.fields.is_empty() && t.parent.is_none())
        {
            let tid = self.type_ids[name];
            ctx.body.i32_const(tid);
            ctx.body.i32_const(0);
            ctx.body.call(RT_MKREC);
            return Ok(());
        }
        if let Some(tidx) = self.const_cell(name) {
            ctx.body.i32_const(tidx as i64);
            ctx.body.call(RT_CONST);
            // a mention from inside the cycle is what the cell is for; one
            // from outside it wants the value the cell settled on
            if !self.knotted.contains(&ctx.group) {
                ctx.body.call(RT_FORCE);
            }
            return Ok(());
        }
        if let Some(idx) = self.dispatchers.get(&(name.to_string(), 0)).copied() {
            match tail && self.tailcalls {
                true => ctx.body.return_call(idx),
                false => ctx.body.call(idx),
            }
            return Ok(());
        }
        // std wrappers name the natives through the builtin_ prefix, the
        // same normalization every other site does
        let bare = name.strip_prefix("builtin_").unwrap_or(name);
        match bare {
            "true" | "false" | "none" => {
                let lit = self.nullary_lit(bare);
                ctx.body.i32_const(lit as i64);
            }
            "args" | "stdin" | "now" => {
                let lit = self.str_lit(bare);
                ctx.body.i32_const(lit as i64);
                ctx.body.i32_const(0);
                ctx.body.call(RT_BUILTIN);
            }
            // A builtin is handed over the same way a group is: `apply length
            // "ab"` reaches it through a dynamic call, which needs something
            // in the table to land on.
            // `print` joins them here rather than in the table the native
            // backend reads: this engine reaches every builtin through the
            // interpreter, which already renders a non-string argument
            // through the ambient group.
            _ if !self.program.fns.iter().any(|d| d.name == name)
                && (bare == "print"
                    || crate::codegen::BUILTIN_CALLS
                        .iter()
                        .any(|(b, a)| *b == bare && *a <= 4)) =>
            {
                let arity = match bare {
                    "print" => 1,
                    _ => {
                        crate::codegen::BUILTIN_CALLS
                            .iter()
                            .find(|(b, _)| *b == bare)
                            .expect("found")
                            .1
                    }
                };
                let widx = self.builtin_wrapper(bare, arity)?;
                ctx.body.i32_const(widx as i64);
                ctx.body.i32_const(0);
                ctx.body.i32_const(-1);
                ctx.body.call(RT_MKCLOSURE);
            }
            _ if self.program.fns.iter().any(|d| d.name == name) => {
                let widx = self.fn_wrapper(name)?;
                ctx.body.i32_const(widx as i64);
                ctx.body.i32_const(0);
                ctx.body.i32_const(-1);
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

    /// The table function a builtin handed out as a value lands on. A builtin
    /// takes exactly one count, so a call bringing another says which count
    /// it takes and which it brought — the sentence the interpreter prints,
    /// spelled once per count a dynamic call can carry.
    fn builtin_wrapper(&mut self, name: &str, arity: usize) -> Result<u32, String> {
        let key = format!("builtin.{name}");
        if let Some(widx) = self.wrappers.get(&key) {
            return Ok(*widx);
        }
        let fn_idx = self.module.declare(2);
        self.module.table.push(fn_idx);
        let widx = (self.module.table.len() - 1) as u32;
        self.wrappers.insert(key, widx);
        let mut body = Body::new(2);
        let len = body.local();
        body.local_get(1);
        body.call(RT_LIST_LEN);
        body.local_set(len);
        body.local_get(len);
        body.i32_const(arity as i64);
        body.op(0x46); // i32.eq
        body.if_void();
        for i in 0..arity {
            body.local_get(1);
            body.i32_const(i as i64);
            body.call(RT_ENVGET);
            body.call(RT_ARG);
        }
        let lit = self.str_lit(name);
        body.i32_const(lit as i64);
        body.i32_const(arity as i64);
        body.call(RT_BUILTIN);
        body.ret();
        body.end();
        for got in 1..=4 {
            if got == arity {
                continue;
            }
            let said = format!("`{name}` takes {arity} argument(s), got {got}");
            let msg = self.str_lit(&said);
            body.local_get(len);
            body.i32_const(got as i64);
            body.op(0x46); // i32.eq
            body.if_void();
            body.i32_const(msg as i64);
            body.call(RT_DIE);
            body.unreachable();
            body.end();
        }
        body.unreachable();
        self.module.define(fn_idx, body);
        Ok(widx)
    }

    /// Lambda lifting: the closure body becomes a table function taking
    /// (env, args); captures ride in env, both read via rt_envget.
    /// A list element or record field inside a constant that names itself
    /// waits: the cell it would read is still being filled, so the position
    /// holds the work and a later read runs it. Everything else is emitted
    /// where it stands.
    fn emit_element(&mut self, ctx: &mut Ctx, item: &Expr) -> Result<(), String> {
        if !self.defers_self(ctx, item) {
            return self.emit_expr(ctx, item, false);
        }
        let fn_idx = self.module.declare(2);
        self.module.table.push(fn_idx);
        let tidx = (self.module.table.len() - 1) as u32;
        let mut inner = Ctx {
            body: Body::new(2),
            scope: HashMap::default(),
            prefix: ctx.prefix.clone(),
            hako: ctx.hako.clone(),
            group: String::new(),
        };
        self.emit_expr(&mut inner, item, true)?;
        self.module.define(fn_idx, inner.body);
        ctx.body.i32_const(tidx as i64);
        ctx.body.call(RT_DEFER);
        Ok(())
    }

    /// Whether this expression names a constant the cycle comes back through.
    /// The free names such an element carries are constants, which the
    /// deferred body loads for itself, so nothing is captured. Two constants
    /// naming each other close a ring the same way one naming itself does, so
    /// the question is which knot the group belongs to rather than whether
    /// the element spells the group's own name.
    fn defers_self(&self, ctx: &Ctx, expr: &Expr) -> bool {
        fn mentions(expr: &Expr, of: &dyn Fn(&str) -> bool) -> bool {
            if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
                if of(n) {
                    return true;
                }
            }
            crate::any_child(expr, |c| mentions(c, of))
        }
        if ctx.group.is_empty() {
            return false;
        }
        match self.knotted.contains(&ctx.group) {
            true => mentions(expr, &|n: &str| self.knotted.contains(n)),
            false => mentions(expr, &|n: &str| n == ctx.group),
        }
    }

    /// A cell over the names it reads: the same closure `emit_lambda` builds,
    /// with no parameters and an arity no call site can ask for, which is what
    /// tells the runtime to demand it rather than call it.
    fn emit_cell(&mut self, ctx: &mut Ctx, body: &Expr) -> Result<(), String> {
        let mut captures: Vec<String> = Vec::new();
        free_idents(body, &mut |name| {
            if ctx.scope.contains_key(name) && !captures.iter().any(|c| c == name) {
                captures.push(name.to_string());
            }
        });
        let fn_idx = self.module.declare(2);
        self.module.table.push(fn_idx);
        let tidx = (self.module.table.len() - 1) as u32;
        let mut inner = Ctx {
            body: Body::new(2),
            scope: HashMap::default(),
            prefix: ctx.prefix.clone(),
            hako: ctx.hako.clone(),
            group: ctx.group.clone(),
        };
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
        ctx.body.i32_const(DEFERRED_ARITY);
        ctx.body.call(RT_MKCLOSURE);
        Ok(())
    }

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
        let mut inner = Ctx {
            body: Body::new(2),
            scope: HashMap::default(),
            prefix: ctx.prefix.clone(),
            hako: ctx.hako.clone(),
            group: String::new(),
        };
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
        ctx.body.i32_const(params.len() as i64);
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
        span: crate::diag::Span,
    ) -> Result<(), String> {
        if piped {
            return self.emit_piped(ctx, head, args, span);
        }
        // an inlined single-use binding can leave a lambda in call
        // position; it is an ordinary closure, built then applied. A value
        // keyword arrives the same way — inlining `list/map [1 2] none` puts
        // `none` where the callee goes — and the runtime names what it cannot
        // call, which is the sentence the other two engines print.
        let keyword_head =
            matches!(head, Expr::Ident(n, _) if matches!(n.as_str(), "true" | "false" | "none"));
        if keyword_head || matches!(head, Expr::Lambda { .. }) {
            for arg in args {
                self.emit_expr(ctx, arg, false)?;
                ctx.body.call(RT_ARG);
            }
            self.emit_expr(ctx, head, false)?;
            ctx.body.i32_const(args.len() as i64);
            ctx.body.call(RT_CALL);
            return Ok(());
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
            let origin = self.origin_lit(&ctx.prefix, &ctx.hako, span);
            ctx.body.i32_const(origin as i64);
            ctx.body.call(RT_MKERR);
            return Ok(());
        }
        // The three worded chain steps of the 2026-08-26 gavel. Named here
        // rather than left to the generic call path, which is where they went
        // before: the page compiled `rescue` and then propagated the failure
        // the other two engines catch, and nothing could see it because every
        // other fixture for these words needs a filesystem to fail an effect.
        // `annotate` takes the site it was written at, like `err`, because the
        // err it builds is a raise.
        if matches!(name.as_str(), "bind" | "rescue" | "annotate") {
            if args.len() != 2 {
                return Err(format!("`{name}` takes an effect and a callback"));
            }
            self.emit_expr(ctx, &args[0], false)?;
            self.emit_expr(ctx, &args[1], false)?;
            match name.as_str() {
                "bind" => ctx.body.call(RT_BIND),
                "rescue" => ctx.body.call(RT_RESCUE),
                _ => {
                    let origin = self.origin_lit(&ctx.prefix, &ctx.hako, span);
                    ctx.body.i32_const(origin as i64);
                    ctx.body.call(RT_ANNOTATE);
                }
            }
            return Ok(());
        }
        if let Some(tid) = self.type_ids.get(name.as_str()).copied() {
            let is_sub = self.program.types.iter().any(|t| t.name == *name && t.parent.is_some());
            if is_sub {
                if args.len() != 1 {
                    return Err(format!("wasm backend: `{name}` wraps one value"));
                }
                self.emit_expr(ctx, &args[0], false)?;
                ctx.body.i32_const(tid);
                ctx.body.call(RT_MKSUB);
                return Ok(());
            }
            let fields = self
                .program
                .types
                .iter()
                .find(|t| t.name == *name)
                .map(|t| t.fields.clone())
                .unwrap_or_default();
            for (i, arg) in args.iter().enumerate() {
                // A field of a constant that names itself waits for the same
                // reason a list element does: the cell it would read is still
                // being filled. Emitted where it stands, the mention calls the
                // constant again and the recursion has no floor.
                self.emit_element(ctx, arg)?;
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
        // builtin_ names bypass group dispatch (the bare-clone recursion trap)
        if let Some(stripped) = name.strip_prefix("builtin_") {
            if crate::check::BUILTINS.contains(&stripped) {
                for arg in args {
                    self.emit_expr(ctx, arg, false)?;
                    ctx.body.call(RT_ARG);
                }
                let lit = self.str_lit(stripped);
                ctx.body.i32_const(lit as i64);
                ctx.body.i32_const(args.len() as i64);
                ctx.body.call(RT_BUILTIN);
                self.stamp_fallible(ctx, stripped, span);
                return Ok(());
            }
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
            self.stamp_fallible(ctx, name, span);
            return Ok(());
        }
        // a constant holding a function value: evaluate it, then apply
        if let Some(idx) = self.dispatchers.get(&(name.clone(), 0)).copied() {
            for arg in args {
                self.emit_expr(ctx, arg, false)?;
                ctx.body.call(RT_ARG);
            }
            match self.const_cell(name) {
                Some(tidx) => {
                    ctx.body.i32_const(tidx as i64);
                    ctx.body.call(RT_CONST);
                    if !self.knotted.contains(&ctx.group) {
                        ctx.body.call(RT_FORCE);
                    }
                }
                None => ctx.body.call(idx),
            }
            ctx.body.i32_const(args.len() as i64);
            ctx.body.call(RT_CALL);
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
        let msg = self.str_lit(&format!("field `{field}` of `{ty_name}` takes {}", tys.join(" ")));
        ctx.body.i32_const(msg as i64);
        ctx.body.call(RT_DIE);
        ctx.body.unreachable();
        ctx.body.end();
        Ok(())
    }

    /// A piped application binds when the piped value is a description:
    /// the rest of the call becomes a continuation closure over the already
    /// evaluated arguments, mirroring the native emitter.
    /// Builtins that can give birth to an err get the site's origin stamped
    /// onto the fresh (still unstamped) err they return.
    fn stamp_fallible(&mut self, ctx: &mut Ctx, name: &str, span: crate::diag::Span) {
        // wrap_err mints an err too — through the generic builtin bridge,
        // where no frame exists, so the site's origin is stamped here
        if matches!(name, "to_int" | "to_float" | "utf8" | "from_code" | "wrap_err" | "to_bytes") {
            let origin = self.origin_lit(&ctx.prefix, &ctx.hako, span);
            ctx.body.i32_const(origin as i64);
            ctx.body.call(RT_ERR_STAMP);
        }
    }

    fn emit_piped(
        &mut self,
        ctx: &mut Ctx,
        head: &Expr,
        args: &[Expr],
        span: crate::diag::Span,
    ) -> Result<(), String> {
        let piped_local = ctx.body.local();
        self.emit_expr(ctx, &args[0], false)?;
        ctx.body.local_set(piped_local);
        let closure: Result<(), String> = match head {
            Expr::Ident(name, _) if self.dispatchers.contains_key(&(name.clone(), args.len())) => {
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
                ctx.body.i32_const(-1);
                ctx.body.call(RT_MKCLOSURE);
                Ok(())
            }
            Expr::Ident(name, _) if crate::check::BUILTINS.contains(&name.as_str()) => {
                let rest = args.len() - 1;
                let name_lit = self.str_lit(name);
                let fallible = matches!(
                    name.as_str(),
                    "to_int" | "to_float" | "utf8" | "from_code" | "to_bytes"
                );
                let origin = match fallible {
                    true => Some(self.origin_lit(&ctx.prefix, &ctx.hako, span)),
                    false => None,
                };
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
                if let Some(origin) = origin {
                    inner.i32_const(origin as i64);
                    inner.call(RT_ERR_STAMP);
                }
                self.module.define(fn_idx, inner);
                for arg in &args[1..] {
                    self.emit_expr(ctx, arg, false)?;
                    ctx.body.call(RT_ARG);
                }
                ctx.body.i32_const(tidx as i64);
                ctx.body.i32_const(rest as i64);
                ctx.body.i32_const(-1);
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
        Expr::Ident(name, _) | Expr::Partial(name, _) => visit(name),
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        free_idents(expr, visit)
                    }
                }
            }
        }
        Expr::Field { base, .. } => free_idents(base, visit),
        Expr::Upcast { expr, .. } => free_idents(expr, visit),
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
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            free_idents(lhs, visit);
            free_idents(rhs, visit);
        }
        Expr::Guard { cond, early, rest, .. } => {
            free_idents(cond, visit);
            free_idents(early, visit);
            for stmt in rest {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        free_idents(expr, visit)
                    }
                }
            }
        }
        Expr::App { head, args, .. } => {
            free_idents(head, visit);
            for arg in args {
                free_idents(arg, visit);
            }
        }
    }
}
