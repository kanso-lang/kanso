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
pub mod hako;
pub mod infer;
pub mod inline;
pub mod lexer;
pub mod linear;
pub mod parser;
pub mod provenance;
pub mod repl;
pub mod trmc;
pub mod wasm;
pub mod wasm_backend;
pub mod wasm_encode;
pub mod wasm_rt;

pub fn compile(file: &str, source: &str, require_entry: bool) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    synthesize_getters(&mut program);
    stamp_file(&mut program, file);
    let diags = check::check(&mut program, require_entry);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    inline::inline_builtin_wrappers(&mut program);
    match diags.is_empty() {
        true => Ok(program),
        false => Err(diag::render(&diags, file, source)),
    }
}

/// Compile a single file as an entry: its statements are the program.
pub fn compile_entry(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse_entry(&lexed).map_err(|d| diag::render(&d, file, source))?;
    synthesize_getters(&mut program);
    stamp_file(&mut program, file);
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
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
    let mut merged = ast::Program {
        fns: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
    };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    merged.types.extend(program.types);
    merged.fns.extend(program.fns);
    let mut merged_diags = check::check_merged(&merged, true);
    check::check_unused_private(&merged, &used, &mut merged_diags);
    synthesize_getters(&mut merged);
    desugar_field_reads(&mut merged);
    prune_unused_getters(&mut merged);
    trmc::rewrite(&mut merged);
    inline::inline_builtin_wrappers(&mut merged);
    let merged_diags: Vec<_> = merged_diags.into_iter().filter(|d| d.kind != "unused").collect();
    match merged_diags.is_empty() {
        true => {
            canonicalize_types(&mut merged);
            canonicalize_bare_aliases(&mut merged);
            hoist_repeated_strings(&mut merged);
            fuse_enumerable(&mut merged);
            synthesize_getters(&mut merged);
            desugar_field_reads(&mut merged);
            prune_unused_getters(&mut merged);
            trmc::rewrite(&mut merged);
            Ok(merged)
        }
        false => Err(diag::render(&merged_diags, file, source)),
    }
}

/// `kanso play`: the playground's convention at the terminal. The file is a
/// library defining `pub play`; the synthesized entry runs it.
/// A `play` file: one source with its imports, and a `pub play` to run.
pub fn compile_play(file: &str, source: &str) -> Result<ast::Program, String> {
    compile_one(file, source, true, false)
}

/// The repl's session: the same thing without an entry point, so a prompt can
/// import a module and reach it. An unused binding at a prompt is exploration
/// rather than a mistake, so those are dropped here and nowhere else.
pub fn compile_repl(file: &str, source: &str) -> Result<ast::Program, String> {
    compile_one(file, source, false, true)
}

