// The expression emitter: function bodies, binds, conditions and indexing,
// the Backend methods that turn one function's body into LLVM IR. Patterns
// are in patterns.rs, the operators in operators.rs and calls in calls.rs.
// codegen.rs reads all four with include!, so the methods stay in the
// codegen module and the compiler builds to the same instructions it did
// when they sat in codegen.rs (kanso#1947). cargo fmt does not reach an
// included file, so CI's format job checks the files under src/'s
// subdirectories itself.

impl<'a> Backend<'a> {
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
                            "{t} = call %KValue @k_bind_settled(%KValue {value}, %KValue {closure})"
                        ));
                        f.record(&t, TOP);
                        self.emit_ret(f, &t);
                        f.start_block(&check);
                        // A lazy binding handed on unread is an effect only
                        // once forced. k_maybe_bind forces it and decides, so
                        // a cell takes that path, and the test sits where a
                        // description has already been ruled out.
                        if self.may_hold_thunk(f, &value) {
                            let is_cell = f.tmp();
                            f.line_fmt(format_args!("{is_cell} = icmp eq i64 {tag}, 14"));
                            let cell_path = f.label();
                            let settled = f.label();
                            f.line_fmt(format_args!(
                                "br i1 {is_cell}, label %{cell_path}, label %{settled}"
                            ));
                            f.start_block(&cell_path);
                            let t = f.tmp();
                            let closure = self.emit_expr(f, head)?;
                            f.line_fmt(format_args!(
                                "{t} = call %KValue @k_maybe_bind(%KValue {value}, %KValue {closure})"
                            ));
                            f.record(&t, TOP);
                            self.emit_ret(f, &t);
                            f.start_block(&settled);
                        }
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
        if f.set_of(&xv) != BYTES
            || (!infer::word(p_set) && p_set | infer::WIDTHS != INT | infer::BIG)
        {
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
        if !infer::word(p_set) {
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
        if infer::word(f.set_of(a)) && infer::word(f.set_of(b)) {
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
        let proven = f.set_of(container) == BYTES && infer::word(f.set_of(key));
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
        if !strict && f.set_of(container) == LIST && infer::word(f.set_of(key)) {
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
        if !strict && f.set_of(container) == STR && infer::word(f.set_of(key)) {
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
