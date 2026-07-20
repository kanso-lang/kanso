pub mod ast;
pub mod beat;
pub mod check;
pub mod codegen;
pub mod diag;
pub mod dispatch;
pub mod escape;
pub mod eval;
pub mod infer;
pub mod lexer;
pub mod linear;
pub mod parser;
pub mod repl;
pub mod wasm;
pub mod wasm_backend;
pub mod wasm_encode;
pub mod wasm_rt;

pub fn compile(file: &str, source: &str, require_main: bool) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    stamp_file(&mut program, file);
    let diags = check::check(&mut program, require_main);
    match diags.is_empty() {
        true => Ok(program),
        false => Err(diag::render(&diags, file, source)),
    }
}

/// Err origins name the function and the file it lives in; the file is
/// per-declaration so it survives multi-file module merging.
fn stamp_file(program: &mut ast::Program, file: &str) {
    for decl in &mut program.fns {
        decl.file = file.to_string();
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
        let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, &file, &source))?;
        stamp_file(&mut program, &file);
        parsed.push((file, source, program));
    }
    let mut all_names = std::collections::HashSet::new();
    let mut all_markers = std::collections::HashSet::new();
    let mut all_type_names = std::collections::HashSet::new();
    for (_, _, program) in &parsed {
        all_names.extend(check::declared_names(program));
        all_markers.extend(check::marker_names(program));
        all_type_names.extend(program.types.iter().map(|t| t.name.clone()));
    }
    let mut used = std::collections::HashSet::new();
    for (file, source, program) in &mut parsed {
        let mut extern_globals = all_names.clone();
        for name in check::declared_names(program) {
            extern_globals.remove(&name);
        }
        let mut diags = check::resolve_markers(program, &all_markers);
        diags.extend(check::check_typesets(program, &all_type_names));
        diags.extend(check::check_file(program, &extern_globals, &mut used));
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        if !diags.is_empty() {
            return Err(diag::render(&diags, file, source));
        }
    }
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new() };
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
