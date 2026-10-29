use crate::ast::*;
use crate::diag::{article, Diagnostic, Span};
use crate::hash::{Map as HashMap, Set as HashSet};
use num_traits::Zero;

pub const BUILTINS: [&str; 81] = [
    "annotate",
    "built_text",
    "append",
    "args",
    "bytes",
    "to_bytes",
    "char_code",
    "chars",
    "concat",
    "entries",
    "keys",
    "values",
    "find2",
    "find2_below",
    "number_span",
    "plain_string",
    "unescape",
    "append_escaped",
    "append_key",
    "append_string",
    "entry_count",
    "key_at",
    "value_at",
    "from_code",
    "list_dir",
    "now",
    "if",
    "is_desc",
    "effect",
    "join",
    "length",
    "print",
    "push",
    "put",
    "delete",
    "random",
    "read_bytes",
    "read_file",
    "read_line",
    "render_value",
    "rescue",
    "run",
    "start",
    "kill",
    "round",
    "sleep",
    "slice",
    "rest_from",
    "slice_in",
    "split",
    "sqrt",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "bit_shl",
    "bind",
    "bit_shr",
    "env",
    "exists",
    "is_dir",
    "modified",
    "stdin",
    "to_float",
    "to_int",
    "utf8",
    "wrap_err",
    "write",
    "write_err",
    "make_dir",
    "write_file",
    "listen",
    "net_port",
    "accept",
    "net_read",
    "net_read_bytes",
    "dial",
    "net_write",
    "net_close",
    "tie",
    "tie_ref",
];

/// The bare-name subset: what resolves without an import. Everything else
/// in BUILTINS is internal, reached only through std wrapper modules. The
/// three chain words are here because a chain step is written wherever an
/// effect is, and importing a module to spell one would be a tax on the
/// failure channel.
pub const AMBIENT: [&str; 14] = [
    "annotate", "bind", "delete", "effect", "entries", "if", "keys", "length", "merge", "print",
    "push", "put", "rescue", "values",
];

/// What each builtin takes. `if` is absent: its count is checked where its
/// branches are, because a guard form spells the same word with a different
/// shape.
///
/// The native backend carried these counts beside its emit list and tested
/// them in the backend, which is late and is one engine. A count the front
/// door does not read is a count three engines answer three ways: the
/// interpreter ran `length x x` in an unreached function and printed the
/// program's output, the native backend refused the whole program with
/// `native backend: `length` takes 1 argument(s)` and no span, the page
/// died at the call, and `kanso check` said ok. So the counts live here,
/// beside the names, and every reader takes them from one place.
pub const BUILTIN_ARITY: [(&str, usize); 86] = [
    ("accept", 1),
    ("annotate", 2),
    ("append", 2),
    ("append_escaped", 2),
    ("append_key", 2),
    ("append_string", 2),
    ("args", 0),
    ("at", 2),
    ("bind", 2),
    ("bit_and", 2),
    ("bit_not", 1),
    ("bit_or", 2),
    ("bit_shl", 2),
    ("bit_shr", 2),
    ("bit_xor", 2),
    ("built_text", 1),
    ("bytes", 1),
    ("char_code", 1),
    ("chars", 1),
    ("concat", 2),
    ("delete", 2),
    ("dial", 2),
    ("effect", 1),
    ("entries", 1),
    ("entry_count", 1),
    ("env", 1),
    ("exists", 1),
    ("filter", 2),
    ("find2", 4),
    ("find2_below", 5),
    ("from_code", 1),
    ("is_desc", 1),
    ("is_dir", 1),
    ("join", 2),
    ("key_at", 2),
    ("keys", 1),
    ("kill", 1),
    ("length", 1),
    ("list_dir", 1),
    ("listen", 1),
    ("make_dir", 1),
    ("map", 2),
    ("merge", 2),
    ("modified", 1),
    ("net_close", 1),
    ("net_port", 1),
    ("net_read", 1),
    ("net_read_bytes", 1),
    ("net_write", 2),
    ("now", 0),
    ("number_span", 2),
    ("plain_string", 2),
    ("print", 1),
    ("push", 2),
    ("put", 3),
    ("random", 1),
    ("read_bytes", 1),
    ("read_file", 1),
    ("read_line", 0),
    ("render_value", 1),
    ("rescue", 2),
    ("rest_from", 2),
    ("round", 1),
    ("run", 2),
    ("sleep", 1),
    ("slice", 3),
    ("slice_in", 3),
    ("sort", 1),
    ("split", 2),
    ("sqrt", 1),
    ("start", 2),
    ("stdin", 0),
    ("sum", 1),
    ("tie", 3),
    ("tie_ref", 3),
    ("to_bytes", 1),
    ("to_float", 1),
    ("to_int", 1),
    ("unescape", 3),
    ("utf8", 1),
    ("value_at", 2),
    ("values", 1),
    ("wrap_err", 2),
    ("write", 1),
    ("write_err", 1),
    ("write_file", 2),
];

/// What a builtin takes, under either spelling. A std wrapper module reaches
/// a native through the `builtin_` prefix, and that call is a call.
///
/// `BUILTIN_ARITY` is in alphabetical order so this can binary-search it:
/// six comparisons where a scan does thirty-one, allocating nothing and
/// holding nothing. `a_builtin_table_a_binary_search_can_read` keeps the
/// order honest — an unsorted table would make this miss in silence.
///
/// The front end asks it of every head no declaration and no binding claims,
/// which is where the cost is; the backends ask it once per emitted call.
///
/// Four shapes were measured on `kanso check lib/json`, which asks this 335
/// times, against a container reading 58,201,174 for main: a scan of the
/// table 58,342,747, this 58,311,708, a map built per `check_merged` call
/// 58,271,359, a map built once for the process 58,232,625.
///
/// The last is the fastest and it is not the one that shipped. The map it
/// holds is 3,216 bytes the process never gives back, which is a
/// compile_peak_bytes term, and welfare prices a byte held about five times
/// what it prices an instruction spent: that shape put the number under its
/// floor and this one leaves it where it was. design/compiler-log.md,
/// 2026-08-29, carries the readings.
pub fn builtin_arity(name: &str) -> Option<usize> {
    let bare = name.strip_prefix("builtin_").unwrap_or(name);
    BUILTIN_ARITY.binary_search_by(|(b, _)| (*b).cmp(bare)).ok().map(|at| BUILTIN_ARITY[at].1)
}

pub fn check(program: &mut Program, require_entry: bool) -> Vec<Diagnostic> {
    let markers = marker_names(program);
    let type_names = program.types.iter().map(|t| t.name.clone()).collect();
    let mut diags = resolve_markers(program, &markers);
    diags.extend(check_typesets(program, &type_names, &HashSet::default()));
    diags.extend(check_file(program, &HashSet::default()));
    if require_entry {
        check_entry(program, &mut diags);
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

/// An effect is a description, and a description that nobody forces never
/// happens. The language already refuses the neighbouring mistake — a body
/// that does io must hand the io back, or it "would abandon the effects above
/// it" — but that reads a function's own body, so an effect abandoned inside
/// somebody else's parameter walked straight past it.
///
/// Only the unambiguous case is refused: every arm of the group discards the
/// position, so there is no reading of the program where the effect happens.
/// A group where one arm reads the parameter and another does not is a real
/// question and a different one.
/// `err` names a raise, and a raise records where it happened. Handed over as
/// a value the record has nowhere to come from: the interpreter attributes it
/// to whatever line applies it, deep inside a callee the author never wrote,
/// and a compiled wrapper has only its own definition to point at. A lambda
/// puts the mention back in the program — `(reason -> err reason)` reads the
/// same and carries a line a reader can open.
/// `flag == true` asks a question that already has its answer in `flag`, and
/// `flag == false` is `not flag` written the long way. Neither can be anything
/// but noise, and kanso has no linter layer for noise to live in, so what a
/// linter elsewhere would warn about is refused here.
/// An `if` is a condition and two branches, and inference indexes all three,
/// so the shape is refused before inference ever runs.
/// ONE DESCENT, MANY CHECKS.
///
/// Each check gathered here visits every expression of every non-synthetic
/// declaration and asks one cheap question of the node it is standing on.
/// They used to do that separately, so the program's syntax tree was walked
/// once per check: the stack traffic and the child enumeration paid for
/// again and again to answer questions that could all be asked at the same
/// node. A bare descent over the compile corpus and the entry corpus costs
/// 1,147,185 instructions, measured with a walk that does no per-node work
/// at all, and every check added here stops paying one.
///
/// WHAT MAY JOIN. A check belongs here when its per-node question is a
/// function of the node alone. It need not descend into every child: a check
/// that visits fewer nodes joins by carrying a flag, and the walk keeps
/// descending for everybody else. `check_decidable_failures` was written up
/// here as the counter-example that could not join, on the grounds that it
/// PRUNES — taking only the condition of an `if`, because a guarded branch
/// may be unreachable and refusing it would refuse a program that runs. That
/// is a reason to stop ASKING at the branches, not a reason to stop walking
/// them, and `shapes_walk` already carried `raised` for the same shape. It
/// joined as `decidable_failure_at` under a `decidable` flag.
///
/// What still cannot join is a question about something other than the node:
/// a check that needs its own bindings in scope, or one whose answer depends
/// on where in the declaration it is standing.
///
/// ORDER. The diagnostics interleave now where they used to arrive in
/// check-sized runs, and that is safe because every route sorts by span
/// before rendering. The sort is stable, so two diagnostics at one line and
/// column would keep the order they were pushed in — the error corpus is
/// what pins that, and no pair here shares a node.
///
/// ONLY THE ARITY ANSWER GATES INFERENCE. The caller returns without running
/// inference when this walk refuses an `if`, because inference indexes an
/// `if`'s three children and must not run over a shape that has fewer. That
/// guarantee is `if_arity_at`'s alone. The other two questions have nothing
/// to do with it, and when their diagnostics shared that gate a program whose
/// only fault was `b == true` returned from it and skipped every check after
/// — the boolean naming rule, the call arities, the field-existence check,
/// all of it, silently. The caller reads the arity answer back off the
/// diagnostic's own kind, and rotates the walk's other two answers to the
/// back. Handing this walk a SECOND vector to separate them at the source is
/// the shape to avoid: it carries a third pointer through every node of every
/// declaration to answer a question asked once per compile, and the reading
/// that says what that costs is CI's, since this row moves with the layout.
/// The tables the fused walk carries. One reference rather than four: the
/// walk is called once per node of every declaration and each table it
/// gained would otherwise be another pointer in the frame.
struct PerNode<'a> {
    arities: crate::hash::Map<&'a str, usize>,
    groups: LiteralGroups<'a>,
    types: HashMap<&'a str, &'a TypeDecl>,
    builtins: &'a HashMap<(&'a str, usize), &'a str>,
    scan: FieldScan<'a>,
    /// The three questions the name walk asked on its own descent. One
    /// reference, like the four above it, because `Named` was already the
    /// struct those three shared.
    named: Named<'a>,
}

/// What the fused walk knows about where it is standing. Three independent
/// answers, each turned off by a different shape, so they travel together
/// rather than as three arguments.
#[derive(Clone, Copy)]
struct Flags {
    /// A decidable failure inside a guarded branch is not one the program
    /// meets. Off inside an `if`'s two arms.
    decidable: bool,
    /// `err` raises, and the head of the call that raises is not a bare
    /// mention. Off for that head alone.
    raised: bool,
    /// A read the program may never perform does not join a run of reads on
    /// one value. Off inside a lambda, an `if` arm, a guard's early exit and
    /// the right operand of `and`/`or`.
    certain: bool,
}

