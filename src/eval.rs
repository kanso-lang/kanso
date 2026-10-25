use crate::ast::*;
use crate::diag::{article, Span};
use crate::hash::{Map, Set};
use crate::int::Int;
use crate::name::Name;
use num_bigint::BigInt;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Int(Int),
    Str(Text),
}

/// A map's entries, in key order.
///
/// A B-tree leaf holds room for eleven entries whatever it is given, and most
/// maps a program builds are records read from JSON with a handful of keys.
/// The interpreted corpus decodes 220 maps of four keys each, and their leaves
/// were 158,400 bytes of the interpreter's peak. A map of up to eight entries
/// is a vector kept sorted, which holds what it holds; past eight it becomes a
/// B-tree, so a large map built one key at a time still inserts in log time.
/// Both walk in key order, which is all equality and rendering ask of them.
#[derive(Clone, Debug)]
pub enum Entries {
    Few(Vec<(MapKey, Value)>),
    Many(BTreeMap<MapKey, Value>),
}

/// A large map, held weakly, beside its entries in order (`Interp::entries_at`).
type EntriesAt = (std::rc::Weak<Entries>, Rc<[(MapKey, Value)]>);

/// The most entries a map keeps as a sorted vector.
const FEW_ENTRIES: usize = 8;

impl Default for Entries {
    fn default() -> Self {
        Entries::Few(Vec::new())
    }
}

impl Entries {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        match self {
            Entries::Few(few) => few.len(),
            Entries::Many(many) => many.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: &MapKey) -> Option<&Value> {
        match self {
            Entries::Few(few) => {
                few.binary_search_by(|(k, _)| k.cmp(key)).ok().map(|at| &few[at].1)
            }
            Entries::Many(many) => many.get(key),
        }
    }

    /// Sets `key` to `value`, replacing what it held.
    pub fn insert(&mut self, key: MapKey, value: Value) {
        match self {
            Entries::Few(few) => match few.binary_search_by(|(k, _)| k.cmp(&key)) {
                Ok(at) => few[at].1 = value,
                Err(at) if few.len() < FEW_ENTRIES => few.insert(at, (key, value)),
                Err(_) => {
                    let mut many: BTreeMap<MapKey, Value> =
                        std::mem::take(few).into_iter().collect();
                    many.insert(key, value);
                    *self = Entries::Many(many);
                }
            },
            Entries::Many(many) => {
                many.insert(key, value);
            }
        }
    }

    /// Drops `key` and what it held; a key that is not there changes nothing.
    pub fn remove(&mut self, key: &MapKey) {
        match self {
            Entries::Few(few) => {
                if let Ok(at) = few.binary_search_by(|(k, _)| k.cmp(key)) {
                    few.remove(at);
                }
            }
            Entries::Many(many) => {
                many.remove(key);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MapKey, &Value)> {
        let (few, many) = match self {
            Entries::Few(few) => (Some(few.iter().map(|(k, v)| (k, v))), None),
            Entries::Many(many) => (None, Some(many.iter())),
        };
        few.into_iter().flatten().chain(many.into_iter().flatten())
    }
}

impl Value {
    /// An integer value, kept in a machine word whenever it fits one.
    pub fn int(n: impl Into<Int>) -> Value {
        Value::Int(n.into())
    }
}

/// The longest string a `Text` holds without a heap allocation. Fifteen bytes
/// and a length fit beside the `String` that a longer one is, so a `Text` is
/// the size of a `String` and `Value` stays at 32 bytes. At 22 the inline form
/// no longer fit, `Value` grew to 40 bytes, and every move and drop of a value
/// paid for it: the interpreted corpus ran 13 million instructions slower.
const TEXT_INLINE: usize = 15;

include!("eval/text.rs");

#[derive(Clone, Debug)]
pub enum Value {
    Int(Int),
    Float(f64),
    Map(Rc<Entries>),
    Str(Text),
    True,
    False,
    NoneV,
    /// What a succeeded effect yields. `none` is absence and nothing else,
    /// so a print, a write or a sleep that finished answers this instead.
    Done,
    ErrV(Rc<ErrInfo>),
    List(Rc<Vec<Value>>),
    /// Byte data, as `text/bytes` and the scanner produce it. Distinct from
    /// a list of small ints on every engine: native has had the tag since it
    /// was written, and a list that happens to hold numbers in 0..255 is not
    /// silently one of these.
    Bytes(Rc<Vec<u8>>),
    Record {
        ty: Rc<str>,
        fields: Rc<RefCell<Vec<Value>>>,
    },
    /// A nominal subtype wrapper: `post_body s`. Transparent — every
    /// consumer unwraps to the base; dispatch sees the chain.
    Sub {
        ty: Rc<str>,
        inner: Rc<Value>,
    },
    FnRef(Rc<str>),
    Closure(Rc<ClosureData>),
    /// A closure the browser backend compiled into the module's table,
    /// named by its slot handle. The native engine tags closures inside its
    /// value union; this is the same freedom, spelled for the wasm registry,
    /// so a closure can sit in a record field or a list like any other value.
    TableFn(u32),
    /// `&f a` — a callee and the arguments supplied so far. Applying it
    /// appends; dispatch fires when the count reaches an arity the callee
    /// answers to, which is why the arity need not be named at the `&`.
    Partial(Rc<Value>, Rc<Vec<Value>>),
    Desc(Rc<Desc>),
    Thunk(Rc<RefCell<ThunkState>>),
}

/// A conditionally-demanded binding (demand.rs marks the sites): the
/// computation and its captures, replaced by the value at first force.
#[derive(Clone, Debug)]
pub enum ThunkState {
    Pending {
        expr: Expr,
        env: Option<Rc<Env>>,
        frame: Frame,
    },
    /// Under evaluation — a re-entrant force is a <<loop>>.
    Blackhole,
    Forced(Value),
}

/// A `list/tie` call while its maker runs: the cell each key's node will
/// fill, made before the first node, and the first link the maker asked for
/// that no key will make.
struct Tie {
    serial: i64,
    cells: Entries,
    broken: Option<(Value, Value)>,
}

include!("eval/errors.rs");

#[derive(Debug)]
pub struct ClosureData {
    pub params: Vec<Name>,
    pub body: Expr,
    pub env: Option<Rc<Env>>,
    pub frame: Frame,
    /// The cycle of the body that made it, for `crate::deep`: a lambda's calls
    /// are its maker's, as its compiled body is.
    pub deep: u32,
}

#[derive(Clone, Debug)]
pub enum Desc {
    Print(String, Span),
    Join(Rc<Desc>, Rc<Desc>),
    Args,
    Stdin,
    /// The next line of standard input without its line ending, or `done`
    /// at the end of it (ruled 2026-10-13, kanso#1914).
    ReadLine,
    ReadFile(String),
    /// the same read with no test of what the bytes are (ruled 2026-08-29)
    ReadBytes(String),
    Write(String),
    WriteErr(String),
    Env(String),
    Exists(String),
    IsDir(String),
    /// A file's last-modified time in milliseconds since the epoch, or `none`
    /// for a file that is not there, which `os/modified` names
    /// `file_not_found` (ruled 2026-10-13, kanso#1914).
    Modified(String),
    ListDir(String),
    Now,
    MakeDir(String),
    WriteFile(String, String),
    /// what read_bytes handed back, written as it was
    WriteBytes(String, Rc<Vec<u8>>),
    Run(String, Vec<String>),
    Start(String, Vec<String>),
    Kill(i64),
    /// The three worded chain steps of the 2026-08-26 gavel. Each runs its
    /// subject and then decides which channel the callback sees: `bind` hands
    /// over the value and lets a failure past untouched, `rescue` hands over
    /// the err and lets a value past. `annotate` reads the same channel as
    /// `rescue` and re-wraps whatever comes back, so it can say more about a
    /// failure and can never clear one. `rescue` and `annotate` carry the site
    /// they were written at: `annotate` because the err it builds is a raise,
    /// and a raise records where it happened; `rescue` because its licence is
    /// foreign-only, and the site's package is what the failure's raiser is
    /// compared against.
    Bind(Rc<Desc>, Value),
    Rescue(Rc<Desc>, Value, Raised),
    Annotate(Rc<Desc>, Value, Raised),
    Await(i64),
    Listen(i64),
    SocketPort(i64),
    Accept(i64),
    Receive(i64),
    Send(i64, String),
    SendBytes(i64, Rc<Vec<u8>>),
    CloseSocket(i64),
    Sleep(u64),
    Random(u64),
    Nil,
    /// A box whose outcome is already known: the value, or the err, that
    /// `effect v` was handed. The box is only ever explicit (ruled
    /// 2026-09-15), and this is the hand-applied one; nothing about it is
    /// deferred, so it holds its answer from the moment it is built and
    /// running it hands the answer over.
    Settled(Value),
    /// What an insisting form handed back, run where it was called: a
    /// failure the form's own package raised is re-pointed at the caller's
    /// line, the place an `!` names (ruled on #1975). The second field is
    /// that line, the third the form's package.
    Insisted(Rc<Desc>, Rc<str>, Rc<str>),
}

/// A read that found nothing answers `none`, the shape `env` already uses for
/// a variable that is not set: the std wrapper turns it into `file_not_found`.
///
/// It was a two-element list once, `[there, text]`, and the list cost the
/// program the string's type. Inference gives a list index the top set, so a
/// document read this way arrived at its consumer untyped — and `beat.rs`
/// reads that set to decide whether a loop may rewind. jsonbench's decode
/// loop stopped rewinding on the day the read changed shape: arena blocks 2
/// to 248, peak 2 MB to 260 MB, on the same bytes.
fn read_value(found: Option<String>) -> Value {
    match found {
        Some(text) => Value::Str(Text::from(text)),
        None => Value::NoneV,
    }
}

/// What a finished process answers: its status and the two streams.
fn ran_value(done: (i64, String, String)) -> Value {
    let (status, out, errs) = done;
    Value::List(Rc::new(vec![
        Value::int(status),
        Value::Str(Text::from(out)),
        Value::Str(Text::from(errs)),
    ]))
}

/// SplitMix64: a deterministic, seedable generator. A real run draws its
/// seed from entropy so dice roll differently each time; KANSO_SEED pins
/// the stream, which is how the differential lattice, the goldens, and any
/// replay of a concurrent program stay byte-identical across engines.
pub struct Rng(u64);

/// A pinned clock, so a run that timestamps can be replayed exactly. Every
/// engine reads the same variable, which is what keeps them agreeing.
pub fn pinned_now() -> Option<i64> {
    std::env::var("KANSO_NOW").ok().and_then(|s| s.parse::<i64>().ok())
}

impl Rng {
    pub fn seeded() -> Self {
        let seed = std::env::var("KANSO_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(entropy_seed);
        Rng(seed)
    }

    pub fn from_seed(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        match n {
            0 => 0,
            n => self.next_u64() % n,
        }
    }
}

/// The browser has no clock at thread-local init and reseeds through
/// `kanso_set_seed` before every run, so any fixed value serves there.
#[cfg(target_arch = "wasm32")]
fn entropy_seed() -> u64 {
    0x2545_F491_4F6C_DD1D
}

#[cfg(not(target_arch = "wasm32"))]
fn entropy_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x2545_F491_4F6C_DD1D);
    nanos ^ (u64::from(std::process::id()) << 32)
}

/// One step of a fiber: it either finished with a value, or blocked on a
/// `sleep` for `ms` with the rest of its work as the continuation. Blocking
/// propagates up through `Bind`, so `sleep` may sit anywhere in a
/// description and suspension needs no coroutine — the continuation is the
/// remaining Desc tree, made explicit.
enum Step {
    Done(Value),
    Blocked(u64, Rc<Desc>),
}

include!("eval/env.rs");

pub struct RuntimeError {
    pub message: String,
    pub span: Span,
    /// Where it happened, as "{fn} at {file}:{line}", the trace line an err
    /// born there would carry. The innermost expression the failure passes
    /// out of fills it in, since that expression's frame is the one whose
    /// file the span is in (ruled on #1975).
    pub at: Option<Rc<str>>,
}

impl RuntimeError {
    /// The line a report prints under the message, when the failure has a
    /// place: `  failed in {fn} at {file}:{line}`.
    pub fn place(&self) -> String {
        match &self.at {
            Some(at) => format!("  failed in {at}\n"),
            None => String::new(),
        }
    }
}

type Bindings = Vec<(Name, Value)>;

/// How many parameters' ranks a `Score` holds without allocating.
const SCORE_INLINE: usize = 12;

/// One rank a parameter, in order. It was a `Vec<u8>`, and every dispatch
/// allocated one and freed it, and a candidate that won first left the next
/// one to grow a fresh vector from nothing. The ranks of a call with more
/// parameters than `SCORE_INLINE` go on in `spill`.
#[derive(Default)]
struct Score {
    len: usize,
    inline: [u8; SCORE_INLINE],
    spill: Vec<u8>,
}

impl Score {
    fn clear(&mut self) {
        self.len = 0;
        self.spill.clear();
    }

    fn push(&mut self, rank: u8) {
        match self.len < SCORE_INLINE {
            true => self.inline[self.len] = rank,
            false => self.spill.push(rank),
        }
        self.len += 1;
    }

    /// Compares as the ranks in order would, as a `Vec<u8>` compared. The two
    /// scores a dispatch compares always have one rank per argument, so their
    /// inline parts are the same length.
    fn beats(&self, other: &Score) -> bool {
        let n = self.len.min(SCORE_INLINE);
        let m = other.len.min(SCORE_INLINE);
        (&self.inline[..n], &self.spill) > (&other.inline[..m], &other.spill)
    }
}

type EvalResult = Result<Value, RuntimeError>;

/// Calling a closure that lives in another engine's table, by handle.
/// Reaching a closure that belongs to another engine. The flag says which
/// call rule to use: `false` guards the arguments against failure the way
/// ordinary application does, `true` is the decided call the worded chain
/// steps need — see `call_decided`.
pub type ForeignCall = fn(u32, Vec<Value>, bool) -> EvalResult;

/// What the browser answers when render reaches one of its cells. `None`
/// where the handle names an ordinary function, which renders as one.
pub type DeferralResolver = fn(u32) -> Option<Value>;

thread_local! {
    /// The browser backend compiles closures into a wasm table the
    /// interpreter cannot reach on its own. It registers the way back in
    /// here, so a description carrying such a continuation runs on the same
    /// scheduler as every other description instead of needing a second one.
    static FOREIGN_CALL: std::cell::RefCell<Option<ForeignCall>> =
        const { std::cell::RefCell::new(None) };
    static DEFERRAL_RESOLVER: std::cell::RefCell<Option<DeferralResolver>> =
        const { std::cell::RefCell::new(None) };
}

/// What a body's tail handed back: a finished value, or a call for the
/// dispatcher to loop on instead of growing the stack. A call to a function
/// value is one too: a loop written in continuation-passing style hands
/// itself back through a lambda every step, and calling that lambda as a
/// nested call ran the toml port out of stack at 10,000 steps.
/// Where the call a dispatch is working on was written, for a failure to
/// name (ruled on #1975).
enum Caller<'a> {
    Outside,
    Decl(&'a FnDecl),
    Lambda(Rc<ClosureData>),
}

enum Flow<'a> {
    Done(Value),
    Tail(Rc<str>, Rc<Vec<&'a FnDecl>>, Vec<Value>, Span),
    Closure(Rc<ClosureData>, Vec<Value>, Span),
}

thread_local! {
    /// The root module's name and the bare types it declares, for a record's
    /// rendered spelling. RULED 2026-08-29, "records print qualified,
    /// everywhere": a record prints `{module}/{type}` whatever the entry
    /// path, so the root's own types take the root's name, the one an
    /// importer would write for it. The set is the root's declarations and
    /// nothing else: the compiler's own `entry` record, which no module
    /// declares, stays bare on every engine. Set when an interpreter is built
    /// over a program, which is how the page's host learns it too.
    static ROOT: std::cell::RefCell<(String, Set<String>)> =
        std::cell::RefCell::new((String::new(), Set::default()));
}

pub fn set_root(program: &Program) {
    let own = program
        .types
        .iter()
        .filter(|t| !crate::ast::has_slash(&t.name))
        .map(|t| t.name.clone())
        .collect();
    ROOT.with(|r| *r.borrow_mut() = (program.root.clone(), own));
}

/// A type as a rendered record spells it: qualified by the root's name when
/// the declaration was the root's own and carries no module of its own.
pub fn shown_type(ty: &str) -> String {
    ROOT.with(|r| {
        let (root, own) = &*r.borrow();
        match !root.is_empty() && own.contains(ty) {
            true => format!("{root}/{ty}"),
            false => ty.to_string(),
        }
    })
}

pub fn set_foreign_call(call: ForeignCall) {
    FOREIGN_CALL.with(|slot| *slot.borrow_mut() = Some(call));
}

pub fn set_deferral_resolver(resolve: DeferralResolver) {
    DEFERRAL_RESOLVER.with(|slot| *slot.borrow_mut() = Some(resolve));
}

/// What a table handle has settled on, where it names a cell rather than a
/// function. Nothing answers in the interpreter, whose own cells are thunks.
fn deferral_value(handle: u32) -> Option<Value> {
    let resolve = DEFERRAL_RESOLVER.with(|slot| *slot.borrow())?;
    resolve(handle)
}

/// A handle stands in the render path beside the pointers, and a pointer is
/// aligned, so the low bit tells the two apart.
fn deferral_key(handle: u32) -> usize {
    ((handle as usize) << 1) | 1
}

fn foreign_call(handle: u32, args: Vec<Value>, span: Span, decided: bool) -> EvalResult {
    match FOREIGN_CALL.with(|slot| *slot.borrow()) {
        Some(call) => call(handle, args, decided),
        None => Err(RuntimeError {
            at: None,
            message: "a compiled closure escaped the engine that owns it".to_string(),
            span,
        }),
    }
}

include!("eval/executor.rs");

/// The slot a key takes in the recent tables: the top bits of the key times
/// the golden ratio, which spreads keys that differ only in their low bits.
///
/// The key is an object's place inside its allocator slice, never its whole
/// address. Until 2026-09-30 it was the whole address, and where the heap
/// landed decided which declarations and names shared a slot, so the
/// interpreter's instruction count moved with the memory layout: a valgrind
/// run with its mappings started at 48 MiB counted 443,467 fewer instructions
/// than one started at the default, and one started at 256 MiB counted
/// 522,700 more, on the same binary and the same program.
fn recent_slot(key: u64, slots: usize) -> usize {
    (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - slots.trailing_zeros())) as usize
}

