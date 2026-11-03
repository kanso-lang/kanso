//! `kanso lsp`: the language server.
//!
//! An editor talks to it in the Language Server Protocol, JSON-RPC framed by
//! `Content-Length` headers over stdin and stdout. It keeps each open buffer's
//! text, analyzes the buffer whenever it changes (see `analysis`), publishes
//! the diagnostics `kanso check` would print, and answers hovers and
//! definitions from the analysis it holds.
//!
//! The playground runs the same `Server` in the tab: `wasm::kanso_lsp` hands
//! it one message at a time and returns what it would have written. The
//! framing is the only part the browser goes without.

pub mod analysis;
pub mod json;

use analysis::{Analysis, Place, Spot};
use json::Json;

struct Doc {
    uri: String,
    text: String,
    analysis: Analysis,
}

#[derive(Default)]
pub struct Server {
    docs: Vec<Doc>,
    /// Set by `exit`; the loop stops reading after it.
    pub exited: bool,
    shut_down: bool,
}

impl Server {
    pub fn new() -> Server {
        Server::default()
    }

    /// Handle one message and return the messages it calls for: a response
    /// to a request, and a diagnostics notification for a buffer that
    /// changed. A message that is not JSON is answered with the protocol's
    /// parse error.
    pub fn handle(&mut self, message: &str) -> Vec<Json> {
        let Some(msg) = json::parse(message) else {
            return vec![error(Json::Null, -32700, "the message is not JSON")];
        };
        let id = msg.get("id").clone();
        let method = msg.get("method").str().unwrap_or("").to_string();
        let params = msg.get("params");
        let request = !id.is_null();
        if self.shut_down && request && method != "exit" {
            return vec![error(id, -32600, "the server has shut down")];
        }
        let mut out = Vec::new();
        match method.as_str() {
            "initialize" => out.push(result(id, capabilities())),
            "shutdown" => {
                self.shut_down = true;
                out.push(result(id, Json::Null));
            }
            "exit" => self.exited = true,
            "textDocument/didOpen" => {
                let doc = params.get("textDocument");
                let uri = doc.get("uri").str().unwrap_or("").to_string();
                let text = doc.get("text").str().unwrap_or("").to_string();
                out.push(self.open(uri, text));
            }
            "textDocument/didChange" => {
                let uri = params.at(&["textDocument", "uri"]).str().unwrap_or("").to_string();
                // Full sync: each change carries the whole text, and the last
                // one is the buffer.
                if let Some(text) =
                    params.get("contentChanges").arr().last().and_then(|c| c.get("text").str())
                {
                    out.push(self.open(uri, text.to_string()));
                }
            }
            "textDocument/didClose" => {
                let uri = params.at(&["textDocument", "uri"]).str().unwrap_or("").to_string();
                self.docs.retain(|d| d.uri != uri);
                out.push(publish(&uri, Json::Arr(Vec::new())));
            }
            "textDocument/hover" => {
                let answer = self.spot(params).map_or(Json::Null, |(doc, spot)| {
                    Json::obj(vec![
                        (
                            "contents",
                            Json::obj(vec![
                                ("kind", Json::str_of("markdown")),
                                ("value", Json::str_of(format!("```kanso\n{}\n```", spot.hover))),
                            ]),
                        ),
                        ("range", range(&doc.text, spot.line, spot.start, spot.end)),
                    ])
                });
                out.push(result(id, answer));
            }
            "textDocument/definition" => {
                let answer = self.spot(params).map_or(Json::Null, |(doc, spot)| {
                    Json::Arr(spot.defs.iter().map(|p| location(doc, p)).collect())
                });
                out.push(result(id, answer));
            }
            _ if request => {
                out.push(error(id, -32601, &format!("`{method}` is not a method kanso answers")))
            }
            // Notifications the server has no use for, `initialized` and
            // `$/cancelRequest` among them, are dropped as the protocol asks.
            _ => {}
        }
        out
    }

    fn open(&mut self, uri: String, text: String) -> Json {
        let analysis = analyze_guarded(&path_of(&uri), &text);
        let diagnostics =
            Json::Arr(analysis.problems.iter().map(|p| diagnostic(&text, p)).collect());
        let note = publish(&uri, diagnostics);
        match self.docs.iter_mut().find(|d| d.uri == uri) {
            Some(doc) => {
                doc.text = text;
                doc.analysis = analysis;
            }
            None => self.docs.push(Doc { uri, text, analysis }),
        }
        note
    }

    fn spot(&self, params: &Json) -> Option<(&Doc, &Spot)> {
        let uri = params.at(&["textDocument", "uri"]).str()?;
        let doc = self.docs.iter().find(|d| d.uri == uri)?;
        let line = params.at(&["position", "line"]).num()? as u32;
        let unit = params.at(&["position", "character"]).num()? as u32;
        let col = char_of(&doc.text, line, unit);
        doc.analysis.spot_at(line, col).map(|s| (doc, s))
    }
}

/// A compile that panics is the compiler's bug, and it should not take the
/// editor's server down with it: the buffer gets one diagnostic saying so.
fn analyze_guarded(file: &str, text: &str) -> Analysis {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let caught = std::panic::catch_unwind(|| analysis::analyze(file, text));
        std::panic::set_hook(hook);
        caught.unwrap_or_else(|_| Analysis {
            problems: vec![analysis::Problem {
                line: 0,
                col: 0,
                width: 0,
                kind: "internal".to_string(),
                message: "the compiler stopped on this file; `kanso check` shows where".to_string(),
                error: true,
            }],
            spots: Vec::new(),
        })
    }
    #[cfg(target_arch = "wasm32")]
    analysis::analyze(file, text)
}

