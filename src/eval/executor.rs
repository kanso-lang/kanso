// The executor: the trait the interpreter performs io through, the real
// one that talks to the operating system and the scripted one the tests
// drive. eval.rs reads this file with include!, so the items stay in the
// eval module and the compiler builds to the same instructions it did when
// they sat in eval.rs (kanso#1867). cargo fmt does not reach an included
// file, so CI's format job checks the files under src/'s subdirectories
// itself.

pub trait Executor {
    /// How this engine reaches a cell. A lazy binding holds its value as a
    /// cell, and the engines keep their cells in different places: the oracle in a reference
    /// counted state the interpreter forces, the browser in a slot table. An
    /// engine whose cells the interpreter already forces needs nothing here.
    fn demand(&mut self, value: Value) -> Value {
        value
    }
    fn print(&mut self, text: &str);
    fn args(&mut self) -> Vec<String>;
    fn stdin(&mut self) -> Result<String, String>;
    /// The next line of standard input without its `\n` or `\r\n`, or `None`
    /// once the input has ended. A last line with no ending is still a line.
    /// Output written before the read is on its way before the read waits,
    /// so a prompt reaches a person before the program waits on them.
    fn read_line(&mut self) -> Result<Option<String>, String>;
    /// Three answers, because absence is not a failure. `Ok(Some(text))` read
    /// it; `Ok(None)` is the file not being there, which the gavel of
    /// 2026-09-03 rules is an anticipated outcome and therefore data; `Err` is
    /// what the program did not plan for — a permission, a device, bytes that
    /// are not text.
    fn read_file(&mut self, path: &str) -> Result<Option<String>, String>;
    /// The same three answers with no test of what the bytes are (ruled
    /// 2026-08-29: read_file is text, read_bytes is bytes, per precedent).
    fn read_bytes(&mut self, path: &str) -> Result<Option<Vec<u8>>, String>;
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
    /// When a file last changed, in milliseconds since the epoch: the unit
    /// `now` answers in. `Ok(None)` is the file not being there, which is
    /// data the way it is for `read_file`; `Err` is any other reason the
    /// question could not be answered.
    fn modified(&mut self, path: &str) -> Result<Option<i64>, String>;
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
    /// Bytes written back as they were read. Defaults to refusing the way the
    /// engine refuses text, since an engine with no files has none for bytes.
    fn write_bytes(&mut self, path: &str, raw: &[u8]) -> Result<(), String> {
        let _ = raw;
        self.write_file(path, "")
    }
    /// A socket listening on a port, answering the handle a later accept
    /// names. Every engine that has a filesystem has these; the browser has
    /// neither, and says so.
    fn listen(&mut self, _port: i64) -> Result<i64, String> {
        Err("this engine has no sockets".to_string())
    }
    /// The port a listener was actually given. Binding port 0 asks the
    /// operating system for a free one, and this reads back its answer, so a
    /// caller never has to drop the listener and re-bind the number.
    fn socket_port(&mut self, _listener: i64) -> Result<i64, String> {
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
    fn send_bytes(&mut self, _conn: i64, _raw: &[u8]) -> Result<(), String> {
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
    running: Map<i64, std::process::Child>,
}

#[derive(Default)]
struct Sockets {
    next: i64,
    listeners: Map<i64, std::net::TcpListener>,
    conns: Map<i64, std::net::TcpStream>,
}

impl Sockets {
    fn hand(&mut self) -> i64 {
        self.next += 1;
        self.next
    }
}

/// Write to stdout. A reader that has gone away, as `head` leaves the pipe,
/// ends the interpreter by SIGPIPE, the way it ends a built binary. The driver
/// puts SIGPIPE's default back at start-up, and on Linux the failed write is
/// enough. On macOS the interpreter's thread met the write as an error
/// instead, and `print!` panicked on it, so the error is turned into the
/// signal here.
fn out(text: &str, end: &[u8]) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    match stdout.write_all(text.as_bytes()).and_then(|()| stdout.write_all(end)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => reader_gone(),
        Err(e) => panic!("failed printing to stdout: {e}"),
    }
}

#[cfg(unix)]
fn reader_gone() -> ! {
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
        fn raise(sig: i32) -> i32;
    }
    const SIGPIPE: i32 = 13;
    unsafe {
        signal(SIGPIPE, 0);
        raise(SIGPIPE);
    }
    std::process::exit(141)
}

