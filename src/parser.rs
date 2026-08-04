use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer::{Lexed, Line, StrPart, Tok};

/// An entry file: imports, then statements — the body IS the program. The
/// statements synthesize an internal `main` constant so every later stage
/// works unchanged; no user writes the name.
/// A play file: declarations first, then the statements that use them, in
/// one file. `kanso play` is the only door — nothing imports a play file,
/// so `pub` is refused, and the statements close the file because a
/// declaration after them would read as part of the run.
pub fn parse_play(lexed: &Lexed) -> Result<Program, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut first_stmt = None;
    for (idx, line) in lexed.lines.iter().enumerate() {
        if line.indent != 0 {
            continue;
        }
        match line.tokens.first() {
            Some((Tok::KwImport | Tok::KwFn | Tok::KwType, _)) => {
                if first_stmt.is_some() {
                    diags.push(Diagnostic::new(
                        "syntax",
                        "declarations and imports come before the statements                          in a play file"
                            .to_string(),
                        head_span(line),
                    ));
                }
            }
            Some((Tok::KwPub, _)) => {
                diags.push(Diagnostic::new(
                    "syntax",
                    "a play file exports nothing — drop the `pub`; everything \
                     here is for this run"
                        .to_string(),
                    head_span(line),
                ));
            }
            // a top-level binding before any statement is a constant, the
            // library reading; after the first statement it is a statement
            Some((Tok::Ident(_), _))
                if first_stmt.is_none()
                    && line.tokens.get(1).is_some_and(|(t, _)| matches!(t, Tok::Bind)) => {}
            _ => {
                if first_stmt.is_none() {
                    first_stmt = Some(idx);
                }
            }
        }
    }
    let split = first_stmt.unwrap_or(lexed.lines.len());
    let decl_lines: Vec<Line> = lexed.lines[..split].to_vec();
    let stmt_lines: &[Line] = &lexed.lines[split..];
    if stmt_lines.is_empty() {
        diags.push(Diagnostic::new(
            "syntax",
            "a play file needs at least one statement to run".to_string(),
            Span { line: 1, col: 1 },
        ));
    }
    // the blank separating declarations from statements is a boundary, not
    // a trailing blank of the declaration half
    let decl_end = decl_lines.last().map(|l| l.number).unwrap_or(0);
    let decl_blanks: Vec<usize> =
        lexed.blank_lines.iter().copied().filter(|n| *n < decl_end).collect();
    let decl_half = Lexed { lines: decl_lines, blank_lines: decl_blanks };
    let mut program = match parse(&decl_half) {
        Ok(program) => program,
        Err(more) => {
            diags.extend(more);
            if !diags.is_empty() {
                return Err(diags);
            }
            unreachable!("parse failed without a diagnostic")
        }
    };
    // a continuation may not restart after a blank: the chain it would
    // splice into has already closed
    for line in stmt_lines {
        if line.indent != 0
            && lexed.blank_lines.contains(&(line.number - 1))
            && matches!(line.tokens.first(), Some((Tok::SeqOp | Tok::Pipe, _)))
        {
            diags.push(Diagnostic::new(
                "formatting",
                "a continuation may not follow a blank line — the statement                  it would splice into has closed"
                    .to_string(),
                head_span(line),
            ));
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
    program.fns.push(FnDecl {
        name: crate::ast::ENTRY.to_string(),
        params: Vec::new(),
        body,
        span,
        is_pub: false,
        file: String::new(),
        synthetic: false,
    });
    Ok(program)
}

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
        name: crate::ast::ENTRY.to_string(),
        params: Vec::new(),
        body,
        span,
        is_pub: false,
        file: String::new(),
        synthetic: false,
    };
    Ok(Program { fns: vec![main], types: Vec::new(), imports, reexports: Vec::new() })
}

pub fn parse(lexed: &Lexed) -> Result<Program, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut fns = Vec::new();
    let mut types = Vec::new();
    let mut imports: Vec<Import> = Vec::new();
    let mut reexports: Vec<crate::ast::Reexport> = Vec::new();
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
        while body_end < lexed.lines.len() && lexed.lines[body_end].indent >= 2 {
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
            // `pub name` / `pub theirs:yours` — a re-export, nothing else
            Some((Tok::Ident(_), _)) if head_idx == 1 => match parse_reexport(line, body) {
                Ok(reexport) => reexports.push(reexport),
                Err(d) => diags.push(d),
            },
            _ => diags.push(Diagnostic::new(
                "syntax",
                "a top-level line must begin with `fn`, `type`, or a constant binding".to_string(),
                head_span(line),
            )),
        }
        i = body_end;
    }
    if diags.is_empty() {
        Ok(Program { fns, types, imports, reexports })
    } else {
        Err(diags)
    }
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
        _ => Err(Diagnostic::new("syntax", "an import path is a plain string".to_string(), span)),
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

