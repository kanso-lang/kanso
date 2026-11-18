// `kanso run` and `kanso play`: a program's binary, cached by what it was
// built from, run with its signals and exit status passed through. main.rs
// reads this file with include!, so its items stay in the binary's root
// module in the order they had there, and the compiler builds as it did
// (kanso#1848, kanso#2502). cargo fmt does not reach an included file, so
// CI's format job checks the files under src/'s subdirectories itself.

/// `run` builds and executes: a dev-mode native binary, cached by IR hash so
/// an unchanged program re-runs with no clang at all. `--plan` stays on the
/// interpreter — it renders the effect DAG instead of executing it. `asked`
/// is the path as the user typed it, before a directory was taken to its entry.
fn run(program: &ast::Program, file: &str, asked: &str, source: &str, plan: bool) -> ExitCode {
    if plan {
        return run_plan(program, file, source);
    }
    match built_binary(program, "run", asked) {
        Ok(binary) => execute(&binary, ended_by_signal),
        Err(code) => code,
    }
}

/// `kanso play` of a file whose modules all came from this binary. The file's
/// text decides its program once the compiler that read it is fixed, so a
/// second play of the same text looks its binary up by that text and does not
/// emit at all. Emitting was two thirds of a warm `kanso play print "x"`:
/// 404,031 of the 615,803 instructions under `kanso::main`, spent writing IR
/// whose only use was to be hashed into the name of a binary already built.
///
/// The first play emits, builds through the IR's key as `run` does, and gives
/// the binary a second name under the text's key. A file that loaded a module
/// from disk has no text key, since that module can change under it, and goes
/// through `run`.
fn play(program: &ast::Program, file: &str, source: &str) -> ExitCode {
    let played = match kanso::play_file_is_self_contained() {
        true => played_path(file, source),
        false => None,
    };
    let Some(played) = played else {
        return run(program, file, file, source, false);
    };
    let explain = ended_by_signal;
    if played.exists() {
        return execute(&played, explain);
    }
    match built_binary(program, "play", file) {
        Ok(binary) => {
            // Two plays racing to name one binary both find the name taken or
            // take it, and either way the file under it is the same build.
            let _ = std::fs::hard_link(&binary, &played);
            execute(&binary, explain)
        }
        Err(code) => code,
    }
}

/// The binary an earlier play of this same text left behind, if there is one.
/// A name under the text's key exists only when a play compiled the file
/// cleanly from embedded modules, and everything that compile read is in the
/// key, so the file is run without being lexed, parsed or checked again: that
/// was 172,583 of the 208,643 instructions left in a warm play. A `KANSO_`
/// setting sends the file through the compiler anyway, because several of them
/// ask it to report on its own work as it goes.
fn played_before(file: &str, source: &str) -> Option<std::path::PathBuf> {
    if std::env::vars_os().any(|(name, _)| name.as_encoded_bytes().starts_with(b"KANSO_")) {
        return None;
    }
    let played = played_path(file, source)?;
    played.exists().then_some(played)
}

fn played_path(file: &str, source: &str) -> Option<std::path::PathBuf> {
    let key = played_key(file, source)?;
    Some(std::env::temp_dir().join(format!("kanso_play_{key}")))
}

/// What decides a play file's binary, hashed. The IR is a function of the
/// file's name and text, the compiler, the runtime it links, the closure
/// convention the installed clang takes, whether counters are compiled in, and
/// the `KANSO_` settings the compiler reads, so each of those goes into the
/// key. The compiler is named by its own file's path, length and modification
/// time: a rebuild writes a new file, and the key moves with it.
fn played_key(file: &str, source: &str) -> Option<String> {
    let compiler = std::env::current_exe().ok()?;
    let meta = std::fs::metadata(&compiler).ok()?;
    let written = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    let mut text: Vec<u8> = Vec::with_capacity(file.len() + source.len() + 256);
    let mut part = |bytes: &[u8]| {
        text.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        text.extend_from_slice(bytes);
    };
    part(file.as_bytes());
    part(source.as_bytes());
    part(compiler.as_os_str().as_encoded_bytes());
    part(&meta.len().to_le_bytes());
    part(&written.as_nanos().to_le_bytes());
    let convention = closure_convention();
    part(&[matches!(convention, kanso::codegen::ClosureConvention::PreserveNone) as u8]);
    for (name, value) in std::env::vars_os() {
        if name.as_encoded_bytes().starts_with(b"KANSO_") {
            part(name.as_encoded_bytes());
            part(value.as_encoded_bytes());
        }
    }
    let (ka, kb) = kanso::hash::key_of(&text);
    let (ra, rb) = kanso::hash::RUNTIME_DIGEST;
    let counting = if kanso::codegen::counters_wanted() { "c" } else { "" };
    Some(format!("{}{}{counting}", hex16(ka ^ ra), hex16(kb ^ rb.rotate_left(17))))
}

