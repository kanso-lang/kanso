use crate::diag::{Diagnostic, Span};
use num_bigint::BigInt;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Bang,
    Ident(String),
    Int(BigInt),
    Float(f64),
    Str(Vec<StrPart>),
    LParen,
    RParen,
    /// The grouping the indented-argument form inserts around a continuation
    /// line. It parses exactly as a parenthesis and is written by nobody, so
    /// rules about what an author may write must not see it.
    LGroup,
    RGroup,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Colon,
    Dot,
    Bind,
    Arrow,
    Pipe,
    SeqOp,
    Op(&'static str),
    Underscore,
    /// The as-pattern's sigil: `r@(rect w h)` binds the whole while its
    /// parts are destructured. It hugs both sides, so it can never be
    /// mistaken for anything else.
    At,
    KwFn,
    KwReturn,
    KwType,
    KwPub,
    KwImport,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Lit(String),
    Interp(Vec<(Tok, Span, u32)>),
}

#[derive(Clone, Debug)]
pub struct Line {
    pub number: usize,
    pub indent: usize,
    /// Each token, where it starts, and the column it ends at. One vector
    /// rather than a `Vec<(Tok, Span)>` beside a `Vec<usize>`: the two were
    /// always the same length and every one of the twelve places that sliced
    /// them sliced both the same way, which is a pair that has to be kept in
    /// step by hand. It cannot fall out of step now, and a line pays one
    /// header and one doubling sequence where it used to pay two.
    pub tokens: Vec<(Tok, Span, u32)>,
}

struct LexedLine {
    tokens: Vec<(Tok, Span, u32)>,
}

pub struct Lexed {
    pub lines: Vec<Line>,
    pub blank_lines: Vec<usize>,
}

const OPS: [&str; 14] = [">=", "<=", "==", "!=", "+", "-", "*", "/", "%", "<", ">", "&", "|", "^"];

pub const MAX_WIDTH: usize = 80;