/// The per-declaration state the walk carries, cleared and refilled once per
/// declaration rather than allocated per node.
struct DeclState<'a> {
    /// Names the declaration binds — parameters and top-level binds.
    bound: crate::hash::Set<&'a str>,
    /// Locals whose type is plain to read off the initialiser.
    local: HashMap<&'a str, &'a str>,
    /// The fields each value has been read for, accumulated as the walk goes
    /// and judged when the declaration ends. This is the vector
    /// `check_per_node`'s doc comment warned about carrying.
    open: Open<'a>,
    /// Whether the declaration in hand is this program's own, which two of the
    /// name questions ask before they answer. Per declaration, so it rides
    /// here rather than in `Flags`, which the walk copies at every node.
    own: bool,
    /// Where the name walk's answers go. NOT the `diags` the rest of the walk
    /// pushes to, and that is the point: `check_named_per_node` used to push
    /// between `check_bare_ambiguity` and `check_binding_patterns`, and the
    /// route hands diagnostics back in push order, so answering them inside a
    /// walk whose answers are rotated to the back would move them. They are
    /// spliced in at the old position instead, and every fixture reads the
    /// same. Held across declarations: `shadowable` addresses it by index.
    named_diags: Vec<Diagnostic>,
    /// Each stretch of `named_diags` one arity question pushed, with the name
    /// it was about — the list that decides whether the bound-name set is
    /// worth building for this declaration. Cleared per declaration.
    shadowable: Vec<(usize, usize, &'a str)>,
    /// The file the declaration in hand came from, for a diagnostic that
    /// would otherwise be placed in whichever file is being rendered.
    file: Option<&'a std::sync::Arc<str>>,
    /// The declaration's parameters: a value one names may come from a module
    /// this one cannot see.
    params: &'a [Pattern],
}

/// Answers `check_named_per_node`'s three questions in its own vector, which
/// the caller splices in where that check used to push.
fn check_per_node<'a>(
    program: &'a Program,
    rewritten: &crate::Rewrites,
    diags: &mut Vec<Diagnostic>,
    builtins: &'a HashMap<(&'a str, usize), &'a str>,
    root: bool,
) -> Vec<Diagnostic> {
    let mut arities: crate::hash::Map<&str, usize> =
        crate::hash::Map::with_capacity_and_hasher(program.fns.len(), Default::default());
    for decl in &program.fns {
        let widest = arities.entry(decl.name.as_str()).or_insert(0);
        *widest = (*widest).max(decl.params.len());
    }
    for decl in &program.types {
        if !decl.members.is_empty() {
            continue;
        }
        let takes = match decl.parent {
            Some(_) => 1,
            None => decl.fields.len(),
        };
        let widest = arities.entry(decl.name.as_str()).or_insert(0);
        *widest = (*widest).max(takes);
    }
    let tables = PerNode {
        arities,
        groups: LiteralGroups::of(program),
        types: program.types.iter().map(|t| (t.name.as_str(), t)).collect(),
        // Borrowed: the map is keyed by an owned name, and looking one up
        // needed a String built from the callee at every call expression.
        builtins,
        named: Named {
            // Construction is positional and complete, and the same seam hid
            // it: a type declared in one file of a module and built in another
            // was checked by nobody, and a foreign type never was. A typeset
            // does not construct, so it is not entered. A subtype takes the
            // one value it wraps and is entered with its parent's name.
            fields: program
                .types
                .iter()
                .filter(|ty| {
                    ty.members.is_empty() && (ty.parent.is_some() || !ty.fields.is_empty())
                })
                .map(|ty| (ty.name.as_str(), (ty.fields.len(), ty.parent.as_deref())))
                .collect(),
            arities: Arities::of(program),
            foreign: program
                .types
                .iter()
                .filter(|ty| {
                    crate::ast::has_slash(&ty.name)
                        && (!ty.fields.is_empty() || ty.parent.is_some())
                })
                .map(|ty| ty.name.as_str())
                .collect(),
            // The short-named clone the loader makes of each imported record,
            // keyed by the short name and answering the qualified one. Built
            // by that short name it is still a construction across the import.
            foreign_short: program
                .types
                .iter()
                .filter(|ty| ty.synthetic && (!ty.fields.is_empty() || ty.parent.is_some()))
                .filter_map(|ty| {
                    let origin = ty.origin.as_deref()?;
                    crate::ast::has_slash(origin).then_some((ty.name.as_str(), origin))
                })
                .collect(),
            annotating: program
                .types
                .iter()
                .filter(|ty| !ty.members.is_empty())
                .map(|ty| ty.name.as_str())
                .collect(),
            rewritten,
        },
        scan: FieldScan {
            program,
            root,
            // an err answers its three readers and an entry its two fields,
            // whatever the program declares
            declared: program
                .types
                .iter()
                .flat_map(|t| t.fields.iter().map(|(f, _, _)| f.as_str()))
                .chain(crate::ast::ERR_READERS)
                .chain(crate::ast::ENTRY_FIELDS)
                .collect(),
            plain: program
                .types
                .iter()
                .filter(|t| t.parent.is_none() && t.members.is_empty() && !t.fields.is_empty())
                .map(|t| (t.name.as_str(), t.fields.iter().map(|(f, _, _)| f.as_str()).collect()))
                .collect(),
        },
    };
    let mut state = DeclState {
        bound: Default::default(),
        local: HashMap::default(),
        open: Vec::new(),
        own: false,
        named_diags: Vec::new(),
        shadowable: Vec::new(),
        file: None,
        params: &[],
    };
    // THE SHADOW SET IS BUILT ONLY WHEN THERE IS SOMETHING TO SHADOW.
    //
    // `arity_at`'s one use of the declaration's bound names is to SUPPRESS a
    // diagnostic: a local binding that shadows a declared group means the
    // call is not that group's. Walking the parameters and every statement to
    // collect those names costs the compile corpus 286,809 instructions, and
    // on a program that compiles it suppresses nothing, because a program
    // that compiles raises no arity diagnostic to suppress.
    //
    // So the walk runs without the set and says what it would say, each
    // stretch of diagnostics one call pushed remembered with the name it was
    // about; the set is built only if that list is non-empty. Same answers,
    // and a clean declaration never builds it. `state.bound` is the walk's own
    // set and is not this one: it holds parameters and top-level binds, where
    // this holds every name bound anywhere in the body.
    let mut shadows: HashSet<&str> = HashSet::default();
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        state.bound.clear();
        state.local.clear();
        state.open.clear();
        state.shadowable.clear();
        state.own = !crate::ast::has_slash(&decl.name) && !imported_arm(decl);
        state.file = Some(&decl.file);
        state.params = &decl.params;
        let (first, first_named) = (diags.len(), state.named_diags.len());
        for p in &decl.params {
            collect_pattern_names(p, &mut state.bound);
        }
        for stmt in &decl.body {
            if let Stmt::Bind { pattern, .. } = stmt {
                collect_pattern_names(pattern, &mut state.bound);
            }
        }
        // Two parameter forms name their type without any inference at all.
        // An annotation says it outright, and a constructor pattern's
        // as-binding is whatever dispatch matched — `r@(rect w h)` reached
        // this arm because the value IS a rect.
        for pattern in &decl.params {
            let named = match pattern {
                Pattern::Annotated { name, ty, .. } => Some((name.as_str(), ty.as_str())),
                Pattern::Ctor { ty, whole: Some(b), .. } => Some((b.0.as_str(), ty.as_str())),
                _ => None,
            };
            if let Some((bind, ty)) = named {
                if let Some((known, _)) = tables.scan.plain.get_key_value(ty) {
                    state.local.insert(bind, known);
                }
            }
        }
        let flags = Flags { decidable: true, raised: true, certain: true };
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            per_node_walk(expr, &tables, &mut state, flags, diags);
            field_reads_after(stmt, &tables.scan, &mut state.open, state.params, diags);
            // Statements are walked in order, so a rebinding of the same name
            // replaces the entry rather than confusing it.
            if let Stmt::Bind { pattern: Pattern::Var(n, _), expr } = stmt {
                match type_in_hand(expr, &tables.scan.plain) {
                    Some(ty) => state.local.insert(n.as_str(), ty),
                    None => state.local.remove(n.as_str()),
                };
            }
        }
        for (base, seen) in &state.open {
            judge_cooccurrence(base, seen, &tables.scan, state.params, diags);
        }
        // This walk reads the merged program, which holds every file of a
        // module, and a diagnostic that names no file is printed with the
        // module's name and no location. Everything said here is about the
        // declaration in hand, so it is placed in that declaration's file.
        place_in(&mut diags[first..], &decl.file);
        place_in(&mut state.named_diags[first_named..], &decl.file);
        if state.shadowable.is_empty() {
            continue;
        }
        shadows.clear();
        for param in &decl.params {
            for_each_param_name(param, &mut |n| {
                shadows.insert(n);
            });
        }
        for stmt in &decl.body {
            bound_in_stmt(stmt, &mut shadows);
        }
        // highest first, so an earlier stretch's indices still address the
        // diagnostics it named
        for (from, to, name) in state.shadowable.iter().rev() {
            if shadows.contains(name) {
                state.named_diags.drain(*from..*to);
            }
        }
    }
    state.named_diags
}

fn per_node_walk<'a>(
    expr: &'a Expr,
    tables: &PerNode<'a>,
    state: &mut DeclState<'a>,
    flags: Flags,
    diags: &mut Vec<Diagnostic>,
) {
    if_arity_at(expr, diags);
    boolean_equality_at(expr, diags);
    none_in_collections_at(expr, diags);
    if flags.decidable {
        decidable_failure_at(expr, diags);
    }
    if flags.raised {
        err_as_value_at(expr, diags);
    }
    call_shaped_at(expr, &tables.arities, &state.bound, diags);
    literal_argument_at(expr, tables, &state.bound, state.file, diags);
    field_read_at(
        expr,
        &tables.scan,
        &state.local,
        &mut state.open,
        flags.certain,
        state.params,
        diags,
    );
    merge_at(expr, &tables.scan, &state.local, diags);
    named_at(expr, &tables.named, state.own, &mut state.named_diags, &mut state.shadowable);
    // Every child of every arm below is exactly what `for_each_child` yields
    // for that shape, in the same order. The arms exist to turn a flag off for
    // some of those children, never to change which children are walked: an
    // arm that dropped one would take the other seven questions with it.
    let down = |f: Flags| Flags { raised: true, ..f };
    match expr {
        // An `if` arm and the remainder below a guard are statement lists,
        // walked in order with the arm's flags.
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                let inner = match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                };
                per_node_walk(inner, tables, state, down(flags), diags);
            }
            return;
        }
        // A lambda may never be called, so a read inside it is not certain.
        Expr::Lambda { body, .. } => {
            per_node_walk(body, tables, state, Flags { certain: false, ..down(flags) }, diags);
            return;
        }
        // An `if` guards its branches, and `and`/`or` are written as one — so
        // a branch may be unreachable, and a decidable failure inside one is
        // not a failure the program will meet. examples/logical_ops.kso
        // demonstrates exactly that: `2 < 1 and 1 / 0 < 9` never divides. The
        // condition itself is unguarded. The other questions are about the
        // node alone and keep descending into all three children, which is why
        // the flags turn off here rather than the walk stopping.
        Expr::App { head, args, .. }
            if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == "if") && args.len() == 3 =>
        {
            let arm = Flags { decidable: false, certain: false, ..down(flags) };
            per_node_walk(head, tables, state, down(flags), diags);
            per_node_walk(&args[0], tables, state, down(flags), diags);
            per_node_walk(&args[1], tables, state, arm, diags);
            per_node_walk(&args[2], tables, state, arm, diags);
            return;
        }
        // The head of a call spelled `err` is the raise itself, not a bare
        // mention of it.
        Expr::App { head, args, .. }
            if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == "err") && !args.is_empty() =>
        {
            per_node_walk(head, tables, state, Flags { raised: false, ..flags }, diags);
            for arg in args {
                per_node_walk(arg, tables, state, down(flags), diags);
            }
            return;
        }
        Expr::Guard { cond, early, rest, .. } => {
            let taken = Flags { certain: false, ..down(flags) };
            per_node_walk(cond, tables, state, down(flags), diags);
            per_node_walk(early, tables, state, taken, diags);
            for stmt in rest.iter() {
                let inner = match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
                };
                per_node_walk(inner, tables, state, taken, diags);
            }
            return;
        }
        Expr::BinOp { op, lhs, rhs, .. } if flags.certain && (*op == "and" || *op == "or") => {
            per_node_walk(lhs, tables, state, down(flags), diags);
            per_node_walk(rhs, tables, state, Flags { certain: false, ..down(flags) }, diags);
            return;
        }
        _ => {}
    }
    crate::for_each_child(expr, |child| per_node_walk(child, tables, state, down(flags), diags));
}

