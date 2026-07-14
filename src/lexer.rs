use crate::diag::{Diagnostic, Span};
use num_bigint::BigInt;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(BigInt),
    Float(f64),
    Str(Vec<StrPart>),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Colon,
    Bind,
    Arrow,
    Pipe,
    SeqOp,
    Op(&'static str),
    Underscore,
    KwFn,
    KwType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Lit(String),
    Interp(Vec<(Tok, Span)>, Vec<usize>),
}

#[derive(Debug)]
pub struct Line {
    pub number: usize,
    pub indent: usize,
    pub tokens: Vec<(Tok, Span)>,
    pub end_cols: Vec<usize>,
}

struct LexedLine {
    tokens: Vec<(Tok, Span)>,
    end_cols: Vec<usize>,
}

pub struct Lexed {
    pub lines: Vec<Line>,
    pub blank_lines: Vec<usize>,
}

const OPS: [&str; 11] = [">=", "<=", "==", "!=", "+", "-", "*", "/", "<", ">", "&"];

pub fn lex(source: &str) -> Result<Lexed, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut lines = Vec::new();
    let mut blank_lines = Vec::new();
    if !source.is_empty() && !source.ends_with('\n') {
        diags.push(Diagnostic::new(
            "formatting",
            "file must end with exactly one newline".to_string(),
            Span { line: source.lines().count(), col: 1 },
        ));
    }
    for (idx, raw) in source.lines().enumerate() {
        let number = idx + 1;
        if let Some(col) = raw.find('\t') {
            diags.push(Diagnostic::new(
                "formatting",
                "tabs are not part of the canonical grammar; indent with spaces".to_string(),
                Span { line: number, col: col + 1 },
            ));
            continue;
        }
        if raw.trim_end() != raw {
            diags.push(Diagnostic::new(
                "formatting",
                "trailing whitespace is not part of the canonical grammar".to_string(),
                Span { line: number, col: raw.trim_end().len() + 1 },
            ));
        }
        let trimmed = raw.trim_end();
        if trimmed.is_empty() {
            blank_lines.push(number);
            continue;
        }
        let indent = trimmed.len() - trimmed.trim_start().len();
        let content = &trimmed[indent..];
        // A continuation line starts with a chain operator (`>>` or `.`) at the
        // parent statement's indent plus two; its tokens splice into the parent
        // so the parser sees one logical line. Spans keep the source line, so
        // diagnostics still point home. No statement can begin with an
        // operator, so the form is unambiguous.
        let continues = content.starts_with(">>") || content.starts_with(". ");
        if continues {
            let parent_ok = lines.last().is_some_and(|p: &Line| indent == p.indent + 2)
                && blank_lines.last() != Some(&(number - 1));
            if !parent_ok {
                diags.push(Diagnostic::new(
                    "formatting",
                    "a `>>`/`.` continuation line sits directly under its statement, \
                     indented two spaces deeper"
                        .to_string(),
                    Span { line: number, col: 1 },
                ));
                continue;
            }
            match lex_line(content, number, indent + 1) {
                Ok(lexed_line) => {
                    validate_spacing(&lexed_line, number, &mut diags);
                    let parent = lines.last_mut().expect("parent_ok checked");
                    parent.tokens.extend(lexed_line.tokens);
                    parent.end_cols.extend(lexed_line.end_cols);
                }
                Err(d) => diags.push(d),
            }
            continue;
        }
        if indent != 0 && indent != 2 {
            diags.push(Diagnostic::new(
                "formatting",
                format!("indentation must be 0 or 2 spaces, found {indent}"),
                Span { line: number, col: 1 },
            ));
            continue;
        }
        if content.starts_with('#') {
            continue;
        }
        match lex_line(content, number, indent + 1) {
            Ok(lexed_line) => {
                validate_spacing(&lexed_line, number, &mut diags);
                lines.push(Line {
                    number,
                    indent,
                    tokens: lexed_line.tokens,
                    end_cols: lexed_line.end_cols,
                });
            }
            Err(d) => diags.push(d),
        }
    }
    if diags.is_empty() { Ok(Lexed { lines, blank_lines }) } else { Err(diags) }
}

