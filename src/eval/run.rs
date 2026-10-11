// Running an IO description: the impl Interp block that executes a Desc
// against an Executor (execute, execute_chain, schedule, step) and the plan
// renderer that prints one. eval.rs reads this file with include!, so the
// items stay in the eval module and the compiler builds to the same
// instructions it did when they sat in eval.rs (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under
// src/'s subdirectories itself.

impl<'a> Interp<'a> {
    pub fn execute(&self, desc: &Desc, executor: &mut dyn Executor) -> EvalResult {
        match desc {
            Desc::Print(text, _) => {
                executor.print(text);
                Ok(Value::Done)
            }
            Desc::Bind(..) | Desc::Rescue(..) | Desc::Annotate(..) => {
                self.execute_chain(Rc::new(desc.clone()), executor)
            }
            Desc::Join(_, _) => self.schedule(desc, executor),
            Desc::Sleep(ms) => {
                executor.sleep(*ms);
                Ok(Value::Done)
            }
            Desc::Random(n) => Ok(Value::int(executor.random(*n))),
            Desc::Nil => Ok(Value::Done),
            Desc::Settled(v) => Ok(v.clone()),
            Desc::Args => {
                let list = executor
                    .args()
                    .into_iter()
                    .map(|arg: String| Value::Str(Text::from(arg)))
                    .collect();
                Ok(Value::List(Rc::new(list)))
            }
            Desc::Stdin => Ok(match executor.stdin() {
                Ok(text) => Value::Str(Text::from(text)),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            // The text, or `none` for a file that is not there — the std
            // wrapper names the second `file_not_found`, because a builtin
            // cannot name a type declared in kanso.
            Desc::ReadFile(path) => Ok(match executor.read_file(path) {
                Ok(found) => read_value(found),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::ReadBytes(path) => Ok(match executor.read_bytes(path) {
                Ok(Some(raw)) => Value::Bytes(Rc::new(raw)),
                Ok(None) => Value::NoneV,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            // three answers in a list, which the std wrapper turns into a
            // record: a builtin cannot name a type declared in kanso.
            Desc::Run(cmd, argv) => Ok(match executor.run(cmd, argv) {
                Ok(done) => ran_value(done),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            // Only reached outside a parallel group; step() starts and awaits.
            Desc::Await(handle) => Ok(match executor.finished(*handle) {
                Ok(Some(done)) => ran_value(done),
                Ok(None) => err_value(
                    Value::Str(Text::from("still running".to_string())),
                    Raised::default(),
                ),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Write(text) => {
                executor.write(text);
                Ok(Value::Done)
            }
            Desc::WriteErr(text) => {
                executor.write_err(text);
                Ok(Value::Done)
            }
            Desc::Env(name) => Ok(match executor.env(name) {
                Some(value) => Value::Str(Text::from(value)),
                None => Value::NoneV,
            }),
            Desc::Now => Ok(Value::int(executor.now())),
            Desc::Exists(path) => Ok(match executor.exists(path) {
                true => Value::True,
                false => Value::False,
            }),
            Desc::IsDir(path) => Ok(match executor.is_dir(path) {
                true => Value::True,
                false => Value::False,
            }),
            Desc::ListDir(path) => Ok(match executor.list_dir(path) {
                Ok(names) => Value::List(Rc::new(
                    names.into_iter().map(|arg: String| Value::Str(Text::from(arg))).collect(),
                )),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::MakeDir(path) => Ok(match executor.make_dir(path) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::WriteFile(path, content) => Ok(match executor.write_file(path, content) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::WriteBytes(path, raw) => Ok(match executor.write_bytes(path, raw) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Start(cmd, argv) => Ok(match executor.start(cmd, argv) {
                Ok(handle) => Value::int(handle),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Kill(handle) => Ok(match executor.kill(*handle) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Listen(port) => Ok(match executor.listen(*port) {
                Ok(handle) => Value::int(handle),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::SocketPort(listener) => Ok(match executor.socket_port(*listener) {
                Ok(port) => Value::int(port),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Accept(listener) => Ok(match executor.accept(*listener) {
                Ok(Some(handle)) => Value::int(handle),
                // Reached only outside a parallel group, where no other fiber
                // could ever connect; step() yields instead of arriving here.
                Ok(None) => err_value(
                    Value::Str(Text::from("nothing connected".to_string())),
                    Raised::default(),
                ),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Receive(conn) => Ok(match executor.receive(*conn) {
                Ok(text) => Value::Str(Text::from(text)),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Send(conn, text) => Ok(match executor.send(*conn, text) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::SendBytes(conn, raw) => Ok(match executor.send_bytes(*conn, raw) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::CloseSocket(handle) => Ok(match executor.close_socket(*handle) {
                Ok(()) => Value::Done,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
        }
    }

    /// Sequencing runs as a loop. Each link answers the description to run
    /// next, which becomes the next iteration rather than a nested call, so a
    /// program that sequences a million effects holds one frame. Only the
    /// continuation is flattened: a link's own left side is a step to run
    /// before the chain advances, and nests as deep as it is written.
    fn execute_chain(&self, start: Rc<Desc>, executor: &mut dyn Executor) -> EvalResult {
        let origin = Span::at(0, 0);
        let mut current = start;
        loop {
            let next = match &*current {
                Desc::Bind(inner, callee) => {
                    let yielded = self.execute(inner, executor)?;
                    match self.worded_step(
                        Word::Bind,
                        yielded,
                        callee,
                        &Raised::default(),
                        origin,
                    )? {
                        Value::Desc(d) => d,
                        other => return Ok(other),
                    }
                }
                Desc::Rescue(inner, callee, raised) => {
                    let yielded = self.execute(inner, executor)?;
                    match self.worded_step(Word::Rescue, yielded, callee, raised, origin)? {
                        Value::Desc(d) => d,
                        other => return Ok(other),
                    }
                }
                Desc::Annotate(inner, callee, raised) => {
                    let yielded = self.execute(inner, executor)?;
                    match self.worded_step(Word::Annotate, yielded, callee, raised, origin)? {
                        Value::Desc(d) => d,
                        other => return Ok(other),
                    }
                }
                leaf => return self.execute(leaf, executor),
            };
            current = next;
        }
    }

    /// Run a parallel group as cooperative green threads. Each member is a
    /// fiber; a fiber runs until it finishes or blocks on `sleep`, at which
    /// point the scheduler advances to the next fiber. The policy is fully
    /// deterministic — always resume the fiber with the earliest wake time,
    /// ties broken by spawn order — so a concurrent program's output is
    /// byte-identical across engines and runs, which the goldens require.
    /// Members' *values* are discarded (a group yields none); their failures
    /// accumulate, exactly as the sequential Join did.
    fn schedule(&self, join: &Desc, executor: &mut dyn Executor) -> EvalResult {
        let mut fibers = Vec::new();
        flatten_join(join, &mut fibers);
        // (wake_time, spawn_index, remaining work)
        let mut ready: Vec<(u64, usize, Rc<Desc>)> =
            fibers.into_iter().enumerate().map(|(i, d)| (0u64, i, d)).collect();
        let mut now = 0u64;
        // wall-credit: real time spent computing counts against a pending
        // wait, so compute overlaps sleeps in wall-clock. the transcript
        // stays purely logical — only the physical wait shrinks.
        #[cfg(not(target_arch = "wasm32"))]
        let wall_start = std::time::Instant::now();
        let mut failures: Vec<Value> = Vec::new();
        while !ready.is_empty() {
            let pick =
                (0..ready.len()).min_by_key(|&i| (ready[i].0, ready[i].1)).expect("non-empty");
            let (wake, idx, desc) = ready.remove(pick);
            if wake > now {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let elapsed = wall_start.elapsed().as_millis() as u64;
                    if wake > elapsed {
                        executor.sleep(wake - elapsed);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                executor.sleep(wake - now);
                now = wake;
            }
            match self.step(&desc, executor)? {
                Step::Done(value) => {
                    if is_failure(&value) {
                        failures.push(value);
                    }
                }
                Step::Blocked(ms, cont) => ready.push((now + ms, idx, cont)),
            }
        }
        Ok(failures.into_iter().reduce(accumulate_failures).unwrap_or(Value::NoneV))
    }

    /// Advance one fiber until it finishes (`Done`) or hits a `sleep`
    /// (`Blocked`, carrying the rest of its work). Blocking threads back up
    /// through `Bind`, so the continuation is always the remaining
    /// description — no saved stack, no coroutine.
    fn step(&self, desc: &Rc<Desc>, executor: &mut dyn Executor) -> Result<Step, RuntimeError> {
        let origin = Span::at(0, 0);
        match &**desc {
            Desc::Sleep(ms) => Ok(Step::Blocked(*ms, Rc::new(Desc::Nil))),
            // A fiber waiting for a connection is a fiber with nothing to do:
            // it goes back in the queue and the group's other statements run,
            // which is how anything ever connects to it.
            Desc::Accept(listener) => match executor.accept(*listener) {
                Ok(Some(handle)) => Ok(Step::Done(Value::int(handle))),
                Ok(None) => Ok(Step::Blocked(1, desc.clone())),
                Err(reason) => {
                    Ok(Step::Done(err_value(Value::Str(Text::from(reason)), Raised::default())))
                }
            },
            // Same shape for a process: start it, then wait by yielding, so
            // the other statements of the group run while it does.
            Desc::Run(cmd, argv) => match executor.start(cmd, argv) {
                Ok(handle) => Ok(Step::Blocked(1, Rc::new(Desc::Await(handle)))),
                Err(reason) => {
                    Ok(Step::Done(err_value(Value::Str(Text::from(reason)), Raised::default())))
                }
            },
            Desc::Await(handle) => match executor.finished(*handle) {
                Ok(Some(done)) => Ok(Step::Done(ran_value(done))),
                Ok(None) => Ok(Step::Blocked(1, desc.clone())),
                Err(reason) => {
                    Ok(Step::Done(err_value(Value::Str(Text::from(reason)), Raised::default())))
                }
            },
            Desc::Bind(inner, callee) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Bind(cont, callee.clone()))))
                }
                Step::Done(yielded) => {
                    let next =
                        self.worded_step(Word::Bind, yielded, callee, &Raised::default(), origin)?;
                    match next {
                        Value::Desc(d) => self.step(&d, executor),
                        other => Ok(Step::Done(other)),
                    }
                }
            },
            Desc::Rescue(inner, callee, raised) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => Ok(Step::Blocked(
                    ms,
                    Rc::new(Desc::Rescue(cont, callee.clone(), raised.clone())),
                )),
                Step::Done(yielded) => {
                    let next = self.worded_step(Word::Rescue, yielded, callee, raised, origin)?;
                    match next {
                        Value::Desc(d) => self.step(&d, executor),
                        other => Ok(Step::Done(other)),
                    }
                }
            },
            Desc::Annotate(inner, callee, raised) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => Ok(Step::Blocked(
                    ms,
                    Rc::new(Desc::Annotate(cont, callee.clone(), raised.clone())),
                )),
                Step::Done(yielded) => {
                    let next = self.worded_step(Word::Annotate, yielded, callee, raised, origin)?;
                    match next {
                        Value::Desc(d) => self.step(&d, executor),
                        other => Ok(Step::Done(other)),
                    }
                }
            },
            // leaf effects and nested joins run to completion synchronously
            _ => Ok(Step::Done(self.execute(desc, executor)?)),
        }
    }
}

/// Collect the members of a (possibly nested) parallel group left-to-right —
/// the spawn order the scheduler breaks ties by.
fn flatten_join(desc: &Desc, out: &mut Vec<Rc<Desc>>) {
    match desc {
        Desc::Join(a, b) => {
            flatten_join(a, out);
            flatten_join(b, out);
        }
        other => out.push(Rc::new(other.clone())),
    }
}

/// A plan describes without running. A bind's callback cannot be called
/// without the value it is handed, except when it ignores that value:
/// `_ -> step` builds the same description whatever arrives, and `step` is
/// how the plan reaches it. Any other callback is shown as a continuation.
pub fn render_plan(desc: &Desc, out: &mut String, step: &dyn Fn(&Value) -> Option<Rc<Desc>>) {
    match desc {
        Desc::Print(text, span) => {
            out.push_str(&format!("  print {text:?}    # from line {}\n", span.line));
        }
        Desc::Join(a, b) => {
            out.push_str("  join {\n");
            render_plan(a, out, step);
            render_plan(b, out, step);
            out.push_str("  } # unordered; both run\n");
        }
        Desc::Args => out.push_str("  args\n"),
        Desc::Stdin => out.push_str("  stdin\n"),
        Desc::ReadFile(path) => out.push_str(&format!("  read_file {path:?}\n")),
        Desc::ReadBytes(path) => out.push_str(&format!("  read_bytes {path:?}\n")),
        Desc::Run(cmd, argv) => out.push_str(&format!("  run {cmd:?} {argv:?}\n")),
        Desc::Start(cmd, argv) => out.push_str(&format!("  start {cmd:?} {argv:?}\n")),
        Desc::Kill(handle) => out.push_str(&format!("  kill {handle}\n")),
        Desc::Await(handle) => out.push_str(&format!("  await {handle}\n")),
        Desc::Write(text) => out.push_str(&format!("  write {text:?}\n")),
        Desc::WriteErr(text) => out.push_str(&format!("  write_err {text:?}\n")),
        Desc::Env(name) => out.push_str(&format!("  env {name:?}\n")),
        Desc::Exists(path) => out.push_str(&format!("  exists {path:?}\n")),
        Desc::IsDir(path) => out.push_str(&format!("  is_dir {path:?}\n")),
        Desc::Now => out.push_str("  now\n"),
        Desc::ListDir(path) => out.push_str(&format!("  list_dir {path:?}\n")),
        Desc::MakeDir(path) => out.push_str(&format!("  make_dir {path:?}\n")),
        Desc::WriteFile(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::WriteBytes(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::Listen(port) => out.push_str(&format!("  listen {port}\n")),
        Desc::SocketPort(l) => out.push_str(&format!("  port {l}\n")),
        Desc::Accept(listener) => out.push_str(&format!("  accept {listener}\n")),
        Desc::Receive(conn) => out.push_str(&format!("  receive {conn}\n")),
        Desc::Send(conn, _) => out.push_str(&format!("  send {conn}\n")),
        Desc::SendBytes(conn, _) => out.push_str(&format!("  send {conn}\n")),
        Desc::CloseSocket(handle) => out.push_str(&format!("  close_socket {handle}\n")),
        Desc::Bind(inner, callee) => {
            render_plan(inner, out, step);
            match step(callee) {
                Some(next) => render_plan(&next, out, step),
                None => out.push_str("  . <continuation>\n"),
            }
        }
        Desc::Rescue(inner, ..) => {
            render_plan(inner, out, step);
            out.push_str("  rescue <continuation>\n");
        }
        Desc::Annotate(inner, ..) => {
            render_plan(inner, out, step);
            out.push_str("  annotate <continuation>\n");
        }
        Desc::Sleep(ms) => out.push_str(&format!("  sleep {ms}\n")),
        Desc::Random(n) => out.push_str(&format!("  random {n}\n")),
        Desc::Nil => {}
        Desc::Settled(_) => out.push_str("  settled\n"),
    }
}

/// A lazy sequence has no length to count — `naturals` would never finish —
/// so refusing it is right. But the reader who wrote `length (map …)` is one
/// call from what they wanted, and the message may as well say which.
fn lazy_hint(v: &Value) -> &'static str {
    match v {
        Value::Record { ty, .. } if ty.starts_with("list/") || ty.contains("/list/") => {
            " — a lazy sequence becomes a list with list/to_list"
        }
        _ => "",
    }
}
