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
pub mod hash;
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
    let lexed =
        phase::watched("lex", || lexer::lex(source)).map_err(|d| diag::render(&d, file, source))?;
    let mut program = phase::watched("parse", || parser::parse(&lexed))
        .map_err(|d| diag::render(&d, file, source))?;
    phase::watched("finish_program", || finish_program(&mut program));
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
    // The entry's directory is the project: its lock pins every hako import
    // the build resolves, exactly as a module root's does.
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    LOCK.with(|l| *l.borrow_mut() = hako::read_lock(&base));
    ENTRY_COMPILE.with(|c| c.set(true));
    let built = compile_entry_inner(file, source);
    ENTRY_COMPILE.with(|c| c.set(false));
    built
}

/// `kanso play`: the relaxed single file — declarations and statements
/// together, no `pub`, no local imports. The verb is the only door, so the
/// form cannot leak into real programs; everything past the parse and the
/// stdlib gate is the entry pipeline unchanged.
pub fn compile_play_file(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let program = parser::parse_play(&lexed).map_err(|d| diag::render(&d, file, source))?;
    for import in &program.imports {
        if !import.path.starts_with("std/") {
            let d = diag::Diagnostic::new(
                "import",
                format!(
                    "a play file imports the stdlib and nothing else — \
                     `{}` needs a real program (`kanso run`)",
                    import.path
                ),
                import.span,
            );
            return Err(diag::render(&[d], file, source));
        }
    }
    ENTRY_COMPILE.with(|c| c.set(true));
    let built = compile_parsed_entry(program, file, source);
    ENTRY_COMPILE.with(|c| c.set(false));
    built
}

fn compile_entry_inner(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let program = parser::parse_entry(&lexed).map_err(|d| diag::render(&d, file, source))?;
    compile_parsed_entry(program, file, source)
}

fn compile_parsed_entry(
    mut program: ast::Program,
    file: &str,
    source: &str,
) -> Result<ast::Program, String> {
    finish_program(&mut program);
    stamp_file(&mut program, file);
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let ownership_diags = phase::watched("merge_ambient_arms", || merge_ambient_arms(&mut program));
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = crate::hash::Set::default();
    let (dep_program, exports, shadowed, surfaced) =
        load_dependencies(&base, &import_list, &mut visited)?;
    let mut quals = crate::hash::Set::default();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &surfaced, &mut quals);
    mark_reexport_quals(&program, |name| exports.contains_key(name), &mut quals);
    let mut diags = unused_imports(&program.imports, &quals);
    // After the import is credited and before the surface is judged: the door
    // resolves to the owner's spelling, so crediting that owner would leave
    // the import the caller wrote reading as unused, and asking whether
    // `geo/order` is pub would ask it of a name nothing declares.
    open_qualified_doors(&mut program, &surfaced, &exports);
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &shadowed, &mut diags);
        }
    }
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    // per-file rules for the entry against the dependency globals, then the
    // merged checks — never file-order rules across module boundaries
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: crate::hash::Set<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let extern_globals = check::declared_names(&dep_program);
    let shadowable: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut used = crate::hash::Set::default();
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
    let merged_diags = check::check_merged(&merged, true);
    finish_program(&mut merged);
    phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
    phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
    trmc::rewrite(&mut merged);
    inline::inline_builtin_wrappers(&mut merged);
    match merged_diags.is_empty() {
        true => {
            phase::watched("canonicalize_types", || canonicalize_types(&mut merged));
            phase::watched("canonicalize_bare_aliases", || canonicalize_bare_aliases(&mut merged));
            phase::watched("hoist_repeated_strings", || hoist_repeated_strings(&mut merged));
            phase::watched("fuse_enumerable", || fuse_enumerable(&mut merged));
            finish_program(&mut merged);
            phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
            phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
            trmc::rewrite(&mut merged);
            Ok(merged)
        }
        false => Err(diag::render(&merged_diags, file, source)),
    }
}

/// Where a compile spends itself. Off unless `KANSO_PHASES` is set, and then
/// one line per phase on stderr at exit — enough to say which phase to attack
/// without a profiler, which this machine does not have.
pub mod phase {
    use std::cell::Cell;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static SPENT: Mutex<Vec<(&'static str, Duration)>> = Mutex::new(Vec::new());

    thread_local! {
        /// Time the phase currently running has already handed to phases
        /// nested inside it. `load_dependencies` calls the whole compiler
        /// again, so without this its bucket would swallow the lexing,
        /// parsing and checking of every module it loads, and the report
        /// would add up to several times the compile it describes.
        static IN_CHILDREN: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    }

    pub fn watched<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
        if std::env::var_os("KANSO_PHASES").is_none() {
            return f();
        }
        let outer = IN_CHILDREN.with(|c| c.replace(Duration::ZERO));
        let started = Instant::now();
        let out = f();
        let whole = started.elapsed();
        let mine = whole - IN_CHILDREN.with(|c| c.get());
        IN_CHILDREN.with(|c| c.set(outer + whole));
        let mut spent = SPENT.lock().unwrap_or_else(|e| e.into_inner());
        match spent.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 += mine,
            None => spent.push((name, mine)),
        }
        out
    }

    /// Longest first, because the question a profile answers is what to attack.
    pub fn report() {
        if std::env::var_os("KANSO_PHASES").is_none() {
            return;
        }
        let mut spent = SPENT.lock().unwrap_or_else(|e| e.into_inner()).clone();
        spent.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        let total: Duration = spent.iter().map(|(_, d)| *d).sum();
        for (name, d) in spent.iter() {
            let share = 100.0 * d.as_secs_f64() / total.as_secs_f64().max(1e-9);
            eprintln!("{name:24} {:>8.2} ms  {share:5.1}%", d.as_secs_f64() * 1000.0);
        }
        eprintln!("{:24} {:>8.2} ms", "watched total", total.as_secs_f64() * 1000.0);
    }
}

/// `kanso play`: the playground's convention at the terminal. The file is a
/// library defining `pub play`; the synthesized entry runs it.
/// A `play` file: one source with its imports, and a `pub play` to run.
/// The repl's session: the same thing without an entry point, so a prompt can
/// import a module and reach it. An unused binding at a prompt is exploration
/// rather than a mistake, so those are dropped here and nowhere else.
pub fn compile_repl(file: &str, source: &str) -> Result<ast::Program, String> {
    compile_one(file, source, true)
}

