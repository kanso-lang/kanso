// The operator paths: word runs that chain arithmetic on raw words, the
// general arithmetic they fall back to, and the binary operators with the
// builtins behind them. Read with include! right after emit.rs, so the
// methods stay in the codegen module.

impl<'a> Backend<'a> {
    /// An expression of two or more `+ - * / %` over words, such as
    /// `(k * 31 + n * 7 + 3) % 997`, runs on raw words with one overflow
    /// test at the end. Every operation on its own merged its overflow call
    /// back through a phi, and the next operation tested the phi's tag:
    /// escapebench's inner loop went from 33 instructions an element to 60.
    /// Its operands are evaluated once, in order, and an overflow anywhere
    /// re-runs the operations the general way on the same values, which the
    /// operations being pure makes the same answer.
    fn emit_word_run(&mut self, f: &mut FnEmit, e: &Expr) -> Result<String, String> {
        let mut leaves = Vec::new();
        arith_leaves(e, &mut leaves);
        let mut vals = Vec::with_capacity(leaves.len());
        for leaf in leaves {
            let v = self.emit_expr(f, leaf)?;
            let v = self.maybe_force(f, v);
            vals.push(self.as_value(f, &v));
        }
        if vals.iter().any(|v| !infer::word(f.set_of(v))) {
            return self.emit_arith_general(f, e, &vals, &mut 0);
        }
        let mut flags = Vec::new();
        let words: Vec<(String, Set)> =
            vals.iter().map(|v| (inline_payload(f, v), f.set_of(v))).collect();
        let slow = f.label();
        let (r, known) = word_run(f, e, &words, &mut 0, &mut flags, Some(&slow));
        let fv = f.tmp();
        f.line_fmt(format_args!("{fv} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"));
        if flags.is_empty() {
            f.record(&fv, known);
            return Ok(fv.to_string());
        }
        let fast = f.cur_label.clone();
        let merge = f.label();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&slow);
        let sv = self.emit_arith_general(f, e, &vals, &mut 0)?;
        let slow_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = phi %KValue [ {fv}, %{fast} ], [ {sv}, %{slow_from} ]"));
        f.record(&t, known | f.set_of(&sv));
        Ok(t.to_string())
    }

    /// The operations of a word run, the general way, over operands already
    /// evaluated.
    fn emit_arith_general(
        &mut self,
        f: &mut FnEmit,
        e: &Expr,
        vals: &[String],
        at: &mut usize,
    ) -> Result<String, String> {
        match e {
            Expr::BinOp { op, lhs, rhs, span } if word_arith(op) => {
                let a = self.emit_arith_general(f, lhs, vals, at)?;
                let b = self.emit_arith_general(f, rhs, vals, at)?;
                self.emit_binop(f, op, &a, &b, *span)
            }
            _ => {
                *at += 1;
                Ok(vals[*at - 1].clone())
            }
        }
    }

    fn emit_binop(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        // an operator reads ordinary values on both sides
        let a_owned = self.as_value(f, a);
        let b_owned = self.as_value(f, b);
        let (a, b) = (a_owned.as_str(), b_owned.as_str());
        // a record on either side dispatches to the operator's user arms; the
        // numeric fast paths below stay untouched for everything else
        let armable = matches!(op, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
            && self.declares_at(op, 2);
        if armable && (f.set_of(a) | f.set_of(b)) & REC != 0 {
            let a_routes = f.tmp();
            f.line_fmt(format_args!("{a_routes} = call i64 @k_routes_to_arms(%KValue {a})"));
            let b_routes = f.tmp();
            f.line_fmt(format_args!("{b_routes} = call i64 @k_routes_to_arms(%KValue {b})"));
            let either = f.tmp();
            f.line_fmt(format_args!("{either} = or i64 {a_routes}, {b_routes}"));
            let isrec = f.tmp();
            f.line_fmt(format_args!("{isrec} = icmp ne i64 {either}, 0"));
            let user = f.label();
            let builtin = f.label();
            let merge = f.label();
            f.line_fmt(format_args!("br i1 {isrec}, label %{user}, label %{builtin}"));
            f.start_block(&user);
            let uv = f.tmp();
            f.line_fmt(format_args!(
                "{uv} = call tailcc %KValue @{}(%KValue {a}, %KValue {b})",
                dsym(op, 2)
            ));
            f.line_fmt(format_args!("br label %{merge}"));
            let user_from = user;
            f.start_block(&builtin);
            let bv = self.emit_binop_builtin(f, op, a, b, span)?;
            let builtin_from = f.cur_label.clone();
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = phi %KValue [ {uv}, %{user_from} ], [ {bv}, %{builtin_from} ]"
            ));
            f.record(
                &t,
                f.set_of(&bv) | self.group_return_set(op, 2) | ((f.set_of(a) | f.set_of(b)) & FAIL),
            );
            return Ok(t.to_string());
        }
        self.emit_binop_builtin(f, op, a, b, span)
    }

    /// What a zero divisor answers beyond the number: a `divide_by_zero`
    /// record where the program declares the type, which an operator's arms
    /// then have to be asked about. Where it does not, the answer is the bare
    /// text, which no arm is asked about.
    fn zero_divisor_set(&self) -> Set {
        match self.type_ids.contains_key(crate::DIVIDE_BY_ZERO) {
            true => REC,
            false => 0,
        }
    }

    fn emit_binop_builtin(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        let bitwise = match op {
            "&" => Some("bit_and"),
            "|" => Some("bit_or"),
            "^" => Some("bit_xor"),
            _ => None,
        };
        if let Some(name) = bitwise {
            let args = [a.to_string(), b.to_string()];
            if let Some(t) = direct_bit_op(f, name, &args, |f| self.origin_arg(f, span)) {
                return Ok(t);
            }
        }
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
            ">=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 5)"),
            "&" => {
                let site = self.origin_arg(f, span);
                format!("call %KValue @k_b_bit_and_fast(%KValue {a}, %KValue {b}, {site})")
            }
            "|" => {
                let site = self.origin_arg(f, span);
                format!("call %KValue @k_b_bit_or_fast(%KValue {a}, %KValue {b}, {site})")
            }
            "^" => {
                let site = self.origin_arg(f, span);
                format!("call %KValue @k_b_bit_xor_fast(%KValue {a}, %KValue {b}, {site})")
            }
            // An operator the parser accepts and this does not know used to
            // land on the last arm and compare, which is a wrong answer with
            // nothing said. Naming every operator means a new one refuses to
            // build instead.
            other => return Err(format!("native backend: no lowering for `{other}`")),
        };
        if matches!(op, "&" | "|" | "^") {
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = {slow_call}"));
            let bound = infer::bitwise_widths(op, f.set_of(a), f.set_of(b));
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | infer::WORD | bound);
            return Ok(t.to_string());
        }
        let pure_int = infer::word(f.set_of(a)) && infer::word(f.set_of(b));
        // Ints of either kind, or a failure riding with one: the quotient
        // and remainder of two words take the same one instruction behind a
        // test that both tags say word, and the rest goes to the call.
        let int_like = |s: Set| s & infer::ANY_INT != 0 && s & !(infer::ANY_INT | FAIL) == 0;
        let guarded = !pure_int && int_like(f.set_of(a)) && int_like(f.set_of(b));
        if (op == "%" || op == "/") && (pure_int || guarded) {
            // Two integers the inference has proved: the remainder or the
            // quotient is one instruction, and the two divisors it cannot
            // take — zero, whose failure the runtime words, and minus one,
            // whose quotient overflows at INT64_MIN and traps on x86 — go to
            // the call as before. runbench asked k_mod 2,565,677 times, 43
            // million instructions inside the call, every one of them an
            // integer pair.
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let zero = f.tmp();
            f.line_fmt(format_args!("{zero} = icmp eq i64 {pb}, 0"));
            let minus = f.tmp();
            f.line_fmt(format_args!("{minus} = icmp eq i64 {pb}, -1"));
            let mut edge = f.tmp();
            f.line_fmt(format_args!("{edge} = or i1 {zero}, {minus}"));
            if guarded {
                let ta = inline_tag(f, a);
                let tb = inline_tag(f, b);
                let both = both_ints(f, &ta, &tb);
                let not_both = f.tmp();
                f.line_fmt(format_args!("{not_both} = xor i1 {both}, true"));
                let any = f.tmp();
                f.line_fmt(format_args!("{any} = or i1 {edge}, {not_both}"));
                edge = any;
            }
            let fast = f.label();
            let slow = f.label();
            let merge = f.label();
            f.line_fmt(format_args!("br i1 {edge}, label %{slow}, label %{fast}"));
            f.start_block(&fast);
            let r = f.tmp();
            let insn = if op == "%" { "srem" } else { "sdiv" };
            f.line_fmt(format_args!("{r} = {insn} i64 {pa}, {pb}"));
            let fv = f.tmp();
            f.line_fmt(format_args!(
                "{fv} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"
            ));
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&slow);
            let sv = f.tmp();
            self.stamp_site(f, span);
            f.line_fmt(format_args!("{sv} = {slow_call}"));
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = phi %KValue [ {fv}, %{fast} ], [ {sv}, %{slow} ]"));
            // The least int over minus one is the one quotient of two words
            // that is not a word. A remainder by a word is smaller than the
            // word, so only a bignum divisor can make it one.
            let wide = match op {
                "/" => infer::BIG,
                _ => f.set_of(b) & infer::BIG,
            };
            let fails = (f.set_of(a) | f.set_of(b)) & FAIL;
            // a divisor written as a literal other than zero cannot fail
            let refused = match pb.parse::<i64>() {
                Ok(d) if d != 0 => 0,
                _ => ERR | self.zero_divisor_set(),
            };
            f.record(&t, fails | INT | wide | refused);
            return Ok(t.to_string());
        }
        if op == "/" || op == "%" {
            let t = f.tmp();
            self.stamp_site(f, span);
            f.line_fmt(format_args!("{t} = {slow_call}"));
            f.record(
                &t,
                (f.set_of(a) & FAIL)
                    | (f.set_of(b) & FAIL)
                    | INT
                    | infer::BIG
                    | FLOAT
                    | ERR
                    | self.zero_divisor_set(),
            );
            return Ok(t.to_string());
        }
        if pure_int {
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let t = match op {
                "+" | "-" if infer::sum_widths(op, f.set_of(a), f.set_of(b)).is_some() => {
                    // Two ints with a bound below 2^62 cannot leave the word,
                    // so the sum is the one instruction with no overflow test
                    // and no bignum behind it (kanso#2304).
                    let bound = infer::sum_widths(op, f.set_of(a), f.set_of(b)).unwrap_or(0);
                    let insn = if op == "+" { "add" } else { "sub" };
                    let r = f.tmp();
                    f.line_fmt(format_args!("{r} = {insn} nsw i64 {pa}, {pb}"));
                    let v = f.tmp();
                    f.line_fmt(format_args!(
                        "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"
                    ));
                    f.known_words.insert(v.to_string(), ("0".to_string(), r.to_string()));
                    f.record(&v, infer::WORD | bound);
                    v
                }
                "+" | "-" | "*" => {
                    let intrinsic = match op {
                        "+" => "llvm.sadd.with.overflow.i64",
                        "-" => "llvm.ssub.with.overflow.i64",
                        _ => "llvm.smul.with.overflow.i64",
                    };
                    let pair = f.tmp();
                    f.line_fmt(format_args!(
                        "{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"
                    ));
                    let sum = f.tmp();
                    f.line_fmt(format_args!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                    let overflow = f.tmp();
                    f.line_fmt(format_args!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                    // A twin that has written nothing yet starts the general
                    // body over instead, so the sum stays a word here.
                    if f.bail_on(&overflow) {
                        let v = f.tmp();
                        f.line_fmt(format_args!(
                            "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"
                        ));
                        f.record(&v, INT);
                        return Ok(v.to_string());
                    }
                    // A result past a word is a bignum, which the runtime
                    // builds from the two operands; the word path stays the
                    // three instructions it was.
                    let ok = f.label();
                    let wide = f.label();
                    let merge = f.label();
                    f.line_fmt(format_args!("br i1 {overflow}, label %{wide}, label %{ok}"));
                    f.start_block(&ok);
                    let v = f.tmp();
                    f.line_fmt(format_args!(
                        "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"
                    ));
                    f.line_fmt(format_args!("br label %{merge}"));
                    f.start_block(&wide);
                    let w = f.tmp();
                    let helper = match op {
                        "+" => "k_int_add",
                        "-" => "k_int_sub",
                        _ => "k_int_mul",
                    };
                    f.line_fmt(format_args!(
                        "{w} = call %KValue @{helper}(%KValue {a}, %KValue {b})"
                    ));
                    f.line_fmt(format_args!("br label %{merge}"));
                    f.start_block(&merge);
                    let t = f.tmp();
                    f.line_fmt(format_args!("{t} = phi %KValue [ {v}, %{ok} ], [ {w}, %{wide} ]"));
                    f.record(&t, INT | infer::BIG);
                    t
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
                    f.line_fmt(format_args!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                    let v = f.tmp();
                    f.line_fmt(format_args!(
                        "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                    ));
                    f.record(&v, infer::BOOL);
                    v
                }
            };
            return Ok(t.to_string());
        }
        let ta = inline_tag(f, a);
        let tb = inline_tag(f, b);
        let both = both_ints(f, &ta, &tb);
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line_fmt(format_args!("br i1 {both}, label %{fast}, label %{slow}"));
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
                f.line_fmt(format_args!(
                    "{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"
                ));
                let sum = f.tmp();
                f.line_fmt(format_args!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                let overflow = f.tmp();
                f.line_fmt(format_args!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                let fast_ok = f.label();
                f.line_fmt(format_args!("br i1 {overflow}, label %{slow}, label %{fast_ok}"));
                f.start_block(&fast_ok);
                let v = f.tmp();
                f.line_fmt(format_args!(
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
                f.line_fmt(format_args!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                let v = f.tmp();
                f.line_fmt(format_args!(
                    "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                ));
                (v, fast)
            }
        };
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&slow);
        let sv = f.tmp();
        self.stamp_site(f, span);
        f.line_fmt(format_args!("{sv} = {slow_call}"));
        let slow_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = phi %KValue [ {fast_value}, %{fast_from} ], [ {sv}, %{slow_from} ]"
        ));
        // This phi carried no set until 2026-09-07, so every reader of it took
        // the default, which is TOP, which contains THUNK -- and an `if` over a
        // comparison then emitted `k_force_fast` on a value that is a boolean
        // by construction. Both arms are known: the fast one is the select or
        // the insertvalue written just above, and the slow one is `k_cmp`,
        // which answers `k_bool` or the failure it was handed, or `k_add` and
        // its two siblings, which answer an int, a float or that same failure.
        // None of the six can answer a thunk.
        f.record(
            &t,
            match op {
                "+" | "-" | "*" => {
                    // the call answers a float only when a float went in
                    let float = match (f.set_of(a) | f.set_of(b)) & FLOAT {
                        0 => 0,
                        _ => FLOAT,
                    };
                    (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT | infer::BIG | float
                }
                _ => (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | infer::BOOL,
            },
        );
        Ok(t.to_string())
    }
}
