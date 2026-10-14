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
//! A body with several effect lines runs them as one group, and each side of
//! the group that begins with a sleep is read the same way; the runtime runs
//! the pool at the group's first idle point, when every side is waiting.
//!
//! Each lifted call becomes a constant named `ahead~N`, which no author can
//! spell, and the call is replaced by a reference to it. A constant builds its
//! own cell on the first read, so the runtime forcing one early fills the cell
//! the continuation later reads, and an err the call answers is held in the
//! cell and answered at that later read.
//!
//! Work bound before the first park can move into it. An entry binding is
//! computed at its own statement, so a closed call bound there runs before
//! the sleep starts and leaves nothing for the wait to hide. When every
//! statement ahead of the first park is a binding, none of them performs an
//! effect, and the park is the first thing the program does that can be
//! seen. A binding whose value is closed becomes a constant named
//! `ahead~moved~N` and its reads read the constant, so a call after the park
//! whose arguments name only moved bindings is closed as well. The pool
//! forces these first, in the order they were written, and a failure among
//! them is raised at the park: nothing has been printed between the binding
//! and the park, so the transcript is the one the program as written
//! produces (the 2026-10-11 gavel on #1862).
//!
//! The pool takes only constants that run some of the program's work
//! (`admits`). A literal moved so a lifted call can name it is built where it
//! is first read.

use crate::ast::{Expr, FnDecl, Pattern, Program, Resolution, Stmt, TemplatePart, ENTRY};
use crate::hash::{Map as HashMap, Set as HashSet};
use crate::name::Name;

/// The prefix of every lifted constant. The runtime's pool is these, in the
/// order the pass made them.
pub const PREFIX: &str = "ahead~";

/// The prefix of a constant made from an entry binding moved into the first
/// park. It starts with `PREFIX`, so the pool forces it with the rest.
pub const MOVED: &str = "ahead~moved~";

/// The effect words whose arguments the effect is certain to evaluate.
const EFFECT_WORDS: &[&str] = &["print", "builtin_print", "write", "builtin_write"];

type Arities = HashMap<String, Vec<usize>>;

