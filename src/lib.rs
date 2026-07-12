pub mod ast;
pub mod check;
pub mod codegen;
pub mod diag;
pub mod eval;
pub mod infer;
pub mod lexer;
pub mod parser;
pub mod repl;

pub fn compile(file: &str, source: &str, require_main: bool) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    let diags = check::check(&program, require_main);
    match diags.is_empty() {
        true => Ok(program),
        false => Err(diag::render(&diags, file, source)),
    }
}

/// A module is a directory: every .kso file in it shares one namespace.
/// Canonical ordering holds per file; an overload group lives in one file.
pub fn compile_module(dir: &std::path::Path, require_main: bool) -> Result<ast::Program, String> {
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|io| format!("error: cannot read {}: {io}\n", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e == "kso"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("error: no .kso files in {}\n", dir.display()));
    }
    let mut parsed = Vec::new();
    for path in &paths {
        let file = path.to_string_lossy().to_string();
        let source = std::fs::read_to_string(path)
            .map_err(|io| format!("error: cannot read {file}: {io}\n"))?;
        let lexed = lexer::lex(&source).map_err(|d| diag::render(&d, &file, &source))?;
        let program = parser::parse(&lexed).map_err(|d| diag::render(&d, &file, &source))?;
        parsed.push((file, source, program));
    }
    let mut all_names = std::collections::HashSet::new();
    for (_, _, program) in &parsed {
        all_names.extend(check::declared_names(program));
    }
    let mut used = std::collections::HashSet::new();
    for (file, source, program) in &parsed {
        let mut extern_globals = all_names.clone();
        for name in check::declared_names(program) {
            extern_globals.remove(&name);
        }
        let diags = check::check_file(program, &extern_globals, &mut used);
        if !diags.is_empty() {
            return Err(diag::render(&diags, file, source));
        }
    }
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new() };
    for (_, _, program) in parsed {
        merged.types.extend(program.types);
        merged.fns.extend(program.fns);
    }
    let mut diags = check::check_merged(&merged, require_main);
    check::check_unused_private(&merged, &used, &mut diags);
    if !diags.is_empty() {
        let file = dir.to_string_lossy();
        let rendered: Vec<String> = diags
            .iter()
            .map(|d| format!("error[{}]: {} (module {file})\n", d.kind, d.message))
            .collect();
        return Err(rendered.join(""));
    }
    Ok(merged)
}
