use crate::ast::Program;
use crate::eval::{render, Desc, Executor, Interp, Value};
use crate::{check, diag, lexer, parser};

/// One committed input: a declaration (all arms of one name), or a synthetic
/// `itN` constant wrapping an expression.
#[derive(Clone)]
struct Unit {
    name: String,
    source: String,
}

pub struct Session {
    units: Vec<Unit>,
    counter: usize,
}

pub enum Outcome {
    /// A declaration was committed; the string names it.
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
        match declaration_intent(trimmed) {
            true => self.eval_declarations(trimmed),
            false => self.eval_expression(trimmed, executor),
        }
    }

    /// Redefining a name replaces its previous definition (all arms); arms
    /// arriving within one input accumulate as overloads of that name.
    fn eval_declarations(&mut self, input: &str) -> Result<Outcome, String> {
        let mut incoming: Vec<Unit> = Vec::new();
        for chunk in split_declarations(input) {
            let program = parse_fragment(&chunk)?;
            let name = first_declared_name(&program);
            match incoming.iter_mut().find(|u| u.name == name) {
                Some(unit) => unit.source = format!("{}\n\n{chunk}", unit.source),
                None => incoming.push(Unit { name, source: chunk }),
            }
        }
        let names: Vec<String> = incoming.iter().map(|u| u.name.clone()).collect();
        let mut candidate = self.units.clone();
        candidate.retain(|u| !names.contains(&u.name));
        candidate.extend(incoming);
        let _ = compile_units(&candidate)?;
        self.units = candidate;
        Ok(Outcome::Defined(names.join(" ")))
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
        candidate.push(Unit { name: name.clone(), source });
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

fn assemble(units: &[Unit]) -> String {
    let mut sorted: Vec<&Unit> = units.iter().collect();
    sorted.sort_by_key(|u| u.name.as_str());
    let sources: Vec<&str> = sorted.iter().map(|u| u.source.as_str()).collect();
    format!("{}\n", sources.join("\n\n"))
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
