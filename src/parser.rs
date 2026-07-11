use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer::{Lexed, Line, StrPart, Tok};

pub fn parse(lexed: &Lexed) -> Result<Program, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut fns = Vec::new();
    let mut types = Vec::new();
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
        match line.tokens.first() {
            Some((Tok::KwFn, _)) => match parse_fn(line, body) {
                Ok(decl) => fns.push(decl),
                Err(d) => diags.push(d),
            },
            Some((Tok::KwType, _)) => match parse_type(line, body) {
                Ok(decl) => types.push(decl),
                Err(d) => diags.push(d),
            },
            _ => diags.push(Diagnostic::new(
                "syntax",
                "a top-level line must begin with `fn` or `type`".to_string(),
                head_span(line),
            )),
        }
        i = body_end;
    }
    if diags.is_empty() { Ok(Program { fns, types }) } else { Err(diags) }
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
        let required = match pair[1].indent {
            0 => 1,
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
    let mut p = P::new(&header.tokens, header.number);
    p.expect_kw_fn()?;
    let (name, span) = p.expect_ident("a function name")?;
    let mut params = Vec::new();
    if !p.done() {
        params.push(p.parse_pattern()?);
        while p.eat_comma() {
            params.push(p.parse_pattern()?);
        }
    }
    p.expect_done()?;
    if body.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            format!("function `{name}` has no body"),
            span,
        ));
    }
    let stmts = body.iter().map(parse_stmt).collect::<Result<Vec<_>, _>>()?;
    Ok(FnDecl { name, span, params, body: stmts })
}

fn parse_type(header: &Line, body: &[Line]) -> Result<TypeDecl, Diagnostic> {
    let mut p = P::new(&header.tokens, header.number);
    p.expect_kw_type()?;
    let (name, span) = p.expect_ident("a type name")?;
    p.expect_done()?;
    if body.is_empty() {
        return Err(Diagnostic::new("syntax", format!("type `{name}` has no fields"), span));
    }
    let fields = body.iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;
    Ok(TypeDecl { name, span, fields })
}

fn parse_field(line: &Line) -> Result<(String, String, Span), Diagnostic> {
    let mut p = P::new(&line.tokens, line.number);
    let (name, span) = p.expect_ident("a field name")?;
    p.expect_colon()?;
    let ty = p.parse_type_expr()?;
    p.expect_done()?;
    Ok((name, ty, span))
}

fn parse_stmt(line: &Line) -> Result<Stmt, Diagnostic> {
    let mut p = P::new(&line.tokens, line.number);
    if let (Some((Tok::Ident(name), span)), Some((Tok::Bind, _))) =
        (line.tokens.first(), line.tokens.get(1))
    {
        p.pos = 2;
        let expr = p.parse_expr_with_commas()?;
        p.expect_done()?;
        return Ok(Stmt::Bind { name: name.clone(), span: *span, expr });
    }
    let expr = p.parse_expr_with_commas()?;
    p.expect_done()?;
    Ok(Stmt::Expr(expr))
}

pub struct P<'a> {
    toks: &'a [(Tok, Span)],
    pub pos: usize,
    line: usize,
}