pub fn lex(source: &str) -> Result<Lexed, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut lines = Vec::new();
    let mut blank_lines = Vec::new();
    if !source.is_empty() && !source.ends_with('\n') {
        diags.push(Diagnostic::new(
            "formatting",
            "file must end with exactly one newline".to_string(),
            Span::at(source.lines().count(), 1),
        ));
    }
    let raws: Vec<&str> = source.lines().collect();
    let mut idx = 0;
    while idx < raws.len() {
        let raw = raws[idx];
        let number = idx + 1;
        idx += 1;
        // `find` answers in bytes and a column is counted in characters, the
        // same parting of ways `block_opener` had: with `ééé` ahead of the
        // tab the caret used to land three columns right of it.
        if let Some(at) = raw.find('\t') {
            diags.push(Diagnostic::new(
                "formatting",
                "tabs are not part of the canonical grammar; indent with spaces".to_string(),
                Span::at(number, raw[..at].chars().count() + 1),
            ));
            continue;
        }
        if raw.trim_end() != raw {
            diags.push(Diagnostic::new(
                "formatting",
                "trailing whitespace is not part of the canonical grammar".to_string(),
                Span::at(number, raw.trim_end().len() + 1),
            ));
        }
        let trimmed = raw.trim_end();
        if trimmed.is_empty() {
            blank_lines.push(number);
            continue;
        }
        let width = trimmed.chars().count();
        if width > MAX_WIDTH {
            diags.push(Diagnostic::new(
                "formatting",
                format!("a line holds at most {MAX_WIDTH} characters — this one has {width}"),
                Span::at(number, MAX_WIDTH + 1),
            ));
        }
        let indent = trimmed.len() - trimmed.trim_start().len();
        let content = &trimmed[indent..];
        // A text block opens at the end of a statement's line and its content
        // sits two spaces deeper, so the lines it holds are consumed here
        // rather than reaching the per-line rules: a blank line inside a block
        // is a newline in a value, not a statement break, and a content line
        // is data the width cap has no way to re-render.
        if let Some(at) = block_opener(content) {
            // `at` is a byte offset and a column is counted in characters, so
            // the two part company the moment the line holds anything outside
            // ASCII. Every column below is the character count of what
            // precedes the fence.
            let col = content[..at].chars().count();
            if content[at..].chars().count() != 3 {
                diags.push(Diagnostic::new(
                    "syntax",
                    "a text block opens at the end of its line — nothing follows `\"\"\"`"
                        .to_string(),
                    Span::at(number, indent + col + 4),
                ));
                continue;
            }
            let (body, consumed) = gather_block(&raws, idx, indent, number, &mut diags);
            idx = consumed;
            match lex_line_with_block(&content[..at], number, indent + 1, indent + 1 + col, &body) {
                Ok(lexed_line) => {
                    validate_spacing(&lexed_line, number, &mut diags);
                    lines.push(Line { number, indent, tokens: lexed_line.tokens });
                }
                Err(d) => diags.push(d),
            }
            continue;
        }
        // A continuation line starts with a chain operator (`.` or `>>`) at
        // the parent statement's indent plus two; its tokens splice into the
        // parent so the parser sees one wrapped statement. Spans keep the
        // source line, so diagnostics still point home. Wrapping never changes
        // how many statements there are — width only breaks a statement's
        // line. A `>>`-led line at the parent's own indent is not a wrap; it
        // flows to the parser as a wall or a fused sequential step. Headers
        // (`fn`, `type`, a bare `name =`) hold no statement to wrap.
        let cont_indent_ok = lines.last().is_some_and(|p: &Line| indent == p.indent + 2)
            && blank_lines.last() != Some(&(number - 1));
        let parent_wrappable = lines.last().is_some_and(|p: &Line| {
            !matches!(p.tokens.first(), Some((Tok::KwFn | Tok::KwType | Tok::KwPub, _, _)))
                && !matches!(p.tokens.last(), Some((Tok::Bind, _, _)))
        });
        let dot_cont = content.starts_with(". ");
        let seq_cont = content.starts_with(">> ") && cont_indent_ok && parent_wrappable;
        if dot_cont || seq_cont {
            if !cont_indent_ok {
                diags.push(Diagnostic::new(
                    "formatting",
                    "a `.` continuation line sits directly under its statement, \
                     indented two spaces deeper"
                        .to_string(),
                    Span::at(number, 1),
                ));
                continue;
            }
            match lex_line(content, number, indent + 1) {
                Ok(lexed_line) => {
                    validate_spacing(&lexed_line, number, &mut diags);
                    let parent = lines.last_mut().expect("parent_ok checked");
                    parent.tokens.extend(lexed_line.tokens);
                }
                Err(d) => diags.push(d),
            }
            continue;
        }
        // An indented line under an argument-taking statement is one more
        // argument — the whole line, one expression, spliced in parens so
        // the parser sees an ordinary call. Headers (`fn`, `type`,
        // `import`, a `pub` form, a trailing `=`) hold bodies, not args —
        // and a block header (`if cond`, `x = if cond`, `else`) holds
        // branch lines, which stay real lines for the parser to group.
        let is_block_header = |p: &Line| {
            matches!(p.tokens.as_slice(), [(Tok::Ident(head), _, _), ..] if head == "if")
                || matches!(
                    p.tokens.as_slice(),
                    [(Tok::Ident(_), _, _), (Tok::Bind, _, _), (Tok::Ident(head), _, _), ..]
                        if head == "if"
                )
                || matches!(p.tokens.as_slice(), [(Tok::Ident(head), _, _)] if head == "else")
                || matches!(p.tokens.as_slice(), [(Tok::Ident(head), _, _)] if head == "build")
                || matches!(
                    p.tokens.as_slice(),
                    [(Tok::Ident(_), _, _), (Tok::Bind, _, _), (Tok::Ident(head), _, _)]
                        if head == "build"
                )
        };
        let block_child =
            lines.last().is_some_and(|p: &Line| is_block_header(p) && indent == p.indent + 2);
        let sibling_or_dedent = lines
            .last()
            .is_some_and(|p: &Line| indent > 2 && indent.is_multiple_of(2) && indent <= p.indent);
        if block_child || sibling_or_dedent {
            match lex_line(content, number, indent + 1) {
                Ok(lexed_line) => {
                    validate_spacing(&lexed_line, number, &mut diags);
                    lines.push(Line { number, indent, tokens: lexed_line.tokens });
                }
                Err(d) => diags.push(d),
            }
            continue;
        }
        let arg_parent_ok = lines.last().is_some_and(|p: &Line| {
            !matches!(
                p.tokens.first(),
                Some((Tok::KwFn | Tok::KwType | Tok::KwImport | Tok::KwPub, _, _))
            ) && !matches!(p.tokens.last(), Some((Tok::Bind, _, _)))
        });
        if cont_indent_ok && arg_parent_ok {
            match lex_line(content, number, indent + 1) {
                Ok(lexed_line) => {
                    validate_spacing(&lexed_line, number, &mut diags);
                    let parent = lines.last_mut().expect("cont_indent_ok checked");
                    let open_span = Span::at(number, 1);
                    let close_span =
                        Span::at(number, lexed_line.tokens.last().map_or(1, |t| t.2 as usize));
                    parent.tokens.push((Tok::LGroup, open_span, 1));
                    parent.tokens.extend(lexed_line.tokens);
                    parent.tokens.push((Tok::RGroup, close_span, close_span.col));
                }
                Err(d) => diags.push(d),
            }
            continue;
        }
        if indent != 0 && indent != 2 {
            diags.push(Diagnostic::new(
                "formatting",
                format!("indentation must be 0 or 2 spaces, found {indent}"),
                Span::at(number, 1),
            ));
            continue;
        }
        if content.starts_with('#') {
            continue;
        }
        match lex_line(content, number, indent + 1) {
            Ok(lexed_line) => {
                validate_spacing(&lexed_line, number, &mut diags);
                lines.push(Line { number, indent, tokens: lexed_line.tokens });
            }
            Err(d) => diags.push(d),
        }
    }
    for line in &lines {
        check_needless_continuation(line, &mut diags);
        check_partial_chain(line, &mut diags);
    }
    if diags.is_empty() {
        Ok(Lexed { lines, blank_lines })
    } else {
        Err(diags)
    }
}

