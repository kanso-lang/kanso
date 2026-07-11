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
