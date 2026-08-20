//! Gavel 15 gives the wall a lazy right side, and gavel 20's guardedness says a
//! name in a storing position is stored rather than demanded. A constructor
//! field already proves the bare case — `ring = cell ring` ties — so a bare
//! name in a wall's right slot ties too.
//!
//! The pin is acceptance rather than output, because the shape this licenses is
//! the control loop: `loop = print "tick" >> loop` describes an endless
//! program, which is what makes it the idiom gavel 15 was ratified to enable.
//! There is no terminating spelling of a constant that names itself through a
//! wall, so there is nothing to print.

use std::process::Command;

fn checks(source: &str) -> (bool, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kanso-wall-store-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to check in");
    let file = dir.join("pkg.kso");
    std::fs::write(&file, source).expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("kanso runs");

    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (done.status.success(), said)
}

/// Watched red: before the wall was admitted as a storing position this said
/// "`loop` is defined in terms of itself, so it has no value".
#[test]
fn a_bare_name_after_a_wall_is_stored() {
    let (ok, said) = checks("pub loop = print \"tick\" >> loop\n");

    assert!(ok, "a constant naming itself after a wall was refused: {said}");
}

/// The sibling that already worked, kept beside it so the two storing
/// positions are read together rather than one at a time.
#[test]
fn a_bare_name_in_a_field_is_stored() {
    let (ok, said) = checks("type cell\n  slot\n\npub ring = cell ring\n");

    assert!(ok, "a constant naming itself in a field was refused: {said}");
}

/// The left side still runs, so a name demanded there has no value and the
/// refusal stands. Without this the fix would read as "walls are exempt".
#[test]
fn a_bare_name_before_a_wall_is_still_demanded() {
    let (ok, said) = checks("pub spin = spin >> print \"tick\"\n");

    assert!(!ok, "a constant demanding itself before a wall was accepted");
    assert!(said.contains("defined in terms of itself"), "refused for the wrong reason: {said}");
}
