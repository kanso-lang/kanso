use crate::ast::*;
use std::collections::HashMap;
use std::fmt::Write as _;

pub struct Backend<'a> {
    program: &'a Program,
    type_ids: HashMap<&'a str, usize>,
    out: String,
}

pub fn emit_c(program: &Program) -> Result<String, String> {
    let mut type_ids = HashMap::new();
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), i + 1);
    }
    let mut backend = Backend { program, type_ids, out: String::new() };
    backend.emit()?;
    Ok(backend.out)
}

const RUNTIME: &str = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

typedef enum { K_INT, K_FLOAT, K_TRUE, K_FALSE, K_NONE, K_ERR, K_STR, K_REC, K_DESC } k_tag;

typedef struct KValue KValue;
typedef struct KStr { long len; char* data; } KStr;
typedef struct KRec { int type_id; int nfields; KValue* fields; } KRec;
typedef struct KDesc KDesc;

struct KValue {
    k_tag tag;
    union { long long i; double f; KStr* s; KRec* r; KDesc* d; KValue* boxed; } as;
};

struct KDesc { int dtag; KStr* text; KDesc* a; KDesc* b; };

static void* k_alloc(size_t n) {
    void* p = malloc(n);
    if (!p) { fputs("out of memory\n", stderr); exit(1); }
    return p;
}

static void k_die(const char* msg) {
    fprintf(stderr, "error[runtime]: %s\n", msg);
    exit(1);
}

static KValue k_int(long long i) { KValue v; v.tag = K_INT; v.as.i = i; return v; }
static KValue k_bool(int b) { KValue v; v.tag = b ? K_TRUE : K_FALSE; return v; }
static KValue k_none(void) { KValue v; v.tag = K_NONE; return v; }

static KValue k_str_n(const char* data, long len) {
    KStr* s = k_alloc(sizeof(KStr));
    s->len = len;
    s->data = k_alloc(len + 1);
    memcpy(s->data, data, len);
    s->data[len] = 0;
    KValue v; v.tag = K_STR; v.as.s = s; return v;
}

static KValue k_str(const char* data) { return k_str_n(data, (long)strlen(data)); }

static int k_is_failure(KValue v) { return v.tag == K_ERR || v.tag == K_NONE; }

static KValue k_err(KValue reason) {
    if (k_is_failure(reason)) return reason;
    KValue* boxed = k_alloc(sizeof(KValue));
    *boxed = reason;
    KValue v; v.tag = K_ERR; v.as.boxed = boxed; return v;
}

static KValue k_rec(int type_id, int n, KValue* args) {
    for (int i = 0; i < n; i++) if (k_is_failure(args[i])) return args[i];
    KRec* r = k_alloc(sizeof(KRec));
    r->type_id = type_id;
    r->nfields = n;
    r->fields = k_alloc(sizeof(KValue) * n);
    memcpy(r->fields, args, sizeof(KValue) * n);
    KValue v; v.tag = K_REC; v.as.r = r; return v;
}

static KValue k_concat(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    KStr* s = k_alloc(sizeof(KStr));
    s->len = a.as.s->len + b.as.s->len;
    s->data = k_alloc(s->len + 1);
    memcpy(s->data, a.as.s->data, a.as.s->len);
    memcpy(s->data + a.as.s->len, b.as.s->data, b.as.s->len);
    s->data[s->len] = 0;
    KValue v; v.tag = K_STR; v.as.s = s; return v;
}

static const char* k_type_name(int type_id);
static KValue k_render(KValue v, int quote);

static KValue k_render_rec(KValue v) {
    KValue out = k_str(k_type_name(v.as.r->type_id));
    for (int i = 0; i < v.as.r->nfields; i++) {
        out = k_concat(out, k_str(i == 0 ? " " : " "));
        out = k_concat(out, k_render(v.as.r->fields[i], 1));
    }
    return out;
}

static KValue k_render(KValue v, int quote) {
    char buf[64];
    switch (v.tag) {
        case K_INT:
            snprintf(buf, sizeof buf, "%lld", v.as.i);
            return k_str(buf);
        case K_FLOAT:
            if (v.as.f == floor(v.as.f) && fabs(v.as.f) < 1e15 && isfinite(v.as.f)) {
                snprintf(buf, sizeof buf, "%.1f", v.as.f);
            } else {
                snprintf(buf, sizeof buf, "%.17g", v.as.f);
            }
            return k_str(buf);
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("none");
        case K_ERR: {
            KValue inner = k_render(*v.as.boxed, 1);
            return k_concat(k_str("err "), inner);
        }
        case K_STR:
            if (!quote) return v;
            {
                KValue out = k_str("\"");
                out = k_concat(out, v);
                return k_concat(out, k_str("\""));
            }
        case K_REC: return k_render_rec(v);
        case K_DESC: return k_str("<description>");
    }
    return k_str("<value>");
}

