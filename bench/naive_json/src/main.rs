//! The *fair* measuring stick: a JSON parser written the way a competent Rust
//! dev writes one in an afternoon — recursive descent into an enum, std
//! `String`/`Vec`/`HashMap`, strings accumulated into a `Vec<u8>` then
//! `String::from_utf8`. No SIMD, no `memchr`, no zero-copy borrows, no serde
//! derive. This is the same *algorithm* kanso's decoder uses, so the race
//! measures the two languages, not two levels of hand-tuning. Same harness
//! shape as serde_bench: read once, decode 150×, report the mean.
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(HashMap<String, Value>),
}

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Parser { data, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while let Some(&c) = self.data.get(self.pos) {
            match c {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> u8 {
        self.data[self.pos]
    }

    fn parse_value(&mut self) -> Value {
        self.skip_ws();
        match self.peek() {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => Value::Str(self.parse_string()),
            b't' => {
                self.pos += 4;
                Value::Bool(true)
            }
            b'f' => {
                self.pos += 5;
                Value::Bool(false)
            }
            b'n' => {
                self.pos += 4;
                Value::Null
            }
            _ => self.parse_number(),
        }
    }

    fn parse_string(&mut self) -> String {
        self.pos += 1; // opening quote
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let c = self.data[self.pos];
            self.pos += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.data[self.pos];
                    self.pos += 1;
                    match e {
                        b'"' => bytes.push(b'"'),
                        b'\\' => bytes.push(b'\\'),
                        b'/' => bytes.push(b'/'),
                        b'n' => bytes.push(b'\n'),
                        b't' => bytes.push(b'\t'),
                        b'r' => bytes.push(b'\r'),
                        b'b' => bytes.push(8),
                        b'f' => bytes.push(12),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.data[self.pos..self.pos + 4]).unwrap();
                            let code = u32::from_str_radix(hex, 16).unwrap();
                            self.pos += 4;
                            let ch = char::from_u32(code).unwrap();
                            let mut buf = [0u8; 4];
                            bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => panic!("bad escape"),
                    }
                }
                _ => bytes.push(c),
            }
        }
        String::from_utf8(bytes).unwrap()
    }

    fn parse_number(&mut self) -> Value {
        let start = self.pos;
        while let Some(&c) = self.data.get(self.pos) {
            match c {
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E' => self.pos += 1,
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.data[start..self.pos]).unwrap();
        Value::Num(s.parse::<f64>().unwrap())
    }

    fn parse_array(&mut self) -> Value {
        self.pos += 1; // [
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == b']' {
            self.pos += 1;
            return Value::Arr(items);
        }
        loop {
            items.push(self.parse_value());
            self.skip_ws();
            match self.peek() {
                b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.pos += 1;
                    break;
                }
                _ => panic!("expected , or ]"),
            }
        }
        Value::Arr(items)
    }

    fn parse_object(&mut self) -> Value {
        self.pos += 1; // {
        let mut map = HashMap::new();
        self.skip_ws();
        if self.peek() == b'}' {
            self.pos += 1;
            return Value::Obj(map);
        }
        loop {
            self.skip_ws();
            let key = self.parse_string();
            self.skip_ws();
            self.pos += 1; // colon
            let val = self.parse_value();
            map.insert(key, val);
            self.skip_ws();
            match self.peek() {
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    self.pos += 1;
                    break;
                }
                _ => panic!("expected , or }}"),
            }
        }
        Value::Obj(map)
    }
}

fn decode(data: &[u8]) -> Value {
    Parser::new(data).parse_value()
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "bench/large.json".to_string());
    let data = std::fs::read(&path).expect("read large.json");

    let v = decode(&data);
    let top = match &v {
        Value::Arr(a) => a.len(),
        _ => 0,
    };
    println!("naive decoded {top} top-level values");

    let runs = 150;
    let start = Instant::now();
    for _ in 0..runs {
        let v = decode(&data);
        // touch the result so the loop can't be optimized away
        if let Value::Arr(a) = &v {
            std::hint::black_box(a.len());
        }
    }
    let per = start.elapsed() / runs;
    println!("naive mean over {runs} decodes: {per:?}");
}
