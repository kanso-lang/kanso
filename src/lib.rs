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
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_list, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &exports, &mut quals);
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
    let shadowable: std::collections::HashSet<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file_shadow(&program, &extern_globals, &mut used, &shadowable));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new(), reexports: Vec::new() };
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
        true => {
            canonicalize_types(&mut merged);
    fuse_enumerable(&mut merged);
            Ok(merged)
        }
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
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_list, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &exports, &mut quals);
    diags.extend(unused_imports(&program.imports, &quals));
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    let extern_globals = check::declared_names(&dep_program);
    let shadowable: std::collections::HashSet<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file_shadow(&program, &extern_globals, &mut used, &shadowable));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    let merged_diags: Vec<_> = check::check_merged(&program, false)
        .into_iter()
        .filter(|d| d.kind != "unused")
        .collect();
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
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
        synthetic: false,
    });
    canonicalize_types(&mut program);
    fuse_enumerable(&mut program);
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
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = std::collections::HashSet::new();
    let (dep_program, exports) = load_dependencies(&base, &import_list, &mut visited)?;
    let mut diags = Vec::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &mut diags);
        }
    }
    let mut quals = std::collections::HashSet::new();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &exports, &mut quals);
    diags.extend(unused_imports(&program.imports, &quals));
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    let extern_globals = check::declared_names(&dep_program);
    let shadowable: std::collections::HashSet<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = std::collections::HashSet::new();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file_shadow(&program, &extern_globals, &mut used, &shadowable));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    let merged_diags: Vec<_> = check::check_merged(&program, false)
        .into_iter()
        .filter(|d| d.kind != "unused")
        .collect();
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    fuse_enumerable(&mut program);
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
/// Rewrite every type reference that resolves to an enrollment clone to
/// the canonical (origin) name: patterns and typeset members are type
/// positions, so no local binding can shadow them. Records then match by
/// one identity no matter which spelling constructed or destructured them.
pub fn canonicalize_types(program: &mut ast::Program) {
    let aliases: std::collections::HashMap<String, String> = program
        .types
        .iter()
        .filter_map(|t| t.origin.clone().map(|o| (t.name.clone(), o)))
        .collect();
    if aliases.is_empty() {
        return;
    }
    fn fix(name: &mut String, aliases: &std::collections::HashMap<String, String>) {
        if let Some(canon) = aliases.get(name.as_str()) {
            *name = canon.clone();
        }
    }
    fn walk_pattern(p: &mut ast::Pattern, aliases: &std::collections::HashMap<String, String>) {
        match p {
            ast::Pattern::Ctor { ty, fields } => {
                fix(ty, aliases);
                for f in fields {
                    walk_pattern(f, aliases);
                }
            }
            ast::Pattern::Annotated { ty, .. } => fix(ty, aliases),
            _ => {}
        }
    }
    for decl in &mut program.fns {
        for p in &mut decl.params {
            walk_pattern(p, &aliases);
        }
        for stmt in &mut decl.body {
            if let ast::Stmt::Bind { pattern, .. } = stmt {
                walk_pattern(pattern, &aliases);
            }
        }
    }
    for ty in &mut program.types {
        for (_, members, _) in &mut ty.fields {
            for member in members {
                fix(member, &aliases);
            }
        }
    }
}

/// Enumerable fusion: a consumer applied to an adapter chain rewrites to
/// one `fold` over the chain's root, the adapter steps composed into the
/// reducer. `fold`'s typed arms make the rewrite sound for any root — a
/// plain list takes the indexed loop, an iterator keeps the protocol — so
/// no per-element wrapper records exist for chains consumed in place.
pub fn fuse_enumerable(program: &mut ast::Program) {
    use ast::Stmt;
    if std::env::var_os("KANSO_NO_FUSE").is_some() {
        return;
    }
    let mut shorts: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let std_names: std::collections::HashSet<String> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list"))
        .map(|d| {
            d.name
                .rsplit_once('/')
                .map(|(_, s)| s.to_string())
                .unwrap_or_else(|| d.name.clone())
        })
        .collect();
    for d in &program.fns {
        let short = d.name.rsplit_once('/').map(|(_, s)| s).unwrap_or(&d.name);
        if std_names.contains(short) {
            shorts.insert(d.name.clone(), short.to_string());
        }
    }
    // the fold the rewrite names: a real decl in this program, whichever
    // qualified spelling the module graph produced
    let Some(fold_name) = program
        .fns
        .iter()
        .find(|d| {
            d.file.starts_with("std/list")
                && !d.synthetic
                && d.name.rsplit_once('/').map(|(_, s)| s).unwrap_or(&d.name) == "fold"
        })
        .map(|d| d.name.clone())
    else {
        return;
    };
    let mut counter = 0usize;
    for decl in &mut program.fns {
        if decl.file.starts_with("std/") {
            continue;
        }
        inline_single_use_chains(&mut decl.body, &shorts);
        for stmt in &mut decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    fuse_expr(expr, &shorts, &fold_name, &mut counter);
                }
            }
        }
    }
}

