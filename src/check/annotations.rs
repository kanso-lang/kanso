// The annotation-name check: every type an annotation names is one some
// declaration provides, read through the annotation's shape. check.rs reads
// this file with include!, so the items stay in the check module and the
// compiler builds to the same instructions it did when they sat in check.rs
// (kanso#1867). cargo fmt does not reach an included file, so CI's format job
// checks the files under src/'s subdirectories itself.

/// An annotation names a type, and a name nothing declares is a mistake the
/// reader cannot see: the arm simply never matches. The native backend already
/// refuses it, so the program did fail — but at build time, in a message about
/// a backend, rather than here in a message about the program.
///
/// A bracketed form is checked through its shape, not around it. Inference
/// reads only the shape — `ends_with("[]")` is a list, `contains('[')` is a map
/// — so the name inside was decoration nothing verified, and `[]banana`
/// compiled and ran while a bare `banana` was refused. A typo in an element
/// name should be as loud as a typo anywhere else.
/// The type names no declaration provides. Two readers ask this — the walk
/// over patterns below, and the resolver, which is where an upcast's target
/// is reached — so the list lives here rather than at either of them.
const BUILT_IN_TYPES: [&str; 9] =
    ["int", "float64", "string", "bool", "none", "done", "err", "some", "any"];

/// The type names a file may use: its own, and in a module every other file's.
#[derive(Clone, Copy)]
struct TypeNames<'a> {
    own: &'a HashSet<&'a str>,
    siblings: &'a HashSet<String>,
}

impl TypeNames<'_> {
    fn contains(&self, name: &str) -> bool {
        self.own.contains(name) || self.siblings.contains(name)
    }
}

/// The names in an annotation that no type answers to. A qualified name is
/// the import resolver's to answer for.
fn undeclared_in(ty: &str, declared: TypeNames) -> Vec<String> {
    annotation_names(ty)
        .into_iter()
        .filter(|n| {
            !crate::ast::has_slash(n) && !BUILT_IN_TYPES.contains(n) && !declared.contains(n)
        })
        .map(str::to_string)
        .collect()
}

