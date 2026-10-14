use crate::ast::*;
use crate::diag::Span;
use crate::hash::Map as HashMap;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, FLOAT, INT, LIST, MAP, NONE, REC, STR, TOP};
use crate::name::Name;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;

/// What an `if`'s condition left behind: the blocks that branched straight to
/// the merge because the condition was a failure, each with the value it
/// carried. A comparison on two ints leaves none, which is the point.
struct Cond {
    failed: Vec<(String, String)>,
}
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
/// A succeeded effect's yield; the runtime's newest tag, after K_SUB.
const K_DONE: i64 = 16;
/// An int past a machine word, in the arena. Ruled 2026-10-07.
const K_BIG: i64 = 17;
const K_ERR: i64 = 5;
const K_DESC: i64 = 8;

/// What an arm's discriminating pattern tests, when a switch on the value's tag
/// can express it. Records are `Rec` rather than a tag because they all carry
/// tag 7 and are told apart by the id and field count inside.
enum ArmCase {
    Tags(Vec<i64>),
    Rec(i64, usize),
}

/// Whether this build wants the allocation counters. BOTH halves of a binary
/// ask it -- the emitter for the eight inlined gates, and the runtime object
/// for its own twenty-seven -- because a counting runtime linked against
/// gate-free IR, or the reverse, is a binary whose counters are half there.
///
/// Either flag says yes. `--counters` is the explicit ask; a build running
/// under KANSO_COUNTERS is a process that is itself counting, and a binary it
/// produces is going to be counted too. Without the second, tests/golden.rs's
/// .mem vein -- which sets KANSO_COUNTERS around a build it drives through the
/// library and has no way to pass a flag -- silently got a gate-free binary,
/// and every allocation counter in the corpus moved at once.
///
/// Read once and kept. The emitter, the runtime object's key and the program
/// binary's key all ask, and each read walks the environment; the one place
/// that sets the flag, `--counters`, does it while parsing the arguments,
/// before anything asks.
pub fn counters_wanted() -> bool {
    static WANTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *WANTED.get_or_init(|| {
        std::env::var_os("KANSO_COUNTERS_BUILD").is_some()
            || std::env::var_os("KANSO_COUNTERS").is_some()
    })
}

/// How many `k_stats_on` gates DECLARES carries. Pinned so that adding one
/// without teaching `index_declares` about it fails the build.
pub const STATS_GATE_SITES: usize = 10;

include!("codegen/declares.rs");

/// Whether a program of the user's calls `list/tie` or `list/tie!`. Their
/// makers store a reference still being made in a constructor's field, which
/// is the same deferral a knotted constant needs, so a constructor in such a
/// program keeps a blackhole rather than demanding it. Asked of the user's
/// declarations only: std/list's `tie!` calls `tie`, and every program that
/// imports the module would otherwise pay for a feature it never calls. The
/// module is known by its names rather than its file, because a module
/// loaded from the checkout is stamped `lib/` rather than `std/`.
fn ties(program: &Program) -> bool {
    fn mentions(e: &Expr) -> bool {
        match e {
            Expr::Ident(n, _, _) | Expr::Partial(n, _) => n == "list/tie" || n == "list/tie!",
            _ => crate::any_child(e, mentions),
        }
    }
    let stmt = |st: &Stmt| match st {
        Stmt::Bind { expr, .. } | Stmt::Expr(expr) => mentions(expr),
    };
    program.fns.iter().filter(|d| !d.name.starts_with("list/")).any(|d| d.body.iter().any(stmt))
}

pub(crate) const BUILTIN_CALLS: [&str; 62] = [
    "effect",
    "built_text",
    "net_port",
    "start",
    "kill",
    "at",
    "is_desc",
    "append",
    "find2",
    "find2_below",
    "number_span",
    "bytes",
    "to_bytes",
    "bind",
    "rescue",
    "annotate",
    "read_file",
    "read_bytes",
    "write",
    "write_err",
    "env",
    "exists",
    "is_dir",
    "list_dir",
    "make_dir",
    "write_file",
    "run",
    "listen",
    "accept",
    "net_read",
    "net_write",
    "net_close",
    "concat",
    "utf8",
    "char_code",
    "chars",
    "split",
    "entries",
    "filter",
    "from_code",
    "join",
    "keys",
    "length",
    "map",
    "push",
    "put",
    "slice",
    "sort",
    "render_value",
    "sqrt",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "bit_shl",
    "bit_shr",
    "round",
    "sum",
    "to_float",
    "to_int",
    "values",
    "tie",
];

/// The bit builtins that have an inline twin. Each is one machine op on two
/// ints, and a call to reach it; the twin does the int case where the tags
/// allow and leaves every other shape to the C entry that owns the message.
pub(crate) const BIT_TWINS: [&str; 6] =
    ["bit_and", "bit_or", "bit_xor", "bit_not", "bit_shl", "bit_shr"];

/// The twin's name for one of them. Separate from the table so a name added
/// to one without the other does not compile.
fn bit_twin(name: &str) -> &'static str {
    match name {
        "bit_and" => "bit_and_fast",
        "bit_or" => "bit_or_fast",
        "bit_xor" => "bit_xor_fast",
        "bit_not" => "bit_not_fast",
        "bit_shl" => "bit_shl_fast",
        "bit_shr" => "bit_shr_fast",
        other => unreachable!("bit_twin asked for `{other}`, which BIT_TWINS does not hold"),
    }
}

/// The map a call `keys m` or `values m` (named by `which`) reads, when its
/// argument is a plain name.
fn column_of<'e>(e: &'e Expr, which: &str) -> Option<&'e str> {
    let Expr::App { head, args, piped: false, .. } = e else { return None };
    match (&**head, args.as_slice()) {
        (Expr::Ident(h, _, _), [Expr::Ident(m, _, _)]) if h.as_str() == which => Some(m.as_str()),
        _ => None,
    }
}

/// The count a builtin the native backend emits a direct call for takes.
/// Membership is this file's business — which builtins get a C entry rather
/// than an inline expansion — and the count is `check`'s, so this asks each
/// question where it is answered instead of keeping a second copy of the
/// counts here. That second copy is what let the backend refuse a call the
/// front door had waved through.
fn arity_of_emitted(name: &str) -> Option<usize> {
    match BUILTIN_CALLS.contains(&name) {
        true => crate::check::builtin_arity(name),
        false => None,
    }
}

/// Groups that are pure builtin forwarders: one arm, plain-var params,
/// body exactly `builtin_X p1 p2 ...` in order. Call sites bypass the
/// dispatch hop and reach the builtin (and its inline twins) directly.
fn forwarder_map(program: &Program) -> Forwarders {
    let mut counts: HashMap<(&str, usize), usize> = HashMap::default();
    for d in &program.fns {
        *counts.entry((d.name.as_str(), d.params.len())).or_default() += 1;
    }
    let mut out = Forwarders::default();
    for d in &program.fns {
        if counts[&(d.name.as_str(), d.params.len())] != 1 || d.body.len() != 1 {
            continue;
        }
        let params: Vec<&str> = d
            .params
            .iter()
            .filter_map(|p| match p {
                Pattern::Var(n, _) => Some(n.as_str()),
                _ => None,
            })
            .collect();
        if params.len() != d.params.len() {
            continue;
        }
        let Stmt::Expr(Expr::App { head, args, piped: false, .. }) = &d.body[0] else {
            continue;
        };
        let Expr::Ident(callee, _, _) = head.as_ref() else { continue };
        let Some(target) = callee.strip_prefix("builtin_") else { continue };
        let all_forwarded = args.len() == params.len()
            && args.iter().zip(&params).all(|(a, p)| matches!(a, Expr::Ident(n, _, _) if n == p));
        if all_forwarded {
            out.0.entry(d.name.clone()).or_default().push((d.params.len(), target.to_string()));
        }
    }
    out
}

/// The forwarder groups, by name: each arity with the builtin it forwards to.
/// Keyed by name alone so a question borrows the name it is asked with; the
/// emitter asks about every call it writes.
#[derive(Default)]
struct Forwarders(HashMap<String, Vec<(usize, String)>>);

impl Forwarders {
    fn get(&self, name: &str, arity: usize) -> Option<&str> {
        let arities = self.0.get(name)?;
        arities.iter().find(|(a, _)| *a == arity).map(|(_, target)| target.as_str())
    }

    fn contains(&self, name: &str, arity: usize) -> bool {
        self.get(name, arity).is_some()
    }
}

/// A name the emitter numbers, `%t7` or `L7`, held in place rather than in a
/// `String`: the emitter names a local for nearly every instruction it writes
/// and a label for every branch.
#[derive(Clone, Copy)]
struct Numbered {
    buf: [u8; 24],
    len: u8,
}

impl Numbered {
    fn new(prefix: &[u8], n: usize) -> Numbered {
        let mut buf = [0u8; 24];
        buf[..prefix.len()].copy_from_slice(prefix);
        let mut digits = [0u8; 20];
        let mut d = 0;
        let mut n = n;
        loop {
            digits[d] = b'0' + (n % 10) as u8;
            d += 1;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        for k in 0..d {
            buf[prefix.len() + k] = digits[d - 1 - k];
        }
        Numbered { buf, len: (prefix.len() + d) as u8 }
    }
}

impl std::ops::Deref for Numbered {
    type Target = str;
    fn deref(&self) -> &str {
        // a prefix and ASCII digits, nothing else
        std::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or_default()
    }
}

impl std::fmt::Display for Numbered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

/// The byte discriminators `crate::dispatch::byte_dispatched` found, keyed by
/// name alone for the same reason as `Forwarders`: the emitter asks about
/// every argument of every call it writes.
#[derive(Default)]
struct ByteDiscs(HashMap<String, Vec<(usize, usize)>>);

impl ByteDiscs {
    fn from(found: crate::hash::Set<(String, usize, usize)>) -> Self {
        let mut out = ByteDiscs::default();
        for (name, arity, param) in found {
            out.0.entry(name).or_default().push((arity, param));
        }
        out
    }

    fn contains(&self, name: &str, arity: usize, param: usize) -> bool {
        self.0.get(name).is_some_and(|found| found.contains(&(arity, param)))
    }
}

/// The constants a constant can reach from its own body, following mentions
/// through other constants. A name that reaches itself must be frozen: an
/// unfrozen mention re-enters the builder, and the recursion has no floor.
/// One name reaching itself and two names reaching each other are the same
/// shape, so the question is cycle membership.
pub(crate) fn knotted_constants(program: &Program) -> crate::hash::Set<String> {
    fn names(expr: &Expr, out: &mut Vec<String>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            out.push(n.to_string());
        }
        crate::for_each_child(expr, |child| names(child, out));
    }
    let mut mentions: HashMap<&str, Vec<String>> = HashMap::default();
    for decl in program.fns.iter().filter(|d| d.params.is_empty()) {
        let found = mentions.entry(decl.name.as_str()).or_default();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => names(expr, found),
            }
        }
    }
    mentions
        .keys()
        .copied()
        .filter(|start| {
            let mut seen: crate::hash::Set<&str> = crate::hash::Set::default();
            let mut queue: Vec<&str> = vec![start];
            while let Some(here) = queue.pop() {
                for next in mentions.get(here).into_iter().flatten() {
                    if next == start {
                        return true;
                    }
                    if let Some((key, _)) = mentions.get_key_value(next.as_str()) {
                        if seen.insert(key) {
                            queue.push(key);
                        }
                    }
                }
            }
            false
        })
        .map(str::to_string)
        .collect()
}

/// Whether the host's clang can take `preserve_none` on a closure body.
///
/// The emitter cannot ask -- it never runs clang -- so the answer arrives
/// from whoever does. It is an explicit choice rather than a probe result on
/// purpose: `tests/compile_cost.rs` and `tests/perf_ratchet.rs` pin
/// compile-side goldens, and a golden that moved with the installed clang
/// would go red on a developer's machine for something the developer did not
/// do. Those callers pass `Absent` and their veins stay a property of the
/// compiler. Only the two build paths in main.rs ask the probe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClosureConvention {
    /// `preserve_nonecc` on closure bodies and the arms that call them.
    PreserveNone,
    /// The C convention, which every clang can take.
    Absent,
}

impl ClosureConvention {
    /// The keyword to write before a closure define or call, with its
    /// trailing space, or nothing.
    fn keyword(self) -> &'static str {
        match self {
            ClosureConvention::PreserveNone => "preserve_nonecc ",
            ClosureConvention::Absent => "",
        }
    }
}

pub fn emit_ir(program: &Program, convention: ClosureConvention) -> Result<String, String> {
    emit_ir_for(Given::Lent(program), convention, true)
}

/// `emit_ir` for a caller done with the program, which the emitter then
/// changes in place rather than copying.
pub fn emit_ir_owned(program: Program, convention: ClosureConvention) -> Result<String, String> {
    emit_ir_for(Given::Owned(program), convention, true)
}

/// The module a dev build compiles: the same as `emit_ir`'s, with the
/// runtime's helpers left to be called rather than marked `alwaysinline`.
///
/// A dev build compiles at `-O0`, where the always-inliner still copies every
/// helper into every call site and the instruction selector then walks each
/// copy. The codegen corpus's module carries thirty-six such definitions, and
/// without the attribute `clang -cc1` read 316,180,072 instructions against
/// 359,109,516, -11.95%. What a dev binary gives up is the inlining itself,
/// which the dev tier does not promise: it is the tier that compiles fast.
/// No helper needs to be inlined to be correct. None allocates on the stack,
/// reads a frame or return address, or makes a `musttail` call, and the
/// program's calls into them are all plain calls.
pub fn emit_ir_dev(program: &Program, convention: ClosureConvention) -> Result<String, String> {
    emit_ir_for(Given::Lent(program), convention, false)
}

/// `emit_ir_dev` for a caller done with the program. See `emit_ir_owned`.
pub fn emit_ir_dev_owned(
    program: Program,
    convention: ClosureConvention,
) -> Result<String, String> {
    emit_ir_for(Given::Owned(program), convention, false)
}

/// `emit_ir_dev_owned` for the playground tab. It emits only the groups the
/// entry can reach, as every build does; the prune in `Written::finish` drops
/// the rest from the module either way. On `bench/interp_corpus` that was 132
/// definitions and 127,415 bytes of text, a third of what the tab wrote, held
/// beside the program at the most its compile ever holds.
pub fn emit_ir_tab_owned(
    program: Program,
    convention: ClosureConvention,
) -> Result<String, String> {
    emit_written(Given::Owned(program), convention, false).map(Written::finish)
}

/// A group's name and arity, and a position in it.
type UnreadPositions = Vec<(String, usize, usize)>;

/// Arms no value in the program can reach, dropped before anything is emitted.
///
/// An arm whose parameter pattern names a record type matches only a value of
/// that type, and the only way a value of a declared type comes to exist is an
/// expression that names the type: a construction, a partial of it, a bare
/// constructor handed on as a function, or an upcast to it. So a type that no
/// expression anywhere in the program names never has a value, and neither does
/// a subtype of it that is never built. `entries` is the one builder outside
/// the program's text, and it makes `entry`, id 0. An arm matching an unbuilt
/// type, at any depth of its pattern, can never be chosen, and what only it
/// calls is dead with it -- which the prune after emission then removes.
///
/// std/list's `next` dispatches over every lazy adapter the library declares,
/// so a program that maps once emitted every adapter's arm and everything each
/// arm calls: 1,560 of the codegen corpus's 4,628 lines of IR were adapters it
/// never builds and what they call. A group keeps all its arms if
/// every one of them would go, so a call that reaches it still fails the way
/// it did. The interpreter is untouched and the differential corpus is the
/// check that the two still agree.
fn without_unbuilt_arms(program: &Program) -> Option<(Vec<bool>, UnreadPositions)> {
    use crate::ast::{Expr, Pattern, Stmt, TemplatePart};
    let mut ids: HashMap<&str, i64> = HashMap::default();
    ids.insert("entry", 0);
    for (i, ty) in program.types.iter().enumerate() {
        ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    for ty in &program.types {
        if let Some(id) = ty.origin.as_deref().and_then(|o| ids.get(o).copied()) {
            ids.insert(ty.name.as_str(), id);
        }
    }
    let mut built: crate::hash::Set<i64> = crate::hash::Set::default();
    built.insert(0);
    fn names_in_stmt<'a>(s: &'a Stmt, out: &mut Vec<&'a str>) {
        match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => names_in(expr, out),
        }
    }
    fn names_in<'a>(e: &'a Expr, out: &mut Vec<&'a str>) {
        match e {
            Expr::Int(..) | Expr::Float(..) => {}
            Expr::Ident(n, _, _) | Expr::Partial(n, _) => out.push(n.as_str()),
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    names_in(k, out);
                    names_in(v, out);
                }
            }
            Expr::Str(parts, _) => {
                for p in parts {
                    if let TemplatePart::Interp(inner) = p {
                        names_in(inner, out);
                    }
                }
            }
            Expr::List(items, _) => items.iter().for_each(|x| names_in(x, out)),
            Expr::App { head, args, .. } => {
                names_in(head, out);
                args.iter().for_each(|x| names_in(x, out));
            }
            Expr::Field { base, .. } => names_in(base, out),
            Expr::Index { base, index, .. } => {
                names_in(base, out);
                names_in(index, out);
            }
            Expr::BinOp { lhs: a, rhs: b, .. } | Expr::Join { lhs: a, rhs: b, .. } => {
                names_in(a, out);
                names_in(b, out);
            }
            Expr::Lambda { body, .. } => names_in(body, out),
            Expr::Block(stmts, _) => stmts.iter().for_each(|s| names_in_stmt(s, out)),
            Expr::Upcast { expr, ty, .. } => {
                names_in(expr, out);
                out.push(ty.as_str());
            }
            Expr::Guard { cond, early, rest, .. } => {
                names_in(cond, out);
                names_in(early, out);
                rest.iter().for_each(|s| names_in_stmt(s, out));
            }
        }
    }
    fn unbuilt(p: &Pattern, ids: &HashMap<&str, i64>, built: &crate::hash::Set<i64>) -> bool {
        match p {
            Pattern::Ctor { ty, fields, .. } => {
                ids.get(ty.as_str()).is_some_and(|id| !built.contains(id))
                    || fields.iter().any(|f| unbuilt(f, ids, built))
            }
            _ => false,
        }
    }
    // Only a body that can run builds anything. std/list declares a builder
    // for every adapter, and counting names over every declaration found
    // every adapter built, whether or not the program ever called the
    // builder. So reachability and construction are one fixpoint: a group is
    // reached when a live arm names it, an arm is live when its group is
    // reached and no type in its pattern is unbuilt, and a live arm's body
    // builds what it names. The roots are the entry, every constant, and the
    // groups the emitter calls without the source naming them -- the
    // renderer and the user operators.
    let root = |d: &crate::ast::FnDecl| {
        d.name == crate::ast::ENTRY
            || d.params.is_empty()
            || d.name == RENDER_GROUP
            || !d.name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '/' | '~'))
    };
    if !program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
        return None;
    }
    let mut reached: crate::hash::Set<&str> = crate::hash::Set::default();
    for d in &program.fns {
        if root(d) {
            reached.insert(d.name.as_str());
        }
    }
    let mut read = vec![false; program.fns.len()];
    loop {
        let mut moved = false;
        for (at, d) in program.fns.iter().enumerate() {
            if read[at] || !reached.contains(d.name.as_str()) {
                continue;
            }
            if d.params.iter().any(|p| unbuilt(p, &ids, &built)) {
                continue;
            }
            read[at] = true;
            moved = true;
            let mut named: Vec<&str> = Vec::new();
            d.body.iter().for_each(|s| names_in_stmt(s, &mut named));
            for name in named {
                if let Some(id) = ids.get(name) {
                    built.insert(*id);
                }
                reached.insert(name);
            }
            // A subtype's value matches its parent's patterns, so building one
            // builds every ancestor it flows as.
            loop {
                let before = built.len();
                for ty in &program.types {
                    let (Some(child), Some(parent)) =
                        (ids.get(ty.name.as_str()), ty.parent.as_deref())
                    else {
                        continue;
                    };
                    if built.contains(child) {
                        if let Some(p) = ids.get(parent) {
                            built.insert(*p);
                        }
                    }
                }
                if built.len() == before {
                    break;
                }
            }
        }
        if !moved {
            break;
        }
    }
    let dead: Vec<bool> =
        program.fns.iter().map(|d| d.params.iter().any(|p| unbuilt(p, &ids, &built))).collect();
    if !dead.iter().any(|d| *d) {
        return None;
    }
    // A group every one of whose arms would go keeps them all. Beside each
    // group's count of live arms sits the positions those arms read, so the
    // positions only a dropped arm read fall out of the same walk.
    let reads = |d: &crate::ast::FnDecl| {
        d.params.iter().enumerate().fold(0u64, |m, (i, p)| {
            if i < 64 && !matches!(p, Pattern::Var(..) | Pattern::Wildcard(_)) {
                m | 1 << i
            } else {
                m
            }
        })
    };
    let mut live_arms: HashMap<(&str, usize), (usize, u64)> = HashMap::default();
    for (d, gone) in program.fns.iter().zip(&dead) {
        let n = live_arms.entry((d.name.as_str(), d.params.len())).or_insert((0, 0));
        if !gone {
            n.0 += 1;
            n.1 |= reads(d);
        }
    }
    let mut unread: UnreadPositions = Vec::new();
    for (d, gone) in program.fns.iter().zip(&dead) {
        let (live, kept) = live_arms[&(d.name.as_str(), d.params.len())];
        if !gone || live == 0 {
            continue;
        }
        let lost = reads(d) & !kept;
        for i in (0..d.params.len().min(64)).filter(|i| lost & 1 << i != 0) {
            unread.push((d.name.clone(), d.params.len(), i));
        }
    }
    unread.sort();
    unread.dedup();
    let keep = program
        .fns
        .iter()
        .zip(&dead)
        .map(|(d, gone)| !gone || live_arms[&(d.name.as_str(), d.params.len())].0 == 0)
        .collect();
    Some((keep, unread))
}

/// A program the emitter was handed: lent, when the caller goes on to use it,
/// or given, when it does not.
enum Given<'a> {
    Lent(&'a Program),
    Owned(Program),
}

impl std::ops::Deref for Given<'_> {
    type Target = Program;
    fn deref(&self) -> &Program {
        match self {
            Given::Lent(p) => p,
            Given::Owned(p) => p,
        }
    }
}

/// The program `without_unbuilt_arms` describes. One the caller owns loses its
/// dropped arms where it stands; one it lends is copied, arm by kept arm, since
/// the caller goes on holding the original.
fn kept_arms(program: Given<'_>, keep: &[bool]) -> Program {
    match program {
        Given::Owned(mut owned) => {
            let mut at = keep.iter();
            owned.fns.retain(|_| *at.next().unwrap_or(&true));
            owned
        }
        Given::Lent(lent) => Program {
            fns: lent.fns.iter().zip(keep).filter(|(_, k)| **k).map(|(d, _)| d.clone()).collect(),
            types: lent.types.clone(),
            imports: lent.imports.clone(),
            reexports: lent.reexports.clone(),
            root: lent.root.clone(),
        },
    }
}

/// The work both tiers share, and the frame `emit_instructions` anchors on.
/// Kept out of line so the anchor exists whichever entry point reached it.
/// The attribute sat on `without_unbuilt_arms` below for a while, with a doc
/// comment between them, and this frame stayed out of line only because it
/// was large; once it was two calls, the profile lost it.
#[inline(never)]
fn emit_ir_for(
    program: Given<'_>,
    convention: ClosureConvention,
    inline_helpers: bool,
) -> Result<String, String> {
    emit_written(program, convention, inline_helpers).map(Written::finish)
}

fn emit_written(
    program: Given<'_>,
    convention: ClosureConvention,
    inline_helpers: bool,
) -> Result<Written, String> {
    // The prune also answers the positions a group read as written and no
    // longer reads. A thunk handed to such a position is forced by
    // the interpreter, whose dispatcher still holds the dropped arm and has to
    // look at the value to rule it out; an err the thunk turns into then answers
    // at that group, with its name in the trace. `list/find` loses its arm for a
    // sorted list in a program that sorts nothing, and a lazy list handed to it
    // went on to fail in `list/next`, two frames later and under other names.
    // Forcing where the program as written would have looked keeps the trace
    // the interpreter's.
    let program = match crate::ahead::lifts(&program) {
        true => {
            let mut owned = match program {
                Given::Owned(p) => p,
                Given::Lent(p) => Program {
                    fns: p.fns.clone(),
                    types: p.types.clone(),
                    imports: p.imports.clone(),
                    reexports: p.reexports.clone(),
                    root: p.root.clone(),
                },
            };
            crate::ahead::lift(&mut owned);
            Given::Owned(owned)
        }
        false => program,
    };
    let (program, unread_positions) = match without_unbuilt_arms(&program) {
        Some((keep, unread)) => (Given::Owned(kept_arms(program, &keep)), unread),
        None => (program, Vec::new()),
    };
    let mut program = program;
    // What the entry cannot reach leaves an owned program before any analysis
    // reads it: inference, linearity, the beat loops and the rest each walked
    // every std function the program imported, and the emitter wrote them all
    // for the prune to drop. The tab did this first; native builds joined on
    // 2026-10-09, and on runbench the emit fell from 23,409,658 instructions
    // to 7,038,906. A smaller program can let a record go back in two words
    // where the whole one kept it boxed, which is why `escape` asks every arm
    // of a group what it answers.
    let mut left_out: Vec<(usize, FnDecl)> = Vec::new();
    if let Given::Owned(owned) = &mut program {
        let reached = reached_from_entry(owned);
        if let Some(reached) = reached {
            let reached: crate::hash::Set<String> = reached.iter().map(|n| n.to_string()).collect();
            for (at, decl) in std::mem::take(&mut owned.fns).into_iter().enumerate() {
                match reached.contains(decl.name.as_str()) {
                    true => owned.fns.push(decl),
                    false => left_out.push((at, decl)),
                }
            }
        }
    }
    let names: crate::hash::Set<String> = left_out.iter().map(|(_, d)| d.name.clone()).collect();
    let first = emit_with(&program, convention, inline_helpers, &unread_positions, names);
    // The walk reads names the bodies mention, and the emitter can reach a
    // group no body names, by calling it or by asking whether it exists. Either
    // one, for a group that was left out, gives the program back what it lost,
    // in its place, and emits it again, so a miss costs time and never changes
    // the module. So does a refusal: a call to a group that is not there is one
    // the emitter declines to lower, and the program as written may not be.
    let missed = match &first {
        Ok(written) => written.asked_left_out || written.names_any(left_out.iter().map(|(_, d)| d)),
        Err(_) => true,
    };
    if left_out.is_empty() || !missed {
        return first;
    }
    if let Given::Owned(owned) = &mut program {
        let total = owned.fns.len() + left_out.len();
        let mut kept = std::mem::take(&mut owned.fns).into_iter();
        let mut back = left_out.into_iter().peekable();
        for at in 0..total {
            match back.next_if(|(was, _)| *was == at) {
                Some((_, decl)) => owned.fns.push(decl),
                None => owned.fns.extend(kept.next()),
            }
        }
    }
    emit_with(&program, convention, inline_helpers, &unread_positions, Default::default())
}

/// The names the entry reaches, by what each body mentions, or `None` for a
/// program with no entry. A mention counts as a call at every arity.
fn reached_from_entry(program: &Program) -> Option<crate::hash::Set<&str>> {
    let (names, adj) = call_graph(program);
    let start = names.iter().position(|n| *n == crate::ast::ENTRY)?;
    let mut seen = vec![false; names.len()];
    let mut work = vec![start];
    // An interpolated value dispatches to the render group by name, and an
    // operator reaches its arms through an expression no walk of mentions
    // reads as one, so every operator group is kept.
    work.extend(names.iter().position(|n| *n == RENDER_GROUP));
    // A value-wrapped `print` renders through `render/to_string`, and whether
    // fused map columns may stand in for `keys` and `values` turns on whether
    // the program declares its own. Each is a question by name a body need not
    // mention, and a miss would only cost a second emit.
    work.extend(
        names
            .iter()
            .enumerate()
            .filter(|(_, n)| matches!(**n, "render/to_string" | "keys" | "values"))
            .map(|(at, _)| at),
    );
    work.extend(
        names
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.bytes().any(|b| b.is_ascii_alphanumeric()))
            .map(|(at, _)| at),
    );
    while let Some(at) = work.pop() {
        if !seen[at] {
            seen[at] = true;
            work.extend(adj[at].iter().copied().filter(|&next| !seen[next]));
        }
    }
    Some(names.iter().zip(seen).filter(|(_, s)| *s).map(|(n, _)| *n).collect())
}

