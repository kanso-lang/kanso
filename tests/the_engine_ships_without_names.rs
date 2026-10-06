//! Every visitor to the playground downloads its engine, and the custom
//! sections are a tenth of the module's bytes that no page reads: the exports
//! keep their names, and only a debugger or a linker reads the rest.
//! `scripts/build_wasm.sh` strips it. This reads the shipped module's
//! sections and asserts that no custom section is left, so a build that
//! stops stripping is a red spec rather than 47,622 gzipped bytes nobody sees.

use std::path::PathBuf;

fn uleb(bytes: &[u8], at: &mut usize) -> usize {
    let (mut value, mut shift) = (0usize, 0);
    loop {
        let b = bytes[*at];
        *at += 1;
        value |= ((b & 0x7f) as usize) << shift;
        if b & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

/// The names of the module's custom sections, in the order they appear.
fn custom_sections(bytes: &[u8]) -> Vec<String> {
    assert_eq!(&bytes[..4], b"\0asm", "a wasm module");
    let mut at = 8;
    let mut names = Vec::new();
    while at < bytes.len() {
        let id = bytes[at];
        at += 1;
        let size = uleb(bytes, &mut at);
        let end = at + size;
        if id == 0 {
            let mut p = at;
            let len = uleb(bytes, &mut p);
            names.push(String::from_utf8_lossy(&bytes[p..p + len]).into_owned());
        }
        at = end;
    }
    names
}

#[test]
fn the_shipped_engine_carries_no_custom_sections() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/kanso.wasm");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("docs/kanso.wasm is not built here; scripts/build_wasm.sh builds it");
        return;
    };
    assert_eq!(custom_sections(&bytes), Vec::<String>::new(), "sections left in docs/kanso.wasm");
}