impl<'a> P<'a> {
    pub fn new(toks: &'a [(Tok, Span)], line: usize) -> Self {
        P { toks, pos: 0, line }
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

    fn eat_comma(&mut self) -> bool {
        match self.peek() {
            Some(Tok::Comma) => {
                self.pos += 1;
                true
            }
            _ => false,
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
                Ok(Pattern::Wildcard)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                match NULLARY.contains(&name.as_str()) {
                    true => Ok(Pattern::Nullary(name, span)),
                    false => Ok(Pattern::Var(name, span)),
                }
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let (name, _) = self.expect_ident("a name or type")?;
                match self.peek() {
                    Some(Tok::Colon) => {
                        self.pos += 1;
                        let ty = self.parse_type_expr()?;
                        self.expect_rparen()?;
                        Ok(Pattern::Annotated { name, ty, span })
                    }
                    _ => {
                        let mut fields = vec![self.parse_pattern()?];
                        while self.eat_comma() {
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

    pub fn parse_expr_with_commas(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_pipe()?;
        while self.eat_comma() {
            let arg = self.parse_pipe()?;
            match &mut expr {
                Expr::App { args, .. } => args.push(arg),
                _ => {
                    let message =
                        "a comma adds an argument to a call, and nothing here is a call"
                            .to_string();
                    return Err(self.err(message));
                }
            }
        }
        Ok(expr)
    }

    fn parse_pipe(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_cmp()?;
        loop {
            match self.peek() {
                Some(Tok::Pipe) => {
                    let span = self.span_here();
                    self.pos += 1;
                    let target = self.parse_atom()?;
                    expr = Expr::App { head: Box::new(target), args: vec![expr], span };
                }
                Some(Tok::SeqOp) => {
                    let span = self.span_here();
                    self.pos += 1;
                    let rhs = self.parse_cmp()?;
                    expr = Expr::Seq(Box::new(expr), Box::new(rhs), span);
                }
                _ => return Ok(expr),
            }
        }
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
            Some(Tok::Ident(_) | Tok::Int(_) | Tok::Str(_) | Tok::LParen | Tok::LBracket)
        )
    }

    fn parse_app(&mut self) -> Result<Expr, Diagnostic> {
        let mut atoms = vec![self.parse_atom()?];
        while self.starts_atom() {
            atoms.push(self.parse_atom()?);
        }
        let mut expr = atoms.pop().expect("at least one atom");
        for head in atoms.into_iter().rev() {
            let span = head.span();
            expr = Expr::App { head: Box::new(head), args: vec![expr], span };
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span_here();
        match self.toks.get(self.pos).map(|(t, _)| t.clone()) {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Ok(Expr::Int(n, span))
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
                let mut items = Vec::new();
                if !matches!(self.peek(), Some(Tok::RBracket)) {
                    items.push(self.parse_pipe()?);
                    while self.eat_comma() {
                        items.push(self.parse_pipe()?);
                    }
                }
                match self.peek() {
                    Some(Tok::RBracket) => {
                        self.pos += 1;
                        Ok(Expr::List(items, span))
                    }
                    _ => Err(self.err("expected `]`".to_string())),
                }
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                if let Some(arrow_end) = self.lambda_lookahead() {
                    let mut params = Vec::new();
                    while self.pos < arrow_end {
                        let (name, pspan) = self.expect_ident("a lambda parameter")?;
                        params.push((name, pspan));
                        self.eat_comma();
                    }
                    self.pos = arrow_end + 1;
                    let body = self.parse_expr_with_commas()?;
                    self.expect_rparen()?;
                    return Ok(Expr::Lambda { params, body: Box::new(body), span });
                }
                let inner = self.parse_expr_with_commas()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            _ => Err(self.err("expected an expression".to_string())),
        }
    }

    fn lambda_lookahead(&self) -> Option<usize> {
        let mut i = self.pos;
        loop {
            match self.toks.get(i).map(|(t, _)| t) {
                Some(Tok::Ident(_)) => i += 1,
                _ => return None,
            }
            match self.toks.get(i).map(|(t, _)| t) {
                Some(Tok::Arrow) => return Some(i),
                Some(Tok::Comma) => i += 1,
                _ => return None,
            }
        }
    }
}

fn literal_string(parts: &[StrPart]) -> Option<String> {
    let mut out = String::new();
    for part in parts {
        match part {
            StrPart::Lit(s) => out.push_str(s),
            StrPart::Interp(_) => return None,
        }
    }
    Some(out)
}

fn template_part(part: &StrPart, line: usize) -> Result<TemplatePart, Diagnostic> {
    match part {
        StrPart::Lit(s) => Ok(TemplatePart::Lit(s.clone())),
        StrPart::Interp(tokens) => {
            let mut p = P::new(tokens, line);
            let expr = p.parse_expr_with_commas()?;
            p.expect_done()?;
            Ok(TemplatePart::Interp(expr))
        }
    }
}