/// A width-forced split must not hide a chain: a binding whose value is an
/// adapter application and whose name is used exactly once — as the
/// collection argument of a later enumerable call — inlines back into the
/// chain before fusion looks. The binding was a rename, not an escape.
fn inline_single_use_chains(
    body: &mut Vec<ast::Stmt>,
    shorts: &std::collections::HashMap<String, String>,
) {
    use ast::{Expr, Stmt};
    const ADAPTERS: [&str; 5] = ["drop", "map", "reject", "select", "take"];
    let mut idx = 0;
    while idx < body.len() {
        let Stmt::Bind { pattern: ast::Pattern::Var(name, _), expr } = &body[idx] else {
            idx += 1;
            continue;
        };
        let Expr::App { head, .. } = expr else {
            idx += 1;
            continue;
        };
        let Expr::Ident(aname, _) = head.as_ref() else {
            idx += 1;
            continue;
        };
        let is_adapter = shorts
            .get(aname.as_str())
            .is_some_and(|s| ADAPTERS.contains(&s.as_str()));
        if !is_adapter {
            idx += 1;
            continue;
        }
        let name = name.clone();
        let mut uses = 0usize;
        for later in body.iter().skip(idx + 1) {
            match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    count_ident_uses(expr, &name, &mut uses);
                }
            }
        }
        let sole_coll_use = uses == 1
            && body.iter().skip(idx + 1).any(|later| match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    coll_arg_use(expr, &name, shorts)
                }
            });
        if !sole_coll_use {
            idx += 1;
            continue;
        }
        let Stmt::Bind { expr, .. } = body.remove(idx) else { unreachable!() };
        for later in body.iter_mut().skip(idx) {
            match later {
                Stmt::Bind { expr: e, .. } | Stmt::Expr(e) => {
                    substitute_ident(e, &name, &expr);
                }
            }
        }
    }
}

fn count_ident_uses(e: &ast::Expr, name: &str, uses: &mut usize) {
    if let ast::Expr::Ident(n, _) = e {
        if n == name {
            *uses += 1;
        }
    }
    for child in expr_children(e) {
        count_ident_uses(child, name, uses);
    }
}

/// Is the sole use of `name` the collection argument of an enumerable call?
fn coll_arg_use(
    e: &ast::Expr,
    name: &str,
    shorts: &std::collections::HashMap<String, String>,
) -> bool {
    if let ast::Expr::App { head, args, .. } = e {
        if let ast::Expr::Ident(h, _) = head.as_ref() {
            if shorts.contains_key(h.as_str()) {
                if let Some(ast::Expr::Ident(first, _)) = args.first() {
                    if first == name {
                        return true;
                    }
                }
            }
        }
    }
    expr_children(e).into_iter().any(|c| coll_arg_use(c, name, shorts))
}

