//! A dev build asks its hot predicates about a value's two words.
//!
//! `k_not_failure`, `k_truthy` and `k_check_rec_fast` take a `%KValue`, and at
//! -O0 clang's fast instruction selector cannot lower a call that passes one:
//! each such call went to the slow selector on its own, 216 of them on the
//! codegen corpus. A dev module pulls the value's tag and payload out and calls
//! `k_not_failure_w`, `k_truthy_w` and `k_check_rec_fast_w`, which take them as
//! `i64`s. On the corpus `clang -cc1` went from 280,905,590 instructions to
//! 256,343,115.
//!
//! What a user can see is the module `kanso build` leaves beside the binary,
//! and what the program prints, which must not change with the tier.
//!
//! Watched red with the dev tier asking the `%KValue` forms: the dev module
//! called `k_truthy`, `k_not_failure` and `k_check_rec_fast` again.

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

/// The calls in `module` that pass a temporary `%KValue` to `name`.
fn by_value(module: &str, name: &str) -> usize {
    module.matches(&format!("call i64 @{name}(%KValue %")).count()
}

/// The calls in `module` to `name`.
fn calls(module: &str, name: &str) -> usize {
    module.matches(&format!("call i64 @{name}(")).count()
}

#[test]
fn the_dev_module_passes_words_and_prints_what_release_prints() {
    let dir = std::env::temp_dir().join(format!("kanso_dev_words_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    // A record pattern, a truth test on a value the checker cannot fold, and
    // a group whose argument may be a failure.
    std::fs::write(
        dir.join("shapes.kso"),
        "type point\n  x\n  y\n\nfn sum (point x y)\n  x + y\n\nfn sum _\n  0\n\n\
         fn pick b\n  if b \"yes\" \"no\"\n\nfn half n\n  return err \"odd\" if n % 2 == 1\n  n / 2\n\n\
         fn shown (err _)\n  \"failed\"\n\nfn shown n\n  \"{n}\"\n\n\
         pub fn first n\n  \"{sum (point n 4)} {sum 5} {pick (sum 5 == 0)}\"\n\n\
         pub fn second n\n  \"{shown (half n)} {shown (half (n - 1))}\"\n",
    )
    .expect("the library writes");
    std::fs::write(dir.join("main.kso"), "import \"./shapes\"\n\nprint (shapes/first 3)\nprint (shapes/second 4)\n")
        .expect("the program writes");
    let (dev, dev_out) = build(&dir, false);
    let (release, release_out) = build(&dir, true);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(dev_out, "7 0 yes\n2 failed\n");
    assert_eq!(dev_out, release_out);
    for name in ["k_not_failure", "k_truthy", "k_check_rec_fast"] {
        assert_eq!(by_value(&dev, name), 0, "the dev module passes a %KValue to {name}");
        assert!(calls(&dev, &format!("{name}_w")) > 0, "the dev module never calls {name}_w");
        assert_eq!(calls(&release, &format!("{name}_w")), 0, "the release module calls {name}_w");
    }
}