fn capabilities() -> Json {
    Json::obj(vec![
        (
            "capabilities",
            Json::obj(vec![
                ("textDocumentSync", Json::Num(1.0)),
                ("hoverProvider", Json::Bool(true)),
                ("definitionProvider", Json::Bool(true)),
            ]),
        ),
        ("serverInfo", Json::obj(vec![("name", Json::str_of("kanso"))])),
    ])
}

fn result(id: Json, value: Json) -> Json {
    Json::obj(vec![("jsonrpc", Json::str_of("2.0")), ("id", id), ("result", value)])
}

fn error(id: Json, code: i32, message: &str) -> Json {
    Json::obj(vec![
        ("jsonrpc", Json::str_of("2.0")),
        ("id", id),
        (
            "error",
            Json::obj(vec![("code", Json::Num(code as f64)), ("message", Json::str_of(message))]),
        ),
    ])
}

fn publish(uri: &str, diagnostics: Json) -> Json {
    Json::obj(vec![
        ("jsonrpc", Json::str_of("2.0")),
        ("method", Json::str_of("textDocument/publishDiagnostics")),
        ("params", Json::obj(vec![("uri", Json::str_of(uri)), ("diagnostics", diagnostics)])),
    ])
}

fn diagnostic(text: &str, p: &analysis::Problem) -> Json {
    let mut fields = vec![
        ("range", range(text, p.line, p.col, p.col + p.width)),
        ("severity", Json::Num(if p.error { 1.0 } else { 2.0 })),
    ];
    if !p.kind.is_empty() {
        fields.push(("code", Json::str_of(p.kind.as_str())));
    }
    fields.push(("source", Json::str_of("kanso")));
    fields.push(("message", Json::str_of(p.message.as_str())));
    Json::obj(fields)
}

fn location(doc: &Doc, p: &Place) -> Json {
    let here = path_of(&doc.uri) == p.file;
    let uri = match here {
        true => doc.uri.clone(),
        false => uri_of(&p.file),
    };
    // A place in another file is counted in characters against a text the
    // server does not hold; on any line that is ASCII up to the name, which a
    // declaration's head is, characters and UTF-16 units agree.
    let range = match here {
        true => range(&doc.text, p.line, p.col, p.col),
        false => {
            Json::obj(vec![("start", position(p.line, p.col)), ("end", position(p.line, p.col))])
        }
    };
    Json::obj(vec![("uri", Json::str_of(uri)), ("range", range)])
}

fn position(line: u32, character: u32) -> Json {
    Json::obj(vec![("line", Json::Num(line as f64)), ("character", Json::Num(character as f64))])
}

/// A range on one line, with its columns turned from characters into the
/// UTF-16 units the protocol counts in.
fn range(text: &str, line: u32, start: u32, end: u32) -> Json {
    Json::obj(vec![
        ("start", position(line, unit_of(text, line, start))),
        ("end", position(line, unit_of(text, line, end))),
    ])
}

fn unit_of(text: &str, line: u32, col: u32) -> u32 {
    let Some(row) = text.lines().nth(line as usize) else { return col };
    let mut chars = row.chars();
    let mut units = 0;
    for _ in 0..col {
        match chars.next() {
            Some(c) => units += c.len_utf16() as u32,
            None => units += 1,
        }
    }
    units
}

fn char_of(text: &str, line: u32, unit: u32) -> u32 {
    let Some(row) = text.lines().nth(line as usize) else { return unit };
    let mut units = 0;
    for (i, c) in row.chars().enumerate() {
        if units >= unit {
            return i as u32;
        }
        units += c.len_utf16() as u32;
    }
    row.chars().count() as u32 + unit.saturating_sub(units)
}

/// The file a `file://` URI names, which is what the compile reads imports
/// beside. Anything else (the playground's buffer) is taken as a bare name.
pub fn path_of(uri: &str) -> String {
    let Some(rest) = uri.strip_prefix("file://") else { return uri.to_string() };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
        match (bytes[i], hex.and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    let path = String::from_utf8_lossy(&out).into_owned();
    // A Windows drive arrives as `/c:/...`.
    match path.as_bytes() {
        [b'/', d, b':', ..] if d.is_ascii_alphabetic() => path[1..].to_string(),
        _ => path,
    }
}

fn uri_of(path: &str) -> String {
    let absolute = match std::path::Path::new(path).is_absolute() {
        true => path.to_string(),
        false => std::env::current_dir()
            .map(|d| d.join(path).to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.to_string()),
    };
    let mut out = String::from("file://");
    for b in absolute.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Serve on stdin and stdout until the client says `exit` or closes the
/// stream. Exits 0 after a `shutdown`, 1 otherwise, as the protocol asks.
pub fn serve_stdio() -> i32 {
    use std::io::{BufRead, Read, Write};
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut server = Server::new();
    loop {
        let mut length: Option<usize> = None;
        loop {
            let mut header = String::new();
            match input.read_line(&mut header) {
                Ok(0) | Err(_) => return i32::from(!server.shut_down),
                Ok(_) => {}
            }
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().ok();
                }
            }
        }
        let Some(length) = length else { continue };
        let mut body = vec![0; length];
        if input.read_exact(&mut body).is_err() {
            return i32::from(!server.shut_down);
        }
        let replies = server.handle(&String::from_utf8_lossy(&body));
        let mut out = stdout.lock();
        for reply in replies {
            let text = reply.text();
            let _ = write!(out, "Content-Length: {}\r\n\r\n{text}", text.len());
        }
        let _ = out.flush();
        if server.exited {
            return i32::from(!server.shut_down);
        }
    }
}