fn substitute_ident(e: &mut ast::Expr, name: &str, replacement: &ast::Expr) {
    use ast::Expr;
    if let Expr::Ident(n, _) = e {
        if n == name {
            *e = replacement.clone();
            return;
        }
    }
    match e {
        Expr::App { head, args, .. } => {
            substitute_ident(head, name, replacement);
            for a in args {
                substitute_ident(a, name, replacement);
            }
        }
        Expr::Lambda { body, .. } => substitute_ident(body, name, replacement),
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                        substitute_ident(expr, name, replacement)
                    }
                }
            }
        }
        Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
            substitute_ident(a, name, replacement);
            substitute_ident(b, name, replacement);
        }
        Expr::List(items, _) => {
            for i in items {
                substitute_ident(i, name, replacement);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                substitute_ident(k, name, replacement);
                substitute_ident(v, name, replacement);
            }
        }
        Expr::Index { base, index, .. } => {
            substitute_ident(base, name, replacement);
            substitute_ident(index, name, replacement);
        }
        Expr::Field { base, .. } => substitute_ident(base, name, replacement),
        Expr::Upcast { expr, .. } => substitute_ident(expr, name, replacement),
        Expr::BinOp { lhs, rhs, .. } => {
            substitute_ident(lhs, name, replacement);
            substitute_ident(rhs, name, replacement);
        }
        Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    substitute_ident(inner, name, replacement);
                }
            }
        }
        _ => {}
    }
}

fn fuse_expr(
    e: &mut ast::Expr,
    shorts: &std::collections::HashMap<String, String>,
    fold_name: &str,
    counter: &mut usize,
) {
    use ast::Expr;
    match e {
        Expr::App { head, args, .. } => {
            fuse_expr(head, shorts, fold_name, counter);
            for a in args.iter_mut() {
                fuse_expr(a, shorts, fold_name, counter);
            }
        }
        Expr::Lambda { body, .. } => fuse_expr(body, shorts, fold_name, counter),
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                        fuse_expr(expr, shorts, fold_name, counter)
                    }
                }
            }
        }
        Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
            fuse_expr(a, shorts, fold_name, counter);
            fuse_expr(b, shorts, fold_name, counter);
        }
        Expr::List(items, _) => {
            for i in items {
                fuse_expr(i, shorts, fold_name, counter);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                fuse_expr(k, shorts, fold_name, counter);
                fuse_expr(v, shorts, fold_name, counter);
            }
        }
        Expr::Index { base, index, .. } => {
            fuse_expr(base, shorts, fold_name, counter);
            fuse_expr(index, shorts, fold_name, counter);
        }
        Expr::Field { base, .. } => fuse_expr(base, shorts, fold_name, counter),
        Expr::Upcast { expr, .. } => fuse_expr(expr, shorts, fold_name, counter),
        Expr::BinOp { lhs, rhs, .. } => {
            fuse_expr(lhs, shorts, fold_name, counter);
            fuse_expr(rhs, shorts, fold_name, counter);
        }
        Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    fuse_expr(inner, shorts, fold_name, counter);
                }
            }
        }
        _ => {}
    }
    if let Some(rewritten) = try_fuse(e, shorts, fold_name, counter) {
        *e = rewritten;
    }
}

