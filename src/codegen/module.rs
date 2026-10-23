// The module writer: what the emitter hands over once it has walked the
// program, and the pass that writes the module around the body. The body is
// pruned, the declarations it names and the call twins it uses are written
// in front of it with the strings, cells and tables, and the doors' calls take
// their convention. codegen.rs reads this file with include!, so these items
// stay in the codegen module, as narrow.rs and the others do.

/// What the emitter has written once it has walked the program: the body in
/// the pieces it was sealed in, and the tables the head of the module is
/// written from. `emit` hands this back and the `Backend` goes, with every
/// analysis and index it held, before `finish` allocates the module. Kept
/// together, they were under the module's allocation, which was the most the
/// tab's compile held.
struct Written {
    convention: ClosureConvention,
    inline_helpers: bool,
    has_entry: bool,
    /// Whether the emitter asked by name about a group the walk left out.
    asked_left_out: bool,
    pieces: Vec<String>,
    closure_consts: Vec<(String, String, usize)>,
    caf_cells: Vec<String>,
    closure_cells: Vec<String>,
    strings: Vec<(String, Vec<u8>)>,
    globals: String,
    /// The literal each stamped site id names, by its position in `strings`,
    /// id 1 first.
    sites: Vec<u32>,
}

impl Written {
    /// Whether the body names a symbol of one of `decls`: its dispatcher,
    /// word twin, value wrapper or wrapper record.
    fn names_any<'d>(&self, decls: impl Iterator<Item = &'d FnDecl>) -> bool {
        let mut symbols: Vec<String> = Vec::new();
        for decl in decls {
            let (name, arity) = (decl.name.as_str(), decl.params.len());
            symbols.extend([
                dsym(name, arity),
                tsym(name, arity),
                wsym(name, arity),
                rsym(name, arity),
            ]);
        }
        let queries: crate::hash::Set<&str> = symbols.iter().map(|s| s.as_str()).collect();
        !queries.is_empty()
            && self.pieces.iter().any(|piece| !queries_named(piece, &queries).is_empty())
    }

    /// The module: the body pruned, the declarations and globals it names
    /// written in front of it, and the tail calls narrowed.
    fn finish(self) -> String {
        let pruned = match self.has_entry {
            true => prune_unnamed(self.pieces, &dsym(crate::ast::ENTRY, 0), &self.closure_consts),
            false => self.pieces,
        };
        let pieces: Vec<String> = pruned.into_iter().map(paired_appends).collect();
        // One inline dispatcher per arity the program actually writes. An
        // unused `internal` definition costs nothing after optimization, but
        // it does cost a line, a define and a branch in the emitted golden —
        // so the twins are generated against the body rather than carried in
        // DECLARES the way the other inline helpers are.
        // Every `@sym(` the body writes, read off in one pass. The question
        // below is asked once per declare line -- 163 of them -- and used to
        // be answered by searching the whole emitted body for each, which is
        // 163 scans of a text that grows with the program. On a build of
        // bench/compile_corpus that search was 17.08% of the process.
        let mut body_calls = crate::hash::Set::default();
        let mut body_lines = crate::hash::Set::default();
        for piece in &pieces {
            body_calls.extend(called_symbols(piece));
            body_lines.extend(symbols_before_newline(piece));
        }
        let call_twins: String = (0..=4)
            .filter(|n| body_calls.contains(format!("k_call{n}_fast").as_str()))
            .map(|n| call_twin(n, self.convention, self.inline_helpers))
            .chain(
                (0..=4)
                    .filter(|n| body_calls.contains(format!("k_tcall{n}").as_str()))
                    .map(tail_call_twin),
            )
            .collect();
        let declares: String = {
            // BOTH SIDES OF THIS BUILT THE SAME INDEX. kanso#1461 landed one
            // inline, joining DECLARES’s non-declare lines and scanning the
            // three haystacks in place; this branch had already factored the
            // scan into `called_symbols` and cached the DECLARES half in a
            // `OnceLock`, so `body_calls` is computed once above and reused by
            // the call-twin filter rather than rebuilt here. The sets are the
            // same for every query the emitter makes: a symbol from a
            // `declare` line holds no whitespace and no `(`, so bounding the
            // span at the first `(` and bounding it at the first `(` or space
            // cannot disagree about one.
            //
            // `crate::hash::Set` and not std’s, which kanso#1461’s comment
            // gives the reason for and `tests/the_compile_path_hashes_with_a
            // _fixed_seed.rs` pins: std seeds per process, and a randomly
            // seeded table makes this count differ between two runs of one
            // binary, which the compile rows read as a reproduction failure.
            let twin_calls = called_symbols(&call_twins);
            // A symbol DECLARES's own definitions call is kept as well, and
            // each line knows whether it is one: `DeclareLine::context`.
            let called = |sym: &str| body_calls.contains(sym) || twin_calls.contains(sym);
            declares_for_program(called, called, counters_wanted(), self.inline_helpers, true)
        };
        let mut out = String::new();
        out.push_str(&call_twins);
        for cell in &self.caf_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
            let _ = writeln!(out, "@{cell}_ready = internal global i8 0");
        }
        for cell in &self.closure_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
        }
        for (cell, w, arity) in
            self.closure_consts.iter().filter(|(cell, _, _)| body_lines.contains(cell.as_str()))
        {
            // K_INT 0 is the env a zero-capture closure never reads; K_CLOSURE
            // is tag 11. The KClosure layout is the runtime's:
            // { fn, env, int ncaps, int arity, body }, the body being the
            // lambda the wrapper `w_` calls.
            let _ = writeln!(out, "@{cell}_env = internal constant %KValue zeroinitializer");
            let _ = writeln!(
                out,
                "@{cell}_clo = internal constant {{ ptr, ptr, i32, i32, ptr }}                  {{ ptr @{w}, ptr @{cell}_env, i32 0, i32 {arity}, ptr @{} }}",
                w.strip_prefix("w_").unwrap_or(w)
            );
            let _ = writeln!(
                out,
                "@{cell} = internal constant %KValue                  {{ i64 11, i64 ptrtoint (ptr @{cell}_clo to i64) }}"
            );
        }
        // A string is interned when a function asks for it, and the function
        // may since have been pruned: on the codegen corpus 198 of 270
        // strings and 264 of their literal cells were named by nothing, 36% of
        // the module's bytes, each parsed and laid out by clang all the same.
        // Only the body and the type tables name a string.
        let texts: Vec<&str> =
            pieces.iter().map(String::as_str).chain([self.globals.as_str()]).collect();
        let named = named_strings(&texts, self.strings.len());
        drop(texts);
        for ((name, bytes), [string, lit]) in self.strings.iter().zip(named) {
            if string {
                let _ = writeln!(
                    out,
                    "@{name} = private unnamed_addr constant [{} x i8] c\"{}\"",
                    bytes.len(),
                    ir_bytes(bytes)
                );
            }
            if lit {
                let _ = writeln!(out, "@{name}_lit = internal global %KValue zeroinitializer");
            }
        }
        // THE MODULE IS WRITTEN ONCE, at its own size. The declarations and
        // everything above go in first and the body's pieces follow, each let
        // go as soon as it is copied, and the narrowing rewrites the lines
        // where they stand. The body used to be copied behind the globals and
        // the narrowing wrote that again into a buffer of its own, so two
        // copies of the module were held at once.
        out.push_str(&self.globals);
        out.push('\n');
        let declared = declares.len() + 1;
        let mut table = Count(0);
        site_table(&mut table, &self.sites, &self.strings);
        let size = declared + out.len() + table.0 + pieces.iter().map(String::len).sum::<usize>();
        let mut module = String::with_capacity(size);
        module.push_str(&declares);
        module.push('\n');
        module.push_str(&out);
        site_table(&mut module, &self.sites, &self.strings);
        drop((declares, out));
        for piece in pieces {
            module.push_str(&piece);
        }
        through_doors(narrowed_in_place(module, declared), self.convention)
    }
}