#[cfg(not(unix))]
fn reader_gone() -> ! {
    std::process::exit(1)
}

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        out(text, b"\n");
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
                return Err("that is not a running process".to_string());
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
                return Err("that is not a running process".to_string());
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

    fn socket_port(&mut self, listener: i64) -> Result<i64, String> {
        SOCKETS.with(|s| {
            let s = s.borrow();
            let found = s
                .listeners
                .get(&listener)
                .ok_or_else(|| "that is not an open socket".to_string())?;
            let addr = found
                .local_addr()
                .map_err(|e| format!("cannot read the port of that listener: {e}"))?;
            Ok(i64::from(addr.port()))
        })
    }

    fn accept(&mut self, listener: i64) -> Result<Option<i64>, String> {
        let socket = SOCKETS.with(|s| {
            let s = s.borrow();
            let found = s.listeners.get(&listener);
            found.map(|l| l.try_clone()).transpose().map_err(|e| e.to_string())
        })?;
        let Some(socket) = socket else {
            return Err("that is not a listener".to_string());
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
            return Err("that is not a connection".to_string());
        };
        let mut buffer = [0u8; 65536];
        let read = stream.read(&mut buffer).map_err(|e| format!("cannot read: {e}"))?;
        Ok(String::from_utf8_lossy(&buffer[..read]).into_owned())
    }

    fn send(&mut self, conn: i64, text: &str) -> Result<(), String> {
        send_raw(conn, text.as_bytes())
    }

    fn send_bytes(&mut self, conn: i64, raw: &[u8]) -> Result<(), String> {
        send_raw(conn, raw)
    }

    fn close_socket(&mut self, handle: i64) -> Result<(), String> {
        SOCKETS.with(|s| {
            let mut s = s.borrow_mut();
            let closed = s.conns.remove(&handle).is_some() || s.listeners.remove(&handle).is_some();
            match closed {
                true => Ok(()),
                false => Err("that is not an open socket".to_string()),
            }
        })
    }

    fn write(&mut self, text: &str) {
        use std::io::Write;
        out(text, b"");
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

    fn read_line(&mut self) -> Result<Option<String>, String> {
        use std::io::{BufRead, Write};
        let _ = std::io::stdout().flush();
        let mut raw = Vec::new();
        // A stream that fails is a stream that has ended, as it is for the
        // native runtime's stdio: a closed descriptor reads as the end of
        // the input on both.
        match std::io::stdin().lock().read_until(b'\n', &mut raw) {
            Ok(0) | Err(_) => Ok(None),
            Ok(_) => line_text(raw).map(Some),
        }
    }

    fn modified(&mut self, path: &str) -> Result<Option<i64>, String> {
        modified_millis(path)
    }

    fn read_file(&mut self, path: &str) -> Result<Option<String>, String> {
        read_file_text(path)
    }

    fn read_bytes(&mut self, path: &str) -> Result<Option<Vec<u8>>, String> {
        read_file_bytes(path)
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

    fn write_bytes(&mut self, path: &str, raw: &[u8]) -> Result<(), String> {
        std::fs::write(path, raw).map_err(|_| format!("cannot write {path}"))
    }
}

/// Reading a file, worded once. The native runtime has a fixed message and
/// three executors implement this trait, so an OS string interpolated in any
/// one of them is a divergence waiting to happen — and it was one: the
/// interpreter said `No such file or directory (os error 2)` where native said
/// `no such file or unreadable`. The fixed wording is also the only one that
/// does not change with the host's libc or this compiler's version, which a
/// reproducible diagnostic cannot afford.
/// The bytes on a socket, whichever value they came from: a string writes its
/// utf-8, a bytes value writes itself, and the socket sees no difference.
fn send_raw(conn: i64, raw: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut stream = SOCKETS.with(|s| {
        let s = s.borrow();
        let found = s.conns.get(&conn);
        found.map(|c| c.try_clone()).transpose().map_err(|e| e.to_string())
    })?;
    let Some(stream) = stream.as_mut() else {
        return Err("that is not a connection".to_string());
    };
    stream.write_all(raw).map_err(|e| format!("cannot write: {e}"))?;
    stream.flush().map_err(|e| format!("cannot write: {e}"))
}

/// One line as `read_line` hands it over: the `\n` and a `\r` before it go,
/// and the rest must be text, worded as the native runtime words it.
pub fn line_text(mut raw: Vec<u8>) -> Result<String, String> {
    if raw.last() == Some(&b'\n') {
        raw.pop();
        if raw.last() == Some(&b'\r') {
            raw.pop();
        }
    }
    String::from_utf8(raw)
        .map_err(|_| "cannot read standard input: the bytes are not text".to_string())
}

/// A file's last-modified time in milliseconds since the epoch, rounded down
/// as the native runtime's integer division rounds a time after the epoch.
/// A path that is not there, or that runs through a file, answers `None`:
/// the two errnos native reads as absence.
pub fn modified_millis(path: &str) -> Result<Option<i64>, String> {
    use std::io::ErrorKind::{NotADirectory, NotFound};
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(why) if matches!(why.kind(), NotFound | NotADirectory) => return Ok(None),
        Err(_) => return Err(format!("cannot read {path}: unreadable")),
    };
    let at = meta.modified().map_err(|_| format!("cannot read {path}: unreadable"))?;
    Ok(Some(match at.duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => after.as_millis() as i64,
        Err(before) => -(before.duration().as_nanos().div_ceil(1_000_000) as i64),
    }))
}

