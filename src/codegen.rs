use crate::ast::*;
use std::collections::HashMap;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
const K_ERR: i64 = 5;

const DECLARES: &str = r#"%KValue = type { i64, i64 }

declare %KValue @k_int(i64)
declare %KValue @k_float(double)
declare %KValue @k_bool(i64)
declare %KValue @k_none()
declare %KValue @k_str_n(ptr, i64)
declare i64 @k_not_failure(%KValue)
declare %KValue @k_err(%KValue)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_field(%KValue, i64)
declare %KValue @k_err_inner(%KValue)
declare i64 @k_check_tag(%KValue, i64)
declare i64 @k_check_int(%KValue, i64)
declare i64 @k_check_rec(%KValue, i64, i64)
declare i64 @k_check_bool(%KValue)
declare i64 @k_check_str(%KValue, ptr, i64)
declare %KValue @k_concat(%KValue, %KValue)
declare %KValue @k_render(%KValue, i64)
declare %KValue @k_add(%KValue, %KValue)
declare %KValue @k_sub(%KValue, %KValue)
declare %KValue @k_mul(%KValue, %KValue)
declare %KValue @k_div(%KValue, %KValue)
declare %KValue @k_cmp(%KValue, %KValue, i64)
declare %KValue @k_desc_print(%KValue)
declare %KValue @k_seq(%KValue, %KValue)
declare i64 @k_truthy(%KValue)
declare void @k_die(ptr) noreturn

"#;

pub fn emit_ir(program: &Program) -> Result<String, String> {
    let mut type_ids = HashMap::new();
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    let mut backend = Backend {
        program,
        type_ids,
        strings: Vec::new(),
        interned: HashMap::new(),
        body: String::new(),
    };
    backend.emit()
}

struct Backend<'a> {
    program: &'a Program,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    interned: HashMap<Vec<u8>, String>,
    body: String,
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
}

impl FnEmit {
    fn new() -> Self {
        FnEmit {
            out: String::new(),
            tmp: 0,
            label: 0,
            cur_label: "entry".to_string(),
            versions: HashMap::new(),
        }
    }

    fn tmp(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }

    fn label(&mut self) -> String {
        self.label += 1;
        format!("L{}", self.label)
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "  {text}");
    }

    fn start_block(&mut self, label: &str) {
        let _ = writeln!(self.out, "{label}:");
        self.cur_label = label.to_string();
    }

    fn bind(&mut self, name: &str, temp: &str) {
        self.versions.insert(name.to_string(), temp.to_string());
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.versions.get(name).cloned()
    }
}

