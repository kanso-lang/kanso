use crate::diag::Span;
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub enum Expr {
    Int(BigInt, Span),
    Float(f64, Span),
    MapLit(Vec<(Expr, Expr)>, Span),
    Str(Vec<TemplatePart>, Span),
    Ident(String, Span),
    List(Vec<Expr>, Span),
    App { head: Box<Expr>, args: Vec<Expr>, span: Span, piped: bool },
    Field { base: Box<Expr>, name: String, span: Span },
    Index { base: Box<Expr>, index: Box<Expr>, strict: bool, span: Span },
    Seq(Box<Expr>, Box<Expr>, Span),
    Lambda { params: Vec<(String, Span)>, body: Box<Expr>, span: Span },
    BinOp { op: &'static str, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    Join { lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    /// A bind-bearing branch body — fn-body statements in expression
    /// position. Exists only where evaluation is deferred (an `if` arm),
    /// so sequencing never braids into ordinary application.
    Block(Vec<Stmt>, Span),
    /// `(expr):type` — the upcast: strips a subtype value to the named
    /// ancestor. Widening only; construction is the downward direction.
    Upcast { expr: Box<Expr>, ty: String, span: Span },
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
            | Expr::List(_, s)
            | Expr::App { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::Seq(_, _, s)
            | Expr::Lambda { span: s, .. }
            | Expr::BinOp { span: s, .. }
            | Expr::Join { span: s, .. }
            | Expr::Block(_, s)
            | Expr::Upcast { span: s, .. } => *s,
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
    Annotated { name: String, ty: String, span: Span },
    Ctor { ty: String, fields: Vec<Pattern> },
    Keyed { entries: Vec<KeyedEntry>, span: Span },
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
            Pattern::Annotated { .. } | Pattern::Ctor { .. } => 1,
            Pattern::Var(..) | Pattern::Wildcard(..) | Pattern::Keyed { .. } => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Bind { pattern: Pattern, expr: Expr },
    Expr(Expr),
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