/// The other reader (ruled 2026-08-29): the same three answers as
/// `read_file_text`, with no test of what the bytes are. There is no
/// `InvalidData` arm because there is nothing the bytes can fail to be.
pub fn read_file_bytes(path: &str) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(format!("cannot read {path}: unreadable")),
    }
}

pub fn read_file_text(path: &str) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        // ABSENCE IS DATA, ruled 2026-09-03: "if you know it's possible for
        // them to not be there, you wouldn't use an exception". A file that is
        // not there is the one outcome a caller can anticipate, so it answers
        // `None` here and `file_not_found` in the language. Every other kind
        // stays a failure, because none of them is part of reading's ordinary
        // vocabulary.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        // A file that is THERE and READABLE and simply is not text was being
        // reported as absent, because the reason was thrown away. `kanso.wasm`
        // is the example that found it: the interpreter said the file did not
        // exist while native read it and printed its bytes back unchanged.
        //
        // The two still differ, and the differential law allows that only for
        // an engine that refuses with a clear diagnostic — which is what this
        // makes it. Native reads any file; a Rust `String` cannot hold bytes
        // that are not utf-8, so the interpreter cannot follow it there, and
        // saying so is the most it can do. Whether `read_file` should be
        // byte-transparent on every engine is a design question, filed.
        //
        // The wording stays fixed rather than interpolating the OS's own
        // message: `ErrorKind` is Rust's classification and does not move with
        // the host's libc, where the OS's own text does.
        Err(why) => Err(match why.kind() {
            std::io::ErrorKind::InvalidData => {
                format!("cannot read {path}: the bytes are not text")
            }
            _ => format!("cannot read {path}: unreadable"),
        }),
    }
}

#[derive(Default)]
pub struct ScriptedExecutor {
    pub transcript: Vec<String>,
    pub script_args: Vec<String>,
    pub script_stdin: String,
    pub files: Map<String, String>,
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

    fn read_line(&mut self) -> Result<Option<String>, String> {
        if self.script_stdin.is_empty() {
            return Ok(None);
        }
        let end = self.script_stdin.find('\n').map_or(self.script_stdin.len(), |at| at + 1);
        let line: String = self.script_stdin.drain(..end).collect();
        line_text(line.into_bytes()).map(Some)
    }

    fn modified(&mut self, path: &str) -> Result<Option<i64>, String> {
        self.transcript.push(format!("modified {path:?}"));
        Ok(self.files.contains_key(path).then_some(0))
    }

    fn read_file(&mut self, path: &str) -> Result<Option<String>, String> {
        self.transcript.push(format!("read_file {path:?}"));
        Ok(self.files.get(path).cloned())
    }

    fn read_bytes(&mut self, path: &str) -> Result<Option<Vec<u8>>, String> {
        self.transcript.push(format!("read_bytes {path:?}"));
        Ok(self.files.get(path).map(|text| text.as_bytes().to_vec()))
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

    fn write_bytes(&mut self, path: &str, raw: &[u8]) -> Result<(), String> {
        self.transcript.push(format!("write_file {path:?} {} bytes", raw.len()));
        Ok(())
    }
}
