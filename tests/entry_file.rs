//! What an entry file may hold.
//!
//! A directory's main.kso is the entry: statements, run top to bottom. A `fn`
//! or a `type` in it belongs in a library file beside it, and saying so is the
//! difference between a reader moving the declaration and a reader wondering
//! why their program does nothing.
//!
//! This is a test rather than a golden because the rule needs a directory to
//! reach, and the error corpus holds single files. The coverage gate counts an
//! assertion here as pinning the message, which is what keeps the diagnostic
//! from being reworded or lost.

use std::process::Command;

fn run(engine: &[&str]) -> (String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("tests/golden/entryfile/defs_in_the_entry")
        .args(engine)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.status.code())
}

/// Both engines refuse it, and refuse it in the same words: the file is parsed
/// before either of them is chosen, so a difference here would mean the
/// parsers had drifted apart.
#[test]
fn a_definition_in_the_entry_file_is_refused_by_both_engines() {
    // one line, deliberately: the coverage gate looks for the message as a
    // contiguous string, and a wrapped literal hides it the same way the
    // compiler's own wrapped literal hides the rest of the sentence
    let said =
        "error[syntax]: an entry file holds statements only; definitions live in library files";

    let (native, native_code) = run(&[]);
    let (interp, interp_code) = run(&["--interp"]);

    assert_eq!(native.lines().next(), Some(said));
    assert_eq!(interp.lines().next(), Some(said));
    assert_eq!(native_code, Some(2));
    assert_eq!(interp_code, Some(2));
}

