pub mod advisory;
pub mod ast;
pub mod beat;
pub mod check;
pub mod codegen;
pub mod demand;
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

/// Compile a single file as an entry: its statements are the program.
pub fn compile_entry(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse_entry(&lexed).map_err(|d| diag::render(&d, file, source))?;
    stamp_file(&mut program, file);
    let base = std::path::Path::new(file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let mut import_paths: Vec<String> = program.imports.iter().map(|i| i.path.clone()).collect();
    ambient_imports(&mut import_paths);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_paths, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    diags.extend(unused_imports(&program.imports, &quals));
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    // per-file rules for the entry against the dependency globals, then the
    // merged checks — never file-order rules across module boundaries
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let extern_globals = check::declared_names(&dep_program);
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file(&program, &extern_globals, &mut used));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new() };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    merged.types.extend(program.types);
    merged.fns.extend(program.fns);
    let mut merged_diags = check::check_merged(&merged, true);
    check::check_unused_private(&merged, &used, &mut merged_diags);
    let merged_diags: Vec<_> = merged_diags
        .into_iter()
        .filter(|d| d.kind != "unused")
        .collect();
    match merged_diags.is_empty() {
        true => Ok(merged),
        false => Err(diag::render(&merged_diags, file, source)),
    }
}

/// `kanso play`: the playground's convention at the terminal. The file is a
/// library defining `pub play`; the synthesized entry runs it.
pub fn compile_play(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    // a play library may import like any module; the ambient module rides
    let mut import_paths: Vec<String> = program.imports.iter().map(|i| i.path.clone()).collect();
    ambient_imports(&mut import_paths);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_paths, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    diags.extend(unused_imports(&program.imports, &quals));
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    let extern_globals = check::declared_names(&dep_program);
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file(&program, &extern_globals, &mut used));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    let has_play = program.fns.iter().any(|d| d.name == "play" && d.is_pub);
    if !has_play {
        return Err(format!(
            "error: nothing to play — define `pub play`, or point `kanso run` \
             at an entry file\n  --> {file}\n"
        ));
    }
    let span = diag::Span { line: 1, col: 1 };
    program.fns.push(ast::FnDecl {
        name: "main".to_string(),
        params: Vec::new(),
        body: vec![ast::Stmt::Expr(ast::Expr::Ident("play".to_string(), span))],
        span,
        is_pub: false,
        file: file.to_string(),
    });
    Ok(program)
}

/// A lone library file under a library verb (`kanso test`/`check`): parses
/// as a library and loads its imports (plus the ambient module) like any
/// other root compile.
pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let mut import_paths: Vec<String> = program.imports.iter().map(|i| i.path.clone()).collect();
    ambient_imports(&mut import_paths);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_paths, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    diags.extend(unused_imports(&program.imports, &quals));
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    let extern_globals = check::declared_names(&dep_program);
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file(&program, &extern_globals, &mut used));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    Ok(program)
}

/// Route a single source file to the right compile for a verb, by content:
/// `pub play` is a play library, bare statements are an entry, definitions
/// alone are a library (runnable only under a library verb like `test`).
/// The CLI and the browser share this so the engines never diverge on which
/// compile a file gets.
pub fn compile_source(command: &str, file: &str, source: &str) -> Result<ast::Program, String> {
    let has_play = source.contains("pub play");
    let has_defs = source
        .lines()
        .any(|l| l.starts_with("fn ") || l.starts_with("type ") || l.starts_with("pub "));
    let library_verb = command == "test";
    match (command, has_play, has_defs) {
        ("play", _, _) => compile_play(file, source),
        (_, true, _) if !library_verb => compile_play(file, source),
        ("check", false, true) => compile_library(file, source),
        (_, false, true) if !library_verb => Err(format!(
            "error: `{file}` is a library — nothing to run. give the \
             module a main.kso entry, or define `pub play` and use \
             `kanso play`\n"
        )),
        _ if library_verb => compile_library(file, source),
        _ => compile_entry(file, source),
    }
}

/// Err origins name the function and the file it lives in; the file is
/// per-declaration so it survives multi-file module merging.
fn stamp_file(program: &mut ast::Program, file: &str) {
    for decl in &mut program.fns {
        decl.file = file.to_string();
    }
}

