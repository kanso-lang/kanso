// Modules: the list the binary ships, the loader that reads a directory or
// the embedded copy, re-exports, and the compile that merges a module's files
// into one program. lib.rs reads this file with include!, so the items stay
// in the crate root and the compiler builds to the same instructions it did
// when they sat in lib.rs (kanso#1867). cargo fmt does not reach an included
// file, so CI's format job checks the files under src/'s subdirectories
// itself.

/// A module is a directory: every .kso file in it shares one namespace.
/// Canonical ordering holds per file; an overload group lives in one file.
/// Every module the loader serves from the copy embedded in this binary.
/// `tests/every_shipped_module_checks_clean.rs` holds this list to the
/// loader's table and compiles each with its merged check asked.
pub const SHIPPED_MODULES: [&str; 15] = [
    "std/render",
    "std/list",
    "std/time",
    "std/io",
    "std/os",
    "std/text",
    "std/math",
    "std/bits",
    "std/testing",
    "std/net",
    "std/net/http",
    "std/path",
    "std/sha256",
    "std/regexp",
    "std/json",
];

/// Compiles one shipped module the way an import loads it, with the merged
/// check asked rather than skipped. An import skips it because the answer is
/// fixed when the binary is built; this is where that answer is checked.
pub fn check_shipped(path: &str) -> Result<(), String> {
    let outer = CHECK_SHIPPED.with(|c| c.replace(true));
    let import = ast::Import {
        path: path.to_string(),
        span: diag::Span::at(0, 0),
        alias: None,
        renames: Vec::new(),
    };
    let mut visited = crate::hash::Set::default();
    let loaded = load_dependencies(std::path::Path::new("."), &[import], &mut visited);
    CHECK_SHIPPED.with(|c| c.set(outer));
    loaded.map(|_| ())
}

pub fn compile_module(dir: &std::path::Path, require_entry: bool) -> Result<ast::Program, String> {
    LOCK.with(|l| *l.borrow_mut() = hako::read_lock(dir));
    let mut visited = crate::hash::Set::default();
    let mut program = compile_module_root(dir, require_entry, &mut visited)?;
    if !require_entry {
        forget_shades(&mut program.fns);
    }
    Ok(program)
}

