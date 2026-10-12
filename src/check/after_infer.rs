// The questions inference makes answerable, asked on one descent: the tables
// they read, the three checks, and the walk that asks them. check.rs reads
// this file with include!, so the items stay in the check module and the
// compiler builds to the same instructions it did when they sat in check.rs
// (kanso#1867). cargo fmt does not reach an included file, so CI's format job
// checks the files under src/'s subdirectories itself.

/// The tables the two questions below read. Each check built its own and then
/// walked the whole program for itself; they are asked on one descent now, so
/// the tables travel together.
struct AfterInfer<'a, 'r> {
    /// `check_effect_discarded`: a position every arm throws away.
    discarded: crate::hash::Map<(&'a str, usize, usize), bool>,
    /// `check_none_exhaustive`: a group's joined return set beside the
    /// positions some arm names a none at, one bit each. Two facts about the
    /// same group live in one entry so a call site pays ONE hash of the name:
    /// a separate (name, arity, position) map cost a string hash per PARAMETER
    /// to build and another per ARGUMENT to read, and both keys start with the
    /// name. A position past the mask's width reads as handled, the
    /// under-refusing direction, and nothing in the tree is close: the widest
    /// group in lib/ takes five.
    nones: crate::hash::Map<(&'a str, usize), (crate::infer::Set, u64)>,
    /// What a dispatch group answers, built once by the caller and handed
    /// round. A fourth build used to sit in `check_none_exhaustive` and still
    /// does, as `nones`: its entry carries a SECOND fact beside the set, so
    /// reading this table instead would leave it hashing the name a second
    /// time for the mask, at every call site.
    returns: &'r HashMap<(&'a str, usize), crate::infer::Set>,
    /// The plain index reads inference proved in range, by span: a read
    /// its `if`s bound cannot miss, so it is not a none the caller owes an
    /// arm for.
    proven: &'r crate::hash::Set<crate::diag::Span>,
    /// The calls inference typed by their own arguments and found cannot
    /// answer `none`, by the called name's span: `math/round 7`.
    none_free: &'r crate::hash::Set<crate::diag::Span>,
}

/// An effect handed to a position every arm throws away never happens.
fn effect_discarded_at(e: &Expr, tables: &AfterInfer, diags: &mut Vec<Diagnostic>) {
    use crate::infer::DESC;
    let Expr::App { head, args, piped: false, .. } = e else { return };
    let Expr::Ident(name, _, _) = head.as_ref() else { return };
    if !tables.returns.contains_key(&(name.as_str(), args.len())) {
        return;
    }
    let describes = |e: &Expr| -> bool {
        let Expr::App { head, args, piped: false, .. } = e else { return false };
        let Expr::Ident(name, _, _) = head.as_ref() else { return false };
        tables.returns.get(&(name.as_str(), args.len())).is_some_and(|s| s & DESC != 0)
    };
    for (pos, arg) in args.iter().enumerate() {
        if !describes(arg) {
            continue;
        }
        if !tables.discarded.get(&(name.as_str(), args.len(), pos)).copied().unwrap_or(false) {
            continue;
        }
        diags.push(Diagnostic::new(
            "effect",
            format!(
                "`{name}` reads none of what it is handed here, so this effect \
                 never happens — hand it back, or give `{name}` a name for it"
            ),
            arg.span(),
        ));
    }
}

/// Whether `e` can answer a bare err the program raised: a call to `err`, a
/// call some raising arm of can be the one that runs, or a name bound to
/// one. A call raises when some arm that can raise can be the one that runs.
/// The arms are read one at a time rather than as the group's joined answer
/// because a literal argument settles a literal pattern here and now:
/// `text/split s "\n"` cannot reach `split _ ""`, the only arm of that group
/// that raises, so the call is not a raise. What a literal cannot match is
/// decided by shape alone — a string is never a record and never a none — and
/// every other pattern is taken to match.
///
/// A name carries its call (#1884): `h = half 3` and then `h + 1` is refused
/// as `half 3 + 1` is, so binding a value never changes what compiles. A
/// local is one `raising_names` found, and a use counts only after its
/// binding, which keeps a same-spelled outer name read earlier out of it. A
/// name the declaration does not bind is a constant, which raises when its
/// own body does, the way a call of it would.
fn raises(
    e: &Expr,
    raisers: &crate::hash::Map<(&str, usize), Vec<&[Pattern]>>,
    consts: &crate::infer::Consts<'_>,
    named: &BoundRaises<'_>,
) -> bool {
    match e {
        Expr::App { head, args, piped: false, .. } => match head.as_ref() {
            Expr::Ident(name, _, _) if name == "err" => true,
            Expr::Ident(name, _, _) => raisers
                .get(&(name.as_str(), args.len()))
                .is_some_and(|arms| arms.iter().any(|arm| crate::infer::arm_can_run(arm, args, consts))),
            _ => false,
        },
        Expr::Ident(name, at, _) if named.local.contains(name.as_str()) => named
            .raising
            .iter()
            .any(|(n, bound)| *n == name.as_str() && (bound.line, bound.col) < (at.line, at.col)),
        Expr::Ident(name, _, _) => raisers.contains_key(&(name.as_str(), 0)),
        _ => false,
    }
}

/// What `raises` needs to know about one declaration's own names: the ones
/// bound once to a raise, with where, and every name it binds at all.
#[derive(Default)]
struct BoundRaises<'a> {
    raising: Vec<(&'a str, crate::diag::Span)>,
    local: crate::hash::Set<&'a str>,
}

/// The names in `decl` bound to a call that can raise, with where each is
/// bound. A name the declaration binds more than once — a parameter, a
/// second binding, a lambda's parameter — is left out, because the check
/// cannot tell which binding a use reads and under-refuses rather than guess.
/// Bindings are read in order, so a name bound to a name bound to a raise
/// carries it too.
fn raising_names<'a>(
    decl: &'a crate::ast::FnDecl,
    raisers: &crate::hash::Map<(&str, usize), Vec<&[Pattern]>>,
    consts: &crate::infer::Consts<'_>,
) -> BoundRaises<'a> {
    fn binds_in_stmts<'a>(body: &'a [Stmt], out: &mut Vec<(&'a str, Option<(crate::diag::Span, &'a Expr)>)>) {
        for stmt in body {
            match stmt {
                Stmt::Bind { pattern: Pattern::Var(n, at), expr } => {
                    out.push((n.as_str(), Some((*at, expr))));
                    binds_in_expr(expr, out);
                }
                Stmt::Bind { pattern, expr } => {
                    for_each_param_name(pattern, &mut |n| out.push((n, None)));
                    binds_in_expr(expr, out);
                }
                Stmt::Expr(expr) => binds_in_expr(expr, out),
            }
        }
    }
    fn binds_in_expr<'a>(e: &'a Expr, out: &mut Vec<(&'a str, Option<(crate::diag::Span, &'a Expr)>)>) {
        if let Expr::Lambda { params, .. } = e {
            out.extend(params.iter().map(|(n, _)| (n.as_str(), None)));
        }
        if let Expr::Guard { rest, .. } = e {
            binds_in_stmts(rest, out);
        }
        crate::for_each_child(e, |c| binds_in_expr(c, out));
    }
    let mut binds = Vec::new();
    for param in &decl.params {
        for_each_param_name(param, &mut |n| binds.push((n, None)));
    }
    binds_in_stmts(&decl.body, &mut binds);
    let mut named = BoundRaises { raising: Vec::new(), local: binds.iter().map(|(n, _)| *n).collect() };
    for (i, (name, at)) in binds.iter().enumerate() {
        let Some((at, expr)) = at else { continue };
        if binds.iter().enumerate().any(|(j, (other, _))| j != i && other == name) {
            continue;
        }
        if raises(expr, raisers, consts, &named) {
            named.raising.push((name, *at));
        }
    }
    named
}

