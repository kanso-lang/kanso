//! A `sibling-goldens-move` licenses one branch, and expires by saying which.
//!
//! The file lets a branch take a sibling's performance goldens from its own
//! coordinated branch rather than from main — which is the whole protection
//! that stops a compiler change bringing the numbers it will be judged against.
//! #724 wrote one, merged it, and every branch afterwards inherited the
//! licence in silence.
//!
//! Naming the branch is what makes it expire without anybody remembering. A
//! file left on main names a branch nobody is on, so it matches nothing and
//! grants nothing. These pin both halves, because a rule that only ever grants
//! and a rule that only ever refuses are both easy to write by accident.

use std::process::Command;

/// The pair that collided. Both are 82 bytes, and that is asserted below
/// rather than left to a reader counting characters, because the collision
/// condition is easy to lose to an innocent reword and nothing would say so.
const LICENSED: &str =
    "branch: a-real-fix\nkq's carry_dedup moves, and what it now buys is a linear walk.\n";
const LEFT_BEHIND: &str =
    "branch: the-branch-that-merged\nkq's carry_dedup moves, and it buys a linear walk.\n";

/// Run the licence check against a file holding `body`, for `branch`.
///
/// The directory is named by a hash of the body, not its LENGTH. Two of the
/// fixtures below are 82 bytes each, cargo runs them on separate threads, and
/// the file inside is the constant `sibling-goldens-move` — so under a length
/// key both tests wrote one path and the first to finish called
/// `remove_dir_all` on the other's program mid-run. 27 failures in 40 runs,
/// measured, reporting exit 2 where the test asserts 0.
///
/// Those two bodies are left the same length deliberately. The condition is
/// what a length key needs to fail, so the corpus carries it and a regression
/// to `body.len()` goes red on the next run rather than the next fortnight.
///
/// kanso#1177 fixed this exact shape in a_bare_list_is_or_is_not_bytes.rs,
/// where the key was a length too; #1169, #1175 and #1178 fixed the sibling
/// shape where the path was a constant.
fn verdict(body: &str, branch: &str) -> i32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(body, &mut hasher);
    let key = std::hash::Hasher::finish(&hasher);
    let dir = std::env::temp_dir().join(format!("kanso-gm-{key:x}"));
    std::fs::create_dir_all(&dir).expect("a directory to write in");
    let file = dir.join("sibling-goldens-move");
    std::fs::write(&file, body).expect("the file writes");

    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/goldens-move-licenses.sh");
    let done =
        Command::new("sh").arg(&script).arg(&file).arg(branch).output().expect("the check runs");
    let _ = std::fs::remove_dir_all(&dir);
    done.status.code().unwrap_or(-1)
}

#[test]
fn a_file_licenses_the_branch_it_names() {
    assert_eq!(verdict(LICENSED, "a-real-fix"), 0, "a file naming this branch did not license it");
}

/// The one that matters: this is the shape #724 left on main.
#[test]
fn a_file_left_behind_licenses_nobody() {
    assert_eq!(
        verdict(LEFT_BEHIND, "some-later-branch"),
        1,
        "a leftover did not read as a leftover — 1 is 'does not apply', and it \
         has to be told apart from a malformed licence so the build carries on \
         with main's goldens rather than refusing"
    );
}

/// The rule that already existed, kept: a trade has two sides.
#[test]
fn a_file_naming_no_gain_licenses_nothing() {
    let body = "branch: a-real-fix\ncarry_dedup moves.\n";
    assert_eq!(
        verdict(body, "a-real-fix"),
        2,
        "a file for this branch claiming no gain must refuse the build, not \
         quietly fall back to main"
    );
}

/// And a file that names no branch at all is the old format, which licensed
/// every branch forever. It must not be readable as a licence.
#[test]
fn the_old_format_licenses_nothing() {
    let body = "kq's carry_dedup moves, and what it buys is a linear walk.\n";
    assert_eq!(verdict(body, "a-real-fix"), 1, "a file with no branch line was honoured");
}

/// The condition itself, pinned. Under the old `kanso-gm-{body.len()}` key
/// these two staged one directory holding one file called
/// `sibling-goldens-move`, and `verdict` ends in `remove_dir_all` — 27 of 40
/// runs failed. The key is a hash of the body now, and these two are left the
/// same length so the corpus keeps asking the question the hash answers.
#[test]
fn the_two_bodies_that_collided_are_still_the_same_length() {
    assert_eq!(
        LICENSED.len(),
        LEFT_BEHIND.len(),
        "the fixture stopped exercising the collision a length key needs"
    );
}
