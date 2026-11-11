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
pub const SHIPPED_MODULES: [&str; 19] = [
    "std/render",
    "std/list",
    "std/time",
    "std/io",
    "std/os",
    "std/text",
    "std/math",
    "std/bits",
    "std/expect",
    "std/testing",
    "std/net",
    "std/net/http",
    "std/path",
    "std/pool",
    "std/sha256",
    "std/base64",
    "std/derive",
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
    let texts = module_sources(dir, None).unwrap_or_default();
    let texts = texts.into_iter().map(|(_, text)| (text, dir.to_path_buf())).collect();
    let mut program = reaching(texts, || {
        let mut visited = crate::hash::Set::default();
        compile_module_root(dir, require_entry, &mut visited)
    })?;
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
    /// What derivations defined for each module (`derive/define`), keyed by
    /// the module's path as `derived_key` spells it. Set while an entry's
    /// dependencies load a second time, so each module compiles the arms a
    /// convention wrote for it as one more of its own files.
    static DERIVED_FILES: std::cell::RefCell<crate::hash::Map<String, String>> =
        std::cell::RefCell::new(crate::hash::Map::default());

    /// Set while an entry that derives loads its dependencies the first time,
    /// to describe them to its derivations. A module may name a function a
    /// derivation is about to write into it, and nothing has written it yet,
    /// so an unknown name is set aside rather than refused (#2265).
    /// `DEFERRED` records that one was; the second load, with the
    /// derivations' arms in place, refuses whatever is still unknown.
    static DESCRIBING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static DEFERRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Set when that first load stood an empty file in for a directory with
    /// no kanso source, which the second load must find a derivation filled.
    static STOOD_IN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Loads an entry's dependencies to describe them to its derivations, setting
/// aside the names only a derivation could supply. Answers what it loaded and
/// whether it set any name aside.
fn load_to_describe(
    base: &std::path::Path,
    imports: &[ast::Import],
) -> (Result<Loaded, String>, bool) {
    DESCRIBING.with(|d| d.set(true));
    DEFERRED.with(|d| d.set(false));
    STOOD_IN.with(|d| d.set(false));
    let mut visited = crate::hash::Set::default();
    let loaded = load_dependencies(base, imports, &mut visited);
    DESCRIBING.with(|d| d.set(false));
    let again = DEFERRED.with(|d| d.replace(false)) | STOOD_IN.with(|d| d.replace(false));
    (loaded, again)
}

/// While an entry's modules load to be described, drops each refusal that a
/// derivation may answer by writing into the module: a name, a type, or a
/// field nothing declares yet, or a call at an arity no arm takes yet, which
/// a derivation may write an arm of (#2389). `DEFERRED` records that one was dropped, and
/// the second load, with what the derivations wrote, refuses whatever is
/// still unknown.
fn set_aside(diags: &mut Vec<crate::diag::Diagnostic>) {
    if !DESCRIBING.with(|d| d.get()) {
        return;
    }
    let before = diags.len();
    diags.retain(|d| !(d.kind == "name" && d.message.starts_with("unknown name `")));
    diags.retain(|d| !(d.kind == "type" && unknown_type(&d.message)));
    diags.retain(|d| !(d.kind == "name" && unknown_field(&d.message)));
    diags.retain(|d| !(d.kind == "arity" && d.message.contains("-argument arm of `")));
    if diags.len() < before {
        DEFERRED.with(|d| d.set(true));
    }
}

/// A refusal of a type no declaration names, which a derivation may be about
/// to declare in the module (`derive/define` of a `type`), so the first load
/// sets it aside as it does an unknown name.
fn unknown_type(message: &str) -> bool {
    message.starts_with("no type is called `")
        || message.starts_with("no type this file can see is called `")
}

/// A refusal of a field no record type has, which a record a derivation is
/// about to declare may have.
fn unknown_field(message: &str) -> bool {
    message.starts_with("no record type has a field `")
        || (message.contains("` is read for ") && message.contains(", and no record type has "))
}

/// A module's path as the derived files are keyed: the canonical path when
/// one exists, and the path as given for a module handed in as sources.
fn derived_key(dir: &std::path::Path) -> String {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()).to_string_lossy().to_string()
}

/// The file name a module's derived arms are compiled and quoted under.
fn derived_name(dir: &std::path::Path) -> String {
    format!("{}.derived.kso", dir.with_extension("").to_string_lossy())
}

thread_local! {
    /// Every word the program's own files spell, and every word of each
    /// shipped declaration kept so far. `None` outside a root compile that
    /// asked for it, and then nothing is set aside.
    static REACH: std::cell::RefCell<Option<crate::hash::Set<String>>> =
        const { std::cell::RefCell::new(None) };
    /// The words each shipped module's kept declarations spell, by the path
    /// an import names it with, and for each shipped module the shipped
    /// modules that import it. A word spelled in a shipped module keeps a
    /// declaration of that module or of one it imports, and of no other.
    static SHIPPED_WORDS: std::cell::RefCell<crate::hash::Map<String, crate::hash::Set<String>>> =
        std::cell::RefCell::new(crate::hash::Map::default());
    static SHIPPED_IMPORTERS: std::cell::RefCell<crate::hash::Map<String, Vec<String>>> =
        std::cell::RefCell::new(crate::hash::Map::default());
    /// The shipped modules the survey pruned, each with how many words the
    /// program had spelled when it did. A compile takes its module from here
    /// and prunes nothing, unless the program has spelled more since.
    static SURVEYED: std::cell::RefCell<crate::hash::Map<String, (usize, Pruned)>> =
        std::cell::RefCell::new(crate::hash::Map::default());
    /// The modules the survey found named by more than one import, which are
    /// kept from their first compile. Empty outside a root compile.
    static SHARED: std::cell::RefCell<crate::hash::Set<std::path::PathBuf>> =
        std::cell::RefCell::new(crate::hash::Set::default());
}

