// The expression emitter: patterns, bodies, binds, conditions and the
// arithmetic paths, the Backend methods that turn one function's body into
// LLVM IR. Calls are in calls.rs. codegen.rs reads both files with include!,
// so the methods stay in the codegen module and the compiler builds to the
// same instructions it did when they sat in codegen.rs (kanso#1947). cargo
// fmt does not reach an included file, so CI's format job checks the files
// under src/'s subdirectories itself.

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
                let value_refuses = matches!(
                    fields.get(1),
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
                self.emit_pattern_known(f, &poskv, &fields[0], fail, TOP & !FAIL)?;
                let va = f.tmp();
                f.line_fmt(format_args!("{va} = insertvalue %KValue undef, i64 {vtag}, 0"));
                let vkv = f.tmp();
                f.line_fmt(format_args!("{vkv} = insertvalue %KValue {va}, i64 {w1}, 1"));
                self.emit_pattern(f, &vkv, &fields[1], fail)?;
                if let Some(named) = whole {
                    f.bind(&named.0, status);
                }
                return Ok(());
            }
        }
        self.emit_pattern_known(f, status, pattern, fail, TOP)
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
    /// function only reaches here with a failure (its record tails are built
    /// directly), so the failure's two words become the `%parsed`.
    ///
    /// A `%KValue` function can be handed a register-returned record to
    /// return: a thunk site is emitted outside the analysis that gives a tail
    /// caller its callee's return shape, so its tail call comes back `%parsed`
    /// and is boxed here.
    fn emit_ret(&self, f: &mut FnEmit, value: &str) {
        let value = release_cells(f, value);
        if f.ret_ty == "%parsed" {
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
        let pos = self.emit_expr(f, &args[0])?;
        self.bail_on_failure(f, &pos);
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_failure(f, &value);
        let pid = self.type_ids[ty];
        let (w0, w1, w2) = pack_words(f, &pos, &value);
        let p = parsed_of(f, &w0, &w1, &w2);
        f.record_parsed(&p, ty, pid);
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
    fn emit_parsed_construction(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<(), String> {
        let pos = self.emit_expr(f, &args[0])?;
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_pair_failure(f, &pos, &value);
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

    /// A sleep carrying the cells `ahead::pool` bound ahead of its chain.
    /// Each cell is handed over as it is, unforced, since the runtime forces
    /// it while the sleep parks.
    fn emit_pooled_sleep(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<String, String> {
        let ms = self.emit_expr(f, &args[0])?;
        let mut cells = String::new();
        for cell in &args[1..] {
            let Expr::Ident(name, _, _) = cell else {
                return Err("native backend: a pooled sleep carries its cells by name".into());
            };
            let held =
                f.lookup(name).ok_or_else(|| format!("native backend: `{name}` is not bound"))?;
            let held = self.as_value(f, &held);
            cells.push_str(&format!(", %KValue {held}"));
        }
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue (%KValue, i32, ...) @k_desc_sleep_pooled(%KValue {ms}, i32 {}{cells})",
            args.len() - 1
        ));
        f.record(&t, DESC | (f.set_of(&ms) & FAIL));
        Ok(t.to_string())
    }

    fn emit_fn_body(&mut self, f: &mut FnEmit, body: &[Stmt]) -> Result<(), String> {
        let last = body.len() - 1;
        let mut values_bound: Option<usize> = None;
        for (i, stmt) in body.iter().enumerate() {
            if values_bound == Some(i) {
                continue;
            }
            // `ks = keys m` straight after which `vs = values m`: both lists
            // from one call, the way `both_columns` does it for arguments.
            if let Some(next) = self.columns_bound(f, body, i) {
                let (
                    Stmt::Bind { pattern: Pattern::Var(ks_name, _), expr },
                    Stmt::Bind { pattern: Pattern::Var(vs_name, _), .. },
                ) = (stmt, &body[next])
                else {
                    unreachable!("columns_bound matched two bindings")
                };
                let (ks, vs) = self.emit_columns(f, expr)?;
                f.bind(ks_name, &ks);
                f.bind(vs_name, &vs);
                values_bound = Some(next);
                continue;
            }
            match stmt {
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&f.group.clone(), f.arity, i)
                        && self.thunkable(f, expr) =>
                {
                    let t = match name.as_str().starts_with(crate::ahead::POOLED) {
                        true => self.emit_cell_by(f, expr, "k_thunk_new_pooled")?,
                        false => self.emit_cell(f, expr)?,
                    };
                    let in_beat = self.beat.ids.contains_key(&(f.group.clone(), f.arity));
                    if !in_beat && self.demand.is_releasable(&f.group, f.arity, i) {
                        f.lazy_cells.push(t.clone());
                    }
                    f.bind(name, &t);
                }
                Stmt::Bind {
                    pattern: Pattern::Var(name, _),
                    expr: Expr::App { args, span, .. },
                } if self.framed_views.contains(&(f.group.clone(), f.arity, *span)) => {
                    // Only a proven string: a header that may be the frame's
                    // or the arena's is a pointer LLVM cannot take apart,
                    // and the frame then saved 11,110,137 of the 28,013,580
                    // instructions it saves on runbench when it can. So
                    // `k_b_bytes_frame` has no slow arm. It allocates nothing,
                    // and the counting build takes it too: the header is not an
                    // arena byte, and k_stat_sh_bytes has nothing to count.
                    let v = self.emit_expr(f, &args[0])?;
                    let v = self.unsub(f, v);
                    let arg_sets = [f.set_of(&v)];
                    let t = f.tmp();
                    if arg_sets[0] == STR {
                        let slot = f.tmp();
                        f.line_fmt(format_args!("{slot} = alloca [3 x i64], align 8"));
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_bytes_frame(%KValue {v}, ptr {slot})"
                        ));
                        f.frame_held = true;
                    } else {
                        let site = self.origin_arg(f, *span);
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_bytes_fast(%KValue {v}, {site})"
                        ));
                    }
                    f.record(&t, infer::builtin_set("bytes", &arg_sets));
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
                        Pattern::Ctor { ty, fields, .. } => {
                            let id = *self
                                .type_ids
                                .get(ty.as_str())
                                .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                            let c = f.tmp();
                            f.predicate(
                                &c,
                                format!(
                                    "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {})",
                                    fields.len()
                                ),
                            );
                            let b = f.tmp();
                            f.line_fmt(format_args!("{b} = icmp ne i64 {c}, 0"));
                            let ok = f.label();
                            let bad = f.label();
                            f.line_fmt(format_args!("br i1 {b}, label %{ok}, label %{bad}"));
                            f.start_block(&bad);
                            // The value goes to the runtime rather than a baked
                            // sentence: the reader wants to see what they bound,
                            // and only the runtime knows it. Its keyed sibling
                            // `k_keyed_check` has always worked this way.
                            let (m, _) = self.intern(&format!("{ty}\0"));
                            self.stamp_site(f, expr.span());
                            f.line_fmt(format_args!(
                                "call void @k_die_destructure(%KValue {value}, ptr @{m})"
                            ));
                            f.line("unreachable");
                            f.start_block(&ok);
                            for (i, field) in fields.iter().enumerate() {
                                if let Pattern::Var(name, _) = field {
                                    let fv = f.tmp();
                                    f.line_fmt(format_args!(
                                        "{fv} = call %KValue @k_field_fast(%KValue {value}, i64 {i})"
                                    ));
                                    f.bind(name, &fv);
                                }
                            }
                        }
                        Pattern::Keyed { entries, .. } => {
                            let checked = f.tmp();
                            self.stamp_site(f, expr.span());
                            f.line_fmt(format_args!(
                                "{checked} = call %KValue @k_keyed_check(%KValue {value}, i64 {})",
                                entries.len()
                            ));
                            for entry in entries {
                                let (name, _) = self.intern(&format!("{}\0", entry.field));
                                let fv = f.tmp();
                                f.line_fmt(format_args!(
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

    /// A partial application, as the lambda it is equivalent to: `&add 2`
    /// becomes `(x -> add 2 x)`. The remaining arity has to be unambiguous —
    /// with both `add a b` and `add a b c` declared, `&add 2` could be waiting
    /// for one argument or two, and the interpreter defers that choice until
    /// the arguments arrive. Nothing here can defer, so an ambiguous partial
    /// is refused out loud rather than guessed at, which is the escape the
    /// differential law allows an engine that covers less.
    fn partial_lambda(&self, name: &str, supplied: &[Expr], span: Span) -> Result<Expr, String> {
        let arities: Vec<usize> = {
            let mut seen: Vec<usize> = self
                .program
                .fns
                .iter()
                .filter(|d| d.name == name && d.params.len() >= supplied.len())
                .map(|d| d.params.len())
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen
        };
        // A name no declaration answers to is a VALUE here — a parameter
        // holding a function, a local, a builtin, a record's constructor —
        // and the callers route that to `emit_partial_value` before asking
        // for a lambda, so what reaches this point is a declared group.
        debug_assert!(self.declares(name));
        // Currying past every arm is the one real error: `&` supplies without
        // running, so supplying an arm's last argument is a partial like any
        // other — the value waits to be called rather than being a call. What
        // nothing can finish is more arguments than any arm accepts.
        if arities.is_empty() {
            return Err(format!(
                "native backend: `&{name}` holds {} argument(s), and no `{name}` takes more",
                supplied.len()
            ));
        }
        // A partial that escapes as a value has to become a closure, and a
        // closure fixes its parameter count when it is written. That is fine
        // when one arm can still finish it; with several, the count is decided
        // by the arguments that arrive, which a closure cannot wait for. The
        // shape that needs it is a partial bound to a name and later applied
        // with more arguments than the shortest arm wants.
        let Some(&arity) = arities.first() else { unreachable!("checked non-empty") };
        if arities.len() > 1 {
            return Err(format!(
                "native backend: `&{name}` escapes as a value while {} arms could still finish \
                 it, and lowering it needs a partial the runtime does not have yet",
                arities.len()
            ));
        }
        let waiting = arity - supplied.len();
        let params: Vec<(String, Span)> =
            (0..waiting).map(|i| (format!("k#partial{i}"), span)).collect();
        let mut args = supplied.to_vec();
        args.extend(params.iter().map(|(n, s)| {
            Expr::Ident(Name::new(&n.clone()), *s, crate::ast::Resolution::default())
        }));
        let head = Expr::Ident(Name::new(name), span, crate::ast::Resolution::default());
        let body = Expr::App { head: Box::new(head), args, piped: false, span };
        Ok(Expr::Lambda { params, body: Box::new(body), span })
    }

    /// `(&g a) b` finished where it is written, lowered to the call `g a b`.
    /// The held arguments are evaluated first, and a failure among them is the
    /// answer without entering `g`, as the interpreter answers it at the `&`.
    /// Entering `g` with it let an arm that ignores the argument answer in its
    /// place: inference never sees a held argument reach `g`'s parameters, so
    /// `g` carried no guard for one.
    #[allow(clippy::too_many_arguments)]
    fn emit_finished_partial(
        &mut self,
        f: &mut FnEmit,
        name: &Name,
        nspan: Span,
        held: &[Expr],
        rest: &[Expr],
        piped: bool,
        span: Span,
    ) -> Result<String, String> {
        let mut all = Vec::new();
        let mut asked = Vec::new();
        for a in held {
            let t = self.emit_expr(f, a)?;
            let n = format!("k#held{}", f.tmp().trim_start_matches('%'));
            f.bind(&n, &t);
            all.push(Expr::Ident(Name::new(&n), span, crate::ast::Resolution::default()));
            if !t.starts_with('{') {
                asked.push(t);
            }
        }
        all.extend(rest.iter().cloned());
        let callee = Box::new(Expr::Ident(name.clone(), nspan, crate::ast::Resolution::default()));
        let Some(last) = asked.last().cloned() else {
            return self.emit_call_full(f, &callee, &all, piped, span);
        };
        let (mut first, mut bad) = (last, "false".to_string());
        for t in asked.iter().rev() {
            let ok = inline_not_failure(f, t);
            let pick = f.tmp();
            f.line_fmt(format_args!("{pick} = select i1 {ok}, %KValue {first}, %KValue {t}"));
            let worse = f.tmp();
            f.line_fmt(format_args!("{worse} = select i1 {ok}, i1 {bad}, i1 true"));
            (first, bad) = (pick.to_string(), worse.to_string());
        }
        let (go, skip, join) = (f.label(), f.label(), f.label());
        f.line_fmt(format_args!("br i1 {bad}, label %{skip}, label %{go}"));
        f.start_block(&go);
        let called = self.emit_call_full(f, &callee, &all, piped, span)?;
        let called = match f.is_parsed(&called) {
            true => f.box_parsed(&called),
            false => called,
        };
        let called_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{join}"));
        f.start_block(&skip);
        f.line_fmt(format_args!("br label %{join}"));
        f.start_block(&join);
        let answer = f.tmp();
        f.line_fmt(format_args!(
            "{answer} = phi %KValue [ {called}, %{called_from} ], [ {first}, %{skip} ]"
        ));
        let set = f.set_of(&called) | FAIL;
        f.record(&answer, set);
        Ok(answer.to_string())
    }

    /// `&g a b` over a declared group, held by `k_partial{n}` over the value
    /// the group is handed out as. The held arguments are evaluated once, at
    /// the `&`, as the interpreter evaluates them, and a held failure is the
    /// partial's answer. Until 2026-09-29 a lambda over the group held the
    /// argument expressions and ran them at each call, so `&g p v` with `v`
    /// an err built a function, and an arm that ignored its argument answered
    /// where the interpreter answered the err.
    fn emit_declared_partial(
        &mut self,
        f: &mut FnEmit,
        name: &str,
        supplied: &[Expr],
        span: Span,
    ) -> Result<String, String> {
        // The group's value names the group: a call that turns down a
        // failing argument says so in the trace, as a direct call does. A
        // closure over the group answered the failure before entering it,
        // and the trace lost the group. `partial_lambda` still refuses what
        // it refuses: several arms, or more arguments than any arm takes.
        self.partial_lambda(name, supplied, span)?;
        self.emit_partial_value(f, &Name::new(name), supplied, span)
    }

    /// Whether a name is a declared group, which is what decides how `&` over
    /// it lowers: a group's arities are known here, a value's are not.
    fn declared(&self, name: &str) -> bool {
        self.declares(name)
    }

    /// `&f 2` where `f` is a VALUE — a parameter, a local, a builtin handed
    /// out, a constructor. Its arity is settled when the arguments arrive,
    /// which is the interpreter's rule and one a lambda cannot follow, since a
    /// lambda fixes its count where it is written. The runtime holds the
    /// callee and the supplied arguments in a body-less closure and settles
    /// the count at each call; `k_partial{n}` builds one.
    fn emit_partial_value(
        &mut self,
        f: &mut FnEmit,
        name: &Name,
        supplied: &[Expr],
        span: Span,
    ) -> Result<String, String> {
        let callee =
            self.emit_expr(f, &Expr::Ident(name.clone(), span, crate::ast::Resolution::default()))?;
        let mut held: Vec<String> = Vec::new();
        for a in supplied {
            held.push(self.emit_expr(f, a)?);
        }
        // Past four the runtime's builders stop, so the rest are held by a
        // partial over the first: the runtime settles a partial over a
        // partial by the count its callee still wants, and a failing
        // argument answers in the order it was written either way.
        let mut callee = callee;
        let mut rest: &[String] = &held;
        loop {
            let take = rest.len().min(4);
            let arg_ir: String = rest[..take].iter().map(|v| format!(", %KValue {v}")).collect();
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = call %KValue @k_partial{take}(%KValue {callee}{arg_ir})"
            ));
            f.record(&t, TOP);
            callee = t.to_string();
            rest = &rest[take..];
            if rest.is_empty() {
                return Ok(callee);
            }
        }
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            // the interpreter is the oracle for `&`; the backends reject it out
            // loud rather than lowering something that would diverge
            Expr::Partial(name, span) => {
                if !self.declared(name) {
                    return self.emit_partial_value(f, name, &[], *span);
                }
                self.emit_declared_partial(f, name, &[], *span)
            }
            Expr::Upcast { expr: inner, ty, span } => {
                let v = self.emit_expr(f, inner)?;
                let v = self.maybe_force(f, v);
                let want = self.sub_want(ty)?;
                let (tyn, _) = self.intern(&format!("{ty}\0"));
                let t = f.tmp();
                self.stamp_site(f, *span);
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_upcast(%KValue {v}, i64 {want}, ptr @{tyn})"
                ));
                f.record(&t, crate::infer::TOP);
                Ok(t.to_string())
            }
            Expr::Block(stmts, _) => self.emit_stmts_value(f, stmts),
            // A guard inside a block that is read for its value, an `if`
            // branch bound to a name, leaves that block: its early answer is
            // the block's, and the rest of the block is the other arm. The
            // interpreter reads it the same way.
            Expr::Guard { cond, early, rest, .. } => self.emit_guard_value(f, cond, early, rest),
            // A literal wider than a word is a bignum the runtime reads from
            // its digits, each time the literal is reached.
            Expr::Int(n, _) => match i64::try_from(n) {
                Ok(fits) => Ok(format!("{{ i64 0, i64 {fits} }}")),
                Err(_) => {
                    let (name, len) = self.intern(&n.to_string());
                    let t = f.tmp();
                    f.line_fmt(format_args!(
                        "{t} = call %KValue @k_int_of_digits(ptr @{name}, i64 {len})"
                    ));
                    f.record(&t, infer::BIG);
                    Ok(t.to_string())
                }
            },
            Expr::Float(x, _) => {
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_float(double 0x{:016X})",
                    x.to_bits()
                ));
                Ok(t.to_string())
            }
            Expr::Str(parts, span) => {
                let joins_builder = self.builder_joins.contains(&(
                    f.file.clone(),
                    span.line as usize,
                    span.col as usize,
                ));
                let mut acc: Option<Vec<String>> = None;
                let mut fails: Set = 0;
                for part in parts {
                    let piece = match part {
                        TemplatePart::Lit(s) => self.str_const(f, s),
                        TemplatePart::Interp(inner) => {
                            let raw = self.emit_expr(f, inner)?;
                            let value = self.maybe_force(f, raw);
                            // A seed that was not a string, or arrived lazily,
                            // renders to a finished string; the in-place join
                            // makes it the builder when it finds no room.
                            let (t, failed) = self.render_interp(f, &value);
                            fails |= failed;
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
                        f.line_fmt(format_args!("{arr} = alloca [{} x %KValue]", pieces.len()));
                        for (i, p) in pieces.iter().enumerate() {
                            let slot = f.tmp();
                            f.line_fmt(format_args!(
                                "{slot} = getelementptr [{} x %KValue], ptr {arr}, i64 0, i64 {i}",
                                pieces.len()
                            ));
                            f.line_fmt(format_args!("store %KValue {p}, ptr {slot}"));
                        }
                        let t = f.tmp();
                        let sym = match joins_builder {
                            true => "k_concat_arr_mut",
                            false => "k_concat_arr",
                        };
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @{sym}(i64 {}, ptr {arr})",
                            pieces.len()
                        ));
                        t.to_string()
                    }
                    Some(pieces) => {
                        let mut it = pieces.into_iter();
                        let mut prev = it.next().expect("non-empty");
                        for piece in it {
                            let t = f.tmp();
                            f.line_fmt(format_args!(
                                "{t} = call %KValue @k_concat(%KValue {prev}, %KValue {piece})"
                            ));
                            prev = t.to_string();
                        }
                        prev
                    }
                    None => self.str_const(f, ""),
                };
                f.record(&out, STR | fails);
                Ok(out)
            }
            Expr::Ident(name, _, _) => {
                if let Some(temp) = f.lookup(name) {
                    return Ok(temp);
                }
                // A record type with no fields IS a value: `type unit` names one
                // thing and naming it builds it. A subtype and a typeset also
                // carry no fields, and neither is that — a subtype's name takes
                // one argument, and a typeset never constructs at all. Both
                // reached this arm and were emitted as nullary records, so
                // `print "{age}"` for `type age int` printed `<mod>/age` where
                // the oracle prints `<fn>` and the page refuses the name. It
                // falls through to the bare-value refusal below now, which is
                // what native already said for a record type that HAS fields.
                let nullary_record = |t: &crate::ast::TypeDecl| {
                    t.name == *name
                        && t.fields.is_empty()
                        && t.parent.is_none()
                        && t.members.is_empty()
                };
                if self.program.types.iter().any(nullary_record) {
                    let id = self.type_ids[name.as_str()];
                    let arr = f.tmp();
                    f.line_fmt(format_args!("{arr} = alloca [1 x %KValue]"));
                    let t = f.tmp();
                    f.line_fmt(format_args!(
                        "{t} = call %KValue @k_rec(i64 {id}, i64 0, ptr {arr})"
                    ));
                    f.record(&t, REC);
                    return Ok(t.to_string());
                }
                if self.declares_at(name, 0) {
                    if let Some(n) = self.literal_word_constant(name) {
                        return Ok(format!("{{ i64 0, i64 {n} }}"));
                    }
                    let callee_ret = self.ret_ty(name, 0);
                    let t = f.tmp();
                    f.line_fmt(format_args!("{t} = call tailcc {callee_ret} @{}()", dsym(name, 0)));
                    // A group that returns its record in registers hands back a
                    // %parsed, and everything downstream of a constant reads a
                    // %KValue: the failure guard, the return, the field read.
                    // Box it here, once, rather than teaching each consumer the
                    // other shape — the saving the register return buys is on
                    // the callee's side, and a constant is evaluated once.
                    let t = match self.escape.returns_ty(name, 0) {
                        Some(ty) if callee_ret == "%parsed" => {
                            f.record_parsed(&t, ty, self.type_ids[ty]);
                            self.as_value(f, &t)
                        }
                        _ => t.to_string(),
                    };
                    f.record(&t, self.group_return_set(name, 0));
                    return Ok(t);
                }
                let arities: Vec<usize> = {
                    let mut seen = Vec::new();
                    for d in self.decls_named(name) {
                        if !seen.contains(&d.params.len()) {
                            seen.push(d.params.len());
                        }
                    }
                    seen
                };
                if arities.len() == 1 && arities[0] >= 1 && self.simple_fn_value(name, arities[0]) {
                    let arity = arities[0];
                    self.fn_value_wrappers.push((name.to_string(), arity));
                    let t = f.tmp();
                    f.line_fmt(format_args!(
                        "{t} = call %KValue @k_fnref(ptr @{})",
                        rsym(name, arity)
                    ));
                    return Ok(t.to_string());
                }
                if !arities.is_empty() {
                    return Err(format!(
                        "native backend: `{name}` cannot be used as a function value \
                         (only functions of one or more arguments over plain values are supported)"
                    ));
                }
                let bare = name.strip_prefix("builtin_").unwrap_or(name.as_str());
                // A builtin is a function too, and `apply length "ab"` hands it
                // over the same way a declared group is handed over. The
                // interpreter calls it; the wrapper below is what lets a
                // compiled `k_callN` reach the same C entry.
                if let Some(arity) = arity_of_emitted(bare) {
                    if (1..=4).contains(&arity) {
                        self.builtin_value_wrappers.push((bare.to_string(), arity));
                        let t = f.tmp();
                        let sym = rsym(&format!("builtin.{bare}"), arity);
                        f.line_fmt(format_args!("{t} = call %KValue @k_fnref(ptr @{sym})"));
                        return Ok(t.to_string());
                    }
                }
                // `print` is not one of them: its argument reaches a user's
                // `render/to_string` arm, and a call site picks that path from
                // the argument's set. Handed over, there is no set to read, so
                // its wrapper carries the choice into the run.
                if bare == "print" {
                    self.print_value_wrapper = true;
                    let t = f.tmp();
                    let sym = rsym("builtin.print", 1);
                    f.line_fmt(format_args!("{t} = call %KValue @k_fnref(ptr @{sym})"));
                    return Ok(t.to_string());
                }
                match bare {
                    "true" => Ok("{ i64 2, i64 0 }".to_string()),
                    "false" => Ok("{ i64 3, i64 0 }".to_string()),
                    "none" => Ok("{ i64 4, i64 0 }".to_string()),
                    "done" => Ok(format!("{{ i64 {K_DONE}, i64 0 }}")),
                    "args" => {
                        let t = f.tmp();
                        f.line_fmt(format_args!("{t} = call %KValue @k_desc_args()"));
                        f.record(&t, DESC);
                        Ok(t.to_string())
                    }
                    "stdin" => {
                        let t = f.tmp();
                        f.line_fmt(format_args!("{t} = call %KValue @k_desc_stdin()"));
                        f.record(&t, DESC);
                        Ok(t.to_string())
                    }
                    "read_line" => {
                        let t = f.tmp();
                        f.line_fmt(format_args!("{t} = call %KValue @k_desc_read_line()"));
                        f.record(&t, DESC);
                        Ok(t.to_string())
                    }
                    "now" => {
                        let t = f.tmp();
                        f.line_fmt(format_args!("{t} = call %KValue @k_desc_now()"));
                        f.record(&t, DESC);
                        Ok(t.to_string())
                    }
                    _ => {
                        // A record constructor handed over as a function is
                        // the lambda that calls it, which the interpreter
                        // already treats it as (lox F20).
                        let fields = self
                            .program
                            .types
                            .iter()
                            .find(|t| t.name == name.as_str() && t.members.is_empty())
                            .map(|t| t.fields.len());
                        match fields {
                            Some(n @ 1..=4) => {
                                let span = expr.span();
                                let params: Vec<(String, Span)> =
                                    (0..n).map(|i| (format!("__field{i}"), span)).collect();
                                let args = params
                                    .iter()
                                    .map(|(p, _)| {
                                        Expr::Ident(
                                            Name::new(p),
                                            span,
                                            crate::ast::Resolution::default(),
                                        )
                                    })
                                    .collect();
                                let lambda = Expr::Lambda {
                                    params,
                                    body: Box::new(Expr::App {
                                        head: Box::new(expr.clone()),
                                        args,
                                        span,
                                        piped: false,
                                    }),
                                    span,
                                };
                                self.emit_expr(f, &lambda)
                            }
                            _ => Err(format!(
                                "native backend: `{name}` as a bare value is not yet supported"
                            )),
                        }
                    }
                }
            }
            Expr::App { head, args, piped, span } => {
                if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == crate::ahead::SLEEP_POOLED)
                {
                    return self.emit_pooled_sleep(f, args);
                }
                // `&f a` supplies a and waits; the arity it waits for is the
                // one the gavel names — supplied plus holes picks the group
                // `(&roll 4) 5` is one application seen whole: the partial and
                // the arguments finishing it are both here, so it lowers to the
                // call it means. Dispatch then happens on the total count, which
                // is what the oracle does and what makes `(&roll 4) 5 6` reach
                // the three-argument arm rather than any arm chosen at the `&`.
                // A partial over a VALUE is not flattened: its arity is
                // the callee's at run time, so the held arguments build the
                // runtime partial and the rest reach it through `k_call`,
                // which is where the count is settled.
                if let Expr::App { head: inner, args: held, .. } = head.as_ref() {
                    if let Expr::Partial(name, nspan) = inner.as_ref() {
                        if self.declared(name) {
                            return self
                                .emit_finished_partial(f, name, *nspan, held, args, *piped, *span);
                        }
                    }
                }
                if let Expr::Partial(name, _) = head.as_ref() {
                    if !self.declared(name) {
                        return self.emit_partial_value(f, name, args, *span);
                    }
                    return self.emit_declared_partial(f, name, args, *span);
                }
                let saved = match (head.as_ref(), piped) {
                    (Expr::Ident(name, _, _), false) => {
                        self.entering_deep(f, name.as_str(), args.len())
                    }
                    _ => None,
                };
                let called = self.emit_call_full(f, head, args, *piped, *span)?;
                self.left_deep(f, saved);
                Ok(called)
            }
            Expr::Field { base, name, span } => {
                let b = self.emit_expr(f, base)?;
                let (label, _) = self.intern(&format!("{name}\0"));
                let t = f.tmp();
                self.stamp_site(f, *span);
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_b_field(%KValue {b}, ptr @{label})"
                ));
                f.record(&t, TOP);
                Ok(t.to_string())
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.emit_expr(f, base)?;
                let container = self.maybe_force(f, container);
                let key = self.emit_expr(f, index)?;
                let key = self.maybe_force(f, key);
                let read = self.emit_at(f, &container, &key, *strict, *span);
                if !*strict {
                    return Ok(read);
                }
                // the sigil is the choice of channel (ruled 2026-09-16): the
                // element, or the missing-index err, settles into a box
                let boxed = f.tmp();
                f.line_fmt(format_args!("{boxed} = call %KValue @k_b_effect(%KValue {read})"));
                f.record(&boxed, DESC | ERR);
                Ok(boxed.to_string())
            }
            Expr::Join { lhs, rhs, span } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let t = f.tmp();
                self.stamp_site(f, *span);
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_desc_join(%KValue {a}, %KValue {b})"
                ));
                f.record(&t, (f.set_of(&a) & FAIL) | (f.set_of(&b) & FAIL) | DESC | ERR);
                Ok(t.to_string())
            }
            Expr::BinOp { op, lhs, rhs, span } => {
                if word_arith(op) && arith_ops(expr) >= 2 {
                    return self.emit_word_run(f, expr);
                }
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                self.emit_binop(f, op, &a, &b, *span)
            }
            Expr::Lambda { params, body, .. } => {
                // No lower bound: `&add 1 2` supplies an arm's last argument
                // and the value it leaves waits to be called, which is a
                // closure of no parameters.
                if params.len() > 4 {
                    return Err("native backend: a lambda takes at most 4 parameters".to_string());
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
                // A lambda that captures nothing is the same value every
                // evaluation, so it builds once into a permanent slot -- the
                // arrangement `str_const` already uses for a string literal.
                // The alloca, the two allocations and the memcpy go away and
                // the site is a load after the first visit.
                if captures.is_empty() {
                    let cell = format!("{lifted}_cell");
                    // the ccc wrapper, never the tailcc fn: C calls this pointer
                    self.closure_consts.push((cell.clone(), format!("w_{lifted}"), params.len()));
                    let t = f.tmp();
                    f.line_fmt(format_args!("{t} = load %KValue, ptr @{cell}"));
                    return Ok(t.to_string());
                }
                let n = captures.len();
                let arr = f.tmp();
                f.line_fmt(format_args!("{arr} = alloca [{n} x %KValue]"));
                for (i, cap) in captures.iter().enumerate() {
                    let temp = f.lookup(cap).expect("capture is bound");
                    let temp = self.as_value(f, &temp);
                    let slot = f.tmp();
                    f.line_fmt(format_args!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line_fmt(format_args!("store %KValue {temp}, ptr {slot}"));
                }
                let t = f.tmp();
                // the ccc wrapper, never the tailcc fn: C calls this pointer
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_closure(ptr @w_{lifted}, i64 {}, i64 {}, ptr {arr}, ptr @{lifted})",
                    params.len(),
                    captures.len()
                ));
                Ok(t.to_string())
            }
            Expr::List(items, _) if items.is_empty() => {
                let t = f.call_pair(format_args!("@k_list_empty()"));
                f.record(&t, LIST);
                Ok(t.to_string())
            }
            Expr::MapLit(pairs, _) if pairs.is_empty() => {
                let t = f.call_pair(format_args!("@k_map_empty()"));
                f.record(&t, MAP);
                Ok(t.to_string())
            }
            Expr::List(items, _) => {
                let mut emitted = Vec::new();
                for item in items {
                    let e = self.deferred_or_emitted(f, item)?;
                    emitted.push(self.as_value(f, &e));
                }
                let n = emitted.len().max(1);
                let arr = f.tmp();
                f.line_fmt(format_args!("{arr} = alloca [{n} x %KValue]"));
                for (i, value) in emitted.iter().enumerate() {
                    let slot = f.tmp();
                    f.line_fmt(format_args!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line_fmt(format_args!("store %KValue {value}, ptr {slot}"));
                }
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_list_lit(i64 {}, ptr {arr})",
                    emitted.len()
                ));
                f.record(&t, LIST);
                Ok(t.to_string())
            }
            Expr::MapLit(pairs, _) => {
                let mut emitted = Vec::new();
                for (key, value) in pairs {
                    let k = self.emit_expr(f, key)?;
                    emitted.push(self.as_value(f, &k));
                    let v = self.emit_expr(f, value)?;
                    emitted.push(self.as_value(f, &v));
                }
                let n = emitted.len().max(1);
                let arr = f.tmp();
                f.line_fmt(format_args!("{arr} = alloca [{n} x %KValue]"));
                for (i, value) in emitted.iter().enumerate() {
                    let slot = f.tmp();
                    f.line_fmt(format_args!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line_fmt(format_args!("store %KValue {value}, ptr {slot}"));
                }
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_map_lit(i64 {}, ptr {arr})",
                    pairs.len()
                ));
                f.record(&t, MAP);
                Ok(t.to_string())
            }
        }
    }

    /// Emit an expression in tail position: direct calls to kanso functions
    /// become guaranteed tail calls, and an if's branches stay tails.
    /// Before a call that enters a cycle growing the stack from outside it:
    /// the record an overflow prints is saved, and set to the cycle's place
    /// when it is empty (see `crate::deep`). Answers the saved record, which
    /// `left_deep` puts back once the call returns.
    fn entering_deep(&mut self, f: &mut FnEmit, name: &str, arity: usize) -> Option<String> {
        let caller = (f.deep_from.0.as_str(), f.deep_from.1);
        let (decl, span) = self.deep.entering(Some(caller), (name, arity))?;
        let d = &self.program.fns[decl];
        let hako = crate::provenance::package_of(&d.file);
        let spelled = crate::deep::spelled(self.program, (decl, span));
        let (lit, _) = self.intern(&format!("{hako}\0{spelled}\0"));
        let saved = f.tmp();
        f.line_fmt(format_args!("{saved} = load ptr, ptr @k_deep_site"));
        let empty = f.tmp();
        f.line_fmt(format_args!("{empty} = icmp eq ptr {saved}, null"));
        let set = f.tmp();
        f.line_fmt(format_args!("{set} = select i1 {empty}, ptr @{lit}, ptr {saved}"));
        f.line_fmt(format_args!("store ptr {set}, ptr @k_deep_site"));
        Some(saved.to_string())
    }

    fn left_deep(&mut self, f: &mut FnEmit, saved: Option<String>) {
        if let Some(saved) = saved {
            f.line_fmt(format_args!("store ptr {saved}, ptr @k_deep_site"));
        }
    }

    /// Whether a call from the function being emitted, naming `name`, crosses
    /// from the program's own files into the library's. A flattened twin is
    /// not a crossing: it carries the program's own continuation, so a call
    /// to it can be a step of the program's loop and keeps its jump.
    fn crosses_into_library(&self, f: &FnEmit, name: &str) -> bool {
        !crate::flat::made(name)
            && !crate::provenance::is_library_file(&f.file)
            && self
                .group_named(name)
                .first()
                .is_some_and(|&i| crate::provenance::is_library_file(&self.program.fns[i].file))
    }

    /// Before a call from the program into the library: the line a failure
    /// inside the library names as its caller is saved, and set to this
    /// call's (ruled on #2099). Answers the saved one, which `left_library`
    /// puts back once the call returns, so a program function the library
    /// calls back leaves the outer line in place when it is done.
    fn entering_library(&mut self, f: &mut FnEmit, name: &str, span: Span) -> Option<String> {
        if !self.crosses_into_library(f, name) {
            return None;
        }
        let (lit, _) = self.intern(&format!("{}:{}\0", f.file, span.line));
        let saved = f.tmp();
        f.line_fmt(format_args!("{saved} = load ptr, ptr @k_from"));
        f.line_fmt(format_args!("store ptr @{lit}, ptr @k_from"));
        Some(saved.to_string())
    }

    fn left_library(&mut self, f: &mut FnEmit, saved: Option<String>) {
        if let Some(saved) = saved {
            f.line_fmt(format_args!("store ptr {saved}, ptr @k_from"));
        }
    }

    /// The group a call names, when it names one by name with its arity.
    fn named_call(expr: &Expr) -> Option<(&str, usize)> {
        match expr {
            Expr::App { head, args, piped: false, .. } => match head.as_ref() {
                Expr::Ident(name, _, _) => Some((name.as_str(), args.len())),
                _ => None,
            },
            _ => None,
        }
    }

    fn emit_tail(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<(), String> {
        if let Expr::Guard { cond, early, rest, .. } = expr {
            // A guard is a tail `if` whose else is the rest of the body, so
            // its condition is asked as a question the same way: a
            // comparison branches on its own icmp and an `or` branches arm by
            // arm. Read for a value, `length xs == 0` in json's encode_list
            // became a tagged boolean that `k_truthy` was then asked about.
            // A failing condition still returns itself: with no merge label
            // `emit_cond` emits the return.
            let early_label = f.label();
            let rest_label = f.label();
            self.emit_cond(f, cond, &early_label, &rest_label, None)?;
            f.start_block(&early_label);
            self.emit_tail(f, early)?;
            f.start_block(&rest_label);
            self.emit_fn_body(f, rest)?;
            return Ok(());
        }
        if let Some((name, n)) = Self::named_call(expr) {
            let caller = (f.deep_from.0.as_str(), f.deep_from.1);
            if self.deep.entering(Some(caller), (name, n)).is_some() {
                // a lambda keeps its jump: it can be a step of a loop the
                // cycle runs through it, so it records and puts nothing back
                if f.lifted {
                    let _ = self.entering_deep(f, name, n);
                } else {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
            }
        }
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(name, _, _) = head.as_ref() {
                let bare = name.strip_prefix("builtin_").unwrap_or(name);
                // an insisting form is called rather than jumped to, so its
                // answer can be pointed at this line on the way back; a call
                // into the library likewise, so the line it was called from
                // is put back when it returns (#2099)
                if self.forwarders.contains(bare, args.len())
                    || self.forwarders.contains(name, args.len())
                    || name.ends_with('!')
                    || self.crosses_into_library(f, name)
                {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
            }
        }
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
                        f.line_fmt(format_args!("{is_desc} = icmp eq i64 {tag}, 8"));
                        let desc_path = f.label();
                        let check = f.label();
                        f.line_fmt(format_args!(
                            "br i1 {is_desc}, label %{desc_path}, label %{check}"
                        ));
                        f.start_block(&desc_path);
                        let t = f.tmp();
                        let closure = self.emit_expr(f, head)?;
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_maybe_bind(%KValue {value}, %KValue {closure})"
                        ));
                        f.record(&t, TOP);
                        self.emit_ret(f, &t);
                        f.start_block(&check);
                        let ok = inline_not_failure(f, &value);
                        let bail = f.label();
                        let cont = f.label();
                        f.line_fmt(format_args!("br i1 {ok}, label %{cont}, label %{bail}"));
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
            if let Expr::Ident(name, _, _) = &**head {
                if name == "if" && f.lookup(name).is_none() {
                    if let Some(t) = self.emit_byte_run(f, args)? {
                        self.emit_ret(f, &t);
                        return Ok(());
                    }
                    // In tail position a failing condition returns rather than
                    // joining a phi, which is the whole difference from the
                    // value form; `emit_cond` takes no merge label and emits
                    // the return itself.
                    let then_label = f.label();
                    let else_label = f.label();
                    self.emit_cond(f, &args[0], &then_label, &else_label, None)?;
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
                let target = (name.to_string(), args.len());
                let outside_cluster = self.beat.ids.contains_key(&target)
                    && !self.beat.same_cluster(&target, &(f.group.clone(), f.arity));
                let region = matches!(expr, Expr::App { span, .. }
                    if self.beat.regions.contains(&(f.file.clone(), span.line as usize, span.col as usize)));
                if outside_cluster
                    || region
                    || self.beat.demoted.contains(&((f.group.clone(), f.arity), target))
                {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
                // the arity has to match a declaration: `d_{name}_{n}` for an
                // n nothing declares is a symbol the module never defines
                let is_program_fn = f.lookup(name).is_none()
                    && !self.type_ids.contains_key(name.as_str())
                    && name != "err"
                    && name != "print"
                    && self
                        .program
                        .fns
                        .iter()
                        .any(|d| d.name == *name && d.params.len() == args.len());
                if is_program_fn {
                    let n = args.len();
                    let mut emitted = Vec::new();
                    let mut packed: Vec<Option<String>> = Vec::new();
                    let word_args =
                        self.ret_ty(name, n) == f.ret_ty && self.twin_reachable(f, name, n);
                    let word_positions = match word_args {
                        true => self.twin_positions(name, n),
                        false => Vec::new(),
                    };
                    let mut deferred: Vec<(usize, Vec<String>)> = Vec::new();
                    let mut overflow: Vec<String> = Vec::new();
                    let entries = self.both_entries(f, args);
                    let mut value_of_the_entry: Option<String> = None;
                    for (i, arg) in args.iter().enumerate() {
                        if entries == Some(i) {
                            let (k, v) = self.emit_entry_pair(f, arg)?;
                            value_of_the_entry = Some(v);
                            emitted.push(k);
                            packed.push(None);
                            continue;
                        }
                        if entries.map(|k| k + 1) == Some(i) {
                            emitted.push(value_of_the_entry.take().expect("the key came first"));
                            packed.push(None);
                            continue;
                        }
                        match self.packed_arg_fields(name, n, i, arg) {
                            Some(fields) => {
                                let fields = fields.to_vec();
                                let ty = self
                                    .escape
                                    .carries_ty(name, n, i)
                                    .expect("a packed argument fills a carried slot")
                                    .to_string();
                                let p = self.emit_packed_arg(f, &fields, &ty)?;
                                emitted.push(String::new());
                                packed.push(Some(p));
                            }
                            None if word_positions.contains(&i) && arith_ops(arg) >= 1 => {
                                let (v, raw) = self.emit_word_arg(f, arg)?;
                                if let Some((vals, flags)) = raw {
                                    deferred.push((i, vals));
                                    overflow.extend(flags);
                                }
                                emitted.push(v);
                                packed.push(None);
                            }
                            None => {
                                emitted.push(self.emit_expr(f, arg)?);
                                packed.push(None);
                            }
                        }
                    }
                    let callee_ret = self.ret_ty(name, n);
                    let same_ret = callee_ret == f.ret_ty;
                    // a header in this frame must outlive the callee's
                    // reads of it, so the call keeps the frame
                    let kind = match f.frame_held {
                        true => "call",
                        false => "musttail call",
                    };
                    // Which body the call reaches is asked before the rewind.
                    // On the way to the twin every word position holds a word,
                    // which crosses a rewind as it is. The way to the general
                    // group is taken when a bignum has arrived, and that one
                    // edge skips its rewind: the general group's next edge
                    // lifts its bignums and takes this iteration's garbage
                    // with its own.
                    let mut twin = match same_ret {
                        true => self.twin_test(f, name, n, &emitted, &packed),
                        false => None,
                    };
                    if let Some(first) = overflow.first().cloned() {
                        let mut any = first;
                        for flag in &overflow[1..] {
                            let t = f.tmp();
                            f.line_fmt(format_args!("{t} = or i1 {any}, {flag}"));
                            any = t.to_string();
                        }
                        match &mut twin {
                            Some(cond) => {
                                let none = f.tmp();
                                f.line_fmt(format_args!("{none} = xor i1 {any}, true"));
                                *cond = Some(match cond.take() {
                                    None => none.to_string(),
                                    Some(c) => {
                                        let both = f.tmp();
                                        f.line_fmt(format_args!("{both} = and i1 {c}, {none}"));
                                        both.to_string()
                                    }
                                });
                            }
                            None => {
                                // no twin after all: the words go back into
                                // values, the general way where one overflowed
                                let fast = f.label();
                                let slow = f.label();
                                let merge = f.label();
                                f.line_fmt(format_args!(
                                    "br i1 {any}, label %{slow}, label %{fast}"
                                ));
                                f.start_block(&slow);
                                let mut wide = Vec::new();
                                for (i, vals) in &deferred {
                                    wide.push(self.emit_arith_general(f, &args[*i], vals, &mut 0)?);
                                }
                                let slow_from = f.cur_label.clone();
                                f.line_fmt(format_args!("br label %{merge}"));
                                f.start_block(&fast);
                                f.line_fmt(format_args!("br label %{merge}"));
                                f.start_block(&merge);
                                for ((i, _), w) in deferred.iter().zip(wide) {
                                    let t = f.tmp();
                                    f.line_fmt(format_args!(
                                        "{t} = phi %KValue [ {}, %{fast} ], [ {w}, %{slow_from} ]",
                                        emitted[*i]
                                    ));
                                    f.record(&t, INT | f.set_of(&w));
                                    emitted[*i] = t.to_string();
                                }
                                deferred.clear();
                            }
                        }
                    }
                    if let Some(Some(words)) = &twin {
                        let to_twin = f.label();
                        let to_general = f.label();
                        f.line_fmt(format_args!(
                            "br i1 {words}, label %{to_twin}, label %{to_general}"
                        ));
                        f.start_block(&to_general);
                        // an argument that overflowed is worked out again,
                        // the general way, from the same operands
                        let mut general = emitted.clone();
                        for (i, vals) in &deferred {
                            general[*i] = self.emit_arith_general(f, &args[*i], vals, &mut 0)?;
                        }
                        let general_ir: Vec<String> = general
                            .iter()
                            .enumerate()
                            .map(|(i, e)| match &packed[i] {
                                Some(p) => format!("%parsed {p}"),
                                None => self.call_arg(f, name, n, i, e, args.get(i)),
                            })
                            .collect();
                        self.settle_cells(f, &general_ir);
                        // The way to the general group runs once a bignum
                        // has arrived. Inlined, it is the general body copied
                        // into the twin, which then costs too much to fold
                        // into its own caller: json/obj_key_end stood alone
                        // on the run program until this said noinline.
                        let g = f.tmp();
                        f.line_fmt(format_args!(
                            "{g} = {kind} tailcc {callee_ret} @{}({}) #90",
                            dsym(name, n),
                            general_ir.join(", ")
                        ));
                        f.line_fmt(format_args!("ret {callee_ret} {g}"));
                        f.start_block(&to_twin);
                    }
                    let words_at: Vec<usize> = match twin {
                        Some(_) => self.twin_positions(name, n),
                        None => Vec::new(),
                    };
                    if same_ret
                        && self
                            .beat
                            .same_cluster(&(name.to_string(), n), &(f.group.clone(), f.arity))
                    {
                        match self.beat.carried.get(&(name.to_string(), n)) {
                            Some(positions) => {
                                // evacuate the loop-varying arguments through
                                // the carry buffers, then rewind — before the
                                // ABI conversion below, so the call passes
                                // the evacuated values. An argument the carry
                                // does not take that may be a bignum is lifted
                                // around it: it was built in the arena this
                                // rewind hands back.
                                let lifted: Vec<usize> = (0..emitted.len())
                                    .filter(|j| {
                                        !positions.contains(j)
                                            && !words_at.contains(j)
                                            && packed[*j].is_none()
                                            && f.set_of(&emitted[*j]) & infer::BIG != 0
                                    })
                                    .collect();
                                let stages: Vec<&str> = positions
                                    .iter()
                                    .map(|&j| {
                                        // The accumulator crosses by identity:
                                        // the copy strips the room it was
                                        // seeded with, and the next join would
                                        // re-seed. Only a slot builder_params
                                        // names is kept, so nothing that merely
                                        // has capacity is aliased.
                                        match self.builder_params.contains(name, n, j) {
                                            true => "k_carry_stage_kept",
                                            false => "k_carry_stage",
                                        }
                                    })
                                    .collect();
                                let mut carry = |f: &mut FnEmit, emitted: &mut [String]| {
                                    f.line("call void @k_carry_reset()");
                                    for (&j, stage) in positions.iter().zip(&stages) {
                                        f.line_fmt(format_args!(
                                            "call void @{stage}(%KValue {})",
                                            emitted[j]
                                        ));
                                    }
                                    f.line("call void @k_beat_lap_carry()");
                                    for (slot, &j) in positions.iter().enumerate() {
                                        let t = f.tmp();
                                        f.line_fmt(format_args!(
                                            "{t} = call %KValue @k_carry_take(i64 {slot})"
                                        ));
                                        // the copy is the value it was handed
                                        let set = f.set_of(&emitted[j]);
                                        f.record(&t, set);
                                        emitted[j] = t.to_string();
                                    }
                                };
                                let failing: Vec<usize> = (0..emitted.len())
                                    .filter(|j| {
                                        !positions.contains(j)
                                            && !words_at.contains(j)
                                            && packed[*j].is_none()
                                            && f.set_of(&emitted[*j]) & infer::FAIL != 0
                                    })
                                    .collect();
                                unless_failing(f, &mut emitted, &failing, &mut |f, emitted| {
                                    lift_around(f, emitted, &lifted, &mut carry)
                                });
                            }
                            None => {
                                // everything this iteration allocated is
                                // dead; rewind to the entry mark
                                let edge = ((f.group.clone(), f.arity), (name.to_string(), n));
                                // a widening loop's word twin carries words
                                // and nothing else, so it has nothing to hand
                                // back; only the general body, which a bignum
                                // reaches, rewinds
                                let words_only = self.beat.widening.contains(&edge.0)
                                    && self.emitting_twin(&f.group, f.arity);
                                if self.beat.rewind.contains(&edge) && !words_only {
                                    let lifted: Vec<usize> = (0..emitted.len())
                                        .filter(|&j| {
                                            packed[j].is_none()
                                                && !words_at.contains(&j)
                                                && f.set_of(&emitted[j]) & infer::BIG != 0
                                        })
                                        .collect();
                                    let failing: Vec<usize> = (0..emitted.len())
                                        .filter(|&j| {
                                            packed[j].is_none()
                                                && !words_at.contains(&j)
                                                && f.set_of(&emitted[j]) & infer::FAIL != 0
                                        })
                                        .collect();
                                    unless_failing(f, &mut emitted, &failing, &mut |f, emitted| {
                                        lift_around(f, emitted, &lifted, &mut |f, _| {
                                            f.line("call void @k_beat_iter()")
                                        })
                                    });
                                }
                            }
                        }
                    }
                    let on_twin = twin.is_some();
                    let args_ir: Vec<String> = emitted
                        .iter()
                        .enumerate()
                        .map(|(i, e)| match &packed[i] {
                            Some(p) => format!("%parsed {p}"),
                            None => self.call_arg_to(f, name, n, i, e, args.get(i), on_twin),
                        })
                        .collect();
                    let t = f.tmp();
                    if self.can_miss_with(f, name, n, &emitted) {
                        self.stamp_site(f, expr.span());
                    }
                    if same_ret {
                        self.settle_cells(f, &args_ir);
                        let sym = match on_twin {
                            true => tsym(name, n),
                            false => dsym(name, n),
                        };
                        f.line_fmt(format_args!(
                            "{t} = {kind} tailcc {callee_ret} @{sym}({})",
                            args_ir.join(", ")
                        ));
                        f.line_fmt(format_args!("ret {callee_ret} {t}"));
                    } else {
                        // A %parsed function tail-calling a KValue failure helper:
                        // can't musttail across the type change, so call and wrap.
                        // The other way round is a thunk site, a `%KValue` function
                        // made outside the analysis that matches a tail caller's
                        // shape to its callee's, calling a group that returns its
                        // record in registers: emit_ret boxes it.
                        f.line_fmt(format_args!(
                            "{t} = call tailcc {callee_ret} @{}({})",
                            dsym(name, n),
                            args_ir.join(", ")
                        ));
                        if let Some(ty) = self.escape.returns_ty(name, n) {
                            if callee_ret == "%parsed" {
                                f.record_parsed(&t, ty, self.type_ids[ty]);
                            }
                        }
                        self.emit_ret(f, &t);
                    }
                    return Ok(());
                }
            }
        }
        if self.emit_value_tail_call(f, expr)? {
            return Ok(());
        }
        let value = self.emit_expr(f, expr)?;
        self.emit_ret(f, &value);
        Ok(())
    }

    /// A call to a bound function value in tail position, as `then (acc + i)`
    /// is in a loop written in continuation-passing style: the frame goes,
    /// and `k_tcall{n}` jumps to the lambda. Only where the plain tail would be
    /// the call and its return with nothing between them, so a frame that
    /// still holds a header or a lazy cell keeps the ordinary call.
    fn emit_value_tail_call(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<bool, String> {
        let Expr::App { head, args, piped: false, .. } = expr else {
            return Ok(false);
        };
        let Expr::Ident(name, _, _) = head.as_ref() else {
            return Ok(false);
        };
        if f.lookup(name).is_none()
            || f.frame_held
            || !f.lazy_cells.is_empty()
            || f.ret_ty != "%KValue"
            || args.len() > 4
        {
            return Ok(false);
        }
        let callee = self.emit_expr(f, head)?;
        let mut args_ir = Vec::with_capacity(args.len());
        for a in args {
            let v = self.emit_expr(f, a)?;
            let v = self.as_value(f, &v);
            args_ir.push(format!(", %KValue {v}"));
        }
        let callee = self.as_value(f, &callee);
        let n = args.len();
        let t = f.tmp();
        self.stamp_site(f, expr.span());
        f.line_fmt(format_args!(
            "{t} = musttail call tailcc %KValue @k_tcall{n}(%KValue {callee}{})",
            args_ir.concat()
        ));
        f.line_fmt(format_args!("ret %KValue {t}"));
        Ok(true)
    }

    /// `x[p + k] == c and x[p + k'] == c' ...` over one bytes value and one
    /// int, read as one range test and plain byte compares.
    ///
    /// The json decoder matches `true`, `false` and `null` this way, and each
    /// read in the chain paid for itself: an overflow check on `p + k`, a test
    /// of each end of the range, a none-or-byte merge and the compare, about
    /// fourteen instructions a byte and 612,500 literals a run on the run
    /// program. Every read here sits between `p + kmin` and `p + kmax`, so
    /// when that window lies inside the bytes no read can miss and no sum can
    /// overflow, and the answer is the bytes compared. Outside the window at
    /// least one read is none, which compares false, but the general path is
    /// kept for it rather than written as `false`: it is also the path a sum
    /// that would overflow takes, and that one widens into a bignum.
    fn emit_byte_run(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<Option<String>, String> {
        let Some((x, p, reads)) = byte_run(args) else { return Ok(None) };
        if f.lookup("false").is_some() || f.lookup("if").is_some() {
            return Ok(None);
        }
        let (Some(xv), Some(pv)) = (f.lookup(x), f.lookup(p)) else { return Ok(None) };
        // A position that may have grown past the word is tested for its tag
        // beside the bounds, and a bignum takes the general arm.
        let p_set = f.set_of(&pv);
        if f.set_of(&xv) != BYTES || (p_set != INT && p_set != INT | infer::BIG) {
            return Ok(None);
        }
        let kmin = reads.iter().map(|r| r.0).min().unwrap_or(0);
        let kmax = reads.iter().map(|r| r.0).max().unwrap_or(0);
        let bp = inline_payload(f, &xv);
        let bptr = f.tmp();
        f.line_fmt(format_args!("{bptr} = inttoptr i64 {bp} to ptr"));
        let len_ptr = f.tmp();
        f.line_fmt(format_args!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
        let len = f.tmp();
        f.line_fmt(format_args!("{len} = load i64, ptr {len_ptr}"));
        let idx = inline_payload(f, &pv);
        let lo = f.tmp();
        f.line_fmt(format_args!("{lo} = icmp sge i64 {idx}, {}", 1 - kmin));
        let top = f.tmp();
        f.line_fmt(format_args!("{top} = sub i64 {len}, {kmax}"));
        let hi = f.tmp();
        f.line_fmt(format_args!("{hi} = icmp sle i64 {idx}, {top}"));
        let mut inside = f.tmp();
        f.line_fmt(format_args!("{inside} = and i1 {lo}, {hi}"));
        if p_set != INT {
            let tag = inline_tag(f, &pv);
            let word = f.tmp();
            f.line_fmt(format_args!("{word} = icmp eq i64 {tag}, 0"));
            let both = f.tmp();
            f.line_fmt(format_args!("{both} = and i1 {inside}, {word}"));
            inside = both;
        }
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line_fmt(format_args!("br i1 {inside}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let data_ptr = f.tmp();
        f.line_fmt(format_args!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
        let data = f.tmp();
        f.line_fmt(format_args!("{data} = load ptr, ptr {data_ptr}"));
        let mut all = "true".to_string();
        for (k, c) in &reads {
            let off = f.tmp();
            f.line_fmt(format_args!("{off} = add i64 {idx}, {}", k - 1));
            let at = f.tmp();
            f.line_fmt(format_args!("{at} = getelementptr i8, ptr {data}, i64 {off}"));
            let byte = f.tmp();
            f.line_fmt(format_args!("{byte} = load i8, ptr {at}"));
            let same = f.tmp();
            f.line_fmt(format_args!("{same} = icmp eq i8 {byte}, {}", *c as u8 as i8));
            let both = f.tmp();
            f.line_fmt(format_args!("{both} = and i1 {all}, {same}"));
            all = both.to_string();
        }
        let tag = f.tmp();
        f.line_fmt(format_args!("{tag} = select i1 {all}, i64 2, i64 3"));
        let hit = f.tmp();
        f.line_fmt(format_args!(
            "{hit} = insertvalue %KValue {{ i64 undef, i64 0 }}, i64 {tag}, 0"
        ));
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&slow);
        let general = self.emit_if_value(f, args)?;
        let general = self.as_value(f, &general);
        let slow_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = phi %KValue [ {hit}, %{fast} ], [ {general}, %{slow_from} ]"
        ));
        f.record(&t, infer::BOOL | f.set_of(&general));
        Ok(Some(t.to_string()))
    }

    /// Statements read for the value of the last one.
    fn emit_stmts_value(&mut self, f: &mut FnEmit, stmts: &[Stmt]) -> Result<String, String> {
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

    /// A guard read for its value: the early answer when the condition
    /// holds, the rest of the block when it does not, and a phi over the two
    /// and any failure the condition carried out, as an `if` has.
    fn emit_guard_value(
        &mut self,
        f: &mut FnEmit,
        cond: &Expr,
        early: &Expr,
        rest: &[Stmt],
    ) -> Result<String, String> {
        let early_label = f.label();
        let rest_label = f.label();
        let merge = f.label();
        let asked = self.emit_cond(f, cond, &early_label, &rest_label, Some(&merge))?;
        f.start_block(&early_label);
        let early_value = self.emit_expr(f, early)?;
        let early_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&rest_label);
        let rest_value = self.emit_stmts_value(f, rest)?;
        let rest_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let mut arms = vec![
            format!("[ {early_value}, %{early_from} ]"),
            format!("[ {rest_value}, %{rest_from} ]"),
        ];
        let mut fail_set = 0;
        for (v, from) in &asked.failed {
            arms.push(format!("[ {v}, %{from} ]"));
            fail_set |= f.set_of(v) & FAIL;
        }
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = phi %KValue {}", arms.join(", ")));
        f.record(&t, f.set_of(&early_value) | f.set_of(&rest_value) | fail_set);
        Ok(t.to_string())
    }

    /// An `if` read for its value: the condition asked as a question, each arm
    /// emitted as a value, and a phi over the arms and any failure the
    /// condition carried out.
    fn emit_if_value(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<String, String> {
        let then_label = f.label();
        let else_label = f.label();
        let merge = f.label();
        let cond = self.emit_cond(f, &args[0], &then_label, &else_label, Some(&merge))?;
        f.start_block(&then_label);
        let then_value = self.emit_expr(f, &args[1])?;
        let then_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&else_label);
        let else_value = self.emit_expr(f, &args[2])?;
        let else_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let mut arms = vec![
            format!("[ {then_value}, %{then_from} ]"),
            format!("[ {else_value}, %{else_from} ]"),
        ];
        let mut fail_set = 0;
        for (v, from) in &cond.failed {
            arms.push(format!("[ {v}, %{from} ]"));
            fail_set |= f.set_of(v) & FAIL;
        }
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = phi %KValue {}", arms.join(", ")));
        f.record(&t, f.set_of(&then_value) | f.set_of(&else_value) | fail_set);
        Ok(t.to_string())
    }

    /// A condition is asked a question, not read for a value, and a
    /// comparison already knows the answer as an i1 before it builds anything.
    ///
    /// The value form costs eleven instructions where two would do. `b < 32`
    /// in the json escape path emits `icmp slt` and then `select` on it to
    /// build a KValue tagged 2 or 3, which a phi merges with `k_cmp`'s answer
    /// from the guarded slow arm, and the `if` then calls `k_not_failure` and
    /// `k_truthy` on the phi and branches on THAT. LLVM folds the pair away
    /// where the select reaches the branch directly; through the phi it
    /// cannot, because `k_cmp` may answer with a failure. So `setl` becomes a
    /// tag and two instructions later the tag is compared back. Measured on
    /// that one site: 108,174,000 instructions where 19,668,000 would do,
    /// 1.60% of encodebench.
    ///
    /// So the fast arm branches on its own i1 and never builds a value. The
    /// slow arm is unchanged — it calls the runtime, tests for a failure, asks
    /// whether the answer is true, and the failure it may carry is one more
    /// arm of the `if`'s phi. A condition that is not a comparison this can
    /// fuse is emitted as a value and tested the way it always was.
    fn emit_cond(
        &mut self,
        f: &mut FnEmit,
        cond: &Expr,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
    ) -> Result<Cond, String> {
        // `a and b` parses as `if a b false`, `a or b` as `if a true b` and
        // `not a` as `if a false true`, so a condition is very often another
        // `if`. Asked as a question the inner one costs nothing: each arm is
        // asked the same question the outer `if` asked, and an arm that is the
        // literal the desugaring wrote is an unconditional branch. Read for a
        // value instead it builds a tagged boolean through a phi and the outer
        // `if` takes it apart again — the same family as the comparison above,
        // and the reason `scan_at`'s digit test measured a sixth of what the
        // arithmetic predicted: the `if`'s own test fused and the `and` under
        // it did not. Nothing is duplicated, because both arms branch to the
        // labels the outer `if` already made.
        if let Expr::App { head, args, .. } = cond {
            if args.len() == 3 {
                if let Expr::Ident(name, _, _) = &**head {
                    if name == "if" && f.lookup(name).is_none() {
                        let inner_then = f.label();
                        let inner_else = f.label();
                        let mut failed =
                            self.emit_cond(f, &args[0], &inner_then, &inner_else, merge)?.failed;
                        f.start_block(&inner_then);
                        let yes = self.emit_cond(f, &args[1], then_label, else_label, merge)?;
                        failed.extend(yes.failed);
                        f.start_block(&inner_else);
                        let no = self.emit_cond(f, &args[2], then_label, else_label, merge)?;
                        failed.extend(no.failed);
                        return Ok(Cond { failed });
                    }
                }
            }
        }
        // The arms that desugaring writes. A constant answers the question
        // rather than being built and asked.
        if let Expr::Ident(name, _, _) = cond {
            if f.lookup(name).is_none() {
                if name == "true" {
                    f.line_fmt(format_args!("br label %{then_label}"));
                    return Ok(Cond { failed: Vec::new() });
                }
                if name == "false" {
                    f.line_fmt(format_args!("br label %{else_label}"));
                    return Ok(Cond { failed: Vec::new() });
                }
            }
        }
        if let Expr::BinOp { op, lhs, rhs, span } = cond {
            if matches!(*op, "==" | "!=" | "<" | "<=" | ">" | ">=") {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let a = self.as_value(f, &a);
                let b = self.as_value(f, &b);
                // A record on either side dispatches to the operator's user
                // arms, which answer with whatever the arm returns rather than
                // a boolean, so that condition is a value like any other.
                let armable = self.declares_at(op, 2);
                let routed = armable && (f.set_of(&a) | f.set_of(&b)) & REC != 0;
                if !routed {
                    return Ok(
                        self.emit_cmp_branch(f, op, &a, &b, *span, then_label, else_label, merge)
                    );
                }
                let v = self.emit_binop(f, op, &a, &b, *span)?;
                return Ok(self.test_cond_value(f, v, then_label, else_label, merge, *span));
            }
        }
        // A condition the demand analysis deferred arrives as a thunk, and
        // asking a thunk whether it is true reads the thunk rather than the
        // answer. Force before testing: `maybe_force` emits nothing where the
        // set proves there is no thunk, so a strict condition is unchanged.
        let v = self.emit_expr(f, cond)?;
        let v = self.maybe_force(f, v);
        Ok(self.test_cond_value(f, v, then_label, else_label, merge, cond.span()))
    }

    /// The value form: is it a failure, is it true, branch. The failure leaves
    /// by the `if`'s merge carrying the condition itself, which is what the
    /// language says an `if` over a failure answers.
    fn test_cond_value(
        &mut self,
        f: &mut FnEmit,
        v: String,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
        span: Span,
    ) -> Cond {
        let ok = inline_not_failure(f, &v);
        let check = f.label();
        let failed = match merge {
            Some(merge) => {
                let fail_from = f.cur_label.clone();
                f.line_fmt(format_args!("br i1 {ok}, label %{check}, label %{merge}"));
                vec![(v.clone(), fail_from)]
            }
            None => {
                let bail = f.label();
                f.line_fmt(format_args!("br i1 {ok}, label %{check}, label %{bail}"));
                f.start_block(&bail);
                self.emit_ret(f, &v);
                Vec::new()
            }
        };
        f.start_block(&check);
        // only a value that is not a boolean stops the program here
        if f.set_of(&v) & !(infer::BOOL | FAIL) != 0 {
            self.stamp_site(f, span);
        }
        let tv = f.tmp();
        f.predicate(&tv, format!("call i64 @k_truthy(%KValue {v})"));
        let tb = f.tmp();
        f.line_fmt(format_args!("{tb} = icmp ne i64 {tv}, 0"));
        f.line_fmt(format_args!("br i1 {tb}, label %{then_label}, label %{else_label}"));
        Cond { failed }
    }

    /// The guarded comparison, branching instead of answering. Two ints take
    /// the icmp straight to the branch; anything else goes to the runtime and
    /// is tested as a value.
    #[allow(clippy::too_many_arguments)]
    fn emit_cmp_branch(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
    ) -> Cond {
        let cmp = match op {
            "==" => "eq",
            "!=" => "ne",
            "<" => "slt",
            "<=" => "sle",
            ">" => "sgt",
            _ => "sge",
        };
        if f.set_of(a) == INT && f.set_of(b) == INT {
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let c = f.tmp();
            f.line_fmt(format_args!("{c} = icmp {cmp} i64 {pa}, {pb}"));
            f.line_fmt(format_args!("br i1 {c}, label %{then_label}, label %{else_label}"));
            return Cond { failed: Vec::new() };
        }
        let code = match op {
            "==" => 0,
            "!=" => 1,
            "<" => 2,
            "<=" => 3,
            ">" => 4,
            _ => 5,
        };
        let ta = inline_tag(f, a);
        let tb = inline_tag(f, b);
        let both = both_ints(f, &ta, &tb);
        let fast = f.label();
        let slow = f.label();
        f.line_fmt(format_args!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let pa = inline_payload(f, a);
        let pb = inline_payload(f, b);
        let c = f.tmp();
        f.line_fmt(format_args!("{c} = icmp {cmp} i64 {pa}, {pb}"));
        f.line_fmt(format_args!("br i1 {c}, label %{then_label}, label %{else_label}"));
        f.start_block(&slow);
        let sv = f.tmp();
        self.stamp_site(f, span);
        f.line_fmt(format_args!(
            "{sv} = call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 {code})"
        ));
        f.record(&sv, infer::BOOL | FAIL);
        self.test_cond_value(f, sv.to_string(), then_label, else_label, merge, span)
    }

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
        if vals.iter().any(|v| f.set_of(v) != INT) {
            return self.emit_arith_general(f, e, &vals, &mut 0);
        }
        let mut flags = Vec::new();
        let words: Vec<String> = vals.iter().map(|v| inline_payload(f, v)).collect();
        let slow = f.label();
        let r = word_run(f, e, &words, &mut 0, &mut flags, Some(&slow));
        let fv = f.tmp();
        f.line_fmt(format_args!("{fv} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"));
        if flags.is_empty() {
            f.record(&fv, INT);
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
        f.record(&t, INT | f.set_of(&sv));
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
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT);
            return Ok(t.to_string());
        }
        let pure_int = f.set_of(a) == INT && f.set_of(b) == INT;
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
        // The strict form's fallback is the twin, which answers a list index
        // without a call and hands the runtime everything else.
        // A container the inference knows is a string can only take the
        // twin's slow arm — the utf-8 seek does not inline — so those sites
        // keep the direct call and pay no tag test for a decision already
        // made at compile time.
        let slow_fn = match (strict, f.set_of(container) == STR) {
            (true, _) => "k_index_fast",
            (false, true) => "k_b_at",
            (false, false) => "k_b_at_fast",
        };
        let slow_extra = match strict {
            true => format!(", {}", self.origin_arg(f, span)),
            false => String::new(),
        };
        let proven = f.set_of(container) == BYTES && f.set_of(key) == INT;
        if proven {
            let bp = inline_payload(f, container);
            let bptr = f.tmp();
            f.line_fmt(format_args!("{bptr} = inttoptr i64 {bp} to ptr"));
            let len_ptr = f.tmp();
            f.line_fmt(format_args!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
            let idx = inline_payload(f, key);
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            // Two branches, the way the room tests ask: joined with `and`, the
            // pair became flags and an `or` wherever nothing upstream settled it.
            index_in_range(f, &idx, &len_ptr, &load, &miss);
            f.start_block(&load);
            let data_ptr = f.tmp();
            f.line_fmt(format_args!(
                "{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"
            ));
            let data = f.tmp();
            f.line_fmt(format_args!("{data} = load ptr, ptr {data_ptr}"));
            let off = f.tmp();
            f.line_fmt(format_args!("{off} = add i64 {idx}, -1"));
            let byte_ptr = f.tmp();
            f.line_fmt(format_args!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
            let byte = f.tmp();
            f.line_fmt(format_args!("{byte} = load i8, ptr {byte_ptr}"));
            let wide = f.tmp();
            f.line_fmt(format_args!("{wide} = zext i8 {byte} to i64"));
            let hit = f.tmp();
            f.line_fmt(format_args!(
                "{hit} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"
            ));
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&miss);
            let miss_value = if strict {
                let mv = f.tmp();
                self.stamp_site(f, span);
                f.line_fmt(format_args!(
                    "{mv} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
                ));
                mv.to_string()
            } else {
                "{ i64 4, i64 0 }".to_string()
            };
            let miss_from = f.cur_label.clone();
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = phi %KValue [ {hit}, %{load} ], [ {miss_value}, %{miss_from} ]"
            ));
            f.record(&t, if strict { INT | ERR } else { INT | NONE });
            if !strict {
                // The same merge as one i64: the byte, or 256 for none. A byte
                // discriminator wants exactly this and would otherwise rebuild
                // it with an extractvalue pair, an icmp and a select. Nothing
                // else reads it, so it costs nothing where it is unused.
                let raw = f.tmp();
                f.line_fmt(format_args!(
                    "{raw} = phi i64 [ {wide}, %{load} ], [ 256, %{miss_from} ]"
                ));
                f.raw_byte.insert(t.to_string(), raw.to_string());
            }
            return t.to_string();
        }
        // A plain index of a container the sets prove is a list, read the way
        // the twin's list arm reads it, with no tag test in front. What comes
        // out is whatever the list holds, so the result carries no set. The
        // strict form answers a box around the element, which is the runtime's
        // to build, so it keeps the general path.
        if !strict && f.set_of(container) == LIST && f.set_of(key) == INT {
            let lp = inline_payload(f, container);
            let lptr = f.tmp();
            f.line_fmt(format_args!("{lptr} = inttoptr i64 {lp} to ptr"));
            let idx = inline_payload(f, key);
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            // Two branches, the way the room tests ask: joined with `and`, the
            // pair became flags and an `or` wherever nothing upstream settled it.
            index_in_range(f, &idx, &lptr, &load, &miss);
            f.start_block(&load);
            let items_ptr = f.tmp();
            f.line_fmt(format_args!("{items_ptr} = getelementptr i8, ptr {lptr}, i64 8"));
            let items = f.tmp();
            f.line_fmt(format_args!("{items} = load ptr, ptr {items_ptr}"));
            let off = f.tmp();
            f.line_fmt(format_args!("{off} = add i64 {idx}, -1"));
            let slot = f.tmp();
            f.line_fmt(format_args!("{slot} = getelementptr %KValue, ptr {items}, i64 {off}"));
            let hit = f.tmp();
            f.line_fmt(format_args!("{hit} = load %KValue, ptr {slot}"));
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&miss);
            f.line_fmt(format_args!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = phi %KValue [ {hit}, %{load} ], [ {{ i64 4, i64 0 }}, %{miss} ]"
            ));
            return t.to_string();
        }
        // A plain index of a container the sets prove is a string, at a
        // position they prove is an int, goes to the string arm of `k_b_at`
        // with the pointer and the position: no bytes test in front of it,
        // which a string never passes, and no tag tests behind it. The arm
        // answers a one-character string or `none`, and stops nothing.
        if !strict && f.set_of(container) == STR && f.set_of(key) == INT {
            let sp = inline_payload(f, container);
            let sptr = f.tmp();
            f.line_fmt(format_args!("{sptr} = inttoptr i64 {sp} to ptr"));
            let idx = inline_payload(f, key);
            let t = f.call_pair(format_args!("@k_b_at_str(ptr {sptr}, i64 {idx})"));
            f.record(&t, STR | infer::NONE);
            return t.to_string();
        }
        let ct = inline_tag(f, container);
        let is_bytes = f.tmp();
        f.line_fmt(format_args!("{is_bytes} = icmp eq i64 {ct}, 13"));
        let kt = inline_tag(f, key);
        // a literal index's tag is known, and then the bytes test is the test
        let both = match kt.as_str() {
            "0" => is_bytes,
            _ => {
                let is_int = f.tmp();
                f.line_fmt(format_args!("{is_int} = icmp eq i64 {kt}, 0"));
                let both = f.tmp();
                f.line_fmt(format_args!("{both} = and i1 {is_bytes}, {is_int}"));
                both
            }
        };
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line_fmt(format_args!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let bp = inline_payload(f, container);
        let bptr = f.tmp();
        f.line_fmt(format_args!("{bptr} = inttoptr i64 {bp} to ptr"));
        let len_ptr = f.tmp();
        f.line_fmt(format_args!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
        let idx = inline_payload(f, key);
        let load = f.label();
        // Two branches, the way the room tests ask: joined with `and`, the
        // pair became flags and an `or` wherever nothing upstream settled it.
        index_in_range(f, &idx, &len_ptr, &load, &slow);
        f.start_block(&load);
        let data_ptr = f.tmp();
        f.line_fmt(format_args!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
        let data = f.tmp();
        f.line_fmt(format_args!("{data} = load ptr, ptr {data_ptr}"));
        let off = f.tmp();
        f.line_fmt(format_args!("{off} = add i64 {idx}, -1"));
        let byte_ptr = f.tmp();
        f.line_fmt(format_args!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
        let byte = f.tmp();
        f.line_fmt(format_args!("{byte} = load i8, ptr {byte_ptr}"));
        let wide = f.tmp();
        f.line_fmt(format_args!("{wide} = zext i8 {byte} to i64"));
        let fast_value = f.tmp();
        f.line_fmt(format_args!(
            "{fast_value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"
        ));
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&slow);
        let slow_value = f.tmp();
        self.stamp_site(f, span);
        f.line_fmt(format_args!(
            "{slow_value} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
        ));
        // The same i64 merge the proven path above builds, with the collapse
        // the crossing would otherwise write AFTER the merge sunk into this
        // arm instead. The fast arm's byte is already an i64 and pays
        // nothing; this arm is the one that calls the runtime, so four more
        // instructions here are four the hot path does not run. They are an
        // extract pair, an icmp and a select -- all pure, so where no byte
        // discriminator reads the result the whole thing is dead and goes.
        let slow_raw = match strict {
            true => String::new(),
            false => {
                let stag = f.tmp();
                f.line_fmt(format_args!("{stag} = extractvalue %KValue {slow_value}, 0"));
                let spay = f.tmp();
                f.line_fmt(format_args!("{spay} = extractvalue %KValue {slow_value}, 1"));
                let sisn = f.tmp();
                f.line_fmt(format_args!("{sisn} = icmp eq i64 {stag}, 4"));
                let sraw = f.tmp();
                f.line_fmt(format_args!("{sraw} = select i1 {sisn}, i64 256, i64 {spay}"));
                sraw.to_string()
            }
        };
        let slow_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = phi %KValue [ {fast_value}, %{load} ], [ {slow_value}, %{slow_from} ]"
        ));
        if !strict {
            let raw = f.tmp();
            f.line_fmt(format_args!(
                "{raw} = phi i64 [ {wide}, %{load} ], [ {slow_raw}, %{slow_from} ]"
            ));
            f.raw_byte.insert(t.to_string(), raw.to_string());
        }
        // This merge carried no set, so every reader took the default, which
        // is TOP, which contains THUNK -- and `maybe_force` then emitted a
        // `k_force_fast` on a value that reaches it through two arms neither
        // of which can answer a thunk. The proven path above has always
        // recorded `INT | NONE`; this one is the same index with the tag test
        // still to run, and it was missed. It is the shape the comment at the
        // arithmetic phi records, found in a second place.
        //
        // The bound is the LIST bit and nothing weaker. The fast arm is the
        // `insertvalue` written just above, an int. The slow arm is `k_b_at`
        // or the shim in front of it, and their cases answer a byte, a
        // one-character string, `none`, or a failure they were handed --
        // except the LIST case, which answers `l->items[i - 1]`, whatever the
        // list holds, thunks included. So the set is safe to narrow exactly
        // when the container cannot be a list.
        if f.set_of(container) & LIST == 0 {
            f.record(&t, TOP & !infer::THUNK);
        }
        t.to_string()
    }
}