fn if_arity_at(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    if let Expr::App { head, args, span, .. } = expr {
        if matches!(head.as_ref(), Expr::Ident(name, _, _) if name == "if") && args.len() != 3 {
            diags.push(Diagnostic::new(
                "arity",
                format!(
                    "an `if` takes a condition and two branches, got {} argument(s)",
                    args.len()
                ),
                *span,
            ));
        }
    }
}

fn boolean_equality_at(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    if let Expr::BinOp { op, lhs, rhs, span } = expr {
        if matches!(*op, "==" | "!=") {
            if let Some(word) = boolean_literal(lhs).or_else(|| boolean_literal(rhs)) {
                diags.push(Diagnostic::new("name", said_plainly(op, word), *span));
            }
        }
    }
}

fn boolean_literal(expr: &Expr) -> Option<&'static str> {
    let Expr::Ident(name, _, _) = expr else {
        return None;
    };
    match name.as_str() {
        "true" => Some("true"),
        "false" => Some("false"),
        _ => None,
    }
}

/// Which of the four spellings this is decides what to say instead, and all
/// four have a shorter answer.
fn said_plainly(op: &str, word: &str) -> String {
    let plain = match (op, word) {
        ("==", "true") | ("!=", "false") => "the value itself",
        _ => "`not` the value",
    };
    format!("comparing to `{word}` asks a question the value already answers — write {plain}")
}

/// `[plotted "x" "y" "z"]` is four elements: the function and three strings.
/// A list element is one atom — the language already says so for operators,
/// where `[1 + 2]` is a syntax error rather than three elements — and this
/// says it for application, where the parser was silently obliging instead.
///
/// There is no guessing involved, because both readings have a spelling: `(f
/// x)` calls, and `&f` holds. Refusing the bare form costs a sigil and takes
/// away a shape whose two meanings looked identical.
/// ONE DESCENT, MANY CHECKS — the pair at the end.
///
/// Both refuse a SHAPE that reads two ways: a bare `err` that looks like a
/// value, and a list element that looks like a call. They are next to each
/// other in the run above and stay in that order here, so nothing about
/// which diagnostic comes first changes.
fn collect_pattern_names<'a>(p: &'a Pattern, out: &mut crate::hash::Set<&'a str>) {
    match p {
        Pattern::Var(name, _) => {
            out.insert(name.as_str());
        }
        Pattern::Annotated { name, .. } => {
            out.insert(name.as_str());
        }
        Pattern::Ctor { fields, whole, .. } => {
            if let Some(named) = whole {
                out.insert(named.0.as_str());
            }
            for f in fields {
                collect_pattern_names(f, out);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for e in entries {
                out.insert(e.bind_name.as_str());
            }
        }
        _ => {}
    }
}

/// `err` raises, so a bare mention of it is not a value.
fn err_as_value_at(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    let (Expr::Ident(name, span, _) | Expr::Partial(name, span)) = expr else {
        return;
    };
    if name != "err" {
        return;
    }
    diags.push(Diagnostic::new(
        "name",
        "`err` raises, so it is a call and not a value — an err records the line it \
         was raised on, and a bare mention has no line to record. Write \
         `(reason -> err reason)`"
            .to_string(),
        *span,
    ));
}

fn call_shaped_at(
    expr: &Expr,
    arities: &crate::hash::Map<&str, usize>,
    bound: &crate::hash::Set<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Expr::List(items, _) = expr {
        for (at, item) in items.iter().enumerate() {
            let Expr::Ident(name, span, _) = item else {
                continue;
            };
            let following = items.len() - at - 1;
            if bound.contains(name.as_str()) {
                continue;
            }
            // `if` keeps no count in the builtin table, because its guard form
            // spells the same word with another shape, but in a list it can
            // only be the call: `[if b "x" "y"]` was four elements on the
            // interpreter and a refusal with no line natively
            let takes =
                arities.get(name.as_str()).copied().unwrap_or_else(|| match name.as_str() {
                    "if" => 3,
                    n if AMBIENT.contains(&n) => builtin_arity(n).unwrap_or(0),
                    _ => 0,
                });
            if takes > 0 && following > 0 {
                diags.push(Diagnostic::new(
                    "syntax",
                    format!(
                        "`{name}` takes {takes} argument(s), and a list element is one \
                         atom — this reads as {} separate elements. Write `({name} …)` \
                         to call it, or `&{name}` to hold it",
                        items.len()
                    ),
                    *span,
                ));
            }
        }
    }
}

include!("check/after_infer.rs");

/// Places diagnostics in the file of the declaration they were raised for,
/// unless the check that raised one already named a file.
fn place_in(diags: &mut [Diagnostic], file: &std::sync::Arc<str>) {
    for d in diags {
        if d.file.is_none() {
            d.file = Some(std::sync::Arc::clone(file));
        }
    }
}

/// A lookup answers "not found" with a none, so a collection that could
/// hold one would make every lenient read ambiguous. A record field is
/// different: it is known to exist, so a none there means the value is
/// nothing and nothing else.
fn none_in_collections_at(e: &Expr, diags: &mut Vec<Diagnostic>) {
    fn is_none_lit(e: &Expr) -> bool {
        matches!(e, Expr::Ident(name, _, _) if name == "none")
    }
    match e {
        Expr::List(items, _) => {
            for item in items.iter().filter(|i| is_none_lit(i)) {
                diags.push(Diagnostic::new(
                    "none",
                    "a list cannot hold a none: a lookup answers \"not found\" \
                     with one, so an element would be indistinguishable"
                        .to_string(),
                    item.span(),
                ));
            }
        }
        Expr::MapLit(pairs, _) => {
            for (_, value) in pairs.iter().filter(|(_, v)| is_none_lit(v)) {
                diags.push(Diagnostic::new(
                    "none",
                    "a map cannot hold a none: a lookup answers \"not found\" \
                     with one, so a value would be indistinguishable"
                        .to_string(),
                    value.span(),
                ));
            }
        }
        _ => {}
    }
}

/// Whatever `primitive` a literal expression is, by the same rule the
/// constructor check uses.
fn literal_type(e: &Expr) -> Option<&'static str> {
    match e {
        Expr::Int(..) => Some("int"),
        Expr::Float(..) => Some("float64"),
        Expr::Str(parts, _) => match parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))) {
            true => Some("string"),
            false => None,
        },
        Expr::List(..) => Some("list"),
        Expr::MapLit(..) => Some("map"),
        Expr::Ident(name, _, _) if name == "true" || name == "false" => Some("bool"),
        _ => None,
    }
}

/// What every construction site puts into each field of each type, as the set
/// of literal kinds seen there. This is the supply side of a field's type,
/// and with no annotation it is all the compiler is told.
fn field_supply(program: &Program) -> HashMap<(&str, &str), Vec<(&'static str, Span)>> {
    let mut supply: HashMap<(&str, &str), Vec<(&'static str, Span)>> = HashMap::default();
    let mut stack: Vec<&Expr> = Vec::new();
    for decl in &program.fns {
        stack.clear();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            stack.push(expr);
        }
        while let Some(cur) = stack.pop() {
            if let Expr::App { head, args, .. } = cur {
                if let Expr::Ident(name, _, _) = head.as_ref() {
                    if let Some(ty) = program.types.iter().find(|t| t.name == *name) {
                        if ty.fields.len() == args.len() {
                            for ((field, _, _), arg) in ty.fields.iter().zip(args) {
                                if let Some(kind) = literal_type(arg) {
                                    supply
                                        .entry((ty.name.as_str(), field.as_str()))
                                        .or_default()
                                        .push((kind, arg.span()));
                                }
                            }
                        }
                    }
                }
            }
            crate::for_each_child(cur, |c| stack.push(c));
        }
    }
    supply
}

/// A field's type is not only what construction sites put in — it is also what
/// its uses require. Reading `p.name` into a parameter declared `string` says
/// the field holds strings, and a construction site that put an int there is
/// not a missing annotation but a disagreement between two things the program
/// already states. Reporting it names both, because neither line is wrong on
/// its own and the author is the one who knows which they meant.
fn check_field_conflicts(program: &Program, diags: &mut Vec<Diagnostic>) {
    let supply = field_supply(program);

    // a local bound straight to a constructor is where a field read's owning
    // type is knowable without knowing anything else, the same footing the
    // assignment check stands on
    let mut stack: Vec<&Expr> = Vec::new();
    for decl in &program.fns {
        let mut built: HashMap<&str, &str> = HashMap::default();
        for stmt in &decl.body {
            if let Stmt::Bind { pattern: Pattern::Var(name, _), expr: Expr::App { head, .. } } =
                stmt
            {
                if let Expr::Ident(ty, _, _) = head.as_ref() {
                    if program.types.iter().any(|t| t.name == *ty) {
                        built.insert(name.as_str(), ty.as_str());
                    }
                }
            }
        }
        if built.is_empty() {
            continue;
        }
        stack.clear();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            stack.push(expr);
        }
        while let Some(cur) = stack.pop() {
            if let Expr::App { head, args, .. } = cur {
                if let Expr::Ident(callee, _, _) = head.as_ref() {
                    demand_conflicts(program, &supply, &built, callee, args, diags);
                }
            }
            crate::for_each_child(cur, |c| stack.push(c));
        }
    }
}

