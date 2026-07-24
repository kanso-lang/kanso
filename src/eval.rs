use crate::ast::*;
use crate::diag::Span;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Int(BigInt),
    Str(String),
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(BigInt),
    Float(f64),
    Map(Rc<BTreeMap<MapKey, Value>>),
    Str(String),
    True,
    False,
    NoneV,
    ErrV(Rc<ErrInfo>),
    List(Rc<Vec<Value>>),
    Record { ty: Rc<str>, fields: Rc<RefCell<Vec<Value>>> },
    /// A nominal subtype wrapper: `post_body s`. Transparent — every
    /// consumer unwraps to the base; dispatch sees the chain.
    Sub { ty: Rc<str>, inner: Rc<Value> },
    FnRef(Rc<str>),
    Closure(Rc<ClosureData>),
    Desc(Rc<Desc>),
    Thunk(Rc<RefCell<ThunkState>>),
}

/// A conditionally-demanded binding (demand.rs marks the sites): the
/// computation and its captures, replaced by the value at first force.
#[derive(Clone, Debug)]
pub enum ThunkState {
    Pending { expr: Expr, env: Option<Rc<Env>>, frame: Frame },
    /// Under evaluation — a re-entrant force is a <<loop>>.
    Blackhole,
    Forced(Value),
}

/// An err value carries its propagation trace: the origin baked at the
/// construction site ("{fn} at {file}:{line}"; executor-born errs have none)
/// and one hop per dispatcher failure pass-through. The happy path never
/// touches any of this — trace data lives on the err value only.
#[derive(Debug)]
pub struct ErrInfo {
    pub reason: Value,
    pub origin: Option<Rc<str>>,
    pub hops: Vec<Rc<str>>,
}

/// The evaluation frame an expression runs in, as an err-origin prefix
/// "{fn} at {file}"; absent where no source frame exists (the wasm host).
pub type Frame = Option<Rc<str>>;

fn frame_of(decl: &FnDecl) -> Frame {
    Some(Rc::from(format!("{} at {}", decl.name, decl.file)))
}

pub fn origin_at(frame: &Frame, span: Span) -> Option<Rc<str>> {
    frame.as_ref().map(|prefix| Rc::from(format!("{prefix}:{}", span.line)))
}

pub fn err_value(reason: Value, origin: Option<Rc<str>>) -> Value {
    Value::ErrV(Rc::new(ErrInfo { reason, origin, hops: Vec::new() }))
}

/// A byte list (the scanner's `bytes`/`slice` view) as its utf-8 text, so a
/// number can be parsed straight from bytes without first materializing a
/// string. None when the bytes aren't valid utf-8 byte values.
fn bytes_to_str(items: &[Value]) -> Option<String> {
    let mut raw = Vec::with_capacity(items.len());
    for item in items {
        let Value::Int(n) = item else { return None };
        raw.push(u8::try_from(n.clone()).ok()?);
    }
    String::from_utf8(raw).ok()
}

/// A dispatcher passing a failure through appends its name; none stays bare.
pub fn hop(failure: Value, name: &str) -> Value {
    match failure {
        Value::ErrV(info) => {
            let mut hops = info.hops.clone();
            hops.push(Rc::from(name));
            Value::ErrV(Rc::new(ErrInfo {
                reason: info.reason.clone(),
                origin: info.origin.clone(),
                hops,
            }))
        }
        other => other,
    }
}

/// The endpoint report's trace lines, newest pass-through first, pointing
/// back toward the birth site. Byte-identical across all three engines.
pub fn trace_lines(info: &ErrInfo) -> String {
    let mut out = String::new();
    if let Some(origin) = &info.origin {
        out.push_str("  born in ");
        out.push_str(origin);
        out.push('\n');
    }
    if !info.hops.is_empty() {
        let path: Vec<&str> = info.hops.iter().rev().map(|h| &**h).collect();
        out.push_str("  passed through ");
        out.push_str(&path.join(" \u{2190} "));
        out.push('\n');
    }
    out
}

#[derive(Debug)]
pub struct ClosureData {
    pub params: Vec<String>,
    pub body: Expr,
    pub env: Option<Rc<Env>>,
    pub frame: Frame,
}

#[derive(Clone, Debug)]
pub enum Desc {
    Print(String, Span),
    Seq(Rc<Desc>, Rc<Desc>),
    Join(Rc<Desc>, Rc<Desc>),
    Args,
    Stdin,
    ReadFile(String),
    WriteFile(String, String),
    Bind(Rc<Desc>, Value),
    Sleep(u64),
    Random(u64),
    Nil,
}

/// SplitMix64: a deterministic, seedable generator. A real run draws its
/// seed from entropy so dice roll differently each time; KANSO_SEED pins
/// the stream, which is how the differential lattice, the goldens, and any
/// replay of a concurrent program stay byte-identical across engines.
pub struct Rng(u64);

impl Rng {
    pub fn seeded() -> Self {
        let seed = std::env::var("KANSO_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(entropy_seed);
        Rng(seed)
    }

    pub fn from_seed(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        match n {
            0 => 0,
            n => self.next_u64() % n,
        }
    }
}

/// The browser has no clock at thread-local init and reseeds through
/// `kanso_set_seed` before every run, so any fixed value serves there.
#[cfg(target_arch = "wasm32")]
fn entropy_seed() -> u64 {
    0x2545_F491_4F6C_DD1D
}

#[cfg(not(target_arch = "wasm32"))]
fn entropy_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x2545_F491_4F6C_DD1D);
    nanos ^ (u64::from(std::process::id()) << 32)
}

/// One step of a fiber: it either finished with a value, or blocked on a
/// `sleep` for `ms` with the rest of its work as the continuation. Blocking
/// propagates up through `Seq` and `Bind`, so `sleep` may sit anywhere in a
/// description and suspension needs no coroutine — the continuation is the
/// remaining Desc tree, made explicit.
enum Step {
    Done(Value),
    Blocked(u64, Rc<Desc>),
}

#[derive(Debug)]
pub struct Env {
    name: String,
    value: Value,
    parent: Option<Rc<Env>>,
}

fn bind(env: Option<Rc<Env>>, name: &str, value: Value) -> Option<Rc<Env>> {
    Some(Rc::new(Env { name: name.to_string(), value, parent: env }))
}

fn lookup(env: &Option<Rc<Env>>, name: &str) -> Option<Value> {
    let mut cur = env.as_ref();
    while let Some(frame) = cur {
        if frame.name == name {
            return Some(frame.value.clone());
        }
        cur = frame.parent.as_ref();
    }
    None
}

pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

type Bindings = Vec<(String, Value)>;
type Score = Vec<u8>;

type EvalResult = Result<Value, RuntimeError>;

pub trait Executor {
    fn print(&mut self, text: &str);
    fn args(&mut self) -> Vec<String>;
    fn stdin(&mut self) -> Result<String, String>;
    fn read_file(&mut self, path: &str) -> Result<String, String>;
    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String>;
    /// Pause wall-clock time. The scheduler decides output *order*; sleep only
    /// makes a concurrent program take real time, so a viewer feels the
    /// overlap. Default is virtual (no wait) — output is identical either way.
    fn sleep(&mut self, _ms: u64) {}
    /// A pseudo-random int in `[0, n)` off the executor's seeded generator.
    fn random(&mut self, _n: u64) -> u64 {
        0
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::seeded()
    }
}

pub struct RealExecutor {
    pub program_args: Vec<String>,
    pub rng: Rng,
}

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }

    fn sleep(&mut self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    fn random(&mut self, n: u64) -> u64 {
        self.rng.below(n)
    }

    fn args(&mut self) -> Vec<String> {
        self.program_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer).map_err(|e| e.to_string())?;
        Ok(buffer)
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String> {
        std::fs::write(path, content).map_err(|e| format!("cannot write {path}: {e}"))
    }
}

#[derive(Default)]
pub struct ScriptedExecutor {
    pub transcript: Vec<String>,
    pub script_args: Vec<String>,
    pub script_stdin: String,
    pub files: std::collections::HashMap<String, String>,
    pub rng: Rng,
}

impl Executor for ScriptedExecutor {
    fn print(&mut self, text: &str) {
        self.transcript.push(format!("print {text:?}"));
    }

    fn random(&mut self, n: u64) -> u64 {
        self.rng.below(n)
    }

    fn args(&mut self) -> Vec<String> {
        self.script_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Ok(self.script_stdin.clone())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        self.transcript.push(format!("read_file {path:?}"));
        self.files.get(path).cloned().ok_or_else(|| format!("cannot read {path}"))
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String> {
        self.transcript.push(format!("write_file {path:?} {content:?}"));
        Ok(())
    }
}

pub struct Interp<'a> {
    fns: HashMap<&'a str, Vec<&'a FnDecl>>,
    types: HashMap<&'a str, &'a TypeDecl>,
    entry_decl: TypeDecl,
    demand: crate::demand::DemandInfo,
    pub thunk_stats: ThunkStats,
}

