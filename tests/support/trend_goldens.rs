/// The goldens the trend gate walks, each with the prefix its fields take,
/// read the way scripts/trend_gate/trend_gate.kso reads them: every file in
/// bench/ whose name holds `golden` and ends `.txt`, less the gate's
/// `not_counters`, with the prefix from its `prefixed` row or none.
pub fn walked() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");
    let kept_out = rows(&gate, "not_counters = [");
    let prefixes = prefixed(&gate);
    let mut names: Vec<String> = std::fs::read_dir(root.join("bench"))
        .expect("the bench directory reads")
        .map(|e| e.expect("a directory entry reads").file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("golden") && n.ends_with(".txt"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| format!("bench/{n}"))
        .filter(|path| !kept_out.contains(path))
        .map(|path| {
            let prefix = prefixes.iter().find(|(p, _)| *p == path).map(|(_, x)| x.clone());
            (path, prefix.unwrap_or_default())
        })
        .collect()
}

/// The gate's `prefixed` rows, as each golden's path and its prefix.
pub fn prefixed(gate: &str) -> Vec<(String, String)> {
    let start = gate.find("prefixed = [").expect("the trend gate has a `prefixed` list");
    let body = &gate[start..];
    let end = body.find("\n]").expect("the `prefixed` list closes");
    body[..end]
        .lines()
        .filter_map(|l| {
            let fields: Vec<&str> = l.split('"').skip(1).step_by(2).collect();
            match fields.as_slice() {
                [path, prefix, ..] if path.starts_with("bench/") => {
                    Some((path.to_string(), prefix.to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

/// The strings of a one-line list the gate opens with `head`.
pub fn rows(gate: &str, head: &str) -> Vec<String> {
    let at = gate.find(head).unwrap_or_else(|| panic!("the trend gate has `{head}`"));
    let line = gate[at..].lines().next().unwrap_or_default();
    line.split('"').skip(1).step_by(2).map(str::to_string).collect()
}
