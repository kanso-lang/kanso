// The environment a body runs in: frames of bindings, the place a reference
// recorded for its binder, and the lookups by place and by name.
// eval.rs reads this file with include!, so the items stay in the eval
// module, as the other files under src/eval/ do (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under src/'s
// subdirectories itself.

/// The name is a `Name` rather than a `String`, and that is the whole of the
/// second half of this change. `Name` is twenty-four bytes — exactly what a
/// `String` costs, so no node grew — and it keeps a name of twenty-two bytes
/// or fewer in the node itself. 99.77% of identifiers across `lib/` are that
/// short, so binding one now copies twenty-four bytes where it used to reach
/// the allocator. The heap variant is still there for the thirty names that
/// run longer, and they are nearly all test function names.
#[derive(Debug)]
/// A scope, as a chain of frames.
///
/// `One` is a single name bound on its own — a lazy thunk, a pattern variable,
/// a `build` step. `Many` is a whole call's parameters in one frame, because
/// dispatch knows all of them at once and pushing a node each made the chain
/// as long as the argument list. A four-parameter call pushed four nodes and
/// the walk crossed all four to reach the caller's scope.
///
/// `Many` scans its slots in REVERSE, which is what keeps shadowing the same:
/// binding a, b, c as three `One` frames leaves c outermost, and a single
/// frame holding [a, b, c] has to answer c first to agree with that.
pub enum Env {
    One(Name, Value, Option<Rc<Env>>),
    Many(Bindings, Option<Rc<Env>>),
}

/// A binding takes its name BY VALUE. Every caller that reaches here with a
/// freshly built `String` — `match_one` pushes one per matched name, and both
/// loops below drain that vector — hands the same allocation on rather than
/// paying for a second copy of the same bytes and dropping the first. The
/// callers holding an AST name clone it here, which is what they did before.
fn bind(env: Option<Rc<Env>>, name: Name, value: Value) -> Option<Rc<Env>> {
    Some(Rc::new(Env::One(name, value, env)))
}

/// Bind a whole call's parameters in one frame.
///
/// The vector is the one arm selection already filled and the winner handed
/// on, so the frame the body runs in costs no allocation of its own. Nothing
/// is pushed for a call that binds nothing, so a chain never carries an empty
/// frame for the walk to step over.
fn bind_all(env: Option<Rc<Env>>, binds: Bindings) -> Option<Rc<Env>> {
    match binds.is_empty() {
        true => env,
        false => Some(Rc::new(Env::Many(binds, env))),
    }
}

/// Each interpreter's generation, for the stamps it leaves on the AST. Sixteen
/// bits, and 0 once they run out, which switches stamping off for that
/// interpreter rather than letting a stamp come round again.
fn next_generation() -> u16 {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let g = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match g {
        1..=0xffff => g as u16,
        _ => 0,
    }
}

/// Where a local was found, as the `Resolution` a node keeps for it: the
/// number of frames above the innermost and the slot inside that frame, packed
/// into the range between plain local and the global stamps. A frame deeper
/// than 127 or a slot past 255 keeps plain local, which walks.
fn local_at(depth: u32, slot: usize) -> u32 {
    match (depth, slot) {
        (d, s) if d < 128 && s < 256 => 3 + (d << 8) + s as u32,
        _ => crate::ast::Resolution::LOCAL,
    }
}

/// The walk `lookup` makes, answering where the binding sat as well.
fn lookup_placed(env: &Option<Rc<Env>>, name: &Name) -> Option<(Value, u32)> {
    let mut cur = env.as_ref();
    let mut depth = 0;
    while let Some(frame) = cur {
        match &**frame {
            Env::One(bound, value, parent) => {
                if bound == name {
                    return Some((value.clone(), local_at(depth, 0)));
                }
                cur = parent.as_ref();
            }
            Env::Many(slots, parent) => {
                for (i, (bound, value)) in slots.iter().enumerate().rev() {
                    if bound == name {
                        return Some((value.clone(), local_at(depth, i)));
                    }
                }
                cur = parent.as_ref();
            }
        }
        depth += 1;
    }
    None
}

/// Drops `value`, and for the variants that own nothing skips the call into
/// `Value`'s drop that would only have found that out. A dispatch lets go of
/// a few values every time it runs, and most of them are words.
#[inline(always)]
fn release(value: Value) {
    match value {
        Value::Int(Int::Small(_))
        | Value::Float(_)
        | Value::True
        | Value::False
        | Value::NoneV
        | Value::Done
        | Value::TableFn(_) => std::mem::forget(value),
        value => drop(value),
    }
}

/// An integer literal's value, out of line so the front door that calls it
/// stays small enough to inline.
#[inline(never)]
fn int_literal(n: &BigInt) -> Value {
    Value::int(n)
}

/// The binding at a place `local_at` recorded, if the name is still there. The
/// frames between the reference and its binder are fixed by where the
/// reference sits, so the place found once is the place every later run finds;
/// the name is compared anyway, and a miss walks.
fn lookup_at(env: &Option<Rc<Env>>, name: &Name, place: u32) -> Option<Value> {
    let place = place - 3;
    let mut frame = env.as_ref()?;
    for _ in 0..place >> 8 {
        frame = match &**frame {
            Env::One(_, _, parent) | Env::Many(_, parent) => parent.as_ref()?,
        };
    }
    match &**frame {
        Env::One(bound, value, _) if place & 0xff == 0 && bound == name => Some(value.clone()),
        Env::Many(slots, _) => match slots.get((place & 0xff) as usize) {
            Some((bound, value)) if bound == name => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn lookup(env: &Option<Rc<Env>>, name: &Name) -> Option<Value> {
    let mut cur = env.as_ref();
    while let Some(frame) = cur {
        match &**frame {
            Env::One(bound, value, parent) => {
                if bound == name {
                    return Some(value.clone());
                }
                cur = parent.as_ref();
            }
            Env::Many(slots, parent) => {
                for (bound, value) in slots.iter().rev() {
                    if bound == name {
                        return Some(value.clone());
                    }
                }
                cur = parent.as_ref();
            }
        }
    }
    None
}