fn compile_one(
    file: &str,
    source: &str,
    require_play: bool,
    drop_unused: bool,
) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    synthesize_getters(&mut program);
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
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
    if drop_unused {
        // An import at a prompt is used on the next line, not this one.
        diags.retain(|d| d.kind != "unused");
    }
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
    if drop_unused {
        diags.retain(|d| d.kind != "unused");
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    let merged_diags: Vec<_> =
        check::check_merged(&program, false).into_iter().filter(|d| d.kind != "unused").collect();
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    let has_play = program.fns.iter().any(|d| d.name == "play" && d.is_pub);
    if require_play && !has_play {
        return Err(format!(
            "error: nothing to play — define `pub play`, or point `kanso run` \
             at an entry file\n  --> {file}\n"
        ));
    }
    let span = diag::Span { line: 1, col: 1 };
    // A session has no entry point until somebody writes one, and a `main`
    // calling a `play` that is not there would name nothing.
    if has_play {
        program.fns.push(ast::FnDecl {
            name: ast::ENTRY.to_string(),
            params: Vec::new(),
            body: vec![ast::Stmt::Expr(ast::Expr::Ident("play".to_string(), span))],
            span,
            is_pub: false,
            file: file.to_string(),
            synthetic: false,
        });
    }
    canonicalize_types(&mut program);
    canonicalize_bare_aliases(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    synthesize_getters(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    Ok(program)
}

/// A lone library file under a library verb (`kanso test`/`check`): parses
/// as a library and loads its imports (plus the ambient module) like any
/// other root compile.
pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    synthesize_getters(&mut program);
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
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
    let merged_diags: Vec<_> =
        check::check_merged(&program, false).into_iter().filter(|d| d.kind != "unused").collect();
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    canonicalize_bare_aliases(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    synthesize_getters(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    Ok(program)
}

/// Route a single source file to the right compile for a verb, by content:
/// `pub play` is a play library, bare statements are an entry, definitions
/// alone are a library (runnable only under a library verb like `test`).
/// Does this file declare `pub play`? Answered off the parse, so comments and
/// string literals cannot change how a file compiles. A file that does not
/// parse declares nothing, and the compile it is routed to reports the syntax
/// error properly.
/// Whether the file declares `pub play`, or None when it does not parse at
/// all. A source with a syntax error has no shape yet, and calling it a
/// library hides the error behind a sentence about libraries — the reader is
/// told to add an entry point they already wrote.
fn declares_play(source: &str) -> Option<bool> {
    let lexed = lexer::lex(source).ok()?;
    let program = parser::parse(&lexed).ok()?;
    Some(program.fns.iter().any(|d| d.is_pub && d.name == "play"))
}

/// The CLI and the browser share this so the engines never diverge on which
/// compile a file gets.
pub fn compile_source(command: &str, file: &str, source: &str) -> Result<ast::Program, String> {
    // Which compile a file gets is decided by what it declares, not by what
    // its text happens to contain: a comment or a string mentioning `pub
    // play` used to reroute the whole file and reject a valid entry with a
    // diagnostic that named none of this.
    let declared = declares_play(source);
    let has_defs = source
        .lines()
        .any(|l| l.starts_with("fn ") || l.starts_with("type ") || l.starts_with("pub "));
    // A file of bare statements never parses as a library, and that is not an
    // error — it is an entry. But a file that declares things and still does
    // not parse has a syntax error, and answering it with the shape of the
    // file tells the reader to add an entry point they already wrote.
    if declared.is_none() && has_defs {
        return compile_play(file, source);
    }
    let has_play = declared.unwrap_or(false);
    let library_verb = command == "test";
    match (command, has_play, has_defs) {
        (_, true, _) if !library_verb => compile_play(file, source),
        ("check", false, true) => compile_library(file, source),
        (_, false, true) if !library_verb => Err(format!(
            "error: `{file}` is a library — nothing to run. give the \
             module a main.kso entry, or define a `pub play` for `run` to \
             find\n"
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

/// A field is read by applying its name, so every field declares an arm:
/// `pub fn name (user _ name) = name`. The accessor is then a value, which
/// is what field syntax could never hand to anything else.
///
/// An enrollment clone re-declares a type it does not own, so it is skipped
/// — the getter belongs to the module that declared the fields, and a
/// second identical arm would collide with the first.
/// Runs immediately before every `desugar_field_reads`, which is the point a
/// read becomes a call and so the point the getter has to exist. It has to run
/// more than once because a module that exposes a record but never reads its
/// own fields prunes every getter it should have handed to its importers, and
/// the importer is a later program. Declaring one twice is harmless by
/// construction: a field already carrying a getter is left alone.
fn synthesize_getters(program: &mut ast::Program) {
    // Keyed by the type as well as the field. One getter group holds an arm
    // per type that has the field, and skipping on the name alone would let
    // the first type through and leave every later one unreadable.
    let already: std::collections::HashSet<(String, String)> = program
        .fns
        .iter()
        .filter(|f| f.is_getter())
        .filter_map(|f| match f.params.first() {
            Some(ast::Pattern::Ctor { ty, .. }) => Some((f.name.clone(), ty.clone())),
            _ => None,
        })
        .collect();
    // Only the fields something reads. Every arm this skips would be deleted
    // by prune_unused_getters a moment later, and there are a great many of
    // them: an imported module brings its types, and each type brings a field
    // per getter nobody asked for.
    let mut read = std::collections::HashSet::new();
    for decl in &program.fns {
        for stmt in &decl.body {
            mentions_in_stmt(stmt, &mut read);
        }
    }
    let mut arms = Vec::new();
    for ty in &program.types {
        // An imported type needs arms here too. The dependency synthesised its
        // own and then pruned the ones it did not itself read, so a field only
        // this program reads arrives with no getter at all — and while nothing
        // else declares that field the read stays a direct one and works, so
        // the hole opens exactly when a local type names the same field.
        if ty.synthetic {
            continue;
        }
        for (index, (field, _, span)) in ty.fields.iter().enumerate() {
            let bound = ty
                .fields
                .iter()
                .enumerate()
                .map(|(at, _)| match at == index {
                    true => ast::Pattern::Var(ast::GETTER_BINDER.to_string(), *span),
                    false => ast::Pattern::Wildcard(*span),
                })
                .collect();
            if already.contains(&(ast::getter_name(field), ty.name.clone()))
                || !read.contains(&ast::getter_name(field))
            {
                continue;
            }
            arms.push(ast::FnDecl {
                name: ast::getter_name(field),
                is_pub: true,
                span: *span,
                params: vec![ast::Pattern::Ctor { ty: ty.name.clone(), fields: bound }],
                body: vec![ast::Stmt::Expr(ast::Expr::Ident(
                    ast::GETTER_BINDER.to_string(),
                    *span,
                ))],
                file: String::new(),
                synthetic: false,
            });
        }
    }
    program.fns.extend(arms);
}

/// Resolve one import path to a directory, per the gaveled table. Each shape
/// answers in exactly one way, and a shape never falls through to another —
/// that is what makes an import's universe readable in its spelling. A bare
/// multi-segment path is a hako name and is never tried as a local directory,
/// so a subtree that happens to be called `owner/repo` cannot shadow one.
fn resolve_import(base: &std::path::Path, path: &str) -> Result<std::path::PathBuf, String> {
    if let Some(rest) = path.strip_prefix("std/") {
        let toolchain =
            std::env::var("KANSO_STD").map(std::path::PathBuf::from).unwrap_or_else(|_| {
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
    if path.starts_with("./") || path.starts_with("../") {
        let relative = base.join(path);
        if relative.is_dir() {
            return Ok(relative);
        }
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a dot-prefixed path names \
             a directory beside the importing module, and there is none there\n"
        ));
    }
    if !path.contains('/') {
        let sibling = base.join(path);
        if sibling.is_dir() {
            return Ok(sibling);
        }
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a bare name is a sibling \
             subdirectory module, and this module has no `{path}` directory\n"
        ));
    }
    let first = path.split('/').next().unwrap_or_default();
    if first.contains('.') {
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a dot in the first segment \
             names a hako by domain, which no source resolves yet\n"
        ));
    }
    // the lock decides which fetched tree a name means; without one, a bare
    // name in the cache still answers, so a checkout can be dropped in by hand
    let cache = hako_cache();
    if let Some((name, module)) = hako::split_name(path) {
        if let Some(pin) = LOCK.with(|l| l.borrow().get(&name).cloned()) {
            let mut dir = hako::cached(&cache, &name, &pin.sha);
            if let Some(rest) = module {
                dir = dir.join(rest);
            }
            if dir.is_dir() {
                return Ok(dir);
            }
        }
    }
    let plain = cache.join(path);
    if plain.is_dir() {
        return Ok(plain);
    }
    Err(format!(
        "error: cannot resolve import \"{path}\" — a bare multi-segment path names \
         a hako, and it is not in the cache (run `kanso install`)\n"
    ))
}

/// Where fetched hakos live. KANSO_HAKO moves it, which is how a test gets a
/// cache of its own rather than reaching into the developer's.
fn hako_cache() -> std::path::PathBuf {
    std::env::var("KANSO_HAKO")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".hako")))
        .unwrap_or_default()
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
fn bound_in_pattern(p: &ast::Pattern, out: &mut std::collections::HashSet<String>) {
    match p {
        ast::Pattern::Var(n, _) => {
            out.insert(n.clone());
        }
        ast::Pattern::Annotated { name, .. } => {
            out.insert(name.clone());
        }
        ast::Pattern::Ctor { fields, .. } => {
            for f in fields {
                bound_in_pattern(f, out);
            }
        }
        ast::Pattern::Keyed { entries, .. } => {
            for e in entries {
                out.insert(e.bind_name.clone());
            }
        }
        ast::Pattern::IntLit(..)
        | ast::Pattern::StrLit(..)
        | ast::Pattern::Nullary(..)
        | ast::Pattern::Wildcard(..) => {}
    }
}

fn bound_in_stmt(stmt: &ast::Stmt, out: &mut std::collections::HashSet<String>) {
    match stmt {
        ast::Stmt::Bind { pattern, expr } => {
            bound_in_pattern(pattern, out);
            bound_in_expr(expr, out);
        }
        ast::Stmt::Expr(e) | ast::Stmt::Set { value: e, .. } => bound_in_expr(e, out),
    }
}

fn bound_in_expr(e: &ast::Expr, out: &mut std::collections::HashSet<String>) {
    match e {
        ast::Expr::Lambda { params, body, .. } => {
            for (n, _) in params {
                out.insert(n.clone());
            }
            bound_in_expr(body, out);
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            for st in stmts {
                bound_in_stmt(st, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            bound_in_expr(cond, out);
            bound_in_expr(early, out);
            for st in rest {
                bound_in_stmt(st, out);
            }
        }
        ast::Expr::App { head, args, .. } => {
            bound_in_expr(head, out);
            for a in args {
                bound_in_expr(a, out);
            }
        }
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                bound_in_expr(k, out);
                bound_in_expr(v, out);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    bound_in_expr(inner, out);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for i in items {
                bound_in_expr(i, out);
            }
        }
        ast::Expr::Field { base, .. } | ast::Expr::Upcast { expr: base, .. } => {
            bound_in_expr(base, out)
        }
        ast::Expr::Index { base, index, .. } => {
            bound_in_expr(base, out);
            bound_in_expr(index, out);
        }
        ast::Expr::Seq(a, b, _) | ast::Expr::Join { lhs: a, rhs: b, .. } => {
            bound_in_expr(a, out);
            bound_in_expr(b, out);
        }
        ast::Expr::BinOp { lhs, rhs, .. } => {
            bound_in_expr(lhs, out);
            bound_in_expr(rhs, out);
        }
        ast::Expr::Int(..)
        | ast::Expr::Float(..)
        | ast::Expr::Ident(..)
        | ast::Expr::Partial(..) => {}
    }
}

/// A bare-enrollment clone whose name has no other arms is one function with
/// two spellings, and the analyses that reason about self-recursion see two.
/// Where the whole bare group is clones of a single qualified origin, every
/// bare reference rewrites to the qualified spelling and the clones go: one
/// group, one emission, and a module's internal recursion reads as
/// self-recursion again. A bare name that also has local arms is a real
/// overload union (the import-incarnation gavel) and is left alone.
pub fn canonicalize_bare_aliases(program: &mut ast::Program) {
    use std::collections::{HashMap, HashSet};
    let mut by_name: HashMap<&str, (bool, HashSet<&str>)> = HashMap::new();
    for d in &program.fns {
        if d.name.contains('/') {
            continue;
        }
        let entry = by_name.entry(d.name.as_str()).or_insert((true, HashSet::new()));
        entry.0 &= d.synthetic;
        if d.synthetic {
            for twin in &program.fns {
                if !twin.synthetic
                    && twin.file == d.file
                    && twin.span.line == d.span.line
                    && twin.span.col == d.span.col
                    && twin.params.len() == d.params.len()
                    && twin.name.ends_with(&format!("/{}", d.name))
                {
                    entry.1.insert(twin.name.as_str());
                }
            }
        }
    }
    let mut skip: HashSet<String> = std::env::var("KANSO_ALIAS_SKIP")
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    // a name that is ever locally bound — a parameter, a `x = ...` binding, a
    // lambda parameter, a destructured field — must not be rewritten, because
    // an occurrence may mean the local rather than the function. Excluding
    // the name entirely keeps its clones and the old dispatch, which is
    // correct and merely unoptimised.
    for d in &program.fns {
        for p in &d.params {
            bound_in_pattern(p, &mut skip);
        }
        for stmt in &d.body {
            bound_in_stmt(stmt, &mut skip);
        }
    }
    let aliases: HashMap<String, String> = by_name
        .into_iter()
        .filter(|(name, (all_synthetic, targets))| {
            *all_synthetic && targets.len() == 1 && !skip.contains(*name)
        })
        .map(|(bare, (_, targets))| {
            (bare.to_string(), (*targets.iter().next().expect("one target")).to_string())
        })
        .collect();
    if aliases.is_empty() {
        return;
    }
    if std::env::var("KANSO_ALIAS_REPORT").is_ok() {
        let mut v: Vec<_> = aliases.iter().collect();
        v.sort();
        for (b, q) in v {
            eprintln!("alias: {b} -> {q}");
        }
    }
    program.fns.retain(|d| !(d.synthetic && aliases.contains_key(&d.name)));
    for d in &mut program.fns {
        for stmt in &mut d.body {
            alias_stmt(stmt, &aliases);
        }
    }
}

fn alias_stmt(stmt: &mut ast::Stmt, aliases: &std::collections::HashMap<String, String>) {
    match stmt {
        ast::Stmt::Bind { expr, .. }
        | ast::Stmt::Expr(expr)
        | ast::Stmt::Set { value: expr, .. } => alias_expr(expr, aliases),
    }
}

fn alias_expr(e: &mut ast::Expr, aliases: &std::collections::HashMap<String, String>) {
    match e {
        ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => {
            if let Some(q) = aliases.get(name) {
                *name = q.clone();
            }
        }
        ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                alias_expr(k, aliases);
                alias_expr(v, aliases);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    alias_expr(inner, aliases);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                alias_expr(item, aliases);
            }
        }
        ast::Expr::App { head, args, .. } => {
            alias_expr(head, aliases);
            for a in args {
                alias_expr(a, aliases);
            }
        }
        ast::Expr::Field { base, .. } => alias_expr(base, aliases),
        ast::Expr::Index { base, index, .. } => {
            alias_expr(base, aliases);
            alias_expr(index, aliases);
        }
        ast::Expr::Seq(a, b, _) | ast::Expr::Join { lhs: a, rhs: b, .. } => {
            alias_expr(a, aliases);
            alias_expr(b, aliases);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            alias_expr(body, aliases)
        }
        ast::Expr::BinOp { lhs, rhs, .. } => {
            alias_expr(lhs, aliases);
            alias_expr(rhs, aliases);
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            for st in stmts {
                alias_stmt(st, aliases);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            alias_expr(cond, aliases);
            alias_expr(early, aliases);
            for st in rest {
                alias_stmt(st, aliases);
            }
        }
    }
}

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
    let mut shorts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let std_names: std::collections::HashSet<String> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list"))
        .map(|d| {
            d.name.rsplit_once('/').map(|(_, s)| s.to_string()).unwrap_or_else(|| d.name.clone())
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
    // private std/list helpers the generated reducers lean on (bump,
    // file_under), resolved to whatever qualified spelling the module
    // graph produced — privacy is a check-time property, and fusion runs
    // after the check
    let helpers: std::collections::HashMap<String, String> = program
        .fns
        .iter()
        .filter(|d| d.file.starts_with("std/list") && !d.synthetic)
        .map(|d| {
            let short = d
                .name
                .rsplit_once('/')
                .map(|(_, s)| s.to_string())
                .unwrap_or_else(|| d.name.clone());
            (short, d.name.clone())
        })
        .collect();
    let mut counter = 0usize;
    for decl in &mut program.fns {
        if decl.file.starts_with("std/") {
            continue;
        }
        inline_single_use_chains(&mut decl.body, &shorts);
        for stmt in &mut decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    fuse_expr(expr, &shorts, &fold_name, &helpers, &mut counter);
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
        let is_adapter = shorts.get(aname.as_str()).is_some_and(|s| ADAPTERS.contains(&s.as_str()));
        if !is_adapter {
            idx += 1;
            continue;
        }
        let name = name.clone();
        let mut uses = 0usize;
        for later in body.iter().skip(idx + 1) {
            match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                    count_ident_uses(expr, &name, &mut uses);
                }
            }
        }
        let sole_coll_use = uses == 1
            && body.iter().skip(idx + 1).any(|later| match later {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
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
                Stmt::Bind { expr: e, .. } | Stmt::Expr(e) | Stmt::Set { value: e, .. } => {
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
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Bind { expr, .. }
                    | ast::Stmt::Expr(expr)
                    | ast::Stmt::Set { value: expr, .. } => {
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
    helpers: &std::collections::HashMap<String, String>,
    counter: &mut usize,
) {
    use ast::Expr;
    match e {
        Expr::App { head, args, .. } => {
            fuse_expr(head, shorts, fold_name, helpers, counter);
            for a in args.iter_mut() {
                fuse_expr(a, shorts, fold_name, helpers, counter);
            }
        }
        Expr::Lambda { body, .. } => fuse_expr(body, shorts, fold_name, helpers, counter),
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Bind { expr, .. }
                    | ast::Stmt::Expr(expr)
                    | ast::Stmt::Set { value: expr, .. } => {
                        fuse_expr(expr, shorts, fold_name, helpers, counter)
                    }
                }
            }
        }
        Expr::Seq(a, b, _) | Expr::Join { lhs: a, rhs: b, .. } => {
            fuse_expr(a, shorts, fold_name, helpers, counter);
            fuse_expr(b, shorts, fold_name, helpers, counter);
        }
        Expr::List(items, _) => {
            for i in items {
                fuse_expr(i, shorts, fold_name, helpers, counter);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                fuse_expr(k, shorts, fold_name, helpers, counter);
                fuse_expr(v, shorts, fold_name, helpers, counter);
            }
        }
        Expr::Index { base, index, .. } => {
            fuse_expr(base, shorts, fold_name, helpers, counter);
            fuse_expr(index, shorts, fold_name, helpers, counter);
        }
        Expr::Field { base, .. } => fuse_expr(base, shorts, fold_name, helpers, counter),
        Expr::Upcast { expr, .. } => fuse_expr(expr, shorts, fold_name, helpers, counter),
        Expr::BinOp { lhs, rhs, .. } => {
            fuse_expr(lhs, shorts, fold_name, helpers, counter);
            fuse_expr(rhs, shorts, fold_name, helpers, counter);
        }
        Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    fuse_expr(inner, shorts, fold_name, helpers, counter);
                }
            }
        }
        _ => {}
    }
    if let Some(rewritten) = try_fuse(e, shorts, fold_name, helpers, counter) {
        *e = rewritten;
    } else if let Some(rewritten) = try_fuse_piped(e, shorts, fold_name, helpers, counter) {
        *e = rewritten;
    }
}