fn try_fuse(
    e: &ast::Expr,
    shorts: &std::collections::HashMap<String, String>,
    fold_name: &str,
    counter: &mut usize,
) -> Option<ast::Expr> {
    use ast::Expr;
    let Expr::App { head, args, span, piped: false } = e else { return None };
    let Expr::Ident(cname, _) = head.as_ref() else { return None };
    let consumer = shorts.get(cname.as_str())?.clone();
    let span = *span;
    let lam = |params: Vec<&String>, body: Expr| Expr::Lambda {
        params: params.iter().map(|p| ((*p).clone(), span)).collect(),
        body: Box::new(body),
        span,
    };
    let ident = |n: &str| Expr::Ident(n.to_string(), span);
    let call = |h: Expr, a: Vec<Expr>| Expr::App {
        head: Box::new(h),
        args: a,
        span,
        piped: false,
    };
    *counter += 1;
    let acc = format!("facc{counter}");
    let x = format!("felem{counter}");
    let (init, reducer) = match (consumer.as_str(), args.len()) {
        ("fold", 3) => (args[1].clone(), args[2].clone()),
        ("sum", 1) => (
            Expr::Int(0u32.into(), span),
            lam(
                vec![&acc, &x],
                Expr::BinOp {
                    op: "+",
                    lhs: Box::new(ident(&acc)),
                    rhs: Box::new(ident(&x)),
                    span,
                },
            ),
        ),
        ("to_list", 1) => (
            Expr::List(Vec::new(), span),
            lam(vec![&acc, &x], call(ident("push"), vec![ident(&acc), ident(&x)])),
        ),
        ("count", 2) => (
            Expr::Int(0u32.into(), span),
            lam(
                vec![&acc, &x],
                call(
                    ident("if"),
                    vec![
                        call(args[1].clone(), vec![ident(&x)]),
                        Expr::BinOp {
                            op: "+",
                            lhs: Box::new(ident(&acc)),
                            rhs: Box::new(Expr::Int(1u32.into(), span)),
                            span,
                        },
                        ident(&acc),
                    ],
                ),
            ),
        ),
        _ => return None,
    };
    let mut source = args[0].clone();
    let mut reducer = reducer;
    let mut fused_any = false;
    #[allow(clippy::while_let_loop)]
    loop {
        let Expr::App { head: ahead, args: aargs, piped: false, .. } = &source else { break };
        let Expr::Ident(aname, _) = ahead.as_ref() else { break };
        let Some(adapter) = shorts.get(aname.as_str()).cloned() else { break };
        if aargs.len() != 2 {
            break;
        }
        *counter += 1;
        let a2 = format!("facc{counter}");
        let x2 = format!("felem{counter}");
        let step = aargs[1].clone();
        reducer = match adapter.as_str() {
            "map" => lam(
                vec![&a2, &x2],
                call(reducer.clone(), vec![ident(&a2), call(step, vec![ident(&x2)])]),
            ),
            "select" => lam(
                vec![&a2, &x2],
                call(
                    ident("if"),
                    vec![
                        call(step, vec![ident(&x2)]),
                        call(reducer.clone(), vec![ident(&a2), ident(&x2)]),
                        ident(&a2),
                    ],
                ),
            ),
            "reject" => lam(
                vec![&a2, &x2],
                call(
                    ident("if"),
                    vec![
                        call(step, vec![ident(&x2)]),
                        ident(&a2),
                        call(reducer.clone(), vec![ident(&a2), ident(&x2)]),
                    ],
                ),
            ),
            _ => break,
        };
        let Expr::App { args: aargs, .. } = source else { unreachable!() };
        source = aargs.into_iter().next().expect("adapters carry a source");
        fused_any = true;
    }
    if !fused_any {
        return None;
    }
    Some(call(ident(fold_name), vec![source, init, reducer]))
}

