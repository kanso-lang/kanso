// Call emission: a call's arguments and the arms it may reach, the
// builtins the emitter fuses into the call site, and the calls to declared
// functions. It is the second half of the Backend's expression emitter, read
// with include! right after emit.rs, so it stays in the codegen module.

impl<'a> Backend<'a> {
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
                // A record returned by value is three words, and the phi
                // below joins `%KValue`s. The call takes it as it is, since
                // the callee may take it by value too; the failure edge
                // carries it boxed. mustache's `half 4 .> shown` handed the
                // three words to the phi and clang refused the module.
                let failed = match f.is_parsed(&piped_value) {
                    true => f.box_parsed(&piped_value),
                    false => piped_value.clone(),
                };
                let docall = f.label();
                let merge = f.label();
                let fail_from = f.cur_label.clone();
                f.line_fmt(format_args!("br i1 {ok}, label %{docall}, label %{merge}"));
                f.start_block(&docall);
                let called = self.emit_call_rest(f, head, args, Some(piped_value.clone()), span)?;
                let called = match f.is_parsed(&called) {
                    true => f.box_parsed(&called),
                    false => called,
                };
                let call_from = f.cur_label.clone();
                f.line_fmt(format_args!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line_fmt(format_args!(
                    "{t} = phi %KValue [ {failed}, %{fail_from} ], [ {called}, %{call_from} ]"
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

    /// Where two adjacent arguments read one map's entry at one position,
    /// `builtin_key_at m i` and then `builtin_value_at m i`, the position of
    /// the first. Both names are the library's own and resolve nowhere else.
    fn both_entries(&self, f: &FnEmit, args: &[Expr]) -> Option<usize> {
        let at = args.windows(2).position(|pair| {
            let key = entry_of(&pair[0], "builtin_key_at");
            key.is_some() && key == entry_of(&pair[1], "builtin_value_at")
        })?;
        let (m, i) = entry_of(&args[at], "builtin_key_at")?;
        (f.lookup(m).is_some() && f.lookup(i).is_some()).then_some(at)
    }

    /// `key_at m i` and `value_at m i` in one call, for `both_entries`.
    fn emit_entry_pair(
        &mut self,
        f: &mut FnEmit,
        key_call: &Expr,
    ) -> Result<(String, String), String> {
        let Expr::App { args, .. } = key_call else { unreachable!("both_entries matched a call") };
        let m = self.emit_expr(f, &args[0])?;
        let m = self.maybe_force(f, m);
        let i = self.emit_expr(f, &args[1])?;
        let i = self.maybe_force(f, i);
        let (ms, is) = (f.set_of(&m), f.set_of(&i));
        let slot = f.tmp();
        f.line_fmt(format_args!("{slot} = alloca %KValue, align 8"));
        let k = f.tmp();
        f.line_fmt(format_args!(
            "{k} = call %KValue @k_b_entry_pair_fast(%KValue {m}, %KValue {i}, ptr {slot})"
        ));
        let v = f.tmp();
        f.line_fmt(format_args!("{v} = load %KValue, ptr {slot}, align 8"));
        f.record(&k, infer::builtin_set("key_at", &[ms, is]));
        f.record(&v, infer::builtin_set("value_at", &[ms, is]));
        Ok((k.to_string(), v.to_string()))
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
            self.stamp_site(f, span);
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
        if first.is_none() {
            if let Some(t) = self.emit_fused_builtin(f, name, args, &first, span)? {
                return Ok(t);
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
        let entries = match columns {
            None => self.both_entries(f, &args[skip..]).map(|k| k + skip),
            Some(_) => None,
        };
        let mut value_of_the_entry: Option<String> = None;
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
                _ if entries == Some(at) => {
                    let (k, v) = self.emit_entry_pair(f, arg)?;
                    value_of_the_entry = Some(v);
                    emitted.push(k);
                }
                _ if entries.map(|k| k + 1) == Some(at) => {
                    emitted.push(value_of_the_entry.take().expect("the key came first"));
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
            self.stamp_site(f, span);
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
            self.stamp_site(f, span);
            f.line_fmt(format_args!("{t} = call %KValue @k_desc_{name}(%KValue {})", emitted[0]));
            f.record(&t, DESC | (f.set_of(&emitted[0]) & FAIL));
            return Ok(t.to_string());
        }
        if let Some(id) = self.constructs(name) {
            if let Some(parent) = self.sub_parents.get(name).cloned() {
                if emitted.len() != 1 {
                    return Err(format!("native backend: `{name}` wraps one value"));
                }
                let inner = self.maybe_force(f, emitted[0].clone());
                let want = self.sub_want(&parent)?;
                let (tyn, _) = self.intern(&format!("{name}\0"));
                let (par, _) = self.intern(&format!("{parent}\0"));
                let t = f.tmp();
                self.stamp_site(f, span);
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
            return self.emit_declared_call(f, name, args, &emitted, region, span);
        }
        // Declared, but at no arity this call can reach. The interpreter
        // reports it when the call runs, so native reports the same words at
        // the same moment rather than refusing to build a program the oracle
        // executes.
        if !was_builtin && self.declares(name) {
            let msg =
                format!("no overload of `{}` matches these arguments", crate::ast::spoken(name));
            let (m, _) = self.intern(&format!("{msg}\0"));
            self.stamp_site(f, span);
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
                assume_length(f, &len);
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
            } else if name == "delete" && in_place {
                // the key comes out of the map nobody else holds, where a
                // shared one is copied without it
                "delete_mut"
            } else if name == "append_escaped" && in_place {
                // a site linearity proved unique writes into the builder it
                // was handed, as an in-place `append` does
                "append_escaped_mut"
            } else if name == "append_key" && in_place {
                // a shared clean token is claimed and written inline
                "append_key_fast"
            } else if name == "append_string" && in_place {
                "append_string_fast"
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
            } else if name == "number_span" {
                // the scan is the work; the two failure guards and the two
                // tag tests in front of it were paid on every number a
                // decode reads, 417,483 on the run program. The twin tests
                // the tags and hands the scan a pointer, a length and a
                // position.
                "number_span_fast"
            } else if name == "plain_string" {
                // the scan and the utf8 of what it found, in one call: the
                // decoder's string tokens went through a span, a dispatch on
                // the byte the span stopped at, and a second door for the
                // utf8. The twin tests the tags and runs both.
                "plain_string_fast"
            } else if name == "entry_count" {
                "entry_count_fast"
            } else if name == "key_at" {
                // a map's sorted view is `[k v k v ...]`: a bounds test and a
                // load where the view is built, the C where it is not
                "key_at_fast"
            } else if name == "value_at" {
                "value_at_fast"
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
            // a twin stores the site on the branch that reaches the C entry,
            // so the call that takes the fast path pays nothing for it
            match takes_site(sym) {
                true => args_ir.push(self.origin_arg(f, span)),
                false => self.stamp_site(f, span),
            }
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

    /// The builtins whose call is fused with what feeds it: `utf8` of a slice,
    /// `length`, the three shapes of `append` and the one-argument doors. Each
    /// reads its arguments as written and answers the call, or leaves it to the
    /// general path by answering nothing.
    fn emit_fused_builtin(
        &mut self,
        f: &mut FnEmit,
        name: &Name,
        args: &[Expr],
        first: &Option<String>,
        span: Span,
    ) -> Result<Option<String>, String> {
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
                        // only a value that is not bytes stops the program,
                        // and the oracle stops it inside the wrapper
                        if sets[0] != BYTES {
                            self.stamp(f, &origin);
                        }
                        let from = self.entering_library(f, name, span);
                        let t = f.tmp();
                        // three KValues and the origin pointer want seven of
                        // the six integer registers the abi has, so the last
                        // spills and the callee reloads it; the twin tests the
                        // three tags here and hands the raw door five scalars.
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_utf8_slice_fast(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        self.left_library(f, from);
                        f.record(&t, infer::builtin_set("utf8", &[sliced]));
                        return Ok(Some(t.to_string()));
                    }
                }
            }
        }
        // `length (bytes s)` builds a view header in the arena for the only
        // purpose of reading its length, and a string's header already holds
        // that length. The twin reads it there when the tag says string and
        // takes both builtins otherwise, so a value that is not a string
        // fails or dies as it did.
        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "length" {
            if let Expr::App { head: inner_head, args: inner_args, piped: false, .. } = &args[0] {
                if let Expr::Ident(inner, _, _) = &**inner_head {
                    if inner_args.len() == 1 && self.builtin_named(inner, 1) == "bytes" {
                        let v = self.emit_expr(f, &inner_args[0])?;
                        let v = self.maybe_force(f, v);
                        let v = self.unsub(f, v);
                        let viewed = infer::builtin_set("bytes", &[f.set_of(&v)]);
                        let site = self.origin_arg(f, span);
                        let t = f.tmp();
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_byte_length_fast(%KValue {v}, {site})"
                        ));
                        f.record(&t, infer::builtin_set("length", &[viewed]));
                        return Ok(Some(t.to_string()));
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
                        // only an accumulator that is not bytes stops the program
                        if f.set_of(&acc) != BYTES {
                            self.stamp_site(f, span);
                        }
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_append_slice_fast(%KValue {acc}, %KValue {}, %KValue {}, %KValue {}, i64 {})",
                            parts[0], parts[1], parts[2], i64::from(mutate)
                        ));
                        f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), sliced]));
                        return Ok(Some(t.to_string()));
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
                        // the wrapper's own line, as for `utf8` above: the
                        // oracle calls the wrapper, and the err is born there
                        let origin = match name.starts_with("builtin_") {
                            true => self.origin_arg(f, span),
                            false => match self.forwarder_origin(name, 1) {
                                Some(o) => o,
                                None => self.origin_arg(f, span),
                            },
                        };
                        let from = self.entering_library(f, name, span);
                        let t = f.tmp();
                        // three KValues and the origin want seven of the six
                        // integer registers, so the last spilled at every
                        // number; the twin tests the tags and hands the raw
                        // door a pointer, a length, two positions and the
                        // origin
                        f.line_fmt(format_args!(
                            "{t} = call %KValue @k_b_{door}_slice_fast(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        self.left_library(f, from);
                        f.record(&t, infer::builtin_set(door, &[sliced]));
                        return Ok(Some(t.to_string()));
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
                    return Ok(Some(t.to_string()));
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
                    // only an accumulator that is not bytes stops the program
                    if f.set_of(&acc) != BYTES {
                        self.stamp_site(f, span);
                    }
                    f.line_fmt(format_args!(
                        "{t} = call %KValue @k_b_append_mut_word(%KValue {acc}, i64 {word}, i64 {n}, ptr @{lit}, ptr @{lit}_lit)"
                    ));
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), STR]));
                    return Ok(Some(t.to_string()));
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
                            // only an accumulator that is not bytes stops the program
                            if f.set_of(&acc) != BYTES {
                                self.stamp_site(f, span);
                            }
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
                            let site = self.origin_arg(f, span);
                            f.line_fmt(format_args!(
                                "{t} = call %KValue @{sym}(%KValue {acc}, %KValue {rendered}, {site})"
                            ));
                            STR | fails
                        }
                    };
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), rendered_set]));
                    return Ok(Some(t.to_string()));
                }
            }
        }
        Ok(None)
    }

    /// A call to a group the program declares at this arity: its arguments
    /// passed as the callee takes them, the bracket that frees what it leaves
    /// behind, and what inference says about the value it returns.
    fn emit_declared_call(
        &mut self,
        f: &mut FnEmit,
        name: &str,
        args: &[Expr],
        emitted: &[String],
        region: bool,
        span: Span,
    ) -> Result<String, String> {
        let n = emitted.len();
        let args_ir: Vec<String> = emitted
            .iter()
            .enumerate()
            .map(|(i, e)| self.call_arg(f, name, n, i, e, args.get(i)))
            .collect();
        let callee_ret = self.ret_ty(name, n);
        // A register-returned record comes back as three raw field words,
        // not a tagged value, and both pops read a KValue. Reinterpreting
        // one would hand them a pair of fields to treat as a tag and a
        // payload, so the frontier goes unmarked instead: these calls give
        // up the rewind rather than get it wrong.
        let register_returned = callee_ret == "%parsed";
        let beat_entry = self.beat.ids.contains_key(&(name.to_string(), n)) && !register_returned;
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
        let crosses_down =
            self.group_named(name).first().is_some_and(|&i| *self.program.fns[i].file != *f.file);
        let cohort_entry = !beat_entry
            && !region
            && !register_returned
            && crosses_down
            && (!self.cycle_reached.contains(f.group.as_str())
                || (self.reaches_cycle.contains(name)
                    && self.group_return_set(name, n) & heapish == 0))
            && !f.synthetic
            && !caller_loops
            && emitted.iter().all(|e| f.set_of(e) & arg_heapish == 0);
        // The twin's arguments are rendered here too, below the mark the
        // push sets, and a slot that is not a word reuses the general
        // rendering. Rendered after the push, a string builder's seed sat
        // above the mark, and the string it grew was evacuated whole at
        // the pop: 55,136 bytes for basket's document.
        let no_pack: Vec<Option<String>> = vec![None; n];
        let twin = self.twin_test(f, name, n, emitted, &no_pack);
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
        if self.can_miss_with(f, name, n, emitted) {
            self.stamp_site(f, span);
        }
        let from = self.entering_library(f, name, span);
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
        self.left_library(f, from);
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
        // an insisting form names its caller's line (ruled on #1975)
        let result = match name.ends_with('!') && callee_ret == "%KValue" {
            true => {
                let site = self.insisted_site(f, name, n, span);
                let w = f.tmp();
                f.line_fmt(format_args!(
                    "{w} = call %KValue @k_b_insisted(%KValue {result}, {site})"
                ));
                w
            }
            false => result,
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
        Ok(result.to_string())
    }
}
