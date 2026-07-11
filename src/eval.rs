use crate::ast::*;
use crate::diag::Span;
use num_bigint::BigInt;
use num_traits::Zero;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value {
    Int(BigInt),
    Str(String),
    True,
    False,
    NoneV,
    ErrV(Rc<Value>),
    List(Rc<Vec<Value>>),
    Record { ty: Rc<str>, fields: Rc<Vec<Value>> },
    FnRef(Rc<str>),
    Closure(Rc<ClosureData>),
    Desc(Rc<Desc>),
}

#[derive(Debug)]
pub struct ClosureData {
    pub params: Vec<String>,
    pub body: Expr,
    pub env: Option<Rc<Env>>,
}

#[derive(Debug)]
pub enum Desc {
    Print(String, Span),
    Seq(Rc<Desc>, Rc<Desc>),
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
}

pub struct RealExecutor;

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }
}

pub struct ScriptedExecutor {
    pub transcript: Vec<String>,
}

impl Executor for ScriptedExecutor {
    fn print(&mut self, text: &str) {
        self.transcript.push(format!("print {text:?}"));
    }
}

pub struct Interp<'a> {
    fns: HashMap<&'a str, Vec<&'a FnDecl>>,
    types: HashMap<&'a str, &'a TypeDecl>,
}

impl<'a> Interp<'a> {
    pub fn new(program: &'a Program) -> Self {
        let mut fns: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        for decl in &program.fns {
            fns.entry(&decl.name).or_default().push(decl);
        }
        let types = program.types.iter().map(|t| (t.name.as_str(), t)).collect();
        Interp { fns, types }
    }

    pub fn run_main(&self) -> EvalResult {
        let main = self.fns.get("main").expect("checked: main exists")[0];
        self.eval_body(&main.body, None)
    }

    fn eval_body(&self, body: &[Stmt], mut env: Option<Rc<Env>>) -> EvalResult {
        let mut result = Value::NoneV;
        for stmt in body {
            match stmt {
                Stmt::Bind { name, expr, .. } => {
                    let value = self.eval(expr, &env)?;
                    env = bind(env, name, value);
                }
                Stmt::Expr(expr) => result = self.eval(expr, &env)?,
            }
        }
        Ok(result)
    }