static int k_eq(KValue a, KValue b) {
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case K_INT: return a.as.i == b.as.i;
        case K_FLOAT: return a.as.f == b.as.f;
        case K_TRUE: case K_FALSE: case K_NONE: return 1;
        case K_STR:
            return a.as.s->len == b.as.s->len
                && memcmp(a.as.s->data, b.as.s->data, a.as.s->len) == 0;
        case K_REC: {
            if (a.as.r->type_id != b.as.r->type_id) return 0;
            for (int i = 0; i < a.as.r->nfields; i++) {
                if (!k_eq(a.as.r->fields[i], b.as.r->fields[i])) return 0;
            }
            return 1;
        }
        default: return 0;
    }
}

static KValue k_add(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_add_overflow(a.as.i, b.as.i, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) { KValue v; v.tag = K_FLOAT; v.as.f = a.as.f + b.as.f; return v; }
    k_die("`+` is not defined for these values");
    return k_none();
}

static KValue k_sub(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_sub_overflow(a.as.i, b.as.i, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) { KValue v; v.tag = K_FLOAT; v.as.f = a.as.f - b.as.f; return v; }
    k_die("`-` is not defined for these values");
    return k_none();
}

static KValue k_mul(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_mul_overflow(a.as.i, b.as.i, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) { KValue v; v.tag = K_FLOAT; v.as.f = a.as.f * b.as.f; return v; }
    k_die("`*` is not defined for these values");
    return k_none();
}

static KValue k_div(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.as.i == 0) return k_err(k_str("division by zero"));
        return k_int(a.as.i / b.as.i);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        if (b.as.f == 0.0) return k_err(k_str("division by zero"));
        KValue v; v.tag = K_FLOAT; v.as.f = a.as.f / b.as.f; return v;
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.as.i > b.as.i) - (a.as.i < b.as.i);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return (a.as.f > b.as.f) - (a.as.f < b.as.f);
    if (a.tag == K_STR && b.tag == K_STR) {
        long n = a.as.s->len < b.as.s->len ? a.as.s->len : b.as.s->len;
        int c = memcmp(a.as.s->data, b.as.s->data, n);
        if (c) return c > 0 ? 1 : -1;
        return (a.as.s->len > b.as.s->len) - (a.as.s->len < b.as.s->len);
    }
    k_die("comparison requires two values of one comparable type");
    return 0;
}

static KValue k_cmp(KValue a, KValue b, int op) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (op == 0) return k_bool(k_eq(a, b));
    if (op == 1) return k_bool(!k_eq(a, b));
    int c = k_order(a, b);
    switch (op) {
        case 2: return k_bool(c < 0);
        case 3: return k_bool(c <= 0);
        case 4: return k_bool(c > 0);
        default: return k_bool(c >= 0);
    }
}

static KValue k_desc_print(KValue text) {
    if (k_is_failure(text)) return text;
    if (text.tag != K_STR) k_die("print takes a string; interpolate instead");
    KDesc* d = k_alloc(sizeof(KDesc));
    d->dtag = 0; d->text = text.as.s; d->a = d->b = NULL;
    KValue v; v.tag = K_DESC; v.as.d = d; return v;
}

static KValue k_seq(KValue a, KValue b) {
    if (k_is_failure(a)) return a;
    if (k_is_failure(b)) return b;
    if (a.tag != K_DESC || b.tag != K_DESC) k_die("`>>` sequences two effect descriptions");
    KDesc* d = k_alloc(sizeof(KDesc));
    d->dtag = 1; d->text = NULL; d->a = a.as.d; d->b = b.as.d;
    KValue v; v.tag = K_DESC; v.as.d = d; return v;
}

static void k_exec(KDesc* d) {
    if (d->dtag == 0) {
        fwrite(d->text->data, 1, d->text->len, stdout);
        fputc('\n', stdout);
    } else {
        k_exec(d->a);
        k_exec(d->b);
    }
}

