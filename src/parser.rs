use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer::{Lexed, Line, StrPart, Tok};

/// An entry file: imports, then statements — the body IS the program. The
/// statements synthesize an internal `main` constant so every later stage
/// works unchanged; no user writes the name.
pub fn parse_entry(lexed: &Lexed) -> Result<Program, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut imports: Vec<Import> = Vec::new();
    let mut first_stmt = None;
    for (idx, line) in lexed.lines.iter().enumerate() {
        if line.indent != 0 {
            continue;
        }
        match line.tokens.first() {
            Some((Tok::KwImport, _)) => {
                if first_stmt.is_some() {
                    diags.push(Diagnostic::new(
                        "formatting",
                        "imports open the file, before any statement".to_string(),
                        head_span(line),
                    ));
                }
                match parse_import(line, &[]) {
                    Ok(import) => {
                        if let Some(prev) = imports.last() {
                            if prev.path >= import.path {
                                let msg = match prev.path == import.path {
                                    true => format!("duplicate import \"{}\"", import.path),
                                    false => "imports appear in alphabetical order".to_string(),
                                };
                                diags.push(Diagnostic::new("formatting", msg, import.span));
                            }
                        }
                        imports.push(import);
                    }
                    Err(d) => diags.push(d),
                }
            }
            Some((Tok::KwFn | Tok::KwType | Tok::KwPub, _)) => {
                diags.push(Diagnostic::new(
                    "syntax",
                    "an entry file holds statements only; definitions live in \
                     library files"
                        .to_string(),
                    head_span(line),
                ));
            }
            _ => {
                if first_stmt.is_none() {
                    first_stmt = Some(idx);
                }
            }
        }
    }
    let stmt_lines: &[Line] = match first_stmt {
        Some(start) => &lexed.lines[start..],
        None => &[],
    };
    if stmt_lines.is_empty() {
        diags.push(Diagnostic::new(
            "syntax",
            "an entry file needs at least one statement".to_string(),
            Span { line: 1, col: 1 },
        ));
    }
    // a continuation line may not restart after a blank: the chain it would
    // splice into has already closed
    if let Some(start) = first_stmt {
        for line in &lexed.lines[start..] {
            if line.indent != 0
                && lexed.blank_lines.contains(&(line.number - 1))
                && matches!(line.tokens.first(), Some((Tok::SeqOp | Tok::Pipe, _)))
            {
                diags.push(Diagnostic::new(
                    "formatting",
                    "a continuation may not follow a blank line — the statement \
                     it would splice into has closed"
                        .to_string(),
                    head_span(line),
                ));
            }
        }
    }
    let body = match parse_body(stmt_lines) {
        Ok(body) => body,
        Err(d) => {
            diags.push(d);
            Vec::new()
        }
    };
    if !diags.is_empty() {
        return Err(diags);
    }
    let span = stmt_lines.first().map(head_span).unwrap_or(Span { line: 1, col: 1 });
    let main = FnDecl {
        name: "main".to_string(),
        params: Vec::new(),
        body,
        span,
        is_pub: false,
        file: String::new(),
        synthetic: false,
    };
    Ok(Program { fns: vec![main], types: Vec::new(), imports })
}

pub fn parse(lexed: &Lexed) -> Result<Program, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut fns = Vec::new();
    let mut types = Vec::new();
    let mut imports: Vec<Import> = Vec::new();
    let mut past_imports = false;
    check_blank_policy(lexed, &mut diags);
    let mut i = 0;
    while i < lexed.lines.len() {
        let line = &lexed.lines[i];
        if line.indent != 0 {
            diags.push(Diagnostic::new(
                "syntax",
                "expected a top-level declaration (`fn` or `type`)".to_string(),
                head_span(line),
            ));
            i += 1;
            continue;
        }
        let body_start = i + 1;
        let mut body_end = body_start;
        while body_end < lexed.lines.len() && lexed.lines[body_end].indent == 2 {
            body_end += 1;
        }
        let body = &lexed.lines[body_start..body_end];
        if !matches!(line.tokens.first(), Some((Tok::KwImport, _))) {
            past_imports = true;
        }
        let head_idx = match line.tokens.first() {
            Some((Tok::KwPub, _)) => 1,
            _ => 0,
        };
        let is_constant = matches!(
            (line.tokens.get(head_idx), line.tokens.get(head_idx + 1)),
            (Some((Tok::Ident(_), _)), Some((Tok::Bind, _)))
        );
        match line.tokens.get(head_idx) {
            Some((Tok::KwFn, _)) => match parse_fn(line, body) {
                Ok(decl) => fns.push(decl),
                Err(d) => diags.push(d),
            },
            Some((Tok::KwType, _)) => match parse_type(line, body) {
                Ok(decl) => types.push(decl),
                Err(d) => diags.push(d),
            },
            Some((Tok::KwImport, _)) => {
                match parse_import(line, body) {
                    Ok(import) => {
                        if past_imports {
                            diags.push(Diagnostic::new(
                                "formatting",
                                "imports open the file, before any declaration".to_string(),
                                head_span(line),
                            ));
                        }
                        if let Some(prev) = imports.last() {
                            if prev.path >= import.path {
                                let msg = match prev.path == import.path {
                                    true => format!("duplicate import \"{}\"", import.path),
                                    false => "imports appear in alphabetical order".to_string(),
                                };
                                diags.push(Diagnostic::new("formatting", msg, import.span));
                            }
                        }
                        imports.push(import);
                    }
                    Err(d) => diags.push(d),
                }
                i = body_end;
                continue;
            }
            Some((Tok::Ident(_), _)) if is_constant => match parse_constant(line, body) {
                Ok(decl) => fns.push(decl),
                Err(d) => diags.push(d),
            },
            _ => diags.push(Diagnostic::new(
                "syntax",
                "a top-level line must begin with `fn`, `type`, or a constant binding"
                    .to_string(),
                head_span(line),
            )),
        }
        i = body_end;
    }
    if diags.is_empty() { Ok(Program { fns, types, imports }) } else { Err(diags) }
}

