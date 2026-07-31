//! Browser playground ABI: raw extern "C" exports, no bindgen. JS writes
//! UTF-8 into a buffer from `kanso_alloc`, calls an entry point, then reads
//! the result from `kanso_out_ptr`/`kanso_out_len`.
use crate::eval::{render, Executor, Interp, Value};
use crate::repl::{Outcome, Session};
use std::cell::RefCell;

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::new());
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static FILE: RefCell<String> = RefCell::new("playground".to_string());
    static RNG: RefCell<crate::eval::Rng> = RefCell::new(crate::eval::Rng::seeded());
}

fn current_file() -> String {
    FILE.with(|f| f.borrow().clone())
}

/// A pseudo-random int in `[0, n)` off the playground's generator, shared by
/// the interpreter and compiled execution paths so a program draws the same
/// stream whichever backend runs it.
pub(crate) fn next_random(n: u64) -> u64 {
    RNG.with(|r| r.borrow_mut().below(n))
}

/// Reseed the playground's generator. The CLI and the differential lattice
/// stay deterministic on a fixed seed; the browser hands in the clock so a
/// program that uses `random` differs from one run to the next.
#[no_mangle]
pub extern "C" fn kanso_set_seed(seed: u32) {
    RNG.with(|r| *r.borrow_mut() = crate::eval::Rng::from_seed(seed as u64));
}

/// Names the source for err origins and diagnostics; the differential
/// harness sets each case's file name so traces match the native engine.
#[no_mangle]
pub extern "C" fn kanso_set_file(ptr: *const u8, len: usize) {
    let name = take_input(ptr, len);
    FILE.with(|f| *f.borrow_mut() = name);
}

/// Playground executor: print goes to a captured stdout; there is no
/// filesystem, argv, or stdin in the browser.
struct BrowserExecutor {
    stdout: String,
    /// Kept apart from stdout and appended after it, exactly as a shell
    /// captures the two streams — so the browser and the native binary
    /// agree byte for byte on a program that writes to both.
    stderr: String,
}

impl Executor for BrowserExecutor {
    fn print(&mut self, text: &str) {
        self.stdout.push_str(text);
        self.stdout.push('\n');
    }

