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

/// A hako remote made on the spot: two releases and a prerelease, so the
/// version race has something to get wrong. Local, because install is the one
/// part of the toolchain that reaches a network and a test should not.
fn published(root: &Path) -> PathBuf {
    let repo = root.join("remote/acme/widgets");
    std::fs::create_dir_all(&repo).expect("the remote's directory");
    let git = |args: &[&str]| {
        let done = Command::new("git").args(args).current_dir(&repo).output().expect("git runs");
        assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
    };
    std::fs::write(repo.join("widgets.kso"), "pub fn greet _\n  \"widgets v1\"\n").expect("writes");
    git(&["init", "-q", "."]);
    git(&["add", "-A"]);
    git(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "one"]);
    git(&["tag", "v0.1.0"]);
    std::fs::write(repo.join("widgets.kso"), "pub fn greet _\n  \"widgets v2\"\n").expect("writes");
    git(&["add", "-A"]);
    git(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "two"]);
    git(&["tag", "v0.2.0"]);
    // neither is a release, and both would win the race if they counted: a
    // prerelease at a higher version, and a four-component tag above both
    git(&["tag", "v9.9.9-rc1"]);
    git(&["tag", "v9.9.9.1"]);
    // an unreleased branch: what a collaborator is working on, and the only
    // thing an interim pin can point at
    git(&["checkout", "-q", "-b", "fix-thing"]);
    std::fs::write(repo.join("widgets.kso"), "pub fn greet _\n  \"widgets unreleased\"\n")
        .expect("writes");
    git(&["add", "-A"]);
    git(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "three"]);
    git(&["checkout", "-q", "-"]);
    root.join("remote")
}

/// The whole flow: an import is the manifest, install resolves the highest
/// release and writes the lock, and the build then reads the cache with no
/// network at all. Watched red before install: the import cannot resolve.
#[test]
fn install_resolves_the_highest_release_and_the_build_reads_the_cache() {
    let root = std::env::temp_dir().join("kanso-hako-install");
    let _ = std::fs::remove_dir_all(&root);
    let remote = published(&root);
    let cache = root.join("cache");
    let app = root.join("app");
    std::fs::create_dir_all(&app).expect("the app's directory");
    std::fs::write(
        app.join("main.kso"),
        "import \"acme/widgets\"\n\nprint \"{widgets/greet 0}\"\n",
    )
    .expect("the entry writes");

    let before = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "."])
        .env("KANSO_HAKO", &cache)
        .current_dir(&app)
        .output()
        .expect("kanso runs");
    assert!(
        String::from_utf8_lossy(&before.stderr).contains("run `kanso install`"),
        "an unfetched hako built anyway: {}",
        String::from_utf8_lossy(&before.stderr)
    );

    let install = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["install", "."])
        .env("KANSO_HAKO", &cache)
        .env("KANSO_HAKO_REMOTE", format!("{}/", remote.display()))
        .current_dir(&app)
        .output()
        .expect("kanso install runs");
    assert!(
        install.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let lock = std::fs::read_to_string(app.join("hako.lock")).expect("the lock is written");
    let fields: Vec<&str> = lock.split_whitespace().collect();
    assert_eq!(fields[0], "acme/widgets");
    assert_eq!(fields[1], "v0.2.0", "a prerelease or an older tag won the race: {lock}");
    assert_eq!(fields[2].len(), 40, "the lock records no commit sha: {lock}");

    // no remote in the environment: the build must be satisfied by the cache
    let after = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "."])
        .env("KANSO_HAKO", &cache)
        .current_dir(&app)
        .output()
        .expect("kanso runs");
    assert_eq!(
        String::from_utf8_lossy(&after.stdout),
        "widgets v2\n",
        "{}",
        String::from_utf8_lossy(&after.stderr)
    );
}

