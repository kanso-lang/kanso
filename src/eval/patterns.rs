// Matching a call's arguments against an arm's patterns: the score that
// picks the arm, the bindings it makes, and the tie links a match settles.
// eval.rs reads this file with include!, so the items stay in the eval
// module, as the other files under src/eval/ do (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under src/'s
// subdirectories itself.

/// Score one candidate into buffers the caller owns, answering whether it
/// matched at all.
///
/// The vectors used to be built here, at the size the parameter list states,
/// once per candidate. That is the right size and the wrong lifetime: arm
/// selection tries every overload in the group and keeps one, so the
/// allocations for every candidate that refused were paid and thrown away.
/// The caller clears and reuses one pair instead, and `binds` keeps whatever
/// capacity the group's widest arm reached.
fn match_params_into(
    params: &[Pattern],
    args: &[Value],
    score: &mut Score,
    binds: &mut Bindings,
) -> bool {
    score.clear();
    binds.clear();
    for (pattern, arg) in params.iter().zip(args) {
        // per-param: literals 200, annotated 100 minus subtype distance
        // (nearer declarations outrank ancestors), generics 10 — the old
        // three-rank ladder, widened so chain depth can order within a rank
        let base: u8 = match pattern.rank() {
            0 => 200,
            1 => 100,
            _ => 10,
        };
        let Some(depth) = match_top(pattern, arg, binds) else {
            return false;
        };
        score.push(base.saturating_sub(depth));
    }
    true
}

/// `match_one` for a whole parameter, which binds a name without cloning.
///
/// A parameter that is a name, bare or annotated, bound a clone of its
/// argument, and arm selection tries every candidate, so each one paid for
/// the clone whether it won or not: on the interpreted corpus that was 620,601
/// clones from `match_one`, 26,424,710 instructions, with as many drops after.
/// The binding now holds `none` until an arm has won, and `bind_moved` puts
/// the argument itself there, since the argument vector is cleared before the
/// body runs and nothing reads it after.
fn match_top(pattern: &Pattern, arg: &Value, binds: &mut Bindings) -> Option<u8> {
    match pattern {
        Pattern::Var(name, _) => match is_failure(arg) {
            true => None,
            false => {
                binds.push((name.clone(), Value::NoneV));
                Some(0)
            }
        },
        Pattern::Annotated { name, ty, .. } => {
            let depth = type_match_depth(ty, arg)?;
            binds.push((name.clone(), Value::NoneV));
            Some(depth)
        }
        _ => match_one(pattern, arg, binds),
    }
}

/// How many names a pattern binds when it matches, which is how many entries
/// `match_one` pushes for it.
fn binder_count(pattern: &Pattern) -> usize {
    match pattern {
        Pattern::Var(..) | Pattern::Annotated { .. } => 1,
        Pattern::Ctor { fields, whole, .. } => {
            fields.iter().map(binder_count).sum::<usize>() + usize::from(whole.is_some())
        }
        Pattern::IntLit(..)
        | Pattern::StrLit(..)
        | Pattern::Nullary(..)
        | Pattern::Wildcard(..)
        | Pattern::Keyed { .. } => 0,
    }
}

/// The winning arm's name parameters take their arguments, moved out of the
/// argument vector into the places `match_top` held for them.
fn bind_moved(params: &[Pattern], args: &mut [Value], binds: &mut Bindings) {
    let mut at = 0;
    for (pattern, arg) in params.iter().zip(args.iter_mut()) {
        match pattern {
            Pattern::Var(..) | Pattern::Annotated { .. } => {
                binds[at].1 = std::mem::replace(arg, Value::NoneV);
                at += 1;
            }
            _ => at += binder_count(pattern),
        }
    }
}

/// The as-pattern's name takes the value the shape matched — the same value
/// the caller passed, not one rebuilt from the parts.
fn bind_whole(whole: &Option<Box<(Name, crate::diag::Span)>>, arg: &Value, binds: &mut Bindings) {
    if let Some(named) = whole {
        binds.push((named.0.clone(), arg.clone()));
    }
}

/// Every field below a tie's nodes that holds a filled cell is rewritten to
/// hold the node, as native does before it releases its cells. A graph left
/// holding cells reads the same through a field, but a nested pattern
/// compares the field as it stands, and `(ring _ (ring d _))` matched native's
/// node and missed the interpreter's cell. Records are rewritten in place, so
/// the walk goes once through each and never through a cell.
/// A field's value with its filled links in place of the cells that held
/// them: the node itself, or a list or map literal of references rebuilt with
/// the nodes. Native stores the node where the reference was, so a read of
/// either never forces anything, and the interpreter counts the same.
fn settled_link(v: &Value) -> Option<Value> {
    let forced = |v: &Value| match v {
        Value::Thunk(cell) => match &*cell.borrow() {
            ThunkState::Forced(node) => Some(node.clone()),
            _ => None,
        },
        _ => None,
    };
    match v {
        Value::Thunk(_) => forced(v),
        Value::List(items) if items.iter().any(|item| forced(item).is_some()) => {
            Some(Value::List(Rc::new(
                items.iter().map(|item| forced(item).unwrap_or_else(|| item.clone())).collect(),
            )))
        }
        Value::Map(entries) if entries.iter().any(|(_, value)| forced(value).is_some()) => {
            let mut settled = Entries::new();
            for (key, value) in entries.iter() {
                settled.insert(key.clone(), forced(value).unwrap_or_else(|| value.clone()));
            }
            Some(Value::Map(Rc::new(settled)))
        }
        _ => None,
    }
}

