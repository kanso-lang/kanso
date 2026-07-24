use crate::ast::Program;
use crate::eval::{render, Desc, Executor, Interp, Value};
use crate::{check, diag, lexer, parser};

/// One committed input: a declaration (all arms of one name), or a synthetic
/// `itN` constant wrapping an expression.
#[derive(Clone)]
struct Unit {
    name: String,
    /// One entry per arm: (signature fingerprint, source). Arms of one name
    /// stay in a single unit so they compile adjacent, as the formatter
    /// requires; replacement is keyed on the fingerprint, so a new signature
    /// adds an arm and only an identical one replaces.
    arms: Vec<(String, String)>,
    is_type: bool,
}

impl Unit {
    fn source(&self) -> String {
        let sources: Vec<&str> = self.arms.iter().map(|(_, s)| s.as_str()).collect();
        sources.join("\n\n")
    }
}

pub struct Session {
    units: Vec<Unit>,
    counter: usize,
}

pub enum Outcome {
    /// A declaration was committed; the string is the full echo, e.g.
    /// `defined foo` or `redefined greet`.
    Defined(String),
    /// An expression evaluated to this rendered value.
    Value(String),
    /// An expression evaluated to a description, which was executed.
    Executed(String),
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Session { units: Vec::new(), counter: 0 }
    }

    /// The name the next expression result will be bound to.
    pub fn next_it(&self) -> String {
        format!("it{}", self.counter)
    }

    pub fn eval(&mut self, input: &str, executor: &mut dyn Executor) -> Result<Outcome, String> {
        let trimmed = input.trim_end();
        if trimmed.trim().is_empty() {
            return Ok(Outcome::Value(String::new()));
        }
        if !declaration_intent(trimmed) {
            return self.eval_expression(trimmed, executor);
        }
        // a paste may mix declarations with trailing expressions: commit
        // the declarations first, then evaluate each expression in order
        let mut decl_chunks: Vec<String> = Vec::new();
        let mut expr_chunks: Vec<String> = Vec::new();
        for chunk in split_declarations(trimmed) {
            match declaration_intent(&chunk) {
                true => decl_chunks.push(chunk),
                false => expr_chunks.push(chunk),
            }
        }
        let mut lines: Vec<String> = Vec::new();
        if !decl_chunks.is_empty() {
            match self.eval_declarations(&decl_chunks.join("\n\n"))? {
                Outcome::Defined(echo) => lines.push(echo),
                Outcome::Value(v) | Outcome::Executed(v) => lines.push(v),
            }
        }
        if expr_chunks.is_empty() {
            return Ok(Outcome::Defined(lines.join("\n")));
        }
        for chunk in expr_chunks {
            match self.eval_expression(&chunk, executor)? {
                Outcome::Value(v) | Outcome::Executed(v) => {
                    if !v.trim().is_empty() {
                        lines.push(v);
                    }
                }
                Outcome::Defined(echo) => lines.push(echo),
            }
        }
        Ok(Outcome::Value(lines.join("\n")))
    }

    /// Arms accumulate as overloads of a name — dispatch is open, so a new
    /// signature adds an arm. Only an identical signature replaces its arm;
    /// redefining a type replaces the type.
    fn eval_declarations(&mut self, input: &str) -> Result<Outcome, String> {
        let mut incoming: Vec<Unit> = Vec::new();
        for chunk in split_declarations(input) {
            let program = parse_fragment(&chunk)?;
            let name = first_declared_name(&program);
            let is_type = !program.types.is_empty();
            let print = fingerprint(&program, is_type);
            match incoming.iter_mut().find(|u| u.name == name) {
                Some(unit) => merge_arm(unit, print, chunk),
                None => incoming.push(Unit { name, arms: vec![(print, chunk)], is_type }),
            }
        }
        let mut echo: Vec<String> = Vec::new();
        let mut candidate = self.units.clone();
        for unit in incoming {
            match candidate.iter_mut().find(|u| u.name == unit.name) {
                Some(existing) if unit.is_type || existing.is_type => {
                    echo.push(format!("redefined {}", unit.name));
                    *existing = unit;
                }
                Some(existing) => {
                    let mut added = false;
                    for (print, source) in unit.arms {
                        match existing.arms.iter_mut().find(|(p, _)| *p == print) {
                            Some(arm) => arm.1 = source,
                            None => {
                                existing.arms.push((print, source));
                                added = true;
                            }
                        }
                    }
                    echo.push(match added {
                        true => format!("overloaded {}", existing.name),
                        false => format!("redefined {}", existing.name),
                    });
                }
                None => {
                    echo.push(format!("defined {}", unit.name));
                    candidate.push(unit);
                }
            }
        }
        let _ = compile_units(&candidate)?;
        self.units = candidate;
        Ok(Outcome::Defined(echo.join(", ")))
    }

    /// `:delete name` — remove a declaration from the session, refused (with
    /// the compiler's own evidence) while anything still depends on it.
    pub fn delete(&mut self, name: &str) -> Result<String, String> {
        if !self.units.iter().any(|u| u.name == name) {
            return Err(format!("error[name]: nothing named `{name}` is defined\n"));
        }
        let mut candidate = self.units.clone();
        candidate.retain(|u| u.name != name);
        match compile_units(&candidate) {
            Ok(_) => {
                self.units = candidate;
                Ok(format!("deleted {name}"))
            }
            Err(diag) => Err(format!("cannot delete `{name}` — the session still uses it:\n{diag}")),
        }
    }

    /// `:show` — the session as its canonical file; `:show name` — one
    /// declaration's source, without running anything.
    pub fn show(&self, name: Option<&str>) -> Result<String, String> {
        match name {
            None => match self.units.is_empty() {
                true => Ok("the session is empty".to_string()),
                false => Ok(assemble(&self.units).trim_end().to_string()),
            },
            Some(n) => self
                .units
                .iter()
                .find(|u| u.name == n)
                .map(|u| u.source())
                .ok_or_else(|| format!("error[name]: nothing named `{n}` is defined\n")),
        }
    }

    fn eval_expression(
        &mut self,
        input: &str,
        executor: &mut dyn Executor,
    ) -> Result<Outcome, String> {
        let name = self.next_it();
        if mentions(input, &name) {
            return Err(format!("error[name]: unknown name `{name}`\n"));
        }
        let source = wrap_expression(&name, input);
        let mut candidate = self.units.clone();
        candidate.push(Unit { name: name.clone(), arms: vec![(name.clone(), source)], is_type: false });
        let program = compile_units(&candidate)?;
        let interp = Interp::new(&program);
        let result = interp.run_named(&name).expect("just-committed constant resolves");
        let value = match result {
            Ok(value) => value,
            Err(runtime) => return Err(format!("error[runtime]: {}\n", runtime.message)),
        };
        self.units = candidate;
        self.counter += 1;
        match value {
            Value::Desc(desc) => execute(&interp, &desc, executor),
            other => Ok(Outcome::Value(render(&other, true))),
        }
    }
}

