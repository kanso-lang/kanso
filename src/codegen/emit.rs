// The expression emitter: patterns, bodies, binds, calls, conditions and
// the arithmetic paths, the Backend methods that turn one function's body
// into LLVM IR. codegen.rs reads this file with include!, so the methods stay
// in the codegen module and the compiler builds to the same instructions it
// did when they sat in codegen.rs (kanso#1947). cargo fmt does not reach an
// included file, so CI's format job checks the files under src/'s
// subdirectories itself.

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
                let (post, posp, vtag, w1) = unpack_words(f, &w0, &w1);
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

    /// A `%parsed` and a `%KValue` share a `{i64,i64}` layout. On the failure
    /// path the two are the same value: the failure's tag/payload become the
    /// `%parsed` words, so the discriminator (low word ∈ {4,5}) stays intact.
    fn emit_parsed_from_failure(&self, f: &mut FnEmit, failure: &str) {
        let w0 = f.tmp();
        f.line_fmt(format_args!("{w0} = extractvalue %KValue {failure}, 0"));
        let w1 = f.tmp();
        f.line_fmt(format_args!("{w1} = extractvalue %KValue {failure}, 1"));
        let a = f.tmp();
        f.line_fmt(format_args!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line_fmt(format_args!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
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
        let (w0, w1) = pack_words(f, pid, &pos, &value);
        let a = f.tmp();
        f.line_fmt(format_args!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line_fmt(format_args!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.record_parsed(&p, ty, pid);
        Ok(p.to_string())
    }

    /// Build a register-returnable record in tail position as a by-value
    /// `%parsed`. The two words hold `(value.tag | pos << 8, value.payload)` — a
    /// non-failure value's tag never collides with the failure tags 4/5, so the
    /// low byte of word 0 still tells success from failure. A failing field
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
        let pos = self.emit_expr(f, &args[0])?;
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_pair_failure(f, &pos, &value);
        let (w0, w1) = pack_words(f, self.type_ids[ty], &pos, &value);
        let a = f.tmp();
        f.line_fmt(format_args!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line_fmt(format_args!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
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
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"));
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
            let held = f.lookup(name).ok_or_else(|| format!("native backend: `{name}` is not bound"))?;
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
                        f.line_fmt(format_args!("{t} = call %KValue @k_b_bytes_fast(%KValue {v})"));
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
            Expr::Upcast { expr: inner, ty, .. } => {
                let v = self.emit_expr(f, inner)?;
                let v = self.maybe_force(f, v);
                let want = self.sub_want(ty)?;
                let (tyn, _) = self.intern(&format!("{ty}\0"));
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_upcast(%KValue {v}, i64 {want}, ptr @{tyn})"
                ));
                f.record(&t, crate::infer::TOP);
                Ok(t.to_string())
            }
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
            Expr::Guard { .. } => {
                Err("native backend: a return guard sits only in tail position".to_string())
            }
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
                if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == crate::ahead::SLEEP_POOLED) {
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
                self.emit_call_full(f, head, args, *piped, *span)
            }
            Expr::Field { base, name, .. } => {
                let b = self.emit_expr(f, base)?;
                let (label, _) = self.intern(&format!("{name}\0"));
                let t = f.tmp();
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
                f.record(&boxed, DESC);
                Ok(boxed.to_string())
            }
            Expr::Join { lhs, rhs, .. } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let t = f.tmp();
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
                let t = f.tmp();
                f.line_fmt(format_args!("{t} = call %KValue @k_list_empty()"));
                f.record(&t, LIST);
                Ok(t.to_string())
            }
            Expr::MapLit(pairs, _) if pairs.is_empty() => {
                let t = f.tmp();
                f.line_fmt(format_args!("{t} = call %KValue @k_map_empty()"));
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
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(name, _, _) = head.as_ref() {
                let bare = name.strip_prefix("builtin_").unwrap_or(name);
                if self.forwarders.contains(bare, args.len())
                    || self.forwarders.contains(name, args.len())
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
                        return self.emit_parsed_construction(f, name, args);
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
                    for (i, arg) in args.iter().enumerate() {
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
                return Ok(self.test_cond_value(f, v, then_label, else_label, merge));
            }
        }
        // A condition the demand analysis deferred arrives as a thunk, and
        // asking a thunk whether it is true reads the thunk rather than the
        // answer. Force before testing: `maybe_force` emits nothing where the
        // set proves there is no thunk, so a strict condition is unchanged.
        let v = self.emit_expr(f, cond)?;
        let v = self.maybe_force(f, v);
        Ok(self.test_cond_value(f, v, then_label, else_label, merge))
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
        let _ = span;
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
        f.line_fmt(format_args!(
            "{sv} = call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 {code})"
        ));
        f.record(&sv, infer::BOOL | FAIL);
        self.test_cond_value(f, sv.to_string(), then_label, else_label, merge)
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
            "&" => format!("call %KValue @k_b_bit_and_fast(%KValue {a}, %KValue {b})"),
            "|" => format!("call %KValue @k_b_bit_or_fast(%KValue {a}, %KValue {b})"),
            "^" => format!("call %KValue @k_b_bit_xor_fast(%KValue {a}, %KValue {b})"),
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

    fn emit_call_full(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        piped: bool,
        span: Span,
    ) -> Result<String, String> {
        if piped && !args.is_empty() {
            // A `.>` whose subject is a strict index is a bind over a box the
            // read settles on the spot (ruled 2026-09-16), so nothing about it
            // is deferred: the element goes straight to the callback, and a
            // miss answers the missing-index err boxed. The chain's answer is
            // a box either way -- what the callback answered when that is one,
            // else its answer settled -- which is what the oracle's bind node
            // yields when the executor reaches it. The bind node, its closure
            // and the box under it are never built.
            if let Expr::Index { base, index, strict: true, span: at } = &args[0] {
                let container = self.emit_expr(f, base)?;
                let container = self.maybe_force(f, container);
                let key = self.emit_expr(f, index)?;
                let key = self.maybe_force(f, key);
                let read = self.emit_at(f, &container, &key, true, *at);
                let ok = inline_not_failure(f, &read);
                let docall = f.label();
                let missed = f.label();
                let merge = f.label();
                f.line_fmt(format_args!("br i1 {ok}, label %{docall}, label %{missed}"));
                f.start_block(&docall);
                let called = self.emit_call_rest(f, head, args, Some(read.clone()), span)?;
                let ctag = inline_tag(f, &called);
                let is_box = f.tmp();
                f.line_fmt(format_args!("{is_box} = icmp eq i64 {ctag}, 8"));
                let wrap = f.label();
                let answered = f.label();
                let called_from = f.cur_label.clone();
                f.line_fmt(format_args!("br i1 {is_box}, label %{answered}, label %{wrap}"));
                f.start_block(&wrap);
                let settled = f.tmp();
                f.line_fmt(format_args!("{settled} = call %KValue @k_b_effect(%KValue {called})"));
                f.line_fmt(format_args!("br label %{answered}"));
                f.start_block(&answered);
                let boxed = f.tmp();
                f.line_fmt(format_args!(
                    "{boxed} = phi %KValue [ {called}, %{called_from} ], [ {settled}, %{wrap} ]"
                ));
                let boxed_from = f.cur_label.clone();
                f.line_fmt(format_args!("br label %{merge}"));
                f.start_block(&missed);
                let failed = f.tmp();
                f.line_fmt(format_args!("{failed} = call %KValue @k_b_effect(%KValue {read})"));
                f.line_fmt(format_args!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = phi %KValue [ {boxed}, %{boxed_from} ], [ {failed}, %{missed} ]"
                ));
                f.record(&t, DESC);
                return Ok(t.to_string());
            }
            let piped_value = self.emit_expr(f, &args[0])?;
            if f.set_of(&piped_value) & DESC != 0 {
                let mut body_args: Vec<Expr> = vec![Expr::Ident(
                    Name::new("__piped"),
                    span,
                    crate::ast::Resolution::default(),
                )];
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
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_maybe_bind(%KValue {piped_value}, %KValue {closure})"
                ));
                f.record(&t, TOP);
                return Ok(t.to_string());
            }
            // a pipe hands its value on; a failure short-circuits before the
            // call (no dispatch, no hop) on every engine
            if f.set_of(&piped_value) & FAIL != 0 {
                let ok = inline_not_failure(f, &piped_value);
                let docall = f.label();
                let merge = f.label();
                let fail_from = f.cur_label.clone();
                f.line_fmt(format_args!("br i1 {ok}, label %{docall}, label %{merge}"));
                f.start_block(&docall);
                let called = self.emit_call_rest(f, head, args, Some(piped_value.clone()), span)?;
                let call_from = f.cur_label.clone();
                f.line_fmt(format_args!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = phi %KValue [ {piped_value}, %{fail_from} ], [ {called}, %{call_from} ]"
                ));
                f.record(&t, f.set_of(&called) | (f.set_of(&piped_value) & FAIL));
                return Ok(t.to_string());
            }
            // no description or failure can flow here: an ordinary call
            return self.emit_call_rest(f, head, args, Some(piped_value), span);
        }
        self.emit_call_rest(f, head, args, None, span)
    }

    /// The origin an err would carry had it been born inside the wrapper
    /// rather than at this call site. Fusing past a wrapper skips the frame
    /// whose file and line name the birthplace, and the oracle still calls it.
    fn forwarder_origin(&mut self, name: &str, arity: usize) -> Option<String> {
        let decl = self.decls_named(name).find(|d| d.params.len() == arity)?;
        let line = match decl.body.first()? {
            Stmt::Expr(Expr::App { span, .. }) => span.line,
            _ => return None,
        };
        // The same two-halves literal `origin_arg` builds, and for the same
        // reason: this stamps a wrapper's own frame on an err the builtin
        // raised, so the package it names is the WRAPPER's, not the caller's.
        let hako = crate::provenance::package_of(&decl.file);
        let prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
        let (interned, _) = self.intern(&format!("{hako}\0{prefix}:{line}\0"));
        Some(format!("ptr @{interned}"))
    }

    /// The builtin a name stands for at a call site: itself with the
    /// `builtin_` prefix off, or whatever the forwarder map says a plain
    /// wrapper of this arity forwards to.
    fn builtin_named(&self, name: &str, arity: usize) -> String {
        match self.forwarders.get(name, arity) {
            Some(target) => target.to_string(),
            None => name.strip_prefix("builtin_").unwrap_or(name).to_string(),
        }
    }

    /// Where a call's arguments are `keys m` and then `values m` of the same
    /// local name, the two positions, so both lists can come from one runtime
    /// call. Only the builtins themselves: a program that declares `keys` or
    /// `values`, or binds either name, is calling something else.
    fn both_columns(&self, f: &FnEmit, args: &[Expr]) -> Option<(usize, usize)> {
        // The shape is asked first because it is almost never there: every
        // call the emitter writes comes through here, and the declarations
        // are walked only for a call that already has the pair.
        let (keys_at, map) =
            args.iter().enumerate().find_map(|(at, a)| Some((at, column_of(a, "keys")?)))?;
        let values_at = args
            .iter()
            .enumerate()
            .skip(keys_at + 1)
            .find(|(_, a)| column_of(a, "values") == Some(map))?
            .0;
        self.columns_are_the_builtins(f, map).then_some((keys_at, values_at))
    }

    /// Whether `keys` and `values` here are the builtins and `map` a local:
    /// a program that declares or binds either name calls its own function.
    fn columns_are_the_builtins(&self, f: &FnEmit, map: &str) -> bool {
        let declared = |name: &str| f.lookup(name).is_some() || self.declares(name);
        f.lookup(map).is_some() && !declared("keys") && !declared("values")
    }

    /// Where statement `i` binds `keys m` to a name and the next binds
    /// `values m` to another, both strict, the index of the second. The first
    /// name must not be `m`, or the second statement reads the new binding.
    fn columns_bound(&self, f: &FnEmit, body: &[Stmt], i: usize) -> Option<usize> {
        let (
            Stmt::Bind { pattern: Pattern::Var(ks, _), expr: first },
            Some(Stmt::Bind { pattern: Pattern::Var(_, _), expr: second }),
        ) = (body.get(i)?, body.get(i + 1))
        else {
            return None;
        };
        let map = column_of(first, "keys")?;
        if column_of(second, "values") != Some(map) || ks.as_str() == map {
            return None;
        }
        let lazy = |at: usize| self.demand.is_lazy_bind(&f.group.clone(), f.arity, at);
        if lazy(i) || lazy(i + 1) || !self.columns_are_the_builtins(f, map) {
            return None;
        }
        Some(i + 1)
    }

    /// `keys m` and `values m` in one call, for `both_columns`.
    fn emit_columns(
        &mut self,
        f: &mut FnEmit,
        keys_call: &Expr,
    ) -> Result<(String, String), String> {
        let Expr::App { args, .. } = keys_call else { unreachable!("both_columns matched a call") };
        let m = self.emit_expr(f, &args[0])?;
        let m = self.maybe_force(f, m);
        let set = infer::builtin_set("keys", &[f.set_of(&m)]);
        let slot = f.tmp();
        f.line_fmt(format_args!("{slot} = alloca %KValue, align 8"));
        let ks = f.tmp();
        f.line_fmt(format_args!("{ks} = call %KValue @k_b_columns(%KValue {m}, ptr {slot})"));
        let vs = f.tmp();
        f.line_fmt(format_args!("{vs} = load %KValue, ptr {slot}, align 8"));
        f.record(&ks, set);
        f.record(&vs, set);
        Ok((ks.to_string(), vs.to_string()))
    }

    fn emit_call_rest(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        first: Option<String>,
        span: Span,
    ) -> Result<String, String> {
        // `first` is args[0], already emitted — a pipe hands its value in as
        // the head's first argument, it does not add one. Counting it twice
        // made a piped call to a lambda pass the value in both positions,
        // which the old unchecked cast to a two-argument signature dropped.
        let call_arity = args.len();
        // A literal lambda applied on the spot is a binding, not a value:
        // bind the arguments and emit the body here, instead of building a
        // closure and dispatching through k_callN. The fusion pass composes
        // adapter chains out of exactly these redexes, so without this step
        // a fused reducer pays two closures and two dynamic calls per
        // element. Failing arguments short-circuit first, as k_callN would.
        if let Expr::Lambda { params, body, .. } = head {
            if params.len() == call_arity {
                let mut vals: Vec<String> = Vec::new();
                let mut rest = args.iter();
                if let Some(v) = first.clone() {
                    vals.push(v);
                    rest.next();
                }
                for a in rest {
                    vals.push(self.emit_expr(f, a)?);
                }
                let mut bails: Vec<(String, String)> = Vec::new();
                let merge = f.label();
                for v in &vals {
                    if f.set_of(v) & FAIL == 0 {
                        continue;
                    }
                    let ok = inline_not_failure(f, v);
                    let bail_from = f.cur_label.clone();
                    let cont = f.label();
                    f.line_fmt(format_args!("br i1 {ok}, label %{cont}, label %{merge}"));
                    bails.push((v.clone(), bail_from));
                    f.start_block(&cont);
                }
                let saved: Vec<(String, Option<String>)> =
                    params.iter().map(|(p, _)| (p.clone(), f.lookup(p))).collect();
                for ((p, _), v) in params.iter().zip(&vals) {
                    f.bind(p, v);
                }
                let out = self.emit_expr(f, body)?;
                for (p, old) in saved {
                    match old {
                        Some(v) => f.bind(&p, &v),
                        None => {
                            f.versions.remove(&p);
                        }
                    }
                }
                if bails.is_empty() {
                    return Ok(out);
                }
                let body_from = f.cur_label.clone();
                let out_set = f.set_of(&out);
                let fail_bits: Set = bails.iter().fold(0, |acc, (v, _)| acc | (f.set_of(v) & FAIL));
                f.line_fmt(format_args!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                let mut sources: Vec<String> =
                    bails.iter().map(|(v, from)| format!("[ {v}, %{from} ]")).collect();
                sources.push(format!("[ {out}, %{body_from} ]"));
                f.line_fmt(format_args!("{t} = phi %KValue {}", sources.join(", ")));
                f.record(&t, out_set | fail_bits);
                return Ok(t.to_string());
            }
        }
        let computed_head = match head {
            // A local binding is a value. So is a top-level constant (a nullary
            // group) invoked with arguments and no arm at that arity: it holds a
            // function value, and `f x` calls that value, not a group named `f`.
            //
            // So is a value keyword, which is spelled like a name and reaches
            // this position when a callback is inlined: `list/map [1 2] none`
            // puts `none` where the callee goes. A number there takes the
            // computed path already and dies naming itself, and these must say
            // the same words rather than the emitter's.
            Expr::Ident(name, _, _) => {
                matches!(name.as_str(), "true" | "false" | "none" | "done")
                    || f.lookup(name).is_some()
                    || (call_arity >= 1
                        && self.declares_at(name, 0)
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
            let mut rest = args.iter();
            if let Some(v) = first {
                arg_vals.push(v);
                rest.next();
            }
            for a in rest {
                arg_vals.push(self.emit_expr(f, a)?);
            }
            let n = arg_vals.len();
            if n > 4 {
                return Err(format!(
                    "native backend: a function value takes at most 4 arguments, got {n}"
                ));
            }
            let arg_ir: String = arg_vals.iter().map(|v| format!(", %KValue {v}")).collect();
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = call %KValue @k_call{n}_fast(%KValue {callee}{arg_ir})"
            ));
            f.record(&t, TOP);
            return Ok(t.to_string());
        }
        let Expr::Ident(name, _, _) = head else {
            unreachable!("non-ident heads take the computed path");
        };
        if name == "if" {
            if let Some(t) = self.emit_byte_run(f, args)? {
                return Ok(t);
            }
            return self.emit_if_value(f, args);
        }
        // utf8 of a slice reads a byte view for a pointer and a length and
        // drops it, three million times in a decode. The wrapper inlining
        // above has already turned `text/utf8` and `text/slice` into their
        // builtins, so the pair is visible here as written, before either
        // argument is emitted.
        // An err's birthplace is the function it is emitted in, so fusing a
        // call that still names a wrapper would move it out of `text/utf8` and
        // into the caller — the oracle, which really does call the wrapper,
        // would then disagree about where an invalid byte was found. The
        // wrapper inlining above rewrites the call to the builtin wherever it
        // can, and only that spelling is fused.
        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "utf8" {
            if let Expr::App { head: inner_head, args: inner_args, piped: false, .. } = &args[0] {
                if let Expr::Ident(inner, _, _) = &**inner_head {
                    if self.builtin_named(inner, inner_args.len()) == "slice"
                        && inner_args.len() == 3
                    {
                        let mut parts = Vec::new();
                        for a in inner_args {
                            let v = self.emit_expr(f, a)?;
                            let v = self.maybe_force(f, v);
                            parts.push(self.unsub(f, v));
                        }
                        let sets: Vec<Set> = parts.iter().map(|e| f.set_of(e)).collect();
                        let sliced = infer::builtin_set("slice", &sets);
                        // the wrapper's own line, where the unfused call
                        // would have been emitted, or this site when the
                        // spelling is already the builtin
                        let origin = match name.as_str() {
                            "builtin_utf8" => self.origin_arg(f, span),
                            _ => match self.forwarder_origin(name, 1) {
                                Some(o) => o,
                                None => self.origin_arg(f, span),
                            },
                        };
                        let t = f.tmp();
                        // three KValues and the origin pointer want seven of
                        // the six integer registers the abi has, so the last
                        // spills and the callee reloads it; the twin tests the
                        // three tags here and hands the raw door five scalars.
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_utf8_slice_fast(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        f.record(&t, infer::builtin_set("utf8", &[sliced]));
                        return Ok(t.to_string());
                    }
                }
            }
        }
        // `append acc (slice cs a b)` builds a view header for the only
        // purpose of copying its bytes out and dropping it. The slice's whole
        // content is a pointer and a length the accumulator's copy needs
        // anyway, so the fused door reads the range out of `cs` and never
        // boxes it. Same wrapper-spelling rule as the pair above: only the
        // builtin spelling is fused, and `append` never gives birth to an err,
        // so no origin has to move with it.
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::App { head: inner_head, args: inner_args, piped: false, .. } = &args[1] {
                if let Expr::Ident(inner, _, _) = &**inner_head {
                    if self.builtin_named(inner, inner_args.len()) == "slice"
                        && inner_args.len() == 3
                    {
                        let acc = self.emit_expr(f, &args[0])?;
                        let acc = self.maybe_force(f, acc);
                        let mut parts = Vec::new();
                        for a in inner_args {
                            let v = self.emit_expr(f, a)?;
                            let v = self.maybe_force(f, v);
                            parts.push(self.unsub(f, v));
                        }
                        let sets: Vec<Set> = parts.iter().map(|e| f.set_of(e)).collect();
                        let sliced = infer::builtin_set("slice", &sets);
                        // the same uniqueness the unfused site would have got
                        let mutate = self.in_place_pushes.contains(&(
                            f.file.clone(),
                            span.line as usize,
                            span.col as usize,
                        ));
                        let t = f.tmp();
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_append_slice_fast(%KValue {acc}, %KValue {}, %KValue {}, %KValue {}, i64 {})",
                            parts[0], parts[1], parts[2], i64::from(mutate)
                        ));
                        f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), sliced]));
                        return Ok(t.to_string());
                    }
                }
            }
        }
        // `to_int (slice cs a b)` and `to_float (slice cs a b)` build a view
        // for the one purpose of reading a number out of it, which is every
        // number the JSON decoder meets. The fused doors read the range out
        // of `cs` in place. Only the builtin spelling is fused, as above, and
        // the origin moves with the call because both conversions can give
        // birth to an err.
        if first.is_none() && args.len() == 1 {
            let door = match self.builtin_named(name, 1).as_str() {
                "to_int" => Some("to_int"),
                "to_float" => Some("to_float"),
                _ => None,
            };
            if let (
                Some(door),
                Expr::App { head: inner_head, args: inner_args, piped: false, .. },
            ) = (door, &args[0])
            {
                if let Expr::Ident(inner, _, _) = &**inner_head {
                    if self.builtin_named(inner, inner_args.len()) == "slice"
                        && inner_args.len() == 3
                    {
                        let mut parts = Vec::new();
                        for a in inner_args {
                            let v = self.emit_expr(f, a)?;
                            let v = self.maybe_force(f, v);
                            parts.push(self.unsub(f, v));
                        }
                        let sets: Vec<Set> = parts.iter().map(|e| f.set_of(e)).collect();
                        let sliced = infer::builtin_set("slice", &sets);
                        let origin = self.origin_arg(f, span);
                        let t = f.tmp();
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_{door}_slice(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        f.record(&t, infer::builtin_set(door, &[sliced]));
                        return Ok(t.to_string());
                    }
                }
            }
        }
        // `append acc "{x}"` where `x` is a number: the template rendered
        // `x` into a string for the one purpose of copying its bytes into
        // `acc` and dropping it, which is every scalar the JSON encoder
        // writes. The fused door renders the digits into a stack buffer and
        // copies them from there, so no string is built. A value the ambient
        // to_string group could claim keeps the dispatch the template would
        // have made, so a user arm is never skipped; every other tag the door
        // hands to k_render itself, the same call the template makes, so the
        // bytes cannot differ. Same wrapper-spelling rule as the pair above.
        // `bytes ""` starts a builder, and the next thing that happens to it
        // is an append. As a view of the empty string it owned no storage, so
        // that append grew it from nothing: 175,527 grows a run on runbench,
        // one for each escaped string the decoder unescapes, at about 88
        // instructions each. The literal takes a builder with 64 bytes of room
        // instead, which is the capacity that first grow chose, so every later
        // grow is the one it was. Only the literal: a view of any other string
        // is what `bytes` has always built, and testing for an empty string at
        // run time cost encodebench two instructions on every string it
        // escapes.
        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "bytes" {
            if let Expr::Str(parts, _) = &args[0] {
                if parts.iter().all(|p| matches!(p, TemplatePart::Lit(s) if s.is_empty())) {
                    let t = f.tmp();
                    f.line_fmt(format_args!("{t} = call %KValue @k_b_bytes_seed()"));
                    f.record(&t, infer::builtin_set("bytes", &[STR]));
                    return Ok(t.to_string());
                }
            }
        }
        // `append acc "true"`: a literal of one to eight bytes into a builder
        // the linearity analysis proved this site owns. `k_b_append_mut_word`
        // says why the literal travels as a word.
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::Str(parts, _) = &args[1] {
                let text: Option<String> = parts
                    .iter()
                    .map(|p| match p {
                        TemplatePart::Lit(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect();
                let mutate = self.in_place_pushes.contains(&(
                    f.file.clone(),
                    span.line as usize,
                    span.col as usize,
                ));
                if let Some(text) = text.filter(|t| mutate && (1..=8).contains(&t.len())) {
                    let acc = self.emit_expr(f, &args[0])?;
                    let acc = self.maybe_force(f, acc);
                    let mut bytes = [0u8; 8];
                    bytes[..text.len()].copy_from_slice(text.as_bytes());
                    let word = i64::from_le_bytes(bytes);
                    let (lit, n) = self.intern(&text);
                    let t = f.tmp();
                    f.line_fmt(format_args!(
                        "{t} = call %KValue @k_b_append_mut_word(%KValue {acc}, i64 {word}, i64 {n}, ptr @{lit}, ptr @{lit}_lit)"
                    ));
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), STR]));
                    return Ok(t.to_string());
                }
            }
        }
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::Str(parts, _) = &args[1] {
                if let [TemplatePart::Interp(inner)] = parts.as_slice() {
                    let acc = self.emit_expr(f, &args[0])?;
                    let acc = self.maybe_force(f, acc);
                    let value = self.emit_expr(f, inner)?;
                    let value = self.maybe_force(f, value);
                    let mutate = self.in_place_pushes.contains(&(
                        f.file.clone(),
                        span.line as usize,
                        span.col as usize,
                    ));
                    let fusable = !self.render_dispatchable(f, &value);
                    let fuse_render = f.set_of(&value) & (INT | FLOAT) != 0 && fusable;
                    let t = f.tmp();
                    let rendered_set = match fuse_render {
                        true => {
                            let fails = f.set_of(&value) & ERR;
                            let value = self.as_value(f, &value);
                            f.line_fmt(format_args!(
                                "{t} = call %KValue @k_b_append_rendered(%KValue {acc}, %KValue {value}, i64 {})",
                                i64::from(mutate)
                            ));
                            STR | fails
                        }
                        false => {
                            // both operands are already emitted, so the call
                            // the generic path would have made is made here:
                            // the byte twin, whose string arm copies inline.
                            // The first cut called k_b_append_mut instead and
                            // paid 22,789,710 in k_b_append_wide on runbench.
                            let (rendered, fails) = self.render_interp(f, &value);
                            let sym = match mutate {
                                true => "k_b_append_mut_byte",
                                false => "k_b_append_byte",
                            };
                            f.line_fmt(format_args!(
                                "{t} = call %KValue @{sym}(%KValue {acc}, %KValue {rendered})"
                            ));
                            STR | fails
                        }
                    };
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), rendered_set]));
                    return Ok(t.to_string());
                }
            }
        }
        // a region: the mark goes down before the arguments, so what they
        // allocate is inside it (see beat::region_sites)
        let region = first.is_none()
            && self.beat.regions.contains(&(f.file.clone(), span.line as usize, span.col as usize))
            && !name.starts_with("builtin_")
            && !self.type_ids.contains_key(name.as_str())
            && self.declares_at(name, args.len());
        if region {
            f.line("call void @k_beat_push()");
        }
        let mut emitted = Vec::new();
        let skip = usize::from(first.is_some());
        let columns = self.both_columns(f, &args[skip..]).map(|(k, v)| (k + skip, v + skip));
        let mut values_of_the_pair: Option<String> = None;
        let mut iter = args.iter().enumerate();
        if let Some(first_value) = first {
            emitted.push(first_value);
            iter.next();
        }
        for (at, arg) in iter {
            match columns {
                Some((keys_at, _)) if at == keys_at => {
                    let (ks, vs) = self.emit_columns(f, arg)?;
                    values_of_the_pair = Some(vs);
                    emitted.push(ks);
                }
                Some((_, values_at)) if at == values_at => {
                    emitted.push(values_of_the_pair.take().expect("the keys came first"));
                }
                _ => emitted.push(self.emit_expr(f, arg)?),
            }
        }
        // std wrappers reach natives through the builtin_ prefix — and the
        // prefix BYPASSES group dispatch entirely, or a bare clone named
        // like the builtin would capture its own wrapper's body (the
        // d_join_2 self-recursion)
        let was_builtin = name.starts_with("builtin_");
        let name: &str = name.strip_prefix("builtin_").unwrap_or(name);

        // Every builtin's count, before anything reads an argument by index.
        // The block below emits `wrap_err` inline and takes `emitted[1]`, and
        // the guard further down covers only the names this file emits a
        // direct C call for — `wrap_err` is not one of them. So
        // `print (wrap_err 1)` walked off the end of a one-element vector and
        // aborted the process: exit 101 with a Rust backtrace, on a two-word
        // program. The front door refuses that program now, and this stays
        // because a backend that indexes an argument it never counted is one
        // front-end regression from doing it again.
        // A declaration of the same name is that declaration — the bail
        // further down says so, and this has to say it too, because it runs
        // first. lib/sha256 declares `bytes`, and a guard that skipped this
        // condition refused its three-argument call as a wrong-count builtin.
        // A type the program declares shadows a builtin too: a file that
        // declares `type run` builds one with `run 1 2 3`.
        let shadows = !was_builtin && (self.declares(name) || self.type_ids.contains_key(name));
        if !shadows {
            if let Some(takes) = crate::check::builtin_arity(name) {
                if emitted.len() != takes {
                    return Err(format!("native backend: `{name}` takes {takes} argument(s)"));
                }
            }
        }

        // A builtin sees a subtype's value as its parent's, which is what the
        // oracle's call_builtin does before anything else. Only a program that
        // declares a subtype can hand one over, so only that program pays.
        // A type's constructor comes through here too and must see the value
        // it wraps, so only a real builtin's arguments are unwrapped.
        // A lazy binding holding a subtype's value is a thunk until forced,
        // and a thunk is not a K_SUB, so the unwrap would pass it through and
        // the builtin would meet the subtype after its own force. Forcing
        // first is what the later force would do anyway; the gated force
        // emits nothing when the set proves the value cannot be a thunk.
        if !shadows && !self.sub_parents.is_empty() && crate::check::builtin_arity(name).is_some() {
            for e in emitted.iter_mut() {
                let forced = self.maybe_force(f, e.clone());
                let t = f.tmp();
                f.line_fmt(format_args!("{t} = call %KValue @k_unsub(%KValue {forced})"));
                *e = t.to_string();
            }
        }
        if name == "err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = call %KValue @k_err(%KValue {}, {origin})", emitted[0]));
            f.record(&t, ERR);
            return Ok(t.to_string());
        }
        // `rescue` and `annotate` are handed the site they were written at,
        // like `err` and `wrap_err`: `annotate` because it raises an err of
        // its own, `rescue` because its licence is foreign-only and the
        // site's package is what the failure's raiser is compared against.
        // The runtime wraps the callback in a closure holding both and hands
        // the result to the one worded node.
        if name == "rescue" || name == "annotate" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = call %KValue @k_b_{name}(%KValue {}, %KValue {}, {origin})",
                emitted[0], emitted[1]
            ));
            f.record(&t, TOP);
            return Ok(t.to_string());
        }
        if name == "wrap_err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = call %KValue @k_b_wrap_err(%KValue {}, %KValue {}, {origin})",
                emitted[0], emitted[1]
            ));
            f.record(&t, ERR);
            return Ok(t.to_string());
        }
        if name == "print" {
            // a non-string argument renders through the same ambient
            // to_string dispatch interpolation uses, so user arms win
            let arg = match f.set_of(&emitted[0]) & !FAIL & !STR {
                0 => emitted[0].clone(),
                _ => {
                    let forced = self.maybe_force(f, emitted[0].clone());
                    let may_be_err = f.set_of(&emitted[0]) & ERR != 0;
                    let r = self.render_through_group(f, &forced, may_be_err);
                    f.record(&r, STR | (f.set_of(&forced) & FAIL) | ERR);
                    r
                }
            };
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = call %KValue @k_desc_print(%KValue {arg})"));
            f.record(&t, DESC | (f.set_of(&arg) & FAIL));
            return Ok(t.to_string());
        }
        if name == "sleep" || name == "random" {
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = call %KValue @k_desc_{name}(%KValue {})", emitted[0]));
            f.record(&t, DESC | (f.set_of(&emitted[0]) & FAIL));
            return Ok(t.to_string());
        }
        if let Some(id) = self.type_ids.get(name).copied() {
            if let Some(parent) = self.sub_parents.get(name).cloned() {
                if emitted.len() != 1 {
                    return Err(format!("native backend: `{name}` wraps one value"));
                }
                let inner = self.maybe_force(f, emitted[0].clone());
                let want = self.sub_want(&parent)?;
                let (tyn, _) = self.intern(&format!("{name}\0"));
                let (par, _) = self.intern(&format!("{parent}\0"));
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = call %KValue @k_sub_ctor(i64 {id}, i64 {want}, %KValue {inner}, ptr @{tyn}, ptr @{par})"
                ));
                f.record(&t, crate::infer::TOP);
                return Ok(t.to_string());
            }
            // A constructor slot is where a knot ties: a field still being
            // computed is stored, so the cell completes here and the field
            // resolves against it afterwards. Whether a cell is mid-flight is
            // a runtime fact, so the emitter cannot decide it — the helper
            // asks, and only programs that defer a self-reference pay for it.
            let emitted: Vec<String> = emitted
                .into_iter()
                .map(|e| match self.defers_self_reference {
                    true => self.force_unless_knot(f, e),
                    false => self.maybe_force(f, e),
                })
                .collect();
            self.emit_typeset_checks(f, name, &emitted)?;
            let n = emitted.len();
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
            // a record this call is the last reader of can be built into
            let victim = self
                .reusable_records
                .get(&(f.file.clone(), span.line as usize, span.col as usize))
                .and_then(|name| f.lookup(name));
            match victim {
                Some(v) => f.line_fmt(format_args!(
                    "{t} = call %KValue @k_rec_reuse(i64 {id}, i64 {n}, ptr {arr}, %KValue {v})"
                )),
                None => f.line_fmt(format_args!(
                    "{t} = call %KValue @k_rec(i64 {id}, i64 {n}, ptr {arr})"
                )),
            }
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            f.record(&t, REC | fails);
            return Ok(t.to_string());
        }
        // The arity has to match a real declaration. Matching on the name
        // alone emits a call to `d_{name}_{n}` for any n the caller wrote,
        // and a dispatcher that was never defined is invalid IR the user
        // meets as a clang error.
        if !was_builtin && self.declares_at(name, emitted.len()) {
            let n = emitted.len();
            let args_ir: Vec<String> = emitted
                .iter()
                .enumerate()
                .map(|(i, e)| self.call_arg(f, name, n, i, e, args.get(i)))
                .collect();
            let callee_ret = self.ret_ty(name, n);
            // A register-returned record comes back as two raw field words,
            // not a tagged value, and both pops read a KValue. Reinterpreting
            // one would hand them a pair of fields to treat as a tag and a
            // payload, so the frontier goes unmarked instead: these calls give
            // up the rewind rather than get it wrong.
            let register_returned = callee_ret == "%parsed";
            let beat_entry =
                self.beat.ids.contains_key(&(name.to_string(), n)) && !register_returned;
            // a construction cohort: a qualified call from user code whose
            // arguments are all immutable shapes (scalars, strings) cannot
            // have its caller's storage grown by the callee, so the call's
            // garbage dies with the pop. loops keep their own tier, and a
            // caller already inside a beat cluster lets its rewind do the
            // reclaiming instead.
            let heapish: Set = BYTES | LIST | MAP | REC | DESC | infer::FN | crate::infer::THUNK;
            // the call must cross down into a nested module — the caller's
            // own code reaching a dependency, at whatever depth the import
            // graph put the caller. a caller that is itself a rewinding loop
            // member keeps its own tier. a group that appears in the beat
            // ids only as a demoted entry is not a loop: its bracket never
            // rewinds mid-body, so the cohort wrap still applies inside it.
            let caller = (f.group.clone(), f.arity);
            let caller_loops = self.beat.ids.contains_key(&caller)
                && !self.beat.demoted.iter().any(|(_, callee)| *callee == caller);
            // bytes join scalars and strings in the license: raw bytes hold
            // no pointers and no thunks, so nothing a rewind frees can be
            // reached through them, and a mut-grown unique arg is
            // unreachable after its last use. containers stay excluded —
            // they can carry thunks whose forced values would die under a
            // cell the caller still holds.
            let arg_heapish = heapish & !BYTES;
            let crosses_down = self
                .group_named(name)
                .first()
                .is_some_and(|&i| *self.program.fns[i].file != *f.file);
            let cohort_entry = !beat_entry
                && !region
                && !register_returned
                && crosses_down
                && !self.cycle_reached.contains(f.group.as_str())
                && !f.synthetic
                && !caller_loops
                && emitted.iter().all(|e| f.set_of(e) & arg_heapish == 0);
            // The twin's arguments are rendered here too, below the mark the
            // push sets, and a slot that is not a word reuses the general
            // rendering. Rendered after the push, a string builder's seed sat
            // above the mark, and the string it grew was evacuated whole at
            // the pop: 55,136 bytes for basket's document.
            let no_pack: Vec<Option<String>> = vec![None; n];
            let twin = self.twin_test(f, name, n, &emitted, &no_pack);
            let twin_ir: Vec<String> = match twin {
                Some(_) => (0..n)
                    .map(|i| match self.twin_param(name, n, i) {
                        true => self.call_arg_to(f, name, n, i, &emitted[i], args.get(i), true),
                        false => args_ir[i].clone(),
                    })
                    .collect(),
                None => Vec::new(),
            };
            if !region && (beat_entry || cohort_entry) {
                // entering a beat loop or a cohort: mark the frontier; args
                // are already evaluated, so they live below the mark
                f.line("call void @k_beat_push()");
            }
            let t = match twin {
                None => {
                    let t = f.tmp();
                    f.line_fmt(format_args!(
                        "{t} = call tailcc {callee_ret} @{}({})",
                        dsym(name, n),
                        args_ir.join(", ")
                    ));
                    t
                }
                Some(words) => {
                    let call_twin = |f: &mut FnEmit| {
                        let t = f.tmp();
                        f.line_fmt(format_args!(
                            "{t} = call tailcc {callee_ret} @{}({})",
                            tsym(name, n),
                            twin_ir.join(", ")
                        ));
                        t
                    };
                    match words {
                        None => call_twin(f),
                        // a call into a loop from outside it picks the body
                        // once, and the loop's own calls pick from there
                        Some(words) => {
                            let to_twin = f.label();
                            let to_general = f.label();
                            let joined = f.label();
                            f.line_fmt(format_args!(
                                "br i1 {words}, label %{to_twin}, label %{to_general}"
                            ));
                            f.start_block(&to_twin);
                            let a = call_twin(f);
                            f.line_fmt(format_args!("br label %{joined}"));
                            f.start_block(&to_general);
                            let b = f.tmp();
                            f.line_fmt(format_args!(
                                "{b} = call tailcc {callee_ret} @{}({}) #90",
                                dsym(name, n),
                                args_ir.join(", ")
                            ));
                            f.line_fmt(format_args!("br label %{joined}"));
                            f.start_block(&joined);
                            let t = f.tmp();
                            f.line_fmt(format_args!(
                                "{t} = phi {callee_ret} [ {a}, %{to_twin} ], [ {b}, %{to_general} ]"
                            ));
                            t
                        }
                    }
                }
            };
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            let result = if region {
                let p = f.tmp();
                f.line_fmt(format_args!("{p} = call %KValue @k_region_pop(%KValue {t})"));
                p
            } else if beat_entry {
                let p = f.tmp();
                f.line_fmt(format_args!("{p} = call %KValue @k_beat_pop(%KValue {t})"));
                p
            } else if cohort_entry {
                let p = f.tmp();
                f.line_fmt(format_args!("{p} = call %KValue @k_cohort_pop(%KValue {t})"));
                p
            } else {
                t
            };
            if let Some(ty) = self.escape.returns_ty(name, n) {
                if callee_ret == "%parsed" {
                    f.record_parsed(&result, ty, self.type_ids[ty]);
                }
            }
            let returned = self.group_return_set(name, n) | fails;
            f.record(&result, returned);
            if callee_ret == "%KValue" && self.sub_parents.is_empty() {
                self.assume_tag(f, &result, returned);
            }
            return Ok(result.to_string());
        }
        // Declared, but at no arity this call can reach. The interpreter
        // reports it when the call runs, so native reports the same words at
        // the same moment rather than refusing to build a program the oracle
        // executes.
        if !was_builtin && self.declares(name) {
            let msg =
                format!("no overload of `{}` matches these arguments", crate::ast::spoken(name));
            let (m, _) = self.intern(&format!("{msg}\0"));
            f.line_fmt(format_args!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
            let after = f.label();
            f.start_block(&after);
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = select i1 true, %KValue {{ i64 4, i64 0 }}, %KValue {{ i64 4, i64 0 }}"
            ));
            f.record(&t, NONE);
            return Ok(t.to_string());
        }
        if name == "at" && emitted.len() == 2 {
            return Ok(self.emit_at(f, &emitted[0].clone(), &emitted[1].clone(), false, span));
        }
        // a std wrapper that only forwards to a builtin costs a dispatched
        // call per use; the call site goes straight to the builtin (and its
        // inline twins). The rename lives INSIDE this branch only — it must
        // never leak into user-group dispatch, whose per-site specialized
        // signatures the renamed identity would not match.
        let forwarded = self.forwarders.get(name, emitted.len()).map(str::to_string);
        let name: &str = match &forwarded {
            Some(target) => target.as_str(),
            None => name,
        };
        if let Some(arity) = arity_of_emitted(name) {
            if emitted.len() != arity {
                return Err(format!("native backend: `{name}` takes {arity} argument(s)"));
            }
            // builtins scrutinize every argument; a thunk forces here (the
            // gated force emits nothing when the set proves it can't be one)
            // A list stores what it is handed, so the item `push` adds may be
            // a `list/tie` cell still being made, as a list literal's may:
            // `list/to_list (list/map xs ref)` fuses to a push of each `ref`.
            let emitted: Vec<String> = emitted
                .into_iter()
                .enumerate()
                .map(|(at, e)| match name == "push" && at == 1 && self.defers_self_reference {
                    true => self.force_unless_knot(f, e),
                    false => self.maybe_force(f, e),
                })
                .collect();
            // `length` of a value the sets already prove is bytes is the
            // header's first field. The twin tests the tag and keeps a call
            // arm for everything else, and that arm is a block merge: a
            // later read of the same field cannot be forwarded across it, so
            // a loop that asks `length coll < i` and then indexes `coll[i]`
            // loads the length twice and compares it twice.
            let held = f.set_of(&emitted[0]);
            if name == "length" && held != 0 && held & !(BYTES | LIST) == 0 {
                let bp = inline_payload(f, &emitted[0]);
                let bptr = f.tmp();
                f.line_fmt(format_args!("{bptr} = inttoptr i64 {bp} to ptr"));
                let len_ptr = f.tmp();
                f.line_fmt(format_args!(
                    "{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"
                ));
                let len = f.tmp();
                f.line_fmt(format_args!("{len} = load i64, ptr {len_ptr}"));
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {len}, 1"
                ));
                f.record(&t, infer::builtin_set("length", &[held]));
                return Ok(t.to_string());
            }
            let mut args_ir: Vec<String> = emitted.iter().map(|e| format!("%KValue {e}")).collect();
            // builtins that can give birth to an err take the site's origin
            if matches!(name, "to_int" | "to_float" | "utf8" | "from_code" | "to_bytes") {
                args_ir.push(self.origin_arg(f, span));
            }
            // A push the linearity analysis proved unique extends its list in
            // place instead of allocating a fresh header.
            let in_place = self.in_place_pushes.contains(&(
                f.file.clone(),
                span.line as usize,
                span.col as usize,
            ));
            let sym = if name == "push" && in_place {
                // the twin claims the frontier slot itself; a grow, a full
                // buffer or anything that is not a list falls to the C. An
                // item the sets prove is no failure and no bignum skips the
                // item test.
                match f.set_of(&emitted[1]) & (FAIL | infer::BIG) == 0 {
                    true => "push_mut_known",
                    false => "push_mut_fast",
                }
            } else if name == "concat" && in_place {
                // every element of the second list goes onto the first's
                // frontier; the first list is the one nobody else holds
                "concat_mut"
            } else if name == "put" && in_place {
                // the twin writes the frontier pair itself where the map has
                // no sorted view; everything else falls to the C by call
                "put_mut_fast"
            } else if name == "append" && in_place {
                // the in-place byte claim inlines whole; a byte that does not
                // fit falls through to the C path inside the twin. Where the
                // sets already prove both tags -- a bytes accumulator and an
                // int -- the twin's two opening tests are asked twice, so
                // there is a second door that starts after them.
                match f.set_of(&emitted[0]) == BYTES && f.set_of(&emitted[1]) == INT {
                    true => "append_mut_int",
                    false => "append_mut_byte",
                }
            } else if BIT_TWINS.contains(&name) {
                // one machine op each where the operand tags say int and a
                // shift is in range. `&` `|` `^` reach the twin through the
                // operator route; these are the same work written as a name,
                // which is how lib/bits spells what no operator says.
                bit_twin(name)
            } else if name == "slice" {
                // the bytes arm is a header read and four compares, and the
                // three failure guards plus two tag tests in front of it cost
                // more than the view it builds. The twin tests the tags and
                // hands the arithmetic a pointer, a length and two integers.
                "slice_fast"
            } else if name == "find2" {
                // four KValues do not fit the six registers the ABI has, so
                // the fourth arrives on the stack and the callee unpacks it
                // before it can splat the byte — fourteen instructions of the
                // fifty-four this cost, where the scan itself was ten. The
                // twin tests the tags, hands the scan five scalars, and folds
                // to nothing at a call site whose needles are literals.
                "find2_fast"
            } else if name == "find2_below" {
                // the same door with one more bound, and the largest spill of
                // the five the emitter declares over six registers: FIVE
                // KValues are ten register-sized arguments, so four arrive on
                // the caller's stack. The twin tests the five tags and hands
                // the scan a pointer, a length and four integers — six, and
                // nothing spills.
                "find2_below_fast"
            } else if name == "bytes" {
                // two field reads and a three-field header into the arena, and
                // the arena bump is already inline in the append twin. The
                // call and its tag ladder were most of the thirty this cost.
                "bytes_fast"
            } else if name == "length" {
                // the list case is a header load; the twin inlines it
                "length_fast"
            } else if name == "append" {
                // the single-byte frontier claim inlines whole; everything
                // else falls through to the C path inside the twin
                "append_byte"
            } else {
                name
            };
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = call %KValue @k_b_{sym}({})", args_ir.join(", ")));
            let arg_sets: Vec<Set> = emitted.iter().map(|e| f.set_of(e)).collect();
            // `join` answers the first item that fails, and a list literal is
            // where an item can: inference admits it the same way.
            let read = match name {
                "join" if arg_sets[0] & LIST != 0 => self.inference.stored_fails,
                _ => 0,
            };
            f.record(&t, infer::builtin_set(name, &arg_sets) | read);
            return Ok(t.to_string());
        }
        Err(format!("native backend: `{name}` is not yet supported"))
    }
}
