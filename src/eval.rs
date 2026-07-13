use crate::ast::*;
use crate::diag::Span;
use num_bigint::BigInt;
use num_traits::Zero;
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
    Record { ty: Rc<str>, fields: Rc<Vec<Value>> },
    FnRef(Rc<str>),
    Closure(Rc<ClosureData>),
    Desc(Rc<Desc>),
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

#[derive(Debug)]
pub enum Desc {
    Print(String, Span),
    Seq(Rc<Desc>, Rc<Desc>),
    Args,
    Stdin,
    ReadFile(String),
    WriteFile(String, String),
    Bind(Rc<Desc>, Value),
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
}

pub struct RealExecutor {
    pub program_args: Vec<String>,
}

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        println!("{text}");
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
}

impl Executor for ScriptedExecutor {
    fn print(&mut self, text: &str) {
        self.transcript.push(format!("print {text:?}"));
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
}

impl<'a> Interp<'a> {
    pub fn new(program: &'a Program) -> Self {
        let mut fns: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        for decl in &program.fns {
            fns.entry(&decl.name).or_default().push(decl);
        }
        let types = program.types.iter().map(|t| (t.name.as_str(), t)).collect();
        let origin = Span { line: 0, col: 0 };
        let entry_decl = TypeDecl {
            name: "entry".to_string(),
            span: origin,
            fields: vec![
                ("key".to_string(), vec!["any".to_string()], origin),
                ("value".to_string(), vec!["any".to_string()], origin),
            ],
        };
        Interp { fns, types, entry_decl }
    }

    fn type_decl(&self, name: &str) -> Option<&TypeDecl> {
        match name {
            "entry" => Some(&self.entry_decl),
            _ => self.types.get(name).copied(),
        }
    }

    pub fn run_main(&self) -> EvalResult {
        let main = self.fns.get("main").expect("checked: main exists")[0];
        self.eval_body(&main.body, None, &frame_of(main))
    }

    pub fn run_named(&self, name: &str) -> Option<EvalResult> {
        let decl = self.fns.get(name)?.iter().find(|d| d.params.is_empty())?;
        Some(self.eval_body(&decl.body, None, &frame_of(decl)))
    }