/// The column a text block opens at, when its line ends with one. A `"""`
/// inside a string is not an opener, so the scan tracks quoting the way the
/// lexer does; anything after the three quotes is refused where the block is
/// read, with a message about the shape rather than about an unclosed string.
/// The BYTE offset of the `"""` that opens a text block on this line, if it
/// has one. Byte, because every caller slices `content` with it. It used to
/// count characters, and a character index and a byte index agree only while
/// the line is ASCII: a line carrying `\u{e9}` before the fence sliced one byte
/// short and was refused as "nothing follows", and one carrying an emoji
/// panicked on a non-boundary.
///
/// Bytes also mean no `Vec<char>`, which was 2,172 of the front end's
/// allocations. The three bytes this scan tests for are ASCII, and every byte
/// inside a multi-byte character is >= 0x80, so those bytes match no arm and
/// are walked past one at a time. `i += 2` past an escape is right for the
/// same reason: it skips the backslash and the escaped character's first
/// byte, and whatever remains of that character matches nothing.
fn block_opener(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = 0;
    let mut in_str = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 2,
            b'"' if in_str => {
                in_str = false;
                i += 1;
            }
            b'"' if bytes[i..].starts_with(b"\"\"\"") => return Some(i),
            b'"' => {
                in_str = true;
                i += 1;
            }
            b'#' if !in_str => return None,
            _ => i += 1,
        }
    }
    None
}

