//! Work-ahead discovery (#1856, ruled 2026-07-22 and 2026-10-11).
//!
//! When a fiber parks on a sleep, the program as written stops computing until
//! the sleep ends, and work certain to follow the sleep runs after it. This
//! pass finds that work and lifts it into constants, so the runtime can force
//! them while the fiber is parked and the wait hides the work.
//!
//! What it lifts is narrow on purpose. An entry statement whose chain begins
//! with a `sleep` has continuations that run, in order, after the sleep. In
//! those continuations, a call to one of the program's own functions whose
//! arguments name nothing local is closed: it has the same value wherever it
//! runs, and since kanso's functions have no effects, running it early cannot
//! be seen. Only calls the continuation is certain to evaluate are taken: an
//! argument of an effect word, or a piece of a string such an argument
//! interpolates. A continuation that names an entry binding is followed into
//! it, so a chain kept under a name is read the same as one written inline.
//!
//! Each lifted call becomes a constant named `ahead~N`, which no author can
//! spell, and the call is replaced by a reference to it. A constant builds its
//! own cell on the first read, so the runtime forcing one early fills the cell
//! the continuation later reads, and an err the call answers is held in the
//! cell and answered at that later read.

use crate::ast::{Expr, FnDecl, Pattern, Program, Resolution, Stmt, TemplatePart, ENTRY};
use crate::hash::{Map as HashMap, Set as HashSet};
use crate::name::Name;

/// The prefix of every lifted constant. The runtime's pool is these, in the
/// order the pass made them.
pub const PREFIX: &str = "ahead~";

/// The effect words whose arguments the effect is certain to evaluate.
const EFFECT_WORDS: &[&str] = &["print", "builtin_print", "write", "builtin_write"];

type Arities = HashMap<String, Vec<usize>>;

/// Whether `lift` could change anything, asked before the emitter copies a
/// program it was only lent.
pub fn lifts(program: &Program) -> bool {
    let Some(entry) = program.fns.iter().find(|f| f.name == ENTRY) else {
        return false;
    };
    entry.body.iter().any(|s| matches!(s, Stmt::Expr(e) if parks_first(e)))
}

/// Lifts the closed calls that follow each entry statement's first park, and
/// answers the constants it made, in order.
pub fn lift(program: &mut Program) -> Vec<String> {
    let mut user: Arities = HashMap::default();
    for f in &program.fns {
        if f.name != ENTRY {
            user.entry(f.name.clone()).or_default().push(f.params.len());
        }
    }
    let Some(at) = program.fns.iter().position(|f| f.name == ENTRY) else {
        return Vec::new();
    };
    let mut entry = std::mem::take(&mut program.fns[at].body);
    let binds: HashMap<String, usize> = entry
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s {
            Stmt::Bind { pattern: Pattern::Var(n, _), .. } => Some((n.to_string(), i)),
            _ => None,
        })
        .collect();
    let mut lifted: Vec<Expr> = Vec::new();
    let mut followed: HashSet<usize> = HashSet::default();
    for i in 0..entry.len() {
        if !matches!(&entry[i], Stmt::Expr(e) if parks_first(e)) {
            continue;
        }
        let mut names: Vec<String> = Vec::new();
        if let Stmt::Expr(e) = &mut entry[i] {
            after_park(e, &user, &mut lifted, &mut names);
        }
        // A continuation that names an entry binding runs that binding's
        // chain, so the binding's certain calls follow the park as well.
        while let Some(name) = names.pop() {
            let Some(&j) = binds.get(&name) else { continue };
            if j >= i || !followed.insert(j) {
                continue;
            }
            if let Stmt::Bind { expr, .. } = &mut entry[j] {
                whole_chain(expr, &user, &mut lifted, &mut names);
            }
        }
    }
    program.fns[at].body = entry;
    let file = program.fns[at].file.clone();
    let mut made = Vec::new();
    for (n, call) in lifted.into_iter().enumerate() {
        let name = format!("{PREFIX}{n}");
        program.fns.push(FnDecl {
            name: name.clone(),
            is_pub: false,
            span: call.span(),
            params: Vec::new(),
            body: vec![Stmt::Expr(call)],
            file: file.clone(),
            synthetic: false,
        });
        made.push(name);
    }
    made
}