/// A map key that is equal by address and hashed by the object's place in its
/// slice, so which entries collide depends on the program and not on where
/// the heap landed. Two keys at one address are one object, so they carry one
/// `stable`.
#[derive(Clone, Copy)]
struct Placed {
    at: usize,
    stable: u64,
}

impl PartialEq for Placed {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at
    }
}

impl Eq for Placed {}

impl std::hash::Hash for Placed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.stable);
    }
}

/// Where an object sits inside the 64 KiB slice the allocator placed it in.
/// The allocator hands out slices on 64 KiB boundaries, so moving the whole
/// heap moves no object within its slice, and the offset is the same on every
/// layout the same program produces.
fn in_slice(at: usize) -> u64 {
    (at & 0xFFFF) as u64
}

/// Slots in `Interp::recent_callees`. The table lives for the whole run, so
/// its size sits under the interpreted run's peak memory. At 1,024 slots,
/// with the frames' 512 beside it, the two held 32,768 bytes; at 256 each
/// they hold 10,240, and the corpus misses often enough to cost about 720,000
/// more instructions, which the objective weighs as the smaller of the two.
const RECENT_CALLEES: usize = 256;

/// Slots in `Interp::recent_frames`, sized with the callees above.
const RECENT_FRAMES: usize = 256;

