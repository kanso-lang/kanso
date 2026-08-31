use crate::diag::Span;
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub enum Expr {
    Int(BigInt, Span),
    Float(f64, Span),
    MapLit(Vec<(Expr, Expr)>, Span),
    Str(Vec<TemplatePart>, Span),
    Ident(String, Span),
    /// `&name` — the head of a partial application. Bare, it is superfluous
    /// (a name already denotes the function); applied to fewer arguments than
    /// any arm takes, it is the only spelling for a partial.
    Partial(String, Span),
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
            | Expr::Ident(_, s)
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
            | Expr::Build(_, s)
            | Expr::Guard { span: s, .. } => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    IntLit(BigInt, Span),
    StrLit(String, Span),
    Nullary(String, Span),
    Var(String, Span),
    Wildcard(Span),
    Annotated {
        name: String,
        ty: String,
        span: Span,
    },
    /// `whole` is the as-pattern's name: `r@(rect w h)` destructures into `w`
    /// and `h` and binds `r` to the value that matched, so an arm can answer
    /// what it was given without building it again.
    Ctor {
        ty: String,
        fields: Vec<Pattern>,
        whole: Option<Box<(String, Span)>>,
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
pub fn frame_name(name: &str) -> &str {
    match name == ENTRY {
        true => "the entry",
        false => name,
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

/// The field a getter reads, for diagnostics: a dispatch failure on `Get_name`
/// is a reader's field error, and must never show them the internal name.
pub fn getter_field(name: &str) -> Option<&str> {
    name.strip_prefix("Get_")
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: String,
    pub is_pub: bool,
    pub span: Span,
    pub params: Vec<Pattern>,
    pub body: Vec<Stmt>,
    /// Source file, stamped after parsing; err origins are "{name} at {file}:{line}".
    pub file: String,
    /// True for bare-enrollment clones of imported decls (the import
    /// incarnation): real for dispatch, invisible to provenance analyses.
    pub synthetic: bool,
}

impl FnDecl {
    /// A field getter, recognised by the binder no source can spell. It
    /// carries its field's span, so checks that read declaration order have
    /// to leave it out — it was never placed by an author.
    pub fn is_getter(&self) -> bool {
        matches!(self.body.as_slice(), [Stmt::Expr(Expr::Ident(name, _))] if name == GETTER_BINDER)
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

pub const NULLARY: [&str; 3] = ["false", "none", "true"];