/// Resolve one import path to a directory, per the gaveled table: `std/` is
/// the toolchain's shipped library, `owner/repo[...]` is the hako cache, and
/// anything else is relative to the importing module's directory.
fn resolve_import(base: &std::path::Path, path: &str) -> Result<std::path::PathBuf, String> {
    if let Some(rest) = path.strip_prefix("std/") {
        let toolchain = std::env::var("KANSO_STD")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("../../lib")))
                    .unwrap_or_else(|| std::path::PathBuf::from("lib"))
            });
        let dir = toolchain.join(rest);
        if dir.is_dir() {
            return Ok(dir);
        }
        // a source checkout: lib/ beside the working directory
        let local = std::path::PathBuf::from("lib").join(rest);
        if local.is_dir() {
            return Ok(local);
        }
        return Err(format!("error: `std/{rest}` is not in the shipped library\n"));
    }
    let relative = base.join(path);
    if relative.is_dir() {
        return Ok(relative);
    }
    let cache = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".hako").join(path))
        .unwrap_or_default();
    if cache.is_dir() {
        return Ok(cache);
    }
    Err(format!(
        "error: cannot resolve import \"{path}\" — not a sibling directory, \
         not in the hako cache (run `kanso install`)\n"
    ))
}

/// The last path segment names the module at use sites: `import "std/json"`
/// qualifies as `json/...`.
fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Prefix every top-level name of `dep` with `qual/`, rewriting the module's
/// own references so it still resolves internally, and record which
/// qualified names are pub — the boundary the checker enforces.
fn qualify(dep: &mut ast::Program, qual: &str, exports: &mut std::collections::HashMap<String, bool>) {
    let owned: std::collections::HashSet<String> = check::declared_names(dep);
    for ty in &mut dep.types {
        exports.insert(format!("{qual}/{}", ty.name), ty.is_pub);
        ty.name = format!("{qual}/{}", ty.name);
        for (_, members, _) in &mut ty.fields {
            for member in members {
                if owned.contains(member.as_str()) {
                    *member = format!("{qual}/{member}");
                }
            }
        }
    }
    for f in &mut dep.fns {
        exports.entry(format!("{qual}/{}", f.name)).or_insert(f.is_pub);
        f.name = format!("{qual}/{}", f.name);
        for stmt in &mut f.body {
            rewrite_stmt(stmt, qual, &owned);
        }
        for p in &mut f.params {
            rewrite_pattern(p, qual, &owned);
        }
    }
}

fn rewrite_pattern(p: &mut ast::Pattern, qual: &str, owned: &std::collections::HashSet<String>) {
    match p {
        ast::Pattern::Ctor { ty, fields } => {
            if owned.contains(ty.as_str()) {
                *ty = format!("{qual}/{ty}");
            }
            for f in fields {
                rewrite_pattern(f, qual, owned);
            }
        }
        ast::Pattern::Annotated { ty, .. } if owned.contains(ty.as_str()) => {
            *ty = format!("{qual}/{ty}");
        }
        _ => {}
    }
}

fn rewrite_stmt(stmt: &mut ast::Stmt, qual: &str, owned: &std::collections::HashSet<String>) {
    match stmt {
        ast::Stmt::Bind { expr, pattern } => {
            rewrite_pattern(pattern, qual, owned);
            rewrite_expr(expr, qual, owned);
        }
        ast::Stmt::Expr(e) => rewrite_expr(e, qual, owned),
    }
}

fn rewrite_expr(e: &mut ast::Expr, qual: &str, owned: &std::collections::HashSet<String>) {
    match e {
        ast::Expr::Ident(name, _) => {
            if owned.contains(name.as_str()) {
                *name = format!("{qual}/{name}");
            }
        }
        ast::Expr::Field { base, .. } => rewrite_expr(base, qual, owned),
        ast::Expr::App { head, args, .. } => {
            rewrite_expr(head, qual, owned);
            for a in args {
                rewrite_expr(a, qual, owned);
            }
        }
        ast::Expr::Index { base, index, .. } => {
            rewrite_expr(base, qual, owned);
            rewrite_expr(index, qual, owned);
        }
        ast::Expr::BinOp { lhs, rhs, .. } | ast::Expr::Join { lhs, rhs, .. } => {
            rewrite_expr(lhs, qual, owned);
            rewrite_expr(rhs, qual, owned);
        }
        ast::Expr::Seq(a, b, _) => {
            rewrite_expr(a, qual, owned);
            rewrite_expr(b, qual, owned);
        }
        ast::Expr::Lambda { body, .. } => rewrite_expr(body, qual, owned),
        ast::Expr::List(items, _) => {
            for i in items {
                rewrite_expr(i, qual, owned);
            }
        }
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                rewrite_expr(k, qual, owned);
                rewrite_expr(v, qual, owned);
            }
        }
        ast::Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    rewrite_expr(inner, qual, owned);
                }
            }
        }
        ast::Expr::Int(..) | ast::Expr::Float(..) => {}
    }
}