/// Engine-shared semantic counters: evaluation counts are semantics, so
/// native must report byte-identical values (design/lazy-v1-plan.md).
#[derive(Default)]
pub struct ThunkStats {
    pub allocs: Cell<u64>,
    pub forces: Cell<u64>,
    pub evals: Cell<u64>,
}

impl ThunkStats {
    pub fn render(&self) -> String {
        let (allocs, forces, evals) = (self.allocs.get(), self.forces.get(), self.evals.get());
        format!(
            "thunk_allocs={allocs}\nthunk_forces={forces}\nthunk_evals={evals}\n"
        )
    }
}

impl<'a> Interp<'a> {
    pub fn new(program: &'a Program) -> Self {
        let mut fns: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        for decl in &program.fns {
            fns.entry(&decl.name).or_default().push(decl);
        }
        // proximity breaks specificity ties: local arms come before
        // bare-enrolled clones, so a same-shape local wins its own file
        for overloads in fns.values_mut() {
            overloads.sort_by_key(|d| d.synthetic);
        }
        let types = program.types.iter().map(|t| (t.name.as_str(), t)).collect();
        TYPESETS.with(|reg| {
            *reg.borrow_mut() = program
                .types
                .iter()
                .filter(|t| !t.members.is_empty())
                .map(|t| (t.name.clone(), t.members.clone()))
                .collect();
        });
        let origin = Span { line: 0, col: 0 };
        let entry_decl = TypeDecl {
            parent: None,
            members: Vec::new(),
            name: "entry".to_string(),
            is_pub: false,
            span: origin,
            synthetic: false,
            origin: None,
            fields: vec![
                ("key".to_string(), vec!["any".to_string()], origin),
                ("value".to_string(), vec!["any".to_string()], origin),
            ],
        };
        let demand = crate::demand::analyze(program);
        Interp { fns, types, entry_decl, demand, thunk_stats: ThunkStats::default() }
    }

    /// Evaluate a declaration's body with its lazy bind sites in view.
    fn eval_body_of(&self, decl: &FnDecl, env: Option<Rc<Env>>) -> EvalResult {
        self.eval_body_in(decl, &decl.body, env, &frame_of(decl))
    }

    fn type_decl(&self, name: &str) -> Option<&TypeDecl> {
        match name {
            "entry" => Some(&self.entry_decl),
            _ => self.types.get(name).copied(),
        }
    }

    pub fn run_main(&self) -> EvalResult {
        let main = self.fns.get("main").expect("checked: main exists")[0];
        self.eval_body_of(main, None)
    }

    pub fn run_named(&self, name: &str) -> Option<EvalResult> {
        let decl = self.fns.get(name)?.iter().find(|d| d.params.is_empty())?;
        Some(self.eval_body_of(decl, None))
    }

