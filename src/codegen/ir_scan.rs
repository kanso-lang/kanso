// Reading emitted IR as text: whether a body branches to a label, names a
// string or a symbol, or calls one; the split of a module into its
// definitions that the prune walks; and the specs that hold the declares
// table and the prune to the same answers as a plain search. codegen.rs reads
// this file with include!, so these items stay in the codegen module, as
// declares.rs, dispatchers.rs and emit.rs do.

/// Whether `text` writes a branch to `label`: `label %name` with the name
/// ending there, so `fail1` is not answered by `label %fail10`.
fn branches_to(text: &str, label: &str) -> bool {
    let probe = format!("label %{label}");
    text.match_indices(&probe).any(|(at, _)| {
        !text[at + probe.len()..]
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
    })
}

/// Which interned strings the texts name, by index: whether `@sN` appears,
/// and whether `@sN_lit` does. `intern` names string N `sN`, so a name is read
/// as a number and nothing is hashed. A name ends where LLVM's unquoted names
/// do, at the first byte that is not a letter, digit, `_`, `.` or `$`, so
/// `@s12` never answers for `@s12_lit` or `@s120`.
///
/// Collecting every `@name` into a set and asking it was 23,334 instructions
/// of `kanso play`'s start-up on a one-line program, most of it hashing names
/// nothing would ask about.
fn named_strings(texts: &[&str], count: usize) -> Vec<[bool; 2]> {
    let name_byte = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$');
    let mut named = vec![[false; 2]; count];
    for text in texts {
        let bytes = text.as_bytes();
        let mut at = 0;
        while let Some(next) = text[at..].find("@s") {
            let from = at + next + 2;
            at = from;
            let mut to = from;
            let mut n = 0usize;
            while to < bytes.len() && bytes[to].is_ascii_digit() {
                n = n.saturating_mul(10).saturating_add(usize::from(bytes[to] - b'0'));
                to += 1;
            }
            if to == from || n >= count {
                continue;
            }
            let ends = |i: usize| !bytes.get(i).is_some_and(|b| name_byte(*b));
            if ends(to) {
                named[n][0] = true;
            } else if bytes[to..].starts_with(b"_lit") && ends(to + 4) {
                named[n][1] = true;
            }
        }
    }
    named
}

/// Every `sym` for which `text` writes `@sym(`, collected in one pass.
///
/// This answers `text.contains(&format!("@{sym}("))` exactly, for any `sym`
/// that holds no newline and no `(` -- which every LLVM symbol here does.
/// From each `@` the scan runs to the first `(`, stopping at a newline, so a
/// probe can never straddle two lines; that matters because one of the three
/// places this replaces searched DECLARES line by line rather than whole.
fn called_symbols(text: &str) -> crate::hash::Set<&str> {
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        let mut to = from;
        while to < bytes.len() && bytes[to] != b'(' && bytes[to] != b'\n' {
            to += 1;
        }
        if to < bytes.len() && bytes[to] == b'(' {
            found.insert(&text[from..to]);
        }
        at = from;
    }
    found
}

/// Every `sym` for which `text` writes `@sym\n`, collected in one pass.
///
/// A line can only end in `@sym`, so only the LAST `@` on each line can start
/// a match and the whole text is read once. That makes the answer exact for a
/// `sym` holding no `@` and no newline, which every symbol the emitter asks
/// about is: `@a @b\n` would have to be found under the name `a @b`, and
/// reading each line's last at alone cannot see it.
///
/// The forward scan this replaced ran from every `@` to the end of its line,
/// which on a big emitted body is the text times the number of ats: it cost
/// `kanso build bench/runbench` 209 million instructions, turning a fall into
/// a rise.
fn symbols_before_newline(text: &str) -> crate::hash::Set<&str> {
    // One walk over the bytes, holding the last `@` since the line began. A
    // search for each newline and then for the `@` before it was two calls a
    // line, 42,784 lines in the tab's compile of its corpus.
    let mut found = crate::hash::Set::default();
    let mut last_at = None;
    for (i, &b) in text.as_bytes().iter().enumerate() {
        match b {
            b'@' => last_at = Some(i),
            b'\n' => {
                if let Some(at) = last_at.take() {
                    found.insert(&text[at + 1..i]);
                }
            }
            _ => {}
        }
    }
    found
}

