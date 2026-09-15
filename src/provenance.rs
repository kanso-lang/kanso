//! Which package does a declaration belong to?
//!
//! The one question left here. The 2026-09-15 ruling made a bare err data,
//! so an `(err _)` arm matches it in the package that raised it as anywhere
//! else, and the pass that once proved which arms could never fire — a
//! fixpoint over the call graph carrying the packages whose errs each group
//! could receive — has nothing to refuse. The 2026-08-29 gavel's foreign-only
//! rescue licence stands and is asked at the word, by comparing the failure's
//! raiser with the rescuing site's package, both of which are `package_of`
//! of a declaration's file.

/// The hako a declaration belongs to.
///
/// Go's rule, which Clay named on 2026-08-26: a package is a DIRECTORY, and
/// its import path is its name. `std/json` and `std/testing` are different
/// packages; `std/json/json.kso` and `std/json/scan.kso` are one; `std/net`
/// and `std/net/http` are two. A fetched hako is `owner/repo`, and a
/// subdirectory inside it is its own package the same way.
///
/// It applies to a program's own modules too, uniformly, because that is what
/// Go does and Clay named Go. It is also what makes the rule teachable: a
/// decoder module and the module that reports its failures are two packages,
/// so the reporting arm is licensed exactly where a reader would write it.
///
/// The rule used to answer `std` for every shipped module. That reading was
/// invisible until gavel 24 made an err's raiser part of dispatch, and then it
/// failed at once: `std/testing` and `std/json` came out the same package, so
/// `when_failed` could not rescue a failure `decode` raised and the harness
/// could not report a test failure. Clay: "testing should be its own hako
/// then... this becomes somewhat of a virtual concept when you're talking
/// about packages that are built in that aren't literally coming from
/// different sources, but for the sake of our rule that makes sense."
pub fn package_of(file: &str) -> &str {
    let path = match after_hako(file) {
        Some(rest) => rest,
        None => file,
    };
    match crate::ast::last_slash(path) {
        Some(at) => &path[..at],
        None => "",
    }
}

/// What follows the first `.hako/` in a path, by a plain byte scan.
///
/// `str::split_once(".hako/")` builds a `StrSearcher`, and a two-way substring
/// search spends most of a short haystack's cost setting itself up — the
/// needle is absent from every path in a program that fetched nothing, so the
/// setup is all there is. callgrind gave `StrSearcher::new` a row of 230,143
/// with `memchr_aligned` at 171,128 under it, against 108,446 on `package_of`
/// itself. `.hako/` is ascii, so the scan is over bytes and the index past it
/// is a character boundary.
fn after_hako(file: &str) -> Option<&str> {
    const NEEDLE: &[u8] = b".hako/";
    let bytes = file.as_bytes();
    for at in 0..bytes.len().saturating_sub(NEEDLE.len() - 1) {
        if bytes[at] == b'.' && &bytes[at..at + NEEDLE.len()] == NEEDLE {
            return Some(&file[at + NEEDLE.len()..]);
        }
    }
    None
}
