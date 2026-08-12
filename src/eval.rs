use crate::ast::*;
use crate::diag::Span;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Int(BigInt),
    Str(String),
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(BigInt),
    Float(f64),
    Map(Rc<BTreeMap<MapKey, Value>>),
    Str(String),
    True,
    False,
    NoneV,
    ErrV(Rc<ErrInfo>),
    List(Rc<Vec<Value>>),
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

/// An err value carries its propagation trace: the origin baked at the
/// construction site ("{fn} at {file}:{line}"; executor-born errs have none)
/// and one hop per dispatcher failure pass-through. The happy path never
/// touches any of this — trace data lives on the err value only.
#[derive(Debug)]
pub struct ErrInfo {
    pub reason: Value,
    pub origin: Option<Rc<str>>,
    pub hops: Vec<Rc<str>>,
    /// The err this one was wrapped around, kept so a re-raise never
    /// discards what it was told (`wrap_err`).
    pub cause: Option<Rc<ErrInfo>>,
    /// Whether this err's reason is a list *of reasons* rather than one
    /// reason that happens to be a list. Merging is a fold, so three failures
    /// answer three reasons however they were grouped — and `err ["a" "b"]`
    /// stays one reason, which is why the mark cannot be read off the shape.
    pub merged: bool,
}

/// The evaluation frame an expression runs in, as an err-origin prefix
/// "{fn} at {file}"; absent where no source frame exists (the wasm host).
pub type Frame = Option<Rc<str>>;

fn frame_of(decl: &FnDecl) -> Frame {
    Some(Rc::from(format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file)))
}

pub fn origin_at(frame: &Frame, span: Span) -> Option<Rc<str>> {
    frame.as_ref().map(|prefix| Rc::from(format!("{prefix}:{}", span.line)))
}

pub fn err_value(reason: Value, origin: Option<Rc<str>>) -> Value {
    Value::ErrV(Rc::new(ErrInfo { reason, origin, hops: Vec::new(), cause: None, merged: false }))
}

/// A byte list (the scanner's `bytes`/`slice` view) as its utf-8 text, so a
/// number can be parsed straight from bytes without first materializing a
/// string. None when the bytes aren't valid utf-8 byte values.
fn bytes_to_str(items: &[Value]) -> Option<String> {
    let mut raw = Vec::with_capacity(items.len());
    for item in items {
        let Value::Int(n) = item else { return None };
        raw.push(u8::try_from(n.clone()).ok()?);
    }
    String::from_utf8(raw).ok()
}

/// A dispatcher passing a failure through appends its name; none stays bare.
///
/// A field getter is not one of them. The reader wrote `x.name`, not a call,
/// and its internal name is not theirs to see — so the trace skips it here,
/// where no backend can forget to.
pub fn hop(failure: Value, name: &str) -> Value {
    if crate::ast::getter_field(name).is_some() {
        return failure;
    }
    match failure {
        Value::ErrV(info) => {
            let mut hops = info.hops.clone();
            hops.push(Rc::from(name));
            Value::ErrV(Rc::new(ErrInfo {
                reason: info.reason.clone(),
                origin: info.origin.clone(),
                hops,
                cause: info.cause.clone(),
                merged: info.merged,
            }))
        }
        other => other,
    }
}

/// The endpoint report's trace lines, newest pass-through first, pointing
/// back toward the birth site. Byte-identical across all three engines.
pub fn trace_lines(info: &ErrInfo) -> String {
    let mut out = String::new();
    if let Some(origin) = &info.origin {
        out.push_str("  born in ");
        out.push_str(origin);
        out.push('\n');
    }
    if !info.hops.is_empty() {
        let path: Vec<&str> = info.hops.iter().rev().map(|h| &**h).collect();
        out.push_str("  passed through ");
        out.push_str(&path.join(" \u{2190} "));
        out.push('\n');
    }
    if let Some(cause) = &info.cause {
        out.push_str("  caused by: ");
        out.push_str(&render(&cause.reason, true));
        out.push('\n');
        out.push_str(&trace_lines(cause));
    }
    out
}

#[derive(Debug)]
pub struct ClosureData {
    pub params: Vec<String>,
    pub body: Expr,
    pub env: Option<Rc<Env>>,
    pub frame: Frame,
}

#[derive(Clone, Debug)]
pub enum Desc {
    Print(String, Span),
    Seq(Rc<Desc>, Rc<Desc>),
    Join(Rc<Desc>, Rc<Desc>),
    Args,
    Stdin,
    ReadFile(String),
    Write(String),
    WriteErr(String),
    Env(String),
    Exists(String),
    IsDir(String),
    ListDir(String),
    Now,
    MakeDir(String),
    WriteFile(String, String),
    Run(String, Vec<String>),
    Start(String, Vec<String>),
    Kill(i64),
    Bind(Rc<Desc>, Value),
    Await(i64),
    Listen(i64),
    Accept(i64),
    Receive(i64),
    Send(i64, String),
    CloseSocket(i64),
    Sleep(u64),
    Random(u64),
    Nil,
}