/// The symbols DECLARES calls from its own inline definitions, which is every
/// line of it that is not itself a `declare`. DECLARES is a constant, so this
/// is read once for the life of the process rather than once per emit.
/// The symbols DECLARES calls from its own inline definitions, written down
/// rather than scanned for.
///
/// DECLARES is a `const`: the same 65 names in every process kanso runs.
/// Reading them off it once cost 62 answers for 1,024 lines of scanning, and
/// the scanning was 94% of the index this branch builds -- 292,701 of the
/// 278,812 `called_symbols` charges on the start-up corpus came through
/// `Once::call_once_force`, over 1,023 calls, against 18,159 from the emitter's
/// two. On a program that emits one `print` that is the whole of the index's
/// cost, and `kanso play` paid it to learn nothing it could not have been told.
///
/// Sorted, which is what the oracle's binary search needs. Emit never asks
/// the list: each `declare` line reads `DeclareLine::context`, computed from
/// it when the compiler is built.
///
/// `tests/the_declares_symbols_are_the_ones_declares_calls.rs` recomputes this
/// list from DECLARES with the same scan it replaces and asserts they are the
/// same set, so an edit to DECLARES that adds or drops a call turns that spec
/// red rather than silently leaving a symbol out of a program's declares.
const DECLARES_CONTEXT_CALLS: &[&str] = &[
    "k_b_append",
    "k_b_append_byte",
    "k_b_append_mut",
    "k_b_append_mut_byte",
    "k_b_append_mut_int",
    "k_b_append_mut_int2",
    "k_b_append_mut_word",
    "k_b_append_slice",
    "k_b_append_slice_fast",
    "k_b_append_word_slow",
    "k_b_at",
    "k_b_at_fast",
    "k_b_bit_and",
    "k_b_bit_and_fast",
    "k_b_bit_not",
    "k_b_bit_not_fast",
    "k_b_bit_or",
    "k_b_bit_or_fast",
    "k_b_bit_shl",
    "k_b_bit_shl_fast",
    "k_b_bit_shr",
    "k_b_bit_shr_fast",
    "k_b_bit_xor",
    "k_b_bit_xor_fast",
    "k_b_bytes",
    "k_b_bytes_fast",
    "k_b_bytes_frame",
    "k_b_find2",
    "k_b_find2_below",
    "k_b_find2_below_fast",
    "k_b_find2_below_raw",
    "k_b_find2_fast",
    "k_b_find2_raw",
    "k_b_length",
    "k_b_length_fast",
    "k_b_push_mut",
    "k_b_push_mut_fast",
    "k_b_push_mut_known",
    "k_b_put_mut",
    "k_b_put_mut_fast",
    "k_b_slice",
    "k_b_slice_fast",
    "k_b_slice_raw",
    "k_b_utf8_slice",
    "k_b_utf8_slice_fast",
    "k_b_utf8_slice_raw",
    "k_bool",
    "k_check_bool",
    "k_check_int",
    "k_check_rec",
    "k_check_rec_fast",
    "k_check_rec_fast_w",
    "k_check_tag",
    "k_field",
    "k_field_fast",
    "k_float",
    "k_force",
    "k_force_fast",
    "k_index",
    "k_index_fast",
    "k_int",
    "k_none",
    "k_str_lit",
    "k_str_lit_fast",
    "k_truthy",
    "k_truthy_bad",
    "k_truthy_w",
    "llvm.assume",
    "llvm.memcpy.p0.p0.i64",
];

/// The binary search each `declare` line used to ask at emit time, kept as
/// the oracle for `DeclareLine::context`.
#[cfg(test)]
fn declares_context_calls(sym: &str) -> bool {
    DECLARES_CONTEXT_CALLS.binary_search(&sym).is_ok()
}

#[cfg(test)]
mod every_declare_line_knows_whether_declares_calls_it {
    use super::{declares_context_calls, DECLARES, DECLARES_DEV, DECLARE_LINES, DECLARE_LINES_DEV};

    #[test]
    fn the_flag_is_the_search() {
        let mut asked = 0;
        for (text, lines) in [(DECLARES, &DECLARE_LINES), (DECLARES_DEV, &DECLARE_LINES_DEV)] {
            for line in lines.iter().filter(|l| l.sym_start < l.sym_end) {
                let sym = &text[line.sym_start..line.sym_end];
                assert_eq!(line.context, declares_context_calls(sym), "{sym}");
                asked += usize::from(line.context);
            }
        }
        assert!(asked > 0, "no declare line is one DECLARES calls, so this proves nothing");
    }
}

