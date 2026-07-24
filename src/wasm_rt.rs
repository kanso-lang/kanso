//! Host side of the browser backend: a registry of values addressed by i32
//! handles, the rt_* imports the compiled module calls, and the executor
//! that runs the resulting description. Values reuse the interpreter's
//! `Value` so semantics stay oracle-identical; closures compiled to wasm
//! call back into the program module through `k_callback`.
#![cfg(target_arch = "wasm32")]
use crate::ast::Program;
use crate::eval::{
    self, err_value, eval_binop, hop, index_value, is_failure, render, trace_lines, Desc,
    Executor, ErrInfo, Interp, Value,
};
use crate::diag::Span;
use crate::wasm_backend::Lit;
use std::cell::RefCell;
use std::rc::Rc;

#[link(wasm_import_module = "env")]
extern "C" {
    fn k_callback(table_idx: u32, env_h: u32, args_h: u32) -> u32;
}

#[derive(Clone)]
enum Slot {
    V(Value),
    /// A list of raw handles: closure environments and callback argument packs.
    E(Rc<Vec<u32>>),
    /// A closure compiled into the program module's table.
    C { tidx: u32, env: u32 },
    Seq(u32, u32),
    Bind(u32, u32),
}

thread_local! {
    static REG: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
    static ARGS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static TYPES: RefCell<Vec<(String, Vec<String>)>> = const { RefCell::new(Vec::new()) };
    static ERROR: RefCell<String> = const { RefCell::new(String::new()) };
    static PRINTS: RefCell<String> = const { RefCell::new(String::new()) };
    static INTERP: RefCell<Option<Interp<'static>>> = const { RefCell::new(None) };
}

const SPAN0: Span = Span { line: 0, col: 0 };

pub fn load(program: Program, lits: &[Lit], types: Vec<(String, Vec<String>)>) {
    let parents: Vec<(String, String)> = program
        .types
        .iter()
        .filter_map(|t| t.parent.clone().map(|p| (t.name.clone(), p)))
        .collect();
    let leaked: &'static Program = Box::leak(Box::new(program));
    INTERP.with(|i| *i.borrow_mut() = Some(Interp::new(leaked)));
    TYPES.with(|t| *t.borrow_mut() = types);
    SUB_PARENTS.with(|t| *t.borrow_mut() = parents);
    REG.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for lit in lits {
            let value = match lit {
                Lit::Int(n) => Value::Int(n.clone()),
                Lit::Float(x) => Value::Float(*x),
                Lit::Str(s) => Value::Str(s.clone()),
                Lit::True => Value::True,
                Lit::False => Value::False,
                Lit::NoneV => Value::NoneV,
            };
            reg.push(Slot::V(value));
        }
    });
    ARGS.with(|a| a.borrow_mut().clear());
}