/// The lines a text block holds, and the index of the first line after it.
/// Content sits at the opening indent plus two and is stripped of exactly
/// that; a line shallower than it ends the block, and if that line is not the
/// fence, saying so where it happens beats saying "unterminated" at the end.
fn gather_block(
    raws: &[&str],
    from: usize,
    indent: usize,
    open_line: usize,
    diags: &mut Vec<Diagnostic>,
) -> (Vec<(usize, String)>, usize) {
    let inner = indent + 2;
    let fence = format!("{}\"\"\"", " ".repeat(indent));
    let mut body = Vec::new();
    let mut at = from;
    while at < raws.len() {
        let raw = raws[at];
        let number = at + 1;
        at += 1;
        let trimmed = raw.trim_end();
        if trimmed == fence {
            return (body, at);
        }
        if trimmed.trim_start() == fence.trim_start() || trimmed.starts_with(&fence) {
            let said = match trimmed.starts_with(&fence) {
                true => "the closing `\"\"\"` sits alone on its line",
                false => "the closing `\"\"\"` sits at the column its opening line starts in",
            };
            diags.push(Diagnostic::new(
                "syntax",
                said.to_string(),
                Span::at(number, trimmed.len() - trimmed.trim_start().len() + 1),
            ));
            return (body, at);
        }
        if raw.trim_end() != raw {
            diags.push(Diagnostic::new(
                "formatting",
                "trailing whitespace is not part of the canonical grammar".to_string(),
                Span::at(number, raw.trim_end().len() + 1),
            ));
        }
        if let Some(at) = trimmed.find('\t') {
            diags.push(Diagnostic::new(
                "formatting",
                "tabs are not part of the canonical grammar; indent with spaces".to_string(),
                Span::at(number, trimmed[..at].chars().count() + 1),
            ));
            continue;
        }
        if trimmed.is_empty() {
            body.push((number, String::new()));
            continue;
        }
        let found = trimmed.len() - trimmed.trim_start().len();
        if found < inner {
            let said = format!(
                "a text block's lines sit {inner} spaces in, under the `\"\"\"` that opened \
                 it — this one is at column {}",
                found + 1
            );
            diags.push(Diagnostic::new("syntax", said, Span::at(number, found + 1)));
            return (body, at - 1);
        }
        body.push((number, trimmed[inner..].to_string()));
    }
    let said = format!(
        "unterminated text block — the closing `\"\"\"` sits alone at column {}",
        indent + 1
    );
    diags.push(Diagnostic::new("syntax", said, Span::at(open_line, indent + 1)));
    (body, at)
}

/// A statement wrapped across `>>` continuation lines gives every step its
/// own line: no step shares a line with another (partial chaining).
fn check_partial_chain(line: &Line, diags: &mut Vec<Diagnostic>) {
    let leads_line = |i: usize, span: &Span| {
        i > 0 && line.tokens[i - 1].1.line != span.line && span.line as usize != line.number
    };
    let wrapped = line
        .tokens
        .iter()
        .enumerate()
        .any(|(i, (tok, span, _))| matches!(tok, Tok::SeqOp) && leads_line(i, span));
    if !wrapped {
        return;
    }
    let mut depth = 0usize;
    for (i, (tok, span, _)) in line.tokens.iter().enumerate() {
        match tok {
            Tok::LParen | Tok::LGroup | Tok::LBracket | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RGroup | Tok::RBracket | Tok::RBrace => {
                depth = depth.saturating_sub(1)
            }
            Tok::SeqOp if depth == 0 && !leads_line(i, span) => {
                diags.push(Diagnostic::new(
                    "formatting",
                    "no partial chaining: a chain fits on one line, or each step \
                     gets its own `>>` continuation line"
                        .to_string(),
                    *span,
                ));
                return;
            }
            _ => {}
        }
    }
}

/// One meaning, one rendering: a statement split across `.` continuation
/// lines is only legal when the spliced one-line form would not fit.
fn check_needless_continuation(line: &Line, diags: &mut Vec<Diagnostic>) {
    let mut pieces: Vec<(usize, usize, Span)> = Vec::new();
    for (_, span, end) in &line.tokens {
        let end = *end as usize;
        match pieces.last_mut() {
            Some(piece) if piece.2.line == span.line => piece.1 = end,
            _ => pieces.push((span.col as usize, end, *span)),
        }
    }
    if pieces.len() < 2 {
        return;
    }
    let mut width = pieces[0].1 - 1;
    for piece in &pieces[1..] {
        width += 1 + (piece.1 - piece.0);
    }
    if width > MAX_WIDTH {
        return;
    }
    // A statement that opens an indented block is read as one long line, so
    // somebody who wrote `for x in xs` and indented under it is told their
    // statement fits on one line — true of the tokens, and nothing to do with
    // what they meant. kanso has no form that opens a block this way.
    if let Some((Tok::Ident(head), span, _)) = line.tokens.first() {
        if let Some(instead) = kanso_form_for(head) {
            diags.push(Diagnostic::new(
                "syntax",
                format!("kanso has no `{head}` — {instead}"),
                *span,
            ));
            return;
        }
    }
    diags.push(Diagnostic::new(
        "formatting",
        format!("needless continuation: this statement fits on one line ({width} characters)"),
        pieces[1].2,
    ));
}

