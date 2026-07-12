use crate::ast::*;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, INT, LIST, MAP, NONE, REC, STR, TOP};
use std::collections::HashMap;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
const K_ERR: i64 = 5;

const DECLARES: &str = r#"%KValue = type { i64, i64 }
%KBytes = type { i64, ptr }

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
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare %KValue @k_list_lit(i64, ptr)
declare %KValue @k_map_lit(i64, ptr)
declare %KValue @k_closure(ptr, i64, ptr)
declare %KValue @k_fnref(ptr)
declare %KValue @k_env_get(ptr, i64)
declare %KValue @k_b_at(%KValue, %KValue)
declare %KValue @k_index(%KValue, %KValue)
declare %KValue @k_b_bytes(%KValue)
declare %KValue @k_b_chars(%KValue)
declare %KValue @k_b_concat(%KValue, %KValue)
declare %KValue @k_b_utf8(%KValue)
declare %KValue @k_b_char_code(%KValue)
declare %KValue @k_b_entries(%KValue)
declare %KValue @k_b_filter(%KValue, %KValue)
declare %KValue @k_b_from_code(%KValue)
declare %KValue @k_b_join(%KValue, %KValue)
declare %KValue @k_b_length(%KValue)
declare %KValue @k_b_map(%KValue, %KValue)
declare %KValue @k_b_push(%KValue, %KValue)
declare %KValue @k_b_put(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice(%KValue, %KValue, %KValue)
declare %KValue @k_b_sort(%KValue)
declare %KValue @k_b_sum(%KValue)
declare %KValue @k_b_to_float(%KValue)
declare %KValue @k_b_to_int(%KValue)

"#;

const BUILTIN_CALLS: [(&str, usize); 19] = [
    ("at", 2),
    ("bytes", 1),
    ("concat", 2),
    ("utf8", 1),
    ("char_code", 1),
    ("chars", 1),
    ("entries", 1),
    ("filter", 2),
    ("from_code", 1),
    ("join", 2),
    ("length", 1),
    ("map", 2),
    ("push", 2),
    ("put", 3),
    ("slice", 3),
    ("sort", 1),
    ("sum", 1),
    ("to_float", 1),
    ("to_int", 1),
];

pub fn emit_ir(program: &Program) -> Result<String, String> {
    let inference = infer::infer(program);
    let mut type_ids = HashMap::new();
    type_ids.insert("entry", 0i64);
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    let mut backend = Backend {
        program,
        inference,
        type_ids,
        strings: Vec::new(),
        interned: HashMap::new(),
        body: String::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
    };
    backend.emit()
}

struct Backend<'a> {
    program: &'a Program,
    inference: infer::Inference,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    interned: HashMap<Vec<u8>, String>,
    body: String,
    lift_counter: usize,
    fn_value_wrappers: Vec<String>,
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
    sets: HashMap<String, Set>,
}

impl FnEmit {
    fn new() -> Self {
        FnEmit {
            out: String::new(),
            tmp: 0,
            label: 0,
            cur_label: "entry".to_string(),
            versions: HashMap::new(),
            sets: HashMap::new(),
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

    fn record(&mut self, operand: &str, set: Set) {
        self.sets.insert(operand.to_string(), set);
    }

    fn set_of(&self, operand: &str) -> Set {
        if operand.starts_with("{ i64 0,") {
            return INT;
        }
        if operand == "{ i64 2, i64 0 }" || operand == "{ i64 3, i64 0 }" {
            return infer::BOOL;
        }
        if operand == "{ i64 4, i64 0 }" {
            return NONE;
        }
        self.sets.get(operand).copied().unwrap_or(TOP)
    }
}

fn inline_tag(f: &mut FnEmit, value: &str) -> String {
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 0"));
    t
}

fn inline_payload(f: &mut FnEmit, value: &str) -> String {
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 1"));
    t
}

fn inline_not_failure(f: &mut FnEmit, value: &str) -> String {
    let tag = inline_tag(f, value);
    let a = f.tmp();
    f.line(&format!("{a} = icmp ne i64 {tag}, 5"));
    let b = f.tmp();
    f.line(&format!("{b} = icmp ne i64 {tag}, 4"));
    let both = f.tmp();
    f.line(&format!("{both} = and i1 {a}, {b}"));
    both
}

impl<'a> Backend<'a> {
    fn group_indices(&self, name: &str, arity: usize) -> Vec<usize> {
        self.program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, d)| d.name == name && d.params.len() == arity)
            .map(|(i, _)| i)
            .collect()
    }

