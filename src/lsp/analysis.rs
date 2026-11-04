//! What an editor asks of one buffer: the diagnostics `kanso check` prints,
//! and for each name in the text, the type the compiler worked with there and
//! where the name was declared.
//!
//! Everything here reads the program the check compiled and the inference the
//! engines dispatch on. Two answers are joined for a parameter:
//!
//! - **What it accepts**: the narrowest set every use of the parameter can
//!   take. A parameter handed to a call can only be what some arm of that
//!   group takes at that position, so the call contributes the union over the
//!   arms it can reach, and the parameter's own uses are intersected. An arm
//!   that binds the position by name hands on what that name accepts in turn,
//!   so the answer follows a value through as many calls as it passes. With
//!   `fn bar s:string` and `fn bar n:int`, `fn foo x = bar x` accepts
//!   `string | int`.
//! - **What it was handed**: `infer::Inference::param`, the set the whole
//!   program's calls widened it to. This is what the engines dispatch on.
//!
//! A use the walk does not read (an operator's operand, a field of a lambda's
//! closure) constrains nothing, so the accepted set can only be wider than the
//! truth, never narrower.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::diag::Span;
use crate::hash::Map as HashMap;
use crate::infer::{self, Inference, Set};

/// A diagnostic, at a 0-based line and character column.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub line: u32,
    pub col: u32,
    pub width: u32,
    pub kind: String,
    pub message: String,
    /// An error refuses the program; a warning (an advisory) does not.
    pub error: bool,
}

/// A place a declaration lives: a file and a 0-based line and column.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub width: u32,
}

/// One name in the text, 0-based, with its columns counted in characters.
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub hover: String,
    pub defs: Vec<Place>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    pub problems: Vec<Problem>,
    /// Sorted by line, then column, and never overlapping.
    pub spots: Vec<Spot>,
}

impl Analysis {
    /// The spot under a 0-based line and character column.
    pub fn spot_at(&self, line: u32, col: u32) -> Option<&Spot> {
        self.spots.iter().find(|s| s.line == line && s.start <= col && col < s.end)
    }
}

/// Analyze one buffer under the name `file`, compiled the way `kanso check`
/// compiles that file.
pub fn analyze(file: &str, source: &str) -> Analysis {
    match crate::compile_source("check", file, source) {
        Ok(program) => {
            let mut problems: Vec<Problem> = crate::advisory::door_advisories(&program)
                .into_iter()
                .map(|line| advisory(&line))
                .collect();
            problems.sort_by_key(|p| (p.line, p.col));
            Analysis { problems, spots: spots(&program, file, source) }
        }
        Err(rendered) => {
            Analysis { problems: problems(&rendered, file, source), spots: Vec::new() }
        }
    }
}

fn advisory(line: &str) -> Problem {
    let (kind, message) = match line.strip_prefix("advisory[").and_then(|r| r.split_once("]: ")) {
        Some((kind, message)) => (kind.to_string(), message.to_string()),
        None => ("advisory".to_string(), line.to_string()),
    };
    Problem { line: 0, col: 0, width: 0, kind, message, error: false }
}

/// The diagnostics in what the check rendered. Reading the rendering rather
/// than the list behind it means an editor shows the same words `kanso check`
/// printed, from every compile route, with no second path to keep in step.
///
/// The rendering is `error[kind]: message`, any continuation lines, then
/// `  --> file:line:col` and the quoted line with its caret. A refusal with no
/// place (`error: ...`) sits on the first line. A diagnostic about another
/// file sits there too, with the place in its message, since the editor has
/// that file's text only when the user opens it.
pub fn problems(rendered: &str, file: &str, source: &str) -> Vec<Problem> {
    let mut out: Vec<Problem> = Vec::new();
    let mut open = false;
    for line in rendered.lines() {
        if let Some(rest) = line.strip_prefix("error") {
            let (kind, message) = match rest.strip_prefix('[').and_then(|r| r.split_once("]: ")) {
                Some((kind, message)) => (kind.to_string(), message.to_string()),
                None => (String::new(), rest.trim_start_matches(':').trim_start().to_string()),
            };
            out.push(Problem { line: 0, col: 0, width: 0, kind, message, error: true });
            open = true;
            continue;
        }
        let Some(last) = out.last_mut() else { continue };
        if let Some(place) = line.strip_prefix("  --> ") {
            open = false;
            let mut parts = place.rsplitn(3, ':');
            let col = parts.next().and_then(|c| c.parse::<u32>().ok());
            let row = parts.next().and_then(|l| l.parse::<u32>().ok());
            let path = parts.next().unwrap_or("");
            match (row, col) {
                (Some(row), Some(col)) if path == file => {
                    last.line = row.saturating_sub(1);
                    last.col = col.saturating_sub(1);
                    last.width = word_width(source, last.line, last.col);
                }
                _ => last.message = format!("{place}: {}", last.message),
            }
            continue;
        }
        if open {
            last.message.push('\n');
            last.message.push_str(line);
        }
    }
    out
}