/// What kanso uses instead of a keyword the reader brought with them. Only
/// the ones that open a block reach here, because only those are mistaken for
/// a continued statement.
fn kanso_form_for(head: &str) -> Option<&'static str> {
    match head {
        "for" | "while" | "loop" | "repeat" | "until" => {
            Some("repetition is recursion, and a chain of list verbs covers most of it")
        }
        "def" | "func" | "function" | "define" => Some("a function is `fn name params`"),
        "class" | "struct" | "record" | "data" => {
            Some("a record is `type name` with its fields indented under it")
        }
        "switch" | "match" | "case" | "when" => {
            Some("dispatch is one arm per shape, written as separate `fn` clauses")
        }
        "try" | "catch" | "except" | "rescue" => {
            Some("a failure rides the same rails as a value; an arm names the err")
        }
        _ => None,
    }
}

struct Scanner {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col_offset: usize,
}

thread_local! {
    /// Character buffers a `Scanner` borrows and gives back.
    ///
    /// `pos` is the column a caret goes under, so the scanner indexes
    /// characters rather than bytes and has to have the line as a `Vec<char>`
    /// to do it. Collecting a fresh one per line was 1,997 of the front end's
    /// allocation blocks, for a vector that dies at the end of the line it was
    /// built for.
    ///
    /// A pool rather than one buffer, because scanners nest: an interpolation
    /// lexes its inner text with a scanner of its own while the outer scanner
    /// still holds the line the interpolation is written on. The pool ends up
    /// holding one buffer per level of nesting reached, each grown to the
    /// longest line it ever took.
    static CHAR_BUFS: std::cell::RefCell<Vec<Vec<char>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl Scanner {
    fn new(content: &str, line: usize, col_offset: usize) -> Scanner {
        let mut chars = CHAR_BUFS.with(|pool| pool.borrow_mut().pop()).unwrap_or_default();
        chars.extend(content.chars());
        Scanner { chars, pos: 0, line, col_offset }
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        let mut chars = std::mem::take(&mut self.chars);
        chars.clear();
        CHAR_BUFS.with(|pool| pool.borrow_mut().push(chars));
    }
}

/// A statement whose last token is a text block: the part before the `"""`
/// lexes as an ordinary line, and the block's lines become one string token
/// spanning them. The token's own span stays on the opening line, where the
/// reader's eye is, while an interpolation inside the block reports against
/// the line it was written on.
fn lex_line_with_block(
    prefix: &str,
    line: usize,
    col_offset: usize,
    open_col: usize,
    body: &[(usize, String)],
) -> Result<LexedLine, Diagnostic> {
    let mut lexed = lex_line(prefix, line, col_offset)?;
    let mut parts = Vec::new();
    let mut lit = String::new();
    for (number, text) in body {
        let mut s = Scanner::new(text, *number, 1);
        s.block_text(&mut parts, &mut lit)?;
        lit.push('\n');
    }
    if !lit.is_empty() {
        parts.push(StrPart::Lit(lit));
    }
    if body.len() < 2 {
        return Err(Diagnostic::new(
            "formatting",
            "a text block holds two or more lines; one line is written `\"...\"`".to_string(),
            Span::at(line, open_col),
        ));
    }
    lexed.tokens.push((Tok::Str(parts), Span::at(line, open_col), (open_col + 3) as u32));
    Ok(lexed)
}

fn lex_line(content: &str, line: usize, col_offset: usize) -> Result<LexedLine, Diagnostic> {
    let mut s = Scanner::new(content, line, col_offset);
    // Eight, because a `Vec` starting empty reaches four and then doubles, and
    // the doubling was 434 allocation blocks on lib/json. Sixteen takes 156
    // more and puts 9.7% on compile_peak_bytes, which a line vector kept for
    // the whole parse pays for; eight leaves the peak where it was.
    let mut tokens = Vec::with_capacity(8);
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
            return Err(Diagnostic::new("formatting", "comments are `#`".to_string(), span));
        }
        if c.is_ascii_digit() {
            let tok = s.lex_int()?;
            tokens.push((tok, span, s.span().col));
            continue;
        }
        // a prefix minus folds into the literal: `-1` is a number, not an
        // operation — binary minus needs an operand on its left
        if c == '-' && s.peek(1).is_some_and(|d| d.is_ascii_digit()) {
            let prefix = !matches!(
                tokens.last(),
                Some((
                    Tok::Ident(_)
                        | Tok::Int(_)
                        | Tok::Float(_)
                        | Tok::Str(_)
                        | Tok::RParen
                        | Tok::RGroup
                        | Tok::RBracket,
                    _,
                    _
                ))
            );
            if prefix {
                s.pos += 1;
                let tok = match s.lex_int()? {
                    Tok::Int(n) => Tok::Int(-n),
                    Tok::Float(x) => Tok::Float(-x),
                    other => other,
                };
                tokens.push((tok, span, s.span().col));
                continue;
            }
        }
        if c.is_ascii_lowercase() || c == '_' {
            let tok = s.lex_word()?;
            tokens.push((tok, span, s.span().col));
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
            let tok = s.lex_string()?;
            tokens.push((tok, span, s.span().col));
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
            // the strict-index sigil: xs[i]! errs where xs[i] returns none.
            // Followed by `=` it is the comparison instead, the same way a
            // bare `=` binds only when it does not head an `==`.
            '!' if s.peek(1) != Some('=') => Some(Tok::Bang),
            '@' => Some(Tok::At),
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            ':' => Some(Tok::Colon),
            '.' => {
                // a dot pressed tight against both neighbors reads a field
                // (u.name); with air around it, it is the pipe
                // `!` ends a value the way `]` and `)` do — it is the strict
                // index asking for the element rather than the option — so a
                // dot after it reads a field. Without it `xs[i]!.name` lexed
                // as a pipe and `name` was applied to the element, which is
                // an unknown name rather than a field read.
                let tight_left = s.pos > 0
                    && s.chars.get(s.pos - 1).is_some_and(|p| {
                        p.is_ascii_alphanumeric()
                            || *p == '_'
                            || *p == ')'
                            || *p == ']'
                            || *p == '!'
                    });
                let tight_right =
                    s.chars.get(s.pos + 1).is_some_and(|n| n.is_ascii_lowercase() || *n == '_');
                Some(if tight_left && tight_right { Tok::Dot } else { Tok::Pipe })
            }
            _ => None,
        };
        if let Some(tok) = tok {
            s.pos += 1;
            tokens.push((tok, span, s.span().col));
            continue;
        }
        if c == '-' && s.peek(1) == Some('>') {
            s.pos += 2;
            tokens.push((Tok::Arrow, span, s.span().col));
            continue;
        }
        if c == '>' && s.peek(1) == Some('>') {
            s.pos += 2;
            tokens.push((Tok::SeqOp, span, s.span().col));
            continue;
        }
        if c == '=' && s.peek(1) != Some('=') {
            s.pos += 1;
            tokens.push((Tok::Bind, span, s.span().col));
            continue;
        }
        let two = [c, s.peek(1).unwrap_or(' ')].iter().collect::<String>();
        if let Some(op) = OPS.iter().find(|op| **op == two || (op.len() == 1 && op.starts_with(c)))
        {
            s.pos += op.len();
            tokens.push((Tok::Op(op), span, s.span().col));
            continue;
        }
        return Err(Diagnostic::new("syntax", format!("unexpected character `{c}`"), span));
    }
    Ok(LexedLine { tokens })
}