    fn eval_body_in(
        &self,
        decl: &FnDecl,
        body: &[Stmt],
        mut env: Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        let mut result = Value::NoneV;
        for (index, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Set { .. } => unreachable!("`set` parses only inside `build`"),
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), index) =>
                {
                    self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
                    let cell = Rc::new(RefCell::new(ThunkState::Pending {
                        expr: expr.clone(),
                        env: env.clone(),
                        frame: frame.clone(),
                    }));
                    env = bind(env, name, Value::Thunk(cell));
                }
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self.destructure(pattern, value, env, expr.span())?;
                }
                Stmt::Expr(expr) => result = self.eval(expr, &env, frame)?,
            }
        }
        Ok(result)
    }


    fn destructure(
        &self,
        pattern: &Pattern,
        value: Value,
        env: Option<Rc<Env>>,
        span: Span,
    ) -> Result<Option<Rc<Env>>, RuntimeError> {
        match pattern {
            Pattern::Var(name, _) => Ok(bind(env, name, value)),
            Pattern::Ctor { ty, .. } => {
                let mut binds = Vec::new();
                match match_one(pattern, &value, &mut binds) {
                    Some(_) => {
                        let mut env = env;
                        for (name, bound) in binds {
                            env = bind(env, &name, bound);
                        }
                        Ok(env)
                    }
                    None => Err(RuntimeError {
                        message: format!(
                            "cannot destructure {} as `{ty}`; bindings are irrefutable, so \
                             handle other types by dispatch first",
                            render(&value, true)
                        ),
                        span,
                    }),
                }
            }
            Pattern::Keyed { entries, .. } => {
                let Value::Record { ty, fields } = &value else {
                    return Err(RuntimeError {
                        message: format!(
                            "cannot read fields of {}; keyed reads take a record",
                            render(&value, true)
                        ),
                        span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                if entries.len() >= decl.fields.len() {
                    return Err(RuntimeError {
                        message: "a keyed read omits at least one field; reading every \
                                  field is the positional form"
                            .to_string(),
                        span,
                    });
                }
                let mut env = env;
                for entry in entries {
                    let position =
                        decl.fields.iter().position(|(name, _, _)| *name == entry.field);
                    let Some(position) = position else {
                        return Err(RuntimeError {
                            message: format!("`{ty}` has no field `{}`", entry.field),
                            span,
                        });
                    };
                    env = bind(env, &entry.bind_name, fields.borrow()[position].clone());
                }
                Ok(env)
            }
            _ => Err(RuntimeError {
                message: "binding patterns are irrefutable: names and constructor \
                          patterns only"
                    .to_string(),
                span,
            }),
        }
    }

    fn eval(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        match expr {
            Expr::Int(n, _) => Ok(Value::Int(n.clone())),
            Expr::Upcast { expr: inner, ty, span } => {
                let v = self.force_thunk(self.eval(inner, env, frame)?)?;
                if is_failure(&v) {
                    return Ok(v);
                }
                let mut cur = v;
                loop {
                    if type_matches_exact(ty, &cur) {
                        return Ok(cur);
                    }
                    match cur {
                        Value::Sub { inner, .. } => cur = (*inner).clone(),
                        _ => {
                            return Err(RuntimeError {
                                message: format!(
                                    "`:{ty}` widens; this value is not a {ty}"
                                ),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
                // a deferred branch body: fn-body statements in a child
                // scope; the env extension is dropped with this frame
                let mut env = env.clone();
                let mut result = Value::NoneV;
                for stmt in stmts {
                    match stmt {
                        Stmt::Bind { pattern, expr } => {
                            let mut value = self.eval(expr, &env, frame)?;
                            if !matches!(pattern, Pattern::Var(..)) {
                                value = self.force_thunk(value)?;
                            }
                            env = self.destructure(pattern, value, env, expr.span())?;
                        }
                        Stmt::Expr(expr) => result = self.eval(expr, &env, frame)?,
                        Stmt::Set { target, field, value, span } => {
                            let current = lookup(&env, target).ok_or_else(|| {
                                RuntimeError {
                                    message: format!("`set` target `{target}` is not bound"),
                                    span: *span,
                                }
                            })?;
                            let current = self.force_thunk(current)?;
                            // a failure target propagates: a constructor that
                            // took a failure argument already handed back that
                            // failure, so the block-born target is not a record.
                            // the write is skipped, exactly as native's
                            // k_set_field returns early on a failure target.
                            if is_failure(&current) {
                                continue;
                            }
                            let Value::Record { ty, fields } = &current else {
                                return Err(RuntimeError {
                                    message: format!(
                                        "`set` writes a record field, not {}",
                                        render(&current, true)
                                    ),
                                    span: *span,
                                });
                            };
                            let decl =
                                self.type_decl(ty).expect("constructed types are declared");
                            let position =
                                decl.fields.iter().position(|(f, _, _)| f == field);
                            let Some(position) = position else {
                                return Err(RuntimeError {
                                    message: format!("`{ty}` has no field `{field}`"),
                                    span: *span,
                                });
                            };
                            let new = self.eval(value, &env, frame)?;
                            if is_failure(&new) {
                                return Ok(new);
                            }
                            fields.borrow_mut()[position] = new;
                        }
                    }
                }
                Ok(result)
            }
            Expr::Float(x, _) => Ok(Value::Float(*x)),
            Expr::MapLit(pairs, span) => {
                let mut entries = BTreeMap::new();
                for (key_expr, value_expr) in pairs {
                    let key = self.eval(key_expr, env, frame)?;
                    let value = self.eval(value_expr, env, frame)?;
                    if is_failure(&key) {
                        return Ok(key);
                    }
                    if is_failure(&value) {
                        return Ok(value);
                    }
                    let key = map_key(key, *span)?;
                    entries.insert(key, value);
                }
                Ok(Value::Map(Rc::new(entries)))
            }
            Expr::Str(parts, _) => self.eval_template(parts, env, frame),
            Expr::Ident(name, span) => self.eval_ident(name, *span, env),
            Expr::List(items, _) => {
                let values = items
                    .iter()
                    .map(|e| self.eval(e, env, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(values)))
            }
            Expr::Field { base, name, span } => {
                let value = self.eval(base, env, frame)?;
                if is_failure(&value) {
                    return Ok(value);
                }
                let Value::Record { ty, fields } = &value else {
                    return Err(RuntimeError {
                        message: format!(
                            "`.` reads a field of a record, not {}",
                            render(&value, true)
                        ),
                        span: *span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                let position = decl.fields.iter().position(|(f, _, _)| f == name);
                match position {
                    Some(position) => Ok(fields.borrow()[position].clone()),
                    None => Err(RuntimeError {
                        message: format!("`{ty}` has no field `{name}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.force_thunk(self.eval(base, env, frame)?)?;
                let key = self.force_thunk(self.eval(index, env, frame)?)?;
                match index_value(container, key.clone(), *span)? {
                    Value::NoneV if *strict => Ok(err_value(
                        Value::Str(format!("missing index {}", render(&key, true))),
                        origin_at(frame, *span),
                    )),
                    found => Ok(found),
                }
            }
            Expr::App { head, args, span, piped } => {
                if *piped && !args.is_empty() {
                    let piped_value = self.eval(&args[0], env, frame)?;
                    if is_failure(&piped_value) {
                        return Ok(piped_value);
                    }
                    if let Value::Desc(inner) = piped_value {
                        let mut body_args: Vec<Expr> =
                            vec![Expr::Ident("__piped".to_string(), *span)];
                        body_args.extend(args[1..].iter().cloned());
                        let closure = Value::Closure(Rc::new(ClosureData {
                            params: vec!["__piped".to_string()],
                            body: Expr::App {
                                head: head.clone(),
                                args: body_args,
                                span: *span,
                                piped: false,
                            },
                            env: env.clone(),
                            frame: frame.clone(),
                        }));
                        return Ok(Value::Desc(Rc::new(Desc::Bind(inner, closure))));
                    }
                    let callee = self.eval(head, env, frame)?;
                    let mut values = vec![piped_value];
                    for arg in &args[1..] {
                        values.push(self.eval(arg, env, frame)?);
                    }
                    return self.call(callee, values, *span, frame);
                }
                let callee = self.eval(head, env, frame)?;
                let lazy_if = matches!(&callee, Value::FnRef(name) if &**name == "if");
                let mut values = Vec::new();
                for arg in args {
                    match lazy_if {
                        true => values.push(Value::Closure(Rc::new(ClosureData {
                            params: Vec::new(),
                            body: arg.clone(),
                            env: env.clone(),
                            frame: frame.clone(),
                        }))),
                        false => values.push(self.eval(arg, env, frame)?),
                    }
                }
                self.call(callee, values, *span, frame)
            }
            Expr::Seq(lhs, rhs, span) => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                if is_failure(&left) {
                    return Ok(left);
                }
                if is_failure(&right) {
                    return Ok(right);
                }
                match (left, right) {
                    (Value::Desc(a), Value::Desc(b)) => {
                        Ok(Value::Desc(Rc::new(Desc::Seq(a, b))))
                    }
                    _ => Err(RuntimeError {
                        message: "`>>` sequences two effect descriptions".to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Lambda { params, body, .. } => Ok(Value::Closure(Rc::new(ClosureData {
                params: params.iter().map(|(n, _)| n.clone()).collect(),
                body: (**body).clone(),
                env: env.clone(),
                frame: frame.clone(),
            }))),
            Expr::BinOp { op, lhs, rhs, span } => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                // records dispatch to the operator's user arms; numbers stay
                // on the builtin (coherence licenses the fast path, and the
                // orphan rule keeps 2 + 3 meaning one thing forever)
                if matches!(&left, Value::Record { .. })
                    && !is_failure(&left)
                    && self.fns.contains_key(op as &str)
                {
                    return self.call_named(op, vec![left, right], *span, frame);
                }
                eval_binop(op, sub_base(left), sub_base(right), *span, frame)
            }
            Expr::Join { lhs, rhs, span } => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                join_values(left, right, *span)
            }
        }
    }

    fn eval_ident(&self, name: &str, span: Span, env: &Option<Rc<Env>>) -> EvalResult {
        if let Some(value) = lookup(env, name) {
            return Ok(value);
        }
        if let Some(decls) = self.fns.get(name) {
            if let Some(constant) = decls.iter().find(|d| d.params.is_empty()) {
                return self.eval_body_of(constant, None);
            }
        }
        match name.strip_prefix("builtin_").unwrap_or(name) {
            "args" => return Ok(Value::Desc(Rc::new(Desc::Args))),
            "stdin" => return Ok(Value::Desc(Rc::new(Desc::Stdin))),
            _ => {}
        }
        if let Some(decl) = self.type_decl(name) {
            if decl.parent.is_none() && decl.members.is_empty() && decl.fields.is_empty() {
                return Ok(Value::Record { ty: Rc::from(name), fields: Rc::new(RefCell::new(Vec::new())) });
            }
        }
        match name {
            "true" => Ok(Value::True),
            "false" => Ok(Value::False),
            "none" => Ok(Value::NoneV),
            _ if self.fns.contains_key(name)
                || self.types.contains_key(name)
                || name == "err"
                || crate::check::BUILTINS.contains(&name)
                || name
                    .strip_prefix("builtin_")
                    .is_some_and(|n| crate::check::BUILTINS.contains(&n)) =>
            {
                Ok(Value::FnRef(Rc::from(name)))
            }
            _ => Err(RuntimeError { message: format!("unknown name `{name}`"), span }),
        }
    }

    fn eval_template(
        &self,
        parts: &[TemplatePart],
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        let mut out = String::new();
        for part in parts {
            match part {
                TemplatePart::Lit(s) => out.push_str(s),
                TemplatePart::Interp(expr) => {
                    let value = self.force_thunk(self.eval(expr, env, frame)?)?;
                    // an err propagates (the whole string returns the err); a
                    // none is a value and renders its sentinel
                    if matches!(value, Value::ErrV(_)) {
                        return Ok(value);
                    }
                    match self.render_interpolated(value)? {
                        Ok(rendered) => out.push_str(&rendered),
                        Err(err) => return Ok(err),
                    }
                }
            }
        }
        Ok(Value::Str(out))
    }

    fn call(&self, callee: Value, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        match callee {
            Value::FnRef(name) => self.call_named(&name, args, span, frame),
            Value::Closure(closure) => self.call_closure(&closure, args, span),
            bad if is_failure(&bad) => Ok(bad),
            other => Err(RuntimeError {
                message: format!("`{}` is not callable", render(&other, false)),
                span,
            }),
        }
    }

    fn call_closure(&self, closure: &ClosureData, args: Vec<Value>, span: Span) -> EvalResult {
        if closure.params.len() != args.len() {
            return Err(RuntimeError {
                message: format!(
                    "this function takes {} argument(s), got {}",
                    closure.params.len(),
                    args.len()
                ),
                span,
            });
        }
        if let Some(bad) = args.iter().find(|a| is_failure(a)) {
            return Ok(bad.clone());
        }
        let mut env = closure.env.clone();
        for (name, value) in closure.params.iter().zip(args) {
            env = bind(env, name, value);
        }
        self.eval(&closure.body, &env, &closure.frame)
    }

    fn call_named(&self, name: &str, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        if name == "err" {
            let [reason] = arity(args, name, span)?;
            let reason = self.force_thunk(reason)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            return Ok(err_value(reason, origin_at(frame, span)));
        }
        if let Some(ty) = self.type_decl(name) {
            let args = args
                .into_iter()
                .map(|a| self.force_thunk(a))
                .collect::<Result<Vec<_>, _>>()?;
            return self.construct(ty, args, span);
        }
        if let Some(overloads) = self.fns.get(name) {
            return self.dispatch(name, overloads, args, span);
        }
        let args = args
            .into_iter()
            .map(|a| self.force_thunk(a))
            .collect::<Result<Vec<_>, _>>()?;
        self.call_builtin(name, args, span, frame)
    }

    fn construct(&self, ty: &TypeDecl, args: Vec<Value>, span: Span) -> EvalResult {
        if !ty.members.is_empty() {
            return Err(RuntimeError {
                message: format!("`{}` is a typeset — it only annotates", ty.name),
                span,
            });
        }
        if let Some(parent) = &ty.parent {
            if args.len() != 1 {
                return Err(RuntimeError {
                    message: format!("`{}` wraps one {} value", ty.name, parent),
                    span,
                });
            }
            let inner = args.into_iter().next().expect("one arg");
            if is_failure(&inner) {
                return Ok(inner);
            }
            if !type_matches(parent, &inner) {
                return Err(RuntimeError {
                    message: format!("`{}` wraps a {}", ty.name, parent),
                    span,
                });
            }
            let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
            return Ok(Value::Sub { ty: Rc::from(canonical), inner: Rc::new(inner) });
        }
        if args.len() != ty.fields.len() {
            return Err(RuntimeError {
                message: format!(
                    "`{}` has {} field(s), got {} (construction is positional, fields \
                     alphabetical)",
                    ty.name,
                    ty.fields.len(),
                    args.len()
                ),
                span,
            });
        }
        if let Some(bad) = args.iter().find(|a| is_failure(a)) {
            return Ok(bad.clone());
        }
        for ((field, tys, _), arg) in ty.fields.iter().zip(&args) {
            if tys.len() >= 2 && !tys.iter().any(|t| type_matches(t, arg)) {
                return Err(RuntimeError {
                    message: format!(
                        "field `{field}` of `{}` takes {}",
                        ty.name,
                        tys.join(" ")
                    ),
                    span,
                });
            }
        }
        let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
        Ok(Value::Record { ty: Rc::from(canonical), fields: Rc::new(RefCell::new(args)) })
    }

    fn dispatch(
        &self,
        name: &str,
        overloads: &[&FnDecl],
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let args_len = args.len();
        // A position is scrutinized when any arity-matching arm inspects it
        // (anything but a bare Var/Wildcard); thunks force before matching.
        let mut args = args;
        for (i, arg) in args.iter_mut().enumerate() {
            if !matches!(arg, Value::Thunk(_)) {
                continue;
            }
            let scrutinized = overloads.iter().any(|decl| {
                decl.params.len() == args_len
                    && !matches!(
                        decl.params.get(i),
                        Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_))
                    )
            });
            if scrutinized {
                let taken = std::mem::replace(arg, Value::NoneV);
                *arg = self.force_thunk(taken)?;
            }
        }
        let mut best: Option<(Score, &FnDecl, Bindings)> = None;
        for decl in overloads {
            if decl.params.len() != args.len() {
                continue;
            }
            let Some((score, binds)) = match_params(&decl.params, &args) else { continue };
            let replace = match &best {
                Some((best_score, ..)) => score > *best_score,
                None => true,
            };
            if replace {
                best = Some((score, decl, binds));
            }
        }
        match best {
            Some((_, decl, binds)) => {
                let mut env = None;
                for (bind_name, value) in binds {
                    env = bind(env, &bind_name, value);
                }
                self.eval_body_of(decl, env)
            }
            None => match args.into_iter().find(is_failure) {
                Some(bad) => Ok(hop(bad, name)),
                None => Err(RuntimeError {
                    message: format!("no overload of `{name}` matches these arguments"),
                    span,
                }),
            },
        }
    }

    pub fn call_builtin(
        &self,
        name: &str,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        // std wrapper modules reach natives through the builtin_ prefix;
        // the checker gates those names to std-origin files
        let name = name.strip_prefix("builtin_").unwrap_or(name);
        let args: Vec<Value> = args.into_iter().map(sub_base).collect();
        if name == "if" {
            return self.builtin_if(args, span);
        }
        if let Some(bad) = args.iter().find(|a| is_failure(a)) {
            return Ok(bad.clone());
        }
        match name {
            "read_file" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError {
                        message: "read_file takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::ReadFile(path))))
            }
            "write_file" => {
                let [path, content] = arity(args, name, span)?;
                let (Value::Str(path), Value::Str(content)) = (&path, &content) else {
                    return Err(RuntimeError {
                        message: "write_file takes a path and content strings".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::WriteFile(path.clone(), content.clone()))))
            }
            "sleep" => {
                let [ms] = arity(args, name, span)?;
                match ms {
                    Value::Int(n) => {
                        let ms = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Sleep(ms))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("sleep takes milliseconds (an int), got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "random" => {
                let [n] = arity(args, name, span)?;
                match n {
                    Value::Int(n) => {
                        let bound = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Random(bound))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("random takes a bound (an int), got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "print" => {
                let [text] = arity(args, name, span)?;
                match text {
                    Value::Str(s) => Ok(Value::Desc(Rc::new(Desc::Print(s, span)))),
                    // any other value renders through the same ambient
                    // to_string dispatch interpolation uses
                    other => {
                        if is_failure(&other) {
                            return Ok(other);
                        }
                        match self.render_interpolated(other)? {
                            Ok(rendered) => {
                                Ok(Value::Desc(Rc::new(Desc::Print(rendered, span))))
                            }
                            Err(failure) => Ok(failure),
                        }
                    }
                }
            }
            "at" => {
                let [container, index] = arity(args, name, span)?;
                index_value(container, index, span)
            }
            "push" => {
                let [list, item] = arity(args, name, span)?;
                let Value::List(items) = &list else {
                    return Err(RuntimeError {
                        message: "push takes a list and a value".to_string(),
                        span,
                    });
                };
                let mut next = (**items).clone();
                next.push(item);
                Ok(Value::List(Rc::new(next)))
            }
            "put" => {
                let [map, key, value] = arity(args, name, span)?;
                let Value::Map(entries) = &map else {
                    return Err(RuntimeError {
                        message: "put takes a map, a key, and a value".to_string(),
                        span,
                    });
                };
                let key = map_key(key, span)?;
                let mut next = (**entries).clone();
                next.insert(key, value);
                Ok(Value::Map(Rc::new(next)))
            }
            "entries" => {
                let [map] = arity(args, name, span)?;
                let Value::Map(map_entries) = &map else {
                    return Err(RuntimeError {
                        message: "entries takes a map".to_string(),
                        span,
                    });
                };
                let list = map_entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => Value::Int(n.clone()),
                            MapKey::Str(s) => Value::Str(s.clone()),
                        };
                        Value::Record {
                            ty: Rc::from("entry"),
                            fields: Rc::new(RefCell::new(vec![key, value.clone()])),
                        }
                    })
                    .collect();
                Ok(Value::List(Rc::new(list)))
            }
            "bytes" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError {
                        message: "bytes takes a string".to_string(),
                        span,
                    });
                };
                let list = text.bytes().map(|b| Value::Int(BigInt::from(b))).collect();
                Ok(Value::List(Rc::new(list)))
            }
            "concat" => {
                let [a, b] = arity(args, name, span)?;
                let (Value::List(xs), Value::List(ys)) = (&a, &b) else {
                    return Err(RuntimeError {
                        message: "concat takes two lists".to_string(),
                        span,
                    });
                };
                let mut joined = (**xs).clone();
                joined.extend(ys.iter().cloned());
                Ok(Value::List(Rc::new(joined)))
            }
            "utf8" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = &list else {
                    return Err(RuntimeError {
                        message: "utf8 takes a list of byte values".to_string(),
                        span,
                    });
                };
                let mut raw = Vec::with_capacity(items.len());
                for item in items.iter() {
                    match item {
                        Value::Int(n) => match u8::try_from(n.clone()) {
                            Ok(b) => raw.push(b),
                            Err(_) => {
                                return Ok(err_value(
                                    Value::Str("utf8 takes byte values (0-255)".to_string()),
                                    origin_at(frame, span),
                                ))
                            }
                        },
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "utf8 takes a list of byte values".to_string(),
                                span,
                            })
                        }
                    }
                }
                match String::from_utf8(raw) {
                    Ok(text) => Ok(Value::Str(text)),
                    Err(_) => Ok(err_value(
                        Value::Str("invalid utf-8".to_string()),
                        origin_at(frame, span),
                    )),
                }
            }
            "chars" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError {
                        message: "chars takes a string".to_string(),
                        span,
                    });
                };
                let list = text.chars().map(|c| Value::Str(c.to_string())).collect();
                Ok(Value::List(Rc::new(list)))
            }
            "char_code" => {
                let [c] = arity(args, name, span)?;
                let code = match &c {
                    Value::Str(s) if s.chars().count() == 1 => {
                        s.chars().next().expect("length checked") as u32
                    }
                    _ => {
                        return Err(RuntimeError {
                            message: "char_code takes a one-character string".to_string(),
                            span,
                        })
                    }
                };
                Ok(Value::Int(BigInt::from(code)))
            }
            "from_code" => {
                let [code] = arity(args, name, span)?;
                let Value::Int(n) = &code else {
                    return Err(RuntimeError {
                        message: "from_code takes an int".to_string(),
                        span,
                    });
                };
                let scalar = u32::try_from(n.clone()).ok().and_then(char::from_u32);
                match scalar {
                    Some(c) => Ok(Value::Str(c.to_string())),
                    None => Ok(err_value(
                        Value::Str("not a unicode scalar value".to_string()),
                        origin_at(frame, span),
                    )),
                }
            }
            "join" => {
                let [list, sep] = arity(args, name, span)?;
                let (Value::List(items), Value::Str(sep)) = (&list, &sep) else {
                    return Err(RuntimeError {
                        message: "join takes a list of strings and a separator".to_string(),
                        span,
                    });
                };
                let mut parts = Vec::new();
                for item in items.iter() {
                    match item {
                        Value::Str(s) => parts.push(s.clone()),
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "join takes a list of strings".to_string(),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::Str(parts.join(sep)))
            }
            "append" => {
                let [acc, x] = arity(args, name, span)?;
                for v in [&acc, &x] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let Value::List(items) = &acc else {
                    return Err(RuntimeError {
                        message: "append takes bytes and a string, bytes, or byte".to_string(),
                        span,
                    });
                };
                let mut out = (**items).clone();
                match &x {
                    Value::Str(s) => {
                        out.extend(s.bytes().map(|b| Value::Int(BigInt::from(b))));
                    }
                    Value::List(more) => out.extend(more.iter().cloned()),
                    Value::Int(_) => out.push(x.clone()),
                    _ => {
                        return Err(RuntimeError {
                            message: "append takes bytes and a string, bytes, or byte"
                                .to_string(),
                            span,
                        })
                    }
                }
                Ok(Value::List(Rc::new(out)))
            }
            "find2_below" => {
                let [cs, from, a, b, lim] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b, &lim] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (
                    Value::List(items),
                    Value::Int(from),
                    Value::Int(a),
                    Value::Int(b),
                    Value::Int(lim),
                ) = (&cs, &from, &a, &b, &lim)
                else {
                    return Err(RuntimeError {
                        message: "find2_below takes bytes".to_string(),
                        span,
                    });
                };
                let len = items.len();
                let start = usize::try_from(from.clone()).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, item) in items.iter().enumerate().skip(start - 1) {
                    if matches!(item, Value::Int(byte) if byte == a || byte == b || byte < lim) {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::Int(BigInt::from(at)))
            }
            "find2" => {
                let [cs, from, a, b] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::List(items), Value::Int(from), Value::Int(a), Value::Int(b)) =
                    (&cs, &from, &a, &b)
                else {
                    return Err(RuntimeError {
                        message: "find2 takes bytes".to_string(),
                        span,
                    });
                };
                let len = items.len();
                let start = usize::try_from(from.clone()).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, item) in items.iter().enumerate().skip(start - 1) {
                    if matches!(item, Value::Int(byte) if byte == a || byte == b) {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::Int(BigInt::from(at)))
            }
            "slice" => {
                let [container, from, to] = arity(args, name, span)?;
                let (Value::Int(from), Value::Int(to)) = (&from, &to) else {
                    return Err(RuntimeError {
                        message: "slice takes 1-based inclusive positions".to_string(),
                        span,
                    });
                };
                let from = usize::try_from(from.clone()).unwrap_or(0);
                let to = usize::try_from(to.clone()).unwrap_or(0);
                match &container {
                    Value::List(items) => {
                        let sliced = slice_range(items.len(), from, to)
                            .map(|r| items[r].to_vec())
                            .unwrap_or_default();
                        Ok(Value::List(Rc::new(sliced)))
                    }
                    Value::Str(text) => {
                        let all: Vec<char> = text.chars().collect();
                        let sliced = slice_range(all.len(), from, to)
                            .map(|r| all[r].iter().collect::<String>())
                            .unwrap_or_default();
                        Ok(Value::Str(sliced))
                    }
                    _ => Err(RuntimeError {
                        message: "slice takes a list or string".to_string(),
                        span,
                    }),
                }
            }
            "sqrt" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Float(v) => Ok(Value::Float(v.sqrt())),
                    Value::Int(n) => Ok(Value::Float(int_f(&n).sqrt())),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("sqrt takes a number, got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "round" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Int(n) => Ok(Value::Int(n)),
                    Value::Float(v) => Ok(Value::Int(BigInt::from(v.round() as i64))),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("round takes a number, got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "to_int" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::List(items) => match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str("bytes are not an integer".to_string()),
                                origin_at(frame, span),
                            ))
                        }
                    },
                    Value::Int(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            message: "to_int takes a string".to_string(),
                            span,
                        })
                    }
                };
                Ok(match text.parse::<BigInt>() {
                    Ok(n) => Value::Int(n),
                    Err(_) => err_value(
                        Value::Str(format!("\"{text}\" is not an integer")),
                        origin_at(frame, span),
                    ),
                })
            }
            "to_float" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::List(items) => match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str("bytes are not a number".to_string()),
                                origin_at(frame, span),
                            ))
                        }
                    },
                    Value::Int(n) => {
                        let approx = n.to_string().parse::<f64>().unwrap_or(f64::INFINITY);
                        return Ok(Value::Float(approx));
                    }
                    Value::Float(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            message: "to_float takes a string or int".to_string(),
                            span,
                        })
                    }
                };
                Ok(match text.parse::<f64>() {
                    Ok(x) => Value::Float(x),
                    Err(_) => err_value(
                        Value::Str(format!("\"{text}\" is not a number")),
                        origin_at(frame, span),
                    ),
                })
            }
            "render_value" => {
                let [v] = arity(args, name, span)?;
                Ok(Value::Str(render(&v, false)))
            }
            "length" => {
                let [list] = arity(args, name, span)?;
                match list {
                    Value::List(items) => Ok(Value::Int(BigInt::from(items.len()))),
                    Value::Str(s) => Ok(Value::Int(BigInt::from(s.chars().count()))),
                    Value::Map(entries) => Ok(Value::Int(BigInt::from(entries.len()))),
                    _ => Err(RuntimeError {
                        message: "length takes a list or string".to_string(),
                        span,
                    }),
                }
            }
            "map" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "map takes a list".to_string(), span });
                };
                let mapped = items
                    .iter()
                    .map(|item| self.call(f.clone(), vec![item.clone()], span, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(mapped)))
            }
            "filter" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "filter takes a list".to_string(), span });
                };
                let mut kept = Vec::new();
                for item in items.iter() {
                    match self.call(f.clone(), vec![item.clone()], span, frame)? {
                        Value::True => kept.push(item.clone()),
                        Value::False => {}
                        other => {
                            return Err(RuntimeError {
                                message: format!(
                                    "a filter predicate returns true or false, got {}",
                                    render(&other, false)
                                ),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::List(Rc::new(kept)))
            }
            "sort" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "sort takes a list".to_string(), span });
                };
                let mut sorted = (*items).clone();
                let mut failed = false;
                sorted.sort_by(|a, b| match compare(a, b) {
                    Some(ord) => ord,
                    None => {
                        failed = true;
                        std::cmp::Ordering::Equal
                    }
                });
                match failed {
                    true => Err(RuntimeError {
                        message: "sort requires comparable elements of one type".to_string(),
                        span,
                    }),
                    false => Ok(Value::List(Rc::new(sorted))),
                }
            }
            "sum" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "sum takes a list".to_string(), span });
                };
                let mut total = BigInt::zero();
                for item in items.iter() {
                    match item {
                        Value::Int(n) => total += n,
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "sum takes a list of int".to_string(),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::Int(total))
            }
            _ => Err(RuntimeError { message: format!("unknown builtin `{name}`"), span }),
        }
    }

    fn builtin_if(&self, args: Vec<Value>, span: Span) -> EvalResult {
        let [cond, then_branch, else_branch] = arity(args, "if", span)?;
        let cond = self.force(cond)?;
        match cond {
            Value::True => self.force(then_branch),
            Value::False => self.force(else_branch),
            bad if is_failure(&bad) => Ok(bad),
            other => Err(RuntimeError {
                message: format!(
                    "an if condition is true or false, got {}",
                    render(&other, false)
                ),
                span,
            }),
        }
    }

    /// One rendering rule for every engine's template path: records, none,
    /// and io route through the ambient render/to_string group so user arms
    /// win; primitives keep the direct renderer (coherence licenses it — no
    /// arm can exist for them). The outer Err is a runtime fault; the inner
    /// Err is an err value the whole template must return.
    pub fn render_interpolated(&self, value: Value) -> Result<Result<String, Value>, RuntimeError> {
        let rendered = match (&value, self.fns.get("render/to_string")) {
            (Value::Record { .. } | Value::NoneV | Value::Desc(_), Some(overloads)) => {
                let overloads = overloads.clone();
                let result = self.dispatch(
                    "render/to_string",
                    &overloads,
                    vec![value],
                    Span { line: 0, col: 0 },
                )?;
                if matches!(result, Value::ErrV(_)) {
                    return Ok(Err(result));
                }
                match result {
                    Value::Str(s) => s,
                    other => render(&other, false),
                }
            }
            _ => render(&value, false),
        };
        Ok(Ok(rendered))
    }

    fn force(&self, value: Value) -> EvalResult {
        match value {
            Value::Closure(c) if c.params.is_empty() => self.eval(&c.body, &c.env, &c.frame),
            other => self.force_thunk(other),
        }
    }

    /// Scrutiny reaches through a thunk: run the pending computation once,
    /// overwrite the cell, drop the captures. Never touches nullary closures
    /// — those are `if`'s deferred branches, forced only by `if` itself.
    fn force_thunk(&self, value: Value) -> EvalResult {
        match value {
            Value::Thunk(cell) => {
                self.thunk_stats.forces.set(self.thunk_stats.forces.get() + 1);
                let state = std::mem::replace(&mut *cell.borrow_mut(), ThunkState::Blackhole);
                match state {
                    ThunkState::Forced(v) => {
                        *cell.borrow_mut() = ThunkState::Forced(v.clone());
                        Ok(v)
                    }
                    ThunkState::Pending { expr, env, frame } => {
                        self.thunk_stats.evals.set(self.thunk_stats.evals.get() + 1);
                        let v = self.eval(&expr, &env, &frame)?;
                        *cell.borrow_mut() = ThunkState::Forced(v.clone());
                        Ok(v)
                    }
                    ThunkState::Blackhole => Err(RuntimeError {
                        message: "a lazy binding demands its own value".to_string(),
                        span: Span { line: 0, col: 0 },
                    }),
                }
            }
            other => Ok(other),
        }
    }
}