/// Names the compiler calls into the shipped library without the program
/// spelling them: fusion's reducers (`fold`, `bump`, `file_under`), an
/// interpolation's `to_string`, and the arm a tie dispatches to.
const COMPILER_ROOTS: [&str; 5] = ["fold", "bump", "file_under", "to_string", "tie"];

/// Groups the compiler calls beside a shipped one the program does spell,
/// so they are reached with it: `slice! s from to .> utf8` asks std/text's
/// `past?` (src/flat.rs).
const IMPLIED_ROOTS: [(&str, &[&str]); 1] = [("slice!", &["past?"])];

/// Adds every word `text` spells as code to `into`: a run of letters,
/// digits and underscores, with and without a trailing `!` or `?`.
///
/// A comment's words are left out, from a `#` to the end of its line, and so
/// is the text of a string outside its `{...}` interpolations. Until #2434
/// the reach read both, so a comment that said `first` in passing or an err
/// that said `nothing to find` kept that group of the shipped library, and
/// the whole front end then read it. Text holding a `"""` block keeps its
/// comments' words, because a block's lines are text whatever they open
/// with.
///
/// It still reads more than names (keywords, a field's name), which can only
/// keep a declaration that is not needed.
fn reach_words(text: &str, into: &mut crate::hash::Set<String>) {
    let bytes = text.as_bytes();
    // whether the text holds a block, asked at the first `#` and not before:
    // most declarations have no `#` to ask about
    let mut blocks = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphabetic() || b == b'_' {
            i = reach_word(text, i, into);
            continue;
        }
        match b {
            b'"' => i = reach_string(text, i + 1, into),
            b'#' if !*blocks.get_or_insert_with(|| text.contains("\"\"\"")) => {
                i = crate::compare::byte_from(bytes, i, b'\n');
            }
            _ => i += 1,
        }
    }
}

/// Adds the word that starts at `start` and answers where it ends. Inlined:
/// as a call it cost the module corpus 65,929 instructions.
#[inline(always)]
fn reach_word(text: &str, start: usize, into: &mut crate::hash::Set<String>) -> usize {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if !into.contains(&text[start..i]) {
        into.insert(text[start..i].to_string());
    }
    if i < bytes.len() && (bytes[i] == b'!' || bytes[i] == b'?') {
        if !into.contains(&text[start..=i]) {
            into.insert(text[start..=i].to_string());
        }
        i += 1;
    }
    i
}

/// Adds the words a string's interpolations spell, from just past its opening
/// quote, and answers where the string ends. The braces nest as the lexer
/// counts them, a backslash takes the byte after it, and the end of the line
/// closes a string nothing else closed. A string inside an interpolation is
/// read whole, which reads more than it must.
fn reach_string(text: &str, mut i: usize, into: &mut crate::hash::Set<String>) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\n' => return i,
            b'\\' if depth == 0 => i += 2,
            b'"' if depth == 0 => return i + 1,
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            _ if depth > 0 && (b.is_ascii_alphabetic() || b == b'_') => {
                i = reach_word(text, i, into);
            }
            _ => i += 1,
        }
    }
    i
}

