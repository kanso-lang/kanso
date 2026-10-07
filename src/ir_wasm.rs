//! The native emitter's LLVM IR, lowered to a WebAssembly module that links at
//! instantiation against `runtime.c` built for wasm32.
//!
//! The playground has no LLVM. What it can carry is the emitter native uses and
//! this translator, which reads the small, fixed subset of IR that emitter
//! writes and produces wasm directly. Values live in linear memory in native's
//! layout, and every helper the module calls is the runtime's own `k_*`
//! function, so the program runs on the same C as a native build.
//!
//! The module is a dynamically linked side module. It imports the runtime's
//! memory, table and `__stack_pointer`, plus two bases the host chooses:
//! `__memory_base`, where its data is placed, and `__table_base`, where its
//! functions enter the shared table. A start function patches the pointers in
//! its data and writes its hook functions into the runtime's hook slots
//! (`wasm/hooks.c`). The host then calls the runtime's `_start`.
//!
//! Every function follows the C ABI clang uses for the same IR on wasm32, so
//! the runtime can call a closure or a hook without knowing which compiler
//! wrote it. A struct argument is passed as its fields. A struct return goes
//! through a pointer passed as the first parameter. A variadic call passes a
//! pointer to a buffer on the shadow stack.
//!
//! Control flow is a dispatch loop: each IR block is the code after the end
//! of one wasm `block`, nested in order inside a `loop`. A forward branch is a
//! `br` to the block that ends just before its target. A backward branch sets
//! the block number and returns to the loop's `br_table`.

use crate::hash::Map as HashMap;
use crate::wasm_encode::{sleb, uleb, uleb_len};
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Ty {
    I1,
    I8,
    I16,
    I32,
    I64,
    F64,
    Ptr,
    Void,
    Struct(Rc<[Ty]>),
    Array(u32, Rc<Ty>),
}

const VI32: u8 = 0x7f;
const VI64: u8 = 0x7e;
const VF64: u8 = 0x7c;

fn align_to(n: u32, a: u32) -> u32 {
    n.div_ceil(a) * a
}

impl Ty {
    fn size_align(&self) -> (u32, u32) {
        match self {
            Ty::I1 | Ty::I8 => (1, 1),
            Ty::I16 => (2, 2),
            Ty::I32 | Ty::Ptr => (4, 4),
            Ty::I64 | Ty::F64 => (8, 8),
            Ty::Void => (0, 1),
            Ty::Struct(fs) => {
                let (mut off, mut al) = (0, 1);
                for f in fs.iter() {
                    let (s, a) = f.size_align();
                    off = align_to(off, a) + s;
                    al = al.max(a);
                }
                (align_to(off, al), al)
            }
            Ty::Array(n, e) => (e.size_align().0 * n, e.size_align().1),
        }
    }

    fn size(&self) -> u32 {
        self.size_align().0
    }

    fn field_offset(fs: &[Ty], i: usize) -> u32 {
        let mut off = 0;
        for (k, f) in fs.iter().enumerate() {
            let (s, a) = f.size_align();
            off = align_to(off, a);
            if k == i {
                return off;
            }
            off += s;
        }
        off
    }

    /// The scalar leaves of this type with their byte offsets, in order.
    fn scalars(&self, base: u32, out: &mut Vec<(Ty, u32)>) {
        match self {
            Ty::Struct(fs) => {
                for (i, f) in fs.iter().enumerate() {
                    f.scalars(base + Ty::field_offset(fs, i), out);
                }
            }
            Ty::Array(n, e) => {
                let s = e.size();
                for i in 0..*n {
                    e.scalars(base + s * i, out);
                }
            }
            Ty::Void => {}
            t => out.push((t.clone(), base)),
        }
    }

    fn leaves(&self) -> Vec<(Ty, u32)> {
        let mut out = Vec::new();
        self.scalars(0, &mut out);
        out
    }

    fn width(&self) -> u32 {
        match self {
            Ty::Struct(fs) => fs.iter().map(Ty::width).sum(),
            Ty::Array(n, e) => n * e.width(),
            Ty::Void => 0,
            _ => 1,
        }
    }

    fn vt(&self) -> u8 {
        match self {
            Ty::I64 => VI64,
            Ty::F64 => VF64,
            _ => VI32,
        }
    }

    /// Bits held by a narrow integer kept in an i32, zero-extended.
    fn narrow(&self) -> Option<u32> {
        match self {
            Ty::I1 => Some(1),
            Ty::I8 => Some(8),
            Ty::I16 => Some(16),
            _ => None,
        }
    }

    /// The component range and type of the member at `path`.
    fn member(&self, path: &[u32]) -> Result<(u32, Ty), String> {
        let mut start = 0;
        let mut ty = self.clone();
        for &i in path {
            ty = match ty {
                Ty::Struct(fs) => {
                    start += fs[..i as usize].iter().map(Ty::width).sum::<u32>();
                    fs[i as usize].clone()
                }
                Ty::Array(_, e) => {
                    start += i * e.width();
                    (*e).clone()
                }
                t => return Err(format!("no member {i} in {t:?}")),
            };
        }
        Ok((start, ty))
    }
}

#[derive(Clone, Debug)]
enum Val {
    Local(u32),
    Int(i64),
    Zero,
    Global(String),
    Agg(Vec<(Ty, Val)>),
    Bytes(Vec<u8>),
    PtrToInt(Box<Val>),
}

// ---------------------------------------------------------------- tokens

#[derive(Clone, Debug, PartialEq)]
enum Tok<'a> {
    Local(Cow<'a, str>),
    Global(Cow<'a, str>),
    Int(i64),
    Float(u64),
    Word(&'a str),
    P(u8),
    Bytes(Vec<u8>),
}

fn name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'$' | b'-')
}

fn unescape(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' && i + 3 <= raw.len() {
            let hex = std::str::from_utf8(&raw[i + 1..i + 3]).unwrap_or("");
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    out
}

