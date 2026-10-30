// The literal-argument check: a literal handed to a group no arm of which
// could take it, refused where the author wrote it. check.rs reads this file
// with include!, so the items stay in the check module and the compiler builds
// to the same instructions it did when they sat in check.rs (kanso#1867).
// cargo fmt does not reach an included file, so CI's format job checks the
// files under src/'s subdirectories itself.

/// A literal argument no arm could ever take.
///
/// The general question — can this expression's type reach any arm — wants
/// the inference sets and their joins, and a join is a superset, so a wrong
/// call can hide behind a right one at another site. A literal needs none of
/// that: its type is on the page. So this asks the narrow question exactly,
/// and says nothing where an argument is anything else. What it catches is
/// the call that could never have worked, reported where the author wrote it
/// instead of when the value arrives.
/// The arms of each dispatch group, keyed by NAME so a `&str` borrows for the
/// lookup, with each arm's arity carried beside its index. A `Vec<&FnDecl>`
/// per (name, arity) was an allocation for a list of two or three references,
/// and a tuple key cannot be probed with a borrowed name the way a `&str` can.
struct LiteralGroups<'a> {
    ranges: HashMap<&'a str, (u32, u32)>,
    flat: Vec<(usize, u32)>,
    fns: &'a [FnDecl],
}

impl<'a> LiteralGroups<'a> {
    fn of(program: &'a Program) -> LiteralGroups<'a> {
        let mut ranges: HashMap<&str, (u32, u32)> =
            HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
        for decl in &program.fns {
            ranges.entry(decl.name.as_str()).or_insert((0, 0)).1 += 1;
        }
        let mut at = 0;
        for slot in ranges.values_mut() {
            let count = slot.1;
            *slot = (at, at);
            at += count;
        }
        let mut flat = vec![(0usize, 0u32); program.fns.len()];
        for (i, decl) in program.fns.iter().enumerate() {
            let slot = ranges.get_mut(decl.name.as_str()).expect("every arm was counted");
            flat[slot.1 as usize] = (decl.params.len(), i as u32);
            slot.1 += 1;
        }
        LiteralGroups { ranges, flat, fns: &program.fns }
    }

    /// The arms of `name` that take `arity` parameters, in declaration order.
    fn arms(&self, name: &str, arity: usize) -> impl Iterator<Item = &FnDecl> + '_ {
        let (start, end) = self.ranges.get(name).copied().unwrap_or((0, 0));
        self.flat[start as usize..end as usize]
            .iter()
            .filter(move |(a, _)| *a == arity)
            .map(|(_, i)| &self.fns[*i as usize])
    }

    fn has(&self, name: &str, arity: usize) -> bool {
        self.arms(name, arity).next().is_some()
    }
}

/// What each builtin will accept in each position, verified by running the
/// wrong thing at each one and reading the diagnostic it gives. Absent
/// entries are unconstrained: this says nothing it has not checked.
fn builtin_demand(name: &str, index: usize) -> Option<&'static [LitKind]> {
    use LitKind::*;
    let table: &[(&str, &[&[LitKind]])] = &[
        ("length", &[&[Str, List, Map]]),
        ("join", &[&[List], &[Str]]),
        ("entries", &[&[Map]]),
        ("keys", &[&[Map]]),
        ("values", &[&[Map]]),
        ("bytes", &[&[Str]]),
        ("to_bytes", &[&[List]]),
        ("char_code", &[&[Str]]),
        ("chars", &[&[Str]]),
        ("split", &[&[Str], &[Str]]),
        ("sqrt", &[&[Int, Float]]),
        ("bit_and", &[&[Int], &[Int]]),
        ("bit_or", &[&[Int], &[Int]]),
        ("bit_xor", &[&[Int], &[Int]]),
        ("bit_not", &[&[Int]]),
        ("bit_shl", &[&[Int], &[Int]]),
        ("bit_shr", &[&[Int], &[Int]]),
        ("round", &[&[Int, Float]]),
        ("print", &[&[Str]]),
        ("write", &[&[Str]]),
        ("write_err", &[&[Str]]),
        ("env", &[&[Str]]),
        ("exists", &[&[Str]]),
        ("is_dir", &[&[Str]]),
        ("modified", &[&[Str]]),
        ("list_dir", &[&[Str]]),
        ("read_bytes", &[&[Str]]),
        ("read_file", &[&[Str]]),
        ("run", &[&[Str], &[List]]),
        ("start", &[&[Str], &[List]]),
        ("kill", &[&[Int]]),
        ("make_dir", &[&[Str]]),
        ("write_file", &[&[Str], &[Str]]),
        ("listen", &[&[Int]]),
        ("net_port", &[&[Int]]),
        ("accept", &[&[Int]]),
        ("net_read", &[&[Int]]),
        ("net_read_bytes", &[&[Int]]),
        ("dial", &[&[Str], &[Int]]),
        ("net_secure", &[&[Int], &[Str]]),
        ("net_write", &[&[Int], &[Str]]),
        ("net_close", &[&[Int]]),
        ("concat", &[&[List], &[List]]),
        ("push", &[&[List]]),
        ("put", &[&[Map, List]]),
        ("delete", &[&[Map]]),
        ("sleep", &[&[Int]]),
        ("random", &[&[Int]]),
        ("from_code", &[&[Int]]),
    ];
    table.iter().find(|(n, _)| *n == name).and_then(|(_, positions)| positions.get(index)).copied()
}

