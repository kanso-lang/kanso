use crate::diag::Span;
use crate::name::Name;
use num_bigint::BigInt;

/// Whether a name, at one place in the text, finds a local when it runs, and
/// for a global, where the running interpreter keeps what it resolved to.
///
/// 0 not yet known, 1 local, 2 global with no slot for the running
/// interpreter; from 65,536 up, a global stamped with an interpreter's
/// generation in the high sixteen bits and its slot in that interpreter's
/// table in the low sixteen. The interpreter learns it on the node's first
/// execution and keeps it, which is sound because the environment a node sees
/// is fixed by where the node sits -- kanso#1575 found no node among 1,214
/// that both found a local and missed. A slot means something only to the
/// interpreter that stamped it, and an AST can outlive that interpreter, so a
/// stamp from another generation is read as plain global and the name is
/// resolved again. A clone of a node copies what it knows, since the clone
/// sits in the same place. Atomic because an `Expr` may be read from the
/// interpreter's own stack thread; every access is `Relaxed`, and on this
/// target that is an ordinary load or store.
#[derive(Default)]
pub struct Resolution(std::sync::atomic::AtomicU32);

impl Resolution {
    pub const UNKNOWN: u32 = 0;
    pub const LOCAL: u32 = 1;
    pub const GLOBAL: u32 = 2;
    #[inline]
    pub fn get(&self) -> u32 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[inline]
    pub fn set(&self, v: u32) {
        self.0.store(v, std::sync::atomic::Ordering::Relaxed)
    }
    /// Whether a raw value is a global, stamped or not.
    #[inline]
    pub fn is_global(v: u32) -> bool {
        v == Self::GLOBAL || v >= 1 << 16
    }
    /// The slot a raw value names for `generation`, if it was stamped by it.
    #[inline]
    pub fn slot_for(v: u32, generation: u16) -> Option<usize> {
        match v >> 16 {
            g if g != 0 && g == generation as u32 => Some((v & 0xffff) as usize),
            _ => None,
        }
    }
    /// The raw value for a global kept in `slot` by `generation`, or plain
    /// global when either does not fit.
    #[inline]
    pub fn stamped(generation: u16, slot: usize) -> u32 {
        match (generation, slot) {
            (0, _) => Self::GLOBAL,
            (_, s) if s > 0xffff => Self::GLOBAL,
            (g, s) => ((g as u32) << 16) | s as u32,
        }
    }
}

impl Clone for Resolution {
    fn clone(&self) -> Self {
        Resolution(std::sync::atomic::AtomicU32::new(self.get()))
    }
}

