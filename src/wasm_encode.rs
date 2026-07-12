//! Minimal WebAssembly binary emitter: just the sections and opcodes the
//! browser backend needs. Values are i32 handles, so every function type is
//! (i32^n) -> i32 and the encoding stays small.

pub fn uleb(mut n: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        match n {
            0 => {
                out.push(byte);
                return;
            }
            _ => out.push(byte | 0x80),
        }
    }
}

pub fn sleb(mut n: i64, out: &mut Vec<u8>) {
    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        let sign = byte & 0x40 != 0;
        if (n == 0 && !sign) || (n == -1 && sign) {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// One function body under construction. Locals are all i32; index space is
/// params first, then extras allocated on demand.
pub struct Body {
    pub code: Vec<u8>,
    pub params: u32,
    pub locals: u32,
}

pub const I32: u8 = 0x7f;
pub const VOID_BLOCK: u8 = 0x40;

impl Body {
    pub fn new(params: u32) -> Self {
        Body { code: Vec::new(), params, locals: 0 }
    }

    pub fn local(&mut self) -> u32 {
        self.locals += 1;
        self.params + self.locals - 1
    }

    pub fn op(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn op_idx(&mut self, byte: u8, idx: u32) {
        self.code.push(byte);
        uleb(idx as u64, &mut self.code);
    }

    pub fn i32_const(&mut self, n: i64) {
        self.code.push(0x41);
        sleb(n, &mut self.code);
    }

    pub fn local_get(&mut self, idx: u32) {
        self.op_idx(0x20, idx);
    }

    pub fn local_set(&mut self, idx: u32) {
        self.op_idx(0x21, idx);
    }

    pub fn local_tee(&mut self, idx: u32) {
        self.op_idx(0x22, idx);
    }

    pub fn call(&mut self, fn_idx: u32) {
        self.op_idx(0x10, fn_idx);
    }

    pub fn return_call(&mut self, fn_idx: u32) {
        self.op_idx(0x12, fn_idx);
    }

    pub fn block_void(&mut self) {
        self.code.push(0x02);
        self.code.push(VOID_BLOCK);
    }

    pub fn if_i32(&mut self) {
        self.code.push(0x04);
        self.code.push(I32);
    }

    pub fn if_void(&mut self) {
        self.code.push(0x04);
        self.code.push(VOID_BLOCK);
    }

    pub fn else_(&mut self) {
        self.code.push(0x05);
    }

    pub fn end(&mut self) {
        self.code.push(0x0b);
    }

    pub fn br_if(&mut self, depth: u32) {
        self.op_idx(0x0d, depth);
    }

    pub fn ret(&mut self) {
        self.code.push(0x0f);
    }

    pub fn unreachable(&mut self) {
        self.code.push(0x00);
    }

    pub fn eqz(&mut self) {
        self.code.push(0x45);
    }

    pub fn drop_(&mut self) {
        self.code.push(0x1a);
    }
}

pub struct Import {
    pub name: &'static str,
    pub params: u32,
    pub returns: bool,
}

/// Assembles the final module: types are deduped (i32^n)->i32 shapes plus
/// void-returning import shapes; the table holds closure-callable functions.
pub struct Module {
    types: Vec<(u32, bool)>,
    pub imports: Vec<Import>,
    fn_types: Vec<u32>,
    codes: Vec<(u32, Vec<u8>)>,
    pub table: Vec<u32>,
    main_idx: Option<u32>,
}

impl Module {
    pub fn new(imports: Vec<Import>) -> Self {
        Module {
            types: Vec::new(),
            imports,
            fn_types: Vec::new(),
            codes: Vec::new(),
            table: Vec::new(),
            main_idx: None,
        }
    }

    fn type_idx(&mut self, params: u32, returns: bool) -> u32 {
        match self.types.iter().position(|t| *t == (params, returns)) {
            Some(i) => i as u32,
            None => {
                self.types.push((params, returns));
                (self.types.len() - 1) as u32
            }
        }
    }

    pub fn import_count(&self) -> u32 {
        self.imports.len() as u32
    }

    /// Reserves the next defined-function index without a body yet.
    pub fn declare(&mut self, params: u32) -> u32 {
        let ty = self.type_idx(params, true);
        self.fn_types.push(ty);
        self.import_count() + (self.fn_types.len() as u32) - 1
    }

    pub fn define(&mut self, fn_idx: u32, body: Body) {
        let slot = fn_idx - self.import_count();
        let mut entry = Vec::new();
        match body.locals {
            0 => uleb(0, &mut entry),
            n => {
                uleb(1, &mut entry);
                uleb(n as u64, &mut entry);
                entry.push(I32);
            }
        }
        entry.extend_from_slice(&body.code);
        entry.push(0x0b);
        self.codes.push((slot, entry));
    }

    pub fn set_main(&mut self, fn_idx: u32) {
        self.main_idx = Some(fn_idx);
    }

    pub fn assemble(mut self) -> Vec<u8> {
        let import_types: Vec<u32> = self
            .imports
            .iter()
            .map(|imp| (imp.params, imp.returns))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(p, r)| self.type_idx(p, r))
            .collect();
        let mut out = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        let mut sec = Vec::new();
        uleb(self.types.len() as u64, &mut sec);
        for (params, returns) in &self.types {
            sec.push(0x60);
            uleb(*params as u64, &mut sec);
            sec.extend(std::iter::repeat_n(I32, *params as usize));
            match returns {
                true => {
                    uleb(1, &mut sec);
                    sec.push(I32);
                }
                false => uleb(0, &mut sec),
            }
        }
        section(1, &sec, &mut out);

        let mut sec = Vec::new();
        uleb(self.imports.len() as u64, &mut sec);
        for (imp, ty) in self.imports.iter().zip(&import_types) {
            name("env", &mut sec);
            name(imp.name, &mut sec);
            sec.push(0x00);
            uleb(*ty as u64, &mut sec);
        }
        section(2, &sec, &mut out);

        let mut sec = Vec::new();
        uleb(self.fn_types.len() as u64, &mut sec);
        for ty in &self.fn_types {
            uleb(*ty as u64, &mut sec);
        }
        section(3, &sec, &mut out);

        let mut sec = Vec::new();
        uleb(1, &mut sec);
        sec.push(0x70);
        sec.push(0x00); // limits: min only
        uleb(self.table.len().max(1) as u64, &mut sec);
        section(4, &sec, &mut out);

        let mut sec = Vec::new();
        uleb(2, &mut sec);
        name("main", &mut sec);
        sec.push(0x00);
        uleb(self.main_idx.expect("main set") as u64, &mut sec);
        name("table", &mut sec);
        sec.push(0x01);
        uleb(0, &mut sec);
        section(7, &sec, &mut out);

        if !self.table.is_empty() {
            let mut sec = Vec::new();
            uleb(1, &mut sec);
            sec.push(0x00);
            sec.push(0x41);
            sleb(0, &mut sec);
            sec.push(0x0b);
            uleb(self.table.len() as u64, &mut sec);
            for idx in &self.table {
                uleb(*idx as u64, &mut sec);
            }
            section(9, &sec, &mut out);
        }

        let mut sec = Vec::new();
        self.codes.sort_by_key(|(slot, _)| *slot);
        uleb(self.codes.len() as u64, &mut sec);
        for (_, entry) in &self.codes {
            uleb(entry.len() as u64, &mut sec);
            sec.extend_from_slice(entry);
        }
        section(10, &sec, &mut out);
        out
    }
}

fn section(id: u8, contents: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    uleb(contents.len() as u64, out);
    out.extend_from_slice(contents);
}

fn name(text: &str, out: &mut Vec<u8>) {
    uleb(text.len() as u64, out);
    out.extend_from_slice(text.as_bytes());
}