fn compile_one(file: &str, source: &str, drop_unused: bool) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    finish_program(&mut program);
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    // a play library may import like any module; the ambient module rides
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = crate::hash::Set::default();
    let (mut dep_program, exports, shadowed, surfaced) =
        phase::watched("load_dependencies", || {
            load_dependencies(&base, &import_list, &mut visited)
        })?;
    check_reexports(&program, &mut dep_program, &import_list, file, source)?;
    let mut quals = crate::hash::Set::default();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &surfaced, &mut quals);
    mark_reexport_quals(&program, |name| exports.contains_key(name), &mut quals);
    let mut diags = unused_imports(&program.imports, &quals);
    // After the import is credited and before the surface is judged: the door
    // resolves to the owner's spelling, so crediting that owner would leave
    // the import the caller wrote reading as unused, and asking whether
    // `geo/order` is pub would ask it of a name nothing declares.
    open_qualified_doors(&mut program, &surfaced, &exports);
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &shadowed, &mut diags);
        }
    }
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
    let shadowable: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: crate::hash::Set<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = crate::hash::Set::default();
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
    let merged_diags = check::check_merged(&program, false);
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    canonicalize_bare_aliases(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    finish_program(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    inline::inline_builtin_wrappers(&mut program);
    Ok(program)
}

/// A lone library file under a library verb (`kanso test`/`check`): parses
/// as a library and loads its imports (plus the ambient module) like any
/// other root compile.
pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {
    let lexed = lexer::lex(source).map_err(|d| diag::render(&d, file, source))?;
    let mut program = parser::parse(&lexed).map_err(|d| diag::render(&d, file, source))?;
    finish_program(&mut program);
    stamp_file(&mut program, file);
    let ownership_diags = merge_ambient_arms(&mut program);
    if !ownership_diags.is_empty() {
        return Err(diag::render(&ownership_diags, file, source));
    }
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut import_list: Vec<ast::Import> = program.imports.clone();
    ambient_imports(&mut import_list);
    let mut visited = crate::hash::Set::default();
    let (mut dep_program, exports, shadowed, surfaced) =
        load_dependencies(&base, &import_list, &mut visited)?;
    check_reexports(&program, &mut dep_program, &import_list, file, source)?;
    let mut quals = crate::hash::Set::default();
    used_quals(&program, &mut quals);
    mark_bare_quals(&program, &surfaced, &mut quals);
    mark_reexport_quals(&program, |name| exports.contains_key(name), &mut quals);
    let mut diags = unused_imports(&program.imports, &quals);
    // After the import is credited and before the surface is judged: the door
    // resolves to the owner's spelling, so crediting that owner would leave
    // the import the caller wrote reading as unused, and asking whether
    // `geo/order` is pub would ask it of a name nothing declares.
    open_qualified_doors(&mut program, &surfaced, &exports);
    for decl in &program.fns {
        for stmt in &decl.body {
            private_uses(stmt, &exports, &shadowed, &mut diags);
        }
    }
    foreign_destructures(&program, &mut diags);
    if !diags.is_empty() {
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return Err(diag::render(&diags, file, source));
    }
    let extern_globals = check::declared_names(&dep_program);
    let shadowable: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut all_markers = check::marker_names(&program);
    all_markers.extend(check::marker_names(&dep_program));
    let mut all_type_names: crate::hash::Set<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let mut used = crate::hash::Set::default();
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names));
    diags.extend(check::check_file_shadow(&program, &extern_globals, &mut used, &shadowable));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    let merged_diags = check::check_merged(&program, false);
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    canonicalize_bare_aliases(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    finish_program(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    inline::inline_builtin_wrappers(&mut program);
    Ok(program)
}

/// Route a single source file to the right compile for a verb, by content:
/// `pub play` is a play library, bare statements are an entry, definitions
/// alone are a library (runnable only under a library verb like `test`).
/// Does this file declare `pub play`? Answered off the parse, so comments and
/// string literals cannot change how a file compiles. A file that does not
/// parse declares nothing, and the compile it is routed to reports the syntax
/// error properly.
/// Whether the file holds tests, by the same rule `kanso test` collects them:
/// a zero-argument constant named `test_*`. A file of those has an entry
/// point already — it is just spelled for a different verb, and saying so
/// beats describing what the file is not.
fn declares_tests(source: &str) -> bool {
    let Ok(lexed) = lexer::lex(source) else { return false };
    let Ok(program) = parser::parse(&lexed) else { return false };
    program.fns.iter().any(|d| d.name.starts_with("test_") && d.params.is_empty())
}

/// Whether the module exports `play`, which decides which of two refusals a
/// non-entry file gets. `kanso play` accepts a file of definitions beside bare
/// statements and refuses one holding `pub play`, so the advice to reach for
/// it is true for the first and a dead end for the second.
///
/// Read from the parsed program rather than a line prefix, so `pub play`
/// inside a string or a comment is not mistaken for the export.
fn exports_play(source: &str) -> bool {
    let Ok(lexed) = lexer::lex(source) else { return false };
    let Ok(program) = parser::parse(&lexed) else { return false };
    program.fns.iter().any(|d| d.name == "play" && d.is_pub)
}

/// The CLI and the browser share this so the engines never diverge on which
/// compile a file gets.
pub fn compile_source(command: &str, file: &str, source: &str) -> Result<ast::Program, String> {
    // main.kso is an entry by NAME (the module-shape gavel): the filename
    // states the intent, so a definition inside one gets the entry
    // diagnostic rather than a guess from the file's shape.
    if std::path::Path::new(file).file_name().is_some_and(|n| n == "main.kso") {
        return compile_entry(file, source);
    }
    // Which compile a file gets is decided by what it declares, not by what
    // its text happens to contain: a comment or a string mentioning `pub
    // play` used to reroute the whole file and reject a valid entry with a
    // diagnostic that named none of this.
    let has_defs = source
        .lines()
        .any(|l| l.starts_with("fn ") || l.starts_with("type ") || l.starts_with("pub "));
    let library_verb = command == "test";
    match (command, has_defs) {
        ("check", true) => compile_library(file, source),
        (_, true) if !library_verb && declares_tests(source) => {
            Err(format!("error: `{file}` holds tests — `kanso test {file}` runs them\n"))
        }
        // `kanso play` refuses a file holding `pub play` — it takes bare
        // statements — so offering it here sent a reader to a refusal that
        // sent them back. A pub-play module is meant to be imported, and
        // that is what this says instead.
        (_, true) if !library_verb && exports_play(source) => Err(format!(
            "error: `{file}` is a library — nothing to run. it exports \
             `play`: import the module from an entry file, or give the \
             module a main.kso entry\n"
        )),
        (_, true) if !library_verb => Err(format!(
            "error: `{file}` is a library — nothing to run. give the module a \
             main.kso entry, or run its definitions beside their statements \
             with `kanso play`\n"
        )),
        _ if library_verb => compile_library(file, source),
        _ => compile_entry(file, source),
    }
}

thread_local! {
    /// Display paths and the canonical paths they resolve to, both mapped to
    /// one id per file. Two routes to a module reach it under two spellings
    /// and both land on the id the canonical path was given.
    static CANON_IDS: std::cell::RefCell<crate::hash::Map<String, u32>> =
        std::cell::RefCell::new(crate::hash::Map::default());
}

/// The identity `file` cannot carry. Two import routes to one module spell
/// the display path differently — `mid/../shape/shape.kso` against
/// `shape/shape.kso` — and err origins print the spelling the reader typed,
/// so the display path stays as it is and the identity is derived when it is
/// needed. Derived rather than stored, because a field on every declaration
/// makes what compiling costs depend on how many declarations are loaded when
/// the peak falls, which tests/import_order exists to forbid.
fn canon_id(file: &str) -> u32 {
    CANON_IDS.with(|ids| {
        let mut ids = ids.borrow_mut();
        if let Some(id) = ids.get(file) {
            return *id;
        }
        let canon = std::fs::canonicalize(file)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file.to_string());
        let next = ids.len() as u32 + 1;
        let id = *ids.entry(canon).or_insert(next);
        ids.insert(file.to_string(), id);
        id
    })
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
/// The two types division answers with. They are the compiler's because `/`
/// is: an arm of an operator may only match a type its own module defines, so
/// no library could name what a primitive division produces. Declaring them
/// here puts them in every program's scope the way `err` is, and everything
/// downstream — subtype dispatch, rendering, the type checks — reads ordinary
/// declarations and needs to know nothing about arithmetic.
///
/// `divide_by_zero` under `math_failure` under `string`, so a handler may ask
/// for whichever it wants: the specific failure, any math failure, or the
/// reason as text.
pub const MATH_FAILURE: &str = "math_failure";
pub const DIVIDE_BY_ZERO: &str = "divide_by_zero";

/// Whether the program ever asks which failure it has. Naming either type is
/// the only way to tell a math failure from the text it carries: a program
/// that never names one cannot distinguish `10 / 0` from the string it reads
/// as, so division answers the bare string there and the two declarations are
/// not made at all. Dividing is not enough on its own — a benchmark that only
/// renders numbers divides in its innermost loop and asks nothing, and the
/// distinction it cannot observe cost it 3.5%.
fn wants_prelude(program: &ast::Program) -> bool {
    fn in_pattern(p: &ast::Pattern) -> bool {
        match p {
            ast::Pattern::Annotated { ty, .. } => ty == MATH_FAILURE || ty == DIVIDE_BY_ZERO,
            ast::Pattern::Ctor { ty, fields, .. } => {
                ty == MATH_FAILURE || ty == DIVIDE_BY_ZERO || fields.iter().any(in_pattern)
            }
            _ => false,
        }
    }
    fn in_expr(e: &ast::Expr) -> bool {
        match e {
            ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => {
                name == MATH_FAILURE || name == DIVIDE_BY_ZERO
            }
            ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => stmts.iter().any(in_stmt),
            ast::Expr::Guard { cond, early, rest, .. } => {
                in_expr(cond) || in_expr(early) || rest.iter().any(in_stmt)
            }
            _ => any_child(e, in_expr),
        }
    }
    fn in_stmt(s: &ast::Stmt) -> bool {
        match s {
            ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => in_expr(expr),
            ast::Stmt::Set { value, .. } => in_expr(value),
        }
    }
    program.types.iter().any(|t| t.parent.as_deref() == Some(MATH_FAILURE))
        || program.fns.iter().any(|f| f.params.iter().any(in_pattern) || f.body.iter().any(in_stmt))
}

fn install_prelude(program: &mut ast::Program) {
    // Every compilation unit is finished, and merging two of them would
    // otherwise carry two copies of each declaration into one program — which
    // reads downstream as two types sharing a name and emits a switch with the
    // same case twice. Dropping any that are already there makes this
    // idempotent under merge.
    program.types.retain(|t| t.name != MATH_FAILURE && t.name != DIVIDE_BY_ZERO);
    if !wants_prelude(program) {
        return;
    }
    let at = diag::Span::at(0, 0);
    for (name, parent) in [(MATH_FAILURE, "string"), (DIVIDE_BY_ZERO, MATH_FAILURE)] {
        program.types.push(ast::TypeDecl {
            name: name.to_string(),
            is_pub: true,
            span: at,
            synthetic: false,
            origin: None,
            parent: Some(parent.to_string()),
            members: Vec::new(),
            fields: Vec::new(),
        });
    }
}

/// Everything the compiler adds to a parsed program before anything reads it.
fn finish_program(program: &mut ast::Program) {
    install_prelude(program);
    synthesize_getters(program);
}

fn synthesize_getters(program: &mut ast::Program) {
    // Keyed by the type as well as the field. One getter group holds an arm
    // per type that has the field, and skipping on the name alone would let
    // the first type through and leave every later one unreadable.
    // Keyed on the FIELD rather than the getter, and borrowed. `getter_name`
    // and `getter_field` are inverse, so asking about a field costs a
    // `strip_prefix` on each existing getter once instead of a `format!` and a
    // clone on every (type, field) pair the loop below considers.
    let already: crate::hash::Set<(&str, &str)> = program
        .fns
        .iter()
        .filter(|f| f.is_getter())
        .filter_map(|f| match f.params.first() {
            Some(ast::Pattern::Ctor { ty, .. }) => {
                ast::getter_field(&f.name).map(|field| (field, ty.as_str()))
            }
            _ => None,
        })
        .collect();
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
            if already.contains(&(field.as_str(), ty.name.as_str())) {
                continue;
            }
            arms.push(ast::FnDecl {
                name: ast::getter_name(field),
                is_pub: true,
                span: *span,
                params: vec![ast::Pattern::Ctor {
                    ty: ty.name.clone(),
                    fields: bound,
                    whole: None,
                }],
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
/// that is what makes an import's universe readable in its spelling. A local
/// import wears `./` or `../`; ANY bare path is a hako name and is never tried
/// as a local directory, whatever its shape, so neither a sibling nor a
/// subtree called `owner/repo` can shadow one.
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
        // `./` is import syntax rather than part of the name — carrying it into
        // the resolved path would put it in every diagnostic the module raises.
        // `../` is the same syntax and was NOT being taken back out: a module
        // reached as `../deep` from `mid/` named itself `mid/../deep` in every
        // diagnostic it raised and in the hako its errs recorded.
        // Each leading `../` is a step up taken here rather than left in the
        // path: `base` loses a component and the name loses the prefix. A
        // `..` with nothing above it stays, because there is nothing to fold
        // it into. The walk is lexical on purpose — the directory a module is
        // reached THROUGH need not be one it lives under, and asking the
        // filesystem would also make every path absolute, into every
        // diagnostic, which is the thing being fixed.
        let mut here = base.to_path_buf();
        let mut bare = path.strip_prefix("./").unwrap_or(path);
        while let Some(above) = bare.strip_prefix("../") {
            let Some(parent) = here.parent() else { break };
            here = parent.to_path_buf();
            bare = above;
        }
        let relative = here.join(bare);
        let file = here.join(format!("{bare}.kso"));
        // one name, two spellings on disk, and both at once is a question the
        // spelling cannot answer
        if relative.is_dir() && file.is_file() {
            return Err(format!(
                "error: import \"{path}\" names both a directory and a `.kso` file \
                 beside this module — rename one\n"
            ));
        }
        if relative.is_dir() {
            return Ok(relative);
        }
        if file.is_file() {
            return Ok(file);
        }
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a dot-prefixed path names a \
             module beside the importing one, and there is no such directory or \
             `.kso` file there\n"
        ));
    }
    // A bare path names a hako, whatever its shape. It is NOT read as a
    // sibling: the canon is that a local import wears `./` or `../`, so a name
    // beside this module and a name from the cache can never be confused, and
    // a reader knows which they are looking at without leaving the line.
    if !path.contains('/') && base.join(format!("{path}.kso")).is_file() {
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a bare path names a hako, \
             and `{path}.kso` sits beside this module: write \"./{path}\"\n"
        ));
    }
    if !path.contains('/') && base.join(path).is_dir() {
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a bare path names a hako, \
             and `{path}/` sits beside this module: write \"./{path}\"\n"
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
        "error: cannot resolve import \"{path}\" — a bare path names a hako, and \
         it is not in the cache (run `kanso install`)\n"
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

/// Names a pattern binds, borrowed from the program.
///
/// `check.rs` has carried a borrowed twin of this walk for as long as this
/// one has owned its names; this was the outlier. Owning them cost a `String`
/// per bound name — every parameter, every binding, every lambda parameter,
/// every destructured field, across every declaration — for names the
/// program was already holding.
fn bound_in_pattern<'a>(p: &'a ast::Pattern, out: &mut crate::hash::Set<&'a str>) {
    match p {
        ast::Pattern::Var(n, _) => {
            out.insert(n.as_str());
        }
        ast::Pattern::Annotated { name, .. } => {
            out.insert(name.as_str());
        }
        ast::Pattern::Ctor { fields, whole, .. } => {
            if let Some(named) = whole {
                out.insert(named.0.as_str());
            }
            for f in fields {
                bound_in_pattern(f, out);
            }
        }
        ast::Pattern::Keyed { entries, .. } => {
            for e in entries {
                out.insert(e.bind_name.as_str());
            }
        }
        ast::Pattern::IntLit(..)
        | ast::Pattern::StrLit(..)
        | ast::Pattern::Nullary(..)
        | ast::Pattern::Wildcard(..) => {}
    }
}