fn arity<const N: usize>(
    args: Vec<Value>,
    name: &str,
    span: Span,
) -> Result<[Value; N], RuntimeError> {
    match <[Value; N]>::try_from(args) {
        Ok(array) => Ok(array),
        Err(actual) => Err(RuntimeError {
            message: format!("`{name}` takes {N} argument(s), got {}", actual.len()),
            span,
        }),
    }
}

fn match_params(params: &[Pattern], args: &[Value]) -> Option<(Score, Bindings)> {
    let mut score = Vec::new();
    let mut binds = Vec::new();
    for (pattern, arg) in params.iter().zip(args) {
        // per-param: literals 200, annotated 100 minus subtype distance
        // (nearer declarations outrank ancestors), generics 10 — the old
        // three-rank ladder, widened so chain depth can order within a rank
        let base: u8 = match pattern.rank() {
            0 => 200,
            1 => 100,
            _ => 10,
        };
        let depth = match_one(pattern, arg, &mut binds)?;
        score.push(base.saturating_sub(depth));
    }
    Some((score, binds))
}

fn match_one(pattern: &Pattern, arg: &Value, binds: &mut Bindings) -> Option<u8> {
    match (pattern, arg) {
        (Pattern::IntLit(n, _), Value::Int(v)) if n == v => Some(0),
        (Pattern::StrLit(s, _), Value::Str(v)) if s == v => Some(0),
        (Pattern::Nullary(name, _), Value::True) if name == "true" => Some(0),
        (Pattern::Nullary(name, _), Value::False) if name == "false" => Some(0),
        (Pattern::Nullary(name, _), Value::NoneV) if name == "none" => Some(0),
        (Pattern::Wildcard(_), _) => match is_failure(arg) {
            true => None,
            false => Some(0),
        },
        (Pattern::Var(name, _), _) => match is_failure(arg) {
            true => None,
            false => {
                binds.push((name.clone(), arg.clone()));
                Some(0)
            }
        },
        (Pattern::Annotated { name, ty, .. }, _) => {
            match type_match_depth(ty, arg) {
                Some(depth) => {
                    binds.push((name.clone(), arg.clone()));
                    Some(depth)
                }
                None => None,
            }
        }
        (Pattern::Keyed { .. }, _) => None,
        (Pattern::Ctor { ty, fields }, Value::ErrV(info)) if ty == "err" => {
            match fields.len() == 1 {
                true => match_one(&fields[0], &info.reason, binds),
                false => None,
            }
        }
        (Pattern::Ctor { ty, fields }, Value::Record { ty: vty, fields: vfields })
            if ty.as_str() == &**vty && fields.len() == vfields.borrow().len() =>
        {
            for (fp, fv) in fields.iter().zip(vfields.borrow().iter()) {
                match_one(fp, fv, binds)?;
            }
            Some(0)
        }
        _ => None,
    }
}

