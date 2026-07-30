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

/// How content is obtained from wherever a name lives: `git`, and later `hg`,
/// `svn` or a hako server. A host is not a protocol — GitHub and GitLab both
/// speak git, and tag listing and ref fetch are git standards, so neither
/// earns a name here. GitHub's one extra ability, a tarball at a ref, trades
/// a commit sha for a vendor's promise of byte-stability, which the January
/// 2023 archive-compression change is the reason not to take.
///
/// Convention answers for every shape resolvable today, so nothing is asked
/// over the network. A domain-shaped name is where a lookup would go.
fn protocol_of(name: &str) -> Result<&'static str, String> {
    match name.split('/').next().unwrap_or_default().contains('.') {
        false => Ok("git"),
        true => Err(format!(
            "`{name}` names a hako by domain, and asking a domain which protocol \
             it speaks is not built yet"
        )),
    }
}

/// The one protocol with an adapter. An unspeakable protocol is refused by
/// name rather than attempted as git, so a lock from a newer toolchain says
/// what it wanted instead of quietly doing something else.
fn speakable(name: &str, protocol: &str) -> Result<(), String> {
    match protocol {
        "git" => Ok(()),
        other => Err(format!(
            "`{name}` is locked to the `{other}` protocol, which this \
                              toolchain cannot speak"
        )),
    }
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

/// A branch's current head. Interim by construction: a branch moves, so the
/// sha beside it in the lock is what makes the build reproducible, and what
/// makes re-pinning a deliberate act rather than a drift.
fn branch_head(name: &str, branch: &str) -> Result<String, String> {
    let url = remote(name);
    let listing = Command::new("git")
        .args(["ls-remote", "--heads", &url, branch])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !listing.status.success() {
        return Err(format!(
            "cannot reach `{name}` at {url}: {}",
            String::from_utf8_lossy(&listing.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&listing.stdout);
    match text.lines().next().and_then(|line| line.split_once('\t')) {
        Some((sha, _)) => Ok(sha.to_string()),
        None => Err(format!("`{name}` has no branch `{branch}` at {url}")),
    }
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
///
/// `--from owner/repo@branch` pins one hako to an unreleased branch. The lock
/// is the only home an override has: an import names identity and nothing
/// else, so trying a collaborator's branch edits a lock line rather than the
/// source that would then have to be edited back before release.
pub fn install(root: &Path, cache: &Path, from: &[String]) -> Result<String, String> {
    let names = required(root)?;
    if names.is_empty() {
        return Ok("no hakos are imported — nothing to install\n".to_string());
    }
    let mut asked = BTreeMap::new();
    for spec in from {
        let (name, reference) = override_pin(spec)?;
        if !names.contains(&name) {
            return Err(format!(
                "nothing imports `{name}`, so pinning it would pin nothing — \
                 imports are the manifest"
            ));
        }
        asked.insert(name, reference);
    }
    // An interim pin already in the lock survives a plain install: it is a
    // decision somebody made, and replacing it with a release would change
    // what the build compiles without anybody asking for that.
    let locked = read_lock(root);
    let mut lines = Vec::new();
    let mut report = String::new();
    for name in &names {
        let protocol = protocol_of(name)?;
        speakable(name, protocol)?;
        let held = locked.get(name).filter(|pin| interim(pin));
        let (tag, sha) = match (asked.get(name), held) {
            (Some(reference), _) => (reference.clone(), branch_head(name, reference)?),
            (None, Some(pin)) => (pin.tag.clone(), pin.sha.clone()),
            (None, None) => highest_release(name)?,
        };
        fetch(cache, name, &tag, &sha)?;
        let mark = match release(&tag) {
            Some(_) => "",
            None => " (interim pin)",
        };
        report.push_str(&format!("  {name} {tag}{mark}\n"));
        lines.push(format!("{name} {tag} {sha} {protocol}\n"));
    }
    std::fs::write(root.join("hako.lock"), lines.concat())
        .map_err(|e| format!("cannot write hako.lock: {e}"))?;
    Ok(format!("installed {} hako(s):\n{report}", names.len()))
}
