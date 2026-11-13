//! A spec written as blocks (#2495). A `*_test.kso` file whose top level holds
//! `it` or `context` is a spec: the file is the `describe`, its subject is the
//! module beside it, and the file is lowered here, line by line, onto the
//! values `std/spec` already runs. Nothing downstream learns a new word.
//!
//! The lowering works on the lexer's lines, so every token the author wrote
//! keeps its span, and a diagnostic about it lands on the line they wrote.
//!
//! What a spec file holds, at the top and inside a `context`:
//!
//! - givens, `name = value`, which play the part of RSpec's `let`. An example
//!   evaluates the givens it reads and no others, fresh, so a given is as lazy
//!   as `let`. A context may restate a file's given, and everything evaluated
//!   for an example in that context sees the restated one, the file's own
//!   givens included. A restatement may read the value it replaces.
//! - hooks, `around`, `before` and `after`, as `std/spec` has them. An
//!   `around` may name what it hands on, `around visit = …`, and that name is
//!   then readable by givens and bodies.
//! - `it "name"` followed by a body, and at the top only, `context "name"`
//!   holding the same things. A context inside a context is refused.
//!
//! Each example becomes a function holding the givens it resolved, in an
//! order where each comes after what it reads, then its body. Each bare line
//! of the body is a check, and `every` answers the first that does not hold.

use crate::diag::{Diagnostic, Span};
use crate::lexer::{Lexed, Line, StrPart, Tok};
use crate::name::Name;

/// The `std/spec` names a lowered spec calls without its author writing them.
/// The shipped library is read only as far as the program's words reach, and
/// these are words the source never holds.
pub const WORDS: &str = "describe it context around before after every";

/// What the generated names start with. An author's own name never needs to.
const MINE: &str = "spec_";

/// Whether a lexed file is a spec written as blocks: some line at the top
/// level is `it "…"` or `context "…"`.
pub fn is_spec(lexed: &Lexed) -> bool {
    lexed.lines.iter().any(|l| l.indent == 0 && opens(l).is_some())
}

#[derive(Clone, Copy, PartialEq)]
enum Opener {
    It,
    Context,
}

fn opens(line: &Line) -> Option<(Opener, String, Span)> {
    let [(Tok::Ident(word), span, _), (Tok::Str(parts), _, _)] = line.tokens.as_slice() else {
        return None;
    };
    let kind = match word.as_str() {
        "it" => Opener::It,
        "context" => Opener::Context,
        _ => return None,
    };
    let [StrPart::Lit(text)] = parts.as_slice() else { return None };
    Some((kind, text.clone(), *span))
}

/// A statement: its first line and the lines continuing it, which are
/// indented further.
#[derive(Clone)]
struct Stmt {
    lines: Vec<Line>,
}

impl Stmt {
    fn head(&self) -> &Line {
        &self.lines[0]
    }
    fn span(&self) -> Span {
        self.head().tokens[0].1
    }
}

#[derive(Clone)]
struct Given {
    name: String,
    span: Span,
    /// The tokens after `=` on the first line, and the lines continuing it.
    stmt: Stmt,
    reads: Vec<String>,
    /// 0 for the file, 1 for a context.
    level: usize,
    used: std::cell::Cell<bool>,
}

struct Hook {
    word: String,
    named: Option<(String, Span)>,
    stmt: Stmt,
}

struct Example {
    name: String,
    span: Span,
    body: Vec<Stmt>,
    reads: Vec<String>,
}

enum Item {
    Hook(usize),
    Example(Example),
    Context(Group),
}

struct Group {
    name: String,
    span: Span,
    givens: Vec<Given>,
    hooks: Vec<Hook>,
    items: Vec<Item>,
}

/// Groups `lines` into statements at the indent of the first.
fn statements(lines: &[Line]) -> Vec<Stmt> {
    let Some(first) = lines.first() else { return Vec::new() };
    let base = first.indent;
    let mut out: Vec<Stmt> = Vec::new();
    for line in lines {
        match (line.indent <= base, out.last_mut()) {
            (false, Some(last)) => last.lines.push(line.clone()),
            _ => out.push(Stmt { lines: vec![line.clone()] }),
        }
    }
    out
}