fn bound_in_stmt<'a>(stmt: &'a ast::Stmt, out: &mut crate::hash::Set<&'a str>) {
    match stmt {
        ast::Stmt::Bind { pattern, expr } => {
            bound_in_pattern(pattern, out);
            bound_in_expr(expr, out);
        }
        ast::Stmt::Expr(e) | ast::Stmt::Set { value: e, .. } => bound_in_expr(e, out),
    }
}

fn bound_in_expr<'a>(e: &'a ast::Expr, out: &mut crate::hash::Set<&'a str>) {
    match e {
        ast::Expr::Lambda { params, body, .. } => {
            for (n, _) in params {
                out.insert(n.as_str());
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
    use crate::hash::{Map as HashMap, Set as HashSet};
    // A synthetic bare alias and the qualified declaration it stands for are
    // the same source position with the same arity, so that tuple indexes
    // them. Finding the twin used to be a scan of every declaration for every
    // synthetic one — quadratic in the program, with a `format!` per pair
    // inside the inner loop.
    let mut at_site: HashMap<(&str, usize, usize, usize), Vec<&str>> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for twin in &program.fns {
        if !twin.synthetic {
            at_site
                .entry((
                    twin.file.as_str(),
                    twin.span.line as usize,
                    twin.span.col as usize,
                    twin.params.len(),
                ))
                .or_default()
                .push(twin.name.as_str());
        }
    }
    let mut by_name: HashMap<&str, (bool, HashSet<&str>)> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for d in &program.fns {
        if d.name.contains('/') {
            continue;
        }
        let entry = by_name.entry(d.name.as_str()).or_insert((true, HashSet::default()));
        entry.0 &= d.synthetic;
        if d.synthetic {
            if let Some(twins) = at_site.get(&(
                d.file.as_str(),
                d.span.line as usize,
                d.span.col as usize,
                d.params.len(),
            )) {
                for name in twins {
                    // `qual/name`, asked without building the needle. A
                    // `format!("/{}", d.name)` here cost a String per
                    // synthetic declaration, which is most of what this pass
                    // allocates and none of what it decides.
                    let qualified =
                        name.strip_suffix(d.name.as_str()).is_some_and(|qual| qual.ends_with('/'));
                    if qualified {
                        entry.1.insert(name);
                    }
                }
            }
        }
    }
    // The escape hatch's names come from the environment rather than the
    // program, so they are owned here and borrowed into `skip` beside the
    // program's own.
    let env_skip: Vec<String> = std::env::var("KANSO_ALIAS_SKIP")
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let mut skip: HashSet<&str> = env_skip.iter().map(String::as_str).collect();
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

fn alias_stmt(stmt: &mut ast::Stmt, aliases: &crate::hash::Map<String, String>) {
    match stmt {
        ast::Stmt::Bind { expr, .. }
        | ast::Stmt::Expr(expr)
        | ast::Stmt::Set { value: expr, .. } => alias_expr(expr, aliases),
    }
}

fn alias_expr(e: &mut ast::Expr, aliases: &crate::hash::Map<String, String>) {
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

/// Rewrite every type reference that resolves to an enrollment clone to
/// the canonical (origin) name: patterns and typeset members are type
/// positions, so no local binding can shadow them. Records then match by
/// one identity no matter which spelling constructed or destructured them.
pub fn canonicalize_types(program: &mut ast::Program) {
    let aliases: crate::hash::Map<String, String> = program
        .types
        .iter()
        .filter_map(|t| t.origin.clone().map(|o| (t.name.clone(), o)))
        .collect();
    if aliases.is_empty() {
        return;
    }
    fn fix(name: &mut String, aliases: &crate::hash::Map<String, String>) {
        if let Some(canon) = aliases.get(name.as_str()) {
            *name = canon.clone();
        }
    }
    fn walk_pattern(p: &mut ast::Pattern, aliases: &crate::hash::Map<String, String>) {
        match p {
            ast::Pattern::Ctor { ty, fields, .. } => {
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
    let mut shorts: crate::hash::Map<String, String> = crate::hash::Map::default();
    let std_names: crate::hash::Set<String> = program
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
    let helpers: crate::hash::Map<String, String> = program
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
fn inline_single_use_chains(body: &mut Vec<ast::Stmt>, shorts: &crate::hash::Map<String, String>) {
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
    for_each_child(e, |child| count_ident_uses(child, name, uses));
}

/// Is the sole use of `name` the collection argument of an enumerable call?
fn coll_arg_use(e: &ast::Expr, name: &str, shorts: &crate::hash::Map<String, String>) -> bool {
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
    any_child(e, |c| coll_arg_use(c, name, shorts))
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
    shorts: &crate::hash::Map<String, String>,
    fold_name: &str,
    helpers: &crate::hash::Map<String, String>,
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
    shorts: &crate::hash::Map<String, String>,
    fold_name: &str,
    helpers: &crate::hash::Map<String, String>,
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
    shorts: &crate::hash::Map<String, String>,
    fold_name: &str,
    helpers: &crate::hash::Map<String, String>,
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

/// Everything a module's imports resolve to: the merged program, whether each
/// qualified name is exported, and the names whose export flag an import took
/// from the module's own declaration.
/// What each import puts within reach, by the bare spelling a caller may
/// write: the qualifier to credit for using it, and — where the name came
/// from somewhere else — who owns it. A re-export puts a dependency's name on
/// the surface without the route, so the name alone says neither.
///
/// A name the import declares itself records no owner. That is the ordinary
/// case, so the inner sets stay empty for a program with no re-exports, and
/// what the front end holds does not grow with a module's own spellings.
type Surfaced = crate::hash::Map<String, crate::hash::Map<String, crate::hash::Set<String>>>;

type Loaded = (ast::Program, crate::hash::Map<String, bool>, crate::hash::Set<String>, Surfaced);

/// The groups syntax names, spelled the same in every module. An arm carries
/// this name because the compiler put it there, not because anybody wrote it,
/// so it is not the module's to rename — the same reason a getter is exempt.
/// An operator is the other shape of that: `<` is not an identifier a module
/// could have declared, and the call site is written by syntax rather than by
/// a name, so prefixing the arm leaves it matching nothing while `a < b` goes
/// on asking for the bare group.
fn is_ambient_group(name: &str) -> bool {
    name == "render/to_string" || is_operator(name)
}

/// The operators an arm may extend, as src/parser.rs accepts them.
pub(crate) fn is_operator(name: &str) -> bool {
    matches!(name, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
}

/// Prefix every top-level name of `dep` with `qual/`, rewriting the module's
/// own references so it still resolves internally, and record which
/// qualified names are pub — the boundary the checker enforces.
fn qualify(
    dep: &mut ast::Program,
    qual: &str,
    exports: &mut crate::hash::Map<String, bool>,
    // GAVEL 51: which DECLARATION claimed each qualified spelling, by
    // interned canonical path. `exports` records only whether a name is pub, and under
    // one module a dependency arrives by every route that reaches it — sealed
    // through a middle module that does not re-export it, open through the
    // importer's own import. Without identity those two are indistinguishable
    // from a genuine shadow, where this module declares a name one of its
    // imports also exports.
    claims: &mut crate::hash::Map<String, u32>,
    // GAVEL 51: a re-exported name arrives under the spelling its owner gave
    // it, so `geo` re-exporting list's `sort` as `order` enrolls `list/order`
    // and nothing in that name says the importer reached it through `geo`.
    // The qualifier is recorded here because this is where it is known.
    surfaced: &mut Surfaced,
    shadowed: &mut crate::hash::Set<String>,
) {
    // A getter's declaration is left bare below, because one group answers a
    // field name across every module. Its calls have to be left bare too: a
    // dependency that reads a field of its own record was qualifying the call
    // to `dep/Get_x` against a declaration that had kept the plain name, and
    // the importer's build asked for a group nothing declares.
    let getters: crate::hash::Set<String> =
        dep.fns.iter().filter(|f| f.is_getter()).map(|f| f.name.clone()).collect();
    // The prelude's types belong to the compiler, not to whichever module is
    // being qualified. Renaming them per module would leave every module with
    // its own `math_failure`, and the one division builds would match none of
    // them.
    // GAVEL 51, Clay 2026-08-17: "one module". A name that already carries a
    // qualification came from this module's own dependency and holds its
    // canonical spelling. Prefixing it again mints a second `shape/blank`
    // under every route that reaches it, and a value built by one matches no
    // arm compiled against the other.
    let owned: crate::hash::Set<String> = check::declared_names(dep)
        .into_iter()
        .filter(|n| !getters.contains(*n))
        .filter(|n| !n.contains('/'))
        .filter(|n| *n != MATH_FAILURE && *n != DIVIDE_BY_ZERO)
        .map(String::from)
        .collect();
    // The prelude's own declarations go, rather than travelling under this
    // module's name: `install_prelude` puts one bare pair back on the merged
    // program, so six modules do not each carry their own `math_failure` for
    // the emitter to write out.
    dep.types.retain(|t| t.name != MATH_FAILURE && t.name != DIVIDE_BY_ZERO);
    // A subtype names its parent, and that name is a type this module owns —
    // so it moves with the rest of them. Gathered before the loop renames
    // anything, because after the first rename the set would not match.
    // GAVEL 51: a name that already carries a qualification is a dependency
    // arriving through this module, and its identity is the canonical path.
    // Left in, a typeset member or a parent spelled `shape/blank` picks up a
    // second prefix and names a type nothing declares.
    let own_types: crate::hash::Set<String> =
        dep.types.iter().map(|t| t.name.clone()).filter(|n| !n.contains('/')).collect();
    for ty in &mut dep.types {
        if ty.name.contains('/') {
            // GAVEL 51: one module, and a qualified name IS its identity, so
            // a second arrival is the same declaration rather than a rival.
            // Its visibility is what the routes grant between them — a sealed
            // route that happens to load later does not close an open one.
            let open = ty.is_pub || exports.get(&ty.name).copied().unwrap_or(false);
            exports.insert(ty.name.clone(), open);
            continue;
        }
        exports.insert(format!("{qual}/{}", ty.name), ty.is_pub);
        ty.name = format!("{qual}/{}", ty.name);
        if let Some(o) = &mut ty.origin {
            *o = format!("{qual}/{o}");
        }
        if let Some(parent) = &mut ty.parent {
            if own_types.contains(parent.as_str()) {
                *parent = format!("{qual}/{parent}");
            }
        }
        // A typeset's membership is a list of type names, and a member this
        // module declares is being renamed under it. A member that is not —
        // `float64`, or a type from somewhere else — keeps its spelling.
        for member in &mut ty.members {
            if own_types.contains(member.as_str()) {
                *member = format!("{qual}/{member}");
            }
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
        // A getter is structural, not owned: one group per field name across
        // every module, reachable without an import. Only its NAME is exempt —
        // the type it matches on belongs to this module and is being renamed
        // under it, so the arm has to follow or it matches nothing.
        if !f.is_getter() && !is_ambient_group(&f.name) {
            // A module that declares a name one of its imports also exports
            // has two claims on one qualified spelling: its own declaration
            // and the bare-enrollment clone of the import. First writer wins,
            // which is the clone whenever the loader reached the import
            // first — so the module's own `pub` read as private from outside.
            // The claim is remembered so the refusal can say what happened.
            // GAVEL 51: an already-qualified name enrolls under the spelling
            // it already has. Composing `{qual}/` onto it would register the
            // route rather than the identity.
            let key = match f.name.contains('/') {
                true => f.name.clone(),
                false => format!("{qual}/{}", f.name),
            };
            let taken = exports.get(&key).copied();
            let same_decl = claims.get(&key).is_some_and(|c| *c == canon_id(&f.file));
            // GAVEL 51: the two-claims rule is about THIS module declaring a
            // name one of its imports also exports. An already-qualified name
            // is not this module's declaration — it is a dependency arriving,
            // and a diamond makes it arrive twice under the identical
            // spelling. Reading the second arrival as a rival claim turned
            // `shape/describe` into an opacity refusal on the very program
            // the ruling exists to make work.
            let own_claim = !f.name.contains('/');
            match (taken, f.synthetic) {
                // The same declaration arriving by a second route. One
                // module, so its visibility is what the importer's routes
                // grant between them: an open route is not vetoed by a sealed
                // one that happened to load first.
                (Some(false), _) if same_decl && f.is_pub => {
                    exports.insert(key.clone(), true);
                }
                (Some(false), false) if f.is_pub && own_claim && !same_decl => {
                    shadowed.insert(key.clone());
                }
                (Some(_), _) => {}
                (None, _) => {
                    exports.insert(key.clone(), f.is_pub);
                    claims.insert(key.clone(), canon_id(&f.file));
                }
            }
            // GAVEL 51: a name that already carries a qualification came
            // from this module's own dependency and keeps its canonical
            // spelling — it still enrolls, it just does not get a second
            // prefix.
            if !f.name.contains('/') {
                f.name = format!("{qual}/{}", f.name);
            }
        }
        let mut bound = Vec::new();
        for p in &mut f.params {
            rewrite_pattern(p, qual, &owned);
            pattern_binds(p, &mut bound);
        }
        for stmt in &mut f.body {
            rewrite_stmt(stmt, qual, &owned, &mut bound);
        }
    }
    // Every name this import puts within reach, by the spelling a caller may
    // write bare. Recorded after the loops above have settled each name, so
    // one walk covers a declaration of this module and one it re-exports
    // alike — the second keeps its owner's qualifier and would otherwise
    // credit that owner for an import the caller never wrote.
    let pubs = dep
        .fns
        .iter()
        .filter(|f| f.is_pub)
        .map(|f| &f.name)
        .chain(dep.types.iter().filter(|t| t.is_pub).map(|t| &t.name));
    for name in pubs {
        let (owner, bare) = match name.rsplit_once('/') {
            Some((owner, bare)) => (owner, bare),
            None => ("", name.as_str()),
        };
        let owners =
            surfaced.entry(bare.to_string()).or_default().entry(qual.to_string()).or_default();
        if !owner.is_empty() && owner != qual {
            owners.insert(owner.to_string());
        }
    }
}

/// `x.name` is the getter applied. The rewrite runs late, after the checks
/// that read field syntax to say something about the field — a type conflict
/// names the read site, and an application would have nothing to point at.
pub fn desugar_field_reads(program: &mut ast::Program) {
    // Inequality rides the same hook: it has to see every module the merge
    // produced, which is exactly what this pass already runs after.
    desugar_inequality(program);
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

/// A type says equality once. Where a module arms `==`, `a != b` becomes the
/// denial of that arm rather than a second builtin, so no type can call two
/// values equal and unequal at the same time. Gated on the arm existing,
/// because a program with no arm should keep paying nothing for the question.
pub fn desugar_inequality(program: &mut ast::Program) {
    let armed = program.fns.iter().any(|d| d.name == "==" && d.params.len() == 2);
    if !armed {
        return;
    }
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            deny_stmt(stmt);
        }
    }
}

fn deny_stmt(stmt: &mut ast::Stmt) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => deny_expr(expr),
        ast::Stmt::Set { value, .. } => deny_expr(value),
    }
}

fn deny_expr(e: &mut ast::Expr) {
    walk_children_mut(e, &mut deny_expr);
    let ast::Expr::BinOp { op: "!=", lhs, rhs, span } = e else {
        return;
    };
    let (span, zero) = (*span, ast::Expr::Int(0.into(), *span));
    let lhs = Box::new(std::mem::replace(lhs.as_mut(), zero.clone()));
    let rhs = Box::new(std::mem::replace(rhs.as_mut(), zero));
    let same = ast::Expr::BinOp { op: "==", lhs, rhs, span };
    *e = ast::Expr::App {
        head: Box::new(ast::Expr::Ident("if".to_string(), span)),
        args: vec![
            same,
            ast::Expr::Ident("false".to_string(), span),
            ast::Expr::Ident("true".to_string(), span),
        ],
        span,
        piped: false,
    };
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
    // The set borrows the program's own names. It used to own them, which cost
    // a String allocation per identifier OCCURRENCE — every mention in the
    // whole program, not every distinct name — and a second for each qualified
    // read's bare half. The keep mask exists so the borrow ends before the
    // retain needs the program mutably.
    let keep: Vec<bool> = {
        let mut mentioned: crate::hash::Set<&str> = crate::hash::Set::default();
        for decl in &program.fns {
            if decl.is_getter() {
                continue;
            }
            for stmt in &decl.body {
                mentions_in_stmt(stmt, &mut mentioned);
            }
        }
        program.fns.iter().map(|d| !d.is_getter() || mentioned.contains(d.name.as_str())).collect()
    };
    let mut mask = keep.into_iter();
    program.fns.retain(|_| mask.next().unwrap_or(true));
}

fn mentions_in_stmt<'a>(stmt: &'a ast::Stmt, out: &mut crate::hash::Set<&'a str>) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => mentions_in_expr(expr, out),
        ast::Stmt::Set { value, .. } => mentions_in_expr(value, out),
    }
}

fn mentions_in_expr<'a>(e: &'a ast::Expr, out: &mut crate::hash::Set<&'a str>) {
    match e {
        ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => {
            out.insert(name.as_str());
            // a qualified read reaches the same getter under its bare name
            if let Some((_, short)) = name.rsplit_once('/') {
                out.insert(short);
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
            for_each_child(e, |child| mentions_in_expr(child, out));
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
        ast::Pattern::Ctor { fields, whole, .. } => {
            if let Some(named) = whole {
                out.push(named.0.clone());
            }
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

fn rewrite_pattern(p: &mut ast::Pattern, qual: &str, owned: &crate::hash::Set<String>) {
    match p {
        ast::Pattern::Ctor { ty, fields, .. } => {
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
    owned: &crate::hash::Set<String>,
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
    owned: &crate::hash::Set<String>,
    bound: &[String],
) {
    let mut inner = bound.to_vec();
    for stmt in stmts {
        rewrite_stmt(stmt, qual, owned, &mut inner);
    }
}

fn rewrite_expr(e: &mut ast::Expr, qual: &str, owned: &crate::hash::Set<String>, bound: &[String]) {
    match e {
        ast::Expr::Guard { cond, early, rest, .. } => {
            rewrite_expr(cond, qual, owned, bound);
            rewrite_expr(early, qual, owned, bound);
            rewrite_scope(rest, qual, owned, bound);
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            rewrite_scope(stmts, qual, owned, bound)
        }
        // `&f` names a function the way a mention does, so it moves with the
        // module the way a mention does. Left behind, the sigil holds a bare
        // name after every declaration has been qualified away from it.
        ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => {
            if owned.contains(name.as_str()) && !bound.iter().any(|b| b == name) {
                *name = format!("{qual}/{name}");
            }
        }
        ast::Expr::Field { base, .. } => rewrite_expr(base, qual, owned, bound),
        // The target names a type the way an annotation does, so it moves
        // with the module the way an annotation's does. Left bare it survives
        // every declaration being qualified away from it, and then names
        // nothing: the interpreter reports that the value is not a `num`
        // while holding one, and both backends refuse the module outright.
        ast::Expr::Upcast { expr, ty, .. } => {
            if owned.contains(ty.as_str()) {
                *ty = format!("{qual}/{ty}");
            }
            rewrite_expr(expr, qual, owned, bound);
        }
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
    exports: &crate::hash::Map<String, bool>,
    renamed: &crate::hash::Set<String>,
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
    let local_types: crate::hash::Set<String> =
        program.types.iter().map(|t| t.name.clone()).collect();
    merge_ambient_arms_with(program, &local_types)
}

/// The same merge for a module spread over files: ownership counts a type
/// defined in any of them.
fn merge_ambient_arms_with(
    program: &mut ast::Program,
    local_types: &crate::hash::Set<String>,
) -> Vec<diag::Diagnostic> {
    let mut diags = Vec::new();
    for decl in &mut program.fns {
        let renders = decl.name == "to_string";
        if !renders && !is_operator(&decl.name) {
            continue;
        }
        // The ownership rule, enforced at the definition site: an arm joining
        // a group this module doesn't own must involve a type it does own.
        // Re-arming a primitive or the sentinels is reserved to the stdlib;
        // wrap the value in your own type instead.
        let owns_a_type = decl.params.iter().any(|p| match p {
            ast::Pattern::Ctor { ty, .. } => local_types.contains(ty),
            ast::Pattern::Annotated { ty, .. } => local_types.contains(ty),
            _ => false,
        });
        if !owns_a_type {
            let what = if renders {
                "rendering of primitives and sentinels is fixed".to_string()
            } else {
                format!("what `{}` means for a primitive is fixed", decl.name)
            };
            diags.push(diag::Diagnostic {
                kind: "ownership",
                message: format!(
                    "an arm of `{}` must match on a type this module defines — {what}; wrap the value in your own type",
                    decl.name
                ),
                span: decl.span,
            });
            continue;
        }
        if renders {
            decl.name = "render/to_string".to_string();
        }
    }
    diags
}

fn ambient_imports(imports: &mut Vec<ast::Import>) {
    if !imports.iter().any(|i| i.path == "std/render") {
        imports.push(ast::Import {
            path: "std/render".to_string(),
            span: diag::Span::at(0, 0),
            alias: None,
            renames: Vec::new(),
        });
    }
}

/// GAVEL 51, Clay 2026-08-17: "one module". A module reached by two import
/// paths contributes its declarations twice, and after qualification both
/// copies carry the same canonical name and the same source position — so
/// they are one declaration, and the second is dropped.
///
/// The key uses `canon`, never `file`. `file` is the display path an err
/// origin prints and the two routes spell it differently; `canon` is the same
/// file by either route. Leaving the source out of the key altogether was
/// tried and is UNSOUND: tests/golden/reexports/torn holds two modules that
/// each declare `strength` on line 1 of their own file, and dropping one as a
/// duplicate of the other turned a torn-call refusal into an answer.
///
/// It includes `synthetic`, because a bare-enrollment clone carries the span
/// of the declaration it was cloned from: dropping one as a duplicate of its
/// own original cost a million-frame accumulating recursion its loop.
fn collapse_diamonds(program: &mut ast::Program) {
    let mut fns = crate::hash::Set::default();
    program.fns.retain(|f| {
        fns.insert((
            canon_id(&f.file),
            f.name.clone(),
            f.params.len(),
            f.span.line,
            f.span.col,
            f.synthetic,
        ))
    });
    let mut types = crate::hash::Set::default();
    program.types.retain(|t| types.insert((t.name.clone(), t.span.line, t.span.col)));
}

/// Load and qualify every imported module, recursively.
fn load_dependencies(
    base: &std::path::Path,
    imports: &[ast::Import],
    visited: &mut crate::hash::Set<std::path::PathBuf>,
) -> Result<Loaded, String> {
    let mut dep_program = ast::Program {
        fns: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
    };
    let mut exports = crate::hash::Map::default();
    let mut claims: crate::hash::Map<String, u32> = crate::hash::Map::default();
    let mut surfaced: Surfaced = crate::hash::Map::default();
    let mut shadowed = crate::hash::Set::default();
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
            "std/os" => Some(("os", &[("os.kso", include_str!("../lib/os/os.kso"))])),
            "std/text" => Some(("text", &[("text.kso", include_str!("../lib/text/text.kso"))])),
            "std/math" => Some(("math", &[("math.kso", include_str!("../lib/math/math.kso"))])),
            "std/bits" => Some(("bits", &[("bits.kso", include_str!("../lib/bits/bits.kso"))])),
            "std/testing" => {
                Some(("testing", &[("testing.kso", include_str!("../lib/testing/testing.kso"))]))
            }
            "std/net" => Some(("net", &[("net.kso", include_str!("../lib/net/net.kso"))])),
            "std/net/http" => {
                Some(("http", &[("http.kso", include_str!("../lib/net/http/http.kso"))]))
            }
            "std/path" => Some(("path", &[("path.kso", include_str!("../lib/path/path.kso"))])),
            "std/sha256" => {
                Some(("sha256", &[("sha256.kso", include_str!("../lib/sha256/sha256.kso"))]))
            }
            "std/regexp" => {
                Some(("regexp", &[("regexp.kso", include_str!("../lib/regexp/regexp.kso"))]))
            }
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
            "./hako" if HAKO_EMBEDDED.with(|c| c.get()) => Some(("hako", HAKO_FILES)),
            _ => None,
        };
        // A handed-in module: the browser has no filesystem, so a program
        // that is a library plus the entry that runs it arrives as sources
        // rather than as files. The canon's `./` is how the entry SPELLS a
        // local import; what it hands over is keyed by the module's name, so
        // the prefix comes off before the lookup.
        let local = path.strip_prefix("./").unwrap_or(path);
        let handed = HANDED_SOURCES.with(|c| c.borrow().get(local).cloned());
        if let Some(files) = handed {
            let borrowed: Vec<(&str, &str)> =
                files.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect();
            let mut dep =
                compile_module_inner(std::path::Path::new(local), false, visited, Some(&borrowed))?;
            qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced, &mut shadowed);
            dep_program.types.extend(dep.types);
            dep_program.fns.extend(dep.fns);
            continue;
        }
        if let Some((short, files)) = embedded {
            // a module importing itself compiles a second copy, so every type
            // gets a twin and a constructor pattern stops matching values the
            // other half built — a confusing failure a long way from here.
            // An entry file is not a member, so its import of the module
            // beside it is the ordinary case.
            if !ENTRY_COMPILE.with(|c| c.get()) && base.file_name().is_some_and(|n| n == short) {
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
            qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced, &mut shadowed);
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
        let same = !ENTRY_COMPILE.with(|c| c.get())
            && std::fs::canonicalize(&dep_dir)
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
        qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced, &mut shadowed);
        dep_program.types.extend(dep.types);
        dep_program.fns.extend(dep.fns);
    }
    // a rename replaces that token's spellings: bare `yours` and
    // `qual/yours` enroll, bare `theirs` never does — the qualified
    // original stays, because the qualified spelling is permanent identity
    let renamed: crate::hash::Set<String> = imports
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
    collapse_diamonds(&mut dep_program);
    Ok((dep_program, exports, shadowed, surfaced))
}

/// A re-export is a use of the import it names.
///
/// A module whose only reason for an import is putting its names back on the
/// surface had that import read as unused, and no spelling would have
/// satisfied the check — the re-export line is the use, and it names the
/// export rather than the module.
fn mark_reexport_quals(
    program: &ast::Program,
    surfaces: impl Fn(&str) -> bool,
    quals: &mut crate::hash::Set<String>,
) {
    for import in &program.imports {
        let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
        let named = program
            .reexports
            .iter()
            .any(|re| re.name == qual || surfaces(&format!("{qual}/{}", re.name)));
        if named {
            quals.insert(qual);
        }
    }
}

/// The qualified door onto a re-exported name.
///
/// A re-export keeps the spelling its owner gave it — geo's rename of list's
/// `sort` is `list/order` — so `order` resolves where `geo/order` names
/// nothing, and a module's own pub gets both doors. The door is a second
/// spelling for one declaration, resolved to the name its owner gave it. A
/// clone would mint a second instance, which gavel 51 forbids.
///
/// Opened only where the qualified spelling is free and one declaration
/// answers it. A module that declares its own `select` beside an import's
/// already owns `mod/select`, and two re-exports landing on one bare name
/// leave the caller nothing to choose between.
fn open_qualified_doors(
    program: &mut ast::Program,
    surfaced: &Surfaced,
    exports: &crate::hash::Map<String, bool>,
) {
    let mut doors = crate::hash::Map::default();
    for (bare, by_qual) in surfaced {
        for (qual, owners) in by_qual {
            let door = format!("{qual}/{bare}");
            // Only a name that answers blocks the door. A dependency's bare
            // enrolment demotes when the module's own surface is drawn, so
            // `mid/blank` exists and is private long before `mid` re-exports
            // `blank` — and what the module published is what the caller
            // asked for.
            if exports.get(&door) == Some(&true) {
                continue;
            }
            let mut answering = owners.iter();
            let (Some(owner), None) = (answering.next(), answering.next()) else {
                continue;
            };
            doors.insert(door, format!("{owner}/{bare}"));
        }
    }
    if doors.is_empty() {
        return;
    }
    for ty in &mut program.types {
        if let Some(parent) = &mut ty.parent {
            door_type(parent, &doors);
        }
        for member in &mut ty.members {
            door_type(member, &doors);
        }
        for (_, members, _) in &mut ty.fields {
            for member in members {
                door_type(member, &doors);
            }
        }
    }
    for decl in &mut program.fns {
        for p in &mut decl.params {
            door_pattern(p, &doors);
        }
        for stmt in &mut decl.body {
            if let ast::Stmt::Bind { pattern, .. } = stmt {
                door_pattern(pattern, &doors);
            }
            door_stmt(stmt, &doors);
            alias_stmt(stmt, &doors);
        }
    }
}

fn door_type(ty: &mut String, doors: &crate::hash::Map<String, String>) {
    if let Some(owner) = doors.get(ty.as_str()) {
        *ty = owner.clone();
    }
}

/// `(v):ty` names a type where nothing else in an expression does.
fn door_stmt(stmt: &mut ast::Stmt, doors: &crate::hash::Map<String, String>) {
    match stmt {
        ast::Stmt::Bind { expr, .. }
        | ast::Stmt::Expr(expr)
        | ast::Stmt::Set { value: expr, .. } => door_expr(expr, doors),
    }
}

fn door_expr(e: &mut ast::Expr, doors: &crate::hash::Map<String, String>) {
    if let ast::Expr::Upcast { ty, .. } = e {
        door_type(ty, doors);
    }
    walk_children_mut(e, &mut |child| door_expr(child, doors));
}

/// A type is named in patterns as well as in expressions, and the door has to
/// answer in both or an arm matches a spelling its caller cannot write.
fn door_pattern(p: &mut ast::Pattern, doors: &crate::hash::Map<String, String>) {
    match p {
        // A user's nullary type parses as a binding — the parser reserves
        // `Nullary` for the built-in names — and only a type can carry a
        // qualifier, because a bound name is always bare.
        ast::Pattern::Var(ty, _) => {
            if let Some(owner) = doors.get(ty.as_str()) {
                *ty = owner.clone();
            }
        }
        ast::Pattern::Ctor { ty, fields, .. } => {
            if let Some(owner) = doors.get(ty.as_str()) {
                *ty = owner.clone();
            }
            for f in fields {
                door_pattern(f, doors);
            }
        }
        ast::Pattern::Annotated { ty, .. } => {
            if let Some(owner) = doors.get(ty.as_str()) {
                *ty = owner.clone();
            }
        }
        _ => {}
    }
}

/// Bare uses count too: a bare `select` that any import exports marks that
/// import used — the bare overload space makes spelling optional, not the
/// dependency.
fn mark_bare_quals(
    program: &ast::Program,
    surfaced: &Surfaced,
    quals: &mut crate::hash::Set<String>,
) {
    // Borrowed from the program: this walks every expression of every
    // declaration and used to keep a `String` per bare identifier OCCURRENCE,
    // for a set that is asked two questions below and dropped.
    let mut bare: crate::hash::Set<&str> = crate::hash::Set::default();
    fn collect<'a>(e: &'a ast::Expr, bare: &mut crate::hash::Set<&'a str>) {
        // `&select` names the import the way `select` does; the sigil holds
        // the name rather than changing it. Left out, a file whose only use of
        // an import was a held name could not be written at all: drop the
        // import and the name does not resolve, keep it and this reads it as
        // unused.
        if let ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) = e {
            if !name.contains('/') {
                bare.insert(name.as_str());
            }
        }
        for_each_child(e, |child| collect(child, bare));
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
    // The qualifier is asked for, never parsed out of the name. A re-export
    // keeps the spelling its owner gave it — geo's rename of list's `sort` is
    // `list/order` — so reading the first segment credits `list` for an import
    // the caller wrote as `geo`, and the caller's import then reads as unused.
    for (name, quals_for) in surfaced {
        if bare.contains(name.as_str()) {
            quals.extend(quals_for.keys().cloned());
        }
    }
    for import in &program.imports {
        let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
        if import.renames.iter().any(|(_, yours)| bare.contains(yours.as_str())) {
            quals.insert(qual);
        }
    }
}

/// Every module qualifier the program references: `json/decode` marks
/// `json` as used, in expressions, patterns, and typeset members alike.
fn used_quals(program: &ast::Program, quals: &mut crate::hash::Set<String>) {
    fn mark(name: &str, quals: &mut crate::hash::Set<String>) {
        if let Some((qual, _)) = name.split_once('/') {
            quals.insert(qual.to_string());
        }
    }
    fn walk_pattern(p: &ast::Pattern, quals: &mut crate::hash::Set<String>) {
        match p {
            ast::Pattern::Ctor { ty, fields, .. } => {
                mark(ty, quals);
                for f in fields {
                    walk_pattern(f, quals);
                }
            }
            ast::Pattern::Annotated { ty, .. } => mark(ty, quals),
            _ => {}
        }
    }
    fn walk_expr(e: &ast::Expr, quals: &mut crate::hash::Set<String>) {
        match e {
            // `&shapes/make` names the module the way a call does: the sigil
            // holds the name rather than changing it.
            ast::Expr::Ident(name, _) | ast::Expr::Partial(name, _) => mark(name, quals),
            // `(x):shapes/num` names `shapes` exactly the way an annotation
            // does. Left out, an import whose only use is a widening target
            // reads as unused and the file cannot be written at all: drop the
            // import and the type does not resolve, keep it and this refuses.
            ast::Expr::Upcast { ty, .. } => mark(ty, quals),
            _ => {}
        }
        for_each_child(e, |child| walk_expr(child, quals));
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
    quals: &crate::hash::Set<String>,
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
        if let ast::Pattern::Ctor { ty, fields, .. } = p {
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
    exports: &crate::hash::Map<String, bool>,
    shadowed: &crate::hash::Set<String>,
    diags: &mut Vec<diag::Diagnostic>,
) {
    fn walk(
        e: &ast::Expr,
        exports: &crate::hash::Map<String, bool>,
        shadowed: &crate::hash::Set<String>,
        diags: &mut Vec<diag::Diagnostic>,
    ) {
        if let ast::Expr::Ident(name, span) = e {
            if let Some(false) = exports.get(name.as_str()) {
                let (module, base) = name.rsplit_once('/').unwrap_or(("", name));
                let shadow = format!(
                    "`{module}` declares `{base}` pub, but an import of `{module}` exports `{base}` too and took the name — rename that import inside `{module}`"
                );
                let private = format!(
                    "`{base}` is private to module `{module}` — only pub names cross an import"
                );
                let said = match shadowed.contains(name.as_str()) {
                    true => shadow,
                    false => private,
                };
                diags.push(diag::Diagnostic::new("opacity", said, *span));
            }
        }
        for_each_child(e, |child| walk(child, exports, shadowed, diags));
    }
    match stmt {
        ast::Stmt::Bind { expr, .. } => walk(expr, exports, shadowed, diags),
        ast::Stmt::Expr(e) => walk(e, exports, shadowed, diags),
        ast::Stmt::Set { value, .. } => walk(value, exports, shadowed, diags),
    }
}

/// The stack-exhaustion message, with a hint when the program holds the one
/// shape that reliably causes it.
///
/// `a . f` hands the continuation over as a closure, so nothing past the
/// current link exists until the link runs. `a >> b` takes `b` as an already
/// evaluated description, so building the first link requires evaluating the
/// second, which requires the third — the whole chain is constructed before any
/// of it runs, and the construction is what exhausts the stack. A reader told
/// only "recursion went deeper than the stack holds" is pointed at the loop,
/// which is the one part of the program that is fine.
///
/// It has to be static to exist at all. The interpreter has a frame guard and
/// can see the recursion; native only sees a SIGSEGV in a child and translates
/// it in the parent; wasm sees a trap nothing recorded. None of the three can
/// work out the cause where it reports, and a function calling itself in the
/// right operand of `>>` is plain in the source, so all three say one sentence.
pub fn stack_exhausted(program: Option<&ast::Program>) -> String {
    let said =
        "error[runtime]: the program ran out of stack: recursion went deeper than the stack holds";
    format!("{said}{}", program.map(stack_hint).unwrap_or_default())
}

/// The hint alone, so the interpreter can append it to the message its own
/// frame guard raises and the two engines say one sentence.
pub fn stack_hint(program: &ast::Program) -> String {
    match seq_recursive_fn(program) {
        Some(name) => format!(
            "\n  `{name}` calls itself in the right side of `>>`, which builds \
             the whole chain before running any of it. `.` hands each step over \
             as it goes."
        ),
        None => String::new(),
    }
}

/// The first function that calls itself inside the right operand of a `>>`.
fn seq_recursive_fn(program: &ast::Program) -> Option<String> {
    for decl in &program.fns {
        for stmt in &decl.body {
            let expr = match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr,
                ast::Stmt::Set { value, .. } => value,
            };
            if seq_calls_self(expr, &decl.name) {
                return Some(decl.name.clone());
            }
        }
    }
    None
}

fn seq_calls_self(e: &ast::Expr, own: &str) -> bool {
    if let ast::Expr::Seq(_, rhs, _) = e {
        if mentions_call(rhs, own) {
            return true;
        }
    }
    any_child(e, |c| seq_calls_self(c, own))
}

fn mentions_call(e: &ast::Expr, own: &str) -> bool {
    if let ast::Expr::App { head, .. } = e {
        if matches!(head.as_ref(), ast::Expr::Ident(n, _) if n == own) {
            return true;
        }
    }
    any_child(e, |c| mentions_call(c, own))
}

/// Every direct sub-expression, handed to `f` as it is found.
///
/// `expr_children` used to answer the same question by building a `Vec`, and
/// the walkers that ask it are the whole front end: 94,784 calls on one
/// `kanso check lib/json`, of which 33,453 returned a non-empty list and so
/// allocated. That was 22.6% of every allocation the compiler made, for lists
/// read once and dropped. Nothing about a walk needs the children gathered
/// first, so this hands them over one at a time and allocates nothing.
pub fn for_each_child<'a>(e: &'a ast::Expr, mut f: impl FnMut(&'a ast::Expr)) {
    walk_children(e, &mut |c| {
        f(c);
        true
    });
}

/// True when any direct sub-expression satisfies `p`, stopping at the first
/// one that does. For the callers that were writing
/// `any_child(e, ..)`; those predicates recurse into whole
/// subtrees, so stopping early is the difference between one match and a full
/// second traversal.
pub fn any_child<'a>(e: &'a ast::Expr, mut p: impl FnMut(&'a ast::Expr) -> bool) -> bool {
    let mut found = false;
    walk_children(e, &mut |c| {
        found = p(c);
        !found
    });
    found
}

/// Every direct sub-expression, in source order, until `f` answers false.
fn walk_children<'a, F: FnMut(&'a ast::Expr) -> bool>(e: &'a ast::Expr, f: &mut F) {
    let stmt_expr = |st: &'a ast::Stmt| match st {
        ast::Stmt::Bind { expr, .. }
        | ast::Stmt::Expr(expr)
        | ast::Stmt::Set { value: expr, .. } => expr,
    };
    match e {
        ast::Expr::Partial(..)
        | ast::Expr::Int(..)
        | ast::Expr::Float(..)
        | ast::Expr::Ident(..) => {}
        ast::Expr::Upcast { expr, .. } => {
            f(expr);
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            if !f(cond) || !f(early) {
                return;
            }
            for st in rest {
                if !f(stmt_expr(st)) {
                    return;
                }
            }
        }
        ast::Expr::Block(stmts, _) | ast::Expr::Build(stmts, _) => {
            for st in stmts {
                if !f(stmt_expr(st)) {
                    return;
                }
            }
        }
        ast::Expr::App { head, args, .. } => {
            if !f(head) {
                return;
            }
            for a in args {
                if !f(a) {
                    return;
                }
            }
        }
        ast::Expr::Field { base, .. } => {
            f(base);
        }
        ast::Expr::Index { base, index, .. } => {
            if f(base) {
                f(index);
            }
        }
        ast::Expr::BinOp { lhs, rhs, .. } | ast::Expr::Join { lhs, rhs, .. } => {
            if f(lhs) {
                f(rhs);
            }
        }
        ast::Expr::Seq(a, b, _) => {
            if f(a) {
                f(b);
            }
        }
        ast::Expr::Lambda { body, .. } => {
            f(body);
        }
        ast::Expr::List(items, _) => {
            for i in items {
                if !f(i) {
                    return;
                }
            }
        }
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                if !f(k) || !f(v) {
                    return;
                }
            }
        }
        ast::Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    if !f(inner) {
                        return;
                    }
                }
            }
        }
    }
}

/// A module is a directory: every .kso file in it shares one namespace.
/// Canonical ordering holds per file; an overload group lives in one file.
pub fn compile_module(dir: &std::path::Path, require_entry: bool) -> Result<ast::Program, String> {
    LOCK.with(|l| *l.borrow_mut() = hako::read_lock(dir));
    let mut visited = crate::hash::Set::default();
    compile_module_root(dir, require_entry, &mut visited)
}

/// hako's own verbs, written in kanso and carried in the binary the way the
/// shipped library is. There is no directory to read: an installed `kanso`
/// has no `hako/` beside it, and the files are the ones in this checkout.
pub fn compile_hako() -> Result<ast::Program, String> {
    LOCK.with(|l| l.borrow_mut().clear());
    HAKO_EMBEDDED.with(|c| c.set(true));
    let built = compile_entry("hako/main.kso", include_str!("../hako/main.kso"));
    HAKO_EMBEDDED.with(|c| c.set(false));
    built
}

thread_local! {
    /// An installed `kanso` has no hako/ checkout beside it, so while the
    /// hako entry compiles, `import "./hako"` resolves to the embedded module
    /// files instead of the filesystem.
    static HAKO_EMBEDDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Whether the current root compile is an entry file, whose imports may
    /// name the module directory it sits in or beside without being cycles.
    static ENTRY_COMPILE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Modules handed in as sources rather than read from disk, by import
    /// path. The browser compiles a program with no filesystem under it, and
    /// a program is a library plus the entry file that runs it.
    static HANDED_SOURCES: std::cell::RefCell<
        crate::hash::Map<String, Vec<(String, String)>>,
    > = std::cell::RefCell::new(crate::hash::Map::default());
}

/// Hands the compiler a module's files under the path an import will name.
/// They live until `forget_sources`, so a caller compiling one program after
/// another says when the last one's sources stop applying.
pub fn hand_source(path: &str, files: Vec<(String, String)>) {
    HANDED_SOURCES.with(|c| c.borrow_mut().insert(path.to_string(), files));
}

pub fn forget_sources() {
    HANDED_SOURCES.with(|c| c.borrow_mut().clear());
}

/// hako's module files, embedded the way the shipped library is.
const HAKO_FILES: &[(&str, &str)] = &[
    ("cache.kso", include_str!("../hako/hako/cache.kso")),
    ("hako.kso", include_str!("../hako/hako/hako.kso")),
    ("install.kso", include_str!("../hako/hako/install.kso")),
    ("lock.kso", include_str!("../hako/hako/lock.kso")),
    ("remote.kso", include_str!("../hako/hako/remote.kso")),
    ("update.kso", include_str!("../hako/hako/update.kso")),
];

/// The root module gets the ambient imports (design/render-plan.md);
/// dependencies never do — deps compile exactly as written.
fn compile_module_root(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut crate::hash::Set<std::path::PathBuf>,
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
/// A lone file's re-exports, held to the rule a module's are held to.
///
/// The two diagnostics `apply_reexport` raises used to be reachable only once
/// a module was merged, so `pub nothing_offers_this` in a single file was
/// accepted and did nothing. A file has imports and a dependency surface like
/// any module, which is everything the check needs.
fn check_reexports(
    program: &ast::Program,
    dep_program: &mut ast::Program,
    import_list: &[ast::Import],
    file: &str,
    source: &str,
) -> Result<(), String> {
    let was_pub: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|f| f.is_pub)
        .map(|f| f.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.is_pub).map(|t| t.name.clone()))
        .collect();
    let import_quals: Vec<String> = import_list
        .iter()
        .map(|i| i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string()))
        .collect();
    for re in &program.reexports {
        apply_reexport(dep_program, &was_pub, &import_quals, re)
            .map_err(|d| diag::render(&[d], file, source))?;
    }
    Ok(())
}

fn apply_reexport(
    dep_program: &mut ast::Program,
    was_pub: &crate::hash::Set<String>,
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
    visited: &mut crate::hash::Set<std::path::PathBuf>,
    embedded: Option<&[(&str, &str)]>,
) -> Result<ast::Program, String> {
    let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canon.clone()) {
        return Err(format!("error: import cycle through {}\n", dir.display()));
    }
    if std::env::var_os("KANSO_PHASES").is_some() {
        eprintln!("load {}", dir.display());
    }
    let loaded = compile_module_loaded(dir, require_entry, visited, embedded);
    visited.remove(&canon);
    loaded
}