fn resolve_tied(v: &Value, seen: &mut crate::hash::Set<usize>) {
    match v {
        Value::Record { fields, .. } => {
            if !seen.insert(Rc::as_ptr(fields) as *const () as usize) {
                return;
            }
            let held: Vec<Value> = {
                let mut fields = fields.borrow_mut();
                for slot in fields.iter_mut() {
                    if let Some(settled) = settled_link(slot) {
                        *slot = settled;
                    }
                }
                fields.clone()
            };
            for field in &held {
                resolve_tied(field, seen);
            }
        }
        Value::List(items) => {
            if seen.insert(Rc::as_ptr(items) as *const () as usize) {
                items.iter().for_each(|item| resolve_tied(item, seen));
            }
        }
        Value::Map(entries) => {
            if seen.insert(Rc::as_ptr(entries) as *const () as usize) {
                entries.iter().for_each(|(_, value)| resolve_tied(value, seen));
            }
        }
        Value::Sub { inner, .. } => resolve_tied(inner, seen),
        _ => {}
    }
}

fn match_one(pattern: &Pattern, arg: &Value, binds: &mut Bindings) -> Option<u8> {
    match (pattern, arg) {
        (Pattern::IntLit(n, _), Value::Int(v)) if v == n => Some(0),
        (Pattern::StrLit(s, _), Value::Str(v)) if **s == **v => Some(0),
        (Pattern::Nullary(name, _), Value::True) if name == "true" => Some(0),
        (Pattern::Nullary(name, _), Value::False) if name == "false" => Some(0),
        (Pattern::Nullary(name, _), Value::NoneV) if name == "none" => Some(0),
        (Pattern::Nullary(name, _), Value::Done) if name == "done" => Some(0),
        (Pattern::Wildcard(_), _) => match is_failure(arg) {
            true => None,
            false => Some(0),
        },
        (Pattern::Var(name, _), _) => match is_failure(arg) {
            true => None,
            false => {
                binds.push((name.clone(), arg.clone()));
                Some(0)
            }
        },
        (Pattern::Annotated { name, ty, .. }, _) => match type_match_depth(ty, arg) {
            Some(depth) => {
                binds.push((name.clone(), arg.clone()));
                Some(depth)
            }
            None => None,
        },
        (Pattern::Keyed { .. }, _) => None,
        (Pattern::Ctor { ty, fields, whole }, Value::Unboxed(info)) if ty == "err" => {
            match fields.len() == 1 {
                true => {
                    let inner = match_one(&fields[0], &info.reason, binds)?;
                    bind_whole(whole, arg, binds);
                    // a bare reason binder demands err-ness but names
                    // nothing: it ranks below every named reason — leaf
                    // and typeset alike — and just above the plain
                    // generics
                    Some(match &fields[0] {
                        Pattern::Var(..) | Pattern::Wildcard(_) => 89,
                        _ => inner,
                    })
                }
                false => None,
            }
        }
        (Pattern::Ctor { ty, fields, whole }, Value::Record { ty: vty, fields: vfields })
            if ty.as_str() == &**vty && fields.len() == vfields.borrow().len() =>
        {
            for (fp, fv) in fields.iter().zip(vfields.borrow().iter()) {
                match_one(fp, fv, binds)?;
            }
            bind_whole(whole, arg, binds);
            Some(0)
        }
        // A pattern naming a type takes a subtype of it, the same way an
        // annotation does, and ranks the same way: how many wrappers come off
        // before the named type is the one in hand, zero when it names the
        // subtype itself. So an arm naming the child still beats one naming
        // the parent, and the fields bound are the base record's either way.
        (Pattern::Ctor { ty, fields, whole }, Value::Sub { .. }) => {
            let mut depth: u8 = 0;
            let mut cur = arg;
            let names_it = loop {
                match cur {
                    Value::Sub { ty: vty, inner } => match ty.as_str() == &**vty {
                        true => break true,
                        false => {
                            depth = depth.saturating_add(1);
                            cur = inner;
                        }
                    },
                    Value::Record { ty: vty, .. } => break ty.as_str() == &**vty,
                    _ => break false,
                }
            };
            if !names_it {
                return None;
            }
            let base = sub_base(arg.clone());
            match &base {
                Value::Record { fields: vfields, .. } if fields.len() == vfields.borrow().len() => {
                    for (fp, fv) in fields.iter().zip(vfields.borrow().iter()) {
                        match_one(fp, fv, binds)?;
                    }
                    bind_whole(whole, arg, binds);
                    Some(depth)
                }
                _ => None,
            }
        }
        _ => None,
    }
}