/// The 2026-09-15 ruling's third part: a bare err a program raised, arriving
/// where a value is wanted, does not compile. The sites are the ruling's
/// three — an operator's operand, an index's base or key, and a call at a
/// position no arm names an err at — and the proof is `RAISED`, the bit
/// infer sets only where a program spells an err. A strict index's miss and
/// a division's zero divisor answer ERR without it, because what the checker
/// should make of those is the ledger's open question and not this rule's.
/// A description is skipped: a boxed failure is not a bare one, and the
/// three words are its doors. An answer that can be any value is not known
/// to be a description, so an arm that spells an err beside it still counts
/// (ruled 2026-09-30).
fn raised_err_at(
    e: &Expr,
    raisers: &crate::hash::Map<(&str, usize), Vec<&[Pattern]>>,
    consts: &crate::infer::Consts<'_>,
    err_arms: &crate::hash::Map<(&str, usize), u64>,
    named: &BoundRaises<'_>,
    owner: &str,
    diags: &mut Vec<Diagnostic>,
) {
    let raised = |e: &Expr| raises(e, raisers, consts, named);
    let short = |owner: &str| -> String {
        owner
            .rsplit_once("/lib/")
            .map(|(_, m)| m)
            .or_else(|| owner.rsplit_once('/').map(|(_, f)| f))
            .unwrap_or(owner)
            .to_string()
    };
    match e {
        Expr::BinOp { op, lhs, rhs, .. } => {
            for side in [lhs.as_ref(), rhs.as_ref()] {
                if raised(side) {
                    diags.push(Diagnostic::new(
                        "exhaustive",
                        format!(
                            "this can be an err and `{op}` wants a value — dispatch on it \
                             first with an `(err _)` arm (in {})",
                            short(owner)
                        ),
                        side.span(),
                    ));
                }
            }
        }
        Expr::Index { base, index, .. } => {
            for side in [base.as_ref(), index.as_ref()] {
                if raised(side) {
                    diags.push(Diagnostic::new(
                        "exhaustive",
                        format!(
                            "this can be an err and an index wants a value — dispatch on \
                             it first with an `(err _)` arm (in {})",
                            short(owner)
                        ),
                        side.span(),
                    ));
                }
            }
        }
        Expr::App { head, args, piped: false, .. } => {
            let Expr::Ident(name, _, _) = head.as_ref() else { return };
            // A getter is synthesized from a field read, so nobody can give it
            // an arm, and the two routes check on opposite sides of the
            // rewrite that makes one: refusing here would refuse `xs[i].x`
            // through an import and run it direct. A field read of an err
            // stays the runtime's sentence on every route, as a none's does.
            if crate::ast::getter_field(name).is_some() {
                return;
            }
            if !args.iter().any(&raised) {
                return;
            }
            let Some(&named) = err_arms.get(&(name.as_str(), args.len())) else { return };
            for (pos, arg) in args.iter().enumerate() {
                if named & (1u64 << pos.min(63)) != 0 || !raised(arg) {
                    continue;
                }
                diags.push(Diagnostic::new(
                    "exhaustive",
                    format!(
                        "this can be an err and `{name}` has no arm for it — dispatch on \
                         it here, or give `{name}` an `(err _)` arm (in {})",
                        short(owner)
                    ),
                    arg.span(),
                ));
            }
        }
        _ => {}
    }
}