fn compile_module_loaded(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut crate::hash::Set<std::path::PathBuf>,
    embedded: Option<&[(&str, &str)]>,
) -> Result<ast::Program, String> {
    let mut sources: Vec<(String, String)> = match embedded {
        Some(files) => files.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect(),
        // A module is a directory of files sharing one namespace, and one file
        // is the smallest of those. `import "./core"` beside `core.kso` reads
        // it the way it reads `core/`, so a runner can sit next to what it
        // runs without either of them becoming a directory first.
        None if dir.is_file() => {
            let file = dir.to_string_lossy().to_string();
            let source = std::fs::read_to_string(dir)
                .map_err(|io| format!("error: cannot read {file}: {io}\n"))?;
            vec![(file, source)]
        }
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
    // An entry file is a program, not a package member (the module-shape
    // gavel): main.kso never joins the merge, whether this directory is the
    // root of a build or somebody's dependency.
    sources
        .retain(|(file, _)| std::path::Path::new(file).file_name().is_none_or(|n| n != "main.kso"));
    if sources.is_empty() {
        return Err(format!(
            "error: {} holds only an entry file — a module is its library \
             files, and main.kso is a program\n",
            dir.display()
        ));
    }
    let mut parsed = Vec::new();
    for (file, source) in sources.drain(..) {
        let lexed = phase::watched("lex", || lexer::lex(&source))
            .map_err(|d| diag::render(&d, &file, &source))?;
        let mut program = phase::watched("parse", || parser::parse(&lexed))
            .map_err(|d| diag::render(&d, &file, &source))?;
        phase::watched("finish_program", || finish_program(&mut program));
        stamp_file(&mut program, &file);
        parsed.push((file, source, program));
    }
    // the module's imports: the union across files, resolved and loaded
    // recursively, each dependency's names qualified by its short name.
    // Sorted by path, because a dependency is compiled on top of everything
    // loaded before it and its own peak stacks on theirs — leaving the order
    // to whichever file happened to name it first makes what the front end
    // holds a property of the file list rather than of the module.
    let mut import_list: Vec<ast::Import> = Vec::new();
    for (_, _, program) in &parsed {
        for import in &program.imports {
            if !import_list.iter().any(|i| i.path == import.path) {
                import_list.push(import.clone());
            }
        }
    }
    import_list.sort_by(|a, b| a.path.cmp(&b.path));
    let root = AMBIENT_ROOT.with(|c| c.replace(false));
    if root {
        ambient_imports(&mut import_list);
    }
    // Arming your own type needs no import, and that holds wherever the type
    // is declared — the merge is per-module, and the ownership rule inside it
    // is what keeps a module out of groups it has no claim on. std/render is
    // the exception because its arms match primitives, which that same rule
    // reserves to the stdlib.
    if !dir.ends_with("render") {
        let module_types: crate::hash::Set<String> =
            parsed.iter().flat_map(|(_, _, p)| p.types.iter().map(|t| t.name.clone())).collect();
        for (file, source, program) in &mut parsed {
            let ownership_diags = merge_ambient_arms_with(program, &module_types);
            if !ownership_diags.is_empty() {
                return Err(diag::render(&ownership_diags, file, source));
            }
        }
    }
    // A module that IS a file resolves its own imports from the directory it
    // sits in. Handing `load_dependencies` the file sent every `./sibling`
    // looking under `a.kso/`, which cannot exist — so a file module could be
    // imported but could never import, and the refusal said the sibling was
    // not there while it sat beside it.
    let base = match dir.is_file() {
        true => dir.parent().unwrap_or(dir),
        false => dir,
    };
    let (mut dep_program, exports, shadowed, surfaced) =
        phase::watched("load_dependencies", || load_dependencies(base, &import_list, visited))?;
    // A module's surface is its own. Dependency pubs demote at this
    // boundary — importers of this module see none of them — and only an
    // explicit re-export puts an imported name back on the surface, as a
    // pub the importer then enrolls like any other.
    let was_pub: crate::hash::Set<String> = dep_program
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
    // Each file names what it uses. Declarations merge across a module's
    // files; imports do not, so a file that leans on a sibling's import is
    // a file whose dependencies are invisible in it.
    let mut import_diags = Vec::new();
    let mut quals = crate::hash::Set::default();
    let mut named = crate::hash::Set::default();
    let mut borrowed: Vec<String> = Vec::new();
    let own = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    for (file, source, program) in &parsed {
        quals.clear();
        used_quals(program, &mut quals);
        named.clear();
        named.extend(
            program
                .imports
                .iter()
                .map(|i| i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string())),
        );
        borrowed.clear();
        borrowed.extend(
            quals.iter().filter(|q| !named.contains(*q) && **q != own && *q != "render").cloned(),
        );
        borrowed.sort();
        mark_bare_quals(program, &surfaced, &mut quals);
        mark_reexport_quals(program, |name| was_pub.contains(name), &mut quals);
        let mut diags = unused_imports(&program.imports, &quals);
        for qual in &borrowed {
            diags.push(diag::Diagnostic::new(
                "import",
                format!(
                    "`{qual}` is not imported here — a module's files share \
                     their declarations, not their imports"
                ),
                diag::Span::at(1, 1),
            ));
        }
        if !diags.is_empty() {
            import_diags.push(diag::render(&diags, file, source));
        }
    }
    if !import_diags.is_empty() {
        return Err(import_diags.join(""));
    }
    // pub bites at the boundary: a qualified reference to a non-pub name.
    // Imports are module-scoped, so use is counted across every file before
    // any one file's import block is called unused. Bare spellings count
    // too — enrollment makes the qualifier optional, not the dependency.
    let mut quals = crate::hash::Set::default();
    for (_, _, program) in &parsed {
        used_quals(program, &mut quals);
        mark_bare_quals(program, &surfaced, &mut quals);
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
    for (_, _, program) in &mut parsed {
        open_qualified_doors(program, &surfaced, &exports);
    }
    let mut all_names = crate::hash::Set::default();
    let mut all_markers = crate::hash::Set::default();
    let mut all_type_names = crate::hash::Set::default();
    for (_, _, program) in &parsed {
        all_names.extend(check::declared_names(program).into_iter().map(String::from));
        all_markers.extend(check::marker_names(program));
        all_type_names.extend(program.types.iter().map(|t| t.name.clone()));
    }
    all_names.extend(check::declared_names(&dep_program).into_iter().map(String::from));
    all_markers.extend(check::marker_names(&dep_program));
    all_type_names.extend(dep_program.types.iter().map(|t| t.name.clone()));
    let shadowable: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    let mut used = crate::hash::Set::default();
    for (file, source, program) in &mut parsed {
        // Every name in the build except this file's own, as references into
        // `all_names`. This used to clone the whole set per file and then
        // remove from the copy — a `String` per name per file, for a set the
        // shadow check only ever reads.
        let extern_globals: crate::hash::Set<&str> = {
            let own = check::declared_names(program);
            all_names.iter().map(String::as_str).filter(|n| !own.contains(n)).collect()
        };
        let mut diags = check::resolve_markers(program, &all_markers);
        diags.extend(check::check_typesets(program, &all_type_names));
        diags.extend(check::check_file_shadow(program, &extern_globals, &mut used, &shadowable));
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        if !diags.is_empty() {
            return Err(diag::render(&diags, file, source));
        }
    }
    for (file, source, program) in &parsed {
        let mut diags = Vec::new();
        for decl in &program.fns {
            for stmt in &decl.body {
                private_uses(stmt, &exports, &shadowed, &mut diags);
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
    let diags = phase::watched("check_merged", || check::check_merged(&merged, require_entry));
    finish_program(&mut merged);
    phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
    phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
    trmc::rewrite(&mut merged);
    inline::inline_builtin_wrappers(&mut merged);
    if !diags.is_empty() {
        // The name an import writes, never the file behind it. A module in a
        // directory read as `(module pkg)` and a module in one file as
        // `(module one.kso)`, from two imports a reader spells the same way —
        // and the page, which has no filesystem and keys a handed module by
        // its import path, said `(module one)` for the second. One rule
        // settles both: the extension is a fact about storage.
        let named = dir.to_string_lossy();
        let named = named.strip_suffix(".kso").unwrap_or(&named);
        let rendered: Vec<String> = diags
            .iter()
            .map(|d| format!("error[{}]: {} (module {named})\n", d.kind, d.message))
            .collect();
        return Err(rendered.join(""));
    }
    phase::watched("canonicalize_types", || canonicalize_types(&mut merged));
    phase::watched("canonicalize_bare_aliases", || canonicalize_bare_aliases(&mut merged));
    phase::watched("hoist_repeated_strings", || hoist_repeated_strings(&mut merged));
    phase::watched("fuse_enumerable", || fuse_enumerable(&mut merged));
    finish_program(&mut merged);
    phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
    phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
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
    for_each_child(e, |child| collect_hoistable(child, found));
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

/// Every direct sub-expression, mutably. NOT the mirror of `for_each_child`
/// it was described as: there is no arm here for a lambda, a block, a build or
/// a guard, so a walk built on this stops at the edge of all four. Its four
/// callers are desugar passes that run before those forms carry anything this
/// would need to reach; a fifth caller that needs the whole tree wants a walk
/// with the missing arms, and `inline::for_each_child_mut` is one.
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
        Expr::Lambda { body, .. } => f(body),
        Expr::Guard { cond, early, rest, .. } => {
            f(cond);
            f(early);
            for stmt in rest {
                f(stmt_expr_mut(stmt));
            }
        }
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                f(stmt_expr_mut(stmt));
            }
        }
        _ => {}
    }
}

fn stmt_expr_mut(s: &mut ast::Stmt) -> &mut ast::Expr {
    match s {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr,
        ast::Stmt::Set { value, .. } => value,
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
        let mut seen: crate::hash::Map<String, usize> = crate::hash::Map::default();
        for (shape, _) in &found {
            *seen.entry(shape.clone()).or_default() += 1;
        }
        let mut repeated: Vec<(String, ast::Expr)> = Vec::new();
        let mut taken: crate::hash::Set<String> = crate::hash::Set::default();
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