/// The runtime functions the program calls that take `preserve_nonecc`
/// wherever closures do. src/runtime.c defines each with `K_DOORCC`, which is
/// the same attribute under the same probe, and says why these six;
/// `tests/the_doors_the_program_calls_agree.rs` holds the two lists equal,
/// since a door on one convention called on the other passes its arguments
/// in the wrong registers.
pub const PRESERVE_NONE_DOORS: [&str; 6] = [
    "k_b_append_rendered",
    "k_b_entries",
    "k_b_to_float_slice",
    "k_b_utf8",
    "k_b_append_string_mut",
    "k_b_to_float_slice_raw",
];

/// Every declare of and call to a door, written with the convention.
///
/// One pass over the module. Every door is spelled `k_b_...`, so the pass
/// stops only where `%KValue @k_b_` appears, and writes the convention in
/// front of it when a door's name and its `(` follow and a `declare ` or a
/// `call ` comes before. This was eight `str::replace` calls, each building
/// the whole module again: on a one-line `kanso play` under clang 19 they were
/// 402,596 of the 1,067,649 instructions under `kanso::main`.
fn through_doors(ir: String, convention: ClosureConvention) -> String {
    if convention != ClosureConvention::PreserveNone {
        return ir;
    }
    const MARK: &str = "%KValue @k_b_";
    let mut out = String::with_capacity(ir.len() + 4096);
    let mut rest = ir.as_str();
    while let Some(at) = rest.find(MARK) {
        let (head, tail) = rest.split_at(at);
        let name = &tail["%KValue @".len()..];
        let door = PRESERVE_NONE_DOORS
            .iter()
            .any(|d| name.strip_prefix(d).is_some_and(|after| after.starts_with('(')));
        out.push_str(head);
        if door && (head.ends_with("declare ") || head.ends_with("call ")) {
            out.push_str("preserve_nonecc ");
        }
        out.push_str(MARK);
        rest = &tail[MARK.len()..];
    }
    out.push_str(rest);
    out
}