/// hako's own verbs, written in kanso and carried in the binary the way the
/// shipped library is. There is no directory to read: an installed `kanso`
/// has no `hako/` beside it, and the files are the ones in this checkout.
pub fn compile_hako() -> Result<ast::Program, String> {
    LOCK.with(|l| l.borrow_mut().clear());
    HAKO_EMBEDDED.with(|c| c.set(true));
    let built = compile_entry("hako/main.kso", include_str!("../../hako/main.kso"));
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
    /// Whether the module being compiled is the shipped library's, loaded
    /// from the copy embedded in this binary. Set around a `std/` import only.
    static SHIPPED_STD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Set by `check_shipped`, which asks the check the loader skips.
    static CHECK_SHIPPED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Whether a module of the program being compiled came from anywhere but
    /// this binary: a directory on disk, or sources handed in. A play file
    /// that loaded nothing else is fixed by its own text and this binary,
    /// which is what lets `kanso play` key its build on the text.
    static LOADED_ELSEWHERE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
    ("cache.kso", include_str!("../../hako/hako/cache.kso")),
    ("hako.kso", include_str!("../../hako/hako/hako.kso")),
    ("install.kso", include_str!("../../hako/hako/install.kso")),
    ("lock.kso", include_str!("../../hako/hako/lock.kso")),
    ("remote.kso", include_str!("../../hako/hako/remote.kso")),
    ("update.kso", include_str!("../../hako/hako/update.kso")),
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

// The files a module is made of, in the order they are merged. Called once to
// compile the module, and a second time only if the merged check raises: a
// located diagnostic needs the text of the file it is about, and holding that
// text across the merge for the compiles that never fail costs the front end
// bytes at its peak. The error path pays instead, where nothing is hot.
fn module_sources(
    dir: &std::path::Path,
    embedded: Option<&[(&str, &str)]>,
) -> Result<Vec<(String, String)>, String> {
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
    // A test file is an outside consumer of the module (the 2026-10-12
    // gavel): `kanso test` compiles it as an importer, and the module itself
    // never reads it. A file module is the file it names, test or not.
    if embedded.is_none() && dir.is_dir() {
        let before = sources.len();
        sources.retain(|(file, _)| !crate::is_test_file(std::path::Path::new(file)));
        if sources.is_empty() && before > 0 {
            return Err(format!(
                "error: {} holds only test files — a module is its library \
                 files, and a test file imports the module beside it\n",
                dir.display()
            ));
        }
    }
    if sources.is_empty() {
        return Err(format!(
            "error: {} holds only an entry file — a module is its library \
             files, and main.kso is a program\n",
            dir.display()
        ));
    }
    Ok(sources)
}

fn compile_module_loaded(
    dir: &std::path::Path,
    require_entry: bool,
    visited: &mut crate::hash::Set<std::path::PathBuf>,
    embedded: Option<&[(&str, &str)]>,
) -> Result<ast::Program, String> {
    let mut sources = module_sources(dir, embedded)?;
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
    let (mut dep_program, exports, surfaced) =
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
    // Names rather than Strings: this set is only ever read back as `&str`,
    // and a short name is held inline where a String was an allocation for
    // every declaration in the build.
    let mut all_names: crate::hash::Set<Name> = crate::hash::Set::default();
    let mut all_markers = crate::hash::Set::default();
    // The module's own types and the imports' are kept apart: an annotation
    // may name a type any file of the module declares, and a typeset member
    // may name either.
    let mut module_types = crate::hash::Set::default();
    for (_, _, program) in &parsed {
        all_names.extend(check::declared_names(program).into_iter().map(Name::new));
        all_markers.extend(check::marker_names(program));
        module_types.extend(program.types.iter().map(|t| t.name.clone()));
    }
    all_names.extend(check::declared_names(&dep_program).into_iter().map(Name::new));
    all_markers.extend(check::marker_names(&dep_program));
    let dep_types: crate::hash::Set<String> =
        dep_program.types.iter().map(|t| t.name.clone()).collect();
    let shadowable: crate::hash::Set<String> = dep_program
        .fns
        .iter()
        .filter(|d| d.synthetic)
        .map(|d| d.name.clone())
        .chain(dep_program.types.iter().filter(|t| t.synthetic).map(|t| t.name.clone()))
        .collect();
    // Markers are resolved in every file before any file is checked, so the
    // checks can read the module's type declarations while they read one
    // file. A pattern names a record the next file declares as readily as
    // one declared above it, and until 2026-09-29 the check that a
    // constructor pattern can match at all asked only the file's own types.
    let marker_diags: Vec<Vec<diag::Diagnostic>> = parsed
        .iter_mut()
        .map(|(_, _, program)| check::resolve_markers(program, &all_markers))
        .collect();
    let module_decls: Vec<&ast::TypeDecl> =
        parsed.iter().flat_map(|(_, _, program)| &program.types).collect();
    // A file names only what it can see (2026-10-12): what the module's files
    // declare, and what this file's own imports bring. A bare name that only
    // a sibling's import surfaces is hidden from it. A module of one file
    // sees exactly its imports, so only a module of several asks.
    let declared_here: crate::hash::Set<&str> = match parsed.len() {
        1 => crate::hash::Set::default(),
        _ => parsed.iter().flat_map(|(_, _, p)| check::declared_names(p)).collect(),
    };
    // The functions the module's other files declare: a call of one on a
    // value a function was handed is shaded, as a call of the file's own is.
    // A module of one file has no others.
    let module_fns: crate::hash::Set<&str> = match parsed.len() {
        1 => crate::hash::Set::default(),
        _ => parsed.iter().flat_map(|(_, _, p)| p.fns.iter().map(|f| f.name.as_str())).collect(),
    };
    let mut file_travels: Vec<Vec<check::Travel>> = Vec::with_capacity(parsed.len());
    for ((file, source, program), mut diags) in parsed.iter().zip(marker_diags) {
        let hidden = match parsed.len() {
            1 => crate::hash::Map::default(),
            _ => hidden_from(program, &parsed, &surfaced, &declared_here),
        };
        // Every name in the build except this file's own, as references into
        // `all_names`. This used to clone the whole set per file and then
        // remove from the copy — a `String` per name per file, for a set the
        // shadow check only ever reads.
        let extern_globals: crate::hash::Set<&str> = {
            let own = check::declared_names(program);
            all_names
                .iter()
                .map(Name::as_str)
                .filter(|n| !own.contains(n) && !hidden.contains_key(*n))
                .collect()
        };
        diags.extend(check::check_typesets(program, &module_types, &dep_types));
        let mut travels = Vec::new();
        diags.extend(check::check_file_shadow(
            program,
            &extern_globals,
            &shadowable,
            &module_types,
            &module_decls,
            &module_fns,
            &mut travels,
        ));
        file_travels.push(travels);
        for d in &mut diags {
            if let Some(quals) = d
                .message
                .strip_prefix("unknown name `")
                .and_then(|rest| rest.split('`').next())
                .and_then(|name| hidden.get(name))
            {
                d.message.push_str(&format!(
                    " — `{quals}` exports it and this file does not import it; an \
                     import reaches only the file that writes it"
                ));
            }
        }
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        if !diags.is_empty() {
            return Err(diag::render(&diags, file, source));
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
        root: String::new(),
    };
    let own_files: crate::hash::Set<String> = parsed.iter().map(|(f, _, _)| f.clone()).collect();
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    for ((_, _, mut program), travels) in parsed.into_iter().zip(file_travels) {
        rename_travels(&mut program, &travels);
        merged.types.extend(program.types);
        merged.fns.extend(program.fns);
    }
    if require_entry {
        enroll_travelers(&mut merged)?;
    }
    phase::watched("canonicalize_types", || canonicalize_types(&mut merged));
    // kanso#1328 moved this pass in front of the check on the module path and
    // handed the check nothing, so both readers below it saw a slash a person
    // never wrote: the arity diagnostic quoted `m/one` for a program that says
    // `one`, and opacity refused a valid call outright. Same record, same two
    // readers as the entry path.
    let rewritten =
        phase::watched("canonicalize_bare_aliases", || canonicalize_bare_aliases(&mut merged));
    let parked = phase::watched("park_unreached_dependencies", || {
        park_unreached_dependencies(&mut merged, &own_files)
    });
    // As on the entry path: the check and the rewrite ask the same program
    // the same question back to back, so the fixpoint and the group count are
    // built once and handed to each.
    let counts = inline::group_sizes(&merged);
    let builtins = inline::aliases_from(&merged, &counts);
    let wrappers = inline::wrapper_table(&builtins, &counts);
    // A shipped library module, merged with its dependencies, holds nothing
    // but the shipped library, which is fixed when this binary is built and
    // checked clean as a root by `every_shipped_module_checks_clean`. Its
    // merged check answers the same empty list in every program that imports
    // it, and it was asked once per import in every compile.
    let shipped =
        SHIPPED_STD.with(|c| c.get()) && embedded.is_some() && !CHECK_SHIPPED.with(|c| c.get());
    let diags = match shipped {
        true => Vec::new(),
        false => phase::watched("check_merged", || {
            check::check_merged_after_aliases_with(&merged, require_entry, &rewritten, builtins)
        }),
    };
    unpark(&mut merged, parked);
    inline::apply_wrappers(&mut merged, &wrappers);
    if !diags.is_empty() {
        // The name an import writes, never the file behind it. A module in a
        // directory read as `(module pkg)` and a module in one file as
        // `(module one.kso)`, from two imports a reader spells the same way —
        // and the page, which has no filesystem and keys a handed module by
        // its import path, said `(module one)` for the second. One rule
        // settles both: the extension is a fact about storage.
        let named = dir.to_string_lossy();
        let named = named.strip_suffix(".kso").unwrap_or(&named);
        // A diagnostic that knows the file it is about gets a location and its
        // source line, the way every per-file diagnostic already does. One
        // that does not keeps the older shape: kind, message, and the module
        // suffix, which was all any of them had.
        // Read a second time, on the path that is about to fail. Holding the
        // first read across the merge would cost every clean compile the bytes
        // at its peak; a module that is about to print an error can afford to
        // open its own files again. If the second read fails the diagnostics
        // still print, without their source lines.
        let reread = module_sources(dir, embedded).unwrap_or_default();
        let borrowed: Vec<(&str, &str)> =
            reread.iter().map(|(f, s)| (f.as_str(), s.as_str())).collect();
        let rendered: Vec<String> = diags
            .iter()
            .map(|d| match d.file.as_deref() {
                // The suffix stays where it has always been, at the end of the
                // header line, and the location and quoted line are added
                // under it. So an existing message reads the same and simply
                // gains the two lines that say where it is.
                Some(_) => {
                    let body = diag::render_across(std::slice::from_ref(d), named, "", &borrowed);
                    match body.find('\n') {
                        Some(at) => format!("{} (module {named}){}", &body[..at], &body[at..]),
                        None => format!("{body} (module {named})\n"),
                    }
                }
                None => format!("error[{}]: {} (module {named})\n", d.kind, d.message),
            })
            .collect();
        return Err(rendered.join(""));
    }
    phase::watched("hoist_repeated_strings", || hoist_repeated_strings(&mut merged));
    phase::watched("fuse_enumerable", || fuse_enumerable(&mut merged));
    finish_program(&mut merged);
    phase::watched("desugar_field_reads", || desugar_field_reads(&mut merged));
    phase::watched("prune_unused_getters", || prune_unused_getters(&mut merged));
    trmc::rewrite(&mut merged);
    merged.root = root_name(&dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()));
    Ok(merged)
}

/// The bare names one file of a several-file module cannot see, each with the
/// imports that would bring it: a name that only a sibling's import surfaces,
/// and that no file of the module declares. A rename's spelling counts the
/// same way. std/render is ambient, so its names are seen everywhere.
fn hidden_from<'a>(
    program: &ast::Program,
    parsed: &'a [(String, String, ast::Program)],
    surfaced: &'a Surfaced,
    declared_here: &crate::hash::Set<&str>,
) -> crate::hash::Map<&'a str, String> {
    let qual_of =
        |i: &ast::Import| i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string());
    let mine: crate::hash::Set<String> = program.imports.iter().map(qual_of).collect();
    let mut hidden = crate::hash::Map::default();
    for (name, by) in surfaced {
        if declared_here.contains(name.as_str())
            || by.keys().any(|q| q == "render" || mine.contains(q))
        {
            continue;
        }
        let mut quals: Vec<&str> = by.keys().map(String::as_str).collect();
        quals.sort();
        hidden.insert(name.as_str(), quals.join("`, `"));
    }
    let spelled_here: crate::hash::Set<&str> = program
        .imports
        .iter()
        .flat_map(|i| i.renames.iter().map(|(_, yours)| yours.as_str()))
        .collect();
    for (_, _, other) in parsed {
        for import in &other.imports {
            for (_, yours) in &import.renames {
                if !spelled_here.contains(yours.as_str()) && !declared_here.contains(yours.as_str())
                {
                    hidden.entry(yours.as_str()).or_insert_with(|| qual_of(import));
                }
            }
        }
    }
    hidden
}