fn emit_with(
    program: &Program,
    convention: ClosureConvention,
    inline_helpers: bool,
    unread_positions: &UnreadPositions,
    left_out: crate::hash::Set<String>,
) -> Result<Written, String> {
    let knotted = knotted_constants(program);
    let inference = infer::infer(program);
    let mut type_ids = HashMap::default();
    type_ids.insert("entry", 0i64);
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    // an enrollment clone is an alias: it constructs and matches as its
    // origin, one identity per type no matter the spelling
    let clone_ids: Vec<(&str, i64)> = program
        .types
        .iter()
        .filter_map(|t| {
            t.origin.as_deref().and_then(|o| type_ids.get(o).map(|id| (t.name.as_str(), *id)))
        })
        .collect();
    for (name, id) in clone_ids {
        type_ids.insert(name, id);
    }
    let mut escape = crate::escape::analyze(program, &inference);
    // The by-value `%parsed` is two i64s, so it only fits a record shaped like
    // the scanner's `_parsed`: exactly two fields, a small int position packed
    // into the tag word and a non-failure value in the payload word. Any other
    // register-returnable record keeps the heap representation. A position
    // that may be a bignum still packs when it is a word, and spills when it
    // is not, as a word past 56 bits does.
    let type_index: HashMap<&str, usize> =
        program.types.iter().enumerate().map(|(i, t)| (t.name.as_str(), i)).collect();
    escape.field_count.retain(|ty, n| {
        *n == 2
            && type_index.get(ty.as_str()).is_some_and(|&i| {
                inference.type_fields.get(i).is_some_and(|fields| {
                    fields.len() == 2
                        && fields[0] & INT != 0
                        && fields[0] & !infer::ANY_INT == 0
                        && fields[1] & FAIL == 0
                })
            })
    });
    let packable: crate::hash::Set<String> = escape.field_count.keys().cloned().collect();
    escape.keep_types(|ty| packable.contains(ty));
    let byte_disc = ByteDiscs::from(crate::dispatch::byte_dispatched(program, &inference));
    // One `Analysis` for all three, rather than one each: see
    // `linear::for_the_emitter`.
    let (in_place_pushes, reusable_records, (builder_joins, builder_params, builder_carried)) =
        crate::linear::for_the_emitter(program);
    // Beat loops rewind the arena between iterations. Groups returning the
    // by-value %parsed are excluded: k_beat_pop judges heap-ness from the
    // returned tag word, and the packed representation would mislead it.
    let mut beat = crate::beat::beat_loops(program, &inference, &in_place_pushes);
    beat.ids.retain(|(n, a), _| escape.returns_ty(n, *a).is_none());
    beat.demoted.retain(|(_, callee)| beat.ids.contains_key(callee));
    let forwarders = forwarder_map(program);
    let framed = framed_views(program, &forwarders);
    let mut backend = Backend {
        convention,
        inline_helpers,
        program,
        forwarders,
        sub_parents: program
            .types
            .iter()
            .filter_map(|t| t.parent.clone().map(|p| (t.name.clone(), p)))
            .collect(),
        typesets: program.flat_typesets(),
        group_by_name: group_indices_by_name(program),
        cycle_reached: cycle_reached(program),
        cycle_members: cycle_members(program),
        twin_of: None,
        kept_out: kept_out(program),
        framed_views: framed,
        inference,
        escape,
        byte_disc,
        in_place_pushes,
        reusable_records,
        builder_joins,
        builder_params,
        builder_carried,
        beat,
        type_ids,
        strings: Vec::new(),
        globals: String::new(),
        interned: HashMap::default(),
        body: String::new(),
        sealed: Vec::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
        builtin_value_wrappers: Vec::new(),
        defers_self_reference: !knotted.is_empty() || ties(program),
        knotted,
        print_value_wrapper: false,
        caf_cells: Vec::new(),
        ahead_cells: Vec::new(),
        closure_cells: Vec::new(),
        closure_consts: Vec::new(),
        demand: crate::demand::analyze(program),
        thunk_sites: Vec::new(),
        unread_positions: unread_positions.clone(),
        left_out,
        asked_left_out: std::cell::Cell::new(false),
    };
    backend.emit()
}

/// What the emitter has written once it has walked the program: the body in
/// the pieces it was sealed in, and the tables the head of the module is
/// written from. `emit` hands this back and the `Backend` goes, with every
/// analysis and index it held, before `finish` allocates the module. Kept
/// together, they were under the module's allocation, which was the most the
/// tab's compile held.
struct Written {
    convention: ClosureConvention,
    inline_helpers: bool,
    has_entry: bool,
    /// Whether the emitter asked by name about a group the walk left out.
    asked_left_out: bool,
    pieces: Vec<String>,
    closure_consts: Vec<(String, String, usize)>,
    caf_cells: Vec<String>,
    closure_cells: Vec<String>,
    strings: Vec<(String, Vec<u8>)>,
    globals: String,
}

impl Written {
    /// Whether the body names a symbol of one of `decls`: its dispatcher,
    /// word twin, value wrapper or wrapper record.
    fn names_any<'d>(&self, decls: impl Iterator<Item = &'d FnDecl>) -> bool {
        let mut symbols: Vec<String> = Vec::new();
        for decl in decls {
            let (name, arity) = (decl.name.as_str(), decl.params.len());
            symbols.extend([
                dsym(name, arity),
                tsym(name, arity),
                wsym(name, arity),
                rsym(name, arity),
            ]);
        }
        let queries: crate::hash::Set<&str> = symbols.iter().map(|s| s.as_str()).collect();
        !queries.is_empty()
            && self.pieces.iter().any(|piece| !queries_named(piece, &queries).is_empty())
    }

    /// The module: the body pruned, the declarations and globals it names
    /// written in front of it, and the tail calls narrowed.
    fn finish(self) -> String {
        let pruned = match self.has_entry {
            true => prune_unnamed(self.pieces, &dsym(crate::ast::ENTRY, 0), &self.closure_consts),
            false => self.pieces,
        };
        let pieces: Vec<String> = pruned.into_iter().map(paired_appends).collect();
        // One inline dispatcher per arity the program actually writes. An
        // unused `internal` definition costs nothing after optimization, but
        // it does cost a line, a define and a branch in the emitted golden —
        // so the twins are generated against the body rather than carried in
        // DECLARES the way the other inline helpers are.
        // Every `@sym(` the body writes, read off in one pass. The question
        // below is asked once per declare line -- 163 of them -- and used to
        // be answered by searching the whole emitted body for each, which is
        // 163 scans of a text that grows with the program. On a build of
        // bench/compile_corpus that search was 17.08% of the process.
        let mut body_calls = crate::hash::Set::default();
        let mut body_lines = crate::hash::Set::default();
        for piece in &pieces {
            body_calls.extend(called_symbols(piece));
            body_lines.extend(symbols_before_newline(piece));
        }
        let call_twins: String = (0..=4)
            .filter(|n| body_calls.contains(format!("k_call{n}_fast").as_str()))
            .map(|n| call_twin(n, self.convention, self.inline_helpers))
            .chain(
                (0..=4)
                    .filter(|n| body_calls.contains(format!("k_tcall{n}").as_str()))
                    .map(tail_call_twin),
            )
            .collect();
        let declares: String = {
            // BOTH SIDES OF THIS BUILT THE SAME INDEX. kanso#1461 landed one
            // inline, joining DECLARES’s non-declare lines and scanning the
            // three haystacks in place; this branch had already factored the
            // scan into `called_symbols` and cached the DECLARES half in a
            // `OnceLock`, so `body_calls` is computed once above and reused by
            // the call-twin filter rather than rebuilt here. The sets are the
            // same for every query the emitter makes: a symbol from a
            // `declare` line holds no whitespace and no `(`, so bounding the
            // span at the first `(` and bounding it at the first `(` or space
            // cannot disagree about one.
            //
            // `crate::hash::Set` and not std’s, which kanso#1461’s comment
            // gives the reason for and `tests/the_compile_path_hashes_with_a
            // _fixed_seed.rs` pins: std seeds per process, and a randomly
            // seeded table makes this count differ between two runs of one
            // binary, which the compile rows read as a reproduction failure.
            let twin_calls = called_symbols(&call_twins);
            // A symbol DECLARES's own definitions call is kept as well, and
            // each line knows whether it is one: `DeclareLine::context`.
            let called = |sym: &str| body_calls.contains(sym) || twin_calls.contains(sym);
            declares_for_program(called, called, counters_wanted(), self.inline_helpers, true)
        };
        let mut out = String::new();
        out.push_str(&call_twins);
        for cell in &self.caf_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
            let _ = writeln!(out, "@{cell}_ready = internal global i8 0");
        }
        for cell in &self.closure_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
        }
        for (cell, w, arity) in
            self.closure_consts.iter().filter(|(cell, _, _)| body_lines.contains(cell.as_str()))
        {
            // K_INT 0 is the env a zero-capture closure never reads; K_CLOSURE
            // is tag 11. The KClosure layout is the runtime's:
            // { fn, env, int ncaps, int arity, body }, the body being the
            // lambda the wrapper `w_` calls.
            let _ = writeln!(out, "@{cell}_env = internal constant %KValue zeroinitializer");
            let _ = writeln!(
                out,
                "@{cell}_clo = internal constant {{ ptr, ptr, i32, i32, ptr }}                  {{ ptr @{w}, ptr @{cell}_env, i32 0, i32 {arity}, ptr @{} }}",
                w.strip_prefix("w_").unwrap_or(w)
            );
            let _ = writeln!(
                out,
                "@{cell} = internal constant %KValue                  {{ i64 11, i64 ptrtoint (ptr @{cell}_clo to i64) }}"
            );
        }
        // A string is interned when a function asks for it, and the function
        // may since have been pruned: on the codegen corpus 198 of 270
        // strings and 264 of their literal cells were named by nothing, 36% of
        // the module's bytes, each parsed and laid out by clang all the same.
        // Only the body and the type tables name a string.
        let texts: Vec<&str> =
            pieces.iter().map(String::as_str).chain([self.globals.as_str()]).collect();
        let named = named_strings(&texts, self.strings.len());
        drop(texts);
        for ((name, bytes), [string, lit]) in self.strings.iter().zip(named) {
            if string {
                let _ = writeln!(
                    out,
                    "@{name} = private unnamed_addr constant [{} x i8] c\"{}\"",
                    bytes.len(),
                    ir_bytes(bytes)
                );
            }
            if lit {
                let _ = writeln!(out, "@{name}_lit = internal global %KValue zeroinitializer");
            }
        }
        // THE MODULE IS WRITTEN ONCE, at its own size. The declarations and
        // everything above go in first and the body's pieces follow, each let
        // go as soon as it is copied, and the narrowing rewrites the lines
        // where they stand. The body used to be copied behind the globals and
        // the narrowing wrote that again into a buffer of its own, so two
        // copies of the module were held at once.
        out.push_str(&self.globals);
        out.push('\n');
        let declared = declares.len() + 1;
        let size = declared + out.len() + pieces.iter().map(String::len).sum::<usize>();
        let mut module = String::with_capacity(size);
        module.push_str(&declares);
        module.push('\n');
        module.push_str(&out);
        drop((declares, out));
        for piece in pieces {
            module.push_str(&piece);
        }
        through_doors(narrowed_in_place(module, declared), self.convention)
    }
}

/// The runtime functions the program calls that take `preserve_nonecc`
/// wherever closures do. src/runtime.c defines each with `K_DOORCC`, which is
/// the same attribute under the same probe, and says why these four;
/// `tests/the_doors_the_program_calls_agree.rs` holds the two lists equal,
/// since a door on one convention called on the other passes its arguments
/// in the wrong registers.
pub const PRESERVE_NONE_DOORS: [&str; 4] =
    ["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice", "k_b_utf8"];

/// Every declare of and call to a door, written with the convention.
///
/// One pass over the module. Every door is spelled `k_b_...`, so the pass
/// stops only where `%KValue @k_b_` appears, and writes the convention in
/// front of it when a door's name and its `(` follow and a `declare ` or a
/// `call ` comes before. This was eight `str::replace` calls, each building
/// the whole module again: on a one-line `kanso play` under clang 19 they were
/// 402,596 of the 1,067,649 instructions under `kanso::main`.
fn through_doors(ir: String, convention: ClosureConvention) -> String {
    if convention != ClosureConvention::PreserveNone {
        return ir;
    }
    const MARK: &str = "%KValue @k_b_";
    let mut out = String::with_capacity(ir.len() + 4096);
    let mut rest = ir.as_str();
    while let Some(at) = rest.find(MARK) {
        let (head, tail) = rest.split_at(at);
        let name = &tail["%KValue @".len()..];
        let door = PRESERVE_NONE_DOORS
            .iter()
            .any(|d| name.strip_prefix(d).is_some_and(|after| after.starts_with('(')));
        out.push_str(head);
        if door && (head.ends_with("declare ") || head.ends_with("call ")) {
            out.push_str("preserve_nonecc ");
        }
        out.push_str(MARK);
        rest = &tail[MARK.len()..];
    }
    out.push_str(rest);
    out
}

struct Backend<'a> {
    /// see `ClosureConvention`; decided by the caller, never probed here
    convention: ClosureConvention,
    /// The group whose word twin is being emitted, while it is. See
    /// `twin_positions`.
    twin_of: Option<(String, usize)>,
    /// Whether the runtime's helpers go out as `alwaysinline`: yes for a
    /// release build, where inlining them is the point, and no for a dev
    /// build. See `emit_ir_dev`.
    inline_helpers: bool,
    program: &'a Program,
    /// Which declarations make up each group, by name, in program order.
    ///
    /// `group_indices` used to answer this by scanning `program.fns` end to
    /// end and collecting the matches into a fresh `Vec<usize>`, and
    /// `group_param_set` and `group_return_set` ask it once per parameter and
    /// once per call site. That is a question about the whole program asked
    /// once per name -- the shape kanso#1464, kanso#1468, kanso#1473 and
    /// kanso#1475 each removed from somewhere else -- and it cost 66,271,847
    /// instructions of `kanso build bench/runbench` over 7,556 calls.
    ///
    /// Keyed by NAME, with arity filtered off the result, for the reason
    /// kanso#1475's index is: the names come from the program's own
    /// declarations but also from call sites, and a `&str` key borrows as
    /// `str` where a tuple key would demand the program's lifetime.
    group_by_name: HashMap<&'a str, Vec<usize>>,
    /// Names a cycle can reach, which get no cohort bracket. See
    /// `cycle_reached`.
    cycle_reached: crate::hash::Set<&'a str>,
    /// Names on a cycle, the groups that get a word twin. See `has_twin`.
    cycle_members: crate::hash::Set<&'a str>,
    /// See `kept_out`.
    kept_out: crate::hash::Set<&'a str>,
    /// See `framed_views`: (group, arity, span of the `bytes` call).
    framed_views: crate::hash::Set<(String, usize, Span)>,
    inference: infer::Inference,
    forwarders: Forwarders,
    /// subtype name -> parent name; non-empty programs get chain-aware
    /// dispatch checks, everyone else keeps the exact ones
    sub_parents: HashMap<String, String>,
    /// typeset name -> members; an annotated param matches any member
    typesets: HashMap<String, Vec<String>>,
    escape: crate::escape::EscapeInfo,
    byte_disc: ByteDiscs,
    in_place_pushes: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    reusable_records: crate::hash::Map<(std::sync::Arc<str>, usize, usize), String>,
    builder_joins: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    builder_params: crate::linear::Slots,
    /// Argument positions already carrying the builder, so no seed is needed.
    builder_carried: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    beat: crate::beat::Beats,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    /// Constant tables the module defines, written beside the interned
    /// strings rather than into `body`, so the scans that read the body for
    /// calls do not walk them.
    globals: String,
    interned: HashMap<Vec<u8>, String>,
    /// The functions written so far, in pieces. `body` is the piece being
    /// written and `sealed` the ones before it, in order. A piece is closed
    /// when the next function would not fit in the room it has, so the text
    /// is never copied into a bigger buffer while it is written. Doubling one
    /// buffer held the old copy and the new one at once, and on
    /// `bench/interp_corpus` that moment was the tab's high-water mark.
    body: String,
    sealed: Vec<String>,
    lift_counter: usize,
    fn_value_wrappers: Vec<(String, usize)>,
    /// (builtin, arity) pairs a program hands out as values, each needing a
    /// wrapper a dynamic call can reach.
    builtin_value_wrappers: Vec<(String, usize)>,
    defers_self_reference: bool,
    /// Zero-arity names that reach themselves through other constants.
    knotted: crate::hash::Set<String>,
    print_value_wrapper: bool,
    /// Work-ahead's lifted constants (#1856) and the cell each was given,
    /// in the order they were emitted: the pool forces these.
    ahead_cells: Vec<(String, String)>,
    /// One cache cell per frozen constant, emitted as globals at the end,
    /// each beside an `i8` that says whether its builder has run. Nothing
    /// fills these before main any more: a constant builds itself on the
    /// first read, so one nobody reads is never built.
    caf_cells: Vec<String>,
    /// one permanent slot per zero-capture lambda site, built on first visit
    closure_cells: Vec<String>,
    /// A lambda that captures nothing is one value for the whole run, so it
    /// is a link-time constant rather than a slot filled on first visit.
    /// The fold's `%fnp = load ptr, ptr %c` then folds to the wrapper and
    /// the indirect call through the closure becomes a direct one LLVM can
    /// inline. (cell, wrapper, arity)
    closure_consts: Vec<(String, String, usize)>,
    demand: crate::demand::DemandInfo<'a>,
    /// (site evaluator symbol, captured-arg count), indexed by site id.
    thunk_sites: Vec<(String, usize)>,
    /// (group, arity, position) an arm the unbuilt-arm prune dropped was
    /// reading and no arm left reads. See `without_unbuilt_arms`.
    unread_positions: Vec<(String, usize, usize)>,
    /// The groups the tab's walk took out of the program, and whether the
    /// emitter asked about one by name. See `emit_written`.
    left_out: crate::hash::Set<String>,
    asked_left_out: std::cell::Cell<bool>,
}

/// The frame epilogue: release each releasable cell unless the outgoing
/// value IS that cell (the returned-thunk case escapes upward, counted by
/// the runtime). Returns the value register to hand to `ret` — threading
/// through the helper keeps the IR linear.
fn release_cells(f: &mut FnEmit, value: &str) -> String {
    if f.lazy_cells.is_empty() || f.parsed.contains_key(value) {
        return value.to_string();
    }
    let cells = f.lazy_cells.clone();
    let mut v = value.to_string();
    for cell in cells {
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue @k_thunk_release_unless(%KValue {cell}, %KValue {v})"
        ));
        v = t.to_string();
    }
    v
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
    sets: HashMap<String, Set>,
    /// Temps carrying the by-value %parsed type rather than a boxed KValue.
    /// Operands living in the by-value convention, and the record type
    /// each one holds — boxing one back needs to name its type and its id.
    parsed: crate::hash::Map<String, (String, i64)>,
    /// Err-origin prefix "{fn lazy_cells: Vec::new(), } at {file}" for the declaration being emitted.
    origin_prefix: String,
    /// Source file of the declaration being emitted, for keying push sites.
    file: std::sync::Arc<str>,
    /// LLVM return type of the function being emitted: `%parsed` or `%KValue`.
    ret_ty: String,
    /// Dispatcher group being emitted, for recognizing self-tail-calls.
    group: String,
    arity: usize,
    /// Whether the current arm is a bare enrollment clone: library plumbing
    /// wearing an unqualified name, which the cohort license must not read
    /// as user code.
    synthetic: bool,
    /// The package this arm belongs to. An arm cannot see an err its own
    /// package raised, and this is the side of that comparison the compiler
    /// knows; the err carries the other.
    hako: String,
    /// Registers of releasable lazy cells born in this body; every return
    /// path releases each unless the result aliases it.
    lazy_cells: Vec<String>,
    /// Stack slots the body asked for, held back for the entry block.
    entry_allocas: Vec<String>,
    /// Whether a byte view's header lives in this frame, which turns the
    /// body's tail calls into plain ones: `framed_views` says why.
    frame_held: bool,
    /// A non-strict byte index builds a `%KValue` — the byte, or none — and a
    /// byte discriminator immediately collapses it back to one i64. The box
    /// is a phi over a struct, and LLVM will not sink the `extractvalue` into
    /// the diamond's two arms, so the collapse survives to machine code as
    /// four instructions on the hot path. This maps such a box to the raw
    /// i64 phi emitted beside it, so the crossing can take the raw form and
    /// leave the box for the dead-code pass.
    raw_byte: crate::hash::Map<String, String>,
    /// Whether the hot predicates are called in their two-word forms, which
    /// the dev tier's instruction selector can lower where it cannot lower a
    /// call passing a `%KValue`.
    words: bool,
    /// Values whose two words are already in hand, tag then payload. An
    /// unboxed parameter crosses as a raw i64 and is boxed on entry, and every
    /// read of its tag or payload used to take it back apart with an
    /// `extractvalue`; the tag is 0 and the payload is the argument itself.
    known_words: crate::hash::Map<String, (String, String)>,
    /// In a twin, the tail call into the general body with this body's own
    /// arguments, and its return type. `bail_on` takes it when a word
    /// overflows, so a sum the twin computes stays a word: the general body
    /// starts again from the arguments and computes the bignum itself.
    general: Option<(String, String)>,
    /// The buffer `line_fmt` formats into, kept between lines so a formatted
    /// line costs no allocation of its own.
    scratch: String,
    /// Whether this body has written anything that running it a second time
    /// would write again: a store, or a call to anything but an intrinsic or
    /// a read-only predicate. A bail re-runs the body from its entry, so it
    /// is taken only while this is false.
    effects: bool,
    /// `effects` as it stood when a switch dispatcher finished its entry. A
    /// switch's arms are reached only from the switch, never from each other,
    /// so each arm starts from this rather than from whatever the arm emitted
    /// before it wrote.
    arms_effects: Option<bool>,
}
/// Whether an emitted line changes something a second run of the body would
/// see: a store, or a call to anything but an LLVM intrinsic or one of the
/// predicates that only read their operand. An allocation counts, which is
/// cautious: a second run would allocate again and leak nothing, but the
/// runtime's region and beat calls change state the same way and are not
/// told apart here.
fn writes_state(line: &str) -> bool {
    const READS: [&str; 6] = [
        "@k_b_at_fast(",
        "@k_truthy(",
        "@k_truthy_w(",
        "@k_str_lit_fast(",
        "@k_check_rec_fast(",
        "@k_check_rec_fast_w(",
    ];
    line.trim_start().starts_with("store ")
        || (line.contains("call ")
            && !line.contains("@llvm.")
            && !READS.iter().any(|r| line.contains(r)))
}

/// Whether a line the emitters wrote is a stack slot, asked at the ONE place
/// the needle can be.
///
/// A slot reads `%name = alloca <type>`, and `%name` holds no space, so
/// ` = alloca ` begins at the line's FIRST space or it is not there at all.
/// Reading the whole line to learn that is what `FnEmit::write` used to do,
/// and it was the emitter's fourth-largest frame: 18.2 million instructions
/// over 144,261 lines on the build profile, about 126 a line to answer a
/// question the first seven bytes settle. Measured on runbench's 36,086
/// emitted lines, all 235 slots put the needle between offsets five and
/// seven, and each of those is that line's first space.
///
/// The check stays in `write` rather than at the seven sites that emit an
/// alloca, for the reason the comment there gives: the rule has to hold for a
/// site nobody has written yet.
/// `tests/a_stack_slot_is_found_where_the_first_space_is.rs` compiles the
/// corpora and asserts this reading and the whole-line one agree for every
/// line, so a line that ever carried the needle elsewhere is caught rather
/// than quietly written into the wrong block.
pub fn is_a_stack_slot(text: &str) -> bool {
    match text.find(' ') {
        Some(at) => text[at..].starts_with(" = alloca "),
        None => false,
    }
}

impl FnEmit {
    fn new(words: bool) -> Self {
        FnEmit {
            out: String::new(),
            tmp: 0,
            label: 0,
            cur_label: "entry".to_string(),
            versions: HashMap::default(),
            sets: HashMap::default(),
            parsed: crate::hash::Map::default(),
            origin_prefix: String::new(),
            hako: String::new(),
            file: crate::ast::unstamped(),
            ret_ty: "%KValue".to_string(),
            group: String::new(),
            synthetic: false,
            arity: 0,
            lazy_cells: Vec::new(),
            entry_allocas: Vec::new(),
            frame_held: false,
            raw_byte: crate::hash::Map::default(),
            words,
            known_words: crate::hash::Map::default(),
            general: None,
            scratch: String::new(),
            effects: false,
            arms_effects: None,
        }
    }

    /// Writes `{r} = {call}`, where `call` asks one of the hot predicates about
    /// a `%KValue` in the form a release module inlines. The dev tier asks the
    /// two-word form instead (`k_truthy_w` and `k_check_rec_fast_w` in
    /// DECLARES), with the value's words pulled out
    /// first and passed as scalars. At -O0 clang's fast instruction selector
    /// lowers a call only when every argument is a scalar, so each call passing
    /// a `%KValue` went to the slow selector on its own: 216 of them on the
    /// codegen corpus. A release module never calls the two-word forms, so it
    /// never carries them.
    fn predicate(&mut self, r: &str, call: String) {
        const FORMS: [(&str, &str, bool); 2] = [
            ("call i64 @k_truthy(%KValue ", "k_truthy_w", true),
            ("call i64 @k_check_rec_fast(%KValue ", "k_check_rec_fast_w", true),
        ];
        if !self.words {
            self.line_fmt(format_args!("{r} = {call}"));
            return;
        }
        let split = FORMS.iter().find_map(|(prefix, to, pay)| {
            let rest = call.strip_prefix(prefix)?;
            // A constant operand spells its own commas; it keeps the old form.
            let end = rest.find([',', ')']).filter(|_| rest.starts_with('%'))?;
            Some((*to, *pay, rest[..end].to_string(), rest[end..].to_string()))
        });
        match split {
            Some((to, pay, value, rest)) => {
                let tag = self.tmp();
                self.line_fmt(format_args!("{tag} = extractvalue %KValue {value}, 0"));
                let words = match pay {
                    true => {
                        let p = self.tmp();
                        self.line_fmt(format_args!("{p} = extractvalue %KValue {value}, 1"));
                        format!("i64 {tag}, i64 {p}")
                    }
                    false => format!("i64 {tag}"),
                };
                self.line_fmt(format_args!("{r} = call i64 @{to}({words}{rest}"));
            }
            None => self.line_fmt(format_args!("{r} = {call}")),
        }
    }

    fn tmp(&mut self) -> Numbered {
        self.tmp += 1;
        Numbered::new(b"%t", self.tmp)
    }

    fn label(&mut self) -> Numbered {
        self.label += 1;
        Numbered::new(b"L", self.label)
    }

    /// Only a carried argument slot reads the two-word convention. Every other
    /// consumer names its operand as a `%KValue`, and a `%parsed` reaching one
    /// is invalid IR the host's clang refuses. Repairing here rather than at
    /// each consumer is what makes the rule hold for consumers nobody has
    /// written yet: five separate sites were fixed one at a time before this,
    /// and the sixth would have shipped the same way.
    /// `line` for a formatted line, written into a buffer the emitter keeps
    /// rather than a string made for it. A line that boxes an operand calls
    /// back into the emitter, which may format a line of its own; it finds
    /// the buffer taken and uses a fresh one.
    fn line_fmt(&mut self, args: std::fmt::Arguments) {
        let mut text = std::mem::take(&mut self.scratch);
        text.clear();
        let _ = std::fmt::Write::write_fmt(&mut text, args);
        self.line(&text);
        self.scratch = text;
    }

    fn line(&mut self, text: &str) {
        let text = self.boxing_any_parsed_operand(text);
        if self.general.is_some() {
            self.note_effects(&text);
        }
        self.write(&text);
    }

