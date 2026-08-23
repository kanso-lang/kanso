//! A build is named for its program, so `kanso build myapp` run from the
//! directory above `myapp/` wants to write a file where the directory already
//! is. The linker's words for that are `cannot open output file myapp: Is a
//! directory` and then `clang failed on myapp.ll`, which name neither the
//! cause nor the way out — and the way out is one line, so the refusal says
//! it.

use std::path::Path;
use std::process::Command;

fn built_in(dir: &Path, target: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(target)
        .current_dir(dir)
        .output()
        .expect("kanso runs")
}

/// One module in a directory, and an entry beside it.
fn a_module_named(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-build-collide-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let module = dir.join(name);
    std::fs::create_dir_all(&module).expect("a directory to build in");
    std::fs::write(
        module.join(format!("{name}.kso")),
        "import \"std/os\"\n\npub play = os/args . (a -> print \"ran with {length a}\")\n",
    )
    .expect("the module writes");
    std::fs::write(module.join("main.kso"), format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry writes");
    dir
}

#[test]
fn a_build_that_would_overwrite_its_own_directory_says_so() {
    let dir = a_module_named("greeter");
    let refused = built_in(&dir, "greeter");
    let said = String::from_utf8_lossy(&refused.stderr).into_owned();

    assert!(
        said.contains("this build is named `greeter`, and a directory of that name is here"),
        "the refusal has to name the collision: {said}"
    );
    assert!(said.contains("cd greeter && kanso build ."), "and the way out: {said}");
    assert!(!said.contains("clang"), "the linker's complaint is not the user's: {said}");
    assert_eq!(refused.status.code(), Some(2), "{said}");
    assert!(!dir.join("greeter.ll").exists(), "nothing is written on the way to refusing");

    // The way out works, which is what makes it worth printing.
    let built = built_in(&dir.join("greeter"), ".");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    assert!(dir.join("greeter").join("greeter").exists(), "the binary lands inside");

    let _ = std::fs::remove_dir_all(&dir);
}