pub fn index_value(container: Value, index: Value, span: Span) -> EvalResult {
    if is_failure(&container) {
        return Ok(container);
    }
    if is_failure(&index) {
        return Ok(index);
    }
    match (&container, &index) {
        (Value::List(items), Value::Int(i)) => {
            let idx = usize::try_from(i.clone()).ok();
            Ok(match idx.filter(|i| *i >= 1 && *i <= items.len()) {
                Some(i) => items[i - 1].clone(),
                None => Value::NoneV,
            })
        }
        (Value::Str(text), Value::Int(i)) => {
            let idx = usize::try_from(i.clone()).ok();
            Ok(
                match idx.and_then(|i| i.checked_sub(1)).and_then(|i| text.chars().nth(i)) {
                    Some(c) => Value::Str(c.to_string()),
                    None => Value::NoneV,
                },
            )
        }
        (Value::Map(entries), Value::Int(_) | Value::Str(_)) => {
            let key = map_key(index, span)?;
            Ok(entries.get(&key).cloned().unwrap_or(Value::NoneV))
        }
        _ => Err(RuntimeError {
            message: "indexing takes a list or string with a 1-based position, or a map                       with a key"
                .to_string(),
            span,
        }),
    }
}

fn slice_range(len: usize, from: usize, to: usize) -> Option<std::ops::Range<usize>> {
    match from >= 1 && from <= to && to <= len {
        true => Some(from - 1..to),
        false => None,
    }
}

