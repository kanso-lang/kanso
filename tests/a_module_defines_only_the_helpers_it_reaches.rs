//! A module defines only the runtime helpers its program reaches.
//!
//! Every module carries the emitter's helpers -- tag tests, the fast arms of
//! append and index, the stats gates -- as internal functions. A release build
//! inlines them and drops what nobody calls. At `-O0` nothing drops an unused
//! internal function, so `clang -cc1` selected instructions for every helper
//! in every module: on the codegen corpus twenty-four of the thirty-three were
//! called by nothing, and leaving them out took the dev row from 430,900,667
//! to 396,836,531.
//!
//! What a user can see is the module `kanso build` leaves beside the binary.
//! Each helper it defines has to be called from somewhere else in it, and the
//! program has to link and print what the release build prints: a helper
//! dropped while something still called it would leave the link unresolved.
//!
//! A release build's optimiser drops unused helpers too, but only after clang
//! has parsed them, and on the codegen corpus leaving them out of the text
//! took `clang` from 555,145,625 to 545,749,531 with the LTO link's count
//! unchanged. So the release module is held to the same rule.
//!
//! Watched red with every helper counted as reached: the dev module for
//! `print "hi"` defined helpers nothing called. Watched red again with the
//! release tier handed no helper index: the release module did the same.

use std::process::Command;

fn build(dir: &std::path::Path, release: bool) -> (String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("build").arg("main.kso").current_dir(dir);
    if release {
        cmd.arg("--release");
    }
    let built = cmd.output().expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(dir.join("main")).output().expect("the binary runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    (module, String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// The internal functions `module` defines that no other line of it names.
fn uncalled(module: &str) -> Vec<String> {
    let lines: Vec<&str> = module.lines().collect();
    let mut out = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if !line.starts_with("define internal ") {
            continue;
        }
        let at = line.find('@').expect("a define names its function");
        let paren = at + line[at..].find('(').expect("a define has parameters");
        let call = &line[at..=paren];
        if !lines.iter().enumerate().any(|(m, l)| m != n && l.contains(call)) {
            out.push(line[at + 1..paren].to_string());
        }
    }
    out
}

fn each(program: &str, printed: &str) {
    let dir = std::env::temp_dir().join(format!(
        "kanso_dev_reach_{}_{}",
        std::process::id(),
        program.len()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("main.kso"), program).expect("the program writes");
    let (dev, dev_out) = build(&dir, false);
    let (release, release_out) = build(&dir, true);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(dev_out, printed);
    assert_eq!(release_out, printed);
    assert_eq!(
        uncalled(&dev),
        Vec::<String>::new(),
        "the dev module defines helpers nothing calls"
    );
    assert_eq!(
        uncalled(&release),
        Vec::<String>::new(),
        "the release module defines helpers nothing calls"
    );
}

#[test]
fn a_print_carries_no_helper_it_does_not_call() {
    each("print \"hi\"\n", "hi\n");
}

#[test]
fn a_closure_and_an_append_keep_the_helpers_they_call() {
    each(
        "import \"std/text\"\n\ntwice = (f x -> f (f x))\n\nprint (twice (s -> text/join [s \"!\"] \"\") \"hi\")\n",
        "hi!!\n",
    );
}
