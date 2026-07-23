use crate::ast::*;
use crate::diag::Span;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, FLOAT, INT, LIST, MAP, NONE, REC, STR, TOP};
use std::collections::HashMap;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
const K_ERR: i64 = 5;

const DECLARES: &str = r#"%KValue = type { i64, i64 }
%parsed = type { i64, i64 }
%KBytes = type { i64, ptr }

; Inline twins of the runtime's hot one-liners (tag tests and value
; constructors). LTO declines to inline these across the .ll/.o module
; boundary, leaving a real call on every `if` condition and constructor;
; internal linkage keeps them from colliding with the runtime's own
; definitions, and alwaysinline folds them into every call site.
define internal %KValue @k_force_fast(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %is = icmp eq i64 %tag, 14
  br i1 %is, label %slow, label %done
slow:
  %f = call %KValue @k_force(%KValue %v)
  ret %KValue %f
done:
  ret %KValue %v
}
define internal %KValue @k_int(i64 %n) alwaysinline {
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %n, 1
  ret %KValue %v
}
define internal %KValue @k_float(double %d) alwaysinline {
  %bits = bitcast double %d to i64
  %v = insertvalue %KValue { i64 1, i64 undef }, i64 %bits, 1
  ret %KValue %v
}
define internal %KValue @k_bool(i64 %b) alwaysinline {
  %c = icmp ne i64 %b, 0
  %tag = select i1 %c, i64 2, i64 3
  %v = insertvalue %KValue { i64 undef, i64 0 }, i64 %tag, 0
  ret %KValue %v
}
define internal %KValue @k_none() alwaysinline {
  ret %KValue { i64 4, i64 0 }
}
define internal i64 @k_not_failure(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %ne = icmp ne i64 %tag, 5
  %nn = icmp ne i64 %tag, 4
  %ok = and i1 %ne, %nn
  %r = zext i1 %ok to i64
  ret i64 %r
}
define internal i64 @k_truthy(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %t = icmp eq i64 %tag, 2
  br i1 %t, label %yes, label %chkf
yes:
  ret i64 1
chkf:
  %f = icmp eq i64 %tag, 3
  br i1 %f, label %no, label %bad
no:
  ret i64 0
bad:
  %r = call i64 @k_truthy_bad()
  ret i64 %r
}
define internal i64 @k_check_tag(%KValue %v, i64 %t) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %c = icmp eq i64 %tag, %t
  %r = zext i1 %c to i64
  ret i64 %r
}
define internal i64 @k_check_int(%KValue %v, i64 %n) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %pay = extractvalue %KValue %v, 1
  %ct = icmp eq i64 %tag, 0
  %cp = icmp eq i64 %pay, %n
  %c = and i1 %ct, %cp
  %r = zext i1 %c to i64
  ret i64 %r
}
define internal i64 @k_check_bool(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %t = icmp eq i64 %tag, 2
  %f = icmp eq i64 %tag, 3
  %c = or i1 %t, %f
  %r = zext i1 %c to i64
  ret i64 %r
}
declare i64 @k_truthy_bad()

