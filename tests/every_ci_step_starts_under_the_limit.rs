//! A step in CI that runs a script directly starts under `scripts/bounded.sh`
//! (kanso#2441).
//!
//! Cargo's runner bounds every test binary and the ratchet bounds every gate
//! it starts, but a `run:` step in a workflow goes through neither. The
//! workflows name a default shell that sources the limit and then runs the
//! step's script as `bash -e`, which is what a step gets when it names no
//! shell. A step that named a shell of its own would start without the limit,
//! so none may.
//!
//! This reads the workflows' text. What a step really starts under is read in
//! CI, by the `a step starts under the limit` step of the specs job.

const WORKFLOWS: &[&str] = &[".github/workflows/ci.yml", ".github/workflows/ratchet.yml"];

const SHELL: &str = "    shell: bash -c \"if [ -f $GITHUB_WORKSPACE/scripts/bounded.sh ]; then . \
                     $GITHUB_WORKSPACE/scripts/bounded.sh; fi; exec bash -e {0}\"";

fn workflow(path: &str) -> String {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|io| panic!("{path}: {io}"))
}

#[test]
fn every_step_starts_under_the_limit() {
    for path in WORKFLOWS {
        let text = workflow(path);
        let wanted = format!("defaults:\n  run:\n{SHELL}\n");
        assert!(
            text.contains(&wanted),
            "{path} does not start its steps under scripts/bounded.sh: it wants, at the top \
             level,\n{wanted}"
        );
    }
}

#[test]
fn no_step_names_a_shell_of_its_own() {
    for path in WORKFLOWS {
        let text = workflow(path);
        let own: Vec<(usize, &str)> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim_start().starts_with("shell:") && *line != SHELL)
            .collect();
        assert!(
            own.is_empty(),
            "{path} has a step that names its own shell and so starts without the limit: \
             {own:?}"
        );
    }
}

#[test]
fn the_limit_is_read_from_inside_a_step() {
    let text = workflow(".github/workflows/ci.yml");
    assert!(
        text.contains("test \"$(ulimit -S -d)\" = \"$(sed -n 's/^ulimit -S -d \\([0-9]*\\).*/\\1/p' scripts/bounded.sh)\""),
        "ci.yml no longer reads the limit from inside a step"
    );
}
