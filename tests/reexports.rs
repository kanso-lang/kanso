use std::process::Command;

fn run(dir: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/reexports/{dir}"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

/// GAVEL 51, Clay 2026-08-17: "one module", and routes are not names. This
/// asked for `geo/list/order` until the fork that minted it was removed —
/// `order` is geo's own rename of list's `sort`, so that spelling reached
/// through geo's PRIVATE import under a name list does not have, and it only
/// resolved because geo's copy of list was a second instance.
///
/// The bare name is what a re-export puts on the surface today. The qualified
/// door name `geo/order` is the other half and does not exist yet — it is not
/// this change's regression (it never resolved, on this branch or on main),
/// and it has its own task.
#[test]
fn reexported_names_join_the_surface_bare() {
    let output = run("app");

    assert_eq!(String::from_utf8_lossy(&output.stdout), "[3 2]\n[1]\n[1 2 3]\n12.56636\n");
    assert!(output.status.success());
}

#[test]
fn dependency_pubs_stay_off_the_surface_without_a_reexport() {
    let output = run("sealed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown name `sum`"),
        "expected the unreexported name to stay sealed, got: {stderr}"
    );
    assert!(!output.status.success());
}

#[test]
fn a_bare_call_two_imports_answer_alike_is_refused() {
    let output = run("torn/use");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("a bare `strength` reaches `brew/strength` and `steep/strength` alike"),
        "expected the torn bare call to be refused, got: {stderr}"
    );
    assert!(!output.status.success());
}

/// The app fixture calls `geo/areas` beside its re-exported names, so the
/// import is marked used by a name geo declares and the re-export lines ride
/// along. This entry uses NOTHING but a re-exported one.
///
/// It went red the moment a re-exported name stopped carrying the route it
/// arrived by: `order` is geo's rename of list's `sort`, so the declaration is
/// spelled `list/order`, and the check that credits an import for a bare
/// spelling used to read the qualifier off the front of that name. It read
/// `list`, the app imports `geo`, and the import was called unused.
#[test]
fn a_re_exported_name_alone_keeps_its_import() {
    let output = run("bare");

    assert_eq!(String::from_utf8_lossy(&output.stdout), "[1 2 3]\n");
    assert!(output.status.success());
}

/// The qualified door. A re-exported name keeps the spelling its owner gave
/// it — `order` is geo's rename of list's `sort`, declared `list/order` — so
/// `geo/order` named nothing while bare `order` resolved, and every plain
/// re-export was the same. A module's own pub has both doors and so does a
/// re-exported one now.
///
/// `geo/areas` is in the fixture because it is geo's own function: the door
/// is opened only where the qualified spelling is free, and this proves the
/// pass leaves a real declaration alone.
#[test]
fn a_reexported_name_answers_to_the_module_that_surfaced_it() {
    let output = run("door");

    assert_eq!(String::from_utf8_lossy(&output.stdout), "[3 1 2]\n[1]\n[1 2 3]\n");
    assert!(output.status.success());
}

/// A facade: `mid` declares nothing and re-exports shape's types and its arms.
///
/// Four things had to hold for it, each watched red on its own. A module whose
/// only reason for an import is re-exporting it read as unused, and no
/// spelling would have satisfied the check. `mid/describe` named nothing from
/// inside another module, where the entry already reached it. `filled` arrived
/// twice — open through `mid`, sealed through `teller`'s route — and the later
/// arrival closed it, which is the visibility half of gavel 51 that types had
/// never been given.
///
/// The rest came from Copilot on the PR, and a door has to answer wherever a
/// type is named. `pub fn tell mid/blank` died as `unknown type
/// mid/shape/blank` on native while the oracle picked the wrong arm — a
/// divergence, so the differential law had it either way. `(f):mid/filled`
/// answered `` `:mid/filled` widens; this value is not a mid/filled ``. The
/// typeset needed the door in its member list.
///
/// It also needed the other half of gavel 51, and the two were isolated
/// against this one fixture. `own_types` in qualify was every name in the
/// merged program, including types that arrived through this module from its
/// own dependencies, where `owned` had filtered qualified names all along —
/// so a member already spelled `shape/blank` picked up a second prefix and
/// named `teller/shape/blank`, which nothing declares. Strip the filter and
/// this fixture says that; strip the member door and it says `no overload`.
#[test]
fn a_module_that_only_re_exports_is_a_module() {
    let output = run("facade/app");

    assert_eq!(String::from_utf8_lossy(&output.stdout), "tell filled 3\ntell nothing\n");
    assert!(output.status.success());
}