/// What a literal is, coarsely enough to compare against a pattern.
#[derive(Clone, Copy, PartialEq)]
enum LitKind {
    Int,
    Float,
    Str,
    List,
    Map,
}

fn literal_kind(e: &Expr) -> Option<LitKind> {
    match e {
        Expr::Int(..) => Some(LitKind::Int),
        Expr::Float(..) => Some(LitKind::Float),
        Expr::Str(parts, _) => match parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))) {
            // an interpolation can render anything; only a plain string is a
            // literal for this purpose
            true => Some(LitKind::Str),
            false => None,
        },
        Expr::List(..) => Some(LitKind::List),
        Expr::MapLit(..) => Some(LitKind::Map),
        _ => None,
    }
}

/// Could this pattern accept a literal of this kind? Unsure counts as yes:
/// the check only fires where every arm is a definite no.
fn pattern_admits(p: &Pattern, kind: LitKind, types: &HashMap<&str, &TypeDecl>) -> bool {
    match p {
        Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => true,
        Pattern::IntLit(..) => kind == LitKind::Int,
        Pattern::StrLit(..) => kind == LitKind::Str,
        Pattern::Nullary(..) => false,
        Pattern::Ctor { .. } => false,
        Pattern::Annotated { ty, .. } => type_admits(ty, kind, types),
    }
}

fn type_admits(ty: &str, kind: LitKind, types: &HashMap<&str, &TypeDecl>) -> bool {
    if ty.ends_with("[]") {
        return kind == LitKind::List;
    }
    if ty.contains('[') {
        return kind == LitKind::Map;
    }
    match ty {
        // a box is never written down as a literal
        t if crate::ast::is_effect_type(t) => false,
        "int" => kind == LitKind::Int,
        // dispatch does not widen: an int literal reaches no float64 arm,
        // whatever arithmetic does with the two together
        "float64" => kind == LitKind::Float,
        "string" => kind == LitKind::Str,
        "bool" | "none" | "done" | "err" => false,
        "some" | "any" => true,
        other => match types.get(other) {
            // a typeset admits what any member admits; a subtype admits what
            // its parent does; a record admits no literal at all
            Some(t) if !t.members.is_empty() => {
                t.members.iter().any(|m| type_admits(m, kind, types))
            }
            Some(t) => match &t.parent {
                Some(parent) => type_admits(parent, kind, types),
                None => false,
            },
            // an unknown name is not something to be confident about
            None => true,
        },
    }
}