/// One call: for each argument that reads a field off a local whose type is
/// known, compare what the callee's parameter demands against what the field
/// is actually given anywhere in the program.
fn demand_conflicts(
    program: &Program,
    supply: &HashMap<(&str, &str), Vec<(&'static str, Span)>>,
    built: &HashMap<&str, &str>,
    callee: &str,
    args: &[Expr],
    diags: &mut Vec<Diagnostic>,
) {
    let Some(target) =
        program.fns.iter().find(|d| d.name == callee && d.params.len() == args.len())
    else {
        return;
    };
    for (param, arg) in target.params.iter().zip(args) {
        let Pattern::Annotated { ty: demanded, .. } = param else { continue };
        let Expr::Field { base, name: field, span } = arg else { continue };
        let Expr::Ident(local, _, _) = base.as_ref() else { continue };
        let Some(owner) = built.get(local.as_str()) else { continue };
        let Some(given) = supply.get(&(*owner, field.as_str())) else { continue };
        for (kind, where_) in given {
            if kind != demanded {
                diags.push(Diagnostic::new(
                    "type",
                    format!(
                        "`{owner}`'s `{field}` is {} here, and `{callee}` takes {} — these \
                         two cannot both hold",
                        article(kind),
                        article(demanded)
                    ),
                    *where_,
                ));
                diags.push(Diagnostic::new(
                    "type",
                    format!("...and this is where it is read as {}", article(demanded)),
                    *span,
                ));
            }
        }
    }
}

include!("check/ties.rs");

/// The engines carry `none` as a tag rather than a declared type, so nothing
/// can derive from it yet. Rejecting here keeps all three engines identical
/// instead of one erroring and another silently dropping the subtype.
///
/// A typeset is refused as a parent for the same reason. It only annotates, so
/// nothing is ever built as one, and the engines disagreed about what wrapping
/// it means: the interpreter took any member, and the native backend looked
/// for the typeset in the value's own chain and refused a member at run time.
fn check_sub_parents(program: &Program, diags: &mut Vec<Diagnostic>) {
    let typeset =
        |name: &str| program.types.iter().any(|t| t.name == name && !t.members.is_empty());
    for ty in &program.types {
        let first = diags.len();
        if let Some(parent) = ty.parent.as_deref().filter(|p| typeset(p)) {
            diags.push(Diagnostic::new(
                "type",
                format!(
                    "`{}` cannot wrap the typeset `{parent}` yet — a typeset only \
                     annotates, so wrap one of its members",
                    ty.name
                ),
                ty.span,
            ));
        }
        if ty.parent.as_deref() == Some("none") {
            diags.push(Diagnostic::new(
                "type",
                format!(
                    "`{}` cannot derive from `none` yet — name the members you \
                     mean, or use a zero-field marker type",
                    ty.name
                ),
                ty.span,
            ));
        }
        if !ty.file.is_empty() {
            place_in(&mut diags[first..], &ty.file);
        }
    }
}

/// Zero-field type names: a bare mention is the marker value, and in
/// parameter position the bare name matches that marker.
pub fn marker_names(program: &Program) -> HashSet<String> {
    program
        .types
        .iter()
        .filter(|t| t.fields.is_empty() && t.parent.is_none() && t.members.is_empty())
        .map(|t| t.name.clone())
        .collect()
}

/// Rewrites bare marker names in parameter position into zero-field
/// constructor patterns, and rejects constructor calls that pass a marker
/// fields — a marker's bare mention is its value.
pub fn resolve_markers(program: &mut Program, markers: &HashSet<String>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for decl in &mut program.fns {
        for param in &mut decl.params {
            resolve_marker_pattern(param, markers, &mut diags);
        }
        for stmt in &mut decl.body {
            match stmt {
                Stmt::Bind { expr, .. } => check_marker_calls(expr, markers, &mut diags),
                Stmt::Expr(expr) => check_marker_calls(expr, markers, &mut diags),
            }
        }
    }
    diags
}

fn resolve_marker_pattern(
    pattern: &mut Pattern,
    markers: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    match pattern {
        Pattern::Var(name, _) if markers.contains(name.as_str()) => {
            *pattern = Pattern::Ctor { ty: name.clone(), fields: Vec::new(), whole: None };
        }
        Pattern::Ctor { ty, fields, .. } => {
            if markers.contains(ty.as_str()) && !fields.is_empty() {
                diags.push(Diagnostic::new(
                    "signature",
                    format!("`{ty}` takes no fields; its bare mention is its value"),
                    other_span(&fields[0]),
                ));
            }
            for field in fields {
                resolve_marker_pattern(field, markers, diags);
            }
        }
        _ => {}
    }
}

fn check_marker_calls(expr: &Expr, markers: &HashSet<String>, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) | Expr::Ident(..) | Expr::Partial(..) => {}
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                        check_marker_calls(expr, markers, diags)
                    }
                }
            }
        }
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                check_marker_calls(key, markers, diags);
                check_marker_calls(value, markers, diags);
            }
        }
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    check_marker_calls(inner, markers, diags);
                }
            }
        }
        Expr::List(items, _) => {
            for item in items {
                check_marker_calls(item, markers, diags);
            }
        }
        Expr::App { head, args, .. } => {
            if let Expr::Ident(name, span, _) = &**head {
                if markers.contains(name.as_str()) && !args.is_empty() {
                    diags.push(Diagnostic::new(
                        "signature",
                        format!("`{name}` takes no fields; its bare mention is its value"),
                        *span,
                    ));
                }
            }
            check_marker_calls(head, markers, diags);
            for arg in args {
                check_marker_calls(arg, markers, diags);
            }
        }
        Expr::Field { base, .. } => check_marker_calls(base, markers, diags),
        Expr::Upcast { expr, .. } => check_marker_calls(expr, markers, diags),
        Expr::Index { base, index, .. } => {
            check_marker_calls(base, markers, diags);
            check_marker_calls(index, markers, diags);
        }
        Expr::Lambda { body, .. } => check_marker_calls(body, markers, diags),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            check_marker_calls(lhs, markers, diags);
            check_marker_calls(rhs, markers, diags);
        }
        Expr::Guard { cond, early, rest, .. } => {
            check_marker_calls(cond, markers, diags);
            check_marker_calls(early, markers, diags);
            for stmt in rest.iter() {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                        check_marker_calls(expr, markers, diags)
                    }
                }
            }
        }
    }
}

/// Builtin type words legal as typeset members alongside declared types.
const TYPESET_BUILTINS: [&str; 6] = ["bool", "done", "float64", "int", "none", "string"];

/// A multi-member field typeset enumerates concrete types: each member must
/// name a declared type or a builtin type word.
pub fn check_typesets(
    program: &Program,
    type_names: &HashSet<String>,
    more_type_names: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for ty in &program.types {
        for (field, tys, span) in &ty.fields {
            if tys.len() < 2 {
                continue;
            }
            for member in tys {
                let known = TYPESET_BUILTINS.contains(&member.as_str())
                    || type_names.contains(member.as_str())
                    || more_type_names.contains(member.as_str());
                if !known {
                    diags.push(Diagnostic::new(
                        "name",
                        format!(
                            "`{member}` in the typeset of field `{field}` names no \
                             declared or builtin type"
                        ),
                        *span,
                    ));
                }
            }
        }
    }
    diags
}

/// Names a file declares, for the module-wide first pass.
pub fn declared_names(program: &Program) -> HashSet<&str> {
    let mut names = HashSet::default();
    for ty in &program.types {
        names.insert(ty.name.as_str());
    }
    for decl in &program.fns {
        names.insert(decl.name.as_str());
    }
    names
}

/// Per-file checks: canonical order plus name resolution against this file's
/// globals extended with the rest of the module.
pub fn check_file(program: &Program, extern_globals: &HashSet<&str>) -> Vec<Diagnostic> {
    // A caller that cannot rename a traveling call has nowhere to send it, so
    // here it stays the unknown name it was before arms traveled.
    let mut travels = Vec::new();
    let mut diags = check_file_shadow(
        program,
        extern_globals,
        &HashSet::default(),
        &HashSet::default(),
        &[],
        &HashMap::default(),
        &mut travels,
    );
    // A call of a function the file declares was resolved where it stands.
    for (name, span, shaded) in travels {
        if shaded.is_none() {
            let name = name.as_str();
            diags.push(Diagnostic::new(
                "name",
                format!("unknown name `{name}`{}", moved_name(name)),
                span,
            ));
        }
    }
    diags.sort_by_key(|d| (d.span.line, d.span.col));
    diags
}

/// Merged-namespace coherence for a directory module.
/// The `?` sigil is enforced through the return-set lattice, both ways: a
/// `?` name must answer with true/false (err allowed — a predicate over
/// fallible work stays a predicate), and a group answering only in booleans
/// must carry the `?`. Mixed sets are exempt in the unmarked direction.
fn check_predicates(
    program: &Program,
    inference: &crate::infer::Inference,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::infer::{ERR, FALSE, TRUE};
    // inference is handed in: one pass serves every check that reads it
    // The file travels with the span: in a module the groups come from every
    // file, and a span alone does not say which one it is in.
    type At<'a> = (crate::infer::Set, crate::diag::Span, &'a std::sync::Arc<str>);
    let mut groups: crate::hash::Map<&str, At> = crate::hash::Map::default();
    for (i, decl) in program.fns.iter().enumerate() {
        // test functions are assertions the test verb consumes — their own
        // convention, not questions
        let short = crate::ast::split_qual(&decl.name).map(|(_, s)| s).unwrap_or(&decl.name);
        // a getter takes its field's name, so this rule would reach past the
        // function and rename the field — a language question (should a
        // boolean field be `drained?`) that is not this check's to answer
        if decl.synthetic
            || decl.is_getter()
            || decl.name == crate::ast::ENTRY
            || short.starts_with("test_")
            // an arm extending an operator is named for the operator, and
            // `<` cannot take a `?` — this rule is about identifiers
            || matches!(short, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
        {
            continue;
        }
        let entry = groups.entry(decl.name.as_str()).or_insert((0, decl.span, &decl.file));
        entry.0 |= inference.returns[i];
    }
    // a HashMap hands these back in a different order every run, which makes
    // a multi-diagnostic file's output unstable; report in source order
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(name, (_, span, _))| (span.line, span.col, *name));
    for (name, (set, span, file)) in groups {
        let short = crate::ast::split_qual(name).map(|(_, s)| s).unwrap_or(name);
        let is_question = short.ends_with('?');
        // only err rides along (a predicate over fallible work is still a
        // predicate). none is not an answer a question may give — when the
        // static absence checker lands, a none-bearing `?` set becomes a
        // compile error; until then a none-polluted set is simply not
        // provably boolean and the unmarked direction stays quiet.
        let boolish = set != 0 && set & !(TRUE | FALSE | ERR) == 0 && set & (TRUE | FALSE) != 0;
        // the marked direction fires only on a provable lie: a `?` group
        // whose set cannot contain a boolean at all. generic drivers widen
        // honest predicates to TOP, and TOP still holds bool.
        if is_question && set != 0 && set & (TRUE | FALSE) == 0 {
            diags.push(
                Diagnostic::new(
                    "naming",
                    format!(
                        "`{short}` asks a question: a `?` function answers \
                     true or false (err may ride along)"
                    ),
                    span,
                )
                .about(file),
            );
        }
        if !is_question && boolish {
            diags.push(
                Diagnostic::new(
                    "naming",
                    format!("`{short}` answers only true or false: name it `{short}?`"),
                    span,
                )
                .about(file),
            );
        }
        // The other suffix, ruled in July and again on 2026-09-03: a `!` name's
        // answer typeset must include an err type. A bang that cannot fail is a
        // lie in the name — `foo[k]!` differs from `foo[k]` precisely by being
        // able to fail, so a declaration wearing the mark and answering a plain
        // value tells the reader the opposite of what it does.
        //
        // Fires on a provable lie only, the same conservatism the `?` direction
        // takes: an empty set means inference learned nothing, and TOP (what a
        // generic driver widens to) still holds ERR, so neither is accused.
        if short.ends_with('!') && set != 0 && set & crate::infer::DESC == 0 {
            diags.push(
                Diagnostic::new(
                    "naming",
                    format!(
                        "`{short}` wears a bang: a `!` function answers an effect, \
                     the box a failure bubbles through"
                    ),
                    span,
                )
                .about(file),
            );
        }
    }
}

/// The record type this expression hands back, when that is plain to read off
/// the expression itself. Anything else answers `None` and the read stays as
/// it was.
///
/// Two expressions say it plainly. A construction names the type as the head
/// of the call. A widening names it after the colon — `(v):point` hands back a
/// point and says so — and until this read it, `p = (v):point` followed by
/// `p.z` compiled clean where the same field read is refused through an
/// annotation, through a constructor pattern's as-binding, and through a local
/// construction. Three ways of naming a type reached the table and the fourth
/// did not; both engines then refused `p.z` at run time, identically, so this
/// was a diagnostic arriving late rather than a disagreement.
fn type_in_hand<'a>(e: &'a Expr, plain: &HashMap<&'a str, HashSet<&'a str>>) -> Option<&'a str> {
    if let Expr::Upcast { ty, .. } = e {
        return plain.get_key_value(ty.as_str()).map(|(k, _)| *k);
    }
    let Expr::App { head, .. } = e else { return None };
    let Expr::Ident(name, _, _) = head.as_ref() else { return None };
    plain.get_key_value(name.as_str()).map(|(k, _)| *k)
}