/// The dispatcher, inlined at the call site for the shape that actually
/// happens: a closure of the arity written, with no failure in an argument.
///
/// `k_call2` is 26 instructions and a fold applies it once a lap. Ten of the
/// 26 ask about the callable — is it a failure, is it a closure, does its
/// arity match — and a fold passes the same callable through its self-call
/// unchanged, so those ten are loop-invariant and LICM can hoist them out of
/// the loop TailCallElim makes of the recursion. It cannot hoist across a
/// call, which is what the runtime dispatcher is: LTO sees the body and
/// declines to inline it on cost, and `always_inline` on the C definition is
/// ignored because the emitted `.ll` calls the symbol by name. So the test
/// is emitted here instead, in the module the optimizer is already in.
///
/// Every shape the fast arm does not cover falls through to `k_call{n}`,
/// which re-asks everything and answers exactly as before — a failing
/// callable, a fnref, a wrong arity, a failing argument. That is why the
/// order here may differ from the runtime's: the arm only fires where all
/// the orders agree.
fn call_twin(n: usize, convention: ClosureConvention, inline: bool) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let attr = if inline { " alwaysinline" } else { "" };
    let _ = writeln!(s, "define internal %KValue @k_call{n}_fast(%KValue %f{args}){attr} {{");
    let _ = writeln!(s, "  %ftag = extractvalue %KValue %f, 0");
    let _ = writeln!(s, "  %isclo = icmp eq i64 %ftag, 11");
    let _ = writeln!(s, "  br i1 %isclo, label %arity, label %slow");
    let _ = writeln!(s, "arity:");
    let _ = writeln!(s, "  %fp = extractvalue %KValue %f, 1");
    let _ = writeln!(s, "  %c = inttoptr i64 %fp to ptr");
    let _ = writeln!(s, "  %arp = getelementptr i8, ptr %c, i64 20");
    let _ = writeln!(s, "  %ar = load i32, ptr %arp");
    let _ = writeln!(s, "  %okar = icmp eq i32 %ar, {n}");
    let _ = writeln!(s, "  br i1 %okar, label %args, label %slow");
    let _ = writeln!(s, "args:");
    for i in 0..n {
        let _ = writeln!(s, "  %t{i} = extractvalue %KValue %a{i}, 0");
        let _ = writeln!(s, "  %e{i} = icmp eq i64 %t{i}, 5");
    }
    for i in 1..n {
        let prev = match i {
            1 => "%e0".to_string(),
            _ => format!("%or{}", i - 1),
        };
        let _ = writeln!(s, "  %or{i} = or i1 {prev}, %e{i}");
    }
    match n {
        0 => {
            let _ = writeln!(s, "  br label %go");
        }
        1 => {
            let _ = writeln!(s, "  br i1 %e0, label %slow, label %go");
        }
        _ => {
            let _ = writeln!(s, "  br i1 %or{}, label %slow, label %go", n - 1);
        }
    }
    let _ = writeln!(s, "go:");
    let _ = writeln!(s, "  %envp = getelementptr i8, ptr %c, i64 8");
    let _ = writeln!(s, "  %env = load ptr, ptr %envp");
    let _ = writeln!(s, "  %fnp = load ptr, ptr %c");
    // The closure pointer this loads was written by the wrapper emitter, so
    // the two carry the same convention or the arguments land in the wrong
    // registers. `%fnp` is only ever a closure body: the arm tested tag == 11
    // above, so the fnref family (no env parameter) never reaches here.
    let cc = convention.keyword();
    let _ = writeln!(s, "  %r = call {cc}%KValue %fnp(ptr %env{args})");
    let _ = writeln!(s, "  ret %KValue %r");
    let _ = writeln!(s, "slow:");
    let _ = writeln!(s, "  %s = call %KValue @k_call{n}(%KValue %f{args})");
    let _ = writeln!(s, "  ret %KValue %s");
    let _ = writeln!(s, "}}");
    s
}

