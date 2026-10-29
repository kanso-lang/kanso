// Calls: applying a value, finding the arms a name stands for, and the
// dispatch that picks one.
impl<'a> Interp<'a> {
    fn call(&self, callee: Value, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        match callee {
            Value::FnRef(name) => self.call_ref(&name, args, span, frame),
            Value::Closure(closure) => self.call_closure(&closure, args, span),
            Value::TableFn(handle) => foreign_call(handle, args, span, false),
            Value::Partial(callee, supplied) => {
                // `()` runs a partial rather than supplying more to it, so an
                // empty argument list is the call form and never an
                // accumulation of nothing. A partial still short of every arm
                // has nothing to run, and says how many it is short by.
                match args.is_empty() {
                    true => {
                        self.run_partial(callee.as_ref().clone(), supplied.as_ref(), span, frame)
                    }
                    false => {
                        let mut all = supplied.as_ref().clone();
                        all.extend(args);
                        self.apply_partial(callee.as_ref().clone(), all, span, frame)
                    }
                }
            }
            bad if is_failure(&bad) => Ok(bad),
            other => Err(RuntimeError {
                at: None,
                message: format!("`{}` is not callable", render(self, &other, false)),
                span,
            }),
        }
    }

    /// Every argument count the callee answers, smallest first.
    fn arities_of(&self, callee: &Value) -> Vec<usize> {
        match callee {
            // A reference answers by what it calls. A constructor takes its
            // fields, a subtype's takes the one value it wraps, and a builtin
            // takes the count the checker holds for it. Answering only for
            // groups left `(&pt 2) "y"` and `(&length) [1 2]` growing into
            // partials that printed `<fn>` where native ran `length`.
            Value::FnRef(name) => match self.callee_of_ref(name) {
                Callee::Group(decls) => {
                    let mut found: Vec<usize> = decls.iter().map(|d| d.params.len()).collect();
                    found.sort_unstable();
                    found.dedup();
                    found
                }
                Callee::Constructor(ty) if ty.parent.is_some() => vec![1],
                Callee::Constructor(ty) if ty.members.is_empty() => vec![ty.fields.len()],
                Callee::Constructor(_) => Vec::new(),
                Callee::EntryType => vec![self.entry_decl.fields.len()],
                Callee::Err => vec![1],
                // `if` is the one builtin the checker's table leaves out, since
                // its count is checked where its branches are; held, it takes
                // the condition and both branches like any call of it.
                Callee::Builtin if &**name == "if" => vec![3],
                Callee::Builtin => crate::check::builtin_arity(name).into_iter().collect(),
            },
            Value::Closure(c) => vec![c.params.len()],
            // A partial answers what its callee answers, less what it holds.
            // Answering nothing left `(&h 3) 1` over `h = &g p` wrapped in a
            // third partial where native ran `g p 3 1`, and `h 3 1` beside it
            // ran here too.
            Value::Partial(inner, held) => self
                .arities_of(inner)
                .into_iter()
                .filter(|a| *a >= held.len())
                .map(|a| a - held.len())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// `foo()` on a partial. Complete, it is the call the partial was built
    /// for; short, it is an arity error naming what is still missing, which is
    /// the count the compiled engines report from the closure they lowered it
    /// to.
    fn run_partial(
        &self,
        callee: Value,
        supplied: &[Value],
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        let arities = self.arities_of(&callee);
        if arities.contains(&supplied.len()) {
            return self.call(callee, supplied.to_vec(), span, frame);
        }
        let waiting = arities.iter().filter(|a| **a > supplied.len()).min().copied();
        let short = waiting.map(|a| a - supplied.len()).unwrap_or(0);
        Err(RuntimeError {
            at: None,
            message: format!("this function takes {short} argument(s), got 0"),
            span,
        })
    }

    fn call_closure(&self, closure: &ClosureData, args: Vec<Value>, span: Span) -> EvalResult {
        let running = self.running.get();
        let answer = self.call_closure_inner(closure, args, span);
        self.running.set(running);
        answer
    }

    fn call_closure_inner(
        &self,
        closure: &ClosureData,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let mut flow = self.closure_flow(closure, args, span)?;
        // the lambda whose body wrote the call being made, which places a
        // call that fails before anything it calls has run
        let mut writer: Option<Rc<ClosureData>> = None;
        let frame_of = |w: &Option<Rc<ClosureData>>| match w {
            Some(c) => c.frame.clone(),
            None => closure.frame.clone(),
        };
        loop {
            match flow {
                Flow::Done(value) => return Ok(value),
                Flow::Tail(name, group, args, span) => {
                    return self
                        .dispatch_loop(name, group, args, span)
                        .map_err(|e| placed(e, &frame_of(&writer)))
                }
                Flow::Closure(next, args, span) => {
                    flow = self
                        .closure_flow(&next, args, span)
                        .map_err(|e| placed(e, &frame_of(&writer)))?;
                    writer = Some(next);
                }
            }
        }
    }

    /// A closure's body run to its tail. What the tail hands back goes to the
    /// caller's loop: `call_closure` for a call that is not itself in tail
    /// position, the dispatcher's loop for one that is.
    fn closure_flow(
        &self,
        closure: &ClosureData,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Flow<'a>, RuntimeError> {
        // a lambda's body is on the cycle of the body that made it, as its
        // compiled function is
        self.running.set(closure.deep);
        if closure.params.len() != args.len() {
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "this function takes {} argument(s), got {}",
                    closure.params.len(),
                    args.len()
                ),
                span,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(Flow::Done(merged_failures(&args)));
        }
        let mut env = closure.env.clone();
        for (name, value) in closure.params.iter().zip(args) {
            env = bind(env, name.clone(), value);
        }
        self.eval_tail(&closure.body, &env, &closure.frame).map_err(|e| placed(e, &closure.frame))
    }

    /// Dispatch fires the moment the supplied count matches an arity the
    /// callee answers to; short of that the partial simply grows. A count
    /// past every arity is the error, and it names what was available.
    fn apply_partial(
        &self,
        callee: Value,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        let arities = self.arities_of(&callee);
        if arities.contains(&args.len()) {
            return self.call(callee, args, span, frame);
        }
        if arities.iter().all(|a| *a < args.len()) && !arities.is_empty() {
            let names: Vec<String> = arities.iter().map(usize::to_string).collect();
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "no {}-argument arm of `{}` (arms take {})",
                    args.len(),
                    render(self, &callee, false),
                    names.join(" or ")
                ),
                span,
            });
        }
        Ok(Value::Partial(Rc::new(callee), Rc::new(args)))
    }

    fn call_named(&self, name: &str, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        let callee = self.callee_named(name);
        self.call_callee(callee, name, args, span, frame)
    }

    /// What a name calls, remembered by name. The borrow is dropped before
    /// the call runs, because the body it reaches calls back in here.
    fn callee_named(&self, name: &str) -> Callee<'a> {
        let known = self.callees.borrow().get(name).cloned();
        match known {
            Some(callee) => callee,
            None => {
                let callee = self.callee(name);
                self.callees.borrow_mut().insert(name.to_string(), callee.clone());
                callee
            }
        }
    }

    /// A call through a function reference, remembered by the reference's
    /// address rather than by hashing its text. Every `Value::FnRef` is made
    /// in `resolve`, whose answer `names` keeps for the interpreter's life,
    /// so one name reaches here through one address. The entry holds its own
    /// count on the name as well, so no other name can come to live at an
    /// address the table knows. Hashing the name was about a hundred
    /// instructions of every interpreted call.
    fn call_ref(&self, name: &Rc<str>, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        let callee = self.callee_of_ref(name);
        self.call_callee(callee, name, args, span, frame)
    }

    /// What a function reference calls, by its address; see `call_ref`.
    ///
    /// The map's probe was most of what a call through a reference paid, so a
    /// direct-mapped table of the last callee per address slot answers first.
    /// A slot holds a key only while the map does, and the map pins the name,
    /// so an address in the table cannot have been handed to another name.
    fn callee_of_ref(&self, name: &Rc<str>) -> Callee<'a> {
        let key = Rc::as_ptr(name) as *const u8 as usize;
        let slot = recent_slot(in_slice(key), RECENT_CALLEES);
        if let (k, Some(callee)) = &self.recent_callees.borrow()[slot] {
            if *k == key {
                return callee.clone();
            }
        }
        self.callee_missed(name, key, slot)
    }

    /// The table did not hold the reference; kept out of line so a hit pays
    /// for no frame.
    #[inline(never)]
    fn callee_missed(&self, name: &Rc<str>, key: usize, slot: usize) -> Callee<'a> {
        let placed = Placed { at: key, stable: in_slice(key) };
        let known = self.callees_by_ref.borrow().get(&placed).map(|(_, callee)| callee.clone());
        if let Some(callee) = &known {
            self.recent_callees.borrow_mut()[slot] = (key, Some(callee.clone()));
        }
        match known {
            Some(callee) => callee,
            None => {
                let callee = self.callee_named(name);
                self.callees_by_ref.borrow_mut().insert(placed, (name.clone(), callee.clone()));
                self.recent_callees.borrow_mut()[slot] = (key, Some(callee.clone()));
                callee
            }
        }
    }

    fn call_callee(
        &self,
        callee: Callee<'a>,
        name: &str,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        if let Callee::Err = callee {
            let [reason] = arity(args, name, span)?;
            let reason = self.force_thunk(reason)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            return Ok(err_value(reason, origin_at(frame, span)));
        }
        let constructed = match callee {
            Callee::EntryType => Some(&self.entry_decl),
            Callee::Constructor(ty) => Some(ty),
            _ => None,
        };
        if let Some(ty) = constructed {
            // A constructor slot is where a knot ties: an argument still
            // being computed is stored rather than demanded, so the cell
            // completes here and the field resolves against it afterwards.
            // Demanding it instead is what made a guarded self-reference
            // blackhole through its own construction.
            let args = args
                .into_iter()
                .map(|a| match pending_knot(&a) {
                    true => Ok(a),
                    false => self.force_thunk(a),
                })
                .collect::<Result<Vec<_>, _>>()?;
            return self.construct(ty, args, span);
        }
        if let Callee::Group(overloads) = callee {
            return self.dispatch(name, &overloads, args, span);
        }
        // Forced where they lie, and only where a thunk is: `force_thunk`
        // hands any other value back as it came. The collect this replaced
        // moved every argument through a `Result` adapter, and whether rustc
        // inlined the adapter's fold moved with edits nowhere near here:
        // kanso#1621 changed only the emitter and the interpreted row rose
        // 4,189,569, all of it this one collect laid out as a call.
        let mut args = args;
        for (at, slot) in args.iter_mut().enumerate() {
            if let Value::Thunk(cell) = slot {
                // A list stores what it is handed: the item `push` adds may be
                // a `list/tie` cell still being made, as a list literal's may.
                if at == 1 && name == "push" && matches!(&*cell.borrow(), ThunkState::Blackhole) {
                    continue;
                }
                let thunk = Value::Thunk(cell.clone());
                *slot = self.force_thunk(thunk)?;
            }
        }
        self.call_builtin(name, args, span, frame)
    }

    /// The ladder `call_named` used to walk, walked once per name per run.
    /// The arms are in the order they were in, because the order decides the
    /// answer: `err` beats a type, and a type beats a group.
    fn callee(&self, name: &str) -> Callee<'a> {
        if name == "err" {
            return Callee::Err;
        }
        if name == "entry" {
            return Callee::EntryType;
        }
        if let Some(ty) = self.types.get(name).copied() {
            // An imported type enrolled under its bare name gives way to a
            // group the program declares: the author's own name wins.
            if ty.origin.is_none() || !self.fns.contains_key(name) {
                return Callee::Constructor(ty);
            }
        }
        match self.fns.get(name) {
            Some(overloads) => Callee::Group(overloads.clone()),
            None => Callee::Builtin,
        }
    }

    fn construct(&self, ty: &TypeDecl, args: Vec<Value>, span: Span) -> EvalResult {
        if !ty.members.is_empty() {
            return Err(RuntimeError {
                at: None,
                message: format!("`{}` is a typeset — it only annotates", ty.name),
                span,
            });
        }
        if let Some(parent) = &ty.parent {
            if args.len() != 1 {
                return Err(RuntimeError {
                    at: None,
                    message: format!("`{}` wraps one {} value", ty.name, parent),
                    span,
                });
            }
            let inner = args.into_iter().next().expect("one arg");
            if is_failure(&inner) {
                return Ok(inner);
            }
            if !type_matches(parent, &inner) {
                return Err(RuntimeError {
                    at: None,
                    message: format!("`{}` wraps {}", ty.name, article(parent)),
                    span,
                });
            }
            let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
            return Ok(Value::Sub { ty: Rc::from(canonical), inner: Rc::new(inner) });
        }
        if args.len() != ty.fields.len() {
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "`{}` has {} field(s), got {} (construction is positional, in the \
                     order the fields are declared)",
                    ty.name,
                    ty.fields.len(),
                    args.len()
                ),
                span,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(merged_failures(&args));
        }
        for ((field, tys, _), arg) in ty.fields.iter().zip(&args) {
            if tys.len() >= 2 && !tys.iter().any(|t| type_matches(t, arg)) {
                return Err(RuntimeError {
                    at: None,
                    message: format!("field `{field}` of `{}` takes {}", ty.name, tys.join(" ")),
                    span,
                });
            }
        }
        let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
        Ok(Value::Record { ty: Rc::from(canonical), fields: Rc::new(RefCell::new(args)) })
    }

    /// The cycle `decl` is on, one more than its number, or 0.
    #[inline(always)]
    fn deep_cycle(&self, decl: &FnDecl) -> u32 {
        let base = self.program.fns.as_ptr() as usize;
        let at = decl as *const FnDecl as usize;
        let i = at.wrapping_sub(base) / std::mem::size_of::<FnDecl>();
        match at >= base {
            true => self.deep_of.get(i).copied().unwrap_or(0),
            false => 0,
        }
    }

    fn dispatch(
        &self,
        name: &str,
        overloads: &Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        self.dispatch_loop(Rc::from(name), Rc::clone(overloads), args, span)
    }

    /// The dispatcher's trampoline: a tail call to a named group re-enters
    /// arm selection here instead of growing the Rust stack, so deep tail
    /// recursion — self or mutual — runs in constant interpreter stack, the
    /// way the native engine's musttail already made it run in constant
    /// machine stack. The oracle should not be the engine that fails first.
    fn dispatch_loop(
        &self,
        name: Rc<str>,
        overloads: Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        // Non-tail recursion nests a Rust frame per call; past this depth
        // the process would die in Rust's own overflow handler instead of
        // reporting. The message matches the one native's parent prints
        // when the compiled child takes the same fall.
        // 10k holds under debug's fat frames and release alike on the
        // 256 MB interpreter thread; recursion past it is non-portable
        // anyway (native's own ceiling is finite and smaller in wasm).
        // a call that enters a cycle growing the stack from outside it sets
        // the record an overflow names, when it is empty, and puts it back
        // on return (see `crate::deep`)
        let running = self.running.get();
        let saved = self.deep_now.get();
        // every overload of a group is on the group's cycle, so the body
        // that runs is on `to` until a tail call hops to another group
        let to = overloads.first().map_or(0, |d| self.deep_cycle(d));
        if to != 0 && to != running && saved == 0 {
            self.deep_now.set(to);
        }
        self.running.set(to);
        self.depth.set(self.depth.get() + 1);
        if self.depth.get() > 10_000 {
            self.depth.set(self.depth.get() - 1);
            // the engines run out of stack at different depths, so the place
            // is the outermost cycle still running, which they share; line 0
            // keeps it from being placed on the way out
            let at = match self.deep_now.get() {
                0 => None,
                d => Some(self.deep_places[d as usize - 1].clone()),
            };
            self.running.set(running);
            self.deep_now.set(saved);
            return Err(RuntimeError {
                at,
                message: "the program ran out of stack: recursion went deeper than the stack holds"
                    .to_string(),
                span: Span::at(0, 0),
            });
        }
        let result = self.dispatch_loop_inner(name, overloads, args, span);
        self.depth.set(self.depth.get() - 1);
        self.running.set(running);
        self.deep_now.set(saved);
        result
    }

    fn dispatch_loop_inner(
        &self,
        name: Rc<str>,
        overloads: Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let mut name = name;
        let mut overloads: Rc<Vec<&'a FnDecl>> = overloads;
        let mut args = args;
        let mut span = span;
        // THE SCORE BUFFER OUTLIVES THE DISPATCH, where the bindings buffer
        // cannot. `binds` is moved into `bind_all` and becomes the environment
        // frame, so its allocation is doing work after the dispatch ends;
        // `score` is only read to compare candidates, kept beside `best` while
        // one wins, and then dropped. Declaring it inside the loop paid one
        // allocation and one free per dispatch for a buffer nothing keeps.
        //
        // The loser's buffer is the one held here: the swap below leaves the
        // outgoing candidate's vector in `score`, so what survives the
        // iteration already has capacity and `match_params_into` clears it
        // before the next candidate fills it.
        let mut scores: [Score; 2] = Default::default();
        // AND THE BINDINGS BUFFER COMES BACK TOO, when the frame it became is
        // the dispatcher's to take back. `bind_all` moves `binds` into
        // `Env::Many`, so unlike the score it cannot simply be kept -- the body
        // needs it. What can be kept is the frame AFTER the body has finished
        // with it, and it almost always is: an instrumented build took a `Weak`
        // to the frame and tried to upgrade it after the body returned, and it
        // was dead 173,921 times out of 173,922. The one survivor is a lazy
        // thunk holding its defining environment, which is the case
        // `Rc::try_unwrap` declines below.
        // AND THE FRAME NODE COMES BACK WITH IT. kanso#1543 took the vector
        // out of the dead frame and let the `Rc` allocation go; this keeps the
        // whole node. `Rc::try_unwrap` had to destroy the node to reach the
        // vector inside it, so a dispatch that reclaimed its bindings still
        // paid `Rc::new` for a fresh `RcBox<Env>` on the next one.
        // `Rc::get_mut` reaches the same vector through a handle that stays
        // alive, so the node survives the dispatch that made it.
        //
        // The two are not interchangeable on the uniqueness they demand:
        // `try_unwrap` reads the strong count alone, `get_mut` reads the weak
        // count too. Nothing in this compiler takes a `Weak<Env>` -- the
        // instrumented build above was reverted -- so today they agree, and
        // the difference is written here rather than assumed away.
        let mut pool: Option<Rc<Env>> = None;
        // Who wrote the call being dispatched: the first is the caller's own
        // expression, which places a failure on the way out; each tail call
        // after it was written in the body that handed it here.
        let mut caller = Caller::Outside;
        loop {
            // The second hole in an err's infectiousness: a reader's getter
            // answers the piece before any arm is tried, so an own-hako err
            // and a foreign one read alike, and a getter arm never sees one.
            if let Some(field) = crate::ast::err_reader(&name) {
                if let [arg] = args.as_mut_slice() {
                    let taken = std::mem::replace(arg, Value::NoneV);
                    *arg = self.force_thunk(taken)?;
                    if let Value::Unboxed(info) = arg {
                        return Ok(err_read(info, field));
                    }
                }
            }
            let args_len = args.len();
            // A position is scrutinized when any arity-matching arm inspects
            // it (anything but a bare Var/Wildcard); thunks force before
            // matching.
            for (i, arg) in args.iter_mut().enumerate() {
                if !matches!(arg, Value::Thunk(_)) {
                    continue;
                }
                let scrutinized = overloads.iter().any(|decl| {
                    decl.params.len() == args_len
                        && !matches!(
                            decl.params.get(i),
                            Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_))
                        )
                });
                if scrutinized {
                    let taken = std::mem::replace(arg, Value::NoneV);
                    *arg = self.force_thunk(taken)?;
                }
            }
            // One pair of buffers for the whole candidate list. `match_params`
            // used to build both at `Vec::with_capacity` on every candidate and
            // give up the moment a pattern refused, so a candidate that failed
            // on its first parameter had already paid for two allocations --
            // and most candidates fail, because arm selection tries them all.
            // AND ONE PAIR PER DISPATCH, not per candidate. The swap below
            // recycles these across the candidate list, so within one dispatch
            // they allocate once; what they do not survive is the dispatch
            // itself, because the winner's pair is moved out to become the
            // environment frame and the score kept beside `best`. So every
            // dispatch starts at capacity zero and climbs 1, 2, 4 from nothing:
            // `dispatch_loop` reached `grow_one` 225,431 times against 55,711
            // dispatches, four reallocations a call. Asking for the arity up
            // front does NOT allocate less: `__rust_alloc` is called
            // 1,362,891 times against 1,361,559, up 1,332. What goes is the
            // GROWTH path -- `grow_one` and `finish_grow` fall from 464,507
            // and 489,542 to 112,013 and 137,048, 352,494 fewer of each --
            // and with it the capacity arithmetic, the doubling branch and
            // the element copy every regrow makes. 21,582,351 over 352,494
            // is 61.2 instructions a growth.
            //
            // kanso#1538 put this reserve inside `match_params_into`, where it
            // is paid once per CANDIDATE, and it cost 12,017,902 instructions.
            // The allocation is per dispatch, so the reserve belongs here.
            //
            // THE COUNT THAT USED TO BE HERE SAID TWENTY CANDIDATES A DISPATCH
            // and it was `match_one` CALLS divided by dispatches. `match_one`
            // runs once per PARAMETER of each candidate tried, so that ratio is
            // calls a dispatch and bounds the candidates from above; it equals
            // them only if every arm took one argument. Measured on the
            // interpreted corpus, 2026-09-19:
            //
            //     dispatches      175,254
            //     candidates      456,967   2.61 a dispatch
            //     match_one     1,135,058   6.48 a dispatch, 2.48 a candidate
            //
            // The old figure's own arithmetic agrees with the smaller number:
            // 12,017,902 over 175,254 is 68.6 instructions a dispatch, which is
            // 26.3 per reserve at 2.61 candidates and an impossible 3.4 at
            // twenty.
            //
            // `args_len` is exact for `score`, which takes one entry per
            // parameter, and a floor for `binds`, since a `Ctor` pattern can
            // bind its fields and a whole.
            // The pooled node lends its vector out for the candidate list
            // and takes it back below. What stays behind in the node is an
            // empty `Vec`, which owns no allocation, so the node is safe to
            // carry through a dispatch that never binds.
            let mut held_binds: Bindings = match pool.as_mut().and_then(Rc::get_mut) {
                Some(Env::Many(slots, _)) => std::mem::take(slots),
                _ => Vec::new(),
            };
            held_binds.clear();
            held_binds.reserve(args_len);
            // TWO OF EACH BUFFER IN FIXED SLOTS. The next candidate matches
            // into slot 0 and slot 1 holds the best so far, so a candidate
            // that takes the lead swaps the two slots rather than moving an
            // 80-byte tuple of score, declaration and bindings through an
            // `Option` -- which also put a drop of that `Option` on every
            // dispatch. The slots are fixed rather than named by an index
            // that flips: an index made every access to them computed, and
            // that cost 3,766,635 instructions more on the interpreted corpus
            // than the swaps it saved. The scores stay with the loop, as the
            // one score did; the bindings start each dispatch from the pooled
            // frame's vector.
            let mut pair: [Bindings; 2] = [held_binds, Vec::new()];
            let mut best: Option<&FnDecl> = None;
            // ONE ARM OF BARE NAMES HAS NOTHING TO CHOOSE. A name refuses a
            // failure and takes anything else, so such an arm wins exactly
            // when no argument is a failure, and with no rival its score is
            // never read. Its bindings are the names in order, each holding
            // `none` until `bind_moved` puts the argument there, which is
            // what `match_params_into` would have pushed.
            let lone = match overloads.as_slice() {
                [decl]
                    if decl.params.len() == args_len
                        && decl.params.iter().all(|p| matches!(p, Pattern::Var(..)))
                        && !args.iter().any(is_failure) =>
                {
                    Some(*decl)
                }
                _ => None,
            };
            if let Some(decl) = lone {
                let binds = &mut pair[0];
                if binds.capacity() == 0 {
                    *binds = self.spare_binds.borrow_mut().pop().unwrap_or_default();
                }
                for pattern in &decl.params {
                    if let Pattern::Var(name, _) = pattern {
                        binds.push((name.clone(), Value::NoneV));
                    }
                }
                best = Some(decl);
                pair.swap(0, 1);
            }
            for decl in overloads.iter().filter(|_| lone.is_none()) {
                if decl.params.len() != args.len() {
                    continue;
                }
                if pair[0].capacity() == 0 {
                    pair[0] = self.spare_binds.borrow_mut().pop().unwrap_or_default();
                }
                if !match_params_into(&decl.params, &args, &mut scores[0], &mut pair[0]) {
                    continue;
                }
                let replace = match best {
                    Some(_) => scores[0].beats(&scores[1]),
                    None => true,
                };
                if replace {
                    best = Some(decl);
                    pair.swap(0, 1);
                    scores.swap(0, 1);
                }
            }
            let [mut spare_binds, mut binds] = pair;
            // The buffer the losers matched into goes back for the next
            // dispatch rather than to the allocator.
            if spare_binds.capacity() > 0 {
                spare_binds.clear();
                let mut spare = self.spare_binds.borrow_mut();
                if spare.len() < 64 {
                    spare.push(spare_binds);
                }
            }
            match best {
                Some(decl) => {
                    bind_moved(&decl.params, &mut args, &mut binds);
                    // One frame for the whole parameter list. Pushing a node
                    // per binding made the chain as long as the arguments, and
                    // the walk paid for that on every name the body mentions.
                    let env = match binds.is_empty() {
                        true => None,
                        false => match pool.take() {
                            // The node is unique -- it was only pooled after
                            // `get_mut` said so, and nothing has been handed a
                            // copy since -- so this writes the winner's
                            // bindings into the frame the last dispatch left.
                            // `spare` carries them back out if it ever is not,
                            // which keeps a refused `get_mut` a lost reuse
                            // rather than a panic on a live interpreter.
                            Some(mut rc) => {
                                let mut spare = Some(binds);
                                if let Some(Env::Many(slots, _)) = Rc::get_mut(&mut rc) {
                                    if let Some(won) = spare.take() {
                                        *slots = won;
                                    }
                                }
                                match spare {
                                    None => Some(rc),
                                    Some(binds) => bind_all(None, binds),
                                }
                            }
                            None => bind_all(None, binds),
                        },
                    };
                    // One handle for the dispatcher beside the one the body
                    // gets. It costs a reference count either way, and it is
                    // what makes the frame reclaimable rather than freed.
                    let held = env.clone();
                    // THE ARGUMENT VECTOR IS A HOLDER, and nothing below
                    // reads it. `match_one` cloned each matched value into
                    // `binds`, so every container argument is now pointed at
                    // twice: once from here and once from the environment the
                    // body will run in. kanso#1497 measured what that costs:
                    // `append`'s accumulator was unique on 3.6% of 36,966
                    // calls, so the builder copied 180 MB it mostly did not
                    // need to, and `Rc::try_unwrap` could not fire.
                    // `bind_moved` left `NoneV` where it took a value, and a
                    // `NoneV` owns nothing, so `release` forgets it rather
                    // than handing it to `Value`'s drop one call at a time.
                    while let Some(arg) = args.pop() {
                        release(arg);
                    }
                    let flowed = self.eval_body_flow(decl, env);
                    // The body has finished with the frame. If nothing it made
                    // kept a handle -- a lazy thunk is the only thing that
                    // does -- the vector inside comes back for the next
                    // dispatch instead of being freed and allocated again.
                    if let Some(mut rc) = held {
                        if Rc::get_mut(&mut rc).is_some() {
                            pool = Some(rc);
                        }
                    }
                    // A closure the tail called runs here, in this loop, so a
                    // group and a lambda that hand each other on cost no stack.
                    let mut flowed = flowed?;
                    let mut lambda: Option<Rc<ClosureData>> = None;
                    while let Flow::Closure(next, next_args, next_span) = flowed {
                        // a call that fails before the lambda runs was written
                        // in this body, or in the lambda that handed it on
                        flowed = self.closure_flow(&next, next_args, next_span).map_err(|e| {
                            match &lambda {
                                Some(c) => placed(e, &c.frame),
                                None => placed(e, &self.frame_for(decl)),
                            }
                        })?;
                        lambda = Some(next);
                    }
                    match flowed {
                        Flow::Closure(..) => unreachable!("closures run in the loop above"),
                        Flow::Done(value) => return Ok(value),
                        Flow::Tail(next, group, next_args, next_span) => {
                            // a tail call that enters a cycle growing the
                            // stack from outside it is made as a call, which
                            // keeps a frame to put the record back in
                            let from = self.running.get();
                            let to = group.first().map_or(0, |d| self.deep_cycle(d));
                            if to != 0 && to != from {
                                // a lambda keeps its jump, as its compiled
                                // body does: it records and puts nothing back
                                if lambda.is_none() {
                                    return self.dispatch_loop(next, group, next_args, next_span);
                                }
                                if self.deep_now.get() == 0 {
                                    self.deep_now.set(to);
                                }
                            }
                            self.running.set(to);
                            // the call was written in this body, or in the
                            // lambda its tail handed on to, so a dispatch of
                            // it that fails is placed there
                            caller = match lambda {
                                Some(c) => Caller::Lambda(c),
                                None => Caller::Decl(decl),
                            };
                            overloads = group;
                            // `Flow::Tail` already carries the name as an
                            // `Rc<str>`, and every use of it below this loop
                            // reads it as `&str`. Moving it costs a pointer
                            // where copying it cost an allocation a hop.
                            name = next;
                            args = next_args;
                            span = next_span;
                        }
                    }
                }
                None => {
                    // a getter that matched nothing is a field error, in the
                    // reader's words: they wrote `x.name`, not `Get_name x`
                    let subject = args.first().cloned();
                    return match args.into_iter().find(is_failure) {
                        Some(bad) => Ok(hop(bad, &name)),
                        None => Err(RuntimeError {
                            at: match &caller {
                                Caller::Outside => None,
                                Caller::Decl(d) => origin_at(&self.frame_for(d), span).at,
                                Caller::Lambda(c) => origin_at(&c.frame, span).at,
                            },
                            message: match (crate::ast::getter_field(&name), subject) {
                                (Some(field), Some(Value::Record { ty, .. })) => {
                                    format!("`{ty}` has no field `{field}`")
                                }
                                (Some(_), Some(value)) => format!(
                                    "`.` reads a field of a record, not {}",
                                    render(self, &value, true)
                                ),
                                _ => format!(
                                    "no overload of `{}` matches these arguments",
                                    crate::ast::spoken(&name)
                                ),
                            },
                            span,
                        }),
                    };
                }
            }
        }
    }

    /// What a worded chain step answers once its subject has settled. This
    /// is the whole difference between the three: `bind` calls the callback
    /// on a value and lets a failure past, `rescue` calls it on a failure and
    /// lets a value past, and `annotate` reads the same channel as `rescue`
    /// but re-wraps the answer as an err with the original as cause — which
    /// is why it cannot resurrect, with nothing checking that it doesn't.
    fn worded_step(
        &self,
        word: Word,
        yielded: Value,
        callee: &Value,
        raised: &Raised,
        span: Span,
    ) -> EvalResult {
        let Value::ErrV(cause) = yielded.clone() else {
            return match word {
                Word::Bind => self.call_decided(callee, yielded, span),
                Word::Rescue | Word::Annotate => Ok(yielded),
            };
        };
        match word {
            Word::Bind => Ok(yielded),
            // The foreign-only licence, at the word. A rescue written in the
            // package that raised the failure hands it on without entering
            // the callback; a failure nobody can be held responsible for —
            // no frame, or merged out of several — passes the test.
            Word::Rescue if own_failure(&cause, raised) => Ok(yielded),
            // the callback is handed the err out of its box, an ordinary
            // value from here on (#2087)
            Word::Rescue => self.call_decided(callee, Value::Unboxed(cause), span),
            Word::Annotate => {
                match self.call_decided(callee, Value::Unboxed(cause.clone()), span)? {
                    // the callback failed on its own account, and that err
                    // already carries what it was told; wrapping it again would
                    // report the same failure twice
                    answer if is_failure(&answer) => Ok(answer),
                    reason => Ok(Value::ErrV(Rc::new(ErrInfo {
                        reason,
                        origin: raised.at.clone(),
                        hako: raised.hako.clone(),
                        hops: Vec::new(),
                        cause: Some(cause),
                        merged: false,
                    }))),
                }
            }
        }
    }
}
