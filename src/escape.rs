//! Escape analysis: which record types provably never escape, so a function
//! returning one can hand it back by value (in registers) instead of heap
//! allocating it. Unsoundness here is memory corruption, so the analysis is
//! deliberately conservative — a type is register-returnable only when *every*
//! syntactic occurrence of it is provably safe, and anything unrecognized
//! forces the type to "escapes".
//!
//! The safety argument leans on one structural fact: a register-returnable
//! type is never bound to a generic name. Its values exist only transiently as
//! expression results that flow construction -> return -> destructure, so there
//! is no variable a value could hide in and leak through. That lets the check
//! stay syntactic (no heap-flow dataflow) while remaining sound.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::hash::{Map as HashMap, Set as HashSet};

/// What codegen needs to hand register-returnable records back by value.
pub struct EscapeInfo {
    /// Register-returnable type name -> field count.
    pub field_count: HashMap<String, usize>,
    /// Groups whose result is a register-returnable type, by function name:
    /// each arity with that type's name.
    returns: HashMap<String, Vec<(usize, String)>>,
    /// Positions that carry a register-returnable type (destructured as
    /// `(T ...)` by some arm), by function name: each arity and parameter
    /// index with the type's name.
    carries: HashMap<String, Vec<(usize, usize, String)>>,
}

// Keyed by name alone so a question borrows the name it is asked with. Keyed
// by `(name, arity)` it copied the name into a key on every question, and
// codegen asks once for every argument of every call it emits.
impl EscapeInfo {
    pub fn returns_ty(&self, name: &str, arity: usize) -> Option<&str> {
        let groups = self.returns.get(name)?;
        groups.iter().find(|(a, _)| *a == arity).map(|(_, ty)| ty.as_str())
    }

    pub fn carries_ty(&self, name: &str, arity: usize, param: usize) -> Option<&str> {
        let slots = self.carries.get(name)?;
        slots.iter().find(|(a, p, _)| *a == arity && *p == param).map(|(_, _, ty)| ty.as_str())
    }

    /// Keeps only the answers whose type `keep` accepts.
    pub fn keep_types(&mut self, keep: impl Fn(&str) -> bool) {
        for groups in self.returns.values_mut() {
            groups.retain(|(_, ty)| keep(ty));
        }
        for slots in self.carries.values_mut() {
            slots.retain(|(_, _, ty)| keep(ty));
        }
    }

    fn from_groups(
        field_count: HashMap<String, usize>,
        returns: HashMap<(String, usize), String>,
        carries: HashMap<(String, usize, usize), String>,
    ) -> Self {
        let mut info =
            EscapeInfo { field_count, returns: HashMap::default(), carries: HashMap::default() };
        for ((name, arity), ty) in returns {
            info.returns.entry(name).or_default().push((arity, ty));
        }
        for ((name, arity, param), ty) in carries {
            info.carries.entry(name).or_default().push((arity, param, ty));
        }
        info
    }
}

/// Full analysis result for codegen: the returnable types plus the groups that
/// return them and the parameter positions that carry them.
pub fn analyze(program: &Program, inference: &crate::infer::Inference) -> EscapeInfo {
    let mut info = analyze_inner(program, inference);
    // union groups (any synthetic member — the bare overload space) stay
    // boxed: their arms come from different modules with independently
    // computed conventions, and mixing %parsed with %KValue in one
    // dispatcher is the register-ABI crash family
    //
    // An operator's arms stay boxed too. `a + b` reaches them through the
    // operator's own dispatch, which asks at run time whether either side is
    // a record and then calls the group with the two values it holds, boxed,
    // and reads a boxed answer back. A record shaped like `_parsed` -- an int
    // and a value -- let `fn + (pt a s) (pt b t)` take and return words, and
    // the arm read a boxed record's tag and pointer as a packed int.
    let mut union_groups = crate::hash::Set::default();
    for d in &program.fns {
        if d.synthetic || crate::is_operator(&d.name) {
            union_groups.insert((d.name.clone(), d.params.len()));
        }
    }
    for (name, arity) in &union_groups {
        if let Some(groups) = info.returns.get_mut(name) {
            groups.retain(|(a, _)| a != arity);
        }
        if let Some(slots) = info.carries.get_mut(name) {
            slots.retain(|(a, _, _)| a != arity);
        }
    }
    info
}

