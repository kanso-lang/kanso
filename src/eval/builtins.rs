// The interpreter's builtins: call_builtin, which answers every builtin a
// program can name. eval.rs reads this file with include!, so the method
// stays in the eval module and the compiler builds to the same
// instructions it did when it sat in eval.rs's impl block (kanso#1867).
// cargo fmt does not reach an included file, so CI's format job checks the
// files under src/'s subdirectories itself.

impl<'a> Interp<'a> {
    pub fn call_builtin(
        &self,
        name: &str,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        // std wrapper modules reach natives through the builtin_ prefix;
        // the checker gates those names to std-origin files
        let name = name.strip_prefix("builtin_").unwrap_or(name);
        // In place, and only where a subtype's value is: every other value is
        // its own base. The in-place collect this replaced was 1.2% of the
        // interpreted corpus.
        let mut args = args;
        for slot in args.iter_mut() {
            if let Value::Sub { inner, .. } = slot {
                let base = sub_base((**inner).clone());
                *slot = base;
            }
        }
        if name == "if" {
            return self.builtin_if(args, span);
        }
        // the one hole in err's infectiousness: wrap_err's second argument
        // IS the err being wrapped, so it arrives as a value
        if name == "wrap_err" {
            let [reason, original] = arity(args, name, span)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            let Value::ErrV(cause) = original else {
                return Err(RuntimeError {
                    at: None,
                    message: "wrap_err takes a reason and the err it wraps".to_string(),
                    span,
                });
            };
            let raised = origin_at(frame, span);
            return Ok(Value::ErrV(Rc::new(ErrInfo {
                reason,
                origin: raised.at,
                hako: raised.hako,
                hops: Vec::new(),
                cause: Some(cause),
                merged: false,
            })));
        }
        // The gavel's three chain words. Like `wrap_err` they must see a
        // failure rather than be skipped by one — two of them exist to read
        // exactly that channel — so they sit above the short-circuit below.
        // An effect defers and becomes a chain node; a value that has already
        // settled decides here, which is what makes a word mean the same
        // thing called prefix-style outside a chain.
        if let Some(word) = chain_word(name) {
            let [subject, callback] = arity(args, name, span)?;
            let raised = origin_at(frame, span);
            if let Value::Desc(inner) = subject {
                return Ok(Value::Desc(Rc::new(match word {
                    Word::Bind => Desc::Bind(inner, callback),
                    Word::Rescue => Desc::Rescue(inner, callback, raised),
                    Word::Annotate => Desc::Annotate(inner, callback, raised),
                })));
            }
            return self.worded_step(word, subject, &callback, &raised, span);
        }
        // the box built by hand: `effect (err r)` holds the failure as its
        // content, so it is the second hole in err's infectiousness
        if name == "effect" {
            let [v] = arity(args, name, span)?;
            return Ok(Value::Desc(Rc::new(Desc::Settled(v))));
        }
        if args.iter().any(is_failure) {
            return Ok(merged_failures(&args));
        }
        match name.as_bytes() {
            b"kill" => {
                let [handle] = arity(args, name, span)?;
                let Value::Int(handle) = &handle else {
                    return Err(RuntimeError {
                        at: None,
                        message: "kill takes a started process".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Kill(handle.to_i64().unwrap_or(-1)))))
            }
            b"run" | b"start" => {
                let [cmd, args_list] = arity(args, name, span)?;
                let Value::Str(cmd) = cmd else {
                    return Err(RuntimeError {
                        at: None,
                        message: format!("{name} takes a command string"),
                        span,
                    });
                };
                let Value::List(items) = args_list else {
                    return Err(RuntimeError {
                        at: None,
                        message: format!("{name} takes a list of argument strings"),
                        span,
                    });
                };
                let mut argv = Vec::new();
                for item in items.iter() {
                    let Value::Str(text) = item else {
                        return Err(RuntimeError {
                            at: None,
                            message: format!("{name} takes a list of argument strings"),
                            span,
                        });
                    };
                    argv.push(text.to_string());
                }
                Ok(Value::Desc(Rc::new(match name {
                    "start" => Desc::Start(cmd.to_string(), argv),
                    _ => Desc::Run(cmd.to_string(), argv),
                })))
            }
            b"read_file" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError {
                        at: None,
                        message: "read_file takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::ReadFile(path.to_string()))))
            }
            b"read_bytes" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError {
                        at: None,
                        message: "read_bytes takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::ReadBytes(path.to_string()))))
            }
            b"write" => {
                let [content] = arity(args, name, span)?;
                let Value::Str(content) = content else {
                    return Err(RuntimeError {
                        at: None,
                        message: "write takes a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Write(content.to_string()))))
            }
            // `args`, `stdin` and `now` are the three builtins a program names
            // without calling, so `eval_ident` answers all three and this
            // function only ever needed `now` — which is how it came to know
            // that one and neither of the others. The wasm backend routes
            // every builtin through here (`wasm_backend.rs` emits a
            // `RT_BUILTIN` call, `wasm_rt.rs` lands it on `call_builtin`), so
            // a page that asked for `stdin` got `unknown builtin` where the
            // same program run natively got a descriptor. `now` reaching the
            // executor on that engine was a coincidence of coverage, not a
            // design; these two arms make the coincidence a rule.
            // `read_line` is a fourth, named the same way (ruled 2026-10-13).
            b"args" | b"stdin" | b"read_line" | b"now" => {
                let [] = arity(args, name, span)?;
                Ok(Value::Desc(Rc::new(match name {
                    "args" => Desc::Args,
                    "stdin" => Desc::Stdin,
                    "read_line" => Desc::ReadLine,
                    _ => Desc::Now,
                })))
            }
            b"exists" | b"is_dir" | b"modified" | b"list_dir" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError {
                        at: None,
                        message: format!("{name} takes a string"),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(match name {
                    "exists" => Desc::Exists(path.to_string()),
                    "is_dir" => Desc::IsDir(path.to_string()),
                    "modified" => Desc::Modified(path.to_string()),
                    _ => Desc::ListDir(path.to_string()),
                })))
            }
            b"env" => {
                let [name] = arity(args, name, span)?;
                let Value::Str(name) = name else {
                    return Err(RuntimeError {
                        at: None,
                        message: "env takes a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Env(name.to_string()))))
            }
            b"write_err" => {
                let [content] = arity(args, name, span)?;
                let Value::Str(content) = content else {
                    return Err(RuntimeError {
                        at: None,
                        message: "write_err takes a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::WriteErr(content.to_string()))))
            }
            b"make_dir" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = &path else {
                    return Err(RuntimeError {
                        at: None,
                        message: "make_dir takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::MakeDir(path.to_string()))))
            }
            b"listen" => {
                let [port] = arity(args, name, span)?;
                let Value::Int(port) = &port else {
                    return Err(RuntimeError {
                        at: None,
                        message: "listen takes a port number".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Listen(port.to_i64().unwrap_or(-1)))))
            }
            b"net_port" => {
                let [listener] = arity(args, name, span)?;
                let Value::Int(listener) = &listener else {
                    return Err(RuntimeError {
                        at: None,
                        message: "port takes a listener".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::SocketPort(listener.to_i64().unwrap_or(-1)))))
            }
            b"accept" => {
                let [listener] = arity(args, name, span)?;
                let Value::Int(listener) = &listener else {
                    return Err(RuntimeError {
                        at: None,
                        message: "accept takes a listener".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Accept(listener.to_i64().unwrap_or(-1)))))
            }
            b"net_read" => {
                let [conn] = arity(args, name, span)?;
                let Value::Int(conn) = &conn else {
                    return Err(RuntimeError {
                        at: None,
                        message: "net_read takes a connection".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Receive(conn.to_i64().unwrap_or(-1)))))
            }
            b"net_write" => {
                let [conn, text] = arity(args, name, span)?;
                match (&conn, &text) {
                    (Value::Int(conn), Value::Str(text)) => Ok(Value::Desc(Rc::new(Desc::Send(
                        conn.to_i64().unwrap_or(-1),
                        text.to_string(),
                    )))),
                    (Value::Int(conn), Value::Bytes(raw)) => Ok(Value::Desc(Rc::new(
                        Desc::SendBytes(conn.to_i64().unwrap_or(-1), raw.clone()),
                    ))),
                    _ => Err(RuntimeError {
                        at: None,
                        message: "net_write takes a connection and a string".to_string(),
                        span,
                    }),
                }
            }
            b"net_close" => {
                let [handle] = arity(args, name, span)?;
                let Value::Int(handle) = &handle else {
                    return Err(RuntimeError {
                        at: None,
                        message: "net_close takes a listener or a connection".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::CloseSocket(handle.to_i64().unwrap_or(-1)))))
            }
            b"write_file" => {
                let [path, content] = arity(args, name, span)?;
                match (&path, &content) {
                    (Value::Str(path), Value::Str(content)) => Ok(Value::Desc(Rc::new(
                        Desc::WriteFile(path.to_string(), content.to_string()),
                    ))),
                    // bytes write back as they were read: the site's wasm
                    // module is read, digested and written under its new name
                    (Value::Str(path), Value::Bytes(raw)) => {
                        Ok(Value::Desc(Rc::new(Desc::WriteBytes(path.to_string(), raw.clone()))))
                    }
                    _ => Err(RuntimeError {
                        at: None,
                        message: "write_file takes a path and content strings".to_string(),
                        span,
                    }),
                }
            }
            b"sleep" => {
                let [ms] = arity(args, name, span)?;
                match ms {
                    Value::Int(n) => {
                        let ms = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Sleep(ms))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "sleep takes milliseconds (an int), got {}",
                            render(self, &other, false)
                        ),
                        span,
                    }),
                }
            }
            b"random" => {
                let [n] = arity(args, name, span)?;
                match n {
                    Value::Int(n) => {
                        let bound = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Random(bound))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "random takes a bound (an int), got {}",
                            render(self, &other, false)
                        ),
                        span,
                    }),
                }
            }
            b"print" => {
                let [text] = arity(args, name, span)?;
                match text {
                    Value::Str(s) => Ok(Value::Desc(Rc::new(Desc::Print(s.to_string(), span)))),
                    // any other value renders through the same ambient
                    // to_string dispatch interpolation uses
                    other => {
                        if is_failure(&other) {
                            return Ok(other);
                        }
                        match self.render_interpolated(other)? {
                            Ok(rendered) => Ok(Value::Desc(Rc::new(Desc::Print(rendered, span)))),
                            Err(failure) => Ok(failure),
                        }
                    }
                }
            }
            b"at" => {
                let [container, index] = arity(args, name, span)?;
                index_value(container, index, span)
            }
            b"push" => {
                let [list, item] = arity(args, name, span)?;
                // `text/bytes` answers `int[]` (appendix b), so a push onto
                // bytes answers the list of their values with the item after
                // them, as `concat` does.
                if let Value::Bytes(raw) = &list {
                    let mut next: Vec<Value> = raw.iter().map(|&b| Value::int(b)).collect();
                    next.push(item);
                    return Ok(Value::List(Rc::new(next)));
                }
                if !matches!(list, Value::List(_)) {
                    return Err(RuntimeError {
                        at: None,
                        message: format!("push takes a list and a value{}", lazy_hint(&list)),
                        span,
                    });
                }
                let Value::List(items) = list else { unreachable!("checked just above") };
                let alone = Rc::strong_count(&items) == 2;
                if self.writes_in_place(span, frame) || (alone && self.moved_here(span, frame)) {
                    Self::written_in_place(&items).push(item);
                    return Ok(Value::List(items));
                }
                let mut next = taken_to_grow(items, 1);
                next.push(item);
                Ok(Value::List(Rc::new(next)))
            }
            b"put" => {
                let [map, key, value] = arity(args, name, span)?;
                let Value::Map(entries) = map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "put takes a map, a key, and a value".to_string(),
                        span,
                    });
                };
                let key = map_key(key, span)?;
                let alone = Rc::strong_count(&entries) == 2;
                if self.writes_in_place(span, frame) || (alone && self.moved_here(span, frame)) {
                    let mut cache = self.entries_at.borrow_mut();
                    if matches!(&*cache,
                        Some((m, _)) if std::ptr::eq(m.as_ptr(), Rc::as_ptr(&entries)))
                    {
                        *cache = None;
                    }
                    drop(cache);
                    Self::written_in_place(&entries).insert(key, value);
                    return Ok(Value::Map(entries));
                }
                let mut next = taken(entries);
                next.insert(key, value);
                Ok(Value::Map(Rc::new(next)))
            }
            // The map without the key, ruled 2026-10-13 (kanso#1914). A key
            // that is not there answers an equal map. A fresh map unless
            // nothing else holds this one, so nothing the caller still holds
            // can see the key go.
            b"delete" => {
                let [map, key] = arity(args, name, span)?;
                let Value::Map(entries) = map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "delete takes a map and a key".to_string(),
                        span,
                    });
                };
                let key = map_key(key, span)?;
                // a map nobody else holds loses the key where it stands
                let alone = Rc::strong_count(&entries) == 2;
                if self.writes_in_place(span, frame) || (alone && self.moved_here(span, frame)) {
                    let mut cache = self.entries_at.borrow_mut();
                    if matches!(&*cache,
                        Some((m, _)) if std::ptr::eq(m.as_ptr(), Rc::as_ptr(&entries)))
                    {
                        *cache = None;
                    }
                    drop(cache);
                    Self::written_in_place(&entries).remove(&key);
                    return Ok(Value::Map(entries));
                }
                let mut next = (*entries).clone();
                next.remove(&key);
                Ok(Value::Map(Rc::new(next)))
            }
            b"entries" => {
                let [map] = arity(args, name, span)?;
                let Value::Map(map_entries) = &map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "entries takes a map".to_string(),
                        span,
                    });
                };
                let list = map_entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => Value::int(n.clone()),
                            MapKey::Str(s) => Value::Str(s.clone()),
                        };
                        Value::Record {
                            ty: Rc::from("entry"),
                            fields: Rc::new(RefCell::new(vec![key, value.clone()])),
                        }
                    })
                    .collect();
                Ok(Value::List(Rc::new(list)))
            }
            // The two columns of `entries`, in its order: position i of each
            // is the pair at position i of `entries m`.
            b"keys" => {
                let [map] = arity(args, name, span)?;
                let Value::Map(map_entries) = &map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "keys takes a map".to_string(),
                        span,
                    });
                };
                let list = map_entries
                    .iter()
                    .map(|(key, _)| match key {
                        MapKey::Int(n) => Value::int(n.clone()),
                        MapKey::Str(s) => Value::Str(s.clone()),
                    })
                    .collect();
                Ok(Value::List(Rc::new(list)))
            }
            b"values" => {
                let [map] = arity(args, name, span)?;
                let Value::Map(map_entries) = &map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "values takes a map".to_string(),
                        span,
                    });
                };
                Ok(Value::List(Rc::new(map_entries.iter().map(|(_, v)| v.clone()).collect())))
            }
            b"bytes" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError {
                        at: None,
                        message: "bytes takes a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Bytes(Rc::new(text.as_bytes().to_vec())))
            }
            // `text/bytes` covers strings; this covers numbers. Without it the
            // gavel would have taken away the only way to write byte data down
            // and left nothing in its place. Loud outside 0-255: a number that
            // is not a byte is a mistake, not a value to truncate.
            b"to_bytes" => {
                let [list] = arity(args, name, span)?;
                if is_failure(&list) {
                    return Ok(list);
                }
                let Value::List(items) = &list else {
                    return Err(RuntimeError {
                        at: None,
                        message: "to_bytes takes a list of byte values".to_string(),
                        span,
                    });
                };
                let mut raw = Vec::with_capacity(items.len());
                for item in items.iter() {
                    match item {
                        Value::Int(n) => match u8::try_from(n) {
                            Ok(b) => raw.push(b),
                            Err(_) => {
                                return Ok(err_value(
                                    Value::Str(Text::from(
                                        "to_bytes takes byte values (0-255)".to_string(),
                                    )),
                                    origin_at(frame, span),
                                ))
                            }
                        },
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        // a subtype of int is an int wherever one goes
                        other => match sub_base(other.clone()) {
                            Value::Int(n) if u8::try_from(&n).is_ok() => {
                                raw.push(u8::try_from(&n).expect("asked"))
                            }
                            _ => {
                                return Ok(err_value(
                                    Value::Str(Text::from(
                                        "to_bytes takes byte values (0-255)".to_string(),
                                    )),
                                    origin_at(frame, span),
                                ))
                            }
                        },
                    }
                }
                Ok(Value::Bytes(Rc::new(raw)))
            }
            // `text/bytes` answers an int list (appendix b), and native code
            // reads a bytes value as the list of its byte values on either
            // side. The answer is a list either way.
            b"concat" => {
                let [a, b] = arity(args, name, span)?;
                let as_items = |v: &Value| -> Option<Vec<Value>> {
                    match v {
                        Value::List(xs) => Some((**xs).clone()),
                        Value::Bytes(raw) => Some(raw.iter().map(|&b| Value::int(b)).collect()),
                        _ => None,
                    }
                };
                // a list nobody else holds is extended where it stands, as a
                // push onto it is; `concat acc acc` is never proven, so the
                // second list is not the one being grown
                if let Value::List(xs) = &a {
                    if self.writes_in_place(span, frame) {
                        if let Some(tail) = as_items(&b) {
                            Self::written_in_place(xs).extend(tail);
                            return Ok(a);
                        }
                    }
                }
                let (Some(mut joined), Some(tail)) = (as_items(&a), as_items(&b)) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "concat takes two lists".to_string(),
                        span,
                    });
                };
                joined.extend(tail);
                Ok(Value::List(Rc::new(joined)))
            }
            // The one declared acceptance the bytes gavel left standing: utf8
            // reads a list of small ints as well as real bytes, because there
            // is no bytes literal and `[104 105]` is the only spelling a
            // program can write down. It is a library choice, spelled the same
            // on every engine, rather than a list quietly being bytes.
            b"utf8" => {
                let [list] = arity(args, name, span)?;
                if is_failure(&list) {
                    return Ok(list);
                }
                let raw = match &list {
                    Value::Bytes(items) => (**items).clone(),
                    Value::List(items) => {
                        let mut raw = Vec::with_capacity(items.len());
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => match u8::try_from(n) {
                                    Ok(b) => raw.push(b),
                                    Err(_) => {
                                        return Ok(err_value(
                                            Value::Str(Text::from(
                                                "utf8 takes byte values (0-255)".to_string(),
                                            )),
                                            origin_at(frame, span),
                                        ))
                                    }
                                },
                                bad if is_failure(bad) => return Ok(bad.clone()),
                                _ => {
                                    return Ok(err_value(
                                        Value::Str(Text::from(
                                            "utf8 takes byte values (0-255)".to_string(),
                                        )),
                                        origin_at(frame, span),
                                    ))
                                }
                            }
                        }
                        raw
                    }
                    _ => {
                        return Err(RuntimeError {
                            at: None,
                            message: "utf8 takes a list of byte values".to_string(),
                            span,
                        })
                    }
                };
                match String::from_utf8(raw) {
                    Ok(text) => Ok(Value::Str(Text::from(text))),
                    Err(_) => Ok(err_value(
                        Value::Str(Text::from("invalid utf-8".to_string())),
                        origin_at(frame, span),
                    )),
                }
            }
            // Bytes the standard library assembled from text and ascii bytes
            // alone, so they are text already. The native runtime takes them
            // without reading them again; a Rust string has to be valid, so
            // this arm still converts with the check, and on such bytes the
            // check cannot fail.
            b"built_text" => {
                let [bytes] = arity(args, name, span)?;
                if is_failure(&bytes) {
                    return Ok(bytes);
                }
                let Value::Bytes(raw) = &bytes else {
                    return Err(RuntimeError {
                        at: None,
                        message: "built_text takes bytes".to_string(),
                        span,
                    });
                };
                match String::from_utf8((**raw).clone()) {
                    Ok(text) => Ok(Value::Str(Text::from(text))),
                    Err(_) => Err(RuntimeError {
                        at: None,
                        message: "built_text was handed bytes that are not text".to_string(),
                        span,
                    }),
                }
            }
            b"split" => {
                let [text, sep] = arity(args, name, span)?;
                let (Value::Str(text), Value::Str(sep)) = (&text, &sep) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "split takes two strings".to_string(),
                        span,
                    });
                };
                if sep.is_empty() {
                    return Err(RuntimeError {
                        at: None,
                        message: "split needs a separator".to_string(),
                        span,
                    });
                }
                let list = text
                    .split(sep.as_str())
                    .map(|p| Value::Str(Text::from(p.to_string())))
                    .collect::<Vec<_>>();
                Ok(Value::List(Rc::new(list)))
            }
            b"chars" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError {
                        at: None,
                        message: "chars takes a string".to_string(),
                        span,
                    });
                };
                let list = text.chars().map(|c| Value::Str(Text::from(c.to_string()))).collect();
                Ok(Value::List(Rc::new(list)))
            }
            b"char_code" => {
                let [c] = arity(args, name, span)?;
                let code = match &c {
                    Value::Str(s) if s.chars().count() == 1 => {
                        s.chars().next().expect("length checked") as u32
                    }
                    _ => {
                        return Err(RuntimeError {
                            at: None,
                            message: "char_code takes a one-character string".to_string(),
                            span,
                        })
                    }
                };
                Ok(Value::int(code))
            }
            b"from_code" => {
                let [code] = arity(args, name, span)?;
                let Value::Int(n) = &code else {
                    return Err(RuntimeError {
                        at: None,
                        message: "from_code takes an int".to_string(),
                        span,
                    });
                };
                let scalar = u32::try_from(n).ok().and_then(char::from_u32);
                match scalar {
                    Some(c) => Ok(Value::Str(Text::from(c.to_string()))),
                    None => Ok(err_value(
                        Value::Str(Text::from("not a unicode scalar value".to_string())),
                        origin_at(frame, span),
                    )),
                }
            }
            b"join" => {
                let [list, sep] = arity(args, name, span)?;
                let (Value::List(items), Value::Str(sep)) = (&list, &sep) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "join takes a list of strings and a separator".to_string(),
                        span,
                    });
                };
                let mut parts = Vec::new();
                for item in items.iter() {
                    // A list stores what it was given, so an element built by a
                    // binding arrives as a pending computation. Joining demands
                    // the string, so ask for it rather than blaming the caller
                    // for what it never sent.
                    let item = self.force(item.clone())?;
                    match &item {
                        Value::Str(s) => parts.push(s.clone()),
                        bad if is_failure(bad) => return Ok(item.clone()),
                        // a subtype of string is a string wherever one goes
                        _ => match sub_base(item) {
                            Value::Str(s) => parts.push(s),
                            _ => {
                                return Err(RuntimeError {
                                    at: None,
                                    message: "join takes a list of strings".to_string(),
                                    span,
                                })
                            }
                        },
                    }
                }
                Ok(Value::Str(
                    parts.iter().map(Text::as_str).collect::<Vec<_>>().join(sep.as_str()).into(),
                ))
            }
            b"append" => {
                let [acc, x] = arity(args, name, span)?;
                for v in [&acc, &x] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let Value::Bytes(items) = acc else {
                    return Err(RuntimeError {
                        at: None,
                        message: "append takes bytes and a string, bytes, or byte".to_string(),
                        span,
                    });
                };
                let grows_by = match &x {
                    Value::Str(s) => s.len(),
                    Value::Bytes(more) => more.len(),
                    _ => 1,
                };
                let alone = Rc::strong_count(&items) == 2;
                let extend = |out: &mut Vec<u8>| match &x {
                    Value::Str(s) => {
                        out.extend_from_slice(s.as_bytes());
                        Ok(())
                    }
                    Value::Bytes(more) => {
                        out.extend_from_slice(more);
                        Ok(())
                    }
                    // The low byte, which is what the compiled engine takes:
                    // `x.payload & 0xff`. Matching it is the differential law;
                    // whether either engine should instead refuse a number
                    // above 255 here is a question this change does not answer.
                    Value::Int(n) => {
                        out.push(low_byte(n));
                        Ok(())
                    }
                    _ => Err(RuntimeError {
                        at: None,
                        message: "append takes bytes and a string, bytes, or byte".to_string(),
                        span,
                    }),
                };
                let in_place =
                    self.writes_in_place(span, frame) || (alone && self.moved_here(span, frame));
                // Bytes appended to themselves would be read through the
                // pointer being written, so that one call takes the contents.
                let onto_itself = matches!(&x, Value::Bytes(more) if Rc::ptr_eq(more, &items));
                if in_place && !onto_itself {
                    extend(Self::written_in_place(&items))?;
                    return Ok(Value::Bytes(items));
                }
                let mut out = match in_place {
                    true => Self::taken_in_place(&items),
                    false => taken_to_grow(items, grows_by),
                };
                extend(&mut out)?;
                Ok(Value::Bytes(Rc::new(out)))
            }
            b"find2_below" => {
                let [cs, from, a, b, lim] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b, &lim] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (
                    Value::Bytes(items),
                    Value::Int(from),
                    Value::Int(a),
                    Value::Int(b),
                    Value::Int(lim),
                ) = (&cs, &from, &a, &b, &lim)
                else {
                    return Err(RuntimeError {
                        at: None,
                        message: "find2_below takes bytes".to_string(),
                        span,
                    });
                };
                let (a, b, lim) = (low_byte(a), low_byte(b), lim.clone());
                let len = items.len();
                let start = usize::try_from(from).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, byte) in items.iter().enumerate().skip(start - 1) {
                    if *byte == a || *byte == b || Int::from(*byte) < lim {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::int(at))
            }
            b"number_span" => {
                let [cs, from] = arity(args, name, span)?;
                for v in [&cs, &from] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::Bytes(items), Value::Int(from)) = (&cs, &from) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "number_span takes bytes and a position".to_string(),
                        span,
                    });
                };
                // The first position at or after `from` whose byte cannot be
                // part of a number, negated when a `.`, `e` or `E` went past.
                // A position outside the bytes is its own answer, and so is
                // `from` when no byte there can be part of a number. RFC 8259
                // asks three things the converters do not (#1999): a digit
                // first, after any minus; no digit after a leading 0; and a
                // digit after a point. A number that breaks one answers zero,
                // as k_b_number_span does.
                let len = items.len();
                let start = match usize::try_from(from) {
                    Ok(p) if p >= 1 && p <= len => p,
                    _ => return Ok(Value::Int(from.clone())),
                };
                let number_byte =
                    |c: u8| matches!(c, b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E');
                let digit_at = |i: usize| items.get(i).is_some_and(u8::is_ascii_digit);
                if number_byte(items[start - 1]) {
                    let lead = start - 1 + usize::from(items[start - 1] == b'-');
                    if !digit_at(lead) || (items[lead] == b'0' && digit_at(lead + 1)) {
                        return Ok(Value::int(0));
                    }
                }
                let mut at = start - 1;
                let mut float = false;
                let mut bare = false;
                while at < len && number_byte(items[at]) {
                    float |= matches!(items[at], b'.' | b'e' | b'E');
                    bare |= items[at] == b'.' && !digit_at(at + 1);
                    at += 1;
                }
                if bare {
                    return Ok(Value::int(0));
                }
                let end = (at + 1) as i64;
                Ok(Value::int(if float { -end } else { end }))
            }
            b"plain_string" => {
                let [cs, from] = arity(args, name, span)?;
                for v in [&cs, &from] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::Bytes(items), Value::Int(from)) = (&cs, &from) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "plain_string takes bytes and a position".to_string(),
                        span,
                    });
                };
                // The first quote or backslash at or after `from`. A quote
                // answers the text in front of it, when that is utf-8; a
                // backslash answers its own position; anything else is none,
                // as k_b_plain_string answers.
                let start = match usize::try_from(from) {
                    Ok(p) if p >= 1 => p,
                    _ => return Ok(Value::NoneV),
                };
                let found = items
                    .get(start - 1..)
                    .and_then(|rest| rest.iter().position(|&b| b == b'"' || b == b'\\'));
                let Some(off) = found else {
                    return Ok(Value::NoneV);
                };
                let mut at = start - 1 + off;
                if items[at] == b'\\' {
                    // the closing quote, each backslash taking the byte
                    // after it along, as k_json_quote_after finds it
                    loop {
                        let next = items
                            .get(at + 2..)
                            .and_then(|rest| rest.iter().position(|&b| b == b'"' || b == b'\\'));
                        let Some(off) = next else {
                            return Ok(Value::NoneV);
                        };
                        at += 2 + off;
                        if items[at] == b'"' {
                            return Ok(Value::int(at + 1));
                        }
                    }
                }
                match std::str::from_utf8(&items[start - 1..at]) {
                    Ok(s) => Ok(Value::Str(Text::from(s.to_string()))),
                    Err(_) => Ok(Value::NoneV),
                }
            }
            b"entry_count" => {
                let [map] = arity(args, name, span)?;
                if is_failure(&map) {
                    return Ok(map);
                }
                let Value::Map(entries) = &map else {
                    return Err(RuntimeError {
                        at: None,
                        message: "entry_count takes a map".to_string(),
                        span,
                    });
                };
                Ok(Value::int(entries.len() as i64))
            }
            b"key_at" | b"value_at" => {
                let [map, at] = arity(args, name, span)?;
                for v in [&map, &at] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::Map(entries), Value::Int(at)) = (&map, &at) else {
                    let message = match name {
                        "key_at" => "key_at takes a map and a position",
                        _ => "value_at takes a map and a position",
                    };
                    return Err(RuntimeError { at: None, message: message.to_string(), span });
                };
                let Some(i) = at.to_i64().filter(|&i| i >= 1 && i as u64 <= entries.len() as u64)
                else {
                    return Ok(Value::NoneV);
                };
                let (key, value) = match &**entries {
                    Entries::Few(few) => {
                        let (k, v) = &few[i as usize - 1];
                        (k.clone(), v.clone())
                    }
                    Entries::Many(_) => {
                        let mut cache = self.entries_at.borrow_mut();
                        let hit = matches!(&*cache,
                            Some((m, _)) if std::ptr::eq(m.as_ptr(), Rc::as_ptr(entries)));
                        if !hit {
                            let all: Rc<[(MapKey, Value)]> =
                                entries.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                            *cache = Some((Rc::downgrade(entries), all));
                        }
                        let (_, all) = cache.as_ref().expect("the cache was just filled");
                        all[i as usize - 1].clone()
                    }
                };
                Ok(match name {
                    "key_at" => match key {
                        MapKey::Int(n) => Value::int(n),
                        MapKey::Str(s) => Value::Str(s),
                    },
                    _ => value,
                })
            }
            b"append_escaped" | b"append_key" | b"append_string" => {
                let [acc, s] = arity(args, name, span)?;
                for v in [&acc, &s] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let key = name == "append_key";
                let quoted = key || name == "append_string";
                let (Value::Bytes(items), Value::Str(text)) = (acc, &s) else {
                    let message = match name {
                        "append_key" => "append_key takes bytes and a string",
                        "append_string" => "append_string takes bytes and a string",
                        _ => "append_escaped takes bytes and a string",
                    };
                    return Err(RuntimeError { at: None, message: message.to_string(), span });
                };
                // `s` as a JSON string holds it, in lib/json's forms, as
                // k_b_append_escaped writes it; in place where `append` would be.
                // `append_key` writes it as an object key, quoted and followed
                // by a colon.
                let mut escaped = Vec::with_capacity(text.len() + 8);
                if quoted {
                    escaped.push(b'"');
                }
                for &c in text.as_bytes() {
                    match c {
                        b'"' => escaped.extend_from_slice(b"\\\""),
                        b'\\' => escaped.extend_from_slice(b"\\\\"),
                        8 => escaped.extend_from_slice(b"\\b"),
                        9 => escaped.extend_from_slice(b"\\t"),
                        10 => escaped.extend_from_slice(b"\\n"),
                        12 => escaped.extend_from_slice(b"\\f"),
                        13 => escaped.extend_from_slice(b"\\r"),
                        c if c < 32 => escaped.extend_from_slice(format!("\\u{c:04x}").as_bytes()),
                        c => escaped.push(c),
                    }
                }
                if key {
                    escaped.extend_from_slice(b"\":");
                } else if quoted {
                    escaped.push(b'"');
                }
                let alone = Rc::strong_count(&items) == 2;
                if self.writes_in_place(span, frame) || (alone && self.moved_here(span, frame)) {
                    Self::written_in_place(&items).extend_from_slice(&escaped);
                    return Ok(Value::Bytes(items));
                }
                let mut out = taken_to_grow(items, escaped.len());
                out.extend_from_slice(&escaped);
                Ok(Value::Bytes(Rc::new(out)))
            }
            b"unescape" => {
                let [cs, from, end] = arity(args, name, span)?;
                for v in [&cs, &from, &end] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::Bytes(items), Value::Int(from), Value::Int(end)) = (&cs, &from, &end)
                else {
                    return Err(RuntimeError {
                        at: None,
                        message: "unescape takes bytes and two positions".to_string(),
                        span,
                    });
                };
                // The bytes from `from` up to the closing quote at `end`, with
                // the eight one-letter escapes undone; anything else is none,
                // as k_b_unescape answers.
                let (Ok(from), Ok(end)) = (usize::try_from(from), usize::try_from(end)) else {
                    return Ok(Value::NoneV);
                };
                if from < 1 || end < from || end > items.len() + 1 {
                    return Ok(Value::NoneV);
                }
                let range = &items[from - 1..end - 1];
                let mut out = Vec::with_capacity(range.len());
                let mut i = 0;
                while i < range.len() {
                    if range[i] != b'\\' {
                        out.push(range[i]);
                        i += 1;
                        continue;
                    }
                    let undone = match range.get(i + 1) {
                        Some(b'"') => b'"',
                        Some(b'\\') => b'\\',
                        Some(b'/') => b'/',
                        Some(b'b') => 8,
                        Some(b'f') => 12,
                        Some(b'n') => 10,
                        Some(b'r') => 13,
                        Some(b't') => 9,
                        _ => return Ok(Value::NoneV),
                    };
                    out.push(undone);
                    i += 2;
                }
                match String::from_utf8(out) {
                    Ok(s) => Ok(Value::Str(Text::from(s))),
                    Err(_) => Ok(Value::NoneV),
                }
            }
            b"find2" => {
                let [cs, from, a, b] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::Bytes(items), Value::Int(from), Value::Int(a), Value::Int(b)) =
                    (&cs, &from, &a, &b)
                else {
                    return Err(RuntimeError {
                        at: None,
                        message: "find2 takes bytes".to_string(),
                        span,
                    });
                };
                let (a, b) = (low_byte(a), low_byte(b));
                let len = items.len();
                let start = usize::try_from(from).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, byte) in items.iter().enumerate().skip(start - 1) {
                    if *byte == a || *byte == b {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::int(at))
            }
            // `slice_in` is std's spelling for a range it measured (#1890). It
            // is renamed to `slice` after the check, before any engine runs, so
            // this arm only keeps the two names answering alike.
            b"slice" | b"slice_in" => {
                let [container, from, to] = arity(args, name, span)?;
                let (Value::Int(from), Value::Int(to)) = (&from, &to) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "slice takes 1-based inclusive positions".to_string(),
                        span,
                    });
                };
                let from = usize::try_from(from).unwrap_or(0);
                let to = usize::try_from(to).unwrap_or(0);
                // A range that ends past the value, or starts outside it,
                // answers none, as an index outside it does (#1890). A start
                // before 1, or one no machine word holds, reads as 0 here.
                // The empty range just past the end is still the empty tail.
                let len = match &container {
                    Value::List(items) => Some(items.len()),
                    Value::Bytes(items) => Some(items.len()),
                    Value::Str(text) if text.is_ascii() => Some(text.len()),
                    // A string with a wider character answers below, by walking.
                    _ => None,
                };
                if let Some(len) = len {
                    if to > len || from == 0 || from > len + 1 {
                        return Ok(Value::NoneV);
                    }
                }
                match &container {
                    Value::List(items) => {
                        let sliced = slice_range(items.len(), from, to)
                            .map(|r| items[r].to_vec())
                            .unwrap_or_default();
                        Ok(Value::List(Rc::new(sliced)))
                    }
                    Value::Bytes(items) => {
                        let sliced = slice_range(items.len(), from, to)
                            .map(|r| items[r].to_vec())
                            .unwrap_or_default();
                        Ok(Value::Bytes(Rc::new(sliced)))
                    }
                    // A position is a byte's in an ASCII string, so a scan that
                    // slices one character at a time reads only that character.
                    Value::Str(text) if text.is_ascii() => {
                        let sliced = slice_range(text.len(), from, to).map(|r| &text[r]);
                        Ok(Value::Str(Text::from(sliced.unwrap_or_default())))
                    }
                    // A string with a wider character is walked to the range,
                    // on from where the last slice of it ended, where it used to
                    // be collected whole.
                    Value::Str(text) => {
                        if from == 0 {
                            return Ok(Value::NoneV);
                        }
                        let start = self.char_offset(text, from - 1);
                        match (start, self.char_offset(text, to)) {
                            (Some(start), Some(end)) if start < end => {
                                Ok(Value::Str(Text::from(&text[start..end])))
                            }
                            (Some(_), Some(_)) => Ok(Value::Str(Text::from(""))),
                            _ => Ok(Value::NoneV),
                        }
                    }
                    _ => Err(RuntimeError {
                        at: None,
                        message: "slice takes a list or string".to_string(),
                        span,
                    }),
                }
            }
            b"bit_and" | b"bit_or" | b"bit_xor" => {
                let [a, b] = arity(args, name, span)?;
                let (x, y) = match (whole(a, name, span)?, whole(b, name, span)?) {
                    (Ok(x), Ok(y)) => (x, y),
                    (Err(bad), _) | (_, Err(bad)) => return Ok(bad),
                };
                let bits = match name {
                    "bit_and" => x & y,
                    "bit_or" => x | y,
                    _ => x ^ y,
                };
                Ok(Value::int(bits))
            }
            b"bit_not" => {
                let [a] = arity(args, name, span)?;
                match whole(a, name, span)? {
                    Ok(x) => Ok(Value::int(!x)),
                    Err(bad) => Ok(bad),
                }
            }
            b"bit_shl" | b"bit_shr" => {
                let [a, b] = arity(args, name, span)?;
                let (bits, by) = match (whole(a, name, span)?, whole(b, name, span)?) {
                    (Ok(x), Ok(y)) => (x, y),
                    (Err(bad), _) | (_, Err(bad)) => return Ok(bad),
                };
                if !(0..=63).contains(&by) {
                    return Err(RuntimeError {
                        at: None,
                        message: format!("{} shifts by 0 to 63 places, got {by}", spelled(name)),
                        span,
                    });
                }
                let out = match name {
                    "bit_shl" => ((bits as u64) << by) as i64,
                    _ => bits >> by,
                };
                Ok(Value::int(out))
            }
            // Ruled 2026-10-04: `list/tie keys maker` calls the maker once
            // per key with the key and `ref`, and answers the map from key to
            // node. Every key's cell exists before the first node is made, so
            // a reference to a key made later is the same cell that key's
            // node fills. A reference is stored, never read, while the maker
            // runs: a constructor keeps a cell that is still a blackhole.
            b"tie" => {
                let [keys, maker, broken] = arity(args, name, span)?;
                let Value::List(keys) = keys else {
                    return Err(RuntimeError {
                        at: None,
                        message: "tie takes a list of ids and a maker".to_string(),
                        span,
                    });
                };
                let mut cells = Entries::new();
                for key in keys.iter() {
                    let cell = Rc::new(RefCell::new(ThunkState::Blackhole));
                    cells.insert(map_key(key.clone(), span)?, Value::Thunk(cell));
                }
                let serial = self.tie_serial.get() + 1;
                self.tie_serial.set(serial);
                self.ties.borrow_mut().push(Tie { serial, cells, broken: None });
                let made = self.make_tied(serial, &keys, maker, span, frame);
                let tie = self.ties.borrow_mut().pop().expect("the tie this call pushed");
                let made = made?;
                let Value::Map(nodes) = &made else { return Ok(made) };
                if let Some((from, to)) = tie.broken {
                    return self.call(broken, vec![from, to], span, frame);
                }
                for (key, cell) in tie.cells.iter() {
                    let (Value::Thunk(cell), Some(node)) = (cell, nodes.get(key)) else {
                        continue;
                    };
                    *cell.borrow_mut() = ThunkState::Forced(node.clone());
                }
                let mut seen: crate::hash::Set<usize> = Default::default();
                for (_, node) in nodes.iter() {
                    resolve_tied(node, &mut seen);
                }
                Ok(made)
            }
            // What a maker's `ref` answers: the cell for `key`. A key no call
            // makes still gets a cell, so the node under construction is
            // built, and the tie answers the first such link as broken.
            b"tie_ref" => {
                let [serial, from, key] = arity(args, name, span)?;
                let Value::Int(serial) = serial else { unreachable!("a ref carries its tie") };
                let serial = serial.to_i64().expect("a tie's serial is small");
                let mut ties = self.ties.borrow_mut();
                let Some(tie) = ties.iter_mut().rev().find(|t| t.serial == serial) else {
                    return Err(RuntimeError {
                        at: None,
                        message: "a reference from list/tie was asked for after its tie returned"
                            .to_string(),
                        span,
                    });
                };
                let wanted = map_key(key.clone(), span)?;
                if let Some(cell) = tie.cells.get(&wanted) {
                    let cell: Value = cell.clone();
                    return Ok(cell);
                }
                if tie.broken.is_none() {
                    tie.broken = Some((from, key));
                }
                Ok(Value::Thunk(Rc::new(RefCell::new(ThunkState::Blackhole))))
            }
            b"sqrt" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Float(v) => Ok(Value::Float(v.sqrt())),
                    Value::Int(n) => Ok(Value::Float(int_f(&n).sqrt())),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "sqrt takes a number, got {}",
                            render(self, &other, false)
                        ),
                        span,
                    }),
                }
            }
            b"round" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Int(n) => Ok(Value::Int(n)),
                    // Every finite float rounds to an integer, and an int is
                    // arbitrary precision, so 1e30 rounds to a thirty-one
                    // digit number. `as i64` saturated it at int64's edge.
                    Value::Float(v) if v.is_finite() => {
                        let whole = <BigInt as num_traits::FromPrimitive>::from_f64(v.round())
                            .expect("a finite float is a whole number once rounded");
                        Ok(Value::int(whole))
                    }
                    // NaN and the infinities have no integer to round to, and
                    // ordinary arithmetic makes them, so the answer is `none`
                    // for an arm to handle (ruled 2026-09-29). `math/round!`
                    // is the caller insisting the float is finite.
                    Value::Float(_) => Ok(Value::NoneV),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "round takes a number, got {}",
                            render(self, &other, false)
                        ),
                        span,
                    }),
                }
            }
            b"to_int" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::Bytes(items) => Text::from(match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str(Text::from("bytes are not an integer".to_string())),
                                origin_at(frame, span),
                            ))
                        }
                    }),
                    Value::Int(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            at: None,
                            message: format!(
                                "to_int takes a string, bytes, or int, not {}",
                                render(self, &value, true)
                            ),
                            span,
                        })
                    }
                };
                // num-bigint reads `_` as a digit separator, which neither
                // `to_float` nor the native parse does, so "1_000" was an
                // integer on this engine alone.
                let parsed = match text.contains('_') {
                    true => None,
                    false => text.parse::<BigInt>().ok(),
                };
                Ok(match parsed {
                    Some(n) => Value::int(n),
                    None => err_value(
                        Value::Str(format!("\"{text}\" is not an integer").into()),
                        origin_at(frame, span),
                    ),
                })
            }
            b"to_float" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::Bytes(items) => Text::from(match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str(Text::from("bytes are not a number".to_string())),
                                origin_at(frame, span),
                            ))
                        }
                    }),
                    Value::Int(n) => {
                        let approx = n.to_string().parse::<f64>().unwrap_or(f64::INFINITY);
                        return Ok(Value::Float(approx));
                    }
                    Value::Float(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            at: None,
                            message: format!(
                                "to_float takes a string, bytes, or number, not {}",
                                render(self, &value, true)
                            ),
                            span,
                        })
                    }
                };
                Ok(match text.parse::<f64>() {
                    Ok(x) => Value::Float(x),
                    Err(_) => err_value(
                        Value::Str(format!("\"{text}\" is not a number").into()),
                        origin_at(frame, span),
                    ),
                })
            }
            b"is_desc" => {
                let [v] = arity(args, name, span)?;
                Ok(match v {
                    Value::Desc(..) => Value::True,
                    _ => Value::False,
                })
            }
            b"render_value" => {
                let [v] = arity(args, name, span)?;
                Ok(Value::Str(Text::from(render(self, &v, false))))
            }
            b"length" => {
                let [list] = arity(args, name, span)?;
                match list {
                    Value::List(items) => Ok(Value::int(items.len())),
                    Value::Bytes(items) => Ok(Value::int(items.len())),
                    Value::Str(s) => Ok(Value::int(s.char_count())),
                    Value::Map(entries) => Ok(Value::int(entries.len())),
                    other => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "length takes a list, string, or map, not {}{}",
                            render(self, &other, true),
                            lazy_hint(&other)
                        ),
                        span,
                    }),
                }
            }
            b"map" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError {
                        at: None,
                        message: "map takes a list".to_string(),
                        span,
                    });
                };
                let mapped = items
                    .iter()
                    .map(|item| self.call(f.clone(), vec![item.clone()], span, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(mapped)))
            }
            b"filter" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError {
                        at: None,
                        message: "filter takes a list".to_string(),
                        span,
                    });
                };
                let mut kept = Vec::new();
                for item in items.iter() {
                    match self.call(f.clone(), vec![item.clone()], span, frame)? {
                        Value::True => kept.push(item.clone()),
                        Value::False => {}
                        other => {
                            return Err(RuntimeError {
                                at: None,
                                message: format!(
                                    "a filter predicate returns true or false, got {}",
                                    render(self, &other, false)
                                ),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::List(Rc::new(kept)))
            }
            b"sort" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError {
                        at: None,
                        message: "sort takes a list".to_string(),
                        span,
                    });
                };
                let mut sorted = (*items).clone();
                let mut failed = false;
                sorted.sort_by(|a, b| match compare(a, b) {
                    Some(ord) => ord,
                    None => {
                        failed = true;
                        std::cmp::Ordering::Equal
                    }
                });
                match failed {
                    true => Err(RuntimeError {
                        at: None,
                        message: "sort requires comparable elements of one type".to_string(),
                        span,
                    }),
                    false => Ok(Value::List(Rc::new(sorted))),
                }
            }
            b"sum" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError {
                        at: None,
                        message: "sum takes a list".to_string(),
                        span,
                    });
                };
                let mut total = Int::Small(0);
                for item in items.iter() {
                    match item {
                        Value::Int(n) => total = total.add(n),
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                at: None,
                                message: "sum takes a list of int".to_string(),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::int(total))
            }
            _ => Err(RuntimeError { at: None, message: format!("unknown builtin `{name}`"), span }),
        }
    }
}