    /// Kept out of `line`, which every emitter calls: only a twin asks.
    #[inline(never)]
    fn note_effects(&mut self, text: &str) {
        if !self.effects && writes_state(text) {
            self.effects = true;
        }
    }

    /// Branches to the general body when `overflow` is set, and carries on in
    /// a block where it is not. False, writing nothing, when this is not a
    /// twin or when the body has already written something a second run
    /// would write again.
    fn can_bail(&self) -> bool {
        self.general.is_some() && !self.effects
    }

    fn bail_on(&mut self, overflow: &str) -> bool {
        if !self.can_bail() {
            return false;
        }
        let ok = self.label();
        let out = self.label();
        self.line_fmt(format_args!("br i1 {overflow}, label %{out}, label %{ok}"));
        self.bail_block(&out);
        self.start_block(&ok);
        true
    }

    /// The block `bail_on` branches to, for a caller that branched to it
    /// itself: it starts the general body over. The block that was open
    /// must already be closed.
    fn bail_block(&mut self, out: &str) {
        let Some((ret, call)) = self.general.clone() else {
            return;
        };
        self.start_block(out);
        let r = self.tmp();
        let kind = if self.frame_held { "call" } else { "musttail call" };
        self.write(&format!("{r} = {kind} {call}"));
        self.write(&format!("ret {ret} {r}"));
    }

    /// Every stack slot the body asks for is held back and written at the top
    /// of the entry block, whichever block asked for it. An alloca standing in
    /// a later block is a dynamic stack object to LLVM: the function keeps a
    /// frame pointer and restores `rsp` through it on every return, and the
    /// slot is claimed again on each pass. Sixty-eight of the decoder's
    /// seventy-three slots stood that way and the decode paid 8.78% for them.
    /// Diverting here rather than at each emitter is what makes the rule hold
    /// for a site nobody has written yet.
    ///
    /// A slot hoisted out of a loop body is shared between passes rather than
    /// claimed afresh. Every consumer of one copies out of it before it
    /// returns — `k_rec` and `k_closure` memcpy, `k_list_lit` and `k_map_lit`
    /// build their own storage — so nothing survives a pass to read the next
    /// pass's bytes.
    fn write(&mut self, text: &str) {
        if is_a_stack_slot(text) {
            self.entry_allocas.push(text.to_string());
            return;
        }
        self.out.push_str("  ");
        self.out.push_str(text);
        self.out.push('\n');
    }

    /// The function's body: what the emitters wrote, with the stack slots at
    /// the head of the entry block so each one dominates its uses.
    fn body(&self) -> String {
        let unboxed = self.without_unread_reboxes();
        let out = unboxed.as_deref().unwrap_or(&self.out);
        if self.entry_allocas.is_empty() {
            return out.to_string();
        }
        let mut head = String::new();
        let mut rest = out;
        if let Some(end) = out.find('\n') {
            let first = &out[..end];
            if first.ends_with(':') && !first.starts_with(' ') {
                head.push_str(first);
                head.push('\n');
                rest = &out[end + 1..];
            }
        }
        for slot in &self.entry_allocas {
            head.push_str("  ");
            head.push_str(slot);
            head.push('\n');
        }
        head.push_str(rest);
        head
    }

    /// The body without the boxing of an unboxed parameter nothing reads.
    /// Every read of such a parameter's words is answered from
    /// `known_words`, so in a function that only compares and passes it on,
    /// the `insertvalue` on entry builds a value no line names.
    fn without_unread_reboxes(&self) -> Option<String> {
        let mut out: Option<String> = None;
        for (value, (_, payload)) in &self.known_words {
            let text = out.as_deref().unwrap_or(&self.out);
            let line = format!(
                "  {value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {payload}, 1\n"
            );
            let Some(at) = text.find(&line) else { continue };
            let named = text.match_indices(value.as_str()).any(|(i, _)| {
                i != at + 2
                    && !text[i + value.len()..]
                        .bytes()
                        .next()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            });
            if !named {
                let mut kept = text[..at].to_string();
                kept.push_str(&text[at + line.len()..]);
                out = Some(kept);
            }
        }
        out
    }

    fn boxing_any_parsed_operand<'t>(&mut self, text: &'t str) -> std::borrow::Cow<'t, str> {
        // Most lines name no carried operand, and those go through as they
        // are: copying each into a string of its own was one allocation per
        // line the emitter wrote.
        if self.parsed.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        // Sorted because the map's order is randomized per process, and a line
        // naming two carried operands would otherwise box them in an order
        // that differs between builds of the same program.
        let mut carried: Vec<String> =
            self.parsed.keys().filter(|t| named_as_a_value(text, t)).cloned().collect();
        if carried.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        carried.sort();
        std::borrow::Cow::Owned(carried.iter().fold(text.to_string(), |acc, t| {
            let boxed = self.box_parsed(t);
            let named = format!("%KValue {t}");
            let boxed_as = format!("%KValue {boxed}");
            let inner = acc
                .replace(&format!("{named},"), &format!("{boxed_as},"))
                .replace(&format!("{named})"), &format!("{boxed_as})"));
            match inner.strip_suffix(&named) {
                Some(head) => format!("{head}{boxed_as}"),
                None => inner,
            }
        }))
    }

    /// Undo the by-value convention: rebuild the record the two words hold.
    /// The type is whatever produced the value, which the escape analysis
    /// already knows, because only a returnable type is ever in this shape.
    /// A failure rides in the same two words and comes back as itself: the
    /// runtime asks before it builds, which the inline build here did not.
    fn box_parsed(&mut self, e: &str) -> String {
        let (_, id) = self.parsed[e];
        let w0 = self.tmp();
        self.raw(&format!("{w0} = extractvalue %parsed {e}, 0"));
        let w1 = self.tmp();
        self.raw(&format!("{w1} = extractvalue %parsed {e}, 1"));
        let t = self.tmp();
        self.raw(&format!("{t} = call %KValue @k_parsed_box(i64 {id}, i64 {w0}, i64 {w1})"));
        t.to_string()
    }

    /// A line whose operands are already values: the boxing rewrite emits
    /// through here, so a fresh temp is never scanned against itself.
    fn raw(&mut self, text: &str) {
        self.write(text);
    }

    fn start_block(&mut self, label: &str) {
        self.out.push_str(label);
        self.out.push_str(":\n");
        self.cur_label.clear();
        self.cur_label.push_str(label);
        if let Some(e) = self.arms_effects {
            if label
                .strip_prefix("arm")
                .is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()))
            {
                self.effects = e;
            }
        }
    }

    /// Branches on `value` to the label its case names, or to `dflt`. A
    /// release module writes a `switch`, which its optimiser turns into a jump
    /// table. clang's fast instruction selector at -O0 does not lower one, so
    /// a dev module asks the cases in turn with a compare and a branch each,
    /// which it does.
    fn switch_on(&mut self, value: &str, dflt: &str, cases: &[String]) {
        if !self.words {
            self.line_fmt(format_args!(
                "switch i64 {value}, label %{dflt} [\n{}\n  ]",
                cases.join("\n")
            ));
            return;
        }
        for case in cases {
            let (n, target) = case
                .trim()
                .strip_prefix("i64 ")
                .and_then(|c| c.split_once(", label %"))
                .expect("a case reads `i64 N, label %L`");
            let hit = self.tmp();
            self.line_fmt(format_args!("{hit} = icmp eq i64 {value}, {n}"));
            let next = self.label();
            self.line_fmt(format_args!("br i1 {hit}, label %{target}, label %{next}"));
            self.start_block(&next);
        }
        self.line_fmt(format_args!("br label %{dflt}"));
    }

    fn bind(&mut self, name: &str, temp: &str) {
        self.versions.insert(name.to_string(), temp.to_string());
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.versions.get(name).cloned()
    }

    fn record_parsed(&mut self, operand: &str, ty: &str, id: i64) {
        self.parsed.insert(operand.to_string(), (ty.to_string(), id));
    }

    fn is_parsed(&self, operand: &str) -> bool {
        self.parsed.contains_key(operand)
    }

    fn record(&mut self, operand: &str, set: Set) {
        self.sets.insert(operand.to_string(), set);
    }

    fn set_of(&self, operand: &str) -> Set {
        if operand.starts_with("{ i64 0,") {
            return INT;
        }
        if operand == "{ i64 2, i64 0 }" || operand == "{ i64 3, i64 0 }" {
            return infer::BOOL;
        }
        if operand == "{ i64 4, i64 0 }" {
            return NONE;
        }
        if operand == "{ i64 16, i64 0 }" {
            return infer::DONE;
        }
        self.sets.get(operand).copied().unwrap_or(TOP)
    }
}

/// Dispatchers, wrappers and lambda bodies nothing live names. A block is kept
/// when a chain of names reaches it from the entry or from a block that is not
/// a candidate; a dead caller's call to its callee does not count, and neither
/// does a cycle of dead blocks naming each other. Only `d_`, `w_` and `klam`
/// symbols are candidates: everything else is either the entry, a
/// builder the constant initialiser calls, or a switch the runtime calls by
/// name. A lambda body is named by its wrapper or by the closure built over
/// it, and one in a library function the program never reaches is named by
/// neither; at `-O0` clang compiled each of them anyway.
/// `cells` pairs a constant closure's cell with the wrapper it points at. The
/// pointer lives in a module global rather than in any function body, so the
/// wrapper is named by the cell rather than by a call, and a site that loads
/// the cell is what keeps it.
///
/// The body arrives as the pieces the emitter sealed, and no block crosses a
/// piece. The blocks kept are moved down inside each piece's own buffer rather
/// than gathered into a new one: they are runs of the piece in order, and a
/// second buffer the size of what survives sat at the tab's high-water mark.
fn prune_unnamed(
    pieces: Vec<String>,
    entry: &str,
    cells: &[(String, String, usize)],
) -> Vec<String> {
    // A block never straddles two pieces, because a piece is only closed
    // between two whole writes. `owner` says which piece each block is in.
    let mut owner: Vec<usize> = Vec::new();
    let mut blocks: Vec<(&str, &str)> = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        for block in ir_defines(piece) {
            owner.push(i);
            blocks.push(block);
        }
    }
    let mut alive = vec![false; blocks.len()];
    {
        // Every name this prune will ever ask about: one per block, plus the
        // closure cells. The index is keyed on exactly these, so a name
        // nothing asks about costs nothing to walk past.
        let queries: crate::hash::Set<&str> = blocks
            .iter()
            .map(|(sym, _)| *sym)
            .chain(cells.iter().map(|(cell, _, _)| cell.as_str()))
            .filter(|sym| !sym.is_empty())
            .collect();
        // Which of those names each block writes, read off once per block. The
        // search this replaced asked
        // the question once per (block, name) pair and built a fresh two-way
        // searcher for each: 72.10% of `kanso build bench/runbench`, with
        // `names_symbol` 71.71% of the process on its own.
        // A name the one-pass scan cannot tokenise is answered the old way,
        // per block, rather than assumed away. Emitted symbols are all one of
        // the two shapes `queries_named` reads, so this list is empty in
        // practice -- and the correctness of the index does not rest on that
        // being true, which is the point of having it.
        let odd: Vec<&str> = queries.iter().copied().filter(|sym| !one_pass_reads(sym)).collect();
        let mut names: Vec<crate::hash::Set<&str>> =
            blocks.iter().map(|(_, text)| queries_named(text, &queries)).collect();
        for (written, (_, text)) in names.iter_mut().zip(blocks.iter()) {
            for sym in &odd {
                if names_symbol(text, sym) {
                    written.insert(sym);
                }
            }
        }
        // A mark from the roots rather than a count of mentions. Striking a
        // block once nothing else named it was reference counting, and a
        // cycle names itself: std/list's merge sort is merge, merge_on, pick
        // and advance calling round, so a program that never sorts struck
        // `sort`, `msort` and `span` and kept the four, which nothing could
        // call.
        let at_sym: crate::hash::Map<&str, usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| !sym.is_empty())
            .map(|(at, (sym, _))| (*sym, at))
            .collect();
        let wrapped: crate::hash::Map<&str, &str> =
            cells.iter().map(|(cell, w, _)| (cell.as_str(), w.as_str())).collect();
        let mut work: Vec<usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| {
                // The runtime calls the thunk dispatcher itself, from
                // `k_force`, so no emitted line names it.
                *sym == entry
                    || *sym == "d_thunk_eval"
                    || !(sym.starts_with("d_")
                        || sym.starts_with("w_")
                        || sym.starts_with("klam")
                        || sym.starts_with("\"d_")
                        || sym.starts_with("\"w_"))
            })
            .map(|(at, _)| at)
            .collect();
        for &at in &work {
            alive[at] = true;
        }
        while let Some(at) = work.pop() {
            for name in &names[at] {
                // A cell is a module global, so the wrapper it points at is
                // named by whoever loads the cell rather than by a call.
                let target = wrapped.get(name).map_or(*name, |w| *w);
                for sym in [*name, target] {
                    if let Some(&next) = at_sym.get(sym) {
                        if !alive[next] {
                            alive[next] = true;
                            work.push(next);
                        }
                    }
                }
            }
        }
    }
    let mut kept: Vec<Vec<std::ops::Range<usize>>> = vec![Vec::new(); pieces.len()];
    for ((&i, (_, text)), alive) in owner.iter().zip(&blocks).zip(alive) {
        if alive {
            let at = text.as_ptr() as usize - pieces[i].as_ptr() as usize;
            kept[i].push(at..at + text.len());
        }
    }
    drop(blocks);
    pieces
        .into_iter()
        .zip(kept)
        .map(|(piece, runs)| {
            let mut bytes = piece.into_bytes();
            let mut to = 0;
            for run in runs {
                let len = run.len();
                bytes.copy_within(run, to);
                to += len;
            }
            // The room a piece was opened with is given back, a piece at a
            // time, before the module they are copied into is allocated.
            bytes.truncate(to);
            bytes.shrink_to_fit();
            String::from_utf8(bytes).expect("every block ends at a line break")
        })
        .collect()
}

/// Whether `queries_named` can tokenise this symbol, which is to say whether
/// it is one of the two shapes the emitter writes: an unquoted run of
/// `[alnum _]`, or a quoted name from `"` to its closing `"` with no other
/// quote inside. Anything else is answered by `names_symbol` per block, so
/// the index stays exact without assuming the emitter's spellings.
fn one_pass_reads(sym: &str) -> bool {
    match sym.strip_prefix('"') {
        Some(rest) => match rest.strip_suffix('"') {
            Some(inner) => !inner.contains('"'),
            None => false,
        },
        None => !sym.is_empty() && sym.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
    }
}

/// Which of `queries` this text names, in one pass, answering exactly what
/// `names_symbol` answers for each of them.
///
/// The delimiter rule is `names_symbol`'s: `@sym` counts only where the byte
/// after it is not alphanumeric, `_` or `"`. Two token shapes cover every
/// symbol the emitter writes, and `the_only_two_shapes_a_symbol_takes` pins
/// it: an unquoted name is a run of those same bytes, and a quoted one runs
/// from `"` to the next `"`, which is how `quoted()` spells a name holding a
/// slash or an operator character. Scanning the unquoted run alone would stop
/// at the slash inside `@"d_add/2"` and miss it.
fn queries_named<'a>(text: &str, queries: &crate::hash::Set<&'a str>) -> crate::hash::Set<&'a str> {
    let held = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'"';
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        at = from;
        let end = match bytes.get(from) {
            Some(b'"') => match bytes[from + 1..].iter().position(|b| *b == b'"') {
                Some(close) => from + close + 2,
                None => continue,
            },
            _ => {
                let mut end = from;
                while end < bytes.len() && held(bytes[end]) {
                    end += 1;
                }
                end
            }
        };
        if matches!(bytes.get(end), Some(b) if held(*b)) {
            continue;
        }
        if let Some(name) = queries.get(&text[from..end]) {
            found.insert(*name);
        }
    }
    found
}

/// Every declaration's index, by name, in program order.
///
/// `Backend::group_indices` scanned `program.fns` end to end and collected the
/// matches into a fresh `Vec<usize>` on every call, and `group_param_set` and
/// `group_return_set` ask it once per parameter and once per call site.
fn group_indices_by_name(program: &Program) -> HashMap<&str, Vec<usize>> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        by_name.entry(decl.name.as_str()).or_default().push(at);
    }
    by_name
}

/// Every declared name a cycle can reach: the members of every cycle in what
/// the bodies mention, and everything those members mention, onward.
///
/// A cohort bracket costs a push and a pop on every call it wraps, and pays
/// only when the call leaves garbage worth a rewind. A name a cycle reaches
/// can run once per element of something -- a parser's recursive descent, a
/// loop, a helper one of those calls per number -- and its calls are the
/// small ones. A name no cycle reaches runs a number of times fixed by
/// straight-line code, which is where a phase of a program starts and ends.
/// Names, not groups, and a mention counts whether it is a call or a value.
/// A cycle through a closure some library calls back is not seen; that miss
/// costs speed and nothing else, because the bracket is sound on any call
/// the license admits.
fn cycle_reached(program: &Program) -> crate::hash::Set<&str> {
    let (names, adj) = call_graph(program);
    let mut hot = vec![false; names.len()];
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !hot[at] {
            hot[at] = true;
            queue.extend(adj[at].iter().copied().filter(|&next| !hot[next]));
        }
    }
    names.iter().zip(hot).filter(|(_, h)| *h).map(|(n, _)| *n).collect()
}

/// The names on a cycle of the call graph: the loops, and the recursions
/// that are not loops. `cycle_reached` is these and everything they call.
fn cycle_members(program: &Program) -> crate::hash::Set<&str> {
    let (names, adj) = call_graph(program);
    let mut out = crate::hash::Set::default();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            out.extend(scc.into_iter().map(|at| names[at]));
        }
    }
    out
}

/// Each declared name, and the names its bodies mention, by index.
fn call_graph(program: &Program) -> (Vec<&str>, Vec<Vec<usize>>) {
    fn mentions(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |child| mentions(child, index, out));
    }
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for decl in &program.fns {
        let from = index[decl.name.as_str()];
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    mentions(expr, &index, &mut adj[from])
                }
            }
        }
    }
    (names, adj)
}

/// Functions a dispatcher calls from its heavy arms, kept out of line.
///
/// A group of clauses compiles to one function with a switch at its head, and
/// LLVM gives that function one frame. When one arm inlines a callee that
/// loops, the loop's registers are callee-saved ones, so the prologue pushes
/// six of them and the epilogue pops them on every call -- including the calls
/// whose arm is a single append. The encoder's `encode_onto` is that shape:
/// 2,380,860 calls on runbench, 1,438,110 of them scalars that paid the frame
/// of the string arm's inlined escaper. A callee that can reach a cycle, named
/// in an arm of a group that also has an arm reaching none, stays a call, and
/// the dispatcher's cheap arms keep a frame of their own size.
fn kept_out(program: &Program) -> crate::hash::Set<&str> {
    fn mentions(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |child| mentions(child, index, out));
    }
    fn decl_mentions(decl: &FnDecl, index: &HashMap<&str, usize>) -> Vec<usize> {
        let mut out = Vec::new();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => mentions(expr, index, &mut out),
            }
        }
        out
    }
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    let per_decl: Vec<Vec<usize>> = program.fns.iter().map(|d| decl_mentions(d, &index)).collect();
    for (decl, m) in program.fns.iter().zip(&per_decl) {
        adj[index[decl.name.as_str()]].extend(m.iter().copied());
    }
    // loops: a name in a cycle, and every name that can reach one
    let mut loops = vec![false; names.len()];
    let mut callers: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (from, to) in adj.iter().enumerate() {
        for &t in to {
            callers[t].push(from);
        }
    }
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !loops[at] {
            loops[at] = true;
            queue.extend(callers[at].iter().copied().filter(|&c| !loops[c]));
        }
    }
    let mut out = crate::hash::Set::default();
    let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(at);
    }
    for ((name, _), clauses) in &groups {
        if clauses.len() < 2 {
            continue;
        }
        // a switch on the argument's type, where the arms do unrelated work
        let typed = |c: &usize| {
            program.fns[*c]
                .params
                .iter()
                .any(|p| matches!(p, crate::ast::Pattern::Annotated { .. }))
        };
        if !clauses.iter().any(typed) {
            continue;
        }
        let own = index[name];
        let heavy = |c: &usize| per_decl[*c].iter().any(|&m| m != own && loops[m]);
        if clauses.iter().all(heavy) || !clauses.iter().any(heavy) {
            continue;
        }
        for c in clauses {
            for &m in &per_decl[*c] {
                if m != own && loops[m] {
                    out.insert(names[m]);
                }
            }
        }
    }
    out
}

/// The byte views that live in their function's frame. `bytes s` of a
/// string writes a three-word header -- length, data, capacity -- and the
/// header is all it allocates, since the view borrows the string's bytes.
/// JSON's `escape_onto` makes one per string it writes, 942,750 on the run
/// program, and reads it for a length, a scan and some slices before it
/// returns. A header in the arena costs the bump and every read through it;
/// a header in the frame is three stores LLVM takes apart into registers.
///
/// The frame must outlive every read. So a binding `x = bytes e` qualifies
/// when its function sits on no cycle of the call graph, which keeps the
/// frame from being claimed once per pass of a loop, and every mention of `x`
/// after it is one of these reads:
///   - the first argument of `length`, `find2`, `find2_below` or `slice`,
///     none of which keeps the header (a slice writes a header of its own
///     over the same bytes);
///   - the base of an index;
///   - an argument to a function whose parameter there is itself read only
///     this way, in every clause, found as the largest such set.
///
/// Anything else fails it: a return, a record or list, a closure that
/// could run after the frame is gone, a binding (which the demand pass
/// may make lazy), an argument to a name the body binds itself. A function
/// holding such a view makes its tail calls as plain calls, because a tail
/// call gives the frame back before the callee reads the header.
fn framed_views(
    program: &Program,
    forwarders: &Forwarders,
) -> crate::hash::Set<(String, usize, Span)> {
    struct Cx<'a> {
        groups: &'a HashMap<(&'a str, usize), Vec<&'a FnDecl>>,
        forwarders: &'a Forwarders,
        safe: &'a crate::hash::Set<(String, usize, usize)>,
        locals: crate::hash::Set<&'a str>,
    }
    fn pattern_names<'a>(p: &'a Pattern, out: &mut crate::hash::Set<&'a str>) {
        match p {
            Pattern::Var(n, _) => {
                out.insert(n.as_str());
            }
            Pattern::Annotated { name, .. } => {
                out.insert(name.as_str());
            }
            Pattern::Ctor { fields, whole, .. } => {
                for f in fields {
                    pattern_names(f, out);
                }
                if let Some(w) = whole {
                    out.insert(w.0.as_str());
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
    fn stmt_expr(st: &Stmt) -> &Expr {
        match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) => expr,
        }
    }
    fn locals_of<'a>(expr: &'a Expr, out: &mut crate::hash::Set<&'a str>) {
        match expr {
            Expr::Lambda { params, .. } => {
                for (n, _) in params {
                    out.insert(n.as_str());
                }
            }
            Expr::Block(stmts, _) => {
                for st in stmts {
                    if let Stmt::Bind { pattern, .. } = st {
                        pattern_names(pattern, out);
                    }
                }
            }
            Expr::Guard { rest, .. } => {
                for st in rest.iter() {
                    if let Stmt::Bind { pattern, .. } = st {
                        pattern_names(pattern, out);
                    }
                }
            }
            _ => {}
        }
        crate::for_each_child(expr, |c| locals_of(c, out));
    }
    // every name a body binds for itself, beside its parameters
    fn bound_in(stmts: &[Stmt]) -> crate::hash::Set<&str> {
        let mut out = crate::hash::Set::default();
        for st in stmts {
            if let Stmt::Bind { pattern, .. } = st {
                pattern_names(pattern, &mut out);
            }
            locals_of(stmt_expr(st), &mut out);
        }
        out
    }
    fn mentions(expr: &Expr, x: &str) -> bool {
        if let Expr::Ident(n, _, _) = expr {
            if n.as_str() == x {
                return true;
            }
        }
        let mut found = false;
        crate::for_each_child(expr, |c| found = found || mentions(c, x));
        found
    }
    fn is_x(expr: &Expr, x: &str) -> bool {
        matches!(expr, Expr::Ident(n, _, _) if n.as_str() == x)
    }
    fn stmts_read_only(stmts: &[Stmt], x: &str, cx: &Cx) -> bool {
        stmts.iter().all(|st| match st {
            Stmt::Expr(e) => reads_only(e, x, cx),
            Stmt::Bind { expr, .. } => !mentions(expr, x),
        })
    }
    fn reads_only(expr: &Expr, x: &str, cx: &Cx) -> bool {
        match expr {
            Expr::Ident(n, _, _) => n.as_str() != x,
            Expr::Lambda { .. } => !mentions(expr, x),
            Expr::Index { base, index, .. } if is_x(base, x) => reads_only(index, x, cx),
            Expr::Block(stmts, _) => stmts_read_only(stmts, x, cx),
            Expr::Guard { cond, early, rest, .. } => {
                reads_only(cond, x, cx) && reads_only(early, x, cx) && stmts_read_only(rest, x, cx)
            }
            Expr::App { head, args, piped: false, .. } => {
                let Expr::Ident(h, _, _) = head.as_ref() else {
                    return !mentions(expr, x);
                };
                let h = h.as_str();
                if h == x {
                    return false;
                }
                let n = args.len();
                let local = cx.locals.contains(h);
                let forwarded = cx.forwarders.get(h, n);
                let user = !local && forwarded.is_none() && cx.groups.contains_key(&(h, n));
                let builtin = match (local, forwarded) {
                    (true, _) => None,
                    (false, Some(b)) => Some(b),
                    (false, None) if user => None,
                    (false, None) => Some(h.strip_prefix("builtin_").unwrap_or(h)),
                };
                args.iter().enumerate().all(|(i, a)| {
                    if !is_x(a, x) {
                        return reads_only(a, x, cx);
                    }
                    match builtin {
                        Some(b) => {
                            i == 0 && matches!(b, "length" | "find2" | "find2_below" | "slice")
                        }
                        None => user && cx.safe.contains(&(h.to_string(), n, i)),
                    }
                })
            }
            _ => {
                let mut ok = true;
                crate::for_each_child(expr, |c| ok = ok && reads_only(c, x, cx));
                ok
            }
        }
    }
    let mut groups: HashMap<(&str, usize), Vec<&FnDecl>> = HashMap::default();
    for d in &program.fns {
        groups.entry((d.name.as_str(), d.params.len())).or_default().push(d);
    }
    fn locals_for(d: &FnDecl) -> crate::hash::Set<&str> {
        let mut out = bound_in(&d.body);
        for p in &d.params {
            pattern_names(p, &mut out);
        }
        out
    }
    // the bindings that could qualify, before anything is asked of their uses
    let mut candidates = Vec::new();
    for d in &program.fns {
        if d.is_getter() {
            continue;
        }
        for (i, st) in d.body.iter().enumerate() {
            let Stmt::Bind { pattern: Pattern::Var(x, _), expr } = st else { continue };
            let Expr::App { head, args, piped: false, span } = expr else { continue };
            let Expr::Ident(h, _, _) = head.as_ref() else { continue };
            if args.len() != 1 {
                continue;
            }
            let target = match forwarders.get(h, 1) {
                Some(b) => b,
                None if groups.contains_key(&(h.as_str(), 1)) => continue,
                None => h.strip_prefix("builtin_").unwrap_or(h),
            };
            if target == "bytes" {
                candidates.push((d, i, x.as_str(), h.as_str(), *span));
            }
        }
    }
    if candidates.is_empty() {
        return crate::hash::Set::default();
    }
    // names on a cycle of the call graph, which may not hold a view
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for d in &program.fns {
        index.entry(d.name.as_str()).or_insert_with(|| {
            names.push(d.name.as_str());
            names.len() - 1
        });
    }
    fn callees(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |c| callees(c, index, out));
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for d in &program.fns {
        let from = index[d.name.as_str()];
        for st in &d.body {
            callees(stmt_expr(st), &index, &mut adj[from]);
        }
    }
    let mut cyclic = vec![false; names.len()];
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            for at in scc {
                cyclic[at] = true;
            }
        }
    }
    candidates.retain(|(d, ..)| !cyclic[index[d.name.as_str()]]);
    if candidates.is_empty() {
        return crate::hash::Set::default();
    }
    // the parameters a view can reach, found by following it into each callee
    // it is handed to; nothing else is ever asked about
    fn handed(
        expr: &Expr,
        x: &str,
        locals: &crate::hash::Set<&str>,
        user: &dyn Fn(&str, usize) -> bool,
        out: &mut Vec<(String, usize, usize)>,
    ) {
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(h, _, _) = head.as_ref() {
                if !locals.contains(h.as_str()) && user(h.as_str(), args.len()) {
                    for (i, a) in args.iter().enumerate() {
                        if is_x(a, x) {
                            out.push((h.to_string(), args.len(), i));
                        }
                    }
                }
            }
        }
        crate::for_each_child(expr, |c| handed(c, x, locals, user, out));
    }
    let user = |h: &str, n: usize| !forwarders.contains(h, n) && groups.contains_key(&(h, n));
    let mut reached: crate::hash::Set<(String, usize, usize)> = crate::hash::Set::default();
    let mut work = Vec::new();
    for (d, i, x, _, _) in &candidates {
        let locals = locals_for(d);
        for st in &d.body[i + 1..] {
            handed(stmt_expr(st), x, &locals, &user, &mut work);
        }
    }
    while let Some(key) = work.pop() {
        if !reached.insert(key.clone()) {
            continue;
        }
        for c in &groups[&(key.0.as_str(), key.1)] {
            if let Pattern::Var(p, _) = &c.params[key.2] {
                let locals = locals_for(c);
                for st in &c.body {
                    handed(stmt_expr(st), p, &locals, &user, &mut work);
                }
            }
        }
    }
    // of those, the ones read only as a view, largest set first: every
    // position whose clauses all take a plain name or `_`, then drop the ones
    // a clause's body uses otherwise until nothing more drops
    let mut safe: crate::hash::Set<(String, usize, usize)> = reached
        .into_iter()
        .filter(|(name, n, i)| {
            groups[&(name.as_str(), *n)].iter().all(|c| {
                !c.is_getter() && matches!(c.params[*i], Pattern::Var(..) | Pattern::Wildcard(_))
            })
        })
        .collect();
    loop {
        let mut drop = Vec::new();
        for key in &safe {
            let clauses = &groups[&(key.0.as_str(), key.1)];
            let holds = clauses.iter().all(|c| {
                let Pattern::Var(p, _) = &c.params[key.2] else { return true };
                let mut cx = Cx { groups: &groups, forwarders, safe: &safe, locals: locals_for(c) };
                // the parameter itself is not a callee the body may bind
                cx.locals.remove(p.as_str());
                let twice = c
                    .params
                    .iter()
                    .enumerate()
                    .any(|(j, q)| j != key.2 && matches!(q, Pattern::Var(o, _) if o == p));
                let shadowed = twice || bound_in(&c.body).contains(p.as_str());
                !shadowed && stmts_read_only(&c.body, p, &cx)
            });
            if !holds {
                drop.push(key.clone());
            }
        }
        if drop.is_empty() {
            break;
        }
        for key in drop {
            safe.remove(&key);
        }
    }
    let mut out = crate::hash::Set::default();
    for (d, i, x, h, span) in candidates {
        let locals = locals_for(d);
        if locals.contains(h) {
            continue;
        }
        let rest = &d.body[i + 1..];
        let mut params = crate::hash::Set::default();
        for p in &d.params {
            pattern_names(p, &mut params);
        }
        let rebound = params.contains(x) || bound_in(rest).contains(x);
        let mut cx = Cx { groups: &groups, forwarders, safe: &safe, locals };
        cx.locals.remove(x);
        if !rebound && stmts_read_only(rest, x, &cx) {
            out.insert((d.name.clone(), d.params.len(), span));
        }
    }
    out
}