/// A call whose argument is a literal the callee cannot take. Rides
/// `per_node_walk`'s descent: its driver was that loop written a second time,
/// down to skipping synthetic declarations and taking each statement's
/// expression the same way.
fn literal_argument_at(
    e: &Expr,
    tables: &PerNode,
    bound: &crate::hash::Set<&str>,
    file: Option<&std::sync::Arc<str>>,
    diags: &mut Vec<Diagnostic>,
) {
    let Expr::App { head, args, .. } = e else { return };
    let Expr::Ident(name, _, _) = &**head else { return };
    // THE CHEAP TEST COMES FIRST, and the shadowing lookup is not one.
    // Every arm below is guarded by `literal_kind(arg)`, so a call
    // with no literal argument can say nothing at all — and asking
    // that is a discriminant match on each argument, where what it
    // skips hashes the callee's name three times over and takes a
    // qualified name apart.
    if !args.iter().any(|arg| literal_kind(arg).is_some()) || bound.contains(name.as_str()) {
        return;
    }
    let groups = &tables.groups;
    let types = &tables.types;
    // a std wrapper is a rename over a builtin, so the builtin's
    // demand is the one that will actually be met
    let alias = tables.builtins.get(&(name.as_str(), args.len())).copied();
    let bare =
        alias.unwrap_or_else(|| crate::ast::split_qual(name).map(|(_, n)| n).unwrap_or(name));
    let bare = bare.strip_prefix("builtin_").unwrap_or(bare);
    // A program that declares its own `run` calls its own `run`,
    // and the arms below say what those take. Without this, every
    // function sharing a bare name with a builtin would inherit
    // the builtin's demands.
    //
    // A std wrapper is a declaration too, and it forwards to the
    // builtin rather than replacing it — so treating it as a
    // shadow silenced the demand for every builtin somebody had
    // written a wrapper over. That is how `print 5` came to be
    // refused at compile time naming its argument while
    // `text/chars 5` answered a bare runtime string: not a
    // decision, an accident of which names have wrappers.
    let forwards = alias.is_some();
    let declared = groups.has(name, args.len());
    // A type the program declares shadows a builtin of that name as a
    // function does: `type run` is built with `run 1 2 3`.
    let shadowed = !forwards && (declared || types.contains_key(name.as_str()));
    for (i, arg) in args.iter().enumerate() {
        let Some(kind) = literal_kind(arg) else { continue };
        let Some(allowed) = builtin_demand(bare, i).filter(|_| !shadowed) else {
            continue;
        };
        if !allowed.contains(&kind) {
            let want: Vec<String> =
                allowed.iter().map(|k| describe_literal(*k).to_string()).collect();
            diags.push(Diagnostic::new(
                "type",
                format!("`{bare}` takes {} here, not {}", or_join(want), describe_literal(kind)),
                arg.span(),
            ));
        }
    }
    if declared {
        for (i, arg) in args.iter().enumerate() {
            let Some(kind) = literal_kind(arg) else { continue };
            let admitted = groups
                .arms(name, args.len())
                .any(|a| a.params.get(i).is_some_and(|p| pattern_admits(p, kind, types)));
            if !admitted {
                let wanted: Vec<String> = groups
                    .arms(name, args.len())
                    .filter_map(|a| a.params.get(i))
                    .map(describe_pattern)
                    .collect();
                // A field read is a call on the field's reader, and the
                // reader's name is the compiler's. Say it the way the
                // runtime does, in the file the read is in: the reader only
                // exists once the module is merged, so the check that finds
                // it is rendering some other file.
                match (getter_field(name), file) {
                    (Some(field), Some(file)) => diags.push(
                        Diagnostic::new(
                            "type",
                            format!(
                                "`.{field}` reads a field of a record, not {}",
                                describe_literal(kind)
                            ),
                            arg.span(),
                        )
                        .about(file),
                    ),
                    _ => diags.push(Diagnostic::new(
                        "type",
                        format!(
                            "no arm of `{name}` takes {} here (arms take {})",
                            describe_literal(kind),
                            dedup_join(wanted)
                        ),
                        arg.span(),
                    )),
                }
            }
        }
    }
}

fn describe_literal(kind: LitKind) -> &'static str {
    match kind {
        LitKind::Int => "an int",
        LitKind::Float => "a float",
        LitKind::Str => "a string",
        LitKind::List => "a list",
        LitKind::Map => "a map",
    }
}

fn describe_pattern(p: &Pattern) -> String {
    match p {
        Pattern::IntLit(n, _) => n.to_string(),
        Pattern::StrLit(s, _) => format!("\"{s}\""),
        Pattern::Nullary(n, _) => n.to_string(),
        Pattern::Ctor { ty, .. } => ty.to_string(),
        Pattern::Annotated { ty, .. } => ty.to_string(),
        Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => "any".to_string(),
    }
}

/// "a list, a map, or a string" — a demand reads as a choice, not a list.
fn or_join(mut names: Vec<String>) -> String {
    names.sort();
    names.dedup();
    match names.len() {
        0 | 1 => names.join(""),
        2 => names.join(" or "),
        n => format!("{}, or {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn dedup_join(mut names: Vec<String>) -> String {
    names.sort();
    names.dedup();
    names.join(", ")
}