/// How many characters the token at a 0-based place spans: the name there,
/// or one character for anything else.
fn word_width(source: &str, line: u32, col: u32) -> u32 {
    let Some(text) = source.lines().nth(line as usize) else { return 0 };
    let n = text.chars().skip(col as usize).take_while(|c| is_word(*c)).count() as u32;
    match n {
        0 => u32::from(text.chars().nth(col as usize).is_some()),
        n => n,
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '/' | '!' | '?')
}

/// What a parameter accepts, as a tag set and, when records are among the
/// tags, which records. `recs: None` with the REC bit is any record.
#[derive(Clone, Debug, PartialEq)]
struct Want {
    set: Set,
    recs: Option<Vec<String>>,
}

impl Want {
    fn top() -> Want {
        Want { set: infer::TOP, recs: None }
    }

    fn none() -> Want {
        Want { set: 0, recs: Some(Vec::new()) }
    }

    fn of(set: Set) -> Want {
        Want { set, recs: None }
    }

    fn record(name: &str) -> Want {
        Want { set: infer::REC, recs: Some(vec![name.to_string()]) }
    }

    fn meet(&self, other: &Want) -> Want {
        let recs = match (&self.recs, &other.recs) {
            (None, r) | (r, None) => r.clone(),
            (Some(a), Some(b)) => Some(a.iter().filter(|n| b.contains(n)).cloned().collect()),
        };
        let mut set = self.set & other.set;
        if recs.as_ref().is_some_and(Vec::is_empty) {
            set &= !infer::REC;
        }
        Want { set, recs }
    }

    fn join(&self, other: &Want) -> Want {
        fn mine(w: &Want) -> Option<Option<&Vec<String>>> {
            (w.set & infer::REC != 0).then_some(w.recs.as_ref())
        }
        let recs = match (mine(self), mine(other)) {
            (Some(None), _) | (_, Some(None)) => None,
            (Some(Some(a)), Some(Some(b))) => {
                let mut both: Vec<String> = a.iter().chain(b).cloned().collect();
                both.sort();
                both.dedup();
                Some(both)
            }
            (Some(Some(a)), None) | (None, Some(Some(a))) => Some(a.clone()),
            (None, None) => Some(Vec::new()),
        };
        Want { set: self.set | other.set, recs }
    }
}

/// What a pattern lets through, by itself.
fn accepts(pat: &Pattern, typesets: &HashMap<String, Vec<String>>) -> Want {
    match pat {
        Pattern::Var(..) | Pattern::Wildcard(_) => Want::top(),
        Pattern::Annotated { ty, .. } => annotated(ty, typesets),
        Pattern::IntLit(..) => Want::of(infer::ANY_INT),
        Pattern::StrLit(..) => Want::of(infer::STR),
        Pattern::Nullary(name, _) => match name.as_str() {
            "none" => Want::of(infer::NONE),
            "true" => Want::of(infer::TRUE),
            "false" => Want::of(infer::FALSE),
            "done" => Want::of(infer::DONE),
            other => Want::record(other),
        },
        Pattern::Ctor { ty, .. } if ty == "err" => Want::of(infer::UNBOXED),
        Pattern::Ctor { ty, .. } => Want::record(ty),
        Pattern::Keyed { .. } => Want::of(infer::REC | infer::MAP),
    }
}

