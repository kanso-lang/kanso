use crate::name::Name;
pub mod advisory;
pub mod ahead;
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
pub mod int;
pub mod ir_wasm;
pub mod lexer;
pub mod linear;
pub mod name;
pub mod parser;
pub mod provenance;
pub mod repl;
pub mod trmc;
pub mod wasm;
pub mod wasm_encode;

/// The runtime's C, its parts joined in the order `src/runtime.c` lists them.
/// build.rs does the joining; every reader of the runtime's text in Rust reads
/// it here.
pub const RUNTIME_C: &str = include_str!(concat!(env!("OUT_DIR"), "/runtime.c"));

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
    let built = reaching(Some(source), || compile_entry_inner(file, source));
    ENTRY_COMPILE.with(|c| c.set(false));
    built
}

/// `kanso play`: the relaxed single file — declarations and statements
/// together, no `pub`, no local imports. The verb is the only door, so the
/// form cannot leak into real programs; everything past the parse and the
/// stdlib gate is the entry pipeline unchanged.
pub fn compile_play_file(file: &str, source: &str) -> Result<ast::Program, String> {
    ENTRY_COMPILE.with(|c| c.set(true));
    LOADED_ELSEWHERE.with(|c| c.set(false));
    let built = reaching(Some(source), || compile_play_inner(file, source));
    ENTRY_COMPILE.with(|c| c.set(false));
    built
}

fn compile_play_inner(file: &str, source: &str) -> Result<ast::Program, String> {
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
    compile_parsed_entry(program, file, source)
}

/// Whether the play file compiled last loaded every module it uses from the
/// copies embedded in this binary. When it did, its text and this binary
/// decide the program, and nothing on disk can change it.
pub fn play_file_is_self_contained() -> bool {
    !LOADED_ELSEWHERE.with(|c| c.get())
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
    let (dep_program, exports, surfaced) = load_dependencies(&base, &import_list, &mut visited)?;
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
            private_uses(stmt, &exports, &mut diags);
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
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names, &Default::default()));
    let mut travels = Vec::new();
    diags.extend(check::check_file_shadow(
        &program,
        &extern_globals,
        &shadowable,
        &Default::default(),
        &[],
        &Default::default(),
        &mut travels,
    ));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    rename_travels(&mut program, &travels);
    let mut merged = ast::Program {
        fns: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
        root: String::new(),
    };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    merged.types.extend(program.types);
    merged.fns.extend(program.fns);
    enroll_travelers(&mut merged)?;
    // AHEAD OF THE CHECK, so the whole-program check does not walk the synthetic
    // twins this pass is about to delete -- kanso#1328 did the same on the
    // module path. `canonicalize_types` stays in the success arm: moving it too
    // reads 8,930 instructions WORSE, because this pass deletes the twins before
    // that one would have walked them. The record is what keeps the reorder
    // honest; see `Rewrites`.
    let rewritten =
        phase::watched("canonicalize_bare_aliases", || canonicalize_bare_aliases(&mut merged));
    // One alias fixpoint and one group count for the two passes that read
    // them. The check runs first and the rewrite second over a program
    // neither changes in between, so asking twice was asking the same
    // question of the same program.
    let counts = inline::group_sizes(&merged);
    let builtins = inline::aliases_from(&merged, &counts);
    let wrappers = inline::wrapper_table(&builtins, &counts);
    let merged_diags = check::check_merged_after_aliases_with(&merged, true, &rewritten, builtins);
    inline::apply_wrappers(&mut merged, &wrappers);
    match merged_diags.is_empty() {
        true => {
            phase::watched("canonicalize_types", || canonicalize_types(&mut merged));
            phase::watched("hoist_repeated_strings", || hoist_repeated_strings(&mut merged));
            phase::watched("fuse_enumerable", || fuse_enumerable(&mut merged));
            // Counted here rather than inside the four, because the module
            // path calls the same functions and the compile gates measure it:
            // a bump inside `finish_program` costs every module compile 502
            // instructions for a number only a spec reads. The trade is that
            // this watches the entry group and not a caller somewhere else —
            // tests/rewrite_passes.rs says so.
            rewrite::pass();
            finish_program(&mut merged);
            rewrite::pass();
            phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
            rewrite::pass();
            phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
            rewrite::pass();
            trmc::rewrite(&mut merged);
            merged.root = root_name(std::path::Path::new(file));
            Ok(merged)
        }
        false => Err(diag::render(&merged_diags, file, source)),
    }
}