/// `import "path"` — one string, nothing else, no body.
fn parse_import(line: &Line, body: &[Line]) -> Result<Import, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            "an import has no body".to_string(),
            head_span(&body[0]),
        ));
    }
    let plain_path = |parts: &[crate::lexer::StrPart], span: Span| match parts {
        [crate::lexer::StrPart::Lit(text)] => Ok(text.clone()),
        _ => Err(Diagnostic::new(
            "syntax",
            "an import path is a plain string".to_string(),
            span,
        )),
    };
    match line.tokens.as_slice() {
        [(Tok::KwImport, _), (Tok::Str(parts), span)] => {
            let path = plain_path(parts, *span)?;
            Ok(Import { path, span: *span, alias: None, renames: Vec::new() })
        }
        // import t "path" — alias the qualifier
        [(Tok::KwImport, _), (Tok::Ident(alias), _), (Tok::Str(parts), span)] => {
            let path = plain_path(parts, *span)?;
            Ok(Import { path, span: *span, alias: Some(alias.clone()), renames: Vec::new() })
        }
        // import { theirs:yours ... } "path" — renames only; a bare word in
        // braces is redundant (the compiler prunes; bare access is default)
        [(Tok::KwImport, _), (Tok::LBrace, brace_span), rest @ ..] => {
            let (renames, i) = parse_renames(rest, *brace_span)?;
            match rest.get(i) {
                Some((Tok::Str(parts), span)) if rest.len() == i + 1 && !renames.is_empty() => {
                    let path = plain_path(parts, *span)?;
                    Ok(Import { path, span: *span, alias: None, renames })
                }
                _ => Err(Diagnostic::new(
                    "syntax",
                    "an import ends with its path string".to_string(),
                    *brace_span,
                )),
            }
        }
        // import t { theirs:yours ... } "path" — alias and renames combined
        [(Tok::KwImport, _), (Tok::Ident(alias), _), (Tok::LBrace, brace_span), rest @ ..] => {
            let (renames, i) = parse_renames(rest, *brace_span)?;
            match rest.get(i) {
                Some((Tok::Str(parts), span)) if rest.len() == i + 1 && !renames.is_empty() => {
                    let path = plain_path(parts, *span)?;
                    Ok(Import { path, span: *span, alias: Some(alias.clone()), renames })
                }
                _ => Err(Diagnostic::new(
                    "syntax",
                    "an import ends with its path string".to_string(),
                    *brace_span,
                )),
            }
        }
        _ => Err(Diagnostic::new(
            "syntax",
            "an import is `import \"path\"`".to_string(),
            head_span(line),
        )),
    }
}

/// The brace body of an import: `theirs:yours` pairs, tight colons, closed
/// by `}`. Returns the pairs and the index just past the closing brace.
fn parse_renames(
    rest: &[(Tok, Span)],
    brace_span: Span,
) -> Result<(Vec<(String, String)>, usize), Diagnostic> {
    let mut renames = Vec::new();
    let mut i = 0;
    loop {
        match rest.get(i) {
            Some((Tok::RBrace, _)) => {
                i += 1;
                break;
            }
            Some((Tok::Ident(theirs), _)) => match rest.get(i + 1) {
                Some((Tok::Colon, _)) => match rest.get(i + 2) {
                    Some((Tok::Ident(yours), _)) => {
                        renames.push((theirs.clone(), yours.clone()));
                        i += 3;
                    }
                    other => {
                        let span = other.map(|(_, s)| *s).unwrap_or(brace_span);
                        return Err(Diagnostic::new(
                            "syntax",
                            "a rename is theirs:yours".to_string(),
                            span,
                        ));
                    }
                },
                other => {
                    let span = other.map(|(_, s)| *s).unwrap_or(brace_span);
                    return Err(Diagnostic::new(
                        "syntax",
                        "an unrenamed selection is redundant — the compiler \
                         prunes unused imports and bare access is the \
                         default; braces hold theirs:yours renames"
                            .to_string(),
                        span,
                    ));
                }
            },
            other => {
                let span = other.map(|(_, s)| *s).unwrap_or(brace_span);
                return Err(Diagnostic::new(
                    "syntax",
                    "braces hold theirs:yours renames".to_string(),
                    span,
                ));
            }
        }
    }
    Ok((renames, i))
}

