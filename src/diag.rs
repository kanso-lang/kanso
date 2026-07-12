#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub kind: &'static str,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(kind: &'static str, message: String, span: Span) -> Self {
        Diagnostic { kind, message, span }
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
    let truecolor = std::env::var("COLORTERM")
        .is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"));
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
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    for d in diags {
        out.push_str(&format!("error[{}]: {}\n", d.kind, d.message));
        out.push_str(&format!("  --> {}:{}:{}\n", file, d.span.line, d.span.col));
        if d.span.line >= 1 && d.span.line <= lines.len() {
            let src_line = lines[d.span.line - 1];
            let num = format!("{:>4}", d.span.line);
            out.push_str(&format!("{} | {}\n", num, src_line));
            let pad = " ".repeat(num.len() + 3 + d.span.col.saturating_sub(1));
            out.push_str(&format!("{}^\n", pad));
        }
    }
    out
}