/// What a finished process answers: its status and the two streams.
fn ran_value(done: (i64, String, String)) -> Value {
    let (status, out, errs) = done;
    Value::List(Rc::new(vec![Value::Int(status.into()), Value::Str(out), Value::Str(errs)]))
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
/// propagates up through `Seq` and `Bind`, so `sleep` may sit anywhere in a
/// description and suspension needs no coroutine — the continuation is the
/// remaining Desc tree, made explicit.
enum Step {
    Done(Value),
    Blocked(u64, Rc<Desc>),
}

#[derive(Debug)]
pub struct Env {
    name: String,
    value: Value,
    parent: Option<Rc<Env>>,
}

fn bind(env: Option<Rc<Env>>, name: &str, value: Value) -> Option<Rc<Env>> {
    Some(Rc::new(Env { name: name.to_string(), value, parent: env }))
}

fn lookup(env: &Option<Rc<Env>>, name: &str) -> Option<Value> {
    let mut cur = env.as_ref();
    while let Some(frame) = cur {
        if frame.name == name {
            return Some(frame.value.clone());
        }
        cur = frame.parent.as_ref();
    }
    None
}

pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

type Bindings = Vec<(String, Value)>;
type Score = Vec<u8>;

type EvalResult = Result<Value, RuntimeError>;

/// Calling a closure that lives in another engine's table, by handle.
pub type ForeignCall = fn(u32, Vec<Value>) -> EvalResult;

thread_local! {
    /// The browser backend compiles closures into a wasm table the
    /// interpreter cannot reach on its own. It registers the way back in
    /// here, so a description carrying such a continuation runs on the same
    /// scheduler as every other description instead of needing a second one.
    static FOREIGN_CALL: std::cell::RefCell<Option<ForeignCall>> =
        const { std::cell::RefCell::new(None) };
}

/// What a body's tail handed back: a finished value, or a named-group call
/// for the dispatcher to loop on instead of growing the stack.
enum Flow {
    Done(Value),
    Tail(Rc<str>, Vec<Value>, Span),
}

pub fn set_foreign_call(call: ForeignCall) {
    FOREIGN_CALL.with(|slot| *slot.borrow_mut() = Some(call));
}

fn foreign_call(handle: u32, args: Vec<Value>, span: Span) -> EvalResult {
    match FOREIGN_CALL.with(|slot| *slot.borrow()) {
        Some(call) => call(handle, args),
        None => Err(RuntimeError {
            message: "a compiled closure escaped the engine that owns it".to_string(),
            span,
        }),
    }
}

pub trait Executor {
    fn print(&mut self, text: &str);
    fn args(&mut self) -> Vec<String>;
    fn stdin(&mut self) -> Result<String, String>;
    fn read_file(&mut self, path: &str) -> Result<String, String>;
    /// Start a process and wait for it. The answer is what it wrote and the
    /// status it ended with; a non-zero status is an outcome, not a failure,
    /// because a caller usually wants to read it rather than be stopped by
    /// it. `Err` is reserved for the process never starting at all.
    fn run(&mut self, cmd: &str, args: &[String]) -> Result<(i64, String, String), String>;
    /// Starts a process and answers a handle. A fiber running one must not
    /// hold the runtime while it waits, or nothing else in its parallel group
    /// can make the progress the process is waiting for.
    fn start(&mut self, cmd: &str, args: &[String]) -> Result<i64, String> {
        let _ = (cmd, args);
        Err("this engine cannot start processes".to_string())
    }
    /// `None` while it is still running; the scheduler puts the fiber back.
    fn finished(&mut self, _handle: i64) -> Result<Option<(i64, String, String)>, String> {
        Err("this engine cannot start processes".to_string())
    }
    /// Ends a process the program is holding, and reaps it. A program that
    /// starts something outliving its usefulness — a browser that ignores its
    /// own exit budget — has no other way to be done with it.
    fn kill(&mut self, _handle: i64) -> Result<(), String> {
        Err("this engine cannot start processes".to_string())
    }
    fn write(&mut self, text: &str);
    /// Diagnostics, so a tool's chatter stays out of what it is piped into.
    fn write_err(&mut self, text: &str);
    /// An environment variable, or None when it is unset — absence is a value
    /// here, not a failure, because not being configured is ordinary.
    fn env(&mut self, name: &str) -> Option<String>;
    fn exists(&mut self, path: &str) -> bool;

    fn is_dir(&mut self, path: &str) -> bool;
    /// Milliseconds since the epoch. KANSO_NOW pins it, the way KANSO_SEED
    /// pins the dice — a program that timestamps is otherwise unrepeatable,
    /// and this project reproduces its runs.
    fn now(&mut self) -> i64;
    /// Names within a directory, sorted, so what a program prints does not
    /// depend on the order a filesystem happens to hand them back.
    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String>;
    /// Makes a directory and every missing parent, and succeeds where it
    /// already exists — a generator run twice is the ordinary case.
    fn make_dir(&mut self, path: &str) -> Result<(), String>;
    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String>;
    /// A socket listening on a port, answering the handle a later accept
    /// names. Every engine that has a filesystem has these; the browser has
    /// neither, and says so.
    fn listen(&mut self, _port: i64) -> Result<i64, String> {
        Err("this engine has no sockets".to_string())
    }
    /// Asks whether a connection is waiting. `None` means not yet, and the
    /// scheduler puts the fiber back — a server is one statement of a
    /// parallel group and must not hold the runtime while it waits.
    fn accept(&mut self, _listener: i64) -> Result<Option<i64>, String> {
        Err("this engine has no sockets".to_string())
    }
    fn receive(&mut self, _conn: i64) -> Result<String, String> {
        Err("this engine has no sockets".to_string())
    }
    fn send(&mut self, _conn: i64, _text: &str) -> Result<(), String> {
        Err("this engine has no sockets".to_string())
    }
    fn close_socket(&mut self, _handle: i64) -> Result<(), String> {
        Err("this engine has no sockets".to_string())
    }
    /// Pause wall-clock time. The scheduler decides output *order*; sleep only
    /// makes a concurrent program take real time, so a viewer feels the
    /// overlap. Default is virtual (no wait) — output is identical either way.
    fn sleep(&mut self, _ms: u64) {}
    /// A pseudo-random int in `[0, n)` off the executor's seeded generator.
    fn random(&mut self, _n: u64) -> u64 {
        0
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::seeded()
    }
}

pub struct RealExecutor {
    pub program_args: Vec<String>,
    pub rng: Rng,
}

thread_local! {
    /// Sockets live for the run rather than in the executor, which is built
    /// in half a dozen places and copied into none of them. A handle is the
    /// number a program holds; what it names stays here.
    static SOCKETS: std::cell::RefCell<Sockets> = std::cell::RefCell::new(Sockets::default());
    static KIDS: std::cell::RefCell<Kids> = std::cell::RefCell::new(Kids::default());
}

#[derive(Default)]
struct Kids {
    next: i64,
    running: std::collections::HashMap<i64, std::process::Child>,
}

#[derive(Default)]
struct Sockets {
    next: i64,
    listeners: std::collections::HashMap<i64, std::net::TcpListener>,
    conns: std::collections::HashMap<i64, std::net::TcpStream>,
}

impl Sockets {
    fn hand(&mut self) -> i64 {
        self.next += 1;
        self.next
    }
}

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }

    fn start(&mut self, cmd: &str, args: &[String]) -> Result<i64, String> {
        let child = std::process::Command::new(cmd)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| format!("cannot start {cmd}"))?;
        KIDS.with(|k| {
            let mut k = k.borrow_mut();
            k.next += 1;
            let handle = k.next;
            k.running.insert(handle, child);
            Ok(handle)
        })
    }

    fn kill(&mut self, handle: i64) -> Result<(), String> {
        KIDS.with(|k| {
            let mut k = k.borrow_mut();
            let Some(mut child) = k.running.remove(&handle) else {
                return Err(format!("{handle} is not a running process"));
            };
            let _ = child.kill();
            child.wait().map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    fn finished(&mut self, handle: i64) -> Result<Option<(i64, String, String)>, String> {
        use std::io::Read;
        KIDS.with(|k| {
            let mut k = k.borrow_mut();
            let Some(child) = k.running.get_mut(&handle) else {
                return Err(format!("{handle} is not a running process"));
            };
            let status = child.try_wait().map_err(|e| e.to_string())?;
            let Some(status) = status else { return Ok(None) };
            let mut child = k.running.remove(&handle).expect("just seen");
            let mut out = String::new();
            let mut errs = String::new();
            if let Some(pipe) = child.stdout.as_mut() {
                let _ = pipe.read_to_string(&mut out);
            }
            if let Some(pipe) = child.stderr.as_mut() {
                let _ = pipe.read_to_string(&mut errs);
            }
            // try_wait already reaped it; this says so to anything reading the
            // types rather than the sequence.
            let _ = child.wait();
            Ok(Some((status.code().map_or(128, i64::from), out, errs)))
        })
    }

    /// A port of 0 asks the operating system for a free one, and the handle
    /// answers it — a test that pinned a port would collide with whatever
    /// else is running on the machine.
    fn listen(&mut self, port: i64) -> Result<i64, String> {
        let socket = std::net::TcpListener::bind(("127.0.0.1", port as u16))
            .map_err(|e| format!("cannot listen on port {port}: {e}"))?;
        SOCKETS.with(|s| {
            let mut s = s.borrow_mut();
            let handle = s.hand();
            s.listeners.insert(handle, socket);
            Ok(handle)
        })
    }

    fn accept(&mut self, listener: i64) -> Result<Option<i64>, String> {
        let socket = SOCKETS.with(|s| {
            let s = s.borrow();
            let found = s.listeners.get(&listener);
            found.map(|l| l.try_clone()).transpose().map_err(|e| e.to_string())
        })?;
        let Some(socket) = socket else {
            return Err(format!("{listener} is not a listener"));
        };
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        let taken = match socket.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(format!("cannot accept: {e}")),
        };
        // the connection itself blocks again, which is what a fiber holding
        // one wants: it has work to do and nothing to wait for
        taken.set_nonblocking(false).map_err(|e| e.to_string())?;
        SOCKETS.with(|s| {
            let mut s = s.borrow_mut();
            let handle = s.hand();
            s.conns.insert(handle, taken);
            Ok(Some(handle))
        })
    }

    /// One read of what has arrived. An HTTP request that spans packets is
    /// the caller's problem to notice, which is what content-length is for.
    fn receive(&mut self, conn: i64) -> Result<String, String> {
        use std::io::Read;
        let mut stream = SOCKETS.with(|s| {
            let s = s.borrow();
            let found = s.conns.get(&conn);
            found.map(|c| c.try_clone()).transpose().map_err(|e| e.to_string())
        })?;
        let Some(stream) = stream.as_mut() else {
            return Err(format!("{conn} is not a connection"));
        };
        let mut buffer = [0u8; 65536];
        let read = stream.read(&mut buffer).map_err(|e| format!("cannot read: {e}"))?;
        Ok(String::from_utf8_lossy(&buffer[..read]).into_owned())
    }

    fn send(&mut self, conn: i64, text: &str) -> Result<(), String> {
        use std::io::Write;
        let mut stream = SOCKETS.with(|s| {
            let s = s.borrow();
            let found = s.conns.get(&conn);
            found.map(|c| c.try_clone()).transpose().map_err(|e| e.to_string())
        })?;
        let Some(stream) = stream.as_mut() else {
            return Err(format!("{conn} is not a connection"));
        };
        stream.write_all(text.as_bytes()).map_err(|e| format!("cannot write: {e}"))?;
        stream.flush().map_err(|e| format!("cannot write: {e}"))
    }

    fn close_socket(&mut self, handle: i64) -> Result<(), String> {
        SOCKETS.with(|s| {
            let mut s = s.borrow_mut();
            let closed = s.conns.remove(&handle).is_some() || s.listeners.remove(&handle).is_some();
            match closed {
                true => Ok(()),
                false => Err(format!("{handle} is not an open socket")),
            }
        })
    }

    fn write(&mut self, text: &str) {
        use std::io::Write;
        print!("{text}");
        let _ = std::io::stdout().flush();
    }

    fn write_err(&mut self, text: &str) {
        use std::io::Write;
        eprint!("{text}");
        let _ = std::io::stderr().flush();
    }

    fn env(&mut self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn exists(&mut self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn is_dir(&mut self, path: &str) -> bool {
        std::path::Path::new(path).is_dir()
    }

    fn now(&mut self) -> i64 {
        pinned_now().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        // the wording matches the native runtime's, since the three engines
        // are held byte-identical on what a program prints
        let entries = std::fs::read_dir(path)
            .map_err(|_| format!("cannot list {path}: no such directory or unreadable"))?;
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    fn sleep(&mut self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    fn random(&mut self, n: u64) -> u64 {
        self.rng.below(n)
    }

    fn args(&mut self) -> Vec<String> {
        self.program_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer).map_err(|e| e.to_string())?;
        Ok(buffer)
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        read_file_text(path)
    }

    fn run(&mut self, cmd: &str, args: &[String]) -> Result<(i64, String, String), String> {
        let done = std::process::Command::new(cmd)
            .args(args)
            .output()
            // no OS detail: the native engine learns of a failed exec only
            // through the child's exit code, and the wording has to match
            .map_err(|_| format!("cannot start {cmd}"))?;
        // a process killed by a signal carries no status of its own; 128 plus
        // the signal is what a shell reports, and a caller comparing against
        // zero gets the same answer either way
        let status = done.status.code().map_or(128, i64::from);
        let out = String::from_utf8_lossy(&done.stdout).into_owned();
        let errs = String::from_utf8_lossy(&done.stderr).into_owned();
        Ok((status, out, errs))
    }

    fn make_dir(&mut self, path: &str) -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|_| format!("cannot make {path}"))
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String> {
        std::fs::write(path, content).map_err(|_| format!("cannot write {path}"))
    }
}

/// Reading a file, worded once. The native runtime has a fixed message and
/// three executors implement this trait, so an OS string interpolated in any
/// one of them is a divergence waiting to happen — and it was one: the
/// interpreter said `No such file or directory (os error 2)` where native said
/// `no such file or unreadable`. The fixed wording is also the only one that
/// does not change with the host's libc or this compiler's version, which a
/// reproducible diagnostic cannot afford.
pub fn read_file_text(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|_| format!("cannot read {path}: no such file or unreadable"))
}

#[derive(Default)]
pub struct ScriptedExecutor {
    pub transcript: Vec<String>,
    pub script_args: Vec<String>,
    pub script_stdin: String,
    pub files: std::collections::HashMap<String, String>,
    pub rng: Rng,
}

impl Executor for ScriptedExecutor {
    fn print(&mut self, text: &str) {
        self.transcript.push(format!("print {text:?}"));
    }

    fn write(&mut self, text: &str) {
        self.transcript.push(format!("write {text:?}"));
    }

    fn write_err(&mut self, text: &str) {
        self.transcript.push(format!("write_err {text:?}"));
    }

    fn env(&mut self, name: &str) -> Option<String> {
        self.transcript.push(format!("env {name:?}"));
        None
    }

    fn is_dir(&mut self, path: &str) -> bool {
        self.transcript.push(format!("is_dir {path:?}"));
        std::path::Path::new(path).is_dir()
    }

    fn exists(&mut self, path: &str) -> bool {
        self.transcript.push(format!("exists {path:?}"));
        false
    }

    fn now(&mut self) -> i64 {
        self.transcript.push("now".to_string());
        0
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        self.transcript.push(format!("list_dir {path:?}"));
        Ok(Vec::new())
    }

    fn random(&mut self, n: u64) -> u64 {
        self.rng.below(n)
    }

    fn args(&mut self) -> Vec<String> {
        self.script_args.clone()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Ok(self.script_stdin.clone())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        self.transcript.push(format!("read_file {path:?}"));
        self.files.get(path).cloned().ok_or_else(|| format!("cannot read {path}"))
    }

    fn run(&mut self, cmd: &str, args: &[String]) -> Result<(i64, String, String), String> {
        self.transcript.push(format!("run {cmd:?} {args:?}"));
        Ok((0, String::new(), String::new()))
    }

    fn make_dir(&mut self, path: &str) -> Result<(), String> {
        self.transcript.push(format!("make_dir {path:?}"));
        Ok(())
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String> {
        self.transcript.push(format!("write_file {path:?} {content:?}"));
        Ok(())
    }
}

pub struct Interp<'a> {
    fns: HashMap<&'a str, Vec<&'a FnDecl>>,
    types: HashMap<&'a str, &'a TypeDecl>,
    entry_decl: TypeDecl,
    demand: crate::demand::DemandInfo,
    pub thunk_stats: ThunkStats,
    depth: Cell<usize>,
    /// Computed once, because a stack exhaustion is reported where the cause
    /// is no longer visible: the interpreter sees only that it is deep.
    stack_hint: String,
    /// One cell per self-referential constant. A mention that arrives while
    /// the constant is still being computed gets the unforced cell, which is
    /// how a value that names itself gets a value at all.
    knots: RefCell<HashMap<String, Rc<RefCell<ThunkState>>>>,
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
        let mut fns: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        for decl in &program.fns {
            fns.entry(&decl.name).or_default().push(decl);
        }
        // proximity breaks specificity ties: local arms come before
        // bare-enrolled clones, so a same-shape local wins its own file
        for overloads in fns.values_mut() {
            overloads.sort_by_key(|d| d.synthetic);
        }
        let types = program.types.iter().map(|t| (t.name.as_str(), t)).collect();
        TYPESETS.with(|reg| {
            *reg.borrow_mut() = program
                .types
                .iter()
                .filter(|t| !t.members.is_empty())
                .map(|t| (t.name.clone(), t.members.clone()))
                .collect();
        });
        let origin = Span { line: 0, col: 0 };
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
        };
        let demand = crate::demand::analyze(program);
        Interp {
            fns,
            types,
            entry_decl,
            demand,
            thunk_stats: ThunkStats::default(),
            depth: Cell::new(0),
            stack_hint: crate::stack_hint(program),
            knots: RefCell::new(HashMap::new()),
        }
    }

    /// Evaluate a declaration's body with its lazy bind sites in view.
    fn eval_body_of(&self, decl: &FnDecl, env: Option<Rc<Env>>) -> EvalResult {
        self.eval_body_in(decl, &decl.body, env, &frame_of(decl))
    }

    /// A body run whose final expression may hand back a tail call for the
    /// dispatcher's loop instead of recursing. Everything before the last
    /// statement evaluates exactly as eval_body_in does.
    fn eval_body_flow(&self, decl: &FnDecl, env: Option<Rc<Env>>) -> Result<Flow, RuntimeError> {
        let frame = frame_of(decl);
        let body = &decl.body;
        let Some((Stmt::Expr(last), lead)) = body.split_last() else {
            return Ok(Flow::Done(self.eval_body_in(decl, body, env, &frame)?));
        };
        let mut env = env;
        for (index, stmt) in lead.iter().enumerate() {
            match stmt {
                Stmt::Set { .. } => unreachable!("`set` parses only inside `build`"),
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), index) =>
                {
                    self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
                    let cell = Rc::new(RefCell::new(ThunkState::Pending {
                        expr: expr.clone(),
                        env: env.clone(),
                        frame: frame.clone(),
                    }));
                    env = bind(env, name, Value::Thunk(cell));
                }
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, &frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self.destructure(pattern, value, env, expr.span())?;
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
    ) -> Result<Flow, RuntimeError> {
        let Expr::App { head, args, span, piped } = expr else {
            return Ok(Flow::Done(self.eval(expr, env, frame)?));
        };
        if matches!(head.as_ref(), Expr::Partial(..)) {
            return Ok(Flow::Done(self.eval(expr, env, frame)?));
        }
        let group_of = |callee: &Value| -> Option<Rc<str>> {
            let Value::FnRef(n) = callee else { return None };
            let plain = &**n != "err" && &**n != "if" && self.type_decl(n).is_none();
            (plain && self.fns.contains_key(&**n)).then(|| n.clone())
        };
        if *piped && !args.is_empty() {
            let piped_value = self.eval(&args[0], env, frame)?;
            if is_failure(&piped_value) {
                return Ok(Flow::Done(piped_value));
            }
            if let Value::Desc(inner) = piped_value {
                let mut body_args: Vec<Expr> = vec![Expr::Ident("__piped".to_string(), *span)];
                body_args.extend(args[1..].iter().cloned());
                let closure = Value::Closure(Rc::new(ClosureData {
                    params: vec!["__piped".to_string()],
                    body: Expr::App {
                        head: head.clone(),
                        args: body_args,
                        span: *span,
                        piped: false,
                    },
                    env: env.clone(),
                    frame: frame.clone(),
                }));
                return Ok(Flow::Done(Value::Desc(Rc::new(Desc::Bind(inner, closure)))));
            }
            let callee = self.eval(head, env, frame)?;
            let mut values = vec![piped_value];
            for arg in &args[1..] {
                values.push(self.eval(arg, env, frame)?);
            }
            return match group_of(&callee) {
                Some(name) => Ok(Flow::Tail(name, values, *span)),
                None => Ok(Flow::Done(self.call(callee, values, *span, frame)?)),
            };
        }
        let callee = self.eval(head, env, frame)?;
        if matches!(&callee, Value::FnRef(n) if &**n == "if") && args.len() == 3 {
            let cond = self.force(self.eval(&args[0], env, frame)?)?;
            return match cond {
                Value::True => self.eval_tail(&args[1], env, frame),
                Value::False => self.eval_tail(&args[2], env, frame),
                bad if is_failure(&bad) => Ok(Flow::Done(bad)),
                other => Err(RuntimeError {
                    message: format!(
                        "an if condition is true or false, got {}",
                        render(&other, false)
                    ),
                    span: *span,
                }),
            };
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.eval(arg, env, frame)?);
        }
        match group_of(&callee) {
            Some(name) => Ok(Flow::Tail(name, values, *span)),
            None => Ok(Flow::Done(self.call(callee, values, *span, frame)?)),
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
    fn eval_stmts(&self, stmts: &[Stmt], env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        let mut env = env.clone();
        let mut result = Value::NoneV;
        for stmt in stmts {
            match stmt {
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self.destructure(pattern, value, env, expr.span())?;
                }
                Stmt::Expr(expr) => result = self.eval(expr, &env, frame)?,
                Stmt::Set { target, field, value, span } => {
                    let current = lookup(&env, target).ok_or_else(|| RuntimeError {
                        message: format!("`set` target `{target}` is not bound"),
                        span: *span,
                    })?;
                    let current = self.force_thunk(current)?;
                    // a constructor given a failure handed the failure
                    // back, so the target is not a record to write to
                    if is_failure(&current) {
                        continue;
                    }
                    let Value::Record { ty, fields } = &current else {
                        return Err(RuntimeError {
                            message: format!(
                                "`set` writes a record field, not {}",
                                render(&current, true)
                            ),
                            span: *span,
                        });
                    };
                    let decl = self.type_decl(ty).expect("constructed types are declared");
                    let position = decl.fields.iter().position(|(f, _, _)| f == field);
                    let Some(position) = position else {
                        return Err(RuntimeError {
                            message: format!("`{ty}` has no field `{field}`"),
                            span: *span,
                        });
                    };
                    let new = self.eval(value, &env, frame)?;
                    if is_failure(&new) {
                        return Ok(new);
                    }
                    fields.borrow_mut()[position] = new;
                }
            }
        }
        Ok(result)
    }

    fn eval_body_in(
        &self,
        decl: &FnDecl,
        body: &[Stmt],
        mut env: Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        let mut result = Value::NoneV;
        for (index, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Set { .. } => unreachable!("`set` parses only inside `build`"),
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&decl.name, decl.params.len(), index) =>
                {
                    self.thunk_stats.allocs.set(self.thunk_stats.allocs.get() + 1);
                    let cell = Rc::new(RefCell::new(ThunkState::Pending {
                        expr: expr.clone(),
                        env: env.clone(),
                        frame: frame.clone(),
                    }));
                    env = bind(env, name, Value::Thunk(cell));
                }
                Stmt::Bind { pattern, expr } => {
                    let mut value = self.eval(expr, &env, frame)?;
                    if !matches!(pattern, Pattern::Var(..)) {
                        value = self.force_thunk(value)?;
                    }
                    env = self.destructure(pattern, value, env, expr.span())?;
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
            Pattern::Var(name, _) => Ok(bind(env, name, value)),
            Pattern::Ctor { ty, .. } => {
                let mut binds = Vec::new();
                match match_one(pattern, &value, &mut binds) {
                    Some(_) => {
                        let mut env = env;
                        for (name, bound) in binds {
                            env = bind(env, &name, bound);
                        }
                        Ok(env)
                    }
                    None => Err(RuntimeError {
                        message: format!(
                            "cannot destructure {} as `{ty}`; bindings are irrefutable, so \
                             handle other types by dispatch first",
                            render(&value, true)
                        ),
                        span,
                    }),
                }
            }
            Pattern::Keyed { entries, .. } => {
                let Value::Record { ty, fields } = &value else {
                    return Err(RuntimeError {
                        message: format!(
                            "cannot read fields of {}; keyed reads take a record",
                            render(&value, true)
                        ),
                        span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                if entries.len() >= decl.fields.len() {
                    return Err(RuntimeError {
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
                            message: format!("`{ty}` has no field `{}`", entry.field),
                            span,
                        });
                    };
                    env = bind(env, &entry.bind_name, fields.borrow()[position].clone());
                }
                Ok(env)
            }
            _ => Err(RuntimeError {
                message: "binding patterns are irrefutable: names and constructor \
                          patterns only"
                    .to_string(),
                span,
            }),
        }
    }

    fn eval(&self, expr: &Expr, env: &Option<Rc<Env>>, frame: &Frame) -> EvalResult {
        match expr {
            Expr::Int(n, _) => Ok(Value::Int(n.clone())),
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
                                message: format!("`:{ty}` widens; this value is not a {ty}"),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => self.eval_stmts(stmts, env, frame),
            Expr::Float(x, _) => Ok(Value::Float(*x)),
            Expr::MapLit(pairs, span) => {
                let mut entries = BTreeMap::new();
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
            Expr::Str(parts, _) => self.eval_template(parts, env, frame),
            Expr::Ident(name, span) => self.eval_ident(name, *span, env),
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
                        message: format!(
                            "`.` reads a field of a record, not {}",
                            render(&value, true)
                        ),
                        span: *span,
                    });
                };
                let decl = self.type_decl(ty).expect("constructed types are declared");
                let position = decl.fields.iter().position(|(f, _, _)| f == name);
                match position {
                    Some(position) => Ok(fields.borrow()[position].clone()),
                    None => Err(RuntimeError {
                        message: format!("`{ty}` has no field `{name}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.force_thunk(self.eval(base, env, frame)?)?;
                let key = self.force_thunk(self.eval(index, env, frame)?)?;
                match index_value(container, key.clone(), *span)? {
                    Value::NoneV if *strict => Ok(err_value(
                        Value::Str(format!("missing index {}", render(&key, true))),
                        origin_at(frame, *span),
                    )),
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
                        let mut body_args: Vec<Expr> =
                            vec![Expr::Ident("__piped".to_string(), *span)];
                        body_args.extend(args[1..].iter().cloned());
                        let closure = Value::Closure(Rc::new(ClosureData {
                            params: vec!["__piped".to_string()],
                            body: Expr::App {
                                head: head.clone(),
                                args: body_args,
                                span: *span,
                                piped: false,
                            },
                            env: env.clone(),
                            frame: frame.clone(),
                        }));
                        return Ok(Value::Desc(Rc::new(Desc::Bind(inner, closure))));
                    }
                    let callee = self.eval(head, env, frame)?;
                    let mut values = vec![piped_value];
                    for arg in &args[1..] {
                        values.push(self.eval(arg, env, frame)?);
                    }
                    return self.call(callee, values, *span, frame);
                }
                let callee = self.eval(head, env, frame)?;
                let lazy_if = matches!(&callee, Value::FnRef(name) if &**name == "if");
                let mut values = Vec::new();
                for arg in args {
                    match lazy_if {
                        true => values.push(Value::Closure(Rc::new(ClosureData {
                            params: Vec::new(),
                            body: arg.clone(),
                            env: env.clone(),
                            frame: frame.clone(),
                        }))),
                        false => values.push(self.eval(arg, env, frame)?),
                    }
                }
                self.call(callee, values, *span, frame)
            }
            Expr::Seq(lhs, rhs, span) => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                // The wall is ordered, so the first failure is the answer and
                // what follows it never speaks. A parallel group accumulates
                // because nothing there is first. Dependence decides, which is
                // the rule chapter four states.
                if is_failure(&left) {
                    return Ok(left);
                }
                if is_failure(&right) {
                    return Ok(right);
                }
                match (left, right) {
                    (Value::Desc(a), Value::Desc(b)) => Ok(Value::Desc(Rc::new(Desc::Seq(a, b)))),
                    _ => Err(RuntimeError {
                        message: "`>>` sequences two effect descriptions".to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Lambda { params, body, .. } => Ok(Value::Closure(Rc::new(ClosureData {
                params: params.iter().map(|(n, _)| n.clone()).collect(),
                body: (**body).clone(),
                env: env.clone(),
                frame: frame.clone(),
            }))),
            Expr::BinOp { op, lhs, rhs, span } => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                // records dispatch to the operator's user arms; numbers stay
                // on the builtin (coherence licenses the fast path, and the
                // orphan rule keeps 2 + 3 meaning one thing forever). Either
                // side carries the question: an arm naming its type in the
                // second position is a declaration too, and a gate reading
                // only the first leaves it matching nothing.
                if (routes_to_arms(&left) || routes_to_arms(&right))
                    && self.fns.contains_key(op as &str)
                {
                    return self.call_named(op, vec![left, right], *span, frame);
                }
                eval_binop(op, sub_base(left), sub_base(right), *span, frame)
            }
            Expr::Guard { cond, early, rest, span } => {
                let c = self.eval(cond, env, frame)?;
                match c {
                    Value::True => self.eval(early, env, frame),
                    Value::False => self.eval_stmts(rest, env, frame),
                    bad if is_failure(&bad) => Ok(bad),
                    other => Err(RuntimeError {
                        message: format!(
                            "a return condition is true or false, got {}",
                            render(&other, false)
                        ),
                        span: *span,
                    }),
                }
            }
            Expr::Join { lhs, rhs, span } => {
                let left = self.force_thunk(self.eval(lhs, env, frame)?)?;
                let right = self.force_thunk(self.eval(rhs, env, frame)?)?;
                join_values(left, right, *span)
            }
        }
    }

    fn eval_ident(&self, name: &str, span: Span, env: &Option<Rc<Env>>) -> EvalResult {
        if let Some(value) = lookup(env, name) {
            return Ok(value);
        }
        if let Some(decls) = self.fns.get(name) {
            if let Some(constant) = decls.iter().find(|d| d.params.is_empty()) {
                // Every constant goes through the knot, not only one that
                // names itself. `knotted` installs a blackhole before it
                // evaluates, so re-entering the name finds it — and a cycle
                // reaches its second name as readily as its first. Asking
                // whether a constant mentions its own name sees `a = f b` and
                // `b = f a` as two ordinary constants, and evaluating either
                // recurses until the process dies.
                return self.knotted(name, constant);
            }
        }
        match name.strip_prefix("builtin_").unwrap_or(name) {
            "args" => return Ok(Value::Desc(Rc::new(Desc::Args))),
            "stdin" => return Ok(Value::Desc(Rc::new(Desc::Stdin))),
            "now" => return Ok(Value::Desc(Rc::new(Desc::Now))),
            _ => {}
        }
        if let Some(decl) = self.type_decl(name) {
            if decl.parent.is_none() && decl.members.is_empty() && decl.fields.is_empty() {
                return Ok(Value::Record {
                    ty: Rc::from(name),
                    fields: Rc::new(RefCell::new(Vec::new())),
                });
            }
        }
        match name {
            "true" => Ok(Value::True),
            "false" => Ok(Value::False),
            "none" => Ok(Value::NoneV),
            _ if self.fns.contains_key(name)
                || self.types.contains_key(name)
                || name == "err"
                || crate::check::BUILTINS.contains(&name)
                || crate::check::HARNESS.contains(&name)
                || name
                    .strip_prefix("builtin_")
                    .is_some_and(|n| crate::check::BUILTINS.contains(&n)) =>
            {
                Ok(Value::FnRef(Rc::from(name)))
            }
            _ => Err(RuntimeError { message: format!("unknown name `{name}`"), span }),
        }
    }

    fn eval_template(
        &self,
        parts: &[TemplatePart],
        env: &Option<Rc<Env>>,
        frame: &Frame,
    ) -> EvalResult {
        let mut out = String::new();
        for part in parts {
            match part {
                TemplatePart::Lit(s) => out.push_str(s),
                TemplatePart::Interp(expr) => {
                    let value = self.force_thunk(self.eval(expr, env, frame)?)?;
                    // an err propagates (the whole string returns the err); a
                    // none is a value and renders its sentinel
                    if matches!(value, Value::ErrV(_)) {
                        return Ok(value);
                    }
                    match self.render_interpolated(value)? {
                        Ok(rendered) => out.push_str(&rendered),
                        Err(err) => return Ok(err),
                    }
                }
            }
        }
        Ok(Value::Str(out))
    }

    fn call(&self, callee: Value, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        match callee {
            Value::FnRef(name) => self.call_named(&name, args, span, frame),
            Value::Closure(closure) => self.call_closure(&closure, args, span),
            Value::TableFn(handle) => foreign_call(handle, args, span),
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
                message: format!("`{}` is not callable", render(&other, false)),
                span,
            }),
        }
    }

    /// Every argument count the callee answers, smallest first.
    fn arities_of(&self, callee: &Value) -> Vec<usize> {
        match callee {
            Value::FnRef(name) => {
                let mut found: Vec<usize> = self
                    .fns
                    .get(name.as_ref())
                    .map(|decls| decls.iter().map(|d| d.params.len()).collect())
                    .unwrap_or_default();
                found.sort_unstable();
                found.dedup();
                found
            }
            Value::Closure(c) => vec![c.params.len()],
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
            message: format!("this function takes {short} argument(s), got 0"),
            span,
        })
    }

    fn call_closure(&self, closure: &ClosureData, args: Vec<Value>, span: Span) -> EvalResult {
        if closure.params.len() != args.len() {
            return Err(RuntimeError {
                message: format!(
                    "this function takes {} argument(s), got {}",
                    closure.params.len(),
                    args.len()
                ),
                span,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(merged_failures(&args));
        }
        let mut env = closure.env.clone();
        for (name, value) in closure.params.iter().zip(args) {
            env = bind(env, name, value);
        }
        self.eval(&closure.body, &env, &closure.frame)
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
                message: format!(
                    "no {}-argument arm of `{}` (arms take {})",
                    args.len(),
                    render(&callee, false),
                    names.join(" or ")
                ),
                span,
            });
        }
        Ok(Value::Partial(Rc::new(callee), Rc::new(args)))
    }

    fn call_named(&self, name: &str, args: Vec<Value>, span: Span, frame: &Frame) -> EvalResult {
        if name == "err" {
            let [reason] = arity(args, name, span)?;
            let reason = self.force_thunk(reason)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            return Ok(err_value(reason, origin_at(frame, span)));
        }
        if let Some(ty) = self.type_decl(name) {
            let args =
                args.into_iter().map(|a| self.force_thunk(a)).collect::<Result<Vec<_>, _>>()?;
            return self.construct(ty, args, span);
        }
        if let Some(overloads) = self.fns.get(name) {
            return self.dispatch(name, overloads, args, span);
        }
        let args = args.into_iter().map(|a| self.force_thunk(a)).collect::<Result<Vec<_>, _>>()?;
        self.call_builtin(name, args, span, frame)
    }

    fn construct(&self, ty: &TypeDecl, args: Vec<Value>, span: Span) -> EvalResult {
        if !ty.members.is_empty() {
            return Err(RuntimeError {
                message: format!("`{}` is a typeset — it only annotates", ty.name),
                span,
            });
        }
        if let Some(parent) = &ty.parent {
            if args.len() != 1 {
                return Err(RuntimeError {
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
                    message: format!("`{}` wraps a {}", ty.name, parent),
                    span,
                });
            }
            let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
            return Ok(Value::Sub { ty: Rc::from(canonical), inner: Rc::new(inner) });
        }
        if args.len() != ty.fields.len() {
            return Err(RuntimeError {
                message: format!(
                    "`{}` has {} field(s), got {} (construction is positional, fields \
                     alphabetical)",
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
                    message: format!("field `{field}` of `{}` takes {}", ty.name, tys.join(" ")),
                    span,
                });
            }
        }
        let canonical = ty.origin.as_deref().unwrap_or(ty.name.as_str());
        Ok(Value::Record { ty: Rc::from(canonical), fields: Rc::new(RefCell::new(args)) })
    }

    fn dispatch(
        &self,
        name: &str,
        overloads: &[&FnDecl],
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        self.dispatch_loop(name.to_string(), overloads.to_vec(), args, span)
    }

    /// The dispatcher's trampoline: a tail call to a named group re-enters
    /// arm selection here instead of growing the Rust stack, so deep tail
    /// recursion — self or mutual — runs in constant interpreter stack, the
    /// way the native engine's musttail already made it run in constant
    /// machine stack. The oracle should not be the engine that fails first.
    fn dispatch_loop(
        &self,
        name: String,
        overloads: Vec<&FnDecl>,
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
        self.depth.set(self.depth.get() + 1);
        if self.depth.get() > 10_000 {
            self.depth.set(self.depth.get() - 1);
            return Err(RuntimeError {
                message: format!(
                    "the program ran out of stack: recursion went deeper than the stack holds{}",
                    self.stack_hint
                ),
                span,
            });
        }
        let result = self.dispatch_loop_inner(name, overloads, args, span);
        self.depth.set(self.depth.get() - 1);
        result
    }

    fn dispatch_loop_inner(
        &self,
        name: String,
        overloads: Vec<&FnDecl>,
        args: Vec<Value>,
        span: Span,
    ) -> EvalResult {
        let mut name = name;
        let mut overloads = overloads;
        let mut args = args;
        let mut span = span;
        loop {
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
            let mut best: Option<(Score, &FnDecl, Bindings)> = None;
            for decl in &overloads {
                if decl.params.len() != args.len() {
                    continue;
                }
                let Some((score, binds)) = match_params(&decl.params, &args) else { continue };
                let replace = match &best {
                    Some((best_score, ..)) => score > *best_score,
                    None => true,
                };
                if replace {
                    best = Some((score, decl, binds));
                }
            }
            match best {
                Some((_, decl, binds)) => {
                    let mut env = None;
                    for (bind_name, value) in binds {
                        env = bind(env, &bind_name, value);
                    }
                    match self.eval_body_flow(decl, env)? {
                        Flow::Done(value) => return Ok(value),
                        Flow::Tail(next, next_args, next_span) => {
                            overloads =
                                self.fns.get(&*next).expect("tails name real groups").clone();
                            name = next.to_string();
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
                            message: match (crate::ast::getter_field(&name), subject) {
                                (Some(field), Some(Value::Record { ty, .. })) => {
                                    format!("`{ty}` has no field `{field}`")
                                }
                                (Some(_), Some(value)) => format!(
                                    "`.` reads a field of a record, not {}",
                                    render(&value, true)
                                ),
                                _ => format!("no overload of `{name}` matches these arguments"),
                            },
                            span,
                        }),
                    };
                }
            }
        }
    }

    pub fn call_builtin(
        &self,
        name: &str,
        args: Vec<Value>,
        span: Span,
        frame: &Frame,
    ) -> EvalResult {
        // std wrapper modules reach natives through the builtin_ prefix;
        // the checker gates those names to std-origin files
        let name = name.strip_prefix("builtin_").unwrap_or(name);
        let args: Vec<Value> = args.into_iter().map(sub_base).collect();
        if name == "if" {
            return self.builtin_if(args, span);
        }
        // the one hole in err's infectiousness: wrap_err's second argument
        // IS the err being wrapped, so it arrives as a value
        if name == "wrap_err" {
            let [reason, original] = arity(args, name, span)?;
            if is_failure(&reason) {
                return Ok(reason);
            }
            let Value::ErrV(cause) = original else {
                return Err(RuntimeError {
                    message: "wrap_err takes a reason and the err it wraps".to_string(),
                    span,
                });
            };
            return Ok(Value::ErrV(Rc::new(ErrInfo {
                reason,
                origin: origin_at(frame, span),
                hops: Vec::new(),
                cause: Some(cause),
                merged: false,
            })));
        }
        // `failed?` exists to look at a failure, so it is asked before a
        // failing argument answers for the whole call.
        if name == "failed?" {
            let [value] = arity(args, name, span)?;
            return Ok(match is_failure(&value) {
                true => Value::True,
                false => Value::False,
            });
        }
        if args.iter().any(is_failure) {
            return Ok(merged_failures(&args));
        }
        match name {
            "kill" => {
                let [handle] = arity(args, name, span)?;
                let Value::Int(handle) = &handle else {
                    return Err(RuntimeError {
                        message: "kill takes a started process".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Kill(handle.to_i64().unwrap_or(-1)))))
            }
            "run" | "start" => {
                let [cmd, args_list] = arity(args, name, span)?;
                let Value::Str(cmd) = cmd else {
                    return Err(RuntimeError {
                        message: format!("{name} takes a command string"),
                        span,
                    });
                };
                let Value::List(items) = args_list else {
                    return Err(RuntimeError {
                        message: format!("{name} takes a list of argument strings"),
                        span,
                    });
                };
                let mut argv = Vec::new();
                for item in items.iter() {
                    let Value::Str(text) = item else {
                        return Err(RuntimeError {
                            message: format!("{name} takes a list of argument strings"),
                            span,
                        });
                    };
                    argv.push(text.clone());
                }
                Ok(Value::Desc(Rc::new(match name {
                    "start" => Desc::Start(cmd, argv),
                    _ => Desc::Run(cmd, argv),
                })))
            }
            "read_file" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError {
                        message: "read_file takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::ReadFile(path))))
            }
            "write" => {
                let [content] = arity(args, name, span)?;
                let Value::Str(content) = content else {
                    return Err(RuntimeError { message: "write takes a string".to_string(), span });
                };
                Ok(Value::Desc(Rc::new(Desc::Write(content))))
            }
            "now" => {
                let [] = arity(args, name, span)?;
                Ok(Value::Desc(Rc::new(Desc::Now)))
            }
            "exists" | "is_dir" | "list_dir" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = path else {
                    return Err(RuntimeError { message: format!("{name} takes a string"), span });
                };
                Ok(Value::Desc(Rc::new(match name {
                    "exists" => Desc::Exists(path),
                    "is_dir" => Desc::IsDir(path),
                    _ => Desc::ListDir(path),
                })))
            }
            "env" => {
                let [name] = arity(args, name, span)?;
                let Value::Str(name) = name else {
                    return Err(RuntimeError { message: "env takes a string".to_string(), span });
                };
                Ok(Value::Desc(Rc::new(Desc::Env(name))))
            }
            "write_err" => {
                let [content] = arity(args, name, span)?;
                let Value::Str(content) = content else {
                    return Err(RuntimeError {
                        message: "write_err takes a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::WriteErr(content))))
            }
            "make_dir" => {
                let [path] = arity(args, name, span)?;
                let Value::Str(path) = &path else {
                    return Err(RuntimeError {
                        message: "make_dir takes a path string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::MakeDir(path.clone()))))
            }
            "listen" => {
                let [port] = arity(args, name, span)?;
                let Value::Int(port) = &port else {
                    return Err(RuntimeError {
                        message: "listen takes a port number".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Listen(port.to_i64().unwrap_or(-1)))))
            }
            "accept" => {
                let [listener] = arity(args, name, span)?;
                let Value::Int(listener) = &listener else {
                    return Err(RuntimeError {
                        message: "accept takes a listener".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Accept(listener.to_i64().unwrap_or(-1)))))
            }
            "net_read" => {
                let [conn] = arity(args, name, span)?;
                let Value::Int(conn) = &conn else {
                    return Err(RuntimeError {
                        message: "net_read takes a connection".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Receive(conn.to_i64().unwrap_or(-1)))))
            }
            "net_write" => {
                let [conn, text] = arity(args, name, span)?;
                let (Value::Int(conn), Value::Str(text)) = (&conn, &text) else {
                    return Err(RuntimeError {
                        message: "net_write takes a connection and a string".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::Send(conn.to_i64().unwrap_or(-1), text.clone()))))
            }
            "net_close" => {
                let [handle] = arity(args, name, span)?;
                let Value::Int(handle) = &handle else {
                    return Err(RuntimeError {
                        message: "net_close takes a listener or a connection".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::CloseSocket(handle.to_i64().unwrap_or(-1)))))
            }
            "write_file" => {
                let [path, content] = arity(args, name, span)?;
                let (Value::Str(path), Value::Str(content)) = (&path, &content) else {
                    return Err(RuntimeError {
                        message: "write_file takes a path and content strings".to_string(),
                        span,
                    });
                };
                Ok(Value::Desc(Rc::new(Desc::WriteFile(path.clone(), content.clone()))))
            }
            "sleep" => {
                let [ms] = arity(args, name, span)?;
                match ms {
                    Value::Int(n) => {
                        let ms = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Sleep(ms))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!(
                            "sleep takes milliseconds (an int), got {}",
                            render(&other, false)
                        ),
                        span,
                    }),
                }
            }
            "random" => {
                let [n] = arity(args, name, span)?;
                match n {
                    Value::Int(n) => {
                        let bound = n.to_u64().unwrap_or(0);
                        Ok(Value::Desc(Rc::new(Desc::Random(bound))))
                    }
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!(
                            "random takes a bound (an int), got {}",
                            render(&other, false)
                        ),
                        span,
                    }),
                }
            }
            "print" => {
                let [text] = arity(args, name, span)?;
                match text {
                    Value::Str(s) => Ok(Value::Desc(Rc::new(Desc::Print(s, span)))),
                    // any other value renders through the same ambient
                    // to_string dispatch interpolation uses
                    other => {
                        if is_failure(&other) {
                            return Ok(other);
                        }
                        match self.render_interpolated(other)? {
                            Ok(rendered) => Ok(Value::Desc(Rc::new(Desc::Print(rendered, span)))),
                            Err(failure) => Ok(failure),
                        }
                    }
                }
            }
            "at" => {
                let [container, index] = arity(args, name, span)?;
                index_value(container, index, span)
            }
            "push" => {
                let [list, item] = arity(args, name, span)?;
                let Value::List(items) = &list else {
                    return Err(RuntimeError {
                        message: format!("push takes a list and a value{}", lazy_hint(&list)),
                        span,
                    });
                };
                let mut next = (**items).clone();
                next.push(item);
                Ok(Value::List(Rc::new(next)))
            }
            "put" => {
                let [map, key, value] = arity(args, name, span)?;
                let Value::Map(entries) = &map else {
                    return Err(RuntimeError {
                        message: "put takes a map, a key, and a value".to_string(),
                        span,
                    });
                };
                let key = map_key(key, span)?;
                let mut next = (**entries).clone();
                next.insert(key, value);
                Ok(Value::Map(Rc::new(next)))
            }
            "entries" => {
                let [map] = arity(args, name, span)?;
                let Value::Map(map_entries) = &map else {
                    return Err(RuntimeError { message: "entries takes a map".to_string(), span });
                };
                let list = map_entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => Value::Int(n.clone()),
                            MapKey::Str(s) => Value::Str(s.clone()),
                        };
                        Value::Record {
                            ty: Rc::from("entry"),
                            fields: Rc::new(RefCell::new(vec![key, value.clone()])),
                        }
                    })
                    .collect();
                Ok(Value::List(Rc::new(list)))
            }
            "bytes" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError { message: "bytes takes a string".to_string(), span });
                };
                let list = text.bytes().map(|b| Value::Int(BigInt::from(b))).collect();
                Ok(Value::List(Rc::new(list)))
            }
            "concat" => {
                let [a, b] = arity(args, name, span)?;
                let (Value::List(xs), Value::List(ys)) = (&a, &b) else {
                    return Err(RuntimeError {
                        message: "concat takes two lists".to_string(),
                        span,
                    });
                };
                let mut joined = (**xs).clone();
                joined.extend(ys.iter().cloned());
                Ok(Value::List(Rc::new(joined)))
            }
            "utf8" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = &list else {
                    return Err(RuntimeError {
                        message: "utf8 takes a list of byte values".to_string(),
                        span,
                    });
                };
                let mut raw = Vec::with_capacity(items.len());
                for item in items.iter() {
                    match item {
                        Value::Int(n) => match u8::try_from(n.clone()) {
                            Ok(b) => raw.push(b),
                            Err(_) => {
                                return Ok(err_value(
                                    Value::Str("utf8 takes byte values (0-255)".to_string()),
                                    origin_at(frame, span),
                                ))
                            }
                        },
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "utf8 takes a list of byte values".to_string(),
                                span,
                            })
                        }
                    }
                }
                match String::from_utf8(raw) {
                    Ok(text) => Ok(Value::Str(text)),
                    Err(_) => Ok(err_value(
                        Value::Str("invalid utf-8".to_string()),
                        origin_at(frame, span),
                    )),
                }
            }
            "split" => {
                let [text, sep] = arity(args, name, span)?;
                let (Value::Str(text), Value::Str(sep)) = (&text, &sep) else {
                    return Err(RuntimeError {
                        message: "split takes two strings".to_string(),
                        span,
                    });
                };
                if sep.is_empty() {
                    return Err(RuntimeError {
                        message: "split needs a separator".to_string(),
                        span,
                    });
                }
                let list =
                    text.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect::<Vec<_>>();
                Ok(Value::List(Rc::new(list)))
            }
            "chars" => {
                let [text] = arity(args, name, span)?;
                let Value::Str(text) = &text else {
                    return Err(RuntimeError { message: "chars takes a string".to_string(), span });
                };
                let list = text.chars().map(|c| Value::Str(c.to_string())).collect();
                Ok(Value::List(Rc::new(list)))
            }
            "char_code" => {
                let [c] = arity(args, name, span)?;
                let code = match &c {
                    Value::Str(s) if s.chars().count() == 1 => {
                        s.chars().next().expect("length checked") as u32
                    }
                    _ => {
                        return Err(RuntimeError {
                            message: "char_code takes a one-character string".to_string(),
                            span,
                        })
                    }
                };
                Ok(Value::Int(BigInt::from(code)))
            }
            "from_code" => {
                let [code] = arity(args, name, span)?;
                let Value::Int(n) = &code else {
                    return Err(RuntimeError {
                        message: "from_code takes an int".to_string(),
                        span,
                    });
                };
                let scalar = u32::try_from(n.clone()).ok().and_then(char::from_u32);
                match scalar {
                    Some(c) => Ok(Value::Str(c.to_string())),
                    None => Ok(err_value(
                        Value::Str("not a unicode scalar value".to_string()),
                        origin_at(frame, span),
                    )),
                }
            }
            "join" => {
                let [list, sep] = arity(args, name, span)?;
                let (Value::List(items), Value::Str(sep)) = (&list, &sep) else {
                    return Err(RuntimeError {
                        message: "join takes a list of strings and a separator".to_string(),
                        span,
                    });
                };
                let mut parts = Vec::new();
                for item in items.iter() {
                    match item {
                        Value::Str(s) => parts.push(s.clone()),
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "join takes a list of strings".to_string(),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::Str(parts.join(sep)))
            }
            "append" => {
                let [acc, x] = arity(args, name, span)?;
                for v in [&acc, &x] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let Value::List(items) = &acc else {
                    return Err(RuntimeError {
                        message: "append takes bytes and a string, bytes, or byte".to_string(),
                        span,
                    });
                };
                let mut out = (**items).clone();
                match &x {
                    Value::Str(s) => {
                        out.extend(s.bytes().map(|b| Value::Int(BigInt::from(b))));
                    }
                    Value::List(more) => out.extend(more.iter().cloned()),
                    Value::Int(_) => out.push(x.clone()),
                    _ => {
                        return Err(RuntimeError {
                            message: "append takes bytes and a string, bytes, or byte".to_string(),
                            span,
                        })
                    }
                }
                Ok(Value::List(Rc::new(out)))
            }
            "find2_below" => {
                let [cs, from, a, b, lim] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b, &lim] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (
                    Value::List(items),
                    Value::Int(from),
                    Value::Int(a),
                    Value::Int(b),
                    Value::Int(lim),
                ) = (&cs, &from, &a, &b, &lim)
                else {
                    return Err(RuntimeError {
                        message: "find2_below takes bytes".to_string(),
                        span,
                    });
                };
                let len = items.len();
                let start = usize::try_from(from.clone()).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, item) in items.iter().enumerate().skip(start - 1) {
                    if matches!(item, Value::Int(byte) if byte == a || byte == b || byte < lim) {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::Int(BigInt::from(at)))
            }
            "find2" => {
                let [cs, from, a, b] = arity(args, name, span)?;
                for v in [&cs, &from, &a, &b] {
                    if is_failure(v) {
                        return Ok(v.clone());
                    }
                }
                let (Value::List(items), Value::Int(from), Value::Int(a), Value::Int(b)) =
                    (&cs, &from, &a, &b)
                else {
                    return Err(RuntimeError { message: "find2 takes bytes".to_string(), span });
                };
                let len = items.len();
                let start = usize::try_from(from.clone()).unwrap_or(1).max(1);
                let mut at = len + 1;
                for (i, item) in items.iter().enumerate().skip(start - 1) {
                    if matches!(item, Value::Int(byte) if byte == a || byte == b) {
                        at = i + 1;
                        break;
                    }
                }
                Ok(Value::Int(BigInt::from(at)))
            }
            "slice" => {
                let [container, from, to] = arity(args, name, span)?;
                let (Value::Int(from), Value::Int(to)) = (&from, &to) else {
                    return Err(RuntimeError {
                        message: "slice takes 1-based inclusive positions".to_string(),
                        span,
                    });
                };
                let from = usize::try_from(from.clone()).unwrap_or(0);
                let to = usize::try_from(to.clone()).unwrap_or(0);
                match &container {
                    Value::List(items) => {
                        let sliced = slice_range(items.len(), from, to)
                            .map(|r| items[r].to_vec())
                            .unwrap_or_default();
                        Ok(Value::List(Rc::new(sliced)))
                    }
                    Value::Str(text) => {
                        let all: Vec<char> = text.chars().collect();
                        let sliced = slice_range(all.len(), from, to)
                            .map(|r| all[r].iter().collect::<String>())
                            .unwrap_or_default();
                        Ok(Value::Str(sliced))
                    }
                    _ => Err(RuntimeError {
                        message: "slice takes a list or string".to_string(),
                        span,
                    }),
                }
            }
            "bit_and" | "bit_or" | "bit_xor" => {
                let [a, b] = arity(args, name, span)?;
                let (x, y) = match (whole(a, name, span)?, whole(b, name, span)?) {
                    (Ok(x), Ok(y)) => (x, y),
                    (Err(bad), _) | (_, Err(bad)) => return Ok(bad),
                };
                let bits = match name {
                    "bit_and" => x & y,
                    "bit_or" => x | y,
                    _ => x ^ y,
                };
                Ok(Value::Int(bits.into()))
            }
            "bit_not" => {
                let [a] = arity(args, name, span)?;
                match whole(a, name, span)? {
                    Ok(x) => Ok(Value::Int((!x).into())),
                    Err(bad) => Ok(bad),
                }
            }
            "bit_shl" | "bit_shr" => {
                let [a, b] = arity(args, name, span)?;
                let (bits, by) = match (whole(a, name, span)?, whole(b, name, span)?) {
                    (Ok(x), Ok(y)) => (x, y),
                    (Err(bad), _) | (_, Err(bad)) => return Ok(bad),
                };
                if !(0..=63).contains(&by) {
                    return Err(RuntimeError {
                        message: format!("{} shifts by 0 to 63 places, got {by}", spelled(name)),
                        span,
                    });
                }
                let out = match name {
                    "bit_shl" => ((bits as u64) << by) as i64,
                    _ => bits >> by,
                };
                Ok(Value::Int(out.into()))
            }
            "sqrt" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Float(v) => Ok(Value::Float(v.sqrt())),
                    Value::Int(n) => Ok(Value::Float(int_f(&n).sqrt())),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("sqrt takes a number, got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "round" => {
                let [x] = arity(args, name, span)?;
                match x {
                    Value::Int(n) => Ok(Value::Int(n)),
                    Value::Float(v) => Ok(Value::Int(BigInt::from(v.round() as i64))),
                    other if is_failure(&other) => Ok(other),
                    other => Err(RuntimeError {
                        message: format!("round takes a number, got {}", render(&other, false)),
                        span,
                    }),
                }
            }
            "to_int" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::List(items) => match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str("bytes are not an integer".to_string()),
                                origin_at(frame, span),
                            ))
                        }
                    },
                    Value::Int(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            message: "to_int takes a string".to_string(),
                            span,
                        })
                    }
                };
                Ok(match text.parse::<BigInt>() {
                    Ok(n) => Value::Int(n),
                    Err(_) => err_value(
                        Value::Str(format!("\"{text}\" is not an integer")),
                        origin_at(frame, span),
                    ),
                })
            }
            "to_float" => {
                let [value] = arity(args, name, span)?;
                let text = match &value {
                    Value::Str(s) => s.clone(),
                    Value::List(items) => match bytes_to_str(items) {
                        Some(s) => s,
                        None => {
                            return Ok(err_value(
                                Value::Str("bytes are not a number".to_string()),
                                origin_at(frame, span),
                            ))
                        }
                    },
                    Value::Int(n) => {
                        let approx = n.to_string().parse::<f64>().unwrap_or(f64::INFINITY);
                        return Ok(Value::Float(approx));
                    }
                    Value::Float(_) => return Ok(value),
                    _ => {
                        return Err(RuntimeError {
                            message: "to_float takes a string or int".to_string(),
                            span,
                        })
                    }
                };
                Ok(match text.parse::<f64>() {
                    Ok(x) => Value::Float(x),
                    Err(_) => err_value(
                        Value::Str(format!("\"{text}\" is not a number")),
                        origin_at(frame, span),
                    ),
                })
            }
            "is_desc" => {
                let [v] = arity(args, name, span)?;
                Ok(match v {
                    Value::Desc(..) => Value::True,
                    _ => Value::False,
                })
            }
            "render_value" => {
                let [v] = arity(args, name, span)?;
                Ok(Value::Str(render(&v, false)))
            }
            "length" => {
                let [list] = arity(args, name, span)?;
                match list {
                    Value::List(items) => Ok(Value::Int(BigInt::from(items.len()))),
                    Value::Str(s) => Ok(Value::Int(BigInt::from(s.chars().count()))),
                    Value::Map(entries) => Ok(Value::Int(BigInt::from(entries.len()))),
                    other => Err(RuntimeError {
                        message: format!(
                            "length takes a list, string, or map, not {}{}",
                            render(&other, true),
                            lazy_hint(&other)
                        ),
                        span,
                    }),
                }
            }
            "map" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "map takes a list".to_string(), span });
                };
                let mapped = items
                    .iter()
                    .map(|item| self.call(f.clone(), vec![item.clone()], span, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(Rc::new(mapped)))
            }
            "filter" => {
                let [list, f] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "filter takes a list".to_string(), span });
                };
                let mut kept = Vec::new();
                for item in items.iter() {
                    match self.call(f.clone(), vec![item.clone()], span, frame)? {
                        Value::True => kept.push(item.clone()),
                        Value::False => {}
                        other => {
                            return Err(RuntimeError {
                                message: format!(
                                    "a filter predicate returns true or false, got {}",
                                    render(&other, false)
                                ),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::List(Rc::new(kept)))
            }
            "sort" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "sort takes a list".to_string(), span });
                };
                let mut sorted = (*items).clone();
                let mut failed = false;
                sorted.sort_by(|a, b| match compare(a, b) {
                    Some(ord) => ord,
                    None => {
                        failed = true;
                        std::cmp::Ordering::Equal
                    }
                });
                match failed {
                    true => Err(RuntimeError {
                        message: "sort requires comparable elements of one type".to_string(),
                        span,
                    }),
                    false => Ok(Value::List(Rc::new(sorted))),
                }
            }
            "sum" => {
                let [list] = arity(args, name, span)?;
                let Value::List(items) = list else {
                    return Err(RuntimeError { message: "sum takes a list".to_string(), span });
                };
                let mut total = BigInt::zero();
                for item in items.iter() {
                    match item {
                        Value::Int(n) => total += n,
                        bad if is_failure(bad) => return Ok(bad.clone()),
                        _ => {
                            return Err(RuntimeError {
                                message: "sum takes a list of int".to_string(),
                                span,
                            })
                        }
                    }
                }
                Ok(Value::Int(total))
            }
            _ => Err(RuntimeError { message: format!("unknown builtin `{name}`"), span }),
        }
    }

    fn builtin_if(&self, args: Vec<Value>, span: Span) -> EvalResult {
        let [cond, then_branch, else_branch] = arity(args, "if", span)?;
        let cond = self.force(cond)?;
        match cond {
            Value::True => self.force(then_branch),
            Value::False => self.force(else_branch),
            bad if is_failure(&bad) => Ok(bad),
            other => Err(RuntimeError {
                message: format!("an if condition is true or false, got {}", render(&other, false)),
                span,
            }),
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
                Value::Record { .. } | Value::NoneV | Value::Desc(_) | Value::Sub { .. },
                Some(overloads),
            ) => {
                let overloads = overloads.clone();
                let result = self.dispatch(
                    "render/to_string",
                    &overloads,
                    vec![value],
                    Span { line: 0, col: 0 },
                )?;
                if matches!(result, Value::ErrV(_)) {
                    return Ok(Err(result));
                }
                match result {
                    Value::Str(s) => s,
                    other => render(&other, false),
                }
            }
            _ => render(&value, false),
        };
        Ok(Ok(rendered))
    }

    fn force(&self, value: Value) -> EvalResult {
        match value {
            // A nullary closure's body can answer with a thunk, and scrutiny
            // wants a value rather than the next cell.
            Value::Closure(c) if c.params.is_empty() => {
                let answered = self.eval(&c.body, &c.env, &c.frame)?;
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
            if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
                out.push(n);
            }
            for child in crate::expr_children(expr) {
                names(child, out);
            }
        }
        let mut mentioned = Vec::new();
        names(expr, &mut mentioned);
        let knots = self.knots.borrow();
        mentioned
            .iter()
            .any(|n| knots.get(*n).is_some_and(|c| matches!(&*c.borrow(), ThunkState::Blackhole)))
    }

    fn knotted(&self, name: &str, constant: &FnDecl) -> EvalResult {
        if let Some(cell) = self.knots.borrow().get(name) {
            let forced = match &*cell.borrow() {
                ThunkState::Forced(v) => Some(v.clone()),
                _ => None,
            };
            return Ok(forced.unwrap_or_else(|| Value::Thunk(Rc::clone(cell))));
        }
        let cell = Rc::new(RefCell::new(ThunkState::Blackhole));
        self.knots.borrow_mut().insert(name.to_string(), Rc::clone(&cell));
        let value = self.eval_body_of(constant, None)?;
        *cell.borrow_mut() = ThunkState::Forced(value.clone());
        Ok(value)
    }

    fn force_thunk(&self, value: Value) -> EvalResult {
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
                    let v = self.eval(&expr, &env, &frame)?;
                    *cell.borrow_mut() = ThunkState::Forced(v.clone());
                    value = v;
                }
                ThunkState::Blackhole => {
                    return Err(RuntimeError {
                        message: "a lazy binding demands its own value".to_string(),
                        span: Span { line: 0, col: 0 },
                    })
                }
            }
        }
    }
}