/// A runner beside the file it runs, which is the shape that lets a sample be
/// an ordinary library and still be one command away. `play` here is a
/// constant like any other: the compiler is told nothing, the runner names it.
#[test]
fn a_bare_import_reads_the_kso_file_beside_it() {
    let fixture = "tests/golden/entryfile/a_runner_beside_what_it_runs";
    let answer = |engine: &[&str]| {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(fixture)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (native, complaint) = answer(&[]);
    let (interp, _) = answer(&["--interp"]);

    assert_eq!(complaint, "", "native refused the sibling import");
    assert_eq!(native, "hello, kanso!\n");
    assert_eq!(interp, native, "the engines disagree on a sibling import");
}

/// Qualifying a module renames its types, and a subtype names its parent by
/// that name. Run the module directly and the two spellings agree because
/// neither is qualified; import it and they must still agree.
#[test]
fn a_subtype_keeps_its_parent_across_an_import() {
    let fixture = "tests/golden/entryfile/a_subtype_across_the_import";
    let answer = |engine: &[&str]| {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(fixture)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (native, complaint) = answer(&[]);
    let (interp, _) = answer(&["--interp"]);

    assert_eq!(complaint, "", "native refused a subtype reached through an import");
    assert_eq!(native, "woof\nsome animal\n");
    assert_eq!(interp, native, "the engines disagree on an imported subtype");
}

/// `&f` supplies some of a function's arguments and holds the rest. The name
/// it holds is a declaration of the module it sits in, so qualifying that
/// module has to move it the way it moves an ordinary mention.
#[test]
fn currying_survives_qualification() {
    // The FILE, not the directory: running the directory merges both files
    // into one module, which is the shape where no qualification happens and
    // the bug this pins cannot appear.
    let fixture = "tests/golden/entryfile/currying_across_the_import/main.kso";
    let answer = |engine: &[&str]| {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(fixture)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (native, complaint) = answer(&[]);
    let (interp, _) = answer(&["--interp"]);

    assert_eq!(complaint, "", "native refused a curried arm reached through an import");
    assert_eq!(native, "6\n30\n");
    assert_eq!(interp, native, "the engines disagree on currying across an import");
}

/// A typeset's membership is a list of type names this module declares, and
/// qualification renames those. The list has to move with them or the set
/// matches nothing and an err walks past the arm written to catch it.
#[test]
fn a_typeset_keeps_its_members_across_an_import() {
    let fixture = "tests/golden/entryfile/a_typeset_across_the_import/main.kso";
    let answer = |engine: &[&str]| {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(fixture)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (native, complaint) = answer(&[]);
    let (interp, _) = answer(&["--interp"]);

    assert_eq!(complaint, "", "native refused a typeset reached through an import");
    assert_eq!(native, "quota: limit 99\nlane trouble: quota/slow_lane 7\nok: plain 3\n");
    assert_eq!(interp, native, "the engines disagree on an imported typeset");
}

/// An err's reason renders by the name its declaring module gave it.
/// Qualification renames that declaration to keep it unique across a merge,
/// and the qualified spelling is the compiler's bookkeeping — a reader never
/// wrote `lane/slow_lane` and should never be shown it. Run the module
/// directly and the import must not change what the program prints.
///
/// Ignored because it fails, and because the obvious fix is wrong. Stripping
/// the qualifier at render breaks two tests that pin it on purpose:
/// cross_module_fields asserts a diagnostic naming `geo/label`, and asserts
/// `lib/pair 6 "v"` as rendered output. Both are right for an IMPORTED type —
/// `lib/pair` is what that program wrote. The bug is only for a module's OWN
/// type, which it wrote bare and never qualified.
///
/// So render would have to know which module is asking, which it does not, and
/// "what does a record print as" is a language-surface question rather than a
/// compiler-internal one. See design/compiler-log.md.
#[test]
#[ignore = "the two conventions collide; the rule is a gavel, not a fix"]
fn an_err_reason_renders_unqualified_across_an_import() {
    let dir = "tests/golden/entryfile/an_err_reason_across_the_import";
    let answer = |target: &str, engine: &[&str]| {
        let verb = if target.ends_with("_alone.kso") { "play" } else { "run" };
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg(verb)
            .arg(target)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (direct, _) = answer(&format!("{dir}/lane_alone.kso"), &[]);
    let (imported, complaint) = answer(&format!("{dir}/main.kso"), &[]);
    let (interp, _) = answer(&format!("{dir}/main.kso"), &["--interp"]);

    assert_eq!(complaint, "", "native refused an err reason reached through an import");
    assert_eq!(direct, "trouble: slow_lane 7\n");
    assert_eq!(imported, direct, "the import changed how the reason renders");
    assert_eq!(interp, imported, "the engines disagree on an imported err reason");
}

/// A module may claim rendering for a type it owns, and importing that module
/// must not silently take the claim away. `fn to_string _:money` joins the
/// ambient render group, which belongs to nobody — so the group's name is not
/// the importing module's to qualify, the same way a getter's is not.
///
/// The failure this pins had no diagnostic at all: the arm was renamed to a
/// group nothing declares, dispatch found only the builtin arms, and the value
/// rendered as the integer underneath. A user reads `350` where they wrote
/// `$3.50` and nothing anywhere says why.
///
/// The second line is the other half of the rule. `plain_tag` has no arm, so
/// it must still render as the integer underneath — a fix that made every
/// imported value find some arm would pass the first assertion and be wrong.
#[test]
fn a_modules_render_arm_survives_being_imported() {
    let dir = "tests/golden/entryfile/a_render_arm_across_the_import";
    let answer = |target: &str, engine: &[&str]| {
        let verb = if target.ends_with("_alone.kso") { "play" } else { "run" };
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg(verb)
            .arg(target)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (direct, _) = answer(&format!("{dir}/coin_alone.kso"), &[]);
    let (imported, complaint) = answer(&format!("{dir}/main.kso"), &[]);
    let (interp, _) = answer(&format!("{dir}/main.kso"), &["--interp"]);

    assert_eq!(complaint, "", "native refused a render arm reached through an import");
    assert_eq!(direct, "custom: $3.50\ndefault: 7\n");
    assert_eq!(imported, direct, "the import took the module's own render arm away");
    assert_eq!(interp, imported, "the engines disagree on an imported render arm");
}

/// `render/to_string` survived qualification because one line named it. The
/// comparison groups that landed after it did not, so a module's `fn <` was
/// renamed to `shelf/<` while `a < b` went on asking for the bare group,
/// dispatch found only the builtin arms, and two books had no order at all.
///
/// The imported case is the only case the gavel was about — a book record's
/// owner is a module you import — so this ran correctly as an entry file and
/// refused the moment anybody used it as a library.
///
/// The second line is the other half of the rule, as the render fixture has
/// it: ints keep the builtin. A fix that let an imported arm answer for every
/// value would pass the first assertion and be wrong.
#[test]
fn a_modules_ordering_arm_survives_being_imported() {
    let dir = "tests/golden/entryfile/an_ordering_arm_across_the_import";
    let answer = |target: &str, engine: &[&str]| {
        let verb = if target.ends_with("_alone.kso") { "play" } else { "run" };
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg(verb)
            .arg(target)
            .args(engine)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso runs");
        (
            String::from_utf8_lossy(&done.stdout).into_owned(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };

    let (direct, _) = answer(&format!("{dir}/shelf_alone.kso"), &[]);
    let (imported, complaint) = answer(&format!("{dir}/main.kso"), &[]);
    let (interp, _) = answer(&format!("{dir}/main.kso"), &["--interp"]);

    assert_eq!(complaint, "", "native refused an ordering arm reached through an import");
    assert_eq!(direct, "own: true false\nints: true false\n");
    assert_eq!(imported, direct, "the import took the module's own ordering arm away");
    assert_eq!(interp, imported, "the engines disagree on an imported ordering arm");
}

/// The rule that makes the fix above safe to have. An operator group belongs
/// to nobody and reaches every module, so joining one through a type you did
/// not define would let a dependency say what `<` means for everyone's ints.
///
/// It was accepted in silence before — the arm compiled, and the builtin won
/// dispatch, so it did nothing and said nothing. Harmless while arms stayed
/// module-local, and exactly the thing making them ambient would have armed.
#[test]
fn an_operator_arm_needs_a_type_the_module_defines() {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("tests/golden/entryfile/an_ordering_arm_for_a_type_you_do_not_own/main.kso")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    let complaint = String::from_utf8_lossy(&done.stderr);

    assert!(
        complaint.contains("error[ownership]") && complaint.contains("`<`"),
        "an arm on `<` for a type the module does not define was accepted: {complaint}"
    );
}