fn map_key(value: Value, span: Span) -> Result<MapKey, RuntimeError> {
    match value {
        Value::Int(n) => Ok(MapKey::Int(n)),
        Value::Str(s) => Ok(MapKey::Str(s)),
        other => Err(RuntimeError {
            message: format!("{} is not usable as a map key", render(&other, true)),
            span,
        }),
    }
}

pub fn is_failure(value: &Value) -> bool {
    matches!(value, Value::ErrV(_) | Value::NoneV)
}

thread_local! {
    static TYPESETS: std::cell::RefCell<std::collections::HashMap<String, Vec<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The typeset ladder rung: below every concrete type, above the bare
/// generic. Encoded as a fixed depth so the score arithmetic orders it.
const TYPESET_DEPTH: u8 = 50;

pub fn type_matches(ty: &str, arg: &Value) -> bool {
    type_match_depth(ty, arg).is_some()
}

/// The value's own outermost type only — used by the upcast, which walks
/// the chain itself one level at a time.
fn type_matches_exact(ty: &str, arg: &Value) -> bool {
    match arg {
        Value::Sub { ty: vty, .. } => ty == &**vty,
        other => type_match_depth(ty, other) == Some(0),
    }
}

/// How far up the subtype chain the annotation sits: an exact match is 0,
/// the immediate parent 1, and so on — the dispatch score prefers nearer.
fn type_match_depth(ty: &str, arg: &Value) -> Option<u8> {
    let member_hit = TYPESETS.with(|t| {
        t.borrow().get(ty).map(|members| {
            members.iter().any(|m| type_match_depth(m, arg).is_some())
        })
    });
    if let Some(hit) = member_hit {
        return hit.then_some(TYPESET_DEPTH);
    }
    if let Value::Sub { ty: vty, inner } = arg {
        if ty == &**vty {
            return Some(0);
        }
        return type_match_depth(ty, inner).map(|d| d.saturating_add(1));
    }
    if ty.ends_with("[]") {
        return matches!(arg, Value::List(_)).then_some(0);
    }
    if ty.contains('[') {
        return matches!(arg, Value::Map(_)).then_some(0);
    }
    let ok = match (ty, arg) {
        ("int", Value::Int(_)) => true,
        ("float64", Value::Float(_)) => true,
        ("string", Value::Str(_)) => true,
        ("bool", Value::True | Value::False) => true,
        ("true", Value::True) => true,
        ("false", Value::False) => true,
        ("none", Value::NoneV) => true,
        ("err", Value::ErrV(_)) => true,
        (name, Value::Record { ty, .. }) => name == &**ty,
        _ => false,
    };
    ok.then_some(0)
}

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    if let Value::Sub { inner, .. } = a {
        return compare(inner, b);
    }
    if let Value::Sub { inner, .. } = b {
        return compare(a, inner);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => Some(x.total_cmp(y)),
        (Value::Int(x), Value::Float(y)) => Some(int_f(x).total_cmp(y)),
        (Value::Float(x), Value::Int(y)) => Some(x.total_cmp(&int_f(y))),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// A BigInt widened to f64 — the `x:float` cast at the value level.
fn int_f(n: &BigInt) -> f64 {
    n.to_f64().unwrap_or(f64::INFINITY)
}

fn div_float(a: f64, b: f64, frame: &Frame, span: Span) -> EvalResult {
    match b == 0.0 {
        true => Ok(err_value(
            Value::Str("division by zero".to_string()),
            origin_at(frame, span),
        )),
        false => Ok(Value::Float(a / b)),
    }
}

/// `a & b`: join two descriptions to run with no order between them.
/// Failures accumulate — two errs merge into one whose reason lists both
/// (origin-less: the merge has no single birthplace) — and a lone failure
/// propagates as itself. Anything that isn't a description or a failure
/// cannot be joined.
fn join_values(left: Value, right: Value, span: Span) -> EvalResult {
    match (is_failure(&left), is_failure(&right)) {
        (true, true) => Ok(accumulate_failures(left, right)),
        (true, false) => Ok(left),
        (false, true) => Ok(right),
        (false, false) => match (&left, &right) {
            (Value::Desc(a), Value::Desc(b)) => {
                Ok(Value::Desc(Rc::new(Desc::Join(a.clone(), b.clone()))))
            }
            _ => Err(RuntimeError {
                message: "a group joins descriptions".to_string(),
                span,
            }),
        },
    }
}

/// Merge two failures: err + err becomes one err whose reason is the list of
/// both reasons; a `none` adds nothing to an err; two nones stay none.
fn accumulate_failures(left: Value, right: Value) -> Value {
    match (&left, &right) {
        (Value::ErrV(a), Value::ErrV(b)) => err_value(
            Value::List(Rc::new(vec![a.reason.clone(), b.reason.clone()])),
            None,
        ),
        (Value::ErrV(_), _) => left,
        (_, Value::ErrV(_)) => right,
        _ => left,
    }
}

pub fn eval_binop(
    op: &str,
    left: Value,
    right: Value,
    span: Span,
    frame: &Frame,
) -> EvalResult {
    if is_failure(&left) {
        return Ok(left);
    }
    if is_failure(&right) {
        return Ok(right);
    }
    if op == "==" || op == "!=" {
        let equal = values_equal(&left, &right);
        return Ok(bool_value(match op {
            "==" => equal,
            _ => !equal,
        }));
    }
    match (op, &left, &right) {
        ("+", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        ("-", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        ("*", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        ("/", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => Ok(err_value(
                Value::Str("division by zero".to_string()),
                origin_at(frame, span),
            )),
            false => Ok(Value::Int(a / b)),
        },
        ("%", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => Ok(err_value(
                Value::Str("modulo by zero".to_string()),
                origin_at(frame, span),
            )),
            false => Ok(Value::Int(a % b)),
        },
        ("+", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        ("-", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        ("*", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        ("/", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => Ok(err_value(
                Value::Str("division by zero".to_string()),
                origin_at(frame, span),
            )),
            false => Ok(Value::Float(a / b)),
        },
        ("%", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => Ok(err_value(
                Value::Str("modulo by zero".to_string()),
                origin_at(frame, span),
            )),
            false => Ok(Value::Float(a % b)),
        },
        // int meets float: the int widens (as if cast `x:float`), result float
        ("+", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) + b)),
        ("+", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + int_f(b))),
        ("-", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) - b)),
        ("-", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - int_f(b))),
        ("*", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) * b)),
        ("*", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * int_f(b))),
        ("/", Value::Int(a), Value::Float(b)) => div_float(int_f(a), *b, frame, span),
        ("/", Value::Float(a), Value::Int(b)) => div_float(*a, int_f(b), frame, span),
        ("<" | "<=" | ">" | ">=", _, _) => match compare(&left, &right) {
            Some(ord) => Ok(bool_value(match op {
                "<" => ord.is_lt(),
                "<=" => ord.is_le(),
                ">" => ord.is_gt(),
                _ => ord.is_ge(),
            })),
            None => Err(RuntimeError {
                message: "comparison requires two values of one comparable type".to_string(),
                span,
            }),
        },
        _ => Err(RuntimeError {
            message: format!("`{op}` is not defined for these values"),
            span,
        }),
    }
}