/// Whether `lift` could change anything, asked before the emitter copies a
/// program it was only lent.
pub fn lifts(program: &Program) -> bool {
    let Some(entry) = program.fns.iter().find(|f| f.name == ENTRY) else {
        return false;
    };
    entry.body.iter().any(|s| matches!(s, Stmt::Expr(e) if side_parks(e)))
        || first_park(&entry.body).is_some_and(|park| {
            entry.body[..park]
                .iter()
                .any(|s| matches!(s, Stmt::Bind { pattern: Pattern::Var(..), .. }))
        })
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
    let moved = move_into_park(&mut entry, &user);
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
        if !matches!(&entry[i], Stmt::Expr(e) if side_parks(e)) {
            continue;
        }
        let mut names: Vec<String> = Vec::new();
        if let Stmt::Expr(e) = &mut entry[i] {
            each_side(e, &mut |side| {
                if parks_first(side) {
                    after_park(side, &user, &mut lifted, &mut names);
                }
            });
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
    let named = moved.into_iter().enumerate().map(|(n, e)| (format!("{MOVED}{n}"), e));
    let named =
        named.chain(lifted.into_iter().enumerate().map(|(n, e)| (format!("{PREFIX}{n}"), e)));
    for (name, call) in named {
        program.fns.push(FnDecl {
            name: name.clone(),
            is_pub: false,
            written_pub: false,
            span: call.span(),
            params: Vec::new(),
            body: vec![Stmt::Expr(call)],
            file: file.clone(),
            synthetic: false,
            shades: None,
        });
        made.push(name);
    }
    made
}

/// The entry statement where the pool runs, when every statement before it
/// is a binding. A binding performs no effect, so the first statement that
/// does is the first thing the program does that can be seen; when it begins
/// with a sleep, the pool runs before anything is printed.
fn first_park(body: &[Stmt]) -> Option<usize> {
    let at = body.iter().position(|s| matches!(s, Stmt::Expr(_)))?;
    match &body[at] {
        Stmt::Expr(e) if is_sleep(e) || parks_first(e) => Some(at),
        _ => None,
    }
}

/// Takes each binding ahead of the first park whose value is closed,
/// and answers the calls in the order they were written. The reads of each
/// name in the statements after it are pointed at the constant the call
/// becomes. A name bound twice anywhere in the entry stays where it is, so a
/// read never has to be told which binding it meant.
fn move_into_park(entry: &mut Vec<Stmt>, user: &Arities) -> Vec<Expr> {
    let Some(mut park) = first_park(entry) else {
        return Vec::new();
    };
    let mut moved = Vec::new();
    let mut at = 0;
    while at < park {
        let name = match &entry[at] {
            Stmt::Bind { pattern: Pattern::Var(n, _), expr }
                if closed(expr, user) && binders(entry, n.as_str()) == 1 =>
            {
                n.to_string()
            }
            _ => {
                at += 1;
                continue;
            }
        };
        let Stmt::Bind { expr, .. } = entry.remove(at) else {
            unreachable!("the statement matched as a binding")
        };
        park -= 1;
        let constant = Name::new(&format!("{MOVED}{}", moved.len()));
        for s in entry[at..].iter_mut() {
            match s {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => reads_to(expr, &name, &constant),
            }
        }
        moved.push(expr);
    }
    moved
}

/// How many times the entry binds `name`: its statements, the statements of
/// its blocks and guards, and its lambdas' parameters.
fn binders(body: &[Stmt], name: &str) -> usize {
    fn in_stmts(stmts: &[Stmt], name: &str, n: &mut usize) {
        for s in stmts {
            match s {
                Stmt::Bind { pattern, expr } => {
                    pattern.each_name(&mut |b| *n += (b == name) as usize);
                    in_expr(expr, name, n);
                }
                Stmt::Expr(expr) => in_expr(expr, name, n),
            }
        }
    }
    fn in_expr(e: &Expr, name: &str, n: &mut usize) {
        match e {
            Expr::Lambda { params, body, .. } => {
                *n += params.iter().filter(|(p, _)| p == name).count();
                in_expr(body, name, n);
            }
            Expr::Block(stmts, _) => in_stmts(stmts, name, n),
            Expr::Guard { cond, early, rest, .. } => {
                in_expr(cond, name, n);
                in_expr(early, name, n);
                in_stmts(rest, name, n);
            }
            _ => crate::for_each_child(e, |c| in_expr(c, name, n)),
        }
    }
    let mut n = 0;
    in_stmts(body, name, &mut n);
    n
}

/// Points every read of `name` at `constant`.
fn reads_to(e: &mut Expr, name: &str, constant: &Name) {
    match e {
        Expr::Ident(n, span, _) if n.as_str() == name => {
            *e = Expr::Ident(constant.clone(), *span, Resolution::default());
        }
        _ => crate::for_each_child_mut(e, &mut |c| reads_to(c, name, constant)),
    }
}

/// Whether a statement, or a side of the group its effect lines make, is a
/// chain that begins with a sleep.
fn side_parks(e: &Expr) -> bool {
    match e {
        Expr::Join { lhs, rhs, .. } => side_parks(lhs) || side_parks(rhs),
        _ => parks_first(e),
    }
}

/// Each side of the group a body's effect lines make, in the order written;
/// a lone statement is its only side.
fn each_side(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    match e {
        Expr::Join { lhs, rhs, .. } => {
            each_side(lhs, f);
            each_side(rhs, f);
        }
        _ => f(e),
    }
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

/// Whether the pool takes a constant this pass made (#1856's grain). The pool
/// reads each constant inside a try whose `sigsetjmp` saves the signal mask,
/// a system call, so a constant earns a place only when it runs some of the
/// program's own work: a call, or a string or list built around one. The
/// lazy cost gate prices a cell the same way (`demand::expensive`). A literal
/// moves into a constant so the calls lifted after the park can name it, and
/// it is built where it is first read. A read of a moved binding is no work
/// either, since the pool forced that binding first.
pub fn admits(decl: &FnDecl) -> bool {
    fn works(e: &Expr) -> bool {
        match e {
            Expr::App { .. } => true,
            Expr::Str(parts, _) => parts.iter().any(|p| match p {
                TemplatePart::Lit(_) => false,
                TemplatePart::Interp(inner) => works(inner),
            }),
            Expr::List(items, _) => items.iter().any(works),
            _ => false,
        }
    }
    matches!(decl.body.as_slice(), [Stmt::Expr(e)] if works(e))
}

/// Whether an expression names nothing local: literals, a binding already
/// moved into the park, and calls of the program's own functions on closed
/// arguments.
fn closed(e: &Expr, user: &Arities) -> bool {
    match e {
        Expr::Int(..) | Expr::Float(..) => true,
        Expr::Ident(n, _, _) => n.as_str().starts_with(MOVED),
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

/// The prefix of a binding `pool` makes in a function's body. Each holds a
/// call the function's chain is certain to make after its sleep, as a cell.
pub const POOLED: &str = "ahead~pool~";

/// The sleep that carries a chain's pooled cells to the runtime, which forces
/// them while the fiber is parked.
pub const SLEEP_POOLED: &str = "ahead~sleep";

/// How many locals a pooled call may name: a cell holds that many captures.
const CAPTURES: usize = 8;

/// Whether `pool` could change anything.
pub fn pools(program: &Program) -> bool {
    program.fns.iter().any(|f| poolable(program, f))
}

/// A function whose result is a chain that begins with a sleep, and the only
/// arm of its name and count, since the emitter asks whether a binding is a
/// cell by its position and a sibling arm's binding there is another one.
fn poolable(program: &Program, f: &FnDecl) -> bool {
    f.name != ENTRY
        && !f.name.starts_with(PREFIX)
        && matches!(f.body.last(), Some(Stmt::Expr(e)) if side_parks(e))
        && program
            .fns
            .iter()
            .filter(|g| g.name == f.name && g.params.len() == f.params.len())
            .count()
            == 1
}

/// The work a function's own chain is certain to do after its sleep, when it
/// reads the function's locals. The value a call computes depends on those
/// locals, so it cannot be a constant: it is bound ahead of the chain as a
/// cell that captures them, and the chain's sleep hands the cell to the
/// runtime. The cell is built when the function runs and forced when the
/// sleep parks; a chain that is never run parks nowhere, so its work never
/// runs early. Answers how many cells it made.
pub fn pool(program: &mut Program) -> usize {
    let mut user: Arities = HashMap::default();
    for f in &program.fns {
        if f.name != ENTRY {
            user.entry(f.name.clone()).or_default().push(f.params.len());
        }
    }
    let chosen: Vec<usize> =
        (0..program.fns.len()).filter(|&i| poolable(program, &program.fns[i])).collect();
    let mut made = 0;
    for i in chosen {
        let f = &mut program.fns[i];
        let mut locals: HashSet<String> = HashSet::default();
        for p in &f.params {
            p.each_name(&mut |n| {
                locals.insert(n.to_string());
            });
        }
        let tail = f.body.len() - 1;
        for s in &f.body[..tail] {
            if let Stmt::Bind { pattern, .. } = s {
                pattern.each_name(&mut |n| {
                    locals.insert(n.to_string());
                });
            }
        }
        let mut binds: Vec<Stmt> = Vec::new();
        let Some(Stmt::Expr(e)) = f.body.last_mut() else { continue };
        each_side(e, &mut |side| {
            if !parks_first(side) {
                return;
            }
            // A continuation's parameter is bound after the park; a call that
            // reads one, or a local it shadows, waits for the park.
            let mut later: HashSet<String> = HashSet::default();
            lambda_params(side, &mut later);
            let mut cells: Vec<Name> = Vec::new();
            let mut at = &mut *side;
            while let Expr::App { piped: true, head, args, .. } = at {
                if let Expr::Lambda { body, .. } = head.as_mut() {
                    certain_local(body, &user, &locals, &later, &mut cells, &mut binds, &mut made);
                }
                at = &mut args[0];
            }
            if cells.is_empty() {
                return;
            }
            let Expr::App { head, args, .. } = at else { unreachable!("the chain parks first") };
            let span = head.span();
            **head = Expr::Ident(Name::new(SLEEP_POOLED), span, Resolution::default());
            args.extend(cells.into_iter().map(|c| Expr::Ident(c, span, Resolution::default())));
        });
        program.fns[i].body.splice(tail..tail, binds);
    }
    made
}

/// The parameters of every lambda in a chain.
fn lambda_params(e: &Expr, out: &mut HashSet<String>) {
    if let Expr::Lambda { params, .. } = e {
        out.extend(params.iter().map(|(p, _)| p.clone()));
    }
    crate::for_each_child(e, |c| lambda_params(c, out));
}

/// `certain` for a function's chain: the effect arguments it is certain to
/// evaluate, where a call of the program's own on arguments that read only
/// the function's locals becomes a pooled cell.
fn certain_local(
    e: &mut Expr,
    user: &Arities,
    locals: &HashSet<String>,
    later: &HashSet<String>,
    cells: &mut Vec<Name>,
    binds: &mut Vec<Stmt>,
    made: &mut usize,
) {
    let Expr::App { piped: false, head, args, .. } = e else { return };
    if !matches!(head.as_ref(), Expr::Ident(n, _, _) if EFFECT_WORDS.contains(&n.as_str())) {
        return;
    }
    for a in args.iter_mut() {
        take_local(a, user, locals, later, cells, binds, made);
    }
}

fn take_local(
    e: &mut Expr,
    user: &Arities,
    locals: &HashSet<String>,
    later: &HashSet<String>,
    cells: &mut Vec<Name>,
    binds: &mut Vec<Stmt>,
    made: &mut usize,
) {
    match e {
        Expr::Str(parts, _) => {
            for p in parts.iter_mut() {
                if let TemplatePart::Interp(inner) = p {
                    take_local(inner, user, locals, later, cells, binds, made);
                }
            }
        }
        Expr::App { piped: false, head, args, .. }
            if is_user_call(head, args.len(), user)
                && args.iter().all(|a| reads_locals(a, user, locals, later)) =>
        {
            let mut read: HashSet<String> = HashSet::default();
            for a in args.iter() {
                names_read(a, locals, &mut read);
            }
            if read.len() > CAPTURES {
                return;
            }
            let span = e.span();
            let name = Name::new(&format!("{POOLED}{made}"));
            *made += 1;
            let call = std::mem::replace(e, Expr::Ident(name.clone(), span, Resolution::default()));
            binds.push(Stmt::Bind { pattern: Pattern::Var(name.clone(), span), expr: call });
            cells.push(name);
        }
        _ => {}
    }
}

/// Whether an expression reads nothing but literals, the function's locals
/// and calls of the program's own functions on such arguments.
fn reads_locals(
    e: &Expr,
    user: &Arities,
    locals: &HashSet<String>,
    later: &HashSet<String>,
) -> bool {
    match e {
        Expr::Int(..) | Expr::Float(..) => true,
        Expr::Ident(n, _, _) => {
            n.as_str().starts_with(MOVED)
                || (locals.contains(n.as_str()) && !later.contains(n.as_str()))
        }
        Expr::Str(parts, _) => parts.iter().all(|p| match p {
            TemplatePart::Lit(_) => true,
            TemplatePart::Interp(inner) => reads_locals(inner, user, locals, later),
        }),
        Expr::List(items, _) => items.iter().all(|i| reads_locals(i, user, locals, later)),
        Expr::BinOp { lhs, rhs, .. } => {
            reads_locals(lhs, user, locals, later) && reads_locals(rhs, user, locals, later)
        }
        Expr::App { piped: false, head, args, .. } => {
            (is_user_call(head, args.len(), user) || reads_a_value(head, args.len()))
                && args.iter().all(|a| reads_locals(a, user, locals, later))
        }
        _ => false,
    }
}

/// Builtins that read a value and answer a new one, which a pooled call may
/// apply to its arguments: `report (length line)` is pooled like `report n`.
/// The words that extend a value (`push`, `put`, `concat` and the rest) are
/// left out, since the beat pass may write their result into the storage of
/// the value they were given, and a pooled cell runs before the program's
/// own reads of it.
const VALUE_READS: &[&str] = &[
    "length", "keys", "values", "entries", "chars", "bytes", "char_code", "from_code", "to_int",
    "to_float", "round", "sqrt", "sum", "bit_and", "bit_or", "bit_xor", "bit_not", "bit_shl",
    "bit_shr",
];

fn reads_a_value(head: &Expr, arity: usize) -> bool {
    let Expr::Ident(n, _, _) = head else { return false };
    let word = n.as_str().strip_prefix("builtin_").unwrap_or(n.as_str());
    VALUE_READS.contains(&word)
        && crate::check::BUILTIN_ARITY.iter().any(|&(b, a)| b == word && a == arity)
}

fn names_read(e: &Expr, locals: &HashSet<String>, out: &mut HashSet<String>) {
    if let Expr::Ident(n, _, _) = e {
        if locals.contains(n.as_str()) {
            out.insert(n.to_string());
        }
    }
    crate::for_each_child(e, |c| names_read(c, locals, out));
}