fn tokenize(line: &str) -> Result<Vec<Tok<'_>>, String> {
    let s = line.as_bytes();
    let mut out = Vec::with_capacity(16);
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        match c {
            b' ' | b'\t' => i += 1,
            b';' => break,
            b'%' | b'@' => {
                i += 1;
                let name = if s.get(i) == Some(&b'"') {
                    let start = i + 1;
                    let mut j = start;
                    while j < s.len() && s[j] != b'"' {
                        j += 1;
                    }
                    i = j + 1;
                    match s[start..j].contains(&b'\\') {
                        true => Cow::Owned(
                            String::from_utf8(unescape(&s[start..j])).map_err(|e| e.to_string())?,
                        ),
                        false => Cow::Borrowed(&line[start..j]),
                    }
                } else {
                    let start = i;
                    while i < s.len() && name_char(s[i]) {
                        i += 1;
                    }
                    Cow::Borrowed(&line[start..i])
                };
                out.push(if c == b'%' { Tok::Local(name) } else { Tok::Global(name) });
            }
            b'c' if s.get(i + 1) == Some(&b'"') => {
                let start = i + 2;
                let mut j = start;
                while j < s.len() && s[j] != b'"' {
                    j += 1;
                }
                out.push(Tok::Bytes(unescape(&s[start..j])));
                i = j + 1;
            }
            b'0' if s.get(i + 1) == Some(&b'x') => {
                let start = i + 2;
                i = start;
                while i < s.len() && s[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let bits = u64::from_str_radix(&line[start..i], 16).map_err(|e| e.to_string())?;
                out.push(Tok::Float(bits));
            }
            b'-' | b'0'..=b'9' => {
                let start = i;
                i += 1;
                while i < s.len() && s[i].is_ascii_digit() {
                    i += 1;
                }
                if i < s.len() && (s[i] == b'.' || s[i] == b'e') {
                    while i < s.len()
                        && (s[i].is_ascii_alphanumeric() || matches!(s[i], b'.' | b'+' | b'-'))
                    {
                        i += 1;
                    }
                    let f: f64 = line[start..i]
                        .parse()
                        .map_err(|_| format!("bad float {}", &line[start..i]))?;
                    out.push(Tok::Float(f.to_bits()));
                } else {
                    let n: i128 = line[start..i]
                        .parse()
                        .map_err(|_| format!("bad int {}", &line[start..i]))?;
                    out.push(Tok::Int(n as i64));
                }
            }
            b'.' if s.get(i + 1) == Some(&b'.') => {
                out.push(Tok::Word("..."));
                i += 3;
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < s.len() && name_char(s[i]) {
                    i += 1;
                }
                out.push(Tok::Word(&line[start..i]));
            }
            _ => {
                out.push(Tok::P(c));
                i += 1;
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- parse

#[derive(Debug, Clone, Copy, PartialEq)]
enum Bin {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    LShr,
    AShr,
    SDiv,
    SRem,
    UDiv,
    URem,
}

#[derive(Debug)]
enum Callee {
    Direct(String),
    Indirect(Val),
}

#[derive(Debug)]
enum Inst {
    Bin(u32, Bin, Ty, Val, Val),
    Icmp(u32, String, Ty, Val, Val),
    Cast(u32, String, Ty, Val, Ty),
    Select(u32, Ty, Val, Val, Val),
    Extract(u32, Ty, Val, Vec<u32>),
    Insert(u32, Ty, Val, Ty, Val, Vec<u32>),
    Load(u32, Ty, Val),
    Store(Ty, Val, Val),
    Gep(u32, Ty, Val, Vec<(Ty, Val)>),
    Alloca(u32, Ty),
    Call(Option<u32>, Ty, Callee, Vec<(Ty, Val)>, bool),
    Phi(u32, Ty, Vec<(Val, String)>),
    Br(String),
    CondBr(Val, String, String),
    Switch(Ty, Val, String, Vec<(i64, String)>),
    Ret(Ty, Option<Val>),
    Unreachable,
}

struct Block {
    name: String,
    insts: Vec<Inst>,
}

struct Sig {
    ret: Ty,
    params: Vec<Ty>,
    varargs: bool,
}

struct Func {
    name: String,
    sig: Sig,
    params: Vec<u32>,
    blocks: Vec<Block>,
    types: Vec<Ty>,
}

struct Global {
    name: String,
    ty: Ty,
    init: Val,
}

#[derive(Default)]
struct Ir<'a> {
    types: HashMap<String, Ty>,
    globals: Vec<Global>,
    externs: Vec<String>,
    aliases: HashMap<String, String>,
    declares: Vec<(String, Sig)>,
    funcs: Vec<Head<'a>>,
}

/// A function as the module's first pass leaves it: its name and signature,
/// which every other function's lowering may need, and its text from the
/// `define` line to the closing brace, which only its own lowering reads.
/// `body` parses that text when the function's turn comes, so one function's
/// instructions are held at a time rather than the whole module's.
struct Head<'a> {
    name: String,
    sig: Sig,
    text: &'a str,
}

struct P<'t, 'a> {
    t: Vec<Tok<'t>>,
    i: usize,
    types: &'a HashMap<String, Ty>,
    names: Option<&'a mut HashMap<String, u32>>,
}

const SKIP: &[&str] = &[
    "noundef",
    "nonnull",
    "noalias",
    "nocapture",
    "readonly",
    "signext",
    "zeroext",
    "inreg",
    "returned",
    "nsw",
    "nuw",
    "exact",
    "inbounds",
    "tail",
    "fastcc",
    "ccc",
    "tailcc",
    "preserve_nonecc",
    "internal",
    "private",
    "unnamed_addr",
    "local_unnamed_addr",
    "dso_local",
    "hidden",
    "noundef",
    "immarg",
    "nofree",
];

impl<'t> P<'t, '_> {
    fn peek(&self) -> Option<&Tok<'t>> {
        self.t.get(self.i)
    }
    fn next(&mut self) -> Result<Tok<'t>, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end of line")?;
        self.i += 1;
        Ok(t)
    }
    fn word(&self) -> Option<&str> {
        match self.peek() {
            Some(Tok::Word(w)) => Some(w),
            _ => None,
        }
    }
    fn eat_word(&mut self, w: &str) -> bool {
        if self.word() == Some(w) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(&Tok::P(c)) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.eat(c) {
            Ok(())
        } else {
            Err(format!("expected '{}' at {:?}", c as char, self.peek()))
        }
    }
    fn skip_attrs(&mut self) {
        loop {
            match self.word() {
                Some(w) if SKIP.contains(&w) => self.i += 1,
                Some("align") | Some("dereferenceable") => {
                    self.i += 1;
                    if self.eat(b'(') {
                        self.i += 2;
                    } else {
                        self.i += 1;
                    }
                }
                _ => return,
            }
        }
    }
    fn local(&mut self, name: &str) -> u32 {
        let names = self.names.as_mut().expect("a function body");
        if let Some(n) = names.get(name) {
            return *n;
        }
        let n = names.len() as u32;
        names.insert(name.to_string(), n);
        n
    }

    fn ty(&mut self) -> Result<Ty, String> {
        let t = match self.next()? {
            Tok::Word(w) => match w {
                "i1" => Ty::I1,
                "i8" => Ty::I8,
                "i16" => Ty::I16,
                "i32" => Ty::I32,
                "i64" => Ty::I64,
                "double" => Ty::F64,
                "ptr" => Ty::Ptr,
                "void" => Ty::Void,
                w => return Err(format!("unknown type {w}")),
            },
            Tok::Local(n) => {
                self.types.get(n.as_ref()).cloned().ok_or_else(|| format!("unknown type %{n}"))?
            }
            Tok::P(b'{') => {
                let mut fs = Vec::new();
                if !self.eat(b'}') {
                    loop {
                        fs.push(self.ty()?);
                        if self.eat(b'}') {
                            break;
                        }
                        self.expect(b',')?;
                    }
                }
                Ty::Struct(fs.into())
            }
            Tok::P(b'[') => {
                let n = match self.next()? {
                    Tok::Int(n) => n as u32,
                    t => return Err(format!("array length {t:?}")),
                };
                if !self.eat_word("x") {
                    return Err("expected x".into());
                }
                let e = self.ty()?;
                self.expect(b']')?;
                Ty::Array(n, Rc::new(e))
            }
            t => return Err(format!("expected a type at {t:?}")),
        };
        Ok(t)
    }

    fn value(&mut self, ty: &Ty) -> Result<Val, String> {
        Ok(match self.next()? {
            Tok::Local(n) => Val::Local(self.local(&n)),
            Tok::Global(n) => Val::Global(n.into_owned()),
            Tok::Int(n) => Val::Int(n),
            Tok::Float(b) => Val::Int(b as i64),
            Tok::Bytes(b) => Val::Bytes(b),
            Tok::Word(w) => match w {
                "true" => Val::Int(1),
                "false" => Val::Int(0),
                "null" | "undef" | "poison" | "zeroinitializer" => Val::Zero,
                "ptrtoint" => {
                    self.expect(b'(')?;
                    let t = self.ty()?;
                    let v = self.value(&t)?;
                    self.eat_word("to");
                    self.ty()?;
                    self.expect(b')')?;
                    Val::PtrToInt(Box::new(v))
                }
                w => return Err(format!("unknown value {w}")),
            },
            Tok::P(b'{') | Tok::P(b'[') => {
                let close = if matches!(ty, Ty::Struct(_)) { b'}' } else { b']' };
                let mut es = Vec::new();
                if !self.eat(close) {
                    loop {
                        let t = self.ty()?;
                        let v = self.value(&t)?;
                        es.push((t, v));
                        if self.eat(close) {
                            break;
                        }
                        self.expect(b',')?;
                    }
                }
                Val::Agg(es)
            }
            t => return Err(format!("unexpected value {t:?}")),
        })
    }

    fn typed(&mut self) -> Result<(Ty, Val), String> {
        let t = self.ty()?;
        self.skip_attrs();
        let v = self.value(&t)?;
        Ok((t, v))
    }

    fn label(&mut self) -> Result<String, String> {
        if !self.eat_word("label") {
            return Err("expected label".into());
        }
        match self.next()? {
            Tok::Local(n) => Ok(n.into_owned()),
            t => Err(format!("label name {t:?}")),
        }
    }

    fn int(&mut self) -> Result<i64, String> {
        match self.next()? {
            Tok::Int(n) => Ok(n),
            t => Err(format!("expected an integer at {t:?}")),
        }
    }

    /// The rest of a `define` line: its signature and the local numbers its
    /// parameters bind.
    fn define(&mut self) -> Result<(String, Sig, Vec<u32>), String> {
        self.skip_attrs();
        let ret = self.ty()?;
        let name = match self.next()? {
            Tok::Global(n) => n.into_owned(),
            t => return Err(format!("define {t:?}")),
        };
        let (params, ids, varargs) = self.params(true)?;
        Ok((name, Sig { ret, params, varargs }, ids))
    }

    /// `(T [attrs] [%name], ...)` with an optional trailing `...`.
    fn params(&mut self, named: bool) -> Result<(Vec<Ty>, Vec<u32>, bool), String> {
        self.expect(b'(')?;
        let (mut tys, mut ids, mut va) = (Vec::new(), Vec::new(), false);
        if self.eat(b')') {
            return Ok((tys, ids, va));
        }
        loop {
            if self.eat_word("...") {
                va = true;
            } else {
                tys.push(self.ty()?);
                self.skip_attrs();
                if named {
                    if let Some(Tok::Local(n)) = self.peek().cloned() {
                        self.i += 1;
                        ids.push(self.local(&n));
                    }
                }
            }
            if self.eat(b')') {
                return Ok((tys, ids, va));
            }
            self.expect(b',')?;
        }
    }
}

fn bin_op(w: &str) -> Option<Bin> {
    Some(match w {
        "add" => Bin::Add,
        "sub" => Bin::Sub,
        "mul" => Bin::Mul,
        "and" => Bin::And,
        "or" => Bin::Or,
        "xor" => Bin::Xor,
        "shl" => Bin::Shl,
        "lshr" => Bin::LShr,
        "ashr" => Bin::AShr,
        "sdiv" => Bin::SDiv,
        "srem" => Bin::SRem,
        "udiv" => Bin::UDiv,
        "urem" => Bin::URem,
        _ => return None,
    })
}

