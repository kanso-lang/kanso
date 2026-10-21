// The tail-convention narrowing: which functions keep `tailcc`, the
// trampolines for the ones that spill, and the rewrite of the finished IR
// text that applies both. codegen.rs reads this file with include!, so these
// items stay in the codegen module, as ir_scan.rs and the others do.

/// The lines of `text` that hold `needle`, each once and in order, as byte
/// ranges without the newline. The same lines `text.lines()` filtered by
/// `contains(needle)` would give, for text with no carriage returns, which
/// emitted IR never has.
fn lines_holding<'a>(text: &'a str, needle: &'a str) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut past = 0;
    text.match_indices(needle).filter_map(move |(at, _)| {
        if at < past {
            return None;
        }
        let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
        past = end;
        Some((start, end))
    })
}

/// Two ascending runs of line ranges as one, a line in both given once.
fn merged_lines(
    a: impl Iterator<Item = (usize, usize)>,
    b: impl Iterator<Item = (usize, usize)>,
) -> impl Iterator<Item = (usize, usize)> {
    let mut a = a.peekable();
    let mut b = b.peekable();
    std::iter::from_fn(move || match (a.peek().copied(), b.peek().copied()) {
        (Some(x), Some(y)) if x.0 == y.0 => {
            a.next();
            b.next()
        }
        (Some(x), Some(y)) if x.0 < y.0 => a.next(),
        (Some(_), Some(_)) => b.next(),
        (Some(_), None) => a.next(),
        (None, _) => b.next(),
    })
}

/// Whether `line` holds `needle`, found from the byte `at` places into it.
/// `str::contains` builds a substring searcher for every call, about a
/// hundred instructions before it reads a byte, and `narrow_tailcc` asks two
/// or three of these of every line it walks: 178 calls on a one-line
/// program's start-up. The anchor byte is found by memchr, and the needle is
/// compared where one lands. Pick `at` so the anchor is rare in the emitter's
/// lines.
fn holds(line: &str, needle: &str, at: usize) -> bool {
    let anchor = needle.as_bytes()[at] as char;
    line.match_indices(anchor)
        .any(|(i, _)| i >= at && line.as_bytes()[i - at..].starts_with(needle.as_bytes()))
}

/// `tailcc` where it is needed, the C convention everywhere else.
///
/// A `musttail` call may cross an arity or a type only under `tailcc`, so the
/// beat machinery's guaranteed tail calls need it. Every other call does not,
/// and paying for it is not free: a non-tail `call tailcc` whose arguments do
/// not all fit in registers is miscompiled on arm64 — five KValues want ten
/// argument registers and there are eight, and the two that spill come back
/// holding each other's values (task #70). So the convention is kept for the
/// functions a musttail reaches, on both ends of every such edge, and dropped
/// from the rest.
///
/// A function that needs the convention AND spills is reached through a
/// trampoline: one `tailcc` call per frame is lowered correctly, and it is
/// only the second one in a frame that comes back wrong.
///
/// The set is read out of the emitted text rather than recomputed, because a
/// second copy of "when do we musttail" would drift from the first and the
/// symptom of drift is silent corruption.
///
/// Only code is read. A line opening with `@` defines a global, a string
/// constant among them, and a constant's bytes are the program's: a program
/// printing `call tailcc` once lost the words from its constant, kept the
/// declared length, and clang refused the module.
#[cfg(test)]
fn narrow_tailcc(ir: String) -> String {
    narrowed_in_place(ir, 0)
}