fn head_span(line: &Line) -> Span {
    line.tokens.first().map(|(_, s)| *s).unwrap_or(Span { line: line.number, col: 1 })
}

fn check_blank_policy(lexed: &Lexed, diags: &mut Vec<Diagnostic>) {
    let Some(first) = lexed.lines.first() else { return };
    for blank in &lexed.blank_lines {
        if *blank < first.number {
            diags.push(Diagnostic::new(
                "formatting",
                "the file may not begin with a blank line".to_string(),
                Span { line: *blank, col: 1 },
            ));
        }
    }
    if let Some(last) = lexed.lines.last() {
        for blank in &lexed.blank_lines {
            if *blank > last.number {
                diags.push(Diagnostic::new(
                    "formatting",
                    "the file may not end with a blank line".to_string(),
                    Span { line: *blank, col: 1 },
                ));
            }
        }
    }
    for pair in lexed.lines.windows(2) {
        let blanks =
            lexed.blank_lines.iter().filter(|b| **b > pair[0].number && **b < pair[1].number).count();
        let both_imports = matches!(pair[0].tokens.first(), Some((Tok::KwImport, _)))
            && matches!(pair[1].tokens.first(), Some((Tok::KwImport, _)));
        let decl_start = matches!(
            pair[1].tokens.first(),
            Some((Tok::KwFn | Tok::KwType | Tok::KwPub, _))
        ) || matches!(
            (pair[1].tokens.first(), pair[1].tokens.get(1)),
            (Some((Tok::Ident(_), _)), Some((Tok::Bind, _)))
        );
        let required = match pair[1].indent {
            // the import block stacks; one blank closes it
            0 if both_imports => 0,
            // a declaration takes its separating blank; statement lines may
            // pack — adjacency is the group grammar
            0 if decl_start => 1,
            _ => 0,
        };
        if blanks != required {
            let message = match required {
                1 => "exactly one blank line separates top-level declarations".to_string(),
                _ => "blank lines may not appear inside a body".to_string(),
            };
            diags.push(Diagnostic::new(
                "formatting",
                message,
                Span { line: pair[1].number, col: 1 },
            ));
        }
    }
}

fn parse_fn(header: &Line, body: &[Line]) -> Result<FnDecl, Diagnostic> {
    let mut p = P::new(&header.tokens, &header.end_cols, header.number);
    let is_pub = p.consume_pub();
    p.expect_kw_fn()?;
    let (name, span) = p.expect_ident("a function name")?;
    let mut params = Vec::new();
    while !p.done() {
        params.push(p.parse_pattern()?);
    }
    if params.is_empty() {
        return Err(Diagnostic::new(
            "formatting",
            format!("a value with no parameters is a constant: `{name} = ...`"),
            span,
        ));
    }
    if body.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            format!("function `{name}` has no body"),
            span,
        ));
    }
    let stmts = parse_body(body)?;
    Ok(FnDecl { name, is_pub, span, params, body: stmts, file: String::new() ,
        synthetic: false,})
}

fn parse_constant(header: &Line, body: &[Line]) -> Result<FnDecl, Diagnostic> {
    let is_pub = matches!(header.tokens.first(), Some((Tok::KwPub, _)));
    let off = usize::from(is_pub);
    let Some((Tok::Ident(name), span)) = header.tokens.get(off) else {
        return Err(Diagnostic::new("syntax", "expected a constant name".to_string(), head_span(header)));
    };
    let name = name.clone();
    let span = *span;
    if header.tokens.len() == off + 2 {
        if body.is_empty() {
            return Err(Diagnostic::new(
                "syntax",
                format!("constant `{name}` has no value"),
                span,
            ));
        }
        if body.len() == 1 {
            return Err(Diagnostic::new(
                "formatting",
                format!("a single-expression constant is written inline: `{name} = ...`"),
                span,
            ));
        }
        let stmts = parse_body(body)?;
        return Ok(FnDecl { name, is_pub, span, params: Vec::new(), body: stmts, file: String::new() ,
        synthetic: false,});
    }
    if !body.is_empty() {
        return Err(Diagnostic::new(
            "formatting",
            "an inline constant has no indented block".to_string(),
            head_span(&body[0]),
        ));
    }
    let mut p = P::new(&header.tokens[off + 2..], &header.end_cols[off + 2..], header.number);
    let expr = p.parse_expr()?;
    p.expect_done()?;
    Ok(FnDecl { name, is_pub, span, params: Vec::new(), body: vec![Stmt::Expr(expr)], file: String::new() ,
        synthetic: false,})
}

