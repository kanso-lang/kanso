//! hako — packages, per design/hako.md.
//!
//! Imports are the manifest: `install` reads them, resolves each hako's
//! highest release tag, fetches it into the cache and writes `hako.lock`.
//! Source never names a version and the compiler never reaches the network;
//! the lock is the only place a version is written down, and the cache is the
//! only place the compiler looks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where a hako name ends and a module path inside it begins. A name is
/// `owner/repo` with an optional `/vN` major fork; everything after that
/// addresses a module within the hako.
pub fn split_name(path: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[0].contains('.') {
        return None;
    }
    let major = parts.get(2).is_some_and(|s| is_major(s));
    let cut = match major {
        true => 3,
        false => 2,
    };
    let name = parts[..cut].join("/");
    let module = match parts.len() > cut {
        true => Some(parts[cut..].join("/")),
        false => None,
    };
    Some((name, module))
}

fn is_major(segment: &str) -> bool {
    segment
        .strip_prefix('v')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// A release tag, ordered. Anything that is not `vX.Y.Z` is not a release —
/// branches are interim pins and never win a version race.
fn release(tag: &str) -> Option<(u64, u64, u64)> {
    let mut parts = tag.strip_prefix('v')?.split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let version = (next()?, next()?, next()?);
    match parts.next() {
        Some(_) => None,
        None => Some(version),
    }
}

/// The lock, read back. Name to (tag, sha) — the two facts that make a build
/// reproducible, and the reason no import ever names a version.
pub fn read_lock(root: &Path) -> BTreeMap<String, (String, String)> {
    let mut locked = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(root.join("hako.lock")) else {
        return locked;
    };
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let [name, tag, sha] = fields[..] {
            locked.insert(name.to_string(), (tag.to_string(), sha.to_string()));
        }
    }
    locked
}

/// Every hako named by an import anywhere under `root`. Reading the sources
/// directly rather than compiling them, because install is what makes
/// compiling possible.
fn required(root: &Path) -> Result<Vec<String>, String> {
    let mut names = std::collections::BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|x| x == "kso") {
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                for line in source.lines() {
                    let Some(rest) = line.strip_prefix("import \"") else { continue };
                    let Some(spec) = rest.split('"').next() else { continue };
                    if spec.starts_with("std/") || spec.starts_with('.') {
                        continue;
                    }
                    if let Some((name, _)) = split_name(spec) {
                        names.insert(name);
                    }
                }
            }
        }
    }
    Ok(names.into_iter().collect())
}

/// Where a hako is fetched from. The name's shape is the declaration —
/// `owner/repo` means GitHub, by convention and without a lookup. The base is
/// configurable because transport is user and CI configuration, never source.
fn remote(name: &str) -> String {
    let base =
        std::env::var("KANSO_HAKO_REMOTE").unwrap_or_else(|_| "https://github.com/".to_string());
    let repo = name.split('/').take(2).collect::<Vec<_>>().join("/");
    format!("{base}{repo}")
}