/// `narrow_tailcc`'s answer for everything in `text` after `from`, written
/// over the lines it replaces. Every rewrite takes words out of a line or
/// renames a callee to its trampoline, which is shorter than the words it
/// lost, so a line never outgrows the one it replaces and the text only moves
/// down. A rewrite that did grow would send the whole answer into a buffer of
/// its own, so the answer does not rest on that.
fn narrowed_in_place(text: String, from: usize) -> String {
    let code = |line: &str| !line.starts_with('@');
    // Two walks over the lines, neither of which keeps a list of them: one
    // reads which functions keep the convention and sets aside the few
    // `define tailcc` lines the trampolines are decided from, and one finds
    // the lines to rewrite. A list of every line, held across the three
    // walks this used to make, was a third of the module's size again.
    let ir = &text[from..];
    let mut keep: crate::hash::Set<String> = crate::hash::Set::default();
    let mut current: Option<String> = None;
    let mut tailcc_defines: Vec<&str> = Vec::new();
    // The walks visit only the lines that hold the words they act on, found by
    // searching the text for the words. On the tab's corpus 125 lines of 8,726
    // are a `define` and 96 a musttail call, and the line-by-line walk this
    // replaced was 3.9 million instructions of finding newlines.
    let defines =
        lines_holding(ir, "define ").filter(|&(start, _)| ir[start..].starts_with("define "));
    let musttails = lines_holding(ir, "musttail call");
    let jumps_to_lambdas = ir.contains("define internal tailcc %KValue @k_tcall");
    for (start, end) in merged_lines(defines, musttails) {
        let line = &ir[start..end];
        if !code(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix("define ") {
            current = symbol_of(rest);
            if rest.starts_with("tailcc ") {
                tailcc_defines.push(line);
            }
            // `k_tcall{n}` jumps to a lambda through the pointer its closure
            // carries, an edge this reading cannot see, so every lambda keeps
            // the convention in a module that has one.
            if jumps_to_lambdas
                && current.as_deref().is_some_and(|n| {
                    n.strip_prefix("klam").is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()))
                })
            {
                keep.extend(current.clone());
            }
        }
        if line.contains("musttail call") {
            // both ends of a musttail edge must agree on the convention
            if let Some(callee) = symbol_of(line) {
                keep.insert(callee);
            }
            if let Some(name) = current.clone() {
                keep.insert(name);
            }
        }
    }
    // Kept functions whose arguments do not all fit in the eight registers
    // AArch64 passes them in. The count is the same on every host so the ir is
    // too; x86 passes fewer and does not exhibit the defect anyway.
    let mut trampolines: Vec<(String, String)> = Vec::new();
    let mut spilling: crate::hash::Set<String> = crate::hash::Set::default();
    for &line in &tailcc_defines {
        let Some(rest) = line.strip_prefix("define tailcc ") else { continue };
        let Some(name) = symbol_of(rest) else { continue };
        if !keep.contains(&name) {
            continue;
        }
        let Some(open) = rest.find('(') else { continue };
        let Some(close) = rest.rfind(')') else { continue };
        let params: Vec<&str> = match rest[open + 1..close].trim().is_empty() {
            true => Vec::new(),
            false => rest[open + 1..close].split(", ").collect(),
        };
        let types: Vec<&str> = params.iter().filter_map(|p| p.split_whitespace().next()).collect();
        let registers: usize = types.iter().map(|t| register_width(t)).sum();
        if registers <= 8 {
            continue;
        }
        let ret = rest[..open].split('@').next().unwrap_or("%KValue").trim().to_string();
        let taken: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        let handed: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        trampolines.push((
            name.clone(),
            format!(
                "define {ret} @{}({}) {{\nentry:\n  %r = call tailcc {ret} @{}({})\n  ret {ret} %r\n}}\n",
                trampoline_name(&name),
                taken.join(", "),
                quoted(&name),
                handed.join(", ")
            ),
        ));
        spilling.insert(name);
    }

    drop(tailcc_defines);
    let mut rerouted: crate::hash::Set<String> = crate::hash::Set::default();
    // (start, end, replacement) for each line that changes, in order.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (start, end) in lines_holding(ir, "tailcc ") {
        // Every rewrite below needs the word, so a line without it stands as
        // it is and its callee is never looked up.
        let line = &ir[start..end];
        if !code(line) {
            continue;
        }
        let named = symbol_of(line);
        // `k_tcall{n}`'s jump through a closure names no symbol. Every lambda
        // it can reach kept the convention above, so the jump keeps it too.
        if named.is_none() && holds(line, "musttail call", 0) {
            continue;
        }
        let reroute = !holds(line, "musttail call", 0)
            && line.contains("call tailcc ")
            && named.as_ref().is_some_and(|n| spilling.contains(n));
        let rewritten = if reroute {
            let name = named.expect("checked above");
            let call = format!("@{}(", quoted(&name));
            let through = format!("@{}(", trampoline_name(&name));
            rerouted.insert(name);
            line.replace("call tailcc ", "call ").replace(&call, &through)
        } else if !named.as_ref().is_some_and(|n| keep.contains(n)) {
            line.replace("tailcc ", "")
        } else {
            continue;
        };
        let at = line.as_ptr() as usize - text.as_ptr() as usize;
        edits.push((at, at + line.len(), rewritten));
    }
    let shrinks = edits.iter().all(|(start, end, line)| line.len() <= end - start);
    let mut out = match shrinks {
        true => {
            let mut bytes = text.into_bytes();
            let (mut to, mut kept) = (0, 0);
            for (start, end, line) in edits {
                bytes.copy_within(kept..start, to);
                to += start - kept;
                bytes[to..to + line.len()].copy_from_slice(line.as_bytes());
                to += line.len();
                kept = end;
            }
            let len = bytes.len();
            bytes.copy_within(kept..len, to);
            bytes.truncate(to + len - kept);
            String::from_utf8(bytes).expect("each edit replaces a whole line")
        }
        false => {
            let mut out = String::with_capacity(text.len());
            let mut kept = 0;
            for (start, end, line) in edits {
                out.push_str(&text[kept..start]);
                out.push_str(&line);
                kept = end;
            }
            out.push_str(&text[kept..]);
            out
        }
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    // A trampoline no call was rerouted through is a function nothing names,
    // which clang compiles at `-O0` all the same.
    for (name, t) in trampolines {
        if rerouted.contains(&name) {
            out.push_str(&t);
        }
    }
    out
}

