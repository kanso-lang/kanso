use crate::ast::*;
use crate::diag::Span;
use crate::hash::Map as HashMap;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, FLOAT, INT, LIST, MAP, NONE, REC, STR, TOP};
use crate::name::Name;
use std::fmt::Write as _;

// The runtime's tags and doors, read from its C by build.rs.
include!(concat!(env!("OUT_DIR"), "/runtime_abi.rs"));

/// What an `if`'s condition left behind: the blocks that branched straight to
/// the merge because the condition was a failure, each with the value it
/// carried. A comparison on two ints leaves none, which is the point.
struct Cond {
    failed: Vec<(String, String)>,
}
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
pub const STATS_GATE_SITES: usize = 12;

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

pub(crate) const BUILTIN_CALLS: [&str; 76] = [
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
    "plain_string",
    "unescape",
    "append_escaped",
    "append_key",
    "append_string",
    "entry_count",
    "key_at",
    "value_at",
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
    "modified",
    "list_dir",
    "make_dir",
    "write_file",
    "run",
    "listen",
    "accept",
    "net_read",
    "net_read_bytes",
    "dial",
    "net_secure",
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
    "delete",
    "slice",
    "rest_from",
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

/// Whether `arms` together take every combination of the kinds in `sets`,
/// one position at a time: the kinds an argument may hold are grouped by the
/// arms that take them, and each group must be covered by those arms on the
/// positions after. An argument carried as a by-value record is that record
/// type and nothing else, so an arm that destructures that type into plain
/// names takes it. `budget` bounds the walk; running out answers no.
fn covered(arms: &[&[Pattern]], sets: &[Set], records: &[Option<&str>], budget: &mut u32) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    let Some((&set, rest)) = sets.split_first() else { return !arms.is_empty() };
    let (record, records) = (records[0], &records[1..]);
    let kinds = set & !FAIL;
    if let Some(record) = record {
        let simple = |p: &Pattern| matches!(p, Pattern::Var(..) | Pattern::Wildcard(_));
        let after: Vec<&[Pattern]> = arms
            .iter()
            .filter(|a| match &a[0] {
                Pattern::Var(..) | Pattern::Wildcard(_) => true,
                Pattern::Ctor { ty, fields, .. } => ty == record && fields.iter().all(simple),
                Pattern::Annotated { ty, .. } => ty == record,
                _ => false,
            })
            .map(|a| &a[1..])
            .collect();
        return !after.is_empty() && covered(&after, rest, records, budget);
    }
    let takes = |p: &Pattern, kind: Set| match p {
        Pattern::Var(..) | Pattern::Wildcard(_) => true,
        Pattern::Annotated { ty, .. } => match ty.as_str() {
            "int" | "float64" | "string" | "bool" => kind & !infer::annotated_set(ty) == 0,
            "none" => kind == NONE,
            "done" => kind == infer::DONE,
            "some" => kind != NONE,
            // a list or a map is tested by its tag, whatever it holds
            t if t.ends_with("[]") => kind == LIST,
            t if t.contains('[') => kind == MAP,
            _ => false,
        },
        Pattern::Nullary(nm, _) => match nm.as_str() {
            "true" => kind == infer::TRUE,
            "false" => kind == infer::FALSE,
            "none" => kind == NONE,
            "done" => kind == infer::DONE,
            _ => false,
        },
        _ => false,
    };
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for bit in (0..Set::BITS).map(|i| 1 << i).filter(|b| kinds & b != 0) {
        let taking: Vec<usize> = (0..arms.len()).filter(|&i| takes(&arms[i][0], bit)).collect();
        if taking.is_empty() {
            return false;
        }
        if !groups.contains(&taking) {
            groups.push(taking);
        }
    }
    groups.iter().all(|g| {
        let after: Vec<&[Pattern]> = g.iter().map(|&i| &arms[i][1..]).collect();
        covered(&after, rest, records, budget)
    })
}

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