/// The twin of `k_call{n}` for a call in tail position. A lambda's body is on the
/// tailcc convention and the closure carries a pointer to it, so a function
/// value called last jumps to the lambda instead of calling its wrapper: a
/// loop written in continuation-passing style runs in constant stack, as
/// Appendix A says a tail call does. Anything the jump does not take (a
/// closure the runtime built, a partial, a reference to a group, a failing
/// argument) goes to `k_call{n}`, which answers as before. The caller
/// reaches this with a musttail of its own, so the helper is a function and
/// never inlined.
fn tail_call_twin(n: usize) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let _ = writeln!(s, "define internal tailcc %KValue @k_tcall{n}(%KValue %f{args}) {{");
    let _ = writeln!(s, "  %ftag = extractvalue %KValue %f, 0");
    let _ = writeln!(s, "  %isclo = icmp eq i64 %ftag, 11");
    let _ = writeln!(s, "  br i1 %isclo, label %arity, label %slow");
    let _ = writeln!(s, "arity:");
    let _ = writeln!(s, "  %fp = extractvalue %KValue %f, 1");
    let _ = writeln!(s, "  %c = inttoptr i64 %fp to ptr");
    let _ = writeln!(s, "  %arp = getelementptr i8, ptr %c, i64 20");
    let _ = writeln!(s, "  %ar = load i32, ptr %arp");
    let _ = writeln!(s, "  %okar = icmp eq i32 %ar, {n}");
    let _ = writeln!(s, "  br i1 %okar, label %args, label %slow");
    let _ = writeln!(s, "args:");
    for i in 0..n {
        let _ = writeln!(s, "  %t{i} = extractvalue %KValue %a{i}, 0");
        let _ = writeln!(s, "  %e{i} = icmp eq i64 %t{i}, 5");
    }
    for i in 1..n {
        let prev = match i {
            1 => "%e0".to_string(),
            _ => format!("%or{}", i - 1),
        };
        let _ = writeln!(s, "  %or{i} = or i1 {prev}, %e{i}");
    }
    match n {
        0 => {
            let _ = writeln!(s, "  br label %body");
        }
        1 => {
            let _ = writeln!(s, "  br i1 %e0, label %slow, label %body");
        }
        _ => {
            let _ = writeln!(s, "  br i1 %or{}, label %slow, label %body", n - 1);
        }
    }
    let _ = writeln!(s, "body:");
    let _ = writeln!(s, "  %bodyp = getelementptr i8, ptr %c, i64 24");
    let _ = writeln!(s, "  %lam = load ptr, ptr %bodyp");
    let _ = writeln!(s, "  %has = icmp ne ptr %lam, null");
    let _ = writeln!(s, "  br i1 %has, label %go, label %slow");
    let _ = writeln!(s, "go:");
    let _ = writeln!(s, "  %envp = getelementptr i8, ptr %c, i64 8");
    let _ = writeln!(s, "  %env = load ptr, ptr %envp");
    let _ = writeln!(s, "  %r = musttail call tailcc %KValue %lam(ptr %env{args})");
    let _ = writeln!(s, "  ret %KValue %r");
    let _ = writeln!(s, "slow:");
    let _ = writeln!(s, "  %s = call %KValue @k_call{n}(%KValue %f{args})");
    let _ = writeln!(s, "  ret %KValue %s");
    let _ = writeln!(s, "}}");
    s
}

/// A lookup from a type id to one of `names`, read out of a constant array
/// rather than a switch, with `fallback` for an id past the end.
fn type_table(globals: &mut String, symbol: &str, names: &[String], fallback: &str) {
    let slots = names.len();
    let row: Vec<String> = names.iter().map(|n| format!("ptr @{n}")).collect();
    let _ = writeln!(
        globals,
        "@{symbol}_table = private unnamed_addr constant [{slots} x ptr] [{}]",
        row.join(", ")
    );
    let _ = writeln!(
        globals,
        "define ptr @{symbol}(i64 %id) {{\nentry:\n  %in = icmp ult i64 %id, {slots}\n  \
         br i1 %in, label %T, label %TD\nT:\n  \
         %at = getelementptr [{slots} x ptr], ptr @{symbol}_table, i64 0, i64 %id\n  \
         %name = load ptr, ptr %at\n  ret ptr %name\nTD:\n  ret ptr @{fallback}\n}}\n"
    );
}

/// A sink that keeps only the length of what is written to it.
struct Count(usize);

impl std::fmt::Write for Count {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0 += s.len();
        Ok(())
    }
}