fn parse_inst(p: &mut P, types: &mut Vec<Option<Ty>>) -> Result<Inst, String> {
    let dst = match (p.t.first(), p.t.get(1)) {
        (Some(Tok::Local(n)), Some(Tok::P(b'='))) => {
            let n = n.clone();
            p.i = 2;
            Some(p.local(&n))
        }
        _ => None,
    };
    let mut def = |id: Option<u32>, t: &Ty| {
        if let Some(id) = id {
            let id = id as usize;
            if types.len() <= id {
                types.resize(id + 1, None);
            }
            types[id] = Some(t.clone());
        }
    };
    let d = || dst.ok_or("an instruction with a result is unnamed".to_string());
    let op = match p.next()? {
        Tok::Word(w) => w,
        t => return Err(format!("expected an opcode at {t:?}")),
    };
    let inst = match op {
        w if bin_op(w).is_some() => {
            p.skip_attrs();
            let t = p.ty()?;
            let a = p.value(&t)?;
            p.expect(b',')?;
            let b = p.value(&t)?;
            def(dst, &t);
            Inst::Bin(d()?, bin_op(w).expect("matched"), t, a, b)
        }
        "icmp" => {
            let pred = match p.next()? {
                Tok::Word(w) => w,
                t => return Err(format!("icmp predicate {t:?}")),
            };
            let t = p.ty()?;
            let a = p.value(&t)?;
            p.expect(b',')?;
            let b = p.value(&t)?;
            def(dst, &Ty::I1);
            Inst::Icmp(d()?, pred.to_string(), t, a, b)
        }
        "zext" | "sext" | "trunc" | "inttoptr" | "ptrtoint" | "bitcast" => {
            let (from, v) = p.typed()?;
            p.eat_word("to");
            let to = p.ty()?;
            def(dst, &to);
            Inst::Cast(d()?, op.to_string(), from, v, to)
        }
        "select" => {
            let (_, c) = p.typed()?;
            p.expect(b',')?;
            let (t, a) = p.typed()?;
            p.expect(b',')?;
            let (_, b) = p.typed()?;
            def(dst, &t);
            Inst::Select(d()?, t, c, a, b)
        }
        "extractvalue" => {
            let (t, v) = p.typed()?;
            let mut path = Vec::new();
            while p.eat(b',') {
                path.push(p.int()? as u32);
            }
            def(dst, &t.member(&path)?.1);
            Inst::Extract(d()?, t, v, path)
        }
        "insertvalue" => {
            let (t, v) = p.typed()?;
            p.expect(b',')?;
            let (et, e) = p.typed()?;
            let mut path = Vec::new();
            while p.eat(b',') {
                path.push(p.int()? as u32);
            }
            def(dst, &t);
            Inst::Insert(d()?, t, v, et, e, path)
        }
        "load" => {
            let t = p.ty()?;
            p.expect(b',')?;
            let (_, a) = p.typed()?;
            def(dst, &t);
            Inst::Load(d()?, t, a)
        }
        "store" => {
            let (t, v) = p.typed()?;
            p.expect(b',')?;
            let (_, a) = p.typed()?;
            Inst::Store(t, v, a)
        }
        "getelementptr" => {
            p.skip_attrs();
            let t = p.ty()?;
            p.expect(b',')?;
            let (_, base) = p.typed()?;
            let mut idx = Vec::new();
            while p.eat(b',') {
                idx.push(p.typed()?);
            }
            def(dst, &Ty::Ptr);
            Inst::Gep(d()?, t, base, idx)
        }
        "alloca" => {
            let t = p.ty()?;
            def(dst, &Ty::Ptr);
            Inst::Alloca(d()?, t)
        }
        "musttail" | "tail" | "call" => {
            let tail = op == "musttail";
            if op != "call" && !p.eat_word("call") {
                return Err("expected call".into());
            }
            p.skip_attrs();
            let ret = p.ty()?;
            if p.peek() == Some(&Tok::P(b'(')) {
                p.params(false)?;
            }
            let callee = match p.next()? {
                Tok::Global(n) => Callee::Direct(n.into_owned()),
                Tok::Local(n) => Callee::Indirect(Val::Local(p.local(&n))),
                t => return Err(format!("callee {t:?}")),
            };
            p.expect(b'(')?;
            let mut args = Vec::new();
            if !p.eat(b')') {
                loop {
                    args.push(p.typed()?);
                    if p.eat(b')') {
                        break;
                    }
                    p.expect(b',')?;
                }
            }
            let ret = match &callee {
                Callee::Direct(n) if n.ends_with(".with.overflow.i64") => {
                    Ty::Struct(Rc::from(vec![Ty::I64, Ty::I1]))
                }
                _ => ret,
            };
            if ret != Ty::Void {
                def(dst, &ret);
            }
            Inst::Call(dst, ret, callee, args, tail)
        }
        "phi" => {
            let t = p.ty()?;
            let mut inc = Vec::new();
            loop {
                p.expect(b'[')?;
                let v = p.value(&t)?;
                p.expect(b',')?;
                let b = match p.next()? {
                    Tok::Local(n) => n.into_owned(),
                    t => return Err(format!("phi block {t:?}")),
                };
                p.expect(b']')?;
                inc.push((v, b));
                if !p.eat(b',') {
                    break;
                }
            }
            def(dst, &t);
            Inst::Phi(d()?, t, inc)
        }
        "br" => {
            if p.word() == Some("label") {
                Inst::Br(p.label()?)
            } else {
                let (_, c) = p.typed()?;
                p.expect(b',')?;
                let a = p.label()?;
                p.expect(b',')?;
                let b = p.label()?;
                Inst::CondBr(c, a, b)
            }
        }
        "switch" => {
            let (t, v) = p.typed()?;
            p.expect(b',')?;
            let def_l = p.label()?;
            p.expect(b'[')?;
            let mut cases = Vec::new();
            while !p.eat(b']') {
                p.ty()?;
                let k = p.int()?;
                p.expect(b',')?;
                cases.push((k, p.label()?));
            }
            Inst::Switch(t, v, def_l, cases)
        }
        "ret" => {
            if p.eat_word("void") {
                Inst::Ret(Ty::Void, None)
            } else {
                let (t, v) = p.typed()?;
                Inst::Ret(t, Some(v))
            }
        }
        "unreachable" => Inst::Unreachable,
        o => return Err(format!("unsupported instruction {o}")),
    };
    Ok(inst)
}

fn parse(ir: &str) -> Result<Ir<'_>, String> {
    let mut m = Ir::default();
    let mut at = 0;
    while at < ir.len() {
        let end = ir[at..].find('\n').map_or(ir.len(), |n| at + n);
        let raw = &ir[at..end];
        at = end + 1;
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with(';')
            || line.starts_with("target ")
            || line.starts_with("source_filename")
            || line.starts_with("attributes")
            || line.starts_with('!')
        {
            continue;
        }
        let t = tokenize(line)?;
        if let (Some(Tok::Local(n)), Some(Tok::P(b'=')), Some(Tok::Word(w))) =
            (t.first(), t.get(1), t.get(2))
        {
            if *w == "type" {
                let n = n.to_string();
                let mut p = P { t, i: 3, types: &m.types, names: None };
                let ty = p.ty()?;
                m.types.insert(n, ty);
                continue;
            }
        }
        if let Some(Tok::Global(name)) = t.first() {
            let name = name.to_string();
            if t.contains(&Tok::Word("alias")) {
                match t.last() {
                    Some(Tok::Global(target)) => m.aliases.insert(name, target.to_string()),
                    _ => return Err(format!("alias {name}")),
                };
                continue;
            }
            let mut p = P { t, i: 2, types: &m.types, names: None };
            let external = p.eat_word("external");
            p.skip_attrs();
            if !p.eat_word("global") && !p.eat_word("constant") {
                return Err(format!("global {name}"));
            }
            let ty = p.ty()?;
            if external {
                m.externs.push(name);
                continue;
            }
            let init = p.value(&ty)?;
            m.globals.push(Global { name, ty, init });
            continue;
        }
        if line.starts_with("declare") {
            let mut p = P { t, i: 1, types: &m.types, names: None };
            p.skip_attrs();
            let ret = p.ty()?;
            let name = match p.next()? {
                Tok::Global(n) => n.into_owned(),
                t => return Err(format!("declare {t:?}")),
            };
            let (params, _, varargs) = p.params(false)?;
            if !name.starts_with("llvm.") {
                m.declares.push((name, Sig { ret, params, varargs }));
            }
            continue;
        }
        if line.starts_with("define") {
            let mut names = HashMap::default();
            let mut p = P { t, i: 1, types: &m.types, names: Some(&mut names) };
            let (name, sig, _) = p.define()?;
            // The body ends at the first line that is `}` alone. Every such
            // line holds a `}`, and few others do, so the search steps from
            // brace to brace rather than reading the body a line at a time;
            // `body` reads it that way once, when the function is lowered.
            let from = end - raw.len();
            let mut look = at.min(ir.len());
            let to = loop {
                let brace = ir[look..].find('}').ok_or_else(|| format!("{name} has no end"))?;
                let brace = look + brace;
                let open = ir[..brace].rfind('\n').map_or(0, |n| n + 1);
                let close = ir[brace..].find('\n').map_or(ir.len(), |n| brace + n);
                if ir[open..close].trim() == "}" {
                    at = close + 1;
                    break open;
                }
                look = close;
            };
            m.funcs.push(Head { name, sig, text: &ir[from..to] });
            continue;
        }
        return Err(format!("unsupported line `{line}`"));
    }
    Ok(m)
}

