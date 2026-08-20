//! A module's files share their declarations, not their imports.

use std::path::PathBuf;
use std::process::Command;

fn check(dir: &PathBuf) -> (String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(dir)
        .output()
        .expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.status.code())
}

fn staged(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-per-file-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, source) in files {
        let landing = dir.join(path);
        std::fs::create_dir_all(landing.parent().expect("a parent")).expect("staging");
        std::fs::write(&landing, source).expect("the file writes");
    }
    dir
}

/// The rule: a file that leans on a sibling's import has a dependency
/// nothing in it names.
#[test]
fn a_qualified_name_needs_this_files_import() {
    let dir = staged(
        "borrowed",
        &[
            ("mod/a.kso", "import \"std/text\"\n\npub fn helper s\n  text/chars s\n"),
            ("mod/b.kso", "pub fn shout s\n  text/join (text/chars s) \"-\"\n"),
            ("main.kso", "import \"./mod\"\n\nprint (mod/shout \"hi\")\n"),
        ],
    );

    let (err, code) = check(&dir);

    assert!(err.contains("`text` is not imported here"), "wrong diagnostic: {err}");
    assert_eq!(code, Some(2));
}

/// Saying it in the file that uses it is all the rule asks.
#[test]
fn the_same_module_passes_once_each_file_says_so() {
    let dir = staged(
        "named",
        &[
            ("mod/a.kso", "import \"std/text\"\n\npub fn helper s\n  text/chars s\n"),
            (
                "mod/b.kso",
                "import \"std/text\"\n\npub fn shout s\n  text/join (text/chars s) \"-\"\n",
            ),
            ("main.kso", "import \"./mod\"\n\nprint (mod/shout \"hi\")\n"),
        ],
    );

    let (err, code) = check(&dir);

    assert_eq!(err, "", "expected a clean check");
    assert_eq!(code, Some(0));
}

/// An import a file does not use is that file's finding, even when a
/// sibling uses the same module.
#[test]
fn an_unused_import_is_per_file() {
    let dir = staged(
        "unused",
        &[
            ("mod/a.kso", "import \"std/text\"\n\npub fn helper s\n  text/chars s\n"),
            ("mod/b.kso", "import \"std/text\"\n\npub fn plain _\n  1\n"),
            ("main.kso", "import \"./mod\"\n\nprint (mod/plain 0)\n"),
        ],
    );

    let (err, code) = check(&dir);

    assert!(err.contains("unused import \"std/text\""), "wrong diagnostic: {err}");
    assert_eq!(code, Some(2));
}