/// `k_site_name`: the literal each site id names, as an offset into one
/// constant holding every stamped literal's bytes. Offsets rather than
/// pointers, because a table of pointers in a position-independent binary is
/// a relocation per slot for the loader to apply at every start. Every site
/// the emitter stamped is written, including one whose function the prune
/// dropped: finding the survivors meant searching the whole body, which cost
/// more of a build than the few hundred bytes of text it saved. It is written
/// twice, once to count and once into the module, which is allocated at its
/// final size: a buffer of its own, or the globals pushed past a doubling,
/// raised the tab compile's peak memory.
fn site_table(w: &mut impl std::fmt::Write, sites: &[u32], strings: &[(String, Vec<u8>)]) {
    let text = |site: &u32| &strings[*site as usize].1;
    let size: usize = 1 + sites.iter().map(|site| text(site).len()).sum::<usize>();
    let _ = write!(w, "@k_site_text = private unnamed_addr constant [{size} x i8] c\"\\00");
    for site in sites {
        ir_bytes_into(w, text(site));
    }
    let slots = sites.len() + 1;
    let _ =
        write!(w, "\"\n@k_site_name_table = private unnamed_addr constant [{slots} x i32] [i32 0");
    let mut offset = 1;
    for site in sites {
        let _ = write!(w, ", i32 {offset}");
        offset += text(site).len();
    }
    let _ = writeln!(
        w,
        "]\ndefine ptr @k_site_name(i64 %id) {{\nentry:\n  \
         %at = getelementptr i32, ptr @k_site_name_table, i64 %id\n  \
         %off = load i32, ptr %at\n  %wide = zext i32 %off to i64\n  \
         %name = getelementptr i8, ptr @k_site_text, i64 %wide\n  ret ptr %name\n}}\n"
    );
}

/// Two in-place byte appends in a row, the second onto the first's result and
/// that result read nowhere else in the function, as one call to
/// `k_b_append_mut_int2`. Both calls were already the proven-bytes,
/// proven-int door, so the pair asks the same questions once; see the helper.
///
/// The search goes from one call to the next rather than line by line, and a
/// body with no pair comes back as it was, because this runs on every build
/// and most programs have none.
fn paired_appends(body: String) -> String {
    const CALL: &str = " = call %KValue @k_b_append_mut_int(%KValue ";
    // the last argument is the site a failure in that append names
    fn parse(line: &str) -> Option<(&str, &str, &str, &str)> {
        let rest = line.strip_prefix("  ")?;
        let (name, rest) = rest.split_once(CALL)?;
        let args = rest.strip_suffix(')')?;
        let (acc, rest) = args.split_once(", %KValue ")?;
        let (x, site) = rest.split_once(", ptr ")?;
        Some((name, acc, x, site))
    }
    let line_end = |from: usize| body[from..].find('\n').map_or(body.len(), |n| from + n);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut from = 0;
    while let Some(n) = body[from..].find(CALL) {
        let at = from + n;
        let start = body[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = line_end(at);
        from = end;
        if end >= body.len() {
            break;
        }
        let next = line_end(end + 1);
        let (Some((a, acc, x, s1)), Some((b, acc2, y, s2))) =
            (parse(&body[start..end]), parse(&body[end + 1..next]))
        else {
            continue;
        };
        if acc2 != a {
            continue;
        }
        // a use count is the function's own
        let first = body[..start].rfind("\ndefine ").map_or(0, |n| n + 1);
        let last = body[next..].find("\n}\n").map_or(body.len(), |n| next + n);
        let used: usize = body[first..last].lines().map(|l| count_operand(l, a)).sum();
        if used == 2 {
            edits.push((
                start,
                next,
                format!(
                    "  {b} = call %KValue @k_b_append_mut_int2(%KValue {acc}, %KValue {x}, %KValue {y}, ptr {s1}, ptr {s2})"
                ),
            ));
            from = next;
        }
    }
    if edits.is_empty() {
        return body;
    }
    // Each pair becomes one line shorter than the two it replaces, so the
    // body is rewritten where it stands and its text only moves down.
    let mut bytes = body.into_bytes();
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
    String::from_utf8(bytes).expect("each edit replaces whole lines")
}

fn ir_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    ir_bytes_into(&mut out, bytes);
    out
}

fn ir_bytes_into(out: &mut impl std::fmt::Write, bytes: &[u8]) {
    for byte in bytes {
        match byte {
            0x20..=0x7e if *byte != b'"' && *byte != b'\\' => {
                let _ = out.write_char(*byte as char);
            }
            _ => {
                let _ = write!(out, "\\{byte:02X}");
            }
        }
    }
}