fn execute(
    interp: &Interp,
    desc: &Desc,
    executor: &mut dyn Executor,
) -> Result<Outcome, String> {
    match interp.execute(desc, executor) {
        Ok(Value::ErrV(info)) => Err(format!(
            "error[endpoint]: unhandled err reached the executor: {}\n{}",
            render(&info.reason, true),
            crate::eval::trace_lines(&info)
        )),
        Ok(Value::NoneV) => Ok(Outcome::Executed(String::new())),
        Ok(other) => Ok(Outcome::Executed(render(&other, true))),
        Err(runtime) => Err(format!("error[runtime]: {}\n", runtime.message)),
    }
}

fn compile_units(units: &[Unit]) -> Result<Program, String> {
    let source = assemble(units);
    let lexed = lexer::lex(&source).map_err(|d| diag::render(&d, "repl", &source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, "repl", &source))?;
    for decl in &mut program.fns {
        decl.file = "repl".to_string();
    }
    let diags: Vec<diag::Diagnostic> = check::check(&mut program, false)
        .into_iter()
        .filter(|d| d.kind != "unused")
        .collect();
    match diags.is_empty() {
        true => Ok(program),
        false => Err(diag::render(&diags, "repl", &source)),
    }
}

/// The session rendered as its canonical file: types first, then values,
/// each alphabetical — the repl canonicalizes on the user's behalf, so
/// declaration order at the prompt is exploration order, never an error.
fn assemble(units: &[Unit]) -> String {
    let mut sorted: Vec<&Unit> = units.iter().collect();
    sorted.sort_by_key(|u| (!u.is_type, u.name.as_str()));
    let sources: Vec<String> = sorted.iter().map(|u| u.source()).collect();
    format!("{}\n", sources.join("\n\n"))
}