/// The gavel makes a receiver responsible for a none it can be handed, and
/// lets the caller discharge that by resolving first. The report belongs at
/// the argument, because that is the line an author edits.
fn none_exhaustive_at(e: &Expr, tables: &AfterInfer, owner: &str, diags: &mut Vec<Diagnostic>) {
    use crate::infer::NONE;
    let Expr::App { head, args, piped: false, .. } = e else { return };
    let Expr::Ident(name, _, _) = head.as_ref() else { return };
    // A getter is synthesized from a field read, so nobody can give it
    // an arm, and the play route checks before the read is rewritten
    // into one while the module route checks after: `xs[i].x` would be
    // refused through an import and run direct. A field read of a none
    // stays the runtime's sentence on every route.
    if crate::ast::getter_field(name).is_some() {
        return;
    }
    // only what is provable: a lenient read, a literal none, or a call whose
    // group's return set carries one. A set holding every value bit is the
    // unknown, and the unknown carries the none bit with the rest: a field
    // read through a variable (`m.to` is TOP in infer) and a strict index
    // (`xs[i]!` is every value but a thunk) both answer it, so a group that
    // hands either back would read as proof of a none it never produces.
    // Unknown is not proof.
    let values = crate::infer::TOP & !crate::infer::FAIL & !crate::infer::THUNK;
    let unknown = |s: crate::infer::Set| s & values == values;
    let yields_none = |e: &Expr| -> bool {
        match e {
            Expr::Index { strict: false, span, .. } => !tables.proven.contains(span),
            Expr::Ident(name, _, _) => name == "none",
            Expr::App { head, args, piped: false, .. } => match head.as_ref() {
                Expr::Ident(_, span, _) if tables.none_free.contains(span) => false,
                Expr::Ident(name, _, _) => tables
                    .nones
                    .get(&(name.as_str(), args.len()))
                    .is_some_and(|&(s, _)| s & NONE != 0 && !unknown(s)),
                _ => false,
            },
            _ => false,
        }
    };
    // THE CHEAP TESTS COME FIRST, and asking the table is not one of
    // them: the lookup hashes the callee's name, where a none question
    // about an argument is a match on the argument's own shape. A call
    // whose arguments are literals, arithmetic or field reads answers no
    // on the match alone and never touches the table, which is most of
    // them. The second walk over the arguments below costs a few compares
    // on the sites that reach it, and those are the rare ones.
    if !args.iter().any(&yields_none) {
        return;
    }
    let Some(&(_, named)) = tables.nones.get(&(name.as_str(), args.len())) else { return };
    for (pos, arg) in args.iter().enumerate() {
        if !yields_none(arg) {
            continue;
        }
        if named & (1u64 << pos.min(63)) != 0 {
            continue;
        }
        // an absolute path built from the binary's location tells a
        // reader nothing; the module name is what they searched for. A
        // file outside lib/ is named by its own name: the corpus stages
        // a fixture into a temporary directory before it runs, and a
        // golden that quoted the staging path would pin that run's
        // temporary directory rather than the diagnostic.
        let short = owner
            .rsplit_once("/lib/")
            .map(|(_, m)| m)
            .or_else(|| owner.rsplit_once('/').map(|(_, f)| f))
            .unwrap_or(owner);
        diags.push(Diagnostic::new(
            "exhaustive",
            format!(
                "this can be a none and `{name}` has no arm for it — resolve it \
                 here, or give `{name}` a `none` arm (in {short})"
            ),
            arg.span(),
        ));
    }
}