/// The lookup over that index. Keyed by NAME with arity filtered off the
/// result, because the names come from call sites as well as declarations and
/// a `&str` key borrows as `str` where a tuple key would demand the program's
/// lifetime.
fn group_indices_in<'s>(
    by_name: &'s HashMap<&str, Vec<usize>>,
    program: &'s Program,
    name: &str,
    arity: usize,
) -> impl Iterator<Item = usize> + 's {
    by_name
        .get(name)
        .map_or(&[][..], |v| v.as_slice())
        .iter()
        .copied()
        .filter(move |at| program.fns[*at].params.len() == arity)
}

/// Whether `text` writes a branch to `label`: `label %name` with the name
/// ending there, so `fail1` is not answered by `label %fail10`.
fn branches_to(text: &str, label: &str) -> bool {
    let probe = format!("label %{label}");
    text.match_indices(&probe).any(|(at, _)| {
        !text[at + probe.len()..]
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
    })
}

/// Which interned strings the texts name, by index: whether `@sN` appears,
/// and whether `@sN_lit` does. `intern` names string N `sN`, so a name is read
/// as a number and nothing is hashed. A name ends where LLVM's unquoted names
/// do, at the first byte that is not a letter, digit, `_`, `.` or `$`, so
/// `@s12` never answers for `@s12_lit` or `@s120`.
///
/// Collecting every `@name` into a set and asking it was 23,334 instructions
/// of `kanso play`'s start-up on a one-line program, most of it hashing names
/// nothing would ask about.
fn named_strings(texts: &[&str], count: usize) -> Vec<[bool; 2]> {
    let name_byte = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$');
    let mut named = vec![[false; 2]; count];
    for text in texts {
        let bytes = text.as_bytes();
        let mut at = 0;
        while let Some(next) = text[at..].find("@s") {
            let from = at + next + 2;
            at = from;
            let mut to = from;
            let mut n = 0usize;
            while to < bytes.len() && bytes[to].is_ascii_digit() {
                n = n.saturating_mul(10).saturating_add(usize::from(bytes[to] - b'0'));
                to += 1;
            }
            if to == from || n >= count {
                continue;
            }
            let ends = |i: usize| !bytes.get(i).is_some_and(|b| name_byte(*b));
            if ends(to) {
                named[n][0] = true;
            } else if bytes[to..].starts_with(b"_lit") && ends(to + 4) {
                named[n][1] = true;
            }
        }
    }
    named
}

/// Every `sym` for which `text` writes `@sym(`, collected in one pass.
///
/// This answers `text.contains(&format!("@{sym}("))` exactly, for any `sym`
/// that holds no newline and no `(` -- which every LLVM symbol here does.
/// From each `@` the scan runs to the first `(`, stopping at a newline, so a
/// probe can never straddle two lines; that matters because one of the three
/// places this replaces searched DECLARES line by line rather than whole.
fn called_symbols(text: &str) -> crate::hash::Set<&str> {
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        let mut to = from;
        while to < bytes.len() && bytes[to] != b'(' && bytes[to] != b'\n' {
            to += 1;
        }
        if to < bytes.len() && bytes[to] == b'(' {
            found.insert(&text[from..to]);
        }
        at = from;
    }
    found
}

/// Every `sym` for which `text` writes `@sym\n`, collected in one pass.
///
/// A line can only end in `@sym`, so only the LAST `@` on each line can start
/// a match and the whole text is read once. That makes the answer exact for a
/// `sym` holding no `@` and no newline, which every symbol the emitter asks
/// about is: `@a @b\n` would have to be found under the name `a @b`, and
/// reading each line's last at alone cannot see it.
///
/// The forward scan this replaced ran from every `@` to the end of its line,
/// which on a big emitted body is the text times the number of ats: it cost
/// `kanso build bench/runbench` 209 million instructions, turning a fall into
/// a rise.
fn symbols_before_newline(text: &str) -> crate::hash::Set<&str> {
    // One walk over the bytes, holding the last `@` since the line began. A
    // search for each newline and then for the `@` before it was two calls a
    // line, 42,784 lines in the tab's compile of its corpus.
    let mut found = crate::hash::Set::default();
    let mut last_at = None;
    for (i, &b) in text.as_bytes().iter().enumerate() {
        match b {
            b'@' => last_at = Some(i),
            b'\n' => {
                if let Some(at) = last_at.take() {
                    found.insert(&text[at + 1..i]);
                }
            }
            _ => {}
        }
    }
    found
}

/// The symbols DECLARES calls from its own inline definitions, which is every
/// line of it that is not itself a `declare`. DECLARES is a constant, so this
/// is read once for the life of the process rather than once per emit.
/// The symbols DECLARES calls from its own inline definitions, written down
/// rather than scanned for.
///
/// DECLARES is a `const`: the same 65 names in every process kanso runs.
/// Reading them off it once cost 62 answers for 1,024 lines of scanning, and
/// the scanning was 94% of the index this branch builds -- 292,701 of the
/// 278,812 `called_symbols` charges on the start-up corpus came through
/// `Once::call_once_force`, over 1,023 calls, against 18,159 from the emitter's
/// two. On a program that emits one `print` that is the whole of the index's
/// cost, and `kanso play` paid it to learn nothing it could not have been told.
///
/// Sorted, which is what the oracle's binary search needs. Emit never asks
/// the list: each `declare` line reads `DeclareLine::context`, computed from
/// it when the compiler is built.
///
/// `tests/the_declares_symbols_are_the_ones_declares_calls.rs` recomputes this
/// list from DECLARES with the same scan it replaces and asserts they are the
/// same set, so an edit to DECLARES that adds or drops a call turns that spec
/// red rather than silently leaving a symbol out of a program's declares.
const DECLARES_CONTEXT_CALLS: &[&str] = &[
    "k_b_append",
    "k_b_append_byte",
    "k_b_append_mut",
    "k_b_append_mut_byte",
    "k_b_append_mut_int",
    "k_b_append_mut_int2",
    "k_b_append_mut_word",
    "k_b_append_slice",
    "k_b_append_slice_fast",
    "k_b_append_word_slow",
    "k_b_at",
    "k_b_at_fast",
    "k_b_bit_and",
    "k_b_bit_and_fast",
    "k_b_bit_not",
    "k_b_bit_not_fast",
    "k_b_bit_or",
    "k_b_bit_or_fast",
    "k_b_bit_shl",
    "k_b_bit_shl_fast",
    "k_b_bit_shr",
    "k_b_bit_shr_fast",
    "k_b_bit_xor",
    "k_b_bit_xor_fast",
    "k_b_bytes",
    "k_b_bytes_fast",
    "k_b_bytes_frame",
    "k_b_find2",
    "k_b_find2_below",
    "k_b_find2_below_fast",
    "k_b_find2_below_raw",
    "k_b_find2_fast",
    "k_b_find2_raw",
    "k_b_length",
    "k_b_length_fast",
    "k_b_push_mut",
    "k_b_push_mut_fast",
    "k_b_push_mut_known",
    "k_b_put_mut",
    "k_b_put_mut_fast",
    "k_b_slice",
    "k_b_slice_fast",
    "k_b_slice_raw",
    "k_b_utf8_slice",
    "k_b_utf8_slice_fast",
    "k_b_utf8_slice_raw",
    "k_bool",
    "k_check_bool",
    "k_check_int",
    "k_check_rec",
    "k_check_rec_fast",
    "k_check_rec_fast_w",
    "k_check_tag",
    "k_field",
    "k_field_fast",
    "k_float",
    "k_force",
    "k_force_fast",
    "k_index",
    "k_index_fast",
    "k_int",
    "k_none",
    "k_str_lit",
    "k_str_lit_fast",
    "k_truthy",
    "k_truthy_bad",
    "k_truthy_w",
    "llvm.assume",
    "llvm.memcpy.p0.p0.i64",
];

/// The binary search each `declare` line used to ask at emit time, kept as
/// the oracle for `DeclareLine::context`.
#[cfg(test)]
fn declares_context_calls(sym: &str) -> bool {
    DECLARES_CONTEXT_CALLS.binary_search(&sym).is_ok()
}

#[cfg(test)]
mod every_declare_line_knows_whether_declares_calls_it {
    use super::{declares_context_calls, DECLARES, DECLARES_DEV, DECLARE_LINES, DECLARE_LINES_DEV};

    #[test]
    fn the_flag_is_the_search() {
        let mut asked = 0;
        for (text, lines) in [(DECLARES, &DECLARE_LINES), (DECLARES_DEV, &DECLARE_LINES_DEV)] {
            for line in lines.iter().filter(|l| l.sym_start < l.sym_end) {
                let sym = &text[line.sym_start..line.sym_end];
                assert_eq!(line.context, declares_context_calls(sym), "{sym}");
                asked += usize::from(line.context);
            }
        }
        assert!(asked > 0, "no declare line is one DECLARES calls, so this proves nothing");
    }
}

#[cfg(test)]
mod the_declares_symbols_are_the_ones_declares_calls {
    use super::{called_symbols, DECLARES, DECLARES_CONTEXT_CALLS};

    /// The scan the static replaces, kept as the oracle.
    fn scanned() -> std::collections::BTreeSet<&'static str> {
        let mut found = std::collections::BTreeSet::new();
        for line in DECLARES.lines().filter(|l| !l.starts_with("declare")) {
            found.extend(called_symbols(line));
        }
        found
    }

    /// Written down and scanned for are the same set. An edit to DECLARES that
    /// adds or drops a call turns this red rather than leaving a symbol out of
    /// a program's declares, where the only symptom is a link error in whatever
    /// program happens to reach it.
    #[test]
    fn the_written_list_is_what_the_scan_finds() {
        let written: std::collections::BTreeSet<&str> =
            DECLARES_CONTEXT_CALLS.iter().copied().collect();
        let found = scanned();
        let missing: Vec<&&str> = found.difference(&written).collect();
        let extra: Vec<&&str> = written.difference(&found).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "DECLARES_CONTEXT_CALLS has drifted from DECLARES.\n               missing (DECLARES calls it, the list does not name it): {missing:?}\n               extra (the list names it, DECLARES does not call it): {extra:?}"
        );
    }

    /// And it is sorted, because it is asked with a binary search.
    #[test]
    fn the_list_is_sorted_and_holds_no_duplicate() {
        let mut sorted = DECLARES_CONTEXT_CALLS.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            DECLARES_CONTEXT_CALLS,
            &sorted[..],
            "the list is asked with binary_search, which answers nonsense on an \
             unsorted slice and answers it quietly"
        );
        sorted.dedup();
        assert_eq!(sorted.len(), DECLARES_CONTEXT_CALLS.len(), "the list repeats a symbol");
    }

    /// The population is not empty, so the two tests above are not agreeing
    /// about nothing.
    #[test]
    fn the_list_is_not_empty() {
        assert!(
            DECLARES_CONTEXT_CALLS.len() > 40,
            "only {} symbols, where DECLARES's inline definitions called sixty-two \
             when this was written",
            DECLARES_CONTEXT_CALLS.len()
        );
    }
}

#[cfg(test)]
mod called_symbols_agrees {
    use super::*;

    /// What the emitter asked before the index existed, kept as the oracle.
    fn searched(text: &str, sym: &str) -> bool {
        text.contains(&format!("@{sym}("))
    }

    /// Every query the emitter makes, asked both ways, over a text.
    ///
    /// The symbols are DECLARES's own -- the real 163 -- plus every prefix and
    /// suffix of each, because a span-based index can agree on a whole name
    /// and still disagree on one letter of it, and the emitter's question is
    /// `@sym(` where a prefix would answer for a longer symbol if the index
    /// stored anything looser than the run up to the paren.
    fn agrees_over(text: &str) {
        let index = called_symbols(text);
        let mut asked = 0;
        for line in DECLARES.lines() {
            let Some(rest) = line.strip_prefix("declare ") else { continue };
            let Some(at) = rest.find('@') else { continue };
            let sym = &rest[at + 1..];
            let Some(paren) = sym.find('(') else { continue };
            let sym = &sym[..paren];
            for end in 1..=sym.len() {
                for start in 0..end {
                    let part = &sym[start..end];
                    assert_eq!(
                        index.contains(part),
                        searched(text, part),
                        "the index and the search disagree on `@{part}(`"
                    );
                    asked += 1;
                }
            }
        }
        assert!(asked > 10_000, "the corpus asked only {asked} questions");
    }

    #[test]
    fn over_the_declares_block() {
        agrees_over(DECLARES);
    }

    /// The `@cell` form, which matches only at the end of a line.
    ///
    /// The scan reads each line's LAST at, so this is the shape that goes
    /// wrong when a line holds more than one.
    #[test]
    fn a_cell_is_named_only_where_its_line_ends() {
        for text in [
            "@a\n",
            "@a",      // no newline, so nothing matches
            "@a @b\n", // only the last at can end the line
            "@a(\n",
            "  @a\n  @b\n",
            "@\n",
            "",
            "@a\n@a\n",
        ] {
            let index = symbols_before_newline(text);
            // Every symbol the emitter asks about is a generated identifier,
            // so the queries here hold no at and no newline -- see the note on
            // symbols_before_newline for the one shape outside that.
            for sym in ["a", "b", "", "a("] {
                assert_eq!(
                    index.contains(sym),
                    text.contains(&format!("@{sym}\n")),
                    "index and search disagree on `@{sym}\\n` in {text:?}"
                );
            }
        }
    }

    /// The edges a span scan can get wrong, each as a whole text.
    #[test]
    fn the_shapes_a_span_scan_can_lose() {
        for text in [
            "@a(",              // the plain case
            "@a",               // no paren at all
            "@a\n(",            // a newline between the name and the paren
            "@@a(",             // two ats, and `@a(` is still in there
            "@a@b(",            // an at inside the span
            "  call @a(i64 1)", // the shape a real line has
            "@a(@b(@c(",        // three in one line
            "",                 // nothing
            "@",                // an at and nothing after it
        ] {
            let index = called_symbols(text);
            for sym in ["a", "b", "c", "@a", "a@b", "", "ab"] {
                assert_eq!(
                    index.contains(sym),
                    searched(text, sym),
                    "index and search disagree on `@{sym}(` in {text:?}"
                );
            }
        }
    }
}

/// Whether this text names `@sym`. A call writes `@sym(`, but a closure hands
/// its wrapper over as `ptr @sym,` — so the delimiter decides, and it also
/// keeps `@w_klam1` from answering for `@w_klam17`.
fn names_symbol(text: &str, sym: &str) -> bool {
    let needle = format!("@{sym}");
    text.match_indices(&needle).any(|(at, _)| {
        !matches!(
            text.as_bytes().get(at + needle.len()),
            Some(b) if b.is_ascii_alphanumeric() || *b == b'_' || *b == b'"'
        )
    })
}

#[cfg(test)]
mod the_prune_agrees_with_the_search {
    use super::*;

    /// The mark written the slow way, kept as the oracle.
    ///
    /// It asks `names_symbol` once per (block, name) pair, which is what made
    /// the search this replaced 72.10% of `kanso build bench/runbench`. It is
    /// the definition of the right answer and nothing else, so it stays here
    /// rather than in the commit message.
    fn by_search(body: &str, entry: &str, cells: &[(String, String, usize)]) -> String {
        let blocks = ir_defines(body);
        let candidate = |sym: &str| {
            sym != entry
                && sym != "d_thunk_eval"
                && (sym.starts_with("d_")
                    || sym.starts_with("w_")
                    || sym.starts_with("klam")
                    || sym.starts_with("\"d_")
                    || sym.starts_with("\"w_"))
        };
        let mut alive: Vec<bool> = blocks.iter().map(|(sym, _)| !candidate(sym)).collect();
        loop {
            let named = |sym: &str| {
                blocks.iter().zip(&alive).any(|((_, text), live)| *live && names_symbol(text, sym))
            };
            let woken = blocks.iter().enumerate().position(|(at, (sym, _))| {
                !alive[at]
                    && (named(sym) || cells.iter().any(|(cell, w, _)| w == sym && named(cell)))
            });
            match woken {
                Some(at) => alive[at] = true,
                None => break,
            };
        }
        blocks.into_iter().zip(alive).filter(|(_, live)| *live).map(|((_, text), _)| text).collect()
    }

    fn define(sym: &str, body: &str) -> String {
        format!("define %KValue @{sym}() {{\nentry:\n{body}\n}}\n")
    }

    /// The prune over a body written as one piece, which is what these
    /// specs were written against.
    fn prune_unnamed(body: String, entry: &str, cells: &[(String, String, usize)]) -> String {
        super::prune_unnamed(vec![body], entry, cells).concat()
    }

    /// The emitter closes a piece between two definitions, so a caller and
    /// its callee can sit in different pieces. The prune reads the names
    /// across all of them, and keeps the same blocks it keeps from one.
    #[test]
    fn a_body_in_pieces_keeps_what_the_whole_body_keeps() {
        let first = format!(
            "{}{}",
            define("d_entry", "  call %KValue @d_a()"),
            define("d_dead", "  call %KValue @d_b()")
        );
        let second = format!(
            "{}{}{}",
            define("d_a", "  call %KValue @d_b()"),
            define("d_b", "  ret %KValue zeroinitializer"),
            define("d_c", "  ret %KValue zeroinitializer")
        );
        let whole = prune_unnamed(format!("{first}{second}"), "d_entry", &[]);
        let pieces = super::prune_unnamed(vec![first, second], "d_entry", &[]);
        assert_eq!(pieces.concat(), whole, "the pieces kept different blocks");
        assert!(whole.contains("@d_b()") && !whole.contains("@d_c("), "{whole}");
    }

    fn agree_on(body: &str, entry: &str, cells: &[(String, String, usize)]) {
        assert_eq!(
            prune_unnamed(body.to_string(), entry, cells),
            by_search(body, entry, cells),
            "the index and the search kept different blocks"
        );
    }