/// The rest of the cycle: a release lands upstream, `list` says so, `update`
/// walks the lock forward, and the build follows — still without a network,
/// because the cache holds what the lock names.
#[test]
fn list_marks_staleness_and_update_walks_the_lock_forward() {
    let root = std::env::temp_dir().join("kanso-hako-update");
    let _ = std::fs::remove_dir_all(&root);
    let remote = published(&root);
    let cache = root.join("cache");
    let app = root.join("app");
    std::fs::create_dir_all(&app).expect("the app's directory");
    std::fs::write(
        app.join("main.kso"),
        "import \"acme/widgets\"\n\nprint \"{widgets/greet 0}\"\n",
    )
    .expect("the entry writes");
    let base = format!("{}/", remote.display());
    let hako = |args: &[&str], with_remote: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
        command.args(args).env("KANSO_HAKO", &cache).current_dir(&app);
        if with_remote {
            command.env("KANSO_HAKO_REMOTE", &base);
        } else {
            command.env("KANSO_HAKO_REMOTE", "/nonexistent-remote/");
        }
        command.output().expect("kanso runs")
    };

    assert!(hako(&["install", "."], true).status.success());
    let fresh = String::from_utf8_lossy(&hako(&["list", "."], true).stdout).into_owned();
    assert!(fresh.contains("v0.2.0"), "{fresh}");
    assert!(!fresh.contains("stale"), "nothing is out yet: {fresh}");

    // a release lands upstream
    let repo = remote.join("acme/widgets");
    std::fs::write(repo.join("widgets.kso"), "pub fn greet _\n  \"widgets v3\"\n").expect("writes");
    for args in [
        &["add", "-A"][..],
        &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "three"][..],
        &["tag", "v0.3.0"][..],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&repo)
            .output()
            .expect("git runs")
            .status
            .success());
    }

    let marked = String::from_utf8_lossy(&hako(&["list", "."], true).stdout).into_owned();
    assert!(marked.contains("stale: v0.3.0 is out"), "list did not mark it: {marked}");

    assert!(hako(&["update", "."], true).status.success());
    let lock = std::fs::read_to_string(app.join("hako.lock")).expect("the lock reads");
    assert!(lock.contains("v0.3.0"), "the lock did not move: {lock}");

    // no remote: the build must still be satisfied by what update fetched
    let built = hako(&["run", "."], false);
    assert_eq!(
        String::from_utf8_lossy(&built.stdout),
        "widgets v3\n",
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    // and listing works with nothing to ask
    let offline = String::from_utf8_lossy(&hako(&["list", "."], false).stdout).into_owned();
    assert!(offline.contains("unreachable"), "list should still answer: {offline}");
}

/// The protocol says how content is obtained from wherever the name lives.
/// It is derived by convention, written into the lock, and refused by name
/// when this toolchain cannot speak it — so a lock from a newer toolchain
/// says what it wanted rather than quietly being fetched some other way.
#[test]
fn the_lock_records_a_protocol_and_refuses_one_it_cannot_speak() {
    let root = std::env::temp_dir().join("kanso-hako-protocol");
    let _ = std::fs::remove_dir_all(&root);
    let remote = published(&root);
    let cache = root.join("cache");
    let app = root.join("app");
    std::fs::create_dir_all(&app).expect("the app's directory");
    std::fs::write(
        app.join("main.kso"),
        "import \"acme/widgets\"\n\nprint \"{widgets/greet 0}\"\n",
    )
    .expect("the entry writes");
    let hako = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(args)
            .env("KANSO_HAKO", &cache)
            .env("KANSO_HAKO_REMOTE", format!("{}/", remote.display()))
            .current_dir(&app)
            .output()
            .expect("kanso runs")
    };

    assert!(hako(&["install", "."]).status.success());
    let lock = std::fs::read_to_string(app.join("hako.lock")).expect("the lock reads");
    let fields: Vec<&str> = lock.split_whitespace().collect();
    assert_eq!(fields.len(), 4, "the lock records no protocol: {lock}");
    assert_eq!(fields[3], "git");

    // a line written before the field existed still means git
    std::fs::write(app.join("hako.lock"), format!("{} {} {}\n", fields[0], fields[1], fields[2]))
        .expect("the lock writes");
    let old = hako(&["run", "."]);
    assert_eq!(
        String::from_utf8_lossy(&old.stdout),
        "widgets v2\n",
        "a three-field lock stopped working: {}",
        String::from_utf8_lossy(&old.stderr)
    );

    // and one this toolchain cannot speak is refused by name
    std::fs::write(
        app.join("hako.lock"),
        format!("{} {} {} hako_server\n", fields[0], fields[1], fields[2]),
    )
    .expect("the lock writes");
    let refused = hako(&["update", "."]);
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("cannot speak"),
        "an unspeakable protocol was not refused: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
}