/// The fields one value is read for, in the order the body reads them.
type Open<'a> = Vec<(&'a str, Vec<(&'a str, Span)>)>;

/// What a field read is checked against, gathered once for the whole program.
struct FieldScan<'a> {
    program: &'a Program,
    /// Every field name any type declares, so a name none declares is refused
    /// wherever it appears.
    declared: HashSet<&'a str>,
    /// Whether this program is the whole of what runs. A field is read without
    /// importing the type that owns it (ch03, #1894), so a library checked
    /// before anything imports it may read, on a value it was handed, a field
    /// only an importer's types declare, as it may call a function only they
    /// write. The root answers for a name no type in the program declares.
    root: bool,
    /// A plain record constructs its own fields: no parent to inherit from, no
    /// typeset members to stand for something else. Only those can say with
    /// certainty which fields a value of theirs has, and certainty is the whole
    /// licence for refusing before anything runs.
    plain: HashMap<&'a str, HashSet<&'a str>>,
}

/// Whether a pattern binds this name, without building the set of every name
/// it binds — the caller asks about the few names it has open.
fn pattern_binds(pattern: &Pattern, name: &str) -> bool {
    match pattern {
        Pattern::Var(bound, _) | Pattern::Annotated { name: bound, .. } => bound == name,
        Pattern::Ctor { fields, whole, .. } => {
            whole.as_ref().is_some_and(|w| w.0 == name)
                || fields.iter().any(|f| pattern_binds(f, name))
        }
        Pattern::Keyed { entries, .. } => entries.iter().any(|e| e.bind_name == name),
        _ => false,
    }
}

fn note_read<'a>(open: &mut Open<'a>, base: &'a str, name: &'a str, span: Span) {
    match open.iter_mut().find(|(b, _)| *b == base) {
        Some((_, seen)) => {
            if !seen.iter().any(|(f, _)| *f == name) {
                seen.push((name, span));
            }
        }
        None => open.push((base, vec![(name, span)])),
    }
}

/// Whether a value is read for fields that no one record declares together.
///
/// A value read for two fields must be some one record, and a record the
/// program declares — the interpreter reads `.` off nothing else. So when no
/// declaration holds every field read off a value, those reads cannot all be
/// of the same thing, and the program is wrong whatever type the value would
/// have had. That refusal needs no inference behind it, and it catches the
/// confusion between two record types, which the single-read fence cannot
/// see: both names are declared, just never on the same record.
///
/// Measured over the fleet before it was built: thirty values are read for
/// two or more fields, every one of them determined by that set alone, and a
/// confused field name is refused in 93.8% of substitutions.
fn judge_cooccurrence(
    base: &str,
    seen: &[(&str, Span)],
    scan: &FieldScan<'_>,
    params: &[Pattern],
    diags: &mut Vec<Diagnostic>,
) {
    // Before the root, a parameter read for a field no type here declares
    // may hold a type an importer declares.
    let unseen = || seen.iter().any(|(name, _)| !scan.declared.contains(name));
    if seen.len() < 2 || (!scan.root && is_param(base, params) && unseen()) {
        return;
    }
    let program = scan.program;
    let holds = |ty: &TypeDecl, without: Option<&str>| {
        seen.iter()
            .filter(|(name, _)| Some(*name) != without)
            .all(|(name, _)| ty.fields.iter().any(|(f, _, _)| f == name))
    };
    if program.types.iter().any(|ty| holds(ty, None)) {
        return;
    }
    // One name is usually the odd one out: drop it and the rest sit on a
    // record together. Point at that read, because it is the one to change.
    let mut odd = None;
    let mut odds = 0;
    for (name, span) in seen {
        if program.types.iter().any(|ty| holds(ty, Some(name))) {
            odd = Some(*span);
            odds += 1;
        }
    }
    let span = match (odds, odd) {
        (1, Some(span)) => span,
        _ => seen[seen.len() - 1].1,
    };
    let quoted: Vec<String> = seen.iter().map(|(name, _)| format!("`{name}`")).collect();
    let message = match quoted.as_slice() {
        [a, b] => format!("`{base}` is read for {a} and {b}, and no record type has both"),
        rest => format!(
            "`{base}` is read for {} and {}, and no record type has all of them",
            rest[..rest.len() - 1].join(", "),
            rest[rest.len() - 1]
        ),
    };
    diags.push(Diagnostic::new("name", message, span));
}

/// The open-list bookkeeping a statement owes AFTER its expression has been
/// walked. Split from the walk because it is the half that is about the
/// STATEMENT rather than about a node: a caller that already descends into
/// the expression for its own reasons needs this half and not the other.
fn field_reads_after<'a>(
    stmt: &'a Stmt,
    scan: &FieldScan<'a>,
    open: &mut Open<'a>,
    params: &[Pattern],
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        // The initialiser is read before the name it binds changes meaning,
        // and from there on the name is a different value.
        Stmt::Bind { pattern, .. } => {
            let mut at = 0;
            while at < open.len() {
                if pattern_binds(pattern, open[at].0) {
                    let (base, seen) = open.remove(at);
                    judge_cooccurrence(base, &seen, scan, params, diags);
                } else {
                    at += 1;
                }
            }
        }
        Stmt::Expr(_) => {}
    }
}

/// Whether `name` is one of the declaration's parameters.
fn is_param(name: &str, params: &[Pattern]) -> bool {
    params.iter().any(|p| matches!(p, Pattern::Var(n, _) if n == name))
}

/// Whether a field read's base is a value the function was handed.
fn handed(base: &Expr, params: &[Pattern]) -> bool {
    matches!(base, Expr::Ident(n, _, _) if is_param(n, params))
}

/// A field read, asked of one node. The walk that carries it decides whether
/// the read is CERTAIN — a read the program may never perform does not join a
/// run of reads on one value, and three shapes defer evaluation: a lambda,
/// which may never be called; the arms of an `if` or a guard, of which one is
/// taken; and the right operand of `and`/`or`, which the left may already have
/// decided. The walk still enters them, because a field no type declares is
/// wrong wherever it is written — it just stops pooling.
fn field_read_at<'a>(
    e: &'a Expr,
    scan: &FieldScan<'a>,
    local: &HashMap<&'a str, &'a str>,
    open: &mut Open<'a>,
    certain: bool,
    params: &[Pattern],
    diags: &mut Vec<Diagnostic>,
) {
    // A dependency's field reads reach the root already rewritten into calls
    // on their readers, so the root asks a reader's call the same question.
    if let Expr::App { head, .. } = e {
        if let Expr::Ident(reader, span, _) = head.as_ref() {
            if let Some(field) = crate::ast::getter_field(reader) {
                if scan.root && !scan.declared.contains(field) {
                    diags.push(Diagnostic::new(
                        "name",
                        format!("no record type has a field `{field}`"),
                        *span,
                    ));
                }
            }
        }
        return;
    }
    let Expr::Field { base, name, span } = e else { return };
    if !scan.declared.contains(name.as_str()) {
        if scan.root || !handed(base, params) {
            diags.push(Diagnostic::new(
                "name",
                format!("no record type has a field `{name}`"),
                *span,
            ));
        }
    } else if let Some(ty) = base_type(base, &scan.plain, local) {
        if !scan.plain[ty].contains(name.as_str()) {
            diags.push(Diagnostic::new("name", format!("`{ty}` has no field `{name}`"), *span));
        }
    }
    if certain {
        if let Expr::Ident(b, _, _) = base.as_ref() {
            note_read(open, b.as_str(), name.as_str(), *span);
        }
    }
}

/// `merge u { "role":"chef" }` copies `u` with the named fields replaced
/// (ruled 2026-10-13, #1886). The rewrite that builds the copy needs every name
/// in hand, so the fields are named by string literals in a map literal. Each
/// one has to be a field of the value's type, and the copy is a construction,
/// so the type has to be one this module declares. Where the type is not
/// known here the question is asked of every type the module declares: one of
/// them has to have every field named, and a value of any other type reaches
/// no arm of the copy.
fn merge_at<'a>(
    e: &'a Expr,
    scan: &FieldScan<'a>,
    local: &HashMap<&'a str, &'a str>,
    diags: &mut Vec<Diagnostic>,
) {
    let Expr::App { head, args, .. } = e else { return };
    let Expr::Ident(name, _, _) = head.as_ref() else { return };
    if name.as_str() != crate::MERGE || args.len() != 2 {
        return;
    }
    let Expr::MapLit(entries, map_span) = &args[1] else {
        diags.push(Diagnostic::new(
            "type",
            "`merge` names the fields it replaces in a map literal: `merge u { \"name\":value }`"
                .to_string(),
            args[1].span(),
        ));
        return;
    };
    if entries.is_empty() {
        diags.push(Diagnostic::new(
            "type",
            "this `merge` replaces no field, so it is the value it was given".to_string(),
            *map_span,
        ));
        return;
    }
    let mut keys: Vec<(&str, Span)> = Vec::new();
    for (key, _) in entries {
        match crate::merge_key(key) {
            Some(k) => keys.push((k, key.span())),
            None => diags.push(Diagnostic::new(
                "type",
                "a field `merge` replaces is named by a plain string literal".to_string(),
                key.span(),
            )),
        }
    }
    if keys.len() != entries.len() {
        return;
    }
    if let Some(ty) = base_type(&args[0], &scan.plain, local) {
        for (key, span) in &keys {
            if !scan.plain[ty].contains(key) {
                diags.push(Diagnostic::new("name", format!("`{ty}` has no field `{key}`"), *span));
            }
        }
        return;
    }
    let mut missing = false;
    for (key, span) in &keys {
        if !scan.declared.contains(key) {
            diags.push(Diagnostic::new(
                "name",
                format!("no record type has a field `{key}`"),
                *span,
            ));
            missing = true;
        }
    }
    if missing {
        return;
    }
    let has_all = |t: &TypeDecl| keys.iter().all(|(k, _)| t.fields.iter().any(|(f, _, _)| f == k));
    let own = scan.program.types.iter().filter(|t| crate::merge_owned(t));
    if own.clone().any(has_all) {
        return;
    }
    let names: Vec<String> = keys.iter().map(|(k, _)| format!("`{k}`")).collect();
    let foreign = scan.program.types.iter().find(|t| !crate::merge_owned(t) && has_all(t));
    let (kind, message) = match foreign {
        Some(t) => {
            let full = t.origin.as_deref().unwrap_or(t.name.as_str());
            let (owner, short) = crate::ast::split_qual(full).unwrap_or(("another module", full));
            let message = format!(
                "`merge` copies a `{short}` only in `{owner}`, the module that declares it; \
                 ask it for the copy through a pub function"
            );
            ("opacity", message)
        }
        None => ("name", format!("no record type here has every field {}", names.join(", "))),
    };
    diags.push(Diagnostic::new(kind, message, *map_span));
}

/// The type of the value this field read is reading from, when the checker can
/// know it: a construction written where it stands, or a local bound to one.
fn base_type<'a>(
    base: &'a Expr,
    plain: &HashMap<&'a str, HashSet<&'a str>>,
    local: &HashMap<&'a str, &'a str>,
) -> Option<&'a str> {
    match base {
        Expr::Ident(n, _, _) => local.get(n.as_str()).copied(),
        other => type_in_hand(other, plain),
    }
}

pub fn check_merged(program: &Program, require_entry: bool) -> Vec<Diagnostic> {
    check_merged_after_aliases(program, require_entry, require_entry, &crate::Rewrites::default())
}

/// The whole-program check, told which call names `canonicalize_bare_aliases`
/// rewrote.
///
/// The entry path runs that pass BEFORE this check, so that the check does not
/// walk the synthetic twins the pass is about to delete. Two of the checks
/// below read a call's name and mean the spelling the program used, so they are
/// handed the record and consult it. Every other caller runs the pass after the
/// check and passes an empty one, where both readers behave exactly as they did.
pub fn check_merged_after_aliases(
    program: &Program,
    require_entry: bool,
    root: bool,
    rewritten: &crate::Rewrites,
) -> Vec<Diagnostic> {
    let builtins = crate::inline::aliases(program);
    check_merged_after_aliases_with(program, require_entry, root, rewritten, builtins)
}