/// The pipe spelling of a chain fuses too, one runtime branch late. A piped
/// chain's subject may be a description — the pipe is a bind there, and no
/// rewrite may touch it — so the chain becomes: bind the subject once, and if
/// it is not a description, run the nested spelling of the same chain, which
/// try_fuse has already flattened into one fold. A description takes the
/// original pipes; everything else takes the loop. The cost of the honesty
/// is one tag test per chain and a second copy of the chain's code.
fn try_fuse_piped(
    e: &ast::Expr,
    shorts: &std::collections::HashMap<String, String>,
    fold_name: &str,
    helpers: &std::collections::HashMap<String, String>,
    counter: &mut usize,
) -> Option<ast::Expr> {
    use ast::{Expr, Pattern, Stmt};
    // collect the piped stages outside-in: consumer first, subject last
    let mut stages: Vec<(&str, &[Expr], crate::diag::Span)> = Vec::new();
    let mut cur = e;
    #[allow(clippy::while_let_loop)]
    loop {
        let Expr::App { head, args, span, piped: true } = cur else { break };
        let Expr::Ident(name, _) = head.as_ref() else { break };
        if !shorts.contains_key(name.as_str()) || args.is_empty() {
            break;
        }
        stages.push((name.as_str(), args.as_slice(), *span));
        cur = &args[0];
    }
    if stages.len() < 2 {
        return None;
    }
    let root = cur.clone();
    let span = stages[0].2;
    *counter += 1;
    let tmp = format!("froot{counter}");
    let tmp_ident = || Expr::Ident(tmp.clone(), span);
    // the nested spelling with the bound subject as its innermost source
    let mut nested = tmp_ident();
    for (name, args, sspan) in stages.iter().rev() {
        let mut all = vec![nested];
        all.extend(args[1..].iter().cloned());
        nested = Expr::App {
            head: Box::new(Expr::Ident((*name).to_string(), *sspan)),
            args: all,
            span: *sspan,
            piped: false,
        };
    }
    let fused = try_fuse(&nested, shorts, fold_name, helpers, counter)?;
    // the original pipes, re-rooted at the binding
    let mut original = tmp_ident();
    for (name, args, sspan) in stages.iter().rev() {
        let mut all = vec![original];
        all.extend(args[1..].iter().cloned());
        original = Expr::App {
            head: Box::new(Expr::Ident((*name).to_string(), *sspan)),
            args: all,
            span: *sspan,
            piped: true,
        };
    }
    let test = Expr::App {
        head: Box::new(Expr::Ident("is_desc".to_string(), span)),
        args: vec![tmp_ident()],
        span,
        piped: false,
    };
    let picked = Expr::App {
        head: Box::new(Expr::Ident("if".to_string(), span)),
        args: vec![test, original, fused],
        span,
        piped: false,
    };
    Some(Expr::Block(
        vec![Stmt::Bind { pattern: Pattern::Var(tmp, span), expr: root }, Stmt::Expr(picked)],
        span,
    ))
}

