#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl Span {
    /// A line and column counted as `usize` by the lexer and the passes,
    /// narrowed to what the struct holds. Four billion lines is the limit and
    /// no file reaches it.
    pub const fn at(line: usize, col: usize) -> Span {
        Span { line: line as u32, col: col as u32 }
    }
}

#[derive(Debug)]
pub struct Diagnostic {
    pub kind: &'static str,
    pub message: String,
    pub span: Span,
    /// The file this diagnostic is ABOUT, when that is not the file being
    /// rendered against. `None` means the rendering file, which is every
    /// per-file check: the caller already knows what it handed in.
    ///
    /// A check over a MERGED program is the case that needs it. The merge
    /// holds every dependency's declarations, so a whole-program check walks
    /// files the caller never named, and a diagnostic raised there was
    /// reported against whatever file the caller happened to be rendering.
    ///
    /// It is set AT THE RAISE SITE, from the declaration in hand. A 2026-09-08
    /// prototype read it from a thread-local the walk set instead, and a
    /// thread-local names whoever is iterating rather than what the diagnostic
    /// is about: its guard outlived the raise site and leaked one file's
    /// attribution onto a diagnostic raised somewhere else entirely. A value
    /// passed in at the raise site cannot do that, which is the whole reason
    /// this is a field rather than an ambient.
    ///
    /// It does not ride on `Span`: kanso#1135 made a span two u32 for a 7.1%
    /// peak win, and an `Arc<str>` there hands that back on every span in the
    /// tree. A diagnostic is built only when a compile is already failing, so
    /// the pointer costs nothing that matters.
    pub file: Option<std::sync::Arc<str>>,
}

impl Diagnostic {
    pub fn new(kind: &'static str, message: String, span: Span) -> Self {
        Diagnostic { kind, message, span, file: None }
    }

    /// The same diagnostic, about a named file rather than the one being
    /// rendered. Hand it the declaration the check has in hand — never a file
    /// read from anywhere else, which is the mistake this replaced.
    pub fn about(mut self, file: &std::sync::Arc<str>) -> Self {
        self.file = Some(file.clone());
        self
    }
}

/// Terminal color for diagnostics, from the site palette: vermillion for the
/// error kind and caret, dim for locations and propagation traces. Applied
/// only when stderr is a tty and NO_COLOR is unset, so piped output (goldens,
/// harnesses, CI) stays byte-identical plain text.
pub fn paint(plain: &str) -> String {
    use std::io::IsTerminal;
    let colorable = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    match colorable {
        true => paint_lines(plain, vermillion()),
        false => plain.to_string(),
    }
}

fn vermillion() -> &'static str {
    let truecolor =
        std::env::var("COLORTERM").is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"));
    match truecolor {
        true => "\x1b[38;2;240;58;0m",
        false => "\x1b[38;5;202m",
    }
}

const DIM: &str = "\x1b[2m";
const OFF: &str = "\x1b[0m";

fn paint_lines(plain: &str, err_color: &str) -> String {
    let mut out = String::new();
    for line in plain.split_inclusive('\n') {
        let text = line.strip_suffix('\n').unwrap_or(line);
        let newline = match line.ends_with('\n') {
            true => "\n",
            false => "",
        };
        out.push_str(&paint_line(text, err_color));
        out.push_str(newline);
    }
    out
}

fn paint_line(text: &str, err_color: &str) -> String {
    if let Some(rest) = header_rest(text) {
        let head_len = text.len() - rest.len();
        return format!("{err_color}{}{OFF}{rest}", &text[..head_len]);
    }
    if text.starts_with("  --> ")
        || text.starts_with("  born in ")
        || text.starts_with("  passed through ")
    {
        return format!("{DIM}{text}{OFF}");
    }
    if !text.is_empty() && text.trim_start() == "^" {
        return format!("{err_color}{text}{OFF}");
    }
    text.to_string()
}

/// For a diagnostic header line, the text after the `error[kind]:` token.
fn header_rest(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("error[")?;
    let close = rest.find("]:")?;
    Some(&rest[close + 2..])
}

pub fn render(diags: &[Diagnostic], file: &str, source: &str) -> String {
    render_across(diags, file, source, &[])
}

/// Render diagnostics that may be about files other than the one handed in.
///
/// `sources` is every other file whose text is in hand, by name. A diagnostic
/// naming one of them quotes ITS line; one naming a file that is not in hand
/// still reports against that file and simply quotes nothing, because the
/// header is the half a reader needs and quoting the wrong file's line at the
/// same number would be worse than quoting none.
pub fn render_across(
    diags: &[Diagnostic],
    file: &str,
    source: &str,
    sources: &[(&str, &str)],
) -> String {
    let mut out = String::new();
    for d in diags {
        let (name, text) = match d.file.as_deref() {
            None => (file, source),
            Some(own) if own == file => (file, source),
            Some(own) => match sources.iter().find(|(f, _)| *f == own) {
                Some((f, s)) => (*f, *s),
                None => (own, ""),
            },
        };
        out.push_str(&format!("error[{}]: {}\n", d.kind, d.message));
        out.push_str(&format!("  --> {}:{}:{}\n", name, d.span.line, d.span.col));
        let line = d.span.line as usize;
        let lines: Vec<&str> = text.lines().collect();
        if line >= 1 && line <= lines.len() {
            let src_line = lines[line - 1];
            let num = format!("{:>4}", d.span.line);
            out.push_str(&format!("{} | {}\n", num, src_line));
            let pad = " ".repeat(num.len() + 3 + (d.span.col as usize).saturating_sub(1));
            out.push_str(&format!("{}^\n", pad));
        }
    }
    out
}

/// A name with the article English gives it. "an int", "a string" — a
/// diagnostic that fumbles its own grammar reads as carelessness about
/// everything else in it.
///
/// It lives here because three engines write sentences that need it and each
/// had its own copy of the wording. The rule is spelling, not pronunciation:
/// a name is a program's word rather than an English one, and the letter is
/// what a reader has in front of them.
pub fn article(word: &str) -> String {
    match word.starts_with(['a', 'e', 'i', 'o', 'u']) {
        true => format!("an {word}"),
        false => format!("a {word}"),
    }
}