fn analyze_inner(program: &Program, inference: &crate::infer::Inference) -> EscapeInfo {
    let returnable = register_returnable(program, inference);
    let mut field_count = HashMap::default();
    let mut returns = HashMap::default();
    let mut carries = HashMap::default();
    for ty in &returnable {
        if let Some(decl) = program.types.iter().find(|t| &t.name == ty) {
            field_count.insert(ty.clone(), decl.fields.len());
        }
        let mut analysis = Analysis { program, returns_ty: HashSet::default() };
        analysis.compute_returns_ty(ty);
        for (name, arity) in analysis.returns_ty {
            returns.insert((name.to_string(), arity), ty.clone());
        }
        for f in &program.fns {
            // A getter is the one function that must look at what it was
            // handed rather than trust its signature: its group is reachable
            // with a record of any type, and the by-value convention would
            // read two words out of a record that may hold one. Reading a
            // field of the wrong record has to say so, not answer a number.
            if f.is_getter() {
                continue;
            }
            for (i, p) in f.params.iter().enumerate() {
                // An as-pattern wants the value, and the by-value convention
                // passes a record's words rather than the record: there is
                // nothing to hand the name that would not have to be built.
                // The parameter stays boxed, which is still cheaper than the
                // rebuild the as-pattern exists to remove.
                if matches!(p, Pattern::Ctor { ty: pty, whole: None, .. } if pty == ty) {
                    carries.insert((f.name.clone(), f.params.len(), i), ty.clone());
                }
            }
        }
    }
    // A position carries a type only when every arm of the group agrees on it.
    // The loop above writes one entry per (group, arity, position) and a second
    // arm naming a different record simply overwrote the first, so the whole
    // group was passed unboxed as one type and the dispatcher assumed that type
    // instead of testing it — every call answered from whichever arm was
    // written last. This is the reason a getter is skipped above, generalized:
    // a parameter reachable with more than one record has to be looked at.
    // The convention is a property of the position, not of one arm: if any arm
    // there names the whole value, every arm at that position is passed boxed,
    // or the arm that named it would be handed two words and no record.
    //
    // An `e@(err _)` arm is the exception, and it costs nothing to admit: the
    // dispatcher already reads the two words back as one KValue, and on the
    // failure path that KValue IS the failure, so the name is bound to the
    // value that arrived and no record was ever wanted. Until 2026-09-16 the
    // arm counted as an as-pattern like any other, and the five hand-back arms
    // the 2026-09-15 ruling asked of the json decoder turned the convention
    // off at every carried slot: sh_rec 0 -> 253,968,000 on the decode.
    carries.retain(|(name, arity, at), _| {
        let as_bound = program
            .fns
            .iter()
            .filter(|f| f.name == *name && f.params.len() == *arity)
            .any(|f| {
                matches!(f.params.get(*at), Some(Pattern::Ctor { ty, whole: Some(_), .. }) if ty != "err")
            });
        !as_bound
    });
    carries.retain(|(name, arity, at), _| {
        let mut named = program
            .fns
            .iter()
            .filter(|f| f.name == *name && f.params.len() == *arity && !f.is_getter())
            .filter_map(|f| match f.params.get(*at) {
                Some(Pattern::Ctor { ty, whole: None, .. }) if ty != "err" => Some(ty.as_str()),
                _ => None,
            });
        let first = named.next();
        named.all(|ty| Some(ty) == first)
    });
    // A by-value slot is handed a record's words, and a caller turns what it
    // holds into them with `k_parsed_words`, which reads two fields off a
    // record and passes a failure through. Nothing else has words to hand
    // over. An arm that takes anything at the position -- `_`, a name, a
    // literal -- lets a caller pass something else, and `total 5` beside
    // `total (point x y)` reached the runtime as an int it read fields off,
    // until the stack ran out. So the slot stays boxed unless every shape the
    // inference sees reaching it is a record, a failure or a thunk forced
    // before the call.
    carries.retain(|(name, arity, at), _| {
        let reaching = program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.name == *name && f.params.len() == *arity)
            .fold(0, |acc, (i, _)| acc | inference.param(i, *at));
        reaching & !(crate::infer::REC | crate::infer::FAIL | crate::infer::THUNK) == 0
    });
    // The inference has one bit for every record, so it cannot say a `circle`
    // never reaches a slot whose `_` arm would take one. An arm that takes any
    // value at the position keeps the slot boxed.
    carries.retain(|(name, arity, at), _| {
        !program.fns.iter().filter(|f| f.name == *name && f.params.len() == *arity).any(|f| {
            matches!(
                f.params.get(*at),
                Some(Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Annotated { .. })
            )
        })
    });
    EscapeInfo::from_groups(field_count, returns, carries)
}