/// `pub name` re-exports one imported pub (or a whole module by its
/// qualifier); `pub theirs:yours` renames on the way out.
fn parse_reexport(line: &Line, body: &[Line]) -> Result<crate::ast::Reexport, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            "a re-export has no body".to_string(),
            head_span(&body[0]),
        ));
    }
    match line.tokens.as_slice() {
        [(Tok::KwPub, _), (Tok::Ident(name), span)] => {
            Ok(crate::ast::Reexport { name: name.clone(), rename: None, span: *span })
        }
        [(Tok::KwPub, _), (Tok::Ident(theirs), span), (Tok::Colon, _), (Tok::Ident(yours), _)] => {
            Ok(crate::ast::Reexport {
                name: theirs.clone(),
                rename: Some(yours.clone()),
                span: *span,
            })
        }
        _ => Err(Diagnostic::new(
            "syntax",
            "a re-export is `pub name` or `pub theirs:yours`".to_string(),
            head_span(line),
        )),
    }
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
        let blanks = lexed
            .blank_lines
            .iter()
            .filter(|b| **b > pair[0].number && **b < pair[1].number)
            .count();
        let both_imports = matches!(pair[0].tokens.first(), Some((Tok::KwImport, _)))
            && matches!(pair[1].tokens.first(), Some((Tok::KwImport, _)));
        let decl_start =
            matches!(pair[1].tokens.first(), Some((Tok::KwFn | Tok::KwType | Tok::KwPub, _)))
                || matches!(
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
    let (name, span) = match p.peek() {
        // an operator arm: `fn + a:user b:user` extends the operator's
        // dispatch group for a type you own
        Some(Tok::Op(op @ ("+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "=="))) => {
            let op = op.to_string();
            let span = p.span_here();
            p.pos += 1;
            (op, span)
        }
        _ => p.expect_ident("a function name")?,
    };
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
        return Err(Diagnostic::new("syntax", format!("function `{name}` has no body"), span));
    }
    let stmts = parse_body(body)?;
    Ok(FnDecl { name, is_pub, span, params, body: stmts, file: String::new(), synthetic: false })
}

fn parse_constant(header: &Line, body: &[Line]) -> Result<FnDecl, Diagnostic> {
    let is_pub = matches!(header.tokens.first(), Some((Tok::KwPub, _)));
    let off = usize::from(is_pub);
    let Some((Tok::Ident(name), span)) = header.tokens.get(off) else {
        return Err(Diagnostic::new(
            "syntax",
            "expected a constant name".to_string(),
            head_span(header),
        ));
    };
    let name = name.clone();
    let span = *span;
    if header.tokens.len() == off + 2 {
        if body.is_empty() {
            return Err(Diagnostic::new("syntax", format!("constant `{name}` has no value"), span));
        }
        if body.len() == 1 {
            return Err(Diagnostic::new(
                "formatting",
                format!("a single-expression constant is written inline: `{name} = ...`"),
                span,
            ));
        }
        let stmts = parse_body(body)?;
        return Ok(FnDecl {
            name,
            is_pub,
            span,
            params: Vec::new(),
            body: stmts,
            file: String::new(),
            synthetic: false,
        });
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
    Ok(FnDecl {
        name,
        is_pub,
        span,
        params: Vec::new(),
        body: vec![Stmt::Expr(expr)],
        file: String::new(),
        synthetic: false,
    })
}

fn parse_type(header: &Line, body: &[Line]) -> Result<TypeDecl, Diagnostic> {
    let mut p = P::new(&header.tokens, &header.end_cols, header.number);
    let is_pub = p.consume_pub();
    p.expect_kw_type()?;
    let (name, span) = p.expect_ident("a type name")?;
    if !p.done() {
        let (parent, parent_span) = p.expect_ident("a parent type")?;
        let mut members = vec![parent.clone()];
        while !p.done() {
            members.push(p.expect_ident("a member type")?.0);
        }
        if !body.is_empty() {
            return Err(Diagnostic::new(
                "syntax",
                "a subtype or typeset declaration is one line; fields belong \
                 to records"
                    .to_string(),
                parent_span,
            ));
        }
        if members.len() >= 2 {
            let mut sorted = members.clone();
            sorted.sort();
            if sorted != members {
                return Err(Diagnostic::new(
                    "formatting",
                    "typeset members appear in alphabetical order".to_string(),
                    parent_span,
                ));
            }
            return Ok(TypeDecl {
                name,
                is_pub,
                span,
                synthetic: false,
                origin: None,
                parent: None,
                members,
                fields: Vec::new(),
            });
        }
        return Ok(TypeDecl {
            name,
            is_pub,
            span,
            synthetic: false,
            origin: None,
            parent: Some(parent),
            members: Vec::new(),
            fields: Vec::new(),
        });
    }
    p.expect_done()?;
    let fields = body.iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;
    Ok(TypeDecl {
        name,
        is_pub,
        span,
        synthetic: false,
        origin: None,
        parent: None,
        members: Vec::new(),
        fields,
    })
}

fn parse_field(line: &Line) -> Result<(String, Vec<String>, Span), Diagnostic> {
    let mut p = P::new(&line.tokens, &line.end_cols, line.number);
    let (name, span) = p.expect_ident("a field name")?;
    // an unannotated field is unconstrained, the same thing an unnamed
    // parameter's absent annotation says
    if p.done() {
        return Ok((name, Vec::new(), span));
    }
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
/// The guard section: leading bindings and `return X if C` lines. A return
/// folds everything after it into the untaken branch of a compiler-built
/// conditional — the body below a fired guard is unreachable, not skipped.
fn parse_body(body: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let is_return = |line: &Line| matches!(line.tokens.first(), Some((Tok::KwReturn, _)));
    // a construct owns the deeper lines beneath it, so the leading run is
    // walked at the top indent and skips past a block's body
    // each unit is a leading line plus whatever it owns beneath it, so a
    // `build` or `if` header travels with its body rather than alone
    let mut units: Vec<std::ops::Range<usize>> = Vec::new();
    let lead_end = {
        let mut i = 0;
        loop {
            if i >= body.len() {
                break i;
            }
            let leads =
                is_return(&body[i]) || matches!(parse_stmt(&body[i]), Ok(Stmt::Bind { .. }));
            if !leads {
                break i;
            }
            let start = i;
            let base = body[i].indent;
            i += 1;
            while i < body.len() && body[i].indent > base {
                i += 1;
            }
            // an `else` at the header's own indent belongs to the block above
            let is_else_line =
                |l: &Line| matches!(l.tokens.as_slice(), [(Tok::Ident(w), _)] if w == "else");
            if body.get(i).is_some_and(|l| l.indent == base && is_else_line(l)) {
                i += 1;
                while i < body.len() && body[i].indent > base {
                    i += 1;
                }
            }
            units.push(start..i);
        }
    };
    if let Some(stray) = body[lead_end..].iter().find(|l| is_return(l)) {
        return Err(Diagnostic::new(
            "formatting",
            "a `return` sits with the bindings, before the effect chain".to_string(),
            stray.tokens[0].1,
        ));
    }
    if !body[..lead_end].iter().any(is_return) {
        return parse_effect_body(&body[lead_end..], &body[..lead_end]);
    }
    let mut cont = parse_effect_body(&body[lead_end..], &[])?;
    for unit in units.iter().rev() {
        let lines = &body[unit.clone()];
        let line = &lines[0];
        if !is_return(line) {
            let stmts = parse_lead_stmts(lines)?;
            cont.splice(0..0, stmts);
            continue;
        }
        let (cond, early, span) = parse_return(line)?;
        if cont.is_empty() {
            return Err(Diagnostic::new(
                "syntax",
                "nothing follows this `return` — the body needs a result for \
                 when the condition does not fire"
                    .to_string(),
                span,
            ));
        }
        let rest = std::mem::take(&mut cont);
        let guard = Expr::Guard { cond: Box::new(cond), early: Box::new(early), rest, span };
        cont = vec![Stmt::Expr(guard)];
    }
    Ok(cont)
}

/// `return X if C`: X and C split at the line's single top-level `if`.
fn parse_return(line: &Line) -> Result<(Expr, Expr, Span), Diagnostic> {
    let span = line.tokens[0].1;
    let mut depth = 0usize;
    let mut splits = Vec::new();
    for (i, (tok, _)) in line.tokens.iter().enumerate().skip(1) {
        match tok {
            Tok::LParen | Tok::LGroup | Tok::LBracket | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RGroup | Tok::RBracket | Tok::RBrace => {
                depth = depth.saturating_sub(1)
            }
            Tok::Ident(name) if depth == 0 && name == "if" => splits.push(i),
            _ => {}
        }
    }
    let [at] = splits.as_slice() else {
        let message = match splits.is_empty() {
            true => "an unconditional result is the final expression — drop the `return`",
            false => "one `if` decides a return — parenthesize any inner `if`",
        };
        return Err(Diagnostic::new("formatting", message.to_string(), span));
    };
    let mut early_p = P::new(&line.tokens[1..*at], &line.end_cols[1..*at], line.number);
    let early = early_p.parse_expr()?;
    early_p.expect_done()?;
    let mut cond_p = P::new(&line.tokens[*at + 1..], &line.end_cols[*at + 1..], line.number);
    let cond = cond_p.parse_expr()?;
    cond_p.expect_done()?;
    Ok((cond, early, span))
}

fn parse_effect_body(body: &[Line], lead_binds: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let mut stmts: Vec<Stmt> = parse_lead_stmts(lead_binds)?;
    let tail = parse_effect_tail(body)?;
    stmts.extend(tail);
    Ok(stmts)
}

/// The leading bindings, where a `build` or `if` header owns the indented
/// lines beneath it exactly as it does in the effect tail — including the
/// `else` at its own indent and the stray-continuation fallback.
fn parse_lead_stmts(body: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let is_else =
        |line: &Line| matches!(line.tokens.as_slice(), [(Tok::Ident(w), _)] if w == "else");
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < body.len() {
        let base = body[idx].indent;
        let mut j = idx + 1;
        while j < body.len() && body[j].indent > base {
            j += 1;
        }
        if j == idx + 1 {
            out.push(parse_stmt(&body[idx])?);
            idx = j;
            continue;
        }
        let children = &body[idx + 1..j];
        let head_is_block = matches!(body[idx].tokens.as_slice(), [(Tok::Ident(w), _), ..] if w == "if" || w == "build")
            || matches!(
                body[idx].tokens.as_slice(),
                [(Tok::Ident(_), _), (Tok::Bind, _), (Tok::Ident(w), _), ..] if w == "if" || w == "build"
            );
        if !head_is_block
            && children
                .iter()
                .all(|c| matches!(c.tokens.first(), Some((Tok::SeqOp | Tok::Pipe, _))))
        {
            out.push(parse_stmt(&body[idx])?);
            idx += 1;
            continue;
        }
        let (else_children, end) =
            match j < body.len() && body[j].indent == base && is_else(&body[j]) {
                true => {
                    let mut k = j + 1;
                    while k < body.len() && body[k].indent > base {
                        k += 1;
                    }
                    (Some(&body[j + 1..k]), k)
                }
                false => (None, j),
            };
        out.push(parse_block_construct(&body[idx], children, else_children)?);
        idx = end;
    }
    Ok(out)
}

fn parse_effect_tail(body: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let is_wall = |line: &Line| matches!(line.tokens.as_slice(), [(Tok::SeqOp, _)]);
    let is_else =
        |line: &Line| matches!(line.tokens.as_slice(), [(Tok::Ident(w), _)] if w == "else");
    // group lines into units: a wall line, or a statement that may own the
    // deeper lines under it (an if/else block construct)
    enum Unit<'a> {
        Wall(&'a Line),
        Parsed(Stmt),
    }
    let mut units: Vec<Unit> = Vec::new();
    let mut idx = 0;
    while idx < body.len() {
        let line = &body[idx];
        let base = line.indent;
        let mut j = idx + 1;
        while j < body.len() && body[j].indent > base {
            j += 1;
        }
        if is_else(line) {
            return Err(Diagnostic::new(
                "syntax",
                "`else` needs an `if` block directly above it".to_string(),
                head_span(line),
            ));
        }
        if j == idx + 1 {
            match is_wall(line) || matches!(line.tokens.first(), Some((Tok::SeqOp, _))) {
                true => units.push(Unit::Wall(line)),
                false => units.push(Unit::Parsed(parse_stmt(line)?)),
            }
            idx = j;
            continue;
        }
        let children = &body[idx + 1..j];
        // chain-led deeper lines under a non-`if` head are stray
        // continuations, diagnosed elsewhere — keep them flat units so the
        // real diagnostic stands alone
        let head_is_if = matches!(body[idx].tokens.as_slice(), [(Tok::Ident(w), _), ..] if w == "if")
            || matches!(
                body[idx].tokens.as_slice(),
                [(Tok::Ident(_), _), (Tok::Bind, _), (Tok::Ident(w), _), ..] if w == "if"
            );
        if !head_is_if
            && children
                .iter()
                .all(|c| matches!(c.tokens.first(), Some((Tok::SeqOp | Tok::Pipe, _))))
        {
            units.push(Unit::Parsed(parse_stmt(line)?));
            idx += 1;
            continue;
        }
        let (else_children, end) =
            match j < body.len() && body[j].indent == base && is_else(&body[j]) {
                true => {
                    let mut k = j + 1;
                    while k < body.len() && body[k].indent > base {
                        k += 1;
                    }
                    if k == j + 1 {
                        return Err(Diagnostic::new(
                            "syntax",
                            "`else` opens a branch: indent its statements beneath it".to_string(),
                            head_span(&body[j]),
                        ));
                    }
                    (Some(&body[j + 1..k]), k)
                }
                false => (None, j),
            };
        units.push(Unit::Parsed(parse_block_construct(line, children, else_children)?));
        idx = end;
    }
    let has_surface = units.iter().enumerate().any(|(i, u)| match u {
        Unit::Wall(_) => true,
        Unit::Parsed(Stmt::Expr(_)) => i + 1 < units.len(),
        Unit::Parsed(_) => false,
    });
    if !has_surface {
        return Ok(units
            .into_iter()
            .map(|u| match u {
                Unit::Parsed(stmt) => stmt,
                Unit::Wall(_) => unreachable!("walls imply surface"),
            })
            .collect());
    }
    let mut binds: Vec<Stmt> = Vec::new();
    let mut segments: Vec<Vec<Expr>> = vec![Vec::new()];
    let mut wall_spans: Vec<Span> = Vec::new();
    let mut wall_fused: Vec<bool> = Vec::new();
    let mut closed_by_fuse = false;
    let unit_count = units.len();
    let mut unit_index = 0usize;
    for unit in units {
        unit_index += 1;
        let is_final_unit = unit_index == unit_count;
        let line = match unit {
            Unit::Wall(line) => line,
            Unit::Parsed(stmt) => {
                match stmt {
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
                        reject_never_effect(&e, is_final_unit)?;
                        segments.last_mut().expect("segment").push(e);
                    }
                    Stmt::Set { .. } => unreachable!("`set` lifts only inside `build`"),
                }
                continue;
            }
        };
        let fused = matches!(line.tokens.first(), Some((Tok::SeqOp, _))) && line.tokens.len() > 1;
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
        match fused {
            true => {
                // `>> expr` is a COMPLETE sequential step: wall plus its one
                // member, closed — nothing may silently join it
                let mut p = P::new(&line.tokens[1..], &line.end_cols[1..], line.number);
                let expr = p.parse_expr()?;
                p.expect_done()?;
                reject_never_effect(&expr, is_final_unit)?;
                segments.last_mut().expect("segment").push(expr);
                closed_by_fuse = true;
            }
            false => closed_by_fuse = false,
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
                "a one-step stage fuses with its wall: write `>> step` on one line".to_string(),
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

/// An `if` (or `x = if`) whose branch lines sit indented beneath it. With an
/// `else`, the branches are blocks — fn-body statements, deferred; without
/// one, each child line is one more argument, exactly as any indented call.
fn parse_block_construct(
    head: &Line,
    children: &[Line],
    else_children: Option<&[Line]>,
) -> Result<Stmt, Diagnostic> {
    let head_is_if = matches!(head.tokens.as_slice(), [(Tok::Ident(w), _), ..] if w == "if")
        || matches!(
            head.tokens.as_slice(),
            [(Tok::Ident(_), _), (Tok::Bind, _), (Tok::Ident(w), _), ..] if w == "if"
        );
    let head_is_build = matches!(head.tokens.as_slice(), [(Tok::Ident(w), _)] if w == "build")
        || matches!(
            head.tokens.as_slice(),
            [(Tok::Ident(_), _), (Tok::Bind, _), (Tok::Ident(w), _)] if w == "build"
        );
    if head_is_build {
        return parse_build(head, children, else_children);
    }
    if !head_is_if {
        return Err(Diagnostic::new(
            "syntax",
            "only `if` and `build` open an indented block; other calls take \
             indented arguments on the line's own indent plus two"
                .to_string(),
            head_span(head),
        ));
    }
    let stmt = parse_stmt(head)?;
    let extend = |expr: Expr, args: Vec<Expr>| -> Result<Expr, Diagnostic> {
        let Expr::App { head: h, args: mut a, span, piped } = expr else {
            return Err(Diagnostic::new(
                "syntax",
                "an `if` block header is `if condition`".to_string(),
                span_of_stmt_head(head),
            ));
        };
        if a.len() != 1 {
            return Err(Diagnostic::new(
                "syntax",
                "an `if` block header holds the condition alone; branches sit \
                 beneath it"
                    .to_string(),
                span_of_stmt_head(head),
            ));
        }
        a.extend(args);
        Ok(Expr::App { head: h, args: a, span, piped })
    };
    let branch_args = match else_children {
        None => {
            let mut args = Vec::new();
            for child in children {
                if child.tokens.iter().any(|(t, _)| matches!(t, Tok::Bind)) {
                    return Err(Diagnostic::new(
                        "formatting",
                        "a branch that binds names needs the block form: put \
                         `else` at the `if`'s indent and a block beneath each"
                            .to_string(),
                        head_span(child),
                    ));
                }
                let mut p = P::new(&child.tokens, &child.end_cols, child.number);
                let expr = p.parse_expr()?;
                p.expect_done()?;
                args.push(expr);
            }
            args
        }
        Some(else_lines) => {
            let then_stmts = parse_body(children)?;
            let else_stmts = parse_body(else_lines)?;
            for (stmts, lines) in [(&then_stmts, children), (&else_stmts, else_lines)] {
                if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
                    return Err(Diagnostic::new(
                        "syntax",
                        "a branch ends with its result expression, not a binding".to_string(),
                        head_span(lines.last().expect("branches are non-empty")),
                    ));
                }
            }
            let plain =
                |stmts: &[Stmt]| stmts.len() == 1 && matches!(stmts.first(), Some(Stmt::Expr(_)));
            if plain(&then_stmts) && plain(&else_stmts) {
                return Err(Diagnostic::new(
                    "formatting",
                    "branches without bindings use the expression form: \
                     `if cond a b`, or indented arguments"
                        .to_string(),
                    head_span(head),
                ));
            }
            let tspan = head_span(children.first().expect("non-empty"));
            let espan = head_span(else_lines.first().expect("non-empty"));
            vec![Expr::Block(then_stmts, tspan), Expr::Block(else_stmts, espan)]
        }
    };
    match stmt {
        Stmt::Expr(expr) => Ok(Stmt::Expr(extend(expr, branch_args)?)),
        Stmt::Bind { pattern, expr } => {
            Ok(Stmt::Bind { pattern, expr: extend(expr, branch_args)? })
        }
        Stmt::Set { .. } => unreachable!("`set` lifts only inside `build`"),
    }
}

fn span_of_stmt_head(line: &Line) -> Span {
    head_span(line)
}

/// The body runs top to bottom and its last expression is the result.
fn parse_build(
    head: &Line,
    children: &[Line],
    else_children: Option<&[Line]>,
) -> Result<Stmt, Diagnostic> {
    if else_children.is_some() {
        return Err(Diagnostic::new(
            "syntax",
            "`build` has no `else` — it is a construction site, not a branch".to_string(),
            head_span(head),
        ));
    }
    let stmts = parse_build_body(children)?;
    if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
        return Err(Diagnostic::new(
            "syntax",
            "a `build` ends with its result expression — the value that \
             freezes and leaves the block"
                .to_string(),
            head_span(children.last().unwrap_or(head)),
        ));
    }
    let build = Expr::Build(stmts, head_span(head));
    match head.tokens.as_slice() {
        [(Tok::Ident(name), nspan), (Tok::Bind, _), _] => {
            Ok(Stmt::Bind { pattern: Pattern::Var(name.clone(), *nspan), expr: build })
        }
        _ => Ok(Stmt::Expr(build)),
    }
}

fn parse_build_body(body: &[Line]) -> Result<Vec<Stmt>, Diagnostic> {
    let is_else =
        |line: &Line| matches!(line.tokens.as_slice(), [(Tok::Ident(w), _)] if w == "else");
    let mut stmts = Vec::new();
    let mut idx = 0;
    while idx < body.len() {
        let line = &body[idx];
        if matches!(line.tokens.first(), Some((Tok::SeqOp, _))) {
            return Err(Diagnostic::new(
                "syntax",
                "a `build` body already runs top to bottom; `>>` walls have \
                 no place here"
                    .to_string(),
                head_span(line),
            ));
        }
        let base = line.indent;
        let mut j = idx + 1;
        while j < body.len() && body[j].indent > base {
            j += 1;
        }
        if j == idx + 1 {
            stmts.push(parse_stmt(line)?);
            idx = j;
            continue;
        }
        let inner = &body[idx + 1..j];
        let (else_lines, end) = match j < body.len() && body[j].indent == base && is_else(&body[j])
        {
            true => {
                let mut k = j + 1;
                while k < body.len() && body[k].indent > base {
                    k += 1;
                }
                (Some(&body[j + 1..k]), k)
            }
            false => (None, j),
        };
        stmts.push(parse_block_construct(line, inner, else_lines)?);
        idx = end;
    }
    Ok(stmts)
}

/// A bare line in an effect group must at least plausibly be a description.
/// Literals, arithmetic, comparisons, and lambdas never are — those keep the
/// classic unused-expression error instead of dying inside the runtime join.
fn reject_never_effect(e: &Expr, is_final: bool) -> Result<(), Diagnostic> {
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
        let message = match is_final {
            true => {
                "a function that does io must return the io (or an err) — this \
                 trailing value would abandon the effects above it. an io's \
                 yield flows onward through `.`; a plain value result belongs \
                 in a pure function"
            }
            false => {
                "this value is never used: a non-final line binds a name, or is \
                 an effect joining the group"
            }
        };
        return Err(Diagnostic::new("unused", message.to_string(), expr_span(e)));
    }
    Ok(())
}

/// `a && b` is `if a b false`; `a || b` is `if a true b` — the operators
/// are spelling, the deferred if family is the semantics.
fn logical_if(cond: Expr, then_e: Expr, else_e: Expr, span: Span) -> Expr {
    Expr::App {
        head: Box::new(Expr::Ident("if".to_string(), span)),
        args: vec![cond, then_e, else_e],
        span,
        piped: false,
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, s)
        | Expr::Partial(_, s)
        | Expr::Field { span: s, .. }
        | Expr::Upcast { span: s, .. }
        | Expr::Build(_, s)
        | Expr::Float(_, s)
        | Expr::MapLit(_, s)
        | Expr::Str(_, s)
        | Expr::Ident(_, s)
        | Expr::List(_, s)
        | Expr::Seq(_, _, s)
        | Expr::Join { span: s, .. }
        | Expr::Block(_, s)
        | Expr::Guard { span: s, .. }
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
            Tok::LParen | Tok::LGroup | Tok::LBracket | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RGroup | Tok::RBracket | Tok::RBrace => {
                depth = depth.saturating_sub(1)
            }
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
    // `a.next = b` writes a field. Mutation lives in a build block and nowhere
    // else, which the checker enforces exactly as it does for the older form.
    if let [(Tok::Ident(target), _), (Tok::Dot, _), (Tok::Ident(field), span)] = &line.tokens[..i] {
        let mut rhs = P::new(&line.tokens[i + 1..], &line.end_cols[i + 1..], line.number);
        let value = rhs.parse_expr()?;
        rhs.expect_done()?;
        return Ok(Stmt::Set { target: target.clone(), field: field.clone(), value, span: *span });
    }
    let mut lhs = P::new(&line.tokens[..i], &line.end_cols[..i], line.number);
    let pattern = lhs.parse_bind_target()?;
    lhs.expect_done()?;
    let mut rhs = P::new(&line.tokens[i + 1..], &line.end_cols[i + 1..], line.number);
    let expr = rhs.parse_expr()?;
    rhs.expect_done()?;
    Ok(Stmt::Bind { pattern, expr })
}

/// How loosely an expression may bind, tightest last. Canonical form is one
/// rendering per program, so a paren pair that groups nothing is a second
/// rendering of the same thing and the grammar rejects it.
const LOOSEST: u8 = 0;
const OR: u8 = 1;
const AND: u8 = 2;
/// `not` claims one operand and binds tighter than the connectives, so the
/// parentheses in `not (a and b)` are load-bearing and the ones in
/// `(not a) and b` are not.
const NOT: u8 = 3;
const CMP: u8 = 4;
/// Bitwise binds tighter than comparison, which is the fix for C's famous
/// mistake — `a & b == c` there means `a & (b == c)`, and nobody has ever
/// wanted that. Looser than arithmetic, so `a & b + 1` adds first.
const BITS: u8 = 5;
const ADD: u8 = 6;
const MUL: u8 = 7;
const APP: u8 = 8;
const ATOM: u8 = 9;

/// What a pair is worth is decided by the two tokens beside it. On the right,
/// an operator claims the parenthesised expression as its left operand, so the
/// pair earns its place only when what it wraps binds more loosely than that
/// operator. Adjacency is application, which binds tightest of all.
fn tolerated_after(tok: Option<&Tok>) -> u8 {
    match tok {
        // a comparison does not chain, so unlike the left-associative
        // operators its left operand may not itself be one
        Some(Tok::Op(op)) if level(op) == CMP => CMP + 1,
        Some(Tok::Op(op)) => level(op),
        Some(Tok::Dot | Tok::LBracket) => ATOM,
        Some(tok) if starts_an_atom(tok) => ATOM,
        _ => LOOSEST,
    }
}

/// On the left, the pair is that operator's *right* operand, and every
/// operator here associates left — so the right side holds one level tighter,
/// which is what makes `10 - (4 - 1)` keep its parentheses.
fn tolerated_before(tok: Option<&Tok>) -> u8 {
    match tok {
        Some(Tok::Op(op)) => level(op) + 1,
        Some(tok) if ends_an_atom(tok) => ATOM,
        Some(Tok::SeqOp | Tok::Pipe) => OR,
        // A container element and a map's value are each exactly one atom —
        // `[one 1]` is two elements and `{ "k":one 1 }` does not parse — so
        // parentheses there are the only way to write anything else, and
        // calling them superfluous makes the value unwritable.
        Some(Tok::LBracket | Tok::LGroup | Tok::Colon) => ATOM,
        _ => LOOSEST,
    }
}

/// Adjacency is application, so anything that can begin an atom claims what
/// follows it as an argument.
fn starts_an_atom(tok: &Tok) -> bool {
    matches!(
        tok,
        Tok::Ident(_)
            | Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::Underscore
            | Tok::LParen
            | Tok::LGroup
            | Tok::LBracket
            | Tok::LBrace
    )
}

/// The mirror: anything an atom can end with. A closing bracket belongs here
/// as much as a name does — `step [x y] (paired l r)` passes two arguments,
/// and reading the `[x y]` as a statement boundary turns it into four.
fn ends_an_atom(tok: &Tok) -> bool {
    matches!(
        tok,
        Tok::Ident(_)
            | Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::RParen
            | Tok::RGroup
            | Tok::RBracket
            | Tok::RBrace
            | Tok::Bang
    )
}

fn level(op: &str) -> u8 {
    match op {
        "or" => OR,
        "and" => AND,
        "not" => NOT,
        "<" | "<=" | ">" | ">=" | "==" | "!=" => CMP,
        "&" | "|" | "^" => BITS,
        "+" | "-" => ADD,
        "*" | "/" | "%" => MUL,
        _ => LOOSEST,
    }
}

pub struct P<'a> {
    toks: &'a [(Tok, Span)],
    ends: &'a [usize],
    pub pos: usize,
    line: usize,
    /// The loosest operator consumed since the enclosing `(`. A paren scope
    /// saves and restores it, so a nested pair is judged on its own contents.
    loosest: u8,
}

impl<'a> P<'a> {
    fn tok_at(&self, i: usize) -> Option<&Tok> {
        self.toks.get(i).map(|(tok, _)| tok)
    }

    /// An operator was taken at this level; the loosest one inside the
    /// current parentheses is what decides whether they did anything.
    fn consumed(&mut self, level: u8) {
        self.loosest = self.loosest.min(level);
    }

    pub fn new(toks: &'a [(Tok, Span)], ends: &'a [usize], line: usize) -> Self {
        P { toks, ends, pos: 0, line, loosest: ATOM }
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
        if self.done() {
            return Ok(());
        }
        // A lambda is always parenthesised, so a bare one leaves its arrow
        // where nothing can take it. Saying only that something is left over
        // sends the reader looking at the body.
        let message = match self.toks.get(self.pos).map(|(t, _)| t) {
            Some(Tok::Arrow) => {
                "a lambda is parenthesised: `f = (x -> …)`, not `f = x -> …`".to_string()
            }
            _ => "unexpected trailing tokens".to_string(),
        };
        Err(self.err(message))
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

    /* design/type-syntax.md ratifies Go's prefix slice — `[]T` reads left to
    right and composes without backtracking, where postfix `T[]` reads
    inside-out. A map is an application, `map[K V]`, which needs one rule
    where Go's `map[K]V` needs a second that only `map` can use.

    Both spellings fold into the same internal name the postfix forms
    already produce, so nothing downstream learns a new shape: `[]int`
    becomes `int[]`, and `map[string int]` keeps its brackets. */
    fn parse_type_expr(&mut self) -> Result<String, Diagnostic> {
        if matches!(self.peek(), Some(Tok::LBracket)) {
            self.pos += 1;
            match self.peek() {
                Some(Tok::RBracket) => self.pos += 1,
                _ => return Err(self.err("expected `]` — a slice is `[]T`".to_string())),
            }
            let inner = self.parse_type_expr()?;
            return Ok(format!("{inner}[]"));
        }
        let (mut ty, _) = self.expect_ident("a type")?;
        if ty == "map" && matches!(self.peek(), Some(Tok::LBracket)) {
            self.pos += 1;
            let key = self.parse_type_expr()?;
            let val = self.parse_type_expr()?;
            match self.peek() {
                Some(Tok::RBracket) => self.pos += 1,
                _ => return Err(self.err("expected `]` — a map is `map[K V]`".to_string())),
            }
            return Ok(format!("map[{key} {val}]"));
        }
        while matches!(self.peek(), Some(Tok::LBracket)) {
            self.pos += 1;
            match self.peek() {
                // The postfix slice, which the design rules out and the parser
                // accepted anyway — so a slice had two spellings in a language
                // whose position is that the canonical form is the grammar.
                Some(Tok::RBracket) => {
                    return Err(self.err(format!(
                        "a slice is `[]{ty}`, written before the type — `{ty}[]` reads \
                         inside-out as soon as anything nests"
                    )));
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
                // `_:type` dispatches on membership and binds nothing — the
                // spelling for an arm that needs the type and not the value
                if matches!(self.peek(), Some(Tok::Colon)) {
                    let colon_span = self.span_here();
                    let tight_before = colon_span.col == span.col + 1;
                    self.pos += 1;
                    let ty_span = self.span_here();
                    let tight_after = ty_span.col == colon_span.col + 1;
                    if !tight_before || !tight_after {
                        return Err(Diagnostic::new(
                            "formatting",
                            "type ascription is tight: `_:type`".to_string(),
                            colon_span,
                        ));
                    }
                    let ty = self.parse_type_expr()?;
                    return Ok(Pattern::Annotated { name: "_".to_string(), ty, span });
                }
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
        self.expect_close(true)
    }

    /// A group closes with the door it opened: an author's `)` for an
    /// author's `(`, and the lexer's for the lexer's. Crossing them would let
    /// an unclosed parenthesis be swallowed by the end of a continuation line.
    fn expect_close(&mut self, written: bool) -> Result<(), Diagnostic> {
        let wanted = match written {
            true => Tok::RParen,
            false => Tok::RGroup,
        };
        match self.peek() == Some(&wanted) {
            true => {
                self.pos += 1;
                Ok(())
            }
            // `=` binds a name and every C-shaped language spells comparison
            // with it at least sometimes, so say which one is wanted rather
            // than which bracket is missing.
            false => match self.peek() {
                Some(Tok::Bind) => Err(self
                    .err("`=` binds a name; `==` asks whether two values are equal".to_string())),
                _ => Err(self.err("expected `)`".to_string())),
            },
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
                    if target_span.col != colon_span.col + 1 {
                        return Err(Diagnostic::new(
                            "formatting",
                            "a rename is tight: `field:name`".to_string(),
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
                        atom => {
                            Expr::App { head: Box::new(atom), args: vec![expr], span, piped: true }
                        }
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
        let lhs = self.parse_or()?;
        Ok(lhs)
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_and()?;
        while let Some(Tok::Op("or")) = self.peek() {
            let span = self.span_here();
            self.pos += 1;
            self.consumed(OR);
            let rhs = self.parse_and()?;
            lhs = logical_if(lhs, Expr::Ident("true".to_string(), span), rhs, span);
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_not()?;
        while let Some(Tok::Op("and")) = self.peek() {
            let span = self.span_here();
            self.pos += 1;
            self.consumed(AND);
            let rhs = self.parse_not()?;
            lhs = logical_if(lhs, rhs, Expr::Ident("false".to_string(), span), span);
        }
        Ok(lhs)
    }

    /// `not` binds tighter than `and` and `or` and looser than a comparison,
    /// so `not a and b` denies only `a` and `not a == b` denies the whole
    /// comparison. It answers the other branch of the same question `and` and
    /// `or` ask, so it is written the same way and no engine sees a new node.
    fn parse_not(&mut self) -> Result<Expr, Diagnostic> {
        let Some(Tok::Op("not")) = self.peek() else {
            return self.parse_cmp();
        };
        let span = self.span_here();
        self.pos += 1;
        self.consumed(NOT);
        let inner = self.parse_not()?;
        let yes = Expr::Ident("false".to_string(), span);
        let no = Expr::Ident("true".to_string(), span);
        Ok(logical_if(inner, yes, no, span))
    }

    fn parse_cmp(&mut self) -> Result<Expr, Diagnostic> {
        let lhs = self.parse_bits()?;
        let cmp = ["<", "<=", ">", ">=", "==", "!="];
        if let Some(Tok::Op(op)) = self.peek() {
            if cmp.contains(op) {
                let op = *op;
                let span = self.span_here();
                self.pos += 1;
                self.consumed(CMP);
                let rhs = self.parse_add()?;
                return Ok(Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span });
            }
        }
        Ok(lhs)
    }

    /// `&`, `|` and `^` over whole numbers. The glyphs are free here because
    /// space is semantic: `&add` hugs its name and is the partial sigil, which
    /// the atom parser reads; a spaced `&` can only be this.
    fn parse_bits(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_add()?;
        while let Some(Tok::Op(op @ ("&" | "|" | "^"))) = self.peek() {
            let op = *op;
            let span = self.span_here();
            self.pos += 1;
            self.consumed(BITS);
            let rhs = self.parse_add()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_mul()?;
        while let Some(Tok::Op(op @ ("+" | "-"))) = self.peek() {
            let op = *op;
            let span = self.span_here();
            self.pos += 1;
            self.consumed(ADD);
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_app()?;
        while let Some(Tok::Op(op @ ("*" | "/" | "%"))) = self.peek() {
            let op = *op;
            let span = self.span_here();
            self.pos += 1;
            self.consumed(MUL);
            let rhs = self.parse_app()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn starts_atom(&self) -> bool {
        // `_.name` is an atom; a bare `_` is not, so the pipe hole and the
        // wildcard pattern keep the meanings they already have
        if matches!(self.peek(), Some(Tok::Underscore))
            && matches!(self.toks.get(self.pos + 1).map(|(t, _)| t), Some(Tok::Dot))
        {
            return true;
        }
        matches!(
            self.peek(),
            Some(
                Tok::Ident(_)
                    | Tok::Int(_)
                    | Tok::Float(_)
                    | Tok::Str(_)
                    | Tok::LParen
                    | Tok::LGroup
                    | Tok::LBracket
                    | Tok::LBrace
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
                self.consumed(APP);
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
                let Some(Tok::Ident(name)) = self.toks.get(self.pos).map(|(t, _)| t.clone()) else {
                    return Err(self.err("a field name follows the dot".to_string()));
                };
                self.pos += 1;
                expr = Expr::Field { base: Box::new(expr), name, span };
                continue;
            }
            // `foo()` runs a value waiting to be called — the complement of
            // `&`, which supplies without running. It hugs its value the way
            // an index does, and it is empty by construction: a parenthesis
            // with something in it is the C-shaped call the lexer refuses.
            let runs = matches!(self.peek(), Some(Tok::LParen))
                && self.span_here().col == self.last_end()
                && matches!(self.toks.get(self.pos + 1).map(|(t, _)| t), Some(Tok::RParen));
            if runs {
                let span = self.span_here();
                self.pos += 2;
                expr = Expr::App { head: Box::new(expr), args: Vec::new(), piped: false, span };
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
        // `_.name` is the read with its record left out — the accessor as a
        // value, so `list/map people _.name` works. The name it denotes is
        // not one a program can write, which is what keeps a field from
        // taking a name away from anything else.
        if matches!(self.peek(), Some(Tok::Underscore))
            && matches!(self.toks.get(self.pos + 1).map(|(t, _)| t), Some(Tok::Dot))
        {
            if let Some((Tok::Ident(field), _)) = self.toks.get(self.pos + 2).cloned() {
                self.pos += 3;
                return Ok(Expr::Ident(crate::ast::getter_name(&field), span));
            }
        }
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
            Some(Tok::Op("&")) => {
                self.pos += 1;
                match self.toks.get(self.pos).map(|(t, _)| t.clone()) {
                    Some(Tok::Ident(name)) => {
                        self.pos += 1;
                        Ok(Expr::Partial(name, span))
                    }
                    _ => Err(self.err("`&` marks a partial application: `&name arg`".to_string())),
                }
            }
            Some(Tok::Str(parts)) => {
                self.pos += 1;
                let template = parts
                    .iter()
                    .map(|part| template_part(part, self.line))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expr::Str(template, span))
            }
            Some(Tok::LBrace) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    self.pos += 1;
                    match self.peek() {
                        Some(Tok::RBrace) => {
                            self.pos += 1;
                            return Ok(Expr::MapLit(Vec::new(), span));
                        }
                        _ => return Err(self.err("`{:}` is the empty map".to_string())),
                    }
                }
                let mut pairs = Vec::new();
                let mut key = self.parse_atom()?;
                loop {
                    self.require_literal_key(&key)?;
                    let colon_span = self.span_here();
                    match self.peek() {
                        Some(Tok::Colon) => self.pos += 1,
                        _ => return Err(self.err("expected `:` after a map key".to_string())),
                    }
                    let value_span = self.span_here();
                    if value_span.col != colon_span.col + 1 {
                        return Err(Diagnostic::new(
                            "formatting",
                            "a map pair is tight: `key:value`".to_string(),
                            colon_span,
                        ));
                    }
                    let value = self.parse_atom()?;
                    pairs.push((key, value));
                    if matches!(self.peek(), Some(Tok::RBrace)) {
                        self.pos += 1;
                        self.check_key_order(&pairs)?;
                        return Ok(Expr::MapLit(pairs, span));
                    }
                    key = self.parse_atom()?;
                }
            }
            Some(Tok::LBracket) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    return Err(
                        self.err("maps use curly braces: `{:}` is the empty map".to_string())
                    );
                }
                if matches!(self.peek(), Some(Tok::RBracket)) {
                    self.pos += 1;
                    return Ok(Expr::List(Vec::new(), span));
                }
                let first = self.parse_atom()?;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    return Err(self.err("maps use curly braces: `{key: value}`".to_string()));
                }
                let mut items = vec![first];
                while !matches!(self.peek(), Some(Tok::RBracket)) {
                    items.push(self.parse_atom()?);
                }
                self.pos += 1;
                Ok(Expr::List(items, span))
            }
            Some(Tok::LParen | Tok::LGroup) => {
                let written = matches!(self.peek(), Some(Tok::LParen));
                let opened = self.pos;
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
                let outer = std::mem::replace(&mut self.loosest, ATOM);
                let inner = self.parse_expr()?;
                let inside = std::mem::replace(&mut self.loosest, outer);
                let rparen_span = self.span_here();
                self.expect_close(written)?;
                // `(expr):type` — the upcast; the colon binds tight to the
                // closing paren, so map pairs never collide with it
                if matches!(self.peek(), Some(Tok::Colon)) {
                    let colon_span = self.span_here();
                    if colon_span.col == rparen_span.col + 1 {
                        self.pos += 1;
                        let ty_span = self.span_here();
                        if ty_span.col != colon_span.col + 1 {
                            return Err(Diagnostic::new(
                                "formatting",
                                "an upcast binds tight: `(expr):type`".to_string(),
                                colon_span,
                            ));
                        }
                        let (ty, _) = self.expect_ident("a type name")?;
                        return Ok(Expr::Upcast { expr: Box::new(inner), ty, span });
                    }
                }
                // an upcast returns above: there the parentheses are the
                // syntax rather than a grouping choice
                let before = tolerated_before(opened.checked_sub(1).and_then(|i| self.tok_at(i)));
                let after = tolerated_after(self.peek());
                if written && inside >= before.max(after) {
                    return Err(Diagnostic::new(
                        "formatting",
                        "these parentheses group nothing — the expression parses \
                         the same without them"
                            .to_string(),
                        span,
                    ));
                }
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
            // An identifier here has two readings and the parser cannot tell
            // them apart: somebody meant a map keyed by a variable, or they
            // meant a block, because every C-shaped language spells one with
            // braces. Naming one reading tells the other reader the wrong
            // thing, so name both.
            Expr::Ident(name, span) => Err(Diagnostic::new(
                "syntax",
                format!(
                    "`{name}` is not a literal: a map's keys are literals, and a \
                     dynamic one is built with `put` — a block is indentation \
                     rather than braces"
                ),
                *span,
            )),
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