/// The dev binary for a program: its IR emitted and looked up by the IR's key,
/// built on a miss. `verb` and `file` word the way out when clang is missing,
/// since the interpreter runs the same program without it.
fn built_binary(
    program: &ast::Program,
    verb: &str,
    file: &str,
) -> Result<std::path::PathBuf, ExitCode> {
    let ir = match kanso::codegen::emit_ir_dev(program, closure_convention()) {
        Ok(ir) => ir,
        Err(unsupported) => {
            eprintln!("error: {unsupported}");
            return Err(ExitCode::from(2));
        }
    };
    match cached_program_binary(&ir) {
        Ok(binary) => Ok(binary),
        Err(io) if clang_is_missing(&io) => {
            eprintln!(
                "error: cannot build: clang is not on PATH, and `kanso {verb}` builds a \
                 native binary with it. install clang, or run on the interpreter with \
                 `kanso {verb} {file} --interp`"
            );
            Err(ExitCode::FAILURE)
        }
        Err(io) => {
            eprintln!("error: cannot build: {io}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// Whether a failed build failed because no `clang` resolves on `PATH`. The
/// spawn reports that as a bare "not found", which reads as a missing file.
fn clang_is_missing(io: &std::io::Error) -> bool {
    let on_path = std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("clang").is_file()));
    io.kind() == std::io::ErrorKind::NotFound && !on_path
}

/// Runs a built binary. `explain` words a death by signal, and is asked only
/// then, so a caller that has no checked program in hand compiles one there.
fn execute(
    binary: &std::path::Path,
    explain: impl FnOnce(&std::process::ExitStatus) -> String,
) -> ExitCode {
    let status =
        std::process::Command::new(binary).args(program_args()).spawn().and_then(|mut child| {
            handing_on_to(child.id());
            let status = child.wait();
            handing_on_to(0);
            status
        });
    match status {
        Ok(code) => match code.code() {
            Some(n) => ExitCode::from(n.clamp(0, 255) as u8),
            None => {
                ended_with_the_reader(&code);
                ended_as_asked(&code);
                eprintln!("{}", explain(&code));
                ExitCode::FAILURE
            }
        },
        Err(io) => {
            eprintln!("error: cannot execute {}: {io}", binary.display());
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
const SIGPIPE: i32 = 13;

/// The signals a supervisor sends to stop a program: `kill`'s default and a
/// closed terminal's. The driver hands them to the program it runs, which
/// would otherwise go on running, reparented, after the driver ended.
#[cfg(unix)]
const HANDED_ON: [i32; 2] = [SIGTERM, SIGHUP];

#[cfg(unix)]
const SIGTERM: i32 = 15;

#[cfg(unix)]
const SIGHUP: i32 = 1;

#[cfg(unix)]
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

/// The program the driver is waiting on, or 0 while there is none.
#[cfg(unix)]
static RUNNING: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// The signal the driver handed on, or 0 when it handed on none. A program
/// that a signal from elsewhere ended is reported as ended by it; only one the
/// driver was sent ends the driver the same way.
#[cfg(unix)]
static HANDED: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// With a program running, the signal goes to it. With none, which is the
/// driver's state once the program has ended, the signal does what it would
/// have done had the driver never caught it, so the handler is installed once
/// and never taken down.
#[cfg(unix)]
extern "C" fn hand_on(sig: i32) {
    let pid = RUNNING.load(std::sync::atomic::Ordering::SeqCst);
    if pid > 0 {
        HANDED.store(sig, std::sync::atomic::Ordering::SeqCst);
        unsafe {
            kill(pid, sig);
        }
    } else {
        unsafe {
            signal(sig, 0);
            raise(sig);
        }
    }
}

/// From here on, a SIGTERM or SIGHUP the driver receives goes to `pid`; 0
/// means no program is running.
#[cfg(unix)]
fn handing_on_to(pid: u32) {
    RUNNING.store(pid as i32, std::sync::atomic::Ordering::SeqCst);
    if pid == 0 {
        return;
    }
    for sig in HANDED_ON {
        unsafe {
            signal(sig, hand_on as extern "C" fn(i32) as usize);
        }
    }
}

#[cfg(not(unix))]
fn handing_on_to(_pid: u32) {}

/// A program ended by the signal the driver handed on ends the driver the
/// same way, saying nothing, so whoever sent it sees what it asked for. One
/// ended by a signal the driver was never sent is reported, as before.
#[cfg(unix)]
fn ended_as_asked(status: &std::process::ExitStatus) {
    use std::os::unix::process::ExitStatusExt;
    let handed = HANDED.load(std::sync::atomic::Ordering::SeqCst);
    if let Some(sig) = status.signal().filter(|s| *s == handed) {
        unsafe {
            raise(sig);
        }
    }
}

#[cfg(not(unix))]
fn ended_as_asked(_status: &std::process::ExitStatus) {}

/// A write to a pipe whose reader has gone, as `head` leaves it, ends the
/// process by SIGPIPE and says nothing. That is what a built binary does. Rust
/// ignores the signal before `main`, so the interpreter's `print` met the
/// broken pipe as an error instead and panicked with two backtraces. Putting
/// the default back makes the interpreter stop the way the binary does.
fn reader_may_close() {
    #[cfg(unix)]
    unsafe {
        signal(SIGPIPE, 0);
    }
}

/// A program the reader stopped reading was ended by SIGPIPE, and the driver
/// ends the same way, saying nothing, so `kanso play` behind `head` stops as
/// the binary alone would.
#[cfg(unix)]
fn ended_with_the_reader(status: &std::process::ExitStatus) {
    use std::os::unix::process::ExitStatusExt;
    if status.signal() == Some(SIGPIPE) {
        unsafe {
            raise(SIGPIPE);
        }
    }
}

#[cfg(not(unix))]
fn ended_with_the_reader(_status: &std::process::ExitStatus) {}

/// A program the operating system killed has no exit code to report, and
/// saying nothing leaves the reader with a bare failure and no cause.
#[cfg(unix)]
fn ended_by_signal(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    const SIGSEGV: i32 = 11;
    match status.signal() {
        Some(SIGSEGV) => kanso::stack_exhausted(None),
        Some(other) => format!("error[runtime]: the program was ended by signal {other}"),
        None => "error[runtime]: the program ended without an exit code".to_string(),
    }
}

#[cfg(not(unix))]
fn ended_by_signal(_status: &std::process::ExitStatus) -> String {
    "error[runtime]: the program ended without an exit code".to_string()
}

fn cached_program_binary(ir: &str) -> std::io::Result<std::path::PathBuf> {
    // 128 bits of the IR, at a third of SipHash's cost: see `hash::key_of`.
    // The runtime the binary would be linked against is a constant, so its
    // digest is folded in rather than walked. Whether it counts is not: a
    // counting run and an ordinary one emit the same IR and link different
    // runtimes, so without the mark the second of them ran the first one's
    // binary, and a counting run printed no counters.
    let (ka, kb) = kanso::hash::key_of(ir.as_bytes());
    let (ra, rb) = kanso::hash::RUNTIME_DIGEST;
    let counting = if kanso::codegen::counters_wanted() { "c" } else { "" };
    let key = format!("{}{}{counting}", hex16(ka ^ ra), hex16(kb ^ rb.rotate_left(17)));
    let binary = std::env::temp_dir().join(format!("kanso_run_{key}"));
    if binary.exists() {
        return Ok(binary);
    }
    // The process writing the IR owns the file it hands clang. Two runs of
    // the same program share a key, so a shared path let one truncate and
    // rewrite what the other's clang was already reading — which surfaces as
    // a segmentation fault inside LLVM's assembly lexer, blamed on the
    // program rather than on the race.
    let ll_path = std::env::temp_dir().join(format!("kanso_run_{key}_{}.ll", pid_tag()));
    std::fs::write(&ll_path, ir)?;
    let staging = std::env::temp_dir().join(format!("kanso_run_{key}_{}", pid_tag()));
    let ll = ll_path.to_string_lossy().into_owned();
    let out = staging.to_string_lossy().into_owned();
    let status = dev_clang(&out, &ll)?;
    if !status.success() {
        return Err(std::io::Error::other("clang failed"));
    }
    std::fs::rename(&staging, &binary)?;
    // clang has read it, and nothing reads it again: the binary beside it is
    // the cache, keyed by the same hash. Left behind, one accumulates per
    // cache MISS -- 42 KB each, and a long-lived container reached 112,000 of
    // them, which is the whole of a session's disk allowance. The staging
    // path above needs no such line because `rename` consumes it.
    let _ = std::fs::remove_file(&ll_path);
    Ok(binary)
}

fn run_plan(program: &ast::Program, file: &str, source: &str) -> ExitCode {
    let interp = eval::Interp::new(program);
    let result = match interp.run_main() {
        Ok(value) => value,
        Err(runtime) => {
            let d = diag::Diagnostic::new("runtime", runtime.message, runtime.span);
            eprint!("{}", diag::paint(&diag::render(&[d], file, source)));
            return ExitCode::FAILURE;
        }
    };
    match result {
        eval::Value::Desc(desc) => {
            let mut out = String::from("plan:\n");
            let step = |callee: &eval::Value| interp.ignoring_step(callee);
            eval::render_plan(&desc, &mut out, &step);
            print!("{out}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("error: main is not an io; there is no plan to show");
            ExitCode::FAILURE
        }
    }
}