/// An arm's dispatch signature, canonically rendered: name, arity, and the
/// shape of each parameter pattern. Two arms with the same fingerprint match
/// identically, so the later one replaces; different fingerprints coexist.
fn fingerprint(program: &Program, is_type: bool) -> String {
    if is_type {
        return "type".to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    for f in &program.fns {
        let shapes: Vec<String> = f.params.iter().map(pattern_shape).collect();
        parts.push(format!("{}/{}({})", f.name, f.params.len(), shapes.join(" ")));
    }
    parts.join(";")
}

fn pattern_shape(p: &crate::ast::Pattern) -> String {
    use crate::ast::Pattern;
    match p {
        Pattern::IntLit(n, _) => n.to_string(),
        Pattern::StrLit(s, _) => format!("{s:?}"),
        Pattern::Nullary(n, _) => n.clone(),
        Pattern::Var(..) | Pattern::Wildcard(_) => "_".to_string(),
        Pattern::Annotated { ty, .. } => format!("_:{ty}"),
        Pattern::Ctor { ty, fields } => {
            let inner: Vec<String> = fields.iter().map(pattern_shape).collect();
            format!("{ty}({})", inner.join(" "))
        }
        Pattern::Keyed { entries, .. } => {
            let keys: Vec<&str> = entries.iter().map(|e| e.field.as_str()).collect();
            format!("{{{}}}", keys.join(" "))
        }
    }
}

fn merge_arm(unit: &mut Unit, print: String, source: String) {
    match unit.arms.iter_mut().find(|(p, _)| *p == print) {
        Some(arm) => arm.1 = source,
        None => unit.arms.push((print, source)),
    }
}

fn first_declared_name(program: &Program) -> String {
    program
        .fns
        .iter()
        .map(|f| f.name.clone())
        .chain(program.types.iter().map(|t| t.name.clone()))
        .next()
        .unwrap_or_default()
}

/// Whether the input reads as a declaration (vs an expression to evaluate).
fn declaration_intent(input: &str) -> bool {
    let first = input.lines().next().unwrap_or("");
    first.starts_with("fn ") || first.starts_with("type ") || constant_head(first)
}

/// `name = ...` at the start of a line: a constant declaration.
fn constant_head(line: &str) -> bool {
    let Some(eq) = line.find('=') else {
        return false;
    };
    let head = line[..eq].trim_end();
    let named = !head.is_empty()
        && head.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && head.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    named && !line[eq + 1..].starts_with('=')
}

/// Top level is declarations only, so every column-0 line opens a new one.
fn split_declarations(input: &str) -> Vec<String> {
    let mut chunks: Vec<Vec<&str>> = Vec::new();
    for line in input.lines() {
        let opens = !line.is_empty() && !line.starts_with(' ');
        match opens || chunks.is_empty() {
            true => chunks.push(vec![line]),
            false => chunks.last_mut().expect("chunk exists").push(line),
        }
    }
    chunks
        .into_iter()
        .map(|lines| lines.join("\n").trim_end().to_string())
        .filter(|c| !c.trim().is_empty())
        .collect()
}

fn parse_fragment(chunk: &str) -> Result<Program, String> {
    let source = format!("{chunk}\n");
    let lexed = lexer::lex(&source).map_err(|d| diag::render(&d, "repl", &source))?;
    parser::parse(&lexed).map_err(|d| diag::render(&d, "repl", &source))
}

/// The synthetic constant must not appear in its own body: `it0` before
/// anything is bound would otherwise define itself and recurse forever.
/// Strings count too — interpolation makes them code.
fn mentions(input: &str, name: &str) -> bool {
    input
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| word == name)
}

fn wrap_expression(name: &str, input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    match lines.len() {
        1 => format!("{name} = {input}"),
        _ => {
            let body: Vec<String> = lines.iter().map(|l| format!("  {l}")).collect();
            format!("{name} =\n{}", body.join("\n"))
        }
    }
}