    fn write(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    fn write_err(&mut self, text: &str) {
        self.stderr.push_str(text);
    }

    /// A page has no environment, so every variable is unset — which is a
    /// value the language already has.
    fn env(&mut self, _name: &str) -> Option<String> {
        None
    }

    fn exists(&mut self, _path: &str) -> bool {
        false
    }

    fn is_dir(&mut self, _path: &str) -> bool {
        false
    }

    /// A page has no clock the differential could agree on, so it reads zero
    /// — and a program that timestamps pins KANSO_NOW anyway.
    fn now(&mut self) -> i64 {
        0
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        Err(format!("the playground has no filesystem: cannot list {path}"))
    }

    fn random(&mut self, n: u64) -> u64 {
        next_random(n)
    }

    fn args(&mut self) -> Vec<String> {
        Vec::new()
    }

    fn stdin(&mut self) -> Result<String, String> {
        Err("the playground has no stdin".to_string())
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        Err(format!("the playground has no filesystem: cannot read {path}"))
    }

    fn run(&mut self, cmd: &str, _args: &[String]) -> Result<(i64, String, String), String> {
        Err(format!("the playground cannot start processes: cannot run {cmd}"))
    }

    fn write_file(&mut self, path: &str, _content: &str) -> Result<(), String> {
        Err(format!("the playground has no filesystem: cannot write {path}"))
    }
}

fn set_out(text: &str) {
    OUT.with(|out| *out.borrow_mut() = text.as_bytes().to_vec());
}

fn take_input(ptr: *const u8, len: usize) -> String {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(bytes).into_owned()
}

#[no_mangle]
pub extern "C" fn kanso_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub extern "C" fn kanso_out_ptr() -> *const u8 {
    OUT.with(|out| out.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn kanso_out_len() -> usize {
    OUT.with(|out| out.borrow().len())
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WASM_BYTES: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Compile the program to a wasm module for in-browser execution. Returns
/// 0 with module bytes ready, 1 when the program uses something the browser
/// backend doesn't cover (caller falls back to the interpreter; reason in
/// the output buffer), or 2 on a compile error (rendered in the buffer).
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_compile_wasm(ptr: *const u8, len: usize, tailcalls: i32) -> i32 {
    let source = take_input(ptr, len);
    let program = match crate::compile_source("run", &current_file(), &source) {
        Ok(program) => program,
        Err(rendered) => {
            set_out(&rendered);
            return 2;
        }
    };
    match crate::wasm_backend::compile(&program, tailcalls != 0) {
        Ok(compiled) => {
            crate::wasm_rt::load(program, &compiled.lits, compiled.types);
            WASM_BYTES.with(|b| *b.borrow_mut() = compiled.bytes);
            0
        }
        Err(reason) => {
            set_out(&reason);
            1
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_wasm_ptr() -> *const u8 {
    WASM_BYTES.with(|b| b.borrow().as_ptr())
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_wasm_len() -> usize {
    WASM_BYTES.with(|b| b.borrow().len())
}

/// Execute the handle the compiled program's main returned; output text
/// lands in the buffer, status mirrors the native binary's exit code.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_exec_main(h: u32) -> i32 {
    let (status, text) = crate::wasm_rt::exec_main(h);
    set_out(&text);
    status
}

/// After a trap inside compiled code, fetch the runtime error message.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_take_rt_error() {
    let message = crate::wasm_rt::take_error();
    if message.is_empty() {
        // a trap nothing recorded: the stack is the one resource compiled
        // code exhausts without a chance to say so — the same translation
        // native's parent makes from the child's SIGSEGV
        set_out(
            "error[runtime]: the program ran out of stack: recursion went deeper than the stack holds\n",
        );
        return;
    }
    set_out(&format!("error[runtime]: {message}\n"));
}

/// Evaluate one repl input against the persistent session. Returns 0 on
/// success, 1 on error; the output buffer holds printed text + result.
#[no_mangle]
pub extern "C" fn kanso_repl_eval(ptr: *const u8, len: usize) -> i32 {
    let input = take_input(ptr, len);
    let mut executor = BrowserExecutor { stdout: String::new(), stderr: String::new() };
    let result = SESSION.with(|session| session.borrow_mut().eval(&input, &mut executor));
    match result {
        Ok(outcome) => {
            // `Defined` already carries the whole echo — `defined foo`,
            // `redefined greet`, `imported list` — which the terminal prints
            // as it stands. Prefixing it here said `defined defined foo` on
            // the copy of the repl most people meet.
            let shown = match outcome {
                Outcome::Defined(echo) => echo,
                Outcome::Value(rendered) | Outcome::Executed(rendered) => rendered,
            };
            let mut text = executor.stdout;
            text.push_str(&executor.stderr);
            if !shown.is_empty() {
                text.push_str(&shown);
                text.push('\n');
            }
            set_out(&text);
            0
        }
        Err(message) => {
            set_out(&message);
            1
        }
    }
}

/// Compile and run a whole program (its `main`). Returns 0 on success,
/// 1 on a compile or runtime error.
#[no_mangle]
pub extern "C" fn kanso_run(ptr: *const u8, len: usize) -> i32 {
    let source = take_input(ptr, len);
    let program = match crate::compile_source("run", &current_file(), &source) {
        Ok(program) => program,
        Err(rendered) => {
            set_out(&rendered);
            return 1;
        }
    };
    let interp = Interp::new(&program);
    let value = match interp.run_main() {
        Ok(value) => value,
        Err(runtime) => {
            set_out(&format!("error[runtime]: {}\n", runtime.message));
            return 1;
        }
    };
    let mut executor = BrowserExecutor { stdout: String::new(), stderr: String::new() };
    let (reached, outcome) = match value {
        Value::Desc(desc) => ("the executor", interp.execute(&desc, &mut executor)),
        other => ("the entry", Ok(other)),
    };
    match outcome {
        Ok(Value::ErrV(info)) => {
            let mut text = executor.stdout;
            text.push_str(&executor.stderr);
            text.push_str(&format!(
                "error[endpoint]: unhandled err reached {reached}: {}\n{}",
                render(&info.reason, true),
                crate::eval::trace_lines(&info)
            ));
            set_out(&text);
            1
        }
        Ok(Value::NoneV) if reached == "the entry" => {
            let mut text = executor.stdout;
            text.push_str(&executor.stderr);
            text.push_str("error[endpoint]: unhandled none reached the entry\n");
            set_out(&text);
            1
        }
        Ok(_) => {
            set_out(&executor.stdout);
            0
        }
        Err(runtime) => {
            let mut text = executor.stdout;
            text.push_str(&executor.stderr);
            text.push_str(&format!("error[runtime]: {}\n", runtime.message));
            set_out(&text);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{kanso_set_seed, next_random};

    fn stream(seed: u32) -> Vec<u64> {
        kanso_set_seed(seed);
        (0..8).map(|_| next_random(6)).collect()
    }

    #[test]
    fn a_seed_reproduces_its_stream() {
        assert_eq!(stream(111), stream(111));
    }

    #[test]
    fn different_seeds_draw_different_streams() {
        assert_ne!(stream(111), stream(222));
    }
}
