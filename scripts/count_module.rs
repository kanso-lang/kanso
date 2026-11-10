//! Runs a module `kanso build --wasm` wrote and says how many wasm
//! instructions it executed.
//!
//!   cargo run --release --example count_module -- runtime.wasm app.wasm \
//!       [NAME=value | NAME=@file | /path=@file ...]
//!
//! The module is linked against the runtime as a page links it and run once
//! under `tests/support/wasm32.rs`, the host the browser rows are counted
//! with, so the count is the same on every machine for the same two modules.
//! `NAME=value` is a variable of the environment the module reads with
//! `os/env`, and `NAME=@file` reads the value from a file, since a value may
//! hold newlines. `/path=@file` is a file the module finds at that absolute
//! path, with the bytes of the local one, for `os/read_bytes`.
//! What the module printed goes to stdout, and the count and the memory the
//! run grew by go to stderr, so a caller can keep either.
//!
//! This is an example and not part of the `kanso` binary because wasmi is a
//! dev-dependency: the toolchain a user installs does not carry an
//! interpreter for wasm. `tests/a_built_module_is_counted.rs` compiles this
//! file into itself and calls `answered`, so the spec reads this source and
//! never a binary an earlier build left.

#[path = "../tests/support/wasm32.rs"]
pub mod wasm32;

/// What a run answers: the two streams and the exit status.
pub struct Answer {
    pub out: String,
    pub err: String,
    pub code: i32,
}

fn refused(why: String) -> Answer {
    Answer { out: String::new(), err: format!("count_module: {why}\n"), code: 2 }
}

const USAGE: &str =
    "usage: count_module runtime.wasm app.wasm [NAME=value | NAME=@file | /path=@file ...]";

/// The answer to one command line, without the program's own name.
pub fn answered(args: &[String]) -> Answer {
    if args.len() < 2 {
        return Answer { out: String::new(), err: format!("{USAGE}\n"), code: 2 };
    }
    let read = |path: &str| std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"));
    let (runtime, module) = match (read(&args[0]), read(&args[1])) {
        (Ok(runtime), Ok(module)) => (runtime, module),
        (Err(why), _) | (_, Err(why)) => return refused(why),
    };
    let Some((data, table)) = wasm32::link(&module) else {
        return refused(format!("{} has no kanso.link section", args[1]));
    };
    let mut env: Vec<(String, String)> = Vec::new();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for pair in &args[2..] {
        let Some((name, value)) = pair.split_once('=') else {
            return refused(format!("`{pair}` is not NAME=value"));
        };
        match (name.starts_with('/'), value.strip_prefix('@')) {
            (true, Some(path)) => match read(path) {
                Ok(bytes) => files.push((name.to_string(), bytes)),
                Err(why) => return refused(why),
            },
            (true, None) => return refused(format!("`{pair}` names a path, so it wants =@file")),
            (false, Some(path)) => match read(path).map(String::from_utf8) {
                Ok(Ok(text)) => env.push((name.to_string(), text)),
                Ok(Err(_)) => return refused(format!("{path} is not utf-8")),
                Err(why) => return refused(why),
            },
            (false, None) => env.push((name.to_string(), value.to_string())),
        }
    }
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let files: Vec<(&str, &[u8])> = files.iter().map(|(k, v)| (k.as_str(), &v[..])).collect();
    match wasm32::execute_beside(&runtime, &module, data, table, &env, &files) {
        Ok(ran) => Answer {
            out: ran.out,
            err: format!("{}instructions={}\ngrown_bytes={}\n", ran.err, ran.executed, ran.grown),
            code: ran.exit,
        },
        Err(why) => Answer { out: String::new(), err: format!("count_module: {why}\n"), code: 1 },
    }
}

#[allow(dead_code)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let answer = answered(&args);
    print!("{}", answer.out);
    eprint!("{}", answer.err);
    std::process::exit(answer.code);
}