/// Modules linked into every program without an import statement: groups
/// that SYNTAX names (design/render-plan.md — "{x}" desugars to
/// render/to_string). Ambient types bring their canonical arms; imports
/// still govern bare-name spelling, so nothing here adds a visible name.
/// A local arm named for an ambient group's export joins that group: a
/// user's `fn to_string (money cents)` is an arm of render/to_string —
/// arming your own types needs no import (the ratified Ruby-shaped rule).
fn merge_ambient_arms(program: &mut ast::Program) -> Vec<diag::Diagnostic> {
    let local_types: std::collections::HashSet<&str> =
        program.types.iter().map(|t| t.name.as_str()).collect();
    let mut diags = Vec::new();
    for decl in &mut program.fns {
        if decl.name == "to_string" {
            // The ownership rule, enforced at the definition site: an arm
            // joining a group this module doesn't own must involve a type it
            // does own. Re-arming a primitive or the sentinels is reserved
            // to the stdlib; wrap the value in your own type instead.
            let owns_a_type = decl.params.iter().any(|p| match p {
                ast::Pattern::Ctor { ty, .. } => local_types.contains(ty.as_str()),
                ast::Pattern::Annotated { ty, .. } => local_types.contains(ty.as_str()),
                _ => false,
            });
            if !owns_a_type {
                diags.push(diag::Diagnostic {
                    kind: "ownership",
                    message: "an arm of `to_string` must match on a type this module defines — rendering of primitives and sentinels is fixed; wrap the value in your own type"
                        .to_string(),
                    span: decl.span,
                });
                continue;
            }
            decl.name = "render/to_string".to_string();
        }
    }
    diags
}

fn ambient_imports(import_paths: &mut Vec<String>) {
    if !import_paths.iter().any(|p| p == "std/render") {
        import_paths.push("std/render".to_string());
    }
}

/// Load and qualify every imported module, recursively.
fn load_dependencies(
    base: &std::path::Path,
    import_paths: &[String],
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(ast::Program, std::collections::HashMap<String, bool>), String> {
    let mut dep_program = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new() };
    let mut exports = std::collections::HashMap::new();
    for path in import_paths {
        // Embedded std modules load where no filesystem exists (the browser)
        // and where no lib/ ships beside the binary (installs). include_str!
        // of the same files keeps the embedded copies incapable of drifting.
        let embedded = match path.as_str() {
            "std/render" => Some(("render", include_str!("../lib/render/render.kso"))),
            "std/list" => Some(("list", include_str!("../lib/list/list.kso"))),
            "std/time" => Some(("time", include_str!("../lib/time/time.kso"))),
            "std/random" => Some(("random", include_str!("../lib/random/random.kso"))),
            "std/io" => Some(("io", include_str!("../lib/io/io.kso"))),
            "std/text" => Some(("text", include_str!("../lib/text/text.kso"))),
            "std/math" => Some(("math", include_str!("../lib/math/math.kso"))),
            _ => None,
        };
        if let Some((short, source)) = embedded {
            let mut dep = compile(&format!("{path}/{short}.kso"), source, false)?;
            qualify(&mut dep, short, &mut exports);
            dep_program.types.extend(dep.types);
            dep_program.fns.extend(dep.fns);
            continue;
        }
        let dep_dir = resolve_import(base, path)?;
        let mut dep = compile_module_inner(&dep_dir, false, visited)?;
        qualify(&mut dep, short_name(path), &mut exports);
        dep_program.types.extend(dep.types);
        dep_program.fns.extend(dep.fns);
    }
    Ok((dep_program, exports))
}

