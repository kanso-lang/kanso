// The wasm32 retarget: the lines of an emitted module that assume a 64-bit
// pointer, and the pass that rewrites them for the tab. src/ir_wasm.rs and the
// tests call it; the emitter does not. codegen.rs reads this file with
// include!, so these items stay in the codegen module.

/// The lines of an emitted module that assume a 64-bit pointer, each with what
/// it says on wasm32.
///
/// The emitter writes its inline helpers against the runtime's structs by byte
/// offset, and most of those offsets hold on both targets because the field
/// before them is a `long long`. These are the ones that follow a pointer:
/// `KStr` is `{ char*, int, int }`, so its length and capacity move up four
/// bytes; `KMap`'s `sorted` follows two pointers where `pairs` follows a
/// `long long`; and `KClosure` is `{ fn, env, int ncaps, int arity, body }`,
/// so `env` follows the function pointer and `arity` and `body` follow two
/// pointers.
///
/// A line is matched whole, so a helper whose text changes stops matching
/// rather than matching something else. tests/native_layout_on_wasm32.rs reads
/// the emitter for each left-hand side and fails when one is missing, and
/// runs the micro corpus through this pass against its goldens.
pub const WASM32_LINES: [(&str, &str); 7] = [
    ("%slenp = getelementptr i8, ptr %sp, i64 8", "%slenp = getelementptr i8, ptr %sp, i64 4"),
    ("%lenp = getelementptr i8, ptr %s, i64 8", "%lenp = getelementptr i8, ptr %s, i64 4"),
    ("%cp = getelementptr i8, ptr %sptr, i64 12", "%cp = getelementptr i8, ptr %sptr, i64 8"),
    ("%sortedp = getelementptr i8, ptr %m, i64 16", "%sortedp = getelementptr i8, ptr %m, i64 12"),
    ("%arp = getelementptr i8, ptr %c, i64 20", "%arp = getelementptr i8, ptr %c, i64 12"),
    ("%bodyp = getelementptr i8, ptr %c, i64 24", "%bodyp = getelementptr i8, ptr %c, i64 16"),
    ("%envp = getelementptr i8, ptr %c, i64 8", "%envp = getelementptr i8, ptr %c, i64 4"),
];

/// An emitted module, rewritten for wasm32 so that clang can build it against
/// the runtime compiled for the same target.
///
/// Three things change and nothing else. The target lines: the triple names
/// wasm32 and the x86 data layout goes, so clang supplies its own. The calling
/// conventions: wasm has neither `tailcc` nor `preserve_none`, and a module
/// built with `-mtail-call` keeps every `musttail` as a real tail call under
/// the C convention. And the six offsets in `WASM32_LINES`.
///
/// Only code is rewritten, for the reason `narrow_tailcc` gives: a line
/// opening with `@` is a global, and a string constant's bytes are the
/// program's.
pub fn retarget_wasm32(module: &str) -> String {
    // The edits are found by searching the module for the words they act on
    // and written between copies of everything else. A pass that visited
    // every line, and searched each one for `cc `, was 6.2 million of the tab
    // compile's 168 million instructions natively, for a few hundred edits in
    // 8,726 lines.
    let line_of = |at: usize| {
        let start = module[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = module[at..].find('\n').map_or(module.len(), |n| at + n);
        (start, end)
    };
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for (start, end) in lines_holding(module, "target ") {
        let line = &module[start..end];
        if line.starts_with("target datalayout") {
            edits.push((start, (end + 1).min(module.len()), ""));
        } else if line.starts_with("target triple") {
            edits.push((start, end, "target triple = \"wasm32-unknown-wasi\""));
        }
    }
    for (at, _) in module.match_indices("cc ") {
        let Some(cc) =
            ["preserve_nonecc ", "tailcc "].iter().find(|cc| module[..at + 3].ends_with(*cc))
        else {
            continue;
        };
        if module[line_of(at).0..].starts_with('@') {
            continue;
        }
        edits.push((at + 3 - cc.len(), at + 3, ""));
    }
    for (start, end) in lines_holding(module, " = getelementptr i8, ptr %") {
        let line = &module[start..end];
        if line.starts_with('@') {
            continue;
        }
        let body = line.trim_start();
        if let Some((_, wasm)) = WASM32_LINES.iter().find(|(native, _)| *native == body) {
            edits.push((end - body.len(), end, wasm));
        }
    }
    edits.sort_unstable_by_key(|&(start, _, _)| start);
    let mut out = String::with_capacity(module.len());
    let mut kept = 0;
    for (start, end, with) in edits {
        out.push_str(&module[kept..start]);
        out.push_str(with);
        kept = end;
    }
    out.push_str(&module[kept..]);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}
