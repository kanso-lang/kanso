pub mod ast;
pub mod check;
pub mod codegen;
pub mod diag;
pub mod eval;
pub mod lexer;
pub mod parser;

pub fn compile(file: &str, source: &str, require_main: bool) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    let diags = check::check(&program, require_main);
    match diags.is_empty() {
        true => Ok(program),
        false => Err(diag::render(&diags, file, source)),
    }
}