impl<'a> Backend<'a> {
    fn emit(&mut self) -> Result<String, String> {
        self.emit_type_names();
        let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            match groups.last_mut() {
                Some((name, decls)) if *name == decl.name => decls.push(decl),
                _ => groups.push((&decl.name, vec![decl])),
            }
        }
        for (name, decls) in &groups {
            let mut by_arity: HashMap<usize, Vec<&FnDecl>> = HashMap::new();
            for d in decls {
                by_arity.entry(d.params.len()).or_default().push(d);
            }
            let mut arity_keys: Vec<usize> = by_arity.keys().copied().collect();
            arity_keys.sort_unstable();
            for arity in arity_keys {
                self.emit_dispatcher(name, arity, &by_arity[&arity])?;
            }
        }
        let mut out = String::from(DECLARES);
        for (name, bytes) in &self.strings {
            let _ = writeln!(
                out,
                "@{name} = private unnamed_addr constant [{} x i8] c\"{}\"",
                bytes.len(),
                ir_bytes(bytes)
            );
        }
        out.push('\n');
        out.push_str(&self.body);
        Ok(out)
    }

    fn intern(&mut self, text: &str) -> (String, usize) {
        let bytes = text.as_bytes().to_vec();
        let len = bytes.len();
        if let Some(name) = self.interned.get(&bytes) {
            return (name.clone(), len);
        }
        let name = format!("s{}", self.strings.len());
        self.interned.insert(bytes.clone(), name.clone());
        self.strings.push((name.clone(), bytes));
        (name, len)
    }

    fn str_const(&mut self, f: &mut FnEmit, text: &str) -> String {
        let (name, len) = self.intern(text);
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_str_n(ptr @{name}, i64 {len})"));
        t
    }

    fn emit_type_names(&mut self) {
        let mut body = String::new();
        body.push_str("define ptr @k_type_name(i64 %id) {\nentry:\n");
        let mut arms = String::new();
        let mut cases = String::new();
        for ty in &self.program.types {
            let id = self.type_ids[ty.name.as_str()];
            let (name, _len) = self.intern(&ty.name);
            let _ = writeln!(cases, "    i64 {id}, label %T{id}");
            let _ = writeln!(arms, "T{id}:\n  ret ptr @{name}");
        }
        let (fallback, _) = self.intern("record");
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{fallback}");
        body.push_str("}\n\n");
        self.body.push_str(&body);
    }

    fn emit_dispatcher(&mut self, name: &str, arity: usize, decls: &[&FnDecl]) -> Result<(), String> {
        let params: Vec<String> = (0..arity).map(|i| format!("%KValue %x{i}")).collect();
        let mut f = FnEmit::new();
        let header = format!("define %KValue @d_{name}_{arity}({}) {{", params.join(", "));
        f.start_block("entry");
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            f.versions.clear();
            for (i, pattern) in decl.params.iter().enumerate() {
                self.emit_pattern(&mut f, &format!("%x{i}"), pattern, &fail)?;
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            f.start_block(&fail);
        }
        for i in 0..arity {
            let c = f.tmp();
            f.line(&format!("{c} = call i64 @k_not_failure(%KValue %x{i})"));
            let b = f.tmp();
            f.line(&format!("{b} = icmp eq i64 {c}, 0"));
            let ret_label = f.label();
            let next = f.label();
            f.line(&format!("br i1 {b}, label %{ret_label}, label %{next}"));
            f.start_block(&ret_label);
            f.line(&format!("ret %KValue %x{i}"));
            f.start_block(&next);
        }
        let msg = format!("no overload of `{name}` matches these arguments");
        let (m, _len) = self.intern(&format!("{msg}\0"));
        f.line(&format!("call void @k_die(ptr @{m})"));
        f.line("unreachable");
        let _ = writeln!(self.body, "{header}\n{}}}\n", f.out);
        Ok(())
    }

    fn emit_pattern(
        &mut self,
        f: &mut FnEmit,
        value: &str,
        pattern: &Pattern,
        fail: &str,
    ) -> Result<(), String> {
        let check = |backend: &mut Backend, f: &mut FnEmit, call: String| {
            let c = f.tmp();
            f.line(&format!("{c} = {call}"));
            let b = f.tmp();
            f.line(&format!("{b} = icmp ne i64 {c}, 0"));
            let ok = f.label();
            f.line(&format!("br i1 {b}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
            let _ = backend;
        };
        match pattern {
            Pattern::IntLit(n, _) => {
                check(self, f, format!("call i64 @k_check_int(%KValue {value}, i64 {n})"));
            }
            Pattern::StrLit(s, _) => {
                let (name, len) = self.intern(s);
                check(
                    self,
                    f,
                    format!("call i64 @k_check_str(%KValue {value}, ptr @{name}, i64 {len})"),
                );
            }
            Pattern::Nullary(name, _) => {
                let tag = match name.as_str() {
                    "true" => K_TRUE,
                    "false" => K_FALSE,
                    _ => K_NONE,
                };
                check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 {tag})"));
            }
            Pattern::Wildcard(_) => {
                check(self, f, format!("call i64 @k_not_failure(%KValue {value})"));
            }
            Pattern::Var(name, _) => {
                check(self, f, format!("call i64 @k_not_failure(%KValue {value})"));
                f.bind(name, value);
            }
            Pattern::Annotated { name, ty, .. } => {
                let call = match ty.as_str() {
                    "int" => format!("call i64 @k_check_tag(%KValue {value}, i64 0)"),
                    "float64" => format!("call i64 @k_check_tag(%KValue {value}, i64 1)"),
                    "string" => format!("call i64 @k_check_tag(%KValue {value}, i64 6)"),
                    "bool" => format!("call i64 @k_check_bool(%KValue {value})"),
                    "err" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"),
                    other => match self.type_ids.get(other) {
                        Some(id) => {
                            let nfields = self.field_count(other)?;
                            format!("call i64 @k_check_rec(%KValue {value}, i64 {id}, i64 {nfields})")
                        }
                        None => return Err(format!("native backend: unknown type `{other}`")),
                    },
                };
                check(self, f, call);
                f.bind(name, value);
            }
            Pattern::Ctor { ty, fields } => {
                if ty == "err" {
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"));
                    let inner = f.tmp();
                    f.line(&format!("{inner} = call %KValue @k_err_inner(%KValue {value})"));
                    return self.emit_pattern(f, &inner, &fields[0], fail);
                }
                let id = *self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                check(
                    self,
                    f,
                    format!(
                        "call i64 @k_check_rec(%KValue {value}, i64 {id}, i64 {})",
                        fields.len()
                    ),
                );
                for (i, field) in fields.iter().enumerate() {
                    let fv = f.tmp();
                    f.line(&format!("{fv} = call %KValue @k_field(%KValue {value}, i64 {i})"));
                    self.emit_pattern(f, &fv, field, fail)?;
                }
            }
            Pattern::Keyed { .. } => {
                return Err("native backend: keyed patterns are slice 2".to_string())
            }
        }
        Ok(())
    }

    fn field_count(&self, ty: &str) -> Result<usize, String> {
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.len())
            .ok_or_else(|| format!("native backend: unknown type `{ty}`"))
    }

    fn emit_fn_body(&mut self, f: &mut FnEmit, body: &[Stmt]) -> Result<(), String> {
        let last = body.len() - 1;
        for (i, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    let value = self.emit_expr(f, expr)?;
                    match pattern {
                        Pattern::Var(name, _) => f.bind(name, &value),
                        Pattern::Ctor { ty, fields } => {
                            let id = *self
                                .type_ids
                                .get(ty.as_str())
                                .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                            let c = f.tmp();
                            f.line(&format!(
                                "{c} = call i64 @k_check_rec(%KValue {value}, i64 {id}, i64 {})",
                                fields.len()
                            ));
                            let b = f.tmp();
                            f.line(&format!("{b} = icmp ne i64 {c}, 0"));
                            let ok = f.label();
                            let bad = f.label();
                            f.line(&format!("br i1 {b}, label %{ok}, label %{bad}"));
                            f.start_block(&bad);
                            let msg = format!("cannot destructure value as `{ty}`\0");
                            let (m, _) = self.intern(&msg);
                            f.line(&format!("call void @k_die(ptr @{m})"));
                            f.line("unreachable");
                            f.start_block(&ok);
                            for (i, field) in fields.iter().enumerate() {
                                if let Pattern::Var(name, _) = field {
                                    let fv = f.tmp();
                                    f.line(&format!(
                                        "{fv} = call %KValue @k_field(%KValue {value}, i64 {i})"
                                    ));
                                    f.bind(name, &fv);
                                }
                            }
                        }
                        _ => {
                            return Err(
                                "native backend: keyed binding patterns are slice 2".to_string()
                            )
                        }
                    }
                }
                Stmt::Expr(expr) => {
                    let value = self.emit_expr(f, expr)?;
                    if i == last {
                        f.line(&format!("ret %KValue {value}"));
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Int(n, _) => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_int(i64 {n})"));
                Ok(t)
            }
            Expr::Float(x, _) => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_float(double 0x{:016X})", x.to_bits()));
                Ok(t)
            }
            Expr::Str(parts, _) => {
                let mut acc: Option<String> = None;
                for part in parts {
                    let piece = match part {
                        TemplatePart::Lit(s) => self.str_const(f, s),
                        TemplatePart::Interp(inner) => {
                            let value = self.emit_expr(f, inner)?;
                            let t = f.tmp();
                            f.line(&format!("{t} = call %KValue @k_render(%KValue {value}, i64 0)"));
                            t
                        }
                    };
                    acc = Some(match acc {
                        None => piece,
                        Some(prev) => {
                            let t = f.tmp();
                            f.line(&format!(
                                "{t} = call %KValue @k_concat(%KValue {prev}, %KValue {piece})"
                            ));
                            t
                        }
                    });
                }
                Ok(match acc {
                    Some(t) => t,
                    None => self.str_const(f, ""),
                })
            }
            Expr::Ident(name, _) => {
                if let Some(temp) = f.lookup(name) {
                    return Ok(temp);
                }
                if self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty()) {
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @d_{name}_0()"));
                    return Ok(t);
                }
                let t = f.tmp();
                match name.as_str() {
                    "true" => f.line(&format!("{t} = call %KValue @k_bool(i64 1)")),
                    "false" => f.line(&format!("{t} = call %KValue @k_bool(i64 0)")),
                    "none" => f.line(&format!("{t} = call %KValue @k_none()")),
                    _ => {
                        return Err(format!(
                            "native backend: `{name}` as a bare value is slice 2 (function values)"
                        ))
                    }
                }
                Ok(t)
            }
            Expr::App { head, args, .. } => self.emit_call(f, head, args),
            Expr::Seq(lhs, rhs, _) => {
                let a = self.emit_expr(f, lhs)?;
                let b = self.emit_expr(f, rhs)?;
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_seq(%KValue {a}, %KValue {b})"));
                Ok(t)
            }
            Expr::BinOp { op, lhs, rhs, .. } => {
                let a = self.emit_expr(f, lhs)?;
                let b = self.emit_expr(f, rhs)?;
                let t = f.tmp();
                let call = match *op {
                    "+" => format!("call %KValue @k_add(%KValue {a}, %KValue {b})"),
                    "-" => format!("call %KValue @k_sub(%KValue {a}, %KValue {b})"),
                    "*" => format!("call %KValue @k_mul(%KValue {a}, %KValue {b})"),
                    "/" => format!("call %KValue @k_div(%KValue {a}, %KValue {b})"),
                    "==" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 0)"),
                    "!=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 1)"),
                    "<" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 2)"),
                    "<=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 3)"),
                    ">" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 4)"),
                    _ => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 5)"),
                };
                f.line(&format!("{t} = {call}"));
                Ok(t)
            }
            Expr::Lambda { .. } => Err("native backend: lambdas are slice 2".to_string()),
            Expr::List(..) | Expr::MapLit(..) => {
                Err("native backend: lists and maps are slice 2".to_string())
            }
        }
    }

    fn emit_call(&mut self, f: &mut FnEmit, head: &Expr, args: &[Expr]) -> Result<String, String> {
        let Expr::Ident(name, _) = head else {
            return Err("native backend: computed call heads are slice 2".to_string());
        };
        if f.lookup(name).is_some() {
            return Err("native backend: calling local function values is slice 2".to_string());
        }
        if name == "if" {
            let cond = self.emit_expr(f, &args[0])?;
            let nf = f.tmp();
            f.line(&format!("{nf} = call i64 @k_not_failure(%KValue {cond})"));
            let ok = f.tmp();
            f.line(&format!("{ok} = icmp ne i64 {nf}, 0"));
            let check = f.label();
            let merge = f.label();
            let fail_from = f.cur_label.clone();
            f.line(&format!("br i1 {ok}, label %{check}, label %{merge}"));
            f.start_block(&check);
            let tv = f.tmp();
            f.line(&format!("{tv} = call i64 @k_truthy(%KValue {cond})"));
            let tb = f.tmp();
            f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
            let then_label = f.label();
            let else_label = f.label();
            f.line(&format!("br i1 {tb}, label %{then_label}, label %{else_label}"));
            f.start_block(&then_label);
            let then_value = self.emit_expr(f, &args[1])?;
            let then_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&else_label);
            let else_value = self.emit_expr(f, &args[2])?;
            let else_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {cond}, %{fail_from} ], [ {then_value}, %{then_from} ], \
                 [ {else_value}, %{else_from} ]"
            ));
            return Ok(t);
        }
        let mut emitted = Vec::new();
        for arg in args {
            emitted.push(self.emit_expr(f, arg)?);
        }
        if name == "err" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_err(%KValue {})", emitted[0]));
            return Ok(t);
        }
        if name == "print" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_print(%KValue {})", emitted[0]));
            return Ok(t);
        }
        if let Some(id) = self.type_ids.get(name.as_str()).copied() {
            let n = emitted.len();
            let arr = f.tmp();
            f.line(&format!("{arr} = alloca [{n} x %KValue]"));
            for (i, value) in emitted.iter().enumerate() {
                let slot = f.tmp();
                f.line(&format!(
                    "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                ));
                f.line(&format!("store %KValue {value}, ptr {slot}"));
            }
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_rec(i64 {id}, i64 {n}, ptr {arr})"));
            return Ok(t);
        }
        if self.program.fns.iter().any(|d| d.name == *name) {
            let n = emitted.len();
            let args_ir: Vec<String> = emitted.iter().map(|e| format!("%KValue {e}")).collect();
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @d_{name}_{n}({})", args_ir.join(", ")));
            return Ok(t);
        }
        Err(format!("native backend: builtin `{name}` is slice 2"))
    }
}

fn ir_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        match byte {
            0x20..=0x7e if *byte != b'"' && *byte != b'\\' => out.push(*byte as char),
            _ => {
                let _ = write!(out, "\\{byte:02X}");
            }
        }
    }
    out
}