fn check_annotation_names(
    program: &Program,
    declared: TypeNames,
    sibling_decls: &[&crate::ast::TypeDecl],
    imported: Imported,
    diags: &mut Vec<Diagnostic>,
) {
    fn patterns(
        p: &Pattern,
        declared: TypeNames,
        types: &crate::hash::Map<&str, &crate::ast::TypeDecl>,
        diags: &mut Vec<Diagnostic>,
    ) {
        match p {
            Pattern::Annotated { ty, span, .. } => {
                for name in undeclared_in(ty, declared) {
                    diags.push(Diagnostic::new(
                        "type",
                        format!("no type is called `{name}`, so this arm can never match"),
                        *span,
                    ));
                }
            }
            // `(shape x)` for a typeset `shape` destructures a record by its
            // type, and a typeset has no fields to destructure and no value
            // is ever one, so the arm can never match whatever arrives. Both
            // engines said "no overload matches these arguments" at run time,
            // which sends the reader to look at an argument that is fine.
            //
            // `Pattern::Ctor` carries no span, so this points at the first
            // field — one token past the type name, the same place the two
            // other ctor diagnostics in this file point. GIVING IT A SPAN WAS
            // MEASURED AND DECLINED: `Pattern` grows 64 bytes to 72, and
            // `kanso check lib/json` costs 482,913 more instructions for the
            // exact column. The log entry has the numbers.
            //
            // `(banana w h)` with no `banana` declared is `unseen_ctor`'s,
            // which reads the import's names beside `declared`.
            Pattern::Ctor { ty, fields, .. } => {
                if !fields.is_empty() {
                    if let Some(why) = ctor_never_matches(ty, fields.len(), types) {
                        diags.push(Diagnostic::new("type", why, other_span(&fields[0])));
                    }
                }
                for f in fields {
                    patterns(f, declared, types, diags);
                }
            }
            _ => {}
        }
    }

    // A subtype names its parent the way an annotation names a type, and
    // until 2026-09-28 nothing asked whether it named one. `type blob bytes`
    // passed `kanso check`; the interpreter then refused every construction
    // at run time, and both backends refused the program with `unknown type`.
    // `any` is a built-in name only so that its retirement can be reported
    // where an annotation uses it; as a parent it names nothing either.
    for ty in &program.types {
        let Some(parent) = &ty.parent else { continue };
        let retired = (parent == "any").then(|| parent.clone());
        for name in undeclared_in(parent, declared).into_iter().chain(retired) {
            diags.push(Diagnostic::new(
                "type",
                format!("no type is called `{name}`, so `{}` has nothing to wrap", ty.name),
                ty.span,
            ));
        }
    }

    // A typeset's members are names of the same kind, and nothing asked about
    // them either. `type either id word` with neither declared ran in the
    // interpreter, where an unknown member matched nothing and a `_:either`
    // arm was passed over, and both backends refused it with `unknown type`.
    for ty in &program.types {
        for member in &ty.members {
            let retired = (member == "any").then(|| member.clone());
            for name in undeclared_in(member, declared).into_iter().chain(retired) {
                diags.push(Diagnostic::new(
                    "type",
                    format!("no type is called `{name}`, so `{}` cannot hold it", ty.name),
                    ty.span,
                ));
            }
        }
    }

    // A record that names a field twice has two readers of one name, and the
    // only thing that ever said so was the overlap check, about two arms of a
    // reader whose name no program can spell. A typeset that names a member
    // twice was passed over in silence.
    for ty in &program.types {
        for (at, (field, _, span)) in ty.fields.iter().enumerate() {
            if ty.fields[..at].iter().any(|(earlier, _, _)| earlier == field) {
                diags.push(Diagnostic::new(
                    "type",
                    format!("`{}` declares the field `{field}` twice", ty.name),
                    *span,
                ));
            }
        }
        for (at, member) in ty.members.iter().enumerate() {
            if ty.members[..at].contains(member) {
                diags.push(Diagnostic::new(
                    "type",
                    format!("`{}` names `{member}` twice", ty.name),
                    ty.span,
                ));
            }
        }
    }

    // A typeset may hold another typeset, and every engine reads the inner
    // one's members through it. One that reaches itself that way names no
    // set at all: the interpreter asked its members forever and overflowed
    // its stack, and neither backend built the program.
    let sets: crate::hash::Map<&str, &Vec<String>> = program
        .types
        .iter()
        .filter(|t| !t.members.is_empty())
        .map(|t| (t.name.as_str(), &t.members))
        .collect();
    for ty in program.types.iter().filter(|t| !t.members.is_empty()) {
        let mut stack: Vec<&str> = ty.members.iter().map(String::as_str).collect();
        let mut seen: Vec<&str> = Vec::new();
        while let Some(member) = stack.pop() {
            if member == ty.name {
                diags.push(Diagnostic::new(
                    "type",
                    format!("`{}` holds itself through its members, so it names no set", ty.name),
                    ty.span,
                ));
                break;
            }
            if seen.contains(&member) {
                continue;
            }
            seen.push(member);
            if let Some(inner) = sets.get(member) {
                stack.extend(inner.iter().map(String::as_str));
            }
        }
    }

    // The typesets, borrowed from the list `declared` was built from. In a
    // module the other files' declarations come first and this file's own
    // after them, so a name both declare reads as this file's.
    let types: crate::hash::Map<&str, &crate::ast::TypeDecl> =
        sibling_decls.iter().copied().chain(&program.types).map(|t| (t.name.as_str(), t)).collect();
    // A field's reader is the compiler's own arm, written from the record it
    // reads. When a program declares a second type of that name, the reader
    // is checked against the wrong declaration and reported as an arm the
    // program never wrote, beside the report that the name is taken.
    for decl in program.fns.iter().filter(|d| getter_field(&d.name).is_none()) {
        for param in &decl.params {
            patterns(param, declared, &types, diags);
            unseen_ctor(param, declared, imported, diags);
        }
        for stmt in &decl.body {
            if let Stmt::Bind { pattern, .. } = stmt {
                patterns(pattern, declared, &types, diags);
            }
        }
    }
}

