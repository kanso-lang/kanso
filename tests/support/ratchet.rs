/// The ratchet's module, every file of it in name order, as one text. Rows
/// live in the `rows_*.kso` files beside the runner in `ratchet.kso`, so a
/// check over the rows reads them all.
pub fn ratchet_source() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ratchet/ratchet");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("the ratchet module reads")
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "kso"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| std::fs::read_to_string(p).expect("a ratchet file reads"))
        .collect::<Vec<_>>()
        .join("\n")
}