fn parse_type(header: &Line, body: &[Line]) -> Result<TypeDecl, Diagnostic> {
    let mut p = P::new(&header.tokens, &header.end_cols, header.number);
    let is_pub = p.consume_pub();
    p.expect_kw_type()?;
    let (name, span) = p.expect_ident("a type name")?;
    p.expect_done()?;
    let fields = body.iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;
    Ok(TypeDecl { name, is_pub, span, synthetic: false, fields })
}

fn parse_field(line: &Line) -> Result<(String, Vec<String>, Span), Diagnostic> {
    let mut p = P::new(&line.tokens, &line.end_cols, line.number);
    let (name, span) = p.expect_ident("a field name")?;
    let colon_span = p.span_here();
    p.expect_colon()?;
    let ty_span = p.span_here();
    if ty_span.col != colon_span.col + 1 {
        return Err(Diagnostic::new(
            "formatting",
            "a field annotation binds tight: `name:type`".to_string(),
            colon_span,
        ));
    }
    let mut tys = vec![p.parse_type_expr()?];
    while !p.done() {
        tys.push(p.parse_type_expr()?);
    }
    Ok((name, tys, span))
}

/// Parse a body's lines into statements, desugaring the concurrency surface:
/// bare description lines form unordered groups (joined with the internal `&`
/// node, failures accumulating) and a lone `>>` line is a wall sequencing the
/// groups. Bindings keep their places; the folded chain becomes the body's
/// final expression. A body with no bare lines and no walls passes through
/// untouched.
fn parse_body(body: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let is_wall = |line: &Line| matches!(line.tokens.as_slice(), [(Tok::SeqOp, _)]);
    let has_surface = body.iter().enumerate().any(|(i, l)| {
        is_wall(l)
            || matches!(l.tokens.first(), Some((Tok::SeqOp, _)))
            || (i + 1 < body.len() && matches!(parse_stmt_shape(l), StmtShape::Expr))
    });
    if !has_surface {
        return body.iter().map(parse_stmt).collect();
    }
    let mut binds: Vec<Stmt> = Vec::new();
    let mut segments: Vec<Vec<Expr>> = vec![Vec::new()];
    let mut wall_spans: Vec<Span> = Vec::new();
    let mut wall_fused: Vec<bool> = Vec::new();
    let mut closed_by_fuse = false;
    for line in body {
        let fused = matches!(line.tokens.first(), Some((Tok::SeqOp, _)))
            && line.tokens.len() > 1;
        if is_wall(line) || fused {
            let span = line.tokens[0].1;
            if segments.last().is_some_and(Vec::is_empty) {
                return Err(Diagnostic::new(
                    "syntax",
                    "nothing to sequence: a `>>` wall needs statements above it".to_string(),
                    span,
                ));
            }
            wall_spans.push(span);
            wall_fused.push(fused);
            segments.push(Vec::new());
            if fused {
                // `>> expr` is a COMPLETE sequential step: wall plus its one
                // member, closed — nothing may silently join it
                let mut p = P::new(&line.tokens[1..], &line.end_cols[1..], line.number);
                let expr = p.parse_expr()?;
                p.expect_done()?;
                reject_never_effect(&expr)?;
                segments.last_mut().expect("segment").push(expr);
                closed_by_fuse = true;
            } else {
                closed_by_fuse = false;
            }
            continue;
        }
        match parse_stmt(line)? {
            Stmt::Bind { pattern, expr } => {
                // every binding runs before every bare effect line, wherever
                // it appears — so the surface may not show it interleaved
                if !segments[0].is_empty() || segments.len() > 1 {
                    return Err(Diagnostic::new(
                        "formatting",
                        "bindings precede the effects in a body: every binding runs \
                         before every bare effect line, so move it above the chain"
                            .to_string(),
                        expr_span(&expr),
                    ));
                }
                binds.push(Stmt::Bind { pattern, expr });
            }
            Stmt::Expr(e) => {
                if closed_by_fuse {
                    return Err(Diagnostic::new(
                        "formatting",
                        "a fused `>> step` is a single sequential step — a line \
                         cannot silently join it. for a group, put the wall alone \
                         and list the members below it"
                            .to_string(),
                        expr_span(&e),
                    ));
                }
                reject_never_effect(&e)?;
                segments.last_mut().expect("segment").push(e);
            }
        }
    }
    let Some(last) = segments.last() else { unreachable!() };
    if last.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            "nothing follows the final `>>` wall".to_string(),
            *wall_spans.last().expect("a trailing wall exists"),
        ));
    }
    // one right way: a lone wall exists for multi-member groups. a stage of
    // one step is a single statement and fuses with its wall
    for (i, fused) in wall_fused.iter().enumerate() {
        if !fused && segments[i + 1].len() == 1 {
            return Err(Diagnostic::new(
                "formatting",
                "a one-step stage fuses with its wall: write `>> step` on one line"
                    .to_string(),
                wall_spans[i],
            ));
        }
    }
    let joined: Vec<Expr> = segments
        .into_iter()
        .map(|seg| {
            let mut it = seg.into_iter();
            let first = it.next().expect("segments are non-empty");
            it.fold(first, |acc, e| {
                let span = expr_span(&e);
                Expr::Join { lhs: Box::new(acc), rhs: Box::new(e), span }
            })
        })
        .collect();
    let mut it = joined.into_iter().rev();
    let tail = it.next().expect("at least one segment");
    let chain = it.fold(tail, |acc, seg| {
        let span = expr_span(&seg);
        Expr::Seq(Box::new(seg), Box::new(acc), span)
    });
    binds.push(Stmt::Expr(chain));
    Ok(binds)
}