/// The plain names a run of tokens reads, interpolations included. A
/// qualified name reads a module, never a given.
fn reads_of(tokens: &[(Tok, Span, u32)], out: &mut Vec<String>) {
    for (tok, _, _) in tokens {
        match tok {
            Tok::Ident(n) if !n.as_str().contains('/') => out.push(n.to_string()),
            Tok::Str(parts) => {
                for part in parts {
                    if let StrPart::Interp(inner) = part {
                        reads_of(inner, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn reads_of_stmt(stmt: &Stmt) -> Vec<String> {
    let mut out = Vec::new();
    for line in &stmt.lines {
        reads_of(&line.tokens, &mut out);
    }
    out
}

/// `stmt` with its first `skip` tokens dropped from the head line.
fn tail(stmt: &Stmt, skip: usize) -> Stmt {
    let mut lines = stmt.lines.clone();
    lines[0].tokens.drain(..skip);
    Stmt { lines }
}

fn binding(stmt: &Stmt) -> Option<(String, Span)> {
    match stmt.head().tokens.as_slice() {
        [(Tok::Ident(n), span, _), (Tok::Bind, _, _), _, ..] if !n.as_str().contains('/') => {
            Some((n.to_string(), *span))
        }
        _ => None,
    }
}

fn hook_of(stmt: &Stmt) -> Option<Hook> {
    let (Tok::Ident(word), _, _) = &stmt.head().tokens[0] else { return None };
    let word = word.as_str();
    if !matches!(word, "around" | "before" | "after") || stmt.head().tokens.len() < 2 {
        return None;
    }
    let named = match stmt.head().tokens.as_slice() {
        [_, (Tok::Ident(n), span, _), (Tok::Bind, _, _), _, ..] if word == "around" => {
            Some((n.to_string(), *span))
        }
        _ => None,
    };
    let skip = if named.is_some() { 3 } else { 1 };
    Some(Hook { word: word.to_string(), named, stmt: tail(stmt, skip) })
}

/// Reads the items of a group: the file (`level` 0) or a context (1).
fn group_of(
    name: String,
    span: Span,
    stmts: &[Stmt],
    level: usize,
    all_hooks: &mut Vec<Hook>,
    diags: &mut Vec<Diagnostic>,
) -> Group {
    let mut group = Group { name, span, givens: Vec::new(), hooks: Vec::new(), items: Vec::new() };
    for stmt in stmts {
        let head = stmt.head();
        if let Some((kind, text, span)) = opens(head) {
            let body = &stmt.lines[1..];
            match kind {
                Opener::Context if level > 0 => diags.push(Diagnostic::new(
                    "syntax",
                    "a spec nests one level: a context holds givens, hooks and `it`, and \
                     this context belongs at the top of the file as one of its own"
                        .to_string(),
                    span,
                )),
                Opener::Context => {
                    let inner = statements(body);
                    let g = group_of(text, span, &inner, level + 1, all_hooks, diags);
                    group.items.push(Item::Context(g));
                }
                Opener::It if body.is_empty() => diags.push(Diagnostic::new(
                    "syntax",
                    format!("`it \"{text}\"` needs a body: the lines under it, indented"),
                    span,
                )),
                Opener::It => {
                    let body = statements(body);
                    let mut reads = Vec::new();
                    for s in &body {
                        reads.extend(reads_of_stmt(s));
                    }
                    if body.iter().all(|s| binding(s).is_some()) {
                        diags.push(Diagnostic::new(
                            "syntax",
                            format!(
                                "`it \"{text}\"` checks nothing: end it with a line that \
                                 answers true when the example holds, such as an `expect`"
                            ),
                            span,
                        ));
                    }
                    group.items.push(Item::Example(Example { name: text, span, body, reads }));
                }
            }
            continue;
        }
        if let Some(hook) = hook_of(stmt) {
            group.items.push(Item::Hook(all_hooks.len()));
            group.hooks.push(Hook {
                word: hook.word.clone(),
                named: hook.named.clone(),
                stmt: hook.stmt.clone(),
            });
            all_hooks.push(hook);
            continue;
        }
        if let Some((name, span)) = binding(stmt) {
            let rhs = tail(stmt, 2);
            let reads = reads_of_stmt(&rhs);
            group.givens.push(Given {
                name,
                span,
                stmt: rhs,
                reads,
                level,
                used: std::cell::Cell::new(false),
            });
            continue;
        }
        diags.push(Diagnostic::new(
            "syntax",
            "a spec holds givens (`name = value`), hooks (`around`, `before`, `after`), \
             `it` and `context`"
                .to_string(),
            stmt.span(),
        ));
    }
    group
}

fn tok(t: Tok, span: Span) -> (Tok, Span, u32) {
    (t, span, span.col)
}

fn ident(name: &str, span: Span) -> (Tok, Span, u32) {
    tok(Tok::Ident(Name::from(name)), span)
}

fn string(text: &str, span: Span) -> (Tok, Span, u32) {
    tok(Tok::Str(vec![StrPart::Lit(text.to_string())]), span)
}

fn line(number: usize, indent: usize, tokens: Vec<(Tok, Span, u32)>) -> Line {
    Line { number, indent, tokens }
}

/// Generated tokens laid out as if written with a space between each, so no
/// two read as adjacent (`"stack" [` is a list, `"stack"[` an index). Each
/// keeps its line, which is where a diagnostic about it points.
fn laid(tokens: Vec<(Tok, Span, u32)>, indent: usize) -> Vec<(Tok, Span, u32)> {
    let mut col = indent + 1;
    let mut out = Vec::new();
    for (t, span, _) in tokens {
        let width = match &t {
            Tok::Ident(n) => n.as_str().len(),
            Tok::Str(parts) => match parts.as_slice() {
                [StrPart::Lit(s)] => s.len() + 2,
                _ => 2,
            },
            Tok::LGroup | Tok::RGroup => 0,
            _ => 1,
        };
        out.push((t, Span::at(span.line as usize, col), (col + width) as u32));
        col += width + 1;
    }
    out
}

/// `stmt`'s lines re-indented so its head sits at `indent`, with `prefix`
/// in front of the head's tokens.
fn placed(stmt: &Stmt, indent: usize, prefix: Vec<(Tok, Span, u32)>) -> Vec<Line> {
    let base = stmt.head().indent;
    let mut out = Vec::new();
    for (i, l) in stmt.lines.iter().enumerate() {
        let mut tokens = l.tokens.clone();
        if i == 0 {
            let mut head = prefix.clone();
            head.extend(tokens);
            tokens = head;
        }
        out.push(line(l.number, indent + l.indent.saturating_sub(base), tokens));
    }
    out
}

/// The givens one example resolves, in an order where each follows what it
/// reads. `file` and `ctx` are the two levels; `wanted` is what the body
/// reads. A restatement that reads its own name reads the file's value.
fn resolved<'a>(
    file: &'a [Given],
    ctx: &'a [Given],
    wanted: &[String],
    diags: &mut Vec<Diagnostic>,
) -> Vec<&'a Given> {
    fn find<'a>(
        file: &'a [Given],
        ctx: &'a [Given],
        name: &str,
        level: usize,
    ) -> Option<&'a Given> {
        let in_ctx = ctx.iter().find(|g| g.name == name);
        let in_file = file.iter().find(|g| g.name == name);
        match level {
            0 => in_ctx.or(in_file),
            _ => in_file,
        }
    }
    let mut order: Vec<&Given> = Vec::new();
    let mut visiting: Vec<*const Given> = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn visit<'a>(
        g: &'a Given,
        file: &'a [Given],
        ctx: &'a [Given],
        order: &mut Vec<&'a Given>,
        visiting: &mut Vec<*const Given>,
        diags: &mut Vec<Diagnostic>,
    ) {
        let key = g as *const Given;
        if order.iter().any(|o| std::ptr::eq(*o, g)) {
            return;
        }
        if visiting.contains(&key) {
            diags.push(Diagnostic::new(
                "name",
                format!("`{}` is defined in terms of itself, so it has no value", g.name),
                g.span,
            ));
            return;
        }
        visiting.push(key);
        for r in &g.reads {
            // A restatement reading its own name reads the value it replaces.
            let level = usize::from(g.level == 1 && *r == g.name);
            if let Some(dep) = find(file, ctx, r, level) {
                if !std::ptr::eq(dep, g) {
                    visit(dep, file, ctx, order, visiting, diags);
                }
            }
        }
        visiting.pop();
        g.used.set(true);
        order.push(g);
    }
    for w in wanted {
        if let Some(g) = find(file, ctx, w, 0) {
            visit(g, file, ctx, &mut order, &mut visiting, diags);
        }
    }
    order
}

/// Lowers a spec written as blocks onto `std/spec`. `subject` heads the
/// outline: the module the file describes.
pub fn lower(lexed: &Lexed, subject: &str) -> Result<Lexed, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut imports: Vec<Line> = Vec::new();
    let mut types: Vec<Line> = Vec::new();
    let mut fns: Vec<Line> = Vec::new();
    let mut spec_stmts: Vec<Stmt> = Vec::new();
    for stmt in statements(&lexed.lines) {
        match stmt.head().tokens.first() {
            Some((Tok::KwImport, _, _)) => imports.extend(stmt.lines),
            Some((Tok::KwType, _, _)) => types.extend(stmt.lines),
            Some((Tok::KwFn, _, _)) => fns.extend(stmt.lines),
            Some((Tok::KwPub, _, _)) => match stmt.head().tokens.get(1) {
                Some((Tok::KwType, _, _)) => types.extend(stmt.lines),
                _ => fns.extend(stmt.lines),
            },
            _ => spec_stmts.push(stmt),
        }
    }
    let first = spec_stmts.first().map(|s| s.span()).unwrap_or(Span::at(1, 1));
    let mut hooks = Vec::new();
    let file = group_of(subject.to_string(), first, &spec_stmts, 0, &mut hooks, &mut diags);

    // `std/spec` beside the author's imports, in alphabetical order.
    if !imports.iter().any(|l| matches!(l.tokens.get(1), Some((Tok::Str(p), _, _)) if matches!(p.as_slice(), [StrPart::Lit(s)] if s == "std/spec"))) {
        let at = imports
            .iter()
            .position(|l| match l.tokens.get(1) {
                Some((Tok::Str(p), _, _)) => {
                    matches!(p.as_slice(), [StrPart::Lit(s)] if s.as_str() > "std/spec")
                }
                _ => false,
            })
            .unwrap_or(imports.len());
        let number = imports.get(at).or(imports.last()).map(|l| l.number).unwrap_or(1);
        let span = Span::at(number, 1);
        imports.insert(at, line(number, 0, vec![tok(Tok::KwImport, span), string("std/spec", span)]));
    }

    let mut consts: Vec<Line> = Vec::new();
    let mut made: Vec<Line> = Vec::new();
    // Each hook's expression becomes a constant, so a hook may span lines.
    for (i, hook) in hooks.iter().enumerate() {
        let span = hook.stmt.head().tokens.first().map(|t| t.1).unwrap_or(first);
        let all: Vec<&Given> = file.givens.iter().collect();
        for r in reads_of_stmt(&hook.stmt) {
            if all.iter().any(|g| g.name == r) {
                diags.push(Diagnostic::new(
                    "name",
                    format!(
                        "`{}` reads the given `{r}`, and a hook runs before any given is \
                         evaluated: pass it what it needs from outside the spec",
                        hook.word
                    ),
                    span,
                ));
            }
        }
        let name = format!("{MINE}hook_{}", i + 1);
        consts.extend(placed(&hook.stmt, 0, vec![ident(&name, span), tok(Tok::Bind, span)]));
    }

    let mut counter = 0;
    let mut tree: Vec<(Tok, Span, u32)> = Vec::new();
    let test_name = format!("test_{subject}");
    let open = Span::at(first.line as usize, 1);
    tree.extend([
        ident(&test_name, open),
        tok(Tok::Bind, open),
        ident("describe", open),
        string(subject, open),
        tok(Tok::LBracket, open),
    ]);
    let none: Vec<Given> = Vec::new();
    let file_named = file.hooks.iter().find_map(|h| h.named.clone());
    emit_items(
        &file,
        &file,
        &none,
        file_named.as_ref(),
        &mut counter,
        &mut tree,
        &mut made,
        &mut diags,
    );
    let last = tree.last().map(|t| t.1).unwrap_or(open);
    tree.push(tok(Tok::RBracket, last));
    let tree = vec![line(open.line as usize, 0, laid(tree, 0))];

    // A given no example reads is refused, as any unused binding is. A spec
    // already refused for its shape left examples out, so it is not asked.
    if !diags.is_empty() {
        return Err(diags);
    }
    let mut unread: Vec<&Given> = file.givens.iter().collect();
    for item in &file.items {
        if let Item::Context(c) = item {
            unread.extend(c.givens.iter());
        }
    }
    for g in unread {
        if !g.used.get() {
            diags.push(Diagnostic::new(
                "unused",
                format!("unused given `{}`: no example reads it", g.name),
                g.span,
            ));
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }
    let mut lines = imports;
    lines.extend(types);
    lines.extend(consts);
    lines.extend(tree);
    lines.extend(fns);
    lines.extend(made);
    Ok(Lexed { lines, blank_lines: Vec::new(), header: 0, pruned: true })
}

#[allow(clippy::too_many_arguments)]
fn emit_items(
    file: &Group,
    group: &Group,
    ctx: &[Given],
    named: Option<&(String, Span)>,
    counter: &mut usize,
    tree: &mut Vec<(Tok, Span, u32)>,
    made: &mut Vec<Line>,
    diags: &mut Vec<Diagnostic>,
) {
    let mut hook_index = 0;
    for item in &group.items {
        match item {
            Item::Hook(i) => {
                let hook = &group.hooks[hook_index];
                hook_index += 1;
                let span = hook.stmt.head().tokens.first().map(|t| t.1).unwrap_or(Span::at(1, 1));
                tree.push(tok(Tok::LGroup, span));
                tree.push(ident(&hook.word, span));
                tree.push(ident(&format!("{MINE}hook_{}", i + 1), span));
                tree.push(tok(Tok::RGroup, span));
            }
            Item::Context(c) => {
                let span = c.span;
                tree.extend([
                    tok(Tok::LGroup, span),
                    ident("context", span),
                    string(&c.name, span),
                    tok(Tok::LBracket, span),
                ]);
                let inner_named = c.hooks.iter().find_map(|h| h.named.clone());
                if let (Some(_), Some((_, at))) = (named, &inner_named) {
                    diags.push(Diagnostic::new(
                        "syntax",
                        "an example is handed one value, and an `around` at the top already \
                         names it"
                            .to_string(),
                        *at,
                    ));
                }
                let named_here = inner_named.as_ref().or(named);
                emit_items(file, c, &c.givens, named_here, counter, tree, made, diags);
                let end = tree.last().map(|t| t.1).unwrap_or(span);
                tree.push(tok(Tok::RBracket, end));
                tree.push(tok(Tok::RGroup, end));
            }
            Item::Example(e) => {
                *counter += 1;
                let fname = format!("{MINE}example_{}", counter);
                tree.extend([
                    tok(Tok::LGroup, e.span),
                    ident("it", e.span),
                    string(&e.name, e.span),
                    ident(&fname, e.span),
                    tok(Tok::RGroup, e.span),
                ]);
                let givens = resolved(&file.givens, ctx, &e.reads, diags);
                let param = match named {
                    Some((n, _))
                        if e.reads.contains(n) || givens.iter().any(|g| g.reads.contains(n)) =>
                    {
                        n.clone()
                    }
                    _ => "_".to_string(),
                };
                let param_tok = match param.as_str() {
                    "_" => tok(Tok::Underscore, e.span),
                    p => ident(p, e.span),
                };
                made.push(line(
                    e.span.line as usize,
                    0,
                    laid(vec![tok(Tok::KwFn, e.span), ident(&fname, e.span), param_tok], 0),
                ));
                for g in givens {
                    made.extend(placed(
                        &g.stmt,
                        2,
                        vec![ident(&g.name, g.span), tok(Tok::Bind, g.span)],
                    ));
                }
                let mut checks = Vec::new();
                for s in &e.body {
                    if binding(s).is_some() {
                        made.extend(placed(s, 2, Vec::new()));
                        continue;
                    }
                    let name = format!("{MINE}check_{}", checks.len() + 1);
                    made.extend(placed(
                        s,
                        2,
                        vec![ident(&name, s.span()), tok(Tok::Bind, s.span())],
                    ));
                    checks.push((name, s.span()));
                }
                let mut every = vec![ident("every", e.span), tok(Tok::LBracket, e.span)];
                for (c, span) in &checks {
                    every.push(ident(c, *span));
                }
                every.push(tok(Tok::RBracket, e.span));
                made.push(line(
                    e.body.last().map(|s| s.head().number).unwrap_or(e.span.line as usize),
                    2,
                    laid(every, 2),
                ));
            }
        }
    }
}