fn try_fuse(
    e: &ast::Expr,
    shorts: &std::collections::HashMap<String, String>,
    fold_name: &str,
    helpers: &std::collections::HashMap<String, String>,
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
    let call = |h: Expr, a: Vec<Expr>| Expr::App { head: Box::new(h), args: a, span, piped: false };
    *counter += 1;
    let acc = format!("facc{counter}");
    let x = format!("felem{counter}");
    let (init, reducer) = match (consumer.as_str(), args.len()) {
        ("fold", 3) => (args[1].clone(), args[2].clone()),
        ("tally", 1) => {
            let bump = helpers.get("bump")?;
            let seen = Expr::Index {
                base: Box::new(ident(&acc)),
                index: Box::new(ident(&x)),
                strict: false,
                span,
            };
            let bumped = call(ident(bump), vec![seen]);
            (
                Expr::MapLit(Vec::new(), span),
                lam(vec![&acc, &x], call(ident("put"), vec![ident(&acc), ident(&x), bumped])),
            )
        }
        ("group_by", 2) => {
            let file_under = helpers.get("file_under")?;
            let keyed = call(args[1].clone(), vec![ident(&x)]);
            (
                Expr::MapLit(Vec::new(), span),
                lam(vec![&acc, &x], call(ident(file_under), vec![ident(&acc), keyed, ident(&x)])),
            )
        }
        ("sum", 1) => (
            Expr::Int(0u32.into(), span),
            lam(
                vec![&acc, &x],
                Expr::BinOp { op: "+", lhs: Box::new(ident(&acc)), rhs: Box::new(ident(&x)), span },
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

fn qualify(
    dep: &mut ast::Program,
    qual: &str,
    exports: &mut std::collections::HashMap<String, bool>,
) {
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
        // a getter is structural, not owned: one group per field name across
        // every module, reachable without an import
        if f.is_getter() {
            continue;
        }
        exports.entry(format!("{qual}/{}", f.name)).or_insert(f.is_pub);
        f.name = format!("{qual}/{}", f.name);
        let mut bound = Vec::new();
        for p in &mut f.params {
            rewrite_pattern(p, qual, &owned);
            pattern_binds(p, &mut bound);
        }
        for stmt in &mut f.body {
            rewrite_stmt(stmt, qual, &owned, &mut bound);
        }
    }
}

/// `x.name` is the getter applied. The rewrite runs late, after the checks
/// that read field syntax to say something about the field — a type conflict
/// names the read site, and an application would have nothing to point at.
pub fn desugar_field_reads(program: &mut ast::Program) {
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            desugar_stmt(stmt);
        }
    }
}

fn desugar_stmt(stmt: &mut ast::Stmt) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => desugar_expr(expr),
        ast::Stmt::Set { value, .. } => desugar_expr(value),
    }
}