/// Printed as nothing, so that a node's debug form reads the same before and
/// after it has run.
impl std::fmt::Debug for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("_")
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(BigInt, Span),
    Float(f64, Span),
    MapLit(Vec<(Expr, Expr)>, Span),
    Str(Vec<TemplatePart>, Span),
    Ident(Name, Span, Resolution),
    /// `&name` — the head of a partial application. Bare, it is superfluous
    /// (a name already denotes the function); applied to fewer arguments than
    /// any arm takes, it is the only spelling for a partial.
    Partial(Name, Span),
    List(Vec<Expr>, Span),
    App {
        head: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
        piped: bool,
    },
    Field {
        base: Box<Expr>,
        name: String,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        strict: bool,
        span: Span,
    },
    Seq(Box<Expr>, Box<Expr>, Span),
    Lambda {
        params: Vec<(String, Span)>,
        body: Box<Expr>,
        span: Span,
    },
    BinOp {
        op: &'static str,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    Join {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// A bind-bearing branch body — fn-body statements in expression
    /// position. Exists only where evaluation is deferred (an `if` arm),
    /// so sequencing never braids into ordinary application.
    Block(Vec<Stmt>, Span),
    /// `(expr):type` — the upcast: strips a subtype value to the named
    /// ancestor. Widening only; construction is the downward direction.
    Upcast {
        expr: Box<Expr>,
        ty: String,
        span: Span,
    },
    /// `_` where a construction argument goes, inside a `build` block: a
    /// hole for a field the block fills exactly once, ruled 2026-08-24. A
    /// none is genuine absence and never a placeholder.
    Hole(Span),
    /// The last expression freezes to an ordinary immutable value.
    Build(Vec<Stmt>, Span),
    /// Everything below a fired guard is folded into the untaken branch,
    /// so first-return-wins holds by unreachability.
    Guard {
        cond: Box<Expr>,
        early: Box<Expr>,
        rest: Vec<Stmt>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum TemplatePart {
    Lit(String),
    Interp(Expr),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Field { span: s, .. }
            | Expr::Float(_, s)
            | Expr::MapLit(_, s)
            | Expr::Str(_, s)
            | Expr::Ident(_, s, _)
            | Expr::Partial(_, s)
            | Expr::List(_, s)
            | Expr::App { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::Seq(_, _, s)
            | Expr::Lambda { span: s, .. }
            | Expr::BinOp { span: s, .. }
            | Expr::Join { span: s, .. }
            | Expr::Block(_, s)
            | Expr::Upcast { span: s, .. }
            | Expr::Hole(s)
            | Expr::Build(_, s)
            | Expr::Guard { span: s, .. } => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    IntLit(BigInt, Span),
    StrLit(String, Span),
    Nullary(Name, Span),
    Var(Name, Span),
    Wildcard(Span),
    Annotated {
        name: Name,
        ty: Name,
        span: Span,
    },
    /// `whole` is the as-pattern's name: `r@(rect w h)` destructures into `w`
    /// and `h` and binds `r` to the value that matched, so an arm can answer
    /// what it was given without building it again.
    Ctor {
        ty: Name,
        fields: Vec<Pattern>,
        whole: Option<Box<(Name, Span)>>,
    },
    Keyed {
        entries: Vec<KeyedEntry>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct KeyedEntry {
    pub field: String,
    pub bind_name: String,
    pub span: Span,
}

impl Pattern {
    pub fn rank(&self) -> u8 {
        match self {
            Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 0,
            // `any` accepts whatever an unnamed parameter accepts, so it ranks
            // where one does; ranking it as a concrete type would let a
            // catch-all sit above the arms it swallows
            Pattern::Annotated { .. } | Pattern::Ctor { .. } => 1,
            Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Bind {
        pattern: Pattern,
        expr: Expr,
    },
    Expr(Expr),
    /// Identity-preserving: rebinding cannot close a cycle.
    Set {
        target: String,
        field: String,
        value: Expr,
        span: Span,
    },
}

/// The binder inside a synthesised field getter. Source identifiers are
/// lowercase, so this collides with nothing a program can write.
pub const GETTER_BINDER: &str = "Read";

/// A field getter's name. Reading a field is applying this function, but the
/// name it is applied under is not one a program can spell, so a field never
/// takes a name away from a type, a local or anything else. `x.name` and the
/// section `_.name` are the two ways to reach it.
///
/// Every type declaring the same field lands in one group, and it is never
/// module-qualified: reading a field is structural, so it needs nothing
/// brought into scope.
/// The declaration a directory's `main.kso`, or a file's `pub play`, is
/// compiled into. Source identifiers are lowercase, so no program can spell
/// this — which is the point. The entry belongs to the compiler, and `main`
/// stays an ordinary name a program may use for whatever it likes.
pub const ENTRY: &str = "Entry";

/// How a declaration names itself in an err's trace. The entry is the
/// compiler's, so a reader shown its internal name would be shown a
/// declaration they never wrote.
pub fn frame_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name == ENTRY {
        true => std::borrow::Cow::Borrowed("the entry"),
        false => spoken(name),
    }
}

/// The byte index of the last `/` in a name, by a plain backward byte scan.
///
/// `str::rsplit_once('/')` and `str::rfind('/')` go through `CharSearcher`,
/// which lands in `memrchr`'s vector path — and that path's setup is most of
/// the cost when the average identifier in a kanso program is five bytes long.
/// callgrind put 794,000 instructions on three callers of it, 1.6% of a check.
/// A `/` is one byte and cannot appear inside a multi-byte character, so the
/// scan is over bytes and the index it returns is a character boundary.
pub fn last_slash(name: &str) -> Option<usize> {
    let bytes = name.as_bytes();
    let mut at = bytes.len();
    while at > 0 {
        at -= 1;
        if bytes[at] == b'/' {
            return Some(at);
        }
    }
    None
}

/// The owner and the bare half of a qualified name, or `None` when it is bare.
pub fn split_qual(name: &str) -> Option<(&str, &str)> {
    last_slash(name).map(|at| (&name[..at], &name[at + 1..]))
}

/// The byte index of the first `/` in a name, by a plain forward byte scan.
///
/// The forward half of `last_slash`, and for the same reason: `contains('/')`
/// and `split_once('/')` go through `memchr`, which is built to scan kilobytes
/// and spends most of a five-byte name setting itself up.
pub fn first_slash(name: &str) -> Option<usize> {
    let bytes = name.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'/' {
            return Some(at);
        }
        at += 1;
    }
    None
}

/// Is this name qualified at all?
pub fn has_slash(name: &str) -> bool {
    first_slash(name).is_some()
}

/// The effect type, `<t>effect`: the unresolved outcome of an operation,
/// which will be a `t` or a failure. The parser folds the spelling into the
/// name as written, so the yield stays readable and every engine asks this
/// one question of it. At run time a box is a box whatever it will yield,
/// so two effect types with different yields are one shape to dispatch.
pub fn is_effect_type(ty: &str) -> bool {
    ty.starts_with('<') && ty.ends_with(">effect")
}

/// What an effect type says it yields: `int` for `<int>effect`.
pub fn effect_yield(ty: &str) -> Option<&str> {
    ty.strip_prefix('<').and_then(|rest| rest.strip_suffix(">effect"))
}

/// The MODULE a qualified name names, and the rest: `json/decode` is
/// `("json", "decode")`, and `std/net/http/get` is `("std", "net/http/get")`.
/// A qualifier is the first segment where an owner is everything before the
/// last slash, which is why this is not `split_qual`.
pub fn split_module(name: &str) -> Option<(&str, &str)> {
    first_slash(name).map(|at| (&name[..at], &name[at + 1..]))
}

/// The last segment of a name: the bare half of a qualified one, or the whole
/// of a bare one.
pub fn bare_name(name: &str) -> &str {
    match last_slash(name) {
        Some(at) => &name[at + 1..],
        None => name,
    }
}

pub fn getter_name(field: &str) -> String {
    format!("Get_{field}")
}

/// The mark that puts a name in a module's bare overload space, where a bare
/// call inside the module dispatches over its own arms and its imports'
/// together (RULED 2026-08-29, "a qualified name is its module's
/// declaration"). The lexer never makes `~` part of a name, so no consumer
/// can write `dep/~join`, and `dep/join` stays the module's own arms.
pub const BARE_MARK: char = '~';

/// `dep/join`: the canonical spelling of `name` under module `qual`.
///
/// Every qualified name the loader mints is this join, and `format!` is an
/// expensive way to write it: a `{}` on a `&str` goes out through
/// `Display::fmt`, `Formatter::pad` and `write_str`, and the string it writes
/// into starts empty and grows. Qualifying the compile corpus makes 712 of
/// these calls at 860 instructions each; the pieces' lengths are known before
/// a byte is written, so one exact allocation and three copies do the same
/// work.
pub fn qualified(qual: &str, name: &str) -> String {
    let mut joined = String::with_capacity(qual.len() + 1 + name.len());
    joined.push_str(qual);
    joined.push('/');
    joined.push_str(name);
    joined
}

/// `dep/~join`: the bare overload space of `join` inside module `dep`.
pub fn bare_space(qual: &str, name: &str) -> String {
    let mut joined = String::with_capacity(qual.len() + 1 + BARE_MARK.len_utf8() + name.len());
    joined.push_str(qual);
    joined.push('/');
    joined.push(BARE_MARK);
    joined.push_str(name);
    joined
}

/// A name as a reader may see it: the bare-space mark comes off, so a
/// sentence about `dep/~join` says `dep/join`, which is what the author
/// wrote at the call site the sentence points at.
pub fn spoken(text: &str) -> std::borrow::Cow<'_, str> {
    match text.contains(BARE_MARK) {
        true => std::borrow::Cow::Owned(text.replace(&format!("/{BARE_MARK}"), "/")),
        false => std::borrow::Cow::Borrowed(text),
    }
}

/// The field a getter reads, for diagnostics: a dispatch failure on `Get_name`
/// is a reader's field error, and must never show them the internal name.
pub fn getter_field(name: &str) -> Option<&str> {
    name.strip_prefix("Get_")
}

/// The three fields an err answers to: `.reason`, `.cause` and `.origin`.
/// Reading one is the second deliberate hole in an err's infectiousness —
/// `wrap_err`'s second argument is the first — so the getter hands back the
/// piece instead of passing the failure through. Ruled 2026-08-29.
pub const ERR_READERS: [&str; 3] = ["cause", "origin", "reason"];

/// The err field a getter reads, when it is one of the three.
pub fn err_reader(name: &str) -> Option<&str> {
    getter_field(name).filter(|field| ERR_READERS.contains(field))
}

/// The file a declaration has before `stamp_file` gives it a real one. An
/// `Arc<str>` always allocates, even for the empty string, and the parser
/// builds every declaration with an unstamped file — so this is made once and
/// handed out, where a fresh `Arc::from("")` apiece would cost one allocation
/// per declaration to hold nothing.
pub fn unstamped() -> std::sync::Arc<str> {
    thread_local! {
        static EMPTY: std::sync::Arc<str> = std::sync::Arc::from("");
    }
    EMPTY.with(std::sync::Arc::clone)
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: String,
    pub is_pub: bool,
    pub span: Span,
    pub params: Vec<Pattern>,
    pub body: Vec<Stmt>,
    /// Source file, stamped after parsing; err origins are "{name} at {file}:{line}".
    pub file: std::sync::Arc<str>,
    /// True for bare-enrollment clones of imported decls (the import
    /// incarnation): real for dispatch, invisible to provenance analyses.
    pub synthetic: bool,
}

impl FnDecl {
    /// A field getter, recognised by the binder no source can spell. It
    /// carries its field's span, so checks that read declaration order have
    /// to leave it out — it was never placed by an author.
    pub fn is_getter(&self) -> bool {
        matches!(self.body.as_slice(), [Stmt::Expr(Expr::Ident(name, _, _))] if name == GETTER_BINDER)
    }
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub name: String,
    pub is_pub: bool,
    pub span: Span,
    pub synthetic: bool,
    /// For an enrollment clone: the declaring module's qualified name. A
    /// record's identity is the canonical name; clones alias, never fork.
    pub origin: Option<String>,
    /// `type post_body string` — a nominal subtype of the named parent.
    /// Mutually exclusive with fields; values construct with one argument
    /// and flow transparently wherever the parent flows.
    pub parent: Option<String>,
    /// `type num float64 int` — a named typeset: annotation-only
    /// vocabulary for a union of types. Never constructs, never carries
    /// dispatch identity; an annotated param matches any member.
    pub members: Vec<String>,
    /// Field name, permitted types (a typeset: one or more members), span.
    pub fields: Vec<(String, Vec<String>, Span)>,
}

#[derive(Debug)]
pub struct Program {
    pub fns: Vec<FnDecl>,
    pub types: Vec<TypeDecl>,
    pub imports: Vec<Import>,
    pub reexports: Vec<Reexport>,
    /// The root module's name, the one an importer would write for it: a
    /// file's stem, a directory's name. RULED 2026-08-29, "records print
    /// qualified, everywhere": a record prints `{module}/{type}` whatever the
    /// entry path, so the root's own types render under this name, the way
    /// they already did when the same file was reached through an import.
    /// Empty when nothing named the root, and a bare type then prints bare.
    pub root: String,
}

/// `pub name` re-exports an imported pub (or, when `name` is an import's
/// qualifier, that module's whole surface); `pub theirs:yours` renames on
/// the way out. Re-exported names join this module's own surface.
#[derive(Clone, Debug)]
pub struct Reexport {
    pub name: String,
    pub rename: Option<String>,
    pub span: crate::diag::Span,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: String,
    pub span: Span,
    /// `import t "path"` — replaces the qualifier for this file.
    pub alias: Option<String>,
    /// `import { theirs:yours } "path"` — bare renames on the way in.
    pub renames: Vec<(String, String)>,
}

pub const NULLARY: [&str; 4] = ["done", "false", "none", "true"];
