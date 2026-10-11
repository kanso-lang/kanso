// Imports: loading a program's dependencies, the qualified names and doors
// they open, and the checks on what an import brings in (unused imports,
// destructures of a foreign type, private uses). lib.rs reads this file with
// include!, so the items stay in the crate root and the compiler builds to
// the same instructions it did when they sat in lib.rs (kanso#1867). cargo
// fmt does not reach an included file, so CI's format job checks the files
// under src/'s subdirectories itself.

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
        root: String::new(),
    };
    let mut exports = crate::hash::Map::default();
    let mut claims: crate::hash::Map<String, u32> = crate::hash::Map::default();
    let mut surfaced: Surfaced = crate::hash::Map::default();
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
                Some(("render", &[("render.kso", include_str!("../../lib/render/render.kso"))]))
            }
            "std/list" => Some(("list", &[("list.kso", include_str!("../../lib/list/list.kso"))])),
            "std/time" => Some(("time", &[("time.kso", include_str!("../../lib/time/time.kso"))])),
            "std/io" => Some(("io", &[("io.kso", include_str!("../../lib/io/io.kso"))])),
            "std/os" => Some(("os", &[("os.kso", include_str!("../../lib/os/os.kso"))])),
            "std/text" => Some(("text", &[("text.kso", include_str!("../../lib/text/text.kso"))])),
            "std/math" => Some(("math", &[("math.kso", include_str!("../../lib/math/math.kso"))])),
            "std/bits" => Some(("bits", &[("bits.kso", include_str!("../../lib/bits/bits.kso"))])),
            "std/testing" => {
                Some(("testing", &[("testing.kso", include_str!("../../lib/testing/testing.kso"))]))
            }
            "std/net" => Some(("net", &[("net.kso", include_str!("../../lib/net/net.kso"))])),
            "std/net/http" => {
                Some(("http", &[("http.kso", include_str!("../../lib/net/http/http.kso"))]))
            }
            "std/path" => Some(("path", &[("path.kso", include_str!("../../lib/path/path.kso"))])),
            "std/sha256" => {
                Some(("sha256", &[("sha256.kso", include_str!("../../lib/sha256/sha256.kso"))]))
            }
            "std/regexp" => {
                Some(("regexp", &[("regexp.kso", include_str!("../../lib/regexp/regexp.kso"))]))
            }
            "std/json" => Some((
                "json",
                &[
                    ("json.kso", include_str!("../../lib/json/json.kso")),
                    ("number.kso", include_str!("../../lib/json/number.kso")),
                    ("scan.kso", include_str!("../../lib/json/scan.kso")),
                    ("text.kso", include_str!("../../lib/json/text.kso")),
                    ("value.kso", include_str!("../../lib/json/value.kso")),
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
            LOADED_ELSEWHERE.with(|c| c.set(true));
            let borrowed: Vec<(&str, &str)> =
                files.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect();
            let mut dep =
                compile_module_inner(std::path::Path::new(local), false, visited, Some(&borrowed))?;
            qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced);
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
            let shipped = path.starts_with("std/");
            let outer = SHIPPED_STD.with(|c| c.replace(shipped));
            let compiled =
                compile_module_inner(std::path::Path::new(path), false, visited, Some(&borrowed));
            SHIPPED_STD.with(|c| c.set(outer));
            let mut dep = compiled?;
            qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced);
            dep_program.types.extend(dep.types);
            dep_program.fns.extend(dep.fns);
            continue;
        }
        if path == "std/random" {
            return Err("error: `std/random` moved — `random` lives in `std/math`
"
            .to_string());
        }
        LOADED_ELSEWHERE.with(|c| c.set(true));
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
        qualify(&mut dep, qual, &mut exports, &mut claims, &mut surfaced);
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
                .map(move |(theirs, _)| ast::qualified(&qual, theirs))
                .collect::<Vec<_>>()
        })
        .collect();
    enroll_bare(&mut dep_program, &exports, &renamed);
    for import in imports {
        let qual = import.alias.clone().unwrap_or_else(|| short_name(&import.path).to_string());
        for (theirs, yours) in &import.renames {
            let qualified = ast::qualified(&qual, theirs);
            let mut found = false;
            let mut clones = Vec::new();
            for f in &dep_program.fns {
                if f.name == qualified {
                    for spelling in [yours.clone(), ast::qualified(&qual, yours)] {
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
                    for spelling in [yours.clone(), ast::qualified(&qual, yours)] {
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
    Ok((dep_program, exports, surfaced))
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
            .any(|re| re.name == qual || surfaces(&ast::qualified(&qual, &re.name)));
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
            let door = ast::qualified(qual, bare);
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
            alias_stmt(stmt, &doors, &mut Rewrites::default());
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
        ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => door_expr(expr, doors),
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
                *ty = Name::new(owner);
            }
        }
        ast::Pattern::Ctor { ty, fields, .. } => {
            if let Some(owner) = doors.get(ty.as_str()) {
                *ty = Name::new(owner);
            }
            for f in fields {
                door_pattern(f, doors);
            }
        }
        ast::Pattern::Annotated { ty, .. } => {
            if let Some(owner) = doors.get(ty.as_str()) {
                *ty = Name::new(owner);
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
    // The two loops below are the only readers, and between them they ask
    // about exactly the surfaced names and the targets of import renames.
    // A bare name outside that set can never be the answer to either
    // question, so the walk skips it — the same necessary condition the
    // getter walk tests, applied to a different question.
    let mut asked: crate::hash::Set<&str> = crate::hash::Set::default();
    for name in surfaced.keys() {
        asked.insert(name.as_str());
    }
    for import in &program.imports {
        for (_, yours) in &import.renames {
            asked.insert(yours.as_str());
        }
    }
    // Borrowed from the program: this walks every expression of every
    // declaration and used to keep a `String` per bare identifier OCCURRENCE,
    // for a set that is asked two questions below and dropped.
    let mut bare: crate::hash::Set<&str> = crate::hash::Set::default();
    fn collect<'a>(
        e: &'a ast::Expr,
        asked: &crate::hash::Set<&str>,
        bare: &mut crate::hash::Set<&'a str>,
    ) {
        // `&select` names the import the way `select` does; the sigil holds
        // the name rather than changing it. Left out, a file whose only use of
        // an import was a held name could not be written at all: drop the
        // import and the name does not resolve, keep it and this reads it as
        // unused.
        if let ast::Expr::Ident(name, _, _) | ast::Expr::Partial(name, _) = e {
            if !ast::has_slash(name) && asked.contains(name.as_str()) {
                bare.insert(name.as_str());
            }
        }
        for_each_child(e, |child| collect(child, asked, bare));
    }
    for decl in &program.fns {
        for stmt in &decl.body {
            match stmt {
                ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => {
                    collect(expr, &asked, &mut bare)
                }
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
    // A TYPE NAME IS NOT ALWAYS ONE NAME. `<os/process>effect`, `map[string
    // os/process]` and `[]os/process` each carry a qualified name inside a
    // shell, and splitting the whole spelling at its first slash reads `<os`
    // or `map[string os` as the qualifier -- a name no import can match, so
    // the file is refused for borrowing an import it wrote and the import
    // itself reads as unused. Every run of name characters is a name, and the
    // ones holding a slash are the qualified ones.
    fn mark(name: &str, quals: &mut crate::hash::Set<String>) {
        // Every identifier in every body reaches this, so the common name --
        // one holding no slash, and so no qualifier anywhere inside it --
        // answers on one scan and never builds the iterator below.
        if !name.contains('/') {
            return;
        }
        for part in name.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '/')) {
            if let Some((qual, _)) = ast::split_module(part) {
                quals.insert(qual.to_string());
            }
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
            ast::Expr::Ident(name, _, _) | ast::Expr::Partial(name, _) => mark(name, quals),
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
            if ast::has_slash(ty) && !fields.is_empty() {
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
        | ast::Expr::Ident(_, s, _)
        | ast::Expr::App { span: s, .. }
        | ast::Expr::Index { span: s, .. }
        | ast::Expr::BinOp { span: s, .. }
        | ast::Expr::Join { span: s, .. }
        | ast::Expr::Block(_, s)
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
    exports: &crate::hash::Map<String, bool>,
    diags: &mut Vec<diag::Diagnostic>,
) {
    fn walk(
        e: &ast::Expr,
        exports: &crate::hash::Map<String, bool>,
        diags: &mut Vec<diag::Diagnostic>,
    ) {
        if let ast::Expr::Ident(name, span, _) = e {
            if let Some(false) = exports.get(name.as_str()) {
                let (module, base) = ast::split_qual(name).unwrap_or(("", name));
                let private = format!(
                    "`{base}` is private to module `{module}` — only pub names cross an import"
                );
                diags.push(diag::Diagnostic::new("opacity", private, *span));
            }
        }
        for_each_child(e, |child| walk(child, exports, diags));
    }
    match stmt {
        ast::Stmt::Bind { expr, .. } => walk(expr, exports, diags),
        ast::Stmt::Expr(e) => walk(e, exports, diags),
    }
}