/// A bare line in an effect group must at least plausibly be a description.
/// Literals, arithmetic, comparisons, and lambdas never are — those keep the
/// classic unused-expression error instead of dying inside the runtime join.
fn reject_never_effect(e: &Expr) -> Result<(), Diagnostic> {
    let never = matches!(
        e,
        Expr::Int(..)
            | Expr::Float(..)
            | Expr::Str(..)
            | Expr::List(..)
            | Expr::MapLit(..)
            | Expr::Lambda { .. }
            | Expr::BinOp { .. }
    );
    if never {
        return Err(Diagnostic::new(
            "unused",
            "unused expression: every non-final line binds a name (sequence effects \
             with `>>`)"
                .to_string(),
            expr_span(e),
        ));
    }
    Ok(())
}

enum StmtShape {
    Bind,
    Expr,
}

/// Is this line a binding or a bare expression? (Mirrors parse_stmt's split
/// without committing to a full parse.)
fn parse_stmt_shape(line: &Line) -> StmtShape {
    let mut depth = 0usize;
    for (tok, _) in &line.tokens {
        match tok {
            Tok::LParen | Tok::LBracket | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RBracket | Tok::RBrace => depth = depth.saturating_sub(1),
            Tok::Bind if depth == 0 => return StmtShape::Bind,
            _ => {}
        }
    }
    StmtShape::Expr
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, s)
        | Expr::Field { span: s, .. }
        | Expr::Float(_, s)
        | Expr::MapLit(_, s)
        | Expr::Str(_, s)
        | Expr::Ident(_, s)
        | Expr::List(_, s)
        | Expr::Seq(_, _, s)
        | Expr::Join { span: s, .. }
        | Expr::Lambda { span: s, .. }
        | Expr::App { span: s, .. }
        | Expr::Index { span: s, .. }
        | Expr::BinOp { span: s, .. } => *s,
    }
}