fn bool_value(b: bool) -> Value {
    match b {
        true => Value::True,
        false => Value::False,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    values_equal_seen(a, b, &mut std::collections::HashSet::new())
}

/// `seen` holds the record-cell pairs already assumed equal. A build block
/// can close a cycle, so two distinct cyclic graphs are compared by
/// bisimulation: assume a pair equal on first encounter, and a re-encounter
/// of the same pair is the coinductive base case (true) rather than infinite
/// recursion. The assumption is global for the comparison — a pair proven
/// contradictory anywhere still returns false.
fn values_equal_seen(
    a: &Value,
    b: &Value,
    seen: &mut std::collections::HashSet<(usize, usize)>,
) -> bool {
    if let Value::Sub { inner, .. } = a {
        return values_equal_seen(inner, b, seen);
    }
    if let Value::Sub { inner, .. } = b {
        return values_equal_seen(a, inner, seen);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.total_cmp(y).is_eq(),
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter().zip(y.iter()).all(|((ka, va), (kb, vb))| {
                    ka == kb && values_equal_seen(va, vb, seen)
                })
        }
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::True, Value::True) | (Value::False, Value::False) => true,
        (Value::NoneV, Value::NoneV) => true,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len()
                && x.iter().zip(y.iter()).all(|(a, b)| values_equal_seen(a, b, seen))
        }
        (Value::Record { ty: tx, fields: fx }, Value::Record { ty: ty_, fields: fy }) => {
            if tx != ty_ {
                return false;
            }
            if Rc::ptr_eq(fx, fy) {
                return true;
            }
            let key = (Rc::as_ptr(fx) as *const () as usize, Rc::as_ptr(fy) as *const () as usize);
            if !seen.insert(key) {
                return true;
            }
            fx.borrow().len() == fy.borrow().len()
                && fx
                    .borrow()
                    .iter()
                    .zip(fy.borrow().iter())
                    .all(|(a, b)| values_equal_seen(a, b, seen))
        }
        _ => false,
    }
}

fn render_float(x: f64) -> String {
    if x == x.floor() && x.abs() < 1e15 && x.is_finite() {
        return format!("{x:.1}");
    }
    // rust's LowerExp gives the shortest round-trip digits; the format
    // layer mirrors the native renderer's %g rules exactly: exponent form
    // at X < -4 or X >= max(15, digit count)
    let neg = x < 0.0;
    let exp_form = format!("{:e}", x.abs());
    let (mant, exp) = exp_form.split_once('e').expect("LowerExp has an e");
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let k = digits.len() as i32;
    let x10: i32 = exp.parse().expect("exponent parses");
    let p = k.max(15);
    let sign = if neg { "-" } else { "" };
    if x10 < -4 || x10 >= p {
        let tail = if k > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        let esign = if x10 < 0 { '-' } else { '+' };
        return format!("{sign}{}{tail}e{esign}{:02}", &digits[..1], x10.abs());
    }
    if x10 >= 0 {
        let ip = (x10 + 1) as usize;
        let whole: String = (0..ip)
            .map(|i| digits.as_bytes().get(i).map(|b| *b as char).unwrap_or('0'))
            .collect();
        let frac = if (k as usize) > ip { format!(".{}", &digits[ip..]) } else { String::new() };
        return format!("{sign}{whole}{frac}");
    }
    let zeros = "0".repeat((-x10 - 1) as usize);
    format!("{sign}0.{zeros}{digits}")
}

/// Transparency: a subtype value IS its base wherever machinery (builtins,
/// operators, comparison) consumes it; only dispatch sees the wrapper.
pub fn sub_base(value: Value) -> Value {
    match value {
        Value::Sub { inner, .. } => sub_base((*inner).clone()),
        other => other,
    }
}

pub fn render(value: &Value, quote_strings: bool) -> String {
    render_seen(value, quote_strings, &mut std::collections::HashSet::new())
}

/// `seen` holds the record cells on the current render path — a build block
/// can close a cycle, and a cycle renders as `<cycle>` at the point of
/// re-entry rather than recursing forever.
fn render_seen(
    value: &Value,
    quote_strings: bool,
    seen: &mut std::collections::HashSet<usize>,
) -> String {
    match value {
        // a subtype renders as its base until a user arm claims it
        Value::Sub { inner, .. } => render_seen(inner, quote_strings, seen),
        Value::Thunk(cell) => match &*cell.borrow() {
            ThunkState::Forced(v) => render_seen(v, quote_strings, seen),
            // A pending thunk reaching render is a missed force point; make
            // it loud so the differential corpus catches it.
            _ => "<thunk>".to_string(),
        },
        Value::Int(n) => n.to_string(),
        Value::Float(x) => render_float(*x),
        Value::Map(entries) => match entries.is_empty() {
            true => "{:}".to_string(),
            false => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => n.to_string(),
                            MapKey::Str(s) => format!("\"{s}\""),
                        };
                        format!("{key}:{}", render_seen(value, true, seen))
                    })
                    .collect();
                format!("{{ {} }}", inner.join(" "))
            }
        },
        Value::Str(s) => match quote_strings {
            true => format!("\"{s}\""),
            false => s.clone(),
        },
        Value::True => "true".to_string(),
        Value::False => "false".to_string(),
        Value::NoneV => "<none>".to_string(),
        Value::ErrV(info) => format!("err {}", render_seen(&info.reason, true, seen)),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(|i| render_seen(i, true, seen)).collect();
            format!("[{}]", inner.join(" "))
        }
        Value::Record { ty, fields } => match fields.borrow().is_empty() {
            true => ty.to_string(),
            false => {
                let ptr = Rc::as_ptr(fields) as usize;
                if !seen.insert(ptr) {
                    return "<cycle>".to_string();
                }
                let inner: Vec<String> =
                    fields.borrow().iter().map(|f| render_seen(f, true, seen)).collect();
                seen.remove(&ptr);
                format!("{} {}", ty, inner.join(" "))
            }
        },
        Value::FnRef(name) => format!("<fn {name}>"),
        Value::Closure(_) => "<fn>".to_string(),
        Value::Desc(_) => "<io>".to_string(),
    }
}

