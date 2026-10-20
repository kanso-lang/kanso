//! The compiler's own binary has one writable segment on Linux.
//!
//! The instruction gates read frames of the compiler by name, so valgrind has
//! to read the executable's symbols. With RELRO, lld writes two writable
//! segments, and when the first ends on a page boundary the two are
//! contiguous; valgrind 3.22 then reads no symbols from the executable at all
//! and every frame comes back as a bare address. kanso#2004 was built into
//! that layout on CI and six gates had nothing to read. build.rs links the
//! compiler with `-z norelro`, which leaves one writable segment whatever the
//! binary's size.

#[cfg(target_os = "linux")]
#[test]
fn the_compiler_has_no_relro_segment_and_one_writable_load() {
    let exe = env!("CARGO_BIN_EXE_kanso");
    let out = std::process::Command::new("readelf")
        .args(["-l", "-W", exe])
        .output()
        .expect("readelf runs");
    assert!(out.status.success(), "readelf -l {exe} failed");
    let headers = String::from_utf8_lossy(&out.stdout);
    let writable = headers
        .lines()
        .filter(|l| l.trim_start().starts_with("LOAD") && l.contains(" RW "))
        .count();
    assert!(
        !headers.contains("GNU_RELRO") && writable == 1,
        "the compiler has {writable} writable LOAD segments and RELRO is {}; \
         valgrind cannot read its symbols when two of them are contiguous:\n{headers}",
        if headers.contains("GNU_RELRO") { "present" } else { "absent" }
    );
}