/// A bitwise operand. Failures travel through as themselves, the way every
/// builtin lets them; anything else is the type complaint native prints, in
/// the same words. Native's ints are 64 bits wide by construction, so a value
/// too wide to be one is a case only the interpreter can reach.
fn whole(v: Value, name: &str, span: Span) -> Result<Result<i64, Value>, RuntimeError> {
    if is_failure(&v) {
        return Ok(Err(v));
    }
    let said = spelled(name);
    match &v {
        Value::Int(n) => match n.to_i64() {
            Some(w) => Ok(Ok(w)),
            None => Err(RuntimeError {
                message: format!("{said} takes whole numbers that fit 64 bits"),
                span,
            }),
        },
        other => Err(RuntimeError {
            message: format!("{said} takes whole numbers, got {}", render(other, false)),
            span,
        }),
    }
}

/// The name the reader wrote, from the builtin the wrapper forwards to.
fn spelled(name: &str) -> &str {
    name.strip_prefix("bit_").unwrap_or(name)
}

fn arity<const N: usize>(
    args: Vec<Value>,
    name: &str,
    span: Span,
) -> Result<[Value; N], RuntimeError> {
    match <[Value; N]>::try_from(args) {
        Ok(array) => Ok(array),
        Err(actual) => Err(RuntimeError {
            message: format!("`{name}` takes {N} argument(s), got {}", actual.len()),
            span,
        }),
    }
}

