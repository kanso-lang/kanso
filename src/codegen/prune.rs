// The prune: which dispatchers, wrappers and lambda bodies nothing live
// names, and the name-level call graph it and `kept_out` read. codegen.rs
// reads this file with include!, so these items stay in the codegen module.

/// Dispatchers, wrappers and lambda bodies nothing live names. A block is kept
/// when a chain of names reaches it from the entry or from a block that is not
/// a candidate; a dead caller's call to its callee does not count, and neither
/// does a cycle of dead blocks naming each other. Only `d_`, `w_` and `klam`
/// symbols are candidates: everything else is either the entry, a
/// builder the constant initialiser calls, or a switch the runtime calls by
/// name. A lambda body is named by its wrapper or by the closure built over
/// it, and one in a library function the program never reaches is named by
/// neither; at `-O0` clang compiled each of them anyway.
/// `cells` pairs a constant closure's cell with the wrapper it points at. The
/// pointer lives in a module global rather than in any function body, so the
/// wrapper is named by the cell rather than by a call, and a site that loads
/// the cell is what keeps it.
///
/// The body arrives as the pieces the emitter sealed, and no block crosses a
/// piece. The blocks kept are moved down inside each piece's own buffer rather
/// than gathered into a new one: they are runs of the piece in order, and a
/// second buffer the size of what survives sat at the tab's high-water mark.
fn prune_unnamed(
    pieces: Vec<String>,
    entry: &str,
    cells: &[(String, String, usize)],
) -> Vec<String> {
    // A block never straddles two pieces, because a piece is only closed
    // between two whole writes. `owner` says which piece each block is in.
    let mut owner: Vec<usize> = Vec::new();
    let mut blocks: Vec<(&str, &str)> = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        for block in ir_defines(piece) {
            owner.push(i);
            blocks.push(block);
        }
    }
    let mut alive = vec![false; blocks.len()];
    {
        // Every name this prune will ever ask about: one per block, plus the
        // closure cells. The index is keyed on exactly these, so a name
        // nothing asks about costs nothing to walk past.
        let queries: crate::hash::Set<&str> = blocks
            .iter()
            .map(|(sym, _)| *sym)
            .chain(cells.iter().map(|(cell, _, _)| cell.as_str()))
            .filter(|sym| !sym.is_empty())
            .collect();
        // Which of those names each block writes, read off once per block. The
        // search this replaced asked
        // the question once per (block, name) pair and built a fresh two-way
        // searcher for each: 72.10% of `kanso build bench/runbench`, with
        // `names_symbol` 71.71% of the process on its own.
        // A name the one-pass scan cannot tokenise is answered the old way,
        // per block, rather than assumed away. Emitted symbols are all one of
        // the two shapes `queries_named` reads, so this list is empty in
        // practice -- and the correctness of the index does not rest on that
        // being true, which is the point of having it.
        let odd: Vec<&str> = queries.iter().copied().filter(|sym| !one_pass_reads(sym)).collect();
        let mut names: Vec<crate::hash::Set<&str>> =
            blocks.iter().map(|(_, text)| queries_named(text, &queries)).collect();
        for (written, (_, text)) in names.iter_mut().zip(blocks.iter()) {
            for sym in &odd {
                if names_symbol(text, sym) {
                    written.insert(sym);
                }
            }
        }
        // A mark from the roots rather than a count of mentions. Striking a
        // block once nothing else named it was reference counting, and a
        // cycle names itself: std/list's merge sort is merge, merge_on, pick
        // and advance calling round, so a program that never sorts struck
        // `sort`, `msort` and `span` and kept the four, which nothing could
        // call.
        let at_sym: crate::hash::Map<&str, usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| !sym.is_empty())
            .map(|(at, (sym, _))| (*sym, at))
            .collect();
        let wrapped: crate::hash::Map<&str, &str> =
            cells.iter().map(|(cell, w, _)| (cell.as_str(), w.as_str())).collect();
        let mut work: Vec<usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| {
                // The runtime calls the thunk dispatcher itself, from
                // `k_force`, so no emitted line names it.
                *sym == entry
                    || *sym == "d_thunk_eval"
                    || !(sym.starts_with("d_")
                        || sym.starts_with("w_")
                        || sym.starts_with("klam")
                        || sym.starts_with("\"d_")
                        || sym.starts_with("\"w_"))
            })
            .map(|(at, _)| at)
            .collect();
        for &at in &work {
            alive[at] = true;
        }
        while let Some(at) = work.pop() {
            for name in &names[at] {
                // A cell is a module global, so the wrapper it points at is
                // named by whoever loads the cell rather than by a call.
                let target = wrapped.get(name).map_or(*name, |w| *w);
                for sym in [*name, target] {
                    if let Some(&next) = at_sym.get(sym) {
                        if !alive[next] {
                            alive[next] = true;
                            work.push(next);
                        }
                    }
                }
            }
        }
    }
    let mut kept: Vec<Vec<std::ops::Range<usize>>> = vec![Vec::new(); pieces.len()];
    for ((&i, (_, text)), alive) in owner.iter().zip(&blocks).zip(alive) {
        if alive {
            let at = text.as_ptr() as usize - pieces[i].as_ptr() as usize;
            kept[i].push(at..at + text.len());
        }
    }
    drop(blocks);
    pieces
        .into_iter()
        .zip(kept)
        .map(|(piece, runs)| {
            let mut bytes = piece.into_bytes();
            let mut to = 0;
            for run in runs {
                let len = run.len();
                bytes.copy_within(run, to);
                to += len;
            }
            // The room a piece was opened with is given back, a piece at a
            // time, before the module they are copied into is allocated.
            bytes.truncate(to);
            bytes.shrink_to_fit();
            String::from_utf8(bytes).expect("every block ends at a line break")
        })
        .collect()
}