/// The three questions inference makes answerable, asked on ONE descent.
///
/// Each used to walk the whole program for itself, over the same declarations,
/// the same statements and the same nodes, in the same order. The answers go
/// into four vectors rather than one because this route hands diagnostics
/// back in push order and the checks pushed from either side of the fused
/// walk's rotation; the caller splices each where its own check used to push.
fn check_after_infer<'p>(
    program: &'p Program,
    inference: &crate::infer::Inference,
    returns: &HashMap<(&str, usize), crate::infer::Set>,
    effect_diags: &mut Vec<Diagnostic>,
    box_diags: &mut Vec<Diagnostic>,
    none_diags: &mut Vec<Diagnostic>,
) {
    let mut discarded: crate::hash::Map<(&str, usize, usize), bool> =
        crate::hash::Map::with_capacity_and_hasher(program.fns.len(), Default::default());
    for d in &program.fns {
        for (pos, param) in d.params.iter().enumerate() {
            let wildcard = matches!(param, Pattern::Wildcard(_));
            let slot = discarded.entry((d.name.as_str(), d.params.len(), pos)).or_insert(true);
            *slot &= wildcard;
        }
    }

    let mut nones: crate::hash::Map<(&str, usize), (crate::infer::Set, u64)> =
        crate::hash::Map::with_capacity_and_hasher(program.fns.len(), Default::default());
    for (i, d) in program.fns.iter().enumerate() {
        // Only an arm that NAMES a none handles one. A wildcard binds a
        // none the same way a bare name does, and the ruling's own
        // fixture is a bare name: `fn describe price` runs for a none
        // and prints `<none> yen`, which is the program the check
        // exists to refuse. Counting either as an arm would leave the
        // rule with nothing to say.
        let mut named = 0u64;
        for (pos, param) in d.params.iter().enumerate() {
            let names_none = match param {
                Pattern::Nullary(n, _) => n == "none",
                Pattern::Annotated { ty, .. } => ty == "none",
                _ => false,
            };
            if names_none {
                named |= 1u64 << pos.min(63);
            }
        }
        let entry = nones.entry((d.name.as_str(), d.params.len())).or_insert((0, 0));
        entry.0 |= inference.returns[i];
        entry.1 |= named;
    }

    // The positions some arm names an err at, one bit each, keyed like
    // `nones`. An arm names an err with `(err _)`, with `x:err`, or with a
    // typeset that holds `err` — `type maybe err int` is an arm for one, and
    // a typeset naming such a typeset is too, so the set is closed first.
    let mut err_typesets: crate::hash::Set<&str> = crate::hash::Set::default();
    loop {
        let before = err_typesets.len();
        for t in program.types.iter().filter(|t| !t.members.is_empty()) {
            if t.members.iter().any(|m| m == "err" || err_typesets.contains(m.as_str())) {
                err_typesets.insert(t.name.as_str());
            }
        }
        if err_typesets.len() == before {
            break;
        }
    }
    let mut err_arms: crate::hash::Map<(&str, usize), u64> =
        crate::hash::Map::with_capacity_and_hasher(program.fns.len(), Default::default());
    for d in program.fns.iter() {
        let mut named = 0u64;
        for (pos, param) in d.params.iter().enumerate() {
            let names_err = match param {
                Pattern::Ctor { ty, .. } => ty == "err",
                Pattern::Annotated { ty, .. } => ty == "err" || err_typesets.contains(ty.as_str()),
                _ => false,
            };
            if names_err {
                named |= 1u64 << pos.min(63);
            }
        }
        *err_arms.entry((d.name.as_str(), d.params.len())).or_insert(0) |= named;
    }
    // The arms that can raise, keyed like `nones`: each one's patterns, for
    // the literal test `raised_err_at` makes at a call. An arm raises when
    // its answer carries RAISED, the bit infer sets only where a program
    // spells an err. An answer known to be a box is skipped, since a boxed
    // failure is not a bare one. An answer that can be ANY value is not known
    // to be one: its description bit is inference not knowing, and an arm
    // that hands on a field of a record the runtime built answers exactly
    // that beside its own err (ruled 2026-09-30). An answer with no RAISED
    // stays lenient either way, since unknown is not proof.
    let mut raisers: crate::hash::Map<(&str, usize), Vec<&[Pattern]>> = crate::hash::Map::default();
    let any = crate::infer::TOP & !crate::infer::FAIL;
    for (i, d) in program.fns.iter().enumerate() {
        let r = inference.returns[i];
        let boxed = r & crate::infer::DESC != 0 && r & any != any;
        if r & crate::infer::RAISED != 0 && !boxed {
            raisers.entry((d.name.as_str(), d.params.len())).or_default().push(&d.params);
        }
    }
    // The module's constants that are one string literal, for the same test.
    let consts = crate::infer::literal_consts(program);
    let tables = AfterInfer {
        discarded,
        nones,
        returns,
        proven: &inference.proven,
        none_free: &inference.none_free,
    };

    use crate::infer::{Set, DESC, FAIL, THUNK, TOP};
    // ONE MAP, NOT TWO. Both keys began with the declaration's name, so the
    // second hashed that string once per PARAMETER to build and once per
    // ARGUMENT to read, on top of the hash the same call site already paid for
    // the return set. The per-position bool is a bitmask beside the set and
    // the group's member list: one hash per declaration to build it, one per
    // call site to read it.
    //
    // A position past the mask's width reads as BINDING, which is the
    // under-refusing direction — the same choice kanso#1369 made for the
    // exhaustiveness mask, and for the same reason: a refusal this pass cannot
    // justify is worse than one it declines to make. The widest group in lib/
    // takes five parameters against a width of sixty-four.
    //
    // The middle field threads the group's members — the first in the map, the
    // rest through `next` — for the tail read below.
    let mut groups: crate::hash::Map<(&str, usize), (Set, u32, u64)> =
        crate::hash::Map::with_capacity_and_hasher(program.fns.len(), Default::default());
    let mut next: Vec<u32> = vec![u32::MAX; program.fns.len()];
    for (i, d) in program.fns.iter().enumerate() {
        let key = (d.name.as_str(), d.params.len());
        let group = groups.entry(key).or_insert((0, u32::MAX, 0));
        group.0 |= inference.returns[i];
        next[i] = group.1;
        group.1 = i as u32;
        for (pos, param) in d.params.iter().enumerate().take(64) {
            // a bare binder takes a box, and so does an arm that names the
            // effect type: `e:<int>effect` says it holds the channel
            let anything = match param {
                Pattern::Var(..) | Pattern::Wildcard(..) => true,
                Pattern::Annotated { ty, .. } => crate::ast::is_effect_type(ty),
                _ => false,
            };
            if anything {
                group.2 |= 1u64 << pos;
            }
        }
    }
    let values = TOP & !FAIL & !THUNK & !DESC;
    let boxed = |s: Set| s & DESC != 0 && s & values == 0;
    // WHETHER ANY NAME IN THE PROGRAM COULD ANSWER A BOX. When none could, a
    // name and a call can never be one, so the two arms below that ask the
    // table answer no without asking -- and the binder set those arms consult
    // is never read, so it is never built either. What is left is the chain,
    // which the expression says on its own. A program that never names an
    // effect pays a walk instead of a walk plus a set.
    //
    // The test is the DESCRIPTION BIT, not `boxed`. A wrapper like
    // `os/read_file` carries its callback's answer beside that bit, so its set
    // is not `boxed` and the tail walk below is what decides it; gating that
    // walk on `boxed` would hide every such group behind the short circuit.
    // Asking for the bit alone admits a superset of what `boxed` admits, so no
    // refusal is lost, and a program with no io at all still pays nothing.
    let any_boxed = groups.values().any(|(s, _, _)| s & DESC != 0);
    // a name the declaration binds itself — a parameter, a binding, a
    // lambda's parameter — is that binding, whatever declaration shares
    // its spelling; `fn either ... args` reads its own list, not `os/args`
    fn yields_box(
        e: &Expr,
        boxed_group: &dyn Fn(&str, usize) -> bool,
        shadows: &dyn Fn(&str) -> bool,
        any_boxed: bool,
        held: Option<&dyn Fn(&str) -> bool>,
    ) -> bool {
        match e {
            // a `.>` step answers the chain its subject opened
            Expr::App { args, piped: true, .. } => {
                args.first().is_some_and(|a| yields_box(a, boxed_group, shadows, any_boxed, held))
            }
            // THE TABLE ANSWERS BEFORE THE BINDER SET DOES, and both are a hash
            // of the same name. A name the table does not hold, or holds as
            // something other than a box, is not a box whoever bound it — so
            // the shadowing question is asked only of the few names that come
            // back boxed. Asking the binder set first paid its hash on every
            // name in every expression to short-circuit the locals, and the
            // locals are the common case only in the arms this pass walks past.
            // the box built by hand answers one whatever else the program
            // holds, so it is asked before the short circuit below, and so
            // does `print`: it is the one effect a program names without an
            // import, so no group's set need carry the bit that opens the
            // short circuit, and `is_effect_builtin` does not list it
            Expr::App { head, args, piped: false, .. }
                if args.len() == 1
                    && matches!(head.as_ref(), Expr::Ident(name, _, _)
                        if name == "effect" || name == "print") =>
            {
                matches!(head.as_ref(), Expr::Ident(name, _, _) if !shadows(name.as_str()))
            }
            // `xs[i]!` is the box a miss bubbles through (ruled 2026-09-16)
            Expr::Index { strict: true, .. } => true,
            Expr::App { head, args, piped: false, .. } if any_boxed => match head.as_ref() {
                // both branches of an `if` answering a box makes the `if` one
                Expr::Ident(name, _, _) if name == "if" && args.len() == 3 => {
                    yields_box(&args[1], boxed_group, shadows, any_boxed, held)
                        && yields_box(&args[2], boxed_group, shadows, any_boxed, held)
                }
                Expr::Ident(name, _, _) => {
                    boxed_group(name.as_str(), args.len()) && !shadows(name.as_str())
                }
                _ => false,
            },
            // a parameter some call hands a box holds one here (ruled
            // 2026-09-30); `held` is `None` everywhere but the pass that
            // reads the declarations those calls reach
            Expr::Ident(name, _, _) if held.is_some_and(|h| h(name.as_str())) => true,
            Expr::Ident(name, _, _) if any_boxed => {
                boxed_group(name.as_str(), 0) && !shadows(name.as_str())
            }
            Expr::Join { .. } => true,
            _ => false,
        }
    }
    // A group answers a box when its set says so, or when every arm ENDS in
    // one. The set alone misses the shape most of std/os takes: `os/read_file`
    // is `builtin_read_file path .> (r -> found path r)`, and a pipe's set
    // carries the callback's answer beside the description bit, because the
    // yield rides in the set — so `length (os/read_file p)` read as a value
    // and died at run time with `chars takes a string`. The tail can tell: a
    // `.>` over a box, a join, an effect builtin, or a call of a group
    // that answers one, followed through the callee's own tail, so a wrapper
    // of a wrapper is seen. Every arm must agree, which the one-shape rule at
    // dispatch already asks of them. A group still being decided — a tail
    // that calls back into its own group — answers no, which refuses nothing.
    //
    // Whether the tail's head is a name the arm binds is asked of the arm
    // itself rather than of a set built per arm: a tail asks it once or
    // twice, and a set for every declaration is an allocation apiece.
    struct Tails<'a> {
        program: &'a Program,
        returns: &'a crate::hash::Map<(&'a str, usize), (Set, u32, u64)>,
        next: &'a [u32],
        boxed: &'a dyn Fn(Set) -> bool,
        state: Vec<std::cell::Cell<u8>>,
    }
    const UNKNOWN: u8 = 0;
    const DECIDING: u8 = 1;
    const NO: u8 = 2;
    const YES: u8 = 3;
    impl Tails<'_> {
        fn group(&self, name: &str, arity: usize) -> bool {
            let Some(&(set, head, _)) = self.returns.get(&(name, arity)) else {
                return crate::infer::is_effect_builtin(name);
            };
            if (self.boxed)(set) {
                return true;
            }
            // the state lives on the group's first member
            let cell = &self.state[head as usize];
            match cell.get() {
                YES => return true,
                NO | DECIDING => return false,
                _ => cell.set(DECIDING),
            }
            let mut i = head;
            let mut all = true;
            while i != u32::MAX && all {
                let decl = &self.program.fns[i as usize];
                all = match decl.body.last() {
                    Some(Stmt::Expr(tail)) => yields_box(
                        tail,
                        &|n, a| self.group(n, a),
                        &|n| binds_name(decl, n),
                        true,
                        None,
                    ),
                    _ => false,
                };
                i = self.next[i as usize];
            }
            cell.set(if all { YES } else { NO });
            all
        }
    }
    /// Every place `param` is handed to something that takes a value: an
    /// operator, an index, a field read, an `if`'s condition or a builtin that
    /// reads its argument. A lambda inside that binds the same name hides it.
    fn read_as_value(
        e: &Expr,
        param: &str,
        reads_value: &dyn Fn(&str) -> bool,
        found: &mut dyn FnMut(&str, Span),
    ) {
        let is_param = |x: &Expr| matches!(x, Expr::Ident(n, _, _) if n == param);
        match e {
            Expr::Lambda { params, .. } if params.iter().any(|(n, _)| n == param) => return,
            Expr::BinOp { op, lhs, rhs, .. } => {
                for side in [lhs, rhs] {
                    if is_param(side) {
                        found(&format!("`{op}`"), side.span());
                    }
                }
            }
            Expr::Index { base, index, .. } => {
                for part in [base, index] {
                    if is_param(part) {
                        found("an index", part.span());
                    }
                }
            }
            Expr::Field { base, name, .. } if is_param(base) => {
                found(&format!("`.{name}`"), base.span());
            }
            Expr::App { head, args, piped: false, .. } => {
                if let Expr::Ident(name, _, _) = head.as_ref() {
                    let reading = name == "if" || reads_value(name.as_str());
                    let args = if name == "if" { &args[..args.len().min(1)] } else { &args[..] };
                    for arg in args.iter().filter(|a| reading && is_param(a)) {
                        found(&format!("`{name}`"), arg.span());
                    }
                }
            }
            _ => {}
        }
        crate::for_each_child(e, |c| read_as_value(c, param, reads_value, found));
    }
    fn binds_name(decl: &FnDecl, name: &str) -> bool {
        fn lambda_binds(e: &Expr, name: &str) -> bool {
            if let Expr::Lambda { params, .. } = e {
                if params.iter().any(|(n, _)| n == name) {
                    return true;
                }
            }
            let mut hit = false;
            crate::for_each_child(e, |c| {
                if !hit {
                    hit = lambda_binds(c, name);
                }
            });
            hit
        }
        let mut hit = false;
        for param in &decl.params {
            for_each_param_name(param, &mut |n| hit |= n == name);
        }
        hit || decl.body.iter().any(|stmt| match stmt {
            Stmt::Bind { pattern, expr } => {
                let mut bound = false;
                for_each_param_name(pattern, &mut |n| bound |= n == name);
                bound || lambda_binds(expr, name)
            }
            Stmt::Expr(expr) => lambda_binds(expr, name),
        })
    }
    let tails = Tails {
        program,
        returns: &groups,
        next: &next,
        boxed: &boxed,
        state: vec![std::cell::Cell::new(UNKNOWN); program.fns.len()],
    };
    let boxed_group = |name: &str, arity: usize| tails.group(name, arity);
    // the builtins that read a value. The words take the box; `print` and an
    // interpolation render it; `is_desc` asks about it; `push` and `put`
    // store it, which is holding; `err` and `wrap_err` carry it as a reason
    let reads_value = |name: &str| {
        BUILTINS.contains(&name)
            && !matches!(
                name,
                "annotate"
                    | "bind"
                    | "rescue"
                    | "print"
                    | "is_desc"
                    | "push"
                    | "put"
                    | "err"
                    | "wrap_err"
                    | "effect"
            )
    };
    let refuse = |diags: &mut Vec<Diagnostic>, who: &str, at: Span| {
        diags.push(Diagnostic::new(
            "effect",
            format!(
                "this is an effect — a box the words open — and {who} takes a value; \
                 open it with `.>`"
            ),
            at,
        ));
    };
    // THE BINDER SET IS BUILT ON FIRST ASK, and most declarations never ask.
    // It is a second full traversal of the body -- `bound_in_expr` walks every
    // expression to find the names lambdas and bindings introduce -- and its
    // only reader is the shadowing test, which now sits behind the returns
    // table and so runs for the few names the table holds as a box. Filling on
    // the first of those asks answers it with the same set the eager build
    // would have handed over, because the fill happens before the answer; a
    // declaration that never names a box pays one walk over its body instead
    // of two.
    struct Binders<'b> {
        loaded: usize,
        set: HashSet<&'b str>,
    }
    let binders: std::cell::RefCell<Binders<'p>> =
        std::cell::RefCell::new(Binders { loaded: usize::MAX, set: HashSet::default() });
    let shadows = |i: usize, name: &str| -> bool {
        let mut b = binders.borrow_mut();
        if b.loaded != i {
            let decl = &program.fns[i];
            let set = &mut b.set;
            set.clear();
            for param in &decl.params {
                for_each_param_name(param, &mut |n| {
                    set.insert(n);
                });
            }
            for stmt in &decl.body {
                bound_in_stmt(stmt, set);
            }
            b.loaded = i;
        }
        b.set.contains(name)
    };
    // A parameter some call hands a box: the name, and the call where the box
    // first went into a parameter, as the declaration making it and the group
    // it called. A box handed on from one parameter to the next keeps the
    // first call, since that is where it can be opened.
    type Held<'a> = (&'a str, &'a str, &'a str);
    type Handed<'a> = (u32, usize, Option<(&'a str, &'a str)>);
    let site = |e: &Expr,
                shadow: &dyn Fn(&str) -> bool,
                held: &[Held<'p>],
                handed: &mut Vec<Handed<'p>>,
                diags: &mut Vec<Diagnostic>| {
        let holds = |n: &str| held.iter().any(|h| h.0 == n);
        let holds: Option<&dyn Fn(&str) -> bool> = (!held.is_empty()).then_some(&holds);
        let is_box = |e: &Expr| yields_box(e, &boxed_group, shadow, any_boxed, holds);
        let say = |diags: &mut Vec<Diagnostic>, who: &str, at: &Expr| {
            let from = match at {
                Expr::Ident(n, _, _) => held.iter().find(|h| h.0 == n.as_str()),
                _ => None,
            };
            let Some((name, caller, callee)) = from else {
                return refuse(diags, who, at.span());
            };
            diags.push(Diagnostic::new(
                "effect",
                format!(
                    "`{name}` holds an effect — `{caller}` hands `{callee}` one — and \
                     {who} takes a value; open it with `.>` where `{caller}` calls `{callee}`"
                ),
                at.span(),
            ));
        };
        match e {
            Expr::BinOp { op, lhs, rhs, .. } => {
                for side in [lhs, rhs] {
                    if is_box(side) {
                        say(diags, &format!("`{op}`"), side);
                    }
                }
            }
            Expr::Index { base, index, .. } => {
                if is_box(base) {
                    say(diags, "an index", base);
                }
                if is_box(index) {
                    say(diags, "an index", index);
                }
            }
            Expr::Field { base, name, .. } => {
                if is_box(base) {
                    say(diags, &format!("`.{name}`"), base);
                }
            }
            // `box . (n -> n - 1)` is the lambda applied to the box, and the
            // dot opens nothing, so `n` IS the box. The operand the checker
            // would refuse is a parameter rather than a call, which the test
            // above cannot see, so the lambda's body is read for the places
            // that parameter meets something that takes a value.
            Expr::App { head, args, piped: false, .. }
                if matches!(head.as_ref(), Expr::Lambda { .. }) =>
            {
                let Expr::Lambda { params, body, .. } = head.as_ref() else { return };
                for ((param, _), arg) in params.iter().zip(args) {
                    if param != "_" && is_box(arg) {
                        read_as_value(body, param, &reads_value, &mut |who, at| {
                            refuse(diags, who, at)
                        });
                    }
                }
            }
            Expr::App { head, args, piped: false, .. } => {
                let Expr::Ident(name, _, _) = head.as_ref() else { return };
                if name == "if" {
                    if let Some(cond) = args.first() {
                        if is_box(cond) {
                            say(diags, "`if`", cond);
                        }
                    }
                    return;
                }
                // NOTHING TO SAY UNLESS AN ARGUMENT IS A BOX, and asking that is a
                // match on the argument where asking the table is a hash of the
                // callee's name. A call whose arguments are literals, arithmetic
                // or field reads answers no on the match alone, so the table is
                // consulted only where a refusal is actually in question. The
                // second walk over the arguments below costs a few compares on
                // the sites that reach it, which are the rare ones.
                if !args.iter().any(&is_box) {
                    return;
                }
                // one lookup for the whole call: whether the name is a group,
                // and which of its positions bind anything
                let found = groups.get(&(name.as_str(), args.len())).copied();
                if found.is_none() && !reads_value(name.as_str()) {
                    return;
                }
                let binds = found.map_or(0, |(_, _, b)| b);
                for (pos, arg) in args.iter().enumerate() {
                    if !is_box(arg) {
                        continue;
                    }
                    if let Some((_, head, _)) = found {
                        if pos >= 64 {
                            continue;
                        }
                        if binds & (1u64 << pos) != 0 {
                            let first = match arg {
                                Expr::Ident(n, _, _) => held.iter().find(|h| h.0 == n.as_str()),
                                _ => None,
                            };
                            handed.push((head, pos, first.map(|h| (h.1, h.2))));
                            continue;
                        }
                    }
                    say(diags, &format!("`{name}`"), arg);
                }
            }
            _ => {}
        }
    };

    // One worklist for the whole program rather than one per statement, which
    // keeps the capacity a long declaration earned instead of doubling up from
    // nothing at the next one. The `clear` is what makes that safe: the loop
    // below happens to drain the stack every time, but nothing tests that it
    // does — a mutation that leaves items behind keeps the whole error corpus
    // green — so the reuse does not rest on it.
    let mut stack: Vec<&Expr> = Vec::new();
    // every group position a box was let into, with the declaration whose call
    // let it in, for the pass after this walk
    let mut handed: Vec<(Handed<'p>, u32)> = Vec::new();
    let mut handed_here: Vec<Handed<'p>> = Vec::new();
    for (i, decl) in program.fns.iter().enumerate() {
        // The box question's shadowing test is asked of THIS declaration, and
        // the set behind it is filled on the first ask — so the closure has to
        // be rebuilt per declaration even though nothing else here is.
        let shadow = |n: &str| shadows(i, n);
        let marks = (effect_diags.len(), box_diags.len(), none_diags.len());
        let named = raising_names(decl, &raisers, &consts);
        for stmt in &decl.body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            stack.clear();
            stack.push(e);
            while let Some(cur) = stack.pop() {
                effect_discarded_at(cur, &tables, effect_diags);
                none_exhaustive_at(cur, &tables, &decl.file, none_diags);
                raised_err_at(cur, &raisers, &consts, &err_arms, &named, &decl.file, none_diags);
                site(cur, &shadow, &[], &mut handed_here, box_diags);
                crate::for_each_child(cur, |c| stack.push(c));
            }
        }
        handed.extend(handed_here.drain(..).map(|h| (h, i as u32)));
        // All three are about this declaration's statements, and in a module
        // the program holds every file's, so each is placed in its file.
        place_in(&mut effect_diags[marks.0..], &decl.file);
        place_in(&mut box_diags[marks.1..], &decl.file);
        place_in(&mut none_diags[marks.2..], &decl.file);
    }
    if handed.is_empty() {
        return;
    }

    // A BOX HANDED TO A PARAMETER IS A BOX INSIDE THE BODY (ruled 2026-09-30).
    // The walk above lets a box through wherever the callee's group binds that
    // position, and a bare binder may hold one: a description reaching a
    // dispatch lands on the bare arm, and `held e` hands its box back. What
    // it cannot see is the body. Every arm that NAMES the position now holds a
    // box under that name, so the body is read again with the name counted as
    // one, and a box it hands on to a further group is followed the same way.
    // Names are only ever added and there are finitely many, so the reading
    // stops. A name the body binds again, in a binding or a lambda, is left
    // out: the check under-refuses where it cannot tell which one is meant.
    let mut held: crate::hash::Map<u32, Vec<Held<'p>>> = crate::hash::Map::default();
    let mut reread: Vec<u32> = Vec::new();
    let mut rebound: HashSet<&str> = HashSet::default();
    let read = |j: u32,
                names: &[Held<'p>],
                stack: &mut Vec<&'p Expr>,
                handed: &mut Vec<Handed<'p>>,
                diags: &mut Vec<Diagnostic>| {
        let shadow = |n: &str| shadows(j as usize, n);
        for stmt in &program.fns[j as usize].body {
            let e = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            stack.clear();
            stack.push(e);
            while let Some(cur) = stack.pop() {
                site(cur, &shadow, names, handed, diags);
                crate::for_each_child(cur, |c| stack.push(c));
            }
        }
    };
    let mut scratch: Vec<Diagnostic> = Vec::new();
    while !handed.is_empty() {
        for ((head, pos, first), caller) in handed.drain(..) {
            let (from, to) = first.unwrap_or((
                program.fns[caller as usize].name.as_str(),
                program.fns[head as usize].name.as_str(),
            ));
            let mut j = head;
            while j != u32::MAX {
                let decl = &program.fns[j as usize];
                let name = match decl.params.get(pos) {
                    Some(Pattern::Var(n, _)) => Some(n.as_str()),
                    Some(Pattern::Annotated { name, ty, .. }) if crate::ast::is_effect_type(ty) => {
                        Some(name.as_str())
                    }
                    _ => None,
                };
                if let Some(name) = name.filter(|n| *n != "_") {
                    rebound.clear();
                    for stmt in &decl.body {
                        bound_in_stmt(stmt, &mut rebound);
                    }
                    let names = held.entry(j).or_default();
                    if !rebound.contains(name) && !names.iter().any(|h| h.0 == name) {
                        names.push((name, from, to));
                        reread.push(j);
                    }
                }
                j = next[j as usize];
            }
        }
        reread.sort_unstable();
        reread.dedup();
        for j in reread.drain(..) {
            let names = held[&j].clone();
            read(j, &names, &mut stack, &mut handed_here, &mut scratch);
            handed.extend(handed_here.drain(..).map(|h| (h, j)));
        }
        scratch.clear();
    }
    // What the second reading says that the first did not is the refusal of a
    // held name, so only a refusal at a span the first reading left alone is
    // kept; everything else there was already said above.
    let mut order: Vec<u32> = held.iter().filter(|(_, n)| !n.is_empty()).map(|(j, _)| *j).collect();
    order.sort_unstable();
    for j in order {
        let mut before: Vec<Diagnostic> = Vec::new();
        read(j, &[], &mut stack, &mut handed_here, &mut before);
        read(j, &held[&j], &mut stack, &mut handed_here, &mut scratch);
        handed_here.clear();
        let mark = box_diags.len();
        box_diags.extend(scratch.drain(..).filter(|d| !before.iter().any(|b| b.span == d.span)));
        place_in(&mut box_diags[mark..], &program.fns[j as usize].file);
    }
}