impl<'a> Interp<'a> {
    pub fn execute(&self, desc: &Desc, executor: &mut dyn Executor) -> EvalResult {
        let origin = Span { line: 0, col: 0 };
        match desc {
            Desc::Print(text, _) => {
                executor.print(text);
                Ok(Value::NoneV)
            }
            Desc::Seq(a, b) => {
                let left = self.execute(a, executor)?;
                if is_failure(&left) && matches!(left, Value::ErrV(_)) {
                    return Ok(left);
                }
                self.execute(b, executor)
            }
            Desc::Join(_, _) => self.schedule(desc, executor),
            Desc::Sleep(ms) => {
                executor.sleep(*ms);
                Ok(Value::NoneV)
            }
            Desc::Random(n) => Ok(Value::Int(executor.random(*n).into())),
            Desc::Nil => Ok(Value::NoneV),
            Desc::Args => {
                let list = executor.args().into_iter().map(Value::Str).collect();
                Ok(Value::List(Rc::new(list)))
            }
            Desc::Stdin => Ok(match executor.stdin() {
                Ok(text) => Value::Str(text),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::ReadFile(path) => Ok(match executor.read_file(path) {
                Ok(text) => Value::Str(text),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::WriteFile(path, content) => Ok(match executor.write_file(path, content) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Bind(inner, callee) => {
                let yielded = self.execute(inner, executor)?;
                let next = self.call(callee.clone(), vec![yielded], origin, &None)?;
                match next {
                    Value::Desc(d) => self.execute(&d, executor),
                    other => Ok(other),
                }
            }
        }
    }

    /// Run a parallel group as cooperative green threads. Each member is a
    /// fiber; a fiber runs until it finishes or blocks on `sleep`, at which
    /// point the scheduler advances to the next fiber. The policy is fully
    /// deterministic — always resume the fiber with the earliest wake time,
    /// ties broken by spawn order — so a concurrent program's output is
    /// byte-identical across engines and runs, which the goldens require.
    /// Members' *values* are discarded (a group yields none); their failures
    /// accumulate, exactly as the sequential Join did.
    fn schedule(&self, join: &Desc, executor: &mut dyn Executor) -> EvalResult {
        let mut fibers = Vec::new();
        flatten_join(join, &mut fibers);
        // (wake_time, spawn_index, remaining work)
        let mut ready: Vec<(u64, usize, Rc<Desc>)> = fibers
            .into_iter()
            .enumerate()
            .map(|(i, d)| (0u64, i, d))
            .collect();
        let mut now = 0u64;
        // wall-credit: real time spent computing counts against a pending
        // wait, so compute overlaps sleeps in wall-clock. the transcript
        // stays purely logical — only the physical wait shrinks.
        #[cfg(not(target_arch = "wasm32"))]
        let wall_start = std::time::Instant::now();
        let mut failures: Vec<Value> = Vec::new();
        while !ready.is_empty() {
            let pick = (0..ready.len())
                .min_by_key(|&i| (ready[i].0, ready[i].1))
                .expect("non-empty");
            let (wake, idx, desc) = ready.remove(pick);
            if wake > now {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let elapsed = wall_start.elapsed().as_millis() as u64;
                    if wake > elapsed {
                        executor.sleep(wake - elapsed);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                executor.sleep(wake - now);
                now = wake;
            }
            match self.step(&desc, executor)? {
                Step::Done(value) => {
                    if is_failure(&value) {
                        failures.push(value);
                    }
                }
                Step::Blocked(ms, cont) => ready.push((now + ms, idx, cont)),
            }
        }
        Ok(failures
            .into_iter()
            .reduce(accumulate_failures)
            .unwrap_or(Value::NoneV))
    }

    /// Advance one fiber until it finishes (`Done`) or hits a `sleep`
    /// (`Blocked`, carrying the rest of its work). Blocking threads back up
    /// through `Seq` and `Bind`, so the continuation is always the remaining
    /// description — no saved stack, no coroutine.
    fn step(&self, desc: &Rc<Desc>, executor: &mut dyn Executor) -> Result<Step, RuntimeError> {
        let origin = Span { line: 0, col: 0 };
        match &**desc {
            Desc::Sleep(ms) => Ok(Step::Blocked(*ms, Rc::new(Desc::Nil))),
            Desc::Seq(a, b) => match self.step(a, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Seq(cont, b.clone()))))
                }
                Step::Done(left) if matches!(left, Value::ErrV(_)) => Ok(Step::Done(left)),
                Step::Done(_) => self.step(b, executor),
            },
            Desc::Bind(inner, callee) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Bind(cont, callee.clone()))))
                }
                Step::Done(yielded) => {
                    let next = self.call(callee.clone(), vec![yielded], origin, &None)?;
                    match next {
                        Value::Desc(d) => self.step(&d, executor),
                        other => Ok(Step::Done(other)),
                    }
                }
            },
            // leaf effects and nested joins run to completion synchronously
            _ => Ok(Step::Done(self.execute(desc, executor)?)),
        }
    }
}

/// Collect the members of a (possibly nested) parallel group left-to-right —
/// the spawn order the scheduler breaks ties by.
fn flatten_join(desc: &Desc, out: &mut Vec<Rc<Desc>>) {
    match desc {
        Desc::Join(a, b) => {
            flatten_join(a, out);
            flatten_join(b, out);
        }
        other => out.push(Rc::new(other.clone())),
    }
}

pub fn render_plan(desc: &Desc, out: &mut String) {
    match desc {
        Desc::Print(text, span) => {
            out.push_str(&format!("  print {text:?}    # from line {}\n", span.line));
        }
        Desc::Seq(a, b) => {
            render_plan(a, out);
            render_plan(b, out);
        }
        Desc::Join(a, b) => {
            out.push_str("  join {\n");
            render_plan(a, out);
            render_plan(b, out);
            out.push_str("  } # unordered; both run\n");
        }
        Desc::Args => out.push_str("  args\n"),
        Desc::Stdin => out.push_str("  stdin\n"),
        Desc::ReadFile(path) => out.push_str(&format!("  read_file {path:?}\n")),
        Desc::WriteFile(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::Bind(inner, _) => {
            render_plan(inner, out);
            out.push_str("  . <continuation>\n");
        }
        Desc::Sleep(ms) => out.push_str(&format!("  sleep {ms}\n")),
        Desc::Random(n) => out.push_str(&format!("  random {n}\n")),
        Desc::Nil => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_main(source: &str) -> Value {
        let lexed = crate::lexer::lex(source).expect("lexes");
        let mut program = crate::parser::parse(&lexed).expect("parses");
        let diags = crate::check::check(&mut program, true);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let interp = Interp::new(&program);
        interp.run_main().map_err(|e| e.message).expect("runs")
    }

    #[test]
    fn scripted_executor_records_the_transcript() {
        let value = run_main("main = print \"a\" >> print \"b\"\n");

        let Value::Desc(desc) = value else { panic!("main yields a description") };
        let lexed = crate::lexer::lex("main = print \"a\" >> print \"b\"\n").expect("lexes");
        let program = crate::parser::parse(&lexed).expect("parses");
        let interp = Interp::new(&program);
        let mut executor = ScriptedExecutor::default();
        interp.execute(&desc, &mut executor).map_err(|e| e.message).expect("executes");

        assert_eq!(executor.transcript, ["print \"a\"", "print \"b\""]);
    }

    #[test]
    fn a_pure_main_yields_a_plain_value() {
        let value = run_main("main = 1 + 2\n");

        assert!(matches!(value, Value::Int(ref n) if *n == num_bigint::BigInt::from(3)));
    }

    #[test]
    fn err_propagates_through_a_generic_function_unhandled() {
        let source = "fn double x\n  x * 2\n\nmain = double (1 / 0)\n";

        assert!(matches!(run_main(source), Value::ErrV(_)));
    }

    #[test]
    fn errs_carry_their_origin_and_dispatcher_hops() {
        let source = "fn grade outcome\n  \"grade {outcome}\"\n\nmain = grade (1 / 0)\n";
        let program = crate::compile("spec.kso", source, true).expect("compiles");
        let interp = Interp::new(&program);
        let value = interp.run_main().map_err(|e| e.message).expect("runs");

        let Value::ErrV(info) = value else { panic!("expected an err") };
        assert_eq!(info.origin.as_deref(), Some("main at spec.kso:4"));
        assert_eq!(info.hops.iter().map(|h| &**h).collect::<Vec<_>>(), ["grade"]);
    }
}