/// The first symbol a line names, without its quotes.
fn symbol_of(line: &str) -> Option<String> {
    let at = line.find('@')?;
    let rest = &line[at + 1..];
    match rest.starts_with('"') {
        true => rest[1..].split('"').next().map(str::to_string),
        false => rest.split('(').next().map(|s| s.trim().to_string()),
    }
}

fn trampoline_name(name: &str) -> String {
    quoted(&format!("{name}.c"))
}

/// How many argument registers a parameter of this type occupies.
fn register_width(ty: &str) -> usize {
    match ty {
        "%parsed" => 3,
        "%KValue" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod a_trampoline_is_emitted_for_a_call_that_uses_it {
    use super::narrow_tailcc;

    /// Five KValues are ten registers, past the eight AArch64 passes, and the
    /// musttail keeps the convention, so the function spills.
    const LOOP: &str = "define tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e) {\nentry:\n  %r = musttail call tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e)\n  ret %KValue %r\n}\n";

    #[test]
    fn none_when_nothing_is_rerouted() {
        let out = narrow_tailcc(LOOP.to_string());
        assert!(!out.contains("@f.c("), "a trampoline nothing calls was emitted:\n{out}");
    }

    #[test]
    fn one_when_a_call_goes_through_it() {
        let caller = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call tailcc %KValue @f(%KValue %a, %KValue %a, %KValue %a, %KValue %a, %KValue %a)\n  ret %KValue %r\n}\n";
        let out = narrow_tailcc(format!("{LOOP}{caller}"));
        assert!(
            out.contains("define %KValue @f.c("),
            "the rerouted call has no trampoline:\n{out}"
        );
        assert!(out.contains("call %KValue @f.c("), "the call was not rerouted:\n{out}");
    }

    /// The narrowing writes over the lines it reads, which holds while no
    /// rewrite is longer than its line. Rerouting takes seven bytes out and
    /// adds two per mention of the callee, so a line naming it four times
    /// grows by one, and the answer is written into a buffer of its own. The
    /// lines after it come through rewritten all the same.
    #[test]
    fn a_line_that_grows_is_written_beside_the_rest() {
        let caller = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call tailcc %KValue @f(@f(@f(@f(%KValue %a)\n  %s = call tailcc %KValue @h(%KValue %a)\n  ret %KValue %r\n}\n";
        let out = narrow_tailcc(format!("{LOOP}{caller}"));
        let narrowed = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call %KValue @f.c(@f.c(@f.c(@f.c(%KValue %a)\n  %s = call %KValue @h(%KValue %a)\n  ret %KValue %r\n}\n";
        assert!(
            out.starts_with(&format!("{LOOP}{narrowed}define %KValue @f.c(")),
            "the narrowed module is not the line-by-line rewrite:\n{out}"
        );
    }
}
