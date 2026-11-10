// Patterns, and the `%parsed` values they read and build: matching a
// parameter against a pattern, returning a failure or a parsed record,
// packing a constructor's arguments and the checks a type set needs. Read
// with include! right after emit.rs, so the methods stay in the codegen
// module.

impl<'a> Backend<'a> {
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
        int_word: &str,
        pattern: &Pattern,
        fail: &str,
        ty: &str,
    ) -> Result<(), String> {
        if let Pattern::Ctor { ty: pty, fields, whole } = pattern {
            if pty == ty {
                // A failure in the status word is the error's own tag with
                // nothing above it, so its low byte, which is the value
                // field's tag, reads as a failure too. When that field's
                // pattern already refuses a failure and sends it to the same
                // label, asking the whole word first is a second test of one
                // fact, made once for every record a parser returns.
                let (int_at, value_at) = self.packed_order(ty);
                let value_refuses = matches!(
                    fields.get(value_at),
                    Some(
                        Pattern::Var(..)
                            | Pattern::Wildcard(_)
                            | Pattern::IntLit(..)
                            | Pattern::Nullary(..)
                    )
                );
                if !value_refuses {
                    let ok = inline_not_failure(f, status);
                    let cont = f.label();
                    f.line_fmt(format_args!("br i1 {ok}, label %{cont}, label %{fail}"));
                    f.start_block(&cont);
                }
                let w0 = f.tmp();
                f.line_fmt(format_args!("{w0} = extractvalue %KValue {status}, 0"));
                let w1 = f.tmp();
                f.line_fmt(format_args!("{w1} = extractvalue %KValue {status}, 1"));
                let (post, posp, vtag, w1) = unpack_words(f, &w0, &w1, int_word);
                let posa = f.tmp();
                f.line_fmt(format_args!("{posa} = insertvalue %KValue undef, i64 {post}, 0"));
                let poskv = f.tmp();
                f.line_fmt(format_args!("{poskv} = insertvalue %KValue {posa}, i64 {posp}, 1"));
                // The position is an int whatever the word held, so its
                // pattern has no failure to refuse.
                self.emit_pattern_known(f, &poskv, &fields[int_at], fail, TOP & !FAIL)?;
                let va = f.tmp();
                f.line_fmt(format_args!("{va} = insertvalue %KValue undef, i64 {vtag}, 0"));
                let vkv = f.tmp();
                f.line_fmt(format_args!("{vkv} = insertvalue %KValue {va}, i64 {w1}, 1"));
                self.emit_pattern(f, &vkv, &fields[value_at], fail)?;
                if let Some(named) = whole {
                    f.bind(&named.0, status);
                }
                return Ok(());
            }
        }
        self.emit_pattern_known(f, status, pattern, fail, TOP)
    }

    /// A field of a record held by value, read from its words. Reading it
    /// the general way builds the record on the heap and asks the runtime
    /// for the field by name. A failure riding in the words comes back as
    /// itself, as it does from the runtime's read. A name that is not one of
    /// the type's two fields is left to that read, which is what says so.
    fn emit_parsed_field(&self, f: &mut FnEmit, record: &str, name: &str) -> Option<String> {
        let (ty, _, back) = f.parsed_as(record)?;
        // An err answers to these three whatever record was expected, and
        // its getter hands back the piece, so they are the getter's to read.
        if crate::ast::ERR_READERS.contains(&name) {
            return None;
        }
        let decl = self.program.types.iter().find(|t| t.name == ty)?;
        let at = decl.fields.iter().position(|(field, _, _)| field == name)?;
        let w0 = f.tmp();
        f.line_fmt(format_args!("{w0} = extractvalue %parsed {record}, 0"));
        let w1 = f.tmp();
        f.line_fmt(format_args!("{w1} = extractvalue %parsed {record}, 1"));
        let (tag, word) = if (at == 1) == back {
            // The int: its tag sits above the value's, and a failure's first
            // word has nothing there, so a failure is asked for by name.
            let w2 = f.tmp();
            f.line_fmt(format_args!("{w2} = extractvalue %parsed {record}, 2"));
            let failed = f.tmp();
            f.line_fmt(format_args!("{failed} = icmp eq i64 {w0}, {K_ERR}"));
            let above = f.tmp();
            f.line_fmt(format_args!("{above} = lshr i64 {w0}, 8"));
            let tag = f.tmp();
            f.line_fmt(format_args!("{tag} = select i1 {failed}, i64 {K_ERR}, i64 {above}"));
            let word = f.tmp();
            f.line_fmt(format_args!("{word} = select i1 {failed}, i64 {w1}, i64 {w2}"));
            (tag.to_string(), word.to_string())
        } else {
            // The value: a failure's first word is the err tag and nothing
            // else, so its low byte and the second word are the failure.
            let tag = f.tmp();
            f.line_fmt(format_args!("{tag} = and i64 {w0}, 255"));
            (tag.to_string(), w1.to_string())
        };
        let a = f.tmp();
        f.line_fmt(format_args!("{a} = insertvalue %KValue undef, i64 {tag}, 0"));
        let v = f.tmp();
        f.line_fmt(format_args!("{v} = insertvalue %KValue {a}, i64 {word}, 1"));
        f.record(&v, TOP);
        Some(v.to_string())
    }

    /// Which of a packed type's two fields is the int and which the value.
    fn packed_order(&self, ty: &str) -> (usize, usize) {
        if self.escape.int_last(ty) {
            (1, 0)
        } else {
            (0, 1)
        }
    }

    /// Return a failure in the group's ABI shape: wrapped in a `%parsed` when the
    /// group returns records by value, a bare KValue otherwise.
    fn emit_ret_failure(&self, f: &mut FnEmit, name: &str, arity: usize, failure: &str) {
        let failure = release_cells(f, failure);
        if self.ret_ty(name, arity) == "%parsed" {
            self.emit_parsed_from_failure(f, &failure);
        } else {
            f.line_fmt(format_args!("ret %KValue {failure}"));
        }
    }

    /// Return a KValue in the current function's ABI shape. A `%parsed`-returning
    /// function builds its record tails directly, so what reaches here as a
    /// value is a failure, whose two words become the `%parsed`, or a record
    /// a group that stays boxed handed back, as a module's bare overload
    /// space does when a local name is also an import's. That record's words
    /// are read off it. Taken for a failure, its tag and pointer went back as
    /// the value word, and the caller read the whole record where a field
    /// belonged (#2432).
    ///
    /// A `%KValue` function can be handed a register-returned record to
    /// return: a thunk site is emitted outside the analysis that gives a tail
    /// caller its callee's return shape, so its tail call comes back `%parsed`
    /// and is boxed here.
    fn emit_ret(&self, f: &mut FnEmit, value: &str) {
        let built = f.built_as(value);
        let value = release_cells(f, value);
        if f.ret_ty == "%parsed" && f.is_parsed(&value) {
            f.line_fmt(format_args!("ret %parsed {value}"));
        } else if let ("%parsed", Some(back)) = (f.ret_ty.as_str(), built) {
            let (w0, w1, w2) = boxed_words(f, &value, back);
            let p = parsed_of(f, &w0, &w1, &w2);
            f.line_fmt(format_args!("ret %parsed {p}"));
        } else if f.ret_ty == "%parsed" {
            self.emit_parsed_from_failure(f, &value);
        } else {
            let value = self.as_value(f, &value);
            f.line_fmt(format_args!("ret %KValue {value}"));
        }
    }

    /// A `%parsed`'s first two words read as a `%KValue`. On the failure path
    /// they are the failure's tag and payload and the third word is zero, so
    /// the discriminator (the err tag in the first word) stays intact.
    fn emit_parsed_from_failure(&self, f: &mut FnEmit, failure: &str) {
        let w0 = f.tmp();
        f.line_fmt(format_args!("{w0} = extractvalue %KValue {failure}, 0"));
        let w1 = f.tmp();
        f.line_fmt(format_args!("{w1} = extractvalue %KValue {failure}, 1"));
        let p = parsed_of(f, &w0, &w1, "0");
        f.line_fmt(format_args!("ret %parsed {p}"));
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
            if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == ty)
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
    fn emit_packed_arg(
        &mut self,
        f: &mut FnEmit,
        args: &[Expr],
        ty: &str,
    ) -> Result<String, String> {
        // The fields are evaluated as written, whichever is the int.
        let first = self.emit_expr(f, &args[0])?;
        self.bail_on_failure(f, &first);
        let second = self.emit_expr(f, &args[1])?;
        self.bail_on_failure(f, &second);
        let back = self.escape.int_last(ty);
        let (pos, value) = if back { (second, first) } else { (first, second) };
        let pid = self.type_ids[ty];
        let (w0, w1, w2) = pack_words(f, &pos, &value);
        let p = parsed_of(f, &w0, &w1, &w2);
        f.record_parsed(&p, ty, pid, back);
        Ok(p.to_string())
    }

    /// Build a register-returnable record in tail position as a by-value
    /// `%parsed`. The three words hold `(pos.tag << 8 | value.tag,
    /// value.payload, pos.payload)`; a non-failure value's tag is never the
    /// err tag, so word 0 still tells success from failure. A failing field
    /// propagates exactly as `k_rec` would have — which means BOTH fields are
    /// evaluated and two failures merge, the way two failing operands of an
    /// operator do. Bailing on the first one skipped the second field
    /// entirely and handed back one reason where the oracle carried two.
    fn emit_parsed_construction(
        &mut self,
        f: &mut FnEmit,
        ty: &str,
        args: &[Expr],
    ) -> Result<(), String> {
        let first = self.emit_expr(f, &args[0])?;
        let second = self.emit_expr(f, &args[1])?;
        self.bail_on_pair_failure(f, &first, &second);
        let (pos, value) = if self.escape.int_last(ty) { (second, first) } else { (first, second) };
        let (w0, w1, w2) = pack_words(f, &pos, &value);
        let p = parsed_of(f, &w0, &w1, &w2);
        f.line_fmt(format_args!("ret %parsed {p}"));
        Ok(())
    }

    /// If either field failed, return the merge of them in the current ABI
    /// shape; otherwise fall through with both known good.
    fn bail_on_pair_failure(&self, f: &mut FnEmit, left: &str, right: &str) {
        let ok_left = inline_not_failure(f, left);
        let ok_right = inline_not_failure(f, right);
        let both = f.tmp();
        f.line_fmt(format_args!("{both} = and i1 {ok_left}, {ok_right}"));
        let cont = f.label();
        let bail = f.label();
        f.line_fmt(format_args!("br i1 {both}, label %{cont}, label %{bail}"));
        f.start_block(&bail);
        let merged = f.tmp();
        f.line_fmt(format_args!(
            "{merged} = call %KValue @k_pair_failure(%KValue {left}, %KValue {right})"
        ));
        self.emit_ret(f, &merged);
        f.start_block(&cont);
    }

    /// If `value` is a failure, return it in the current ABI shape; otherwise
    /// fall through with `value` known good.
    fn bail_on_failure(&self, f: &mut FnEmit, value: &str) {
        let ok = inline_not_failure(f, value);
        let cont = f.label();
        let bail = f.label();
        f.line_fmt(format_args!("br i1 {ok}, label %{cont}, label %{bail}"));
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
            f.predicate(&c, call);
            let b = f.tmp();
            f.line_fmt(format_args!("{b} = icmp ne i64 {c}, 0"));
            let ok = f.label();
            f.line_fmt(format_args!("br i1 {b}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
            let _ = backend;
        };
        let branch_i1 = |f: &mut FnEmit, cond: String| {
            let ok = f.label();
            f.line_fmt(format_args!("br i1 {cond}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
        };
        let tag_is = |f: &mut FnEmit, value: &str, tag: i64| {
            let t = inline_tag(f, value);
            let b = f.tmp();
            f.line_fmt(format_args!("{b} = icmp eq i64 {t}, {tag}"));
            b
        };
        match pattern {
            Pattern::IntLit(n, _) if i64::try_from(n).is_err() => {
                let (name, len) = self.intern(&n.to_string());
                check(
                    self,
                    f,
                    format!("call i64 @k_check_big_lit(%KValue {value}, ptr @{name}, i64 {len})"),
                );
            }
            Pattern::IntLit(n, _) => {
                let is_int = tag_is(f, value, 0);
                let payload = inline_payload(f, value);
                let eq = f.tmp();
                f.line_fmt(format_args!("{eq} = icmp eq i64 {payload}, {n}"));
                let both = f.tmp();
                f.line_fmt(format_args!("{both} = and i1 {is_int}, {eq}"));
                branch_i1(f, both.to_string());
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
                    "done" => K_DONE,
                    _ => K_NONE,
                };
                let b = tag_is(f, value, tag);
                branch_i1(f, b.to_string());
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
                // a thunk passes the check above and can still force to an
                // err, so its FAIL bit stays for the read that forces it
                let kept = if known & crate::infer::THUNK != 0 { known } else { known & !FAIL };
                f.record(value, kept);
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
                // a typeset matches when any member does: OR the members'
                // checks and branch once. A plain annotation is the same
                // shape with one member.
                let members = match self.typesets.get(ty.as_str()) {
                    Some(members) => members.clone(),
                    None => vec![ty.to_string()],
                };
                let mut acc: Option<String> = None;
                for member in &members {
                    let call = self.type_check_call(value, member)?;
                    let c = f.tmp();
                    f.predicate(&c, call);
                    acc = Some(match acc {
                        None => c.to_string(),
                        Some(prev) => {
                            let o = f.tmp();
                            f.line_fmt(format_args!("{o} = or i64 {prev}, {c}"));
                            o.to_string()
                        }
                    });
                }
                let combined = acc.expect("an annotation names at least one type");
                let b = f.tmp();
                f.line_fmt(format_args!("{b} = icmp ne i64 {combined}, 0"));
                branch_i1(f, b.to_string());
                f.bind(name, value);
            }
            Pattern::Ctor { ty, fields, whole } => {
                if ty == "err" {
                    // an arm sees an err out of its box, never the box (#2087)
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERRV})"));
                    let inner = f.tmp();
                    f.line_fmt(format_args!(
                        "{inner} = call %KValue @k_err_inner(%KValue {value})"
                    ));
                    self.emit_pattern(f, &inner, &fields[0], fail)?;
                    if let Some(named) = whole {
                        f.bind(&named.0, value);
                    }
                    return Ok(());
                }
                let id = *self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                check(
                    self,
                    f,
                    format!(
                        "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {})",
                        fields.len()
                    ),
                );
                for (i, field) in fields.iter().enumerate() {
                    let fv = f.tmp();
                    f.line_fmt(format_args!(
                        "{fv} = call %KValue @k_field_fast(%KValue {value}, i64 {i})"
                    ));
                    self.emit_pattern(f, &fv, field, fail)?;
                }
                // the as-pattern's name takes the value that matched, so an
                // arm answering it hands back what it was given
                if let Some(named) = whole {
                    f.bind(&named.0, value);
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
                f.line_fmt(format_args!("{c} = {call}"));
                let b = f.tmp();
                f.line_fmt(format_args!("{b} = icmp ne i64 {c}, 0"));
                matched = Some(match matched {
                    None => b.to_string(),
                    Some(prev) => {
                        let t = f.tmp();
                        f.line_fmt(format_args!("{t} = or i1 {prev}, {b}"));
                        t.to_string()
                    }
                });
            }
            let matched = matched.expect("a typeset has members");
            let not_fail = inline_not_failure(f, value);
            let not_matched = f.tmp();
            f.line_fmt(format_args!("{not_matched} = xor i1 {matched}, true"));
            let bad = f.tmp();
            f.line_fmt(format_args!("{bad} = and i1 {not_matched}, {not_fail}"));
            let die = f.label();
            let ok = f.label();
            f.line_fmt(format_args!("br i1 {bad}, label %{die}, label %{ok}"));
            f.start_block(&die);
            let msg = format!("field `{field}` of `{name}` takes {}\0", tys.join(" "));
            let (m, _) = self.intern(&msg);
            f.line_fmt(format_args!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
            f.start_block(&ok);
        }
        Ok(())
    }

    /// A field typeset's member resolves exactly as a parameter annotation
    /// does; keeping one resolver is what stops the two drifting apart.
    fn member_check_call(&self, value: &str, member: &str) -> Result<String, String> {
        self.type_check_call(value, member)
    }

    fn field_count(&self, ty: &str) -> Result<usize, String> {
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.len())
            .ok_or_else(|| format!("native backend: unknown type `{ty}`"))
    }
}