/// A function's instructions, parsed from the text its head kept. The
/// `define` line is read again here because the names it binds number the
/// body's locals.
fn body(text: &str, types: &HashMap<String, Ty>) -> Result<Func, String> {
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("").trim();
    let mut names = HashMap::default();
    let mut slots: Vec<Option<Ty>> = Vec::new();
    let mut p = P { t: tokenize(head)?, i: 1, types, names: Some(&mut names) };
    let (name, sig, ids) = p.define()?;
    for (id, t) in ids.iter().zip(&sig.params) {
        let id = *id as usize;
        if slots.len() <= id {
            slots.resize(id + 1, None);
        }
        slots[id] = Some(t.clone());
    }
    let mut blocks = vec![Block { name: String::new(), insts: Vec::new() }];
    while let Some(raw) = lines.next() {
        // The six lines whose offsets differ on wasm32 are rewritten as they
        // are read, so the native module translates as it is and needs no
        // retargeted copy of its own. `codegen::retarget_wasm32` makes the
        // same substitution for the text clang reads.
        let line = raw.trim();
        let line = crate::codegen::WASM32_LINES
            .iter()
            .find(|(native, _)| *native == line)
            .map_or(line, |(_, wasm)| wasm);
        let mut body = Cow::Borrowed(line);
        if body.is_empty() || body.starts_with(';') {
            continue;
        }
        if body.starts_with("switch") || body.contains("= switch") {
            while !body.trim_end().ends_with(']') {
                let next = lines.next().ok_or("switch runs off the end")?.trim();
                body.to_mut().push(' ');
                body.to_mut().push_str(next);
            }
        }
        if let Some(label) = body.strip_suffix(':') {
            if !label.contains(' ') {
                let label = label.trim_matches('"').to_string();
                if blocks.len() == 1 && blocks[0].insts.is_empty() && blocks[0].name.is_empty() {
                    blocks[0].name = label;
                } else {
                    blocks.push(Block { name: label, insts: Vec::new() });
                }
                continue;
            }
        }
        let mut p = P { t: tokenize(&body)?, i: 0, types, names: Some(&mut names) };
        let inst =
            parse_inst(&mut p, &mut slots).map_err(|e| format!("{name}: {e} in `{body}`"))?;
        blocks.last_mut().expect("a block").insts.push(inst);
    }
    let types = slots.into_iter().map(|t| t.unwrap_or(Ty::Void)).collect();
    Ok(Func { name, sig, params: ids, blocks, types })
}

// ---------------------------------------------------------------- module

/// The wasm signature of an IR function under the C ABI clang uses on wasm32.
fn wasm_sig(sig: &Sig) -> (Vec<u8>, Vec<u8>) {
    let sret = sig.ret.width() > 1;
    let mut params = Vec::new();
    if sret {
        params.push(VI32);
    }
    for p in &sig.params {
        params.extend(p.leaves().iter().map(|(t, _)| t.vt()));
    }
    if sig.varargs {
        params.push(VI32);
    }
    let results = match (&sig.ret, sret) {
        (Ty::Void, _) | (_, true) => vec![],
        (t, false) => vec![t.vt()],
    };
    (params, results)
}

#[derive(Clone, Copy)]
enum Reloc {
    Data(u32),
    Func(u32),
    Extern(u32),
}

const HOOKS: [(&str, &str); 7] = [
    ("k_hook_thunk_eval", "d_thunk_eval"),
    ("k_hook_type_name", "k_type_name"),
    ("k_hook_type_shown", "k_type_shown"),
    ("k_hook_type_field_count", "k_type_field_count"),
    ("k_hook_type_field_name", "k_type_field_name"),
    ("k_hook_caf_init", "k_caf_init"),
    ("k_hook_user_main", "k_user_main"),
];

const G_SP: u32 = 0;
const G_MB: u32 = 1;
const G_TB: u32 = 2;

struct Mx {
    types: Vec<(Vec<u8>, Vec<u8>)>,
    func_idx: HashMap<String, u32>,
    sigs: Vec<Sig>,
    data_off: HashMap<String, u32>,
    extern_idx: HashMap<String, u32>,
    aliases: HashMap<String, String>,
    slots: Vec<u32>,
    slot_of: HashMap<u32, u32>,
}

impl Mx {
    fn type_idx(&mut self, sig: (Vec<u8>, Vec<u8>)) -> u32 {
        match self.types.iter().position(|t| *t == sig) {
            Some(i) => i as u32,
            None => {
                self.types.push(sig);
                self.types.len() as u32 - 1
            }
        }
    }
    fn resolve<'s>(&'s self, name: &'s str) -> &'s str {
        self.aliases.get(name).map(String::as_str).unwrap_or(name)
    }
    fn slot(&mut self, f: u32) -> u32 {
        if let Some(s) = self.slot_of.get(&f) {
            return *s;
        }
        self.slots.push(f);
        let s = self.slots.len() as u32 - 1;
        self.slot_of.insert(f, s);
        s
    }
    fn target(&mut self, name: &str) -> Result<Reloc, String> {
        let found = {
            let name = self.resolve(name);
            match (self.data_off.get(name), self.func_idx.get(name), self.extern_idx.get(name)) {
                (Some(off), _, _) => Some(Reloc::Data(*off)),
                (_, Some(f), _) => Some(Reloc::Func(*f)),
                (_, _, Some(g)) => Some(Reloc::Extern(*g)),
                _ => None,
            }
        };
        match found {
            // a function's address is its slot in the table, assigned here
            Some(Reloc::Func(f)) => Ok(Reloc::Func(self.slot(f))),
            Some(r) => Ok(r),
            None => Err(format!("unknown symbol @{name}")),
        }
    }
}

fn op(code: &mut Vec<u8>, byte: u8, idx: u32) {
    code.push(byte);
    uleb(idx as u64, code);
}

fn i32c(code: &mut Vec<u8>, n: i64) {
    code.push(0x41);
    sleb(n as i32 as i64, code);
}

fn i64c(code: &mut Vec<u8>, n: i64) {
    code.push(0x42);
    sleb(n, code);
}

fn reloc_addr(code: &mut Vec<u8>, r: Reloc) {
    match r {
        Reloc::Data(off) => {
            op(code, 0x23, G_MB);
            if off != 0 {
                i32c(code, off as i64);
                code.push(0x6a);
            }
        }
        Reloc::Func(slot) => {
            op(code, 0x23, G_TB);
            if slot != 0 {
                i32c(code, slot as i64);
                code.push(0x6a);
            }
        }
        Reloc::Extern(g) => op(code, 0x23, g),
    }
}

fn mem(code: &mut Vec<u8>, opc: u8, align: u32, offset: u32) {
    code.push(opc);
    uleb(align as u64, code);
    uleb(offset as u64, code);
}

fn load_op(t: &Ty) -> (u8, u32) {
    match t {
        Ty::I1 | Ty::I8 => (0x2d, 0),
        Ty::I16 => (0x2f, 1),
        Ty::I32 | Ty::Ptr => (0x28, 2),
        Ty::I64 => (0x29, 3),
        Ty::F64 => (0x2b, 3),
        _ => (0x00, 0),
    }
}

fn store_op(t: &Ty) -> (u8, u32) {
    match t {
        Ty::I1 | Ty::I8 => (0x3a, 0),
        Ty::I16 => (0x3b, 1),
        Ty::I32 | Ty::Ptr => (0x36, 2),
        Ty::I64 => (0x37, 3),
        Ty::F64 => (0x39, 3),
        _ => (0x00, 0),
    }
}

/// Writes a constant into the data image, recording each address it holds.
fn write_const(
    mx: &mut Mx,
    img: &mut [u8],
    relocs: &mut Vec<(u32, Reloc)>,
    ty: &Ty,
    v: &Val,
    at: u32,
) -> Result<(), String> {
    match v {
        Val::Zero => {}
        Val::Int(n) => {
            let s = ty.size() as usize;
            img[at as usize..at as usize + s].copy_from_slice(&n.to_le_bytes()[..s]);
        }
        Val::Bytes(b) => img[at as usize..at as usize + b.len()].copy_from_slice(b),
        Val::Global(g) => relocs.push((at, mx.target(g)?)),
        Val::PtrToInt(inner) => match &**inner {
            Val::Global(g) => relocs.push((at, mx.target(g)?)),
            v => return Err(format!("ptrtoint of {v:?}")),
        },
        Val::Agg(es) => {
            for (k, (t, e)) in es.iter().enumerate() {
                let off = match ty {
                    Ty::Struct(fs) => Ty::field_offset(fs, k),
                    Ty::Array(_, el) => el.size() * k as u32,
                    t => return Err(format!("aggregate constant of {t:?}")),
                };
                write_const(mx, img, relocs, t, e, at + off)?;
            }
        }
        Val::Local(_) => return Err("a local in a constant".into()),
    }
    Ok(())
}