/// Blanks every `fn` declaration of a shipped module that no reached text
/// names, so the lexer and parser never read it. A declaration is a line at
/// column 0 that opens with `fn` or `pub fn`, and the lines under it up to
/// the next line at column 0. Everything else (types, imports, comments) is
/// kept. Reach is by name over the module's files together, closed over the
/// words of each declaration kept; a name kept keeps every arm of it.
/// Blanked lines stay as empty lines, so every span the module reports points
/// where it did.
fn prune_shipped(
    sources: &mut [(String, String)],
    roots: &crate::hash::Set<String>,
    lent: &[&crate::hash::Set<String>],
    mine: &mut crate::hash::Set<String>,
) -> (Vec<bool>, Vec<bool>) {
    struct Decl {
        file: usize,
        start: usize,
        end: usize,
        name: Option<(usize, usize)>,
    }
    let mut decls: Vec<Decl> = Vec::new();
    for (f, (_, source)) in sources.iter().enumerate() {
        let mut at = 0;
        // Lines a word at a time, as `codegen::ir_defines` reads IR:
        // `split_inclusive` called memchr once a line.
        while at < source.len() {
            let end = crate::compare::byte_from(source.as_bytes(), at, b'\n');
            let line = &source[at..(end + 1).min(source.len())];
            let here = at;
            at += line.len();
            let first = line.as_bytes().first().copied().unwrap_or(b'\n');
            if first == b' ' || first == b'\n' || first == b']' || first == b'}' || first == b')' {
                if let Some(d) = decls.last_mut().filter(|d| d.file == f) {
                    d.end = at;
                }
                continue;
            }
            if first == b'#' {
                // a comment at column 0 closes the declaration above it and
                // belongs to none
                decls.push(Decl { file: f, start: here, end: here, name: None });
                continue;
            }
            let head = line.strip_prefix("pub ").unwrap_or(line);
            let name = head.strip_prefix("fn ").map(|rest| {
                let from = here + (line.len() - rest.len());
                let len = rest
                    .bytes()
                    .position(|b| {
                        !(b.is_ascii_alphanumeric() || b == b'_' || b == b'!' || b == b'?')
                    })
                    .unwrap_or(rest.len());
                let qualified = rest.as_bytes().get(len) == Some(&b'/');
                (from, if qualified { from } else { from + len })
            });
            // an operator arm's name is not a word, and an arm of another
            // module's function is reached through that module: both stay
            let name = name.filter(|(from, to)| from < to);
            decls.push(Decl { file: f, start: here, end: at, name });
        }
    }
    // What the program and the importers name is settled before this module
    // is read, so each declaration asks them once. Only the module's own
    // words grow while it is walked, and the walk asks those alone.
    let keep = |d: &Decl, mine: &mut crate::hash::Set<String>| {
        let source = &sources[d.file].1;
        reach_words(&source[d.start..d.end], mine);
        if let Some((from, to)) = d.name {
            for (name, implied) in IMPLIED_ROOTS {
                if &source[from..to] == name {
                    mine.extend(implied.iter().map(|n| n.to_string()));
                }
            }
        }
    };
    let mut kept = vec![false; decls.len()];
    for (i, d) in decls.iter().enumerate() {
        let named = match d.name {
            None => true,
            Some((from, to)) => {
                let name = &sources[d.file].1[from..to];
                roots.contains(name) || lent.iter().any(|words| words.contains(name))
            }
        };
        if named {
            kept[i] = true;
            keep(d, mine);
        }
    }
    loop {
        let mut moved = false;
        for (i, d) in decls.iter().enumerate() {
            let Some((from, to)) = d.name.filter(|_| !kept[i]) else { continue };
            if mine.contains(&sources[d.file].1[from..to]) {
                kept[i] = true;
                moved = true;
                keep(d, mine);
            }
        }
        if !moved {
            break;
        }
    }
    let mut pruned = vec![false; sources.len()];
    for (f, (_, source)) in sources.iter_mut().enumerate() {
        if !decls.iter().zip(&kept).any(|(d, k)| d.file == f && !k) {
            continue;
        }
        pruned[f] = true;
        let mut out = String::with_capacity(source.len());
        let mut copied = 0;
        for (d, _) in decls.iter().zip(&kept).filter(|(d, k)| d.file == f && !**k) {
            out.push_str(&source[copied..d.start]);
            out.extend(std::iter::repeat_n('\n', source[d.start..d.end].matches('\n').count()));
            copied = d.end;
        }
        out.push_str(&source[copied..]);
        // nothing below the last line kept can be reported, and a file may
        // not end with a blank line
        while out.ends_with("\n\n") {
            out.pop();
        }
        *source = out;
    }
    (pruned, kept)
}

/// An entry file as the one text a root compile starts from, with the
/// directory its imports are read from.
fn from_entry(file: &str, source: &str) -> Vec<(String, std::path::PathBuf)> {
    let base = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    vec![(source.to_string(), base)]
}

/// The paths a source names on its `import` lines. Imports open a file, so
/// the reading stops at the first line that is not one, a comment or blank.
fn imports_named(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .take_while(|line| line.is_empty() || line.starts_with('#') || line.starts_with("import "))
        .filter_map(|line| line.strip_prefix("import ")?.split('"').nth(1))
}