fn highest_release(name: &str) -> Result<(String, String), String> {
    let url = remote(name);
    let listing = Command::new("git")
        .args(["ls-remote", "--tags", &url])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !listing.status.success() {
        return Err(format!(
            "cannot reach `{name}` at {url}: {}",
            String::from_utf8_lossy(&listing.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&listing.stdout);
    let mut best: Option<((u64, u64, u64), String, String)> = None;
    for line in text.lines() {
        let Some((sha, reference)) = line.split_once('\t') else { continue };
        let Some(tag) = reference.strip_prefix("refs/tags/") else { continue };
        let tag = tag.strip_suffix("^{}").unwrap_or(tag);
        let Some(version) = release(tag) else { continue };
        if best.as_ref().is_none_or(|(seen, _, _)| version > *seen) {
            best = Some((version, tag.to_string(), sha.to_string()));
        }
    }
    match best {
        Some((_, tag, sha)) => Ok((tag, sha)),
        None => Err(format!("`{name}` publishes no vX.Y.Z release tag at {url}")),
    }
}

/// The cache is keyed by name and sha, so two versions coexist and a fetched
/// tree is never mistaken for a different one.
pub fn cached(cache: &Path, name: &str, sha: &str) -> PathBuf {
    cache.join(format!("{name}@{sha}"))
}

fn fetch(cache: &Path, name: &str, tag: &str, sha: &str) -> Result<(), String> {
    let into = cached(cache, name, sha);
    if into.is_dir() {
        return Ok(());
    }
    if let Some(parent) = into.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    let url = remote(name);
    let clone = Command::new("git")
        .args(["clone", "--quiet", "--depth", "1", "--branch", tag, &url])
        .arg(&into)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !clone.status.success() {
        return Err(format!(
            "cannot fetch `{name}` {tag}: {}",
            String::from_utf8_lossy(&clone.stderr).trim()
        ));
    }
    let _ = std::fs::remove_dir_all(into.join(".git"));
    Ok(())
}

/// `kanso install` — resolve every imported hako, fetch it, write the lock.
pub fn install(root: &Path, cache: &Path) -> Result<String, String> {
    let names = required(root)?;
    if names.is_empty() {
        return Ok("no hakos are imported — nothing to install\n".to_string());
    }
    let mut lines = Vec::new();
    let mut report = String::new();
    for name in &names {
        let (tag, sha) = highest_release(name)?;
        fetch(cache, name, &tag, &sha)?;
        report.push_str(&format!("  {name} {tag}\n"));
        lines.push(format!("{name} {tag} {sha}\n"));
    }
    std::fs::write(root.join("hako.lock"), lines.concat())
        .map_err(|e| format!("cannot write hako.lock: {e}"))?;
    Ok(format!("installed {} hako(s):\n{report}", names.len()))
}

/// `kanso list` — what the lock pins. Staleness needs the remote, so it is
/// reported when one answers and left off when none does: listing what a
/// build will use has to work on a train.
pub fn list(root: &Path) -> Result<String, String> {
    let locked = read_lock(root);
    if locked.is_empty() {
        return Ok("hako.lock pins nothing — run `kanso install`\n".to_string());
    }
    let mut out = String::new();
    for (name, (tag, sha)) in &locked {
        let mark = match highest_release(name) {
            Ok((latest, _)) if latest != *tag => format!(" (stale: {latest} is out)"),
            Ok(_) => String::new(),
            Err(_) => " (unreachable)".to_string(),
        };
        out.push_str(&format!("{name} {tag} {}{mark}\n", &sha[..sha.len().min(12)]));
    }
    Ok(out)
}

/// `kanso update` — walk tags forward and rewrite the lock. A release is the
/// only thing it walks to; a pin that is not a release stays where it is,
/// which is the dev-sha discipline made structural.
pub fn update(root: &Path, cache: &Path, only: Option<&str>) -> Result<String, String> {
    let locked = read_lock(root);
    if locked.is_empty() {
        return Err("hako.lock pins nothing — run `kanso install` first".to_string());
    }
    if let Some(name) = only {
        if !locked.contains_key(name) {
            return Err(format!("`{name}` is not in hako.lock"));
        }
    }
    let mut lines = Vec::new();
    let mut moved = String::new();
    for (name, (tag, sha)) in &locked {
        let chosen = match only.is_none_or(|one| one == name) {
            false => (tag.clone(), sha.clone()),
            true => {
                let (latest, at) = highest_release(name)?;
                if latest != *tag {
                    fetch(cache, name, &latest, &at)?;
                    moved.push_str(&format!("  {name} {tag} -> {latest}\n"));
                }
                (latest, at)
            }
        };
        lines.push(format!("{name} {} {}\n", chosen.0, chosen.1));
    }
    std::fs::write(root.join("hako.lock"), lines.concat())
        .map_err(|e| format!("cannot write hako.lock: {e}"))?;
    match moved.is_empty() {
        true => Ok("every hako is already at its highest release\n".to_string()),
        false => Ok(format!("updated:\n{moved}")),
    }
}