/// The same check, handed an alias map its caller has already built.
///
/// `inline_builtin_wrappers` runs straight after this check on the same
/// program and asks the same question of it. A caller that does both builds
/// the map once and passes it here, rather than paying for the fixpoint and
/// the group count twice over a program neither of them changed.
pub fn check_merged_after_aliases_with<'a>(
    program: &'a Program,
    require_entry: bool,
    root: bool,
    rewritten: &crate::Rewrites,
    builtins: HashMap<(&'a str, usize), &'a str>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Three checks read what inference knows, and inference over a whole
    // program is the most expensive thing the front end does. One pass,
    // handed round.
    let named_diags = check_per_node(program, rewritten, &mut diags, &builtins, root);
    let walked = diags.len();
    if diags.iter().any(|d| d.kind == "arity") {
        // inference indexes an if's branches, so it never runs over a shape
        // the walk above refused. ONLY that refusal stops it, and only the
        // refusal is reported: what `retain` drops here is what the checks
        // below would have said about a program they never got to see.
        diags.retain(|d| d.kind == "arity");
        diags.sort_by_key(|d| (d.span.line, d.span.col));
        return diags;
    }
    let inference = crate::phase::watched("infer", || crate::infer::infer(program));
    // What a dispatch group answers, keyed by name and arity. Three checks
    // below asked this and each built it for itself, in loops identical to
    // the byte; inference is handed round for exactly this reason and the
    // table over it should be too. A fourth build sits in
    // `check_none_exhaustive` and stays there: its entry carries a SECOND
    // fact beside the set -- the positions some arm names a none at, one bit
    // each -- so reading this table instead would leave it hashing the name a
    // second time for the mask, at every call site. That is the cost its own
    // comment weighs, and it is why one richer map beats two.
    let mut returns: HashMap<(&str, usize), crate::infer::Set> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for (i, d) in program.fns.iter().enumerate() {
        *returns.entry((d.name.as_str(), d.params.len())).or_insert(0) |= inference.returns[i];
    }
    let returns = returns;
    check_constants(program, &mut diags);
    check_constant_cycles(program, &HashSet::default(), &mut diags);
    check_predicates(program, &inference, &mut diags);
    check_arm_ties(program, &mut diags);
    check_sub_parents(program, &mut diags);
    check_tie_references(program, &mut diags);
    check_bare_ambiguity(program, &mut diags);
    // Where the name questions' answers go. They are asked inside the one
    // descent now; the route hands diagnostics back in push order, so they are
    // spliced in at the position their own check used to push from.
    diags.extend(named_diags);
    check_binding_patterns(program, &mut diags);
    check_overlapping_arms(program, &mut diags);
    // The three questions inference makes answerable are asked on ONE descent,
    // and their answers are spliced in where each check used to push: the
    // effect answers here, the exhaustiveness and box answers after
    // the rotation.
    let mut effect_diags = Vec::new();
    let mut box_diags = Vec::new();
    let mut none_diags = Vec::new();
    check_after_infer(
        program,
        &inference,
        &returns,
        &builtins,
        &mut effect_diags,
        &mut box_diags,
        &mut none_diags,
    );
    diags.append(&mut effect_diags);
    check_discarded_value(program, &returns, &mut diags);
    // This route hands its diagnostics back in push order — only the gated
    // return above sorts — so where a check pushes is what a reader sees.
    // The walk ran first and its two non-gating questions belong where the
    // checks they replaced used to push, which for `check_boolean_equality`
    // was last of the run. Rotating the walk's answers to the back puts them
    // there and leaves both halves in their own order.
    diags.rotate_left(walked);
    diags.append(&mut none_diags);
    diags.append(&mut box_diags);
    if require_entry {
        check_entry(program, &mut diags);
    }
    diags
}

/// A call's argument count against the arms that could answer it. The
/// per-file pass sees only that file's declarations, so a wrong-arity call
/// to an *imported* group reached codegen unchecked and surfaced as an
/// undefined symbol from the assembler — compiler internals, where a
/// diagnostic belongs. This runs on the merged program, where the
/// dependency's arms are in scope.
/// A binding's pattern naming a type nobody declared. `poynt a = 1` passed
/// check and reached the emitter, which answered `native backend: unknown
/// type` — the backend's own words, for a name the reader typed. The
/// interpreter said something better and different, so the two engines
/// disagreed about a program neither should have accepted.
///
/// Parameter patterns were already covered: an arm taking an undeclared type
/// never matches, and arm selection says so.
fn check_binding_patterns(program: &Program, diags: &mut Vec<Diagnostic>) {
    let declared: HashSet<&str> = program.types.iter().map(|ty| ty.name.as_str()).collect();
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        for stmt in &decl.body {
            let Stmt::Bind { pattern: Pattern::Ctor { ty, fields, .. }, .. } = stmt else {
                continue;
            };
            if declared.contains(ty.as_str()) || fields.is_empty() {
                continue;
            }
            // `let x = 1` reads as destructuring a `let`, which is how every
            // language that spells a binding with a keyword arrives here.
            let bound = first_param_name(&fields[0]).unwrap_or_default();
            let instead = match ty.as_str() {
                "let" | "var" | "const" | "val" => {
                    format!(" — a binding is `{bound} = …`, with no keyword")
                }
                _ => String::new(),
            };
            // Attributed to the declaration in hand. This check runs over the
            // MERGED program, so `decl` can belong to any dependency the
            // compile loaded, and the module path had no way to say which.
            diags.push(
                Diagnostic::new(
                    "name",
                    format!(
                        "`{ty}` is not a type, so a binding cannot destructure with it{instead}"
                    ),
                    other_span(&fields[0]),
                )
                .about(&decl.file),
            );
        }
    }
}

/// GAVEL 1b: the owner's pub functions are the only door, so a value wearing a
/// A typeset is annotation-only vocabulary: `type shape circle square` names
/// a union for a parameter to stand in, and no value is ever a `shape`. So a
/// call whose head names one is a mistake the compiler can see, and until this
/// it was three different answers. The interpreter refused it at run time —
/// "`shape` is a typeset — it only annotates" — the page said the same, and
/// native built a one-field record and PRINTED `shape 1`. A silent divergence
/// on the engine most programs run on.
///
/// Naming one without calling it diverges too, and more quietly: `print
/// "{shape}"` compiles clean, prints `<mod>/shape` on native and `<fn>` on the
/// interpreter. Two engines, two answers, neither an error. So the refusal is
/// on the NAME rather than the call — an `Ident` or a `&` partial in any
/// expression position — and a call is the case where the name is a head.
///
/// Refusing it here makes the three agree by construction, because none of
/// them gets to see the name. Annotations are untouched: a parameter's `:shape`
/// is a pattern, a field's is a type list, and neither is an expression. What
/// stays at run time is the same test on the interpreter's construction path.
fn typeset_at(e: &Expr, annotating: &HashSet<&str>, diags: &mut Vec<Diagnostic>) {
    let named = match e {
        Expr::Ident(name, span, _) | Expr::Partial(name, span) => Some((name.as_str(), span)),
        // A widening names its target after the colon, and a typeset is
        // not something a value can become: `(circle 1):shape` for a
        // typeset holding `circle` refused at RUN time, on both engines,
        // with "this value is not a shape" — a sentence that blames a
        // program whose circle is exactly what the typeset admits.
        Expr::Upcast { ty, span, .. } => Some((ty.as_str(), span)),
        _ => None,
    };
    if let Some((name, span)) = named {
        if annotating.contains(name) {
            diags.push(Diagnostic::new(
                "type",
                format!("`{name}` is a typeset — it only annotates"),
                *span,
            ));
        }
    }
}

/// Whether an operator's arm came in through an import. The import prefixes
/// every name a module declares except an operator's, because `a + b` asks for
/// the bare group, so the slash that marks an imported function is missing
/// here. The type the arm names still carries it: the ownership rule makes
/// every operator arm name a type of its own module, and the import qualified
/// that type. An arm whose named types are all qualified was written by the
/// module that owns them, and building them is that module's own business.
fn imported_arm(decl: &FnDecl) -> bool {
    if !crate::is_operator(&decl.name) {
        return false;
    }
    let (mut local, mut qualified) = (false, false);
    for p in &decl.params {
        if let Pattern::Ctor { ty, .. } | Pattern::Annotated { ty, .. } = p {
            match crate::ast::has_slash(ty) {
                true => qualified = true,
                false => local = true,
            }
        }
    }
    qualified && !local
}

/// type's name was built by its owner. Reading one is free — naming it,
/// annotating with it, destructuring it by name — and so is building your own.
/// What this refuses is reaching across an import to construct.
///
/// A qualified name can never be a local binding, so unlike the arity walk
/// beside it this needs no shadowing set: the slash IS the foreignness.
fn foreign_at(name: &str, span: Span, named: &Named<'_>, diags: &mut Vec<Diagnostic>) {
    // A slash `canonicalize_bare_aliases` wrote may stand for a call: a
    // module can export a function and a type under one name, and the short
    // name then calls the function. Refusing that as a construction rejects a
    // program that compiles. With no function of that name the short name can
    // only build the record, and building one by its short name is refused
    // the same as by its qualified one (ruled 2026-09-30).
    let calls_a_function = named.arities.get(name).is_some()
        && match named.rewritten.get(&(span.line, span.col)) {
            Some(bare) => name.ends_with(bare.as_str()),
            None => false,
        };
    // Before the pass, the short name is the loader's clone. A function of
    // that name, the module's own or an import's twin, is what a call reaches.
    let name = match named.foreign_short.get(name) {
        Some(origin) if named.arities.get(name).is_none() => origin,
        _ => name,
    };
    if !calls_a_function && named.foreign.contains(name) {
        let (owner, base) = crate::ast::split_qual(name).unwrap_or(("", name));
        diags.push(Diagnostic::new(
            "opacity",
            format!(
                "`{name}` is foreign — only `{owner}` builds a `{base}`; \
                 ask it for one through a pub function"
            ),
            span,
        ));
    }
}

/// The distinct arities each name is declared at, as one flat vector and a
/// range apiece. A `Vec` per name held one or two numbers and cost an
/// allocation to say so; two tables in this file were built that way.
struct Arities<'a> {
    ranges: HashMap<&'a str, (u32, u32)>,
    flat: Vec<usize>,
}

impl<'a> Arities<'a> {
    /// Whether this file declares a function of that name.
    fn declares(&self, name: &str) -> bool {
        self.ranges.contains_key(name)
    }

    /// Count the arms per name, turn the counts into starts, then place each
    /// declaration's arity at its name's cursor, skipping one already there.
    /// The counts bound the ranges from above, so a name declared three times
    /// at one arity leaves two cells unread between it and the next name.
    fn of(program: &'a Program) -> Arities<'a> {
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
        let mut flat: Vec<usize> = vec![0; program.fns.len()];
        for decl in &program.fns {
            // One hash of the name, not two: the read and the bump were a
            // `get` and a `get_mut` of the same key, and the slot the first
            // returns is the slot the second wanted.
            let slot = ranges.get_mut(decl.name.as_str()).expect("every name was counted");
            let (start, end) = *slot;
            let arity = decl.params.len();
            if !flat[start as usize..end as usize].contains(&arity) {
                flat[end as usize] = arity;
                slot.1 = end + 1;
            }
        }
        Arities { ranges, flat }
    }

    fn get(&self, name: &str) -> Option<&[usize]> {
        self.ranges.get(name).map(|&(start, end)| &self.flat[start as usize..end as usize])
    }
}

/// What the three share: four tables built once over the whole program, and
/// the record of what the alias pass rewrote.
struct Named<'a> {
    /// How many values each constructor takes: a record's field count, or
    /// one for a subtype, beside the name of the parent it wraps.
    fields: HashMap<&'a str, (usize, Option<&'a str>)>,
    arities: Arities<'a>,
    foreign: HashSet<&'a str>,
    foreign_short: HashMap<&'a str, &'a str>,
    annotating: HashSet<&'a str>,
    rewritten: &'a crate::Rewrites,
}

