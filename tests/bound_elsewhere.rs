use std::process::Command;

/// A bare call reaches the declaration it names, whatever another module
/// calls its parameters. `doubles` binds a parameter `run`, and the alias
/// pass, which keeps a name bare wherever a local of that spelling might
/// mean it, kept `run` bare in every module of the program. The entry's
/// `run` then went through the loader's short-named clone, so an err born
/// in it said `born in run` where the same program without `doubles` says
/// `born in refusals/run`. basket, on the same path, read 6,113M
/// instructions against 31M once std/text bound a parameter `run`.
#[test]
fn a_parameter_elsewhere_leaves_a_call_its_owner() {
    let path = format!("{}/tests/golden/bound_elsewhere", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg("main.kso")
            .args(engine)
            .current_dir(&path)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "error[endpoint]: unhandled err reached the entry: \"refused 4\"\n  born in refusals/run at refusals/refusals.kso:2\n",
            "engine {engine:?}"
        );
        assert_eq!(output.status.code(), Some(1), "engine {engine:?}");
    }
}