/// Every module qualifier the program references: `json/decode` marks
/// `json` as used, in expressions, patterns, and typeset members alike.
fn used_quals(program: &ast::Program, quals: &mut std::collections::HashSet<String>) {
    fn mark(name: &str, quals: &mut std::collections::HashSet<String>) {
        if let Some((qual, _)) = name.split_once('/') {
            quals.insert(qual.to_string());
        }
    }
    fn walk_pattern(p: &ast::Pattern, quals: &mut std::collections::HashSet<String>) {
        match p {
            ast::Pattern::Ctor { ty, fields } => {
                mark(ty, quals);
                for f in fields {
                    walk_pattern(f, quals);
                }
            }
            ast::Pattern::Annotated { ty, .. } => mark(ty, quals),
            _ => {}
        }
    }
    fn walk_expr(e: &ast::Expr, quals: &mut std::collections::HashSet<String>) {
        if let ast::Expr::Ident(name, _) = e {
            mark(name, quals);
        }
        for child in expr_children(e) {
            walk_expr(child, quals);
        }
    }
    for ty in &program.types {
        for (_, members, _) in &ty.fields {
            for member in members {
                mark(member, quals);
            }
        }
    }
    for f in &program.fns {
        for p in &f.params {
            walk_pattern(p, quals);
        }
        for stmt in &f.body {
            match stmt {
                ast::Stmt::Bind { expr, pattern } => {
                    walk_pattern(pattern, quals);
                    walk_expr(expr, quals);
                }
                ast::Stmt::Expr(e) => walk_expr(e, quals),
            }
        }
    }
}

/// An import no qualified name ever touches.
fn unused_imports(
    imports: &[ast::Import],
    quals: &std::collections::HashSet<String>,
) -> Vec<diag::Diagnostic> {
    imports
        .iter()
        .filter(|i| !quals.contains(short_name(&i.path)))
        .map(|i| {
            diag::Diagnostic::new(
                "unused",
                format!("unused import \"{}\"", i.path),
                i.span,
            )
        })
        .collect()
}

/// A positional read into a foreign type. Naming a foreign type (annotation,
/// nullary arm) is free; opening its structure is the owner's privilege.
fn foreign_destructures(program: &ast::Program, diags: &mut Vec<diag::Diagnostic>) {
    fn walk(p: &ast::Pattern, diags: &mut Vec<diag::Diagnostic>, span: diag::Span) {
        if let ast::Pattern::Ctor { ty, fields } = p {
            if ty.contains('/') && !fields.is_empty() {
                diags.push(diag::Diagnostic::new(
                    "opacity",
                    format!(
                        "`{ty}` is foreign — its structure does not cross an \
                         import; use its module's pub operations"
                    ),
                    span,
                ));
                return;
            }
            for f in fields {
                walk(f, diags, span);
            }
        }
    }
    for decl in &program.fns {
        for p in &decl.params {
            walk(p, diags, decl.span);
        }
        for stmt in &decl.body {
            if let ast::Stmt::Bind { pattern, expr } = stmt {
                walk(pattern, diags, *expr_span(expr));
            }
        }
    }
}

fn expr_span(e: &ast::Expr) -> &diag::Span {
    match e {
        ast::Expr::Ident(_, s)
        | ast::Expr::App { span: s, .. }
        | ast::Expr::Index { span: s, .. }
        | ast::Expr::BinOp { span: s, .. }
        | ast::Expr::Join { span: s, .. }
        | ast::Expr::Seq(_, _, s)
        | ast::Expr::Lambda { span: s, .. }
        | ast::Expr::List(_, s)
        | ast::Expr::MapLit(_, s)
        | ast::Expr::Str(_, s)
        | ast::Expr::Int(_, s)
        | ast::Expr::Float(_, s) => s,
        ast::Expr::Field { span: s, .. } => s,
    }
}

/// A qualified reference to a name its module did not mark pub.
fn private_uses(
    stmt: &ast::Stmt,
    exports: &std::collections::HashMap<String, bool>,
    diags: &mut Vec<diag::Diagnostic>,
) {
    fn walk(e: &ast::Expr, exports: &std::collections::HashMap<String, bool>, diags: &mut Vec<diag::Diagnostic>) {
        if let ast::Expr::Ident(name, span) = e {
            if let Some(false) = exports.get(name.as_str()) {
                let (module, base) = name.rsplit_once('/').unwrap_or(("", name));
                diags.push(diag::Diagnostic::new(
                    "opacity",
                    format!("`{base}` is private to module `{module}` — only pub names cross an import"),
                    *span,
                ));
            }
        }
        for child in expr_children(e) {
            walk(child, exports, diags);
        }
    }
    match stmt {
        ast::Stmt::Bind { expr, .. } => walk(expr, exports, diags),
        ast::Stmt::Expr(e) => walk(e, exports, diags),
    }
}