/// Whether `queries_named` can tokenise this symbol, which is to say whether
/// it is one of the two shapes the emitter writes: an unquoted run of
/// `[alnum _]`, or a quoted name from `"` to its closing `"` with no other
/// quote inside. Anything else is answered by `names_symbol` per block, so
/// the index stays exact without assuming the emitter's spellings.
fn one_pass_reads(sym: &str) -> bool {
    match sym.strip_prefix('"') {
        Some(rest) => match rest.strip_suffix('"') {
            Some(inner) => !inner.contains('"'),
            None => false,
        },
        None => !sym.is_empty() && sym.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
    }
}

/// Which of `queries` this text names, in one pass, answering exactly what
/// `names_symbol` answers for each of them.
///
/// The delimiter rule is `names_symbol`'s: `@sym` counts only where the byte
/// after it is not alphanumeric, `_` or `"`. Two token shapes cover every
/// symbol the emitter writes, and `the_only_two_shapes_a_symbol_takes` pins
/// it: an unquoted name is a run of those same bytes, and a quoted one runs
/// from `"` to the next `"`, which is how `quoted()` spells a name holding a
/// slash or an operator character. Scanning the unquoted run alone would stop
/// at the slash inside `@"d_add/2"` and miss it.
fn queries_named<'a>(text: &str, queries: &crate::hash::Set<&'a str>) -> crate::hash::Set<&'a str> {
    let held = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'"';
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        at = from;
        let end = match bytes.get(from) {
            Some(b'"') => match bytes[from + 1..].iter().position(|b| *b == b'"') {
                Some(close) => from + close + 2,
                None => continue,
            },
            _ => {
                let mut end = from;
                while end < bytes.len() && held(bytes[end]) {
                    end += 1;
                }
                end
            }
        };
        if matches!(bytes.get(end), Some(b) if held(*b)) {
            continue;
        }
        if let Some(name) = queries.get(&text[from..end]) {
            found.insert(*name);
        }
    }
    found
}

/// Every declaration's index, by name, in program order.
///
/// `Backend::group_indices` scanned `program.fns` end to end and collected the
/// matches into a fresh `Vec<usize>` on every call, and `group_param_set` and
/// `group_return_set` ask it once per parameter and once per call site.
fn group_indices_by_name(program: &Program) -> HashMap<&str, Vec<usize>> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        by_name.entry(decl.name.as_str()).or_default().push(at);
    }
    by_name
}

/// Every declared name a cycle can reach: the members of every cycle in what
/// the bodies mention, and everything those members mention, onward.
///
/// A cohort bracket costs a push and a pop on every call it wraps, and pays
/// only when the call leaves garbage worth a rewind. A name a cycle reaches
/// can run once per element of something -- a parser's recursive descent, a
/// loop, a helper one of those calls per number -- and its calls are the
/// small ones. A name no cycle reaches runs a number of times fixed by
/// straight-line code, which is where a phase of a program starts and ends.
/// Names, not groups, and a mention counts whether it is a call or a value.
/// A cycle through a closure some library calls back is not seen; that miss
/// costs speed and nothing else, because the bracket is sound on any call
/// the license admits.
fn cycle_reached(program: &Program) -> crate::hash::Set<&str> {
    let (names, adj) = call_graph(program);
    let mut hot = vec![false; names.len()];
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !hot[at] {
            hot[at] = true;
            queue.extend(adj[at].iter().copied().filter(|&next| !hot[next]));
        }
    }
    names.iter().zip(hot).filter(|(_, h)| *h).map(|(n, _)| *n).collect()
}

