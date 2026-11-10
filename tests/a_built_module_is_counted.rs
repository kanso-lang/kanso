//! `count_module` runs a module `kanso build --wasm` wrote and reports the
//! wasm instructions it executed, for a project that prices its own code in
//! the browser the way `tests/browser_cost.rs` prices the corpus.
//!
//! The example's source is compiled into this spec and its `answered` is
//! called with a command line, so the spec can never read a binary an
//! earlier build left: `cargo test --test` does not rebuild examples, and a
//! first version of this spec passed against an example edited to report one
//! instruction too many.
//!
//! The spec builds a program that prints a variable of its environment and
//! the bytes of a file, hands both over from local files, and reads three
//! things: the program's output, the count equal to the count the host gives
//! when called here directly, and the same count on a second run. The file
//! holds a zero byte and bytes that are not utf-8, which a variable could not
//! carry, and a path the host was not given reads as absent, as it does
//! natively.

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../scripts/count_module.rs"]
mod count_module;
use count_module::wasm32::{execute_beside, link, runtime, toolchain};

fn reported(stderr: &str, row: &str) -> u64 {
    let line = stderr
        .lines()
        .find_map(|l| l.strip_prefix(row).and_then(|l| l.strip_prefix('=')))
        .unwrap_or_else(|| panic!("no `{row}=` line in:\n{stderr}"));
    line.parse().expect("the row is a number")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the work dir makes");
    dir
}

fn at(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn the_example_reports_what_the_host_counts() {
    if !toolchain() {
        eprintln!("no wasm32 toolchain here: clang, wasm-ld and wasi-libc are wanted");
        return;
    }
    let work = work("count-module");
    let entry = "import \"std/list\"\nimport \"std/os\"\n\n\
                 os/env \"HEARD\" .> (h -> print \"heard {h}\")\n\
                 os/read_bytes \"/held/raw.bin\" .> (b -> print (list/to_list b))\n\
                 os/read_bytes \"/held/absent\" .> (b -> print b)\n";
    std::fs::write(work.join("page.kso"), entry).expect("the entry writes");
    std::fs::write(work.join("said.txt"), "two\nlines").expect("the value writes");
    let raw: &[u8] = &[104, 0, 255, 10, 195];
    std::fs::write(work.join("raw.bin"), raw).expect("the bytes write");
    let printed = "heard two\nlines\n[104 0 255 10 195]\nos/file_not_found \"/held/absent\"\n";

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "page.kso", "--wasm"])
        .current_dir(&work)
        .output()
        .expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));

    let rt = runtime(&work);
    std::fs::write(work.join("runtime.wasm"), &rt).expect("the runtime writes");
    let module = std::fs::read(work.join("page.wasm")).expect("the module was written");
    let (data, table) = link(&module).expect("the module says how to link it");
    let env = [("HEARD", "two\nlines")];
    let direct = execute_beside(&rt, &module, data, table, &env, &[("/held/raw.bin", raw)])
        .expect("the module links and runs");
    assert_eq!((direct.out.as_str(), direct.err.as_str()), (printed, ""));

    let line = [
        at(&work, "runtime.wasm"),
        at(&work, "page.wasm"),
        format!("HEARD=@{}", at(&work, "said.txt")),
        format!("/held/raw.bin=@{}", at(&work, "raw.bin")),
    ];
    let first = count_module::answered(&line);
    assert_eq!((first.out.as_str(), first.code), (printed, 0), "{}", first.err);
    assert_eq!(reported(&first.err, "instructions"), direct.executed);
    assert_eq!(reported(&first.err, "grown_bytes"), direct.grown);
    let again = count_module::answered(&line);
    assert_eq!(reported(&again.err, "instructions"), direct.executed);
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn a_file_that_is_no_module_is_refused_by_name() {
    let work = work("count-refused");
    std::fs::write(work.join("plain.wasm"), b"\0asm\x01\0\0\0").expect("writes");
    let plain = at(&work, "plain.wasm");
    let refused = count_module::answered(&[plain.clone(), plain.clone()]);
    assert_eq!(refused.code, 2);
    assert_eq!(refused.err, format!("count_module: {plain} has no kanso.link section\n"));
    let _ = std::fs::remove_dir_all(&work);
}