struct Scanner {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col_offset: usize,
}

fn lex_line(content: &str, line: usize, col_offset: usize) -> Result<LexedLine, Diagnostic> {
    let mut s = Scanner { chars: content.chars().collect(), pos: 0, line, col_offset };
    let mut tokens = Vec::new();
    let mut end_cols = Vec::new();
    while s.pos < s.chars.len() {
        let c = s.chars[s.pos];
        let span = s.span();
        if c == ' ' {
            s.pos += 1;
            continue;
        }
        if c == '#' {
            break;
        }
        if c == '/' && s.peek(1) == Some('/') {
            return Err(Diagnostic::new(
                "formatting",
                "comments are `#`".to_string(),
                span,
            ));
        }
        if c.is_ascii_digit() {
            tokens.push((s.lex_int()?, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c.is_ascii_lowercase() || c == '_' {
            tokens.push((s.lex_word()?, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c.is_ascii_uppercase() {
            return Err(Diagnostic::new(
                "formatting",
                "identifiers are snake_case, all lowercase, always".to_string(),
                span,
            ));
        }
        if c == '"' {
            tokens.push((s.lex_string()?, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c == ',' {
            return Err(Diagnostic::new(
                "formatting",
                "kanso has no commas; enumerations are space-separated".to_string(),
                span,
            ));
        }
        let tok = match c {
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '[' => Some(Tok::LBracket),
            ']' => Some(Tok::RBracket),
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            ':' => Some(Tok::Colon),
            '.' => Some(Tok::Pipe),
            _ => None,
        };
        if let Some(tok) = tok {
            s.pos += 1;
            tokens.push((tok, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c == '-' && s.peek(1) == Some('>') {
            s.pos += 2;
            tokens.push((Tok::Arrow, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c == '>' && s.peek(1) == Some('>') {
            s.pos += 2;
            tokens.push((Tok::SeqOp, span));
            end_cols.push(s.span().col);
            continue;
        }
        if c == '=' && s.peek(1) != Some('=') {
            s.pos += 1;
            tokens.push((Tok::Bind, span));
            end_cols.push(s.span().col);
            continue;
        }
        let two = [c, s.peek(1).unwrap_or(' ')].iter().collect::<String>();
        if let Some(op) = OPS.iter().find(|op| **op == two || (op.len() == 1 && op.starts_with(c))) {
            s.pos += op.len();
            tokens.push((Tok::Op(op), span));
            end_cols.push(s.span().col);
            continue;
        }
        return Err(Diagnostic::new("syntax", format!("unexpected character `{c}`"), span));
    }
    Ok(LexedLine { tokens, end_cols })
}

impl Scanner {
    fn span(&self) -> Span {
        Span { line: self.line, col: self.col_offset + self.pos }
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).copied()
    }

    fn lex_int(&mut self) -> Result<Tok, Diagnostic> {
        let start = self.pos;
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let is_float = self.peek(0) == Some('.')
            && self.peek(1).is_some_and(|c| c.is_ascii_digit());
        if is_float {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            let text: String = self.chars[start..self.pos].iter().collect();
            let value = text.parse::<f64>().expect("digit-dot-digit parses as f64");
            return Ok(Tok::Float(value));
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        let value = text.parse::<BigInt>().expect("digits parse as BigInt");
        Ok(Tok::Int(value))
    }

    fn lex_word(&mut self) -> Result<Tok, Diagnostic> {
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_ascii_lowercase()
                || self.chars[self.pos].is_ascii_digit()
                || self.chars[self.pos] == '_')
        {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        Ok(match word.as_str() {
            "_" => Tok::Underscore,
            "fn" => Tok::KwFn,
            "type" => Tok::KwType,
            _ => Tok::Ident(word),
        })
    }

    fn lex_string(&mut self) -> Result<Tok, Diagnostic> {
        let open_span = self.span();
        self.pos += 1;
        let mut parts = Vec::new();
        let mut lit = String::new();
        loop {
            let Some(c) = self.peek(0) else {
                return Err(Diagnostic::new("syntax", "unterminated string".to_string(), open_span));
            };
            match c {
                '"' => {
                    self.pos += 1;
                    if !lit.is_empty() {
                        parts.push(StrPart::Lit(lit));
                    }
                    return Ok(Tok::Str(parts));
                }
                '\\' => {
                    let escaped = self.peek(1).ok_or_else(|| {
                        Diagnostic::new("syntax", "unterminated escape".to_string(), self.span())
                    })?;
                    let resolved = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        '{' => '{',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => {
                            let msg = format!("unknown escape `\\{other}`");
                            return Err(Diagnostic::new("syntax", msg, self.span()));
                        }
                    };
                    lit.push(resolved);
                    self.pos += 2;
                }
                '{' => {
                    if !lit.is_empty() {
                        parts.push(StrPart::Lit(std::mem::take(&mut lit)));
                    }
                    let interp_span = self.span();
                    self.pos += 1;
                    let start = self.pos;
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek(0) {
                            Some('{') => depth += 1,
                            Some('}') => depth -= 1,
                            Some(_) => {}
                            None => {
                                let msg = "unterminated interpolation".to_string();
                                return Err(Diagnostic::new("syntax", msg, interp_span));
                            }
                        }
                        self.pos += 1;
                    }
                    let inner: String = self.chars[start..self.pos - 1].iter().collect();
                    if inner != inner.trim() || inner.is_empty() {
                        let msg = "interpolation braces wrap the expression exactly, \
                                   with no padding"
                            .to_string();
                        return Err(Diagnostic::new("formatting", msg, interp_span));
                    }
                    let col = self.col_offset + start;
                    let lexed = lex_line(&inner, self.line, col)?;
                    parts.push(StrPart::Interp(lexed.tokens, lexed.end_cols));
                }
                other => {
                    lit.push(other);
                    self.pos += 1;
                }
            }
        }
    }
}

fn required_gap(prev: &Tok, next: &Tok) -> usize {
    match (prev, next) {
        (_, Tok::RParen) | (_, Tok::RBracket) => 0,
        (Tok::LParen, _) | (Tok::LBracket, _) => 0,
        (_, Tok::Colon) => 0,
        _ => 1,
    }
}

fn validate_spacing(lexed_line: &LexedLine, line: usize, diags: &mut Vec<Diagnostic>) {
    for (pair, prev_end) in lexed_line.tokens.windows(2).zip(&lexed_line.end_cols) {
        let (prev, _) = &pair[0];
        let (next, next_span) = &pair[1];
        let gap = next_span.col.saturating_sub(*prev_end);
        if matches!(prev, Tok::Colon) {
            if gap > 1 {
                diags.push(Diagnostic::new(
                    "formatting",
                    "canonical form requires at most one space here".to_string(),
                    Span { line, col: next_span.col },
                ));
            }
            continue;
        }
        if matches!((prev, next), (Tok::Ident(_), Tok::LBracket)) {
            if gap > 1 {
                diags.push(Diagnostic::new(
                    "formatting",
                    "canonical form requires at most one space here".to_string(),
                    Span { line, col: next_span.col },
                ));
            }
            continue;
        }
        let required = required_gap(prev, next);
        if gap != required {
            let wanted = match required {
                0 => "no space".to_string(),
                _ => "exactly one space".to_string(),
            };
            diags.push(Diagnostic::new(
                "formatting",
                format!("canonical form requires {wanted} here"),
                Span { line, col: next_span.col },
            ));
        }
    }
}