/// The dependency functions nothing in this module can reach, taken out of
/// the program while the merged check reads it and handed back after
/// (kanso#1952). An import brings its whole module, and the merged check
/// inferred and checked every function of it, so a library grew the cost of
/// every program that imports it. Each dependency is checked whole by its own
/// module's compile. They go back because the program is also what an
/// importer of this module receives, and an arm this module never calls can
/// still travel to one that does. The roots are
/// the module's own functions, whatever is on its surface, every synthetic
/// declaration (a field read becomes a call to its getter later), the render
/// group a print reaches by name, and the operator groups, which an
/// expression reaches without naming them. A group is kept whole: every arm
/// shares the name a body mentions.
fn park_unreached_dependencies(
    merged: &mut ast::Program,
    own_files: &crate::hash::Set<String>,
) -> Vec<(usize, ast::FnDecl)> {
    fn mentions<'a>(e: &'a ast::Expr, out: &mut Vec<&'a str>) {
        if let ast::Expr::Ident(n, _, _) | ast::Expr::Partial(n, _) = e {
            out.push(n.as_str());
        }
        crate::for_each_child(e, |child| mentions(child, out));
    }
    // A bare name can travel: `combine x y` reaches `ay/combine` and
    // `bee/combine` by the types it is handed, so it reaches every function
    // whose last segment it spells.
    let mut arms: crate::hash::Map<&str, Vec<usize>> = crate::hash::Map::default();
    for (at, f) in merged.fns.iter().enumerate() {
        arms.entry(f.name.as_str()).or_default().push(at);
        if let Some((_, last)) = f.name.rsplit_once('/') {
            arms.entry(last).or_default().push(at);
        }
    }
    let mut reached: crate::hash::Set<&str> = crate::hash::Set::default();
    let mut work: Vec<&str> = merged
        .fns
        .iter()
        .filter(|f| {
            own_files.contains(&*f.file)
                || !f.name.bytes().any(|b| b.is_ascii_alphanumeric())
                || matches!(f.name.as_str(), "render/to_string" | "keys" | "values")
        })
        .map(|f| f.name.as_str())
        .collect();
    let mut seen = Vec::new();
    let mut keep = vec![false; merged.fns.len()];
    while let Some(name) = work.pop() {
        if !reached.insert(name) {
            continue;
        }
        for &at in arms.get(name).map(Vec::as_slice).unwrap_or(&[]) {
            keep[at] = true;
            for stmt in &merged.fns[at].body {
                match stmt {
                    ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                        seen.clear();
                        mentions(expr, &mut seen);
                        // A traveling group (`~combine`, `~mix~combine`) is
                        // filled later from every owner's arms of its short
                        // name, so it reaches what the short name reaches.
                        work.extend(
                            seen.iter()
                                .map(|n| n.rsplit('~').next().unwrap_or(n))
                                .filter(|n| arms.contains_key(n)),
                        );
                    }
                }
            }
        }
    }
    if keep.iter().all(|&k| k) {
        return Vec::new();
    }
    let mut parked = Vec::new();
    let mut kept = Vec::with_capacity(merged.fns.len());
    for (at, f) in std::mem::take(&mut merged.fns).into_iter().enumerate() {
        match keep[at] {
            true => kept.push(f),
            false => parked.push((at, f)),
        }
    }
    merged.fns = kept;
    parked
}

/// Puts parked declarations back where they stood, so the program reads in
/// its original order again.
fn unpark(merged: &mut ast::Program, parked: Vec<(usize, ast::FnDecl)>) {
    if parked.is_empty() {
        return;
    }
    let total = merged.fns.len() + parked.len();
    let mut kept = std::mem::take(&mut merged.fns).into_iter();
    let mut parked = parked.into_iter().peekable();
    merged.fns = (0..total)
        .map(|at| match parked.next_if(|(i, _)| *i == at) {
            Some((_, f)) => f,
            None => kept.next().expect("a kept declaration for every slot"),
        })
        .collect();
}