/// The map and position of `which m i`, a read of one map entry.
fn entry_of<'e>(e: &'e Expr, which: &str) -> Option<(&'e str, &'e str)> {
    let Expr::App { head, args, piped: false, .. } = e else { return None };
    match (&**head, args.as_slice()) {
        (Expr::Ident(h, _, _), [Expr::Ident(m, _, _), Expr::Ident(i, _, _)])
            if h.as_str() == which =>
        {
            Some((m.as_str(), i.as_str()))
        }
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
    fn new(prefix: &str, n: usize) -> Numbered {
        let prefix = prefix.as_bytes();
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
        // SAFETY: `Numbered::new` is the only place a `Numbered` is built, and
        // it writes a `&str` prefix and ASCII digits after it, so `buf[..len]`
        // is UTF-8 by construction. Checking it on every read was 1.1M of the
        // tab's 257M compile instructions; the emitter reads one for nearly
        // every instruction it writes.
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
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

/// `emit_ir_dev_owned` for the playground tab, in the pieces
/// `ir_wasm::translate_pieces` reads (`Written::finish_pieces`). It emits only
/// the groups the entry can reach, as every build does; the prune in
/// `Written::finish_pieces` drops the rest from the module either way. On
/// `bench/interp_corpus` that was 132 definitions and 127,415 bytes of text, a
/// third of what the tab wrote, held beside the program at the most its
/// compile ever holds.
///
/// The groups the entry cannot reach are let go before anything is emitted,
/// where every other build keeps them in case the emitter asks for one; that
/// held 65,530 bytes of syntax at the tab's compile peak on the corpus. A miss
/// answers `None`, and the tab compiles the source again and calls
/// `emit_ir_tab_pieces_whole`, which keeps them, so a miss costs the tab a
/// second compile and never changes the module.
pub fn emit_ir_tab_pieces(
    program: Program,
    convention: ClosureConvention,
) -> Result<Option<Vec<String>>, String> {
    let written = emit_written_as(Given::Owned(program), convention, false, true)?;
    Ok(written.map(Written::finish_pieces))
}

/// `emit_ir_tab_pieces` holding what the entry cannot reach, for a program
/// that missed without it.
pub fn emit_ir_tab_pieces_whole(
    program: Program,
    convention: ClosureConvention,
) -> Result<Vec<String>, String> {
    emit_written(Given::Owned(program), convention, false).map(Written::finish_pieces)
}

include!("codegen/unbuilt.rs");

/// The work both tiers share, and the frame `emit_instructions` anchors on.
/// Kept out of line so the anchor exists whichever entry point reached it.
/// The attribute sat on `without_unbuilt_arms`, now in unbuilt.rs, for a
/// while, with a doc comment between them, and this frame stayed out of line
/// only because it was large; once it was two calls, the profile lost it.
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
    emit_written_as(program, convention, inline_helpers, false)
        .map(|written| written.expect("a build that keeps what it left out never misses"))
}

/// `emit_written`, and when `lean`, letting the groups the entry cannot reach
/// go before emitting and answering `None` where the emitter missed one.
fn emit_written_as(
    program: Given<'_>,
    convention: ClosureConvention,
    inline_helpers: bool,
    lean: bool,
) -> Result<Option<Written>, String> {
    // The prune also answers the positions a group read as written and no
    // longer reads. A thunk handed to such a position is forced by
    // the interpreter, whose dispatcher still holds the dropped arm and has to
    // look at the value to rule it out; an err the thunk turns into then answers
    // at that group, with its name in the trace. `list/find` loses its arm for a
    // sorted list in a program that sorts nothing, and a lazy list handed to it
    // went on to fail in `list/next`, two frames later and under other names.
    // Forcing where the program as written would have looked keeps the trace
    // the interpreter's.
    let program = match crate::ahead::lifts(&program) || crate::ahead::pools(&program) {
        true => {
            let mut owned = match program {
                Given::Owned(p) => p,
                Given::Lent(p) => Program {
                    fns: p.fns.clone(),
                    types: p.types.clone(),
                    imports: p.imports.clone(),
                    reexports: p.reexports.clone(),
                    derives: p.derives.clone(),
                    root: p.root.clone(),
                },
            };
            crate::ahead::lift(&mut owned);
            crate::ahead::pool(&mut owned);
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
    // The tab is priced by the most it holds, and that is during emission,
    // with the syntax alive. A list the parser grew by pushing keeps the room
    // it doubled into; what the walk kept gives it back here, where a native
    // build has no reason to pay for the copies.
    if let (true, Given::Owned(owned)) = (lean, &mut program) {
        for decl in owned.fns.iter_mut() {
            decl.body.shrink_to_fit();
            decl.params.shrink_to_fit();
        }
        owned.fns.shrink_to_fit();
    }
    let names: crate::hash::Set<String> = left_out.iter().map(|(_, d)| d.name.clone()).collect();
    let left_names: Vec<(String, usize)> = match lean {
        true => std::mem::take(&mut left_out)
            .into_iter()
            .map(|(_, d)| (d.name, d.params.len()))
            .collect(),
        false => Vec::new(),
    };
    let first = emit_with(&program, convention, inline_helpers, &unread_positions, names);
    // The walk reads names the bodies mention, and the emitter can reach a
    // group no body names, by calling it or by asking whether it exists. Either
    // one, for a group that was left out, gives the program back what it lost,
    // in its place, and emits it again, so a miss costs time and never changes
    // the module. So does a refusal: a call to a group that is not there is one
    // the emitter declines to lower, and the program as written may not be.
    if lean {
        let missed = match &first {
            Ok(written) => {
                written.asked_left_out
                    || written.names_any(left_names.iter().map(|(n, a)| (n.as_str(), *a)))
            }
            Err(_) => true,
        };
        return match (missed && !left_names.is_empty(), first) {
            (true, _) => Ok(None),
            (false, first) => first.map(Some),
        };
    }
    let missed = match &first {
        Ok(written) => {
            written.asked_left_out
                || written
                    .names_any(left_out.iter().map(|(_, d)| (d.name.as_str(), d.params.len())))
        }
        Err(_) => true,
    };
    if left_out.is_empty() || !missed {
        return first.map(Some);
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
    emit_with(&program, convention, inline_helpers, &unread_positions, Default::default()).map(Some)
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
    // The by-value `%parsed` is three i64s, so it only fits a record shaped
    // like the scanner's `_parsed`: exactly two fields, an int position whose
    // tag shares the first word with the value's, and a non-failure value.
    // Any other register-returnable record keeps the heap representation.
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
        reaches_cycle: reaches_cycle(program, &beat.allocating),
        deep: crate::deep::Deep::of(program),
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
        sites: Vec::new(),
        site_ids: Vec::new(),
        body: String::new(),
        sealed: Vec::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
        builtin_value_wrappers: Vec::new(),
        defers_self_reference: !knotted.is_empty() || ties(program),
        knotted,
        read_links: crate::read_at::links(program),
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
    /// Names that run a loop before they answer, whose calls keep a cohort
    /// bracket from a caller a cycle reaches. See `reaches_cycle`.
    reaches_cycle: crate::hash::Set<&'a str>,
    /// The cycles that grow the stack, and the place an overflow in each
    /// names (see `crate::deep`).
    deep: crate::deep::Deep,
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
    /// The interned literal each stamped site id names, by its position in
    /// `strings`, id 1 first; and the id each position was given, 0 for none.
    /// A stamp stores the id, and `k_site_name` reads the literal back only
    /// when the program dies.
    sites: Vec<u32>,
    site_ids: Vec<u32>,
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
    /// Groups whose every call passes a collection, a value read out of it
    /// and the position it was read at: `read_at`.
    read_links: crate::read_at::Links,
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

include!("codegen/fn_emit.rs");

include!("codegen/prune.rs");

/// The byte views that live in their function's frame. `bytes s` of a
/// string writes a three-word header -- length, data, capacity -- and the
/// header is all it allocates, since the view borrows the string's bytes.
/// A function that scans a string makes one and reads it for a length, a
/// scan and some slices before it returns. A header in the arena
/// costs the bump and every read through it; a header in the frame is three
/// stores LLVM takes apart into registers.
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
        p.each_name(&mut |n| {
            out.insert(n);
        });
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

include!("codegen/ir_scan.rs");

include!("codegen/words.rs");

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

    /// Whether a bare call to `name` builds a record. An imported type enrolled
    /// under its bare name gives way to a group the program declares, as the
    /// interpreter's `callee` does: the author's own name wins.
    fn constructs(&self, name: &str) -> Option<i64> {
        let id = self.type_ids.get(name).copied()?;
        let enrolled = || self.program.types.iter().any(|t| t.name == name && t.origin.is_some());
        match self.declares(name) && enrolled() {
            true => None,
            false => Some(id),
        }
    }

    fn declares_at(&self, name: &str, arity: usize) -> bool {
        self.group_indices(name, arity).next().is_some()
    }

    /// The value of a constant whose one arm is an integer literal that fits a
    /// word. A mention of such a constant is that literal: building it does no
    /// work, so reading its cell, testing the cell's flag and checking the
    /// tag on every mention bought nothing. sha256's `whole` is read about a
    /// dozen times a round, and each read was five instructions and a branch.
    fn literal_word_constant(&self, name: &str) -> Option<i64> {
        let mut arms = self.group_indices(name, 0);
        let at = arms.next()?;
        if arms.next().is_some() {
            return None;
        }
        match self.program.fns[at].body.as_slice() {
            [Stmt::Expr(Expr::Int(n, _))] => i64::try_from(n).ok(),
            _ => None,
        }
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
    /// would pay the code twice for one test saved. A group the flatten pass
    /// made is the exception: it carries a record's fields in place of the
    /// record, positions among them, and is entered as often as the record
    /// was built. Boxed, those positions held jsonbench at 938M instructions;
    /// as words it read 864M, against 833M before the decoder took spans.
    fn has_twin(&self, name: &str, arity: usize) -> bool {
        // A twin buys run speed with a second copy of the group, which a dev
        // build and the tab pay for in compile time and memory and have no
        // use for: on 2026-10-06 the tab's compile of interp_corpus read
        // 24.7% more fuel and held 49% more memory with them.
        self.inline_helpers
            && arity > 0
            && !self.knotted.contains(name)
            && (self.cycle_members.contains(name) || crate::flat::made(name))
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
            Some(Expr::Str(_, span)) => self.builder_joins.contains(&(
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
        // value has to be built back into one. The three words carry the whole
        // record — `(field0.tag << 8 | field1.tag, field1.payload,
        // field0.payload)` — so
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
            let w2 = f.tmp();
            f.line_fmt(format_args!("{w2} = call i64 @k_parsed_int(%KValue {e})"));
            let p = parsed_of(f, &w0, &w1, &w2);
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
        if !f.assumes() {
            return;
        }
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
        // Every constant is frozen behind a cell that holds a `%KValue`, and
        // its symbol returns what the cell holds. The two-word record shared
        // a `%KValue`'s layout, so a constant returning one read back by
        // accident; three words do not.
        if arity == 0 {
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
        self.emit_cell_by(f, expr, "k_thunk_new")
    }

    /// `emit_cell` with the runtime entry that builds the cell named: the
    /// work-ahead pool's cells come from `k_thunk_new_pooled`.
    fn emit_cell_by(&mut self, f: &mut FnEmit, expr: &Expr, maker: &str) -> Result<String, String> {
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
            "{t} = call %KValue (i64, i32, ...) @{maker}(i64 {site}, i32 {}{args})",
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
        f.deep_from = outer.deep_from.clone();
        f.lifted = true;
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
            // a group called as a value enters its cycle from wherever the
            // value was called, so the wrapper records as a call from outside
            // does (see `crate::deep`)
            let deep = self.deep.entering(None, (name, arity)).map(|(decl, span)| {
                let d = &self.program.fns[decl];
                let hako = crate::provenance::package_of(&d.file);
                let spelled = crate::deep::spelled(self.program, (decl, span));
                self.intern(&format!("{hako}\0{spelled}\0")).0
            });
            if let Some(lit) = &deep {
                let _ = writeln!(conv, "  %deep = load ptr, ptr @k_deep_site");
                let _ = writeln!(conv, "  %deep_empty = icmp eq ptr %deep, null");
                let _ =
                    writeln!(conv, "  %deep_set = select i1 %deep_empty, ptr @{lit}, ptr %deep");
                let _ = writeln!(conv, "  store ptr %deep_set, ptr @k_deep_site");
            }
            let _ = writeln!(conv, "  %r = call tailcc %KValue @{sym}({})", call_args.join(", "));
            if deep.is_some() {
                let _ = writeln!(conv, "  store ptr %deep, ptr @k_deep_site");
            }
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
        // have been building any of them. A constant moved from a binding ahead
        // of the park is read again outside the try when it dies, so its
        // failure is raised here, where the program as written raised it.
        let mut pool = String::new();
        let mut shims = String::new();
        let lifted: Vec<(String, bool)> = self
            .program
            .fns
            .iter()
            .filter(|d| d.params.is_empty() && d.name.starts_with(crate::ahead::PREFIX))
            .filter(|d| crate::ahead::admits(d))
            .map(|d| (dsym(&d.name, 0), d.name.starts_with(crate::ahead::MOVED)))
            .collect();
        let forced: Vec<(&String, bool)> = lifted
            .iter()
            .filter(|(sym, _)| self.ahead_cells.iter().any(|(s, _)| s == sym))
            .map(|(sym, moved)| (sym, *moved))
            .collect();
        // A program whose functions pool cells at run time hands the runtime
        // the same table at every park, where it drains them.
        let pools = self
            .program
            .fns
            .iter()
            .any(|d| d.body.iter().any(|s| matches!(s, Stmt::Bind { pattern: Pattern::Var(n, _), .. } if n.as_str().starts_with(crate::ahead::POOLED))));
        let n = self.caf_cells.len();
        if !forced.is_empty() || pools {
            let pairs: Vec<String> =
                self.caf_cells.iter().map(|c| format!("ptr @{c}, ptr @{c}_ready")).collect();
            let _ = writeln!(
                shims,
                "@k_ahead_cells = internal constant [{} x ptr] [{}]",
                2 * n,
                pairs.join(", ")
            );
        }
        if !forced.is_empty() {
            let _ = writeln!(
                pool,
                "  %first_park = icmp ne i32 %first, 0\n  br i1 %first_park, label %constants, label %done\nconstants:"
            );
            for (i, (sym, moved)) in forced.iter().enumerate() {
                let _ = writeln!(
                    shims,
                    "define internal void @k_ahead_force_{i}() {{\nentry:\n  \
                     %v = call tailcc %KValue @{sym}()\n  ret void\n}}"
                );
                let died = if *moved { format!("again{i}") } else { "done".to_string() };
                let _ = writeln!(
                    pool,
                    "  %go{i} = call i32 @k_ahead_try(ptr @k_ahead_force_{i}, ptr @k_ahead_cells, i64 {n})\n  \
                     %on{i} = icmp ne i32 %go{i}, 0\n  \
                     br i1 %on{i}, label %next{i}, label %{died}"
                );
                if *moved {
                    let _ = writeln!(
                        pool,
                        "again{i}:\n  call void @k_ahead_force_{i}()\n  br label %done"
                    );
                }
                let _ = writeln!(pool, "next{i}:");
            }
            let _ = writeln!(shims, "declare i32 @k_ahead_try(ptr, ptr, i64)");
            let _ = writeln!(pool, "  br label %done\ndone:");
        }
        if pools {
            let _ = writeln!(shims, "declare void @k_ahead_drain(ptr, i64)");
            let _ = writeln!(pool, "  call void @k_ahead_drain(ptr @k_ahead_cells, i64 {n})");
        }
        self.room(shims.len() + pool.len() + 128);
        self.body.push_str(&shims);
        let _ = writeln!(
            self.body,
            "define void @k_ahead_pool(i32 %first) {{\nentry:\n{pool}  ret void\n}}\n"
        );
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
            sites: std::mem::take(&mut self.sites),
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

    /// Whether a call of this group can match no arm and stop the program.
    /// An arm whose every parameter is a plain name or `_` takes anything,
    /// and a group with one never falls through, so its calls need no stamp.
    /// Erring the other way costs a store, never a wrong place.
    fn can_miss(&self, name: &str, arity: usize) -> bool {
        !self.decls_named(name).any(|d| {
            d.params.len() == arity
                && d.params.iter().all(|p| matches!(p, Pattern::Var(..) | Pattern::Wildcard(_)))
        })
    }

    /// `can_miss`, narrowed by what the call's arguments are proven to be.
    /// The call always reaches an arm when every kind each argument may hold
    /// is taken, position by position, by arms that also take the rest: a
    /// plain name takes every kind, an annotation the dispatcher tests by kind
    /// alone (`int`, `float64`, `string`, `bool`, `none`, `done`, `some`, a
    /// list or a map) takes those kinds, and `true`, `false`, `none` and
    /// `done` take themselves. So `f x true` and
    /// `f x false` together take a proven boolean. A failure is handed on
    /// rather than matched, so it never misses.
    fn can_miss_with(&self, f: &FnEmit, name: &str, arity: usize, emitted: &[String]) -> bool {
        if !self.can_miss(name, arity) {
            return false;
        }
        // a declared subtype is tested by its chain, not by its kind alone
        if !self.sub_parents.is_empty() {
            return true;
        }
        let arms: Vec<&[Pattern]> = self
            .decls_named(name)
            .filter(|d| d.params.len() == arity)
            .map(|d| &d.params[..])
            .collect();
        let sets: Vec<Set> = emitted.iter().map(|e| f.set_of(e)).collect();
        let records: Vec<Option<&str>> =
            emitted.iter().map(|e| f.parsed.get(e).map(|(ty, _)| ty.as_str())).collect();
        let mut budget = 256;
        !covered(&arms, &sets, &records, &mut budget)
    }

    /// The literal an insisting form's answer is pointed at: the form's own
    /// package, which a failure it raised names, and then this call's line
    /// in the frame of the caller.
    fn insisted_site(&mut self, f: &FnEmit, name: &str, arity: usize, span: Span) -> String {
        let file = self
            .decls_named(name)
            .find(|d| d.params.len() == arity)
            .map(|d| d.file.clone())
            .unwrap_or_default();
        let hako = crate::provenance::package_of(&file);
        let (lit, _) = self.intern(&format!("{hako}\0{}:{}\0", f.origin_prefix, span.line));
        format!("ptr @{lit}")
    }

    /// Records this site as the place a runtime death names: the same
    /// literal an err born here would carry, stored where `k_died` reads it
    /// (ruled on #1975). One store, before a call that can stop the program.
    fn stamp_site(&mut self, f: &mut FnEmit, span: Span) {
        let site = self.origin_arg(f, span);
        self.stamp(f, &site);
    }

    /// Stores `site`, an interned `ptr @name`, as the place a death names.
    /// The store is its id, an immediate, where the literal's address would
    /// be an address to materialise first; `k_site_name` maps it back.
    fn stamp(&mut self, f: &mut FnEmit, site: &str) {
        let index: usize = site
            .strip_prefix("ptr @s")
            .and_then(|n| n.parse().ok())
            .expect("a site is an interned literal");
        if self.site_ids.len() <= index {
            self.site_ids.resize(index + 1, 0);
        }
        if self.site_ids[index] == 0 {
            self.sites.push(index as u32);
            self.site_ids[index] = self.sites.len() as u32;
        }
        let id = self.site_ids[index];
        f.line_fmt(format_args!("store i32 {id}, ptr @k_site_id"));
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
include!("codegen/calls.rs");

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
        f.deep_from = outer.deep_from.clone();
        f.lifted = true;
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

include!("codegen/module.rs");

include!("codegen/narrow.rs");

include!("codegen/wasm32.rs");

/// How much of the emitted body a piece holds before the next is opened.
const BODY_PIECE: usize = 64 * 1024;

/// A qualified name, or one carrying a naming sigil, needs LLVM's quoted-
/// identifier form. One list, because a second copy drifts and what it costs
/// is a module clang refuses to read.
fn quoted(name: &str) -> String {
    match name.contains(['/', '!', '?', '+', '-', '*', '%', '<', '>', '=', '~']) {
        true => format!("\"{name}\""),
        false => name.to_string(),
    }
}

#[cfg(test)]
mod the_emitter_group_index_answers_what_the_scan_answered;

#[cfg(test)]
mod a_group_the_walk_misses_is_put_back;