fn expr_children(e: &ast::Expr) -> Vec<&ast::Expr> {
    match e {
        ast::Expr::App { head, args, .. } => {
            let mut v: Vec<&ast::Expr> = vec![head.as_ref()];
            v.extend(args.iter());
            v
        }
        ast::Expr::Field { base, .. } => vec![base.as_ref()],
        ast::Expr::Index { base, index, .. } => vec![base.as_ref(), index.as_ref()],
        ast::Expr::BinOp { lhs, rhs, .. } | ast::Expr::Join { lhs, rhs, .. } => {
            vec![lhs.as_ref(), rhs.as_ref()]
        }
        ast::Expr::Seq(a, b, _) => vec![a.as_ref(), b.as_ref()],
        ast::Expr::Lambda { body, .. } => vec![body.as_ref()],
        ast::Expr::List(items, _) => items.iter().collect(),
        ast::Expr::MapLit(pairs, _) => pairs.iter().flat_map(|(k, v)| [k, v]).collect(),
        ast::Expr::Str(parts, _) => parts
            .iter()
            .filter_map(|p| match p {
                ast::TemplatePart::Interp(inner) => Some(inner),
                ast::TemplatePart::Lit(_) => None,
            })
            .collect(),
        ast::Expr::Int(..) | ast::Expr::Float(..) | ast::Expr::Ident(..) => Vec::new(),
    }
}

/// A module is a directory: every .kso file in it shares one namespace.
/// Canonical ordering holds per file; an overload group lives in one file.
pub fn compile_module(dir: &std::path::Path, require_main: bool) -> Result<ast::Program, String> {
    let mut visited = std::collections::HashSet::new();
    compile_module_root(dir, require_main, &mut visited)
}

/// The root module gets the ambient imports (design/render-plan.md);
/// dependencies never do — deps compile exactly as written.
fn compile_module_root(
    dir: &std::path::Path,
    require_main: bool,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<ast::Program, String> {
    AMBIENT_ROOT.with(|c| c.set(true));
    let result = compile_module_inner(dir, require_main, visited);
    AMBIENT_ROOT.with(|c| c.set(false));
    result
}

thread_local! {
    static AMBIENT_ROOT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn compile_module_inner(
    dir: &std::path::Path,
    require_main: bool,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<ast::Program, String> {
    let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canon) {
        return Err(format!(
            "error: import cycle through {}\n",
            dir.display()
        ));
    }
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
        let is_entry = path.file_name().is_some_and(|n| n == "main.kso");
        let mut program = match is_entry {
            true => parser::parse_entry(&lexed).map_err(|d| diag::render(&d, &file, &source))?,
            false => parser::parse(&lexed).map_err(|d| diag::render(&d, &file, &source))?,
        };
        stamp_file(&mut program, &file);
        parsed.push((file, source, program));
    }
    // the module's imports: the union across files, resolved and loaded
    // recursively, each dependency's names qualified by its short name
    let mut import_paths: Vec<String> = Vec::new();
    for (_, _, program) in &parsed {
        for import in &program.imports {
            if !import_paths.contains(&import.path) {
                import_paths.push(import.path.clone());
            }
        }
    }
    let root = AMBIENT_ROOT.with(|c| c.replace(false));
    if root && !dir.ends_with("render") {
        ambient_imports(&mut import_paths);
    }
    let (dep_program, exports) = load_dependencies(dir, &import_paths, visited)?;
    let mut all_names = std::collections::HashSet::new();
    let mut all_markers = std::collections::HashSet::new();
    let mut all_type_names = std::collections::HashSet::new();
    for (_, _, program) in &parsed {
        all_names.extend(check::declared_names(program));
        all_markers.extend(check::marker_names(program));
        all_type_names.extend(program.types.iter().map(|t| t.name.clone()));
    }
    all_names.extend(check::declared_names(&dep_program));
    all_markers.extend(check::marker_names(&dep_program));
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
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
    // pub bites at the boundary: a qualified reference to a non-pub name.
    // Imports are module-scoped, so use is counted across every file before
    // any one file's import block is called unused.
    let mut quals = std::collections::HashSet::new();
    for (_, _, program) in &parsed {
        used_quals(program, &mut quals);
    }
    for (file, source, program) in &parsed {
        let mut diags = Vec::new();
        for decl in &program.fns {
            for stmt in &decl.body {
                private_uses(stmt, &exports, &mut diags);
            }
        }
        diags.extend(unused_imports(&program.imports, &quals));
        foreign_destructures(program, &mut diags);
        if !diags.is_empty() {
            diags.sort_by_key(|d| (d.span.line, d.span.col));
            return Err(diag::render(&diags, file, source));
        }
    }
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new() };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
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