    /// A chain the fixpoint has to walk down: the entry names nothing, so
    /// `d_a` goes, and only then is `d_b` unnamed, and only then `d_c`.
    #[test]
    fn a_chain_that_falls_one_round_at_a_time() {
        let body = [
            define("d_entry", "  ret %KValue zeroinitializer"),
            define("d_a", "  %x = call %KValue @d_b()\n  ret %KValue %x"),
            define("d_b", "  %x = call %KValue @d_c()\n  ret %KValue %x"),
            define("d_c", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        assert!(!prune_unnamed(body.clone(), "d_entry", &[]).contains("@d_c("));
        agree_on(&body, "d_entry", &[]);
    }

    /// Blocks that name each other and nothing live names. std/list's merge
    /// sort is four of them calling round, and counting mentions kept all
    /// four in every program that imported the library and never sorted.
    #[test]
    fn a_cycle_nothing_reaches_goes_whole() {
        let body = [
            define("d_entry", "  %x = call %KValue @d_kept()\n  ret %KValue %x"),
            define("d_kept", "  ret %KValue zeroinitializer"),
            define("d_merge", "  %x = call %KValue @d_pick()\n  ret %KValue %x"),
            define("d_pick", "  %x = call %KValue @d_merge()\n  ret %KValue %x"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@d_kept("), "the named block was pruned");
        assert!(!kept.contains("@d_merge("), "a cycle nothing reaches was kept");
        assert!(!kept.contains("@d_pick("), "a cycle nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// The delimiter rule, which is the whole reason `names_symbol` exists:
    /// `@w_klam1` must not answer for `@w_klam17`.
    #[test]
    fn a_longer_name_does_not_answer_for_a_shorter_one() {
        let body = [
            define("d_entry", "  %x = call %KValue @w_klam17()\n  ret %KValue %x"),
            define("w_klam1", "  ret %KValue zeroinitializer"),
            define("w_klam17", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@w_klam17("), "the named wrapper was pruned");
        assert!(!kept.contains("define %KValue @w_klam1()"), "the unnamed wrapper was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A quoted name, which is how `quoted()` spells one holding a slash. The
    /// one-pass scan reads it from quote to quote; a run over the delimiter
    /// class alone would stop at the slash and lose it.
    #[test]
    fn a_quoted_name_is_read_to_its_closing_quote() {
        let body = [
            define("d_entry", "  %x = call %KValue @\"d_add/2\"()\n  ret %KValue %x"),
            define("\"d_add/2\"", "  ret %KValue zeroinitializer"),
            define("\"d_sub/2\"", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@\"d_add/2\"("), "the named quoted block was pruned");
        assert!(!kept.contains("@\"d_sub/2\"()"), "the unnamed quoted block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A lambda body is a candidate too. Its wrapper names it, so it goes in
    /// the round after the wrapper does; one a closure is built over is named
    /// by that construction and stays.
    #[test]
    fn a_lambda_body_goes_with_the_wrapper_that_named_it() {
        let body = [
            define(
                "d_entry",
                "  %c = call %KValue @k_closure(ptr @klam5, i64 1)\n  ret %KValue %c",
            ),
            define("w_klam3", "  %x = call %KValue @klam3()\n  ret %KValue %x"),
            define("klam3", "  ret %KValue zeroinitializer"),
            define("klam5", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("define %KValue @klam5("), "the lambda a closure names was pruned");
        assert!(!kept.contains("define %KValue @klam3("), "the lambda nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A wrapper is kept when a CELL that stands for it is named, which is the
    /// second half of the question and the one that reads `cells`.
    #[test]
    fn a_wrapper_lives_while_its_cell_is_named() {
        let body = [
            define("d_entry", "  %x = load %KValue, ptr @k_clo_7\n  ret %KValue %x"),
            define("w_klam3", "  ret %KValue zeroinitializer"),
            define("w_klam4", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let cells = [("k_clo_7".to_string(), "w_klam3".to_string(), 1usize)];
        let kept = prune_unnamed(body.clone(), "d_entry", &cells);
        assert!(kept.contains("@w_klam3("), "the wrapper its cell names was pruned");
        assert!(!kept.contains("@w_klam4("), "the wrapper nothing names was kept");
        agree_on(&body, "d_entry", &cells);
    }

    /// A name the one-pass scan cannot tokenise falls back to the search, so
    /// the index is exact without assuming how a symbol is spelled.
    #[test]
    fn an_odd_name_is_answered_the_old_way() {
        assert!(one_pass_reads("d_foo"));
        assert!(one_pass_reads("\"d_add/2\""));
        assert!(!one_pass_reads("d_foo.bar"));
        assert!(!one_pass_reads("\"un\"closed"));
        let body = [
            define("d_entry", "  %x = call %KValue @d_foo.bar()\n  ret %KValue %x"),
            define("d_foo.bar", "  ret %KValue zeroinitializer"),
            define("d_foo.baz", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@d_foo.bar("), "the named odd block was pruned");
        assert!(!kept.contains("@d_foo.baz("), "the unnamed odd block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// Real emitted IR, with the symbols the emitter really writes.
    ///
    /// The text is what `emit_ir` produced, so the prune has already run over
    /// it once; feeding it back asks both implementations the same several
    /// hundred questions about a body neither was written against.
    #[test]
    fn real_emitted_ir_reads_the_same_both_ways() {
        let source = "fn count 0 acc\n  acc\n\nfn count n acc\n  count (n - 1) (acc + n)\n\nmain = print \"{count 10 0}\"\n";
        let program = crate::compile("sample.kso", source, false).expect("the sample compiles");
        let ir = emit_ir(&program, ClosureConvention::Absent).expect("the sample lowers");
        assert!(ir.lines().count() > 200, "the sample emitted only {} lines", ir.lines().count());
        for entry in ["d_main", "d_count"] {
            agree_on(&ir, entry, &[]);
        }
    }
}

/// Split emitted IR into segments, keeping every byte. A `define` segment
/// runs from its header to the closing brace on its own line and carries its
/// symbol; everything between definitions — globals the fnref statics live in
/// among them — is a segment with no symbol, which the prune never touches.
fn ir_defines(body: &str) -> Vec<(&str, &str)> {
    // Every block is a run of whole lines, so it is a slice of the body
    // rather than a copy: the prune that reads these runs at the emitter's
    // high-water mark, beside the body it splits.
    let mut out: Vec<(&str, &str)> = Vec::new();
    let mut sym = "";
    let mut from = 0;
    let mut at = 0;
    let mut inside = false;
    for line in body.split_inclusive('\n') {
        if !inside && line.starts_with("define ") {
            if at > from {
                out.push(("", &body[from..at]));
                from = at;
            }
            inside = true;
            sym = line
                .find('@')
                .and_then(|a| line[a + 1..].find('(').map(|p| &line[a + 1..a + 1 + p]))
                .unwrap_or_default();
        }
        at += line.len();
        if inside && line.trim_end() == "}" {
            out.push((std::mem::take(&mut sym), &body[from..at]));
            from = at;
            inside = false;
        }
    }
    if at > from {
        out.push(("", &body[from..at]));
    }
    out
}

/// Whether an emitted line names this temp in a `%KValue` operand position.
/// The delimiter matters: `%t2` is a prefix of `%t20`, and boxing the wrong
/// register writes a program that type-checks and computes the wrong record.
fn named_as_a_value(text: &str, temp: &str) -> bool {
    // Asked of every line for every carried temp, so the needle is not built:
    // each place the temp appears is checked for the type in front of it.
    text.match_indices(temp).any(|(at, _)| {
        text[..at].ends_with("%KValue ")
            && matches!(text.as_bytes().get(at + temp.len()), Some(b',') | Some(b')') | None)
    })
}

/// The dispatcher, inlined at the call site for the shape that actually
/// happens: a closure of the arity written, with no failure in an argument.
///
/// `k_call2` is 26 instructions and a fold applies it once a lap. Ten of the
/// 26 ask about the callable — is it a failure, is it a closure, does its
/// arity match — and a fold passes the same callable through its self-call
/// unchanged, so those ten are loop-invariant and LICM can hoist them out of
/// the loop TailCallElim makes of the recursion. It cannot hoist across a
/// call, which is what the runtime dispatcher is: LTO sees the body and
/// declines to inline it on cost, and `always_inline` on the C definition is
/// ignored because the emitted `.ll` calls the symbol by name. So the test
/// is emitted here instead, in the module the optimizer is already in.
///
/// Every shape the fast arm does not cover falls through to `k_call{n}`,
/// which re-asks everything and answers exactly as before — a failing
/// callable, a fnref, a wrong arity, a failing argument. That is why the
/// order here may differ from the runtime's: the arm only fires where all
/// the orders agree.
fn call_twin(n: usize, convention: ClosureConvention, inline: bool) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let attr = if inline { " alwaysinline" } else { "" };
    let _ = writeln!(s, "define internal %KValue @k_call{n}_fast(%KValue %f{args}){attr} {{");
    let _ = writeln!(s, "  %ftag = extractvalue %KValue %f, 0");
    let _ = writeln!(s, "  %isclo = icmp eq i64 %ftag, 11");
    let _ = writeln!(s, "  br i1 %isclo, label %arity, label %slow");
    let _ = writeln!(s, "arity:");
    let _ = writeln!(s, "  %fp = extractvalue %KValue %f, 1");
    let _ = writeln!(s, "  %c = inttoptr i64 %fp to ptr");
    let _ = writeln!(s, "  %arp = getelementptr i8, ptr %c, i64 20");
    let _ = writeln!(s, "  %ar = load i32, ptr %arp");
    let _ = writeln!(s, "  %okar = icmp eq i32 %ar, {n}");
    let _ = writeln!(s, "  br i1 %okar, label %args, label %slow");
    let _ = writeln!(s, "args:");
    for i in 0..n {
        let _ = writeln!(s, "  %t{i} = extractvalue %KValue %a{i}, 0");
        let _ = writeln!(s, "  %e{i} = icmp eq i64 %t{i}, 5");
    }
    for i in 1..n {
        let prev = match i {
            1 => "%e0".to_string(),
            _ => format!("%or{}", i - 1),
        };
        let _ = writeln!(s, "  %or{i} = or i1 {prev}, %e{i}");
    }
    match n {
        0 => {
            let _ = writeln!(s, "  br label %go");
        }
        1 => {
            let _ = writeln!(s, "  br i1 %e0, label %slow, label %go");
        }
        _ => {
            let _ = writeln!(s, "  br i1 %or{}, label %slow, label %go", n - 1);
        }
    }
    let _ = writeln!(s, "go:");
    let _ = writeln!(s, "  %envp = getelementptr i8, ptr %c, i64 8");
    let _ = writeln!(s, "  %env = load ptr, ptr %envp");
    let _ = writeln!(s, "  %fnp = load ptr, ptr %c");
    // The closure pointer this loads was written by the wrapper emitter, so
    // the two carry the same convention or the arguments land in the wrong
    // registers. `%fnp` is only ever a closure body: the arm tested tag == 11
    // above, so the fnref family (no env parameter) never reaches here.
    let cc = convention.keyword();
    let _ = writeln!(s, "  %r = call {cc}%KValue %fnp(ptr %env{args})");
    let _ = writeln!(s, "  ret %KValue %r");
    let _ = writeln!(s, "slow:");
    let _ = writeln!(s, "  %s = call %KValue @k_call{n}(%KValue %f{args})");
    let _ = writeln!(s, "  ret %KValue %s");
    let _ = writeln!(s, "}}");
    s
}

/// The twin of `k_call{n}` for a call in tail position. A lambda's body is on the
/// tailcc convention and the closure carries a pointer to it, so a function
/// value called last jumps to the lambda instead of calling its wrapper: a
/// loop written in continuation-passing style runs in constant stack, as
/// Appendix A says a tail call does. Anything the jump does not take (a
/// closure the runtime built, a partial, a reference to a group, a failing
/// argument) goes to `k_call{n}`, which answers as before. The caller
/// reaches this with a musttail of its own, so the helper is a function and
/// never inlined.
fn tail_call_twin(n: usize) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let _ = writeln!(s, "define internal tailcc %KValue @k_tcall{n}(%KValue %f{args}) {{");
    let _ = writeln!(s, "  %ftag = extractvalue %KValue %f, 0");
    let _ = writeln!(s, "  %isclo = icmp eq i64 %ftag, 11");
    let _ = writeln!(s, "  br i1 %isclo, label %arity, label %slow");
    let _ = writeln!(s, "arity:");
    let _ = writeln!(s, "  %fp = extractvalue %KValue %f, 1");
    let _ = writeln!(s, "  %c = inttoptr i64 %fp to ptr");
    let _ = writeln!(s, "  %arp = getelementptr i8, ptr %c, i64 20");
    let _ = writeln!(s, "  %ar = load i32, ptr %arp");
    let _ = writeln!(s, "  %okar = icmp eq i32 %ar, {n}");
    let _ = writeln!(s, "  br i1 %okar, label %args, label %slow");
    let _ = writeln!(s, "args:");
    for i in 0..n {
        let _ = writeln!(s, "  %t{i} = extractvalue %KValue %a{i}, 0");
        let _ = writeln!(s, "  %e{i} = icmp eq i64 %t{i}, 5");
    }
    for i in 1..n {
        let prev = match i {
            1 => "%e0".to_string(),
            _ => format!("%or{}", i - 1),
        };
        let _ = writeln!(s, "  %or{i} = or i1 {prev}, %e{i}");
    }
    match n {
        0 => {
            let _ = writeln!(s, "  br label %body");
        }
        1 => {
            let _ = writeln!(s, "  br i1 %e0, label %slow, label %body");
        }
        _ => {
            let _ = writeln!(s, "  br i1 %or{}, label %slow, label %body", n - 1);
        }
    }
    let _ = writeln!(s, "body:");
    let _ = writeln!(s, "  %bodyp = getelementptr i8, ptr %c, i64 24");
    let _ = writeln!(s, "  %lam = load ptr, ptr %bodyp");
    let _ = writeln!(s, "  %has = icmp ne ptr %lam, null");
    let _ = writeln!(s, "  br i1 %has, label %go, label %slow");
    let _ = writeln!(s, "go:");
    let _ = writeln!(s, "  %envp = getelementptr i8, ptr %c, i64 8");
    let _ = writeln!(s, "  %env = load ptr, ptr %envp");
    let _ = writeln!(s, "  %r = musttail call tailcc %KValue %lam(ptr %env{args})");
    let _ = writeln!(s, "  ret %KValue %r");
    let _ = writeln!(s, "slow:");
    let _ = writeln!(s, "  %s = call %KValue @k_call{n}(%KValue %f{args})");
    let _ = writeln!(s, "  ret %KValue %s");
    let _ = writeln!(s, "}}");
    s
}

/// LLVM symbol for a dispatcher: quoted when the kanso name carries a
/// module qualifier's slash.
fn wsym(name: &str, arity: usize) -> String {
    quoted(&format!("w_{name}_{arity}"))
}

/// The static a `k_fnref` value points at: the wrapper, its arity, and the
/// name the diagnostic says when a call brings the wrong number of arguments.
fn rsym(name: &str, arity: usize) -> String {
    quoted(&format!("r_{name}_{arity}"))
}

/// The ambient group an interpolated value may dispatch to.
const RENDER_GROUP: &str = "render/to_string";

fn dsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}"))
}

/// A group's word twin: the same body, its int parameters taken as raw
/// words. A `dsym` always ends in the arity, so `_w` cannot name a group.
fn tsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}_w"))
}

/// A lookup from a type id to one of `names`, read out of a constant array
/// rather than a switch, with `fallback` for an id past the end.
fn type_table(globals: &mut String, symbol: &str, names: &[String], fallback: &str) {
    let slots = names.len();
    let row: Vec<String> = names.iter().map(|n| format!("ptr @{n}")).collect();
    let _ = writeln!(
        globals,
        "@{symbol}_table = private unnamed_addr constant [{slots} x ptr] [{}]",
        row.join(", ")
    );
    let _ = writeln!(
        globals,
        "define ptr @{symbol}(i64 %id) {{\nentry:\n  %in = icmp ult i64 %id, {slots}\n  \
         br i1 %in, label %T, label %TD\nT:\n  \
         %at = getelementptr [{slots} x ptr], ptr @{symbol}_table, i64 0, i64 %id\n  \
         %name = load ptr, ptr %at\n  ret ptr %name\nTD:\n  ret ptr @{fallback}\n}}\n"
    );
}

/// Word `n` of `value` when it is a literal `{ i64 A, i64 B }`. Reading one
/// off a constant with `extractvalue` is an instruction clang's fast selector
/// at -O0 does not handle, and it sends the rest of the block to the slow one.
fn literal_word(value: &str, n: usize) -> Option<&str> {
    let inner = value.strip_prefix("{ i64 ")?.strip_suffix(" }")?;
    let (a, b) = inner.split_once(", i64 ")?;
    Some(if n == 0 { a } else { b })
}

/// Runs `rewind` with every argument in `lifted` that is a bignum when the
/// edge is reached moved out of the arena around it. A bignum the iteration
/// built sits above the mark the rewind returns to; the runtime copies each
/// one out before the rewind and back in after it. An edge whose arguments
/// are all words pays two tag tests and takes no call.
fn lift_around(
    f: &mut FnEmit,
    emitted: &mut [String],
    lifted: &[usize],
    rewind: &mut dyn FnMut(&mut FnEmit, &mut [String]),
) {
    if lifted.is_empty() {
        rewind(f, emitted);
        return;
    }
    let mut any: Option<String> = None;
    for &j in lifted {
        let t = inline_tag(f, &emitted[j]);
        let b = f.tmp();
        f.line_fmt(format_args!("{b} = icmp eq i64 {t}, {K_BIG}"));
        any = Some(match any {
            None => b.to_string(),
            Some(prev) => {
                let o = f.tmp();
                f.line_fmt(format_args!("{o} = or i1 {prev}, {b}"));
                o.to_string()
            }
        });
    }
    let any = any.expect("a lift lifts something");
    let n = lifted.len();
    let arr = match n {
        1 => String::new(),
        _ => {
            let arr = f.tmp();
            f.line_fmt(format_args!("{arr} = alloca [{n} x %KValue]"));
            arr.to_string()
        }
    };
    let sets: Vec<Set> = lifted.iter().map(|&j| f.set_of(&emitted[j])).collect();
    // Moves the lifted arguments through a runtime call, and answers what
    // came back: one by value, several through `arr`.
    let through = |f: &mut FnEmit, values: &[String], call: &str| -> Vec<String> {
        if n == 1 {
            let v = f.tmp();
            f.line_fmt(format_args!("{v} = call %KValue @{call}1(%KValue {})", values[0]));
            return vec![v.to_string()];
        }
        for (k, v) in values.iter().enumerate() {
            let at = f.tmp();
            f.line_fmt(format_args!(
                "{at} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {k}"
            ));
            f.line_fmt(format_args!("store %KValue {v}, ptr {at}"));
        }
        f.line_fmt(format_args!("call void @{call}(ptr {arr}, i64 {n})"));
        (0..n)
            .map(|k| {
                let at = f.tmp();
                f.line_fmt(format_args!(
                    "{at} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {k}"
                ));
                let v = f.tmp();
                f.line_fmt(format_args!("{v} = load %KValue, ptr {at}"));
                v.to_string()
            })
            .collect()
    };
    // merges each lifted argument from the two ways into `at`
    let merge =
        |f: &mut FnEmit, emitted: &mut [String], moved: &[String], from: &str, via: &str| {
            for (k, &j) in lifted.iter().enumerate() {
                let v = f.tmp();
                f.line_fmt(format_args!(
                    "{v} = phi %KValue [ {}, %{from} ], [ {}, %{via} ]",
                    emitted[j], moved[k]
                ));
                f.record(&v, sets[k]);
                emitted[j] = v.to_string();
            }
        };
    let out = f.label();
    let mid = f.label();
    let before = f.cur_label.clone();
    f.line_fmt(format_args!("br i1 {any}, label %{out}, label %{mid}"));
    f.start_block(&out);
    let current: Vec<String> = lifted.iter().map(|&j| emitted[j].clone()).collect();
    let moved = through(f, &current, "k_big_lift_out");
    f.line_fmt(format_args!("br label %{mid}"));
    f.start_block(&mid);
    merge(f, emitted, &moved, &before, &out);
    rewind(f, emitted);
    let rewound = f.cur_label.clone();
    let back = f.label();
    let done = f.label();
    f.line_fmt(format_args!("br i1 {any}, label %{back}, label %{done}"));
    f.start_block(&back);
    let current: Vec<String> = lifted.iter().map(|&j| emitted[j].clone()).collect();
    let moved = through(f, &current, "k_big_lift_in");
    f.line_fmt(format_args!("br label %{done}"));
    f.start_block(&done);
    merge(f, emitted, &moved, &rewound, &back);
}

/// Runs `rewind` unless one of the `failing` arguments is an err, and merges
/// what the rewind changed back into `emitted`. Inference lets a slot that may
/// hold a failure cross a rewind as a number does, because the err path never
/// reads the slot again. The err does need its storage, though: one built in
/// the lap lies above the mark, and the callee answers it by adding a hop,
/// which the arena places over the record the err wraps. The carry path skips
/// the rewind on an err in a carried slot; this is the same test for the
/// arguments it does not carry.
fn unless_failing(
    f: &mut FnEmit,
    emitted: &mut [String],
    failing: &[usize],
    rewind: &mut dyn FnMut(&mut FnEmit, &mut [String]),
) {
    if failing.is_empty() {
        rewind(f, emitted);
        return;
    }
    let mut any: Option<String> = None;
    for &j in failing {
        let t = inline_tag(f, &emitted[j]);
        let b = f.tmp();
        f.line_fmt(format_args!("{b} = icmp eq i64 {t}, {K_ERR}"));
        any = Some(match any {
            None => b.to_string(),
            Some(prev) => {
                let o = f.tmp();
                f.line_fmt(format_args!("{o} = or i1 {prev}, {b}"));
                o.to_string()
            }
        });
    }
    let any = any.expect("a failing argument was named");
    let run = f.label();
    let done = f.label();
    let before = f.cur_label.clone();
    f.line_fmt(format_args!("br i1 {any}, label %{done}, label %{run}"));
    f.start_block(&run);
    let old: Vec<String> = emitted.to_vec();
    rewind(f, emitted);
    let ran = f.cur_label.clone();
    f.line_fmt(format_args!("br label %{done}"));
    f.start_block(&done);
    for j in 0..emitted.len() {
        if emitted[j] == old[j] {
            continue;
        }
        let set = f.set_of(&emitted[j]);
        let v = f.tmp();
        f.line_fmt(format_args!(
            "{v} = phi %KValue [ {}, %{before} ], [ {}, %{ran} ]",
            old[j], emitted[j]
        ));
        f.record(&v, set);
        emitted[j] = v.to_string();
    }
}

/// An `i1` saying both tags are the int tag, 0. A tag already known to be 0,
/// a literal's or an unboxed parameter's, needs no compare, and two such
/// need no `and`: `n + 1` used to write `icmp eq i64 0, 0` and an `and` for
/// the literal on every addition.
fn both_ints(f: &mut FnEmit, ta: &str, tb: &str) -> String {
    let mut tests: Vec<String> = Vec::new();
    for tag in [ta, tb] {
        if tag != "0" {
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = icmp eq i64 {tag}, 0"));
            tests.push(t.to_string());
        }
    }
    match tests.as_slice() {
        [] => "true".to_string(),
        [one] => one.clone(),
        [a, b] => {
            let both = f.tmp();
            f.line_fmt(format_args!("{both} = and i1 {a}, {b}"));
            both.to_string()
        }
        _ => unreachable!("two tags make at most two tests"),
    }
}

/// Two in-place byte appends in a row, the second onto the first's result and
/// that result read nowhere else in the function, as one call to
/// `k_b_append_mut_int2`. Both calls were already the proven-bytes,
/// proven-int door, so the pair asks the same questions once; see the helper.
///
/// The search goes from one call to the next rather than line by line, and a
/// body with no pair comes back as it was, because this runs on every build
/// and most programs have none.
fn paired_appends(body: String) -> String {
    const CALL: &str = " = call %KValue @k_b_append_mut_int(%KValue ";
    fn parse(line: &str) -> Option<(&str, &str, &str)> {
        let rest = line.strip_prefix("  ")?;
        let (name, rest) = rest.split_once(CALL)?;
        let args = rest.strip_suffix(')')?;
        let (acc, x) = args.split_once(", %KValue ")?;
        Some((name, acc, x))
    }
    let line_end = |from: usize| body[from..].find('\n').map_or(body.len(), |n| from + n);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut from = 0;
    while let Some(n) = body[from..].find(CALL) {
        let at = from + n;
        let start = body[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = line_end(at);
        from = end;
        if end >= body.len() {
            break;
        }
        let next = line_end(end + 1);
        let (Some((a, acc, x)), Some((b, acc2, y))) =
            (parse(&body[start..end]), parse(&body[end + 1..next]))
        else {
            continue;
        };
        if acc2 != a {
            continue;
        }
        // a use count is the function's own
        let first = body[..start].rfind("\ndefine ").map_or(0, |n| n + 1);
        let last = body[next..].find("\n}\n").map_or(body.len(), |n| next + n);
        let used: usize = body[first..last].lines().map(|l| count_operand(l, a)).sum();
        if used == 2 {
            edits.push((
                start,
                next,
                format!("  {b} = call %KValue @k_b_append_mut_int2(%KValue {acc}, %KValue {x}, %KValue {y})"),
            ));
            from = next;
        }
    }
    if edits.is_empty() {
        return body;
    }
    // Each pair becomes one line shorter than the two it replaces, so the
    // body is rewritten where it stands and its text only moves down.
    let mut bytes = body.into_bytes();
    let (mut to, mut kept) = (0, 0);
    for (start, end, line) in edits {
        bytes.copy_within(kept..start, to);
        to += start - kept;
        bytes[to..to + line.len()].copy_from_slice(line.as_bytes());
        to += line.len();
        kept = end;
    }
    let len = bytes.len();
    bytes.copy_within(kept..len, to);
    bytes.truncate(to + len - kept);
    String::from_utf8(bytes).expect("each edit replaces whole lines")
}

/// How many times `name` appears in `line` as a whole operand.
fn count_operand(line: &str, name: &str) -> usize {
    let bytes = line.as_bytes();
    line.match_indices(name)
        .filter(|(at, _)| {
            let after = bytes.get(at + name.len()).copied().unwrap_or(b' ');
            !(after.is_ascii_alphanumeric() || after == b'_' || after == b'.')
        })
        .count()
}

/// One read of a byte run: `x[p + k] == c` or `x[p] == c`, non-strict, with
/// `c` a byte literal and `k` a small literal. Answers `(x, p, k, c)`.
fn byte_read(e: &Expr) -> Option<(&str, &str, i64, i64)> {
    use num_traits::ToPrimitive;
    let Expr::BinOp { op: "==", lhs, rhs, .. } = e else { return None };
    let Expr::Index { base, index, strict: false, .. } = &**lhs else { return None };
    let Expr::Ident(x, _, _) = &**base else { return None };
    let Expr::Int(c, _) = &**rhs else { return None };
    let c = c.to_i64().filter(|c| (0..=255).contains(c))?;
    let (p, k) = match &**index {
        Expr::Ident(p, _, _) => (p, 0),
        Expr::BinOp { op: "+", lhs, rhs, .. } => {
            let Expr::Ident(p, _, _) = &**lhs else { return None };
            let Expr::Int(k, _) = &**rhs else { return None };
            (p, k.to_i64().filter(|k| k.abs() < 1 << 20)?)
        }
        _ => return None,
    };
    Some((x.as_str(), p.as_str(), k, c))
}

/// The bytes and the position a run reads, and each read's offset and byte.
type ByteRun<'e> = (&'e str, &'e str, Vec<(i64, i64)>);

/// A conjunction of `byte_read`s of the same two names. `a and b` desugars to
/// `if a b false`, and `and` groups to the left, so `r1 and r2 and r3` is
/// `if (if r1 r2 false) r3 false`; either side of an `if` may be another. Two
/// reads at least; one is an ordinary compare.
fn byte_run(args: &[Expr]) -> Option<ByteRun<'_>> {
    fn conj<'e>(e: &'e Expr, out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if let Some(read) = byte_read(e) {
            out.push(read);
            return Some(());
        }
        let Expr::App { head, args, piped: false, .. } = e else { return None };
        if !matches!(&**head, Expr::Ident(n, _, _) if n == "if") {
            return None;
        }
        both(args, out)
    }
    fn both<'e>(args: &'e [Expr], out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if args.len() != 3 || !matches!(&args[2], Expr::Ident(n, _, _) if n == "false") {
            return None;
        }
        conj(&args[0], out)?;
        conj(&args[1], out)
    }
    let mut reads = Vec::new();
    both(args, &mut reads)?;
    let (x, p, _, _) = *reads.first()?;
    if reads.len() < 2 || reads.iter().any(|r| (r.0, r.1) != (x, p)) {
        return None;
    }
    Some((x, p, reads.iter().map(|r| (r.2, r.3)).collect()))
}

/// The operands of an argument that ran on words, and the flags that say
/// whether any of its operations overflowed.
type WordFlags = Option<(Vec<String>, Vec<String>)>;

fn word_arith(op: &str) -> bool {
    matches!(op, "+" | "-" | "*" | "/" | "%")
}

fn arith_ops(e: &Expr) -> usize {
    match e {
        Expr::BinOp { op, lhs, rhs, .. } if word_arith(op) => 1 + arith_ops(lhs) + arith_ops(rhs),
        _ => 0,
    }
}

/// The operands of a word run, in the order the operations would evaluate
/// them.
fn arith_leaves<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match e {
        Expr::BinOp { op, lhs, rhs, .. } if word_arith(op) => {
            arith_leaves(lhs, out);
            arith_leaves(rhs, out);
        }
        _ => out.push(e),
    }
}

/// The operations of a word run on raw words. Each one that can leave the
/// word, or divide by zero or by minus one, adds its flag to `flags`; the
/// answer is meaningful only when none is set.
///
/// Given a `slow` label, a flag is a branch there instead, taken where the
/// operation stands. An `or` of the flags made x86 keep each one in a
/// register with `seto` and test the `or` at the end; a branch after each
/// operation is a `jo` on the flag the operation already set.
fn word_run(
    f: &mut FnEmit,
    e: &Expr,
    words: &[String],
    at: &mut usize,
    flags: &mut Vec<String>,
    slow: Option<&str>,
) -> String {
    let Expr::BinOp { op, lhs, rhs, .. } = e else {
        *at += 1;
        return words[*at - 1].clone();
    };
    if !word_arith(op) {
        *at += 1;
        return words[*at - 1].clone();
    }
    let a = word_run(f, lhs, words, at, flags, slow);
    let b = word_run(f, rhs, words, at, flags, slow);
    let t = f.tmp();
    match *op {
        "+" | "-" | "*" => {
            let intrinsic = match *op {
                "+" => "llvm.sadd.with.overflow.i64",
                "-" => "llvm.ssub.with.overflow.i64",
                _ => "llvm.smul.with.overflow.i64",
            };
            let pair = f.tmp();
            f.line_fmt(format_args!("{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {a}, i64 {b})"));
            f.line_fmt(format_args!("{t} = extractvalue {{ i64, i1 }} {pair}, 0"));
            let over = f.tmp();
            f.line_fmt(format_args!("{over} = extractvalue {{ i64, i1 }} {pair}, 1"));
            flag(f, over.to_string(), flags, slow);
        }
        _ => {
            if !matches!(b.parse::<i64>(), Ok(d) if d != 0 && d != -1) {
                let zero = f.tmp();
                f.line_fmt(format_args!("{zero} = icmp eq i64 {b}, 0"));
                let minus = f.tmp();
                f.line_fmt(format_args!("{minus} = icmp eq i64 {b}, -1"));
                let edge = f.tmp();
                f.line_fmt(format_args!("{edge} = or i1 {zero}, {minus}"));
                flag(f, edge.to_string(), flags, slow);
                // the instruction must not trap on the path it does not take
                let safe = f.tmp();
                f.line_fmt(format_args!("{safe} = select i1 {edge}, i64 1, i64 {b}"));
                let insn = if *op == "%" { "srem" } else { "sdiv" };
                f.line_fmt(format_args!("{t} = {insn} i64 {a}, {safe}"));
                return t.to_string();
            }
            let insn = if *op == "%" { "srem" } else { "sdiv" };
            f.line_fmt(format_args!("{t} = {insn} i64 {a}, {b}"));
        }
    }
    t.to_string()
}

/// A word run's flag: kept for the caller, or a branch to `slow` here.
fn flag(f: &mut FnEmit, over: String, flags: &mut Vec<String>, slow: Option<&str>) {
    match slow {
        Some(slow) => {
            let on = f.label();
            f.line_fmt(format_args!("br i1 {over}, label %{slow}, label %{on}"));
            f.start_block(&on);
            flags.push(over);
        }
        None => flags.push(over),
    }
}

fn inline_tag(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 0) {
        return word.to_string();
    }
    if let Some((tag, _)) = f.known_words.get(value) {
        return tag.clone();
    }
    let t = f.tmp();
    f.line_fmt(format_args!("{t} = extractvalue %KValue {value}, 0"));
    t.to_string()
}

fn inline_payload(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 1) {
        return word.to_string();
    }
    if let Some((_, payload)) = f.known_words.get(value) {
        return payload.clone();
    }
    let t = f.tmp();
    f.line_fmt(format_args!("{t} = extractvalue %KValue {value}, 1"));
    t.to_string()
}

/// Branch to `hit` when a 1-based index lands inside the container whose
/// length `len_ptr` points at, and to `miss` when it does not, one signed
/// compare at a time. The length is loaded between the two, which keeps LLVM
/// from folding the pair back into flags and an `and`. The signed spelling
/// is the one a guard such as `return acc if n < 1 or length bs < n` proves,
/// so LLVM can drop both where one ran upstream.
fn index_in_range(f: &mut FnEmit, idx: &str, len_ptr: &str, hit: &str, miss: &str) {
    let ge1 = f.tmp();
    f.line_fmt(format_args!("{ge1} = icmp sge i64 {idx}, 1"));
    let above = f.label();
    f.line_fmt(format_args!("br i1 {ge1}, label %{above}, label %{miss}"));
    f.start_block(&above);
    let len = f.tmp();
    f.line_fmt(format_args!("{len} = load i64, ptr {len_ptr}"));
    assume_length(f, &len);
    let le_len = f.tmp();
    f.line_fmt(format_args!("{le_len} = icmp sle i64 {idx}, {len}"));
    f.line_fmt(format_args!("br i1 {le_len}, label %{hit}, label %{miss}"));
}

/// A length read out of a container is never negative, and saying so lets
/// LLVM merge a `1 <= i <= len` test into one unsigned compare where the
/// signed pair also stays visible to whatever proved it true upstream.
fn assume_length(f: &mut FnEmit, len: &str) {
    let ok = f.tmp();
    f.line_fmt(format_args!("{ok} = icmp sge i64 {len}, 0"));
    f.line_fmt(format_args!("call void @llvm.assume(i1 {ok})"));
}

/// The low byte of a `%parsed` first word that holds no value's tag: the int
/// did not fit above the tag, and the second word is the record on the heap.
/// `K_PARSED_WIDE` in the runtime.
const PARSED_WIDE: i64 = 255;

/// Pack an int and a value into a `%parsed`'s two words: the int above the
/// value's tag in the first, the value's payload in the second. An int that
/// does not survive the shift and back is spilled, so the record is built on
/// the heap and crosses as `PARSED_WIDE` over its pointer.
fn pack_words(f: &mut FnEmit, id: i64, pos: &str, value: &str) -> (String, String) {
    let ptag = f.tmp();
    f.line_fmt(format_args!("{ptag} = extractvalue %KValue {pos}, 0"));
    let n = f.tmp();
    f.line_fmt(format_args!("{n} = extractvalue %KValue {pos}, 1"));
    let shifted = f.tmp();
    f.line_fmt(format_args!("{shifted} = shl i64 {n}, 8"));
    let back = f.tmp();
    f.line_fmt(format_args!("{back} = ashr i64 {shifted}, 8"));
    let same = f.tmp();
    f.line_fmt(format_args!("{same} = icmp eq i64 {back}, {n}"));
    // a bignum is a pointer, which the shift would ruin, so it spills too
    let word = f.tmp();
    f.line_fmt(format_args!("{word} = icmp eq i64 {ptag}, 0"));
    let fits = f.tmp();
    f.line_fmt(format_args!("{fits} = and i1 {same}, {word}"));
    let vtag = f.tmp();
    f.line_fmt(format_args!("{vtag} = extractvalue %KValue {value}, 0"));
    let vpay = f.tmp();
    f.line_fmt(format_args!("{vpay} = extractvalue %KValue {value}, 1"));
    let packed = f.tmp();
    f.line_fmt(format_args!("{packed} = or i64 {shifted}, {vtag}"));
    let from = f.cur_label.clone();
    let (wide, join) = (f.label(), f.label());
    f.line_fmt(format_args!("br i1 {fits}, label %{join}, label %{wide}"));
    f.start_block(&wide);
    let spilled = f.tmp();
    f.line_fmt(format_args!(
        "{spilled} = call %KValue @k_parsed_spill(i64 {id}, i64 {ptag}, i64 {n}, i64 {vtag}, i64 {vpay})"
    ));
    let (s0, s1) = (f.tmp(), f.tmp());
    f.line_fmt(format_args!("{s0} = extractvalue %KValue {spilled}, 0"));
    f.line_fmt(format_args!("{s1} = extractvalue %KValue {spilled}, 1"));
    f.line_fmt(format_args!("br label %{join}"));
    f.start_block(&join);
    let (w0, w1) = (f.tmp(), f.tmp());
    f.line_fmt(format_args!("{w0} = phi i64 [ {packed}, %{from} ], [ {s0}, %{wide} ]"));
    f.line_fmt(format_args!("{w1} = phi i64 [ {vpay}, %{from} ], [ {s1}, %{wide} ]"));
    (w0.to_string(), w1.to_string())
}

/// Read a `%parsed`'s two words back as the int's tag and word, the value's
/// tag and the value's payload. The shift is arithmetic, so a negative int
/// keeps its sign, and a spilled record is read off the heap, where the int
/// may be a bignum.
fn unpack_words(f: &mut FnEmit, w0: &str, w1: &str) -> (String, String, String, String) {
    let low = f.tmp();
    f.line_fmt(format_args!("{low} = and i64 {w0}, 255"));
    let spilled = f.tmp();
    f.line_fmt(format_args!("{spilled} = icmp eq i64 {low}, {PARSED_WIDE}"));
    let n = f.tmp();
    f.line_fmt(format_args!("{n} = ashr i64 {w0}, 8"));
    let from = f.cur_label.clone();
    let (wide, join) = (f.label(), f.label());
    f.line_fmt(format_args!("br i1 {spilled}, label %{wide}, label %{join}"));
    f.start_block(&wide);
    let wfirst = f.tmp();
    f.line_fmt(format_args!("{wfirst} = call %KValue @k_parsed_wide_first(i64 {w1})"));
    let (wnt, wn) = (f.tmp(), f.tmp());
    f.line_fmt(format_args!("{wnt} = extractvalue %KValue {wfirst}, 0"));
    f.line_fmt(format_args!("{wn} = extractvalue %KValue {wfirst}, 1"));
    let wv = f.tmp();
    f.line_fmt(format_args!("{wv} = call %KValue @k_parsed_wide_value(i64 {w1})"));
    let (wt, wp) = (f.tmp(), f.tmp());
    f.line_fmt(format_args!("{wt} = extractvalue %KValue {wv}, 0"));
    f.line_fmt(format_args!("{wp} = extractvalue %KValue {wv}, 1"));
    f.line_fmt(format_args!("br label %{join}"));
    f.start_block(&join);
    let (pnt, pn, pt, pp) = (f.tmp(), f.tmp(), f.tmp(), f.tmp());
    f.line_fmt(format_args!("{pnt} = phi i64 [ 0, %{from} ], [ {wnt}, %{wide} ]"));
    f.line_fmt(format_args!("{pn} = phi i64 [ {n}, %{from} ], [ {wn}, %{wide} ]"));
    f.line_fmt(format_args!("{pt} = phi i64 [ {low}, %{from} ], [ {wt}, %{wide} ]"));
    f.line_fmt(format_args!("{pp} = phi i64 [ {w1}, %{from} ], [ {wp}, %{wide} ]"));
    (pnt.to_string(), pn.to_string(), pt.to_string(), pp.to_string())
}

/// Whether a value is not a failure, as one compare of its tag against the
/// err tag.
fn inline_not_failure(f: &mut FnEmit, value: &str) -> String {
    let set = f.set_of(value);
    // An EMPTY set is not a proof. `group_param_set` answers 0 for a parameter
    // the inference reached no shapes for, and 0 satisfies every `& mask == 0`
    // test written about it -- so the first draft of this fold read "nothing is
    // known" as "proved to be a boolean" and turned the hop below into `true`.
    // A group whose argument is a failure then dispatched instead of hopping
    // and died on `no overload matches these arguments`. The micro corpus
    // caught it: a_construction_merges_its_failures lost its third line.
    if set != 0 && set & !infer::BOOL == 0 {
        return "true".to_string();
    }
    not_failure_test(f, value)
}

/// The same test with no fold, for the one place a set cannot speak: a block
/// reached BECAUSE a value is outside the set recorded for it.
///
/// It was a call to an alwaysinline `k_not_failure`, then a compare of what
/// that returned. Every module paid for the inliner to open the call, and the
/// dev tier, which does no other optimising, kept the widening and the second
/// compare as well. The test is the one the runtime's `k_not_failure` makes,
/// and `tests/the_err_tag_is_the_runtime_s.rs` holds the number to the
/// runtime's.
fn not_failure_test(f: &mut FnEmit, value: &str) -> String {
    let tag = inline_tag(f, value);
    if let Ok(known) = tag.parse::<i64>() {
        return (known != K_ERR_TAG).to_string();
    }
    let ok = f.tmp();
    f.line_fmt(format_args!("{ok} = icmp ne i64 {tag}, {K_ERR_TAG}"));
    ok.to_string()
}

/// The runtime's `K_ERR`, the sixth tag in its enum.
pub const K_ERR_TAG: i64 = 5;

impl<'a> Backend<'a> {
    /// A group's declaration indices, read out of the index rather than
    /// scanned for. Program order, which is what the scan gave.
    fn group_indices<'s>(&'s self, name: &str, arity: usize) -> impl Iterator<Item = usize> + 's {
        self.asked(name);
        group_indices_in(&self.group_by_name, self.program, name, arity)
    }

    /// Every declaration of a group, at every arity, in program order. The
    /// emitter asks about a group by name only through this, `group_named`
    /// and `group_indices`, so a question about a group the tab's walk left
    /// out is seen; `a_lookup_by_name_goes_through_the_index` holds it to that.
    fn decls_named(&self, name: &str) -> impl Iterator<Item = &'a FnDecl> + '_ {
        let program = self.program;
        self.group_named(name).iter().map(move |&at| &program.fns[at])
    }

    fn group_named(&self, name: &str) -> &[usize] {
        self.asked(name);
        self.group_by_name.get(name).map_or(&[][..], |at| at.as_slice())
    }

    fn declares(&self, name: &str) -> bool {
        !self.group_named(name).is_empty()
    }

    fn declares_at(&self, name: &str, arity: usize) -> bool {
        self.group_indices(name, arity).next().is_some()
    }

    /// Notes a question about a group the walk left out. The answer is wrong
    /// for the program as written, so the module goes back to be emitted again.
    fn asked(&self, name: &str) {
        if self.left_out.contains(name) {
            self.asked_left_out.set(true);
        }
    }

    fn group_param_set(&self, name: &str, arity: usize, param: usize) -> Set {
        self.group_indices(name, arity).fold(0, |acc, i| acc | self.inference.param(i, param))
    }

    fn group_return_set(&self, name: &str, arity: usize) -> Set {
        self.group_indices(name, arity).fold(0, |acc, i| acc | self.inference.returns[i])
    }

    /// A parameter proven to be exactly `int` crosses the tailcc boundary as a
    /// raw i64 instead of a boxed KValue. The dispatcher re-boxes it at entry so
    /// the body is untouched; LLVM's SROA folds that rebox against the body's
    /// payload reads (same function), and folds each caller's box against the
    /// extract we emit here — so only a raw i64 travels the musttail edge LLVM
    /// cannot otherwise see through. Sound because inference forces every param
    /// of a function used as a first-class value to TOP, never a bare `int`.
    ///
    /// Not an operator's arms: `a + b` reaches them through the operator's own
    /// dispatch, which hands them both values boxed, as escape analysis keeps
    /// them for the same reason.
    fn unboxed_param(&self, name: &str, arity: usize, param: usize) -> bool {
        self.group_param_set(name, arity, param) == INT && !crate::is_operator(name)
    }

    /// What the body being emitted knows of a parameter: the group's set, or
    /// `int` for a twin's word position.
    fn own_param_set(&self, name: &str, arity: usize, param: usize) -> Set {
        match self.emitting_twin(name, arity) && self.twin_param(name, arity, param) {
            true => INT,
            false => self.group_param_set(name, arity, param),
        }
    }

    /// Whether the body being emitted receives this slot as a raw word: the
    /// general ABI's answer, or a twin's own word positions.
    fn own_unboxed(&self, name: &str, arity: usize, param: usize) -> bool {
        self.unboxed_param(name, arity, param)
            || (self.emitting_twin(name, arity) && self.twin_param(name, arity, param))
    }

    /// The positions a group's word twin takes as raw words: those inference
    /// says hold an int of either kind and nothing else. Since the 2026-10-07
    /// gavel `n - 1` can widen, so a loop's counter is `int` or bignum, and
    /// boxed it pays a tag test at every use and keeps the byte-run and
    /// payload paths shut. Nearly every call carries words, and the twin is
    /// the group compiled for that case; a call site tests the tags and takes
    /// the general group only for a bignum. A slot with its own convention
    /// keeps it.
    fn twin_positions(&self, name: &str, arity: usize) -> Vec<usize> {
        (0..arity).filter(|&i| self.twin_param(name, arity, i)).collect()
    }

    fn twin_param(&self, name: &str, arity: usize, param: usize) -> bool {
        self.group_param_set(name, arity, param) == infer::ANY_INT
            && !crate::is_operator(name)
            && !self.is_byte_disc(name, arity, param)
            && self.escape.carries_ty(name, arity, param).is_none()
    }

    /// A group calls for a twin when it is a loop -- a member of a cycle --
    /// with a parameter the twin can take as a word. A group run once a call
    /// would pay the code twice for one test saved.
    fn has_twin(&self, name: &str, arity: usize) -> bool {
        // A twin buys run speed with a second copy of the group, which a dev
        // build and the tab pay for in compile time and memory and have no
        // use for: on 2026-10-06 the tab's compile of interp_corpus read
        // 24.7% more fuel and held 49% more memory with them.
        self.inline_helpers
            && arity > 0
            && !self.knotted.contains(name)
            && self.cycle_members.contains(name)
            && (0..arity).any(|i| self.twin_param(name, arity, i))
    }

    /// A frame with releasable cells settles them before a musttail: a cell
    /// riding out in the arguments escapes (counted); every other cell dies
    /// here.
    fn settle_cells(&self, f: &mut FnEmit, args_ir: &[String]) {
        let cells = f.lazy_cells.clone();
        for cell in cells {
            if args_ir.iter().any(|a| a.ends_with(cell.as_str())) {
                f.line_fmt(format_args!("call void @k_thunk_note_escape(%KValue {cell})"));
            } else {
                let d = f.tmp();
                f.line_fmt(format_args!(
                    "{d} = call %KValue @k_thunk_release_unless(%KValue {cell}, %KValue {{ i64 0, i64 0 }})"
                ));
            }
        }
    }

    fn emitting_twin(&self, name: &str, arity: usize) -> bool {
        self.twin_of.as_ref().is_some_and(|(n, a)| n == name && *a == arity)
    }

    /// Which of a group's two bodies a call reaches. `None` is the general
    /// group: there is no twin, or an argument in a word position cannot be
    /// a word. `Some(None)` is the twin, every such argument already known to
    /// be a word. `Some(Some(c))` tests at run time, `c` true when every one
    /// is a word.
    fn twin_reachable(&self, f: &FnEmit, name: &str, n: usize) -> bool {
        // A general body stays general. It runs only once a bignum has
        // arrived, and a call from it into a twin would give every twin a
        // second caller, which costs the single-caller inlining that folds a
        // loop's steps into one function.
        self.has_twin(name, n)
            && !(self.has_twin(&f.group, f.arity) && !self.emitting_twin(&f.group, f.arity))
    }

    /// An argument a twin takes as a word, written as arithmetic over words,
    /// such as the `n - 1` of a counting loop: the operations run on raw
    /// words and their overflow flags go back to the caller, which folds
    /// them into the test that picks the twin. Merging the overflow call
    /// back first left a phi whose tag the test then read, and LLVM does not
    /// thread a branch into a loop's header, so a counting loop paid three
    /// instructions a step to learn what the arithmetic had already said.
    /// The second value is the operands and the flags; the general way
    /// re-runs the operations on those operands.
    fn emit_word_arg(&mut self, f: &mut FnEmit, arg: &Expr) -> Result<(String, WordFlags), String> {
        let mut leaves = Vec::new();
        arith_leaves(arg, &mut leaves);
        let mut vals = Vec::with_capacity(leaves.len());
        for leaf in leaves {
            let v = self.emit_expr(f, leaf)?;
            let v = self.maybe_force(f, v);
            vals.push(self.as_value(f, &v));
        }
        if vals.iter().any(|v| f.set_of(v) != INT) {
            return Ok((self.emit_arith_general(f, arg, &vals, &mut 0)?, None));
        }
        let words: Vec<String> = vals.iter().map(|v| inline_payload(f, v)).collect();
        let mut flags = Vec::new();
        // In a twin that has written nothing yet, an overflow here starts
        // the general body over instead of going back to the caller, and the
        // call this argument feeds can take the twin without asking. Each
        // operation branches there as it overflows.
        let bail = f.can_bail().then(|| f.label());
        let r = word_run(f, arg, &words, &mut 0, &mut flags, bail.as_deref());
        let v = f.tmp();
        f.line_fmt(format_args!("{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"));
        f.record(&v, INT);
        if let Some(out) = bail {
            if !flags.is_empty() {
                let ok = f.label();
                f.line_fmt(format_args!("br label %{ok}"));
                f.bail_block(&out);
                f.start_block(&ok);
            }
            return Ok((v.to_string(), None));
        }
        match flags.is_empty() {
            true => Ok((v.to_string(), None)),
            false => Ok((v.to_string(), Some((vals, flags)))),
        }
    }

    fn twin_test(
        &self,
        f: &mut FnEmit,
        name: &str,
        n: usize,
        emitted: &[String],
        packed: &[Option<String>],
    ) -> Option<Option<String>> {
        if !self.twin_reachable(f, name, n) {
            return None;
        }
        let positions = self.twin_positions(name, n);
        let mut unknown = Vec::new();
        for &j in &positions {
            if packed.get(j).is_some_and(|p| p.is_some()) {
                return None;
            }
            // the test reads the tag, so it is exact whatever the set says;
            // only a set that rules a word out sends the call straight on
            let s = f.set_of(&emitted[j]);
            if s == INT {
                continue;
            }
            if s != 0 && s & INT == 0 {
                return None;
            }
            unknown.push(j);
        }
        let mut all: Option<String> = None;
        for j in unknown {
            let t = inline_tag(f, &emitted[j]);
            let w = f.tmp();
            f.line_fmt(format_args!("{w} = icmp eq i64 {t}, 0"));
            all = Some(match all {
                None => w.to_string(),
                Some(prev) => {
                    let a = f.tmp();
                    f.line_fmt(format_args!("{a} = and i1 {prev}, {w}"));
                    a.to_string()
                }
            });
        }
        Some(all)
    }

    /// Render one call argument in the callee's ABI: raw i64 for an unboxed
    /// slot (extract the payload), boxed KValue otherwise.
    /// Any arity-matching arm inspecting this position (anything but a bare
    /// Var/Wildcard) means a thunk must force before dispatch can select.
    fn scrutinizes(&self, callee: &str, arity: usize, i: usize) -> bool {
        self.decls_named(callee).any(|d| {
            d.params.len() == arity
                && !matches!(d.params.get(i), Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_)))
        }) || self.unread_positions.iter().any(|(g, n, p)| g == callee && *n == arity && *p == i)
    }

    /// An operand as an ordinary value. Only a carried argument slot reads the
    /// two-word convention; every other consumer — a render, a list, an
    /// ordinary parameter — needs the record itself. Converting here rather
    /// than at the call is what keeps the hot path free: a chain of carried
    /// slots never builds a record at all, which is the whole point of the
    /// convention and worth 254 MB on a json decode.
    fn as_value(&self, f: &mut FnEmit, e: &str) -> String {
        match f.is_parsed(e) {
            true => f.box_parsed(e),
            false => e.to_string(),
        }
    }

    fn call_arg(
        &self,
        f: &mut FnEmit,
        callee: &str,
        arity: usize,
        i: usize,
        e: &str,
        arg: Option<&Expr>,
    ) -> String {
        self.call_arg_to(f, callee, arity, i, e, arg, false)
    }

    /// `call_arg` for either of a group's bodies: `twin` passes the twin's
    /// word positions as raw words. The caller has tested that they hold
    /// words.
    #[allow(clippy::too_many_arguments)]
    fn call_arg_to(
        &self,
        f: &mut FnEmit,
        callee: &str,
        arity: usize,
        i: usize,
        e: &str,
        arg: Option<&Expr>,
        twin: bool,
    ) -> String {
        if twin && self.twin_param(callee, arity, i) {
            let p = inline_payload(f, e);
            return format!("i64 {p}");
        }
        // A string this group builds by joining onto itself needs its seed
        // converted where it enters from outside: a builder writes into the
        // header it was given, and an interned literal cannot be written
        // through. The recursive call is not converted — it is already
        // carrying the builder made here.
        // A parameter forwarded round the same cycle is carrying the builder
        // already, and seeding it again copies the whole string once per hop.
        // A beat loop rewinds the arena between iterations and the shelf carries
        // the accumulator's header across the rewind, so the seed has to happen
        // inside the bracket: converted outside it, the header sits below the
        // mark and the join finds a string that is not a builder.
        let carried = match arg {
            Some(Expr::Ident(_, span, _)) => self.builder_carried.contains(&(
                f.file.clone(),
                span.line as usize,
                span.col as usize,
            )),
            _ => false,
        };
        let entering = self.builder_params.contains(callee, arity, i)
            && !(f.group == callee && f.arity == arity)
            && !carried;
        let seeded;
        let e = match entering {
            true => {
                // A subtype of string builds from the string it wraps, and a
                // seed that is not a string is handed on as it is; the runtime
                // tells them apart, and the first join adopts the second.
                let t = f.tmp();
                f.line_fmt(format_args!("{t} = call %KValue @k_b_str_builder(%KValue {e})"));
                f.record(&t, f.set_of(e));
                seeded = t;
                &*seeded
            }
            false => e,
        };
        let forced;
        let e = match f.set_of(e) & crate::infer::THUNK != 0 && self.scrutinizes(callee, arity, i) {
            true => {
                forced = self.maybe_force(f, e.to_string());
                forced.as_str()
            }
            false => e,
        };
        if self.is_byte_disc(callee, arity, i) {
            // `e` is an `at`-on-bytes KValue (byte or none); hand it over as a
            // raw i64 — the byte value, or 256 for none. The box `at` built and
            // this unbox fold away in the caller, so a raw byte crosses the edge.
            if let Some(raw) = f.raw_byte.get(e) {
                // The index already merged the two arms as an i64; take that
                // and the box goes unread.
                return format!("i64 {raw}");
            }
            let tag = f.tmp();
            f.line_fmt(format_args!("{tag} = extractvalue %KValue {e}, 0"));
            let payload = f.tmp();
            f.line_fmt(format_args!("{payload} = extractvalue %KValue {e}, 1"));
            let is_none = f.tmp();
            f.line_fmt(format_args!("{is_none} = icmp eq i64 {tag}, {K_NONE}"));
            let raw = f.tmp();
            f.line_fmt(format_args!("{raw} = select i1 {is_none}, i64 256, i64 {payload}"));
            return format!("i64 {raw}");
        }
        // A register-returned record reaching a slot that wants an ordinary
        // value has to be built back into one. The two words carry the whole
        // record — `(field0.payload << 8 | field1.tag, field1.payload)` — so
        // nothing is lost, but only a real record can be dispatched on, and a
        // getter is exactly the caller that dispatches.
        let boxed;
        let e = match f.is_parsed(e) && self.escape.carries_ty(callee, arity, i).is_none() {
            true => {
                boxed = f.box_parsed(e);
                boxed.as_str()
            }
            false => e,
        };
        if self.escape.carries_ty(callee, arity, i).is_some() {
            if f.is_parsed(e) {
                return format!("%parsed {e}");
            }
            // a boxed record reached a by-value slot (a construction bound or
            // passed outside tail position, or a carry take): unpack it into
            // the convention. A failure in the slot is its own two words, and
            // the runtime hands them over unread rather than reading fields
            // off a value that has none.
            let u = f.tmp();
            f.line_fmt(format_args!("{u} = call %KValue @k_parsed_words(%KValue {e})"));
            let w0 = f.tmp();
            f.line_fmt(format_args!("{w0} = extractvalue %KValue {u}, 0"));
            let w1 = f.tmp();
            f.line_fmt(format_args!("{w1} = extractvalue %KValue {u}, 1"));
            let a = f.tmp();
            f.line_fmt(format_args!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
            let p = f.tmp();
            f.line_fmt(format_args!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
            format!("%parsed {p}")
        } else if self.unboxed_param(callee, arity, i) {
            let p = inline_payload(f, e);
            format!("i64 {p}")
        } else {
            format!("%KValue {e}")
        }
    }

    /// Emit the entry-block reboxes that reconstruct each unboxed `%xi` param as
    /// the KValue the body expects.
    /// What inference proved about each slot, said out loud at the entry.
    ///
    /// The ladder dispatcher has always had this: a bare `Var` parameter
    /// records its group's set as it binds, so a body indexing that slot
    /// skips the tag test whose answer the front end already holds. The
    /// switch dispatcher binds the name straight to `%x{i}` — the switch has
    /// decided what the value is, so there is no pattern left to emit — and
    /// recorded nothing, so every group it took over went back to paying the
    /// test. Three of the decoder's four hot groups are switch-shaped, and
    /// jsonbench carried 16,017,450 instructions of tag test on that account.
    ///
    /// The set keeps its failure bits. A ladder's `Var` arm can drop them
    /// because the pattern it just emitted checked them off; a switch's
    /// default arm is reached BY a failing discriminator, so here they stay.
    ///
    /// Called from both dispatchers. On the ladder it moves no emitted line
    /// in today's corpus — every parameter a body indexes there is a bare
    /// `Var`, which already recorded — and it is called anyway so the two
    /// dispatchers tell a body the same things.
    fn record_param_sets(&self, f: &mut FnEmit, name: &str, arity: usize) {
        for i in 0..arity {
            let set = self.own_param_set(name, arity, i);
            f.record(&format!("%x{i}"), set);
            self.assume_param_tag(f, name, arity, i, set);
        }
    }

    /// A boxed parameter inference proves is one kind of heap value says so
    /// to LLVM, which then folds every test of that parameter's tag in the
    /// body. `record` tells this emitter the same thing, but the helpers it
    /// inlines -- `k_b_push_mut_fast` asking whether its list is a list --
    /// test the tag word themselves, and only LLVM sees into them.
    ///
    /// Not in a program that declares a subtype: a subtype's value carries tag
    /// 15 whatever it holds, which is why `tag_switch_shape` refuses the same
    /// programs.
    fn assume_param_tag(&self, f: &mut FnEmit, name: &str, arity: usize, i: usize, set: Set) {
        if !self.sub_parents.is_empty()
            || self.is_byte_disc(name, arity, i)
            || self.escape.carries_ty(name, arity, i).is_some()
            || self.own_unboxed(name, arity, i)
        {
            return;
        }
        self.assume_tag(f, &format!("%x{i}"), set);
    }

    /// `llvm.assume` that a boxed value's tag is the one kind its set allows,
    /// for the kinds whose set bit names exactly one runtime tag. The caller
    /// has already refused programs that declare a subtype.
    fn assume_tag(&self, f: &mut FnEmit, value: &str, set: Set) {
        let tag = match set {
            infer::FLOAT => 1,
            infer::STR => 6,
            infer::REC => 7,
            infer::LIST => 9,
            infer::MAP => 10,
            infer::BYTES => 13,
            _ => return,
        };
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = extractvalue %KValue {value}, 0"));
        let is = f.tmp();
        f.line_fmt(format_args!("{is} = icmp eq i64 {t}, {tag}"));
        f.line_fmt(format_args!("call void @llvm.assume(i1 {is})"));
    }

    fn rebox_params(&self, f: &mut FnEmit, name: &str, arity: usize) {
        if self.emitting_twin(name, arity) {
            // The general body's own convention, which differs from the
            // twin's only at the positions the twin takes as words; those are
            // reboxed below, so `%x{i}` names a boxed value either way.
            let args: Vec<String> = (0..arity)
                .map(|i| {
                    if self.is_byte_disc(name, arity, i) || self.unboxed_param(name, arity, i) {
                        format!("i64 %x{i}r")
                    } else if self.escape.carries_ty(name, arity, i).is_some() {
                        format!("%parsed %x{i}")
                    } else {
                        format!("%KValue %x{i}")
                    }
                })
                .collect();
            let ret = self.ret_ty(name, arity).to_string();
            let call = format!("tailcc {ret} @{}({})", dsym(name, arity), args.join(", "));
            f.general = Some((ret, call));
        }
        for i in 0..arity {
            if self.is_byte_disc(name, arity, i) {
                // Reconstruct the KValue the boxed dispatch expects: 256 is none,
                // anything else is that byte. The reconstruction folds back into
                // a raw switch, so only the raw i64 actually crossed the edge.
                let is_none = f.tmp();
                f.line_fmt(format_args!("{is_none} = icmp eq i64 %x{i}r, 256"));
                f.line_fmt(format_args!(
                    "%x{i}b = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
                f.line_fmt(format_args!(
                    "%x{i} = select i1 {is_none}, %KValue {{ i64 4, i64 0 }}, %KValue %x{i}b"
                ));
            } else if self.own_unboxed(name, arity, i) {
                f.line_fmt(format_args!(
                    "%x{i} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
                // the unboxing condition is the proof: this slot is an int, and
                // saying so lets arithmetic on it skip the tag test and the
                // boxed fallback it guards
                f.record(&format!("%x{i}"), INT);
                f.known_words.insert(format!("%x{i}"), ("0".to_string(), format!("%x{i}r")));
            }
        }
    }

    /// A switch discriminator inference proves is `at`-on-bytes, so it crosses
    /// as a raw i64 (byte value, or 256 for none) and is switched on directly.
    fn is_byte_disc(&self, name: &str, arity: usize, param: usize) -> bool {
        self.byte_disc.contains(name, arity, param)
    }

    /// The dispatcher's parameter list: a raw i64 for a byte discriminator or a
    /// proven-int slot, a `%parsed` struct for a register-returnable record,
    /// else a boxed KValue.
    fn abi_params(&self, name: &str, arity: usize) -> Vec<String> {
        (0..arity)
            .map(|i| {
                if self.is_byte_disc(name, arity, i) {
                    format!("i64 %x{i}r")
                } else if self.escape.carries_ty(name, arity, i).is_some() {
                    format!("%parsed %x{i}")
                } else if self.own_unboxed(name, arity, i) {
                    format!("i64 %x{i}r")
                } else {
                    format!("%KValue %x{i}")
                }
            })
            .collect()
    }

    /// The LLVM return type of a function group: `%parsed` when it hands back a
    /// register-returnable record by value, else `%KValue`.
    fn ret_ty(&self, name: &str, arity: usize) -> &'static str {
        // A knotted constant's cell holds a blackhole while the cycle is still
        // building, and a register-returned record has nowhere to put a thunk:
        // the caller would read the thunk's payload as a record pointer.
        if arity == 0 && self.knotted.contains(name) {
            return "%KValue";
        }
        if self.escape.returns_ty(name, arity).is_some() {
            "%parsed"
        } else {
            "%KValue"
        }
    }

    /// A group we can hand out as a first-class value through a `w_` wrapper: a
    /// `%KValue` return and no by-value or byte-discriminated parameters, which
    /// the generic wrapper does not know how to convert.
    fn simple_fn_value(&self, name: &str, arity: usize) -> bool {
        self.ret_ty(name, arity) == "%KValue"
            && (0..arity).all(|i| {
                !self.is_byte_disc(name, arity, i)
                    && self.escape.carries_ty(name, arity, i).is_none()
            })
    }

    /// A bind is representable as a native thunk when its captures fit the
    /// cell (args[8]).
    fn thunkable(&self, f: &FnEmit, expr: &Expr) -> bool {
        let mut idents = Vec::new();
        collect_idents(expr, &mut idents);
        let mut captures: Vec<&String> = Vec::new();
        for id in &idents {
            if f.lookup(id).is_some() && !captures.contains(&id) {
                captures.push(id);
            }
        }
        captures.len() <= 8
    }

    /// Compile an expression into a cell: a site function over the names it
    /// reads, plus a `k_thunk_new` that captures their current values. The
    /// computation runs at first force.
    fn emit_cell(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        let mut idents = Vec::new();
        collect_idents(expr, &mut idents);
        let mut captures: Vec<String> = Vec::new();
        for id in idents {
            if f.lookup(&id).is_some() && !captures.contains(&id) {
                captures.push(id);
            }
        }
        let site = self.thunk_sites.len();
        let sym = format!("tsite{site}");
        self.thunk_sites.push((sym.clone(), captures.len()));
        self.emit_thunk_site(&sym, &captures, expr, f)?;
        let mut args = String::new();
        for cap in &captures {
            let temp = f.lookup(cap).expect("capture is bound");
            // A capture is stored as a %KValue, and a group that returns its
            // record in registers holds a %parsed.
            let temp = self.as_value(f, &temp);
            args.push_str(&format!(", %KValue {temp}"));
        }
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue (i64, i32, ...) @k_thunk_new(i64 {site}, i32 {}{args})",
            captures.len()
        ));
        f.record(&t, crate::infer::TOP);
        Ok(t.to_string())
    }

    /// Force a value that may be a thunk; no-op (no IR) when the set proves
    /// it can't be one, so strict code pays nothing. A program the demand
    /// analysis deferred nothing in can hold no thunk anywhere — every site
    /// vanishes, not just the set-proven ones (conservative TOP widenings
    /// carry the THUNK bit into code no thunk can reach).
    /// Force unless the value is a cell still being computed. Only a
    /// constructor's fields take this path; everywhere else a blackhole
    /// reached is the error it exists to report.
    fn force_unless_knot(&self, f: &mut FnEmit, value: String) -> String {
        if f.set_of(&value) & crate::infer::THUNK == 0 {
            return value;
        }
        let post = f.set_of(&value) & !crate::infer::THUNK;
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = call %KValue @k_force_unless_black(%KValue {value})"));
        f.record(&t, if post == 0 { crate::infer::TOP } else { post | crate::infer::THUNK });
        t.to_string()
    }

    /// Whether a rendered value goes through the ambient `render/to_string`
    /// group: a set carrying REC may hit a user arm, so it is routed there.
    /// Primitive-only sets keep the direct call — coherence proves no arm can
    /// exist for them (design/render-plan.md).
    fn render_dispatchable(&self, f: &FnEmit, value: &str) -> bool {
        f.set_of(value) & (REC | NONE | infer::DONE | DESC) != 0 && self.declares(RENDER_GROUP)
    }

    /// The string an interpolated `value` renders to, and the fail set it
    /// adds: only an err propagates out of an interpolation (a none renders
    /// `<none>`, so it is not a fail), and a dispatched arm may fail on its
    /// own.
    fn render_interp(&self, f: &mut FnEmit, value: &str) -> (String, Set) {
        let mut fails = f.set_of(value) & ERR;
        let dispatchable = self.render_dispatchable(f, value);
        let value = self.as_value(f, value);
        let value = match dispatchable {
            true => self.maybe_force(f, value),
            false => value,
        };
        match dispatchable {
            true => {
                let rendered = self.render_through_group(f, &value, fails != 0);
                fails |= ERR;
                (rendered, fails)
            }
            false => {
                let t = f.tmp();
                f.line_fmt(format_args!("{t} = call %KValue @k_render(%KValue {value}, i64 0)"));
                (t.to_string(), fails)
            }
        }
    }

    /// `value` rendered by the ambient `render/to_string` group. An err is
    /// handed on as it is, without the call: the interpreter returns it from
    /// an interpolation or a `print` before anything renders, and handed to
    /// the group it came back carrying `render/to_string` as a hop the
    /// program never wrote.
    fn render_through_group(&self, f: &mut FnEmit, value: &str, may_be_err: bool) -> String {
        let call = |f: &mut FnEmit| {
            let t = f.tmp();
            f.line_fmt(format_args!(
                "{t} = call tailcc %KValue @{}(%KValue {value})",
                dsym(RENDER_GROUP, 1)
            ));
            t
        };
        if !may_be_err {
            return call(f).to_string();
        }
        let tag = f.tmp();
        f.line_fmt(format_args!("{tag} = extractvalue %KValue {value}, 0"));
        let failing = f.tmp();
        f.line_fmt(format_args!("{failing} = icmp eq i64 {tag}, 5"));
        let render = f.label();
        let merge = f.label();
        let from = f.cur_label.clone();
        f.line_fmt(format_args!("br i1 {failing}, label %{merge}, label %{render}"));
        f.start_block(&render);
        let rendered = call(f);
        let rendered_from = f.cur_label.clone();
        f.line_fmt(format_args!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = phi %KValue [ {value}, %{from} ], [ {rendered}, %{rendered_from} ]"
        ));
        t.to_string()
    }

    /// A builtin sees a subtype's value as its parent's, and the generic call
    /// path unwraps every argument before it calls one. The fused doors and the
    /// frame-held view emit their builtin themselves, ahead of that path, so
    /// each argument they read goes through here. Only a program that declares
    /// a subtype can hand one over, so only that program pays.
    fn unsub(&self, f: &mut FnEmit, value: String) -> String {
        if self.sub_parents.is_empty() {
            return value;
        }
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = call %KValue @k_unsub(%KValue {value})"));
        f.record(&t, f.set_of(&value));
        t.to_string()
    }

    fn maybe_force(&self, f: &mut FnEmit, value: String) -> String {
        // A deferred self-reference is a thunk that no lazy-bind count knows
        // about, so the cheap exit has to admit it too.
        if self.demand.lazy_bind_count() == 0 && !self.defers_self_reference {
            return value;
        }
        if f.set_of(&value) & crate::infer::THUNK == 0 {
            return value;
        }
        let post = f.set_of(&value) & !crate::infer::THUNK;
        let t = f.tmp();
        f.line_fmt(format_args!("{t} = call %KValue @k_force_fast(%KValue {value})"));
        // A forced thunk can yield anything its expr could; the bind site
        // recorded TOP, so widen conservatively past the removed bit.
        f.record(&t, if post == 0 { crate::infer::TOP & !crate::infer::THUNK } else { post });
        t.to_string()
    }

    /// A storing position inside a constant that names itself holds a thunk
    /// rather than a value. The constant's cell is empty while its own body
    /// runs, so reading it there would read nothing; deferred, the read
    /// happens after `k_caf_init` has filled the cell, and the cycle closes.
    ///
    /// No captures: the only free name in such an expression is the global
    /// itself, which the site loads for itself.
    fn deferred_or_emitted(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        if !self.defers_self(f, expr) {
            return self.emit_expr(f, expr);
        }
        let site = self.thunk_sites.len();
        let sym = format!("tsite{site}");
        self.thunk_sites.push((sym.clone(), 0));
        self.emit_thunk_site(&sym, &[], expr, f)?;
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue (i64, i32, ...) @k_thunk_new(i64 {site}, i32 0)"
        ));
        f.record(&t, crate::infer::TOP);
        Ok(t.to_string())
    }

    /// Whether this expression, in this frame, mentions the constant being
    /// built. `arity == 0` keeps it to constants: a function that mentions
    /// itself is ordinary recursion and has a base case.
    fn defers_self(&self, f: &FnEmit, expr: &Expr) -> bool {
        fn mentions(expr: &Expr, name: &str) -> bool {
            if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
                if n == name {
                    return true;
                }
            }
            crate::any_child(expr, |c| mentions(c, name))
        }
        f.arity == 0 && !f.group.is_empty() && mentions(expr, &f.group)
    }

    fn emit_thunk_site(
        &mut self,
        sym: &str,
        captures: &[String],
        expr: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new(!self.inline_helpers);
        f.origin_prefix = outer.origin_prefix.clone();
        f.hako = outer.hako.clone();
        f.hako = outer.hako.clone();
        f.file = outer.file.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            f.bind(cap, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, expr)?;
        let sig: Vec<String> = (0..captures.len()).map(|i| format!("%KValue %a{i}")).collect();
        let text = f.body();
        self.room(sym.len() + 16 * sig.len() + text.len() + 64);
        let _ =
            writeln!(self.body, "define tailcc %KValue @{sym}({}) {{\n{text}}}\n", sig.join(", "),);
        Ok(())
    }

    fn emit_thunk_dispatcher(&mut self) {
        let mut arms = String::new();
        let mut cases = String::new();
        for (site, (sym, argc)) in self.thunk_sites.iter().enumerate() {
            let _ = writeln!(cases, "    i64 {site}, label %s{site}");
            let mut loads = String::new();
            let mut args: Vec<String> = Vec::new();
            for i in 0..*argc {
                let _ =
                    writeln!(loads, "  %s{site}a{i}p = getelementptr %KValue, ptr %args, i64 {i}");
                let _ = writeln!(loads, "  %s{site}a{i} = load %KValue, ptr %s{site}a{i}p");
                args.push(format!("%KValue %s{site}a{i}"));
            }
            let _ = writeln!(
                arms,
                "s{site}:\n{loads}  %s{site}r = call tailcc %KValue @{sym}({})\n  ret %KValue %s{site}r",
                args.join(", ")
            );
        }
        self.room(cases.len() + arms.len() + 256);
        let _ = writeln!(
            self.body,
            "define %KValue @d_thunk_eval(i64 %site, ptr %args) {{\nentry:\n  switch i64 %site, label %bad [\n{cases}  ]\n{arms}bad:\n  unreachable\n}}\n"
        );
    }

    fn emit(&mut self) -> Result<Written, String> {
        self.emit_type_names();
        self.emit_type_fields();
        // group by name across the whole program: the bare overload space
        // interleaves same-named decls from different modules
        let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            match groups.iter_mut().find(|(name, _)| *name == decl.name) {
                Some((_, decls)) => decls.push(decl),
                None => groups.push((&decl.name, vec![decl])),
            }
        }
        // proximity breaks specificity ties: local arms precede clones —
        // and a subtype annotation outranks its ancestors, so arms sort
        // by total chain depth, deepest first (the interp's scores, as an
        // ordering; tie-rejection outlaws the incomparable cases)
        let depth_of = |ty: &str| -> i64 {
            let mut d = 0i64;
            let mut cur = ty;
            while let Some(p) = self.sub_parents.get(cur) {
                d += 1;
                cur = p.as_str();
            }
            d
        };
        // the ladder as a sort: literals, then concrete annotations (nearer
        // subtype declarations first), then typesets, then the generics
        for (_, decls) in &mut groups {
            decls.sort_by_key(|d| {
                let total: i64 = d
                    .params
                    .iter()
                    .map(|p| match p {
                        Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 3000,
                        Pattern::Annotated { ty, .. } => {
                            match self.typesets.contains_key(ty.as_str()) {
                                true => 1000,
                                false => 2000 + depth_of(ty),
                            }
                        }
                        // an err arm ranks as its reason pattern does: a
                        // named leaf with the concretes, a typeset with the
                        // typesets, a bare binder just above the generics
                        Pattern::Ctor { ty, fields, .. } if ty == "err" && fields.len() == 1 => {
                            match &fields[0] {
                                Pattern::Annotated { ty: rty, .. } => {
                                    match self.typesets.contains_key(rty.as_str()) {
                                        true => 1000,
                                        false => 2000 + depth_of(rty),
                                    }
                                }
                                Pattern::Var(..) | Pattern::Wildcard(..) => 1,
                                _ => 2000,
                            }
                        }
                        // a constructor pattern ranks by the same chain the
                        // annotations use: naming the subtype is nearer than
                        // naming what it wraps
                        Pattern::Ctor { ty, .. } => 2000 + depth_of(ty),
                        Pattern::Keyed { .. } => 2000,
                        Pattern::Var(..) | Pattern::Wildcard(..) => 0,
                    })
                    .sum();
                (std::cmp::Reverse(total), d.synthetic)
            });
        }
        for (name, decls) in &groups {
            let mut by_arity: HashMap<usize, Vec<&FnDecl>> = HashMap::default();
            for d in decls {
                by_arity.entry(d.params.len()).or_default().push(d);
            }
            let mut arity_keys: Vec<usize> = by_arity.keys().copied().collect();
            arity_keys.sort_unstable();
            for arity in arity_keys {
                self.emit_dispatcher(name, arity, &by_arity[&arity])?;
                if self.has_twin(name, arity) {
                    self.twin_of = Some((name.to_string(), arity));
                    let emitted =
                        self.emit_dispatcher_as(&tsym(name, arity), name, arity, &by_arity[&arity]);
                    self.twin_of = None;
                    emitted?;
                }
            }
        }
        self.fn_value_wrappers.sort();
        self.fn_value_wrappers.dedup();
        let wrappers = self.fn_value_wrappers.clone();
        for (name, arity) in &wrappers {
            let arity = *arity;
            // Past eight a C caller cannot name the signature for every width,
            // so the wrapper takes the arguments as one array and spreads them;
            // `k_call_ref_wide` calls it that way.
            let spread = arity > 8;
            let params: Vec<String> = match spread {
                true => vec!["ptr %args".to_string()],
                false => (0..arity).map(|i| format!("%KValue %a{i}")).collect(),
            };
            let mut conv = String::new();
            if spread {
                for i in 0..arity {
                    let _ = writeln!(conv, "  %at{i} = getelementptr %KValue, ptr %args, i64 {i}");
                    let _ = writeln!(conv, "  %a{i} = load %KValue, ptr %at{i}");
                }
            }
            let call_args: Vec<String> = (0..arity)
                .map(|i| {
                    if self.unboxed_param(name, arity, i) {
                        let _ = writeln!(conv, "  %p{i} = extractvalue %KValue %a{i}, 1");
                        format!("i64 %p{i}")
                    } else {
                        format!("%KValue %a{i}")
                    }
                })
                .collect();
            let sym = dsym(name, arity);
            let _ = writeln!(conv, "  %r = call tailcc %KValue @{sym}({})", call_args.join(", "));
            self.room(conv.len() + 3 * sym.len() + 32 * arity + 256);
            let _ = writeln!(
                self.body,
                "define %KValue @{}({}) {{\nentry:\n{conv}  ret %KValue %r\n}}\n",
                wsym(name, arity),
                params.join(", ")
            );
            let (label, _) = self.intern(&format!("{name}\0"));
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 {arity}, ptr @{label}, i64 0 }}",
                rsym(name, arity),
                wsym(name, arity)
            );
        }
        self.builtin_value_wrappers.sort();
        self.builtin_value_wrappers.dedup();
        let builtins = self.builtin_value_wrappers.clone();
        for (name, arity) in &builtins {
            let arity = *arity;
            let params: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let call_args: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let held = format!("builtin.{name}");
            self.room(4 * held.len() + 48 * arity + 256);
            let _ = writeln!(
                self.body,
                "define %KValue @{}({}) {{\nentry:\n  %r = call %KValue @k_b_{name}({})\n  \
                 ret %KValue %r\n}}\n",
                wsym(&held, arity),
                params.join(", "),
                call_args.join(", ")
            );
            let (label, _) = self.intern(&format!("{name}\0"));
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 {arity}, ptr @{label}, i64 1 }}",
                rsym(&held, arity),
                wsym(&held, arity)
            );
        }
        if self.print_value_wrapper {
            let group = "render/to_string";
            let render = match self.declares(group) {
                true => format!(
                    "  %s1 = call tailcc %KValue @{}(%KValue %v)\n  br label %join",
                    dsym(group, 1)
                ),
                false => "  %s1 = call %KValue @k_render(%KValue %v, i64 0)\n  br label %join"
                    .to_string(),
            };
            let held = "builtin.print";
            self.room(render.len() + 1024);
            let _ = writeln!(
                self.body,
                "define %KValue @{}(%KValue %a0) {{\nentry:\n  \
                 %v = call %KValue @k_force(%KValue %a0)\n  \
                 %d = call i64 @k_render_dispatchable(%KValue %v)\n  \
                 %c = icmp ne i64 %d, 0\n  \
                 br i1 %c, label %arm, label %plain\n\
                 arm:\n{render}\n\
                 plain:\n  \
                 %s2 = call %KValue @k_render(%KValue %v, i64 0)\n  \
                 br label %join\n\
                 join:\n  \
                 %s = phi %KValue [ %s1, %arm ], [ %s2, %plain ]\n  \
                 %r = call %KValue @k_desc_print(%KValue %s)\n  \
                 ret %KValue %r\n}}\n",
                wsym(held, 1)
            );
            let (label, _) = self.intern("print\0");
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 1, ptr @{label}, i64 1 }}",
                rsym(held, 1),
                wsym(held, 1)
            );
        }
        // Lazy v1: the thunk-site dispatcher the runtime's k_force calls.
        // Sites are emitted as cases as lazy binds are compiled; a program
        // with no lazy sites still defines the symbol so every binary links.
        self.emit_thunk_dispatcher();
        // No fills. Each constant seeds and builds its own cell on the first
        // read, so a program that never demands one never builds it. What is
        // left here is the math-id handshake below.
        let fills = String::new();
        // Division answers a declared type, so the runtime has to be told which
        // id the compiler gave it. Before the constants, because a constant may
        // divide. A program that cannot reach a math failure never declares the
        // pair and never builds one, so it has nothing to tell.
        let ids = match (
            self.type_ids.get(crate::DIVIDE_BY_ZERO),
            self.type_ids.get(crate::MATH_FAILURE),
        ) {
            (Some(dz), Some(mf)) => format!("  call void @k_math_ids(i64 {dz}, i64 {mf})\n"),
            _ => String::new(),
        };
        self.room(ids.len() + 128);
        let _ = writeln!(
            self.body,
            "define void @k_caf_init() {{\nentry:\n{ids}{fills}  ret void\n}}\n"
        );
        // Work-ahead (#1856): the constants `ahead::lift` made, forced in the
        // order it made them when a fiber first parks. Every binary defines
        // the pool, empty when nothing was lifted, because the runtime calls it.
        // Each constant is read through `k_ahead_try`, which catches a death
        // and empties the cells it left half-built; the table it is handed
        // is every constant's cell beside its ready flag, since the work may
        // have been building any of them.
        let mut pool = String::new();
        let mut shims = String::new();
        let lifted: Vec<String> = self
            .program
            .fns
            .iter()
            .filter(|d| d.params.is_empty() && d.name.starts_with(crate::ahead::PREFIX))
            .map(|d| dsym(&d.name, 0))
            .collect();
        let forced: Vec<&(String, String)> = lifted
            .iter()
            .filter_map(|sym| self.ahead_cells.iter().find(|(s, _)| s == sym))
            .collect();
        if !forced.is_empty() {
            let n = self.caf_cells.len();
            let pairs: Vec<String> =
                self.caf_cells.iter().map(|c| format!("ptr @{c}, ptr @{c}_ready")).collect();
            let _ = writeln!(
                shims,
                "@k_ahead_cells = internal constant [{} x ptr] [{}]",
                2 * n,
                pairs.join(", ")
            );
            for (i, (sym, _)) in forced.iter().enumerate() {
                let _ = writeln!(
                    shims,
                    "define internal void @k_ahead_force_{i}() {{\nentry:\n  \
                     %v = call tailcc %KValue @{sym}()\n  ret void\n}}"
                );
                let _ = writeln!(
                    pool,
                    "  %go{i} = call i32 @k_ahead_try(ptr @k_ahead_force_{i}, ptr @k_ahead_cells, i64 {n})\n  \
                     %on{i} = icmp ne i32 %go{i}, 0\n  \
                     br i1 %on{i}, label %next{i}, label %done\n\
                     next{i}:"
                );
            }
            let _ = writeln!(shims, "declare i32 @k_ahead_try(ptr, ptr, i64)");
            let _ = writeln!(pool, "  br label %done\ndone:");
        }
        self.room(shims.len() + pool.len() + 128);
        self.body.push_str(&shims);
        let _ =
            writeln!(self.body, "define void @k_ahead_pool() {{\nentry:\n{pool}  ret void\n}}\n");
        // A library has no entry to call, and a stub calling one that is not
        // there is a symbol the linker would ask about.
        if self.declares(crate::ast::ENTRY) {
            let entry = dsym(crate::ast::ENTRY, 0);
            self.room(entry.len() + 128);
            let _ = writeln!(
                self.body,
                "define %KValue @k_user_main() {{\nentry:\n  %r = call tailcc %KValue \
             @{entry}()\n  ret %KValue %r\n}}\n"
            );
        }
        // A declaration the program never calls is a line the compile golden
        // counts and the reader scrolls past. Keep a declare only when its
        // symbol appears somewhere outside the declare itself — in the body,
        // or inside one of the preamble's own inline definitions.
        // Only a program with an entry has a place for the walk to start. A
        // library's surface is its callers' business, and every definition in
        // it is reachable from outside the module the emitter can see.
        // The body is taken rather than copied, pruned in its own buffer, and
        // let go once the paired appends have rewritten it. Kept to the end,
        // `self.body` and the pruned copy sat under the tail below, which is
        // the tab's high-water mark.
        let mut pieces = std::mem::take(&mut self.sealed);
        pieces.push(std::mem::take(&mut self.body));
        Ok(Written {
            convention: self.convention,
            inline_helpers: self.inline_helpers,
            has_entry: self.declares(crate::ast::ENTRY),
            asked_left_out: self.asked_left_out.get(),
            pieces,
            closure_consts: std::mem::take(&mut self.closure_consts),
            caf_cells: std::mem::take(&mut self.caf_cells),
            closure_cells: std::mem::take(&mut self.closure_cells),
            strings: std::mem::take(&mut self.strings),
            globals: std::mem::take(&mut self.globals),
        })
    }

    /// Room for `need` more bytes in the piece being written, which closes
    /// it and opens the next when it has less. Called before each definition
    /// is written, and only there, so a definition is never split between two
    /// pieces. Every write to the body asks first, with `need` taken from the
    /// parts of the text that vary. A write that did not ask doubled the
    /// piece once it ran past 64 KiB, and the tab's allocator holds the old
    /// block and the new one while it copies: on `bench/interp_corpus` that
    /// was a dispatcher's write.
    fn room(&mut self, need: usize) {
        if self.body.capacity() - self.body.len() < need {
            let next = String::with_capacity(need.max(BODY_PIECE));
            let piece = std::mem::replace(&mut self.body, next);
            if !piece.is_empty() {
                self.sealed.push(piece);
            }
        }
    }

    fn intern(&mut self, text: &str) -> (String, usize) {
        let bytes = text.as_bytes().to_vec();
        let len = bytes.len();
        if let Some(name) = self.interned.get(&bytes) {
            return (name.clone(), len);
        }
        let name = format!("s{}", self.strings.len());
        self.interned.insert(bytes.clone(), name.clone());
        self.strings.push((name.clone(), bytes));
        (name, len)
    }

    fn str_const(&mut self, f: &mut FnEmit, text: &str) -> String {
        // a literal is the same value every evaluation, so it builds once
        // into a permanent slot instead of allocating per visit
        let (name, len) = self.intern(text);
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue @k_str_lit_fast(ptr @{name}, i64 {len}, ptr @{name}_lit)"
        ));
        t.to_string()
    }

    /// The interned literal for an err construction site: the package that
    /// raises here, then the trace line, each NUL-terminated. The runtime
    /// reads the first for the match rule and the second for the report —
    /// one argument instead of a second threaded through nineteen runtime
    /// signatures to carry a package name.
    fn origin_arg(&mut self, f: &FnEmit, span: Span) -> String {
        let (name, _) = self.intern(&format!("{}\0{}:{}\0", f.hako, f.origin_prefix, span.line));
        format!("ptr @{name}")
    }

    fn emit_type_names(&mut self) {
        // One slot per id: 0 is the entry, a declared type is its position
        // plus one, and an alias's position falls back, since the alias
        // constructs and matches under its origin's id.
        let (fallback, _) = self.intern("record\0");
        let (entry_name, _) = self.intern("entry\0");
        let slots = self.program.types.len() + 1;
        let mut names = vec![fallback.clone(); slots];
        names[0] = entry_name;
        // The spelling a rendered record prints, beside the identity the
        // runtime matches on. RULED 2026-08-29, "records print qualified,
        // everywhere": the root's own types take the root's name.
        let mut shown = names.clone();
        let mut differs = false;
        let root = self.program.root.clone();
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the slot
                continue;
            }
            let id = self.type_ids[ty.name.as_str()] as usize;
            let (name, _len) = self.intern(&format!("{}\0", ty.name));
            shown[id] = match root.is_empty() || crate::ast::has_slash(&ty.name) {
                true => name.clone(),
                false => {
                    differs = true;
                    self.intern(&format!("{root}/{}\0", ty.name)).0
                }
            };
            names[id] = name;
        }
        type_table(&mut self.globals, "k_type_name", &names, &fallback);
        // A program whose root declares no bare type prints every record
        // under the identity's spelling, and the second table is an alias
        // rather than a copy of the first.
        match differs {
            true => type_table(&mut self.globals, "k_type_shown", &shown, &fallback),
            false => self.globals.push_str("@k_type_shown = alias ptr (i64), ptr @k_type_name\n"),
        }
    }

    /// Field metadata for keyed reads: name-indexed lookup reads these
    /// per-type tables at runtime.
    fn emit_type_fields(&mut self) {
        let slots = self.program.types.len() + 1;
        let mut tables: Vec<Vec<String>> = vec![Vec::new(); slots];
        tables[0] = vec!["key".into(), "value".into()];
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the slot
                continue;
            }
            let id = self.type_ids[ty.name.as_str()] as usize;
            tables[id] = ty.fields.iter().map(|(name, _, _)| name.clone()).collect();
        }
        let (empty, _) = self.intern("\0");
        let mut globals = String::new();
        let counts: Vec<String> = tables.iter().map(|f| f.len().to_string()).collect();
        let _ = writeln!(
            globals,
            "@k_type_field_counts = private unnamed_addr constant [{slots} x i64] [{}]",
            counts.iter().map(|c| format!("i64 {c}")).collect::<Vec<_>>().join(", ")
        );
        let mut rows = Vec::with_capacity(slots);
        for (id, fields) in tables.iter().enumerate() {
            if fields.is_empty() {
                rows.push("ptr null".to_string());
                continue;
            }
            let names: Vec<String> = fields
                .iter()
                .map(|field| format!("ptr @{}", self.intern(&format!("{field}\0")).0))
                .collect();
            let _ = writeln!(
                globals,
                "@k_type_fields_{id} = private unnamed_addr constant [{} x ptr] [{}]",
                names.len(),
                names.join(", ")
            );
            rows.push(format!("ptr @k_type_fields_{id}"));
        }
        let _ = writeln!(
            globals,
            "@k_type_fields = private unnamed_addr constant [{slots} x ptr] [{}]",
            rows.join(", ")
        );
        let _ = writeln!(
            globals,
            "define i64 @k_type_field_count(i64 %id) {{\nentry:\n  %in = icmp ult i64 %id, {slots}\n  br i1 %in, label %C, label %CD\nC:\n  %at = getelementptr [{slots} x i64], ptr @k_type_field_counts, i64 0, i64 %id\n  %n = load i64, ptr %at\n  ret i64 %n\nCD:\n  ret i64 0\n}}\n"
        );
        let _ = writeln!(
            globals,
            "define ptr @k_type_field_name(i64 %id, i64 %i) {{\nentry:\n  %n = call i64 @k_type_field_count(i64 %id)\n  %in = icmp ult i64 %i, %n\n  br i1 %in, label %T, label %TD\nT:\n  %at = getelementptr [{slots} x ptr], ptr @k_type_fields, i64 0, i64 %id\n  %row = load ptr, ptr %at\n  %f = getelementptr ptr, ptr %row, i64 %i\n  %name = load ptr, ptr %f\n  ret ptr %name\nTD:\n  ret ptr @{empty}\n}}\n"
        );
        self.globals.push_str(&globals);
    }
}