/// Record type names that may be returned by value. A type qualifies when:
///  - it is never a field of another record type,
///  - it is only ever destructured via `(T ...)` patterns (never bound to a
///    Var/Wildcard/Annotated pattern), and
///  - every expression that produces a T value (a `T ...` construction, or a
///    call to a function whose group returns T) sits in a safe position: the
///    tail of a T-returning function, an `if` branch in tail position, or an
///    argument whose callee destructures that parameter as `(T ...)`.
pub fn register_returnable(
    program: &Program,
    inference: &crate::infer::Inference,
) -> HashSet<String> {
    let ctors: HashSet<&str> =
        program.types.iter().filter(|t| !t.fields.is_empty()).map(|t| t.name.as_str()).collect();

    let analysis = Analysis { program, returns_ty: HashSet::default() };
    let values = value_names(program);

    ctors
        .iter()
        .filter(|ty| analysis.clone().returnable(ty, inference, &values))
        .map(|ty| ty.to_string())
        .collect()
}

/// Every name handed out as a value rather than called: `&f` anywhere, and a
/// bare `f` that is not the head of a call. A local shares the spelling of a
/// function only by accident, and counting it keeps a record boxed that could
/// have gone by value, which costs time and never correctness.
fn value_names(program: &Program) -> HashSet<String> {
    fn walk(e: &Expr, out: &mut HashSet<String>) {
        match e {
            Expr::Partial(name, _) | Expr::Ident(name, _, _) => {
                out.insert(name.as_str().to_string());
            }
            Expr::App { head, args, .. } if matches!(head.as_ref(), Expr::Ident(..)) => {
                for a in args {
                    walk(a, out);
                }
            }
            _ => crate::for_each_child(e, |c| walk(c, out)),
        }
    }
    let mut out = HashSet::default();
    for f in &program.fns {
        for st in &f.body {
            match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    walk(expr, &mut out)
                }
            }
        }
    }
    out
}

#[derive(Clone)]
struct Analysis<'a> {
    program: &'a Program,
    /// (function name, arity) groups whose result is a `ty` value or a failure.
    /// The names are borrowed from the program: the fixpoint below asks after
    /// every function on every round, and `produces_ty` after every call it
    /// reads, and a key holding a copy of the name cost a copy per question.
    returns_ty: HashSet<(&'a str, usize)>,
}

