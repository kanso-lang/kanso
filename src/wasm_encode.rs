//! The two integer encodings a WebAssembly binary is written in, for the
//! translator in `ir_wasm`.

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