    fn group_param_set(&self, name: &str, arity: usize, param: usize) -> Set {
        self.group_indices(name, arity)
            .iter()
            .fold(0, |acc, i| acc | self.inference.params[*i][param])
    }

    fn group_return_set(&self, name: &str, arity: usize) -> Set {
        self.group_indices(name, arity)
            .iter()
            .fold(0, |acc, i| acc | self.inference.returns[*i])
    }

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
        self.fn_value_wrappers.sort();
        self.fn_value_wrappers.dedup();
        for name in &self.fn_value_wrappers {
            let _ = writeln!(
                self.body,
                "define %KValue @w_{name}_1(%KValue %a0) {{\nentry:\n  %r = call tailcc \
                 %KValue @d_{name}_1(%KValue %a0)\n  ret %KValue %r\n}}\n"
            );
        }
        self.body.push_str(
            "define %KValue @k_user_main() {\nentry:\n  %r = call tailcc %KValue \
             @d_main_0()\n  ret %KValue %r\n}\n",
        );
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
        let (entry_name, _) = self.intern("entry");
        let _ = writeln!(cases, "    i64 0, label %T0");
        let _ = writeln!(arms, "T0:\n  ret ptr @{entry_name}");
        let (fallback, _) = self.intern("record");
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{fallback}");
        body.push_str("}\n\n");
        self.body.push_str(&body);
    }

    /// A group whose arms discriminate on one parameter with int/none literals
    /// (other params generic) compiles to a switch instead of an arm cascade.
    fn switch_shape(decls: &[&FnDecl]) -> Option<usize> {
        let arity = decls[0].params.len();
        if arity == 0 {
            return None;
        }
        let mut disc: Option<usize> = None;
        let mut int_arms = 0;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    Pattern::IntLit(..) | Pattern::Nullary(..) => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                        if matches!(pattern, Pattern::IntLit(..)) {
                            int_arms += 1;
                        }
                    }
                    _ => return None,
                }
            }
        }
        match (disc, int_arms >= 2) {
            (Some(d), true) => Some(d),
            _ => None,
        }
    }

    fn emit_switch_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
        disc: usize,
    ) -> Result<(), String> {
        let params: Vec<String> = (0..arity).map(|i| format!("%KValue %x{i}")).collect();
        let mut f = FnEmit::new();
        let header = format!("define tailcc %KValue @d_{name}_{arity}({}) {{", params.join(", "));
        f.start_block("entry");
        // any non-discriminator failure means no arm can match: propagate leftmost
        let mut all_ok: Option<String> = None;
        for i in 0..arity {
            if i == disc {
                continue;
            }
            if self.group_param_set(name, arity, i) & FAIL == 0 {
                continue;
            }
            let ok = inline_not_failure(&mut f, &format!("%x{i}"));
            all_ok = Some(match all_ok {
                None => ok,
                Some(prev) => {
                    let t = f.tmp();
                    f.line(&format!("{t} = and i1 {prev}, {ok}"));
                    t
                }
            });
        }
        let dispatch = f.label();
        if let Some(ok) = all_ok {
            let propagate = f.label();
            f.line(&format!("br i1 {ok}, label %{dispatch}, label %{propagate}"));
            f.start_block(&propagate);
            for i in 0..arity {
                let good = inline_not_failure(&mut f, &format!("%x{i}"));
                let next = f.label();
                let ret_it = f.label();
                f.line(&format!("br i1 {good}, label %{next}, label %{ret_it}"));
                f.start_block(&ret_it);
                f.line(&format!("ret %KValue %x{i}"));
                f.start_block(&next);
            }
            f.line("unreachable");
        } else {
            f.line(&format!("br label %{dispatch}"));
        }
        f.start_block(&dispatch);
        let dv = format!("%x{disc}");
        let tag = inline_tag(&mut f, &dv);
        // classify arms
        let mut int_cases: Vec<(String, String)> = Vec::new();
        let mut nullary_cases: Vec<(i64, String)> = Vec::new();
        let mut generic_arm: Option<usize> = None;
        let mut arm_labels = Vec::new();
        for (k, decl) in decls.iter().enumerate() {
            let label = format!("arm{k}");
            arm_labels.push(label.clone());
            match &decl.params[disc] {
                Pattern::IntLit(n, _) => int_cases.push((n.to_string(), label)),
                Pattern::Nullary(nm, _) => {
                    let t = match nm.as_str() {
                        "true" => K_TRUE,
                        "false" => K_FALSE,
                        _ => K_NONE,
                    };
                    nullary_cases.push((t, label));
                }
                _ => generic_arm = Some(k),
            }
        }
        let is_int = f.tmp();
        f.line(&format!("{is_int} = icmp eq i64 {tag}, 0"));
        let int_block = f.label();
        let not_int = f.label();
        f.line(&format!("br i1 {is_int}, label %{int_block}, label %{not_int}"));
        f.start_block(&int_block);
        let payload = inline_payload(&mut f, &dv);
        let generic_label = match generic_arm {
            Some(k) => format!("arm{k}"),
            None => "nomatch".to_string(),
        };
        let cases: Vec<String> = int_cases
            .iter()
            .map(|(n, l)| format!("    i64 {n}, label %{l}"))
            .collect();
        f.line(&format!(
            "switch i64 {payload}, label %{generic_label} [
{}
  ]",
            cases.join("
")
        ));
        f.start_block(&not_int);
        // nullary tags, then generic (non-failure) or propagation
        for (t, l) in &nullary_cases {
            let hit = f.tmp();
            f.line(&format!("{hit} = icmp eq i64 {tag}, {t}"));
            let next = f.label();
            f.line(&format!("br i1 {hit}, label %{l}, label %{next}"));
            f.start_block(&next);
        }
        let disc_ok = inline_not_failure(&mut f, &dv);
        let nomatch = "nomatch".to_string();
        f.line(&format!("br i1 {disc_ok}, label %{generic_label}, label %{nomatch}"));
        f.start_block("nomatch");
        // no arm matched: the discriminator is the only possible failure here
        let disc_fail = f.tmp();
        f.line(&format!("{disc_fail} = extractvalue %KValue {dv}, 0"));
        let is_err = f.tmp();
        f.line(&format!("{is_err} = icmp eq i64 {disc_fail}, 5"));
        let is_none = f.tmp();
        f.line(&format!("{is_none} = icmp eq i64 {disc_fail}, 4"));
        let failing = f.tmp();
        f.line(&format!("{failing} = or i1 {is_err}, {is_none}"));
        let ret_disc = f.label();
        let die = f.label();
        f.line(&format!("br i1 {failing}, label %{ret_disc}, label %{die}"));
        f.start_block(&ret_disc);
        f.line(&format!("ret %KValue {dv}"));
        f.start_block(&die);
        let msg = format!("no overload of `{name}` matches these arguments ");
        let (m, _len) = self.intern(&msg);
        f.line(&format!("call void @k_die(ptr @{m})"));
        f.line("unreachable");
        // arm bodies: patterns are known matched, only bind generics
        for (k, decl) in decls.iter().enumerate() {
            f.start_block(&arm_labels[k]);
            f.versions.clear();
            for (i, pattern) in decl.params.iter().enumerate() {
                if let Pattern::Var(pname, _) = pattern {
                    f.bind(pname, &format!("%x{i}"));
                }
            }
            self.emit_fn_body(&mut f, &decl.body)?;
        }
        let _ = writeln!(self.body, "{header}
{}}}
", f.out);
        Ok(())
    }

    fn emit_dispatcher(&mut self, name: &str, arity: usize, decls: &[&FnDecl]) -> Result<(), String> {
        if let Some(disc) = Self::switch_shape(decls) {
            return self.emit_switch_dispatcher(name, arity, decls, disc);
        }
        let params: Vec<String> = (0..arity).map(|i| format!("%KValue %x{i}")).collect();
        let mut f = FnEmit::new();
        let header = format!("define tailcc %KValue @d_{name}_{arity}({}) {{", params.join(", "));
        f.start_block("entry");
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            f.versions.clear();
            for (i, pattern) in decl.params.iter().enumerate() {
                let known = self.group_param_set(name, arity, i);
                self.emit_pattern_known(&mut f, &format!("%x{i}"), pattern, &fail, known)?;
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            f.start_block(&fail);
        }
        for i in 0..arity {
            let ok = inline_not_failure(&mut f, &format!("%x{i}"));
            let ret_label = f.label();
            let next = f.label();
            f.line(&format!("br i1 {ok}, label %{next}, label %{ret_label}"));
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
        self.emit_pattern_known(f, value, pattern, fail, TOP)
    }

    fn emit_pattern_known(
        &mut self,
        f: &mut FnEmit,
        value: &str,
        pattern: &Pattern,
        fail: &str,
        known: Set,
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
        let branch_i1 = |f: &mut FnEmit, cond: String| {
            let ok = f.label();
            f.line(&format!("br i1 {cond}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
        };
        let tag_is = |f: &mut FnEmit, value: &str, tag: i64| {
            let t = inline_tag(f, value);
            let b = f.tmp();
            f.line(&format!("{b} = icmp eq i64 {t}, {tag}"));
            b
        };
        match pattern {
            Pattern::IntLit(n, _) => {
                let is_int = tag_is(f, value, 0);
                let payload = inline_payload(f, value);
                let eq = f.tmp();
                f.line(&format!("{eq} = icmp eq i64 {payload}, {n}"));
                let both = f.tmp();
                f.line(&format!("{both} = and i1 {is_int}, {eq}"));
                branch_i1(f, both);
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
                let b = tag_is(f, value, tag);
                branch_i1(f, b);
            }
            Pattern::Wildcard(_) => {
                if known & FAIL != 0 {
                    let ok = inline_not_failure(f, value);
                    branch_i1(f, ok);
                }
            }
            Pattern::Var(name, _) => {
                if known & FAIL != 0 {
                    let ok = inline_not_failure(f, value);
                    branch_i1(f, ok);
                }
                f.bind(name, value);
                f.record(value, known & !FAIL);
            }
            Pattern::Annotated { name, ty, .. } => {
                if ty.ends_with("[]") {
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 9)"));
                    f.bind(name, value);
                    return Ok(());
                }
                if ty.contains('[') {
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 10)"));
                    f.bind(name, value);
                    return Ok(());
                }
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
                    if i == last {
                        self.emit_tail(f, expr)?;
                    } else {
                        let _ = self.emit_expr(f, expr)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Int(n, _) => Ok(format!("{{ i64 0, i64 {n} }}")),
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
                let out = match acc {
                    Some(t) => t,
                    None => self.str_const(f, ""),
                };
                f.record(&out, STR);
                Ok(out)
            }
            Expr::Ident(name, _) => {
                if let Some(temp) = f.lookup(name) {
                    return Ok(temp);
                }
                if self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty()) {
                    let t = f.tmp();
                    f.line(&format!("{t} = call tailcc %KValue @d_{name}_0()"));
                    let ret = self.group_return_set(name, 0);
                    f.record(&t, ret);
                    return Ok(t);
                }
                let arities: Vec<usize> = {
                    let mut seen = Vec::new();
                    for d in self.program.fns.iter().filter(|d| d.name == *name) {
                        if !seen.contains(&d.params.len()) {
                            seen.push(d.params.len());
                        }
                    }
                    seen
                };
                if arities == [1] {
                    self.fn_value_wrappers.push(name.clone());
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_fnref(ptr @w_{name}_1)"));
                    return Ok(t);
                }
                if !arities.is_empty() {
                    return Err(format!(
                        "native backend: `{name}` as a function value needs arity 1"
                    ));
                }
                match name.as_str() {
                    "true" => Ok("{ i64 2, i64 0 }".to_string()),
                    "false" => Ok("{ i64 3, i64 0 }".to_string()),
                    "none" => Ok("{ i64 4, i64 0 }".to_string()),
                    _ => Err(format!(
                        "native backend: `{name}` as a bare value is not yet supported"
                    )),
                }
            }
            Expr::App { head, args, .. } => self.emit_call(f, head, args),
            Expr::Index { base, index, .. } => {
                let container = self.emit_expr(f, base)?;
                let key = self.emit_expr(f, index)?;
                Ok(self.emit_at(f, &container, &key, true))
            }
            Expr::Seq(lhs, rhs, _) => {
                let a = self.emit_expr(f, lhs)?;
                let b = self.emit_expr(f, rhs)?;
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_seq(%KValue {a}, %KValue {b})"));
                f.record(&t, DESC | (f.set_of(&a) & FAIL) | (f.set_of(&b) & FAIL));
                Ok(t)
            }
            Expr::BinOp { op, lhs, rhs, .. } => {
                let a = self.emit_expr(f, lhs)?;
                let b = self.emit_expr(f, rhs)?;
                self.emit_binop(f, op, &a, &b)
            }
            Expr::Lambda { params, body, .. } => {
                if params.len() != 1 {
                    return Err("native backend: multi-parameter lambdas need arity 1".to_string());
                }
                let mut idents = Vec::new();
                collect_idents(body, &mut idents);
                let mut captures: Vec<String> = Vec::new();
                for name in idents {
                    if f.lookup(&name).is_some() && !captures.contains(&name) && name != params[0].0 {
                        captures.push(name);
                    }
                }
                let lifted = format!("klam{}", self.lift_counter);
                self.lift_counter += 1;
                self.emit_lifted(&lifted, &params[0].0, &captures, body, f)?;
                let n = captures.len().max(1);
                let arr = f.tmp();
                f.line(&format!("{arr} = alloca [{n} x %KValue]"));
                for (i, cap) in captures.iter().enumerate() {
                    let temp = f.lookup(cap).expect("capture is bound");
                    let slot = f.tmp();
                    f.line(&format!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line(&format!("store %KValue {temp}, ptr {slot}"));
                }
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_closure(ptr @{lifted}, i64 {}, ptr {arr})",
                    captures.len()
                ));
                Ok(t)
            }
            Expr::List(items, _) => {
                let mut emitted = Vec::new();
                for item in items {
                    emitted.push(self.emit_expr(f, item)?);
                }
                let n = emitted.len().max(1);
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
                f.line(&format!(
                    "{t} = call %KValue @k_list_lit(i64 {}, ptr {arr})",
                    emitted.len()
                ));
                f.record(&t, LIST);
                Ok(t)
            }
            Expr::MapLit(pairs, _) => {
                let mut emitted = Vec::new();
                for (key, value) in pairs {
                    emitted.push(self.emit_expr(f, key)?);
                    emitted.push(self.emit_expr(f, value)?);
                }
                let n = emitted.len().max(1);
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
                f.line(&format!(
                    "{t} = call %KValue @k_map_lit(i64 {}, ptr {arr})",
                    pairs.len()
                ));
                f.record(&t, MAP);
                Ok(t)
            }
        }
    }

    /// Emit an expression in tail position: direct calls to kanso functions
    /// become guaranteed tail calls, and an if's branches stay tails.
    fn emit_tail(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<(), String> {
        if let Expr::App { head, args, .. } = expr {
            if let Expr::Ident(name, _) = &**head {
                if name == "if" && f.lookup(name).is_none() {
                    let cond = self.emit_expr(f, &args[0])?;
                    let ok = inline_not_failure(f, &cond);
                    let check = f.label();
                    let bail = f.label();
                    f.line(&format!("br i1 {ok}, label %{check}, label %{bail}"));
                    f.start_block(&bail);
                    f.line(&format!("ret %KValue {cond}"));
                    f.start_block(&check);
                    let tv = f.tmp();
                    f.line(&format!("{tv} = call i64 @k_truthy(%KValue {cond})"));
                    let tb = f.tmp();
                    f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
                    let then_label = f.label();
                    let else_label = f.label();
                    f.line(&format!("br i1 {tb}, label %{then_label}, label %{else_label}"));
                    f.start_block(&then_label);
                    self.emit_tail(f, &args[1])?;
                    f.start_block(&else_label);
                    self.emit_tail(f, &args[2])?;
                    return Ok(());
                }
                let is_program_fn = f.lookup(name).is_none()
                    && !self.type_ids.contains_key(name.as_str())
                    && name != "err"
                    && name != "print"
                    && self.program.fns.iter().any(|d| d.name == *name);
                if is_program_fn {
                    let mut emitted = Vec::new();
                    for arg in args {
                        emitted.push(self.emit_expr(f, arg)?);
                    }
                    let n = emitted.len();
                    let args_ir: Vec<String> =
                        emitted.iter().map(|e| format!("%KValue {e}")).collect();
                    let t = f.tmp();
                    f.line(&format!(
                        "{t} = musttail call tailcc %KValue @d_{name}_{n}({})",
                        args_ir.join(", ")
                    ));
                    f.line(&format!("ret %KValue {t}"));
                    return Ok(());
                }
            }
        }
        let value = self.emit_expr(f, expr)?;
        f.line(&format!("ret %KValue {value}"));
        Ok(())
    }

    fn emit_binop(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
    ) -> Result<String, String> {
        let slow_call = match op {
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
        if op == "/" {
            let t = f.tmp();
            f.line(&format!("{t} = {slow_call}"));
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT | FLOAT | ERR);
            return Ok(t);
        }
        let pure_int = f.set_of(a) == INT && f.set_of(b) == INT;
        if pure_int {
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let t = match op {
                "+" | "-" | "*" => {
                    let intrinsic = match op {
                        "+" => "llvm.sadd.with.overflow.i64",
                        "-" => "llvm.ssub.with.overflow.i64",
                        _ => "llvm.smul.with.overflow.i64",
                    };
                    let pair = f.tmp();
                    f.line(&format!(
                        "{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"
                    ));
                    let sum = f.tmp();
                    f.line(&format!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                    let overflow = f.tmp();
                    f.line(&format!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                    let ok = f.label();
                    let trap = f.label();
                    f.line(&format!("br i1 {overflow}, label %{trap}, label %{ok}"));
                    f.start_block(&trap);
                    let (m, _) = self.intern("integer overflow (int64 native build; spec int is arbitrary precision) ");
                    f.line(&format!("call void @k_die(ptr @{m})"));
                    f.line("unreachable");
                    f.start_block(&ok);
                    let v = f.tmp();
                    f.line(&format!(
                        "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"
                    ));
                    f.record(&v, INT);
                    v
                }
                _ => {
                    let cmp = match op {
                        "==" => "eq",
                        "!=" => "ne",
                        "<" => "slt",
                        "<=" => "sle",
                        ">" => "sgt",
                        _ => "sge",
                    };
                    let c = f.tmp();
                    f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                    let v = f.tmp();
                    f.line(&format!(
                        "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                    ));
                    f.record(&v, infer::BOOL);
                    v
                }
            };
            return Ok(t);
        }
        let ta = inline_tag(f, a);
        let tb = inline_tag(f, b);
        let ia = f.tmp();
        f.line(&format!("{ia} = icmp eq i64 {ta}, 0"));
        let ib = f.tmp();
        f.line(&format!("{ib} = icmp eq i64 {tb}, 0"));
        let both = f.tmp();
        f.line(&format!("{both} = and i1 {ia}, {ib}"));
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line(&format!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let pa = inline_payload(f, a);
        let pb = inline_payload(f, b);
        let (fast_value, fast_from) = match op {
            "+" | "-" | "*" => {
                let intrinsic = match op {
                    "+" => "llvm.sadd.with.overflow.i64",
                    "-" => "llvm.ssub.with.overflow.i64",
                    _ => "llvm.smul.with.overflow.i64",
                };
                let pair = f.tmp();
                f.line(&format!("{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"));
                let sum = f.tmp();
                f.line(&format!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                let overflow = f.tmp();
                f.line(&format!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                let fast_ok = f.label();
                f.line(&format!("br i1 {overflow}, label %{slow}, label %{fast_ok}"));
                f.start_block(&fast_ok);
                let v = f.tmp();
                f.line(&format!(
                    "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"
                ));
                (v, fast_ok)
            }
            _ => {
                let cmp = match op {
                    "==" => "eq",
                    "!=" => "ne",
                    "<" => "slt",
                    "<=" => "sle",
                    ">" => "sgt",
                    _ => "sge",
                };
                let c = f.tmp();
                f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                let v = f.tmp();
                f.line(&format!(
                    "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                ));
                (v, fast.clone())
            }
        };
        f.line(&format!("br label %{merge}"));
        f.start_block(&slow);
        let sv = f.tmp();
        f.line(&format!("{sv} = {slow_call}"));
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!(
            "{t} = phi %KValue [ {fast_value}, %{fast_from} ], [ {sv}, %{slow_from} ]"
        ));
        Ok(t)
    }

    /// bytes-view indexing inlines to a bounds check and a byte load; every
    /// other container falls back to the runtime call.
    fn emit_at(&mut self, f: &mut FnEmit, container: &str, key: &str, strict: bool) -> String {
        let slow_fn = if strict { "k_index" } else { "k_b_at" };
        let proven = f.set_of(container) == BYTES && f.set_of(key) == INT;
        if proven {
            let bp = inline_payload(f, container);
            let bptr = f.tmp();
            f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
            let len_ptr = f.tmp();
            f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
            let len = f.tmp();
            f.line(&format!("{len} = load i64, ptr {len_ptr}"));
            let idx = inline_payload(f, key);
            let ge1 = f.tmp();
            f.line(&format!("{ge1} = icmp sge i64 {idx}, 1"));
            let le_len = f.tmp();
            f.line(&format!("{le_len} = icmp sle i64 {idx}, {len}"));
            let in_range = f.tmp();
            f.line(&format!("{in_range} = and i1 {ge1}, {le_len}"));
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            f.line(&format!("br i1 {in_range}, label %{load}, label %{miss}"));
            f.start_block(&load);
            let data_ptr = f.tmp();
            f.line(&format!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
            let data = f.tmp();
            f.line(&format!("{data} = load ptr, ptr {data_ptr}"));
            let off = f.tmp();
            f.line(&format!("{off} = add i64 {idx}, -1"));
            let byte_ptr = f.tmp();
            f.line(&format!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
            let byte = f.tmp();
            f.line(&format!("{byte} = load i8, ptr {byte_ptr}"));
            let wide = f.tmp();
            f.line(&format!("{wide} = zext i8 {byte} to i64"));
            let hit = f.tmp();
            f.line(&format!(
                "{hit} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"
            ));
            f.line(&format!("br label %{merge}"));
            f.start_block(&miss);
            let miss_value = if strict {
                let mv = f.tmp();
                f.line(&format!(
                    "{mv} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key})"
                ));
                mv
            } else {
                "{ i64 4, i64 0 }".to_string()
            };
            let miss_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {hit}, %{load} ], [ {miss_value}, %{miss_from} ]"
            ));
            f.record(&t, if strict { INT | ERR } else { INT | NONE });
            return t;
        }
        let ct = inline_tag(f, container);
        let is_bytes = f.tmp();
        f.line(&format!("{is_bytes} = icmp eq i64 {ct}, 13"));
        let kt = inline_tag(f, key);
        let is_int = f.tmp();
        f.line(&format!("{is_int} = icmp eq i64 {kt}, 0"));
        let both = f.tmp();
        f.line(&format!("{both} = and i1 {is_bytes}, {is_int}"));
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line(&format!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let bp = inline_payload(f, container);
        let bptr = f.tmp();
        f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
        let len_ptr = f.tmp();
        f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
        let len = f.tmp();
        f.line(&format!("{len} = load i64, ptr {len_ptr}"));
        let idx = inline_payload(f, key);
        let ge1 = f.tmp();
        f.line(&format!("{ge1} = icmp sge i64 {idx}, 1"));
        let le_len = f.tmp();
        f.line(&format!("{le_len} = icmp sle i64 {idx}, {len}"));
        let in_range = f.tmp();
        f.line(&format!("{in_range} = and i1 {ge1}, {le_len}"));
        let load = f.label();
        f.line(&format!("br i1 {in_range}, label %{load}, label %{slow}"));
        f.start_block(&load);
        let data_ptr = f.tmp();
        f.line(&format!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
        let data = f.tmp();
        f.line(&format!("{data} = load ptr, ptr {data_ptr}"));
        let off = f.tmp();
        f.line(&format!("{off} = add i64 {idx}, -1"));
        let byte_ptr = f.tmp();
        f.line(&format!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
        let byte = f.tmp();
        f.line(&format!("{byte} = load i8, ptr {byte_ptr}"));
        let wide = f.tmp();
        f.line(&format!("{wide} = zext i8 {byte} to i64"));
        let fast_value = f.tmp();
        f.line(&format!(
            "{fast_value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"
        ));
        f.line(&format!("br label %{merge}"));
        f.start_block(&slow);
        let slow_value = f.tmp();
        f.line(&format!(
            "{slow_value} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key})"
        ));
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!(
            "{t} = phi %KValue [ {fast_value}, %{load} ], [ {slow_value}, %{slow_from} ]"
        ));
        t
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
            f.record(
                &t,
                f.set_of(&then_value) | f.set_of(&else_value) | (f.set_of(&cond) & FAIL),
            );
            return Ok(t);
        }
        let mut emitted = Vec::new();
        for arg in args {
            emitted.push(self.emit_expr(f, arg)?);
        }
        if name == "err" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_err(%KValue {})", emitted[0]));
            f.record(&t, ERR);
            return Ok(t);
        }
        if name == "print" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_print(%KValue {})", emitted[0]));
            f.record(&t, DESC | (f.set_of(&emitted[0]) & FAIL));
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
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            f.record(&t, REC | fails);
            return Ok(t);
        }
        if self.program.fns.iter().any(|d| d.name == *name) {
            let n = emitted.len();
            let args_ir: Vec<String> = emitted.iter().map(|e| format!("%KValue {e}")).collect();
            let t = f.tmp();
            f.line(&format!("{t} = call tailcc %KValue @d_{name}_{n}({})", args_ir.join(", ")));
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            f.record(&t, self.group_return_set(name, n) | fails);
            return Ok(t);
        }
        if name == "at" && emitted.len() == 2 {
            return Ok(self.emit_at(f, &emitted[0].clone(), &emitted[1].clone(), false));
        }
        if let Some((_, arity)) = BUILTIN_CALLS.iter().find(|(b, _)| b == name) {
            if emitted.len() != *arity {
                return Err(format!("native backend: `{name}` takes {arity} argument(s)"));
            }
            let args_ir: Vec<String> = emitted.iter().map(|e| format!("%KValue {e}")).collect();
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_b_{name}({})", args_ir.join(", ")));
            let arg_sets: Vec<Set> = emitted.iter().map(|e| f.set_of(e)).collect();
            f.record(&t, infer::builtin_set(name, &arg_sets));
            return Ok(t);
        }
        Err(format!("native backend: `{name}` is not yet supported"))
    }
}

impl<'a> Backend<'a> {
    fn emit_lifted(
        &mut self,
        lifted: &str,
        param: &str,
        captures: &[String],
        body: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let _ = outer;
        let mut f = FnEmit::new();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_env_get(ptr %env, i64 {i})"));
            f.bind(cap, &t);
        }
        f.bind(param, "%a0");
        self.emit_tail(&mut f, body)?;
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{lifted}(ptr %env, %KValue %a0) {{\n{}}}\n",
            f.out
        );
        let _ = writeln!(
            self.body,
            "define %KValue @w_{lifted}(ptr %env, %KValue %a0) {{\nentry:\n  %r = call \
             tailcc %KValue @{lifted}(ptr %env, %KValue %a0)\n  ret %KValue %r\n}}\n"
        );
        Ok(())
    }
}

fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) => {}
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    collect_idents(inner, out);
                }
            }
        }
        Expr::Ident(name, _) => out.push(name.clone()),
        Expr::List(items, _) => {
            for item in items {
                collect_idents(item, out);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                collect_idents(key, out);
                collect_idents(value, out);
            }
        }
        Expr::App { head, args, .. } => {
            collect_idents(head, out);
            for arg in args {
                collect_idents(arg, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_idents(base, out);
            collect_idents(index, out);
        }
        Expr::Seq(lhs, rhs, _) => {
            collect_idents(lhs, out);
            collect_idents(rhs, out);
        }
        Expr::Lambda { body, .. } => collect_idents(body, out),
        Expr::BinOp { lhs, rhs, .. } => {
            collect_idents(lhs, out);
            collect_idents(rhs, out);
        }
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