fn annotated(ty: &str, typesets: &HashMap<String, Vec<String>>) -> Want {
    if let Some(members) = typesets.get(ty) {
        return members.iter().fold(Want::none(), |acc, m| acc.join(&annotated(m, typesets)));
    }
    match infer::annotated_set(ty) {
        infer::REC => Want::record(ty),
        set => Want::of(set),
    }
}

/// The name a pattern binds the whole argument to, if it binds one.
fn binder(pat: &Pattern) -> Option<&str> {
    match pat {
        Pattern::Var(n, _) => Some(n.as_str()),
        Pattern::Annotated { name, .. } => Some(name.as_str()),
        Pattern::Ctor { whole: Some(w), .. } => Some(w.0.as_str()),
        _ => None,
    }
}

fn pattern_span(pat: &Pattern) -> Option<Span> {
    match pat {
        Pattern::Var(_, s) | Pattern::Annotated { span: s, .. } => Some(*s),
        Pattern::Ctor { whole: Some(w), .. } => Some(w.1),
        _ => None,
    }
}

/// One use of a parameter as a call's argument: the group called, the
/// position, and the arguments, which decide the arms the call can reach.
struct Use<'p> {
    name: &'p str,
    at: usize,
    args: &'p [Expr],
}

/// Every place a declaration's body hands one of its parameters, as it is,
/// to a call. A lambda that binds the same name hides it below.
fn uses_of<'p>(decl: &'p FnDecl, name: &str) -> Vec<Use<'p>> {
    fn walk<'p>(e: &'p Expr, name: &str, out: &mut Vec<Use<'p>>) {
        match e {
            Expr::Lambda { params, .. } if params.iter().any(|(p, _)| p == name) => return,
            Expr::App { piped: false, head, args, .. } => {
                if let Expr::Ident(callee, _, _) = head.as_ref() {
                    for (at, arg) in args.iter().enumerate() {
                        if matches!(arg, Expr::Ident(n, _, _) if n == name) {
                            out.push(Use { name: callee.as_str(), at, args });
                        }
                    }
                }
            }
            _ => {}
        }
        crate::for_each_child(e, |c| walk(c, name, out));
    }
    let mut out = Vec::new();
    for stmt in &decl.body {
        match stmt {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => walk(expr, name, &mut out),
        }
    }
    out
}

struct Facts<'p> {
    program: &'p Program,
    inference: Inference,
    groups: HashMap<(&'p str, usize), Vec<usize>>,
    consts: infer::Consts<'p>,
    typesets: HashMap<String, Vec<String>>,
    /// Per declaration, per parameter: what it accepts.
    wants: Vec<Vec<Want>>,
}