declare %KValue @k_str_n(ptr, i64)
declare %KValue @k_err(%KValue, ptr)
declare %KValue @k_err_hop(%KValue, ptr)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_field(%KValue, i64)
declare %KValue @k_keyed_check(%KValue, i64)
declare %KValue @k_keyed_field(%KValue, ptr)
declare %KValue @k_b_field(%KValue, ptr)
declare %KValue @k_err_inner(%KValue)
declare i64 @k_check_rec(%KValue, i64, i64)
declare i64 @k_check_str(%KValue, ptr, i64)
declare %KValue @k_concat(%KValue, %KValue)
declare %KValue @k_concat_arr(i64, ptr)
declare %KValue @k_render(%KValue, i64)
declare %KValue @k_add(%KValue, %KValue)
declare %KValue @k_sub(%KValue, %KValue)
declare %KValue @k_mul(%KValue, %KValue)
declare %KValue @k_div(%KValue, %KValue, ptr)
declare %KValue @k_mod(%KValue, %KValue, ptr)
declare %KValue @k_cmp(%KValue, %KValue, i64)
declare %KValue @k_desc_print(%KValue)
declare %KValue @k_seq(%KValue, %KValue)
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
declare %KValue @k_index(%KValue, %KValue, ptr)
declare %KValue @k_b_bytes(%KValue)
declare %KValue @k_b_chars(%KValue)
declare %KValue @k_b_concat(%KValue, %KValue)
declare %KValue @k_b_utf8(%KValue, ptr)
declare %KValue @k_desc_args()
declare %KValue @k_desc_stdin()
declare %KValue @k_b_read_file(%KValue)
declare %KValue @k_b_write_file(%KValue, %KValue)
declare %KValue @k_maybe_bind(%KValue, %KValue)
declare %KValue @k_desc_join(%KValue, %KValue)
declare %KValue @k_desc_sleep(%KValue)
declare %KValue @k_desc_random(%KValue)
declare void @k_beat_push()
declare void @k_beat_iter()
declare void @k_carry_reset()
declare void @k_carry_stage(%KValue)
declare %KValue @k_carry_take(i64)
declare void @k_beat_iter_carry()
declare %KValue @k_beat_pop(%KValue)
declare %KValue @k_call1(%KValue, %KValue)
declare %KValue @k_call2(%KValue, %KValue, %KValue)
declare %KValue @k_call3(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_call4(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_char_code(%KValue)
declare %KValue @k_b_entries(%KValue)
declare %KValue @k_b_filter(%KValue, %KValue)
declare %KValue @k_b_from_code(%KValue, ptr)
declare %KValue @k_b_join(%KValue, %KValue)
declare %KValue @k_b_length(%KValue)
declare %KValue @k_b_map(%KValue, %KValue)
declare %KValue @k_b_push(%KValue, %KValue)
declare %KValue @k_b_push_mut(%KValue, %KValue)
declare %KValue @k_b_put(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice(%KValue, %KValue, %KValue)
declare %KValue @k_b_find2(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_find2_below(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_sort(%KValue)
declare %KValue @k_b_sum(%KValue)
declare %KValue @k_b_to_float(%KValue, ptr)
declare %KValue @k_b_sqrt(%KValue)
declare %KValue @k_b_round(%KValue)
declare %KValue @k_b_to_int(%KValue, ptr)
declare %KValue @k_b_render_value(%KValue)
declare %KValue @k_thunk_new(i64, i32, ...)
declare %KValue @k_force(%KValue)

"#;

const BUILTIN_CALLS: [(&str, usize); 26] = [
    ("at", 2),
    ("find2", 4),
    ("find2_below", 5),
    ("bytes", 1),
    ("read_file", 1),
    ("write_file", 2),
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
    ("render_value", 1),
    ("sqrt", 1),
    ("round", 1),
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
    // an enrollment clone is an alias: it constructs and matches as its
    // origin, one identity per type no matter the spelling
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
    let mut escape = crate::escape::analyze(program);
    // The by-value `%parsed` is two i64s, so it only fits a record shaped like
    // the scanner's `_parsed`: exactly two fields, a small int position packed
    // into the tag word and a non-failure value in the payload word. Any other
    // register-returnable record keeps the heap representation.
    let type_index: HashMap<&str, usize> = program
        .types
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();
    escape.field_count.retain(|ty, n| {
        *n == 2
            && type_index.get(ty.as_str()).is_some_and(|&i| {
                inference.type_fields.get(i).is_some_and(|fields| {
                    fields.len() == 2 && fields[0] == INT && fields[1] & FAIL == 0
                })
            })
    });
    let packable: std::collections::HashSet<String> = escape.field_count.keys().cloned().collect();
    escape.returns.retain(|_, ty| packable.contains(ty));
    escape.carries.retain(|_, ty| packable.contains(ty));
    let byte_disc = crate::dispatch::byte_dispatched(program, &inference);
    let in_place_pushes = crate::linear::in_place_pushes(program);
    // Beat loops rewind the arena between iterations. Groups returning the
    // by-value %parsed are excluded: k_beat_pop judges heap-ness from the
    // returned tag word, and the packed representation would mislead it.
    let mut beat = crate::beat::beat_loops(program, &inference);
    beat.ids.retain(|(n, a), _| escape.returns_ty(n, *a).is_none());
    beat.demoted.retain(|(_, callee)| beat.ids.contains_key(callee));
    let mut backend = Backend {
        program,
        inference,
        escape,
        byte_disc,
        in_place_pushes,
        beat,
        type_ids,
        strings: Vec::new(),
        interned: HashMap::new(),
        body: String::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
        demand: crate::demand::analyze(program),
        thunk_sites: Vec::new(),
    };
    backend.emit()
}

struct Backend<'a> {
    program: &'a Program,
    inference: infer::Inference,
    escape: crate::escape::EscapeInfo,
    byte_disc: std::collections::HashSet<(String, usize, usize)>,
    in_place_pushes: std::collections::HashSet<(String, usize, usize)>,
    beat: crate::beat::Beats,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    interned: HashMap<Vec<u8>, String>,
    body: String,
    lift_counter: usize,
    fn_value_wrappers: Vec<(String, usize)>,
    demand: crate::demand::DemandInfo,
    /// (site evaluator symbol, captured-arg count), indexed by site id.
    thunk_sites: Vec<(String, usize)>,
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
    sets: HashMap<String, Set>,
    /// Temps carrying the by-value %parsed type rather than a boxed KValue.
    parsed: std::collections::HashSet<String>,
    /// Err-origin prefix "{fn} at {file}" for the declaration being emitted.
    origin_prefix: String,
    /// Source file of the declaration being emitted, for keying push sites.
    file: String,
    /// LLVM return type of the function being emitted: `%parsed` or `%KValue`.
    ret_ty: String,
    /// Dispatcher group being emitted, for recognizing self-tail-calls.
    group: String,
    arity: usize,
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
            parsed: std::collections::HashSet::new(),
            origin_prefix: String::new(),
            file: String::new(),
            ret_ty: "%KValue".to_string(),
            group: String::new(),
            arity: 0,
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

    fn record_parsed(&mut self, operand: &str) {
        self.parsed.insert(operand.to_string());
    }

    fn is_parsed(&self, operand: &str) -> bool {
        self.parsed.contains(operand)
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

/// LLVM symbol for a dispatcher: quoted when the kanso name carries a
/// module qualifier's slash.
fn wsym(name: &str, arity: usize) -> String {
    // fn-value wrapper symbols share dsym's quoted-identifier rule
    match name.contains(['/', '!', '?', '+', '-', '*', '%']) {
        true => format!("\"w_{name}_{arity}\""),
        false => format!("w_{name}_{arity}"),
    }
}

fn dsym(name: &str, arity: usize) -> String {
    // qualified names and the naming sigils need LLVM's quoted-identifier form
    match name.contains(['/', '!', '?', '+', '-', '*', '%']) {
        true => format!("\"d_{name}_{arity}\""),
        false => format!("d_{name}_{arity}"),
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

    /// A parameter proven to be exactly `int` crosses the tailcc boundary as a
    /// raw i64 instead of a boxed KValue. The dispatcher re-boxes it at entry so
    /// the body is untouched; LLVM's SROA folds that rebox against the body's
    /// payload reads (same function), and folds each caller's box against the
    /// extract we emit here — so only a raw i64 travels the musttail edge LLVM
    /// cannot otherwise see through. Sound because inference forces every param
    /// of a function used as a first-class value to TOP, never a bare `int`.
    fn unboxed_param(&self, name: &str, arity: usize, param: usize) -> bool {
        self.group_param_set(name, arity, param) == INT
    }

    /// Render one call argument in the callee's ABI: raw i64 for an unboxed
    /// slot (extract the payload), boxed KValue otherwise.
    /// Any arity-matching arm inspecting this position (anything but a bare
    /// Var/Wildcard) means a thunk must force before dispatch can select.
    fn scrutinizes(&self, callee: &str, arity: usize, i: usize) -> bool {
        self.program.fns.iter().any(|d| {
            d.name == callee
                && d.params.len() == arity
                && !matches!(d.params.get(i), Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_)))
        })
    }

    fn call_arg(&self, f: &mut FnEmit, callee: &str, arity: usize, i: usize, e: &str) -> String {
        let forced;
        let e = match f.set_of(e) & crate::infer::THUNK != 0 && self.scrutinizes(callee, arity, i) {
            true => {
                forced = self.maybe_force(f, e.to_string());
                forced.as_str()
            }
            false => e,
        };
        if self.is_byte_disc(callee, arity, i) {
            // `e` is an `at`-on-bytes KValue (byte or none); hand it over as a
            // raw i64 — the byte value, or 256 for none. The box `at` built and
            // this unbox fold away in the caller, so a raw byte crosses the edge.
            let tag = f.tmp();
            f.line(&format!("{tag} = extractvalue %KValue {e}, 0"));
            let payload = f.tmp();
            f.line(&format!("{payload} = extractvalue %KValue {e}, 1"));
            let is_none = f.tmp();
            f.line(&format!("{is_none} = icmp eq i64 {tag}, {K_NONE}"));
            let raw = f.tmp();
            f.line(&format!("{raw} = select i1 {is_none}, i64 256, i64 {payload}"));
            return format!("i64 {raw}");
        }
        if self.escape.carries_ty(callee, arity, i).is_some() {
            if f.is_parsed(e) {
                return format!("%parsed {e}");
            }
            // a boxed record reached a by-value slot (a construction bound or
            // passed outside tail position): unpack it into the convention
            let f0 = f.tmp();
            f.line(&format!("{f0} = call %KValue @k_field(%KValue {e}, i64 0)"));
            let f1 = f.tmp();
            f.line(&format!("{f1} = call %KValue @k_field(%KValue {e}, i64 1)"));
            let posp = f.tmp();
            f.line(&format!("{posp} = extractvalue %KValue {f0}, 1"));
            let sh = f.tmp();
            f.line(&format!("{sh} = shl i64 {posp}, 8"));
            let vt = f.tmp();
            f.line(&format!("{vt} = extractvalue %KValue {f1}, 0"));
            let w0 = f.tmp();
            f.line(&format!("{w0} = or i64 {sh}, {vt}"));
            let w1 = f.tmp();
            f.line(&format!("{w1} = extractvalue %KValue {f1}, 1"));
            let a = f.tmp();
            f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
            let p = f.tmp();
            f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
            format!("%parsed {p}")
        } else if self.unboxed_param(callee, arity, i) {
            let p = f.tmp();
            f.line(&format!("{p} = extractvalue %KValue {e}, 1"));
            format!("i64 {p}")
        } else {
            format!("%KValue {e}")
        }
    }

    /// Emit the entry-block reboxes that reconstruct each unboxed `%xi` param as
    /// the KValue the body expects.
    fn rebox_params(&self, f: &mut FnEmit, name: &str, arity: usize) {
        for i in 0..arity {
            if self.is_byte_disc(name, arity, i) {
                // Reconstruct the KValue the boxed dispatch expects: 256 is none,
                // anything else is that byte. The reconstruction folds back into
                // a raw switch, so only the raw i64 actually crossed the edge.
                let is_none = f.tmp();
                f.line(&format!("{is_none} = icmp eq i64 %x{i}r, 256"));
                f.line(&format!(
                    "%x{i}b = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
                f.line(&format!(
                    "%x{i} = select i1 {is_none}, %KValue {{ i64 4, i64 0 }}, %KValue %x{i}b"
                ));
            } else if self.unboxed_param(name, arity, i) {
                f.line(&format!(
                    "%x{i} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
            }
        }
    }

    /// A switch discriminator inference proves is `at`-on-bytes, so it crosses
    /// as a raw i64 (byte value, or 256 for none) and is switched on directly.
    fn is_byte_disc(&self, name: &str, arity: usize, param: usize) -> bool {
        self.byte_disc.contains(&(name.to_string(), arity, param))
    }

    /// The dispatcher's parameter list: a raw i64 for a byte discriminator or a
    /// proven-int slot, a `%parsed` struct for a register-returnable record,
    /// else a boxed KValue.
    fn abi_params(&self, name: &str, arity: usize) -> Vec<String> {
        (0..arity)
            .map(|i| {
                if self.is_byte_disc(name, arity, i) {
                    format!("i64 %x{i}r")
                } else if self.escape.carries_ty(name, arity, i).is_some() {
                    format!("%parsed %x{i}")
                } else if self.unboxed_param(name, arity, i) {
                    format!("i64 %x{i}r")
                } else {
                    format!("%KValue %x{i}")
                }
            })
            .collect()
    }

    /// The LLVM return type of a function group: `%parsed` when it hands back a
    /// register-returnable record by value, else `%KValue`.
    fn ret_ty(&self, name: &str, arity: usize) -> &'static str {
        if self.escape.returns_ty(name, arity).is_some() {
            "%parsed"
        } else {
            "%KValue"
        }
    }

    /// A group we can hand out as a first-class value through a `w_` wrapper: a
    /// `%KValue` return and no by-value or byte-discriminated parameters, which
    /// the generic wrapper does not know how to convert.
    fn simple_fn_value(&self, name: &str, arity: usize) -> bool {
        self.ret_ty(name, arity) == "%KValue"
            && (0..arity).all(|i| {
                !self.is_byte_disc(name, arity, i)
                    && self.escape.carries_ty(name, arity, i).is_none()
            })
    }

    /// A bind is representable as a native thunk when its captures fit the
    /// cell (args[8]).
    fn thunkable(&self, f: &FnEmit, expr: &Expr) -> bool {
        let mut idents = Vec::new();
        collect_idents(expr, &mut idents);
        let mut captures: Vec<&String> = Vec::new();
        for id in &idents {
            if f.lookup(id).is_some() && !captures.contains(&id) {
                captures.push(id);
            }
        }
        captures.len() <= 8
    }

    /// Force a value that may be a thunk; no-op (no IR) when the set proves
    /// it can't be one, so strict code pays nothing. A program the demand
    /// analysis deferred nothing in can hold no thunk anywhere — every site
    /// vanishes, not just the set-proven ones (conservative TOP widenings
    /// carry the THUNK bit into code no thunk can reach).
    fn maybe_force(&self, f: &mut FnEmit, value: String) -> String {
        if self.demand.lazy_bind_count() == 0 {
            return value;
        }
        if f.set_of(&value) & crate::infer::THUNK == 0 {
            return value;
        }
        let post = f.set_of(&value) & !crate::infer::THUNK;
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_force_fast(%KValue {value})"));
        // A forced thunk can yield anything its expr could; the bind site
        // recorded TOP, so widen conservatively past the removed bit.
        f.record(&t, if post == 0 { crate::infer::TOP & !crate::infer::THUNK } else { post });
        t
    }

    fn emit_thunk_site(
        &mut self,
        sym: &str,
        captures: &[String],
        expr: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new();
        f.origin_prefix = outer.origin_prefix.clone();
        f.file = outer.file.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            f.bind(cap, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, expr)?;
        let sig: Vec<String> = (0..captures.len()).map(|i| format!("%KValue %a{i}")).collect();
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{sym}({}) {{\n{}}}\n",
            sig.join(", "),
            f.out
        );
        Ok(())
    }

    fn emit_thunk_dispatcher(&mut self) {
        let mut arms = String::new();
        let mut cases = String::new();
        for (site, (sym, argc)) in self.thunk_sites.iter().enumerate() {
            let _ = writeln!(cases, "    i64 {site}, label %s{site}");
            let mut loads = String::new();
            let mut args: Vec<String> = Vec::new();
            for i in 0..*argc {
                let _ = writeln!(loads, "  %s{site}a{i}p = getelementptr %KValue, ptr %args, i64 {i}");
                let _ = writeln!(loads, "  %s{site}a{i} = load %KValue, ptr %s{site}a{i}p");
                args.push(format!("%KValue %s{site}a{i}"));
            }
            let _ = writeln!(
                arms,
                "s{site}:\n{loads}  %s{site}r = call tailcc %KValue @{sym}({})\n  ret %KValue %s{site}r",
                args.join(", ")
            );
        }
        let _ = writeln!(
            self.body,
            "define %KValue @d_thunk_eval(i64 %site, ptr %args) {{\nentry:\n  switch i64 %site, label %bad [\n{cases}  ]\n{arms}bad:\n  unreachable\n}}\n"
        );
    }

    fn emit(&mut self) -> Result<String, String> {
        self.emit_type_names();
        self.emit_type_fields();
        // group by name across the whole program: the bare overload space
        // interleaves same-named decls from different modules
        let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            match groups.iter_mut().find(|(name, _)| *name == decl.name) {
                Some((_, decls)) => decls.push(decl),
                None => groups.push((&decl.name, vec![decl])),
            }
        }
        // proximity breaks specificity ties: local arms precede clones
        for (_, decls) in &mut groups {
            decls.sort_by_key(|d| d.synthetic);
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
        let wrappers = self.fn_value_wrappers.clone();
        for (name, arity) in &wrappers {
            let arity = *arity;
            let params: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let mut conv = String::new();
            let call_args: Vec<String> = (0..arity)
                .map(|i| {
                    if self.unboxed_param(name, arity, i) {
                        let _ = writeln!(conv, "  %p{i} = extractvalue %KValue %a{i}, 1");
                        format!("i64 %p{i}")
                    } else {
                        format!("%KValue %a{i}")
                    }
                })
                .collect();
            let sym = dsym(name, arity);
            let _ = writeln!(conv, "  %r = call tailcc %KValue @{sym}({})", call_args.join(", "));
            let _ = writeln!(
                self.body,
                "define %KValue @{}({}) {{\nentry:\n{conv}  ret %KValue %r\n}}\n",
                wsym(name, arity),
                params.join(", ")
            );
        }
        // Lazy v1: the thunk-site dispatcher the runtime's k_force calls.
        // Sites are emitted as cases as lazy binds are compiled; a program
        // with no lazy sites still defines the symbol so every binary links.
        self.emit_thunk_dispatcher();
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

    /// The interned origin literal for an err construction site.
    fn origin_arg(&mut self, f: &FnEmit, span: Span) -> String {
        let (name, _) = self.intern(&format!("{}:{}\0", f.origin_prefix, span.line));
        format!("ptr @{name}")
    }

    fn emit_type_names(&mut self) {
        let mut body = String::new();
        body.push_str("define ptr @k_type_name(i64 %id) {\nentry:\n");
        let mut arms = String::new();
        let mut cases = String::new();
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the case
                continue;
            }
            let id = self.type_ids[ty.name.as_str()];
            let (name, _len) = self.intern(&format!("{}\0", ty.name));
            let _ = writeln!(cases, "    i64 {id}, label %T{id}");
            let _ = writeln!(arms, "T{id}:\n  ret ptr @{name}");
        }
        let (entry_name, _) = self.intern("entry\0");
        let _ = writeln!(cases, "    i64 0, label %T0");
        let _ = writeln!(arms, "T0:\n  ret ptr @{entry_name}");
        let (fallback, _) = self.intern("record\0");
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{fallback}");
        body.push_str("}\n\n");
        self.body.push_str(&body);
    }

    /// Field metadata for keyed reads: name-indexed lookup resolves against
    /// these per-type switch tables at runtime.
    fn emit_type_fields(&mut self) {
        let mut tables: Vec<(i64, Vec<String>)> = vec![(0, vec!["key".into(), "value".into()])];
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the case
                continue;
            }
            let id = self.type_ids[ty.name.as_str()];
            let fields = ty.fields.iter().map(|(name, _, _)| name.clone()).collect();
            tables.push((id, fields));
        }
        let mut body = String::new();
        body.push_str("define i64 @k_type_field_count(i64 %id) {\nentry:\n");
        let mut cases = String::new();
        let mut arms = String::new();
        for (id, fields) in &tables {
            let _ = writeln!(cases, "    i64 {id}, label %C{id}");
            let _ = writeln!(arms, "C{id}:\n  ret i64 {}", fields.len());
        }
        let _ = writeln!(body, "  switch i64 %id, label %CD [\n{cases}  ]");
        body.push_str(&arms);
        body.push_str("CD:\n  ret i64 0\n}\n\n");
        body.push_str("define ptr @k_type_field_name(i64 %id, i64 %i) {\nentry:\n");
        let (empty, _) = self.intern("\0");
        let mut cases = String::new();
        let mut arms = String::new();
        for (id, fields) in &tables {
            let _ = writeln!(cases, "    i64 {id}, label %T{id}");
            let mut inner = String::new();
            for (i, field) in fields.iter().enumerate() {
                let (name, _) = self.intern(&format!("{field}\0"));
                let _ = writeln!(inner, "    i64 {i}, label %T{id}F{i}");
                let _ = writeln!(arms, "T{id}F{i}:\n  ret ptr @{name}");
            }
            let _ = writeln!(arms, "T{id}:\n  switch i64 %i, label %TD [\n{inner}  ]");
        }
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{empty}");
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
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new();
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let sym_hdr = dsym(name, arity);
        let header = format!("define tailcc {ret} @{sym_hdr}({}) {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
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
                let hopped = f.tmp();
                f.line(&format!(
                    "{hopped} = call %KValue @k_err_hop(%KValue %x{i}, ptr @{hop_name})"
                ));
                self.emit_ret_failure(&mut f, name, arity, &hopped);
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
        let hopped = f.tmp();
        f.line(&format!("{hopped} = call %KValue @k_err_hop(%KValue {dv}, ptr @{hop_name})"));
        self.emit_ret_failure(&mut f, name, arity, &hopped);
        f.start_block(&die);
        let msg = format!("no overload of `{name}` matches these arguments ");
        let (m, _len) = self.intern(&msg);
        f.line(&format!("call void @k_die(ptr @{m})"));
        f.line("unreachable");
        // arm bodies: patterns are known matched, only bind generics
        for (k, decl) in decls.iter().enumerate() {
            f.start_block(&arm_labels[k]);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", decl.name, decl.file);
            f.file = decl.file.clone();
            for (i, pattern) in decl.params.iter().enumerate() {
                if let Pattern::Var(pname, _) = pattern {
                    f.bind(pname, &format!("%x{i}"));
                }
            }
            self.emit_fn_body(&mut f, decl, &decl.body)?;
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
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new();
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let sym_hdr = dsym(name, arity);
        let header = format!("define tailcc {ret} @{sym_hdr}({}) {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        // A `%parsed` and a `%KValue` share a `{i64,i64}` layout: reinterpret the
        // parameter's two words as the discriminator KValue once, so the arms can
        // match failures and the propagation loop can hop. On the failure path it
        // *is* the failure; on success its low word (value.tag | pos<<8) is never
        // a failure tag, so `k_not_failure` still separates the two.
        for i in 0..arity {
            if self.escape.carries_ty(name, arity, i).is_some() {
                f.line(&format!("%x{i}w0 = extractvalue %parsed %x{i}, 0"));
                f.line(&format!("%x{i}w1 = extractvalue %parsed %x{i}, 1"));
                f.line(&format!("%x{i}sa = insertvalue %KValue undef, i64 %x{i}w0, 0"));
                f.line(&format!("%x{i}s = insertvalue %KValue %x{i}sa, i64 %x{i}w1, 1"));
            }
        }
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", decl.name, decl.file);
            f.file = decl.file.clone();
            for (i, pattern) in decl.params.iter().enumerate() {
                match self.escape.carries_ty(name, arity, i) {
                    Some(ty) => {
                        let ty = ty.to_string();
                        self.emit_parsed_pattern(&mut f, &format!("%x{i}s"), pattern, &fail, &ty)?;
                    }
                    None => {
                        let known = self.group_param_set(name, arity, i);
                        self.emit_pattern_known(&mut f, &format!("%x{i}"), pattern, &fail, known)?;
                    }
                }
            }
            self.emit_fn_body(&mut f, decl, &decl.body)?;
            f.start_block(&fail);
        }
        for i in 0..arity {
            let val = match self.escape.carries_ty(name, arity, i).is_some() {
                true => format!("%x{i}s"),
                false => format!("%x{i}"),
            };
            let ok = inline_not_failure(&mut f, &val);
            let ret_label = f.label();
            let next = f.label();
            f.line(&format!("br i1 {ok}, label %{next}, label %{ret_label}"));
            f.start_block(&ret_label);
            let hopped = f.tmp();
            f.line(&format!(
                "{hopped} = call %KValue @k_err_hop(%KValue {val}, ptr @{hop_name})"
            ));
            self.emit_ret_failure(&mut f, name, arity, &hopped);
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

    /// Match a pattern against a `%parsed` parameter. The `(ty ...)` arm succeeds
    /// when the discriminator is not a failure and binds the fields straight from
    /// the struct (no heap read); every other pattern (`none`, `(err ...)`,
    /// wildcard) matches the discriminator KValue exactly as the old boxed value
    /// would have.
    fn emit_parsed_pattern(
        &mut self,
        f: &mut FnEmit,
        status: &str,
        pattern: &Pattern,
        fail: &str,
        ty: &str,
    ) -> Result<(), String> {
        if let Pattern::Ctor { ty: pty, fields } = pattern {
            if pty == ty {
                let ok = inline_not_failure(f, status);
                let cont = f.label();
                f.line(&format!("br i1 {ok}, label %{cont}, label %{fail}"));
                f.start_block(&cont);
                let w0 = f.tmp();
                f.line(&format!("{w0} = extractvalue %KValue {status}, 0"));
                let w1 = f.tmp();
                f.line(&format!("{w1} = extractvalue %KValue {status}, 1"));
                // field 0: the position, unshifted out of the tag word.
                let posp = f.tmp();
                f.line(&format!("{posp} = lshr i64 {w0}, 8"));
                let posa = f.tmp();
                f.line(&format!("{posa} = insertvalue %KValue undef, i64 0, 0"));
                let poskv = f.tmp();
                f.line(&format!("{poskv} = insertvalue %KValue {posa}, i64 {posp}, 1"));
                self.emit_pattern(f, &poskv, &fields[0], fail)?;
                // field 1: the value, its tag masked back out of the low byte.
                let vtag = f.tmp();
                f.line(&format!("{vtag} = and i64 {w0}, 255"));
                let va = f.tmp();
                f.line(&format!("{va} = insertvalue %KValue undef, i64 {vtag}, 0"));
                let vkv = f.tmp();
                f.line(&format!("{vkv} = insertvalue %KValue {va}, i64 {w1}, 1"));
                self.emit_pattern(f, &vkv, &fields[1], fail)?;
                return Ok(());
            }
        }
        self.emit_pattern_known(f, status, pattern, fail, TOP)
    }

    /// Return a failure in the group's ABI shape: wrapped in a `%parsed` when the
    /// group returns records by value, a bare KValue otherwise.
    fn emit_ret_failure(&self, f: &mut FnEmit, name: &str, arity: usize, failure: &str) {
        if self.ret_ty(name, arity) == "%parsed" {
            self.emit_parsed_from_failure(f, failure);
        } else {
            f.line(&format!("ret %KValue {failure}"));
        }
    }

    /// Return a KValue in the current function's ABI shape. A `%parsed`-returning
    /// function only reaches here with a failure (its record tails are built
    /// directly), so the failure's two words become the `%parsed`.
    fn emit_ret(&self, f: &mut FnEmit, value: &str) {
        if f.ret_ty == "%parsed" {
            self.emit_parsed_from_failure(f, value);
        } else {
            f.line(&format!("ret %KValue {value}"));
        }
    }

    /// A `%parsed` and a `%KValue` share a `{i64,i64}` layout. On the failure
    /// path the two are the same value: the failure's tag/payload become the
    /// `%parsed` words, so the discriminator (low word ∈ {4,5}) stays intact.
    fn emit_parsed_from_failure(&self, f: &mut FnEmit, failure: &str) {
        let w0 = f.tmp();
        f.line(&format!("{w0} = extractvalue %KValue {failure}, 0"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {failure}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.line(&format!("ret %parsed {p}"));
    }

    /// A direct construction of the register-returnable type a callee slot
    /// carries may cross the boundary packed; anything else in such a slot is
    /// already %parsed (a returnable call's result) by the analysis.
    fn packed_arg_fields<'e>(
        &self,
        callee: &str,
        arity: usize,
        i: usize,
        arg: &'e Expr,
    ) -> Option<&'e [Expr]> {
        let ty = self.escape.carries_ty(callee, arity, i)?;
        if let Expr::App { head, args, piped: false, .. } = arg {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == ty)
                && Some(&args.len()) == self.escape.field_count.get(ty).as_ref().map(|v| *v)
            {
                return Some(args);
            }
        }
        None
    }

    /// Pack a register-returnable construction into its by-value form for an
    /// argument position: same two words the tail form uses, yielded as a
    /// temp instead of returned.
    fn emit_packed_arg(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<String, String> {
        let pos = self.emit_expr(f, &args[0])?;
        self.bail_on_failure(f, &pos);
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_failure(f, &value);
        let pos_payload = f.tmp();
        f.line(&format!("{pos_payload} = extractvalue %KValue {pos}, 1"));
        let shifted = f.tmp();
        f.line(&format!("{shifted} = shl i64 {pos_payload}, 8"));
        let vtag = f.tmp();
        f.line(&format!("{vtag} = extractvalue %KValue {value}, 0"));
        let w0 = f.tmp();
        f.line(&format!("{w0} = or i64 {shifted}, {vtag}"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {value}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.record_parsed(&p);
        Ok(p)
    }

    /// Build a register-returnable record in tail position as a by-value
    /// `%parsed`. The two words hold `(value.tag | pos << 8, value.payload)` — a
    /// non-failure value's tag never collides with the failure tags 4/5, so the
    /// low byte of word 0 still tells success from failure. A failing field
    /// propagates exactly as `k_rec` would have.
    fn emit_parsed_construction(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<(), String> {
        let pos = self.emit_expr(f, &args[0])?;
        self.bail_on_failure(f, &pos);
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_failure(f, &value);
        let pos_payload = f.tmp();
        f.line(&format!("{pos_payload} = extractvalue %KValue {pos}, 1"));
        let shifted = f.tmp();
        f.line(&format!("{shifted} = shl i64 {pos_payload}, 8"));
        let vtag = f.tmp();
        f.line(&format!("{vtag} = extractvalue %KValue {value}, 0"));
        let w0 = f.tmp();
        f.line(&format!("{w0} = or i64 {shifted}, {vtag}"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {value}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.line(&format!("ret %parsed {p}"));
        Ok(())
    }

    /// If `value` is a failure, return it in the current ABI shape; otherwise
    /// fall through with `value` known good.
    fn bail_on_failure(&self, f: &mut FnEmit, value: &str) {
        let ok = inline_not_failure(f, value);
        let cont = f.label();
        let bail = f.label();
        f.line(&format!("br i1 {ok}, label %{cont}, label %{bail}"));
        f.start_block(&bail);
        self.emit_ret(f, value);
        f.start_block(&cont);
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

    /// Constructor enforcement for multi-member field typesets: a field value
    /// matching no member is a defect (failures skip the check and propagate
    /// through `k_rec`).
    fn emit_typeset_checks(
        &mut self,
        f: &mut FnEmit,
        name: &str,
        emitted: &[String],
    ) -> Result<(), String> {
        let Some(decl) = self.program.types.iter().find(|t| t.name == name) else {
            return Ok(());
        };
        let fields = decl.fields.clone();
        for ((field, tys, _), value) in fields.iter().zip(emitted) {
            if tys.len() < 2 {
                continue;
            }
            let mut matched: Option<String> = None;
            for member in tys {
                let call = self.member_check_call(value, member)?;
                let c = f.tmp();
                f.line(&format!("{c} = {call}"));
                let b = f.tmp();
                f.line(&format!("{b} = icmp ne i64 {c}, 0"));
                matched = Some(match matched {
                    None => b,
                    Some(prev) => {
                        let t = f.tmp();
                        f.line(&format!("{t} = or i1 {prev}, {b}"));
                        t
                    }
                });
            }
            let matched = matched.expect("a typeset has members");
            let not_fail = inline_not_failure(f, value);
            let not_matched = f.tmp();
            f.line(&format!("{not_matched} = xor i1 {matched}, true"));
            let bad = f.tmp();
            f.line(&format!("{bad} = and i1 {not_matched}, {not_fail}"));
            let die = f.label();
            let ok = f.label();
            f.line(&format!("br i1 {bad}, label %{die}, label %{ok}"));
            f.start_block(&die);
            let msg = format!("field `{field}` of `{name}` takes {}\0", tys.join(" "));
            let (m, _) = self.intern(&msg);
            f.line(&format!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
            f.start_block(&ok);
        }
        Ok(())
    }

    fn member_check_call(&self, value: &str, member: &str) -> Result<String, String> {
        Ok(match member {
            "int" => format!("call i64 @k_check_tag(%KValue {value}, i64 0)"),
            "float64" => format!("call i64 @k_check_tag(%KValue {value}, i64 1)"),
            "string" => format!("call i64 @k_check_tag(%KValue {value}, i64 6)"),
            "bool" => format!("call i64 @k_check_bool(%KValue {value})"),
            other => {
                let id = self
                    .type_ids
                    .get(other)
                    .ok_or_else(|| format!("native backend: unknown type `{other}`"))?;
                let nfields = self.field_count(other)?;
                format!("call i64 @k_check_rec(%KValue {value}, i64 {id}, i64 {nfields})")
            }
        })
    }

    fn field_count(&self, ty: &str) -> Result<usize, String> {
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.len())
            .ok_or_else(|| format!("native backend: unknown type `{ty}`"))
    }

    fn emit_fn_body(&mut self, f: &mut FnEmit, decl: &FnDecl, body: &[Stmt]) -> Result<(), String> {
        let last = body.len() - 1;
        for (i, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), i)
                        && self.thunkable(f, expr) =>
                {
                    let mut idents = Vec::new();
                    collect_idents(expr, &mut idents);
                    let mut captures: Vec<String> = Vec::new();
                    for id in idents {
                        if f.lookup(&id).is_some() && !captures.contains(&id) {
                            captures.push(id);
                        }
                    }
                    let site = self.thunk_sites.len();
                    let sym = format!("tsite{site}");
                    self.thunk_sites.push((sym.clone(), captures.len()));
                    self.emit_thunk_site(&sym, &captures, expr, f)?;
                    let mut args = String::new();
                    for cap in &captures {
                        let temp = f.lookup(cap).expect("capture is bound");
                        args.push_str(&format!(", %KValue {temp}"));
                    }
                    let t = f.tmp();
                    f.line(&format!(
                        "{t} = call %KValue (i64, i32, ...) @k_thunk_new(i64 {site}, i32 {}{args})",
                        captures.len()
                    ));
                    f.record(&t, crate::infer::TOP);
                    f.bind(name, &t);
                }
                Stmt::Bind { pattern, expr } => {
                    self.emit_bind(f, pattern, expr)?;
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

    /// One strict binding: evaluate, then bind the pattern's names.
    fn emit_bind(&mut self, f: &mut FnEmit, pattern: &Pattern, expr: &Expr) -> Result<(), String> {
        {
            {
                {
                    let value = self.emit_expr(f, expr)?;
                    let value = match pattern {
                        Pattern::Var(..) => value,
                        _ => self.maybe_force(f, value),
                    };
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
                        Pattern::Keyed { entries, .. } => {
                            let checked = f.tmp();
                            f.line(&format!(
                                "{checked} = call %KValue @k_keyed_check(%KValue {value}, i64 {})",
                                entries.len()
                            ));
                            for entry in entries {
                                let (name, _) = self.intern(&format!("{}\0", entry.field));
                                let fv = f.tmp();
                                f.line(&format!(
                                    "{fv} = call %KValue @k_keyed_field(%KValue {checked}, ptr @{name})"
                                ));
                                f.bind(&entry.bind_name, &fv);
                            }
                        }
                        _ => {
                            return Err(
                                "native backend: this binding pattern is not supported".to_string()
                            )
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Block(stmts, _) => {
                let mut value = "{ i64 4, i64 0 }".to_string();
                let last = stmts.len().saturating_sub(1);
                for (i, stmt) in stmts.iter().enumerate() {
                    match stmt {
                        Stmt::Bind { pattern, expr } => self.emit_bind(f, pattern, expr)?,
                        Stmt::Expr(e) => {
                            let v = self.emit_expr(f, e)?;
                            if i == last {
                                value = v;
                            }
                        }
                    }
                }
                Ok(value)
            }
            Expr::Int(n, _) => Ok(format!("{{ i64 0, i64 {n} }}")),
            Expr::Float(x, _) => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_float(double 0x{:016X})", x.to_bits()));
                Ok(t)
            }
            Expr::Str(parts, _) => {
                let mut acc: Option<Vec<String>> = None;
                let mut fails: Set = 0;
                for part in parts {
                    let piece = match part {
                        TemplatePart::Lit(s) => self.str_const(f, s),
                        TemplatePart::Interp(inner) => {
                            let value = self.emit_expr(f, inner)?;
                            let value = self.maybe_force(f, value);
                            // only an err propagates out of interpolation; a none
                            // renders `<none>` via k_render, so it is not a fail
                            fails |= f.set_of(&value) & ERR;
                            // a set carrying REC may hit a user to_string arm:
                            // route through the ambient group. Primitive-only
                            // sets keep the direct call — coherence proves no
                            // arm can exist for them (design/render-plan.md).
                            let group = "render/to_string";
                            let dispatchable = f.set_of(&value) & (REC | NONE | DESC) != 0
                                && self.program.fns.iter().any(|d| d.name == group);
                            let t = f.tmp();
                            match dispatchable {
                                true => {
                                    f.line(&format!(
                                        "{t} = call tailcc %KValue @{}(%KValue {value})",
                                        dsym(group, 1)
                                    ));
                                    fails |= ERR;
                                }
                                false => f.line(&format!(
                                    "{t} = call %KValue @k_render(%KValue {value}, i64 0)"
                                )),
                            }
                            t
                        }
                    };
                    match acc {
                        None => acc = Some(vec![piece]),
                        Some(ref mut pieces) => pieces.push(piece),
                    }
                }
                let out = match acc {
                    Some(pieces) if pieces.len() == 1 => {
                        pieces.into_iter().next().expect("one piece")
                    }
                    Some(pieces) if pieces.len() <= 16 => {
                        let arr = f.tmp();
                        f.line(&format!("{arr} = alloca [{} x %KValue]", pieces.len()));
                        for (i, p) in pieces.iter().enumerate() {
                            let slot = f.tmp();
                            f.line(&format!(
                                "{slot} = getelementptr [{} x %KValue], ptr {arr}, i64 0, i64 {i}",
                                pieces.len()
                            ));
                            f.line(&format!("store %KValue {p}, ptr {slot}"));
                        }
                        let t = f.tmp();
                        f.line(&format!(
                            "{t} = call %KValue @k_concat_arr(i64 {}, ptr {arr})",
                            pieces.len()
                        ));
                        t
                    }
                    Some(pieces) => {
                        let mut it = pieces.into_iter();
                        let mut prev = it.next().expect("non-empty");
                        for piece in it {
                            let t = f.tmp();
                            f.line(&format!(
                                "{t} = call %KValue @k_concat(%KValue {prev}, %KValue {piece})"
                            ));
                            prev = t;
                        }
                        prev
                    }
                    None => self.str_const(f, ""),
                };
                f.record(&out, STR | fails);
                Ok(out)
            }
            Expr::Ident(name, _) => {
                if let Some(temp) = f.lookup(name) {
                    return Ok(temp);
                }
                if self.program.types.iter().any(|t| t.name == *name && t.fields.is_empty()) {
                    let id = self.type_ids[name.as_str()];
                    let arr = f.tmp();
                    f.line(&format!("{arr} = alloca [1 x %KValue]"));
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_rec(i64 {id}, i64 0, ptr {arr})"));
                    f.record(&t, REC);
                    return Ok(t);
                }
                if self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty()) {
                    let callee_ret = self.ret_ty(name, 0);
                    let t = f.tmp();
                    f.line(&format!("{t} = call tailcc {callee_ret} @{}()", dsym(name, 0)));
                    if callee_ret == "%parsed" {
                        f.record_parsed(&t);
                    }
                    f.record(&t, self.group_return_set(name, 0));
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
                if arities.len() == 1
                    && (1..=4).contains(&arities[0])
                    && self.simple_fn_value(name, arities[0])
                {
                    let arity = arities[0];
                    self.fn_value_wrappers.push((name.clone(), arity));
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_fnref(ptr @{})", wsym(name, arity)));
                    return Ok(t);
                }
                if !arities.is_empty() {
                    return Err(format!(
                        "native backend: `{name}` cannot be used as a function value \
                         (only 1-4 argument functions over plain values are supported)"
                    ));
                }
                match name.strip_prefix("builtin_").unwrap_or(name.as_str()) {
                    "true" => Ok("{ i64 2, i64 0 }".to_string()),
                    "false" => Ok("{ i64 3, i64 0 }".to_string()),
                    "none" => Ok("{ i64 4, i64 0 }".to_string()),
                    "args" => {
                        let t = f.tmp();
                        f.line(&format!("{t} = call %KValue @k_desc_args()"));
                        f.record(&t, DESC);
                        Ok(t)
                    }
                    "stdin" => {
                        let t = f.tmp();
                        f.line(&format!("{t} = call %KValue @k_desc_stdin()"));
                        f.record(&t, DESC);
                        Ok(t)
                    }
                    _ => Err(format!(
                        "native backend: `{name}` as a bare value is not yet supported"
                    )),
                }
            }
            Expr::App { head, args, piped, span } => {
                self.emit_call_full(f, head, args, *piped, *span)
            }
            Expr::Field { base, name, .. } => {
                let b = self.emit_expr(f, base)?;
                let (label, _) = self.intern(&format!("{name}\0"));
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_b_field(%KValue {b}, ptr @{label})"
                ));
                f.record(&t, TOP);
                Ok(t)
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.emit_expr(f, base)?;
                let container = self.maybe_force(f, container);
                let key = self.emit_expr(f, index)?;
                let key = self.maybe_force(f, key);
                Ok(self.emit_at(f, &container, &key, *strict, *span))
            }
            Expr::Seq(lhs, rhs, _) => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_seq(%KValue {a}, %KValue {b})"));
                f.record(&t, DESC | (f.set_of(&a) & FAIL) | (f.set_of(&b) & FAIL));
                Ok(t)
            }
            Expr::Join { lhs, rhs, .. } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_desc_join(%KValue {a}, %KValue {b})"
                ));
                f.record(&t, (f.set_of(&a) & FAIL) | (f.set_of(&b) & FAIL) | DESC | ERR);
                Ok(t)
            }
            Expr::BinOp { op, lhs, rhs, span } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                self.emit_binop(f, op, &a, &b, *span)
            }
            Expr::Lambda { params, body, .. } => {
                if params.is_empty() || params.len() > 4 {
                    return Err("native backend: a lambda takes 1 to 4 parameters".to_string());
                }
                let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
                let mut idents = Vec::new();
                collect_idents(body, &mut idents);
                let mut captures: Vec<String> = Vec::new();
                for name in idents {
                    if f.lookup(&name).is_some()
                        && !captures.contains(&name)
                        && !param_names.contains(&name)
                    {
                        captures.push(name);
                    }
                }
                let lifted = format!("klam{}", self.lift_counter);
                self.lift_counter += 1;
                self.emit_lifted(&lifted, &param_names, &captures, body, f)?;
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
                // the ccc wrapper, never the tailcc fn: C calls this pointer
                f.line(&format!(
                    "{t} = call %KValue @k_closure(ptr @w_{lifted}, i64 {}, ptr {arr})",
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
        if let Expr::App { head, args, piped, .. } = expr {
            if *piped && !args.is_empty() {
                // a tail pipe into a literal lambda is the bind, inlined:
                // guard the failure exactly as k_maybe_bind would, bind the
                // parameter, and the lambda body becomes this function's own
                // tail — a self-call there is a real musttail, so beats and
                // the carry apply through the ordinary machinery
                if let Expr::Lambda { params, body, .. } = head.as_ref() {
                    if params.len() == 1 && args.len() == 1 {
                        let value = self.emit_expr(f, &args[0])?;
                        // a description takes the executor's bind at runtime;
                        // anything else binds the parameter here and the
                        // lambda body becomes this function's own tail — the
                        // branch keeps both semantics exact with no reliance
                        // on inference
                        let tag = inline_tag(f, &value);
                        let is_desc = f.tmp();
                        f.line(&format!("{is_desc} = icmp eq i64 {tag}, 8"));
                        let desc_path = f.label();
                        let check = f.label();
                        f.line(&format!(
                            "br i1 {is_desc}, label %{desc_path}, label %{check}"
                        ));
                        f.start_block(&desc_path);
                        let t = f.tmp();
                        let closure = self.emit_expr(f, head)?;
                        f.line(&format!(
                            "{t} = call %KValue @k_maybe_bind(%KValue {value}, %KValue {closure})"
                        ));
                        f.record(&t, TOP);
                        self.emit_ret(f, &t);
                        f.start_block(&check);
                        let ok = inline_not_failure(f, &value);
                        let bail = f.label();
                        let cont = f.label();
                        f.line(&format!("br i1 {ok}, label %{cont}, label %{bail}"));
                        f.start_block(&bail);
                        self.emit_ret(f, &value);
                        f.start_block(&cont);
                        f.bind(&params[0].0, &value);
                        return self.emit_tail(f, body);
                    }
                }
                let value = self.emit_expr(f, expr)?;
                self.emit_ret(f, &value);
                return Ok(());
            }
            if let Expr::Ident(name, _) = &**head {
                if name == "if" && f.lookup(name).is_none() {
                    let cond = self.emit_expr(f, &args[0])?;
                    let ok = inline_not_failure(f, &cond);
                    let check = f.label();
                    let bail = f.label();
                    f.line(&format!("br i1 {ok}, label %{check}, label %{bail}"));
                    f.start_block(&bail);
                    self.emit_ret(f, &cond);
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
                // A register-returnable record built in tail position becomes the
                // by-value %parsed result directly — no heap allocation.
                if let Some(&nfields) = self.escape.field_count.get(name.as_str()) {
                    if f.ret_ty == "%parsed" && args.len() == nfields {
                        return self.emit_parsed_construction(f, args);
                    }
                }
                // A demoted tail entry: emitted as a plain call so the
                // beat loop it enters gets its push/pop bracket. The caller
                // is acyclic, so the one retained frame is bounded. Lifted
                // lambdas never appear in the analysis's caller set, so ANY
                // tail entry into a beat-headed loop from outside its
                // cluster demotes — an unbracketed entry would let the
                // loop's rewinds unwind to an enclosing mark and free the
                // caller's own live data.
                let target = (name.clone(), args.len());
                let outside_cluster = self.beat.ids.contains_key(&target)
                    && !self.beat.same_cluster(&target, &(f.group.clone(), f.arity));
                if outside_cluster
                    || self
                        .beat
                        .demoted
                        .contains(&((f.group.clone(), f.arity), target))
                {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
                let is_program_fn = f.lookup(name).is_none()
                    && !self.type_ids.contains_key(name.as_str())
                    && name != "err"
                    && name != "print"
                    && self.program.fns.iter().any(|d| d.name == *name);
                if is_program_fn {
                    let n = args.len();
                    let mut emitted = Vec::new();
                    let mut packed: Vec<Option<String>> = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        match self.packed_arg_fields(name, n, i, arg) {
                            Some(fields) => {
                                let fields = fields.to_vec();
                                let p = self.emit_packed_arg(f, &fields)?;
                                emitted.push(String::new());
                                packed.push(Some(p));
                            }
                            None => {
                                emitted.push(self.emit_expr(f, arg)?);
                                packed.push(None);
                            }
                        }
                    }
                    let callee_ret = self.ret_ty(name, n);
                    let same_ret = callee_ret == f.ret_ty;
                    if same_ret
                        && self
                            .beat
                            .same_cluster(&(name.clone(), n), &(f.group.clone(), f.arity))
                    {
                        match self.beat.carried.get(&(name.clone(), n)) {
                            Some(positions) => {
                                // evacuate the loop-varying arguments through
                                // the carry buffers, then rewind — before the
                                // ABI conversion below, so the call passes
                                // the evacuated values
                                f.line("call void @k_carry_reset()");
                                for &j in positions {
                                    let a = &emitted[j];
                                    f.line(&format!(
                                        "call void @k_carry_stage(%KValue {a})"
                                    ));
                                }
                                f.line("call void @k_beat_iter_carry()");
                                for (slot, &j) in positions.iter().enumerate() {
                                    let t = f.tmp();
                                    f.line(&format!(
                                        "{t} = call %KValue @k_carry_take(i64 {slot})"
                                    ));
                                    emitted[j] = t;
                                }
                            }
                            None => {
                                // everything this iteration allocated is
                                // dead; rewind to the entry mark
                                f.line("call void @k_beat_iter()");
                            }
                        }
                    }
                    let args_ir: Vec<String> = emitted
                        .iter()
                        .enumerate()
                        .map(|(i, e)| match &packed[i] {
                            Some(p) => format!("%parsed {p}"),
                            None => self.call_arg(f, name, n, i, e),
                        })
                        .collect();
                    let t = f.tmp();
                    if same_ret {
                        f.line(&format!(
                            "{t} = musttail call tailcc {callee_ret} @{}({})",
                            dsym(name, n),
                            args_ir.join(", ")
                        ));
                        f.line(&format!("ret {callee_ret} {t}"));
                    } else {
                        // A %parsed function tail-calling a KValue failure helper:
                        // can't musttail across the type change, so call and wrap.
                        f.line(&format!(
                            "{t} = call tailcc {callee_ret} @{}({})",
                dsym(name, n),
                            args_ir.join(", ")
                        ));
                        self.emit_ret(f, &t);
                    }
                    return Ok(());
                }
            }
        }
        let value = self.emit_expr(f, expr)?;
        self.emit_ret(f, &value);
        Ok(())
    }

    fn emit_binop(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        // a record on the left dispatches to the operator's user arms; the
        // numeric fast paths below stay untouched for everything else
        let armable = matches!(op, "+" | "-" | "*" | "/" | "%")
            && self.program.fns.iter().any(|d| d.name == op && d.params.len() == 2);
        if armable && f.set_of(a) & REC != 0 {
            let tag = f.tmp();
            f.line(&format!("{tag} = extractvalue %KValue {a}, 0"));
            let isrec = f.tmp();
            f.line(&format!("{isrec} = icmp eq i64 {tag}, 7"));
            let user = f.label();
            let builtin = f.label();
            let merge = f.label();
            f.line(&format!("br i1 {isrec}, label %{user}, label %{builtin}"));
            f.start_block(&user);
            let uv = f.tmp();
            f.line(&format!(
                "{uv} = call tailcc %KValue @{}(%KValue {a}, %KValue {b})",
                dsym(op, 2)
            ));
            f.line(&format!("br label %{merge}"));
            let user_from = user.clone();
            f.start_block(&builtin);
            let bv = self.emit_binop_builtin(f, op, a, b, span)?;
            let builtin_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {uv}, %{user_from} ], [ {bv}, %{builtin_from} ]"
            ));
            f.record(
                &t,
                f.set_of(&bv) | self.group_return_set(op, 2) | (f.set_of(a) & FAIL),
            );
            return Ok(t);
        }
        self.emit_binop_builtin(f, op, a, b, span)
    }

    fn emit_binop_builtin(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        let slow_call = match op {
            "+" => format!("call %KValue @k_add(%KValue {a}, %KValue {b})"),
            "-" => format!("call %KValue @k_sub(%KValue {a}, %KValue {b})"),
            "*" => format!("call %KValue @k_mul(%KValue {a}, %KValue {b})"),
            "/" => {
                let origin = self.origin_arg(f, span);
                format!("call %KValue @k_div(%KValue {a}, %KValue {b}, {origin})")
            }
            "%" => {
                let origin = self.origin_arg(f, span);
                format!("call %KValue @k_mod(%KValue {a}, %KValue {b}, {origin})")
            }
            "==" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 0)"),
            "!=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 1)"),
            "<" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 2)"),
            "<=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 3)"),
            ">" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 4)"),
            _ => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 5)"),
        };
        if op == "/" || op == "%" {
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
    fn emit_at(
        &mut self,
        f: &mut FnEmit,
        container: &str,
        key: &str,
        strict: bool,
        span: Span,
    ) -> String {
        let slow_fn = if strict { "k_index" } else { "k_b_at" };
        let slow_extra = match strict {
            true => format!(", {}", self.origin_arg(f, span)),
            false => String::new(),
        };
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
                    "{mv} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
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
            "{slow_value} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
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

    fn emit_call_full(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        piped: bool,
        span: Span,
    ) -> Result<String, String> {
        if piped && !args.is_empty() {
            let piped_value = self.emit_expr(f, &args[0])?;
            if f.set_of(&piped_value) & DESC != 0 {
                let mut body_args: Vec<Expr> =
                    vec![Expr::Ident("__piped".to_string(), span)];
                body_args.extend(args[1..].iter().cloned());
                let lambda = Expr::Lambda {
                    params: vec![("__piped".to_string(), span)],
                    body: Box::new(Expr::App {
                        head: Box::new(head.clone()),
                        args: body_args,
                        span,
                        piped: false,
                    }),
                    span,
                };
                let closure = self.emit_expr(f, &lambda)?;
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_maybe_bind(%KValue {piped_value}, %KValue {closure})"
                ));
                f.record(&t, TOP);
                return Ok(t);
            }
            // a pipe hands its value on; a failure short-circuits before the
            // call (no dispatch, no hop) on every engine
            if f.set_of(&piped_value) & FAIL != 0 {
                let ok = inline_not_failure(f, &piped_value);
                let docall = f.label();
                let merge = f.label();
                let fail_from = f.cur_label.clone();
                f.line(&format!("br i1 {ok}, label %{docall}, label %{merge}"));
                f.start_block(&docall);
                let called =
                    self.emit_call_rest(f, head, args, Some(piped_value.clone()), span)?;
                let call_from = f.cur_label.clone();
                f.line(&format!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line(&format!(
                    "{t} = phi %KValue [ {piped_value}, %{fail_from} ], [ {called}, %{call_from} ]"
                ));
                f.record(&t, f.set_of(&called) | (f.set_of(&piped_value) & FAIL));
                return Ok(t);
            }
            // no description or failure can flow here: an ordinary call
            return self.emit_call_rest(f, head, args, Some(piped_value), span);
        }
        self.emit_call_rest(f, head, args, None, span)
    }

    fn emit_call_rest(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        first: Option<String>,
        span: Span,
    ) -> Result<String, String> {
        let call_arity = args.len() + first.is_some() as usize;
        let computed_head = match head {
            // A local binding is a value. So is a top-level constant (a nullary
            // group) invoked with arguments and no arm at that arity: it holds a
            // function value, and `f x` calls that value, not a group named `f`.
            Expr::Ident(name, _) => {
                f.lookup(name).is_some()
                    || (call_arity >= 1
                        && self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty())
                        && !self
                            .program
                            .fns
                            .iter()
                            .any(|d| d.name == *name && d.params.len() == call_arity))
            }
            _ => true,
        };
        if computed_head {
            // The callee is a value (a lambda, a parameter, a bound function),
            // not a declared group: emit the head and all arguments as values
            // and dispatch at runtime via the arity-matched k_callN.
            let callee = self.emit_expr(f, head)?;
            let mut arg_vals: Vec<String> = Vec::new();
            if let Some(v) = first {
                arg_vals.push(v);
            }
            for a in args {
                arg_vals.push(self.emit_expr(f, a)?);
            }
            let n = arg_vals.len();
            if n == 0 || n > 4 {
                return Err(format!(
                    "native backend: a function value takes 1 to 4 arguments, got {n}"
                ));
            }
            let arg_ir: String =
                arg_vals.iter().map(|v| format!(", %KValue {v}")).collect();
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_call{n}(%KValue {callee}{arg_ir})"));
            f.record(&t, TOP);
            return Ok(t);
        }
        let Expr::Ident(name, _) = head else {
            unreachable!("non-ident heads take the computed path");
        };
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
        let mut iter = args.iter();
        if let Some(first_value) = first {
            emitted.push(first_value);
            iter.next();
        }
        for arg in iter {
            emitted.push(self.emit_expr(f, arg)?);
        }
        // std wrappers reach natives through the builtin_ prefix — and the
        // prefix BYPASSES group dispatch entirely, or a bare clone named
        // like the builtin would capture its own wrapper's body (the
        // d_join_2 self-recursion)
        let was_builtin = name.starts_with("builtin_");
        let name: &str = name.strip_prefix("builtin_").unwrap_or(name);
        if name == "err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_err(%KValue {}, {origin})", emitted[0]));
            f.record(&t, ERR);
            return Ok(t);
        }
        if name == "print" {
            // a non-string argument renders through the same ambient
            // to_string dispatch interpolation uses, so user arms win
            let arg = match f.set_of(&emitted[0]) & !FAIL & !STR {
                0 => emitted[0].clone(),
                _ => {
                    let forced = self.maybe_force(f, emitted[0].clone());
                    let r = f.tmp();
                    f.line(&format!(
                        "{r} = call tailcc %KValue @{}(%KValue {forced})",
                        dsym("render/to_string", 1)
                    ));
                    f.record(&r, STR | (f.set_of(&forced) & FAIL) | ERR);
                    r
                }
            };
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_print(%KValue {arg})"));
            f.record(&t, DESC | (f.set_of(&arg) & FAIL));
            return Ok(t);
        }
        if name == "sleep" || name == "random" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_{name}(%KValue {})", emitted[0]));
            f.record(&t, DESC | (f.set_of(&emitted[0]) & FAIL));
            return Ok(t);
        }
        if let Some(id) = self.type_ids.get(name).copied() {
            // constructors store fields; records never hold thunks in v1
            let emitted: Vec<String> =
                emitted.into_iter().map(|e| self.maybe_force(f, e)).collect();
            self.emit_typeset_checks(f, name, &emitted)?;
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
        if !was_builtin && self.program.fns.iter().any(|d| d.name == *name) {
            let n = emitted.len();
            let args_ir: Vec<String> = emitted
                .iter()
                .enumerate()
                .map(|(i, e)| self.call_arg(f, name, n, i, e))
                .collect();
            let callee_ret = self.ret_ty(name, n);
            let beat_entry = self.beat.ids.contains_key(&(name.to_string(), n));
            if beat_entry {
                // entering a beat loop: mark the frontier; args are already
                // evaluated, so they live below the mark
                f.line("call void @k_beat_push()");
            }
            let t = f.tmp();
            f.line(&format!(
                "{t} = call tailcc {callee_ret} @{}({})",
                dsym(name, n),
                args_ir.join(", ")
            ));
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            let result = if beat_entry {
                let p = f.tmp();
                f.line(&format!("{p} = call %KValue @k_beat_pop(%KValue {t})"));
                p
            } else {
                t
            };
            if callee_ret == "%parsed" {
                f.record_parsed(&result);
            }
            f.record(&result, self.group_return_set(name, n) | fails);
            return Ok(result);
        }
        if name == "at" && emitted.len() == 2 {
            return Ok(self.emit_at(f, &emitted[0].clone(), &emitted[1].clone(), false, span));
        }
        if let Some((_, arity)) = BUILTIN_CALLS.iter().find(|(b, _)| *b == name) {
            if emitted.len() != *arity {
                return Err(format!("native backend: `{name}` takes {arity} argument(s)"));
            }
            // builtins scrutinize every argument; a thunk forces here (the
            // gated force emits nothing when the set proves it can't be one)
            let emitted: Vec<String> =
                emitted.into_iter().map(|e| self.maybe_force(f, e)).collect();
            let mut args_ir: Vec<String> =
                emitted.iter().map(|e| format!("%KValue {e}")).collect();
            // builtins that can give birth to an err take the site's origin
            if matches!(name, "to_int" | "to_float" | "utf8" | "from_code") {
                args_ir.push(self.origin_arg(f, span));
            }
            // A push the linearity analysis proved unique extends its list in
            // place instead of allocating a fresh header.
            let sym = if name == "push"
                && self
                    .in_place_pushes
                    .contains(&(f.file.clone(), span.line, span.col))
            {
                "push_mut"
            } else {
                name
            };
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_b_{sym}({})", args_ir.join(", ")));
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
        params: &[String],
        captures: &[String],
        body: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new();
        f.origin_prefix = outer.origin_prefix.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_env_get(ptr %env, i64 {i})"));
            f.bind(cap, &t);
        }
        for (i, p) in params.iter().enumerate() {
            f.bind(p, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, body)?;
        let sig: String =
            (0..params.len()).map(|i| format!(", %KValue %a{i}")).collect();
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{lifted}(ptr %env{sig}) {{\n{}}}\n",
            f.out
        );
        let _ = writeln!(
            self.body,
            "define %KValue @w_{lifted}(ptr %env{sig}) {{\nentry:\n  %r = call \
             tailcc %KValue @{lifted}(ptr %env{sig})\n  ret %KValue %r\n}}\n"
        );
        Ok(())
    }
}

fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) => {}
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => collect_idents(expr, out),
                }
            }
        }
        Expr::Field { base, .. } => collect_idents(base, out),
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
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
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