/// What a CALL on a name does. `call_named` asked `err`, then the types map,
/// then the function map, on every call; `group_of` inside `eval_tail` probed
/// the same two maps to decide one bit. Neither map changes during a run, so
/// the answer is remembered the first time it is worked out -- the same
/// bargain `Named` makes for what a name is WORTH, at the sites that ask what
/// it DOES.
///
/// Two memories rather than one because the rows would not fit in one: a value
/// needs the name as an `Rc<str>` and a call needs the group, and an arm
/// carrying both would size every row of both tables by the pair.
#[derive(Clone)]
enum Callee<'a> {
    Err,
    /// `entry` names a declaration the interpreter owns rather than one the
    /// program holds, so it cannot be stored as a borrow of the program and
    /// gets an arm of its own.
    EntryType,
    Constructor(&'a TypeDecl),
    Group(Rc<Vec<&'a FnDecl>>),
    Builtin,
}

/// What a name that is not a local stands for. `eval_ident` used to answer
/// this by walking a ladder -- `fns`, the three descriptor names, `types`,
/// then `fns` and `types` a second time and a sixty-name array -- and a
/// program asks it about the same name on every evaluation, once per loop
/// iteration on the same node. The answer depends on the program alone, so it
/// is remembered the first time it is worked out.
///
/// A constant is `Constant` rather than its value: `knotted` evaluates, and
/// what it returns is a value rather than a resolution. A field-less record is
/// `EmptyRecord` for the same reason -- its fields are a fresh `RefCell` on
/// every mention.
///
/// Every arm is a name or a pointer, and the four descriptors and four
/// literals are arms of their own rather than a `Desc` or a `Value`: this
/// lives in a table with one row per name the run mentions, and inlining
/// either enum would size every row by its largest variant.
#[derive(Clone)]
enum Named<'a> {
    Constant(&'a FnDecl),
    Args,
    Stdin,
    ReadLine,
    Now,
    EmptyRecord(Rc<str>),
    True,
    False,
    NoneV,
    Done,
    FnRef(Rc<str>),
}

impl Named<'_> {
    /// Whether two resolutions are the same one, for the debug check on a
    /// global's slot: the same variant, and for a constant the same
    /// declaration, for a reference or a record the same text.
    fn same_as(&self, other: &Named<'_>) -> bool {
        match (self, other) {
            (Named::Constant(a), Named::Constant(b)) => std::ptr::eq(*a, *b),
            (Named::EmptyRecord(a), Named::EmptyRecord(b)) => a == b,
            (Named::FnRef(a), Named::FnRef(b)) => a == b,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
}

pub struct Interp<'a> {
    fns: Map<&'a str, Rc<Vec<&'a FnDecl>>>,
    types: Map<&'a str, &'a TypeDecl>,
    entry_decl: TypeDecl,
    demand: crate::demand::DemandInfo<'a>,
    pub thunk_stats: ThunkStats,
    depth: Cell<usize>,
    /// For each declaration, one more than the number of the cycle that grows
    /// the stack it is on, or 0; and each cycle's place (see `crate::deep`).
    deep_of: Vec<u32>,
    deep_places: Vec<Rc<str>>,
    /// The record an overflow names, one more than a cycle's number or 0, and
    /// the cycle of the body running now, 0 in a lambda's body or outside.
    deep_now: Cell<u32>,
    running: Cell<u32>,
    /// One cell per self-referential constant. A mention that arrives while
    /// the constant is still being computed gets the unforced cell, which is
    /// how a value that names itself gets a value at all.
    knots: RefCell<Map<String, Rc<RefCell<ThunkState>>>>,
    /// One entry per `list/tie` call still making its nodes, innermost last.
    /// A `ref` the maker holds names its call by serial, never by position: a
    /// lambda the maker stored can call `ref` after its tie has returned, and
    /// by then the position may hold a later tie.
    ties: RefCell<Vec<Tie>>,
    tie_serial: Cell<i64>,
    /// Bindings buffers a dispatch finished with, kept for the next one. A
    /// dispatch that tries a second candidate after the first matched needs a
    /// buffer beside the winner's, and it was allocated for that dispatch and
    /// freed at its end.
    spare_binds: RefCell<Vec<Bindings>>,
    /// The entries of the last large map `key_at` or `value_at` read, in
    /// order. A map past eight entries is a B-tree, where reaching the i-th
    /// entry walks i of them, so reading every position of one would be
    /// quadratic; lib/json's encoder reads every position of every map.
    ///
    /// The map is held weakly, so holding it changes no count that `put`
    /// asks before writing in place, and its address cannot be reused while
    /// the cache names it. A put that writes in place regardless, at a site
    /// the linearity analysis proved unique, empties the cache first.
    entries_at: RefCell<Option<EntriesAt>>,
    /// Where the last slice of a string with a wider character ended: the
    /// string, a character's position and the byte it starts at. A scan asks
    /// for each next character of one string in turn, so the walk to it goes
    /// on from here; see `char_offset`.
    text_cursor: RefCell<Option<(Text, usize, usize)>>,
    /// The constants that reach themselves through a chain of mentions --
    /// the same set the emitter computes, so the two engines count the same
    /// cells. Every constant goes through `knotted`, but only these are the
    /// ones native builds a thunk for, and the counters are semantics.
    ///
    /// Computed on the first constant this run builds a cell for rather than
    /// at construction, because `kanso check` makes an `Interp` and never
    /// evaluates a constant, and that route is a weighed welfare term.
    cycles: std::cell::OnceCell<crate::hash::Set<String>>,
    /// The push, put and append sites the linearity analysis proves extend a
    /// container nothing else will read -- the same set, by the same key,
    /// that the emitter consults before it writes `push_mut_fast`. The
    /// compiled engine has mutated at these sites since kanso#1359; until now
    /// the interpreter cloned at every one of them.
    ///
    /// A `OnceCell` for the reason `cycles` above is one: `kanso check` makes
    /// an `Interp` and evaluates nothing, and that route is a weighed welfare
    /// term. The analysis is not cheap and must not run for a check.
    in_place: std::cell::OnceCell<crate::hash::Set<(std::sync::Arc<str>, usize, usize)>>,
    /// `linear::moved_writes`, built the first time a write's value has no
    /// holder but the name that moves it. Lazy for the reason `in_place` is.
    moved: std::cell::OnceCell<crate::hash::Set<(std::sync::Arc<str>, usize, usize)>>,
    /// The joins of `linear::string_builders`, built the first time a template
    /// opens with a name. Lazy for the reason `in_place` is.
    joins: std::cell::OnceCell<crate::hash::Set<(std::sync::Arc<str>, usize, usize)>>,
    /// The one whole-program analysis `in_place` and `joins` are read from.
    linearity: crate::linear::ForTheInterpreter<'a>,
    /// One entry per non-local name this run has evaluated. Filled on first
    /// sight rather than at construction, because `kanso check` makes an
    /// `Interp` and evaluates nothing, and that route is a weighed welfare
    /// term -- the same reason `cycles` above is a `OnceCell`.
    names: RefCell<Map<String, Named<'a>>>,
    /// What a global resolved to, by the slot its node was stamped with.
    /// Filled on first sight, for the reason `names` above is.
    slots: RefCell<Vec<Named<'a>>>,
    /// Beside each slot in `slots`, the value its name always reads as, when
    /// that value cannot change: a function reference or a literal word. A
    /// constant has none, because reading one runs it the first time.
    fixed: RefCell<Vec<Option<Value>>>,
    /// This interpreter's stamp on the nodes it resolves; see `Resolution`.
    /// 0 when the process has made more interpreters than sixteen bits hold,
    /// which turns stamping off rather than letting a stamp be reused.
    generation: u16,
    /// One entry per name this run has CALLED, filled on first sight for the
    /// reason `names` above is.
    callees: RefCell<Map<String, Callee<'a>>>,
    /// `callees` again, keyed by the address of a `Value::FnRef`'s name; see
    /// `call_ref`.
    callees_by_ref: RefCell<Map<Placed, (Rc<str>, Callee<'a>)>>,
    /// The last callee each of a few hundred reference addresses asked for,
    /// in front of `callees_by_ref`; see `callee_of_ref`.
    recent_callees: RefCell<Vec<(usize, Option<Callee<'a>>)>>,
    /// One entry per declaration this run has ENTERED, keyed by the
    /// declaration's address, which `&'a FnDecl` on `frame_for` is what makes
    /// safe: the compiler refuses a borrow that does not outlive this
    /// interpreter, so no temporary can land here and reuse a freed address.
    /// `frame_of` formats a trace line and asks which
    /// package a file belongs to, on every entry into every body, for a string
    /// only read when an err is raised. The answer depends on the declaration
    /// alone, and a declaration lives in the `Program` this interpreter
    /// borrows, so its address is stable for the whole run and unique to it.
    frames: RefCell<Map<Placed, Frame>>,
    /// The last frame each of a few hundred declaration slots asked for, in
    /// front of `frames`; see `frame_for`.
    recent_frames: RefCell<Vec<(usize, Frame)>>,
    program: &'a Program,
}

/// Engine-shared semantic counters: evaluation counts are semantics, so
/// native must report byte-identical values (design/lazy-v1-plan.md).
#[derive(Default)]
pub struct ThunkStats {
    pub allocs: Cell<u64>,
    pub forces: Cell<u64>,
    pub evals: Cell<u64>,
}

impl ThunkStats {
    pub fn render(&self) -> String {
        let (allocs, forces, evals) = (self.allocs.get(), self.forces.get(), self.evals.get());
        format!("thunk_allocs={allocs}\nthunk_forces={forces}\nthunk_evals={evals}\n")
    }
}

impl<'a> Interp<'a> {
    pub fn new(program: &'a Program) -> Self {
        set_root(program);
        let mut groups: Map<&str, Vec<&FnDecl>> = Map::default();
        for decl in &program.fns {
            groups.entry(&decl.name).or_default().push(decl);
        }
        // proximity breaks specificity ties: local arms come before
        // bare-enrolled clones, so a same-shape local wins its own file
        for overloads in groups.values_mut() {
            overloads.sort_by_key(|d| d.synthetic);
        }
        // A group is shared rather than copied: the callee memory below holds
        // one without borrowing from `self`, and a tail hop takes a refcount
        // bump where it used to clone the whole vector.
        let fns: Map<&str, Rc<Vec<&FnDecl>>> =
            groups.into_iter().map(|(name, decls)| (name, Rc::new(decls))).collect();
        let types = program.types.iter().map(|t| (t.name.as_str(), t)).collect();
        TYPESETS.with(|reg| {
            *reg.borrow_mut() = program
                .types
                .iter()
                .filter(|t| !t.members.is_empty())
                .map(|t| (t.name.clone(), t.members.clone()))
                .collect();
        });
        let origin = Span::at(0, 0);
        let entry_decl = TypeDecl {
            parent: None,
            members: Vec::new(),
            name: "entry".to_string(),
            is_pub: false,
            span: origin,
            synthetic: false,
            origin: None,
            fields: vec![
                ("key".to_string(), vec!["some".to_string()], origin),
                ("value".to_string(), vec!["some".to_string()], origin),
            ],
            file: crate::ast::unstamped(),
        };
        let demand = crate::demand::analyze(program);
        let deep = crate::deep::Deep::of(program);
        Interp {
            fns,
            types,
            entry_decl,
            demand,
            thunk_stats: ThunkStats::default(),
            depth: Cell::new(0),
            deep_of: deep.by_decl(program),
            deep_places: deep
                .places()
                .iter()
                .map(|&place| Rc::from(crate::deep::spelled(program, place)))
                .collect(),
            deep_now: Cell::new(0),
            running: Cell::new(0),
            knots: RefCell::new(Map::default()),
            ties: RefCell::new(Vec::new()),
            tie_serial: Cell::new(0),
            spare_binds: RefCell::new(Vec::new()),
            entries_at: RefCell::new(None),
            text_cursor: RefCell::new(None),
            cycles: std::cell::OnceCell::new(),
            in_place: std::cell::OnceCell::new(),
            moved: std::cell::OnceCell::new(),
            joins: std::cell::OnceCell::new(),
            linearity: crate::linear::ForTheInterpreter::new(program),
            names: RefCell::new(Map::default()),
            slots: RefCell::new(Vec::new()),
            fixed: RefCell::new(Vec::new()),
            generation: next_generation(),
            callees: RefCell::new(Map::default()),
            callees_by_ref: RefCell::new(Map::default()),
            recent_callees: RefCell::new(vec![(0, None); RECENT_CALLEES]),
            frames: RefCell::new(Map::default()),
            recent_frames: RefCell::new(vec![(0, None); RECENT_FRAMES]),
            program,
        }
    }

    /// `frame_of` for a declaration this run has entered before, which after
    /// the first entry is a clone of an `Rc` rather than two formatted strings.
    ///
    /// A direct-mapped table of the last frame per slot answers before the
    /// map, the way `callee_of_ref` does. A declaration borrows from the
    /// program for as long as the interpreter lives, so its address cannot be
    /// handed to another while a slot names it.
    #[inline]
    fn frame_for(&self, decl: &'a FnDecl) -> Frame {
        let key = decl as *const FnDecl as usize;
        let slot = recent_slot(in_slice(key), RECENT_FRAMES);
        if let (at, frame @ Some(_)) = &self.recent_frames.borrow()[slot] {
            if *at == key {
                return frame.clone();
            }
        }
        self.frame_missed(decl, key, slot)
    }

    /// The table did not hold the declaration; out of line, as `callee_missed`.
    #[inline(never)]
    fn frame_missed(&self, decl: &'a FnDecl, key: usize, slot: usize) -> Frame {
        let placed = Placed { at: key, stable: in_slice(key) };
        let known = self.frames.borrow().get(&placed).cloned();
        let frame = match known {
            Some(frame) => frame,
            None => {
                let made = frame_of(decl);
                self.frames.borrow_mut().insert(placed, made.clone());
                made
            }
        };
        self.recent_frames.borrow_mut()[slot] = (key, frame.clone());
        frame
    }

    /// Evaluate a declaration's body with its lazy bind sites in view.
    fn eval_body_of(&self, decl: &'a FnDecl, env: Option<Rc<Env>>) -> EvalResult {
        self.eval_body_in(decl, &decl.body, env, &self.frame_for(decl))
    }

    /// What a direct call of an insisting form answered, pointed at the
    /// caller's line: a failure now, or a box whose failure will be when it
    /// runs. Every other call answers as it did.
    #[inline(always)]
    fn insisted(&self, head: &Expr, answer: Value, span: Span, frame: &Frame) -> Value {
        // only an err or an effect that may yield one is pointed anywhere,
        // and nearly every answer is a plain value
        match matches!(answer, Value::ErrV(_) | Value::Desc(_)) {
            true => self.insisted_failing(head, answer, span, frame),
            false => answer,
        }
    }

    #[cold]
    #[inline(never)]
    fn insisted_failing(&self, head: &Expr, answer: Value, span: Span, frame: &Frame) -> Value {
        let Expr::Ident(name, _, _) = head else { return answer };
        if !name.as_str().ends_with('!') {
            return answer;
        }
        let Some(group) = self.fns.get(name.as_str()) else { return answer };
        let Some(first) = group.first() else { return answer };
        let Some(at) = origin_at(frame, span).at else { return answer };
        let hako: Rc<str> = Rc::from(crate::provenance::package_of(&first.file));
        match answer {
            Value::Desc(d) => Value::Desc(Rc::new(Desc::Insisted(d, at, hako))),
            other => restamped(other, &at, &hako),
        }
    }

    /// A body run whose final expression may hand back a tail call for the
    /// dispatcher's loop instead of recursing. Everything before the last
    /// statement evaluates exactly as eval_body_in does.
    /// A body run to its tail. A failure is placed as it leaves the body:
    /// every expression in it shares the body's frame, so the body is the one
    /// place the frame changes on the way out (ruled on #1975).
    ///
    /// Inlined into its one caller, the dispatch loop, with the function
    /// below: kanso#2004's additions to the loop left them out of line, and
    /// the interpreted run read 16M instructions more for it.
    #[inline(always)]
    fn eval_body_flow(
        &self,
        decl: &'a FnDecl,
        env: Option<Rc<Env>>,
    ) -> Result<Flow<'a>, RuntimeError> {
        self.eval_body_flow_unplaced(decl, env).map_err(|e| placed(e, &self.frame_for(decl)))
    }

    #[inline(always)]
    fn eval_body_flow_unplaced(
        &self,
        decl: &'a FnDecl,
        env: Option<Rc<Env>>,
    ) -> Result<Flow<'a>, RuntimeError> {
        let frame = self.frame_for(decl);
        let body = &decl.body;
        let Some((Stmt::Expr(last), lead)) = body.split_last() else {
            return Ok(Flow::Done(self.eval_body_in(decl, body, env, &frame)?));
        };
        let mut env = env;
        for (index, stmt) in lead.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), index) =>
                {
                    self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
                    let cell = Rc::new(RefCell::new(ThunkState::Pending {
                        expr: expr.clone(),
                        env: env.clone(),
                        frame: frame.clone(),
                    }));
                    env = bind(env, name.clone(), Value::Thunk(cell));
                }
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, &frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self
                        .destructure(pattern, value, env, expr.span())
                        .map_err(|e| placed(e, &frame))?;
                }
                Stmt::Expr(expr) => {
                    let _ = self.eval(expr, &env, &frame)?;
                }
            }
        }
        self.eval_tail(last, &env, &frame)
    }

    /// The tail-position evaluator: a call to a named group in tail position
    /// — directly, or through either branch of an `if` — returns the call
    /// for the dispatcher to loop on. Every other shape evaluates as `eval`
    /// would, callee and arguments included, so resolution order and
    /// failure behavior stay identical.
    fn eval_tail(
        &self,
        expr: &Expr,
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> Result<Flow<'a>, RuntimeError> {
        self.eval_tail_inner(expr, env, frame)
    }

    #[inline(always)]
    fn eval_tail_inner(
        &self,
        expr: &Expr,
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> Result<Flow<'a>, RuntimeError> {
        // a guard keeps the tail position of the lines under it: `return x
        // if c` followed by a self-call is the loop shape the library's
        // folds are written in, and a frame nested per iteration ran the
        // oracle out of stack at 10,000 elements where native looped
        if let Expr::Guard { cond, early, rest, span } = expr {
            let c = self.eval(cond, env, frame)?;
            return match c {
                Value::True => self.eval_tail(early, env, frame),
                Value::False => self.eval_stmts_flow(rest, env, frame),
                bad if is_failure(&bad) => Ok(Flow::Done(bad)),
                other => match self.wrapped_condition(other, *span)? {
                    true => self.eval_tail(early, env, frame),
                    false => self.eval_stmts_flow(rest, env, frame),
                },
            };
        }
        let Expr::App { head, args, span, piped } = expr else {
            return Ok(Flow::Done(self.eval(expr, env, frame)?));
        };
        if matches!(head.as_ref(), Expr::Partial(..)) {
            return Ok(Flow::Done(self.eval(expr, env, frame)?));
        }
        let group_of = |callee: &Value| -> Option<(Rc<str>, Rc<Vec<&'a FnDecl>>)> {
            let Value::FnRef(n) = callee else { return None };
            // `if` is the one name the memory would answer Group for and this
            // may not: it is a declaration AND the conditional form, and the
            // form wins here. The group rides along with the name, so the
            // dispatcher that takes the tail has the overloads in hand and
            // does not hash the name to find them a second time.
            if &**n == "if" {
                return None;
            }
            match self.callee_of_ref(n) {
                Callee::Group(group) => Some((n.clone(), group)),
                _ => None,
            }
        };
        if *piped && !args.is_empty() {
            let piped_value = self.eval(&args[0], env, frame)?;
            if is_failure(&piped_value) {
                return Ok(Flow::Done(piped_value));
            }
            if let Value::Desc(inner) = piped_value {
                let mut body_args: Vec<Expr> = vec![Expr::Ident(
                    Name::new("__piped"),
                    *span,
                    crate::ast::Resolution::default(),
                )];
                body_args.extend(args[1..].iter().cloned());
                let closure = Value::Closure(Rc::new(ClosureData {
                    params: vec![Name::new("__piped")],
                    body: Expr::App {
                        head: head.clone(),
                        args: body_args,
                        span: *span,
                        piped: false,
                    },
                    env: env.clone(),
                    frame: frame.clone(),
                    deep: self.running.get(),
                }));
                return Ok(Flow::Done(Value::Desc(Rc::new(Desc::Bind(inner, closure)))));
            }
            let callee = self.eval(head, env, frame)?;
            let mut values = vec![piped_value];
            for arg in &args[1..] {
                values.push(self.eval(arg, env, frame)?);
            }
            if insists(head) {
                let answer = self.call(callee, values, *span, frame)?;
                return Ok(Flow::Done(self.insisted(head, answer, *span, frame)));
            }
            return match (group_of(&callee), callee) {
                (Some((name, group)), _) => Ok(Flow::Tail(name, group, values, *span)),
                (None, Value::Closure(c)) => Ok(Flow::Closure(c, values, *span)),
                (None, callee) => Ok(Flow::Done(self.call(callee, values, *span, frame)?)),
            };
        }
        let callee = self.eval(head, env, frame)?;
        if matches!(&callee, Value::FnRef(n) if &**n == "if") && args.len() == 3 {
            let cond = self.force(self.eval(&args[0], env, frame)?)?;
            return match cond {
                Value::True => self.eval_tail(&args[1], env, frame),
                Value::False => self.eval_tail(&args[2], env, frame),
                bad if is_failure(&bad) => Ok(Flow::Done(bad)),
                other => match self.wrapped_condition(other, *span)? {
                    true => self.eval_tail(&args[1], env, frame),
                    false => self.eval_tail(&args[2], env, frame),
                },
            };
        }
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(arg, env, frame)?);
        }
        // an insisting form is called rather than looped on, so what it
        // answers can be pointed at this line on the way back
        if insists(head) {
            let answer = self.call(callee, values, *span, frame)?;
            return Ok(Flow::Done(self.insisted(head, answer, *span, frame)));
        }
        match (group_of(&callee), callee) {
            (Some((name, group)), _) => Ok(Flow::Tail(name, group, values, *span)),
            (None, Value::Closure(c)) => Ok(Flow::Closure(c, values, *span)),
            (None, callee) => Ok(Flow::Done(self.call(callee, values, *span, frame)?)),
        }
    }

    /// A zero divisor's answer as the program can see it; see `bare_math`.
    /// The wrapper comes off here, where a value is about to be handed to an
    /// operator's arms, and nowhere on the path every sum takes.
    fn as_declared_math(&self, value: Value) -> Value {
        match self.type_decl(crate::DIVIDE_BY_ZERO) {
            Some(_) => value,
            None => bare_math(value),
        }
    }

    fn type_decl(&self, name: &str) -> Option<&TypeDecl> {
        match name {
            "entry" => Some(&self.entry_decl),
            _ => self.types.get(name).copied(),
        }
    }

    pub fn run_main(&self) -> EvalResult {
        let entry = self.fns.get(crate::ast::ENTRY).expect("checked: an entry exists")[0];
        self.eval_body_of(entry, None)
    }

    pub fn run_named(&self, name: &str) -> Option<EvalResult> {
        let decl = self.fns.get(name)?.iter().find(|d| d.params.is_empty())?;
        Some(self.eval_body_of(decl, None))
    }

    /// A statement list in expression position: a branch body, a build
    /// block, or the tail a fired guard skipped past.
    /// A block whose last statement may hand back a tail call: the lines
    /// before it run as `eval_stmts` runs them, on a copy of the environment.
    fn eval_stmts_flow(
        &self,
        stmts: &[Stmt],
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> Result<Flow<'a>, RuntimeError> {
        let Some((Stmt::Expr(last), lead)) = stmts.split_last() else {
            return Ok(Flow::Done(self.eval_stmts(stmts, env, frame)?));
        };
        let mut env = env.clone();
        self.run_stmts(lead, &mut env, frame)?;
        self.eval_tail(last, &env, frame)
    }

    fn eval_stmts(&self, stmts: &[Stmt], env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        let mut env = env.clone();
        self.run_stmts(stmts, &mut env, frame)
    }

    /// The statements of a block, threading the environment they build.
    fn run_stmts(&self, stmts: &[Stmt], env: &mut Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        let mut result = Value::NoneV;
        for stmt in stmts {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, env, frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    *env = self
                        .destructure(pattern, value, env.clone(), expr.span())
                        .map_err(|e| placed(e, frame))?;
                }
                Stmt::Expr(expr) => result = self.eval(expr, env, frame)?,
            }
        }
        Ok(result)
    }

    fn eval_body_in(
        &self,
        decl: &FnDecl,
        body: &[Stmt],
        env: Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        self.eval_body_in_unplaced(decl, body, env, frame).map_err(|e| placed(e, frame))
    }

    fn eval_body_in_unplaced(
        &self,
        decl: &FnDecl,
        body: &[Stmt],
        mut env: Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        let mut result = Value::NoneV;
        for (index, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), index) =>
                {
                    self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
                    let cell = Rc::new(RefCell::new(ThunkState::Pending {
                        expr: expr.clone(),
                        env: env.clone(),
                        frame: frame.clone(),
                    }));
                    env = bind(env, name.clone(), Value::Thunk(cell));
                }
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self
                        .destructure(pattern, value, env, expr.span())
                        .map_err(|e| placed(e, frame))?;
                }
                Stmt::Expr(expr) => result = self.eval(expr, &env, frame)?,
            }
        }
        Ok(result)
    }

    fn destructure(
        &self,
        pattern: &Pattern,
        value: Value,
        env: Option<Rc<Env>>,
        span: Span,
    ) -> Result<Option<Rc<Env>>, RuntimeError> {
        match pattern {
            Pattern::Var(name, _) => Ok(bind(env, name.clone(), value)),
            Pattern::Ctor { ty, .. } => {
                let mut binds = Vec::new();
                match match_one(pattern, &value, &mut binds) {
                    Some(_) => {
                        let mut env = env;
                        for (name, bound) in binds {
                            env = bind(env, name, bound);
                        }
                        Ok(env)
                    }
                    None => Err(RuntimeError {
                        at: None,
                        message: format!(
                            "cannot destructure {} as `{ty}`; bindings are irrefutable, so \
                             handle other types by dispatch first",
                            render(self, &value, true)
                        ),
                        span,
                    }),
                }
            }
            Pattern::Keyed { entries, .. } => {
                let Value::Record { ty, fields } = &value else {
                    return Err(RuntimeError {
                        at: None,
                        message: format!(
                            "cannot read fields of {}; keyed reads take a record",
                            render(self, &value, true)
                        ),
                        span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                if entries.len() >= decl.fields.len() {
                    return Err(RuntimeError {
                        at: None,
                        message: "a keyed read omits at least one field; reading every \
                                  field is the positional form"
                            .to_string(),
                        span,
                    });
                }
                let mut env = env;
                for entry in entries {
                    let position = decl.fields.iter().position(|(name, _, _)| *name == entry.field);
                    let Some(position) = position else {
                        return Err(RuntimeError {
                            at: None,
                            message: format!("`{ty}` has no field `{}`", entry.field),
                            span,
                        });
                    };
                    env = bind(env, Name::new(&entry.bind_name), fields.borrow()[position].clone());
                }
                Ok(env)
            }
            _ => Err(RuntimeError {
                at: None,
                message: "binding patterns are irrefutable: names and constructor \
                          patterns only"
                    .to_string(),
                span,
            }),
        }
    }

    /// A local whose place is kept answers here, a global whose slot holds a
    /// fixed value answers here, and so does an integer literal; everything
    /// else goes to `eval_node`.
    ///
    /// `eval_node` holds every expression form, so its frame is sized for the
    /// largest of them: six registers pushed and 792 bytes reserved on every
    /// call. Local names were 653,358 of the expressions the interpreted
    /// corpus evaluates, global names 299,294 and integer literals 122,017.
    /// The gain depends on this function being inlined where it is called, so
    /// that a local read there makes no call at all: kept out of line with
    /// `#[inline(never)]` the corpus read 542,946,770, worse than no door.
    /// With the literal arm the compiler stopped inlining it on its own and
    /// the corpus read 533,252,591, so the attribute below says what the
    /// whole design relies on. Anything added here is copied into every
    /// caller.
    #[inline(always)]
    fn eval(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        if let Expr::Ident(name, _, resolved) = expr {
            let v = resolved.get();
            if (3..1 << 16).contains(&v) {
                if let Some(value) = lookup_at(env, name, v) {
                    return Ok(value);
                }
            } else if let Some(value) = self.fixed_global(v) {
                return Ok(value);
            }
        } else if let Expr::Int(n, _) = expr {
            return Ok(int_literal(n));
        }
        self.eval_node(expr, env, frame)
    }

    /// A global name whose slot holds a fixed value, read without going
    /// through `eval_node`. Of the first 300,000 global reads on the
    /// interpreted corpus, 298,949 were answered from the table; the rest
    /// were first reads, which stamp the slot. Out of line, so the door stays
    /// small.
    #[inline(never)]
    fn fixed_global(&self, v: u32) -> Option<Value> {
        let slot = crate::ast::Resolution::slot_for(v, self.generation)?;
        self.fixed.borrow().get(slot)?.clone()
    }

    /// A failure leaving the expression that raised it takes that
    /// expression's frame as its place. The innermost node it passes out of
    /// is the one whose file the span is in, so a failure already placed is
    /// left alone on the way up (ruled on #1975).
    #[inline(never)]
    fn eval_node(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        self.eval_node_inner(expr, env, frame)
    }

    #[inline(always)]
    fn eval_node_inner(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        match expr {
            Expr::Int(n, _) => Ok(Value::int(n)),
            Expr::Upcast { expr: inner, ty, span } => {
                let v = self.force_thunk(self.eval(inner, env, frame)?)?;
                if is_failure(&v) {
                    return Ok(v);
                }
                let mut cur = v;
                loop {
                    if type_matches_exact(ty, &cur) {
                        return Ok(cur);
                    }
                    match cur {
                        Value::Sub { inner, .. } => cur = (*inner).clone(),
                        _ => {
                            return Err(RuntimeError {
                                at: None,
                                message: format!(
                                    "`:{ty}` widens; this value is not {}",
                                    article(ty)
                                ),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Block(stmts, _) => self.eval_stmts(stmts, env, frame),
            Expr::Float(x, _) => Ok(Value::Float(*x)),
            Expr::MapLit(pairs, span) => {
                let mut entries = Entries::new();
                for (key_expr, value_expr) in pairs {
                    let key = self.eval(key_expr, env, frame)?;
                    let value = self.eval(value_expr, env, frame)?;
                    if is_failure(&key) {
                        return Ok(key);
                    }
                    let key = map_key(key, *span)?;
                    entries.insert(key, value);
                }
                Ok(Value::Map(Rc::new(entries)))
            }
            Expr::Str(parts, span) => self.eval_template(parts, *span, env, frame),
            Expr::Ident(name, span, resolved) => self.eval_ident_at(name, *span, resolved, env),
            Expr::Partial(name, span) => {
                let callee = self.eval_ident(name, *span, env)?;
                Ok(Value::Partial(Rc::new(callee), Rc::new(Vec::new())))
            }
            Expr::List(items, _) => {
                let values = items
                    .iter()
                    .map(|e| self.stored(e, env, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(values)))
            }
            Expr::Field { base, name, span } => {
                let value = self.eval(base, env, frame)?;
                if is_failure(&value) {
                    return Ok(value);
                }
                // a getter is a constructor pattern, so it reads through a
                // subtype for the same reason one matches
                let value = match value {
                    Value::Sub { .. } => sub_base(value),
                    other => other,
                };
                let Value::Record { ty, fields } = &value else {
                    return Err(RuntimeError {
                        at: None,
                        message: format!(
                            "`.` reads a field of a record, not {}",
                            render(self, &value, true)
                        ),
                        span: *span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                let position = decl.fields.iter().position(|(f, _, _)| f == name);
                match position {
                    Some(position) => Ok(fields.borrow()[position].clone()),
                    None => Err(RuntimeError {
                        at: None,
                        message: format!("`{ty}` has no field `{name}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.force_thunk(self.eval(base, env, frame)?)?;
                let key = self.force_thunk(self.eval(index, env, frame)?)?;
                // the sigil is the choice of channel (ruled 2026-09-16): the
                // read settles a box holding the element or the miss
                match index_value(container, key.clone(), *span)? {
                    Value::NoneV if *strict => Ok(Value::Desc(Rc::new(Desc::Settled(err_value(
                        Value::Str(format!("missing index {}", render(self, &key, true)).into()),
                        origin_at(frame, *span),
                    ))))),
                    found if *strict => Ok(Value::Desc(Rc::new(Desc::Settled(found)))),
                    found => Ok(found),
                }
            }
            // `&f a b` supplies without finishing: it never dispatches at the
            // count it was written with, or `&` would be unreachable whenever
            // a shorter arm exists — which is the case it exists for.
            Expr::App { head, args, span, .. } if matches!(head.as_ref(), Expr::Partial(..)) => {
                let Expr::Partial(name, name_span) = head.as_ref() else { unreachable!() };
                let callee = self.eval_ident(name, *name_span, env)?;
                if is_failure(&callee) {
                    return Ok(callee);
                }
                let supplied =
                    args.iter().map(|a| self.eval(a, env, frame)).collect::<Result<Vec<_>, _>>()?;
                if let Some(bad) = supplied.iter().find(|v| is_failure(v)) {
                    return Ok(bad.clone());
                }
                let _ = span;
                Ok(Value::Partial(Rc::new(callee), Rc::new(supplied)))
            }
            Expr::App { head, args, span, piped } => {
                if *piped && !args.is_empty() {
                    let piped_value = self.eval(&args[0], env, frame)?;
                    if is_failure(&piped_value) {
                        return Ok(piped_value);
                    }
                    if let Value::Desc(inner) = piped_value {
                        let mut body_args: Vec<Expr> = vec![Expr::Ident(
                            Name::new("__piped"),
                            *span,
                            crate::ast::Resolution::default(),
                        )];
                        body_args.extend(args[1..].iter().cloned());
                        let closure = Value::Closure(Rc::new(ClosureData {
                            params: vec![Name::new("__piped")],
                            body: Expr::App {
                                head: head.clone(),
                                args: body_args,
                                span: *span,
                                piped: false,
                            },
                            env: env.clone(),
                            frame: frame.clone(),
                            deep: self.running.get(),
                        }));
                        return Ok(Value::Desc(Rc::new(Desc::Bind(inner, closure))));
                    }
                    let callee = self.eval(head, env, frame)?;
                    let mut values = vec![piped_value];
                    for arg in &args[1..] {
                        values.push(self.eval(arg, env, frame)?);
                    }
                    let answer = self.call(callee, values, *span, frame)?;
                    return Ok(self.insisted(head, answer, *span, frame));
                }
                let callee = self.eval(head, env, frame)?;
                // `if` in VALUE position, the way `eval_tail` has always done it
                // in tail position: ask the condition, then evaluate the branch
                // that answer chooses. Nothing is deferred, so nothing is
                // wrapped.
                //
                // What it replaces built a nullary closure around EVERY
                // argument -- copying the whole subtree, cloning the
                // environment and the frame, allocating an `Rc` -- and handed
                // all three to `builtin_if`, which forces the condition on the
                // way in and then the one branch it picks. Two of the three
                // closures were built to be thrown away and the third to be
                // opened immediately. The annotated profile put the subtree
                // copying alone at 11,471,717 instructions over 50,235 clones.
                //
                // `force` matches what `builtin_if` did: it opens a nullary
                // closure and then reaches through a thunk, so `force_thunk` on
                // the branch is the second half of that. A failure VALUE is
                // still the answer rather than a fault, as it was there.
                if matches!(&callee, Value::FnRef(n) if &**n == "if") && args.len() == 3 {
                    let cond = self.force(self.eval(&args[0], env, frame)?)?;
                    return match cond {
                        Value::True => self.force_thunk(self.eval(&args[1], env, frame)?),
                        Value::False => self.force_thunk(self.eval(&args[2], env, frame)?),
                        bad if is_failure(&bad) => Ok(bad),
                        other => match self.wrapped_condition(other, *span)? {
                            true => self.force_thunk(self.eval(&args[1], env, frame)?),
                            false => self.force_thunk(self.eval(&args[2], env, frame)?),
                        },
                    };
                }
                let lazy_if = matches!(&callee, Value::FnRef(name) if &**name == "if");
                let mut values = Vec::with_capacity(args.len());
                // Any other arity reaches `builtin_if` as before -- a partial
                // application, or a miscount the arity check is there to name.
                for arg in args {
                    match lazy_if {
                        true => values.push(Value::Closure(Rc::new(ClosureData {
                            params: Vec::new(),
                            body: arg.clone(),
                            env: env.clone(),
                            frame: frame.clone(),
                            deep: self.running.get(),
                        }))),
                        false => values.push(self.eval(arg, env, frame)?),
                    }
                }
                let answer = self.call(callee, values, *span, frame)?;
                Ok(self.insisted(head, answer, *span, frame))
            }
            Expr::Lambda { params, body, .. } => Ok(Value::Closure(Rc::new(ClosureData {
                params: params.iter().map(|(n, _)| Name::new(n)).collect(),
                body: (**body).clone(),
                env: env.clone(),
                frame: frame.clone(),
                deep: self.running.get(),
            }))),
            Expr::BinOp { op, lhs, rhs, span } => {
                let mut left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let mut right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                // records dispatch to the operator's user arms; numbers stay
                // on the builtin (coherence licenses the fast path, and the
                // orphan rule keeps 2 + 3 meaning one thing forever). Either
                // side carries the question: an arm naming its type in the
                // second position is a declaration too, and a gate reading
                // only the first leaves it matching nothing.
                if (routes_to_arms(&left) || routes_to_arms(&right))
                    && self.fns.contains_key(op as &str)
                {
                    left = self.as_declared_math(left);
                    right = self.as_declared_math(right);
                    if routes_to_arms(&left) || routes_to_arms(&right) {
                        return self.call_named(op, vec![left, right], *span, frame);
                    }
                }
                let cells = Cells { id: &thunk_identity, force: &|v| self.force_thunk(v) };
                eval_binop(op, sub_base(left), sub_base(right), *span, &cells)
            }
            Expr::Guard { cond, early, rest, span } => {
                let c = self.eval(cond, env, frame)?;
                match c {
                    Value::True => self.eval(early, env, frame),
                    Value::False => self.eval_stmts(rest, env, frame),
                    bad if is_failure(&bad) => Ok(bad),
                    other => match self.wrapped_condition(other, *span)? {
                        true => self.eval(early, env, frame),
                        false => self.eval_stmts(rest, env, frame),
                    },
                }
            }
            Expr::Join { lhs, rhs, span } => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                join_values(left, right, *span)
            }
        }
    }

    /// An identifier at a place in the text, which remembers whether it found
    /// a local there. A global goes straight to `eval_global` without walking
    /// the environment: on the interpreted corpus that walk missed 332,025
    /// times over 1,071,803 frames, every one of them for a name no frame
    /// could hold. The first execution decides, and the node keeps the answer,
    /// because the frames a node can see are fixed by where it sits.
    ///
    /// Debug builds check the kept answer on every execution: a node marked
    /// global that finds a local, or marked local that misses, stops the run.
    /// Every spec runs in a debug build, so the differential suites check the
    /// property on every program they carry.
    fn eval_ident_at(
        &self,
        name: &Name,
        span: Span,
        resolved: &crate::ast::Resolution,
        env: &Option<Rc<Env>>,
    ) -> EvalResult {
        use crate::ast::Resolution;
        let v = resolved.get();
        if let Some(slot) = Resolution::slot_for(v, self.generation) {
            debug_assert!(
                lookup(env, name).is_none(),
                "`{}` at {:?} was kept as global and found a local",
                name.as_str(),
                span
            );
            let named = self.slots.borrow()[slot].clone();
            debug_assert!(
                self.names
                    .borrow()
                    .get(name.as_str())
                    .is_some_and(|by_name| by_name.same_as(&named)),
                "`{}` at {:?} read slot {} and it is not what the name resolves to",
                name.as_str(),
                span,
                slot
            );
            return self.named_value(name.as_str(), named);
        }
        match v {
            v if (3..1 << 16).contains(&v) => match lookup_at(env, name, v) {
                Some(value) => {
                    debug_assert!(
                        lookup_placed(env, name).is_some_and(|(_, at)| at == v),
                        "`{}` at {:?} was kept at {} and a nearer binding holds it",
                        name.as_str(),
                        span,
                        v
                    );
                    Ok(value)
                }
                None => self.local_moved(name, span, resolved, env),
            },
            Resolution::LOCAL => self.local_moved(name, span, resolved, env),
            v if Resolution::is_global(v) => {
                debug_assert!(
                    lookup(env, name).is_none(),
                    "`{}` at {:?} was kept as global and found a local",
                    name.as_str(),
                    span
                );
                self.global_at(name.as_str(), span, resolved)
            }
            _ => match lookup_placed(env, name) {
                Some((value, at)) => {
                    resolved.set(at);
                    Ok(value)
                }
                None => self.global_at(name.as_str(), span, resolved),
            },
        }
    }

    /// A local whose kept place no longer holds it, or that never had one:
    /// walk, and keep where the walk found it.
    #[inline(never)]
    fn local_moved(
        &self,
        name: &Name,
        span: Span,
        resolved: &crate::ast::Resolution,
        env: &Option<Rc<Env>>,
    ) -> EvalResult {
        match lookup_placed(env, name) {
            Some((value, at)) => {
                resolved.set(at);
                Ok(value)
            }
            None => {
                debug_assert!(
                    false,
                    "`{}` at {:?} was kept as local and missed",
                    name.as_str(),
                    span
                );
                self.eval_global(name.as_str(), span)
            }
        }
    }

    /// A global reached from a node that does not yet carry this
    /// interpreter's stamp: resolve it by name, keep it in a slot, stamp the
    /// node, and evaluate it.
    fn global_at(&self, name: &str, span: Span, resolved: &crate::ast::Resolution) -> EvalResult {
        use crate::ast::Resolution;
        let named = self.resolve_named(name, span)?;
        // A slot is taken only when the stamp can be written, so a node that
        // cannot be stamped -- stamping switched off, or the table full --
        // resolves by name each time rather than growing the table each time.
        let stamp = {
            let mut slots = self.slots.borrow_mut();
            match Resolution::stamped(self.generation, slots.len()) {
                Resolution::GLOBAL => Resolution::GLOBAL,
                stamp => {
                    let fixed = match &named {
                        Named::FnRef(n) => Some(Value::FnRef(n.clone())),
                        Named::True => Some(Value::True),
                        Named::False => Some(Value::False),
                        Named::NoneV => Some(Value::NoneV),
                        Named::Done => Some(Value::Done),
                        _ => None,
                    };
                    self.fixed.borrow_mut().push(fixed);
                    slots.push(named.clone());
                    stamp
                }
            }
        };
        resolved.set(stamp);
        self.named_value(name, named)
    }

    fn eval_ident(&self, name: &Name, span: Span, env: &Option<Rc<Env>>) -> EvalResult {
        match lookup(env, name) {
            Some(value) => Ok(value),
            None => self.eval_global(name.as_str(), span),
        }
    }

    /// The path a name takes when no scope binds it. It is out of line so that
    /// `eval_ident` -- which a scope hit leaves after two loads -- does not
    /// carry this one's frame.
    #[inline(never)]
    fn eval_global(&self, name: &str, span: Span) -> EvalResult {
        let named = self.resolve_named(name, span)?;
        self.named_value(name, named)
    }

    /// What a global name resolves to, by name, once per run.
    fn resolve_named(&self, name: &str, span: Span) -> Result<Named<'a>, RuntimeError> {
        // The borrow is dropped before anything below runs: `knotted` evaluates
        // a constant's body, which reaches `eval_ident` again and would find
        // this cell already borrowed.
        let known = self.names.borrow().get(name).cloned();
        match known {
            Some(named) => Ok(named),
            None => {
                let named = self.resolve(name, span)?;
                self.names.borrow_mut().insert(name.to_string(), named.clone());
                Ok(named)
            }
        }
    }

    /// The value a resolved global stands for.
    fn named_value(&self, name: &str, named: Named<'a>) -> EvalResult {
        match named {
            Named::Constant(constant) => self.knotted(name, constant),
            Named::Args => Ok(Value::Desc(Rc::new(Desc::Args))),
            Named::Stdin => Ok(Value::Desc(Rc::new(Desc::Stdin))),
            Named::ReadLine => Ok(Value::Desc(Rc::new(Desc::ReadLine))),
            Named::Now => Ok(Value::Desc(Rc::new(Desc::Now))),
            Named::EmptyRecord(ty) => {
                Ok(Value::Record { ty, fields: Rc::new(RefCell::new(Vec::new())) })
            }
            Named::True => Ok(Value::True),
            Named::False => Ok(Value::False),
            Named::NoneV => Ok(Value::NoneV),
            Named::Done => Ok(Value::Done),
            Named::FnRef(name) => Ok(Value::FnRef(name)),
        }
    }

    /// The ladder, walked once per name per run. Every arm below was an arm of
    /// `eval_ident` and they are in the order they were in, because the order
    /// decides the answer: a constant beats a descriptor name, a descriptor
    /// name beats a field-less type, and a literal beats a reference.
    fn resolve(&self, name: &str, span: Span) -> Result<Named<'a>, RuntimeError> {
        // A name the program declares wins over what an import enrolled under
        // the same bare name and over the descriptor names below, as an
        // enrolled type gives way in `callee`: make's `fn read_line at s`
        // beside std/io's `read_line` constant was read as the constant and
        // stopped with "`<io>` is not callable".
        let own = self.fns.get(name).is_some_and(|decls| decls.iter().any(|d| !d.synthetic));
        if let Some(decls) = self.fns.get(name) {
            if let Some(constant) =
                decls.iter().find(|d| d.params.is_empty() && !(d.synthetic && own))
            {
                // Every constant goes through the knot, not only one that
                // names itself. `knotted` installs a blackhole before it
                // evaluates, so re-entering the name finds it — and a cycle
                // reaches its second name as readily as its first. Asking
                // whether a constant mentions its own name sees `a = f b` and
                // `b = f a` as two ordinary constants, and evaluating either
                // recurses until the process dies.
                return Ok(Named::Constant(constant));
            }
        }
        let described = match own {
            true => "",
            false => name.strip_prefix("builtin_").unwrap_or(name),
        };
        match described {
            "args" => return Ok(Named::Args),
            "stdin" => return Ok(Named::Stdin),
            "read_line" => return Ok(Named::ReadLine),
            "now" => return Ok(Named::Now),
            _ => {}
        }
        if let Some(decl) = self.type_decl(name) {
            if decl.parent.is_none() && decl.members.is_empty() && decl.fields.is_empty() {
                // a bare spelling an import enrolled is the type it stands
                // for, as a constructor call's record already is
                return Ok(Named::EmptyRecord(Rc::from(decl.origin.as_deref().unwrap_or(name))));
            }
        }
        match name {
            "true" => Ok(Named::True),
            "false" => Ok(Named::False),
            "none" => Ok(Named::NoneV),
            "done" => Ok(Named::Done),
            _ if self.fns.contains_key(name)
                || self.types.contains_key(name)
                || name == "err"
                || crate::check::BUILTINS.contains(&name)
                || name
                    .strip_prefix("builtin_")
                    .is_some_and(|n| crate::check::BUILTINS.contains(&n)) =>
            {
                Ok(Named::FnRef(Rc::from(name)))
            }
            _ => Err(RuntimeError { at: None, message: format!("unknown name `{name}`"), span }),
        }
    }

    fn eval_template(
        &self,
        parts: &[TemplatePart],
        span: Span,
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        if let [TemplatePart::Interp(Expr::Ident(name, _, _)), rest @ ..] = parts {
            if !rest.is_empty() {
                if let Some(joined) = self.join_onto(name, rest, span, env, frame)? {
                    return Ok(joined);
                }
            }
        }
        let mut out = String::new();
        self.render_parts(parts, env, frame, &mut out).map(|failed| match failed {
            Some(err) => err,
            None => Value::Str(Text::from(out)),
        })
    }

    /// Each part rendered onto `out`, or the err a part answered.
    fn render_parts(
        &self,
        parts: &[TemplatePart],
        env: &Option<Rc<Env>>,
        frame: &Frame,
        out: &mut String,
    ) -> Result<Option<Value>, RuntimeError> {
        for part in parts {
            match part {
                TemplatePart::Lit(s) => out.push_str(s),
                TemplatePart::Interp(expr) => {
                    let value = self.force_thunk(self.eval(expr, env, frame)?)?;
                    // an err propagates (the whole string returns the err); a
                    // none is a value and renders its sentinel
                    if matches!(value, Value::ErrV(_)) {
                        return Ok(Some(value));
                    }
                    match self.render_interpolated(value)? {
                        Ok(rendered) => out.push_str(&rendered),
                        Err(err) => return Ok(Some(err)),
                    }
                }
            }
        }
        Ok(None)
    }

    /// `"{s}..."` where `linear::string_builders` proved nothing reads `s`
    /// after this join: the string comes out of the binding and the rest is
    /// pushed onto it. Reading `s` would copy it, and a loop that grows a
    /// string this way copied the whole string on every lap, 77 million
    /// instructions for 3,000 pieces. Native and the tab join in place at the
    /// same sites. `None` takes the ordinary path: a binding that holds
    /// something other than a string, or a string short enough that copying
    /// it costs less than asking. The analysis is only built once a long
    /// string reaches a join.
    fn join_onto(
        &self,
        name: &Name,
        rest: &[TemplatePart],
        span: Span,
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(slot) = slot_of(env, name) else { return Ok(None) };
        // SAFETY: the slot lives as long as `env`, which this call holds.
        let long = matches!(unsafe { &*slot }, Value::Str(t) if t.len() > TEXT_INLINE);
        if !long || !self.joins_here(span, frame) {
            return Ok(None);
        }
        // The rest first: a later piece may read `s` as well, `"{s}{s}"`, and
        // has to read the whole of it.
        let mut tail = String::new();
        if let Some(err) = self.render_parts(rest, env, frame, &mut tail)? {
            return Ok(Some(err));
        }
        // SAFETY: as `written_in_place`. The analysis proved no later read of
        // the binding exists, and nothing else refers into the slot across
        // the replace. A string some other holder shares is copied once by
        // `push_str` rather than written.
        let Value::Str(text) = std::mem::replace(unsafe { &mut *slot }, Value::NoneV) else {
            unreachable!("checked to be a string above")
        };
        Ok(Some(Value::Str(text.push_str(&tail))))
    }

    fn call(&self, callee: Value, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        match callee {
            Value::FnRef(name) => self.call_ref(&name, args, span, frame),
            Value::Closure(closure) => self.call_closure(&closure, args, span),
            Value::TableFn(handle) => foreign_call(handle, args, span, false),
            Value::Partial(callee, supplied) => {
                // `()` runs a partial rather than supplying more to it, so an
                // empty argument list is the call form and never an
                // accumulation of nothing. A partial still short of every arm
                // has nothing to run, and says how many it is short by.
                match args.is_empty() {
                    true => {
                        self.run_partial(callee.as_ref().clone(), supplied.as_ref(), span, frame)
                    }
                    false => {
                        let mut all = supplied.as_ref().clone();
                        all.extend(args);
                        self.apply_partial(callee.as_ref().clone(), all, span, frame)
                    }
                }
            }
            bad if is_failure(&bad) => Ok(bad),
            other => Err(RuntimeError {
                at: None,
                message: format!("`{}` is not callable", render(self, &other, false)),
                span,
            }),
        }
    }

    /// Every argument count the callee answers, smallest first.
    fn arities_of(&self, callee: &Value) -> Vec<usize> {
        match callee {
            // A reference answers by what it calls. A constructor takes its
            // fields, a subtype's takes the one value it wraps, and a builtin
            // takes the count the checker holds for it. Answering only for
            // groups left `(&pt 2) "y"` and `(&length) [1 2]` growing into
            // partials that printed `<fn>` where native ran `length`.
            Value::FnRef(name) => match self.callee_of_ref(name) {
                Callee::Group(decls) => {
                    let mut found: Vec<usize> = decls.iter().map(|d| d.params.len()).collect();
                    found.sort_unstable();
                    found.dedup();
                    found
                }
                Callee::Constructor(ty) if ty.parent.is_some() => vec![1],
                Callee::Constructor(ty) if ty.members.is_empty() => vec![ty.fields.len()],
                Callee::Constructor(_) => Vec::new(),
                Callee::EntryType => vec![self.entry_decl.fields.len()],
                Callee::Err => vec![1],
                // `if` is the one builtin the checker's table leaves out, since
                // its count is checked where its branches are; held, it takes
                // the condition and both branches like any call of it.
                Callee::Builtin if &**name == "if" => vec![3],
                Callee::Builtin => crate::check::builtin_arity(name).into_iter().collect(),
            },
            Value::Closure(c) => vec![c.params.len()],
            // A partial answers what its callee answers, less what it holds.
            // Answering nothing left `(&h 3) 1` over `h = &g p` wrapped in a
            // third partial where native ran `g p 3 1`, and `h 3 1` beside it
            // ran here too.
            Value::Partial(inner, held) => self
                .arities_of(inner)
                .into_iter()
                .filter(|a| *a >= held.len())
                .map(|a| a - held.len())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// `foo()` on a partial. Complete, it is the call the partial was built
    /// for; short, it is an arity error naming what is still missing, which is
    /// the count the compiled engines report from the closure they lowered it
    /// to.
    fn run_partial(
        &self,
        callee: Value,
        supplied: &[Value],
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        let arities = self.arities_of(&callee);
        if arities.contains(&supplied.len()) {
            return self.call(callee, supplied.to_vec(), span, frame);
        }
        let waiting = arities.iter().filter(|a| **a > supplied.len()).min().copied();
        let short = waiting.map(|a| a - supplied.len()).unwrap_or(0);
        Err(RuntimeError {
            at: None,
            message: format!("this function takes {short} argument(s), got 0"),
            span,
        })
    }

    fn call_closure(&self, closure: &ClosureData, args: Vec<Value>, span: Span) -> EvalResult {
        let running = self.running.get();
        let answer = self.call_closure_inner(closure, args, span);
        self.running.set(running);
        answer
    }

    fn call_closure_inner(
        &self,
        closure: &ClosureData,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let mut flow = self.closure_flow(closure, args, span)?;
        // the lambda whose body wrote the call being made, which places a
        // call that fails before anything it calls has run
        let mut writer: Option<Rc<ClosureData>> = None;
        let frame_of = |w: &Option<Rc<ClosureData>>| match w {
            Some(c) => c.frame.clone(),
            None => closure.frame.clone(),
        };
        loop {
            match flow {
                Flow::Done(value) => return Ok(value),
                Flow::Tail(name, group, args, span) => {
                    return self
                        .dispatch_loop(name, group, args, span)
                        .map_err(|e| placed(e, &frame_of(&writer)))
                }
                Flow::Closure(next, args, span) => {
                    flow = self
                        .closure_flow(&next, args, span)
                        .map_err(|e| placed(e, &frame_of(&writer)))?;
                    writer = Some(next);
                }
            }
        }
    }

    /// A closure's body run to its tail. What the tail hands back goes to the
    /// caller's loop: `call_closure` for a call that is not itself in tail
    /// position, the dispatcher's loop for one that is.
    fn closure_flow(
        &self,
        closure: &ClosureData,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Flow<'a>, RuntimeError> {
        // a lambda's body is on the cycle of the body that made it, as its
        // compiled function is
        self.running.set(closure.deep);
        if closure.params.len() != args.len() {
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "this function takes {} argument(s), got {}",
                    closure.params.len(),
                    args.len()
                ),
                span,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(Flow::Done(merged_failures(&args)));
        }
        let mut env = closure.env.clone();
        for (name, value) in closure.params.iter().zip(args) {
            env = bind(env, name.clone(), value);
        }
        self.eval_tail(&closure.body, &env, &closure.frame).map_err(|e| placed(e, &closure.frame))
    }

    /// Dispatch fires the moment the supplied count matches an arity the
    /// callee answers to; short of that the partial simply grows. A count
    /// past every arity is the error, and it names what was available.
    fn apply_partial(
        &self,
        callee: Value,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        let arities = self.arities_of(&callee);
        if arities.contains(&args.len()) {
            return self.call(callee, args, span, frame);
        }
        if arities.iter().all(|a| *a < args.len()) && !arities.is_empty() {
            let names: Vec<String> = arities.iter().map(usize::to_string).collect();
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "no {}-argument arm of `{}` (arms take {})",
                    args.len(),
                    render(self, &callee, false),
                    names.join(" or ")
                ),
                span,
            });
        }
        Ok(Value::Partial(Rc::new(callee), Rc::new(args)))
    }

    fn call_named(&self, name: &str, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        let callee = self.callee_named(name);
        self.call_callee(callee, name, args, span, frame)
    }

    /// What a name calls, remembered by name. The borrow is dropped before
    /// the call runs, because the body it reaches calls back in here.
    fn callee_named(&self, name: &str) -> Callee<'a> {
        let known = self.callees.borrow().get(name).cloned();
        match known {
            Some(callee) => callee,
            None => {
                let callee = self.callee(name);
                self.callees.borrow_mut().insert(name.to_string(), callee.clone());
                callee
            }
        }
    }

    /// A call through a function reference, remembered by the reference's
    /// address rather than by hashing its text. Every `Value::FnRef` is made
    /// in `resolve`, whose answer `names` keeps for the interpreter's life,
    /// so one name reaches here through one address. The entry holds its own
    /// count on the name as well, so no other name can come to live at an
    /// address the table knows. Hashing the name was about a hundred
    /// instructions of every interpreted call.
    fn call_ref(&self, name: &Rc<str>, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        let callee = self.callee_of_ref(name);
        self.call_callee(callee, name, args, span, frame)
    }

    /// What a function reference calls, by its address; see `call_ref`.
    ///
    /// The map's probe was most of what a call through a reference paid, so a
    /// direct-mapped table of the last callee per address slot answers first.
    /// A slot holds a key only while the map does, and the map pins the name,
    /// so an address in the table cannot have been handed to another name.
    fn callee_of_ref(&self, name: &Rc<str>) -> Callee<'a> {
        let key = Rc::as_ptr(name) as *const u8 as usize;
        let slot = recent_slot(in_slice(key), RECENT_CALLEES);
        if let (k, Some(callee)) = &self.recent_callees.borrow()[slot] {
            if *k == key {
                return callee.clone();
            }
        }
        self.callee_missed(name, key, slot)
    }

    /// The table did not hold the reference; kept out of line so a hit pays
    /// for no frame.
    #[inline(never)]
    fn callee_missed(&self, name: &Rc<str>, key: usize, slot: usize) -> Callee<'a> {
        let placed = Placed { at: key, stable: in_slice(key) };
        let known = self.callees_by_ref.borrow().get(&placed).map(|(_, callee)| callee.clone());
        if let Some(callee) = &known {
            self.recent_callees.borrow_mut()[slot] = (key, Some(callee.clone()));
        }
        match known {
            Some(callee) => callee,
            None => {
                let callee = self.callee_named(name);
                self.callees_by_ref.borrow_mut().insert(placed, (name.clone(), callee.clone()));
                self.recent_callees.borrow_mut()[slot] = (key, Some(callee.clone()));
                callee
            }
        }
    }

    fn call_callee(
        &self,
        callee: Callee<'a>,
        name: &str,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        if let Callee::Err = callee {
            let [reason] = arity(args, name, span)?;
            let reason = self.force_thunk(reason)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            return Ok(err_value(reason, origin_at(frame, span)));
        }
        let constructed = match callee {
            Callee::EntryType => Some(&self.entry_decl),
            Callee::Constructor(ty) => Some(ty),
            _ => None,
        };
        if let Some(ty) = constructed {
            // A constructor slot is where a knot ties: an argument still
            // being computed is stored rather than demanded, so the cell
            // completes here and the field resolves against it afterwards.
            // Demanding it instead is what made a guarded self-reference
            // blackhole through its own construction.
            let args = args
                .into_iter()
                .map(|a| match pending_knot(&a) {
                    true => Ok(a),
                    false => self.force_thunk(a),
                })
                .collect::<Result<Vec<_>, _>>()?;
            return self.construct(ty, args, span);
        }
        if let Callee::Group(overloads) = callee {
            return self.dispatch(name, &overloads, args, span);
        }
        // Forced where they lie, and only where a thunk is: `force_thunk`
        // hands any other value back as it came. The collect this replaced
        // moved every argument through a `Result` adapter, and whether rustc
        // inlined the adapter's fold moved with edits nowhere near here:
        // kanso#1621 changed only the emitter and the interpreted row rose
        // 4,189,569, all of it this one collect laid out as a call.
        let mut args = args;
        for (at, slot) in args.iter_mut().enumerate() {
            if let Value::Thunk(cell) = slot {
                // A list stores what it is handed: the item `push` adds may be
                // a `list/tie` cell still being made, as a list literal's may.
                if at == 1 && name == "push" && matches!(&*cell.borrow(), ThunkState::Blackhole) {
                    continue;
                }
                let thunk = Value::Thunk(cell.clone());
                *slot = self.force_thunk(thunk)?;
            }
        }
        self.call_builtin(name, args, span, frame)
    }

    /// The ladder `call_named` used to walk, walked once per name per run.
    /// The arms are in the order they were in, because the order decides the
    /// answer: `err` beats a type, and a type beats a group.
    fn callee(&self, name: &str) -> Callee<'a> {
        if name == "err" {
            return Callee::Err;
        }
        if name == "entry" {
            return Callee::EntryType;
        }
        if let Some(ty) = self.types.get(name).copied() {
            // An imported type enrolled under its bare name gives way to a
            // group the program declares: the author's own name wins.
            if ty.origin.is_none() || !self.fns.contains_key(name) {
                return Callee::Constructor(ty);
            }
        }
        match self.fns.get(name) {
            Some(overloads) => Callee::Group(overloads.clone()),
            None => Callee::Builtin,
        }
    }

    fn construct(&self, ty: &TypeDecl, args: Vec<Value>, span: Span) -> EvalResult {
        if !ty.members.is_empty() {
            return Err(RuntimeError {
                at: None,
                message: format!("`{}` is a typeset — it only annotates", ty.name),
                span,
            });
        }
        if let Some(parent) = &ty.parent {
            if args.len() != 1 {
                return Err(RuntimeError {
                    at: None,
                    message: format!("`{}` wraps one {} value", ty.name, parent),
                    span,
                });
            }
            let inner = args.into_iter().next().expect("one arg");
            if is_failure(&inner) {
                return Ok(inner);
            }
            if !type_matches(parent, &inner) {
                return Err(RuntimeError {
                    at: None,
                    message: format!("`{}` wraps {}", ty.name, article(parent)),
                    span,
                });
            }
            let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
            return Ok(Value::Sub { ty: Rc::from(canonical), inner: Rc::new(inner) });
        }
        if args.len() != ty.fields.len() {
            return Err(RuntimeError {
                at: None,
                message: format!(
                    "`{}` has {} field(s), got {} (construction is positional, in the \
                     order the fields are declared)",
                    ty.name,
                    ty.fields.len(),
                    args.len()
                ),
                span,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(merged_failures(&args));
        }
        for ((field, tys, _), arg) in ty.fields.iter().zip(&args) {
            if tys.len() >= 2 && !tys.iter().any(|t| type_matches(t, arg)) {
                return Err(RuntimeError {
                    at: None,
                    message: format!("field `{field}` of `{}` takes {}", ty.name, tys.join(" ")),
                    span,
                });
            }
        }
        let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
        Ok(Value::Record { ty: Rc::from(canonical), fields: Rc::new(RefCell::new(args)) })
    }

    /// The cycle `decl` is on, one more than its number, or 0.
    #[inline(always)]
    fn deep_cycle(&self, decl: &FnDecl) -> u32 {
        let base = self.program.fns.as_ptr() as usize;
        let at = decl as *const FnDecl as usize;
        let i = at.wrapping_sub(base) / std::mem::size_of::<FnDecl>();
        match at >= base {
            true => self.deep_of.get(i).copied().unwrap_or(0),
            false => 0,
        }
    }

    fn dispatch(
        &self,
        name: &str,
        overloads: &Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        self.dispatch_loop(Rc::from(name), Rc::clone(overloads), args, span)
    }

    /// The dispatcher's trampoline: a tail call to a named group re-enters
    /// arm selection here instead of growing the Rust stack, so deep tail
    /// recursion — self or mutual — runs in constant interpreter stack, the
    /// way the native engine's musttail already made it run in constant
    /// machine stack. The oracle should not be the engine that fails first.
    fn dispatch_loop(
        &self,
        name: Rc<str>,
        overloads: Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        // Non-tail recursion nests a Rust frame per call; past this depth
        // the process would die in Rust's own overflow handler instead of
        // reporting. The message matches the one native's parent prints
        // when the compiled child takes the same fall.
        // 10k holds under debug's fat frames and release alike on the
        // 256 MB interpreter thread; recursion past it is non-portable
        // anyway (native's own ceiling is finite and smaller in wasm).
        // a call that enters a cycle growing the stack from outside it sets
        // the record an overflow names, when it is empty, and puts it back
        // on return (see `crate::deep`)
        let running = self.running.get();
        let saved = self.deep_now.get();
        // every overload of a group is on the group's cycle, so the body
        // that runs is on `to` until a tail call hops to another group
        let to = overloads.first().map_or(0, |d| self.deep_cycle(d));
        if to != 0 && to != running && saved == 0 {
            self.deep_now.set(to);
        }
        self.running.set(to);
        self.depth.set(self.depth.get() + 1);
        if self.depth.get() > 10_000 {
            self.depth.set(self.depth.get() - 1);
            // the engines run out of stack at different depths, so the place
            // is the outermost cycle still running, which they share; line 0
            // keeps it from being placed on the way out
            let at = match self.deep_now.get() {
                0 => None,
                d => Some(self.deep_places[d as usize - 1].clone()),
            };
            self.running.set(running);
            self.deep_now.set(saved);
            return Err(RuntimeError {
                at,
                message: "the program ran out of stack: recursion went deeper than the stack holds"
                    .to_string(),
                span: Span::at(0, 0),
            });
        }
        let result = self.dispatch_loop_inner(name, overloads, args, span);
        self.depth.set(self.depth.get() - 1);
        self.running.set(running);
        self.deep_now.set(saved);
        result
    }

    fn dispatch_loop_inner(
        &self,
        name: Rc<str>,
        overloads: Rc<Vec<&'a FnDecl>>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let mut name = name;
        let mut overloads: Rc<Vec<&'a FnDecl>> = overloads;
        let mut args = args;
        let mut span = span;
        // THE SCORE BUFFER OUTLIVES THE DISPATCH, where the bindings buffer
        // cannot. `binds` is moved into `bind_all` and becomes the environment
        // frame, so its allocation is doing work after the dispatch ends;
        // `score` is only read to compare candidates, kept beside `best` while
        // one wins, and then dropped. Declaring it inside the loop paid one
        // allocation and one free per dispatch for a buffer nothing keeps.
        //
        // The loser's buffer is the one held here: the swap below leaves the
        // outgoing candidate's vector in `score`, so what survives the
        // iteration already has capacity and `match_params_into` clears it
        // before the next candidate fills it.
        let mut scores: [Score; 2] = Default::default();
        // AND THE BINDINGS BUFFER COMES BACK TOO, when the frame it became is
        // the dispatcher's to take back. `bind_all` moves `binds` into
        // `Env::Many`, so unlike the score it cannot simply be kept -- the body
        // needs it. What can be kept is the frame AFTER the body has finished
        // with it, and it almost always is: an instrumented build took a `Weak`
        // to the frame and tried to upgrade it after the body returned, and it
        // was dead 173,921 times out of 173,922. The one survivor is a lazy
        // thunk holding its defining environment, which is the case
        // `Rc::try_unwrap` declines below.
        // AND THE FRAME NODE COMES BACK WITH IT. kanso#1543 took the vector
        // out of the dead frame and let the `Rc` allocation go; this keeps the
        // whole node. `Rc::try_unwrap` had to destroy the node to reach the
        // vector inside it, so a dispatch that reclaimed its bindings still
        // paid `Rc::new` for a fresh `RcBox<Env>` on the next one.
        // `Rc::get_mut` reaches the same vector through a handle that stays
        // alive, so the node survives the dispatch that made it.
        //
        // The two are not interchangeable on the uniqueness they demand:
        // `try_unwrap` reads the strong count alone, `get_mut` reads the weak
        // count too. Nothing in this compiler takes a `Weak<Env>` -- the
        // instrumented build above was reverted -- so today they agree, and
        // the difference is written here rather than assumed away.
        let mut pool: Option<Rc<Env>> = None;
        // Who wrote the call being dispatched: the first is the caller's own
        // expression, which places a failure on the way out; each tail call
        // after it was written in the body that handed it here.
        let mut caller = Caller::Outside;
        loop {
            // The second hole in an err's infectiousness: a reader's getter
            // answers the piece before any arm is tried, so an own-hako err
            // and a foreign one read alike, and a getter arm never sees one.
            if let Some(field) = crate::ast::err_reader(&name) {
                if let [arg] = args.as_mut_slice() {
                    let taken = std::mem::replace(arg, Value::NoneV);
                    *arg = self.force_thunk(taken)?;
                    if let Value::ErrV(info) = arg {
                        return Ok(err_read(info, field));
                    }
                }
            }
            let args_len = args.len();
            // A position is scrutinized when any arity-matching arm inspects
            // it (anything but a bare Var/Wildcard); thunks force before
            // matching.
            for (i, arg) in args.iter_mut().enumerate() {
                if !matches!(arg, Value::Thunk(_)) {
                    continue;
                }
                let scrutinized = overloads.iter().any(|decl| {
                    decl.params.len() == args_len
                        && !matches!(
                            decl.params.get(i),
                            Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_))
                        )
                });
                if scrutinized {
                    let taken = std::mem::replace(arg, Value::NoneV);
                    *arg = self.force_thunk(taken)?;
                }
            }
            // One pair of buffers for the whole candidate list. `match_params`
            // used to build both at `Vec::with_capacity` on every candidate and
            // give up the moment a pattern refused, so a candidate that failed
            // on its first parameter had already paid for two allocations --
            // and most candidates fail, because arm selection tries them all.
            // AND ONE PAIR PER DISPATCH, not per candidate. The swap below
            // recycles these across the candidate list, so within one dispatch
            // they allocate once; what they do not survive is the dispatch
            // itself, because the winner's pair is moved out to become the
            // environment frame and the score kept beside `best`. So every
            // dispatch starts at capacity zero and climbs 1, 2, 4 from nothing:
            // `dispatch_loop` reached `grow_one` 225,431 times against 55,711
            // dispatches, four reallocations a call. Asking for the arity up
            // front does NOT allocate less: `__rust_alloc` is called
            // 1,362,891 times against 1,361,559, up 1,332. What goes is the
            // GROWTH path -- `grow_one` and `finish_grow` fall from 464,507
            // and 489,542 to 112,013 and 137,048, 352,494 fewer of each --
            // and with it the capacity arithmetic, the doubling branch and
            // the element copy every regrow makes. 21,582,351 over 352,494
            // is 61.2 instructions a growth.
            //
            // kanso#1538 put this reserve inside `match_params_into`, where it
            // is paid once per CANDIDATE, and it cost 12,017,902 instructions.
            // The allocation is per dispatch, so the reserve belongs here.
            //
            // THE COUNT THAT USED TO BE HERE SAID TWENTY CANDIDATES A DISPATCH
            // and it was `match_one` CALLS divided by dispatches. `match_one`
            // runs once per PARAMETER of each candidate tried, so that ratio is
            // calls a dispatch and bounds the candidates from above; it equals
            // them only if every arm took one argument. Measured on the
            // interpreted corpus, 2026-09-19:
            //
            //     dispatches      175,254
            //     candidates      456,967   2.61 a dispatch
            //     match_one     1,135,058   6.48 a dispatch, 2.48 a candidate
            //
            // The old figure's own arithmetic agrees with the smaller number:
            // 12,017,902 over 175,254 is 68.6 instructions a dispatch, which is
            // 26.3 per reserve at 2.61 candidates and an impossible 3.4 at
            // twenty.
            //
            // `args_len` is exact for `score`, which takes one entry per
            // parameter, and a floor for `binds`, since a `Ctor` pattern can
            // bind its fields and a whole.
            // The pooled node lends its vector out for the candidate list
            // and takes it back below. What stays behind in the node is an
            // empty `Vec`, which owns no allocation, so the node is safe to
            // carry through a dispatch that never binds.
            let mut held_binds: Bindings = match pool.as_mut().and_then(Rc::get_mut) {
                Some(Env::Many(slots, _)) => std::mem::take(slots),
                _ => Vec::new(),
            };
            held_binds.clear();
            held_binds.reserve(args_len);
            // TWO OF EACH BUFFER IN FIXED SLOTS. The next candidate matches
            // into slot 0 and slot 1 holds the best so far, so a candidate
            // that takes the lead swaps the two slots rather than moving an
            // 80-byte tuple of score, declaration and bindings through an
            // `Option` -- which also put a drop of that `Option` on every
            // dispatch. The slots are fixed rather than named by an index
            // that flips: an index made every access to them computed, and
            // that cost 3,766,635 instructions more on the interpreted corpus
            // than the swaps it saved. The scores stay with the loop, as the
            // one score did; the bindings start each dispatch from the pooled
            // frame's vector.
            let mut pair: [Bindings; 2] = [held_binds, Vec::new()];
            let mut best: Option<&FnDecl> = None;
            // ONE ARM OF BARE NAMES HAS NOTHING TO CHOOSE. A name refuses a
            // failure and takes anything else, so such an arm wins exactly
            // when no argument is a failure, and with no rival its score is
            // never read. Its bindings are the names in order, each holding
            // `none` until `bind_moved` puts the argument there, which is
            // what `match_params_into` would have pushed.
            let lone = match overloads.as_slice() {
                [decl]
                    if decl.params.len() == args_len
                        && decl.params.iter().all(|p| matches!(p, Pattern::Var(..)))
                        && !args.iter().any(is_failure) =>
                {
                    Some(*decl)
                }
                _ => None,
            };
            if let Some(decl) = lone {
                let binds = &mut pair[0];
                if binds.capacity() == 0 {
                    *binds = self.spare_binds.borrow_mut().pop().unwrap_or_default();
                }
                for pattern in &decl.params {
                    if let Pattern::Var(name, _) = pattern {
                        binds.push((name.clone(), Value::NoneV));
                    }
                }
                best = Some(decl);
                pair.swap(0, 1);
            }
            for decl in overloads.iter().filter(|_| lone.is_none()) {
                if decl.params.len() != args.len() {
                    continue;
                }
                if pair[0].capacity() == 0 {
                    pair[0] = self.spare_binds.borrow_mut().pop().unwrap_or_default();
                }
                if !match_params_into(&decl.params, &args, &mut scores[0], &mut pair[0]) {
                    continue;
                }
                let replace = match best {
                    Some(_) => scores[0].beats(&scores[1]),
                    None => true,
                };
                if replace {
                    best = Some(decl);
                    pair.swap(0, 1);
                    scores.swap(0, 1);
                }
            }
            let [mut spare_binds, mut binds] = pair;
            // The buffer the losers matched into goes back for the next
            // dispatch rather than to the allocator.
            if spare_binds.capacity() > 0 {
                spare_binds.clear();
                let mut spare = self.spare_binds.borrow_mut();
                if spare.len() < 64 {
                    spare.push(spare_binds);
                }
            }
            match best {
                Some(decl) => {
                    bind_moved(&decl.params, &mut args, &mut binds);
                    // One frame for the whole parameter list. Pushing a node
                    // per binding made the chain as long as the arguments, and
                    // the walk paid for that on every name the body mentions.
                    let env = match binds.is_empty() {
                        true => None,
                        false => match pool.take() {
                            // The node is unique -- it was only pooled after
                            // `get_mut` said so, and nothing has been handed a
                            // copy since -- so this writes the winner's
                            // bindings into the frame the last dispatch left.
                            // `spare` carries them back out if it ever is not,
                            // which keeps a refused `get_mut` a lost reuse
                            // rather than a panic on a live interpreter.
                            Some(mut rc) => {
                                let mut spare = Some(binds);
                                if let Some(Env::Many(slots, _)) = Rc::get_mut(&mut rc) {
                                    if let Some(won) = spare.take() {
                                        *slots = won;
                                    }
                                }
                                match spare {
                                    None => Some(rc),
                                    Some(binds) => bind_all(None, binds),
                                }
                            }
                            None => bind_all(None, binds),
                        },
                    };
                    // One handle for the dispatcher beside the one the body
                    // gets. It costs a reference count either way, and it is
                    // what makes the frame reclaimable rather than freed.
                    let held = env.clone();
                    // THE ARGUMENT VECTOR IS A HOLDER, and nothing below
                    // reads it. `match_one` cloned each matched value into
                    // `binds`, so every container argument is now pointed at
                    // twice: once from here and once from the environment the
                    // body will run in. kanso#1497 measured what that costs:
                    // `append`'s accumulator was unique on 3.6% of 36,966
                    // calls, so the builder copied 180 MB it mostly did not
                    // need to, and `Rc::try_unwrap` could not fire.
                    // `bind_moved` left `NoneV` where it took a value, and a
                    // `NoneV` owns nothing, so `release` forgets it rather
                    // than handing it to `Value`'s drop one call at a time.
                    while let Some(arg) = args.pop() {
                        release(arg);
                    }
                    let flowed = self.eval_body_flow(decl, env);
                    // The body has finished with the frame. If nothing it made
                    // kept a handle -- a lazy thunk is the only thing that
                    // does -- the vector inside comes back for the next
                    // dispatch instead of being freed and allocated again.
                    if let Some(mut rc) = held {
                        if Rc::get_mut(&mut rc).is_some() {
                            pool = Some(rc);
                        }
                    }
                    // A closure the tail called runs here, in this loop, so a
                    // group and a lambda that hand each other on cost no stack.
                    let mut flowed = flowed?;
                    let mut lambda: Option<Rc<ClosureData>> = None;
                    while let Flow::Closure(next, next_args, next_span) = flowed {
                        // a call that fails before the lambda runs was written
                        // in this body, or in the lambda that handed it on
                        flowed = self.closure_flow(&next, next_args, next_span).map_err(|e| {
                            match &lambda {
                                Some(c) => placed(e, &c.frame),
                                None => placed(e, &self.frame_for(decl)),
                            }
                        })?;
                        lambda = Some(next);
                    }
                    match flowed {
                        Flow::Closure(..) => unreachable!("closures run in the loop above"),
                        Flow::Done(value) => return Ok(value),
                        Flow::Tail(next, group, next_args, next_span) => {
                            // a tail call that enters a cycle growing the
                            // stack from outside it is made as a call, which
                            // keeps a frame to put the record back in
                            let from = self.running.get();
                            let to = group.first().map_or(0, |d| self.deep_cycle(d));
                            if to != 0 && to != from {
                                // a lambda keeps its jump, as its compiled
                                // body does: it records and puts nothing back
                                if lambda.is_none() {
                                    return self.dispatch_loop(next, group, next_args, next_span);
                                }
                                if self.deep_now.get() == 0 {
                                    self.deep_now.set(to);
                                }
                            }
                            self.running.set(to);
                            // the call was written in this body, or in the
                            // lambda its tail handed on to, so a dispatch of
                            // it that fails is placed there
                            caller = match lambda {
                                Some(c) => Caller::Lambda(c),
                                None => Caller::Decl(decl),
                            };
                            overloads = group;
                            // `Flow::Tail` already carries the name as an
                            // `Rc<str>`, and every use of it below this loop
                            // reads it as `&str`. Moving it costs a pointer
                            // where copying it cost an allocation a hop.
                            name = next;
                            args = next_args;
                            span = next_span;
                        }
                    }
                }
                None => {
                    // a getter that matched nothing is a field error, in the
                    // reader's words: they wrote `x.name`, not `Get_name x`
                    let subject = args.first().cloned();
                    return match args.into_iter().find(is_failure) {
                        Some(bad) => Ok(hop(bad, &name)),
                        None => Err(RuntimeError {
                            at: match &caller {
                                Caller::Outside => None,
                                Caller::Decl(d) => origin_at(&self.frame_for(d), span).at,
                                Caller::Lambda(c) => origin_at(&c.frame, span).at,
                            },
                            message: match (crate::ast::getter_field(&name), subject) {
                                (Some(field), Some(Value::Record { ty, .. })) => {
                                    format!("`{ty}` has no field `{field}`")
                                }
                                (Some(_), Some(value)) => format!(
                                    "`.` reads a field of a record, not {}",
                                    render(self, &value, true)
                                ),
                                _ => format!(
                                    "no overload of `{}` matches these arguments",
                                    crate::ast::spoken(&name)
                                ),
                            },
                            span,
                        }),
                    };
                }
            }
        }
    }

    /// What a worded chain step answers once its subject has settled. This
    /// is the whole difference between the three: `bind` calls the callback
    /// on a value and lets a failure past, `rescue` calls it on a failure and
    /// lets a value past, and `annotate` reads the same channel as `rescue`
    /// but re-wraps the answer as an err with the original as cause — which
    /// is why it cannot resurrect, with nothing checking that it doesn't.
    fn worded_step(
        &self,
        word: Word,
        yielded: Value,
        callee: &Value,
        raised: &Raised,
        span: Span,
    ) -> EvalResult {
        let Value::ErrV(cause) = yielded.clone() else {
            return match word {
                Word::Bind => self.call_decided(callee, yielded, span),
                Word::Rescue | Word::Annotate => Ok(yielded),
            };
        };
        match word {
            Word::Bind => Ok(yielded),
            // The foreign-only licence, at the word. A rescue written in the
            // package that raised the failure hands it on without entering
            // the callback; a failure nobody can be held responsible for —
            // no frame, or merged out of several — passes the test.
            Word::Rescue if own_failure(&cause, raised) => Ok(yielded),
            Word::Rescue => self.call_decided(callee, yielded, span),
            Word::Annotate => match self.call_decided(callee, yielded, span)? {
                // the callback failed on its own account, and that err
                // already carries what it was told; wrapping it again would
                // report the same failure twice
                answer if is_failure(&answer) => Ok(answer),
                reason => Ok(Value::ErrV(Rc::new(ErrInfo {
                    reason,
                    origin: raised.at.clone(),
                    hako: raised.hako.clone(),
                    hops: Vec::new(),
                    cause: Some(cause),
                    merged: false,
                }))),
            },
        }
    }

    /// Ordinary application without its argument guard, for the callers that
    /// have already decided about the argument themselves.
    ///
    /// `rescue` and `annotate` are two: a failure handed to a lambda
    /// propagates instead of entering the body, which is what threads failures
    /// through a chain for free, and they are the steps that must get past it.
    /// A group needs no help; dispatch already routes an err to the arm
    /// written for it.
    ///
    /// `bind` is the third, and it is there because the guard is redundant
    /// rather than in the way: the chain step has already tested the yielded
    /// value for failure — that test IS the difference between the worded
    /// steps — so `call_closure`'s copy is work nobody needs. The C twin
    /// carries the same name and the same two reasons.
    fn call_decided(&self, callee: &Value, arg: Value, span: Span) -> EvalResult {
        match callee {
            Value::Closure(c) if c.params.len() == 1 => {
                let env = bind(c.env.clone(), c.params[0].clone(), arg);
                self.eval(&c.body, &env, &c.frame).map_err(|e| placed(e, &c.frame))
            }
            // a callback compiled into another engine's table. Without this
            // arm the argument goes through that engine's guarded call and a
            // failure comes straight back, so a rescue scheduled as a green
            // thread would skip its callback — which is what the page did.
            Value::TableFn(handle) => foreign_call(*handle, vec![arg], span, true),
            other => self.call(other.clone(), vec![arg], span, &None),
        }
    }

    /// Whether the linearity analysis proved this call site extends a
    /// container nothing else will read.
    ///
    /// The key is the declaration's file and the call's line and column, which
    /// is what `codegen.rs` uses at its own push arm. A frame the wasm host
    /// built carries no file and answers no, which is the safe direction.
    ///
    /// The analysis keys its set by the file's path, and hashing the path on
    /// every container call was most of what the question cost. Each frame
    /// gathers its own file's sites once, as (line, column), and asks those.
    fn writes_in_place(&self, span: Span, frame: &Frame) -> bool {
        let Some(site) = frame else { return false };
        let sites = site.in_place.get_or_init(|| {
            let all = self.in_place.get_or_init(|| self.linearity.in_place_pushes());
            all.iter()
                .filter(|(file, _, _)| **file == *site.file)
                .map(|(_, l, c)| (*l, *c))
                .collect()
        });
        sites.contains(&(span.line as usize, span.col as usize))
    }

    /// Whether a write whose value has exactly one other holder may take it.
    ///
    /// Asked only where `writes_in_place` said no and the count says the
    /// argument and one binding hold the value. `linear::moved_writes` proves
    /// that binding is the name this write moves, which nothing reads again,
    /// so the count is the rest of the proof `in_place_pushes` could not make
    /// from the callers.
    /// The byte where character `n` (counted from 0) of `text` starts, or the
    /// string's length when `n` is its count, walking on from `text_cursor`
    /// when it holds the same string at or before `n`. `None` is past the end.
    fn char_offset(&self, text: &Text, n: usize) -> Option<usize> {
        let mut cursor = self.text_cursor.borrow_mut();
        let (mut k, mut b) = match &*cursor {
            Some((held, k, b)) if held.same(text) && *k <= n => (*k, *b),
            _ => (0, 0),
        };
        while k < n {
            b += text[b..].chars().next()?.len_utf8();
            k += 1;
        }
        *cursor = Some((text.clone(), k, b));
        Some(b)
    }

    /// Whether `linear::string_builders` licensed this template to join onto
    /// the string its first part names. Keyed as `writes_in_place` is.
    fn joins_here(&self, span: Span, frame: &Frame) -> bool {
        let Some(site) = frame else { return false };
        let sites = site.joins.get_or_init(|| {
            let all = self.joins.get_or_init(|| self.linearity.string_joins());
            all.iter()
                .filter(|(file, _, _)| **file == *site.file)
                .map(|(_, l, c)| (*l, *c))
                .collect()
        });
        sites.contains(&(span.line as usize, span.col as usize))
    }

    fn moved_here(&self, span: Span, frame: &Frame) -> bool {
        let Some(site) = frame else { return false };
        let sites = site.moved.get_or_init(|| {
            let all = self.moved.get_or_init(|| crate::linear::moved_writes(self.program));
            all.iter()
                .filter(|(file, _, _)| **file == *site.file)
                .map(|(_, l, c)| (*l, *c))
                .collect()
        });
        let at = (span.line as usize, span.col as usize);
        sites.contains(&at)
    }

    /// Take a container's contents where the analysis proved nobody else will
    /// read them.
    ///
    /// SAFETY, and it is the analysis rather than the refcount. `Rc` refuses
    /// `&mut` above one holder, and the other holder here is the environment
    /// node binding the accumulator's name -- so `try_unwrap` fails on 96.99%
    /// of this corpus's container calls and every one of them clones. What
    /// `linear::in_place_pushes` proves is that no later read of that binding
    /// exists, which is why the COMPILED engine writes through the same
    /// pointer at the same sites and has since kanso#1359. Taking the contents
    /// leaves an empty vector behind for a holder that never looks.
    ///
    /// The differential law is what keeps this honest rather than argued: the
    /// two engines must agree, and five specs go red the moment this fires
    /// where the analysis did not say so --
    /// `a_list_held_twice_is_not_pushed_into`,
    /// `a_builder_passed_twice_through_a_wrapper_is_not_written_through`,
    /// `an_imported_builder_held_twice_is_not_written_through`,
    /// `micro_corpus_agrees_across_engines` and
    /// `a_unique_container_is_extended_in_place`. That was measured by
    /// building the ungated version and watching them fail.
    fn taken_in_place<T>(rc: &Rc<T>) -> T
    where
        T: Default,
    {
        let slot = Rc::as_ptr(rc) as *mut T;
        // SAFETY: see above. The pointer is derived from a live `Rc` this
        // frame holds, so it is valid and aligned for `T`, and no other
        // reference into it is live across this line -- the only other holder
        // is an environment binding the analysis proved dead.
        unsafe { std::mem::take(&mut *slot) }
    }

    /// The same proof, used to write through the pointer instead of taking
    /// the contents out of it. The container stays in the allocation it was
    /// born in, so a push hands back the `Rc` it was given rather than boxing
    /// the vector afresh, and the dead binding's holder sees the grown vector
    /// where `taken_in_place` left it an empty one -- which it never reads.
    #[allow(clippy::mut_from_ref)]
    fn written_in_place<T>(rc: &Rc<T>) -> &mut T {
        // SAFETY: as `taken_in_place`. The `&mut` lives for one write, and no
        // other reference into the container is live across it.
        unsafe { &mut *(Rc::as_ptr(rc) as *mut T) }
    }
}

include!("eval/builtins.rs");

impl<'a> Interp<'a> {
    /// A condition that is neither true nor false can still be a subtype of
    /// bool, which is a condition like the bool it wraps. Only the arm that
    /// would refuse asks, so a plain bool pays nothing for it.
    fn wrapped_condition(&self, other: Value, span: Span) -> Result<bool, RuntimeError> {
        match sub_base(other.clone()) {
            Value::True => Ok(true),
            Value::False => Ok(false),
            _ => Err(RuntimeError {
                at: None,
                message: format!(
                    "an if condition is true or false, got {}",
                    render(self, &other, false)
                ),
                span,
            }),
        }
    }

    fn builtin_if(&self, args: Vec<Value>, span: Span) -> EvalResult {
        let [cond, then_branch, else_branch] = arity(args, "if", span)?;
        let cond = self.force(cond)?;
        match cond {
            Value::True => self.force(then_branch),
            Value::False => self.force(else_branch),
            bad if is_failure(&bad) => Ok(bad),
            other => match self.wrapped_condition(other, span)? {
                true => self.force(then_branch),
                false => self.force(else_branch),
            },
        }
    }

    /// One rendering rule for every engine's template path: records, none,
    /// and io route through the ambient render/to_string group so user arms
    /// win; primitives keep the direct renderer (coherence licenses it — no
    /// arm can exist for them). The outer Err is a runtime fault; the inner
    /// Err is an err value the whole template must return.
    pub fn render_interpolated(&self, value: Value) -> Result<Result<String, Value>, RuntimeError> {
        let rendered = match (&value, self.fns.get("render/to_string")) {
            // a subtype is user-owned, so an arm can exist for it — the
            // coherence licence that keeps primitives on the direct path
            // does not reach a wrapper
            (
                Value::Record { .. }
                | Value::NoneV
                | Value::Done
                | Value::Desc(_)
                | Value::Sub { .. },
                Some(overloads),
            ) => {
                let overloads = overloads.clone();
                let result =
                    self.dispatch("render/to_string", &overloads, vec![value], Span::at(0, 0))?;
                if matches!(result, Value::ErrV(_)) {
                    return Ok(Err(result));
                }
                match result {
                    Value::Str(s) => s.into_string(),
                    other => render(self, &other, false),
                }
            }
            _ => render(self, &value, false),
        };
        Ok(Ok(rendered))
    }

    fn force(&self, value: Value) -> EvalResult {
        match value {
            // A nullary closure's body can answer with a thunk, and scrutiny
            // wants a value rather than the next cell.
            Value::Closure(c) if c.params.is_empty() => {
                let answered =
                    self.eval(&c.body, &c.env, &c.frame).map_err(|e| placed(e, &c.frame))?;
                self.force_thunk(answered)
            }
            other => self.force_thunk(other),
        }
    }

    /// Scrutiny reaches through a thunk: run the pending computation once,
    /// overwrite the cell, drop the captures. Never touches nullary closures
    /// — those are `if`'s deferred branches, forced only by `if` itself.
    /// A constant that names itself is computed once, behind a cell. The
    /// mention that arrives mid-computation reads Blackhole and hands back the
    /// cell unforced, so a storing position keeps a reference the later read
    /// resolves.
    /// A list only stores what it is given, so an element naming a constant
    /// still being computed waits rather than demanding a value that does not
    /// exist yet.
    /// Runs a tie's maker over its keys and answers the map of what it made,
    /// or the first failure a maker answered.
    fn make_tied(
        &self,
        serial: i64,
        keys: &[Value],
        maker: Value,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        let mut nodes = Entries::new();
        for key in keys {
            let held = vec![Value::int(serial), key.clone()];
            let r =
                Value::Partial(Rc::new(Value::FnRef(Rc::from("builtin_tie_ref"))), Rc::new(held));
            let node = self.call(maker.clone(), vec![key.clone(), r], span, frame)?;
            let node = self.force_thunk(node)?;
            if is_failure(&node) {
                return Ok(node);
            }
            nodes.insert(map_key(key.clone(), span)?, node);
        }
        Ok(Value::Map(Rc::new(nodes)))
    }

    fn stored(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        if !self.awaits_a_knot(expr) {
            return self.eval(expr, env, frame);
        }
        Ok(Value::Thunk(Rc::new(RefCell::new(ThunkState::Pending {
            expr: expr.clone(),
            env: env.clone(),
            frame: frame.clone(),
        }))))
    }

    fn awaits_a_knot(&self, expr: &Expr) -> bool {
        fn names<'e>(expr: &'e Expr, out: &mut Vec<&'e str>) {
            if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
                out.push(n);
            }
            crate::for_each_child(expr, |child| names(child, out));
        }
        let mut mentioned = Vec::new();
        names(expr, &mut mentioned);
        let knots = self.knots.borrow();
        mentioned
            .iter()
            .any(|n| knots.get(*n).is_some_and(|c| matches!(&*c.borrow(), ThunkState::Blackhole)))
    }

    fn knotted(&self, name: &str, constant: &'a FnDecl) -> EvalResult {
        if let Some(cell) = self.knots.borrow().get(name) {
            let forced = match &*cell.borrow() {
                ThunkState::Forced(v) => Some(v.clone()),
                _ => None,
            };
            return Ok(forced.unwrap_or_else(|| Value::Thunk(Rc::clone(cell))));
        }
        // Ruled 2026-08-24: a demanded knot counts, and this is the side that
        // moves. The cell below is the only way a self-naming constant can be
        // built, so building it is an allocation the native engine has always
        // reported; the counters are semantics rather than a resource
        // heuristic, and two engines may not disagree about what the work was.
        // The early return above allocates nothing and does not count.
        // Ruled 2026-08-24: a demanded knot counts, and this is the side that
        // moves. Only a constant that reaches itself is one native builds a
        // cell for, and every constant arrives here, so the count is filtered
        // by the emitter's own predicate rather than by getting this far. The
        // early return above allocates nothing and does not count.
        if self
            .cycles
            .get_or_init(|| crate::codegen::knotted_constants(self.program))
            .contains(name)
        {
            self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
        }
        let cell = Rc::new(RefCell::new(ThunkState::Blackhole));
        self.knots.borrow_mut().insert(name.to_string(), Rc::clone(&cell));
        let value = self.eval_body_of(constant, None)?;
        // A body that answers this very cell has computed nothing: completing
        // it would tie the knot to itself and every later demand would chase
        // the loop instead of finding a value. The blackhole is the answer,
        // and it is the one a guarded slot never reaches.
        if matches!(&value, Value::Thunk(answered) if Rc::ptr_eq(answered, &cell)) {
            return Err(RuntimeError {
                at: None,
                message: "a lazy binding demands its own value".to_string(),
                span: Span::at(0, 0),
            });
        }
        *cell.borrow_mut() = ThunkState::Forced(value.clone());
        Ok(value)
    }

    /// Call a value from outside the evaluator. The page reaches this with a
    /// callable the interpreter made and handed out, which is the `ref` a
    /// `list/tie` hands its maker.
    pub fn call_value(&self, callee: Value, args: Vec<Value>, span: Span) -> EvalResult {
        self.call(callee, args, span, &None)
    }

    /// Force a cell from outside the evaluator.
    pub fn demand(&self, value: &Value) -> EvalResult {
        self.force_thunk(value.clone())
    }

    /// The description a bind's callback builds when it ignores what it is
    /// handed, for the plan. Only `_ -> step` qualifies: any other callback
    /// reads its argument, and the plan has no value to give it.
    pub fn ignoring_step(&self, callee: &Value) -> Option<Rc<Desc>> {
        let Value::Closure(c) = callee else { return None };
        // `a .> (_ -> step)` arrives as a closure over `__piped` whose body
        // applies the written lambda to it, so the lambda is one level in.
        let ignores = |params: &[(String, Span)]| params.len() == 1 && params[0].0 == "_";
        let ignored = match &c.body {
            Expr::App { head, args, .. } if args.len() == 1 => {
                matches!(head.as_ref(), Expr::Lambda { params, .. } if ignores(params))
            }
            _ => false,
        };
        if !ignored {
            return None;
        }
        match self.call_decided(callee, Value::Done, Span::at(0, 0)).and_then(|v| self.demand(&v)) {
            Ok(Value::Desc(d)) => Some(d),
            _ => None,
        }
    }

    /// Most values are not cells, and asking was a call of its own: 10.6
    /// million instructions of the interpreted corpus in this function's own
    /// frame. The question inlines at every caller and the loop stays out of
    /// line.
    #[inline]
    fn force_thunk(&self, value: Value) -> EvalResult {
        match value {
            Value::Thunk(_) => self.force_cell(value),
            other => Ok(other),
        }
    }

    #[inline(never)]
    fn force_cell(&self, value: Value) -> EvalResult {
        let mut value = value;
        loop {
            let Value::Thunk(cell) = value else {
                return Ok(value);
            };
            self.thunk_stats.forces.set(self.thunk_stats.forces.get() + 1);
            let state = std::mem::replace(&mut *cell.borrow_mut(), ThunkState::Blackhole);
            match state {
                ThunkState::Forced(v) => {
                    *cell.borrow_mut() = ThunkState::Forced(v.clone());
                    value = v;
                }
                ThunkState::Pending { expr, env, frame } => {
                    self.thunk_stats.evals.set(self.thunk_stats.evals.get() + 1);
                    let v = self.eval(&expr, &env, &frame).map_err(|e| placed(e, &frame))?;
                    *cell.borrow_mut() = ThunkState::Forced(v.clone());
                    value = v;
                }
                ThunkState::Blackhole => {
                    return Err(RuntimeError {
                        at: None,
                        message: "a lazy binding demands its own value".to_string(),
                        span: Span::at(0, 0),
                    })
                }
            }
        }
    }
}

/// A value that is a knot mid-construction: its cell is entered and has not
/// answered yet. Only a constructor stores one — everywhere else demanding it
/// is the blackhole, which is what the blackhole is for.
fn pending_knot(value: &Value) -> bool {
    let Value::Thunk(cell) = value else { return false };
    matches!(&*cell.borrow(), ThunkState::Blackhole)
}

/// A bitwise operand. Failures travel through as themselves, the way every
/// builtin lets them; anything else is the type complaint native prints, in
/// the same words. Bits are counted over a machine word, so a bignum is
/// refused on every engine.
fn whole(v: Value, name: &str, span: Span) -> Result<Result<i64, Value>, RuntimeError> {
    if is_failure(&v) {
        return Ok(Err(v));
    }
    let said = spelled(name);
    match &v {
        Value::Int(n) => match n.to_i64() {
            Some(w) => Ok(Ok(w)),
            None => Err(RuntimeError {
                at: None,
                message: format!("{said} takes whole numbers that fit 64 bits"),
                span,
            }),
        },
        other => Err(RuntimeError {
            at: None,
            message: format!("{said} takes whole numbers, got {}", render_demanded(other, false)),
            span,
        }),
    }
}

/// The name the reader wrote, from the builtin the wrapper forwards to.
fn spelled(name: &str) -> &str {
    name.strip_prefix("bit_").unwrap_or(name)
}

/// The name of a chain word, or nothing. One table so the builtin, the
/// checker's ambient list and a reader all read the same three.
pub fn chain_word(name: &str) -> Option<Word> {
    match name {
        "bind" => Some(Word::Bind),
        "rescue" => Some(Word::Rescue),
        "annotate" => Some(Word::Annotate),
        _ => None,
    }
}

fn arity<const N: usize>(
    args: Vec<Value>,
    name: &str,
    span: Span,
) -> Result<[Value; N], RuntimeError> {
    match <[Value; N]>::try_from(args) {
        Ok(array) => Ok(array),
        Err(actual) => Err(RuntimeError {
            at: None,
            message: format!("`{name}` takes {N} argument(s), got {}", actual.len()),
            span,
        }),
    }
}

/// Whether a failure was raised by the package a rescue is written in. The
/// 2026-09-15 ruling made a bare err data — an `(err _)` arm matches it
/// anywhere — and kept the 2026-08-29 gavel's foreign-only rescue licence,
/// so the one place provenance is still asked is the word. `None` on either
/// side always passes: an err with no frame (the wasm host) or merged out of
/// several belongs to no single package, and a site with no frame holds
/// nobody responsible.
fn own_failure(cause: &ErrInfo, site: &Raised) -> bool {
    match (&cause.hako, &site.hako) {
        (Some(raiser), Some(here)) => raiser == here,
        _ => false,
    }
}

/// Take a container's contents to change, copying only when somebody else
/// still points at it.
///
/// `push`, `put`, `concat` and `append` each answer with a new container, and
/// the copy that makes is only necessary while the old one is still reachable.
/// `Rc::try_unwrap` asks exactly that, and a value no other reference points
/// at cannot be observed changing, so the two arms are the same function.
///
/// kanso#1497 measured this alone and declined it: `append`'s accumulator was
/// unique on 1,326 of 36,966 calls, 3.6%, because the argument vector and the
/// wrapper's environment both held it. The argument vector is cleared before
/// the body runs now, which is the other half.
fn taken<T: Clone>(rc: Rc<T>) -> T {
    match Rc::try_unwrap(rc) {
        Ok(owned) => owned,
        Err(shared) => (*shared).clone(),
    }
}

/// `taken` for a vector that is about to GROW by a known amount.
///
/// `Vec::clone` allocates capacity exactly equal to length, so the clone arm of
/// `taken` hands back a full vector and the push or extend that follows must
/// reallocate and copy the whole buffer a second time. Every shared `push` and
/// every shared `append` was paying for its contents twice: once to clone, once
/// to grow.
///
/// Sizing the clone for what is coming pays once. The unique arm is untouched,
/// because a vector nobody else points at may already have spare capacity and
/// reserving on it would be the same mistake in the other direction.
fn taken_to_grow<T: Clone>(rc: Rc<Vec<T>>, extra: usize) -> Vec<T> {
    match Rc::try_unwrap(rc) {
        Ok(owned) => owned,
        Err(shared) => {
            let mut out = Vec::with_capacity(shared.len() + extra);
            out.extend(shared.iter().cloned());
            out
        }
    }
}

include!("eval/patterns.rs");

pub fn index_value(container: Value, index: Value, span: Span) -> EvalResult {
    if is_failure(&container) {
        return Ok(container);
    }
    if is_failure(&index) {
        return Ok(index);
    }
    match (&container, &index) {
        (Value::List(items), Value::Int(i)) => {
            let idx = usize::try_from(i).ok();
            Ok(match idx.filter(|i| *i >= 1 && *i <= items.len()) {
                Some(i) => items[i - 1].clone(),
                None => Value::NoneV,
            })
        }
        (Value::Bytes(items), Value::Int(i)) => {
            let idx = usize::try_from(i).ok();
            Ok(match idx.filter(|i| *i >= 1 && *i <= items.len()) {
                Some(i) => Value::int(items[i - 1]),
                None => Value::NoneV,
            })
        }
        // An ASCII string answers by byte; one with a wider character walks.
        (Value::Str(text), Value::Int(i)) if text.is_ascii() => {
            let idx = usize::try_from(i).ok().and_then(|i| i.checked_sub(1));
            Ok(match idx.filter(|i| *i < text.len()) {
                Some(i) => Value::Str(Text::from(&text[i..i + 1])),
                None => Value::NoneV,
            })
        }
        (Value::Str(text), Value::Int(i)) => {
            let idx = usize::try_from(i).ok();
            Ok(match idx.and_then(|i| i.checked_sub(1)).and_then(|i| text.chars().nth(i)) {
                Some(c) => Value::Str(Text::from(c.to_string())),
                None => Value::NoneV,
            })
        }
        (Value::Map(entries), Value::Int(_) | Value::Str(_)) => {
            let key = map_key(index, span)?;
            Ok(entries.get(&key).cloned().unwrap_or(Value::NoneV))
        }
        // An index is `at` written as syntax, and `at`, like every builtin,
        // reads through a subtype to its base value. A subtype matches none of
        // the arms above, so it is unwrapped here, where the reads that do
        // match never pay for the question. Here rather than at each caller,
        // because the browser's runtime calls this too.
        (Value::Sub { .. }, _) | (_, Value::Sub { .. }) => {
            let base_of = |v: Value| match v {
                Value::Sub { .. } => sub_base(v),
                other => other,
            };
            index_value(base_of(container), base_of(index), span)
        }
        (base, _) => Err(RuntimeError {
            at: None,
            message: format!(
                "indexing takes a list or string with a 1-based position, or a map with a key{}",
                lazy_hint(base)
            ),
            span,
        }),
    }
}

fn slice_range(len: usize, from: usize, to: usize) -> Option<std::ops::Range<usize>> {
    match from >= 1 && from <= to && to <= len {
        true => Some(from - 1..to),
        false => None,
    }
}

fn map_key(value: Value, span: Span) -> Result<MapKey, RuntimeError> {
    match value {
        Value::Int(n) => Ok(MapKey::Int(n)),
        Value::Str(s) => Ok(MapKey::Str(s)),
        other => Err(RuntimeError {
            at: None,
            message: format!("{} is not usable as a map key", render_demanded(&other, true)),
            span,
        }),
    }
}

pub fn is_failure(value: &Value) -> bool {
    matches!(value, Value::ErrV(_))
}

/// Whether an operand sends an operator to its user arms. A record does, and
/// so does a subtype of one, which is the same value wearing a narrower name.
/// An err does not: it carries the operation's failure past the operator
/// rather than answering it.
pub fn routes_to_arms(value: &Value) -> bool {
    match value {
        Value::Record { .. } => true,
        // a subtype is user-owned whatever it wraps, so an arm written for
        // `type money int` is reachable where the int's builtin would
        // otherwise answer first and the arm would never run
        Value::Sub { .. } => true,
        _ => false,
    }
}

thread_local! {
    static TYPESETS: std::cell::RefCell<Map<String, Vec<String>>> =
        std::cell::RefCell::new(Map::default());
}

/// The typeset ladder rung: below every concrete type, above the bare
/// generic. Encoded as a fixed depth so the score arithmetic orders it.
const TYPESET_DEPTH: u8 = 50;

pub fn type_matches(ty: &str, arg: &Value) -> bool {
    type_match_depth(ty, arg).is_some()
}

/// The value's own outermost type only — used by the upcast, which walks
/// the chain itself one level at a time.
fn type_matches_exact(ty: &str, arg: &Value) -> bool {
    match arg {
        Value::Sub { ty: vty, .. } => ty == &**vty,
        other => type_match_depth(ty, other) == Some(0),
    }
}

/// How far up the subtype chain the annotation sits: an exact match is 0,
/// the immediate parent 1, and so on — the dispatch score prefers nearer.
fn type_match_depth(ty: &str, arg: &Value) -> Option<u8> {
    let member_hit = TYPESETS.with(|t| {
        t.borrow().get(ty).map(|members| members.iter().any(|m| type_match_depth(m, arg).is_some()))
    });
    if let Some(hit) = member_hit {
        return hit.then_some(TYPESET_DEPTH);
    }
    if let Value::Sub { ty: vty, inner } = arg {
        if ty == &**vty {
            return Some(0);
        }
        return type_match_depth(ty, inner).map(|d| d.saturating_add(1));
    }
    if crate::ast::is_effect_type(ty) {
        return matches!(arg, Value::Desc(_)).then_some(0);
    }
    if ty.ends_with("[]") {
        return matches!(arg, Value::List(_)).then_some(0);
    }
    if ty.contains('[') {
        return matches!(arg, Value::Map(_)).then_some(0);
    }
    let ok = match (ty, arg) {
        ("some", Value::NoneV) => false,
        // a failure is not "some value", it is the absence of one. Answering
        // true here made `_:some` an unmarked rescue: an arm that names no
        // err at all took a foreign failure and returned whatever it liked,
        // which is the one door the two-universe rule exists to watch.
        ("some", Value::ErrV(_)) => false,
        ("some", _) => true,
        ("int", Value::Int(_)) => true,
        ("float64", Value::Float(_)) => true,
        ("string", Value::Str(_)) => true,
        ("bool", Value::True | Value::False) => true,
        ("true", Value::True) => true,
        ("false", Value::False) => true,
        ("none", Value::NoneV) => true,
        ("done", Value::Done) => true,
        ("err", Value::ErrV(_)) => true,
        (name, Value::Record { ty, .. }) => name == &**ty,
        _ => false,
    };
    ok.then_some(0)
}

include!("eval/values.rs");

include!("eval/run.rs");

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of the constant `main` in this source. It is an ordinary name
    /// here, chosen because these fixtures read as programs; the entry a real
    /// program runs is the compiler's and cannot be written.
    fn run_main(source: &str) -> Value {
        let lexed = crate::lexer::lex(source).expect("lexes");
        let mut program = crate::parser::parse(&lexed).expect("parses");
        let diags = crate::check::check(&mut program, false);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let interp = Interp::new(&program);
        interp
            .run_named("main")
            .expect("a constant named main")
            .map_err(|e| e.message)
            .expect("runs")
    }

    /// `Score::beats` orders as the `Vec<u8>` it replaced, at every length
    /// either side of the inline width.
    #[test]
    fn a_score_compares_as_its_ranks_in_order() {
        let mut seed: u32 = 0x2545_f491;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            [10u8, 100, 99, 200][(seed % 4) as usize]
        };
        for len in 1..=SCORE_INLINE + 4 {
            for _ in 0..200 {
                // b differs from a in one position, inline or spilled, so a
                // comparison that skipped the spill would call them tied
                let a: Vec<u8> = (0..len).map(|_| next()).collect();
                let mut b = a.clone();
                let at = (next() as usize * 7 + next() as usize) % len;
                b[at] = next();
                let (mut x, mut y) = (Score::default(), Score::default());
                a.iter().for_each(|r| x.push(*r));
                b.iter().for_each(|r| y.push(*r));
                assert_eq!(x.beats(&y), a > b, "{a:?} against {b:?}");
            }
        }
    }

    /// A short string lives inside the value without making it bigger. At 22
    /// inline bytes `Text` lost the niche in `String`'s capacity and `Value`
    /// grew to 40 bytes, which cost more than the allocations saved.
    #[test]
    fn a_string_value_is_no_bigger_than_a_string() {
        assert_eq!(std::mem::size_of::<Text>(), std::mem::size_of::<String>());
        assert_eq!(std::mem::size_of::<Value>(), 32);
    }

    /// A string reads back as it was given at every length either side of the
    /// inline width, in one-byte and two-byte characters, and two strings
    /// compare as their `str`s do whichever side of the width each one is.
    #[test]
    fn a_string_value_reads_back_what_it_was_given() {
        let mut given = Vec::new();
        for len in 0..=TEXT_INLINE + 4 {
            given.push("k".repeat(len));
            given.push("é".repeat(len / 2) + &"k".repeat(len % 2));
        }
        let held: Vec<Text> = given.iter().map(|s| Text::from(s.clone())).collect();
        for (s, t) in given.iter().zip(&held) {
            assert_eq!(t.as_str(), s);
            assert_eq!(t.clone().as_str(), s);
            assert_eq!(Text::from(s.as_str()).into_string(), *s);
        }
        for (s, t) in given.iter().zip(&held) {
            for (r, u) in given.iter().zip(&held) {
                assert_eq!(t.cmp(u), s.cmp(r), "{s:?} against {r:?}");
                assert_eq!(t == u, s == r, "{s:?} against {r:?}");
            }
        }
    }

    #[test]
    fn scripted_executor_records_the_transcript() {
        let value = run_main("main = print \"a\" .> (_ -> print \"b\")\n");

        let Value::Desc(desc) = value else { panic!("main yields a description") };
        let lexed = crate::lexer::lex("main = print \"a\" .> (_ -> print \"b\")\n").expect("lexes");
        let program = crate::parser::parse(&lexed).expect("parses");
        let interp = Interp::new(&program);
        let mut executor = ScriptedExecutor::default();
        interp.execute(&desc, &mut executor).map_err(|e| e.message).expect("executes");

        assert_eq!(executor.transcript, ["print \"a\"", "print \"b\""]);
    }

    #[test]
    fn a_pure_main_yields_a_plain_value() {
        let value = run_main("main = 1 + 2\n");

        assert!(matches!(value, Value::Int(ref n) if *n == num_bigint::BigInt::from(3)));
    }

    #[test]
    fn err_propagates_through_a_generic_function_unhandled() {
        // Raised rather than divided: division by zero answers a math failure
        // value now, and a generic function is meant to pass it through the
        // way it passes anything else. What this pins is the err path.
        let source = "fn double x\n  x * 2\n\nmain = double (err \"no\")\n";

        assert!(matches!(run_main(source), Value::ErrV(_)));
    }

    #[test]
    fn errs_carry_their_origin_and_dispatcher_hops() {
        let source = "fn grade outcome\n  \"grade {outcome}\"\n\nmain = grade (err \"no\")\n";
        let program = crate::compile("spec.kso", source, false).expect("compiles");
        let interp = Interp::new(&program);
        let value = interp
            .run_named("main")
            .expect("a constant named main")
            .map_err(|e| e.message)
            .expect("runs");

        let Value::ErrV(info) = value else { panic!("expected an err") };
        assert_eq!(info.origin.as_deref(), Some("main at spec.kso:4"));
        assert_eq!(info.hops.iter().map(|h| &**h).collect::<Vec<_>>(), ["grade"]);
    }
}