/// The three name questions at one node. They had a descent of their own until
/// the fold; it was the same descent `per_node_walk` already makes, over the
/// same declarations and the same statements, so this is what is left once the
/// `for_each_child` line goes.
fn named_at<'a>(
    e: &'a Expr,
    named: &Named<'_>,
    own: bool,
    diags: &mut Vec<Diagnostic>,
    shadowable: &mut Vec<(usize, usize, &'a str)>,
) {
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(name, span, _) = &**head {
            let from = diags.len();
            arity_at(name, *span, args.len(), named, diags);
            if diags.len() > from {
                shadowable.push((from, diags.len(), name.as_str()));
            }
            // Each of these two answered nothing on a program with no
            // foreign types or no typesets, and said so by returning before
            // its walk began. The walk is shared now, so the emptiness test
            // moves to the node — a field read, where the lookup it skips is
            // a hash of the name.
            // A short name can be a local binding, which is not the import's
            // record, so a refusal of one is shadowable like an arity's.
            if own && (!named.foreign.is_empty() || !named.foreign_short.is_empty()) {
                let refused = diags.len();
                foreign_at(name, *span, named, diags);
                if diags.len() > refused {
                    shadowable.push((refused, diags.len(), name.as_str()));
                }
            }
        }
    }
    if own && !named.annotating.is_empty() {
        typeset_at(e, &named.annotating, diags);
    }
}

/// Hands each name a pattern binds to `f`, in the order the pattern spells
/// them. This used to answer a `Vec<&str>`, and the `Ctor` arm built one per
/// field and then a second to put the `whole` name in front, so a nested
/// pattern allocated a vector per level for names nobody kept — the shape
/// `expr_children` had before it took a callback.
fn for_each_param_name<'a>(p: &'a Pattern, f: &mut dyn FnMut(&'a str)) {
    match p {
        Pattern::Var(n, _) => f(n.as_str()),
        Pattern::Annotated { name, .. } => f(name.as_str()),
        Pattern::Ctor { fields, whole, .. } => {
            if let Some(named) = whole {
                f(named.0.as_str());
            }
            for field in fields {
                for_each_param_name(field, f);
            }
        }
        Pattern::Keyed { entries, .. } => {
            for e in entries {
                f(e.bind_name.as_str());
            }
        }
        _ => {}
    }
}

/// The first name a pattern binds, which is the one a diagnostic quotes back.
fn first_param_name(p: &Pattern) -> Option<&str> {
    let mut first = None;
    for_each_param_name(p, &mut |n| {
        if first.is_none() {
            first = Some(n);
        }
    });
    first
}

/// Every name this declaration binds anywhere in its body — its parameters,
/// its bindings, and the parameters of every lambda inside it. A binding may
/// not shadow a *function* declaration, so the call half never needed this;
/// a type's name is not covered by that rule, and `grown = (mark -> ...)`
/// beside std/list's two-field `grown` is a legal thing to write.
///
/// The whole body is gathered before the walk rather than as it goes: a name
/// bound after the line that uses it is still a name this code owns, and a
/// check that rejects programs should err towards saying nothing.
fn bound_in_stmt<'a>(stmt: &'a Stmt, out: &mut HashSet<&'a str>) {
    match stmt {
        Stmt::Bind { pattern, expr } => {
            for_each_param_name(pattern, &mut |n| {
                out.insert(n);
            });
            bound_in_expr(expr, out);
        }
        Stmt::Expr(expr) => bound_in_expr(expr, out),
    }
}

fn bound_in_expr<'a>(e: &'a Expr, out: &mut HashSet<&'a str>) {
    if let Expr::Lambda { params, .. } = e {
        out.extend(params.iter().map(|(n, _)| n.as_str()));
    }
    // the statements after a `return .. if` bind names too: until this walk
    // read them, a name bound behind a guard was taken for the declaration
    // sharing its spelling, and `start = text/split ..` read as the effect
    if let Expr::Guard { rest, .. } = e {
        for stmt in rest.iter() {
            bound_in_stmt(stmt, out);
        }
    }
    crate::for_each_child(e, |child| bound_in_expr(child, out));
}

/// A construction's argument count against its type. A record takes one value
/// per field. A subtype takes the one value it wraps: `tall 1 2` for a
/// subtype of a two-field record passed `kanso check` until 2026-09-29, and
/// then the interpreter refused it at run time while the native backend
/// refused the build, each in its own words.
fn construction_arity(
    name: &str,
    fields: usize,
    wraps: Option<&str>,
    argc: usize,
) -> Option<String> {
    match wraps {
        Some(parent) if argc != 1 => Some(format!(
            "`{name}` wraps one `{parent}` value, got {argc} \
             (a subtype is built from the one value it wraps)"
        )),
        None if argc != fields => Some(format!(
            "`{name}` has {fields} field(s), got {argc} \
             (construction is positional, in the order the fields are declared)"
        )),
        _ => None,
    }
}

/// A call's argument count against the arms that could answer it.
fn arity_at(name: &str, span: Span, argc: usize, named: &Named<'_>, diags: &mut Vec<Diagnostic>) {
    // Bare names count too. No binding may shadow a declaration —
    // `adder = ...` beside `fn adder` is a name error — so a bare
    // name matching a declared group is that group, and the per-file
    // pass sees only one file of a module, which leaves every call
    // to a sibling file unchecked.
    let known = named.arities.get(name);
    // A type and a function cannot share a name in one module, so a bare
    // name that is both is a function here and a type another module
    // declared: `fn sorted` beside `import "std/list"` is the function.
    if let (None, Some(&(count, wraps))) = (known, named.fields.get(name)) {
        if let Some(message) = construction_arity(name, count, wraps, argc) {
            diags.push(Diagnostic::new("arity", message, span));
        }
    }
    // A type the program declares is its own name too, and its field count
    // was asked above: a record named `run` is not std/os's builtin.
    if known.is_none() && !named.fields.contains_key(name) {
        if let Some(takes) = builtin_arity(name) {
            if argc != takes {
                diags.push(Diagnostic::new(
                    "arity",
                    format!("`{name}` takes {takes} argument(s), got {argc}"),
                    span,
                ));
            }
        }
    }
    if let Some(known) = known {
        if !known.contains(&argc) && !known.contains(&0) {
            let mut takes: Vec<String> = known.iter().map(|a| a.to_string()).collect();
            takes.sort();
            // kanso#1120: a diagnostic names what the import
            // writes. Where the alias pass qualified a bare call,
            // that is the bare name, not the one it wrote.
            let said = match named.rewritten.get(&(span.line, span.col)) {
                Some(bare) if name.ends_with(bare.as_str()) => bare.as_str(),
                _ => name,
            };
            diags.push(Diagnostic::new(
                "arity",
                format!("no {argc}-argument arm of `{said}` (arms take {})", takes.join(", ")),
                span,
            ));
        }
    }
}

include!("check/literals.rs");

/// Every name a module's own declarations put in scope, borrowed from the
/// program. These used to be owned, and `check_file_shadow` then extended the
/// set with a clone of every extern global on top — a `String` per declared
/// name and per imported name, for a set that is read and dropped.
fn collect_globals<'a>(program: &'a Program, diags: &mut Vec<Diagnostic>) -> HashSet<&'a str> {
    let mut globals: HashSet<&str> = AMBIENT.iter().copied().collect();
    globals.insert("entry");
    globals.insert("err");
    globals.insert("wrap_err");
    for nullary in NULLARY {
        globals.insert(nullary);
    }
    for ty in &program.types {
        if !globals.insert(ty.name.as_str()) {
            diags.push(Diagnostic::new(
                "name",
                format!("the name `{}` is already taken", ty.name),
                ty.span,
            ));
        }
    }
    for decl in &program.fns {
        if AMBIENT.contains(&decl.name.as_str())
            || program.types.iter().any(|t| t.name == decl.name)
        {
            diags.push(Diagnostic::new(
                "name",
                format!("the name `{}` is already taken", decl.name),
                decl.span,
            ));
        }
        globals.insert(decl.name.as_str());
    }
    globals
}

fn check_type_order(program: &Program, diags: &mut Vec<Diagnostic>) {
    if let Some(first_fn_line) =
        program.fns.iter().filter(|f| !f.is_getter()).map(|f| f.span.line).min()
    {
        for ty in program.types.iter().filter(|t| t.span.line > first_fn_line) {
            diags.push(Diagnostic::new(
                "formatting",
                format!(
                    "canonical order places type declarations before functions; move `{}` up",
                    ty.name
                ),
                ty.span,
            ));
        }
    }
    for ty in &program.types {
        for (_, tys, span) in &ty.fields {
            for pair in tys.windows(2) {
                if pair[0] >= pair[1] {
                    diags.push(Diagnostic::new(
                        "formatting",
                        "typeset members appear in alphabetical order, without duplicates"
                            .to_string(),
                        *span,
                    ));
                }
            }
        }
    }
}

/// Declaration order is the author's. What survives here is the ranking rule,
/// which is about which arm dispatch picks rather than about where the arms sit.
fn check_fn_order(program: &Program, diags: &mut Vec<Diagnostic>) {
    check_overload_ranks(program, diags);
}

/// The constants this expression demands *while computing itself*, which is a
/// smaller set than the ones it mentions.
///
/// A constructor stores its arguments; it never looks at them. So a mention
/// inside one is not a demand — the value is put in a field and read later,
/// by which time the constant it names has a value. A call that scrutinizes,
/// an operator, a guard: those force what they are handed, and a mention
/// there is a demand.
///
/// That difference is the whole of the cycle rule. `x = x` asks for its own
/// answer to compute its own answer. `x = { "a":(node x) }` asks for a place
/// to put a name, which is a question the definition can answer.
fn demanded_refs<'a>(
    expr: &'a Expr,
    known: &HashSet<&str>,
    types: TypeNames,
    out: &mut Vec<&'a str>,
) {
    if let Expr::Ident(name, _, _) | Expr::Partial(name, _) = expr {
        if known.contains(name.as_str()) {
            out.push(name);
        }
    }
    // a list or map literal stores every element, so nothing inside is demanded
    if matches!(expr, Expr::List(..) | Expr::MapLit(..)) {
        return;
    }
    if let Expr::App { head, args, .. } = expr {
        if let Expr::Ident(name, _, _) = head.as_ref() {
            if types.contains(name) {
                // a constructor's arguments are stored, so only the head is a
                // demand, and the head is a type rather than a constant
                let _ = args;
                return;
            }
        }
    }
    crate::for_each_child(expr, |child| demanded_refs(child, known, types, out));
}

/// A constant defined in terms of itself has no value to compute: the
/// definition asks for the answer it is meant to produce. Undetected, it
/// recurses until the stack ends, which reports the machine's limit rather
/// than the program's mistake.
/// `any` was the old name for the type that accepts every value except
/// `none`, which is what it never said. An unannotated field or parameter is
/// the unconstrained one.
/// A field's type is what construction sites put in it and uses ask of it,
/// and the compiler works that out for itself — so writing it down again adds
/// a second copy that can disagree with the first. The editor shows the
/// inferred answer on hover, which is where a reader should learn it.
fn check_field_annotations(program: &Program, diags: &mut Vec<Diagnostic>) {
    for ty in &program.types {
        for (field, declared, span) in &ty.fields {
            if declared.is_empty() {
                continue;
            }
            diags.push(Diagnostic::new(
                "type",
                format!(
                    "a record field carries no type — write `{field}` and let the compiler \
                     infer what it holds"
                ),
                *span,
            ));
        }
    }
}

fn check_retired_any(program: &Program, diags: &mut Vec<Diagnostic>) {
    let retired = |ty: &str, span, diags: &mut Vec<Diagnostic>| {
        if ty == "any" {
            diags.push(Diagnostic::new(
                "type",
                "there is no `any` type; leave the annotation off for a field or \
                 parameter that accepts anything"
                    .to_string(),
                span,
            ));
        }
    };
    for ty in &program.types {
        for (_, declared, span) in &ty.fields {
            for name in declared {
                retired(name, *span, diags);
            }
        }
    }
    for decl in &program.fns {
        for param in &decl.params {
            if let Pattern::Annotated { ty, span, .. } = param {
                retired(ty, *span, diags);
            }
        }
    }
}