/// The names a file reaches through its imports. A type is among `globals`
/// by its constructor, and among `enrolled` by its bare twin, which stays a
/// type's name in a pattern when a function of the file takes the call.
#[derive(Clone, Copy)]
struct Imported<'a> {
    globals: &'a HashSet<&'a str>,
    enrolled: &'a HashSet<String>,
}

/// An arm that takes apart a record of a type the file cannot see. mal's
/// `shown (held v)`, with `held` private to another module, passed check;
/// the interpreter passed the arm over and native stopped with `native
/// backend: unknown type`. A binding's pattern is `check_binding_patterns`'s
/// to ask, on the merged program. `entry` is the map's own record, which the
/// compiler declares and no file does.
fn unseen_ctor(p: &Pattern, declared: TypeNames, imported: Imported, diags: &mut Vec<Diagnostic>) {
    let Pattern::Ctor { ty, fields, .. } = p else { return };
    let Some(first) = fields.first() else { return };
    if !crate::ast::has_slash(ty)
        && ty != "entry"
        && !BUILT_IN_TYPES.contains(&ty.as_str())
        && !declared.contains(ty)
        && !imported.globals.contains(ty.as_str())
        && !imported.enrolled.contains(ty.as_str())
    {
        diags.push(Diagnostic::new(
            "type",
            format!("no type this file can see is called `{ty}`, so this arm can never match"),
            other_span(first),
        ));
    }
    for f in fields {
        unseen_ctor(f, declared, imported, diags);
    }
}

/// Why a pattern that takes `ty` apart into `taken` fields can never match, or
/// nothing when it can. Every engine passed such an arm over in silence, so the
/// value fell to the next arm and the program answered something else.
///
/// A wrapper is taken apart through what it wraps: `(sale_price c)` binds the
/// one field of the `money` a sale price wraps. So the question is asked of
/// the record at the bottom of the chain, and a chain that ends at `int`,
/// `string` or a typeset ends at something with no fields to take. An
/// enrolled clone is left alone, since its fields are its module's to know,
/// and so is a chain that runs in a circle, which construction refuses.
fn ctor_never_matches(
    ty: &str,
    taken: usize,
    types: &crate::hash::Map<&str, &crate::ast::TypeDecl>,
) -> Option<String> {
    let decl = *types.get(ty)?;
    if !decl.members.is_empty() {
        return Some(format!(
            "`{ty}` is a typeset — it only annotates, so this arm can never match"
        ));
    }
    let mut root = decl;
    let mut hops = 0;
    while let Some(parent) = &root.parent {
        hops += 1;
        if hops > types.len() {
            return None;
        }
        match types.get(parent.as_str()) {
            Some(next) if next.members.is_empty() => root = next,
            _ => {
                return Some(format!(
                    "`{ty}` wraps `{parent}`, which has no fields, so this arm can never \
                     match; write `x:{ty}` to take one"
                ))
            }
        }
    }
    if decl.synthetic || root.synthetic {
        return None;
    }
    let have = root.fields.len();
    (have != taken).then(|| {
        let noun = if have == 1 { "field" } else { "fields" };
        let whose = if root.name == ty {
            format!("`{ty}` has")
        } else {
            format!("`{ty}` wraps `{}`, which has", root.name)
        };
        format!("{whose} {have} {noun} and this arm takes {taken}, so it can never match")
    })
}

/// The names an annotation mentions, whatever shape holds them. The parser
/// folds `[]T` to `T[]` and `map[K V]` to `map[K V]`, so both are read back
/// here rather than at each comparison — one place that knows the spelling.
fn annotation_names(ty: &str) -> Vec<&str> {
    if let Some(yield_) = crate::ast::effect_yield(ty) {
        return annotation_names(yield_);
    }
    if let Some(rest) = ty.strip_prefix("map[").and_then(|r| r.strip_suffix(']')) {
        return rest.split_whitespace().filter(|n| *n != "map").collect();
    }
    let inner = ty.trim_end_matches("[]");
    if inner.is_empty() {
        return Vec::new();
    }
    // `T[K]` is the older map spelling the lexer still folds to; both names
    // in it are types and both are checked.
    if let Some(open) = inner.find('[') {
        let (base, rest) = inner.split_at(open);
        let key = rest.trim_start_matches('[').trim_end_matches(']');
        return vec![base, key];
    }
    vec![inner]
}