/// How many times the front end rewrites the whole program.
///
/// `bench/compile_golden_modules.txt` counts what the emitter WROTE and what
/// the inference fixpoint cost getting there. Neither can see a rewrite pass
/// run twice: the passes are idempotent on their own output, so the emitted
/// text is byte-identical, and they are not inference, so rounds and visits
/// hold. `compile_parsed_entry` ran four of them twice for as long as anyone
/// has looked, and every gate in the repository stayed green.
///
/// This is the same instrument `infer::work::passes` is, pointed at the other
/// half of the front end. Counted per invocation rather than per expression
/// because the question is how many times a pass is ASKED for; what one pass
/// costs is already the fixpoint's two numbers.
pub mod rewrite {
    use std::cell::Cell;
    thread_local! {
        static PASSES: Cell<u64> = const { Cell::new(0) };
    }
    pub fn reset() {
        PASSES.with(|c| c.set(0));
    }
    pub fn pass() {
        PASSES.with(|c| c.set(c.get() + 1));
    }
    pub fn passes() -> u64 {
        PASSES.with(Cell::get)
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
    let (mut dep_program, exports, surfaced) = phase::watched("load_dependencies", || {
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
            private_uses(stmt, &exports, &mut diags);
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
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names, &Default::default()));
    let mut travels = Vec::new();
    diags.extend(check::check_file_shadow(
        &program,
        &extern_globals,
        &shadowable,
        &Default::default(),
        &[],
        &Default::default(),
        &mut travels,
    ));
    if drop_unused {
        diags.retain(|d| d.kind != "unused");
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    rename_travels(&mut program, &travels);
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    forget_shades(&mut program.fns);
    // AHEAD OF THE CHECK, the third and last of the three paths to take this
    // shape -- kanso#1328 did it on the module path and kanso#1335 on the
    // entry path, and the reasons are the same here: the whole-program check
    // stops walking the synthetic twins this pass is about to delete.
    // `canonicalize_types` stays where it is; on the entry path moving it too
    // read 8,930 instructions WORSE, because this pass deletes the twins
    // before that one would have walked them.
    //
    // THE RECORD IS NOT AN EXTRA. Two checks in `check_merged` read the head
    // of a call and mean the SPELLING by it, and after this pass the head is a
    // qualified name the source never wrote. Without the record the same
    // reorder takes scripts/module_differential from 29 modules 0 wrong to 2
    // wrong: the opacity check refuses a program that compiles (`m/thing` is
    // foreign, on a bare call from a library file), and the arity diagnostic
    // quotes `m/one` where the source says `one` -- the spelling kanso#1120
    // settled. `Rewrites` is how the check gets told.
    let rewritten = canonicalize_bare_aliases(&mut program);
    let merged_diags = check::check_merged_after_aliases(&program, false, &rewritten);
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    finish_program(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    inline::inline_builtin_wrappers(&mut program);
    program.root = root_name(std::path::Path::new(file));
    Ok(program)
}

/// A lone library file under a library verb (`kanso test`/`check`): parses
/// as a library and loads its imports (plus the ambient module) like any
/// other root compile.
pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {
    reaching(Some(source), || compile_library_beside(file, source, None))
}

/// A test file, under the 2026-10-12 gavel (#1883): an outside consumer of
/// the module it sits beside. It imports that module the way any importer
/// does, so it reads the pub surface and nothing private, and the module's
/// loader never reads the test file, so nothing it declares or imports
/// reaches the module. A test file with no library files beside it has no
/// module to import and tests what it declares itself.
pub fn compile_test_file(file: &str, source: &str) -> Result<ast::Program, String> {
    let beside = module_beside(file);
    // the module is the test file's own directory, which an import from
    // inside it would otherwise refuse as the module importing itself
    ENTRY_COMPILE.with(|c| c.set(true));
    let built = reaching(Some(source), || compile_library_beside(file, source, beside.clone()));
    ENTRY_COMPILE.with(|c| c.set(false));
    built
}

/// Whether a file names a test file: `kanso test` is the only verb that
/// reads one, and a module's loader leaves it out.
pub fn is_test_file(path: &std::path::Path) -> bool {
    path.file_name().is_some_and(|n| n.to_string_lossy().ends_with("_test.kso"))
}

/// The import a test file is compiled with: its own directory, spelled from
/// inside it, when that directory holds library files. An empty parent is
/// the working directory, whose name only the filesystem knows.
fn module_beside(file: &str) -> Option<ast::Import> {
    let dir = std::path::Path::new(file).parent()?;
    let listed = match dir.as_os_str().is_empty() {
        true => std::path::Path::new("."),
        false => dir,
    };
    let mut library = false;
    for entry in std::fs::read_dir(listed).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "kso")
            || entry.file_name() == "main.kso"
            || is_test_file(&path)
        {
            continue;
        }
        // A directory holding a program beside the test file is a folder of
        // programs, the book's sample chapters among them, and there is no
        // module there to import: the test file tests what it declares. A file
        // that parses as nothing at all still counts as the module's, so its
        // syntax error is what the test run reports.
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let Ok(lexed) = lexer::lex(&source) else {
            library = true;
            continue;
        };
        if parser::parse(&lexed).is_err()
            && (parser::parse_entry(&lexed).is_ok() || parser::parse_play(&lexed).is_ok())
        {
            return None;
        }
        library = true;
    }
    if !library {
        return None;
    }
    let name = match dir.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => std::fs::canonicalize(listed).ok()?.file_name()?.to_string_lossy().into_owned(),
    };
    Some(ast::Import {
        path: format!("../{name}"),
        span: diag::Span::at(0, 0),
        alias: None,
        renames: Vec::new(),
    })
}

fn compile_library_beside(
    file: &str,
    source: &str,
    beside: Option<ast::Import>,
) -> Result<ast::Program, String> {
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
    // the module under test is loaded with the file's own imports but never
    // joins `program.imports`, so it cannot read as an import left unused
    import_list.extend(beside);
    ambient_imports(&mut import_list);
    let mut visited = crate::hash::Set::default();
    let (mut dep_program, exports, surfaced) =
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
            private_uses(stmt, &exports, &mut diags);
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
    let mut diags = check::resolve_markers(&mut program, &all_markers);
    diags.extend(check::check_typesets(&program, &all_type_names, &Default::default()));
    let mut travels = Vec::new();
    diags.extend(check::check_file_shadow(
        &program,
        &extern_globals,
        &shadowable,
        &Default::default(),
        &[],
        &Default::default(),
        &mut travels,
    ));
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    if !diags.is_empty() {
        return Err(diag::render(&diags, file, source));
    }
    rename_travels(&mut program, &travels);
    program.types.extend(dep_program.types);
    program.fns.extend(dep_program.fns);
    forget_shades(&mut program.fns);
    // AHEAD OF THE CHECK, the third and last of the three paths to take this
    // shape -- kanso#1328 did it on the module path and kanso#1335 on the
    // entry path, and the reasons are the same here: the whole-program check
    // stops walking the synthetic twins this pass is about to delete.
    // `canonicalize_types` stays where it is; on the entry path moving it too
    // read 8,930 instructions WORSE, because this pass deletes the twins
    // before that one would have walked them.
    //
    // THE RECORD IS NOT AN EXTRA. Two checks in `check_merged` read the head
    // of a call and mean the SPELLING by it, and after this pass the head is a
    // qualified name the source never wrote. Without the record the same
    // reorder takes scripts/module_differential from 29 modules 0 wrong to 2
    // wrong: the opacity check refuses a program that compiles (`m/thing` is
    // foreign, on a bare call from a library file), and the arity diagnostic
    // quotes `m/one` where the source says `one` -- the spelling kanso#1120
    // settled. `Rewrites` is how the check gets told.
    let rewritten = canonicalize_bare_aliases(&mut program);
    let merged_diags = check::check_merged_after_aliases(&program, false, &rewritten);
    if !merged_diags.is_empty() {
        return Err(diag::render(&merged_diags, file, source));
    }
    canonicalize_types(&mut program);
    hoist_repeated_strings(&mut program);
    fuse_enumerable(&mut program);
    finish_program(&mut program);
    desugar_field_reads(&mut program);
    prune_unused_getters(&mut program);
    trmc::rewrite(&mut program);
    inline::inline_builtin_wrappers(&mut program);
    program.root = root_name(std::path::Path::new(file));
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
    // One allocation for the path and a refcount bump per declaration. It was
    // a `String` apiece — 803 copies of about seven distinct values on the
    // fixed corpus — and the sets in linear.rs, beat.rs and codegen.rs that
    // key on (file, line, col) cloned the whole path again on every insert
    // and lookup.
    let file: std::sync::Arc<str> = std::sync::Arc::from(file);
    for decl in &mut program.fns {
        decl.file = std::sync::Arc::clone(&file);
    }
    for decl in &mut program.types {
        decl.file = std::sync::Arc::clone(&file);
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
            ast::Expr::Ident(name, _, _) | ast::Expr::Partial(name, _) => {
                name == MATH_FAILURE || name == DIVIDE_BY_ZERO
            }
            ast::Expr::Block(stmts, _) => stmts.iter().any(in_stmt),
            ast::Expr::Guard { cond, early, rest, .. } => {
                in_expr(cond) || in_expr(early) || rest.iter().any(in_stmt)
            }
            _ => any_child(e, in_expr),
        }
    }
    fn in_stmt(s: &ast::Stmt) -> bool {
        match s {
            ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => in_expr(expr),
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
            file: ast::unstamped(),
        });
    }
}

