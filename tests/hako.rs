use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/hako"))
}

fn run(dir: &Path, cache: Option<&Path>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
    command.arg("run").arg(".").current_dir(dir);
    match cache {
        Some(path) => command.env("KANSO_HAKO", path),
        // a cache that cannot exist, so the developer's own is never read
        None => command.env("KANSO_HAKO", "/nonexistent-hako-cache"),
    };
    command.output().expect("kanso binary runs")
}

/// The reason local imports are dot-prefixed: a bare multi-segment path is a
/// hako name and is never tried as a directory, so a subtree that happens to
/// be called `owner/repo` cannot stand in for the hako of that name. The
/// fixture has exactly such a subtree, and importing it must still ask for
/// `kanso install`.
#[test]
fn a_local_subtree_never_shadows_a_hako_name() {
    let output = run(&fixtures().join("app"), None);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("names a hako, and it is not in the cache"),
        "a local owner/repo answered for the hako: {stderr}"
    );
}

/// And the same import resolves once the hako is actually cached.
#[test]
fn a_hako_resolves_from_the_cache() {
    let cache = fixtures().join("cache");
    let output = run(&fixtures().join("app"), Some(&cache));

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "from the hako cache\n",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Each shape in the table answers in its own words, so a failed import says
/// which rule it was judged by rather than a single catch-all. Staged in a
/// directory of its own — the corpus fixture is shared, and a test that
/// rewrites shared state cannot run beside its neighbours.
#[test]
fn every_import_shape_names_its_own_rule() {
    let app = std::env::temp_dir().join("kanso-hako-shapes");

    for (import, phrase) in [
        ("./nope", "a directory beside the importing module"),
        ("sibling", "a sibling subdirectory module"),
        ("corp.dev/team/thing", "names a hako by domain"),
        ("owner/missing", "names a hako, and it is not in the cache"),
    ] {
        let _ = std::fs::remove_dir_all(&app);
        std::fs::create_dir_all(&app).expect("temp work dir");
        std::fs::write(app.join("main.kso"), format!("import \"{import}\"\n\nprint \"x\"\n"))
            .expect("the entry writes");

        let output = run(&app, None);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(phrase), "`{import}` was judged by another rule: {stderr}");
    }
}