fn qualify(dep: &mut ast::Program, qual: &str, exports: &mut std::collections::HashMap<String, bool>) {
    let owned: std::collections::HashSet<String> = check::declared_names(dep);
    for ty in &mut dep.types {
        exports.insert(format!("{qual}/{}", ty.name), ty.is_pub);
        ty.name = format!("{qual}/{}", ty.name);
        if let Some(o) = &mut ty.origin {
            *o = format!("{qual}/{o}");
        }
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
        ast::Expr::Block(stmts, _) => {
            for stmt in stmts {
                rewrite_stmt(stmt, qual, owned);
            }
        }
        ast::Expr::Ident(name, _) => {
            if owned.contains(name.as_str()) {
                *name = format!("{qual}/{name}");
            }
        }
        ast::Expr::Field { base, .. } => rewrite_expr(base, qual, owned),
        ast::Expr::Upcast { expr, .. } => rewrite_expr(expr, qual, owned),
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

/// The bare overload space (the import-incarnation gavel): every pub name
/// of every imported module also exists under its short name, so bare
/// calls dispatch over the union of local and imported arms — overloading
/// is the resolution mechanism. The clones are real declarations, which is
/// what lets every downstream consumer (check, both engines, inference,
/// specificity) work unchanged.
fn enroll_bare(
    dep_program: &mut ast::Program,
    exports: &std::collections::HashMap<String, bool>,
    renamed: &std::collections::HashSet<String>,
) {
    let mut bare_fns = Vec::new();
    for f in &dep_program.fns {
        if exports.get(&f.name).copied().unwrap_or(false) && !renamed.contains(&f.name) {
            if let Some((_, short)) = f.name.rsplit_once('/') {
                let mut clone = f.clone();
                clone.name = short.to_string();
                clone.synthetic = true;
                bare_fns.push(clone);
            }
        }
    }
    dep_program.fns.extend(bare_fns);
    let mut bare_types = Vec::new();
    for t in &dep_program.types {
        if exports.get(&t.name).copied().unwrap_or(false) && !renamed.contains(&t.name) {
            if let Some((_, short)) = t.name.rsplit_once('/') {
                let mut clone = t.clone();
                clone.name = short.to_string();
                clone.synthetic = true;
                clone.origin = Some(t.origin.clone().unwrap_or_else(|| t.name.clone()));
                bare_types.push(clone);
            }
        }
    }
    dep_program.types.extend(bare_types);
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

fn ambient_imports(imports: &mut Vec<ast::Import>) {
    if !imports.iter().any(|i| i.path == "std/render") {
        imports.push(ast::Import {
            path: "std/render".to_string(),
            span: diag::Span { line: 0, col: 0 },
            alias: None,
            renames: Vec::new(),
        });
    }
}

/// Load and qualify every imported module, recursively.
fn load_dependencies(
    base: &std::path::Path,
    imports: &[ast::Import],
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(ast::Program, std::collections::HashMap<String, bool>), String> {
    let mut dep_program = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new(), reexports: Vec::new() };
    let mut exports = std::collections::HashMap::new();
    for import in imports {
        let path = &import.path;
        let qual_owned;
        let qual = match &import.alias {
            Some(alias) => alias.as_str(),
            None => {
                qual_owned = short_name(path).to_string();
                &qual_owned
            }
        };
        // Embedded std modules load where no filesystem exists (the browser)
        // and where no lib/ ships beside the binary (installs). include_str!
        // of the same files keeps the embedded copies incapable of drifting.
        let embedded = match path.as_str() {
            "std/render" => Some(("render", include_str!("../lib/render/render.kso"))),
            "std/list" => Some(("list", include_str!("../lib/list/list.kso"))),
            "std/time" => Some(("time", include_str!("../lib/time/time.kso"))),

            "std/io" => Some(("io", include_str!("../lib/io/io.kso"))),
            "std/text" => Some(("text", include_str!("../lib/text/text.kso"))),
            "std/math" => Some(("math", include_str!("../lib/math/math.kso"))),
            _ => None,
        };
        if let Some((short, source)) = embedded {
            let mut dep = compile(&format!("{path}/{short}.kso"), source, false)?;
            qualify(&mut dep, qual, &mut exports);
            dep_program.types.extend(dep.types);
            dep_program.fns.extend(dep.fns);
            continue;
        }
        if path == "std/random" {
            return Err(
                "error: `std/random` moved — `random` lives in `std/math`
".to_string()
            );
        }
        let dep_dir = resolve_import(base, path)?;
        let mut dep = compile_module_inner(&dep_dir, false, visited)?;
        qualify(&mut dep, qual, &mut exports);
        dep_program.types.extend(dep.types);
        dep_program.fns.extend(dep.fns);
    }
    // a rename replaces that token's spellings: bare `yours` and
    // `qual/yours` enroll, bare `theirs` never does — the qualified
    // original stays, because the qualified spelling is permanent identity
    let renamed: std::collections::HashSet<String> = imports
        .iter()
        .flat_map(|import| {
            let qual =
                import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
            import
                .renames
                .iter()
                .map(move |(theirs, _)| format!("{qual}/{theirs}"))
                .collect::<Vec<_>>()
        })
        .collect();
    enroll_bare(&mut dep_program, &exports, &renamed);
    for import in imports {
        let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
        for (theirs, yours) in &import.renames {
            let qualified = format!("{qual}/{theirs}");
            let mut found = false;
            let mut clones = Vec::new();
            for f in &dep_program.fns {
                if f.name == qualified {
                    for spelling in [yours.clone(), format!("{qual}/{yours}")] {
                        let mut c = f.clone();
                        c.name = spelling;
                        c.synthetic = true;
                        clones.push(c);
                    }
                    found = true;
                }
            }
            dep_program.fns.extend(clones);
            let mut tclones = Vec::new();
            for t in &dep_program.types {
                if t.name == qualified {
                    for spelling in [yours.clone(), format!("{qual}/{yours}")] {
                        let mut c = t.clone();
                        c.name = spelling;
                        c.synthetic = true;
                        c.origin = Some(t.origin.clone().unwrap_or_else(|| t.name.clone()));
                        tclones.push(c);
                    }
                    found = true;
                }
            }
            dep_program.types.extend(tclones);
            if !found {
                return Err(format!(
                    "error: `{}` exports no `{theirs}` to rename\n",
                    import.path
                ));
            }
        }
    }
    Ok((dep_program, exports))
}

/// Bare uses count too: a bare `select` that any import exports marks that
/// import used — the bare overload space makes spelling optional, not the
/// dependency.
fn mark_bare_quals(
    program: &ast::Program,
    exports: &std::collections::HashMap<String, bool>,
    quals: &mut std::collections::HashSet<String>,
) {
    let mut bare = std::collections::HashSet::new();
    fn collect(e: &ast::Expr, bare: &mut std::collections::HashSet<String>) {
        if let ast::Expr::Ident(name, _) = e {
            if !name.contains('/') {
                bare.insert(name.clone());
            }
        }
        for child in expr_children(e) {
            collect(child, bare);
        }
    }
    for decl in &program.fns {
        for stmt in &decl.body {
            match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => collect(expr, &mut bare),
            }
        }
    }
    for (qualified, is_pub) in exports {
        if !is_pub {
            continue;
        }
        // a re-export surfaces as a nested qual (geo/list/select): the
        // import that owns it is the first segment, the bare spelling the last
        if let Some((first, _)) = qualified.split_once('/') {
            if let Some((_, short)) = qualified.rsplit_once('/') {
                if bare.contains(short) {
                    quals.insert(first.to_string());
                }
            }
        }
    }
    for import in &program.imports {
        let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
        if import.renames.iter().any(|(_, yours)| bare.contains(yours)) {
            quals.insert(qual);
        }
    }
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
        .filter(|i| {
            let qual = i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string());
            !quals.contains(&qual)
        })
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
        | ast::Expr::Block(_, s)
        | ast::Expr::Seq(_, _, s)
        | ast::Expr::Lambda { span: s, .. }
        | ast::Expr::List(_, s)
        | ast::Expr::MapLit(_, s)
        | ast::Expr::Str(_, s)
        | ast::Expr::Int(_, s)
        | ast::Expr::Float(_, s) => s,
        ast::Expr::Field { span: s, .. } => s,
        ast::Expr::Upcast { span: s, .. } => s,
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
        ast::Expr::Upcast { expr, .. } => vec![expr.as_ref()],
        ast::Expr::Block(stmts, _) => stmts
            .iter()
            .map(|st| match st {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr,
            })
            .collect(),
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

/// One `pub name` line: elevate the matching demoted dependency pubs back
/// onto this module's surface. A qualifier elevates its module's whole
/// surface; a lone name elevates that export wherever imports offer it;
/// `theirs:yours` clones under the new name instead.
fn apply_reexport(
    dep_program: &mut ast::Program,
    was_pub: &std::collections::HashSet<String>,
    import_quals: &[String],
    re: &ast::Reexport,
) -> Result<(), diag::Diagnostic> {
    if import_quals.iter().any(|q| q == &re.name) {
        if re.rename.is_some() {
            return Err(diag::Diagnostic::new(
                "syntax",
                "a whole module re-exports by its own name; rename exports one at a time"
                    .to_string(),
                re.span,
            ));
        }
        let prefix = format!("{}/", re.name);
        let mut any = false;
        for f in &mut dep_program.fns {
            if f.name.starts_with(&prefix) && was_pub.contains(&f.name) {
                f.is_pub = true;
                any = true;
            }
        }
        for t in &mut dep_program.types {
            if t.name.starts_with(&prefix) && was_pub.contains(&t.name) {
                t.is_pub = true;
                any = true;
            }
        }
        if !any {
            return Err(diag::Diagnostic::new(
                "name",
                format!("`{}` exports nothing to re-export", re.name),
                re.span,
            ));
        }
        return Ok(());
    }
    let mut any = false;
    for q in import_quals {
        let qualified = format!("{q}/{}", re.name);
        if !was_pub.contains(&qualified) {
            continue;
        }
        match &re.rename {
            None => {
                for f in &mut dep_program.fns {
                    if f.name == qualified {
                        f.is_pub = true;
                        any = true;
                    }
                }
                for t in &mut dep_program.types {
                    if t.name == qualified {
                        t.is_pub = true;
                        any = true;
                    }
                }
            }
            Some(yours) => {
                let mut fclones = Vec::new();
                for f in &dep_program.fns {
                    if f.name == qualified {
                        let mut c = f.clone();
                        c.name = format!("{q}/{yours}");
                        c.synthetic = true;
                        c.is_pub = true;
                        fclones.push(c);
                        any = true;
                    }
                }
                dep_program.fns.extend(fclones);
                let mut tclones = Vec::new();
                for t in &dep_program.types {
                    if t.name == qualified {
                        let mut c = t.clone();
                        c.name = format!("{q}/{yours}");
                        c.synthetic = true;
                        c.is_pub = true;
                        c.origin = Some(t.origin.clone().unwrap_or_else(|| t.name.clone()));
                        tclones.push(c);
                        any = true;
                    }
                }
                dep_program.types.extend(tclones);
            }
        }
    }
    if !any {
        return Err(diag::Diagnostic::new(
            "name",
            format!("no import offers a pub `{}` to re-export", re.name),
            re.span,
        ));
    }
    Ok(())
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
    let mut import_list: Vec<ast::Import> = Vec::new();
    for (_, _, program) in &parsed {
        for import in &program.imports {
            if !import_list.iter().any(|i| i.path == import.path) {
                import_list.push(import.clone());
            }
        }
    }
    let root = AMBIENT_ROOT.with(|c| c.replace(false));
    if root && !dir.ends_with("render") {
        ambient_imports(&mut import_list);
    }
    let (mut dep_program, exports) = load_dependencies(dir, &import_list, visited)?;
    // A module's surface is its own. Dependency pubs demote at this
    // boundary — importers of this module see none of them — and only an
    // explicit re-export puts an imported name back on the surface, as a
    // pub the importer then enrolls like any other.
    let was_pub: std::collections::HashSet<String> = dep_program
        .fns
        .iter()
        .filter(|f| f.is_pub)
        .map(|f| f.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.is_pub).map(|t| t.name.clone()))
        .collect();
    for f in &mut dep_program.fns {
        f.is_pub = false;
    }
    for t in &mut dep_program.types {
        t.is_pub = false;
    }
    let import_quals: Vec<String> = import_list
        .iter()
        .map(|i| i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string()))
        .collect();
    for (file, source, program) in &parsed {
        for re in &program.reexports {
            apply_reexport(&mut dep_program, &was_pub, &import_quals, re)
                .map_err(|d| diag::render(&[d], file, source))?;
        }
    }
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
    let shadowable: std::collections::HashSet<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut used = std::collections::HashSet::new();
    for (file, source, program) in &mut parsed {
        let mut extern_globals = all_names.clone();
        for name in check::declared_names(program) {
            extern_globals.remove(&name);
        }
        let mut diags = check::resolve_markers(program, &all_markers);
        diags.extend(check::check_typesets(program, &all_type_names));
        diags.extend(check::check_file_shadow(program, &extern_globals, &mut used, &shadowable));
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        if !diags.is_empty() {
            return Err(diag::render(&diags, file, source));
        }
    }
    // pub bites at the boundary: a qualified reference to a non-pub name.
    // Imports are module-scoped, so use is counted across every file before
    // any one file's import block is called unused. Bare spellings count
    // too — enrollment makes the qualifier optional, not the dependency.
    let mut quals = std::collections::HashSet::new();
    for (_, _, program) in &parsed {
        used_quals(program, &mut quals);
        mark_bare_quals(program, &exports, &mut quals);
        for re in &program.reexports {
            match import_quals.iter().find(|q| *q == &re.name) {
                Some(q) => {
                    quals.insert(q.clone());
                }
                None => {
                    for q in &import_quals {
                        if was_pub.contains(&format!("{q}/{}", re.name)) {
                            quals.insert(q.clone());
                        }
                    }
                }
            }
        }
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
    let mut merged = ast::Program { fns: Vec::new(), types: Vec::new(), imports: Vec::new(), reexports: Vec::new() };
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
    canonicalize_types(&mut merged);
    fuse_enumerable(&mut merged);
    Ok(merged)
}
