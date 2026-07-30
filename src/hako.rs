//! hako — packages, per design/hako.md.
//!
//! Imports are the manifest: `install` reads them, resolves each hako's
//! highest release tag, fetches it into the cache and writes `hako.lock`.
//! Source never names a version and the compiler never reaches the network;
//! the lock is the only place a version is written down, and the cache is the
//! only place the compiler looks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

/// Whether a pin is interim: it names something that is not a release, so it
/// is somebody's unmerged branch, and every later step has to keep saying so.
pub fn interim(pin: &Pin) -> bool {
    release(&pin.tag).is_none()
}

/// What a lock line says: the tag and sha that make a build reproducible, and
/// the protocol that says how to speak to wherever the name lives.
#[derive(Clone)]
pub struct Pin {
    pub tag: String,
    pub sha: String,
    pub protocol: String,
}

/// The lock, read back. A line without a protocol reads as git, which is what
/// every line written before the field existed meant.
pub fn read_lock(root: &Path) -> BTreeMap<String, Pin> {
    let mut locked = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(root.join("hako.lock")) else {
        return locked;
    };
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (name, tag, sha, protocol) = match fields[..] {
            [name, tag, sha] => (name, tag, sha, "git"),
            [name, tag, sha, protocol] => (name, tag, sha, protocol),
            _ => continue,
        };
        let pin =
            Pin { tag: tag.to_string(), sha: sha.to_string(), protocol: protocol.to_string() };
        locked.insert(name.to_string(), pin);
    }
    locked
}

/// `--from owner/repo@branch` — the spelling of an interim pin. A name and a
/// ref, because the name is permanent identity and the ref is where content
/// comes from this once; an import never learns either.
pub fn override_pin(spec: &str) -> Result<(String, String), String> {
    match spec.split_once('@') {
        Some((name, reference)) if !name.is_empty() && !reference.is_empty() => {
            Ok((name.to_string(), reference.to_string()))
        }
        _ => Err(format!("`{spec}` is not `owner/repo@branch`")),
    }
}

/// The cache is keyed by name and sha, so two versions coexist and a fetched
/// tree is never mistaken for a different one.
pub fn cached(cache: &Path, name: &str, sha: &str) -> PathBuf {
    cache.join(format!("{name}@{sha}"))
}