fn parse_stmt(line: &Line) -> Result<Stmt, Diagnostic> {
    let mut depth = 0usize;
    let mut bind_at = None;
    for (i, (tok, _)) in line.tokens.iter().enumerate() {
        match tok {
            Tok::LParen | Tok::LBracket | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RBracket | Tok::RBrace => depth = depth.saturating_sub(1),
            Tok::Bind if depth == 0 => {
                bind_at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(i) = bind_at else {
        let mut p = P::new(&line.tokens, &line.end_cols, line.number);
        let expr = p.parse_expr()?;
        p.expect_done()?;
        return Ok(Stmt::Expr(expr));
    };
    let mut lhs = P::new(&line.tokens[..i], &line.end_cols[..i], line.number);
    let pattern = lhs.parse_bind_target()?;
    lhs.expect_done()?;
    let mut rhs = P::new(&line.tokens[i + 1..], &line.end_cols[i + 1..], line.number);
    let expr = rhs.parse_expr()?;
    rhs.expect_done()?;
    Ok(Stmt::Bind { pattern, expr })
}

pub struct P<'a> {
    toks: &'a [(Tok, Span)],
    ends: &'a [usize],
    pub pos: usize,
    line: usize,
}

impl<'a> P<'a> {
    pub fn new(toks: &'a [(Tok, Span)], ends: &'a [usize], line: usize) -> Self {
        P { toks, ends, pos: 0, line }
    }

    fn last_end(&self) -> usize {
        match self.pos {
            0 => 0,
            n => self.ends.get(n - 1).copied().unwrap_or(0),
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn span_here(&self) -> Span {
        self.toks
            .get(self.pos)
            .or_else(|| self.toks.last())
            .map(|(_, s)| *s)
            .unwrap_or(Span { line: self.line, col: 1 })
    }

    fn done(&self) -> bool {
        self.pos >= self.toks.len()
    }

    fn err(&self, message: String) -> Diagnostic {
        Diagnostic::new("syntax", message, self.span_here())
    }

    fn expect_done(&self) -> Result<(), Diagnostic> {
        match self.done() {
            true => Ok(()),
            false => Err(self.err("unexpected trailing tokens".to_string())),
        }
    }

    fn expect_kw_fn(&mut self) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(Tok::KwFn) => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.err("expected `fn`".to_string())),
        }
    }

    fn expect_kw_type(&mut self) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(Tok::KwType) => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.err("expected `type`".to_string())),
        }
    }

    fn consume_pub(&mut self) -> bool {
        match self.peek() {
            Some(Tok::KwPub) => {
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<(String, Span), Diagnostic> {
        match self.toks.get(self.pos) {
            Some((Tok::Ident(name), span)) => {
                self.pos += 1;
                Ok((name.clone(), *span))
            }
            _ => Err(self.err(format!("expected {what}"))),
        }
    }

    fn expect_colon(&mut self) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(Tok::Colon) => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.err("expected `:`".to_string())),
        }
    }

    fn parse_type_expr(&mut self) -> Result<String, Diagnostic> {
        let (mut ty, _) = self.expect_ident("a type")?;
        while matches!(self.peek(), Some(Tok::LBracket)) {
            self.pos += 1;
            match self.peek() {
                Some(Tok::RBracket) => {
                    self.pos += 1;
                    ty.push_str("[]");
                }
                Some(Tok::Ident(key)) => {
                    let key = key.clone();
                    self.pos += 1;
                    match self.peek() {
                        Some(Tok::RBracket) => {
                            self.pos += 1;
                            ty = format!("{ty}[{key}]");
                        }
                        _ => return Err(self.err("expected `]`".to_string())),
                    }
                }
                _ => return Err(self.err("expected `]` or a key type".to_string())),
            }
        }
        Ok(ty)
    }

    pub fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        let span = self.span_here();
        match self.toks.get(self.pos).map(|(t, _)| t.clone()) {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Ok(Pattern::IntLit(n, span))
            }
            Some(Tok::Str(parts)) => {
                self.pos += 1;
                let lit = literal_string(&parts)
                    .ok_or_else(|| self.err("string patterns may not interpolate".to_string()))?;
                Ok(Pattern::StrLit(lit, span))
            }
            Some(Tok::Underscore) => {
                self.pos += 1;
                Ok(Pattern::Wildcard(span))
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    let colon_span = self.span_here();
                    let tight_before = colon_span.col == span.col + name.len();
                    self.pos += 1;
                    let ty_span = self.span_here();
                    let tight_after = ty_span.col == colon_span.col + 1;
                    if !tight_before || !tight_after {
                        return Err(Diagnostic::new(
                            "formatting",
                            format!("type ascription is tight: `{name}:type`"),
                            colon_span,
                        ));
                    }
                    let ty = self.parse_type_expr()?;
                    return Ok(Pattern::Annotated { name, ty, span });
                }
                match NULLARY.contains(&name.as_str()) {
                    true => Ok(Pattern::Nullary(name, span)),
                    false => Ok(Pattern::Var(name, span)),
                }
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let (name, name_span) = self.expect_ident("a name or type")?;
                match self.peek() {
                    Some(Tok::Colon) => {
                        let _ = name_span;
                        Err(Diagnostic::new(
                            "formatting",
                            format!(
                                "a single-type ascription is written tight: `{name}:type` \
                                 (parenthesized guards return with typesets)"
                            ),
                            self.span_here(),
                        ))
                    }
                    _ => {
                        let mut fields = vec![self.parse_pattern()?];
                        while !matches!(self.peek(), Some(Tok::RParen)) {
                            fields.push(self.parse_pattern()?);
                        }
                        self.expect_rparen()?;
                        Ok(Pattern::Ctor { ty: name, fields })
                    }
                }
            }
            _ => Err(self.err("expected a parameter pattern".to_string())),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(Tok::RParen) => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.err("expected `)`".to_string())),
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_pipe()
    }

    pub fn parse_bind_target(&mut self) -> Result<Pattern, Diagnostic> {
        if matches!(self.peek(), Some(Tok::LBrace)) {
            return self.parse_keyed();
        }
        let (first, span) = self.expect_ident("a binding name or type")?;
        match self.done() {
            true => Ok(Pattern::Var(first, span)),
            false => {
                let mut fields = Vec::new();
                while !self.done() {
                    fields.push(self.parse_pattern()?);
                }
                Ok(Pattern::Ctor { ty: first, fields })
            }
        }
    }

    fn parse_keyed(&mut self) -> Result<Pattern, Diagnostic> {
        let span = self.span_here();
        self.pos += 1;
        let mut entries = Vec::new();
        while !matches!(self.peek(), Some(Tok::RBrace)) {
            let (field, field_span) = self.expect_ident("a field name")?;
            let bind_name = match self.peek() {
                Some(Tok::Colon) => {
                    let colon_span = self.span_here();
                    self.pos += 1;
                    let target_span = self.span_here();
                    if target_span.col != colon_span.col + 2 {
                        return Err(Diagnostic::new(
                            "formatting",
                            "a rename is spaced: `field: name`".to_string(),
                            colon_span,
                        ));
                    }
                    self.expect_ident("a binding name")?.0
                }
                _ => field.clone(),
            };
            entries.push(KeyedEntry { field, bind_name, span: field_span });
        }
        self.pos += 1;
        match entries.is_empty() {
            true => Err(self.err("a keyed read names at least one field".to_string())),
            false => Ok(Pattern::Keyed { entries, span }),
        }
    }

    fn parse_pipe(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_join()?;
        loop {
            match self.peek() {
                Some(Tok::Pipe) => {
                    let span = self.span_here();
                    self.pos += 1;
                    let target = self.parse_app()?;
                    expr = match target {
                        Expr::App { head, mut args, .. } => {
                            args.insert(0, expr);
                            Expr::App { head, args, span, piped: true }
                        }
                        atom => Expr::App {
                            head: Box::new(atom),
                            args: vec![expr],
                            span,
                            piped: true,
                        },
                    };
                }
                Some(Tok::SeqOp) => {
                    let span = self.span_here();
                    self.pos += 1;
                    let rhs = self.parse_join()?;
                    expr = Expr::Seq(Box::new(expr), Box::new(rhs), span);
                }
                _ => return Ok(expr),
            }
        }
    }

    fn parse_join(&mut self) -> Result<Expr, Diagnostic> {
        let lhs = self.parse_cmp()?;
        if let Some(Tok::Op("&")) = self.peek() {
            return Err(Diagnostic::new(
                "syntax",
                "parallel statements are unordered by default — write them as \
                 separate lines; sequence with `>>`"
                    .to_string(),
                self.span_here(),
            ));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, Diagnostic> {
        let lhs = self.parse_add()?;
        let cmp = ["<", "<=", ">", ">=", "==", "!="];
        if let Some(Tok::Op(op)) = self.peek() {
            if cmp.contains(op) {
                let op = *op;
                let span = self.span_here();
                self.pos += 1;
                let rhs = self.parse_add()?;
                return Ok(Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span });
            }
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_mul()?;
        while let Some(Tok::Op(op @ ("+" | "-"))) = self.peek() {
            let op = *op;
            let span = self.span_here();
            self.pos += 1;
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_app()?;
        while let Some(Tok::Op(op @ ("*" | "/"))) = self.peek() {
            let op = *op;
            let span = self.span_here();
            self.pos += 1;
            let rhs = self.parse_app()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn starts_atom(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Tok::Ident(_)
                    | Tok::Int(_)
                    | Tok::Float(_)
                    | Tok::Str(_)
                    | Tok::LParen
                    | Tok::LBracket
            )
        )
    }

    fn parse_app(&mut self) -> Result<Expr, Diagnostic> {
        let head = self.parse_atom()?;
        let mut args = Vec::new();
        while self.starts_atom() {
            args.push(self.parse_atom()?);
        }
        match args.is_empty() {
            true => Ok(head),
            false => {
                let span = head.span();
                Ok(Expr::App { head: Box::new(head), args, span, piped: false })
            }
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_atom_base()?;
        loop {
            if matches!(self.peek(), Some(Tok::Dot)) {
                let span = self.span_here();
                self.pos += 1;
                let Some(Tok::Ident(name)) = self.toks.get(self.pos).map(|(t, _)| t.clone())
                else {
                    return Err(self.err("a field name follows the dot".to_string()));
                };
                self.pos += 1;
                expr = Expr::Field { base: Box::new(expr), name, span };
                continue;
            }
            let tight = matches!(self.peek(), Some(Tok::LBracket))
                && self.span_here().col == self.last_end();
            if !tight {
                return Ok(expr);
            }
            let span = self.span_here();
            self.pos += 1;
            let index = self.parse_pipe()?;
            match self.peek() {
                Some(Tok::RBracket) => {
                    self.pos += 1;
                }
                _ => return Err(self.err("expected `]`".to_string())),
            }
            let strict = matches!(self.peek(), Some(Tok::Bang));
            if strict {
                self.pos += 1;
            }
            expr = Expr::Index { base: Box::new(expr), index: Box::new(index), strict, span };
        }
    }

    fn parse_atom_base(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span_here();
        match self.toks.get(self.pos).map(|(t, _)| t.clone()) {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Ok(Expr::Int(n, span))
            }
            Some(Tok::Float(x)) => {
                self.pos += 1;
                Ok(Expr::Float(x, span))
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                Ok(Expr::Ident(name, span))
            }
            Some(Tok::Str(parts)) => {
                self.pos += 1;
                let template = parts
                    .iter()
                    .map(|part| template_part(part, self.line))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expr::Str(template, span))
            }
            Some(Tok::LBracket) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    self.pos += 1;
                    match self.peek() {
                        Some(Tok::RBracket) => {
                            self.pos += 1;
                            return Ok(Expr::MapLit(Vec::new(), span));
                        }
                        _ => return Err(self.err("`[:]` is the empty map".to_string())),
                    }
                }
                if matches!(self.peek(), Some(Tok::RBracket)) {
                    self.pos += 1;
                    return Ok(Expr::List(Vec::new(), span));
                }
                let first = self.parse_atom()?;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    let mut pairs = Vec::new();
                    let mut key = first;
                    loop {
                        self.require_literal_key(&key)?;
                        let colon_span = self.span_here();
                        self.pos += 1;
                        let value_span = self.span_here();
                        if value_span.col != colon_span.col + 2 {
                            return Err(Diagnostic::new(
                                "formatting",
                                "a map pair is spaced: `key: value`".to_string(),
                                colon_span,
                            ));
                        }
                        let value = self.parse_atom()?;
                        pairs.push((key, value));
                        if matches!(self.peek(), Some(Tok::RBracket)) {
                            self.pos += 1;
                            self.check_key_order(&pairs)?;
                            return Ok(Expr::MapLit(pairs, span));
                        }
                        key = self.parse_atom()?;
                        match self.peek() {
                            Some(Tok::Colon) => {}
                            _ => return Err(self.err("expected `:` after a map key".to_string())),
                        }
                    }
                }
                let mut items = vec![first];
                while !matches!(self.peek(), Some(Tok::RBracket)) {
                    items.push(self.parse_atom()?);
                }
                self.pos += 1;
                Ok(Expr::List(items, span))
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                if let Some(arrow_end) = self.lambda_lookahead() {
                    let mut params = Vec::new();
                    while self.pos < arrow_end {
                        if let Some((Tok::Underscore, uspan)) = self.toks.get(self.pos) {
                            params.push(("_".to_string(), *uspan));
                            self.pos += 1;
                            continue;
                        }
                        let (name, pspan) = self.expect_ident("a lambda parameter")?;
                        params.push((name, pspan));
                    }
                    self.pos = arrow_end + 1;
                    let body = self.parse_expr()?;
                    self.expect_rparen()?;
                    return Ok(Expr::Lambda { params, body: Box::new(body), span });
                }
                let inner = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            _ => Err(self.err("expected an expression".to_string())),
        }
    }

    fn require_literal_key(&self, key: &Expr) -> Result<(), Diagnostic> {
        match key {
            Expr::Int(..) => Ok(()),
            Expr::Str(parts, _) if parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))) => {
                Ok(())
            }
            _ => Err(Diagnostic::new(
                "syntax",
                "map literal keys are literals; build dynamic maps with `put`".to_string(),
                key.span(),
            )),
        }
    }

    fn check_key_order(&self, pairs: &[(Expr, Expr)]) -> Result<(), Diagnostic> {
        let mut rendered: Vec<(String, Span)> = Vec::new();
        for (key, _) in pairs {
            let text = match key {
                Expr::Int(n, span) => (format!("#{n:0>40}"), *span),
                Expr::Str(parts, span) => {
                    let mut out = String::new();
                    for part in parts {
                        if let TemplatePart::Lit(lit) = part {
                            out.push_str(lit);
                        }
                    }
                    (out, *span)
                }
                _ => continue,
            };
            rendered.push(text);
        }
        for pair in rendered.windows(2) {
            if pair[0].0 >= pair[1].0 {
                return Err(Diagnostic::new(
                    "formatting",
                    "map literal keys appear in sorted order, without duplicates".to_string(),
                    pair[1].1,
                ));
            }
        }
        Ok(())
    }

    fn lambda_lookahead(&self) -> Option<usize> {
        let mut i = self.pos;
        while matches!(
            self.toks.get(i).map(|(t, _)| t),
            Some(Tok::Ident(_)) | Some(Tok::Underscore)
        ) {
            i += 1;
            if let Some(Tok::Arrow) = self.toks.get(i).map(|(t, _)| t) {
                return Some(i);
            }
        }
        None
    }
}

fn literal_string(parts: &[StrPart]) -> Option<String> {
    let mut out = String::new();
    for part in parts {
        match part {
            StrPart::Lit(s) => out.push_str(s),
            StrPart::Interp(..) => return None,
        }
    }
    Some(out)
}

fn template_part(part: &StrPart, line: usize) -> Result<TemplatePart, Diagnostic> {
    match part {
        StrPart::Lit(s) => Ok(TemplatePart::Lit(s.clone())),
        StrPart::Interp(tokens, ends) => {
            let mut p = P::new(tokens, ends, line);
            let expr = p.parse_expr()?;
            p.expect_done()?;
            Ok(TemplatePart::Interp(expr))
        }
    }
}
