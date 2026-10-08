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

/// Hands the compiler a one-file module under the path an import will name,
/// so a program that is a library plus the entry file that runs it compiles
/// where there is no filesystem to read either from. The sources stay until
/// `kanso_forget_sources`.
#[no_mangle]
pub extern "C" fn kanso_hand_source(
    path_ptr: *const u8,
    path_len: usize,
    file_ptr: *const u8,
    file_len: usize,
    src_ptr: *const u8,
    src_len: usize,
) {
    let path = take_input(path_ptr, path_len);
    let file = take_input(file_ptr, file_len);
    let source = take_input(src_ptr, src_len);
    crate::hand_source(&path, vec![(file, source)]);
}

#[no_mangle]
pub extern "C" fn kanso_forget_sources() {
    crate::forget_sources();
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

    fn read_file(&mut self, path: &str) -> Result<Option<String>, String> {
        Err(format!("the playground has no filesystem: cannot read {path}"))
    }

    fn read_bytes(&mut self, path: &str) -> Result<Option<Vec<u8>>, String> {
        Err(format!("the playground has no filesystem: cannot read {path}"))
    }

    fn run(&mut self, cmd: &str, _args: &[String]) -> Result<(i64, String, String), String> {
        Err(format!("the playground cannot start processes: cannot run {cmd}"))
    }

    fn make_dir(&mut self, path: &str) -> Result<(), String> {
        Err(format!("the playground has no filesystem: cannot make {path}"))
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

#[cfg(target_arch = "wasm32")]
thread_local! {
    static SIDE: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
}

/// Compile the program the way `kanso build` does, and lower the module the
/// native emitter writes to a side module for the wasm32 runtime
/// (`ir_wasm`). Returns 0 with the module ready, 1 when the emitter or the
/// translator refuses the program (reason in the output buffer), or 2 on a
/// compile error. `kanso_side_data` and `kanso_side_table` then say what the
/// host sets aside for it before instantiation.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_compile_native(ptr: *const u8, len: usize) -> i32 {
    let source = take_input(ptr, len);
    match crate::compile_source("run", &current_file(), &source) {
        Ok(program) => lower_native(program),
        Err(rendered) => {
            set_out(&rendered);
            2
        }
    }
}

/// The play door onto the same route.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_play_native(ptr: *const u8, len: usize) -> i32 {
    let source = take_input(ptr, len);
    match crate::compile_play_file(&current_file(), &source) {
        Ok(program) => lower_native(program),
        Err(rendered) => {
            set_out(&rendered);
            2
        }
    }
}

/// THE PROGRAM IS HANDED TO THE EMITTER, which drops the arms nothing can
/// reach from it in place and lets it go before the module is assembled.
/// The tab's compile is priced by the most it holds at once
/// (`browser_compile_peak_bytes`), and that peak falls inside the translation.
/// The translation reads the native IR as it is, rewriting the six wasm32
/// offsets line by line, so the module text is the only copy it holds.
#[cfg(target_arch = "wasm32")]
fn lower_native(program: crate::ast::Program) -> i32 {
    let convention = crate::codegen::ClosureConvention::Absent;
    let ir = crate::codegen::emit_ir_tab_owned(program, convention);
    let side = ir.and_then(crate::ir_wasm::translate_owned);
    match side {
        Ok(side) => {
            SIDE.with(|s| s.set((side.data, side.table)));
            WASM_BYTES.with(|b| *b.borrow_mut() = side.wasm);
            0
        }
        Err(reason) => {
            set_out(&reason);
            1
        }
    }
}

/// The page's answer when a natively compiled program exhausts the wasm call
/// stack: the sentence every other engine prints, in the output buffer.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_stack_exhausted() {
    set_out(&format!("{}\n", crate::stack_exhausted()));
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_side_data() -> u32 {
    SIDE.with(|s| s.get().0)
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn kanso_side_table() -> u32 {
    SIDE.with(|s| s.get().1)
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

/// Compile and run a play file — the relaxed form the playground buffer
/// holds: declarations and statements in one file, stdlib imports only.
/// Returns 0 on success, 1 on a compile or runtime error.
#[no_mangle]
pub extern "C" fn kanso_play(ptr: *const u8, len: usize) -> i32 {
    let source = take_input(ptr, len);
    let program = match crate::compile_play_file(&current_file(), &source) {
        Ok(program) => program,
        Err(rendered) => {
            set_out(&rendered);
            return 1;
        }
    };
    finish_run(&program)
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
    finish_run(&program)
}

/// Run a compiled program's `main` and report through the browser executor.
fn finish_run(program: &crate::ast::Program) -> i32 {
    let interp = Interp::new(program);
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
                render(&interp, &info.reason, true),
                crate::eval::trace_lines(&interp, &info)
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