impl<'a> Analysis<'a> {
    fn returnable(
        mut self,
        ty: &str,
        inference: &crate::infer::Inference,
        values: &HashSet<String>,
    ) -> bool {
        // The packed convention shifts field 0's payload into the tag word,
        // which is only sound for an int: a pointer payload would lose its
        // tag and overflow the shift. Fields carry no written types, so the
        // question is put to inference — every construction site's first
        // argument joined, which must be an int and nothing else. A bignum
        // is an int whose word would not survive the shift, so it spills to
        // the heap record the way a word past 56 bits does.
        let first_field_is_int = self
            .program
            .types
            .iter()
            .position(|t| t.name == ty)
            .and_then(|idx| inference.type_fields.get(idx))
            .and_then(|fields| fields.first())
            .is_some_and(|set| *set & crate::infer::INT != 0 && *set & !crate::infer::ANY_INT == 0);
        if !first_field_is_int {
            return false;
        }
        // A type stored inside another record escapes through that record.
        for decl in &self.program.types {
            for (_, members, _) in &decl.fields {
                if members.iter().any(|m| m == ty) {
                    return false;
                }
            }
        }
        self.compute_returns_ty(ty);
        // A group returns ty when one arm's tail produces it, and the other
        // arms may answer anything. Only a failure crosses the two words
        // intact: its first word is exactly K_ERR, which no packed word is.
        // Anything else an arm answers is read back as a packed ty: `done`
        // from `list/next` when a `take` runs out, or a `cursor` from the
        // `iter` arm beside the one answering `capped`, which came back as
        // a capped. The inference has one bit for every record, so it cannot
        // tell those two apart, and the question is put to the arms
        // themselves: each branch ends in ty, in `err`, or in a call to a
        // group that answers nothing but failures, unless the arm as a whole
        // answers nothing but failures. Until 2026-10-09 a program
        // importing std/list never let `step` or `capped` go by value,
        // because functions nobody called kept them boxed; once native
        // builds dropped unreached functions, both programs answered wrong.
        let answers_other = self.program.fns.iter().enumerate().any(|(i, f)| {
            self.returns_ty.contains(&(f.name.as_str(), f.params.len()))
                && !inference.returns.get(i).is_some_and(|set| set & !crate::infer::FAIL == 0)
                && !matches!(f.body.last(), Some(Stmt::Expr(e)) if self.ends_in_ty(ty, e, inference))
        });
        if answers_other {
            return false;
        }
        // A function handed out as a value is called through a wrapper that
        // answers one boxed word, so no group returning ty may be one. A
        // constant named bare is evaluated where it is named, and codegen
        // boxes its answer there, so a group of no arguments is not one.
        if self.returns_ty.iter().any(|(name, arity)| *arity > 0 && values.contains(*name)) {
            return false;
        }
        self.program.fns.iter().all(|f| self.body_is_safe(ty, &f.body))
    }