/// Reads the program's own modules for their words before any of it is
/// compiled, and notes which modules more than one import names.
///
/// A shipped module is pruned against the words read so far, and a compile
/// read them as it went: the same module imported from three of the
/// program's files was pruned three ways and compiled three times, each a
/// little larger than the last. With the program's words in hand first, the
/// three imports ask for one pruning, and a module known to be asked for
/// again is kept from its first compile.
///
/// It follows `import` lines as text and passes over whatever it cannot
/// read. What it misses the compile still adds as it goes, at the cost of
/// the compile this is here to save. A shipped module's own words are left
/// to the compile: pruning each to a fixed point here cost 3.4 million
/// instructions on the smallest corpus and saved less than that on the
/// largest.
fn survey(
    mut pending: Vec<(String, std::path::PathBuf)>,
    roots: &mut crate::hash::Set<String>,
) -> crate::hash::Set<std::path::PathBuf> {
    let mut named: crate::hash::Map<std::path::PathBuf, usize> = crate::hash::Map::default();
    // the entry imports std/render whether it says so or not
    named.insert(std::path::PathBuf::from("std/render"), 1);
    type Shipped = (String, &'static [(&'static str, &'static str)]);
    let mut shipped: Vec<Shipped> = Vec::new();
    // counts an import of a shipped module, and says whether it was one
    fn note(
        path: &str,
        named: &mut crate::hash::Map<std::path::PathBuf, usize>,
        shipped: &mut Vec<Shipped>,
    ) -> bool {
        let Some((_, files)) = embedded_module(path).filter(|_| path.starts_with("std/")) else {
            return false;
        };
        let times = named.entry(std::path::PathBuf::from(path)).or_insert(0);
        *times += 1;
        if *times == 1 {
            shipped.push((path.to_string(), files));
        }
        true
    }
    shipped
        .extend(embedded_module("std/render").map(|(_, files)| ("std/render".to_string(), files)));
    while let Some((text, base)) = pending.pop() {
        for path in imports_named(&text) {
            if note(path, &mut named, &mut shipped) {
                continue;
            }
            let local = path.strip_prefix("./").unwrap_or(path);
            let handed = HANDED_SOURCES.with(|c| c.borrow().get(local).cloned());
            let (key, sources, inner) = match handed {
                Some(files) => (std::path::PathBuf::from(local), Some(files), base.clone()),
                None => {
                    let Ok(dir) = resolve_import(&base, path) else { continue };
                    let inner = match dir.is_file() {
                        true => dir.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
                        false => dir.clone(),
                    };
                    (dir.canonicalize().unwrap_or_else(|_| dir.clone()), None, inner)
                }
            };
            let times = named.entry(key.clone()).or_insert(0);
            *times += 1;
            if *times > 1 {
                continue;
            }
            let Ok(sources) = sources.map_or_else(|| module_sources(&key, None), Ok) else {
                continue;
            };
            for (_, text) in sources {
                reach_words(&text, roots);
                pending.push((text, inner.clone()));
            }
        }
    }
    // a shipped module's imports are shipped modules, counted the same way,
    // and each import is one more importer the imported module waits for
    let mut waits: crate::hash::Map<String, usize> = crate::hash::Map::default();
    let mut at = 0;
    while at < shipped.len() {
        let files = shipped[at].1;
        for path in files.iter().flat_map(|(_, text)| imports_named(text)) {
            if note(path, &mut named, &mut shipped) {
                *waits.entry(path.to_string()).or_insert(0) += 1;
            }
        }
        at += 1;
    }
    // Importers first. A word a shipped module spells can keep a declaration
    // of a module it imports and of no module further on, so by the time a
    // module is pruned every word that can reach it has been read, and one
    // pruning each is the last one.
    let spelled = roots.len();
    let mut ready: Vec<usize> =
        (0..shipped.len()).filter(|m| !waits.contains_key(&shipped[*m].0)).collect();
    while let Some(m) = ready.pop() {
        let (module, files) = &shipped[m];
        for path in files.iter().flat_map(|(_, text)| imports_named(text)) {
            let Some(left) = waits.get_mut(path) else { continue };
            *left -= 1;
            if *left == 0 {
                ready.extend(shipped.iter().position(|(name, _)| name == path));
            }
        }
        // named as the loader names them: a file's name is how the checker
        // knows it for the library's
        let named: Vec<(String, &str)> =
            files.iter().map(|(n, text)| (format!("{module}/{n}"), *text)).collect();
        let borrowed: Vec<(&str, &str)> = named.iter().map(|(n, s)| (n.as_str(), *s)).collect();
        let Ok(sources) = module_sources(std::path::Path::new(module), Some(&borrowed)) else {
            continue;
        };
        let done = pruned_shipped(module, sources, roots);
        SURVEYED.with(|s| s.borrow_mut().insert(module.clone(), (spelled, done)));
    }
    named.into_iter().filter(|(_, times)| *times > 1).map(|(module, _)| module).collect()
}

/// Runs a root compile with the shipped library read only as far as the
/// program reaches, and runs it again whole if that fails. The second run is
/// what reports, so every diagnostic is the one the whole library gives; a
/// program the whole library accepts and the pruned one refused is a name
/// this reach missed, which a debug build refuses to hide.
fn reaching<T>(
    texts: Vec<(String, std::path::PathBuf)>,
    compile: impl Fn() -> Result<T, String>,
) -> Result<T, String> {
    let mut roots = crate::hash::Set::default();
    for root in COMPILER_ROOTS {
        roots.insert(root.to_string());
    }
    for (text, _) in &texts {
        reach_words(text, &mut roots);
    }
    // The shipped modules' words belong to one compile, as the roots do: a
    // compile inside another starts with none and hands the outer's back.
    let outer_words = SHIPPED_WORDS.with(|w| w.take());
    let outer_importers = SHIPPED_IMPORTERS.with(|i| i.take());
    let outer_surveyed = SURVEYED.with(|s| s.take());
    let shared = survey(texts, &mut roots);
    let outer = REACH.with(|r| r.replace(Some(roots)));
    let outer_shared = SHARED.with(|s| s.replace(shared));
    let pruned = compile();
    REACH.with(|r| *r.borrow_mut() = None);
    SHARED.with(|s| *s.borrow_mut() = outer_shared);
    let answer = match pruned {
        Ok(program) => Ok(program),
        Err(refused) => {
            let whole = compile();
            debug_assert!(
                whole.is_err(),
                "the shipped library read only as far as the program reaches refused \
                 what the whole library accepts:\n{refused}"
            );
            whole
        }
    };
    REACH.with(|r| *r.borrow_mut() = outer);
    // a run holds none of these: they are dropped here, where the compile ends
    SHIPPED_WORDS.with(|w| *w.borrow_mut() = outer_words);
    SHIPPED_IMPORTERS.with(|i| *i.borrow_mut() = outer_importers);
    SURVEYED.with(|s| *s.borrow_mut() = outer_surveyed);
    answer
}

/// Hands the compiler a module's files under the path an import will name.
/// They live until `forget_sources`, so a caller compiling one program after
/// another says when the last one's sources stop applying.
pub fn hand_source(path: &str, files: Vec<(String, String)>) {
    HANDED_SOURCES.with(|c| c.borrow_mut().insert(path.to_string(), files));
}

/// The text of a handed-in file by the name its diagnostics carry, so a
/// refusal in a module the browser was handed quotes its line as one read from
/// disk does.
fn handed_text(file: &str) -> Option<String> {
    let handed = HANDED_SOURCES.with(|c| {
        c.borrow().values().flatten().find(|(name, _)| name == file).map(|(_, text)| text.clone())
    });
    handed.or_else(|| DERIVED_TEXT.with(|t| t.borrow().get(file).cloned()))
}

thread_local! {
    /// The derived files of the last entry compiled, by the name their
    /// diagnostics carry, so a refusal in an arm a convention wrote quotes
    /// the line it wrote.
    static DERIVED_TEXT: std::cell::RefCell<crate::hash::Map<String, String>> =
        std::cell::RefCell::new(crate::hash::Map::default());
}

/// The import lines a definition opens with, and the text after them and the
/// blank lines that follow. A module's definitions share one derived file,
/// and a file's imports open it, so they are gathered there.
fn opening_imports(source: &str) -> (Vec<&str>, &str) {
    let mut imports = Vec::new();
    let mut rest = source;
    while let Some(line) = rest.split_inclusive('\n').next() {
        let text = line.trim_end();
        if text.starts_with("import ") {
            imports.push(text);
        } else if !(text.is_empty() && !imports.is_empty()) {
            break;
        }
        rest = &rest[line.len()..];
    }
    (imports, rest)
}

/// A definition's import line as the module it joins reads it. A relative
/// path is written from the derivation's own module, as that module's own
/// imports are, so a convention names its library however the app reached
/// it; the path is rewritten from the module the definition joins. Any other
/// import, `std/list` say, reads the same from anywhere.
fn reread(line: &str, writer: Option<&std::path::Path>, joins: &std::path::Path) -> String {
    let Some(path) = line.strip_prefix("import \"").and_then(|r| r.strip_suffix('"')) else {
        return line.to_string();
    };
    let Some(writer) = writer.filter(|_| path.starts_with("./") || path.starts_with("../")) else {
        return line.to_string();
    };
    // A module file named bare, `stock.kso` beside an entry run from its own
    // directory, has an empty parent, which is the current directory.
    let at = |p: &std::path::Path| match p.is_dir() {
        true => std::fs::canonicalize(p).ok(),
        false => p
            .parent()
            .map(|d| if d.as_os_str().is_empty() { std::path::Path::new(".") } else { d })
            .and_then(|d| std::fs::canonicalize(d).ok()),
    };
    let (Some(from), Some(to)) = (at(writer), at(joins)) else { return line.to_string() };
    let mut named = from;
    for part in std::path::Path::new(path).components() {
        match part {
            std::path::Component::ParentDir => {
                named.pop();
            }
            std::path::Component::Normal(p) => named.push(p),
            _ => {}
        }
    }
    let shared = named.components().zip(to.components()).take_while(|(a, b)| a == b).count();
    let ups = to.components().count() - shared;
    let down: Vec<String> = named
        .components()
        .skip(shared)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let lead = if ups == 0 { vec![".".to_string()] } else { vec!["..".to_string(); ups] };
    format!("import \"{}\"", [lead, down].concat().join("/"))
}

/// Loads an entry's dependencies again with the arms its derivations
/// defined, each joined to the module it was defined in as one more file.
fn load_with_definitions(
    base: &std::path::Path,
    imports: &[ast::Import],
    definitions: &[(String, String, String)],
) -> Result<Loaded, String> {
    // Per module, in the order its first definition arrives: the imports its
    // definitions open with, and the rest of their text.
    let mut order: Vec<(String, String)> = Vec::new();
    let mut parts: crate::hash::Map<String, (Vec<String>, String)> = crate::hash::Map::default();
    for (path, source, from) in definitions {
        let dir = resolve_import(base, path)
            .unwrap_or_else(|_| std::path::PathBuf::from(path.strip_prefix("./").unwrap_or(path)));
        let key = derived_key(&dir);
        if !parts.contains_key(&key) {
            order.push((key.clone(), derived_name(&dir)));
        }
        let writer = resolve_import(base, from).ok();
        let (imports, body) = parts.entry(key).or_default();
        let (opened, rest) = opening_imports(source);
        imports.extend(opened.into_iter().map(|line| reread(line, writer.as_deref(), &dir)));
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(rest);
    }
    let mut files: crate::hash::Map<String, String> = crate::hash::Map::default();
    let mut texts: crate::hash::Map<String, String> = crate::hash::Map::default();
    for (key, name) in order {
        let (mut imports, body) = parts.remove(&key).unwrap_or_default();
        imports.sort_unstable();
        imports.dedup();
        let text =
            if imports.is_empty() { body } else { format!("{}\n\n{body}", imports.join("\n")) };
        texts.insert(name, text.clone());
        files.insert(key, text);
    }
    DERIVED_TEXT.with(|t| *t.borrow_mut() = texts);
    DERIVED_FILES.with(|d| *d.borrow_mut() = files);
    let mut visited = crate::hash::Set::default();
    let loaded = load_dependencies(base, imports, &mut visited);
    DERIVED_FILES.with(|d| d.borrow_mut().clear());
    loaded
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

/// The root module gets the ambient imports, such as render/to_string;
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
    // A shipped module is pruned before it is looked up: what it was pruned
    // to is what a compile of it depends on, and that is the key. An import
    // that finds as many roots as an earlier import of the module found
    // prunes to the same declarations, so it asks the cache under those
    // first and reads no source when the compile is there.
    let prunes = prunes(embedded.is_some());
    let reached = match prunes {
        true => shipped_reach(dir),
        false => 0,
    };
    let before = match prunes {
        true => PRUNINGS.with(|p| p.borrow().get(&canon).and_then(|m| m.at(reached))),
        false => None,
    };
    if let Some(kept) = before {
        let key = module_key(&canon, require_entry, embedded.is_some(), kept);
        if let Some(done) = MODULE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
            visited.remove(&canon);
            return Ok(done);
        }
    }
    let shipped = match prunes {
        true => match shipped_pruned(dir, embedded) {
            Ok(pruned) => pruned,
            Err(refused) => {
                visited.remove(&canon);
                return Err(refused);
            }
        },
        false => None,
    };
    let (sources, kept) = match shipped {
        Some((sources, pruned, kept)) => {
            let kept = PRUNINGS
                .with(|p| p.borrow_mut().entry(canon.clone()).or_default().noted(reached, kept));
            (Some((sources, pruned)), kept)
        }
        None => (None, usize::MAX),
    };
    let key = module_key(&canon, require_entry, embedded.is_some(), kept);
    if let Some(done) = MODULE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        visited.remove(&canon);
        return Ok(done);
    }
    // A module is kept once a second import asks for it, or from its first
    // compile when the survey saw two imports name it. Keeping the first
    // compile of every module cloned each one, and a program whose modules
    // are each imported once paid the clones and the memory for nothing:
    // compile_corpus's front end rose 43% in what it held.
    let again = MODULE_SEEN.with(|s| !s.borrow_mut().insert(key.clone()))
        || SHARED.with(|s| s.borrow().contains(&canon));
    if std::env::var_os("KANSO_PHASES").is_some() {
        eprintln!("load {}", dir.display());
    }
    MODULE_LOADS.with(|c| c.set(c.get() + 1));
    let loaded = compile_module_loaded(dir, require_entry, visited, embedded, sources);
    visited.remove(&canon);
    if let (true, Ok(program)) = (again, &loaded) {
        MODULE_CACHE.with(|c| c.borrow_mut().insert(key, program.clone()));
    }
    loaded
}