/// Everything the compiler adds to a parsed program before anything reads it.
fn finish_program(program: &mut ast::Program) {
    install_prelude(program);
    synthesize_getters(program);
    measured_slices_are_slices(program);
}

/// `builtin_slice_in` is std's spelling for a slice whose range the library
/// measured from the value it slices (#1890). The check reads it as a slice
/// that answers no none; every engine after the check runs it as `slice`, so
/// the fused shapes the emitter knows (an append, a decode or a number read
/// of a slice) are the shapes it sees.
fn measured_slices_are_slices(program: &mut ast::Program) {
    fn renamed(e: &mut ast::Expr) {
        if let ast::Expr::Ident(n, _, _) = e {
            if n.as_str() == "builtin_slice_in" {
                *n = "builtin_slice".into();
            }
        }
        for_each_child_mut(e, &mut renamed);
    }
    for f in &mut program.fns {
        for stmt in &mut f.body {
            match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => renamed(expr),
            }
        }
    }
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
            // Asked before the arm is built. A field that already has a getter
            // used to pay for the whole pattern vector and the binder's name
            // first and throw them away on the next line.
            if already.contains(&(field.as_str(), ty.name.as_str())) {
                continue;
            }
            let bound = ty
                .fields
                .iter()
                .enumerate()
                .map(|(at, _)| match at == index {
                    true => ast::Pattern::Var(Name::new(ast::GETTER_BINDER), *span),
                    false => ast::Pattern::Wildcard(*span),
                })
                .collect();
            arms.push(ast::FnDecl {
                name: ast::getter_name(field),
                is_pub: true,
                written_pub: false,
                span: *span,
                params: vec![ast::Pattern::Ctor {
                    ty: Name::new(&ty.name.clone()),
                    fields: bound,
                    whole: None,
                }],
                body: vec![ast::Stmt::Expr(ast::Expr::Ident(
                    Name::new(ast::GETTER_BINDER),
                    *span,
                    crate::ast::Resolution::default(),
                ))],
                file: crate::ast::unstamped(),
                synthetic: false,
                shades: None,
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
        // path: `base` loses a component and the name loses the prefix. A base
        // with no name left to drop (empty, `.`, or already `..`) gains a `..`
        // instead, since `.` is the directory being run from and its parent is
        // `..`. Dropping the `.` would leave the step inside it. A `..` above
        // the root stays in the name, because there is nothing to fold it
        // into. The walk is lexical on purpose — the directory a module is
        // reached THROUGH need not be one it lives under, and asking the
        // filesystem would also make every path absolute, into every
        // diagnostic, which is the thing being fixed.
        let mut here = base.to_path_buf();
        let mut bare = path.strip_prefix("./").unwrap_or(path);
        while let Some(above) = bare.strip_prefix("../") {
            here = match here.components().next_back() {
                Some(std::path::Component::Normal(_)) => {
                    here.parent().map(|p| p.to_path_buf()).unwrap_or_default()
                }
                None | Some(std::path::Component::CurDir) => std::path::PathBuf::from(".."),
                Some(std::path::Component::ParentDir) => here.join(".."),
                Some(std::path::Component::RootDir | std::path::Component::Prefix(_)) => break,
            };
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
    if !ast::has_slash(path) && base.join(format!("{path}.kso")).is_file() {
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a bare path names a hako, \
             and `{path}.kso` sits beside this module: write \"./{path}\"\n"
        ));
    }
    if !ast::has_slash(path) && base.join(path).is_dir() {
        return Err(format!(
            "error: cannot resolve import \"{path}\" — a bare path names a hako, \
             and `{path}/` sits beside this module: write \"./{path}\"\n"
        ));
    }
    let first = ast::split_module(path).map_or(path, |(head, _)| head);
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
    ast::bare_name(path)
}

