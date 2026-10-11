// The dispatchers Backend emits for a group of arms: the switch shapes
// a group compiles to when its arms discriminate on one parameter, the
// type-switch dispatcher and the arm cascade. codegen.rs reads this file
// with include!, so the methods stay in the codegen module and the compiler
// builds to the same instructions it did when they sat in codegen.rs's one
// impl block (kanso#1848). cargo fmt does not reach an included file, so
// CI's format job checks the files under src/'s subdirectories itself.

impl<'a> Backend<'a> {
    /// A group whose arms discriminate on one parameter with int/none literals
    /// (other params generic) compiles to a switch instead of an arm cascade.
    fn switch_shape(decls: &[&FnDecl]) -> Option<usize> {
        let arity = decls[0].params.len();
        if arity == 0 {
            return None;
        }
        let mut disc: Option<usize> = None;
        let mut int_arms = 0;
        let mut nullary_arms = 0;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    // a case label is a word, so a literal past one keeps the
                    // group on the cascade, which compares it whole
                    Pattern::IntLit(n, _) if i64::try_from(n).is_err() => return None,
                    Pattern::IntLit(..) | Pattern::Nullary(..) => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                        match matches!(pattern, Pattern::IntLit(..)) {
                            true => int_arms += 1,
                            false => nullary_arms += 1,
                        }
                    }
                    _ => return None,
                }
            }
        }
        // Two int arms, or one against a generic tail and nothing else — the
        // shape every counted recursion is written in, `fn f acc 0` beside
        // `fn f acc n`.
        let switchable = int_arms >= 2 || (int_arms == 1 && nullary_arms == 0);
        match (disc, switchable) {
            (Some(d), true) => Some(d),
            _ => None,
        }
    }

    /// What an arm's discriminating pattern tests, when a switch on the tag can
    /// express it. `None` means the arm cannot be a case: it wants a guard no
    /// tag names, it admits everything, or this backend cannot say what it
    /// matches.
    fn arm_case(&self, p: &Pattern) -> Option<ArmCase> {
        match p {
            Pattern::Nullary(nm, _) => Some(ArmCase::Tags(vec![match nm.as_str() {
                "true" => K_TRUE,
                "false" => K_FALSE,
                "done" => K_DONE,
                _ => K_NONE,
            }])),
            // A marker's bare mention is its value, so it binds nothing and
            // the whole of it is the record check. A ctor with fields wants
            // bindings the arm bodies below do not make.
            Pattern::Ctor { ty, fields, whole } => match fields.is_empty() && whole.is_none() {
                true => Some(ArmCase::Rec(*self.type_ids.get(ty.as_str())?, 0)),
                false => None,
            },
            Pattern::Annotated { ty, .. } => {
                // These answer before the guard and typeset logic below
                // them in the cascade, so they answer before it here too.
                if crate::ast::is_effect_type(ty) {
                    return Some(ArmCase::Tags(vec![K_DESC]));
                }
                if ty.ends_with("[]") {
                    return Some(ArmCase::Tags(vec![9]));
                }
                if ty.contains('[') {
                    return Some(ArmCase::Tags(vec![10]));
                }
                // A typeset matches when any member does, which is not a
                // tag, so not a case.
                if self.typesets.contains_key(ty.as_str()) {
                    return None;
                }
                match ty.as_str() {
                    "int" => Some(ArmCase::Tags(vec![0, K_BIG])),
                    "float64" => Some(ArmCase::Tags(vec![1])),
                    "string" => Some(ArmCase::Tags(vec![6])),
                    "bool" => Some(ArmCase::Tags(vec![K_TRUE, K_FALSE])),
                    "none" => Some(ArmCase::Tags(vec![K_NONE])),
                    "done" => Some(ArmCase::Tags(vec![K_DONE])),
                    // `some` is every tag but none and err, which a default
                    // expresses and a case does not.
                    "some" => None,
                    other => Some(ArmCase::Rec(
                        *self.type_ids.get(other)?,
                        self.field_count(other).ok()?,
                    )),
                }
            }
            _ => None,
        }
    }

    /// The set a discriminator carries inside one arm of a switch dispatcher:
    /// the tags the arm's case names, as inference spells them. `None` for an
    /// arm the default reaches, whose value the switch proved nothing about.
    fn arm_tags_set(&self, p: &Pattern, by_tag: bool) -> Option<Set> {
        let tag_set = |t: i64| match t {
            0 => INT,
            1 => FLOAT,
            2 => infer::TRUE,
            3 => infer::FALSE,
            4 => NONE,
            6 => STR,
            16 => infer::DONE,
            17 => infer::BIG,
            9 => LIST,
            10 => MAP,
            _ => TOP,
        };
        match by_tag {
            true => match self.arm_case(p)? {
                ArmCase::Tags(tags) => Some(tags.into_iter().fold(0, |acc, t| acc | tag_set(t))),
                ArmCase::Rec(..) => Some(REC),
            },
            false => match p {
                Pattern::IntLit(n, _) => Some(match i64::try_from(n) {
                    Ok(_) => INT,
                    Err(_) => infer::BIG,
                }),
                Pattern::Nullary(nm, _) => Some(match nm.as_str() {
                    "true" => infer::TRUE,
                    "false" => infer::FALSE,
                    "done" => infer::DONE,
                    _ => NONE,
                }),
                _ => None,
            },
        }
    }

    /// A group whose arms discriminate on one parameter by the kind of value it
    /// is compiles to a switch on the tag instead of a cascade of checks. The
    /// cascade tests arms in order, so the switch is only the same program when
    /// no two arms can match the same value: every arm names a set of tags,
    /// those sets are pairwise disjoint, and at most one arm — the last — is
    /// generic and stands as the default.
    ///
    /// Records are the exception that needs no disjointness. They all carry tag
    /// 7, so they chain inside that one case in source order, which is the
    /// order the cascade would have tried them in.
    fn tag_switch_shape(&self, decls: &[&FnDecl]) -> Option<usize> {
        // A subtype is a K_SUB wrapper whose own tag is 15 whatever it holds,
        // and the cascade's checks see through it. A switch on the tag would
        // send every subtype value to the default.
        if !self.sub_parents.is_empty() {
            return None;
        }
        let arity = decls[0].params.len();
        if arity == 0 {
            return None;
        }
        let mut disc: Option<usize> = None;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    _ => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                    }
                }
            }
        }
        let disc = disc?;
        let mut claimed: Vec<i64> = Vec::new();
        let mut cases = 0;
        for (k, decl) in decls.iter().enumerate() {
            match &decl.params[disc] {
                Pattern::Var(..) | Pattern::Wildcard(..) => {
                    if k + 1 != decls.len() {
                        return None;
                    }
                }
                p => {
                    match self.arm_case(p)? {
                        ArmCase::Tags(tags) => {
                            for t in tags {
                                if t == 7 || claimed.contains(&t) {
                                    return None;
                                }
                                claimed.push(t);
                            }
                        }
                        ArmCase::Rec(..) => {
                            if !claimed.contains(&7) {
                                claimed.push(7);
                            }
                        }
                    }
                    cases += 1;
                }
            }
        }
        match cases >= 2 {
            true => Some(disc),
            false => None,
        }
    }

    fn emit_switch_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
        disc: usize,
        by_tag: bool,
    ) -> Result<(), String> {
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new(!self.inline_helpers);
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let sym_hdr = match self.emitting_twin(name, arity) {
            true => tsym(name, arity),
            false => dsym(name, arity),
        };
        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };
        let header = format!("define tailcc {ret} @{sym_hdr}({}){apart} {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        self.emit_reader_hole(&mut f, name, arity)?;
        self.record_param_sets(&mut f, name, arity);
        if f.general.is_some() {
            f.arms_effects = Some(f.effects);
        }
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
                    f.line_fmt(format_args!("{t} = and i1 {prev}, {ok}"));
                    t.to_string()
                }
            });
        }
        let dispatch = f.label();
        if let Some(ok) = all_ok {
            let propagate = f.label();
            f.line_fmt(format_args!("br i1 {ok}, label %{dispatch}, label %{propagate}"));
            f.start_block(&propagate);
            for i in 0..arity {
                // NOT the folding twin. This block runs only when one of the
                // arguments IS a failure, which is exactly the case the
                // parameter sets say cannot happen -- they describe what the
                // arms accept, and a caller is free to hand over something
                // else. Asking here is the whole point of the block.
                let good = not_failure_test(&mut f, &format!("%x{i}"));
                let next = f.label();
                let ret_it = f.label();
                f.line_fmt(format_args!("br i1 {good}, label %{next}, label %{ret_it}"));
                f.start_block(&ret_it);
                let hopped = f.tmp();
                f.line_fmt(format_args!(
                    "{hopped} = call %KValue @k_err_hop(%KValue %x{i}, ptr @{hop_name})"
                ));
                self.emit_ret_failure(&mut f, name, arity, &hopped);
                f.start_block(&next);
            }
            f.line("unreachable");
        } else {
            f.line_fmt(format_args!("br label %{dispatch}"));
        }
        f.start_block(&dispatch);
        let dv = format!("%x{disc}");
        let tag = inline_tag(&mut f, &dv);
        let mut arm_labels = Vec::new();
        for k in 0..decls.len() {
            arm_labels.push(format!("arm{k}"));
        }
        if by_tag {
            // One case per tag the arms name, and the arms that name none —
            // the generic one, a value the group does not answer for, and a
            // failure — share the default. The record arms live inside case 7
            // together, in the order the cascade would have tried them.
            let mut tag_cases: Vec<(i64, String)> = Vec::new();
            let mut rec_arms: Vec<(i64, usize, String)> = Vec::new();
            let mut generic_arm: Option<usize> = None;
            for (k, decl) in decls.iter().enumerate() {
                let label = arm_labels[k].clone();
                match self.arm_case(&decl.params[disc]) {
                    Some(ArmCase::Tags(tags)) => {
                        for t in tags {
                            tag_cases.push((t, label.clone()));
                        }
                    }
                    Some(ArmCase::Rec(id, nfields)) => rec_arms.push((id, nfields, label)),
                    None => generic_arm = Some(k),
                }
            }
            let generic_label = match generic_arm {
                Some(k) => format!("arm{k}"),
                None => "nomatch".to_string(),
            };
            let dflt = f.label();
            let rec7 = f.label();
            let mut cases: Vec<String> =
                tag_cases.iter().map(|(t, l)| format!("    i64 {t}, label %{l}")).collect();
            if !rec_arms.is_empty() {
                cases.push(format!("    i64 7, label %{rec7}"));
            }
            f.switch_on(&tag, &dflt, &cases);
            if !rec_arms.is_empty() {
                f.start_block(&rec7);
                for (id, nfields, label) in &rec_arms {
                    let c = f.tmp();
                    f.predicate(
                        &c,
                        format!(
                            "call i64 @k_check_rec_fast(%KValue {dv}, i64 {id}, i64 {nfields})"
                        ),
                    );
                    let b = f.tmp();
                    f.line_fmt(format_args!("{b} = icmp ne i64 {c}, 0"));
                    let next = f.label();
                    f.line_fmt(format_args!("br i1 {b}, label %{label}, label %{next}"));
                    f.start_block(&next);
                }
                f.line_fmt(format_args!("br label %{dflt}"));
            }
            f.start_block(&dflt);
            // With no generic arm both sides of the test are `nomatch`, so
            // there is nothing to ask.
            match generic_arm {
                None => f.line("br label %nomatch"),
                Some(_) => {
                    let disc_ok = inline_not_failure(&mut f, &dv);
                    f.line_fmt(format_args!(
                        "br i1 {disc_ok}, label %{generic_label}, label %nomatch"
                    ));
                }
            }
        } else {
            // classify arms
            let mut int_cases: Vec<(String, String)> = Vec::new();
            let mut nullary_cases: Vec<(i64, String)> = Vec::new();
            let mut generic_arm: Option<usize> = None;
            for (k, decl) in decls.iter().enumerate() {
                let label = arm_labels[k].clone();
                match &decl.params[disc] {
                    Pattern::IntLit(n, _) => int_cases.push((n.to_string(), label)),
                    Pattern::Nullary(nm, _) => {
                        let t = match nm.as_str() {
                            "true" => K_TRUE,
                            "false" => K_FALSE,
                            "done" => K_DONE,
                            _ => K_NONE,
                        };
                        nullary_cases.push((t, label));
                    }
                    _ => generic_arm = Some(k),
                }
            }
            let generic_label = match generic_arm {
                Some(k) => format!("arm{k}"),
                None => "nomatch".to_string(),
            };
            // A byte discriminator crossed as a raw i64 — the byte, or 256 for
            // none — and `rebox_params` rebuilt a KValue from it for the tree below
            // to take apart again. The comment there says the round trip folds back
            // into a raw switch. It does not: `str_char`'s loop spends seven
            // instructions a byte on `cmp $0x100 / sete / cmove / shl` before it
            // reaches its first arm, and every byte-dispatching function in the
            // json decoder pays the same. Switch on the raw value instead, with 256
            // standing for the `none` arm. The rebox stays emitted for the arms
            // whose bodies read the byte as a value; where none do it is dead and
            // the optimiser drops it, and where some do it sinks into those arms.
            //
            // Only when every nullary arm is `none`: a byte is never `true` or
            // `false`, so a group naming one is not the shape this describes.
            //
            // And only when every int arm is a byte. 256 is the sentinel this
            // switch reads as `none`, so a group that writes `fn kind 256` would
            // send a read past the end of a byte string to that arm — an arm no
            // byte can ever reach. The boxed tree below is immune, because it
            // tests the tag before it looks at the payload. A literal outside
            // 0..255 keeps its group on the boxed path, where it stays dead the
            // way the oracle says it is.
            //
            // The divergence this was found by is no longer reachable: it needed
            // a group with a 256 arm and no `none` arm, and the exhaustiveness
            // check refuses that call now. What remains is the group carrying
            // both, where dropping this clause writes `i64 256` twice into one
            // switch and clang refuses the module. Either way the guard is what
            // keeps the two apart; tests/a_byte_arm_no_byte_can_reach.rs pins it.
            let raw_switchable = self.is_byte_disc(name, arity, disc)
                && nullary_cases.iter().all(|(t, _)| *t == K_NONE)
                && int_cases
                    .iter()
                    .all(|(n, _)| n.parse::<i128>().is_ok_and(|v| (0..=255).contains(&v)));
            if raw_switchable {
                let mut cases: Vec<String> =
                    int_cases.iter().map(|(n, l)| format!("    i64 {n}, label %{l}")).collect();
                for (_, l) in &nullary_cases {
                    cases.push(format!("    i64 256, label %{l}"));
                }
                f.switch_on(&format!("%x{disc}r"), &generic_label, &cases);
            } else {
                let is_int = f.tmp();
                f.line_fmt(format_args!("{is_int} = icmp eq i64 {tag}, 0"));
                let int_block = f.label();
                let not_int = f.label();
                f.line_fmt(format_args!("br i1 {is_int}, label %{int_block}, label %{not_int}"));
                f.start_block(&int_block);
                let payload = inline_payload(&mut f, &dv);
                let cases: Vec<String> =
                    int_cases.iter().map(|(n, l)| format!("    i64 {n}, label %{l}")).collect();
                f.switch_on(&payload, &generic_label, &cases);
                f.start_block(&not_int);
                // nullary tags, then generic (non-failure) or propagation
                for (t, l) in &nullary_cases {
                    let hit = f.tmp();
                    f.line_fmt(format_args!("{hit} = icmp eq i64 {tag}, {t}"));
                    let next = f.label();
                    f.line_fmt(format_args!("br i1 {hit}, label %{l}, label %{next}"));
                    f.start_block(&next);
                }
                let disc_ok = inline_not_failure(&mut f, &dv);
                let nomatch = "nomatch".to_string();
                f.line_fmt(format_args!(
                    "br i1 {disc_ok}, label %{generic_label}, label %{nomatch}"
                ));
            }
        }

        // A switch whose cases cover every value the discriminator can hold
        // never falls to `nomatch`, and then the failure path is blocks
        // nothing reaches, the way a dispatcher's `fail` is.
        if branches_to(&f.out, "nomatch") {
            f.start_block("nomatch");
            // No arm matched, and the discriminator is the only possible
            // failure here. A none is a value, not a failure (ruled
            // 2026-07-24), so it is refused like any other value no arm takes.
            let disc_fail = f.tmp();
            f.line_fmt(format_args!("{disc_fail} = extractvalue %KValue {dv}, 0"));
            let failing = f.tmp();
            f.line_fmt(format_args!("{failing} = icmp eq i64 {disc_fail}, 5"));
            let ret_disc = f.label();
            let die = f.label();
            f.line_fmt(format_args!("br i1 {failing}, label %{ret_disc}, label %{die}"));
            f.start_block(&ret_disc);
            let hopped = f.tmp();
            f.line_fmt(format_args!(
                "{hopped} = call %KValue @k_err_hop(%KValue {dv}, ptr @{hop_name})"
            ));
            self.emit_ret_failure(&mut f, name, arity, &hopped);
            f.start_block(&die);
            let msg =
                format!("no overload of `{}` matches these arguments\0", crate::ast::spoken(name));
            let (m, _len) = self.intern(&msg);
            f.line_fmt(format_args!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
        }
        // arm bodies: patterns are known matched, only bind generics
        for (k, decl) in decls.iter().enumerate() {
            f.start_block(&arm_labels[k]);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            f.hako = crate::provenance::package_of(&decl.file).to_string();
            f.file = decl.file.clone();
            f.synthetic = decl.synthetic;
            for (i, pattern) in decl.params.iter().enumerate() {
                // The switch has already decided what the value is, so an
                // annotation here is only a name for it.
                match pattern {
                    Pattern::Var(pname, _) | Pattern::Annotated { name: pname, .. } => {
                        f.bind(pname, &format!("%x{i}"))
                    }
                    _ => {}
                }
            }
            // And the body is told what the switch decided. Until 2026-09-09
            // the discriminator kept the whole group's set inside every arm,
            // so `n:int`'s body forced `n` again, asked whether a user
            // to_string arm could claim it, and took the generic door on
            // every builtin it handed `n` to. The case that reached this arm
            // is the proof, so the set inside it is the case's tags.
            let narrowed = self.arm_tags_set(&decl.params[disc], by_tag);
            let whole = f.set_of(&format!("%x{disc}"));
            if let Some(set) = narrowed {
                f.record(&format!("%x{disc}"), set);
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            f.record(&format!("%x{disc}"), whole);
        }
        let text = f.body();
        self.room(header.len() + text.len() + 4);
        let _ = writeln!(
            self.body,
            "{header}
{text}}}
"
        );
        Ok(())
    }

    /// The check call accepting one named type (chain-aware when the
    /// program declares subtypes). Shared by concrete annotations and
    /// typeset members.
    fn type_check_call(&self, value: &str, ty: &str) -> Result<String, String> {
        let subs = !self.sub_parents.is_empty();
        Ok(match ty {
            // a box is a box whatever it yields, and never a subtype
            t if crate::ast::is_effect_type(t) => {
                format!("call i64 @k_check_tag(%KValue {value}, i64 {K_DESC})")
            }
            "int" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 0)"),
            "int" => format!("call i64 @k_check_int_type(%KValue {value})"),
            "float64" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 1)"),
            "float64" => format!("call i64 @k_check_tag(%KValue {value}, i64 1)"),
            "string" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 6)"),
            "string" => format!("call i64 @k_check_tag(%KValue {value}, i64 6)"),
            "bool" if subs => format!("call i64 @k_check_sub_bool(%KValue {value})"),
            "bool" => format!("call i64 @k_check_bool(%KValue {value})"),
            "err" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"),
            "none" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_NONE})"),
            "done" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 {K_DONE})"),
            "done" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_DONE})"),
            // `some` is any value that is not none, and a failure is not a
            // value: without this arm the backend refused the annotation
            // outright, where the interpreter took it and the checker had
            // already passed the program.
            "some" => format!("call i64 @k_check_some(%KValue {value})"),
            other => match self.type_ids.get(other) {
                Some(id) if self.sub_parents.contains_key(other) => {
                    format!("call i64 @k_check_sub_id(%KValue {value}, i64 {id})")
                }
                Some(id) => {
                    let nfields = self.field_count(other)?;
                    match subs {
                        true => format!(
                            "call i64 @k_check_sub_rec(%KValue {value}, i64 {id}, i64 {nfields})"
                        ),
                        false => format!(
                            "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {nfields})"
                        ),
                    }
                }
                None => return Err(format!("native backend: unknown type `{other}`")),
            },
        })
    }

    /// The runtime's `want` encoding for a chain target: a declared
    /// type's id, or -(tag + 1) for a primitive.
    fn sub_want(&self, ty: &str) -> Result<i64, String> {
        Ok(match ty {
            "int" => -1,
            "float64" => -2,
            "string" => -7,
            "err" => -(K_ERR + 1),
            "done" => -(K_DONE + 1),
            // the runtime's K_WANT_BOOL and K_WANT_SOME: neither is one tag
            "bool" => -100,
            "some" => -101,
            other => match self.type_ids.get(other) {
                Some(id) => *id,
                None => return Err(format!("native backend: unknown type `{other}`")),
            },
        })
    }

    fn emit_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
    ) -> Result<(), String> {
        if arity == 0 && decls.len() == 1 && self.is_constant_body(decls[0]) {
            return self.emit_frozen_constant(name, decls);
        }
        self.emit_dispatcher_as(&dsym(name, arity), name, arity, decls)
    }

    /// A zero-argument definition is a constant, and a constant is built once:
    /// the body emits unchanged under a build symbol and the real symbol
    /// becomes a cache in front of it, filled on first demand. Until
    /// 2026-09-07 only a body that was a literal froze, and every other
    /// constant -- a table concatenated from literals, a map built by a call
    /// -- was recomputed at every mention: sha256's sixty-four round
    /// constants were concatenated from eleven literal lists 16,000 times a
    /// run on runbench. The interpreter computes every constant once behind a
    /// knot cell, so recomputing was a divergence from the oracle in cost,
    /// and for a body with an effect in count as well. The literal rule dated
    /// from when the cells were filled before main, where a body that could
    /// fail would have failed early; a freeze on first demand fails exactly
    /// where the interpreter does.
    /// A knotted constant is the case that cannot be left unfrozen whatever
    /// the rule: unfrozen, the real symbol recomputes its body, so a mention
    /// inside that body re-enters the builder and the recursion has no floor.
    fn is_constant_body(&self, _decl: &FnDecl) -> bool {
        true
    }

    fn emit_frozen_constant(&mut self, name: &str, decls: &[&FnDecl]) -> Result<(), String> {
        let sym = dsym(name, 0);
        // a module-qualified name is quoted, so the suffix goes inside the quotes
        let build = match sym.strip_suffix('"') {
            Some(head) => format!("{head}_build\""),
            None => format!("{sym}_build"),
        };
        self.emit_dispatcher_as(&build, name, 0, decls)?;
        let cell = format!("caf_{}", self.caf_cells.len());
        self.caf_cells.push(cell.clone());
        // Built on first demand, not before main. `k_caf_init` used to run
        // every builder at startup, which made an undemanded knot do its work
        // anyway — ruled 2026-08-23 to be wrong, because work defers until it
        // is presented to IO and eager evaluation is a resource heuristic
        // inside that contract rather than a semantic an engine may expose.
        //
        // The ready flag is set BEFORE the builder runs, for the same reason
        // `k_caf_init` seeded every cell before running any builder: a
        // constant that mentions itself re-enters here, and it has to find the
        // blackhole rather than the zeroed global, which is an integer zero
        // and reads as one. That seeding is what makes the cycle finite.
        //
        // One branch, taken once. The alternative Clay named is update in
        // place — rewriting the indirection at first evaluation so later reads
        // check nothing — and it is the better shape if this costs anything
        // measurable. The number goes to the ledger before any freeze returns.
        self.room(sym.len() + 4 * cell.len() + build.len() + 640);
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{sym}() {{\n\
             entry:\n  \
               %r = load i8, ptr @{cell}_ready\n  \
               %is = icmp eq i8 %r, 0\n  \
               br i1 %is, label %build, label %ready\n\
             build:\n  \
               %b = call %KValue @k_caf_blackhole()\n  \
               store %KValue %b, ptr @{cell}\n  \
               store i8 1, ptr @{cell}_ready\n  \
               %v = call tailcc %KValue @{build}()\n  \
               %f = call %KValue @k_caf_complete(%KValue %v, %KValue %b)\n  \
               store %KValue %f, ptr @{cell}\n  \
               ret %KValue %f\n\
             ready:\n  \
               %c = load %KValue, ptr @{cell}\n  \
               ret %KValue %c\n\
             }}\n"
        );
        Ok(())
    }

    /// The second hole in an err's infectiousness: a reader's getter answers
    /// the piece at the dispatcher's entry, before any arm is tried, which is
    /// where the other two engines answer it too. A getter's parameter is
    /// boxed and its body is a bare binder, so the read needs no convention
    /// of its own — and a reader group that grew a `%parsed` return would be
    /// one this hole cannot serve, so that is refused rather than skipped.
    fn emit_reader_hole(&mut self, f: &mut FnEmit, name: &str, arity: usize) -> Result<(), String> {
        let Some(field) = crate::ast::err_reader(name) else {
            return Ok(());
        };
        if arity != 1 {
            return Ok(());
        }
        if f.ret_ty != "%KValue" {
            return Err(format!("the reader `{name}` returns a parsed record"));
        }
        let (lit, _) = self.intern(&format!("{field}\0"));
        let is = f.tmp();
        f.line_fmt(format_args!("{is} = call i64 @k_is_err(%KValue %x0)"));
        let err = f.tmp();
        f.line_fmt(format_args!("{err} = icmp ne i64 {is}, 0"));
        let read = f.label();
        let arms = f.label();
        f.line_fmt(format_args!("br i1 {err}, label %{read}, label %{arms}"));
        f.start_block(&read);
        let got = f.tmp();
        f.line_fmt(format_args!("{got} = call %KValue @k_err_read(%KValue %x0, ptr @{lit})"));
        f.line_fmt(format_args!("ret %KValue {got}"));
        f.start_block(&arms);
        Ok(())
    }

    fn emit_dispatcher_as(
        &mut self,
        sym_hdr: &str,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
    ) -> Result<(), String> {
        if let Some(disc) = Self::switch_shape(decls) {
            return self.emit_switch_dispatcher(name, arity, decls, disc, false);
        }
        if let Some(disc) = self.tag_switch_shape(decls) {
            return self.emit_switch_dispatcher(name, arity, decls, disc, true);
        }
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new(!self.inline_helpers);
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };
        let header = format!("define tailcc {ret} @{sym_hdr}({}){apart} {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        self.emit_reader_hole(&mut f, name, arity)?;
        self.record_param_sets(&mut f, name, arity);
        // A `%parsed` and a `%KValue` share a `{i64,i64}` layout: reinterpret the
        // parameter's two words as the discriminator KValue once, so the arms can
        // match failures and the propagation loop can hop. On the failure path it
        // *is* the failure; on success its low word (value.tag | pos<<8) is never
        // a failure tag, so `k_not_failure` still separates the two.
        for i in 0..arity {
            if self.escape.carries_ty(name, arity, i).is_some() {
                f.line_fmt(format_args!("%x{i}w0 = extractvalue %parsed %x{i}, 0"));
                f.line_fmt(format_args!("%x{i}w1 = extractvalue %parsed %x{i}, 1"));
                f.line_fmt(format_args!("%x{i}sa = insertvalue %KValue undef, i64 %x{i}w0, 0"));
                f.line_fmt(format_args!("%x{i}s = insertvalue %KValue %x{i}sa, i64 %x{i}w1, 1"));
            }
        }
        // A releasable cell is created inside an arm's body, so it exists only
        // in blocks that body dominates. The next arm's blocks and the
        // parameter-failure blocks below are reached without running it, and
        // releasing it there emits a use LLVM's verifier refuses. The cells
        // an arm registers are its own: each arm starts from this watermark.
        let cells_before = f.lazy_cells.len();
        // Whether an arm left a way into the next one. An arm whose every
        // parameter check was proved away never branches to its `fail`, and
        // then the arms after it and the failure path below are blocks nothing
        // reaches: 13% of the decoder's emitted lines were such blocks, each
        // parsed by clang only to be deleted by its first pass.
        let mut open = true;
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            let arm_at = f.out.len();
            f.lazy_cells.truncate(cells_before);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            f.hako = crate::provenance::package_of(&decl.file).to_string();
            f.file = decl.file.clone();
            f.synthetic = decl.synthetic;
            for (i, pattern) in decl.params.iter().enumerate() {
                match self.escape.carries_ty(name, arity, i) {
                    Some(ty) => {
                        let ty = ty.to_string();
                        self.emit_parsed_pattern(&mut f, &format!("%x{i}s"), pattern, &fail, &ty)?;
                    }
                    None => {
                        let known = self.own_param_set(name, arity, i);
                        self.emit_pattern_known(&mut f, &format!("%x{i}"), pattern, &fail, known)?;
                    }
                }
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            if !branches_to(&f.out[arm_at..], &fail) {
                open = false;
                break;
            }
            f.start_block(&fail);
        }
        f.lazy_cells.truncate(cells_before);
        if !open {
            let text = f.body();
            self.room(header.len() + text.len() + 4);
            let _ = writeln!(self.body, "{header}\n{text}}}\n");
            return Ok(());
        }
        for i in 0..arity {
            let val = match self.escape.carries_ty(name, arity, i).is_some() {
                true => format!("%x{i}s"),
                false => format!("%x{i}"),
            };
            let ok = inline_not_failure(&mut f, &val);
            let ret_label = f.label();
            let next = f.label();
            f.line_fmt(format_args!("br i1 {ok}, label %{next}, label %{ret_label}"));
            f.start_block(&ret_label);
            let hopped = f.tmp();
            f.line_fmt(format_args!(
                "{hopped} = call %KValue @k_err_hop(%KValue {val}, ptr @{hop_name})"
            ));
            self.emit_ret_failure(&mut f, name, arity, &hopped);
            f.start_block(&next);
        }
        // a getter that matched nothing is a field error to the reader, and
        // only the runtime can name the value it was handed
        match crate::ast::getter_field(name) {
            Some(field) if arity == 1 => {
                let (lit, _len) = self.intern(&format!("{field}\0"));
                // a getter never takes the by-value convention, so its
                // parameter is already an ordinary value here
                if self.ret_ty(name, arity) == "%parsed" {
                    f.line_fmt(format_args!("call void @k_no_field(%KValue %x0, ptr @{lit})"));
                } else {
                    let got = f.tmp();
                    f.line_fmt(format_args!(
                        "{got} = call %KValue @k_field_forced(%KValue %x0, ptr @{lit})"
                    ));
                    self.emit_ret(&mut f, &got);
                }
            }
            _ => {
                let msg = format!(
                    "no overload of `{}` matches these arguments",
                    crate::ast::spoken(name)
                );
                let (m, _len) = self.intern(&format!("{msg}\0"));
                f.line_fmt(format_args!("call void @k_die(ptr @{m})"));
            }
        }
        f.line("unreachable");
        let text = f.body();
        self.room(header.len() + text.len() + 4);
        let _ = writeln!(self.body, "{header}\n{text}}}\n");
        Ok(())
    }
}