#[cfg(test)]
mod the_declares_symbols_are_the_ones_declares_calls {
    use super::{called_symbols, DECLARES, DECLARES_CONTEXT_CALLS};

    /// The scan the static replaces, kept as the oracle.
    fn scanned() -> std::collections::BTreeSet<&'static str> {
        let mut found = std::collections::BTreeSet::new();
        for line in DECLARES.lines().filter(|l| !l.starts_with("declare")) {
            found.extend(called_symbols(line));
        }
        found
    }

    /// Written down and scanned for are the same set. An edit to DECLARES that
    /// adds or drops a call turns this red rather than leaving a symbol out of
    /// a program's declares, where the only symptom is a link error in whatever
    /// program happens to reach it.
    #[test]
    fn the_written_list_is_what_the_scan_finds() {
        let written: std::collections::BTreeSet<&str> =
            DECLARES_CONTEXT_CALLS.iter().copied().collect();
        let found = scanned();
        let missing: Vec<&&str> = found.difference(&written).collect();
        let extra: Vec<&&str> = written.difference(&found).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "DECLARES_CONTEXT_CALLS has drifted from DECLARES.\n               missing (DECLARES calls it, the list does not name it): {missing:?}\n               extra (the list names it, DECLARES does not call it): {extra:?}"
        );
    }

    /// And it is sorted, because it is asked with a binary search.
    #[test]
    fn the_list_is_sorted_and_holds_no_duplicate() {
        let mut sorted = DECLARES_CONTEXT_CALLS.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            DECLARES_CONTEXT_CALLS,
            &sorted[..],
            "the list is asked with binary_search, which answers nonsense on an \
             unsorted slice and answers it quietly"
        );
        sorted.dedup();
        assert_eq!(sorted.len(), DECLARES_CONTEXT_CALLS.len(), "the list repeats a symbol");
    }

    /// The population is not empty, so the two tests above are not agreeing
    /// about nothing.
    #[test]
    fn the_list_is_not_empty() {
        assert!(
            DECLARES_CONTEXT_CALLS.len() > 40,
            "only {} symbols, where DECLARES's inline definitions called sixty-two \
             when this was written",
            DECLARES_CONTEXT_CALLS.len()
        );
    }
}

#[cfg(test)]
mod called_symbols_agrees {
    use super::*;

    /// What the emitter asked before the index existed, kept as the oracle.
    fn searched(text: &str, sym: &str) -> bool {
        text.contains(&format!("@{sym}("))
    }

    /// Every query the emitter makes, asked both ways, over a text.
    ///
    /// The symbols are DECLARES's own -- the real 163 -- plus every prefix and
    /// suffix of each, because a span-based index can agree on a whole name
    /// and still disagree on one letter of it, and the emitter's question is
    /// `@sym(` where a prefix would answer for a longer symbol if the index
    /// stored anything looser than the run up to the paren.
    fn agrees_over(text: &str) {
        let index = called_symbols(text);
        let mut asked = 0;
        for line in DECLARES.lines() {
            let Some(rest) = line.strip_prefix("declare ") else { continue };
            let Some(at) = rest.find('@') else { continue };
            let sym = &rest[at + 1..];
            let Some(paren) = sym.find('(') else { continue };
            let sym = &sym[..paren];
            for end in 1..=sym.len() {
                for start in 0..end {
                    let part = &sym[start..end];
                    assert_eq!(
                        index.contains(part),
                        searched(text, part),
                        "the index and the search disagree on `@{part}(`"
                    );
                    asked += 1;
                }
            }
        }
        assert!(asked > 10_000, "the corpus asked only {asked} questions");
    }

    #[test]
    fn over_the_declares_block() {
        agrees_over(DECLARES);
    }