    fn eval(&self, expr: &Expr, env: &Option<Rc<Env>>) -> EvalResult {
        match expr {
            Expr::Int(n, _) => Ok(Value::Int(n.clone())),
            Expr::Str(parts, _) => self.eval_template(parts, env),
            Expr::Ident(name, span) => self.eval_ident(name, *span, env),
            Expr::List(items, _) => {
                let values =
                    items.iter().map(|e| self.eval(e, env)).collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(values)))
            }
            Expr::App { head, args, span } => {
                let callee = self.eval(head, env)?;
                let lazy_if = matches!(&callee, Value::FnRef(name) if &**name == "if");
                let mut values = Vec::new();
                for arg in args {
                    match lazy_if {
                        true => values.push(Value::Closure(Rc::new(ClosureData {
                            params: Vec::new(),
                            body: arg.clone(),
                            env: env.clone(),
                        }))),
                        false => values.push(self.eval(arg, env)?),
                    }
                }
                self.call(callee, values, *span)
            }
            Expr::Seq(lhs, rhs, span) => {
                let left = self.eval(lhs, env)?;
                let right = self.eval(rhs, env)?;
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
            }))),
            Expr::BinOp { op, lhs, rhs, span } => {
                let left = self.eval(lhs, env)?;
                let right = self.eval(rhs, env)?;
                eval_binop(op, left, right, *span)
            }
        }
    }

    fn eval_ident(&self, name: &str, span: Span, env: &Option<Rc<Env>>) -> EvalResult {
        if let Some(value) = lookup(env, name) {
            return Ok(value);
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

    fn eval_template(&self, parts: &[TemplatePart], env: &Option<Rc<Env>>) -> EvalResult {
        let mut out = String::new();
        for part in parts {
            match part {
                TemplatePart::Lit(s) => out.push_str(s),
                TemplatePart::Interp(expr) => {
                    let value = self.eval(expr, env)?;
                    if is_failure(&value) {
                        return Ok(value);
                    }
                    out.push_str(&render(&value, false));
                }
            }
        }
        Ok(Value::Str(out))
    }

    fn call(&self, callee: Value, args: Vec<Value>, span: Span) -> EvalResult {
        match callee {
            Value::FnRef(name) => self.call_named(&name, args, span),
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
        self.eval(&closure.body, &env)
    }

    fn call_named(&self, name: &str, args: Vec<Value>, span: Span) -> EvalResult {
        if name == "err" {
            let [reason] = arity(args, name, span)?;
            return Ok(Value::ErrV(Rc::new(reason)));
        }
        if let Some(ty) = self.types.get(name) {
            return self.construct(ty, args, span);
        }
        if let Some(overloads) = self.fns.get(name) {
            return self.dispatch(name, overloads, args, span);
        }
        self.call_builtin(name, args, span)
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
                self.eval_body(&decl.body, env)
            }
            None => propagate_or(args, || RuntimeError {
                message: format!("no overload of `{name}` matches these arguments"),
                span,
            }),
        }
    }

    fn call_builtin(&self, name: &str, args: Vec<Value>, span: Span) -> EvalResult {
        if name == "if" {
            return self.builtin_if(args, span);
        }
        if let Some(bad) = args.iter().find(|a| is_failure(a)) {
            return Ok(bad.clone());
        }
        match name {
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
                let [list, index] = arity(args, name, span)?;
                let (Value::List(items), Value::Int(i)) = (&list, &index) else {
                    return Err(RuntimeError {
                        message: "at takes a list and a 1-based position".to_string(),
                        span,
                    });
                };
                let idx = usize::try_from(i.clone()).ok();
                match idx.filter(|i| *i >= 1 && *i <= items.len()) {
                    Some(i) => Ok(items[i - 1].clone()),
                    None => Ok(Value::NoneV),
                }
            }
            "length" => {
                let [list] = arity(args, name, span)?;
                match list {
                    Value::List(items) => Ok(Value::Int(BigInt::from(items.len()))),
                    Value::Str(s) => Ok(Value::Int(BigInt::from(s.chars().count()))),
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
                    .map(|item| self.call(f.clone(), vec![item.clone()], span))
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
                    match self.call(f.clone(), vec![item.clone()], span)? {
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
            Value::Closure(c) if c.params.is_empty() => self.eval(&c.body, &c.env),
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

fn propagate_or(
    args: Vec<Value>,
    err: impl FnOnce() -> RuntimeError,
) -> EvalResult {
    match args.into_iter().find(is_failure) {
        Some(bad) => Ok(bad),
        None => Err(err()),
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
        (Pattern::Wildcard, _) => match is_failure(arg) {
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
        (Pattern::Ctor { ty, fields }, Value::ErrV(reason)) if ty == "err" => {
            match fields.len() == 1 {
                true => match_one(&fields[0], reason, binds),
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

fn is_failure(value: &Value) -> bool {
    matches!(value, Value::ErrV(_) | Value::NoneV)
}

fn type_matches(ty: &str, arg: &Value) -> bool {
    match (ty, arg) {
        ("int", Value::Int(_)) => true,
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
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

fn eval_binop(op: &str, left: Value, right: Value, span: Span) -> EvalResult {
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
            true => Ok(Value::ErrV(Rc::new(Value::Str("division by zero".to_string())))),
            false => Ok(Value::Int(a / b)),
        },
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

pub fn render(value: &Value, quote_strings: bool) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Str(s) => match quote_strings {
            true => format!("{s:?}"),
            false => s.clone(),
        },
        Value::True => "true".to_string(),
        Value::False => "false".to_string(),
        Value::NoneV => "none".to_string(),
        Value::ErrV(reason) => format!("err {}", render(reason, true)),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(|i| render(i, true)).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Record { ty, fields } => {
            let inner: Vec<String> = fields.iter().map(|f| render(f, true)).collect();
            format!("{} {}", ty, inner.join(", "))
        }
        Value::FnRef(name) => format!("<fn {name}>"),
        Value::Closure(_) => "<fn>".to_string(),
        Value::Desc(_) => "<description>".to_string(),
    }
}

pub fn execute(desc: &Desc, executor: &mut dyn Executor) {
    match desc {
        Desc::Print(text, _) => executor.print(text),
        Desc::Seq(a, b) => {
            execute(a, executor);
            execute(b, executor);
        }
    }
}

pub fn render_plan(desc: &Desc, out: &mut String) {
    match desc {
        Desc::Print(text, span) => {
            out.push_str(&format!("  print {text:?}    // from line {}\n", span.line));
        }
        Desc::Seq(a, b) => {
            render_plan(a, out);
            render_plan(b, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_main(source: &str) -> Value {
        let lexed = crate::lexer::lex(source).expect("lexes");
        let program = crate::parser::parse(&lexed).expect("parses");
        let diags = crate::check::check(&program);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let interp = Interp::new(&program);
        interp.run_main().map_err(|e| e.message).expect("runs")
    }

    #[test]
    fn scripted_executor_records_the_transcript() {
        let value = run_main("fn main\n  print \"a\" >> print \"b\"\n");

        let Value::Desc(desc) = value else { panic!("main yields a description") };
        let mut executor = ScriptedExecutor { transcript: Vec::new() };
        execute(&desc, &mut executor);

        assert_eq!(executor.transcript, ["print \"a\"", "print \"b\""]);
    }

    #[test]
    fn a_pure_main_yields_a_plain_value() {
        let value = run_main("fn main\n  1 + 2\n");

        assert!(matches!(value, Value::Int(ref n) if *n == num_bigint::BigInt::from(3)));
    }

    #[test]
    fn err_propagates_through_a_generic_function_unhandled() {
        let source = "fn double x\n  x * 2\n\nfn main\n  double (1 / 0)\n";

        assert!(matches!(run_main(source), Value::ErrV(_)));
    }
}
