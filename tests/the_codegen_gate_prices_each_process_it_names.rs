// A reproduction failure says how much the row moved and nothing about where.
// The codegen row is a whole process tree -- kanso, the clang driver, `clang
// -cc1`, ld and the linker's child -- and on 2026-09-17 the release tier read
// 7,239,553,333 and then 7,239,550,228 in one job, 3,105 apart, with the dev
// tier byte-identical across the same pair of runs. The gate named the
// processes in both readings and priced neither, so the job log could not say
// which of the five moved.
//
// So `processes_in` prints a cost beside every name. This spec runs the gate's
// OWN function text against two hand-made profiles and reads what it prints;
// it does not copy the function, because a copy would pass with the gate
// unchanged.

use std::fs;
use std::process::Command;

fn function_text(script: &str, name: &str) -> String {
    let src = fs::read_to_string(script).expect("the gate is readable");
    let open = format!("{name}() {{\n");
    let start = src.find(&open).unwrap_or_else(|| panic!("{script} defines no `{name}`"));
    let rest = &src[start..];
    let end = rest.find("\n}\n").unwrap_or_else(|| panic!("`{name}` in {script} never closes"));
    rest[..end + 3].to_string()
}

fn profile(dir: &std::path::Path, file: &str, cmd: &str, summary: u64) -> String {
    let path = dir.join(file);
    fs::write(&path, format!("version: 1\ncreator: callgrind\ncmd: {cmd}\nsummary: {summary}\n"))
        .expect("the profile is writable");
    path.to_string_lossy().into_owned()
}

#[test]
fn every_process_the_gate_names_carries_the_cost_it_counted() {
    let dir = std::env::temp_dir().join("kanso-procs-priced");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory is writable");

    let a = profile(&dir, "cg.1", "/tmp/kanso-codegen/kanso build pkg/x", 1234);
    let b = profile(&dir, "cg.2", "/usr/lib/llvm-19/bin/clang -cc1 -O3 x.ll", 5678);

    let body = function_text("scripts/gates/codegen_instructions.sh", "processes_in");
    let runner = dir.join("run.sh");
    fs::write(&runner, format!("{body}\nprocesses_in \"$@\"\necho\n"))
        .expect("the runner is writable");

    let out = Command::new("sh").arg(&runner).arg(&a).arg(&b).output().expect("sh runs");
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();

    assert_eq!(
        printed, "kanso=1234 clang=5678",
        "the gate named its processes as `{printed}`. A reading that prices \
         none of them cannot say which process moved when the row does."
    );
}