pub fn take_error() -> String {
    ERROR.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

fn die(msg: String) -> ! {
    ERROR.with(|e| *e.borrow_mut() = msg);
    std::process::abort();
}

fn slot(h: u32) -> Slot {
    REG.with(|r| r.borrow()[h as usize].clone())
}

fn push(s: Slot) -> u32 {
    REG.with(|r| {
        let mut reg = r.borrow_mut();
        reg.push(s);
        (reg.len() - 1) as u32
    })
}

fn val(h: u32) -> Value {
    match slot(h) {
        Slot::V(v) => v,
        _ => die("a closure or bound description cannot be used as data here".to_string()),
    }
}

fn pop_args(n: u32) -> Vec<u32> {
    ARGS.with(|a| {
        let mut args = a.borrow_mut();
        let at = args.len() - n as usize;
        args.split_off(at)
    })
}

fn descish(s: &Slot) -> bool {
    matches!(s, Slot::V(Value::Desc(_)) | Slot::Seq(..) | Slot::Bind(..))
}

fn type_index(name: &str) -> Option<usize> {
    TYPES.with(|t| t.borrow().iter().position(|(n, _)| n == name))
}

fn type_name(idx: usize) -> String {
    TYPES.with(|t| t.borrow()[idx].0.clone())
}

/// The subtype parents, mirrored from the program at init like TYPES.
fn sub_parent(name: &str) -> Option<String> {
    SUB_PARENTS.with(|t| t.borrow().iter().find(|(n, _)| n == name).map(|(_, p)| p.clone()))
}

thread_local! {
    static SUB_PARENTS: std::cell::RefCell<Vec<(String, String)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn call_closure(c_h: u32, arg_handles: Vec<u32>) -> u32 {
    for &h in &arg_handles {
        if let Slot::V(v) = slot(h) {
            if is_failure(&v) {
                return h;
            }
        }
    }
    let Slot::C { tidx, env } = slot(c_h) else {
        let v = slot(c_h);
        if let Slot::V(value) = v {
            if is_failure(&value) {
                return c_h;
            }
            die(format!("`{}` is not callable", render(&value, false)));
        }
        die("this value is not callable".to_string());
    };
    let args = push(Slot::E(Rc::new(arg_handles)));
    unsafe { k_callback(tidx, env, args) }
}

fn with_interp<T>(f: impl FnOnce(&Interp<'static>) -> T) -> T {
    INTERP.with(|i| f(i.borrow().as_ref().expect("program loaded")))
}

/* ---------- rt_* imports ---------- */

#[no_mangle]
pub extern "C" fn rt_is_failure(h: u32) -> u32 {
    match slot(h) {
        Slot::V(v) => is_failure(&v) as u32,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn rt_eq_lit(h: u32, lit: u32) -> u32 {
    let (Slot::V(a), Slot::V(b)) = (slot(h), slot(lit)) else {
        return 0;
    };
    let eq = match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::True, Value::True) | (Value::False, Value::False) => true,
        (Value::NoneV, Value::NoneV) => true,
        _ => false,
    };
    eq as u32
}

#[no_mangle]
pub extern "C" fn rt_check_type(h: u32, code: u32) -> u32 {
    let Slot::V(v) = slot(h) else {
        return 0;
    };
    fn check(v: &Value, code: u32) -> bool {
        if let Value::Sub { ty, inner } = v {
            if code >= 100 && type_index(ty).is_some_and(|i| i == (code - 100) as usize) {
                return true;
            }
            return check(inner, code);
        }
        match code {
            0 => matches!(v, Value::Int(_)),
            1 => matches!(v, Value::Float(_)),
            2 => matches!(v, Value::Str(_)),
            3 => matches!(v, Value::True | Value::False),
            4 => matches!(v, Value::List(_)),
            5 => matches!(v, Value::Map(_)),
            6 => matches!(v, Value::ErrV(_)),
            tid => match v {
                Value::Record { ty, .. } => {
                    type_index(ty).is_some_and(|i| i == (tid - 100) as usize)
                }
                _ => false,
            },
        }
    }
    let ok = check(&v, code);
    ok as u32
}

#[no_mangle]
pub extern "C" fn rt_check_rec(h: u32, tid: u32, nfields: u32) -> u32 {
    let Slot::V(Value::Record { ty, fields }) = slot(h) else {
        return 0;
    };
    let matches_ty = type_index(&ty).is_some_and(|i| i == tid as usize);
    (matches_ty && fields.len() == nfields as usize) as u32
}

#[no_mangle]
pub extern "C" fn rt_check_err(h: u32) -> u32 {
    matches!(slot(h), Slot::V(Value::ErrV(_))) as u32
}

#[no_mangle]
pub extern "C" fn rt_field(h: u32, i: u32) -> u32 {
    let Slot::V(Value::Record { fields, .. }) = slot(h) else {
        die("not a record".to_string());
    };
    push(Slot::V(fields[i as usize].clone()))
}

#[no_mangle]
pub extern "C" fn rt_err_inner(h: u32) -> u32 {
    let Slot::V(Value::ErrV(info)) = slot(h) else {
        die("not an err".to_string());
    };
    push(Slot::V(info.reason.clone()))
}

#[no_mangle]
pub extern "C" fn rt_keyed_check(h: u32, entries: u32) -> u32 {
    let Slot::V(value) = slot(h) else {
        die("cannot read fields of this value; keyed reads take a record".to_string());
    };
    let Value::Record { ty, .. } = &value else {
        die(format!(
            "cannot read fields of {}; keyed reads take a record",
            render(&value, true)
        ));
    };
    let declared = TYPES.with(|t| {
        let types = t.borrow();
        let i = types.iter().position(|(n, _)| n == &**ty).expect("declared type");
        types[i].1.len()
    });
    if entries as usize >= declared {
        die("a keyed read omits at least one field; reading every field is the positional form"
            .to_string());
    }
    h
}

#[no_mangle]
pub extern "C" fn rt_keyed_field(h: u32, name_lit: u32) -> u32 {
    let name = match val(name_lit) {
        Value::Str(s) => s,
        _ => die("field name must be a string".to_string()),
    };
    let Slot::V(Value::Record { ty, fields }) = slot(h) else {
        die("not a record".to_string());
    };
    let position = TYPES.with(|t| {
        let types = t.borrow();
        let i = types.iter().position(|(n, _)| *n == *ty).expect("declared type");
        types[i].1.iter().position(|f| *f == name)
    });
    match position {
        Some(i) => push(Slot::V(fields[i].clone())),
        None => die(format!("`{ty}` has no field `{name}`")),
    }
}

#[no_mangle]
pub extern "C" fn rt_mkerr(h: u32, origin_lit: u32) -> u32 {
    let v = val(h);
    if is_failure(&v) {
        return h;
    }
    push(Slot::V(err_value(v, Some(lit_str(origin_lit)))))
}

fn lit_str(h: u32) -> Rc<str> {
    match val(h) {
        Value::Str(s) => Rc::from(s.as_str()),
        _ => die("an origin or hop literal must be a string".to_string()),
    }
}

/// A dispatcher passing a failure through appends its name to the trace.
#[no_mangle]
pub extern "C" fn rt_err_hop(h: u32, name_lit: u32) -> u32 {
    match slot(h) {
        Slot::V(v @ Value::ErrV(_)) => push(Slot::V(hop(v, &lit_str(name_lit)))),
        _ => h,
    }
}

/// Origin for errs born inside an rt call (division, indexing, fallible
/// builtins): the compiled site stamps the fresh err it gets back.
#[no_mangle]
pub extern "C" fn rt_err_stamp(h: u32, origin_lit: u32) -> u32 {
    match slot(h) {
        Slot::V(Value::ErrV(info)) if info.origin.is_none() => {
            push(Slot::V(Value::ErrV(Rc::new(ErrInfo {
                reason: info.reason.clone(),
                origin: Some(lit_str(origin_lit)),
                hops: info.hops.clone(),
            }))))
        }
        _ => h,
    }
}

#[no_mangle]
pub extern "C" fn rt_arg(h: u32) {
    ARGS.with(|a| a.borrow_mut().push(h));
}

#[no_mangle]
pub extern "C" fn rt_mklist(n: u32) -> u32 {
    let handles = pop_args(n);
    let mut items = Vec::with_capacity(handles.len());
    for h in handles {
        let v = val(h);
        if is_failure(&v) {
            return h;
        }
        items.push(v);
    }
    push(Slot::V(Value::List(Rc::new(items))))
}

#[no_mangle]
pub extern "C" fn rt_mkmap(n: u32) -> u32 {
    let handles = pop_args(n * 2);
    let mut values = Vec::with_capacity(handles.len());
    for h in &handles {
        let v = val(*h);
        if is_failure(&v) {
            return *h;
        }
        values.push(v);
    }
    let mut map = std::collections::BTreeMap::new();
    for pair in values.chunks(2) {
        let key = match &pair[0] {
            Value::Int(n) => eval::MapKey::Int(n.clone()),
            Value::Str(s) => eval::MapKey::Str(s.clone()),
            other => die(format!("map keys are ints or strings, not {}", render(other, true))),
        };
        map.insert(key, pair[1].clone());
    }
    push(Slot::V(Value::Map(Rc::new(map))))
}

#[no_mangle]
pub extern "C" fn rt_mksub(inner: u32, tid: u32) -> u32 {
    let Slot::V(v) = slot(inner) else {
        return inner;
    };
    if is_failure(&v) {
        return inner;
    }
    let name = type_name(tid as usize);
    let parent = sub_parent(&name).unwrap_or_default();
    if crate::eval::type_matches(&parent, &v) {
        push(Slot::V(Value::Sub { ty: Rc::from(name.as_str()), inner: Rc::new(v) }))
    } else {
        die(format!("`{name}` wraps a {parent}"))
    }
}

#[no_mangle]
pub extern "C" fn rt_upcast(inner: u32, code: u32) -> u32 {
    let Slot::V(mut v) = slot(inner) else {
        return inner;
    };
    if is_failure(&v) {
        return inner;
    }
    let want_name = if code >= 100 { type_name(code as usize - 100) } else {
        match code { 0 => "int", 1 => "float64", 2 => "string", 3 => "bool", 6 => "err", _ => "" }
            .to_string()
    };
    loop {
        match &v {
            Value::Sub { ty, inner } => {
                if **ty == *want_name {
                    return push(Slot::V(v.clone()));
                }
                let next = (**inner).clone();
                v = next;
            }
            other => {
                if crate::eval::type_matches(&want_name, other) {
                    return push(Slot::V(v.clone()));
                }
                return die(format!("`:{want_name}` widens; this value is not a {want_name}"));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn rt_mkrec(tid: u32, n: u32) -> u32 {
    let handles = pop_args(n);
    let mut fields = Vec::with_capacity(handles.len());
    for h in handles {
        let v = val(h);
        if is_failure(&v) {
            return h;
        }
        fields.push(v);
    }
    let name = TYPES.with(|t| t.borrow()[tid as usize].0.clone());
    push(Slot::V(Value::Record { ty: Rc::from(name.as_str()), fields: Rc::new(fields) }))
}

#[no_mangle]
pub extern "C" fn rt_template(n: u32) -> u32 {
    let handles = pop_args(n);
    let mut out = String::new();
    for h in handles {
        let v = val(h);
        // only an err propagates; none renders its sentinel via the group —
        // the same rule as the other engines, through the same helper
        if matches!(v, Value::ErrV(_)) {
            return h;
        }
        let rendered = with_interp(|i| i.render_interpolated(v.clone()));
        match rendered {
            Ok(Ok(s)) => out.push_str(&s),
            Ok(Err(err)) => return push(Slot::V(err)),
            Err(fault) => die(fault.message),
        }
    }
    push(Slot::V(Value::Str(out)))
}

#[no_mangle]
pub extern "C" fn rt_binop(op: u32, a: u32, b: u32) -> u32 {
    let a = {
        let Slot::V(v) = slot(a) else { return a };
        push(Slot::V(crate::eval::sub_base(v)))
    };
    let b = {
        let Slot::V(v) = slot(b) else { return b };
        push(Slot::V(crate::eval::sub_base(v)))
    };
    let op = match op {
        0 => "+",
        1 => "-",
        2 => "*",
        3 => "/",
        4 => "%",
        10 => "==",
        11 => "!=",
        12 => "<",
        13 => ">",
        14 => "<=",
        _ => ">=",
    };
    match eval_binop(op, val(a), val(b), SPAN0, &None) {
        Ok(v) => push(Slot::V(v)),
        Err(rt) => die(rt.message),
    }
}

/// Strict indexing: a miss is an err (unlike `at`, whose miss is none).
#[no_mangle]
pub extern "C" fn rt_index(base: u32, index: u32) -> u32 {
    let idx = val(index);
    match index_value(val(base), idx.clone(), SPAN0) {
        Ok(Value::NoneV) => {
            let msg = format!("missing index {}", render(&idx, true));
            push(Slot::V(err_value(Value::Str(msg), None)))
        }
        Ok(v) => push(Slot::V(v)),
        Err(rt) => die(rt.message),
    }
}

/// Lenient indexing: a miss is none — the plain `xs[i]` form.
#[no_mangle]
pub extern "C" fn rt_at(base: u32, index: u32) -> u32 {
    match index_value(val(base), val(index), SPAN0) {
        Ok(v) => push(Slot::V(v)),
        Err(rt) => die(rt.message),
    }
}

#[no_mangle]
pub extern "C" fn rt_truthy(h: u32) -> u32 {
    match val(h) {
        Value::True => 1,
        Value::False => 0,
        other => die(format!("if takes a bool condition (got {})", render(&other, true))),
    }
}

#[no_mangle]
pub extern "C" fn rt_builtin(name_lit: u32, n: u32) -> u32 {
    let name = match val(name_lit) {
        Value::Str(s) => s,
        _ => die("builtin name must be a string".to_string()),
    };
    let name = name.strip_prefix("builtin_").map(str::to_string).unwrap_or(name);
    let handles = pop_args(n);
    if (name == "map" || name == "filter") && handles.len() == 2 {
        if let Slot::C { .. } = slot(handles[1]) {
            return map_or_filter(&name, handles[0], handles[1]);
        }
    }
    let mut args = Vec::with_capacity(handles.len());
    for h in handles {
        args.push(val(h));
    }
    let result = with_interp(|interp| interp.call_builtin(&name, args, SPAN0, &None));
    match result {
        Ok(v) => push(Slot::V(v)),
        Err(rt) => die(rt.message),
    }
}

fn map_or_filter(name: &str, list_h: u32, closure_h: u32) -> u32 {
    let list = val(list_h);
    if is_failure(&list) {
        return list_h;
    }
    let Value::List(items) = list else {
        die(format!("{name} takes a list"));
    };
    let mut out = Vec::new();
    for item in items.iter() {
        let item_h = push(Slot::V(item.clone()));
        let r_h = call_closure(closure_h, vec![item_h]);
        let r = val(r_h);
        if is_failure(&r) {
            return r_h;
        }
        match name {
            "map" => out.push(r),
            _ => match r {
                Value::True => out.push(item.clone()),
                Value::False => {}
                other => die(format!("filter needs a bool (got {})", render(&other, true))),
            },
        }
    }
    push(Slot::V(Value::List(Rc::new(out))))
}

#[no_mangle]
pub extern "C" fn rt_seq(a: u32, b: u32) -> u32 {
    for h in [a, b] {
        if let Slot::V(v) = slot(h) {
            if is_failure(&v) {
                return h;
            }
        }
    }
    match (slot(a), slot(b)) {
        (Slot::V(Value::Desc(da)), Slot::V(Value::Desc(db))) => {
            push(Slot::V(Value::Desc(Rc::new(Desc::Seq(da, db)))))
        }
        (sa, sb) if descish(&sa) && descish(&sb) => push(Slot::Seq(a, b)),
        _ => die("`>>` sequences two effect descriptions".to_string()),
    }
}

#[no_mangle]
pub extern "C" fn rt_maybe_bind(piped: u32, closure: u32) -> u32 {
    match slot(piped) {
        Slot::V(Value::Desc(_)) | Slot::Seq(..) | Slot::Bind(..) => {
            push(Slot::Bind(piped, closure))
        }
        _ => call_closure(closure, vec![piped]),
    }
}

#[no_mangle]
pub extern "C" fn rt_mkclosure(tidx: u32, ncap: u32) -> u32 {
    let env_handles = pop_args(ncap);
    let env = push(Slot::E(Rc::new(env_handles)));
    push(Slot::C { tidx, env })
}

#[no_mangle]
pub extern "C" fn rt_call(callee: u32, n: u32) -> u32 {
    let args = pop_args(n);
    call_closure(callee, args)
}

#[no_mangle]
pub extern "C" fn rt_envget(env: u32, i: u32) -> u32 {
    let Slot::E(handles) = slot(env) else {
        die("bad environment access".to_string());
    };
    handles[i as usize]
}

#[no_mangle]
pub extern "C" fn rt_die(msg_lit: u32) {
    let msg = match val(msg_lit) {
        Value::Str(s) => s,
        _ => "runtime error".to_string(),
    };
    die(msg);
}

#[no_mangle]
pub extern "C" fn rt_list_len(h: u32) -> u32 {
    match slot(h) {
        Slot::E(handles) => handles.len() as u32,
        Slot::V(Value::List(items)) => items.len() as u32,
        _ => 0,
    }
}

/* ---------- execution ---------- */

struct RtExecutor;

impl Executor for RtExecutor {
    fn print(&mut self, text: &str) {
        PRINTS.with(|p| {
            let mut out = p.borrow_mut();
            out.push_str(text);
            out.push('\n');
        });
    }

    fn random(&mut self, n: u64) -> u64 {
        crate::wasm::next_random(n)
    }

    fn args(&mut self) -> Vec<String> {
        Vec::new()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Err("the playground has no stdin".to_string())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        Err(format!("the playground has no filesystem: cannot read {path}"))
    }

    fn write_file(&mut self, path: &str, _content: &str) -> Result<(), String> {
        Err(format!("the playground has no filesystem: cannot write {path}"))
    }
}

fn exec_slot(h: u32) -> Result<u32, String> {
    match slot(h) {
        Slot::V(Value::Desc(d)) => {
            let result = with_interp(|interp| interp.execute(&d, &mut RtExecutor));
            match result {
                Ok(v) => Ok(push(Slot::V(v))),
                Err(rt) => Err(rt.message),
            }
        }
        Slot::Seq(a, b) => {
            let left = exec_slot(a)?;
            if matches!(slot(left), Slot::V(Value::ErrV(_))) {
                return Ok(left);
            }
            exec_slot(b)
        }
        Slot::Bind(inner, closure) => {
            let yielded = exec_slot(inner)?;
            let next = call_closure(closure, vec![yielded]);
            match slot(next) {
                Slot::V(Value::Desc(_)) | Slot::Seq(..) | Slot::Bind(..) => exec_slot(next),
                _ => Ok(next),
            }
        }
        _ => Err("main is not an io".to_string()),
    }
}

/// Runs the value `main` returned. Fills the print transcript and returns
/// (status, text) mirroring the native binary's endpoint behavior.
pub fn exec_main(h: u32) -> (i32, String) {
    PRINTS.with(|p| p.borrow_mut().clear());
    let outcome = match slot(h) {
        Slot::V(Value::Desc(_)) | Slot::Seq(..) | Slot::Bind(..) => match exec_slot(h) {
            Ok(y) => match slot(y) {
                Slot::V(Value::ErrV(info)) => Some((
                    1,
                    format!(
                        "error[endpoint]: unhandled err reached the executor: {}\n{}",
                        render(&info.reason, true),
                        trace_lines(&info)
                    ),
                )),
                _ => Some((0, String::new())),
            },
            Err(msg) => Some((1, format!("error[runtime]: {msg}\n"))),
        },
        Slot::V(Value::ErrV(info)) => Some((
            1,
            format!(
                "error[endpoint]: unhandled err reached main: {}\n{}",
                render(&info.reason, true),
                trace_lines(&info)
            ),
        )),
        Slot::V(Value::NoneV) => {
            Some((1, "error[endpoint]: unhandled none reached main\n".to_string()))
        }
        _ => Some((0, String::new())),
    };
    let (status, tail) = outcome.expect("outcome");
    let mut text = PRINTS.with(|p| p.borrow().clone());
    text.push_str(&tail);
    (status, text)
}