/// What a module's compile depends on besides its own sources, which no
/// compile in one load changes: where the module is, whether it must be an
/// entry, where its sources came from, the flags its importer sets around
/// it, and, for a shipped module, which of its declarations the pruning
/// kept.
///
/// The key held the number of words reached so far until #2434. That number
/// grows with every module a load reads, so two imports of `std/text` a few
/// modules apart never shared a compile, though the words that arrived
/// between them named nothing in it: the entry corpus compiled `std/text`
/// and `std/list` ten times each. A module's kept declarations name
/// everything it reaches in the modules it imports, and the roots only grow,
/// so a compile kept under the same declarations holds what this import
/// needs. The key holds which of the module's prunings this is, counted in
/// the order they were first seen (`Prunings`), and `usize::MAX` for a
/// module that is not pruned.
type ModuleKey = (std::path::PathBuf, bool, bool, bool, bool, bool, bool, usize);

/// The ways one shipped module has been pruned in this load: each distinct
/// set of kept declarations once, and which of them the roots gave when
/// they numbered so many. The roots only grow, so as many roots are the
/// same roots.
#[derive(Default)]
struct Prunings {
    sets: Vec<Vec<bool>>,
    at: Vec<(usize, usize)>,
}

impl Prunings {
    fn at(&self, reached: usize) -> Option<usize> {
        self.at.iter().find(|(r, _)| *r == reached).map(|(_, set)| *set)
    }