include!("codegen/dispatchers.rs");

include!("codegen/emit.rs");

impl<'a> Backend<'a> {
    fn emit_lifted(
        &mut self,
        lifted: &str,
        params: &[String],
        captures: &[String],
        body: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new(!self.inline_helpers);
        f.origin_prefix = outer.origin_prefix.clone();
        f.hako = outer.hako.clone();
        // A lifted lambda is still code from the file it was written in, and
        // in-place sites are keyed by source position — without this the key
        // is ("", line, col) and every mark inside a lambda body is missed.
        // That miss was load-bearing until the analysis stopped marking writes
        // in lambdas whose run time it cannot account for; it now marks only a
        // fold's reducer, which is applied at once and per element to the
        // accumulator the fold owns.
        f.file = outer.file.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            let slot = f.tmp();
            f.line_fmt(format_args!("{slot} = getelementptr %KValue, ptr %env, i64 {i}"));
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = load %KValue, ptr {slot}"));
            f.bind(cap, &t);
        }
        for (i, p) in params.iter().enumerate() {
            f.bind(p, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, body)?;
        let sig: String = (0..params.len()).map(|i| format!(", %KValue %a{i}")).collect();
        let text = f.body();
        self.room(2 * lifted.len() + 3 * sig.len() + text.len() + 256);
        let _ =
            writeln!(self.body, "define tailcc %KValue @{lifted}(ptr %env{sig}) {{\n{text}}}\n");
        // The convention rides on the wrapper AND on every arm that calls
        // through a closure pointer. Split them and the arguments arrive in
        // the wrong registers: leaving the runtime's `k_call{n}` on the C
        // convention while this carries preserve_none made encodebench print
        // `error[runtime]: bytes takes a string` where its answer is
        // `done: 74072800`.
        let cc = self.convention.keyword();
        let _ = writeln!(
            self.body,
            "define {cc}%KValue @w_{lifted}(ptr %env{sig}) {{\nentry:\n  %r = call \
             tailcc %KValue @{lifted}(ptr %env{sig})\n  ret %KValue %r\n}}\n"
        );
        Ok(())
    }
}

fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) | Expr::Partial(..) => {}
        Expr::Block(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => collect_idents(expr, out),
                }
            }
        }
        Expr::Field { base, .. } => collect_idents(base, out),
        Expr::Upcast { expr, .. } => collect_idents(expr, out),
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    collect_idents(inner, out);
                }
            }
        }
        Expr::Ident(name, _, _) => out.push(name.to_string()),
        Expr::List(items, _) => {
            for item in items {
                collect_idents(item, out);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                collect_idents(key, out);
                collect_idents(value, out);
            }
        }
        Expr::App { head, args, .. } => {
            collect_idents(head, out);
            for arg in args {
                collect_idents(arg, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_idents(base, out);
            collect_idents(index, out);
        }
        Expr::Lambda { body, .. } => collect_idents(body, out),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            collect_idents(lhs, out);
            collect_idents(rhs, out);
        }
        Expr::Guard { cond, early, rest, .. } => {
            collect_idents(cond, out);
            collect_idents(early, out);
            for stmt in rest.iter() {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) => collect_idents(expr, out),
                }
            }
        }
    }
}

fn ir_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        match byte {
            0x20..=0x7e if *byte != b'"' && *byte != b'\\' => out.push(*byte as char),
            _ => {
                let _ = write!(out, "\\{byte:02X}");
            }
        }
    }
    out
}