    /// The `@cell` form, which matches only at the end of a line.
    ///
    /// The scan reads each line's LAST at, so this is the shape that goes
    /// wrong when a line holds more than one.
    #[test]
    fn a_cell_is_named_only_where_its_line_ends() {
        for text in [
            "@a\n",
            "@a",      // no newline, so nothing matches
            "@a @b\n", // only the last at can end the line
            "@a(\n",
            "  @a\n  @b\n",
            "@\n",
            "",
            "@a\n@a\n",
        ] {
            let index = symbols_before_newline(text);
            // Every symbol the emitter asks about is a generated identifier,
            // so the queries here hold no at and no newline -- see the note on
            // symbols_before_newline for the one shape outside that.
            for sym in ["a", "b", "", "a("] {
                assert_eq!(
                    index.contains(sym),
                    text.contains(&format!("@{sym}\n")),
                    "index and search disagree on `@{sym}\\n` in {text:?}"
                );
            }
        }
    }

    /// The edges a span scan can get wrong, each as a whole text.
    #[test]
    fn the_shapes_a_span_scan_can_lose() {
        for text in [
            "@a(",              // the plain case
            "@a",               // no paren at all
            "@a\n(",            // a newline between the name and the paren
            "@@a(",             // two ats, and `@a(` is still in there
            "@a@b(",            // an at inside the span
            "  call @a(i64 1)", // the shape a real line has
            "@a(@b(@c(",        // three in one line
            "",                 // nothing
            "@",                // an at and nothing after it
        ] {
            let index = called_symbols(text);
            for sym in ["a", "b", "c", "@a", "a@b", "", "ab"] {
                assert_eq!(
                    index.contains(sym),
                    searched(text, sym),
                    "index and search disagree on `@{sym}(` in {text:?}"
                );
            }
        }
    }
}

/// Whether this text names `@sym`. A call writes `@sym(`, but a closure hands
/// its wrapper over as `ptr @sym,` — so the delimiter decides, and it also
/// keeps `@w_klam1` from answering for `@w_klam17`.
fn names_symbol(text: &str, sym: &str) -> bool {
    let needle = format!("@{sym}");
    text.match_indices(&needle).any(|(at, _)| {
        !matches!(
            text.as_bytes().get(at + needle.len()),
            Some(b) if b.is_ascii_alphanumeric() || *b == b'_' || *b == b'"'
        )
    })
}

#[cfg(test)]
mod the_prune_agrees_with_the_search {
    use super::*;

    /// The mark written the slow way, kept as the oracle.
    ///
    /// It asks `names_symbol` once per (block, name) pair, which is what made
    /// the search this replaced 72.10% of `kanso build bench/runbench`. It is
    /// the definition of the right answer and nothing else, so it stays here
    /// rather than in the commit message.
    fn by_search(body: &str, entry: &str, cells: &[(String, String, usize)]) -> String {
        let blocks = ir_defines(body);
        let candidate = |sym: &str| {
            sym != entry
                && sym != "d_thunk_eval"
                && (sym.starts_with("d_")
                    || sym.starts_with("w_")
                    || sym.starts_with("klam")
                    || sym.starts_with("\"d_")
                    || sym.starts_with("\"w_"))
        };
        let mut alive: Vec<bool> = blocks.iter().map(|(sym, _)| !candidate(sym)).collect();
        loop {
            let named = |sym: &str| {
                blocks.iter().zip(&alive).any(|((_, text), live)| *live && names_symbol(text, sym))
            };
            let woken = blocks.iter().enumerate().position(|(at, (sym, _))| {
                !alive[at]
                    && (named(sym) || cells.iter().any(|(cell, w, _)| w == sym && named(cell)))
            });
            match woken {
                Some(at) => alive[at] = true,
                None => break,
            };
        }
        blocks.into_iter().zip(alive).filter(|(_, live)| *live).map(|((_, text), _)| text).collect()
    }

    fn define(sym: &str, body: &str) -> String {
        format!("define %KValue @{sym}() {{\nentry:\n{body}\n}}\n")
    }

    /// The prune over a body written as one piece, which is what these
    /// specs were written against.
    fn prune_unnamed(body: String, entry: &str, cells: &[(String, String, usize)]) -> String {
        super::prune_unnamed(vec![body], entry, cells).concat()
    }