fn match_params(params: &[Pattern], args: &[Value]) -> Option<(Score, Bindings)> {
    let mut score = Vec::new();
    let mut binds = Vec::new();
    for (pattern, arg) in params.iter().zip(args) {
        // per-param: literals 200, annotated 100 minus subtype distance
        // (nearer declarations outrank ancestors), generics 10 — the old
        // three-rank ladder, widened so chain depth can order within a rank
        let base: u8 = match pattern.rank() {
            0 => 200,
            1 => 100,
            _ => 10,
        };
        let depth = match_one(pattern, arg, &mut binds)?;
        score.push(base.saturating_sub(depth));
    }
    Some((score, binds))
}

/// The as-pattern's name takes the value the shape matched — the same value
/// the caller passed, not one rebuilt from the parts.
fn bind_whole(whole: &Option<Box<(String, crate::diag::Span)>>, arg: &Value, binds: &mut Bindings) {
    if let Some(named) = whole {
        binds.push((named.0.clone(), arg.clone()));
    }
}

fn match_one(pattern: &Pattern, arg: &Value, binds: &mut Bindings) -> Option<u8> {
    match (pattern, arg) {
        (Pattern::IntLit(n, _), Value::Int(v)) if n == v => Some(0),
        (Pattern::StrLit(s, _), Value::Str(v)) if s == v => Some(0),
        (Pattern::Nullary(name, _), Value::True) if name == "true" => Some(0),
        (Pattern::Nullary(name, _), Value::False) if name == "false" => Some(0),
        (Pattern::Nullary(name, _), Value::NoneV) if name == "none" => Some(0),
        (Pattern::Wildcard(_), _) => match is_failure(arg) {
            true => None,
            false => Some(0),
        },
        (Pattern::Var(name, _), _) => match is_failure(arg) {
            true => None,
            false => {
                binds.push((name.clone(), arg.clone()));
                Some(0)
            }
        },
        (Pattern::Annotated { name, ty, .. }, _) => match type_match_depth(ty, arg) {
            Some(depth) => {
                binds.push((name.clone(), arg.clone()));
                Some(depth)
            }
            None => None,
        },
        (Pattern::Keyed { .. }, _) => None,
        (Pattern::Ctor { ty, fields, whole }, Value::ErrV(info)) if ty == "err" => {
            match fields.len() == 1 {
                true => {
                    let inner = match_one(&fields[0], &info.reason, binds)?;
                    bind_whole(whole, arg, binds);
                    // a bare reason binder demands err-ness but names
                    // nothing: it ranks below every named reason — leaf
                    // and typeset alike — and just above the plain
                    // generics
                    Some(match &fields[0] {
                        Pattern::Var(..) | Pattern::Wildcard(_) => 89,
                        _ => inner,
                    })
                }
                false => None,
            }
        }
        (Pattern::Ctor { ty, fields, whole }, Value::Record { ty: vty, fields: vfields })
            if ty.as_str() == &**vty && fields.len() == vfields.borrow().len() =>
        {
            for (fp, fv) in fields.iter().zip(vfields.borrow().iter()) {
                match_one(fp, fv, binds)?;
            }
            bind_whole(whole, arg, binds);
            Some(0)
        }
        // A pattern naming a type takes a subtype of it, the same way an
        // annotation does, and ranks the same way: how many wrappers come off
        // before the named type is the one in hand, zero when it names the
        // subtype itself. So an arm naming the child still beats one naming
        // the parent, and the fields bound are the base record's either way.
        (Pattern::Ctor { ty, fields, whole }, Value::Sub { .. }) => {
            let mut depth: u8 = 0;
            let mut cur = arg;
            let names_it = loop {
                match cur {
                    Value::Sub { ty: vty, inner } => match ty.as_str() == &**vty {
                        true => break true,
                        false => {
                            depth = depth.saturating_add(1);
                            cur = inner;
                        }
                    },
                    Value::Record { ty: vty, .. } => break ty.as_str() == &**vty,
                    _ => break false,
                }
            };
            if !names_it {
                return None;
            }
            let base = sub_base(arg.clone());
            match &base {
                Value::Record { fields: vfields, .. } if fields.len() == vfields.borrow().len() => {
                    for (fp, fv) in fields.iter().zip(vfields.borrow().iter()) {
                        match_one(fp, fv, binds)?;
                    }
                    bind_whole(whole, arg, binds);
                    Some(depth)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn index_value(container: Value, index: Value, span: Span) -> EvalResult {
    if is_failure(&container) {
        return Ok(container);
    }
    if is_failure(&index) {
        return Ok(index);
    }
    match (&container, &index) {
        (Value::List(items), Value::Int(i)) => {
            let idx = usize::try_from(i.clone()).ok();
            Ok(match idx.filter(|i| *i >= 1 && *i <= items.len()) {
                Some(i) => items[i - 1].clone(),
                None => Value::NoneV,
            })
        }
        (Value::Str(text), Value::Int(i)) => {
            let idx = usize::try_from(i.clone()).ok();
            Ok(match idx.and_then(|i| i.checked_sub(1)).and_then(|i| text.chars().nth(i)) {
                Some(c) => Value::Str(c.to_string()),
                None => Value::NoneV,
            })
        }
        (Value::Map(entries), Value::Int(_) | Value::Str(_)) => {
            let key = map_key(index, span)?;
            Ok(entries.get(&key).cloned().unwrap_or(Value::NoneV))
        }
        (base, _) => Err(RuntimeError {
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
            message: format!("{} is not usable as a map key", render(&other, true)),
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
    static TYPESETS: std::cell::RefCell<std::collections::HashMap<String, Vec<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
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
    if ty.ends_with("[]") {
        return matches!(arg, Value::List(_)).then_some(0);
    }
    if ty.contains('[') {
        return matches!(arg, Value::Map(_)).then_some(0);
    }
    let ok = match (ty, arg) {
        ("some", Value::NoneV) => false,
        ("some", _) => true,
        ("int", Value::Int(_)) => true,
        ("float64", Value::Float(_)) => true,
        ("string", Value::Str(_)) => true,
        ("bool", Value::True | Value::False) => true,
        ("true", Value::True) => true,
        ("false", Value::False) => true,
        ("none", Value::NoneV) => true,
        ("err", Value::ErrV(_)) => true,
        (name, Value::Record { ty, .. }) => name == &**ty,
        _ => false,
    };
    ok.then_some(0)
}

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    if let Value::Sub { inner, .. } = a {
        return compare(inner, b);
    }
    if let Value::Sub { inner, .. } = b {
        return compare(a, inner);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => Some(x.total_cmp(y)),
        (Value::Int(x), Value::Float(y)) => Some(cmp_int_float(x, *y)),
        (Value::Float(x), Value::Int(y)) => Some(cmp_int_float(y, *x).reverse()),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// A BigInt widened to f64 — the `x:float` cast at the value level.
fn int_f(n: &BigInt) -> f64 {
    n.to_f64().unwrap_or(f64::INFINITY)
}

/// A whole number against a fractional one, decided exactly. Widening the
/// integer first would round it onto the nearest value a float can hold and
/// report two different numbers as equal — 9007199254740993 is the smallest
/// integer where that happens. Comparing against the float's floor keeps the
/// integer whole, and the fraction breaks the tie.
fn cmp_int_float(x: &BigInt, y: f64) -> std::cmp::Ordering {
    let floor = y.floor();
    // Only a NaN or an infinity has no floor to compare against, and the
    // total order over the widened value is what ranks those.
    let Some(whole) = <BigInt as num_traits::FromPrimitive>::from_f64(floor) else {
        return int_f(x).total_cmp(&y);
    };
    match x.cmp(&whole) {
        std::cmp::Ordering::Equal if y > floor => std::cmp::Ordering::Less,
        settled => settled,
    }
}

fn div_float(a: f64, b: f64, frame: &Frame, span: Span) -> EvalResult {
    match b == 0.0 {
        true => Ok(err_value(Value::Str("division by zero".to_string()), origin_at(frame, span))),
        false => Ok(Value::Float(a / b)),
    }
}

/// `a & b`: join two descriptions to run with no order between them.
/// Failures accumulate — two errs merge into one whose reason lists both
/// (origin-less: the merge has no single birthplace) — and a lone failure
/// propagates as itself. Anything that isn't a description or a failure
/// cannot be joined.
pub fn join_values(left: Value, right: Value, span: Span) -> EvalResult {
    match (is_failure(&left), is_failure(&right)) {
        (true, true) => Ok(accumulate_failures(left, right)),
        (true, false) => Ok(left),
        (false, true) => Ok(right),
        (false, false) => match (&left, &right) {
            (Value::Desc(a), Value::Desc(b)) => {
                Ok(Value::Desc(Rc::new(Desc::Join(a.clone(), b.clone()))))
            }
            _ => Err(RuntimeError { message: "a group joins descriptions".to_string(), span }),
        },
    }
}

/// Merge two failures: err + err becomes one err whose reason is the list of
/// both reasons; a `none` adds nothing to an err; two nones stay none.
/// Every failure among a call's arguments, merged. Two failures in one call
/// are two facts, and the one that lost a race to be first is not less true —
/// the parallel group has always merged them, and Clay ruled (2026-08-05) that
/// an operation does too.
fn merged_failures(args: &[Value]) -> Value {
    args.iter()
        .filter(|a| is_failure(a))
        .cloned()
        .reduce(accumulate_failures)
        .expect("a caller checked that one argument fails")
}

fn accumulate_failures(left: Value, right: Value) -> Value {
    match (&left, &right) {
        (Value::ErrV(a), Value::ErrV(b)) => {
            let mut reasons = reasons_of(a);
            reasons.extend(reasons_of(b));
            Value::ErrV(Rc::new(ErrInfo {
                reason: Value::List(Rc::new(reasons)),
                origin: None,
                hops: Vec::new(),
                cause: None,
                merged: true,
            }))
        }
        (Value::ErrV(_), _) => left,
        (_, Value::ErrV(_)) => right,
        _ => left,
    }
}

/// What an err contributes to a merge: the reasons it already carries if it
/// was itself merged, and otherwise itself, whatever shape its reason has.
fn reasons_of(info: &Rc<ErrInfo>) -> Vec<Value> {
    match (info.merged, &info.reason) {
        (true, Value::List(items)) => items.as_ref().clone(),
        _ => vec![info.reason.clone()],
    }
}

/// The name the runtime prints for a bitwise operator, which is the name of
/// the std/bits function it shares its meaning with.
fn spelled_op(op: &str) -> &str {
    match op {
        "&" => "and",
        "|" => "or",
        _ => "xor",
    }
}

/// The three bitwise operators share a shape: both sides must fit the machine
/// word the compiled engines use, and the answer is an int.
fn bitwise(op: &str, a: &BigInt, b: &BigInt, span: Span) -> EvalResult {
    let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) else {
        return Err(RuntimeError {
            message: format!("`{op}` takes whole numbers that fit 64 bits"),
            span,
        });
    };
    let bits = match op {
        "&" => x & y,
        "|" => x | y,
        _ => x ^ y,
    };
    Ok(Value::Int(bits.into()))
}

pub fn eval_binop(op: &str, left: Value, right: Value, span: Span, frame: &Frame) -> EvalResult {
    // Two failures in one operation merge, exactly as two failures in a
    // parallel group do (Clay, 2026-08-05): neither side caused the other, so
    // neither deserves top billing, and the one that loses would otherwise be
    // discarded without a word. One failure propagates as itself.
    match (is_failure(&left), is_failure(&right)) {
        (true, true) => return Ok(accumulate_failures(left, right)),
        (true, false) => return Ok(left),
        (false, true) => return Ok(right),
        (false, false) => {}
    }
    if op == "==" || op == "!=" {
        // Asking whether two functions are the same function is asking which
        // one you were handed, and that is what dispatch answers. The same
        // goes for an effect: if the answer would change what you do next,
        // an arm names it. So the question is refused rather than answered
        // falsely — the way `<` already refuses it on the same values.
        if opaque_to_equality(&left) || opaque_to_equality(&right) {
            return Err(RuntimeError {
                message: "equality is not defined on a function or an effect — \
                          write an arm for the case you mean"
                    .to_string(),
                span,
            });
        }
        let equal = values_equal(&left, &right);
        return Ok(bool_value(match op {
            "==" => equal,
            _ => !equal,
        }));
    }
    match (op, &left, &right) {
        ("+", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        ("-", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        ("*", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        ("/", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => {
                Ok(err_value(Value::Str("division by zero".to_string()), origin_at(frame, span)))
            }
            false => Ok(Value::Int(a / b)),
        },
        ("%", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => Ok(err_value(Value::Str("modulo by zero".to_string()), origin_at(frame, span))),
            false => Ok(Value::Int(a % b)),
        },
        // Bitwise, over whole numbers only. Native's ints are 64 bits wide by
        // construction, so a value too wide to be one is a case only the
        // interpreter can reach, and it says so rather than answering.
        ("&", Value::Int(a), Value::Int(b)) => bitwise("&", a, b, span),
        ("|", Value::Int(a), Value::Int(b)) => bitwise("|", a, b, span),
        ("^", Value::Int(a), Value::Int(b)) => bitwise("^", a, b, span),
        // Anything else meeting a bitwise operator is the type complaint the
        // compiled engines print, in their words: the operator and the named
        // function in std/bits are one operation, so they answer alike.
        ("&" | "|" | "^", other, _) if !matches!(other, Value::Int(_)) => Err(RuntimeError {
            message: format!(
                "{} takes whole numbers, got {}",
                spelled_op(op),
                render(other, false)
            ),
            span,
        }),
        ("&" | "|" | "^", _, other) => Err(RuntimeError {
            message: format!(
                "{} takes whole numbers, got {}",
                spelled_op(op),
                render(other, false)
            ),
            span,
        }),
        ("+", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        ("-", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        ("*", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        ("/", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => {
                Ok(err_value(Value::Str("division by zero".to_string()), origin_at(frame, span)))
            }
            false => Ok(Value::Float(a / b)),
        },
        ("%", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => Ok(err_value(Value::Str("modulo by zero".to_string()), origin_at(frame, span))),
            false => Ok(Value::Float(a % b)),
        },
        // int meets float: the int widens (as if cast `x:float`), result float
        ("+", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) + b)),
        ("+", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + int_f(b))),
        ("-", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) - b)),
        ("-", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - int_f(b))),
        ("*", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) * b)),
        ("*", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * int_f(b))),
        ("/", Value::Int(a), Value::Float(b)) => div_float(int_f(a), *b, frame, span),
        ("/", Value::Float(a), Value::Int(b)) => div_float(*a, int_f(b), frame, span),
        ("<" | "<=" | ">" | ">=", _, _) => match compare(&left, &right) {
            Some(ord) => Ok(bool_value(match op {
                "<" => ord.is_lt(),
                "<=" => ord.is_le(),
                ">" => ord.is_gt(),
                _ => ord.is_ge(),
            })),
            None => Err(RuntimeError {
                message: "comparison requires two values of one comparable type".to_string(),
                span,
            }),
        },
        _ => Err(RuntimeError { message: format!("`{op}` is not defined for these values"), span }),
    }
}

fn bool_value(b: bool) -> Value {
    match b {
        true => Value::True,
        false => Value::False,
    }
}

/// Values equality declines to answer about. A function and an effect are
/// both things you dispatch on rather than compare: asking which one you hold
/// is a question an arm answers, and answering it with a comparison is the
/// shape kanso refuses everywhere else. A subtype is transparent, so the
/// question is really about what it wraps.
fn opaque_to_equality(v: &Value) -> bool {
    match v {
        Value::FnRef(_) | Value::Closure(_) | Value::Desc(_) => true,
        Value::Sub { inner, .. } => opaque_to_equality(inner),
        _ => false,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    values_equal_seen(a, b, &mut std::collections::HashSet::new())
}

/// Two cyclic graphs would compare forever, so a pair of cells is assumed
/// equal once seen and any contradiction still returns false.
fn values_equal_seen(
    a: &Value,
    b: &Value,
    seen: &mut std::collections::HashSet<(usize, usize)>,
) -> bool {
    if let Value::Sub { inner, .. } = a {
        return values_equal_seen(inner, b, seen);
    }
    if let Value::Sub { inner, .. } = b {
        return values_equal_seen(a, inner, seen);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.total_cmp(y).is_eq(),
        // One numeric domain, which `<=` and `>=` already assert: both answer
        // true for 1 and 1.0, and `<` answers false either way round because
        // neither is strictly less. Equality was the one operator dissenting.
        // A float cannot be a map key, so nothing needs them told apart.
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => {
            cmp_int_float(x, *y).is_eq()
        }
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && values_equal_seen(va, vb, seen))
        }
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::True, Value::True) | (Value::False, Value::False) => true,
        (Value::NoneV, Value::NoneV) => true,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_equal_seen(a, b, seen))
        }
        (Value::Record { ty: tx, fields: fx }, Value::Record { ty: ty_, fields: fy }) => {
            if tx != ty_ {
                return false;
            }
            if Rc::ptr_eq(fx, fy) {
                return true;
            }
            let key = (Rc::as_ptr(fx) as *const () as usize, Rc::as_ptr(fy) as *const () as usize);
            if !seen.insert(key) {
                return true;
            }
            fx.borrow().len() == fy.borrow().len()
                && fx
                    .borrow()
                    .iter()
                    .zip(fy.borrow().iter())
                    .all(|(a, b)| values_equal_seen(a, b, seen))
        }
        _ => false,
    }
}

fn render_float(x: f64) -> String {
    if x == x.floor() && x.abs() < 1e15 && x.is_finite() {
        return format!("{x:.1}");
    }
    // rust's LowerExp gives the shortest round-trip digits; the format
    // layer mirrors the native renderer's %g rules exactly: exponent form
    // at X < -4 or X >= max(15, digit count)
    let neg = x < 0.0;
    let exp_form = format!("{:e}", x.abs());
    let (mant, exp) = exp_form.split_once('e').expect("LowerExp has an e");
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let k = digits.len() as i32;
    let x10: i32 = exp.parse().expect("exponent parses");
    let p = k.max(15);
    let sign = if neg { "-" } else { "" };
    if x10 < -4 || x10 >= p {
        let tail = if k > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        let esign = if x10 < 0 { '-' } else { '+' };
        return format!("{sign}{}{tail}e{esign}{:02}", &digits[..1], x10.abs());
    }
    if x10 >= 0 {
        let ip = (x10 + 1) as usize;
        let whole: String =
            (0..ip).map(|i| digits.as_bytes().get(i).map(|b| *b as char).unwrap_or('0')).collect();
        let frac = if (k as usize) > ip { format!(".{}", &digits[ip..]) } else { String::new() };
        return format!("{sign}{whole}{frac}");
    }
    let zeros = "0".repeat((-x10 - 1) as usize);
    format!("{sign}0.{zeros}{digits}")
}

/// Transparency: a subtype value IS its base wherever machinery (builtins,
/// operators, comparison) consumes it; only dispatch sees the wrapper.
pub fn sub_base(value: Value) -> Value {
    match value {
        Value::Sub { inner, .. } => sub_base((*inner).clone()),
        other => other,
    }
}

pub fn render(value: &Value, quote_strings: bool) -> String {
    render_seen(value, quote_strings, &mut std::collections::HashSet::new())
}

/// A cycle would render forever, so re-entering a cell on the current path
/// prints `<cycle>` instead.
fn render_seen(
    value: &Value,
    quote_strings: bool,
    seen: &mut std::collections::HashSet<usize>,
) -> String {
    match value {
        // a subtype renders as its base until a user arm claims it
        Value::Sub { inner, .. } => render_seen(inner, quote_strings, seen),
        Value::Thunk(cell) => match &*cell.borrow() {
            ThunkState::Forced(v) => render_seen(v, quote_strings, seen),
            // A pending thunk reaching render is a missed force point; make
            // it loud so the differential corpus catches it.
            _ => "<thunk>".to_string(),
        },
        Value::Int(n) => n.to_string(),
        Value::Float(x) => render_float(*x),
        Value::Map(entries) => match entries.is_empty() {
            true => "{}".to_string(),
            false => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => n.to_string(),
                            MapKey::Str(s) => format!("\"{s}\""),
                        };
                        format!("{key}:{}", render_seen(value, true, seen))
                    })
                    .collect();
                format!("{{ {} }}", inner.join(" "))
            }
        },
        Value::Str(s) => match quote_strings {
            true => format!("\"{s}\""),
            false => s.clone(),
        },
        Value::True => "true".to_string(),
        Value::False => "false".to_string(),
        Value::NoneV => "<none>".to_string(),
        Value::ErrV(info) => format!("err {}", render_seen(&info.reason, true, seen)),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(|i| render_seen(i, true, seen)).collect();
            format!("[{}]", inner.join(" "))
        }
        Value::Record { ty, fields } => match fields.borrow().is_empty() {
            true => ty.to_string(),
            false => {
                let ptr = Rc::as_ptr(fields) as usize;
                if !seen.insert(ptr) {
                    return "<cycle>".to_string();
                }
                let inner: Vec<String> =
                    fields.borrow().iter().map(|f| render_seen(f, true, seen)).collect();
                seen.remove(&ptr);
                format!("{} {}", ty, inner.join(" "))
            }
        },
        // every engine renders a function the same way and none of them names
        // it: the name is the compiler's, and for a field getter it is one no
        // program could have written
        Value::FnRef(_) | Value::Closure(_) | Value::TableFn(_) | Value::Partial(..) => {
            "<fn>".to_string()
        }
        Value::Desc(_) => "<io>".to_string(),
    }
}