    fn noted(&mut self, reached: usize, kept: Vec<bool>) -> usize {
        let set = match self.sets.iter().position(|s| *s == kept) {
            Some(set) => set,
            None => {
                self.sets.push(kept);
                self.sets.len() - 1
            }
        };
        self.at.push((reached, set));
        set
    }
}

/// Whether a module loaded now is a shipped one read only as far as the
/// program reaches (see `compile_module_loaded`); any other compiles the same
/// against any roots.
fn prunes(embedded: bool) -> bool {
    SHIPPED_STD.with(|c| c.get())
        && embedded
        && !CHECK_SHIPPED.with(|c| c.get())
        && REACH.with(|r| r.borrow().is_some())
}

/// A module's sources as `(file, text)`, and for each whether it lost a
/// declaration to the pruning.
type PrunedSources = (Vec<(String, String)>, Vec<bool>);

/// A shipped module's sources pruned against the roots, with which files
/// lost a declaration and which declarations were kept. `None` when no root
/// compile asked for pruning.
type Pruned = (Vec<(String, String)>, Vec<bool>, Vec<bool>);

fn shipped_pruned(
    dir: &std::path::Path,
    embedded: Option<&[(&str, &str)]>,
) -> Result<Option<Pruned>, String> {
    let module = dir.to_string_lossy().into_owned();
    REACH.with(|r| {
        let r = r.borrow();
        let Some(roots) = r.as_ref() else { return Ok(None) };
        let surveyed = SURVEYED.with(|s| s.borrow_mut().remove(&module));
        if let Some((_, done)) = surveyed.filter(|(spelled, _)| *spelled == roots.len()) {
            return Ok(Some(done));
        }
        let sources = module_sources(dir, embedded)?;
        Ok(Some(pruned_shipped(&module, sources, roots)))
    })
}

/// Prunes one shipped module against the program's words, its own, and
/// those of the shipped modules that import it, and notes it as an importer
/// of the shipped modules it imports.
fn pruned_shipped(
    module: &str,
    mut sources: Vec<(String, String)>,
    roots: &crate::hash::Set<String>,
) -> Pruned {
    let importers = SHIPPED_IMPORTERS.with(|i| i.borrow().get(module).cloned().unwrap_or_default());
    let (pruned, kept) = SHIPPED_WORDS.with(|w| {
        let mut w = w.borrow_mut();
        let mut mine = w.remove(module).unwrap_or_default();
        let lent: Vec<&crate::hash::Set<String>> =
            importers.iter().filter_map(|importer| w.get(importer)).collect();
        let done = prune_shipped(&mut sources, roots, &lent, &mut mine);
        w.insert(module.to_string(), mine);
        done
    });
    SHIPPED_IMPORTERS.with(|i| {
        let mut i = i.borrow_mut();
        for imported in sources.iter().flat_map(|(_, s)| imports_named(s)) {
            let importers = i.entry(imported.to_string()).or_default();
            if !importers.iter().any(|importer| importer == module) {
                importers.push(module.to_string());
            }
        }
    });
    (sources, pruned, kept)
}