/// The dev-sha discipline, made structural. You can build against a
/// collaborator's unreleased branch, and every later step refuses to let the
/// pin go quiet: the lock records the branch rather than a version, `list`
/// says the word interim, `update` walks tags and leaves it alone, and a
/// plain `install` does not silently replace it with a release.
#[test]
fn an_interim_pin_is_built_against_flagged_and_never_walked_forward() {
    let root = std::env::temp_dir().join("kanso-hako-interim");
    let _ = std::fs::remove_dir_all(&root);
    let remote = published(&root);
    let cache = root.join("cache");
    let app = root.join("app");
    std::fs::create_dir_all(&app).expect("the app's directory");
    std::fs::write(
        app.join("main.kso"),
        "import \"acme/widgets\"\n\nprint \"{widgets/greet 0}\"\n",
    )
    .expect("the entry writes");
    let kanso = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(args)
            .env("KANSO_HAKO", &cache)
            .env("KANSO_HAKO_REMOTE", format!("{}/", remote.display()))
            .current_dir(&app)
            .output()
            .expect("kanso runs")
    };

    let install = kanso(&["install", ".", "--from", "acme/widgets@fix-thing"]);
    assert!(
        install.status.success(),
        "install --from failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    let lock = std::fs::read_to_string(app.join("hako.lock")).expect("the lock is written");
    let fields: Vec<&str> = lock.split_whitespace().collect();
    assert_eq!(fields[1], "fix-thing", "the lock did not record the branch: {lock}");
    assert_eq!(fields[2].len(), 40, "the lock records no commit sha: {lock}");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "."])
        .env("KANSO_HAKO", &cache)
        .current_dir(&app)
        .output()
        .expect("kanso runs");
    assert_eq!(
        String::from_utf8_lossy(&built.stdout),
        "widgets unreleased\n",
        "the build did not read the pinned branch: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let listed = String::from_utf8_lossy(&kanso(&["list", "."]).stdout).to_string();
    assert!(listed.contains("interim"), "list does not flag the pin: {listed}");

    let updated = kanso(&["update", "."]);
    let said = String::from_utf8_lossy(&updated.stdout);
    assert!(
        updated.status.success(),
        "update failed: {}",
        String::from_utf8_lossy(&updated.stderr)
    );
    let after = std::fs::read_to_string(app.join("hako.lock")).expect("the lock survives");
    assert_eq!(after, lock, "update walked an interim pin forward: {said}{after}");

    let named = kanso(&["update", ".", "acme/widgets"]);
    assert!(
        String::from_utf8_lossy(&named.stderr).contains("which is not a release"),
        "naming an interim pin to update did not say why it cannot: {}",
        String::from_utf8_lossy(&named.stderr)
    );

    let again = kanso(&["install", "."]);
    assert!(again.status.success(), "install failed: {}", String::from_utf8_lossy(&again.stderr));
    let kept = std::fs::read_to_string(app.join("hako.lock")).expect("the lock survives");
    assert_eq!(kept, lock, "a plain install replaced the interim pin: {kept}");
}

/// hako's first verb, written in kanso. `hako/` is a module in this repo that
/// reads a lock and reports what it pins — the same job `hako::list` does in
/// Rust, and the beginning of moving that work into the language it manages.
/// It runs here so it is exercised rather than merely present.
///
/// The remote is a directory of repositories, so this needs no network and
/// still exercises the `git ls-remote` that staleness is measured with.
#[test]
fn the_kanso_listing_reads_a_lock() {
    let root = std::env::temp_dir().join("kanso-hako-kanso-list");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a directory of its own");
    let remote = published(&root);
    std::fs::write(
        root.join("hako.lock"),
        "acme/widgets v0.1.0 abc123def4567890 git\nb/c fix-thing 99aabbccddeeff00 git\n",
    )
    .expect("the lock writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "hako", "--"])
        .arg(&root)
        .env("KANSO_HAKO_REMOTE", format!("{}/", remote.display()))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");

    assert_eq!(
        String::from_utf8_lossy(&done.stdout),
        "acme/widgets v0.1.0 abc123def456 git (stale: v0.2.0 is out)\n\
         b/c fix-thing 99aabbccddee git (interim pin: not a release)\n",
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// A pin at the release the remote publishes carries no mark at all.
#[test]
fn the_kanso_listing_is_quiet_when_a_pin_is_current() {
    let root = std::env::temp_dir().join("kanso-hako-kanso-current");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a directory of its own");
    let remote = published(&root);
    std::fs::write(root.join("hako.lock"), "acme/widgets v0.2.0 abc123def4567890 git\n")
        .expect("the lock writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "hako", "--"])
        .arg(&root)
        .env("KANSO_HAKO_REMOTE", format!("{}/", remote.display()))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");

    assert_eq!(
        String::from_utf8_lossy(&done.stdout),
        "acme/widgets v0.2.0 abc123def456 git\n",
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// A remote nobody can reach is reported, never guessed at.
#[test]
fn the_kanso_listing_says_when_a_remote_does_not_answer() {
    let root = std::env::temp_dir().join("kanso-hako-kanso-unreachable");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a directory of its own");
    std::fs::write(root.join("hako.lock"), "acme/widgets v0.1.0 abc123def4567890 git\n")
        .expect("the lock writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "hako", "--"])
        .arg(&root)
        .env("KANSO_HAKO_REMOTE", "/nonexistent-remote/")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");

    assert_eq!(
        String::from_utf8_lossy(&done.stdout),
        "acme/widgets v0.1.0 abc123def456 git (unreachable)\n",
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// An empty lock says what to do about it.
#[test]
fn the_kanso_listing_says_when_nothing_is_pinned() {
    let root = std::env::temp_dir().join("kanso-hako-kanso-empty");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a directory of its own");
    std::fs::write(root.join("hako.lock"), "").expect("the lock writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "hako", "--"])
        .arg(&root)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");

    assert_eq!(
        String::from_utf8_lossy(&done.stdout),
        "hako.lock pins nothing — run `kanso install`\n",
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
}