static int k_truthy(KValue v) {
    if (v.tag == K_TRUE) return 1;
    if (v.tag == K_FALSE) return 0;
    k_die("an if condition is true or false");
    return 0;
}
"#;

impl<'a> Backend<'a> {
    fn emit(&mut self) -> Result<(), String> {
        self.out.push_str(RUNTIME);
        self.emit_type_names();
        let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            match groups.last_mut() {
                Some((name, decls)) if *name == decl.name => decls.push(decl),
                _ => groups.push((&decl.name, vec![decl])),
            }
        }
        let mut arities: Vec<(String, Vec<usize>)> = Vec::new();
        for (name, decls) in &groups {
            let mut seen: Vec<usize> = Vec::new();
            for d in decls {
                if !seen.contains(&d.params.len()) {
                    seen.push(d.params.len());
                }
            }
            arities.push((name.to_string(), seen));
        }
        for (name, ns) in &arities {
            for n in ns {
                let params: Vec<String> = (0..*n).map(|i| format!("KValue x{i}")).collect();
                let _ = writeln!(self.out, "static KValue d_{name}_{n}({});", params.join(", "));
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
        self.emit_main();
        Ok(())
    }

    fn emit_type_names(&mut self) {
        self.out.push_str("static const char* k_type_name(int type_id) {\n    switch (type_id) {\n");
        for ty in &self.program.types {
            let id = self.type_ids[ty.name.as_str()];
            let _ = writeln!(self.out, "        case {id}: return \"{}\";", ty.name);
        }
        self.out.push_str("        default: return \"record\";\n    }\n}\n\n");
    }

    fn emit_dispatcher(&mut self, name: &str, arity: usize, decls: &[&FnDecl]) -> Result<(), String> {
        let params: Vec<String> = (0..arity).map(|i| format!("KValue x{i}")).collect();
        let _ = writeln!(self.out, "static KValue d_{name}_{arity}({}) {{", params.join(", "));
        for decl in decls {
            let mut cond = Vec::new();
            let mut binds = Vec::new();
            for (i, pattern) in decl.params.iter().enumerate() {
                self.pattern_check(pattern, &format!("x{i}"), &mut cond, &mut binds)?;
            }
            let cond = match cond.is_empty() {
                true => "1".to_string(),
                false => cond.join(" && "),
            };
            let _ = writeln!(self.out, "    if ({cond}) {{");
            let mut ctx = BodyCtx::new(binds);
            let body = self.emit_body(&decl.body, &mut ctx)?;
            self.out.push_str(&body);
            self.out.push_str("    }\n");
        }
        for i in 0..arity {
            let _ = writeln!(self.out, "    if (k_is_failure(x{i})) return x{i};");
        }
        let _ = writeln!(self.out, "    k_die(\"no overload of `{name}` matches these arguments\");");
        self.out.push_str("    return k_none();\n}\n\n");
        Ok(())
    }

    fn pattern_check(
        &self,
        pattern: &Pattern,
        access: &str,
        cond: &mut Vec<String>,
        binds: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        match pattern {
            Pattern::IntLit(n, _) => {
                cond.push(format!("({access}.tag == K_INT && {access}.as.i == {n}LL)"));
            }
            Pattern::StrLit(s, _) => {
                let lit = c_string(s);
                cond.push(format!("({access}.tag == K_STR && k_eq({access}, k_str({lit})))"));
            }
            Pattern::Nullary(name, _) => {
                let tag = match name.as_str() {
                    "true" => "K_TRUE",
                    "false" => "K_FALSE",
                    _ => "K_NONE",
                };
                cond.push(format!("({access}.tag == {tag})"));
            }
            Pattern::Wildcard(_) => {
                cond.push(format!("!k_is_failure({access})"));
            }
            Pattern::Var(name, _) => {
                cond.push(format!("!k_is_failure({access})"));
                binds.push((name.clone(), access.to_string()));
            }
            Pattern::Annotated { name, ty, .. } => {
                let check = match ty.as_str() {
                    "int" => format!("{access}.tag == K_INT"),
                    "float64" => format!("{access}.tag == K_FLOAT"),
                    "string" => format!("{access}.tag == K_STR"),
                    "bool" => format!("({access}.tag == K_TRUE || {access}.tag == K_FALSE)"),
                    "err" => format!("{access}.tag == K_ERR"),
                    other => match self.type_ids.get(other) {
                        Some(id) => {
                            format!("({access}.tag == K_REC && {access}.as.r->type_id == {id})")
                        }
                        None => return Err(format!("native backend: unknown type `{other}`")),
                    },
                };
                cond.push(format!("({check})"));
                binds.push((name.clone(), access.to_string()));
            }
            Pattern::Ctor { ty, fields } => {
                if ty == "err" {
                    cond.push(format!("({access}.tag == K_ERR)"));
                    self.pattern_check(&fields[0], &format!("(*{access}.as.boxed)"), cond, binds)?;
                    return Ok(());
                }
                let id = self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                cond.push(format!(
                    "({access}.tag == K_REC && {access}.as.r->type_id == {id} && \
                     {access}.as.r->nfields == {})",
                    fields.len()
                ));
                for (i, field) in fields.iter().enumerate() {
                    self.pattern_check(field, &format!("{access}.as.r->fields[{i}]"), cond, binds)?;
                }
            }
            Pattern::Keyed { .. } => {
                return Err("native backend: keyed patterns are slice 2".to_string())
            }
        }
        Ok(())
    }

    fn emit_body(&self, body: &[Stmt], ctx: &mut BodyCtx) -> Result<String, String> {
        let mut out = String::new();
        for (name, access) in ctx.initial_binds.clone() {
            let var = ctx.fresh(&name);
            let _ = writeln!(out, "        KValue {var} = {access};");
        }
        let last = body.len() - 1;
        for (i, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    let value = self.emit_expr(expr, ctx)?;
                    match pattern {
                        Pattern::Var(name, _) => {
                            let var = ctx.fresh(name);
                            let _ = writeln!(out, "        KValue {var} = {value};");
                        }
                        Pattern::Ctor { ty, fields } => {
                            let id = self
                                .type_ids
                                .get(ty.as_str())
                                .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                            let tmp = ctx.fresh("destructured");
                            let _ = writeln!(out, "        KValue {tmp} = {value};");
                            let _ = writeln!(
                                out,
                                "        if (!({tmp}.tag == K_REC && {tmp}.as.r->type_id == {id})) \
                                 k_die(\"cannot destructure value as `{ty}`\");"
                            );
                            for (i, field) in fields.iter().enumerate() {
                                if let Pattern::Var(name, _) = field {
                                    let var = ctx.fresh(name);
                                    let _ = writeln!(
                                        out,
                                        "        KValue {var} = {tmp}.as.r->fields[{i}];"
                                    );
                                }
                            }
                        }
                        _ => return Err("native backend: keyed binding patterns are slice 2".to_string()),
                    }
                }
                Stmt::Expr(expr) => {
                    let value = self.emit_expr(expr, ctx)?;
                    if i == last {
                        let _ = writeln!(out, "        return {value};");
                    }
                }
            }
        }
        Ok(out)
    }

    fn emit_expr(&self, expr: &Expr, ctx: &mut BodyCtx) -> Result<String, String> {
        match expr {
            Expr::Int(n, _) => Ok(format!("k_int({n}LL)")),
            Expr::Float(x, _) => Ok(format!("((KValue){{ .tag = K_FLOAT, .as.f = {x:?} }})")),
            Expr::Str(parts, _) => {
                let mut acc = String::from("k_str(\"\")");
                for part in parts {
                    let piece = match part {
                        TemplatePart::Lit(s) => format!("k_str({})", c_string(s)),
                        TemplatePart::Interp(inner) => {
                            let value = self.emit_expr(inner, ctx)?;
                            format!("k_render({value}, 0)")
                        }
                    };
                    acc = format!("k_concat({acc}, {piece})");
                }
                Ok(acc)
            }
            Expr::Ident(name, _) => {
                if let Some(var) = ctx.lookup(name) {
                    return Ok(var);
                }
                match name.as_str() {
                    "true" => Ok("k_bool(1)".to_string()),
                    "false" => Ok("k_bool(0)".to_string()),
                    "none" => Ok("k_none()".to_string()),
                    _ => Err(format!(
                        "native backend: `{name}` as a bare value is slice 2 (function values)"
                    )),
                }
            }
            Expr::App { head, args, .. } => self.emit_call(head, args, ctx),
            Expr::Seq(lhs, rhs, _) => {
                let a = self.emit_expr(lhs, ctx)?;
                let b = self.emit_expr(rhs, ctx)?;
                Ok(format!("k_seq({a}, {b})"))
            }
            Expr::BinOp { op, lhs, rhs, .. } => {
                let a = self.emit_expr(lhs, ctx)?;
                let b = self.emit_expr(rhs, ctx)?;
                let call = match *op {
                    "+" => format!("k_add({a}, {b})"),
                    "-" => format!("k_sub({a}, {b})"),
                    "*" => format!("k_mul({a}, {b})"),
                    "/" => format!("k_div({a}, {b})"),
                    "==" => format!("k_cmp({a}, {b}, 0)"),
                    "!=" => format!("k_cmp({a}, {b}, 1)"),
                    "<" => format!("k_cmp({a}, {b}, 2)"),
                    "<=" => format!("k_cmp({a}, {b}, 3)"),
                    ">" => format!("k_cmp({a}, {b}, 4)"),
                    _ => format!("k_cmp({a}, {b}, 5)"),
                };
                Ok(call)
            }
            Expr::Lambda { .. } => Err("native backend: lambdas are slice 2".to_string()),
            Expr::List(..) | Expr::MapLit(..) => {
                Err("native backend: lists and maps are slice 2".to_string())
            }
        }
    }

    fn emit_call(&self, head: &Expr, args: &[Expr], ctx: &mut BodyCtx) -> Result<String, String> {
        let Expr::Ident(name, _) = head else {
            return Err("native backend: computed call heads are slice 2".to_string());
        };
        if ctx.lookup(name).is_some() {
            return Err("native backend: calling local function values is slice 2".to_string());
        }
        if name == "if" {
            let cond = self.emit_expr(&args[0], ctx)?;
            let then_branch = self.emit_expr(&args[1], ctx)?;
            let else_branch = self.emit_expr(&args[2], ctx)?;
            return Ok(format!(
                "({{ KValue k_c = {cond}; k_is_failure(k_c) ? k_c : (k_truthy(k_c) ? \
                 ({then_branch}) : ({else_branch})); }})"
            ));
        }
        let mut emitted = Vec::new();
        for arg in args {
            emitted.push(self.emit_expr(arg, ctx)?);
        }
        if name == "err" {
            return Ok(format!("k_err({})", emitted[0]));
        }
        if name == "print" {
            return Ok(format!("k_desc_print({})", emitted[0]));
        }
        if let Some(id) = self.type_ids.get(name.as_str()) {
            return Ok(format!(
                "k_rec({id}, {}, (KValue[]){{{}}})",
                emitted.len(),
                emitted.join(", ")
            ));
        }
        if self.program.fns.iter().any(|d| d.name == *name) {
            return Ok(format!("d_{name}_{}({})", emitted.len(), emitted.join(", ")));
        }
        Err(format!("native backend: builtin `{name}` is slice 2"))
    }

    fn emit_main(&mut self) {
        self.out.push_str(
            "int main(void) {\n    KValue v = d_main_0();\n    if (v.tag == K_DESC) { \
             k_exec(v.as.d); return 0; }\n    if (v.tag == K_ERR) {\n        KValue r = \
             k_render(*v.as.boxed, 1);\n        fprintf(stderr, \"error[endpoint]: unhandled \
             err reached main: %s\\n\", r.as.s->data);\n        return 1;\n    }\n    if \
             (v.tag == K_NONE) { fputs(\"error[endpoint]: unhandled none reached main\\n\", \
             stderr); return 1; }\n    return 0;\n}\n",
        );
    }
}

struct BodyCtx {
    initial_binds: Vec<(String, String)>,
    versions: HashMap<String, usize>,
    counter: usize,
}

impl BodyCtx {
    fn new(initial_binds: Vec<(String, String)>) -> Self {
        BodyCtx { initial_binds, versions: HashMap::new(), counter: 0 }
    }

    fn fresh(&mut self, name: &str) -> String {
        self.counter += 1;
        let var = format!("k_{}_{}", sanitize(name), self.counter);
        self.versions.insert(name.to_string(), self.counter);
        var
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.versions.get(name).map(|v| format!("k_{}_{}", sanitize(name), v))
    }
}

fn sanitize(name: &str) -> String {
    name.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
}

fn c_string(s: &str) -> String {
    let mut out = String::from("\"");
    for byte in s.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:03o}");
                out.push_str("\" \"");
            }
        }
    }
    out.push('"');
    out
}