/// How many words can keep a declaration of this shipped module: the
/// program's, its own, and those of the shipped modules that import it.
/// Each of the three only grows, so as many words are the same words.
fn shipped_reach(dir: &std::path::Path) -> usize {
    let module = dir.to_string_lossy().into_owned();
    let importers =
        SHIPPED_IMPORTERS.with(|i| i.borrow().get(&module).cloned().unwrap_or_default());
    let shipped = SHIPPED_WORDS.with(|w| {
        let w = w.borrow();
        let own = w.get(&module).map_or(0, |words| words.len());
        own + importers.iter().filter_map(|p| w.get(p)).map(|words| words.len()).sum::<usize>()
    });
    shipped + REACH.with(|r| r.borrow().as_ref().map_or(0, |s| s.len()))
}

fn module_key(
    canon: &std::path::Path,
    require_entry: bool,
    embedded: bool,
    kept: usize,
) -> ModuleKey {
    (
        canon.to_path_buf(),
        require_entry,
        embedded,
        SHIPPED_STD.with(|c| c.get()),
        CHECK_SHIPPED.with(|c| c.get()),
        ENTRY_COMPILE.with(|c| c.get()),
        HAKO_EMBEDDED.with(|c| c.get()),
        kept,
    )
}

thread_local! {
    /// Each module compiled once per load (kanso#2373). An import used to
    /// compile its module afresh, and so did every import inside it, so a
    /// module cost one compile per path that reached it: the pantry loaded
    /// its 27 modules 1,094 times. A module compiled a second time under the
    /// same key is kept and handed out after that; the outermost load empties
    /// both maps when it ends.
    static MODULE_CACHE: std::cell::RefCell<crate::hash::Map<ModuleKey, ast::Program>> =
        std::cell::RefCell::new(crate::hash::Map::default());
    static MODULE_SEEN: std::cell::RefCell<crate::hash::Set<ModuleKey>> =
        std::cell::RefCell::new(crate::hash::Set::default());
    /// How each shipped module has been pruned in this load. Emptied with
    /// the cache.
    static PRUNINGS: std::cell::RefCell<crate::hash::Map<std::path::PathBuf, Prunings>> =
        std::cell::RefCell::new(crate::hash::Map::default());
    static LOAD_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

thread_local! {
    static MODULE_LOADS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many modules this thread has compiled since `reset_module_loads`: one
/// for each module read, lexed, parsed and checked. The compile-cost golden
/// pins it per sample (kanso#2373).
pub fn module_loads() -> u64 {
    MODULE_LOADS.with(|c| c.get())
}

pub fn reset_module_loads() {
    MODULE_LOADS.with(|c| c.set(0));
}

/// A module file's name as a diagnostic prints it. A test run from inside
/// its module imports the module as `../<name>`, which named a file beside
/// the reader `../t/p.kso` (#2205). A path that climbs out of the working
/// directory and back into it is shown from the working directory.
fn shown(path: &std::path::Path) -> String {
    let file = path.to_string_lossy().into_owned();
    if !file.starts_with("..") {
        return file;
    }
    let here = std::env::current_dir().and_then(|d| d.canonicalize());
    match (here, path.canonicalize()) {
        (Ok(here), Ok(full)) => match full.strip_prefix(&here) {
            Ok(rest) => rest.to_string_lossy().into_owned(),
            Err(_) => file,
        },
        _ => file,
    }
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
                let file = shown(path);
                let source = std::fs::read_to_string(path)
                    .map_err(|io| format!("error: cannot read {file}: {io}\n"))?;
                out.push((file, source));
            }
            out
        }
    };
    // Asked only while a derivation's arms are loading: the key canonicalizes
    // the path, and every other compile would pay for it at every module.
    let derived = DERIVED_FILES.with(|d| {
        let d = d.borrow();
        (!d.is_empty()).then(|| d.get(&derived_key(dir)).cloned()).flatten()
    });
    if let Some(text) = derived {
        sources.push((derived_name(dir), text));
    }
    // A directory of templates alone is a module whose functions a derivation
    // writes from them (#2280). While the entry's dependencies load to be
    // described nothing has written them yet, so it loads empty; the second
    // load compiles what was written, and refuses it if nothing was.
    if sources.is_empty() && embedded.is_none() && dir.is_dir() && DESCRIBING.with(|d| d.get()) {
        STOOD_IN.with(|d| d.set(true));
        sources.push((derived_name(dir), String::new()));
    }
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    if sources.is_empty() {
        return Err(format!(
            "error: no .kso files in {} — a directory without kanso source is a \
             module only when a derivation writes its functions\n",
            dir.display()
        ));
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
    shipped: Option<PrunedSources>,
) -> Result<ast::Program, String> {
    // a shipped module arrives pruned, because its key needed that; any
    // other adds its words to what the shipped ones are pruned against
    let (mut sources, pruned) = match shipped {
        Some((sources, pruned)) => (sources, Some(pruned)),
        None => {
            let sources = module_sources(dir, embedded)?;
            REACH.with(|r| {
                if let Some(roots) = r.borrow_mut().as_mut() {
                    sources.iter().for_each(|(_, s)| reach_words(s, roots));
                }
            });
            (sources, None)
        }
    };
    // a file with declarations blanked out of it can leave an import that
    // only they used, and the whole file's imports are checked where the
    // shipped modules are
    let mut pruned_files: crate::hash::Set<String> = crate::hash::Set::default();
    let mut parsed = Vec::new();
    for (at, (file, source)) in sources.drain(..).enumerate() {
        let mut lexed = phase::watched("lex", || lexer::lex(&source))
            .map_err(|d| diag::render(&d, &file, &source))?;
        lexed.pruned = pruned.as_ref().is_some_and(|p| p[at]);
        if lexed.pruned {
            pruned_files.insert(file.clone());
        }
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
    // An import reaches only the file that writes it (2026-10-12). Its bare
    // twin of a name the module also declares joins the module's own arms in
    // one overload group, which is what a file that imports it and writes the
    // name asks for. When no such file writes the name, the twin can only
    // reach files that never imported it: xsv's private `matches` in one file
    // took std/regexp's arms because another file imported std/regexp.
    if parsed.len() > 1 {
        let away = twins_no_importer_names(&parsed, &surfaced);
        if !away.is_empty() {
            dep_program
                .fns
                .retain(|f| !(f.synthetic && !ast::has_slash(&f.name) && away.contains(&f.name)));
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
        let mut diags = match pruned_files.contains(file.as_str()) {
            true => Vec::new(),
            false => unused_imports(&program.imports, &quals),
        };
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
    // The functions the module's files declare: a call of one on a value a
    // function was handed is shaded, as a call of the file's own is. Each
    // carries the first file that declares it and whether another file
    // declares arms of it too, which leaves its arity to the merged check. A
    // module of one file has no others.
    let module_fns: crate::hash::Map<&str, (u32, bool)> = match parsed.len() {
        1 => crate::hash::Map::default(),
        _ => {
            let mut seen = crate::hash::Map::default();
            for (at, (_, _, p)) in parsed.iter().enumerate() {
                for f in &p.fns {
                    let slot = seen.entry(f.name.as_str()).or_insert((at as u32, false));
                    slot.1 |= slot.0 != at as u32;
                }
            }
            seen
        }
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
        // A binding refused for a name another file of the module declares
        // pointed at the innocent local and nowhere else (#2085). Name the
        // declaration it collides with.
        for d in &mut diags {
            let Some(name) = d.message.strip_prefix('`').and_then(|rest| {
                rest.strip_suffix("` is already a declaration; rename the binding")
            }) else {
                continue;
            };
            let elsewhere =
                parsed.iter().filter(|(other, _, _)| other != file).find_map(|(other, _, p)| {
                    let at = p.fns.iter().find(|f| f.name == name).map(|f| f.span);
                    let at = at.or_else(|| p.types.iter().find(|t| t.name == name).map(|t| t.span));
                    at.map(|span| (other, span.line))
                });
            if let Some((other, line)) = elsewhere {
                d.message = format!(
                    "`{name}` is already a declaration, in {other} at line {line}; rename the \
                     binding"
                );
            }
        }
        set_aside(&mut diags);
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
        if !pruned_files.contains(file.as_str()) {
            diags.extend(unused_imports(&program.imports, &quals));
        }
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
        derives: Vec::new(),
        root: String::new(),
    };
    merged.types.extend(dep_program.types);
    merged.fns.extend(dep_program.fns);
    for ((_, _, mut program), travels) in parsed.into_iter().zip(file_travels) {
        rename_travels(&mut program, &travels);
        merged.types.extend(program.types);
        merged.fns.extend(program.fns);
    }
    own_types_beat_twins(&mut merged);
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
    let mut diags = match shipped {
        true => Vec::new(),
        false => phase::watched("check_merged", || {
            check::check_merged_after_aliases_with(
                &merged,
                require_entry,
                require_entry,
                &rewritten,
                builtins,
            )
        }),
    };
    inline::apply_wrappers(&mut merged, &wrappers);
    set_aside(&mut diags);
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
/// The names the module declares that an import also enrols bare, where no
/// file importing that module writes the bare name.
fn twins_no_importer_names(
    parsed: &[(String, String, ast::Program)],
    surfaced: &Surfaced,
) -> Vec<String> {
    let qual_of =
        |i: &ast::Import| i.alias.clone().unwrap_or_else(|| short_name(&i.path).to_string());
    let declared = |name: &str| parsed.iter().any(|(_, _, p)| p.fns.iter().any(|f| f.name == name));
    let mut away = Vec::new();
    for (name, by) in surfaced {
        if !declared(name) {
            continue;
        }
        let written_where_seen = parsed.iter().any(|(_, _, p)| {
            p.imports.iter().any(|i| by.contains_key(&qual_of(i))) && writes_bare(p, name)
        });
        if !written_where_seen {
            away.push(name.clone());
        }
    }
    away
}

/// Whether any declaration in `program` mentions `name` bare.
fn writes_bare(program: &ast::Program, name: &str) -> bool {
    fn in_expr(e: &ast::Expr, name: &str) -> bool {
        if let ast::Expr::Ident(n, _, _) | ast::Expr::Partial(n, _) = e {
            if n == name {
                return true;
            }
        }
        crate::any_child(e, |c| in_expr(c, name))
    }
    program.fns.iter().any(|f| {
        f.body.iter().any(|st| match st {
            ast::Stmt::Bind { expr, .. } | ast::Stmt::Expr(expr) => in_expr(expr, name),
        })
    })
}

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
