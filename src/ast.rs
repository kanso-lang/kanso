use crate::diag::Span;
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub enum Expr {
    Int(BigInt, Span),
    Str(Vec<TemplatePart>, Span),
    Ident(String, Span),
    List(Vec<Expr>, Span),
    App { head: Box<Expr>, args: Vec<Expr>, span: Span },
    Seq(Box<Expr>, Box<Expr>, Span),
    Lambda { params: Vec<(String, Span)>, body: Box<Expr>, span: Span },
    BinOp { op: &'static str, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
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
            | Expr::Str(_, s)
            | Expr::Ident(_, s)
            | Expr::List(_, s)
            | Expr::App { span: s, .. }
            | Expr::Seq(_, _, s)
            | Expr::Lambda { span: s, .. }
            | Expr::BinOp { span: s, .. } => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    IntLit(BigInt, Span),
    StrLit(String, Span),
    Nullary(String, Span),
    Var(String, Span),
    Wildcard,
    Annotated { name: String, ty: String, span: Span },
    Ctor { ty: String, fields: Vec<Pattern> },
}

impl Pattern {
    pub fn rank(&self) -> u8 {
        match self {
            Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 0,
            Pattern::Annotated { .. } | Pattern::Ctor { .. } => 1,
            Pattern::Var(..) | Pattern::Wildcard => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Bind { name: String, span: Span, expr: Expr },
    Expr(Expr),
}

#[derive(Debug)]
pub struct FnDecl {
    pub name: String,
    pub span: Span,
    pub params: Vec<Pattern>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct TypeDecl {
    pub name: String,
    pub span: Span,
    pub fields: Vec<(String, String, Span)>,
}

#[derive(Debug)]
pub struct Program {
    pub fns: Vec<FnDecl>,
    pub types: Vec<TypeDecl>,
}

pub const NULLARY: [&str; 3] = ["false", "none", "true"];