/// `tailcc` where it is needed, the C convention everywhere else.
///
/// A `musttail` call may cross an arity or a type only under `tailcc`, so the
/// beat machinery's guaranteed tail calls need it. Every other call does not,
/// and paying for it is not free: a non-tail `call tailcc` whose arguments do
/// not all fit in registers is miscompiled on arm64 — five KValues want ten
/// argument registers and there are eight, and the two that spill come back
/// holding each other's values (task #70). So the convention is kept for the
/// functions a musttail reaches, on both ends of every such edge, and dropped
/// from the rest.
///
/// A function that needs the convention AND spills is reached through a
/// trampoline: one `tailcc` call per frame is lowered correctly, and it is
/// only the second one in a frame that comes back wrong.
///
/// The set is read out of the emitted text rather than recomputed, because a
/// second copy of "when do we musttail" would drift from the first and the
/// symptom of drift is silent corruption.
///
/// Only code is read. A line opening with `@` defines a global, a string
/// constant among them, and a constant's bytes are the program's: a program
/// printing `call tailcc` once lost the words from its constant, kept the
/// declared length, and clang refused the module.
/// Whether `line` holds `needle`, found from the byte `at` places into it.
/// `str::contains` builds a substring searcher for every call, about a
/// hundred instructions before it reads a byte, and `narrow_tailcc` asks two
/// or three of these of every line it walks: 178 calls on a one-line
/// program's start-up. The anchor byte is found by memchr, and the needle is
/// compared where one lands. Pick `at` so the anchor is rare in the emitter's
/// lines.
/// The lines of `text` that hold `needle`, each once and in order, as byte
/// ranges without the newline. The same lines `text.lines()` filtered by
/// `contains(needle)` would give, for text with no carriage returns, which
/// emitted IR never has.
fn lines_holding<'a>(text: &'a str, needle: &'a str) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut past = 0;
    text.match_indices(needle).filter_map(move |(at, _)| {
        if at < past {
            return None;
        }
        let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
        past = end;
        Some((start, end))
    })
}

/// Two ascending runs of line ranges as one, a line in both given once.
fn merged_lines(
    a: impl Iterator<Item = (usize, usize)>,
    b: impl Iterator<Item = (usize, usize)>,
) -> impl Iterator<Item = (usize, usize)> {
    let mut a = a.peekable();
    let mut b = b.peekable();
    std::iter::from_fn(move || match (a.peek().copied(), b.peek().copied()) {
        (Some(x), Some(y)) if x.0 == y.0 => {
            a.next();
            b.next()
        }
        (Some(x), Some(y)) if x.0 < y.0 => a.next(),
        (Some(_), Some(_)) => b.next(),
        (Some(_), None) => a.next(),
        (None, _) => b.next(),
    })
}

fn holds(line: &str, needle: &str, at: usize) -> bool {
    let anchor = needle.as_bytes()[at] as char;
    line.match_indices(anchor)
        .any(|(i, _)| i >= at && line.as_bytes()[i - at..].starts_with(needle.as_bytes()))
}

#[cfg(test)]
fn narrow_tailcc(ir: String) -> String {
    narrowed_in_place(ir, 0)
}

/// `narrow_tailcc`'s answer for everything in `text` after `from`, written
/// over the lines it replaces. Every rewrite takes words out of a line or
/// renames a callee to its trampoline, which is shorter than the words it
/// lost, so a line never outgrows the one it replaces and the text only moves
/// down. A rewrite that did grow would send the whole answer into a buffer of
/// its own, so the answer does not rest on that.
fn narrowed_in_place(text: String, from: usize) -> String {
    let code = |line: &str| !line.starts_with('@');
    // Two walks over the lines, neither of which keeps a list of them: one
    // reads which functions keep the convention and sets aside the few
    // `define tailcc` lines the trampolines are decided from, and one finds
    // the lines to rewrite. A list of every line, held across the three
    // walks this used to make, was a third of the module's size again.
    let ir = &text[from..];
    let mut keep: crate::hash::Set<String> = crate::hash::Set::default();
    let mut current: Option<String> = None;
    let mut tailcc_defines: Vec<&str> = Vec::new();
    // The walks visit only the lines that hold the words they act on, found by
    // searching the text for the words. On the tab's corpus 125 lines of 8,726
    // are a `define` and 96 a musttail call, and the line-by-line walk this
    // replaced was 3.9 million instructions of finding newlines.
    let defines =
        lines_holding(ir, "define ").filter(|&(start, _)| ir[start..].starts_with("define "));
    let musttails = lines_holding(ir, "musttail call");
    let jumps_to_lambdas = ir.contains("define internal tailcc %KValue @k_tcall");
    for (start, end) in merged_lines(defines, musttails) {
        let line = &ir[start..end];
        if !code(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix("define ") {
            current = symbol_of(rest);
            if rest.starts_with("tailcc ") {
                tailcc_defines.push(line);
            }
            // `k_tcall{n}` jumps to a lambda through the pointer its closure
            // carries, an edge this reading cannot see, so every lambda keeps
            // the convention in a module that has one.
            if jumps_to_lambdas
                && current.as_deref().is_some_and(|n| {
                    n.strip_prefix("klam").is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()))
                })
            {
                keep.extend(current.clone());
            }
        }
        if line.contains("musttail call") {
            // both ends of a musttail edge must agree on the convention
            if let Some(callee) = symbol_of(line) {
                keep.insert(callee);
            }
            if let Some(name) = current.clone() {
                keep.insert(name);
            }
        }
    }
    // Kept functions whose arguments do not all fit in the eight registers
    // AArch64 passes them in. The count is the same on every host so the ir is
    // too; x86 passes fewer and does not exhibit the defect anyway.
    let mut trampolines: Vec<(String, String)> = Vec::new();
    let mut spilling: crate::hash::Set<String> = crate::hash::Set::default();
    for &line in &tailcc_defines {
        let Some(rest) = line.strip_prefix("define tailcc ") else { continue };
        let Some(name) = symbol_of(rest) else { continue };
        if !keep.contains(&name) {
            continue;
        }
        let Some(open) = rest.find('(') else { continue };
        let Some(close) = rest.rfind(')') else { continue };
        let params: Vec<&str> = match rest[open + 1..close].trim().is_empty() {
            true => Vec::new(),
            false => rest[open + 1..close].split(", ").collect(),
        };
        let types: Vec<&str> = params.iter().filter_map(|p| p.split_whitespace().next()).collect();
        let registers: usize = types.iter().map(|t| register_width(t)).sum();
        if registers <= 8 {
            continue;
        }
        let ret = rest[..open].split('@').next().unwrap_or("%KValue").trim().to_string();
        let taken: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        let handed: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        trampolines.push((
            name.clone(),
            format!(
                "define {ret} @{}({}) {{\nentry:\n  %r = call tailcc {ret} @{}({})\n  ret {ret} %r\n}}\n",
                trampoline_name(&name),
                taken.join(", "),
                quoted(&name),
                handed.join(", ")
            ),
        ));
        spilling.insert(name);
    }

    drop(tailcc_defines);
    let mut rerouted: crate::hash::Set<String> = crate::hash::Set::default();
    // (start, end, replacement) for each line that changes, in order.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (start, end) in lines_holding(ir, "tailcc ") {
        // Every rewrite below needs the word, so a line without it stands as
        // it is and its callee is never looked up.
        let line = &ir[start..end];
        if !code(line) {
            continue;
        }
        let named = symbol_of(line);
        // `k_tcall{n}`'s jump through a closure names no symbol. Every lambda
        // it can reach kept the convention above, so the jump keeps it too.
        if named.is_none() && holds(line, "musttail call", 0) {
            continue;
        }
        let reroute = !holds(line, "musttail call", 0)
            && line.contains("call tailcc ")
            && named.as_ref().is_some_and(|n| spilling.contains(n));
        let rewritten = if reroute {
            let name = named.expect("checked above");
            let call = format!("@{}(", quoted(&name));
            let through = format!("@{}(", trampoline_name(&name));
            rerouted.insert(name);
            line.replace("call tailcc ", "call ").replace(&call, &through)
        } else if !named.as_ref().is_some_and(|n| keep.contains(n)) {
            line.replace("tailcc ", "")
        } else {
            continue;
        };
        let at = line.as_ptr() as usize - text.as_ptr() as usize;
        edits.push((at, at + line.len(), rewritten));
    }
    let shrinks = edits.iter().all(|(start, end, line)| line.len() <= end - start);
    let mut out = match shrinks {
        true => {
            let mut bytes = text.into_bytes();
            let (mut to, mut kept) = (0, 0);
            for (start, end, line) in edits {
                bytes.copy_within(kept..start, to);
                to += start - kept;
                bytes[to..to + line.len()].copy_from_slice(line.as_bytes());
                to += line.len();
                kept = end;
            }
            let len = bytes.len();
            bytes.copy_within(kept..len, to);
            bytes.truncate(to + len - kept);
            String::from_utf8(bytes).expect("each edit replaces a whole line")
        }
        false => {
            let mut out = String::with_capacity(text.len());
            let mut kept = 0;
            for (start, end, line) in edits {
                out.push_str(&text[kept..start]);
                out.push_str(&line);
                kept = end;
            }
            out.push_str(&text[kept..]);
            out
        }
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    // A trampoline no call was rerouted through is a function nothing names,
    // which clang compiles at `-O0` all the same.
    for (name, t) in trampolines {
        if rerouted.contains(&name) {
            out.push_str(&t);
        }
    }
    out
}

/// How much of the emitted body a piece holds before the next is opened.
const BODY_PIECE: usize = 64 * 1024;

/// The first symbol a line names, without its quotes.
fn symbol_of(line: &str) -> Option<String> {
    let at = line.find('@')?;
    let rest = &line[at + 1..];
    match rest.starts_with('"') {
        true => rest[1..].split('"').next().map(str::to_string),
        false => rest.split('(').next().map(|s| s.trim().to_string()),
    }
}

/// A qualified name, or one carrying a naming sigil, needs LLVM's quoted-
/// identifier form. One list, because a second copy drifts and what it costs
/// is a module clang refuses to read.
fn quoted(name: &str) -> String {
    match name.contains(['/', '!', '?', '+', '-', '*', '%', '<', '>', '=', '~']) {
        true => format!("\"{name}\""),
        false => name.to_string(),
    }
}

fn trampoline_name(name: &str) -> String {
    quoted(&format!("{name}.c"))
}

/// How many argument registers a parameter of this type occupies.
fn register_width(ty: &str) -> usize {
    match ty {
        "%KValue" | "%parsed" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod the_emitter_group_index_answers_what_the_scan_answered {
    use super::*;

    /// What `Backend::group_indices` did before the index existed, kept as the
    /// oracle.
    fn scanned(program: &Program, name: &str, arity: usize) -> Vec<usize> {
        program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, d)| d.name == name && d.params.len() == arity)
            .map(|(i, _)| i)
            .collect()
    }

    /// Every question the emitter can ask, asked both ways. `group_param_set`
    /// and `group_return_set` FOLD over the answer, so a missing or extra
    /// index is a wrong type set and a silently wrong ABI -- and the fold is
    /// order-independent, which is why order is checked here anyway rather
    /// than left to a future caller to discover.
    fn agrees_over(program: &Program) {
        let by_name = group_indices_by_name(program);
        let mut arities: Vec<usize> = program.fns.iter().map(|d| d.params.len()).collect();
        arities.sort_unstable();
        arities.dedup();
        let mut names: Vec<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
        names.push("a name nothing declares");
        for name in names {
            for &arity in arities.iter().chain(std::iter::once(&99)) {
                let want = scanned(program, name, arity);
                let got: Vec<usize> = group_indices_in(&by_name, program, name, arity).collect();
                assert_eq!(
                    want, got,
                    "`{name}` at arity {arity}: the scan found {want:?} and the index found {got:?}"
                );
            }
        }
    }

    #[test]
    fn over_the_library_the_compiler_carries() {
        let program =
            crate::compile_module(std::path::Path::new("lib/json"), false).expect("lib/json");
        agrees_over(&program);
    }

    #[test]
    fn over_a_name_declared_at_two_arities() {
        // One name at two arities, and a second sharing neither: the case a
        // name-keyed index has to get right.
        let src = concat!(
            "fn f x\n  x\n\n",
            "fn f x y\n  x + y\n\n",
            "fn g x\n  x\n\n",
            "main = print \"{f 1} {f 1 2} {g 3}\"\n"
        );
        let program = crate::compile("test.kso", src, false).expect("the fixture compiles");
        agrees_over(&program);
    }
}

#[cfg(test)]
mod a_trampoline_is_emitted_for_a_call_that_uses_it {
    use super::narrow_tailcc;

    /// Five KValues are ten registers, past the eight AArch64 passes, and the
    /// musttail keeps the convention, so the function spills.
    const LOOP: &str = "define tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e) {\nentry:\n  %r = musttail call tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e)\n  ret %KValue %r\n}\n";

    #[test]
    fn none_when_nothing_is_rerouted() {
        let out = narrow_tailcc(LOOP.to_string());
        assert!(!out.contains("@f.c("), "a trampoline nothing calls was emitted:\n{out}");
    }

    #[test]
    fn one_when_a_call_goes_through_it() {
        let caller = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call tailcc %KValue @f(%KValue %a, %KValue %a, %KValue %a, %KValue %a, %KValue %a)\n  ret %KValue %r\n}\n";
        let out = narrow_tailcc(format!("{LOOP}{caller}"));
        assert!(
            out.contains("define %KValue @f.c("),
            "the rerouted call has no trampoline:\n{out}"
        );
        assert!(out.contains("call %KValue @f.c("), "the call was not rerouted:\n{out}");
    }

    /// The narrowing writes over the lines it reads, which holds while no
    /// rewrite is longer than its line. Rerouting takes seven bytes out and
    /// adds two per mention of the callee, so a line naming it four times
    /// grows by one, and the answer is written into a buffer of its own. The
    /// lines after it come through rewritten all the same.
    #[test]
    fn a_line_that_grows_is_written_beside_the_rest() {
        let caller = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call tailcc %KValue @f(@f(@f(@f(%KValue %a)\n  %s = call tailcc %KValue @h(%KValue %a)\n  ret %KValue %r\n}\n";
        let out = narrow_tailcc(format!("{LOOP}{caller}"));
        let narrowed = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call %KValue @f.c(@f.c(@f.c(@f.c(%KValue %a)\n  %s = call %KValue @h(%KValue %a)\n  ret %KValue %r\n}\n";
        assert!(
            out.starts_with(&format!("{LOOP}{narrowed}define %KValue @f.c(")),
            "the narrowed module is not the line-by-line rewrite:\n{out}"
        );
    }
}

/// The lines of an emitted module that assume a 64-bit pointer, each with what
/// it says on wasm32.
///
/// The emitter writes its inline helpers against the runtime's structs by byte
/// offset, and most of those offsets hold on both targets because the field
/// before them is a `long long`. These are the ones that follow a pointer:
/// `KStr` is `{ char*, int, int }`, so its length and capacity move up four
/// bytes; `KMap`'s `sorted` follows two pointers where `pairs` follows a
/// `long long`; and `KClosure` is `{ fn, env, int ncaps, int arity, body }`,
/// so `env` follows the function pointer and `arity` and `body` follow two
/// pointers.
///
/// A line is matched whole, so a helper whose text changes stops matching
/// rather than matching something else. tests/native_layout_on_wasm32.rs reads
/// the emitter for each left-hand side and fails when one is missing, and
/// runs the micro corpus through this pass against its goldens.
pub const WASM32_LINES: [(&str, &str); 7] = [
    ("%slenp = getelementptr i8, ptr %sp, i64 8", "%slenp = getelementptr i8, ptr %sp, i64 4"),
    ("%lenp = getelementptr i8, ptr %s, i64 8", "%lenp = getelementptr i8, ptr %s, i64 4"),
    ("%cp = getelementptr i8, ptr %sptr, i64 12", "%cp = getelementptr i8, ptr %sptr, i64 8"),
    ("%sortedp = getelementptr i8, ptr %m, i64 16", "%sortedp = getelementptr i8, ptr %m, i64 12"),
    ("%arp = getelementptr i8, ptr %c, i64 20", "%arp = getelementptr i8, ptr %c, i64 12"),
    ("%bodyp = getelementptr i8, ptr %c, i64 24", "%bodyp = getelementptr i8, ptr %c, i64 16"),
    ("%envp = getelementptr i8, ptr %c, i64 8", "%envp = getelementptr i8, ptr %c, i64 4"),
];

/// An emitted module, rewritten for wasm32 so that clang can build it against
/// the runtime compiled for the same target.
///
/// Three things change and nothing else. The target lines: the triple names
/// wasm32 and the x86 data layout goes, so clang supplies its own. The calling
/// conventions: wasm has neither `tailcc` nor `preserve_none`, and a module
/// built with `-mtail-call` keeps every `musttail` as a real tail call under
/// the C convention. And the six offsets in `WASM32_LINES`.
///
/// Only code is rewritten, for the reason `narrow_tailcc` gives: a line
/// opening with `@` is a global, and a string constant's bytes are the
/// program's.
pub fn retarget_wasm32(module: &str) -> String {
    // The edits are found by searching the module for the words they act on
    // and written between copies of everything else. A pass that visited
    // every line, and searched each one for `cc `, was 6.2 million of the tab
    // compile's 168 million instructions natively, for a few hundred edits in
    // 8,726 lines.
    let line_of = |at: usize| {
        let start = module[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = module[at..].find('\n').map_or(module.len(), |n| at + n);
        (start, end)
    };
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for (start, end) in lines_holding(module, "target ") {
        let line = &module[start..end];
        if line.starts_with("target datalayout") {
            edits.push((start, (end + 1).min(module.len()), ""));
        } else if line.starts_with("target triple") {
            edits.push((start, end, "target triple = \"wasm32-unknown-wasi\""));
        }
    }
    for (at, _) in module.match_indices("cc ") {
        let Some(cc) =
            ["preserve_nonecc ", "tailcc "].iter().find(|cc| module[..at + 3].ends_with(*cc))
        else {
            continue;
        };
        if module[line_of(at).0..].starts_with('@') {
            continue;
        }
        edits.push((at + 3 - cc.len(), at + 3, ""));
    }
    for (start, end) in lines_holding(module, " = getelementptr i8, ptr %") {
        let line = &module[start..end];
        if line.starts_with('@') {
            continue;
        }
        let body = line.trim_start();
        if let Some((_, wasm)) = WASM32_LINES.iter().find(|(native, _)| *native == body) {
            edits.push((end - body.len(), end, wasm));
        }
    }
    edits.sort_unstable_by_key(|&(start, _, _)| start);
    let mut out = String::with_capacity(module.len());
    let mut kept = 0;
    for (start, end, with) in edits {
        out.push_str(&module[kept..start]);
        out.push_str(with);
        kept = end;
    }
    out.push_str(&module[kept..]);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The tab emits only the groups the entry reaches, by the names its bodies
/// mention. When the walk misses one the module still calls, the emitter has
/// to notice and emit again with the group put back; these pin that it
/// notices, and that it stays quiet when nothing called was left out.
#[cfg(test)]
mod a_group_the_walk_misses_is_put_back {
    use super::{emit_with, reached_from_entry, ClosureConvention};

    const SOURCE: &str = "import \"std/list\"\n\nprint \"{list/sum [3 1 2]}\"\n";

    fn program() -> crate::ast::Program {
        crate::compile_source("run", "main.kso", SOURCE).expect("the fixture compiles")
    }

    /// The program without the groups named `gone`, and those groups.
    fn without(gone: &[&str]) -> (crate::ast::Program, Vec<crate::ast::FnDecl>) {
        let mut program = program();
        let (out, kept): (Vec<_>, Vec<_>) =
            program.fns.drain(..).partition(|d| gone.contains(&d.name.as_str()));
        program.fns = kept;
        (program, out)
    }

    #[test]
    fn the_walk_reaches_what_the_entry_calls_and_not_the_rest() {
        let program = program();
        let reached = reached_from_entry(&program).expect("the fixture has an entry");
        assert!(reached.contains("list/sum"), "the walk missed a call: {reached:?}");
        assert!(!reached.contains("list/sort"), "the walk kept a group nothing calls: {reached:?}");
    }

    /// Emits the program with the groups named `gone` taken out, as the tab
    /// takes out what its walk does not reach, and says whether the emitter
    /// noticed one it needed.
    fn noticed_without(gone: &[&str]) -> bool {
        let (program, out) = without(gone);
        assert_eq!(out.len(), gone.len(), "the fixture lacks one of {gone:?}");
        let names = out.iter().map(|d| d.name.clone()).collect();
        match emit_with(&program, ClosureConvention::Absent, false, &Vec::new(), names) {
            Ok(written) => written.asked_left_out || written.names_any(out.iter()),
            Err(_) => true,
        }
    }

    #[test]
    fn a_group_nothing_calls_left_out_goes_unremarked() {
        assert!(!noticed_without(&["list/sort"]), "the emitter asked for a group nothing calls");
    }

    #[test]
    fn a_called_group_left_out_is_noticed() {
        assert!(
            noticed_without(&["list/sum"]),
            "the entry calls `list/sum`, which was left out, and nothing noticed"
        );
    }

    /// An operator's arms are found by asking whether the program declares
    /// the operator, and with the arm gone the answer sends records down the
    /// numeric path. Nothing calls `+` by name, so the module names no symbol
    /// of it either way: only the question shows the miss.
    #[test]
    fn a_question_about_a_left_out_operator_is_noticed() {
        // the micro golden an_operator_arm_takes_its_records_boxed, inline
        let source = "import \"std/os\"\n\ntype pt\n  n\n  s\n\n\
                      fn + (pt a s) (pt b t)\n  pt (a + b) \"{s}{t}\"\n\n\
                      pub play = os/args .> (_ -> print \"{pt 1 \"a\" + pt 3 \"b\"}\")\n";
        let mut program = crate::compile_source("check", "arms.kso", source).expect("it compiles");
        let before = program.fns.len();
        program.fns.retain(|d| d.name != "+");
        assert_eq!(program.fns.len() + 1, before, "the fixture has one `+` arm to leave out");
        let names = std::iter::once("+".to_string()).collect();
        let written = emit_with(&program, ClosureConvention::Absent, false, &Vec::new(), names)
            .expect("records added with no arm still lower");
        assert!(written.asked_left_out, "the emitter asked about `+` and nothing saw it");
    }

    /// The fallback sees a question about a left-out group only when the
    /// question goes through `group_named` or `group_indices`. A scan of the
    /// program's groups by name anywhere else in the emitter would answer
    /// without being seen, and the tab would emit a program with the answer
    /// wrong. The patterns are built here so this file does not match itself.
    #[test]
    fn a_lookup_by_name_goes_through_the_index() {
        let source = include_str!("codegen.rs");
        let scan = concat!("self.program.fns", ".iter()");
        let index = concat!("self.group_by", "_name");
        assert_eq!(source.matches(scan).count(), 0, "the emitter scans its groups by hand");
        assert_eq!(
            source.matches(index).count(),
            2,
            "the emitter reads the index around the helpers"
        );
    }
}