impl<'a> Interp<'a> {
    pub fn execute(&self, desc: &Desc, executor: &mut dyn Executor) -> EvalResult {
        match desc {
            Desc::Print(text, _) => {
                executor.print(text);
                Ok(Value::NoneV)
            }
            Desc::Seq(..) | Desc::Bind(..) => self.execute_chain(Rc::new(desc.clone()), executor),
            Desc::Join(_, _) => self.schedule(desc, executor),
            Desc::Sleep(ms) => {
                executor.sleep(*ms);
                Ok(Value::NoneV)
            }
            Desc::Random(n) => Ok(Value::Int(executor.random(*n).into())),
            Desc::Nil => Ok(Value::NoneV),
            Desc::Args => {
                let list = executor.args().into_iter().map(Value::Str).collect();
                Ok(Value::List(Rc::new(list)))
            }
            Desc::Stdin => Ok(match executor.stdin() {
                Ok(text) => Value::Str(text),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::ReadFile(path) => Ok(match executor.read_file(path) {
                Ok(text) => Value::Str(text),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            // three answers in a list, which the std wrapper turns into a
            // record: a builtin cannot name a type declared in kanso.
            Desc::Run(cmd, argv) => Ok(match executor.run(cmd, argv) {
                Ok(done) => ran_value(done),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            // Only reached outside a parallel group; step() starts and awaits.
            Desc::Await(handle) => Ok(match executor.finished(*handle) {
                Ok(Some(done)) => ran_value(done),
                Ok(None) => err_value(Value::Str("still running".to_string()), None),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Write(text) => {
                executor.write(text);
                Ok(Value::NoneV)
            }
            Desc::WriteErr(text) => {
                executor.write_err(text);
                Ok(Value::NoneV)
            }
            Desc::Env(name) => Ok(match executor.env(name) {
                Some(value) => Value::Str(value),
                None => Value::NoneV,
            }),
            Desc::Now => Ok(Value::Int(BigInt::from(executor.now()))),
            Desc::Exists(path) => Ok(match executor.exists(path) {
                true => Value::True,
                false => Value::False,
            }),
            Desc::IsDir(path) => Ok(match executor.is_dir(path) {
                true => Value::True,
                false => Value::False,
            }),
            Desc::ListDir(path) => Ok(match executor.list_dir(path) {
                Ok(names) => Value::List(Rc::new(names.into_iter().map(Value::Str).collect())),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::MakeDir(path) => Ok(match executor.make_dir(path) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::WriteFile(path, content) => Ok(match executor.write_file(path, content) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Start(cmd, argv) => Ok(match executor.start(cmd, argv) {
                Ok(handle) => Value::Int(handle.into()),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Kill(handle) => Ok(match executor.kill(*handle) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Listen(port) => Ok(match executor.listen(*port) {
                Ok(handle) => Value::Int(handle.into()),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Accept(listener) => Ok(match executor.accept(*listener) {
                Ok(Some(handle)) => Value::Int(handle.into()),
                // Reached only outside a parallel group, where no other fiber
                // could ever connect; step() yields instead of arriving here.
                Ok(None) => err_value(Value::Str("nothing connected".to_string()), None),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Receive(conn) => Ok(match executor.receive(*conn) {
                Ok(text) => Value::Str(text),
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::Send(conn, text) => Ok(match executor.send(*conn, text) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
            Desc::CloseSocket(handle) => Ok(match executor.close_socket(*handle) {
                Ok(()) => Value::NoneV,
                Err(reason) => err_value(Value::Str(reason), None),
            }),
        }
    }

    /// Sequencing runs as a loop. Each link answers the description to run
    /// next, which becomes the next iteration rather than a nested call, so a
    /// program that sequences a million effects holds one frame. Only the
    /// continuation is flattened: a link's own left side is a step to run
    /// before the chain advances, and nests as deep as it is written.
    fn execute_chain(&self, start: Rc<Desc>, executor: &mut dyn Executor) -> EvalResult {
        let origin = Span { line: 0, col: 0 };
        let mut current = start;
        loop {
            let next = match &*current {
                Desc::Seq(a, b) => {
                    let left = self.execute(a, executor)?;
                    if is_failure(&left) && matches!(left, Value::ErrV(_)) {
                        return Ok(left);
                    }
                    b.clone()
                }
                Desc::Bind(inner, callee) => {
                    let yielded = self.execute(inner, executor)?;
                    match self.call(callee.clone(), vec![yielded], origin, &None)? {
                        Value::Desc(d) => d,
                        other => return Ok(other),
                    }
                }
                leaf => return self.execute(leaf, executor),
            };
            current = next;
        }
    }

    /// Run a parallel group as cooperative green threads. Each member is a
    /// fiber; a fiber runs until it finishes or blocks on `sleep`, at which
    /// point the scheduler advances to the next fiber. The policy is fully
    /// deterministic — always resume the fiber with the earliest wake time,
    /// ties broken by spawn order — so a concurrent program's output is
    /// byte-identical across engines and runs, which the goldens require.
    /// Members' *values* are discarded (a group yields none); their failures
    /// accumulate, exactly as the sequential Join did.
    fn schedule(&self, join: &Desc, executor: &mut dyn Executor) -> EvalResult {
        let mut fibers = Vec::new();
        flatten_join(join, &mut fibers);
        // (wake_time, spawn_index, remaining work)
        let mut ready: Vec<(u64, usize, Rc<Desc>)> =
            fibers.into_iter().enumerate().map(|(i, d)| (0u64, i, d)).collect();
        let mut now = 0u64;
        // wall-credit: real time spent computing counts against a pending
        // wait, so compute overlaps sleeps in wall-clock. the transcript
        // stays purely logical — only the physical wait shrinks.
        #[cfg(not(target_arch = "wasm32"))]
        let wall_start = std::time::Instant::now();
        let mut failures: Vec<Value> = Vec::new();
        while !ready.is_empty() {
            let pick =
                (0..ready.len()).min_by_key(|&i| (ready[i].0, ready[i].1)).expect("non-empty");
            let (wake, idx, desc) = ready.remove(pick);
            if wake > now {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let elapsed = wall_start.elapsed().as_millis() as u64;
                    if wake > elapsed {
                        executor.sleep(wake - elapsed);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                executor.sleep(wake - now);
                now = wake;
            }
            match self.step(&desc, executor)? {
                Step::Done(value) => {
                    if is_failure(&value) {
                        failures.push(value);
                    }
                }
                Step::Blocked(ms, cont) => ready.push((now + ms, idx, cont)),
            }
        }
        Ok(failures.into_iter().reduce(accumulate_failures).unwrap_or(Value::NoneV))
    }

    /// Advance one fiber until it finishes (`Done`) or hits a `sleep`
    /// (`Blocked`, carrying the rest of its work). Blocking threads back up
    /// through `Seq` and `Bind`, so the continuation is always the remaining
    /// description — no saved stack, no coroutine.
    fn step(&self, desc: &Rc<Desc>, executor: &mut dyn Executor) -> Result<Step, RuntimeError> {
        let origin = Span { line: 0, col: 0 };
        match &**desc {
            Desc::Sleep(ms) => Ok(Step::Blocked(*ms, Rc::new(Desc::Nil))),
            // A fiber waiting for a connection is a fiber with nothing to do:
            // it goes back in the queue and the group's other statements run,
            // which is how anything ever connects to it.
            Desc::Accept(listener) => match executor.accept(*listener) {
                Ok(Some(handle)) => Ok(Step::Done(Value::Int(handle.into()))),
                Ok(None) => Ok(Step::Blocked(1, desc.clone())),
                Err(reason) => Ok(Step::Done(err_value(Value::Str(reason), None))),
            },
            // Same shape for a process: start it, then wait by yielding, so
            // the other statements of the group run while it does.
            Desc::Run(cmd, argv) => match executor.start(cmd, argv) {
                Ok(handle) => Ok(Step::Blocked(1, Rc::new(Desc::Await(handle)))),
                Err(reason) => Ok(Step::Done(err_value(Value::Str(reason), None))),
            },
            Desc::Await(handle) => match executor.finished(*handle) {
                Ok(Some(done)) => Ok(Step::Done(ran_value(done))),
                Ok(None) => Ok(Step::Blocked(1, desc.clone())),
                Err(reason) => Ok(Step::Done(err_value(Value::Str(reason), None))),
            },
            Desc::Seq(a, b) => match self.step(a, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Seq(cont, b.clone()))))
                }
                Step::Done(left) if matches!(left, Value::ErrV(_)) => Ok(Step::Done(left)),
                Step::Done(_) => self.step(b, executor),
            },
            Desc::Bind(inner, callee) => match self.step(inner, executor)? {
                Step::Blocked(ms, cont) => {
                    Ok(Step::Blocked(ms, Rc::new(Desc::Bind(cont, callee.clone()))))
                }
                Step::Done(yielded) => {
                    let next = self.call(callee.clone(), vec![yielded], origin, &None)?;
                    match next {
                        Value::Desc(d) => self.step(&d, executor),
                        other => Ok(Step::Done(other)),
                    }
                }
            },
            // leaf effects and nested joins run to completion synchronously
            _ => Ok(Step::Done(self.execute(desc, executor)?)),
        }
    }
}

/// Collect the members of a (possibly nested) parallel group left-to-right —
/// the spawn order the scheduler breaks ties by.
fn flatten_join(desc: &Desc, out: &mut Vec<Rc<Desc>>) {
    match desc {
        Desc::Join(a, b) => {
            flatten_join(a, out);
            flatten_join(b, out);
        }
        other => out.push(Rc::new(other.clone())),
    }
}

pub fn render_plan(desc: &Desc, out: &mut String) {
    match desc {
        Desc::Print(text, span) => {
            out.push_str(&format!("  print {text:?}    # from line {}\n", span.line));
        }
        Desc::Seq(a, b) => {
            render_plan(a, out);
            render_plan(b, out);
        }
        Desc::Join(a, b) => {
            out.push_str("  join {\n");
            render_plan(a, out);
            render_plan(b, out);
            out.push_str("  } # unordered; both run\n");
        }
        Desc::Args => out.push_str("  args\n"),
        Desc::Stdin => out.push_str("  stdin\n"),
        Desc::ReadFile(path) => out.push_str(&format!("  read_file {path:?}\n")),
        Desc::Run(cmd, argv) => out.push_str(&format!("  run {cmd:?} {argv:?}\n")),
        Desc::Start(cmd, argv) => out.push_str(&format!("  start {cmd:?} {argv:?}\n")),
        Desc::Kill(handle) => out.push_str(&format!("  kill {handle}\n")),
        Desc::Await(handle) => out.push_str(&format!("  await {handle}\n")),
        Desc::Write(text) => out.push_str(&format!("  write {text:?}\n")),
        Desc::WriteErr(text) => out.push_str(&format!("  write_err {text:?}\n")),
        Desc::Env(name) => out.push_str(&format!("  env {name:?}\n")),
        Desc::Exists(path) => out.push_str(&format!("  exists {path:?}\n")),
        Desc::IsDir(path) => out.push_str(&format!("  is_dir {path:?}\n")),
        Desc::Now => out.push_str("  now\n"),
        Desc::ListDir(path) => out.push_str(&format!("  list_dir {path:?}\n")),
        Desc::MakeDir(path) => out.push_str(&format!("  make_dir {path:?}\n")),
        Desc::WriteFile(path, _) => out.push_str(&format!("  write_file {path:?}\n")),
        Desc::Listen(port) => out.push_str(&format!("  listen {port}\n")),
        Desc::Accept(listener) => out.push_str(&format!("  accept {listener}\n")),
        Desc::Receive(conn) => out.push_str(&format!("  receive {conn}\n")),
        Desc::Send(conn, _) => out.push_str(&format!("  send {conn}\n")),
        Desc::CloseSocket(handle) => out.push_str(&format!("  close_socket {handle}\n")),
        Desc::Bind(inner, _) => {
            render_plan(inner, out);
            out.push_str("  . <continuation>\n");
        }
        Desc::Sleep(ms) => out.push_str(&format!("  sleep {ms}\n")),
        Desc::Random(n) => out.push_str(&format!("  random {n}\n")),
        Desc::Nil => {}
    }
}

/// A lazy sequence has no length to count — `naturals` would never finish —
/// so refusing it is right. But the reader who wrote `length (map …)` is one
/// call from what they wanted, and the message may as well say which.
fn lazy_hint(v: &Value) -> &'static str {
    match v {
        Value::Record { ty, .. } if ty.starts_with("list/") || ty.contains("/list/") => {
            " — a lazy sequence becomes a list with list/to_list"
        }
        _ => "",
    }
}

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

    #[test]
    fn scripted_executor_records_the_transcript() {
        let value = run_main("main = print \"a\" >> print \"b\"\n");

        let Value::Desc(desc) = value else { panic!("main yields a description") };
        let lexed = crate::lexer::lex("main = print \"a\" >> print \"b\"\n").expect("lexes");
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
        let source = "fn double x\n  x * 2\n\nmain = double (1 / 0)\n";

        assert!(matches!(run_main(source), Value::ErrV(_)));
    }

    #[test]
    fn errs_carry_their_origin_and_dispatcher_hops() {
        let source = "fn grade outcome\n  \"grade {outcome}\"\n\nmain = grade (1 / 0)\n";
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