/// A translated side module and what the host sets aside for it before
/// instantiation: `data` bytes of linear memory at `__memory_base`, and
/// `table` entries at `__table_base`.
pub struct Side {
    pub wasm: Vec<u8>,
    pub data: u32,
    pub table: u32,
}

/// Translates an emitted module into a side module for the wasm32 runtime.
/// It takes the native text or the text `codegen::retarget_wasm32` made from
/// it, and the two give the same module.
pub fn translate(ir: &str) -> Result<Side, String> {
    translate_given(Text::Lent(ir))
}

/// `translate` for a caller done with the IR. The text is let go once every
/// function is lowered, before the module is put together; the tab's compile
/// peaks in that assembly, and the IR is the largest thing it would otherwise
/// still hold.
pub fn translate_owned(ir: String) -> Result<Side, String> {
    translate_given(Text::Owned(ir))
}

/// The IR handed to the translator: lent, or given to it to let go.
enum Text<'a> {
    Lent(&'a str),
    Owned(String),
}

fn translate_given(text: Text<'_>) -> Result<Side, String> {
    let src = match &text {
        Text::Lent(s) => *s,
        Text::Owned(s) => s.as_str(),
    };
    let ir = parse(src)?;
    let mut mx = Mx {
        types: Vec::new(),
        func_idx: HashMap::default(),
        sigs: Vec::new(),
        data_off: HashMap::default(),
        extern_idx: HashMap::default(),
        aliases: ir.aliases.clone(),
        slots: Vec::new(),
        slot_of: HashMap::default(),
    };

    // Imported globals: the three the linker supplies, the runtime's data
    // symbols the module names, and the hook slots it fills.
    let mut gimports: Vec<(String, bool)> = vec![
        ("__stack_pointer".into(), true),
        ("__memory_base".into(), false),
        ("__table_base".into(), false),
    ];
    for e in &ir.externs {
        mx.extern_idx.insert(e.clone(), gimports.len() as u32);
        gimports.push((e.clone(), false));
    }
    let defined: HashMap<&str, ()> = ir.funcs.iter().map(|f| (f.name.as_str(), ())).collect();
    let mut hooks = Vec::new();
    for (slot, f) in HOOKS {
        let f = mx.resolve(f).to_string();
        if defined.contains_key(f.as_str()) {
            hooks.push((gimports.len() as u32, f));
            gimports.push((slot.to_string(), false));
        }
    }

    let mut fimports = Vec::new();
    for (name, sig) in &ir.declares {
        mx.func_idx.insert(name.clone(), mx.sigs.len() as u32);
        let t = mx.type_idx(wasm_sig(sig));
        fimports.push((name.clone(), t));
        mx.sigs.push(Sig {
            ret: sig.ret.clone(),
            params: sig.params.clone(),
            varargs: sig.varargs,
        });
    }
    let nimports = fimports.len() as u32;
    let mut ftypes = Vec::new();
    for f in &ir.funcs {
        mx.func_idx.insert(f.name.clone(), mx.sigs.len() as u32);
        ftypes.push(mx.type_idx(wasm_sig(&f.sig)));
        mx.sigs.push(Sig {
            ret: f.sig.ret.clone(),
            params: f.sig.params.clone(),
            varargs: f.sig.varargs,
        });
    }

    // Data: every defined global, laid out in order.
    let mut size = 0;
    for g in &ir.globals {
        let (s, a) = g.ty.size_align();
        size = align_to(size, a);
        mx.data_off.insert(g.name.clone(), size);
        size += s;
    }
    let size = align_to(size, 16);
    let mut img = vec![0u8; size as usize];
    let mut relocs = Vec::new();
    for g in &ir.globals {
        let at = mx.data_off[&g.name];
        write_const(&mut mx, &mut img, &mut relocs, &g.ty, &g.init, at)?;
    }

    let mut codes = Vec::new();
    for h in &ir.funcs {
        let f = body(h.text, &ir.types)?;
        codes.push(lower(&mut mx, &f).map_err(|e| format!("{}: {e}", f.name))?);
    }

    // The start function: patch the data's addresses, then fill the hooks.
    let mut start = Vec::new();
    for (at, r) in &relocs {
        op(&mut start, 0x23, G_MB);
        reloc_addr(&mut start, *r);
        mem(&mut start, 0x36, 2, *at);
    }
    for (g, f) in &hooks {
        let slot = mx.slot(mx.func_idx[f.as_str()]);
        op(&mut start, 0x23, *g);
        reloc_addr(&mut start, Reloc::Func(slot));
        mem(&mut start, 0x36, 2, 0);
    }
    let start_ty = mx.type_idx((vec![], vec![]));
    let start_idx = nimports + ftypes.len() as u32;
    ftypes.push(start_ty);
    let mut entry = vec![0u8];
    entry.extend_from_slice(&start);
    entry.push(0x0b);
    codes.push(entry);
    // Nothing below reads the IR or what was parsed out of it.
    drop(defined);
    drop(ir);
    drop(text);

    // ---- assemble
    let mut out = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut sec = Vec::new();
    uleb(mx.types.len() as u64, &mut sec);
    for (p, r) in &mx.types {
        sec.push(0x60);
        uleb(p.len() as u64, &mut sec);
        sec.extend_from_slice(p);
        uleb(r.len() as u64, &mut sec);
        sec.extend_from_slice(r);
    }
    section(1, &sec, &mut out);

    let mut sec = Vec::new();
    uleb((fimports.len() + 2 + gimports.len()) as u64, &mut sec);
    for (name, t) in &fimports {
        str_(&mut sec, "env");
        str_(&mut sec, name);
        sec.push(0x00);
        uleb(*t as u64, &mut sec);
    }
    str_(&mut sec, "env");
    str_(&mut sec, "memory");
    sec.extend_from_slice(&[0x02, 0x00, 0x00]);
    str_(&mut sec, "env");
    str_(&mut sec, "__indirect_function_table");
    sec.extend_from_slice(&[0x01, 0x70, 0x00, 0x00]);
    for (name, mutable) in &gimports {
        str_(&mut sec, "env");
        str_(&mut sec, name);
        sec.extend_from_slice(&[0x03, VI32, *mutable as u8]);
    }
    section(2, &sec, &mut out);

    let mut sec = Vec::new();
    uleb(ftypes.len() as u64, &mut sec);
    for t in &ftypes {
        uleb(*t as u64, &mut sec);
    }
    section(3, &sec, &mut out);

    let mut sec = Vec::new();
    uleb(start_idx as u64, &mut sec);
    section(8, &sec, &mut out);

    if !mx.slots.is_empty() {
        let mut sec = Vec::new();
        uleb(1, &mut sec);
        sec.push(0x00);
        op(&mut sec, 0x23, G_TB);
        sec.push(0x0b);
        uleb(mx.slots.len() as u64, &mut sec);
        for f in &mx.slots {
            uleb(*f as u64, &mut sec);
        }
        section(9, &sec, &mut out);
    }

    // The code and data sections are the bulk of the module, and they are
    // written into `out` where they will stay, at a size known in advance:
    // each function's code is let go as it is copied in, and nothing is
    // gathered into a section buffer first or grown by doubling.
    let code_len = uleb_len(codes.len() as u64)
        + codes.iter().map(|c| uleb_len(c.len() as u64) + c.len()).sum::<usize>();
    let mut data_head = Vec::new();
    uleb(1, &mut data_head);
    data_head.push(0x00);
    op(&mut data_head, 0x23, G_MB);
    data_head.push(0x0b);
    uleb(img.len() as u64, &mut data_head);
    let data_len = data_head.len() + img.len();
    out.reserve_exact(
        2 + uleb_len(code_len as u64) + code_len + uleb_len(data_len as u64) + data_len,
    );
    out.push(10);
    uleb(code_len as u64, &mut out);
    uleb(codes.len() as u64, &mut out);
    for c in codes {
        uleb(c.len() as u64, &mut out);
        out.extend_from_slice(&c);
    }
    out.push(11);
    uleb(data_len as u64, &mut out);
    out.extend_from_slice(&data_head);
    out.extend_from_slice(&img);
    Ok(Side { wasm: out, data: size, table: mx.slots.len() as u32 })
}

fn section(id: u8, contents: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    uleb(contents.len() as u64, out);
    out.extend_from_slice(contents);
}

fn str_(out: &mut Vec<u8>, s: &str) {
    uleb(s.len() as u64, out);
    out.extend_from_slice(s.as_bytes());
}

