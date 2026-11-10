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
            Desc::Insisted(inner, at, hako) => {
                Ok(restamped(self.execute(inner, executor)?, at, hako))
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
            // the line without its ending, or `done` when the input has ended
            Desc::ReadLine => Ok(match executor.read_line() {
                Ok(Some(line)) => Value::Str(Text::from(line)),
                Ok(None) => Value::Done,
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
            Desc::RandomBytes(n) => Ok(match executor.random_bytes(*n) {
                Ok(raw) => Value::Bytes(Rc::new(raw)),
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
            // milliseconds since the epoch, or `none` for a file that is not
            // there, which os/modified names `file_not_found`
            Desc::Modified(path) => Ok(match executor.modified(path) {
                Ok(Some(ms)) => Value::int(ms),
                Ok(None) => Value::NoneV,
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
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
            // Outside a group no other fiber can take or give, so a pool that
            // cannot answer now never will.
            Desc::PoolMake(n) => Ok(self.pool_make(*n)),
            Desc::PoolTake(handle) => Ok(match self.pool_take(*handle) {
                Some(v) => v,
                None => pool_stuck(TAKE_STUCK),
            }),
            Desc::PoolGive(handle, v) => Ok(match self.pool_give(*handle, v) {
                Some(v) => v,
                None => pool_stuck(GIVE_STUCK),
            }),
            Desc::Listen(port) => Ok(match executor.listen(*port) {
                Ok(handle) => Value::int(handle),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::SocketPort(listener) => Ok(match executor.socket_port(*listener) {
                Ok(port) => Value::int(port),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Peer(conn) => Ok(match executor.peer(*conn) {
                Ok(address) => Value::Str(Text::from(address)),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            // Reached only outside a parallel group; step() yields instead of
            // arriving here. No other fiber can connect then, but a client
            // outside the program can, so this waits for one.
            Desc::Accept(listener) => Ok(match executor.accept_waiting(*listener) {
                Ok(handle) => Value::int(handle),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Receive(conn) => Ok(match executor.receive(*conn) {
                Ok(text) => Value::Str(Text::from(text)),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::ReceiveBytes(conn) => Ok(match executor.receive_bytes(*conn) {
                Ok(raw) => Value::Bytes(Rc::new(raw)),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Dial(host, port) => Ok(match executor.dial(host, *port) {
                Ok(handle) => Value::int(handle),
                Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
            }),
            Desc::Secure(conn, host) => Ok(match executor.secure(*conn, host) {
                Ok(handle) => Value::int(handle),
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
            // Outside the scheduler there is no other fiber to run, so a send
            // it began is finished where it stands.
            Desc::Sending(whole, sent) => {
                let (conn, raw) = sent_bytes(whole);
                Ok(match executor.send_bytes(conn, &raw[(*sent).min(raw.len())..]) {
                    Ok(()) => Value::Done,
                    Err(reason) => err_value(Value::Str(Text::from(reason)), Raised::default()),
                })
            }
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
        let mut members = Vec::new();
        flatten_join(join, &mut members);
        // The live fibers in spawn order, which breaks ties between equal
        // wake times. A join a fiber reaches puts its members where that
        // fiber stood, so a nested group that never waits runs exactly as it
        // would inline, and what follows the join takes the place of the
        // member that finishes last.
        let mut fibers: Vec<Fiber> =
            members.into_iter().map(|work| Fiber { wake: 0, work, group: 0 }).collect();
        let mut groups = vec![Group {
            remaining: fibers.len(),
            failures: Vec::new(),
            parent: None,
            rejoin: None,
        }];
        let mut free: Vec<usize> = Vec::new();
        let mut now = 0u64;
        // wall-credit: real time spent computing counts against a pending
        // wait, so compute overlaps sleeps in wall-clock. the transcript
        // stays purely logical — only the physical wait shrinks.
        #[cfg(not(target_arch = "wasm32"))]
        let wall_start = std::time::Instant::now();
        while !fibers.is_empty() {
            // A take or give went through: the fibers parked on a pool try
            // again, in spawn order, and nothing is stuck any more.
            if self.pool_moved.replace(false) {
                self.pool_failing.set(false);
                for fiber in fibers.iter_mut().filter(|f| f.wake == PARKED) {
                    fiber.wake = now;
                }
            }
            let mut pick = 0;
            for (at, fiber) in fibers.iter().enumerate() {
                if fiber.wake < fibers[pick].wake {
                    pick = at;
                }
            }
            // Every fiber is parked on a pool: nothing can give or take, so
            // each parked operation is woken to answer that it never will.
            if fibers[pick].wake == PARKED {
                self.pool_failing.set(true);
                for fiber in fibers.iter_mut() {
                    fiber.wake = now;
                }
                continue;
            }
            let wake = fibers[pick].wake;
            if wake > now {
                // Every fiber is waiting. Where some wait on sockets the
                // wait is on those, for as long as the earliest fiber on the
                // clock allows: asking each socket again every millisecond
                // answered a read up to a millisecond late and woke an idle
                // server a thousand times a second (#2405). A socket that is
                // ready makes its fiber due now, and one that is not stays a
                // tick away. The clock moves to the time that passed, and no
                // further than the fiber it was waiting for.
                #[cfg(not(target_arch = "wasm32"))]
                if self.sock_waiting.get()
                    && self.awaited(&mut fibers, &mut now, wall_start, executor)
                {
                    continue;
                }
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
            let work = fibers[pick].work.clone();
            match self.step(&work, executor)? {
                Step::Blocked(ms, cont) => {
                    fibers[pick].wake = if ms == PARKED { PARKED } else { now + ms };
                    fibers[pick].work = cont;
                }
                Step::Forked(spawned, rejoin) => {
                    let made = Group {
                        remaining: spawned.len(),
                        failures: Vec::new(),
                        parent: Some(fibers[pick].group),
                        rejoin,
                    };
                    let group = match free.pop() {
                        Some(slot) => {
                            groups[slot] = made;
                            slot
                        }
                        None => {
                            groups.push(made);
                            groups.len() - 1
                        }
                    };
                    let at = pick;
                    fibers.remove(at);
                    for (k, work) in spawned.into_iter().enumerate() {
                        fibers.insert(at + k, Fiber { wake: now, work, group });
                    }
                }
                Step::Done(value) => {
                    let group = fibers[pick].group;
                    fibers.remove(pick);
                    if is_failure(&value) {
                        groups[group].failures.push(value);
                    }
                    groups[group].remaining -= 1;
                    let Some(parent) = groups[group].parent else { continue };
                    // The member that finishes a group hands its place to the
                    // work after the join, in the group the fiber that
                    // reached the join belonged to; that fiber still counts
                    // there, so the parent's count is left alone.
                    if groups[group].remaining == 0 {
                        let failures = std::mem::take(&mut groups[group].failures);
                        let yielded = failures
                            .into_iter()
                            .reduce(accumulate_failures)
                            .unwrap_or(Value::NoneV);
                        let settled = Rc::new(Desc::Settled(yielded));
                        let work = match groups[group].rejoin.take() {
                            Some(rejoin) => rejoin(settled),
                            None => settled,
                        };
                        fibers.insert(pick, Fiber { wake: now, work, group: parent });
                        groups[group].parent = None;
                        free.push(group);
                    } else if groups[group].remaining == 1
                        && groups[group].failures.is_empty()
                        && groups[group].rejoin.is_none()
                    {
                        // One member left, nothing failed and nothing after
                        // the join: what the group will yield is what that
                        // member yields, so the member counts toward the
                        // parent from here and the group goes. A server's
                        // loop is this shape, each request a group the next
                        // request is a member of, and keeping the groups kept
                        // every request it had answered.
                        match fibers.iter_mut().find(|f| f.group == group) {
                            Some(last) => last.group = parent,
                            None => {
                                let nested = groups
                                    .iter_mut()
                                    .find(|g| g.remaining > 0 && g.parent == Some(group))
                                    .expect("a group's last member is a fiber or a group");
                                nested.parent = Some(parent);
                            }
                        }
                        groups[group].parent = None;
                        groups[group].remaining = 0;
                        free.push(group);
                    }
                }
            }
        }
        let failures = std::mem::take(&mut groups[0].failures);
        Ok(failures.into_iter().reduce(accumulate_failures).unwrap_or(Value::NoneV))
    }

    /// The scheduler's wait when a fiber is waiting on a socket: `true` when
    /// it waited, and the pick is to be made again. What each fiber waits on
    /// is read from the work it has left, so nothing is kept per fiber and
    /// nothing is done per step. Answers `false`, having waited for nothing,
    /// when no fiber waits on a socket or the executor cannot wait on one.
    ///
    /// Kept out of line and cold: the interpreted corpus opens no socket,
    /// and it ran longer with this folded into the scheduler's loop.
    #[cfg(not(target_arch = "wasm32"))]
    #[cold]
    #[inline(never)]
    fn awaited(
        &self,
        fibers: &mut [Fiber],
        now: &mut u64,
        wall_start: std::time::Instant,
        executor: &mut dyn Executor,
    ) -> bool {
        let waits: Vec<Option<(i64, bool)>> =
            fibers.iter().map(|f| waited_socket(&f.work)).collect();
        let on: Vec<(i64, bool)> = waits.iter().flatten().copied().collect();
        self.sock_waiting.set(!on.is_empty());
        if on.is_empty() {
            return false;
        }
        let clocked = fibers.iter().zip(&waits).filter(|(f, w)| w.is_none() && f.wake != PARKED);
        let due = clocked.map(|(f, _)| f.wake).min();
        // Until a socket is ready or the fiber on the clock is due. A wait
        // that comes back a moment early is neither, and waits again, so one
        // wait is one thing waited for.
        let ready = loop {
            let elapsed = wall_start.elapsed().as_millis() as u64;
            let Some(ready) = executor.await_sockets(&on, due.map(|d| d.saturating_sub(elapsed)))
            else {
                return false;
            };
            let passed = wall_start.elapsed().as_millis() as u64;
            if ready.iter().any(|r| *r) || due.is_some_and(|d| passed >= d) {
                break ready;
            }
        };
        self.sock_waits.set(self.sock_waits.get() + 1);
        *now = (*now).max(wall_start.elapsed().as_millis() as u64);
        if let Some(due) = due {
            *now = (*now).min(due);
        }
        let waiting = fibers.iter_mut().zip(&waits).filter(|(_, w)| w.is_some());
        for ((fiber, _), ready) in waiting.zip(ready) {
            fiber.wake = if ready { *now } else { *now + 1 };
        }
        true
    }

    /// Advance one fiber until it finishes (`Done`) or hits a `sleep`
    /// (`Blocked`, carrying the rest of its work). Blocking threads back up
    /// through `Bind`, so the continuation is always the remaining
    /// description — no saved stack, no coroutine.
    ///
    /// A chain whose links never block runs within one step. Each link that
    /// answers another description goes round this loop rather than into a
    /// deeper call: the recursion it replaced held a frame per link and
    /// overflowed the stack at three million of them (#2276).
    fn step(&self, desc: &Rc<Desc>, executor: &mut dyn Executor) -> Result<Step, RuntimeError> {
        let origin = Span::at(0, 0);
        let mut desc = desc.clone();
        loop {
            let (inner, callee, word, raised) = match &*desc {
                Desc::Bind(inner, callee) => (inner, callee, Word::Bind, None),
                Desc::Rescue(inner, callee, raised) => (inner, callee, Word::Rescue, Some(raised)),
                Desc::Annotate(inner, callee, raised) => {
                    (inner, callee, Word::Annotate, Some(raised))
                }
                _ => return self.step_leaf(&desc, executor),
            };
            match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => return Ok(Step::Blocked(ms, relinked(&desc, cont))),
                Step::Forked(members, rejoin) => {
                    let link = desc.clone();
                    return Ok(Step::Forked(
                        members,
                        around(rejoin, move |done| relinked(&link, done)),
                    ));
                }
                Step::Done(yielded) => {
                    let unraised = Raised::default();
                    let raised = raised.unwrap_or(&unraised);
                    match self.worded_step(word, yielded, callee, raised, origin)? {
                        Value::Desc(next) => desc = next,
                        other => return Ok(Step::Done(other)),
                    }
                }
            }
        }
    }

    /// One step of anything but a chain word.
    /// A send, from the byte it has reached. What the connection takes now
    /// is counted, and when it will take no more the fiber is left holding
    /// the send and that count, so it picks up where it stopped and no byte
    /// is copied to remember the rest. The connection is asked at least
    /// once, so a send of nothing still learns that its handle names none.
    fn send_on(
        &self,
        whole: &Rc<Desc>,
        mut sent: usize,
        executor: &mut dyn Executor,
    ) -> Result<Step, RuntimeError> {
        let (conn, raw) = sent_bytes(whole);
        loop {
            match executor.send_some(conn, &raw[sent.min(raw.len())..]) {
                Ok(Some(taken)) => sent += taken,
                Ok(None) => {
                    self.sock_waiting.set(true);
                    return Ok(Step::Blocked(1, Rc::new(Desc::Sending(whole.clone(), sent))));
                }
                Err(reason) => {
                    let said = Value::Str(Text::from(reason));
                    return Ok(Step::Done(err_value(said, Raised::default())));
                }
            }
            if sent >= raw.len() {
                return Ok(Step::Done(Value::Done));
            }
        }
    }

    fn step_leaf(
        &self,
        desc: &Rc<Desc>,
        executor: &mut dyn Executor,
    ) -> Result<Step, RuntimeError> {
        match &**desc {
            Desc::Sleep(ms) => Ok(Step::Blocked(*ms, Rc::new(Desc::Nil))),
            // A join the fiber reaches is the scheduler's to run, beside the
            // fibers already running, rather than a second scheduler inside
            // this step that the others wait on (#2263).
            Desc::Join(..) => {
                let mut members = Vec::new();
                flatten_join(desc, &mut members);
                Ok(Step::Forked(members, None))
            }
            // A fiber waiting for a connection is a fiber with nothing to do:
            // it goes back in the queue and the group's other statements run,
            // which is how anything ever connects to it.
            // A take from an empty pool, or a give to a full one, parks the
            // fiber until another fiber gives or takes (#2264). When every
            // fiber is parked nothing can move, and the scheduler sets
            // `pool_failing`: each parked operation then answers a failure
            // that names it, rather than the program waiting forever.
            Desc::PoolTake(handle) => match self.pool_take(*handle) {
                Some(v) => Ok(Step::Done(v)),
                None if self.pool_failing.get() => Ok(Step::Done(pool_stuck(TAKE_STUCK))),
                None => Ok(Step::Blocked(PARKED, desc.clone())),
            },
            Desc::PoolGive(handle, v) => match self.pool_give(*handle, v) {
                Some(v) => Ok(Step::Done(v)),
                None if self.pool_failing.get() => Ok(Step::Done(pool_stuck(GIVE_STUCK))),
                None => Ok(Step::Blocked(PARKED, desc.clone())),
            },
            Desc::Accept(listener) => match executor.accept(*listener) {
                Ok(Some(handle)) => Ok(Step::Done(Value::int(handle))),
                Ok(None) => {
                    self.sock_waiting.set(true);
                    Ok(Step::Blocked(1, desc.clone()))
                }
                Err(reason) => {
                    Ok(Step::Done(err_value(Value::Str(Text::from(reason)), Raised::default())))
                }
            },
            // A read with nothing arrived yet waits the same way, so a program
            // can be both ends of a connection: the fiber reading yields, and
            // the one that will write runs.
            Desc::Receive(conn) | Desc::ReceiveBytes(conn) => match executor.readable(*conn) {
                Ok(false) => {
                    self.sock_waiting.set(true);
                    Ok(Step::Blocked(1, desc.clone()))
                }
                Ok(true) => Ok(Step::Done(self.execute(desc, executor)?)),
                Err(reason) => {
                    Ok(Step::Done(err_value(Value::Str(Text::from(reason)), Raised::default())))
                }
            },
            // A send the connection has no room for waits the same way, so
            // one client that has stopped reading holds its own fiber and no
            // other (#2407).
            Desc::Send(..) | Desc::SendBytes(..) => self.send_on(desc, 0, executor),
            Desc::Sending(whole, sent) => self.send_on(whole, *sent, executor),
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
            Desc::Insisted(inner, at, hako) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Insisted(cont, at.clone(), hako.clone()))))
                }
                Step::Forked(members, rejoin) => {
                    let (at, hako) = (at.clone(), hako.clone());
                    Ok(Step::Forked(
                        members,
                        around(rejoin, move |done| {
                            Rc::new(Desc::Insisted(done, at.clone(), hako.clone()))
                        }),
                    ))
                }
                Step::Done(v) => Ok(Step::Done(restamped(v, at, hako))),
            },
            // leaf effects run to completion synchronously
            _ => Ok(Step::Done(self.execute(desc, executor)?)),
        }
    }
}

/// A fiber the scheduler holds: when it may run next, what it has left, and
/// the group it counts toward.
struct Fiber {
    wake: u64,
    work: Rc<Desc>,
    group: usize,
}

/// A parallel group: how many members are still running, the failures the
/// finished ones yielded, the group the fiber that reached its join counted
/// toward, and the work after that join, if the join was not all the fiber
/// had left. The group a scheduler starts with has no parent; its failures are
/// what the scheduler answers. A slot whose group has finished has no parent
/// and no members, and the next join takes it.
struct Group {
    remaining: usize,
    failures: Vec<Value>,
    parent: Option<usize>,
    rejoin: Option<Rejoin>,
}

/// The socket a fiber is waiting on, by its handle, and whether it waits to
/// write: a listener nobody has called, a connection nothing has arrived on,
/// or one with no room for the rest of a send, found under the chain words
/// around it. A fiber the scheduler holds with one of these at the bottom of
/// its work asked it and was told not yet.
#[cfg(not(target_arch = "wasm32"))]
fn waited_socket(work: &Rc<Desc>) -> Option<(i64, bool)> {
    let mut at = work;
    loop {
        match &**at {
            Desc::Bind(inner, _)
            | Desc::Rescue(inner, _, _)
            | Desc::Annotate(inner, _, _)
            | Desc::Insisted(inner, _, _) => at = inner,
            Desc::Accept(handle) | Desc::Receive(handle) | Desc::ReceiveBytes(handle) => {
                return Some((*handle, false))
            }
            Desc::Sending(whole, _) => return Some((sent_bytes(whole).0, true)),
            _ => return None,
        }
    }
}

/// The connection and the bytes of a send, whichever value carries them.
fn sent_bytes(send: &Desc) -> (i64, &[u8]) {
    match send {
        Desc::Send(conn, text) => (*conn, text.as_bytes()),
        Desc::SendBytes(conn, raw) => (*conn, raw),
        _ => (-1, &[]),
    }
}

/// The same chain link with a different subject: the rest of the work once
/// the subject blocked or forked part way through.
fn relinked(link: &Desc, subject: Rc<Desc>) -> Rc<Desc> {
    Rc::new(match link {
        Desc::Bind(_, callee) => Desc::Bind(subject, callee.clone()),
        Desc::Rescue(_, callee, raised) => Desc::Rescue(subject, callee.clone(), raised.clone()),
        Desc::Annotate(_, callee, raised) => {
            Desc::Annotate(subject, callee.clone(), raised.clone())
        }
        _ => unreachable!("only a chain word is relinked"),
    })
}

/// A rejoin with one more layer around it. No rejoin is a join that was the
/// whole of the fiber's work, which is what lets a group with nothing after
/// it go early.
fn around(rejoin: Option<Rejoin>, wrap: impl Fn(Rc<Desc>) -> Rc<Desc> + 'static) -> Option<Rejoin> {
    Some(Rc::new(move |done| {
        wrap(match &rejoin {
            Some(inner) => inner(done),
            None => done,
        })
    }))
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
        Desc::ReadLine => out.push_str("  read_line\n"),
        Desc::ReadFile(path) => out.push_str(&format!("  read_file {path:?}\n")),
        Desc::ReadBytes(path) => out.push_str(&format!("  read_bytes {path:?}\n")),
        Desc::RandomBytes(n) => out.push_str(&format!("  random_bytes {n}\n")),
        Desc::Run(cmd, argv) => out.push_str(&format!("  run {cmd:?} {argv:?}\n")),
        Desc::Start(cmd, argv) => out.push_str(&format!("  start {cmd:?} {argv:?}\n")),
        Desc::Kill(handle) => out.push_str(&format!("  kill {handle}\n")),
        Desc::Await(handle) => out.push_str(&format!("  await {handle}\n")),
        Desc::Write(text) => out.push_str(&format!("  write {text:?}\n")),
        Desc::WriteErr(text) => out.push_str(&format!("  write_err {text:?}\n")),
        Desc::Env(name) => out.push_str(&format!("  env {name:?}\n")),
        Desc::Exists(path) => out.push_str(&format!("  exists {path:?}\n")),
        Desc::IsDir(path) => out.push_str(&format!("  is_dir {path:?}\n")),
        Desc::Modified(path) => out.push_str(&format!("  modified {path:?}\n")),
        Desc::Now => out.push_str("  now\n"),
        Desc::ListDir(path) => out.push_str(&format!("  list_dir {path:?}\n")),
        Desc::MakeDir(path) => out.push_str(&format!("  make_dir {path:?}\n")),
        Desc::WriteFile(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::WriteBytes(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::Listen(port) => out.push_str(&format!("  listen {port}\n")),
        Desc::SocketPort(l) => out.push_str(&format!("  port {l}\n")),
        Desc::Peer(c) => out.push_str(&format!("  peer {c}\n")),
        Desc::Accept(listener) => out.push_str(&format!("  accept {listener}\n")),
        Desc::Receive(conn) => out.push_str(&format!("  receive {conn}\n")),
        Desc::ReceiveBytes(conn) => out.push_str(&format!("  receive_bytes {conn}\n")),
        Desc::Dial(host, port) => out.push_str(&format!("  dial {host:?} {port}\n")),
        Desc::Secure(conn, host) => out.push_str(&format!("  secure {conn} {host:?}\n")),
        Desc::Send(conn, _) => out.push_str(&format!("  send {conn}\n")),
        Desc::SendBytes(conn, _) => out.push_str(&format!("  send {conn}\n")),
        Desc::Sending(whole, _) => out.push_str(&format!("  send {}\n", sent_bytes(whole).0)),
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
        Desc::Insisted(inner, ..) => render_plan(inner, out, step),
        Desc::Annotate(inner, ..) => {
            render_plan(inner, out, step);
            out.push_str("  annotate <continuation>\n");
        }
        Desc::PoolMake(n) => out.push_str(&format!("  pool_make {n}\n")),
        Desc::PoolTake(handle) => out.push_str(&format!("  pool_take {handle}\n")),
        Desc::PoolGive(handle, _) => out.push_str(&format!("  pool_give {handle}\n")),
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

/// The wake time of a fiber parked on a pool: no clock reaches it, so only a
/// take or give by another fiber, or the scheduler finding every fiber
/// parked, moves it.
const PARKED: u64 = u64::MAX;

const TAKE_STUCK: &str = "a take from an empty pool that no fiber is left to give to";
const GIVE_STUCK: &str = "a give to a full pool that no fiber is left to take from";

fn pool_stuck(reason: &str) -> Value {
    err_value(Value::Str(Text::from(reason)), Raised::default())
}

impl<'a> Interp<'a> {
    fn pool_make(&self, n: i64) -> Value {
        if n < 1 {
            return err_value(
                Value::Str(Text::from("a pool holds at least one value")),
                Raised::default(),
            );
        }
        let mut pools = self.pools.borrow_mut();
        pools.push(Pool { cap: n as usize, items: Vec::new() });
        Value::int((pools.len() - 1) as i64)
    }

    /// The value a take answers, or `None` while the pool is empty. The last
    /// value given is the one taken.
    fn pool_take(&self, handle: i64) -> Option<Value> {
        let mut pools = self.pools.borrow_mut();
        let Some(pool) = usize::try_from(handle).ok().and_then(|h| pools.get_mut(h)) else {
            return Some(err_value(
                Value::Str(Text::from("that is not a pool")),
                Raised::default(),
            ));
        };
        let taken = pool.items.pop()?;
        self.pool_moved.set(true);
        Some(taken)
    }

    /// `done` once the value is in the pool, or `None` while it is full.
    fn pool_give(&self, handle: i64, v: &Value) -> Option<Value> {
        let mut pools = self.pools.borrow_mut();
        let Some(pool) = usize::try_from(handle).ok().and_then(|h| pools.get_mut(h)) else {
            return Some(err_value(
                Value::Str(Text::from("that is not a pool")),
                Raised::default(),
            ));
        };
        if pool.items.len() >= pool.cap {
            return None;
        }
        pool.items.push(v.clone());
        self.pool_moved.set(true);
        Some(Value::Done)
    }
}