impl Scanner {
    fn span(&self) -> Span {
        Span::at(self.line, self.col_offset + self.pos)
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).copied()
    }

    fn lex_int(&mut self) -> Result<Tok, Diagnostic> {
        let start = self.pos;
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let is_float =
            self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit());
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
            // a slash pressed tight between word characters qualifies a name
            // (json/decode); division between named values breathes, like the
            // pipe and unlike nothing else
            if self.chars.get(self.pos) == Some(&'/')
                && self.chars.get(self.pos + 1).is_some_and(|n| n.is_ascii_lowercase() || *n == '_')
            {
                self.pos += 1;
            }
        }
        // the naming sigils, one each, terminal only: `!` marks the strict
        // variant (errs where the plain form is lenient), `?` a predicate
        if self.chars.get(self.pos).is_some_and(|c| *c == '!' || *c == '?') {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        if word.len() > 1 && word.starts_with('_') {
            return Err(Diagnostic::new(
                "naming",
                "leading underscores are retired: privacy is `pub`'s absence, \
                 and `_` alone is the wildcard"
                    .to_string(),
                self.span(),
            ));
        }
        Ok(match word.as_str() {
            "_" => Tok::Underscore,
            // `and` and `or` are spelled, not punctuated: a reader says them
            // aloud the way they mean them, and `&` `|` stay with the bits
            "and" => Tok::Op("and"),
            "or" => Tok::Op("or"),
            "not" => Tok::Op("not"),
            "fn" => Tok::KwFn,
            "return" => Tok::KwReturn,
            "type" => Tok::KwType,
            "pub" => Tok::KwPub,
            "import" => Tok::KwImport,
            _ => Tok::Ident(word),
        })
    }

    /// `{expr}` in either string form: the braces wrap the expression exactly
    /// and it is lexed as a line of its own, so what it reports points at the
    /// column it was written in.
    fn interpolation(&mut self) -> Result<StrPart, Diagnostic> {
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
            let msg = "interpolation braces wrap the expression exactly, with no padding";
            return Err(Diagnostic::new("formatting", msg.to_string(), interp_span));
        }
        let col = self.col_offset + start;
        let lexed = lex_line(&inner, self.line, col)?;
        Ok(StrPart::Interp(lexed.tokens))
    }

    /// One line of a text block. The quoting a one-line literal needs is
    /// gone — a quote is a quote and the line ends where it ends — so the
    /// escapes that spell those two things are refused rather than merely
    /// unnecessary: one value, one spelling, and no formatter to choose.
    fn block_text(&mut self, parts: &mut Vec<StrPart>, lit: &mut String) -> Result<(), Diagnostic> {
        while let Some(c) = self.peek(0) {
            match c {
                '\\' => {
                    let escaped = self.peek(1).ok_or_else(|| {
                        Diagnostic::new("syntax", "unterminated escape".to_string(), self.span())
                    })?;
                    let resolved = match escaped {
                        '\\' => '\\',
                        '{' => '{',
                        'r' => '\r',
                        't' => '\t',
                        'n' => {
                            let said = "a newline inside a text block is a line break, not `\\n`";
                            return Err(Diagnostic::new(
                                "formatting",
                                said.to_string(),
                                self.span(),
                            ));
                        }
                        '"' => {
                            let said = "a quote inside a text block is written plainly";
                            return Err(Diagnostic::new(
                                "formatting",
                                said.to_string(),
                                self.span(),
                            ));
                        }
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
                        parts.push(StrPart::Lit(std::mem::take(lit)));
                    }
                    parts.push(self.interpolation()?);
                }
                other => {
                    lit.push(other);
                    self.pos += 1;
                }
            }
        }
        Ok(())
    }

    fn lex_string(&mut self) -> Result<Tok, Diagnostic> {
        let open_span = self.span();
        self.pos += 1;
        let mut parts = Vec::new();
        let mut lit = String::new();
        loop {
            let Some(c) = self.peek(0) else {
                return Err(Diagnostic::new(
                    "syntax",
                    "unterminated string".to_string(),
                    open_span,
                ));
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
                    parts.push(self.interpolation()?);
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
        (_, Tok::RParen) | (_, Tok::RGroup) | (_, Tok::RBracket) => 0,
        (Tok::LParen, _) | (Tok::LGroup, _) | (Tok::LBracket, _) => 0,
        (_, Tok::Colon) => 0,
        // field access hugs both neighbors: u.age
        (_, Tok::Dot) | (Tok::Dot, _) => 0,
        // the strict-index sigil hugs its bracket: xs[i]!
        (Tok::RBracket, Tok::Bang) => 0,
        // the partial sigil hugs its name: &add 2
        (Tok::Op("&"), Tok::Ident(_)) => 0,
        // the as-pattern's sigil hugs both sides: r@(rect w h)
        (_, Tok::At) | (Tok::At, _) => 0,
        // A map's braces hold their contents apart — `{ "a":1 "b":2 }` — but
        // the empty pair has no contents to hold, so it closes on itself and
        // `{ }` is the non-canonical spelling of it.
        (Tok::LBrace, Tok::RBrace) => 0,
        _ => 1,
    }
}

fn validate_spacing(lexed_line: &LexedLine, line: usize, diags: &mut Vec<Diagnostic>) {
    for (at, pair) in lexed_line.tokens.windows(2).enumerate() {
        let (prev, _, prev_end) = &pair[0];
        let (next, next_span, _) = &pair[1];
        let gap = (next_span.col as usize).saturating_sub(*prev_end as usize);
        if matches!(prev, Tok::Colon) {
            if gap > 1 {
                diags.push(Diagnostic::new(
                    "formatting",
                    "canonical form requires at most one space here".to_string(),
                    Span::at(line, next_span.col as usize),
                ));
            }
            continue;
        }
        if matches!(
            (prev, next),
            (Tok::Ident(_) | Tok::RParen | Tok::RGroup | Tok::RBracket, Tok::LBracket)
        ) {
            if gap > 1 {
                diags.push(Diagnostic::new(
                    "formatting",
                    "canonical form requires at most one space here".to_string(),
                    Span::at(line, next_span.col as usize),
                ));
            }
            continue;
        }
        // `f(x)` is how every C-shaped language spells a call, so it is the
        // first thing somebody writes here, and the general spacing rule
        // answers it by talking about spaces. Application is a space.
        if let (Tok::Ident(name), Tok::LParen) = (prev, next) {
            if gap == 0 {
                // `foo()` runs a value that is waiting to be called, which is
                // what `&` leaves when it has supplied every argument. Only a
                // parenthesis holding something is the C-shaped call.
                if matches!(lexed_line.tokens.get(at + 2), Some((Tok::RParen, _, _))) {
                    continue;
                }
                diags.push(Diagnostic::new(
                    "formatting",
                    format!(
                        "application is a space, so `{name} x` calls `{name}` — \
                         `{name}(x)` reads as `{name}` applied to nothing, then a \
                         parenthesised `x`"
                    ),
                    Span::at(line, next_span.col as usize),
                ));
                continue;
            }
        }
        // `&` hugs its name as the partial sigil and takes a space as the
        // bitwise operator, and the pair alone cannot tell them apart: what
        // decides is whether something precedes it that a value can end with.
        // `x & y` is an operator; `apply &y` is a sigil.
        let infix_amp = matches!((prev, next), (Tok::Op("&"), Tok::Ident(_)))
            && at > 0
            && matches!(
                lexed_line.tokens.get(at - 1).map(|(t, _, _)| t),
                Some(
                    Tok::Ident(_)
                        | Tok::Int(_)
                        | Tok::Float(_)
                        | Tok::Str(_)
                        | Tok::RParen
                        | Tok::RGroup
                        | Tok::RBracket
                        | Tok::Bang
                )
            );
        // The ratified slice is prefix — `[]int`, not `int[]` — so an empty
        // bracket pair hugs the type it slices. The pair alone cannot say so:
        // `merge [] left right` passes an empty list and then an argument.
        // What decides is the colon, because only an annotation follows one.
        // Walking back over a run of empty pairs also admits `[][]int`.
        let slice_marker = matches!((prev, next), (Tok::RBracket, Tok::Ident(_))) && at > 0 && {
            let mut k = at;
            while k >= 1
                && matches!(lexed_line.tokens.get(k - 1).map(|(t, _, _)| t), Some(Tok::LBracket))
                && matches!(lexed_line.tokens.get(k).map(|(t, _, _)| t), Some(Tok::RBracket))
            {
                k = k.saturating_sub(2);
            }
            matches!(lexed_line.tokens.get(k).map(|(t, _, _)| t), Some(Tok::Colon))
        };
        let required = match (infix_amp, slice_marker) {
            (_, true) => 0,
            (true, _) => 1,
            _ => required_gap(prev, next),
        };
        if gap != required {
            let wanted = match required {
                0 => "no space".to_string(),
                _ => "exactly one space".to_string(),
            };
            diags.push(Diagnostic::new(
                "formatting",
                format!("canonical form requires {wanted} here"),
                Span::at(line, next_span.col as usize),
            ));
        }
    }
}