// ---------------------------------------------------------------- functions

/// A phi's result, its type, and its value along each incoming edge.
type PhiAt = (u32, Ty, Vec<(Val, String)>);

struct Fx<'m> {
    mx: &'m mut Mx,
    code: Vec<u8>,
    locals: Vec<u8>,
    nparams: u32,
    base: Vec<u32>,
    fp: u32,
    frame: u32,
    scratch: u32,
    va: u32,
    bb: u32,
    tmp64: u32,
    n: usize,
    sret: bool,
    phis: Vec<Rc<Vec<PhiAt>>>,
    block_of: HashMap<String, usize>,
}

fn lower(mx: &mut Mx, f: &Func) -> Result<Vec<u8>, String> {
    let (params, _) = wasm_sig(&f.sig);
    let sret = f.sig.ret.width() > 1;
    let mut fx = Fx {
        mx,
        code: Vec::new(),
        locals: Vec::new(),
        nparams: params.len() as u32,
        base: vec![u32::MAX; f.types.len()],
        fp: 0,
        frame: 0,
        scratch: 0,
        va: 0,
        bb: 0,
        tmp64: 0,
        n: f.blocks.len(),
        sret,
        phis: Vec::new(),
        block_of: HashMap::default(),
    };
    let mut at = sret as u32;
    for (id, t) in f.params.iter().zip(&f.sig.params) {
        fx.base[*id as usize] = at;
        at += t.width();
    }
    for (id, t) in f.types.iter().enumerate() {
        if fx.base[id] == u32::MAX && *t != Ty::Void {
            fx.base[id] = fx.nparams + fx.locals.len() as u32;
            for (l, _) in t.leaves() {
                fx.locals.push(l.vt());
            }
        }
    }
    fx.fp = fx.new_local(VI32);
    fx.bb = fx.new_local(VI32);
    fx.tmp64 = fx.new_local(VI64);

    // The frame: allocas, then one struct-return slot, then the largest
    // variadic buffer any call here writes.
    let (mut frame, mut va_size, mut scratch) = (0u32, 0u32, 0u32);
    for (bi, b) in f.blocks.iter().enumerate() {
        fx.block_of.insert(b.name.clone(), bi);
        let mut phis = Vec::new();
        for inst in &b.insts {
            match inst {
                Inst::Alloca(_, t) => {
                    let (s, a) = t.size_align();
                    frame = align_to(frame, a.max(8)) + s;
                }
                Inst::Call(_, ret, callee, args, _) => {
                    scratch = scratch.max(if ret.width() > 1 { ret.size() } else { 0 });
                    if let Callee::Direct(name) = callee {
                        let resolved = fx.mx.resolve(name);
                        if let Some(&fi) = fx.mx.func_idx.get(resolved) {
                            let sig = &fx.mx.sigs[fi as usize];
                            if sig.varargs {
                                let mut s = 0;
                                for (t, _) in &args[sig.params.len()..] {
                                    s = align_to(s, 8) + t.size();
                                }
                                va_size = va_size.max(s);
                            }
                        }
                    }
                }
                Inst::Phi(d, t, inc) => phis.push((*d, t.clone(), inc.clone())),
                _ => {}
            }
        }
        fx.phis.push(Rc::new(phis));
    }
    frame = align_to(frame, 8);
    fx.scratch = frame;
    frame += align_to(scratch, 8);
    fx.va = frame;
    frame += va_size;
    fx.frame = align_to(frame, 16);

    let back = f.blocks.iter().enumerate().any(|(bi, b)| {
        b.insts.iter().any(|i| {
            let targets: Vec<&String> = match i {
                Inst::Br(t) => vec![t],
                Inst::CondBr(_, a, b) => vec![a, b],
                Inst::Switch(_, _, d, cs) => {
                    std::iter::once(d).chain(cs.iter().map(|(_, l)| l)).collect()
                }
                _ => vec![],
            };
            targets.iter().any(|t| fx.block_of.get(*t).is_some_and(|&j| j <= bi))
        })
    });

    if fx.frame > 0 {
        op(&mut fx.code, 0x23, G_SP);
        i32c(&mut fx.code, fx.frame as i64);
        fx.code.push(0x6b);
        op(&mut fx.code, 0x22, fx.fp);
        op(&mut fx.code, 0x24, G_SP);
    }
    if back {
        fx.code.extend_from_slice(&[0x03, 0x40]);
    }
    for _ in 1..fx.n {
        fx.code.extend_from_slice(&[0x02, 0x40]);
    }
    if back {
        fx.code.extend_from_slice(&[0x02, 0x40]);
        op(&mut fx.code, 0x20, fx.bb);
        fx.code.push(0x0e);
        uleb(fx.n as u64, &mut fx.code);
        for d in 0..fx.n {
            uleb(d as u64, &mut fx.code);
        }
        uleb(0, &mut fx.code);
        fx.code.push(0x0b);
    }
    let mut alloca_at = 0u32;
    for (bi, b) in f.blocks.iter().enumerate() {
        if bi > 0 {
            fx.code.push(0x0b);
        }
        let mut k = 0;
        while k < b.insts.len() {
            let inst = &b.insts[k];
            if let Inst::Alloca(d, t) = inst {
                let (s, a) = t.size_align();
                alloca_at = align_to(alloca_at, a.max(8));
                op(&mut fx.code, 0x20, fx.fp);
                i32c(&mut fx.code, alloca_at as i64);
                fx.code.push(0x6a);
                fx.set(*d, 0);
                alloca_at += s;
            } else {
                fx.inst(bi, inst)?;
            }
            if matches!(inst, Inst::Call(_, _, _, _, true)) {
                break;
            }
            k += 1;
        }
    }
    if back {
        fx.code.push(0x0b);
    }
    fx.code.push(0x00);

    let mut entry = Vec::new();
    let mut groups: Vec<(u32, u8)> = Vec::new();
    for t in &fx.locals {
        match groups.last_mut() {
            Some((n, g)) if g == t => *n += 1,
            _ => groups.push((1, *t)),
        }
    }
    uleb(groups.len() as u64, &mut entry);
    for (n, t) in groups {
        uleb(n as u64, &mut entry);
        entry.push(t);
    }
    entry.extend_from_slice(&fx.code);
    entry.push(0x0b);
    Ok(entry)
}

