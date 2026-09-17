// The codegen row is a whole process tree, and one of those processes waits
// for another. Measured 2026-09-17, two readings of one binary in one staging
// on this project's container: the three `clang` processes and `ld` came back
// byte for byte, and kanso's own moved 233. A probe binary with a constant
// `pid_tag_of` took that to 112, all of it in `kanso::build`'s self cost with
// every callee identical -- the inlined loop that waits for clang. A loop
// whose iteration count belongs to the scheduler is external state, so under
// the 2026-09-15 rule it is excluded and the exclusion is named.
//
// This runs the gate's OWN `is_kanso` against the five command lines a real
// build produced, so the classifier is checked rather than described. The
// probe's `clang` names `kanso_pn_probe` in its arguments and is the case a
// looser rule gets wrong.

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

const SEEN: &[(&str, bool)] = &[
    ("./kanso build pkg/codegen_corpus --release", true),
    ("/usr/bin/clang -Wno-override-module -c /tmp/kanso_pn_probe_0028241.ll -o /tmp/x.o", false),
    ("/usr/bin/clang -O3 -flto -mllvm -inline-threshold=2000 -o codegen_corpus", false),
    ("/usr/lib/llvm-18/bin/clang -cc1 -triple x86_64-pc-linux-gnu -emit-llvm-bc", false),
    ("/usr/bin/ld -z relro --hash-style=gnu -m elf_x86_64 -pie", false),
];

#[test]
fn the_compiler_is_the_one_process_the_row_leaves_out() {
    let dir = std::env::temp_dir().join("kanso-iskanso");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory is writable");

    let body = function_text("scripts/gates/codegen_instructions.sh", "is_kanso");
    let runner = dir.join("run.sh");
    fs::write(&runner, format!("{body}\nif is_kanso \"$1\"; then echo yes; else echo no; fi\n"))
        .expect("the runner is writable");

    for (n, (cmd, want)) in SEEN.iter().enumerate() {
        let profile = dir.join(format!("cg.{n}"));
        fs::write(&profile, format!("version: 1\ncmd: {cmd}\nsummary: 7\n"))
            .expect("the profile is writable");
        let out = Command::new("sh").arg(&runner).arg(&profile).output().expect("sh runs");
        let said = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let want = if *want { "yes" } else { "no" };
        assert_eq!(
            said, want,
            "the gate answered `{said}` for `{cmd}`. The row excludes exactly \
             one of the five processes, and which one decides what the number \
             means."
        );
    }
}
