//! `kanso build .` addresses a directory the way a shell does, and the binary
//! it writes carries the directory's name — kq's README says `kanso build .`,
//! and what comes out has to be `kq`.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn a_directory_addressed_as_dot_names_the_binary() {
    let work = std::env::temp_dir().join(format!("kanso-dot-build-{}", std::process::id()));
    let program = work.join("greeter");
    std::fs::create_dir_all(&program).expect("the program directory");
    std::fs::write(program.join("main.kso"), "print \"hi\"\n").expect("the entry writes");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(".")
        .current_dir(&program)
        .output()
        .expect("kanso build runs");

    assert!(
        built.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let named: PathBuf = program.join("greeter");
    let anonymous: PathBuf = program.join("main");
    assert!(
        named.is_file() && !anonymous.is_file(),
        "built {}, wanted greeter: {}",
        match anonymous.is_file() {
            true => "main",
            false => "nothing",
        },
        String::from_utf8_lossy(&built.stdout)
    );
}