impl Fx<'_> {
    fn new_local(&mut self, t: u8) -> u32 {
        self.locals.push(t);
        self.nparams + self.locals.len() as u32 - 1
    }

    fn set(&mut self, id: u32, k: u32) {
        op(&mut self.code, 0x21, self.base[id as usize] + k);
    }

    fn get(&mut self, id: u32, k: u32) {
        op(&mut self.code, 0x20, self.base[id as usize] + k);
    }

    /// Pushes component `k` of a value of type `ty`.
    fn comp(&mut self, ty: &Ty, v: &Val, k: u32) -> Result<(), String> {
        match v {
            Val::Local(id) => self.get(*id, k),
            Val::Zero => {
                let leaves = ty.leaves();
                self.zero(&leaves[k as usize].0)
            }
            Val::Int(n) => match ty {
                Ty::I64 => i64c(&mut self.code, *n),
                Ty::F64 => {
                    self.code.push(0x44);
                    self.code.extend_from_slice(&(*n as u64).to_le_bytes());
                }
                t => {
                    let n = match t.narrow() {
                        Some(bits) => n & ((1i64 << bits) - 1),
                        None => *n,
                    };
                    i32c(&mut self.code, n)
                }
            },
            Val::Global(g) => {
                let r = self.mx.target(g)?;
                reloc_addr(&mut self.code, r);
            }
            Val::PtrToInt(inner) => {
                self.comp(&Ty::Ptr, inner, 0)?;
                self.code.push(0xad);
            }
            Val::Agg(es) => {
                let mut k = k;
                for (t, e) in es {
                    let w = t.width();
                    if k < w {
                        return self.comp(t, e, k);
                    }
                    k -= w;
                }
                return Err("component past the end of a constant".into());
            }
            Val::Bytes(_) => return Err("bytes as a value".into()),
        }
        Ok(())
    }

    fn zero(&mut self, t: &Ty) {
        match t.vt() {
            VI64 => i64c(&mut self.code, 0),
            VF64 => {
                self.code.push(0x44);
                self.code.extend_from_slice(&[0; 8]);
            }
            _ => i32c(&mut self.code, 0),
        }
    }

    fn push(&mut self, ty: &Ty, v: &Val) -> Result<(), String> {
        for k in 0..ty.width() {
            self.comp(ty, v, k)?;
        }
        Ok(())
    }

    fn epilogue(&mut self) {
        if self.frame > 0 {
            op(&mut self.code, 0x20, self.fp);
            i32c(&mut self.code, self.frame as i64);
            self.code.push(0x6a);
            op(&mut self.code, 0x24, G_SP);
        }
    }

    /// The phi copies for the edge `from -> to`, written as a parallel move.
    fn moves(&mut self, from: usize, to: usize) -> Result<bool, String> {
        let phis = Rc::clone(&self.phis[to]);
        let mut sets = Vec::new();
        for (d, t, inc) in phis.iter() {
            let v = inc
                .iter()
                .find(|(_, b)| self.block_of.get(b) == Some(&from))
                .map(|(v, _)| v.clone())
                .ok_or("a phi has no value for an edge")?;
            self.push(t, &v)?;
            for k in 0..t.width() {
                sets.push((*d, k));
            }
        }
        for (d, k) in sets.iter().rev() {
            self.set(*d, *k);
        }
        Ok(!phis.is_empty())
    }

    fn jump(&mut self, from: usize, to: usize, extra: u32, at_end: bool) {
        if to > from {
            if !(to == from + 1 && at_end && extra == 0) {
                op(&mut self.code, 0x0c, (to - from - 1) as u32 + extra);
            }
        } else {
            i32c(&mut self.code, to as i64);
            op(&mut self.code, 0x21, self.bb);
            op(&mut self.code, 0x0c, (self.n - 1 - from) as u32 + extra);
        }
    }

    fn edge(&mut self, from: usize, to: usize, extra: u32, at_end: bool) -> Result<(), String> {
        self.moves(from, to)?;
        self.jump(from, to, extra, at_end);
        Ok(())
    }

    fn block(&self, name: &str) -> Result<usize, String> {
        self.block_of.get(name).copied().ok_or_else(|| format!("no block %{name}"))
    }

    fn simple(&self, from: usize, to: usize) -> bool {
        to > from && self.phis[to].is_empty()
    }

    fn inst(&mut self, bi: usize, inst: &Inst) -> Result<(), String> {
        match inst {
            Inst::Bin(d, o, t, a, b) => self.bin(*d, *o, t, a, b)?,
            Inst::Icmp(d, pred, t, a, b) => {
                let signed = pred.starts_with('s');
                for v in [a, b] {
                    self.comp(t, v, 0)?;
                    if let (Some(bits), true) = (t.narrow(), signed) {
                        self.sext_narrow(bits);
                    }
                }
                let base: u8 = if *t == Ty::I64 { 0x51 } else { 0x46 };
                let off = match pred.as_str() {
                    "eq" => 0,
                    "ne" => 1,
                    "slt" => 2,
                    "ult" => 3,
                    "sgt" => 4,
                    "ugt" => 5,
                    "sle" => 6,
                    "ule" => 7,
                    "sge" => 8,
                    "uge" => 9,
                    p => return Err(format!("icmp {p}")),
                };
                self.code.push(base + off);
                self.set(*d, 0);
            }
            Inst::Cast(d, how, from, v, to) => {
                self.comp(from, v, 0)?;
                match (how.as_str(), from, to) {
                    ("zext", f, Ty::I64) if *f != Ty::I64 => self.code.push(0xad),
                    ("zext", _, _) => {}
                    ("sext", f, t) => {
                        if let Some(bits) = f.narrow() {
                            self.sext_narrow(bits);
                        }
                        if *t == Ty::I64 {
                            self.code.push(0xac);
                        } else if let Some(bits) = t.narrow() {
                            self.mask(bits);
                        }
                    }
                    ("trunc", Ty::I64, t) => {
                        self.code.push(0xa7);
                        if let Some(bits) = t.narrow() {
                            self.mask(bits);
                        }
                    }
                    ("trunc", _, t) => {
                        if let Some(bits) = t.narrow() {
                            self.mask(bits);
                        }
                    }
                    ("inttoptr", Ty::I64, _) => self.code.push(0xa7),
                    ("ptrtoint", _, Ty::I64) => self.code.push(0xad),
                    ("bitcast", Ty::F64, Ty::I64) => self.code.push(0xbd),
                    ("bitcast", Ty::I64, Ty::F64) => self.code.push(0xbf),
                    ("bitcast" | "inttoptr" | "ptrtoint", _, _) => {}
                    (h, f, t) => return Err(format!("{h} {f:?} to {t:?}")),
                }
                self.set(*d, 0);
            }
            Inst::Select(d, t, c, a, b) => {
                for k in 0..t.width() {
                    self.comp(t, a, k)?;
                    self.comp(t, b, k)?;
                    self.comp(&Ty::I1, c, 0)?;
                    self.code.push(0x1b);
                    self.set(*d, k);
                }
            }
            Inst::Extract(d, t, v, path) => {
                let (start, m) = t.member(path)?;
                for k in 0..m.width() {
                    self.comp(t, v, start + k)?;
                    self.set(*d, k);
                }
            }
            Inst::Insert(d, t, v, et, e, path) => {
                let (start, m) = t.member(path)?;
                for k in 0..t.width() {
                    if k >= start && k < start + m.width() {
                        self.comp(et, e, k - start)?;
                    } else {
                        self.comp(t, v, k)?;
                    }
                    self.set(*d, k);
                }
            }
            Inst::Load(d, t, a) => {
                for (k, (l, off)) in t.leaves().iter().enumerate() {
                    self.comp(&Ty::Ptr, a, 0)?;
                    let (o, al) = load_op(l);
                    mem(&mut self.code, o, al, *off);
                    self.set(*d, k as u32);
                }
            }
            Inst::Store(t, v, a) => {
                for (k, (l, off)) in t.leaves().iter().enumerate() {
                    self.comp(&Ty::Ptr, a, 0)?;
                    self.comp(t, v, k as u32)?;
                    let (o, al) = store_op(l);
                    mem(&mut self.code, o, al, *off);
                }
            }
            Inst::Gep(d, t, base, idx) => {
                self.comp(&Ty::Ptr, base, 0)?;
                let mut konst: i64 = 0;
                let mut cur = t.clone();
                for (n, (it, iv)) in idx.iter().enumerate() {
                    let scale = if n == 0 {
                        t.size()
                    } else {
                        match &cur {
                            Ty::Struct(fs) => {
                                let Val::Int(i) = iv else {
                                    return Err("a struct index is not constant".into());
                                };
                                konst += Ty::field_offset(fs, *i as usize) as i64;
                                cur = fs[*i as usize].clone();
                                continue;
                            }
                            Ty::Array(_, e) => {
                                let s = e.size();
                                cur = (**e).clone();
                                s
                            }
                            t => return Err(format!("gep into {t:?}")),
                        }
                    };
                    match iv {
                        Val::Int(i) => konst += i * scale as i64,
                        Val::Zero => {}
                        v => {
                            self.comp(it, v, 0)?;
                            if *it == Ty::I64 {
                                self.code.push(0xa7);
                            }
                            if scale != 1 {
                                i32c(&mut self.code, scale as i64);
                                self.code.push(0x6c);
                            }
                            self.code.push(0x6a);
                        }
                    }
                }
                if konst != 0 {
                    i32c(&mut self.code, konst);
                    self.code.push(0x6a);
                }
                self.set(*d, 0);
            }
            Inst::Alloca(..) | Inst::Phi(..) => {}
            Inst::Call(d, ret, callee, args, tail) => self.call(*d, ret, callee, args, *tail)?,
            Inst::Br(l) => {
                let to = self.block(l)?;
                self.edge(bi, to, 0, true)?;
            }
            Inst::CondBr(c, a, b) => {
                let (a, b) = (self.block(a)?, self.block(b)?);
                match c {
                    Val::Int(n) => self.edge(bi, if *n & 1 == 1 { a } else { b }, 0, true)?,
                    _ if self.simple(bi, a) => {
                        self.comp(&Ty::I1, c, 0)?;
                        op(&mut self.code, 0x0d, (a - bi - 1) as u32);
                        self.edge(bi, b, 0, true)?;
                    }
                    _ if self.simple(bi, b) => {
                        self.comp(&Ty::I1, c, 0)?;
                        self.code.push(0x45);
                        op(&mut self.code, 0x0d, (b - bi - 1) as u32);
                        self.edge(bi, a, 0, true)?;
                    }
                    _ => {
                        self.comp(&Ty::I1, c, 0)?;
                        self.code.extend_from_slice(&[0x04, 0x40]);
                        self.edge(bi, a, 1, false)?;
                        self.code.push(0x05);
                        self.edge(bi, b, 1, false)?;
                        self.code.push(0x0b);
                    }
                }
            }
            Inst::Switch(t, v, dl, cases) => {
                let dflt = self.block(dl)?;
                let targets: Vec<(i64, usize)> = cases
                    .iter()
                    .map(|(k, l)| Ok((*k, self.block(l)?)))
                    .collect::<Result<_, String>>()?;
                let all_simple = self.simple(bi, dflt)
                    && targets.iter().all(|(_, j)| self.simple(bi, *j))
                    && *t == Ty::I64;
                let (lo, hi) = targets
                    .iter()
                    .fold((i64::MAX, i64::MIN), |(lo, hi), (k, _)| (lo.min(*k), hi.max(*k)));
                let span = (hi as i128 - lo as i128 + 1) as usize;
                if all_simple && !targets.is_empty() && span <= 4 * targets.len() + 8 {
                    let mut table = vec![(dflt - bi - 1) as u32; span];
                    for (k, j) in &targets {
                        table[(*k - lo) as usize] = (*j - bi - 1) as u32;
                    }
                    self.comp(t, v, 0)?;
                    i64c(&mut self.code, lo);
                    self.code.push(0x7d);
                    op(&mut self.code, 0x22, self.tmp64);
                    self.code.push(0xa7);
                    i32c(&mut self.code, span as i64);
                    op(&mut self.code, 0x20, self.tmp64);
                    i64c(&mut self.code, span as i64);
                    self.code.push(0x54);
                    self.code.push(0x1b);
                    self.code.push(0x0e);
                    uleb(span as u64, &mut self.code);
                    for e in &table {
                        uleb(*e as u64, &mut self.code);
                    }
                    uleb((dflt - bi - 1) as u64, &mut self.code);
                } else {
                    for (k, j) in &targets {
                        self.comp(t, v, 0)?;
                        if *t == Ty::I64 {
                            i64c(&mut self.code, *k);
                            self.code.push(0x51);
                        } else {
                            i32c(&mut self.code, *k);
                            self.code.push(0x46);
                        }
                        self.code.extend_from_slice(&[0x04, 0x40]);
                        self.edge(bi, *j, 1, false)?;
                        self.code.push(0x0b);
                    }
                    self.edge(bi, dflt, 0, true)?;
                }
            }
            Inst::Ret(t, v) => {
                match (v, self.sret) {
                    (None, _) => {}
                    (Some(v), true) => {
                        for (k, (l, off)) in t.leaves().iter().enumerate() {
                            op(&mut self.code, 0x20, 0);
                            self.comp(t, v, k as u32)?;
                            let (o, al) = store_op(l);
                            mem(&mut self.code, o, al, *off);
                        }
                    }
                    (Some(v), false) => self.comp(t, v, 0)?,
                }
                self.epilogue();
                self.code.push(0x0f);
            }
            Inst::Unreachable => self.code.push(0x00),
        }
        Ok(())
    }

    fn sext_narrow(&mut self, bits: u32) {
        let s = 32 - bits as i64;
        i32c(&mut self.code, s);
        self.code.push(0x74);
        i32c(&mut self.code, s);
        self.code.push(0x75);
    }

    fn mask(&mut self, bits: u32) {
        i32c(&mut self.code, (1i64 << bits) - 1);
        self.code.push(0x71);
    }

    fn bin(&mut self, d: u32, o: Bin, t: &Ty, a: &Val, b: &Val) -> Result<(), String> {
        let wide = *t == Ty::I64;
        let narrow = t.narrow();
        let signed = matches!(o, Bin::AShr | Bin::SDiv | Bin::SRem);
        for v in [a, b] {
            self.comp(t, v, 0)?;
            if let (Some(bits), true) = (narrow, signed) {
                self.sext_narrow(bits);
            }
        }
        let k = match o {
            Bin::Add => 0,
            Bin::Sub => 1,
            Bin::Mul => 2,
            Bin::SDiv => 3,
            Bin::UDiv => 4,
            Bin::SRem => 5,
            Bin::URem => 6,
            Bin::And => 7,
            Bin::Or => 8,
            Bin::Xor => 9,
            Bin::Shl => 10,
            Bin::AShr => 11,
            Bin::LShr => 12,
        };
        self.code.push(if wide { 0x7c } else { 0x6a } + k);
        if let Some(bits) = narrow {
            if !matches!(o, Bin::And | Bin::Or | Bin::LShr | Bin::UDiv | Bin::URem) {
                self.mask(bits);
            }
        }
        self.set(d, 0);
        Ok(())
    }

    fn call(
        &mut self,
        d: Option<u32>,
        ret: &Ty,
        callee: &Callee,
        args: &[(Ty, Val)],
        tail: bool,
    ) -> Result<(), String> {
        if let Callee::Direct(name) = callee {
            if let Some(intr) = name.strip_prefix("llvm.") {
                return self.intrinsic(d, intr, args);
            }
        }
        let (target, sig_params, varargs) = match callee {
            Callee::Direct(name) => {
                let fi = *self
                    .mx
                    .func_idx
                    .get(self.mx.resolve(name))
                    .ok_or_else(|| format!("call to unknown @{name}"))?;
                let sig = &self.mx.sigs[fi as usize];
                (Some(fi), sig.params.len(), sig.varargs)
            }
            Callee::Indirect(_) => (None, args.len(), false),
        };
        let sret = ret.width() > 1;
        if tail {
            self.epilogue();
        }
        if sret {
            if tail {
                op(&mut self.code, 0x20, 0);
            } else {
                op(&mut self.code, 0x20, self.fp);
                if self.scratch != 0 {
                    i32c(&mut self.code, self.scratch as i64);
                    self.code.push(0x6a);
                }
            }
        }
        for (t, v) in &args[..sig_params] {
            self.push(t, v)?;
        }
        if varargs {
            let mut at = 0;
            for (t, v) in &args[sig_params..] {
                at = align_to(at, 8);
                for (k, (l, off)) in t.leaves().iter().enumerate() {
                    op(&mut self.code, 0x20, self.fp);
                    self.comp(t, v, k as u32)?;
                    let (o, al) = store_op(l);
                    mem(&mut self.code, o, al, self.va + at + off);
                }
                at += t.size();
            }
            op(&mut self.code, 0x20, self.fp);
            if self.va != 0 {
                i32c(&mut self.code, self.va as i64);
                self.code.push(0x6a);
            }
        }
        match (target, callee) {
            (Some(fi), _) => op(&mut self.code, if tail { 0x12 } else { 0x10 }, fi),
            (None, Callee::Indirect(fp)) => {
                self.comp(&Ty::Ptr, fp, 0)?;
                let sig = Sig {
                    ret: ret.clone(),
                    params: args.iter().map(|(t, _)| t.clone()).collect(),
                    varargs: false,
                };
                let ty = self.mx.type_idx(wasm_sig(&sig));
                op(&mut self.code, if tail { 0x13 } else { 0x11 }, ty);
                self.code.push(0x00);
            }
            _ => unreachable!(),
        }
        if tail {
            return Ok(());
        }
        match (d, sret) {
            (Some(d), true) => {
                for (k, (l, off)) in ret.leaves().iter().enumerate() {
                    op(&mut self.code, 0x20, self.fp);
                    let (o, al) = load_op(l);
                    mem(&mut self.code, o, al, self.scratch + off);
                    self.set(d, k as u32);
                }
            }
            (Some(d), false) => self.set(d, 0),
            (None, false) if *ret != Ty::Void => self.code.push(0x1a),
            (None, _) => {}
        }
        Ok(())
    }

    fn intrinsic(&mut self, d: Option<u32>, name: &str, args: &[(Ty, Val)]) -> Result<(), String> {
        let arg = |fx: &mut Self, i: usize| fx.comp(&args[i].0, &args[i].1, 0);
        match name {
            "assume" => {}
            "memcpy.p0.p0.i64" => {
                arg(self, 0)?;
                arg(self, 1)?;
                arg(self, 2)?;
                self.code.push(0xa7);
                self.code.extend_from_slice(&[0xfc, 0x0a, 0x00, 0x00]);
            }
            "sadd.with.overflow.i64" | "ssub.with.overflow.i64" => {
                let d = d.ok_or("an unnamed overflow")?;
                let add = name.starts_with("sadd");
                arg(self, 0)?;
                arg(self, 1)?;
                self.code.push(if add { 0x7c } else { 0x7d });
                self.set(d, 0);
                // add: ((a ^ r) & (b ^ r)) < 0; sub: ((a ^ b) & (a ^ r)) < 0
                arg(self, 0)?;
                if add {
                    self.get(d, 0);
                } else {
                    arg(self, 1)?;
                }
                self.code.push(0x85);
                if add {
                    arg(self, 1)?;
                } else {
                    arg(self, 0)?;
                }
                self.get(d, 0);
                self.code.push(0x85);
                self.code.push(0x83);
                i64c(&mut self.code, 0);
                self.code.push(0x53);
                self.set(d, 1);
            }
            "smul.with.overflow.i64" => {
                let d = d.ok_or("an unnamed overflow")?;
                arg(self, 0)?;
                arg(self, 1)?;
                self.code.push(0x7e);
                self.set(d, 0);
                arg(self, 0)?;
                self.code.push(0x50);
                self.code.extend_from_slice(&[0x04, VI32]);
                i32c(&mut self.code, 0);
                self.code.push(0x05);
                arg(self, 0)?;
                i64c(&mut self.code, -1);
                self.code.push(0x51);
                self.code.extend_from_slice(&[0x04, VI32]);
                arg(self, 1)?;
                i64c(&mut self.code, i64::MIN);
                self.code.push(0x51);
                self.code.push(0x05);
                self.get(d, 0);
                arg(self, 0)?;
                self.code.push(0x7f);
                arg(self, 1)?;
                self.code.push(0x52);
                self.code.push(0x0b);
                self.code.push(0x0b);
                self.set(d, 1);
            }
            n => return Err(format!("unsupported intrinsic llvm.{n}")),
        }
        Ok(())
    }
}