include!("check/annotations.rs");

/// A record declared in the next file of a module constructs as surely as one
/// declared here. Until 2026-09-29 only this file's types counted, so
/// `ring = node 1 ring` was refused as defined in terms of itself whenever
/// `node` was declared beside it rather than above it.
fn check_constant_cycles(
    program: &Program,
    sibling_types: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    let constants: Vec<&FnDecl> = program.fns.iter().filter(|d| d.params.is_empty()).collect();
    let names: HashSet<&str> = constants.iter().map(|d| d.name.as_str()).collect();
    let own: HashSet<&str> = program.types.iter().map(|t| t.name.as_str()).collect();
    let types = TypeNames { own: &own, siblings: sibling_types };
    let mut refs: HashMap<&str, Vec<&str>> =
        HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
    for decl in &constants {
        let out = refs.entry(decl.name.as_str()).or_default();
        for stmt in &decl.body {
            let expr = match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
            };
            demanded_refs(expr, &names, types, out);
        }
    }
    for decl in &constants {
        let mut seen: HashSet<&str> = HashSet::default();
        let mut stack: Vec<&str> = refs.get(decl.name.as_str()).cloned().unwrap_or_default();
        while let Some(cur) = stack.pop() {
            if cur == decl.name {
                diags.push(
                    Diagnostic::new(
                        "name",
                        format!(
                            "`{}` is defined in terms of itself, so it has no value",
                            decl.name
                        ),
                        decl.span,
                    )
                    .about(&decl.file),
                );
                break;
            }
            if seen.insert(cur) {
                stack.extend(refs.get(cur).into_iter().flatten());
            }
        }
    }
}

fn check_constants(program: &Program, diags: &mut Vec<Diagnostic>) {
    // A run of consecutive declarations sharing a name, walked in place. Each
    // run used to be collected into a `Vec<&FnDecl>` of its own — 579
    // allocation blocks for three facts: whether any arm takes no parameters,
    // how many arms there are, and where the second one is.
    let mut at = 0;
    while at < program.fns.len() {
        let name = &program.fns[at].name;
        let start = at;
        while at < program.fns.len() && program.fns[at].name == *name {
            at += 1;
        }
        // Only the arms the module wrote. A bare overload space (the
        // qualified-name ruling: `dep/~bytes`, holding a clone of the module's
        // own `bytes` beside the twin of its import's) is a union nobody
        // declared, every arm in it is synthetic, and the module's own
        // constant has already stood alone under `dep/bytes`. Until
        // 2026-09-10 the pair passed here by accident, because the clone was
        // appended after every twin and the two were never adjacent; the
        // dispatch-order fix put them side by side and this refused a program
        // that had compiled.
        let mut real = program.fns[start..at].iter().filter(|d| !d.synthetic);
        let first = real.next();
        let second = real.next();
        if let (Some(first), Some(second)) = (first, second) {
            if first.params.is_empty()
                || second.params.is_empty()
                || real.any(|d| d.params.is_empty())
            {
                diags.push(Diagnostic::new(
                    "dispatch",
                    format!("`{name}` is a constant (arity 0); a constant admits no overloads"),
                    second.span,
                ));
            }
        }
    }
}

include!("check/arms.rs");

/// A directory is run through the file `main.kso`, whose statements are
/// compiled in under a name no program can spell. What is missing here is
/// that file — never a declaration the reader was supposed to write called
/// `main`.
fn check_entry(program: &Program, diags: &mut Vec<Diagnostic>) {
    if !program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
        diags.push(Diagnostic::new(
            "name",
            "a directory runs through `main.kso`, and this module has none".to_string(),
            Span::at(1, 1),
        ));
    }
}

fn other_span(pattern: &Pattern) -> Span {
    match pattern {
        Pattern::IntLit(_, s) | Pattern::StrLit(_, s) | Pattern::Nullary(_, s) => *s,
        Pattern::Annotated { span, .. } | Pattern::Keyed { span, .. } => *span,
        Pattern::Var(_, s) => *s,
        Pattern::Wildcard(s) => *s,
        // A constructor pattern has no span of its own, so it answers with
        // its first field that has one. Until 2026-09-29 it answered 0:0,
        // and a refusal pointed at a constructor nested first in another
        // was printed at the top of the file.
        Pattern::Ctor { fields, .. } => {
            fields.iter().map(other_span).find(|s| s.line != 0).unwrap_or(Span::at(0, 0))
        }
    }
}

include!("check/shadow.rs");

/// Where a name that left the library went. The fourteen names that left
/// `std/io` for `std/os` when the library took Go's split, and `text/concat`,
/// which became `list/concat` (#1906). A program written before says
/// `io/read_file` and gets told the name is unknown, which is true and
/// useless: the name is right and the module moved. Answers the empty string
/// for everything else, so the ordinary unknown name reads as it always did.
fn moved_name(name: &str) -> String {
    if name == "text/concat" {
        return " — two lists join with `list/concat`, and two strings by interpolation, `\"{a}{b}\"`".to_string();
    }
    const MOVED: [&str; 14] = [
        "args",
        "env",
        "exists",
        "exit",
        "exit_status",
        "is_dir",
        "kill",
        "list_dir",
        "make_dir",
        "process",
        "read_file",
        "run",
        "start",
        "write_file",
    ];
    match name.strip_prefix("io/") {
        Some(rest) if MOVED.contains(&rest) => {
            format!(" — it moved to `os/{rest}`, and `std/io` keeps the reading and writing")
        }
        _ => String::new(),
    }
}

/// A non-final statement that is a call answering neither an effect nor a
/// failure computes something nobody reads.
///
/// The parser catches the same thing for a literal, where shape alone decides.
/// A call needs the fixpoint: `print x` and `double 21` are the same shape, and
/// only inference separates them. Left to the runtime it dies inside the
/// group's join naming `&`, an operator the author never wrote.
fn check_discarded_value(
    program: &Program,
    returns: &HashMap<(&str, usize), crate::infer::Set>,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::infer::{DESC, ERR};

    // A construction answers its own type and nothing else, so it is refused
    // wherever a call that can only answer a value is. A typeset is a
    // vocabulary rather than a constructor, and takes no arguments.
    let ctors: HashSet<(&str, usize)> = program
        .types
        .iter()
        .filter(|t| t.members.is_empty())
        .map(|t| match t.parent {
            Some(_) => (t.name.as_str(), 1),
            None => (t.name.as_str(), t.fields.len()),
        })
        .collect();

    let refused = |e: &Expr| -> bool {
        let Expr::App { head, args, piped: false, .. } = e else { return false };
        let Expr::Ident(name, _, _) = head.as_ref() else { return false };
        // An err propagates through the group, so a line that only fails is
        // still doing something — a call is refused when it can be neither.
        match returns.get(&(name.as_str(), args.len())) {
            Some(set) => set & (DESC | ERR) == 0,
            None => ctors.contains(&(name.as_str(), args.len())),
        }
    };

    // One vector for every statement: a fresh one per statement was an
    // allocation for each of lib/json's 718 expression lines.
    let mut leaves = Vec::new();
    for decl in &program.fns {
        if decl.synthetic {
            continue;
        }
        let Some(end) = decl.body.len().checked_sub(1) else { continue };
        for (at, stmt) in decl.body.iter().enumerate() {
            let Stmt::Expr(expr) = stmt else { continue };
            // Adjacent lines are one joined expression by now, so the members
            // of a group are leaves of its join spine rather than statements.
            leaves.clear();
            flatten_join(expr, &mut leaves);
            // The body's result is the last leaf of the last statement; every
            // other leaf is a line whose value nothing reads.
            let keep = match at == end {
                true => leaves.len().saturating_sub(1),
                false => leaves.len(),
            };
            for leaf in &leaves[..keep] {
                if refused(leaf) {
                    diags.push(
                        Diagnostic::new(
                            "unused",
                            "this value is never used: a non-final line binds a name, or \
                         is an effect joining the group"
                                .to_string(),
                            leaf.span(),
                        )
                        .about(&decl.file),
                    );
                }
            }
        }
    }
}

fn flatten_join<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
    match e {
        Expr::Join { lhs, rhs, .. } => {
            flatten_join(lhs, out);
            flatten_join(rhs, out);
        }
        _ => out.push(e),
    }
}

/// A failure the compiler can work out by reading is not a runtime failure.
///
/// Only what is written in the source: a divisor that IS the literal zero, a
/// conversion whose argument IS a literal that does not parse. A divisor that
/// arrives as a parameter cannot be decided here and is left alone — refusing
/// what it cannot read would be worse than the bug.
///
/// The descent is `per_node_walk`'s, which carries a flag saying whether the
/// node is reached unguarded; see the rule there for why a guarded branch is
/// not asked.
fn decidable_failure_at(e: &Expr, diags: &mut Vec<Diagnostic>) {
    if let Expr::BinOp { op, rhs, span, .. } = e {
        if matches!(*op, "/" | "%") && matches!(rhs.as_ref(), Expr::Int(n, _) if n.is_zero()) {
            let named = match *op {
                "/" => "division by zero",
                _ => "modulo by zero",
            };
            diags.push(Diagnostic::new("value", named.to_string(), *span));
        }
    }
    if let Expr::App { head, args, .. } = e {
        if let Expr::Ident(name, _, _) = head.as_ref() {
            unparseable_conversion(name, args, diags);
        }
    }
}

/// `to_int!` and `to_float!` given a literal that will not parse. The plain
/// forms answer none for it, an outcome rather than a failure (#2087).
fn unparseable_conversion(name: &str, args: &[Expr], diags: &mut Vec<Diagnostic>) {
    let bare = crate::ast::split_qual(name).map(|(_, s)| s).unwrap_or(name);
    let Some(Expr::Str(parts, span)) = args.first() else { return };
    let [TemplatePart::Lit(text)] = parts.as_slice() else { return };
    let refuses = match bare {
        // An int has no ceiling, so a literal past a word still parses.
        "to_int!" => text.contains('_') || text.trim().parse::<num_bigint::BigInt>().is_err(),
        "to_float!" => text.trim().parse::<f64>().is_err(),
        _ => return,
    };
    if refuses {
        diags.push(Diagnostic::new(
            "value",
            format!(
                "{text:?} is not {}",
                match bare {
                    "to_int!" => "an integer",
                    _ => "a number",
                }
            ),
            *span,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{builtin_arity, BUILTINS, BUILTIN_ARITY};

    /// `builtin_arity` binary-searches the table, so an entry out of order is
    /// a builtin the front end stops knowing the count of — no error, no
    /// diagnostic, just a check that quietly stops covering one name.
    #[test]
    fn a_builtin_table_a_binary_search_can_read() {
        let mut sorted: Vec<&str> = BUILTIN_ARITY.iter().map(|(b, _)| *b).collect();
        let written = sorted.clone();
        sorted.sort_unstable();
        assert_eq!(written, sorted, "BUILTIN_ARITY is out of alphabetical order");
        for (name, takes) in BUILTIN_ARITY {
            assert_eq!(builtin_arity(name), Some(takes), "the search missed `{name}`");
            assert_eq!(
                builtin_arity(&format!("builtin_{name}")),
                Some(takes),
                "the search missed `builtin_{name}`"
            );
        }
    }

    /// A name a program can write bare and no count to check it against. `if`
    /// is the one deliberate absence: its count is checked where its branches
    /// are, because the guard form spells the word with a different shape.
    #[test]
    fn every_bare_builtin_has_a_count() {
        let missing: Vec<&str> =
            BUILTINS.iter().copied().filter(|n| *n != "if" && builtin_arity(n).is_none()).collect();
        assert!(missing.is_empty(), "no count for {missing:?}");
    }
}