    fn eval_body(&self, body: &[Stmt], mut env: Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        let mut result = Value::NoneV;
        for stmt in body {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    let value = self.eval(expr, &env, frame)?;
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
                    Some(()) => {
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
                    env = bind(env, &entry.bind_name, fields[position].clone());
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
            Expr::Index { base, index, span } => {
                let container = self.eval(base, env, frame)?;
                let key = self.eval(index, env, frame)?;
                match index_value(container, key.clone(), *span)? {
                    Value::NoneV => Ok(err_value(
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
                let left = self.eval(lhs, env, frame)?;
                let right = self.eval(rhs, env, frame)?;
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
                let left = self.eval(lhs, env, frame)?;
                let right = self.eval(rhs, env, frame)?;
                eval_binop(op, left, right, *span, frame)
            }
        }
    }

    fn eval_ident(&self, name: &str, span: Span, env: &Option<Rc<Env>>) -> EvalResult {
        if let Some(value) = lookup(env, name) {
            return Ok(value);
        }
        if let Some(decls) = self.fns.get(name) {
            if let Some(constant) = decls.iter().find(|d| d.params.is_empty()) {
                return self.eval_body(&constant.body, None, &frame_of(constant));
            }
        }
        match name {
            "args" => return Ok(Value::Desc(Rc::new(Desc::Args))),
            "stdin" => return Ok(Value::Desc(Rc::new(Desc::Stdin))),
            _ => {}
        }
        if let Some(decl) = self.type_decl(name) {
            if decl.fields.is_empty() {
                return Ok(Value::Record { ty: Rc::from(name), fields: Rc::new(Vec::new()) });
            }
        }
        match name {
            "true" => Ok(Value::True),
            "false" => Ok(Value::False),
            "none" => Ok(Value::NoneV),
            _ if self.fns.contains_key(name)
                || self.types.contains_key(name)
                || name == "err"
                || crate::check::BUILTINS.contains(&name) =>
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
                    let value = self.eval(expr, env, frame)?;
                    if is_failure(&value) {
                        return Ok(value);
                    }
                    out.push_str(&render(&value, false));
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
            if is_failure(&reason) {
                return Ok(reason);
            }
            return Ok(err_value(reason, origin_at(frame, span)));
        }
        if let Some(ty) = self.type_decl(name) {
            return self.construct(ty, args, span);
        }
        if let Some(overloads) = self.fns.get(name) {
            return self.dispatch(name, overloads, args, span);
        }
        self.call_builtin(name, args, span, frame)
    }

    fn construct(&self, ty: &TypeDecl, args: Vec<Value>, span: Span) -> EvalResult {
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
        Ok(Value::Record { ty: Rc::from(ty.name.as_str()), fields: Rc::new(args) })
    }

    fn dispatch(
        &self,
        name: &str,
        overloads: &[&FnDecl],
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
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
                self.eval_body(&decl.body, env, &frame_of(decl))
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
            "print" => {
                let [text] = arity(args, name, span)?;
                match text {
                    Value::Str(s) => Ok(Value::Desc(Rc::new(Desc::Print(s, span)))),
                    other => Err(RuntimeError {
                        message: format!(
                            "print takes a string; interpolate instead: \"{{...}}\" (got {})",
                            render(&other, false)
                        ),
                        span,
                    }),
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
                            fields: Rc::new(vec![key, value.clone()]),
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

    fn force(&self, value: Value) -> EvalResult {
        match value {
            Value::Closure(c) if c.params.is_empty() => self.eval(&c.body, &c.env, &c.frame),
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
        score.push(3 - pattern.rank());
        match_one(pattern, arg, &mut binds)?;
    }
    Some((score, binds))
}

fn match_one(pattern: &Pattern, arg: &Value, binds: &mut Bindings) -> Option<()> {
    match (pattern, arg) {
        (Pattern::IntLit(n, _), Value::Int(v)) if n == v => Some(()),
        (Pattern::StrLit(s, _), Value::Str(v)) if s == v => Some(()),
        (Pattern::Nullary(name, _), Value::True) if name == "true" => Some(()),
        (Pattern::Nullary(name, _), Value::False) if name == "false" => Some(()),
        (Pattern::Nullary(name, _), Value::NoneV) if name == "none" => Some(()),
        (Pattern::Wildcard(_), _) => match is_failure(arg) {
            true => None,
            false => Some(()),
        },
        (Pattern::Var(name, _), _) => match is_failure(arg) {
            true => None,
            false => {
                binds.push((name.clone(), arg.clone()));
                Some(())
            }
        },
        (Pattern::Annotated { name, ty, .. }, _) => {
            match type_matches(ty, arg) {
                true => {
                    binds.push((name.clone(), arg.clone()));
                    Some(())
                }
                false => None,
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
            if ty.as_str() == &**vty && fields.len() == vfields.len() =>
        {
            for (fp, fv) in fields.iter().zip(vfields.iter()) {
                match_one(fp, fv, binds)?;
            }
            Some(())
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

fn type_matches(ty: &str, arg: &Value) -> bool {
    if ty.ends_with("[]") {
        return matches!(arg, Value::List(_));
    }
    if ty.contains('[') {
        return matches!(arg, Value::Map(_));
    }
    match (ty, arg) {
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
    }
}

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => Some(x.total_cmp(y)),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => None,
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
        ("+" | "-" | "*" | "/", Value::Int(_), Value::Float(_))
        | ("+" | "-" | "*" | "/", Value::Float(_), Value::Int(_)) => Err(RuntimeError {
            message: "no implicit numeric coercion; convert explicitly with `to_float`"
                .to_string(),
            span,
        }),
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
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.total_cmp(y).is_eq(),
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter().zip(y.iter()).all(|((ka, va), (kb, vb))| {
                    ka == kb && values_equal(va, vb)
                })
        }
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::True, Value::True) | (Value::False, Value::False) => true,
        (Value::NoneV, Value::NoneV) => true,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Record { ty: tx, fields: fx }, Value::Record { ty: ty_, fields: fy }) => {
            tx == ty_ && fx.iter().zip(fy.iter()).all(|(a, b)| values_equal(a, b))
        }
        _ => false,
    }
}

fn render_float(x: f64) -> String {
    match x.is_finite() && x.fract() == 0.0 && x.abs() < 1e15 {
        true => format!("{x:.1}"),
        false => format!("{x}"),
    }
}

pub fn render(value: &Value, quote_strings: bool) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Float(x) => render_float(*x),
        Value::Map(entries) => match entries.is_empty() {
            true => "[:]".to_string(),
            false => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => n.to_string(),
                            MapKey::Str(s) => format!("\"{s}\""),
                        };
                        format!("{key}: {}", render(value, true))
                    })
                    .collect();
                format!("[{}]", inner.join(" "))
            }
        },
        Value::Str(s) => match quote_strings {
            true => format!("\"{s}\""),
            false => s.clone(),
        },
        Value::True => "true".to_string(),
        Value::False => "false".to_string(),
        Value::NoneV => "none".to_string(),
        Value::ErrV(info) => format!("err {}", render(&info.reason, true)),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(|i| render(i, true)).collect();
            format!("[{}]", inner.join(" "))
        }
        Value::Record { ty, fields } => match fields.is_empty() {
            true => ty.to_string(),
            false => {
                let inner: Vec<String> = fields.iter().map(|f| render(f, true)).collect();
                format!("{} {}", ty, inner.join(" "))
            }
        },
        Value::FnRef(name) => format!("<fn {name}>"),
        Value::Closure(_) => "<fn>".to_string(),
        Value::Desc(_) => "<description>".to_string(),
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
        Desc::Args => out.push_str("  args\n"),
        Desc::Stdin => out.push_str("  stdin\n"),
        Desc::ReadFile(path) => out.push_str(&format!("  read_file {path:?}\n")),
        Desc::WriteFile(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::Bind(inner, _) => {
            render_plan(inner, out);
            out.push_str("  . <continuation>\n");
        }
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