/// The root module's name, the one an importer would write for it: a file's
/// stem, a directory's name. RULED 2026-08-29, "records print qualified,
/// everywhere": a record prints under it whatever the entry path.
fn root_name(path: &std::path::Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string()
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
        ast::Stmt::Expr(e) => bound_in_expr(e, out),
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
        ast::Expr::Block(stmts, _) => {
            for st in stmts {
                bound_in_stmt(st, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            bound_in_expr(cond, out);
            bound_in_expr(early, out);
            for st in rest.iter() {
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
        ast::Expr::Join { lhs: a, rhs: b, .. } => {
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
/// Where `canonicalize_bare_aliases` rewrote a bare name, and what the program
/// actually said there.
///
/// Two checks in `check_merged` read a call's name and mean the spelling the
/// program used: the arity refusal quotes it, and `foreign_constructions`
/// decides foreignness BY the slash ("the slash IS the foreignness", its own
/// comment). Run the alias pass first and both read a name the pass wrote —
/// the first quotes `m/one` where the source says `one`, and the second refuses
/// a call of an imported function as a construction of the imported type of the
/// same name. This says which sites those are.
///
/// Keyed by line and column, because a Span is exactly that (kanso#1135) and
/// carries no file. A merged program can hold two files with a call at the same
/// position, so a reader confirms the name it holds is the one the pass would
/// have written before trusting the entry.
pub type Rewrites = crate::hash::Map<(u32, u32), Name>;

/// A declaration's source position and arity, which a synthetic bare alias
/// shares with the qualified declaration it stands for.
type Site<'a> = (&'a str, usize, usize, usize);

/// The qualified declarations a bare name could stand for, counted only as far
/// as the one question asked of them: exactly one, or not. A set per name was
/// an allocation for every bare name with a twin, to hold a single entry.
enum Targets<'a> {
    None,
    One(&'a str),
    Many,
}

impl<'a> Targets<'a> {
    fn add(&mut self, name: &'a str) {
        *self = match *self {
            Targets::None => Targets::One(name),
            Targets::One(was) if was == name => Targets::One(was),
            _ => Targets::Many,
        };
    }
}

pub fn canonicalize_bare_aliases(program: &mut ast::Program) -> Rewrites {
    use crate::hash::{Map as HashMap, Set as HashSet};
    // A name can be an alias only if a synthetic declaration carries it, so a
    // program with none has nothing to rewrite and every map below would be
    // built to answer no.
    let synthetic = program.fns.iter().filter(|d| d.synthetic).count();
    if synthetic == 0 {
        return Rewrites::default();
    }
    fn site(d: &ast::FnDecl) -> Site<'_> {
        (&d.file, d.span.line as usize, d.span.col as usize, d.params.len())
    }
    // A synthetic bare alias and the qualified declaration it stands for are
    // the same source position with the same arity, so that tuple indexes
    // them. Only the sites a synthetic declaration stands at are asked about,
    // so only those are kept: indexing every declaration made this the
    // largest thing alive in the front end, for answers nothing read.
    // A site nearly always holds one declaration, so the first is kept in the
    // entry and only a second reaches the vector.
    let mut at_site: HashMap<Site, (Option<&str>, Vec<&str>)> =
        HashMap::with_capacity_and_hasher(synthetic, Default::default());
    for d in program.fns.iter().filter(|d| d.synthetic) {
        at_site.entry(site(d)).or_insert((None, Vec::new()));
    }
    for twin in program.fns.iter().filter(|d| !d.synthetic) {
        if let Some(entry) = at_site.get_mut(&site(twin)) {
            match entry.0 {
                None => entry.0 = Some(twin.name.as_str()),
                Some(_) => entry.1.push(twin.name.as_str()),
            }
        }
    }
    // Likewise only a name some synthetic declaration carries can be an
    // alias; a qualified declaration of the same bare name only disqualifies
    // it.
    let mut by_name: HashMap<&str, (bool, Targets)> =
        HashMap::with_capacity_and_hasher(synthetic, Default::default());
    for d in program.fns.iter().filter(|d| d.synthetic) {
        if ast::has_slash(&d.name) {
            continue;
        }
        let entry = by_name.entry(d.name.as_str()).or_insert((true, Targets::None));
        if let Some((first, rest)) = at_site.get(&site(d)) {
            for name in first.iter().chain(rest) {
                // `qual/name`, asked without building the needle. A
                // `format!("/{}", d.name)` here cost a String per
                // synthetic declaration, which is most of what this pass
                // allocates and none of what it decides.
                let qualified =
                    name.strip_suffix(d.name.as_str()).is_some_and(|qual| qual.ends_with('/'));
                if qualified {
                    entry.1.add(name);
                }
            }
        }
    }
    drop(at_site);
    for d in program.fns.iter().filter(|d| !d.synthetic) {
        if let Some(entry) = by_name.get_mut(d.name.as_str()) {
            entry.0 = false;
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
        .filter_map(|(bare, (all_synthetic, targets))| match targets {
            Targets::One(target) if all_synthetic && !skip.contains(bare) => {
                Some((bare.to_string(), target.to_string()))
            }
            _ => None,
        })
        .collect();
    drop(skip);
    drop(env_skip);
    if aliases.is_empty() {
        return Rewrites::default();
    }
    if std::env::var("KANSO_ALIAS_REPORT").is_ok() {
        let mut v: Vec<_> = aliases.iter().collect();
        v.sort();
        for (b, q) in v {
            eprintln!("alias: {b} -> {q}");
        }
    }
    program.fns.retain(|d| !(d.synthetic && aliases.contains_key(&d.name)));
    let mut wrote = Rewrites::default();
    for d in &mut program.fns {
        for stmt in &mut d.body {
            alias_stmt(stmt, &aliases, &mut wrote);
        }
    }
    wrote
}

fn alias_stmt(
    stmt: &mut ast::Stmt,
    aliases: &crate::hash::Map<String, String>,
    wrote: &mut Rewrites,
) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => alias_expr(expr, aliases, wrote),
    }
}

fn alias_expr(e: &mut ast::Expr, aliases: &crate::hash::Map<String, String>, wrote: &mut Rewrites) {
    match e {
        ast::Expr::Ident(name, sp, _) | ast::Expr::Partial(name, sp) => {
            if let Some(q) = aliases.get(name.as_str()) {
                wrote.insert((sp.line, sp.col), name.clone());
                *name = Name::new(q);
            }
        }
        ast::Expr::Int(..) | ast::Expr::Float(..) => {}
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                alias_expr(k, aliases, wrote);
                alias_expr(v, aliases, wrote);
            }
        }
        ast::Expr::Str(parts, _) => {
            for part in parts {
                if let ast::TemplatePart::Interp(inner) = part {
                    alias_expr(inner, aliases, wrote);
                }
            }
        }
        ast::Expr::List(items, _) => {
            for item in items {
                alias_expr(item, aliases, wrote);
            }
        }
        ast::Expr::App { head, args, .. } => {
            alias_expr(head, aliases, wrote);
            for a in args {
                alias_expr(a, aliases, wrote);
            }
        }
        ast::Expr::Field { base, .. } => alias_expr(base, aliases, wrote),
        ast::Expr::Index { base, index, .. } => {
            alias_expr(base, aliases, wrote);
            alias_expr(index, aliases, wrote);
        }
        ast::Expr::Join { lhs: a, rhs: b, .. } => {
            alias_expr(a, aliases, wrote);
            alias_expr(b, aliases, wrote);
        }
        ast::Expr::Lambda { body, .. } | ast::Expr::Upcast { expr: body, .. } => {
            alias_expr(body, aliases, wrote)
        }
        ast::Expr::BinOp { lhs, rhs, .. } => {
            alias_expr(lhs, aliases, wrote);
            alias_expr(rhs, aliases, wrote);
        }
        ast::Expr::Block(stmts, _) => {
            for st in stmts {
                alias_stmt(st, aliases, wrote);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            alias_expr(cond, aliases, wrote);
            alias_expr(early, aliases, wrote);
            for st in rest.iter_mut() {
                alias_stmt(st, aliases, wrote);
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
    fn fix(name: &mut Name, aliases: &crate::hash::Map<String, String>) {
        if let Some(canon) = aliases.get(name.as_str()) {
            *name = Name::new(canon);
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
                let mut held = Name::new(member);
                fix(&mut held, &aliases);
                *member = held.to_string();
            }
        }
    }
}

include!("lib/fuse.rs");

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

type Loaded = (ast::Program, crate::hash::Map<String, bool>, Surfaced);

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
    // RULED 2026-08-29, "a qualified name is its module's declaration":
    // `dep/join` is dep's own `pub fn join` and nothing else. A bare name
    // this module declares AND one of its imports exports has two kinds of
    // declaration under it — the module's own arms and the bare-enrollment
    // twins of the import's — and that mixed group is the bare overload
    // space, where a bare call inside the module dispatches over both. It
    // moves to a spelling no consumer can write (`dep/~join`, the mark being
    // a character the lexer never makes part of a name), the module's own
    // bare call sites are rewritten into it, and a synthetic clone of each
    // own arm joins it there; the qualified spelling keeps the own arms and
    // only them. Until this, the twin took `dep/join` first and the module's
    // own pub read as private from outside, or — with that refusal lifted —
    // a consumer's `dep/join` reached std's arm under dep's name.
    // An import's twin is a synthetic bare declaration, and so is the loop
    // wrapper trmc writes over a module's own counted recursion, under the
    // module's own name. `synthetic` alone cannot tell them apart: read that
    // way, the wrapper of `weigh` made `weigh` a mixed group in a module that
    // imports nothing, the wrapper went to the bare space as if it were std's,
    // and once the module's own arms stood ahead of it there the bare call
    // took the plain recursion and ran out of stack. What separates them is
    // the file: a twin is cloned from its import's declaration and carries
    // that file, where everything the module writes, and everything a pass
    // writes for it, carries one of the module's own.
    let own_files: crate::hash::Set<std::sync::Arc<str>> = dep
        .fns
        .iter()
        .filter(|f| !f.synthetic && !ast::has_slash(&f.name))
        .map(|f| f.file.clone())
        .collect();
    let is_twin = |f: &ast::FnDecl| f.synthetic && !own_files.contains(&f.file);
    let mut own_bare: crate::hash::Set<&str> = crate::hash::Set::default();
    let mut twin_bare: crate::hash::Set<&str> = crate::hash::Set::default();
    for f in &dep.fns {
        if f.is_getter() || is_ambient_group(&f.name) || ast::has_slash(&f.name) {
            continue;
        }
        match is_twin(f) {
            true => twin_bare.insert(f.name.as_str()),
            false => own_bare.insert(f.name.as_str()),
        };
    }
    let mixed: crate::hash::Set<String> =
        own_bare.iter().filter(|n| twin_bare.contains(*n)).map(|n| n.to_string()).collect();
    // WHICH OF THESE NAMES IS A TYPE, recorded beside the spelling rather than
    // asked for later, because `declared_names` merges the two namespaces and
    // a caller downstream cannot tell them apart from the name alone.
    //
    // A function this module declares beside the bare twin of a type an import
    // exports takes the bare name, as a type the module declares does. The two
    // cannot share one spelling: a call there reached the constructor whenever
    // the counts agreed, so `fn sorted x` beside `import "std/list"` built the
    // import's record. The twin goes, and a type position that names it reads
    // the declaration the twin was cloned from.
    let beaten_twins: crate::hash::Map<String, String> = dep
        .types
        .iter()
        .filter(|t| t.synthetic && !ast::has_slash(&t.name) && own_bare.contains(t.name.as_str()))
        .filter_map(|t| t.origin.clone().map(|o| (t.name.clone(), o)))
        .collect();
    dep.types.retain(|t| !beaten_twins.contains_key(&t.name));
    let type_names: crate::hash::Set<&str> = dep.types.iter().map(|t| t.name.as_str()).collect();
    let owned: crate::hash::Map<Name, Owned> = check::declared_names(dep)
        .into_iter()
        .filter(|n| !getters.contains(*n))
        .filter(|n| !ast::has_slash(n))
        .filter(|n| *n != MATH_FAILURE && *n != DIVIDE_BY_ZERO)
        .map(|n| {
            let spelling = match mixed.contains(n) {
                true => ast::bare_space(qual, n),
                false => ast::qualified(qual, n),
            };
            let ty_spelling = beaten_twins.get(n).cloned();
            let a_type = type_names.contains(n) || beaten_twins.contains_key(n);
            (Name::new(n), Owned { spelling, ty_spelling, a_type })
        })
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
        dep.types.iter().map(|t| t.name.clone()).filter(|n| !ast::has_slash(n)).collect();
    for ty in &mut dep.types {
        if ast::has_slash(&ty.name) {
            // GAVEL 51: one module, and a qualified name IS its identity, so
            // a second arrival is the same declaration rather than a rival.
            // Its visibility is what the routes grant between them — a sealed
            // route that happens to load later does not close an open one.
            let open = ty.is_pub || exports.get(&ty.name).copied().unwrap_or(false);
            exports.insert(ty.name.clone(), open);
            continue;
        }
        let joined = ast::qualified(qual, &ty.name);
        exports.insert(joined.clone(), ty.is_pub);
        ty.name = joined;
        // GAVEL 51: a qualified origin is already the declaration's identity.
        // An import's short-named twin remembers `list/step`, and prefixing it
        // again named a type nothing declares: the interpreter printed
        // `defs/list/step` for std/list's own record and native printed
        // `record`, since no type's slot was left for it.
        if let Some(o) = ty.origin.as_mut().filter(|o| !ast::has_slash(o)) {
            *o = ast::qualified(qual, o);
        }
        if let Some(parent) = &mut ty.parent {
            if own_types.contains(parent.as_str()) {
                *parent = ast::qualified(qual, parent);
            }
        }
        // A typeset's membership is a list of type names, and a member this
        // module declares is being renamed under it. A member that is not —
        // `float64`, or a type from somewhere else — keeps its spelling.
        for member in &mut ty.members {
            if own_types.contains(member.as_str()) {
                *member = ast::qualified(qual, member);
            }
        }
        for (_, members, _) in &mut ty.fields {
            for member in members {
                if let Some(o) = owned.get(member.as_str()).filter(|o| o.a_type) {
                    *member = o.ty_position().to_string();
                }
            }
        }
    }
    let mut bare_clones: Vec<ast::FnDecl> = Vec::new();
    for f in &mut dep.fns {
        // A getter is structural, not owned: one group per field name across
        // every module, reachable without an import. Only its NAME is exempt —
        // the type it matches on belongs to this module and is being renamed
        // under it, so the arm has to follow or it matches nothing.
        let in_mixed = !ast::has_slash(&f.name) && mixed.contains(f.name.as_str());
        if !f.is_getter() && !is_ambient_group(&f.name) {
            // GAVEL 51: an already-qualified name enrolls under the spelling
            // it already has. Composing `{qual}/` onto it would register the
            // route rather than the identity.
            if in_mixed && is_twin(f) {
                // The import's twin: into the bare overload space, and never
                // exported. It holds no claim on the qualified spelling.
                f.name = ast::bare_space(qual, &f.name);
                f.is_pub = false;
                exports.insert(f.name.to_string(), false);
            } else {
                let key = match ast::has_slash(&f.name) {
                    true => f.name.clone(),
                    false => ast::qualified(qual, &f.name),
                };
                let taken = exports.get(&key).copied();
                let same_decl = claims.get(&key).is_some_and(|c| *c == canon_id(&f.file));
                // GAVEL 51: the two-claims rule is about THIS module declaring
                // a name one of its imports also exports. An already-qualified
                // name is not this module's declaration — it is a dependency
                // arriving, and a diamond makes it arrive twice under the
                // identical spelling. Reading the second arrival as a rival
                // claim turned `shape/describe` into an opacity refusal on the
                // very program the ruling exists to make work.
                match taken {
                    // The same declaration arriving by a second route. One
                    // module, so its visibility is what the importer's routes
                    // grant between them: an open route is not vetoed by a
                    // sealed one that happened to load first.
                    Some(false) if same_decl && f.is_pub => {
                        exports.insert(key.clone(), true);
                    }
                    Some(_) => {}
                    None => {
                        exports.insert(key.clone(), f.is_pub);
                        claims.insert(key.clone(), canon_id(&f.file));
                    }
                }
                // GAVEL 51: a name that already carries a qualification came
                // from this module's own dependency and keeps its canonical
                // spelling — it still enrolls, it just does not get a second
                // prefix.
                // The key is already that spelling, so it moves rather than
                // being composed a second time.
                if !ast::has_slash(&f.name) {
                    f.name = key;
                }
            }
        }
        let mut bound = Vec::new();
        for p in &mut f.params {
            rewrite_pattern(p, &owned);
            pattern_binds(p, &mut bound);
        }
        for stmt in &mut f.body {
            rewrite_stmt(stmt, &owned, &mut bound);
        }
        if in_mixed && !is_twin(f) {
            // The module's own arm, once more in the bare overload space, so a
            // bare call inside the module still dispatches over its own arms
            // and the import's together. The clone's body was rewritten with
            // the original's, so its own bare calls already point inside.
            let mut clone = f.clone();
            let short = ast::split_qual(&f.name).map_or(f.name.as_str(), |(_, n)| n);
            clone.name = ast::bare_space(qual, short);
            clone.synthetic = true;
            clone.is_pub = false;
            bare_clones.push(clone);
        }
    }
    for clone in &bare_clones {
        exports.insert(clone.name.to_string(), false);
    }
    // The clones go in AHEAD of the import's twins. Dispatch tries a group's
    // arms in declaration order, and before the ruling the module's own arms
    // stood before the twins `enroll_bare` appended, so a bare call inside the
    // module that both could take reached the module's own. Appending the
    // clones flipped that: kq's `sum`, declared over its own import of the
    // same name, dispatched into std's, and its specs stopped compiling.
    let mut pending: Vec<(String, Vec<ast::FnDecl>)> = Vec::new();
    for clone in bare_clones {
        match pending.iter_mut().find(|(n, _)| *n == clone.name) {
            Some((_, group)) => group.push(clone),
            None => pending.push((clone.name.clone(), vec![clone])),
        }
    }
    let mut fns = Vec::with_capacity(dep.fns.len() + pending.len());
    for f in dep.fns.drain(..) {
        if let Some(at) = pending.iter().position(|(n, _)| *n == f.name) {
            fns.extend(pending.remove(at).1);
        }
        fns.push(f);
    }
    for (_, group) in pending {
        fns.extend(group);
    }
    dep.fns = fns;
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
        let (owner, bare) = match ast::split_qual(name) {
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
    desugar_merges(program);
    synthesize_reader_groups(program);
    for decl in &mut program.fns {
        for stmt in &mut decl.body {
            desugar_stmt(stmt);
        }
    }
}

/// The builtin that copies a record with named fields replaced (#1886).
pub const MERGE: &str = "merge";

/// The field a `merge` key names, when it is a plain string literal.
pub fn merge_key(key: &ast::Expr) -> Option<&str> {
    match key {
        ast::Expr::Str(parts, _) => match parts.as_slice() {
            [ast::TemplatePart::Lit(text)] => Some(text.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// A record this compilation unit declares, as the checker sees it: an
/// imported record is a qualified name or the short clone the loader makes.
pub fn merge_owned(ty: &ast::TypeDecl) -> bool {
    !ty.synthetic && !ast::has_slash(&ty.name) && !ty.fields.is_empty()
}

/// `merge u { "b":2 }` becomes a call to a group with one arm per record the
/// calling module declares that has every field named:
///
/// ```text
/// fn Merge_1b (t Kept0 _ Kept2) Given0 = t Kept0 Given0 Kept2
/// ```
///
/// so every engine runs a dispatch and a construction it already has. One
/// group per module and set of names, shared by every call that names them.
/// A value of a type with no arm meets the dispatch's own refusal.
fn desugar_merges(program: &mut ast::Program) {
    let records: Vec<(String, Vec<String>, crate::diag::Span)> = program
        .types
        .iter()
        .filter(|t| !t.synthetic && !t.fields.is_empty())
        .map(|t| (t.name.clone(), t.fields.iter().map(|(f, _, _)| f.clone()).collect(), t.span))
        .collect();
    let mut made: crate::hash::Set<String> = crate::hash::Set::default();
    let mut arms = Vec::new();
    for decl in &mut program.fns {
        let owner = ast::split_qual(&decl.name).map(|(o, _)| o.to_string()).unwrap_or_default();
        let mut at = |e: &mut ast::Expr| merge_expr(e, &owner, &records, &mut made, &mut arms);
        for stmt in &mut decl.body {
            match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => at(expr),
            }
        }
    }
    program.fns.extend(arms);
}

fn merge_expr(
    e: &mut ast::Expr,
    owner: &str,
    records: &[(String, Vec<String>, crate::diag::Span)],
    made: &mut crate::hash::Set<String>,
    arms: &mut Vec<ast::FnDecl>,
) {
    for_each_child_mut(e, &mut |child| merge_expr(child, owner, records, made, arms));
    let ast::Expr::App { head, args, span, .. } = e else { return };
    let ast::Expr::Ident(name, _, _) = head.as_ref() else { return };
    if name.as_str() != MERGE || args.len() != 2 {
        return;
    }
    let ast::Expr::MapLit(entries, _) = &args[1] else { return };
    let Some(keys) =
        entries.iter().map(|(k, _)| merge_key(k).map(str::to_string)).collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let span = *span;
    // Each name carries its length, so two sets of names never spell the same
    // group, and the result is a word every engine takes as a symbol.
    let bare: String =
        keys.iter().fold("Merge".to_string(), |acc, k| format!("{acc}_{}{k}", k.len()));
    let group = match owner.is_empty() {
        true => bare,
        false => format!("{owner}/{bare}"),
    };
    if made.insert(group.clone()) {
        for (ty, fields, declared) in records {
            let ty_owner = ast::split_qual(ty).map(|(o, _)| o).unwrap_or("");
            if ty_owner != owner || !keys.iter().all(|k| fields.contains(k)) {
                continue;
            }
            let given = |k: &str| keys.iter().position(|g| g == k);
            let pattern = fields
                .iter()
                .enumerate()
                .map(|(at, f)| match given(f) {
                    Some(_) => ast::Pattern::Wildcard(span),
                    None => ast::Pattern::Var(Name::new(&format!("Kept{at}")), span),
                })
                .collect();
            let mut params =
                vec![ast::Pattern::Ctor { ty: Name::new(ty), fields: pattern, whole: None }];
            params.extend(
                (0..keys.len()).map(|at| ast::Pattern::Var(Name::new(&format!("Given{at}")), span)),
            );
            let built = fields
                .iter()
                .enumerate()
                .map(|(at, f)| {
                    let local = match given(f) {
                        Some(g) => format!("Given{g}"),
                        None => format!("Kept{at}"),
                    };
                    ast::Expr::Ident(Name::new(&local), span, crate::ast::Resolution::default())
                })
                .collect();
            let body = ast::Expr::App {
                head: Box::new(ast::Expr::Ident(
                    Name::new(ty),
                    span,
                    crate::ast::Resolution::default(),
                )),
                args: built,
                span,
                piped: false,
            };
            // Each arm carries its type's span: arms of one group that share a
            // span read as one declaration made twice when a module is merged.
            arms.push(ast::FnDecl {
                name: group.clone(),
                is_pub: true,
                written_pub: false,
                span: *declared,
                params,
                body: vec![ast::Stmt::Expr(body)],
                file: crate::ast::unstamped(),
                synthetic: false,
                shades: None,
            });
        }
    }
    let ast::Expr::App { args, .. } = e else { return };
    let map = args.pop().expect("two arguments");
    let ast::Expr::MapLit(entries, _) = map else { return };
    args.extend(entries.into_iter().map(|(_, v)| v));
    *head_of(e) = ast::Expr::Ident(Name::new(&group), span, crate::ast::Resolution::default());
}

fn head_of(e: &mut ast::Expr) -> &mut ast::Expr {
    match e {
        ast::Expr::App { head, .. } => head.as_mut(),
        _ => unreachable!("a merge is an application"),
    }
}

/// An err answers `.reason`, `.cause` and `.origin` whatever the program
/// declares, and a read desugars to the getter's name, so the group has to
/// exist when no record declares the field. This arm makes it exist. The
/// reader hole at every dispatcher's entry answers an err before any arm is
/// tried, so the arm is never reached by one; a record reaching it matches
/// nothing and gets the field error every other getter gives.
///
/// Once, over the merged program: synthesised per module, every module
/// carried an identical arm and the merge refused them as overlapping.
fn synthesize_reader_groups(program: &mut ast::Program) {
    for field in ast::ERR_READERS {
        let name = ast::getter_name(field);
        if program.fns.iter().any(|f| f.name == name) {
            continue;
        }
        let span = crate::diag::Span::at(0, 0);
        program.fns.push(ast::FnDecl {
            name,
            is_pub: true,
            written_pub: false,
            span,
            params: vec![ast::Pattern::Ctor {
                ty: Name::new("err"),
                fields: vec![ast::Pattern::Var(Name::new(ast::GETTER_BINDER), span)],
                whole: None,
            }],
            body: vec![ast::Stmt::Expr(ast::Expr::Ident(
                Name::new(ast::GETTER_BINDER),
                span,
                crate::ast::Resolution::default(),
            ))],
            file: crate::ast::unstamped(),
            synthetic: false,
            shades: None,
        });
    }
}

fn desugar_stmt(stmt: &mut ast::Stmt) {
    match stmt {
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => desugar_expr(expr),
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
    }
}

fn deny_expr(e: &mut ast::Expr) {
    for_each_child_mut(e, &mut deny_expr);
    let ast::Expr::BinOp { op: "!=", lhs, rhs, span } = e else {
        return;
    };
    let (span, zero) = (*span, ast::Expr::Int(0.into(), *span));
    let lhs = Box::new(std::mem::replace(lhs.as_mut(), zero.clone()));
    let rhs = Box::new(std::mem::replace(rhs.as_mut(), zero));
    let same = ast::Expr::BinOp { op: "==", lhs, rhs, span };
    *e = ast::Expr::App {
        head: Box::new(ast::Expr::Ident(Name::new("if"), span, crate::ast::Resolution::default())),
        args: vec![
            same,
            ast::Expr::Ident(Name::new("false"), span, crate::ast::Resolution::default()),
            ast::Expr::Ident(Name::new("true"), span, crate::ast::Resolution::default()),
        ],
        span,
        piped: false,
    };
}

fn desugar_expr(e: &mut ast::Expr) {
    if let ast::Expr::Field { base, name, span } = e {
        desugar_expr(base);
        let head = ast::Expr::Ident(
            Name::new(&ast::getter_name(name)),
            *span,
            crate::ast::Resolution::default(),
        );
        let base = std::mem::replace(base.as_mut(), ast::Expr::Int(0.into(), *span));
        *e = ast::Expr::App { head: Box::new(head), args: vec![base], span: *span, piped: false };
        return;
    }
    for_each_child_mut(e, &mut desugar_expr);
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
    }
}

fn mentions_in_expr<'a>(e: &'a ast::Expr, out: &mut crate::hash::Set<&'a str>) {
    match e {
        ast::Expr::Ident(name, _, _) | ast::Expr::Partial(name, _) => {
            // Only a name that could BE a getter's is worth remembering, and
            // every getter is synthesised in one place as `Get_{field}` — the
            // binder that makes `is_getter` true is one no source can spell,
            // so there is no other way for one to exist. The prefix test is
            // four bytes; the insert it replaces was a hash of the whole name,
            // and eleven thousand of the twelve thousand occurrences in
            // lib/json are of names no getter can have.
            let short = ast::split_qual(name).map(|(_, s)| s);
            if !short.unwrap_or(name).starts_with("Get_") {
                return;
            }
            out.insert(name.as_str());
            // a qualified read reaches the same getter under its bare name
            if let Some(short) = short {
                out.insert(short);
            }
        }
        ast::Expr::Block(stmts, _) => {
            for s in stmts {
                mentions_in_stmt(s, out);
            }
        }
        ast::Expr::Guard { cond, early, rest, .. } => {
            mentions_in_expr(cond, out);
            mentions_in_expr(early, out);
            for s in rest.iter() {
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
// The names are `Name`s so that a short one is copied rather than allocated:
// a `String` per binding, and another per binding every time a scope cloned
// the list, was the largest source of allocations in `qualify`.
fn pattern_binds(p: &ast::Pattern, out: &mut Vec<Name>) {
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
            out.extend(entries.iter().map(|e| Name::new(&e.bind_name)))
        }
        _ => {}
    }
}

/// A spelling this module owns, and whether the name it replaces is a TYPE.
///
/// Both namespaces live in one map because a constructor is CALLED by its
/// type's name, so a value position has to be able to find a type here. A type
/// position must not be able to find a function, and until 2026-09-16 it
/// could: `check::declared_names` hands back one flat set of type names and
/// function names, `qualify` built this map from it, and a module declaring
/// `fn entry` beside an `import "std/json"` rewrote the bare `entry` TYPE in a
/// constructor pattern to its own `ec3/entry`. `kanso check` said ok, the
/// interpreter ran the program correctly, and the native backend answered
/// `unknown type` on a name no type table has ever held.
#[derive(Clone)]
struct Owned {
    spelling: String,
    /// Where a type position goes when it is not the spelling: the declaration
    /// an import's type twin was cloned from, when a function of this module
    /// took the twin's name. Held only then, so the common case allocates
    /// nothing more than it did.
    ty_spelling: Option<String>,
    a_type: bool,
}

impl Owned {
    /// The spelling a type position takes.
    fn ty_position(&self) -> &str {
        self.ty_spelling.as_deref().unwrap_or(&self.spelling)
    }
}

fn rewrite_pattern(p: &mut ast::Pattern, owned: &crate::hash::Map<Name, Owned>) {
    match p {
        ast::Pattern::Ctor { ty, fields, .. } => {
            if let Some(o) = owned.get(ty.as_str()).filter(|o| o.a_type) {
                *ty = Name::new(o.ty_position());
            }
            for f in fields {
                rewrite_pattern(f, owned);
            }
        }
        ast::Pattern::Annotated { ty, .. } if owned.get(ty.as_str()).is_some_and(|o| o.a_type) => {
            *ty = Name::new(owned[ty.as_str()].ty_position());
        }
        _ => {}
    }
}

fn rewrite_stmt(
    stmt: &mut ast::Stmt,
    owned: &crate::hash::Map<Name, Owned>,
    bound: &mut Vec<Name>,
) {
    match stmt {
        ast::Stmt::Bind { expr, pattern } => {
            rewrite_pattern(pattern, owned);
            rewrite_expr(expr, owned, bound);
            pattern_binds(pattern, bound);
        }
        ast::Stmt::Expr(e) => rewrite_expr(e, owned, bound),
    }
}

fn rewrite_scope(stmts: &mut [ast::Stmt], owned: &crate::hash::Map<Name, Owned>, bound: &[Name]) {
    let mut inner = bound.to_vec();
    for stmt in stmts {
        rewrite_stmt(stmt, owned, &mut inner);
    }
}

fn rewrite_expr(e: &mut ast::Expr, owned: &crate::hash::Map<Name, Owned>, bound: &[Name]) {
    match e {
        ast::Expr::Guard { cond, early, rest, .. } => {
            rewrite_expr(cond, owned, bound);
            rewrite_expr(early, owned, bound);
            rewrite_scope(rest, owned, bound);
        }
        ast::Expr::Block(stmts, _) => rewrite_scope(stmts, owned, bound),
        // `&f` names a function the way a mention does, so it moves with the
        // module the way a mention does. Left behind, the sigil holds a bare
        // name after every declaration has been qualified away from it.
        ast::Expr::Ident(name, _, _) | ast::Expr::Partial(name, _) => {
            if let Some(o) = owned.get(name.as_str()) {
                if !bound.iter().any(|b| b == name) {
                    *name = Name::new(&o.spelling);
                }
            }
        }
        ast::Expr::Field { base, .. } => rewrite_expr(base, owned, bound),
        // The target names a type the way an annotation does, so it moves
        // with the module the way an annotation's does. Left bare it survives
        // every declaration being qualified away from it, and then names
        // nothing: the interpreter reports that the value is not a `num`
        // while holding one, and both backends refuse the module outright.
        ast::Expr::Upcast { expr, ty, .. } => {
            if let Some(o) = owned.get(ty.as_str()).filter(|o| o.a_type) {
                *ty = o.ty_position().to_string();
            }
            rewrite_expr(expr, owned, bound);
        }
        ast::Expr::App { head, args, .. } => {
            rewrite_expr(head, owned, bound);
            for a in args {
                rewrite_expr(a, owned, bound);
            }
        }
        ast::Expr::Index { base, index, .. } => {
            rewrite_expr(base, owned, bound);
            rewrite_expr(index, owned, bound);
        }
        ast::Expr::BinOp { lhs, rhs, .. } | ast::Expr::Join { lhs, rhs, .. } => {
            rewrite_expr(lhs, owned, bound);
            rewrite_expr(rhs, owned, bound);
        }
        ast::Expr::Lambda { params, body, .. } => {
            let mut inner = bound.to_vec();
            inner.extend(params.iter().map(|(n, _)| Name::new(n)));
            rewrite_expr(body, owned, &inner);
        }
        ast::Expr::List(items, _) => {
            for i in items {
                rewrite_expr(i, owned, bound);
            }
        }
        ast::Expr::MapLit(pairs, _) => {
            for (k, v) in pairs {
                rewrite_expr(k, owned, bound);
                rewrite_expr(v, owned, bound);
            }
        }
        ast::Expr::Str(parts, _) => {
            for p in parts {
                if let ast::TemplatePart::Interp(inner) = p {
                    rewrite_expr(inner, owned, bound);
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
            if let Some((_, short)) = ast::split_qual(&f.name) {
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
            if let Some((_, short)) = ast::split_qual(&t.name) {
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
            ast::Pattern::Ctor { ty, .. } => local_types.contains(ty.as_str()),
            ast::Pattern::Annotated { ty, .. } => local_types.contains(ty.as_str()),
            _ => false,
        });
        if !owns_a_type {
            let what = if renders {
                "rendering of primitives and sentinels is fixed".to_string()
            } else {
                format!("what `{}` means for a primitive is fixed", decl.name)
            };
            diags.push(diag::Diagnostic::new(
                "ownership",
                format!(
                    "an arm of `{}` must match on a type this module defines — {what}; wrap the value in your own type",
                    decl.name
                ),
                decl.span,
            ));
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
    // The keys BORROW the names, the way `prune_unused_getters` does and for
    // the same reason: a `retain` needs the program mutably, so a set holding
    // `&str` into it cannot outlive the closure, and the mask is what carries
    // the answer across. Owning them cost a `String` allocation per
    // declaration and per type, for keys that are read once and dropped.
    let keep: Vec<bool> = {
        let mut fns: crate::hash::Set<(u32, &str, usize, u32, u32, bool)> =
            crate::hash::Set::default();
        program
            .fns
            .iter()
            .map(|f| {
                fns.insert((
                    canon_id(&f.file),
                    f.name.as_str(),
                    f.params.len(),
                    f.span.line,
                    f.span.col,
                    f.synthetic,
                ))
            })
            .collect()
    };
    let mut mask = keep.into_iter();
    program.fns.retain(|_| mask.next().unwrap_or(true));
    let keep: Vec<bool> = {
        let mut types: crate::hash::Set<(&str, u32, u32)> = crate::hash::Set::default();
        program
            .types
            .iter()
            .map(|t| types.insert((t.name.as_str(), t.span.line, t.span.col)))
            .collect()
    };
    let mut mask = keep.into_iter();
    program.types.retain(|_| mask.next().unwrap_or(true));
}

include!("lib/imports.rs");

/// The stack-exhaustion message, one sentence on every engine.
pub fn stack_exhausted() -> String {
    "error[runtime]: the program ran out of stack: recursion went deeper than the stack holds"
        .to_string()
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

/// Every direct sub-expression, mutably, and the mirror of `for_each_child`:
/// it has an arm for every form that holds an expression, the lambda, block
/// and guard included. `substitute_ident` relies on the walk reaching a
/// guard's statements.
pub fn for_each_child_mut(e: &mut ast::Expr, f: &mut dyn FnMut(&mut ast::Expr)) {
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
            for stmt in rest.iter_mut() {
                f(stmt_expr_mut(stmt));
            }
        }
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                f(stmt_expr_mut(stmt));
            }
        }
        _ => {}
    }
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
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => expr,
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
            for st in rest.iter() {
                if !f(stmt_expr(st)) {
                    return;
                }
            }
        }
        ast::Expr::Block(stmts, _) => {
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

include!("lib/modules.rs");

include!("lib/travel.rs");

include!("lib/hoist.rs");