/// Whether a chain `a .> f .> g` begins with a sleep.
fn parks_first(e: &Expr) -> bool {
    let mut at = e;
    let mut steps = 0;
    while let Expr::App { piped: true, head, args, .. } = at {
        if args.len() != 1 || !matches!(head.as_ref(), Expr::Lambda { .. }) {
            return false;
        }
        steps += 1;
        at = &args[0];
    }
    steps > 0 && is_sleep(at)
}

fn is_sleep(e: &Expr) -> bool {
    matches!(e, Expr::App { piped: false, head, .. }
        if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == "builtin_sleep" || n == "sleep"))
}

/// Every continuation of a statement whose first step parks.
fn after_park(e: &mut Expr, user: &Arities, lifted: &mut Vec<Expr>, names: &mut Vec<String>) {
    let mut at = e;
    while let Expr::App { piped: true, head, args, .. } = at {
        if args.len() != 1 {
            return;
        }
        if let Expr::Lambda { body, .. } = head.as_mut() {
            certain(body, user, lifted, names);
        }
        at = &mut args[0];
    }
}

/// A bound chain a continuation runs: its first step and every continuation
/// run, so all of them are certain.
fn whole_chain(e: &mut Expr, user: &Arities, lifted: &mut Vec<Expr>, names: &mut Vec<String>) {
    let piped = matches!(e, Expr::App { piped: true, args, .. } if args.len() == 1);
    if !piped {
        certain(e, user, lifted, names);
        return;
    }
    if let Expr::App { head, args, .. } = e {
        if let Expr::Lambda { body, .. } = head.as_mut() {
            certain(body, user, lifted, names);
        }
        whole_chain(&mut args[0], user, lifted, names);
    }
}

/// The calls a continuation body is certain to evaluate.
fn certain(e: &mut Expr, user: &Arities, lifted: &mut Vec<Expr>, names: &mut Vec<String>) {
    match e {
        Expr::Ident(n, _, _) => names.push(n.to_string()),
        Expr::App { piped: true, .. } => whole_chain(e, user, lifted, names),
        Expr::App { piped: false, head, args, .. } => {
            let effect =
                matches!(head.as_ref(), Expr::Ident(n, _, _) if EFFECT_WORDS.contains(&n.as_str()));
            if effect {
                for a in args.iter_mut() {
                    take(a, user, lifted);
                }
            }
        }
        _ => {}
    }
}

/// An effect's argument: a closed call is lifted, and each piece a string
/// interpolates is looked at the same way.
fn take(e: &mut Expr, user: &Arities, lifted: &mut Vec<Expr>) {
    match e {
        Expr::Str(parts, _) => {
            for p in parts.iter_mut() {
                if let TemplatePart::Interp(inner) = p {
                    take(inner, user, lifted);
                }
            }
        }
        Expr::App { piped: false, head, args, .. }
            if is_user_call(head, args.len(), user) && args.iter().all(|a| closed(a, user)) =>
        {
            let span = e.span();
            let name = format!("{PREFIX}{}", lifted.len());
            let call =
                std::mem::replace(e, Expr::Ident(Name::new(&name), span, Resolution::default()));
            lifted.push(call);
        }
        _ => {}
    }
}

fn is_user_call(head: &Expr, arity: usize, user: &Arities) -> bool {
    matches!(head, Expr::Ident(n, _, _) if user.get(n.as_str()).is_some_and(|a| a.contains(&arity)))
}

/// Whether an expression names nothing local: literals, and calls of the
/// program's own functions on closed arguments.
fn closed(e: &Expr, user: &Arities) -> bool {
    match e {
        Expr::Int(..) | Expr::Float(..) => true,
        Expr::Str(parts, _) => parts.iter().all(|p| match p {
            TemplatePart::Lit(_) => true,
            TemplatePart::Interp(inner) => closed(inner, user),
        }),
        Expr::List(items, _) => items.iter().all(|i| closed(i, user)),
        Expr::App { piped: false, head, args, .. } => {
            is_user_call(head, args.len(), user) && args.iter().all(|a| closed(a, user))
        }
        _ => false,
    }
}