    /// Fixpoint: a group returns `ty` when every tail leaf of every member is a
    /// `ty` construction, a failure, or a call to another group that returns
    /// `ty`.
    fn compute_returns_ty(&mut self, ty: &str) {
        let program = self.program;
        loop {
            let mut changed = false;
            for f in &program.fns {
                let key = (f.name.as_str(), f.params.len());
                if self.returns_ty.contains(&key) {
                    continue;
                }
                if self.tail_returns_ty(ty, &f.body) {
                    self.returns_ty.insert(key);
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// A group returns `ty` when some arm's tail produces an actual `ty` value
    /// (a construction or a call to another `ty`-returning group) — not merely a
    /// failure. `err(...)`/`none` arms are propagation, not `ty` production, so
    /// they must not pull a function that returns json-or-failure into the set.
    fn tail_returns_ty(&self, ty: &str, body: &[Stmt]) -> bool {
        match body.last() {
            Some(Stmt::Expr(e)) => self.produces_ty(ty, e),
            _ => false,
        }
    }

    fn body_is_safe(&self, ty: &str, body: &[Stmt]) -> bool {
        let Some((last, rest)) = body.split_last() else {
            return true;
        };
        // Non-tail statements never mention ty (no construction, no binding).
        for stmt in rest {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    if self.pattern_binds_ty(ty, pattern) || self.expr_mentions_ty(ty, expr) {
                        return false;
                    }
                }
                Stmt::Expr(e) => {
                    if self.expr_mentions_ty(ty, e) {
                        return false;
                    }
                }
                Stmt::Set { value, .. } => {
                    if self.expr_mentions_ty(ty, value) {
                        return false;
                    }
                }
            }
        }
        match last {
            Stmt::Bind { .. } | Stmt::Set { .. } => false,
            Stmt::Expr(e) => self.tail_position_safe(ty, e),
        }
    }

    /// A tail expression may itself be a ty construction/return, or an `if`
    /// whose branches are each tail-safe. Anywhere else, ty must not appear
    /// except as an argument the callee immediately destructures.
    fn tail_position_safe(&self, ty: &str, e: &Expr) -> bool {
        if let Expr::App { head, args, .. } = e {
            if let Expr::Ident(name, _, _) = head.as_ref() {
                if name == "if" && args.len() == 3 {
                    return !self.expr_mentions_ty(ty, &args[0])
                        && self.tail_position_safe(ty, &args[1])
                        && self.tail_position_safe(ty, &args[2]);
                }
                if name == ty {
                    // a ty construction in tail position: its field args must
                    // not themselves smuggle a ty value anywhere unsafe.
                    return args.iter().all(|a| self.arg_safe(ty, a));
                }
            }
        }
        // Any other tail expression: safe as long as ty only appears in
        // immediately-destructured argument positions.
        self.expr_safe_calls(ty, e)
    }

    /// `e` is a call argument. It may produce a ty value only if the enclosing
    /// call destructures it; that is validated by the caller of this via
    /// `call_safe`. Here we ensure any *nested* ty usage inside `e` is safe.
    fn arg_safe(&self, ty: &str, e: &Expr) -> bool {
        self.expr_safe_calls(ty, e)
    }

    /// Walk `e` verifying every call that passes a ty-producing argument is
    /// received by a parameter destructured as `(ty ...)`, and that ty never
    /// appears in a container, index, binop, lambda, or template.
    fn expr_safe_calls(&self, ty: &str, e: &Expr) -> bool {
        match e {
            Expr::Int(..)
            | Expr::Float(..)
            | Expr::Ident(..)
            | Expr::Partial(..)
            | Expr::Hole(..) => true,
            Expr::Upcast { expr, .. } => self.expr_safe_calls(ty, expr),
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts.iter().all(|st| match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    self.expr_safe_calls(ty, expr)
                }
            }),
            Expr::Field { base, .. } => {
                !self.produces_ty(ty, base) && self.expr_safe_calls(ty, base)
            }
            Expr::Str(parts, _) => parts.iter().all(|p| match p {
                crate::ast::TemplatePart::Lit(_) => true,
                crate::ast::TemplatePart::Interp(x) => {
                    !self.produces_ty(ty, x) && self.expr_safe_calls(ty, x)
                }
            }),
            Expr::List(items, _) => {
                items.iter().all(|x| !self.produces_ty(ty, x) && self.expr_safe_calls(ty, x))
            }
            Expr::MapLit(pairs, _) => pairs.iter().all(|(k, v)| {
                !self.produces_ty(ty, k)
                    && !self.produces_ty(ty, v)
                    && self.expr_safe_calls(ty, k)
                    && self.expr_safe_calls(ty, v)
            }),
            Expr::Index { base, index, .. } => {
                !self.produces_ty(ty, base)
                    && !self.produces_ty(ty, index)
                    && self.expr_safe_calls(ty, base)
                    && self.expr_safe_calls(ty, index)
            }
            Expr::Guard { cond, early, rest, .. } => {
                !self.produces_ty(ty, cond)
                    && !self.produces_ty(ty, early)
                    && self.expr_safe_calls(ty, cond)
                    && self.expr_safe_calls(ty, early)
                    && rest.iter().all(|s| {
                        let e = match s {
                            Stmt::Bind { expr, .. }
                            | Stmt::Expr(expr)
                            | Stmt::Set { value: expr, .. } => expr,
                        };
                        !self.produces_ty(ty, e) && self.expr_safe_calls(ty, e)
                    })
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                !self.produces_ty(ty, lhs)
                    && !self.produces_ty(ty, rhs)
                    && self.expr_safe_calls(ty, lhs)
                    && self.expr_safe_calls(ty, rhs)
            }
            Expr::Lambda { body, .. } => {
                !self.produces_ty(ty, body) && self.expr_safe_calls(ty, body)
            }
            Expr::App { head, args, .. } => {
                let Expr::Ident(name, _, _) = head.as_ref() else {
                    // higher-order head: any ty inside is unsafe
                    return !self.produces_ty(ty, head)
                        && args
                            .iter()
                            .all(|a| !self.produces_ty(ty, a) && self.expr_safe_calls(ty, a));
                };
                if name == "if" && args.len() == 3 {
                    return !self.produces_ty(ty, &args[0])
                        && self.expr_safe_calls(ty, &args[0])
                        && args[1..]
                            .iter()
                            .all(|a| !self.produces_ty(ty, a) && self.expr_safe_calls(ty, a));
                }
                for (i, a) in args.iter().enumerate() {
                    if self.produces_ty(ty, a) {
                        if !self.callee_destructures(ty, name, args.len(), i) {
                            return false;
                        }
                        // the produced value is consumed here; still check its
                        // own subexpressions.
                        if !self.expr_safe_calls(ty, a) {
                            return false;
                        }
                    } else if !self.expr_safe_calls(ty, a) {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Does every branch of `e` end in a ty value or a failure? A failure is
    /// an `err`, or a call to a group whose every arm answers only failures,
    /// such as json's `fail`. `produces_ty` asks whether one branch is a ty.
    fn ends_in_ty(&self, ty: &str, e: &Expr, inference: &crate::infer::Inference) -> bool {
        match e {
            Expr::App { head, args, .. } => match head.as_ref() {
                Expr::Ident(name, _, _) if name == "if" && args.len() == 3 => {
                    self.ends_in_ty(ty, &args[1], inference)
                        && self.ends_in_ty(ty, &args[2], inference)
                }
                Expr::Ident(name, _, _) if name == "err" => true,
                Expr::Ident(name, _, _) => {
                    self.produces_ty(ty, e) || self.fails_only(name, args.len(), inference)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn fails_only(&self, name: &str, arity: usize, inference: &crate::infer::Inference) -> bool {
        let mut arms = self
            .program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.name == name && f.params.len() == arity)
            .peekable();
        arms.peek().is_some()
            && arms.all(|(i, _)| {
                inference.returns.get(i).is_some_and(|set| set & !crate::infer::FAIL == 0)
            })
    }

    /// Does `e` evaluate to a ty value (a construction or a ty-returning call)?
    fn produces_ty(&self, ty: &str, e: &Expr) -> bool {
        match e {
            Expr::App { head, args, .. } => match head.as_ref() {
                Expr::Ident(name, _, _) => {
                    name == ty
                        || (name == "if"
                            && args.len() == 3
                            && (self.produces_ty(ty, &args[1]) || self.produces_ty(ty, &args[2])))
                        || self.returns_ty.contains(&(name.as_str(), args.len()))
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// No member of the callee group binds parameter `i` to a *generic* name.
    /// A ty value flowing here is therefore either destructured by a `(ty ...)`
    /// arm or simply not matched by an arm meant for some other shape (a failure
    /// arm, a literal, a different constructor) — never captured as a variable
    /// it could leak through. Var/Annotated/Keyed bind generically and escape.
    fn callee_destructures(&self, _ty: &str, name: &str, arity: usize, i: usize) -> bool {
        let members: Vec<&FnDecl> =
            self.program.fns.iter().filter(|f| f.name == name && f.params.len() == arity).collect();
        !members.is_empty()
            && members.iter().all(|f| match f.params.get(i) {
                Some(Pattern::Var(..))
                | Some(Pattern::Annotated { .. })
                | Some(Pattern::Keyed { .. })
                | None => false,
                Some(_) => true,
            })
    }

    fn pattern_binds_ty(&self, ty: &str, p: &Pattern) -> bool {
        // A ty destructure is fine; anything that could bind a ty value to a
        // generic name is not. Since ty is only ever produced by our tracked
        // expressions, a Var/Annotated is only dangerous if it *could* hold ty
        // — which we forbid by treating any Var-bound ty flow as escape upstream.
        matches!(p, Pattern::Annotated { ty: pty, .. } if pty == ty)
    }

    fn expr_mentions_ty(&self, ty: &str, e: &Expr) -> bool {
        // Conservative: ty appears anywhere in this (non-tail) expression.
        match e {
            Expr::Ident(name, _, _) | Expr::Partial(name, _) => name == ty,
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => stmts.iter().any(|st| match st {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    self.expr_mentions_ty(ty, expr)
                }
            }),
            Expr::Field { base, .. } => self.expr_mentions_ty(ty, base),
            Expr::Upcast { expr, .. } => self.expr_mentions_ty(ty, expr),
            Expr::App { head, args, .. } => {
                self.expr_mentions_ty(ty, head) || args.iter().any(|a| self.expr_mentions_ty(ty, a))
            }
            Expr::Index { base, index, .. } => {
                self.expr_mentions_ty(ty, base) || self.expr_mentions_ty(ty, index)
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                self.expr_mentions_ty(ty, lhs) || self.expr_mentions_ty(ty, rhs)
            }
            Expr::Guard { cond, early, rest, .. } => {
                self.expr_mentions_ty(ty, cond)
                    || self.expr_mentions_ty(ty, early)
                    || rest.iter().any(|s| {
                        let e = match s {
                            Stmt::Bind { expr, .. }
                            | Stmt::Expr(expr)
                            | Stmt::Set { value: expr, .. } => expr,
                        };
                        self.expr_mentions_ty(ty, e)
                    })
            }
            Expr::Lambda { body, .. } => self.expr_mentions_ty(ty, body),
            Expr::List(items, _) => items.iter().any(|x| self.expr_mentions_ty(ty, x)),
            Expr::MapLit(pairs, _) => pairs
                .iter()
                .any(|(k, v)| self.expr_mentions_ty(ty, k) || self.expr_mentions_ty(ty, v)),
            Expr::Str(parts, _) => parts.iter().any(|p| match p {
                crate::ast::TemplatePart::Interp(x) => self.expr_mentions_ty(ty, x),
                crate::ast::TemplatePart::Lit(_) => false,
            }),
            Expr::Int(..) | Expr::Float(..) | Expr::Hole(..) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::register_returnable;
    use std::path::Path;

    #[test]
    fn json_parsed_returnable_error_types_not() {
        let program = crate::compile_module(Path::new("lib/json"), false).unwrap();
        let r = register_returnable(&program, &crate::infer::infer(&program));
        assert!(r.contains("parsed"), "_parsed should be register-returnable, got {r:?}");
        assert!(!r.contains("defect"), "defect escapes via err-wrapping, got {r:?}");
        assert!(!r.contains("parse_failure"), "parse_failure escapes via err-wrapping, got {r:?}");
    }

    #[test]
    fn record_in_a_list_escapes() {
        let src = "type pt\n  x\n  y\n\nmain = print \"{length [(mk 1 2) (mk 3 4)]}\"\n\nfn mk a b\n  pt a b\n";
        let program = crate::compile("test.kso", src, false).unwrap();
        assert!(
            !register_returnable(&program, &crate::infer::infer(&program)).contains("pt"),
            "pt escapes into a list"
        );
    }

    #[test]
    fn construct_then_destructure_is_returnable() {
        let src = "type pair\n  a\n  b\n\nfn add (pair x y)\n  x + y\n\nmain = print \"{add (mk 5)}\"\n\nfn mk n\n  pair n n\n";
        let program = crate::compile("test.kso", src, false).unwrap();
        assert!(
            register_returnable(&program, &crate::infer::infer(&program)).contains("pair"),
            "pair is construct-then-destructure"
        );
    }
}