    /// The emitter closes a piece between two definitions, so a caller and
    /// its callee can sit in different pieces. The prune reads the names
    /// across all of them, and keeps the same blocks it keeps from one.
    #[test]
    fn a_body_in_pieces_keeps_what_the_whole_body_keeps() {
        let first = format!(
            "{}{}",
            define("d_entry", "  call %KValue @d_a()"),
            define("d_dead", "  call %KValue @d_b()")
        );
        let second = format!(
            "{}{}{}",
            define("d_a", "  call %KValue @d_b()"),
            define("d_b", "  ret %KValue zeroinitializer"),
            define("d_c", "  ret %KValue zeroinitializer")
        );
        let whole = prune_unnamed(format!("{first}{second}"), "d_entry", &[]);
        let pieces = super::prune_unnamed(vec![first, second], "d_entry", &[]);
        assert_eq!(pieces.concat(), whole, "the pieces kept different blocks");
        assert!(whole.contains("@d_b()") && !whole.contains("@d_c("), "{whole}");
    }

    fn agree_on(body: &str, entry: &str, cells: &[(String, String, usize)]) {
        assert_eq!(
            prune_unnamed(body.to_string(), entry, cells),
            by_search(body, entry, cells),
            "the index and the search kept different blocks"
        );
    }

    /// A chain the fixpoint has to walk down: the entry names nothing, so
    /// `d_a` goes, and only then is `d_b` unnamed, and only then `d_c`.
    #[test]
    fn a_chain_that_falls_one_round_at_a_time() {
        let body = [
            define("d_entry", "  ret %KValue zeroinitializer"),
            define("d_a", "  %x = call %KValue @d_b()\n  ret %KValue %x"),
            define("d_b", "  %x = call %KValue @d_c()\n  ret %KValue %x"),
            define("d_c", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        assert!(!prune_unnamed(body.clone(), "d_entry", &[]).contains("@d_c("));
        agree_on(&body, "d_entry", &[]);
    }

    /// Blocks that name each other and nothing live names. std/list's merge
    /// sort is four of them calling round, and counting mentions kept all
    /// four in every program that imported the library and never sorted.
    #[test]
    fn a_cycle_nothing_reaches_goes_whole() {
        let body = [
            define("d_entry", "  %x = call %KValue @d_kept()\n  ret %KValue %x"),
            define("d_kept", "  ret %KValue zeroinitializer"),
            define("d_merge", "  %x = call %KValue @d_pick()\n  ret %KValue %x"),
            define("d_pick", "  %x = call %KValue @d_merge()\n  ret %KValue %x"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@d_kept("), "the named block was pruned");
        assert!(!kept.contains("@d_merge("), "a cycle nothing reaches was kept");
        assert!(!kept.contains("@d_pick("), "a cycle nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// The delimiter rule, which is the whole reason `names_symbol` exists:
    /// `@w_klam1` must not answer for `@w_klam17`.
    #[test]
    fn a_longer_name_does_not_answer_for_a_shorter_one() {
        let body = [
            define("d_entry", "  %x = call %KValue @w_klam17()\n  ret %KValue %x"),
            define("w_klam1", "  ret %KValue zeroinitializer"),
            define("w_klam17", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@w_klam17("), "the named wrapper was pruned");
        assert!(!kept.contains("define %KValue @w_klam1()"), "the unnamed wrapper was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A quoted name, which is how `quoted()` spells one holding a slash. The
    /// one-pass scan reads it from quote to quote; a run over the delimiter
    /// class alone would stop at the slash and lose it.
    #[test]
    fn a_quoted_name_is_read_to_its_closing_quote() {
        let body = [
            define("d_entry", "  %x = call %KValue @\"d_add/2\"()\n  ret %KValue %x"),
            define("\"d_add/2\"", "  ret %KValue zeroinitializer"),
            define("\"d_sub/2\"", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@\"d_add/2\"("), "the named quoted block was pruned");
        assert!(!kept.contains("@\"d_sub/2\"()"), "the unnamed quoted block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A lambda body is a candidate too. Its wrapper names it, so it goes in
    /// the round after the wrapper does; one a closure is built over is named
    /// by that construction and stays.
    #[test]
    fn a_lambda_body_goes_with_the_wrapper_that_named_it() {
        let body = [
            define(
                "d_entry",
                "  %c = call %KValue @k_closure(ptr @klam5, i64 1)\n  ret %KValue %c",
            ),
            define("w_klam3", "  %x = call %KValue @klam3()\n  ret %KValue %x"),
            define("klam3", "  ret %KValue zeroinitializer"),
            define("klam5", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("define %KValue @klam5("), "the lambda a closure names was pruned");
        assert!(!kept.contains("define %KValue @klam3("), "the lambda nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A wrapper is kept when a CELL that stands for it is named, which is the
    /// second half of the question and the one that reads `cells`.
    #[test]
    fn a_wrapper_lives_while_its_cell_is_named() {
        let body = [
            define("d_entry", "  %x = load %KValue, ptr @k_clo_7\n  ret %KValue %x"),
            define("w_klam3", "  ret %KValue zeroinitializer"),
            define("w_klam4", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let cells = [("k_clo_7".to_string(), "w_klam3".to_string(), 1usize)];
        let kept = prune_unnamed(body.clone(), "d_entry", &cells);
        assert!(kept.contains("@w_klam3("), "the wrapper its cell names was pruned");
        assert!(!kept.contains("@w_klam4("), "the wrapper nothing names was kept");
        agree_on(&body, "d_entry", &cells);
    }

    /// A name the one-pass scan cannot tokenise falls back to the search, so
    /// the index is exact without assuming how a symbol is spelled.
    #[test]
    fn an_odd_name_is_answered_the_old_way() {
        assert!(one_pass_reads("d_foo"));
        assert!(one_pass_reads("\"d_add/2\""));
        assert!(!one_pass_reads("d_foo.bar"));
        assert!(!one_pass_reads("\"un\"closed"));
        let body = [
            define("d_entry", "  %x = call %KValue @d_foo.bar()\n  ret %KValue %x"),
            define("d_foo.bar", "  ret %KValue zeroinitializer"),
            define("d_foo.baz", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(body.clone(), "d_entry", &[]);
        assert!(kept.contains("@d_foo.bar("), "the named odd block was pruned");
        assert!(!kept.contains("@d_foo.baz("), "the unnamed odd block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// Real emitted IR, with the symbols the emitter really writes.
    ///
    /// The text is what `emit_ir` produced, so the prune has already run over
    /// it once; feeding it back asks both implementations the same several
    /// hundred questions about a body neither was written against.
    #[test]
    fn real_emitted_ir_reads_the_same_both_ways() {
        let source = "fn count 0 acc\n  acc\n\nfn count n acc\n  count (n - 1) (acc + n)\n\nmain = print \"{count 10 0}\"\n";
        let program = crate::compile("sample.kso", source, false).expect("the sample compiles");
        let ir = emit_ir(&program, ClosureConvention::Absent).expect("the sample lowers");
        assert!(ir.lines().count() > 200, "the sample emitted only {} lines", ir.lines().count());
        for entry in ["d_main", "d_count"] {
            agree_on(&ir, entry, &[]);
        }
    }
}

/// Split emitted IR into segments, keeping every byte. A `define` segment
/// runs from its header to the closing brace on its own line and carries its
/// symbol; everything between definitions — globals the fnref statics live in
/// among them — is a segment with no symbol, which the prune never touches.
fn ir_defines(body: &str) -> Vec<(&str, &str)> {
    // Every block is a run of whole lines, so it is a slice of the body
    // rather than a copy: the prune that reads these runs at the emitter's
    // high-water mark, beside the body it splits.
    let mut out: Vec<(&str, &str)> = Vec::new();
    let mut sym = "";
    let mut from = 0;
    let mut at = 0;
    let mut inside = false;
    for line in body.split_inclusive('\n') {
        if !inside && line.starts_with("define ") {
            if at > from {
                out.push(("", &body[from..at]));
                from = at;
            }
            inside = true;
            sym = line
                .find('@')
                .and_then(|a| line[a + 1..].find('(').map(|p| &line[a + 1..a + 1 + p]))
                .unwrap_or_default();
        }
        at += line.len();
        if inside && line.trim_end() == "}" {
            out.push((std::mem::take(&mut sym), &body[from..at]));
            from = at;
            inside = false;
        }
    }
    if at > from {
        out.push(("", &body[from..at]));
    }
    out
}

/// Whether an emitted line names this temp in a `%KValue` operand position.
/// The delimiter matters: `%t2` is a prefix of `%t20`, and boxing the wrong
/// register writes a program that type-checks and computes the wrong record.
fn named_as_a_value(text: &str, temp: &str) -> bool {
    // Asked of every line for every carried temp, so the needle is not built:
    // each place the temp appears is checked for the type in front of it.
    text.match_indices(temp).any(|(at, _)| {
        text[..at].ends_with("%KValue ")
            && matches!(text.as_bytes().get(at + temp.len()), Some(b',') | Some(b')') | None)
    })
}
