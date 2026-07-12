//! Browser playground ABI: raw extern "C" exports, no bindgen. JS writes
//! UTF-8 into a buffer from `kanso_alloc`, calls an entry point, then reads
//! the result from `kanso_out_ptr`/`kanso_out_len`.
use crate::eval::{render, Executor, Interp, Value};
use crate::repl::{Outcome, Session};
use std::cell::RefCell;

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::new());
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Playground executor: print goes to a captured stdout; there is no
/// filesystem, argv, or stdin in the browser.
struct BrowserExecutor {
    stdout: String,
}

impl Executor for BrowserExecutor {
    fn print(&mut self, text: &str) {
        self.stdout.push_str(text);
        self.stdout.push('\n');
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

#[no_mangle]
pub extern "C" fn kanso_reset() {
    SESSION.with(|session| *session.borrow_mut() = Session::new());
}

/// Evaluate one repl input against the persistent session. Returns 0 on
/// success, 1 on error; the output buffer holds printed text + result.
#[no_mangle]
pub extern "C" fn kanso_repl_eval(ptr: *const u8, len: usize) -> i32 {
    let input = take_input(ptr, len);
    let mut executor = BrowserExecutor { stdout: String::new() };
    let result =
        SESSION.with(|session| session.borrow_mut().eval(&input, &mut executor));
    match result {
        Ok(outcome) => {
            let shown = match outcome {
                Outcome::Defined(names) => format!("defined {names}"),
                Outcome::Value(rendered) | Outcome::Executed(rendered) => rendered,
            };
            let mut text = executor.stdout;
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
    let program = match crate::compile("playground", &source, true) {
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
    let mut executor = BrowserExecutor { stdout: String::new() };
    let outcome = match value {
        Value::Desc(desc) => interp.execute(&desc, &mut executor),
        other => Ok(other),
    };
    match outcome {
        Ok(Value::ErrV(reason)) => {
            let mut text = executor.stdout;
            text.push_str(&format!(
                "error[endpoint]: unhandled err reached the executor: {}\n",
                render(&reason, true)
            ));
            set_out(&text);
            1
        }
        Ok(_) => {
            set_out(&executor.stdout);
            0
        }
        Err(runtime) => {
            let mut text = executor.stdout;
            text.push_str(&format!("error[runtime]: {}\n", runtime.message));
            set_out(&text);
            1
        }
    }
}