impl<'p> Facts<'p> {
    fn new(program: &'p Program) -> Facts<'p> {
        let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::default();
        for (i, d) in program.fns.iter().enumerate() {
            groups.entry((d.name.as_str(), d.params.len())).or_default().push(i);
        }
        let typesets = program.flat_typesets();
        let mut facts = Facts {
            program,
            inference: infer::infer(program),
            groups,
            consts: infer::literal_consts(program),
            wants: Vec::new(),
            typesets,
        };
        facts.settle_wants();
        facts
    }

    /// The arms a call with these arguments can reach.
    fn reached(&self, name: &str, args: &[Expr]) -> Vec<usize> {
        self.groups.get(&(name, args.len())).map_or(Vec::new(), |arms| {
            arms.iter()
                .copied()
                .filter(|&i| infer::arm_can_run(&self.program.fns[i].params, args, &self.consts))
                .collect()
        })
    }

    /// The greatest fixpoint of what each parameter accepts: every parameter
    /// starts out accepting anything, and each round narrows a parameter to
    /// what its own pattern and every use of it allow. The sets only shrink
    /// and there are finitely many, so it ends.
    fn settle_wants(&mut self) {
        let fns = &self.program.fns;
        self.wants = fns
            .iter()
            .map(|d| d.params.iter().map(|p| accepts(p, &self.typesets)).collect())
            .collect();
        let uses: Vec<Vec<Vec<Use<'p>>>> = fns
            .iter()
            .map(|d| {
                d.params.iter().map(|p| binder(p).map_or(Vec::new(), |n| uses_of(d, n))).collect()
            })
            .collect();
        let reached: Vec<Vec<Vec<Vec<usize>>>> = uses
            .iter()
            .map(|per_param| {
                per_param
                    .iter()
                    .map(|us| us.iter().map(|u| self.reached(u.name, u.args)).collect())
                    .collect()
            })
            .collect();
        loop {
            let mut changed = false;
            for (i, d) in fns.iter().enumerate() {
                for p in 0..d.params.len() {
                    let mut want = accepts(&d.params[p], &self.typesets);
                    for (u, arms) in uses[i][p].iter().zip(&reached[i][p]) {
                        if arms.is_empty() {
                            continue;
                        }
                        let took = arms
                            .iter()
                            .fold(Want::none(), |acc, &j| acc.join(&self.wants[j][u.at]));
                        want = want.meet(&took);
                    }
                    if want != self.wants[i][p] {
                        self.wants[i][p] = want;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// The set one of a declaration's parameters was handed, as inference
    /// widened it, in the record vocabulary of what it accepts.
    fn handed(&self, decl: usize, at: usize) -> Want {
        let set = self.inference.param(decl, at);
        let want = &self.wants[decl][at];
        Want { set, recs: want.recs.clone().filter(|_| set & infer::REC != 0) }
    }

    /// A declaration's head as it was compiled: each parameter with what it
    /// accepts, then what the arm answers.
    fn signature(&self, i: usize) -> String {
        let d = &self.program.fns[i];
        let mut out = format!("fn {}", crate::ast::spoken(&d.name));
        for (p, pat) in d.params.iter().enumerate() {
            out.push(' ');
            out.push_str(&param_text(pat, &self.wants[i][p]));
        }
        let answer = self.inference.returns[i];
        if answer != 0 {
            out.push_str(" → ");
            out.push_str(&type_text(&Want::of(answer)));
        }
        out
    }

    fn param_hover(&self, i: usize, p: usize) -> String {
        let pat = &self.program.fns[i].params[p];
        let name = binder(pat).unwrap_or("_");
        let want = &self.wants[i][p];
        let mut out = format!("{name}: {}", type_text(want));
        let handed = self.handed(i, p);
        let (said, told) = (type_text(want), type_text(&handed));
        if handed.set != 0 && told != said {
            out.push_str(&format!("\nhanded: {told}"));
        }
        out
    }

    /// The set an expression answers, where the inference already holds it:
    /// a literal, a parameter, or a call (the union of what its reachable
    /// arms answer).
    fn answer(
        &self,
        e: &Expr,
        params: &HashMap<&str, (usize, usize)>,
        locals: &HashMap<&str, Set>,
    ) -> Set {
        match e {
            Expr::Int(..) => infer::ANY_INT,
            Expr::Float(..) => infer::FLOAT,
            Expr::Str(..) => infer::STR,
            Expr::List(..) => infer::LIST,
            Expr::MapLit(..) => infer::MAP,
            Expr::Lambda { .. } => infer::FN,
            Expr::Ident(n, _, _) => match (params.get(n.as_str()), locals.get(n.as_str())) {
                (_, Some(set)) => *set,
                (Some(&(i, p)), None) => self.inference.param(i, p),
                (None, None) => self.call_answer(n, &[]),
            },
            Expr::App { piped: false, head, args, .. } => match head.as_ref() {
                Expr::Ident(n, _, _)
                    if !params.contains_key(n.as_str()) && !locals.contains_key(n.as_str()) =>
                {
                    self.call_answer(n, args)
                }
                _ => 0,
            },
            Expr::BinOp { op, .. } if matches!(*op, "==" | "!=" | "<" | ">" | "<=" | ">=") => {
                infer::BOOL
            }
            _ => 0,
        }
    }

    fn call_answer(&self, name: &str, args: &[Expr]) -> Set {
        self.reached(name, args).iter().fold(0, |acc, &i| acc | self.inference.returns[i])
    }
}

/// A parameter as a reader would write it: a name with what it accepts, or
/// the pattern itself.
fn param_text(pat: &Pattern, want: &Want) -> String {
    match pat {
        Pattern::Var(n, _) if want.set == infer::TOP => n.to_string(),
        Pattern::Var(n, _) | Pattern::Annotated { name: n, .. } => {
            let ty = type_text(want);
            match ty.contains(' ') {
                true => format!("{n}:({ty})"),
                false => format!("{n}:{ty}"),
            }
        }
        Pattern::Wildcard(_) => "_".to_string(),
        Pattern::IntLit(n, _) => n.to_string(),
        Pattern::StrLit(s, _) => format!("{s:?}"),
        Pattern::Nullary(n, _) => n.to_string(),
        Pattern::Ctor { ty, fields, whole } => {
            let mut out = String::new();
            if let Some(w) = whole {
                out.push_str(&format!("{}@", w.0));
            }
            out.push('(');
            out.push_str(&crate::ast::spoken(ty));
            for f in fields {
                out.push(' ');
                out.push_str(&param_text(f, &Want::top()));
            }
            out.push(')');
            out
        }
        Pattern::Keyed { entries, .. } => {
            let names: Vec<&str> = entries.iter().map(|e| e.bind_name.as_str()).collect();
            format!("{{{}}}", names.join(" "))
        }
    }
}

/// A set in the words of the language's typesets: `string | int`, members
/// joined by a bar, the way appendix B writes a builtin that takes either.
fn type_text(want: &Want) -> String {
    use crate::infer::*;
    let set = want.set & !THUNK;
    if set & TOP & !(ERR | UNBOXED) == TOP & !(ERR | UNBOXED) && want.recs.is_none() {
        return "anything".to_string();
    }
    let mut words: Vec<String> = Vec::new();
    let mut word = |w: &str| words.push(w.to_string());
    if set & ANY_INT != 0 {
        word("int");
    }
    if set & FLOAT != 0 {
        word("float64");
    }
    if set & STR != 0 {
        word("string");
    }
    match set & BOOL {
        BOOL => word("bool"),
        TRUE => word("true"),
        FALSE => word("false"),
        _ => {}
    }
    if set & BYTES != 0 {
        word("bytes");
    }
    if set & LIST != 0 {
        word("list");
    }
    if set & MAP != 0 {
        word("map");
    }
    if set & REC != 0 {
        match &want.recs {
            Some(names) if !names.is_empty() => {
                for n in names {
                    words.push(crate::ast::spoken(n).into_owned());
                }
            }
            _ => words.push("record".to_string()),
        }
    }
    if set & FN != 0 {
        words.push("fn".to_string());
    }
    if set & DESC != 0 {
        words.push("effect".to_string());
    }
    if set & DONE != 0 {
        words.push("done".to_string());
    }
    if set & NONE != 0 {
        words.push("none".to_string());
    }
    if set & (ERR | UNBOXED) != 0 {
        words.push("err".to_string());
    }
    match words.is_empty() {
        true => "nothing".to_string(),
        false => words.join(" | "),
    }
}

/// Every name in `file`'s own declarations that the editor can say something
/// about.
fn spots(program: &Program, file: &str, source: &str) -> Vec<Spot> {
    let facts = Facts::new(program);
    let lines: Vec<&str> = source.lines().collect();
    let mut found: HashMap<(u32, u32), Spot> = HashMap::default();
    let mut put = |span: Span, hover: String, defs: Vec<Place>| {
        if span.line == 0 || span.col == 0 {
            return;
        }
        let (line, start) = (span.line - 1, span.col - 1);
        let Some(text) = lines.get(line as usize) else { return };
        let width = text.chars().skip(start as usize).take_while(|c| is_word(*c)).count() as u32;
        if width == 0 {
            return;
        }
        // The first declaration to claim a place is the one the author
        // wrote; a twin the compiler made keeps its original's spans.
        found.entry((line, start)).or_insert(Spot { line, start, end: start + width, hover, defs });
    };
    let place = |d: &FnDecl, span: Span| Place {
        file: d.file.to_string(),
        line: span.line.saturating_sub(1),
        col: span.col.saturating_sub(1),
        width: 0,
    };
    for (i, d) in program.fns.iter().enumerate() {
        if d.synthetic || &*d.file != file {
            continue;
        }
        let mut params: HashMap<&str, (usize, usize)> = HashMap::default();
        for (p, pat) in d.params.iter().enumerate() {
            if let (Some(n), Some(span)) = (binder(pat), pattern_span(pat)) {
                params.insert(n, (i, p));
                put(span, facts.param_hover(i, p), vec![place(d, span)]);
            }
        }
        if d.name != crate::ast::ENTRY && !d.name.starts_with(char::is_uppercase) {
            put(d.span, facts.signature(i), arms_of(&facts, &d.name, d.params.len()));
        }
        let mut locals: HashMap<&str, Set> = HashMap::default();
        let mut local_at: HashMap<&str, Span> = HashMap::default();
        for stmt in &d.body {
            let expr = match stmt {
                Stmt::Bind { pattern, expr } => {
                    if let (Pattern::Var(n, span), set) =
                        (pattern, facts.answer(expr, &params, &locals))
                    {
                        locals.insert(n.as_str(), set);
                        local_at.insert(n.as_str(), *span);
                        if set != 0 {
                            put(
                                *span,
                                format!("{n}: {}", type_text(&Want::of(set))),
                                vec![place(d, *span)],
                            );
                        }
                    }
                    expr
                }
                Stmt::Expr(expr) => expr,
            };
            names_in(expr, &mut |e| match e {
                Expr::App { piped: false, head, args, .. } => {
                    if let Expr::Ident(n, span, _) = head.as_ref() {
                        if !params.contains_key(n.as_str()) && !locals.contains_key(n.as_str()) {
                            call_spot(&facts, n, *span, args, &mut put);
                        }
                    }
                }
                Expr::Ident(n, span, _) => {
                    if let Some(&(i, p)) = params.get(n.as_str()) {
                        let at = pattern_span(&program.fns[i].params[p]).unwrap_or(*span);
                        put(*span, facts.param_hover(i, p), vec![place(d, at)]);
                    } else if let (Some(set), Some(at)) =
                        (locals.get(n.as_str()), local_at.get(n.as_str()))
                    {
                        if *set != 0 {
                            put(
                                *span,
                                format!("{n}: {}", type_text(&Want::of(*set))),
                                vec![place(d, *at)],
                            );
                        }
                    } else if facts.groups.contains_key(&(n.as_str(), 0)) {
                        call_spot(&facts, n, *span, &[], &mut put);
                    }
                }
                _ => {}
            });
        }
    }
    let mut out: Vec<Spot> = found.into_values().collect();
    out.sort_by_key(|s| (s.line, s.start));
    out
}

/// The hover for a call's head: each arm the call can reach, as compiled.
fn call_spot(
    facts: &Facts<'_>,
    name: &str,
    span: Span,
    args: &[Expr],
    put: &mut impl FnMut(Span, String, Vec<Place>),
) {
    let reached = facts.reached(name, args);
    if reached.is_empty() {
        return;
    }
    let lines: Vec<String> = reached.iter().map(|&i| facts.signature(i)).collect();
    put(span, lines.join("\n"), arms_of(facts, name, args.len()));
}

fn arms_of(facts: &Facts<'_>, name: &str, arity: usize) -> Vec<Place> {
    let Some(arms) = facts.groups.get(&(name, arity)) else { return Vec::new() };
    arms.iter()
        .map(|&i| &facts.program.fns[i])
        .filter(|d| !d.synthetic && !d.name.starts_with(char::is_uppercase))
        .map(|d| Place {
            file: d.file.to_string(),
            line: d.span.line.saturating_sub(1),
            col: d.span.col.saturating_sub(1),
            width: 0,
        })
        .fold(Vec::new(), |mut acc, p| {
            if !acc.contains(&p) {
                acc.push(p);
            }
            acc
        })
}

/// Every expression under `e`, outermost first.
fn names_in<'p>(e: &'p Expr, f: &mut impl FnMut(&'p Expr)) {
    f(e);
    crate::for_each_child(e, |c| names_in(c, f));
}