fn desugar_expr(e: &mut ast::Expr) {
    if let ast::Expr::Field { base, name, span } = e {
        desugar_expr(base);
        let head = ast::Expr::Ident(ast::getter_name(name), *span);
        let base = std::mem::replace(base.as_mut(), ast::Expr::Int(0.into(), *span));
        *e = ast::Expr::App { head: Box::new(head), args: vec![base], span: *span, piped: false };
        return;
    }
    walk_children_mut(e, &mut desugar_expr);
}

/// Every field declares a getter so that name resolves wherever it is read,
/// but a program that never reads a field should not pay for its accessor.
/// Dropping the unreferenced ones is unobservable — nothing can call a
/// function whose name the program does not mention — and it keeps the
/// emitted output the size it was before accessors became functions.
pub fn prune_unused_getters(program: &mut ast::Program) {
    let mut mentioned = std::collections::HashSet::new();
    for decl in &program.fns {
        if decl.is_getter() {
            continue;
        }
        for stmt in &decl.body {
            mentions_in_stmt(stmt, &mut mentioned);
        }
    }
    program.fns.retain(|d| !d.is_getter() || mentioned.contains(&d.name));
}

fn mentions_in_stmt(stmt: &ast::Stmt, out: &mut std::collections::HashSet<String>) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => mentions_in_expr(expr, out),
        ast::Stmt::Set { value, .. } => mentions_in_expr(value, out),
    }
}