/// Every declared name that can reach a cycle: the members of every cycle,
/// and every name that mentions one of those, backward. A call to one of
/// these runs a loop or a recursion before it answers, so a cohort bracket
/// around it is paid once against the work of a whole loop.
fn reaches_cycle(program: &Program) -> crate::hash::Set<&str> {
    let (names, adj) = call_graph(program);
    let mut back: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (from, outs) in adj.iter().enumerate() {
        for &to in outs {
            back[to].push(from);
        }
    }
    let mut on = vec![false; names.len()];
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !on[at] {
            on[at] = true;
            queue.extend(back[at].iter().copied().filter(|&prev| !on[prev]));
        }
    }
    names.iter().zip(on).filter(|(_, o)| *o).map(|(n, _)| *n).collect()
}

/// The names on a cycle of the call graph: the loops, and the recursions
/// that are not loops. `cycle_reached` is these and everything they call.
fn cycle_members(program: &Program) -> crate::hash::Set<&str> {
    let (names, adj) = call_graph(program);
    let mut out = crate::hash::Set::default();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            out.extend(scc.into_iter().map(|at| names[at]));
        }
    }
    out
}

/// The functions `expr` names, as their places in `index`: a call's head, a
/// name passed as a value and a partial alike. `call_graph` and `kept_out`
/// both build their edges from it.
fn name_mentions(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
    if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
        if let Some(&at) = index.get(&**n) {
            out.push(at);
        }
    }
    crate::for_each_child(expr, |child| name_mentions(child, index, out));
}

/// Each declared name, and the names its bodies mention, by index.
fn call_graph(program: &Program) -> (Vec<&str>, Vec<Vec<usize>>) {
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for decl in &program.fns {
        let from = index[decl.name.as_str()];
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    name_mentions(expr, &index, &mut adj[from])
                }
            }
        }
    }
    (names, adj)
}

/// Functions a dispatcher calls from its heavy arms, kept out of line.
///
/// A group of clauses compiles to one function with a switch at its head, and
/// LLVM gives that function one frame. When one arm inlines a callee that
/// loops, the loop's registers are callee-saved ones, so the prologue pushes
/// six of them and the epilogue pops them on every call -- including the calls
/// whose arm is a single append. The encoder's `encode_onto` is that shape:
/// 2,380,860 calls on runbench, 1,438,110 of them scalars that paid the frame
/// of the string arm's inlined escaper. A callee that can reach a cycle, named
/// in an arm of a group that also has an arm reaching none, stays a call, and
/// the dispatcher's cheap arms keep a frame of their own size.
fn kept_out(program: &Program) -> crate::hash::Set<&str> {
    fn decl_mentions(decl: &FnDecl, index: &HashMap<&str, usize>) -> Vec<usize> {
        let mut out = Vec::new();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => name_mentions(expr, index, &mut out),
            }
        }
        out
    }
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    let per_decl: Vec<Vec<usize>> = program.fns.iter().map(|d| decl_mentions(d, &index)).collect();
    for (decl, m) in program.fns.iter().zip(&per_decl) {
        adj[index[decl.name.as_str()]].extend(m.iter().copied());
    }
    // loops: a name in a cycle, and every name that can reach one
    let mut loops = vec![false; names.len()];
    let mut callers: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (from, to) in adj.iter().enumerate() {
        for &t in to {
            callers[t].push(from);
        }
    }
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !loops[at] {
            loops[at] = true;
            queue.extend(callers[at].iter().copied().filter(|&c| !loops[c]));
        }
    }
    let mut out = crate::hash::Set::default();
    let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(at);
    }
    for ((name, _), clauses) in &groups {
        if clauses.len() < 2 {
            continue;
        }
        // a switch on the argument's type, where the arms do unrelated work
        let typed = |c: &usize| {
            program.fns[*c]
                .params
                .iter()
                .any(|p| matches!(p, crate::ast::Pattern::Annotated { .. }))
        };
        if !clauses.iter().any(typed) {
            continue;
        }
        let own = index[name];
        let heavy = |c: &usize| per_decl[*c].iter().any(|&m| m != own && loops[m]);
        if clauses.iter().all(heavy) || !clauses.iter().any(heavy) {
            continue;
        }
        for c in clauses {
            for &m in &per_decl[*c] {
                if m != own && loops[m] {
                    out.insert(names[m]);
                }
            }
        }
    }
    out
}
