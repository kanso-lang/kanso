/// The binary's source: `src/main.rs` and the files under `src/main/` that it
/// reads with `include!`, as (path, text), main.rs first and the rest in name
/// order. They compile as one module, so a function moves between them
/// without changing what the binary does, and a spec over the binary reads
/// them all.
pub fn driver_sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut included: Vec<String> = std::fs::read_dir(root.join("src/main"))
        .expect("src/main reads")
        .map(|e| e.expect("a directory entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs"))
        .map(|n| format!("src/main/{n}"))
        .collect();
    included.sort();
    std::iter::once("src/main.rs".to_string())
        .chain(included)
        .map(|path| {
            let text = std::fs::read_to_string(root.join(&path))
                .unwrap_or_else(|e| panic!("{path} reads: {e}"));
            (path, text)
        })
        .collect()
}

/// The text of the one file that defines `fn {name}`, with its path.
#[allow(dead_code)] // not every spec that reads the sources asks for one file
pub fn defining(name: &str) -> (String, String) {
    let needle = format!("fn {name}(");
    let mut found: Vec<(String, String)> =
        driver_sources().into_iter().filter(|(_, text)| text.contains(&needle)).collect();
    assert_eq!(
        found.len(),
        1,
        "`fn {name}` should be defined in exactly one of the binary's files, and is in {:?}",
        found.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    found.pop().unwrap()
}