fn mentions_in_expr(e: &ast::Expr, out: &mut std::collections::HashSet<String>) {
    match e {
        // Before desugaring a read is still `x.name`; after it, a call to the
        // getter. Both count, so this answers the same set either side of it.
        ast::Expr::Field { base, name, .. } => {
            out.insert(ast::getter_name(name));
            mentions_in_expr(base, out);
        }
        ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => {
            out.insert(name.clone());
            // a qualified read reaches the same getter under its bare name
            if let Some((_, short)) = name.rsplit_once('/') {
                out.insert(short.to_string());
            }
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            for s in stmts {
                mentions_in_stmt(s, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            mentions_in_expr(cond, out);
            mentions_in_expr(early, out);
            for s in rest {
                mentions_in_stmt(s, out);
            }
        }
        _ => {
            for child in expr_children(e) {
                mentions_in_expr(child, out);
            }
        }
    }
}

/// Names a pattern brings into scope. Qualification has to know them: a
/// local named like one of the module's own declarations is the local, and
/// rewriting it into a module reference silently swaps a value for a
/// function. Field getters make this reachable — `left` is a declared name
/// the moment some type has that field — where before the shadow ban made
/// the case impossible to write.
fn pattern_binds(p: &ast::Pattern, out: &mut Vec<String>) {
    match p {
        ast::Pattern::Var(name, _) | ast::Pattern::Annotated { name, .. } => out.push(name.clone()),
        ast::Pattern::Ctor { fields, .. } => {
            for f in fields {
                pattern_binds(f, out);
            }
        }
        ast::Pattern::Keyed { entries, .. } => {
            out.extend(entries.iter().map(|e| e.bind_name.clone()))
        }
        _ => {}
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

fn rewrite_stmt(
    stmt: &mut ast::Stmt,
    qual: &str,
    owned: &std::collections::HashSet<String>,
    bound: &mut Vec<String>,
) {
    match stmt {
        ast::Stmt::Bind { expr, pattern } => {
            rewrite_pattern(pattern, qual, owned);
            rewrite_expr(expr, qual, owned, bound);
            pattern_binds(pattern, bound);
        }
        ast::Stmt::Expr(e) => rewrite_expr(e, qual, owned, bound),
        ast::Stmt::Set { value, .. } => rewrite_expr(value, qual, owned, bound),
    }
}

fn rewrite_scope(
    stmts: &mut [ast::Stmt],
    qual: &str,
    owned: &std::collections::HashSet<String>,
    bound: &[String],
) {
    let mut inner = bound.to_vec();
    for stmt in stmts {
        rewrite_stmt(stmt, qual, owned, &mut inner);
    }
}

fn rewrite_expr(
    e: &mut ast::Expr,
    qual: &str,
    owned: &std::collections::HashSet<String>,
    bound: &[String],
) {
    match e {
        ast::Expr::Partial(..) => {}
        ast::Expr::Guard { cond, early, rest, .. } => {
            rewrite_expr(cond, qual, owned, bound);
            rewrite_expr(early, qual, owned, bound);
            rewrite_scope(rest, qual, owned, bound);
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            rewrite_scope(stmts, qual, owned, bound)
        }
        ast::Expr::Ident(name, _) => {
            if owned.contains(name.as_str()) && !bound.iter().any(|b| b == name) {
                *name = format!("{qual}/{name}");
            }
        }
        ast::Expr::Field { base, .. } => rewrite_expr(base, qual, owned, bound),
        ast::Expr::Upcast { expr, .. } => rewrite_expr(expr, qual, owned, bound),
        ast::Expr::App { head, args, .. } => {
            rewrite_expr(head, qual, owned, bound);
            for a in args {
                rewrite_expr(a, qual, owned, bound);
            }
        }
        ast::Expr::Index { base, index, .. } => {
            rewrite_expr(base, qual, owned, bound);
            rewrite_expr(index, qual, owned, bound);
        }
        ast::Expr::BinOp { lhs, rhs, .. } | ast::Expr::Join { lhs, rhs, .. } => {
            rewrite_expr(lhs, qual, owned, bound);
            rewrite_expr(rhs, qual, owned, bound);
        }
        ast::Expr::Seq(a, b, _) => {
            rewrite_expr(a, qual, owned, bound);
            rewrite_expr(b, qual, owned, bound);
        }
        ast::Expr::Lambda { params, body, .. } => {
            let mut inner = bound.to_vec();
            inner.extend(params.iter().map(|(n, _)| n.clone()));
            rewrite_expr(body, qual, owned, &inner);
        }
        ast::Expr::List(items, _) => {
            for i in items {
                rewrite_expr(i, qual, owned, bound);
            }
        }
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                rewrite_expr(k, qual, owned, bound);
                rewrite_expr(v, qual, owned, bound);
            }
        }
        ast::Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    rewrite_expr(inner, qual, owned, bound);
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
        if f.is_getter() {
            continue;
        }
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
    let local_types: std::collections::HashSet<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    merge_ambient_arms_with(program, &local_types)
}

/// The same merge for a module spread over files: ownership counts a type
/// defined in any of them.
fn merge_ambient_arms_with(
    program: &mut ast::Program,
    local_types: &std::collections::HashSet<String>,
) -> Vec<diag::Diagnostic> {
    let mut diags = Vec::new();
    for decl in &mut program.fns {
        if decl.name == "to_string" {
            // The ownership rule, enforced at the definition site: an arm
            // joining a group this module doesn't own must involve a type it
            // does own. Re-arming a primitive or the sentinels is reserved
            // to the stdlib; wrap the value in your own type instead.
            let owns_a_type = decl.params.iter().any(|p| match p {
                ast::Pattern::Ctor { ty, .. } => local_types.contains(ty),
                ast::Pattern::Annotated { ty, .. } => local_types.contains(ty),
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
    let mut dep_program = ast::Program {
        fns: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
    };
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
        // the shipped library, embedded so the browser (no filesystem) and
        // an installed binary (no lib/ beside it) resolve the same sources
        // the checkout does. A module is its whole file set, so a
        // multi-file module lists every file.
        let embedded: Option<(&str, &[(&str, &str)])> = match path.as_str() {
            "std/render" => {
                Some(("render", &[("render.kso", include_str!("../lib/render/render.kso"))]))
            }
            "std/list" => Some(("list", &[("list.kso", include_str!("../lib/list/list.kso"))])),
            "std/time" => Some(("time", &[("time.kso", include_str!("../lib/time/time.kso"))])),
            "std/io" => Some(("io", &[("io.kso", include_str!("../lib/io/io.kso"))])),
            "std/text" => Some(("text", &[("text.kso", include_str!("../lib/text/text.kso"))])),
            "std/math" => Some(("math", &[("math.kso", include_str!("../lib/math/math.kso"))])),
            "std/path" => Some(("path", &[("path.kso", include_str!("../lib/path/path.kso"))])),
            "std/json" => Some((
                "json",
                &[
                    ("json.kso", include_str!("../lib/json/json.kso")),
                    ("number.kso", include_str!("../lib/json/number.kso")),
                    ("scan.kso", include_str!("../lib/json/scan.kso")),
                    ("text.kso", include_str!("../lib/json/text.kso")),
                    ("value.kso", include_str!("../lib/json/value.kso")),
                ],
            )),
            _ => None,
        };
        if let Some((short, files)) = embedded {
            // a module importing itself compiles a second copy, so every type
            // gets a twin and a constructor pattern stops matching values the
            // other half built — a confusing failure a long way from here
            if base.file_name().is_some_and(|n| n == short) {
                return Err(format!(
                    "error: a module cannot import itself — `{path}` is this \
                     module, and the second copy's types would not match this \
                     one's\n"
                ));
            }
            let qualified: Vec<(String, String)> =
                files.iter().map(|(n, s)| (format!("{path}/{n}"), s.to_string())).collect();
            let borrowed: Vec<(&str, &str)> =
                qualified.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect();
            let mut dep =
                compile_module_inner(std::path::Path::new(path), false, visited, Some(&borrowed))?;
            qualify(&mut dep, qual, &mut exports);
            dep_program.types.extend(dep.types);
            dep_program.fns.extend(dep.fns);
            continue;
        }
        if path == "std/random" {
            return Err("error: `std/random` moved — `random` lives in `std/math`
"
            .to_string());
        }
        let dep_dir = resolve_import(base, path)?;
        // importing one's own module compiles a second copy of it, so every
        // type gets a twin and a constructor pattern stops matching values
        // built by the other half — a confusing failure a long way from here
        let same = std::fs::canonicalize(&dep_dir)
            .ok()
            .zip(std::fs::canonicalize(base).ok())
            .is_some_and(|(a, b)| a == b);
        if same {
            return Err(format!(
                "error: a module cannot import itself — `{path}` resolves to \
                 this module, and the second copy's types would not match \
                 this one's\n"
            ));
        }
        let mut dep = compile_module_inner(&dep_dir, false, visited, None)?;
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
            let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
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
                return Err(format!("error: `{}` exports no `{theirs}` to rename\n", import.path));
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
                ast::Stmt::Bind { expr, .. }
                | ast::Stmt::Expr(expr)
                | ast::Stmt::Set { value: expr, .. } => collect(expr, &mut bare),
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
                ast::Stmt::Set { value, .. } => walk_expr(value, quals),
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
        .map(|i| diag::Diagnostic::new("unused", format!("unused import \"{}\"", i.path), i.span))
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
        ast::Expr::Partial(_, s) => s,
        ast::Expr::Guard { span: s, .. }
        | ast::Expr::Ident(_, s)
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
        ast::Expr::Build(_, s) => s,
    }
}

/// A qualified reference to a name its module did not mark pub.
fn private_uses(
    stmt: &ast::Stmt,
    exports: &std::collections::HashMap<String, bool>,
    diags: &mut Vec<diag::Diagnostic>,
) {
    fn walk(
        e: &ast::Expr,
        exports: &std::collections::HashMap<String, bool>,
        diags: &mut Vec<diag::Diagnostic>,
    ) {
        if let ast::Expr::Ident(name, span) = e {
            if let Some(false) = exports.get(name.as_str()) {
                let (module, base) = name.rsplit_once('/').unwrap_or(("", name));
                diags.push(diag::Diagnostic::new(
                    "opacity",
                    format!(
                        "`{base}` is private to module `{module}` — only pub names cross an import"
                    ),
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
        ast::Stmt::Set { value, .. } => walk(value, exports, diags),
    }
}

pub fn expr_children(e: &ast::Expr) -> Vec<&ast::Expr> {
    match e {
        ast::Expr::Partial(..) => Vec::new(),
        ast::Expr::Upcast { expr, .. } => vec![expr.as_ref()],
        ast::Expr::Guard { cond, early, rest, .. } => {
            let mut v = vec![cond.as_ref(), early.as_ref()];
            v.extend(rest.iter().map(|st| match st {
                ast::Stmt::Bind { expr, .. }
                | ast::Stmt::Expr(expr)
                | ast::Stmt::Set { value: expr, .. } => expr,
            }));
            v
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => stmts
            .iter()
            .map(|st| match st {
                ast::Stmt::Bind { expr, .. }
                | ast::Stmt::Expr(expr)
                | ast::Stmt::Set { value: expr, .. } => expr,
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
pub fn compile_module(dir: &std::path::Path, require_entry: bool) -> Result<ast::Program, String> {
    LOCK.with(|l| *l.borrow_mut() = hako::read_lock(dir));
    let mut visited = std::collections::HashSet::new();
    compile_module_root(dir, require_entry, &mut visited)
}

/// hako's own verbs, written in kanso and carried in the binary the way the
/// shipped library is. There is no directory to read: an installed `kanso`
/// has no `hako/` beside it, and the files are the ones in this checkout.
pub fn compile_hako() -> Result<ast::Program, String> {
    const FILES: &[(&str, &str)] = &[
        ("cache.kso", include_str!("../hako/cache.kso")),
        ("hako.kso", include_str!("../hako/hako.kso")),
        ("install.kso", include_str!("../hako/install.kso")),
        ("lock.kso", include_str!("../hako/lock.kso")),
        ("main.kso", include_str!("../hako/main.kso")),
        ("remote.kso", include_str!("../hako/remote.kso")),
        ("update.kso", include_str!("../hako/update.kso")),
    ];
    LOCK.with(|l| l.borrow_mut().clear());
    let mut visited = std::collections::HashSet::new();
    AMBIENT_ROOT.with(|c| c.set(true));
    let built = compile_module_inner(std::path::Path::new("hako"), true, &mut visited, Some(FILES));
    AMBIENT_ROOT.with(|c| c.set(false));
    built
}

/// The root module gets the ambient imports (design/render-plan.md);
/// dependencies never do — deps compile exactly as written.
fn compile_module_root(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<ast::Program, String> {
    AMBIENT_ROOT.with(|c| c.set(true));
    let result = compile_module_inner(dir, require_entry, visited, None);
    AMBIENT_ROOT.with(|c| c.set(false));
    result
}

thread_local! {
    /// The lock read from the module root, so every import in a build sees the
    /// same pins. Empty when there is no lock, which is what lets a cache
    /// entry answer by bare name.
    static LOCK: std::cell::RefCell<std::collections::BTreeMap<String, hako::Pin>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };

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

/// A module is a cycle when it is already being loaded, not when it has been
/// loaded before: two modules that both import `std/list` are a diamond, and
/// a diamond is the ordinary shape of any program with a dependency. So the
/// set is the path currently open, and a module leaves it once its own load
/// finishes.
fn compile_module_inner(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
    embedded: Option<&[(&str, &str)]>,
) -> Result<ast::Program, String> {
    let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canon.clone()) {
        return Err(format!("error: import cycle through {}\n", dir.display()));
    }
    let loaded = compile_module_loaded(dir, require_entry, visited, embedded);
    visited.remove(&canon);
    loaded
}

fn compile_module_loaded(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
    embedded: Option<&[(&str, &str)]>,
) -> Result<ast::Program, String> {
    let mut sources: Vec<(String, String)> = match embedded {
        Some(files) => files.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect(),
        None => {
            let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
                .map_err(|io| format!("error: cannot read {}: {io}\n", dir.display()))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|p| p.extension().is_some_and(|e| e == "kso"))
                .collect();
            paths.sort();
            let mut out = Vec::new();
            for path in &paths {
                let file = path.to_string_lossy().to_string();
                let source = std::fs::read_to_string(path)
                    .map_err(|io| format!("error: cannot read {file}: {io}\n"))?;
                out.push((file, source));
            }
            out
        }
    };
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    if sources.is_empty() {
        return Err(format!("error: no .kso files in {}\n", dir.display()));
    }
    let mut parsed = Vec::new();
    for (file, source) in sources.drain(..) {
        let path = std::path::PathBuf::from(&file);
        let lexed = lexer::lex(&source).map_err(|d| diag::render(&d, &file, &source))?;
        let is_entry = path.file_name().is_some_and(|n| n == "main.kso");
        let mut program = match is_entry {
            true => parser::parse_entry(&lexed).map_err(|d| diag::render(&d, &file, &source))?,
            false => parser::parse(&lexed).map_err(|d| diag::render(&d, &file, &source))?,
        };
        synthesize_getters(&mut program);
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
        let module_types: std::collections::HashSet<String> =
            parsed.iter().flat_map(|(_, _, p)| p.types.iter().map(|t| t.name.clone())).collect();
        for (file, source, program) in &mut parsed {
            let ownership_diags = merge_ambient_arms_with(program, &module_types);
            if !ownership_diags.is_empty() {
                return Err(diag::render(&ownership_diags, file, source));
            }
        }
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
    let mut merged = ast::Program {
        fns: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
    };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    for (_, _, program) in parsed {
        merged.types.extend(program.types);
        merged.fns.extend(program.fns);
    }
    let mut diags = check::check_merged(&merged, require_entry);
    check::check_unused_private(&merged, &used, &mut diags);
    synthesize_getters(&mut merged);
    desugar_field_reads(&mut merged);
    prune_unused_getters(&mut merged);
    trmc::rewrite(&mut merged);
    inline::inline_builtin_wrappers(&mut merged);
    if !diags.is_empty() {
        let file = dir.to_string_lossy();
        let rendered: Vec<String> = diags
            .iter()
            .map(|d| format!("error[{}]: {} (module {file})\n", d.kind, d.message))
            .collect();
        return Err(rendered.join(""));
    }
    canonicalize_types(&mut merged);
    canonicalize_bare_aliases(&mut merged);
    hoist_repeated_strings(&mut merged);
    fuse_enumerable(&mut merged);
    synthesize_getters(&mut merged);
    desugar_field_reads(&mut merged);
    prune_unused_getters(&mut merged);
    trmc::rewrite(&mut merged);
    Ok(merged)
}

/// Evaluate a repeated pure interpolation once.
///
/// Reading a key and then writing it is the ordinary shape of map code, and
/// spelled directly it builds the key twice: `put m "k{i}" (bump m["k{i}"])`
/// allocates two identical strings an iteration. Binding it costs one.
///
/// The licence is deliberately small, because hoisting is only safe when
/// evaluating once instead of twice cannot be observed. An interpolation
/// qualifies when every piece of it is a name, a number or arithmetic over
/// those — no calls, no indexing, nothing that can fail or reach an effect.
/// Lambda bodies are left alone entirely: a folder's shape is what the
/// linearity analysis matches on, and a binding in front of it would hide the
/// write that makes a fold write in place.
pub fn hoist_repeated_strings(program: &mut ast::Program) {
    let mut counter = 0usize;
    for decl in &mut program.fns {
        if decl.synthetic {
            continue;
        }
        hoist_in_body(&mut decl.body, &mut counter);
    }
}

/// Arithmetic over names and numbers, and nothing else.
fn purely_computed(e: &ast::Expr) -> bool {
    use ast::Expr;
    match e {
        Expr::Ident(..) | Expr::Int(..) | Expr::Float(..) => true,
        Expr::BinOp { op, lhs, rhs, .. } => {
            matches!(*op, "+" | "-" | "*" | "/" | "%")
                && purely_computed(lhs)
                && purely_computed(rhs)
        }
        Expr::Str(parts, _) => parts.iter().all(|p| match p {
            ast::TemplatePart::Lit(_) => true,
            ast::TemplatePart::Interp(inner) => purely_computed(inner),
        }),
        _ => false,
    }
}

/// A shape key that ignores where the expression was written.
fn shape_of(e: &ast::Expr) -> String {
    let text = format!("{e:?}");
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find("Span {") {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find('}') {
            Some(j) => &rest[i + j + 1..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn collect_hoistable(e: &ast::Expr, found: &mut Vec<(String, ast::Expr)>) {
    use ast::Expr;
    if let Expr::Lambda { .. } = e {
        return;
    }
    if let Expr::Str(parts, _) = e {
        let interpolated = parts.iter().any(|p| matches!(p, ast::TemplatePart::Interp(_)));
        if interpolated && purely_computed(e) {
            found.push((shape_of(e), e.clone()));
            return;
        }
    }
    for child in expr_children(e) {
        collect_hoistable(child, found);
    }
}

fn replace_shape(e: &mut ast::Expr, shape: &str, name: &str) {
    use ast::Expr;
    if let Expr::Lambda { .. } = e {
        return;
    }
    if shape_of(e) == shape {
        let span = e.span();
        *e = Expr::Ident(name.to_string(), span);
        return;
    }
    walk_children_mut(e, &mut |c| replace_shape(c, shape, name));
}

/// Every direct sub-expression, mutably. Mirrors `expr_children`.
fn walk_children_mut(e: &mut ast::Expr, f: &mut dyn FnMut(&mut ast::Expr)) {
    use ast::Expr;
    match e {
        Expr::App { head, args, .. } => {
            f(head);
            for a in args {
                f(a);
            }
        }
        Expr::List(items, _) => {
            for i in items {
                f(i);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                f(k);
                f(v);
            }
        }
        Expr::Index { base, index, .. } => {
            f(base);
            f(index);
        }
        Expr::Field { base, .. } => f(base),
        Expr::Upcast { expr, .. } => f(expr),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
        Expr::Seq(a, b, _) => {
            f(a);
            f(b);
        }
        Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    f(inner);
                }
            }
        }
        _ => {}
    }
}

fn hoist_in_body(body: &mut Vec<ast::Stmt>, counter: &mut usize) {
    let mut i = 0;
    while i < body.len() {
        let mut found = Vec::new();
        match &body[i] {
            ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                collect_hoistable(expr, &mut found)
            }
            ast::Stmt::Set { value, .. } => collect_hoistable(value, &mut found),
        }
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (shape, _) in &found {
            *seen.entry(shape.clone()).or_default() += 1;
        }
        let mut repeated: Vec<(String, ast::Expr)> = Vec::new();
        let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (shape, expr) in found {
            if seen.get(&shape).copied().unwrap_or(0) > 1 && taken.insert(shape.clone()) {
                repeated.push((shape, expr));
            }
        }
        if repeated.is_empty() {
            i += 1;
            continue;
        }
        let mut inserted = 0;
        for (shape, expr) in repeated {
            let name = format!("once{counter}");
            *counter += 1;
            let span = expr.span();
            match &mut body[i + inserted] {
                ast::Stmt::Bind { expr: e, .. } | ast::Stmt::Expr(e) => {
                    replace_shape(e, &shape, &name)
                }
                ast::Stmt::Set { value, .. } => replace_shape(value, &shape, &name),
            }
            body.insert(
                i + inserted,
                ast::Stmt::Bind { pattern: ast::Pattern::Var(name, span), expr },
            );
            inserted += 1;
        }
        i += inserted + 1;
    }
}
