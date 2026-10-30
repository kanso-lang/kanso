// The free helpers the emitter writes words with: symbol names, literal
// words, the lift around a call that may fail, word arithmetic, and reading
// and writing a value's inline tag and payload. codegen.rs reads this file
// with include!, so these items stay in the codegen module.

/// LLVM symbol for a dispatcher: quoted when the kanso name carries a
/// module qualifier's slash.
fn wsym(name: &str, arity: usize) -> String {
    quoted(&format!("w_{name}_{arity}"))
}

/// The static a `k_fnref` value points at: the wrapper, its arity, and the
/// name the diagnostic says when a call brings the wrong number of arguments.
fn rsym(name: &str, arity: usize) -> String {
    quoted(&format!("r_{name}_{arity}"))
}

/// The ambient group an interpolated value may dispatch to.
const RENDER_GROUP: &str = "render/to_string";

fn dsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}"))
}

/// A group's word twin: the same body, its int parameters taken as raw
/// words. A `dsym` always ends in the arity, so `_w` cannot name a group.
fn tsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}_w"))
}

/// Word `n` of `value` when it is a literal `{ i64 A, i64 B }`. Reading one
/// off a constant with `extractvalue` is an instruction clang's fast selector
/// at -O0 does not handle, and it sends the rest of the block to the slow one.
fn literal_word(value: &str, n: usize) -> Option<&str> {
    let inner = value.strip_prefix("{ i64 ")?.strip_suffix(" }")?;
    let (a, b) = inner.split_once(", i64 ")?;
    Some(if n == 0 { a } else { b })
}

/// Runs `rewind` with every argument in `lifted` that is a bignum when the
/// edge is reached moved out of the arena around it. A bignum the iteration
/// built sits above the mark the rewind returns to; the runtime copies each
/// one out before the rewind and back in after it. An edge whose arguments
/// are all words pays two tag tests and takes no call.
fn lift_around(
    f: &mut FnEmit,
    emitted: &mut [String],
    lifted: &[usize],
    rewind: &mut dyn FnMut(&mut FnEmit, &mut [String]),
) {
    if lifted.is_empty() {
        rewind(f, emitted);
        return;
    }
    let mut any: Option<String> = None;
    for &j in lifted {
        let t = inline_tag(f, &emitted[j]);
        let b = f.tmp();
        f.line_fmt(format_args!("{b} = icmp eq i64 {t}, {K_BIG}"));
        any = Some(match any {
            None => b.to_string(),
            Some(prev) => {
                let o = f.tmp();
                f.line_fmt(format_args!("{o} = or i1 {prev}, {b}"));
                o.to_string()
            }
        });
    }
    let any = any.expect("a lift lifts something");
    let n = lifted.len();
    let arr = match n {
        1 => String::new(),
        _ => {
            let arr = f.tmp();
            f.line_fmt(format_args!("{arr} = alloca [{n} x %KValue]"));
            arr.to_string()
        }
    };
    let sets: Vec<Set> = lifted.iter().map(|&j| f.set_of(&emitted[j])).collect();
    // Moves the lifted arguments through a runtime call, and answers what
    // came back: one by value, several through `arr`.
    let through = |f: &mut FnEmit, values: &[String], call: &str| -> Vec<String> {
        if n == 1 {
            let v = f.tmp();
            f.line_fmt(format_args!("{v} = call %KValue @{call}1(%KValue {})", values[0]));
            return vec![v.to_string()];
        }
        for (k, v) in values.iter().enumerate() {
            let at = f.tmp();
            f.line_fmt(format_args!(
                "{at} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {k}"
            ));
            f.line_fmt(format_args!("store %KValue {v}, ptr {at}"));
        }
        f.line_fmt(format_args!("call void @{call}(ptr {arr}, i64 {n})"));
        (0..n)
            .map(|k| {
                let at = f.tmp();
                f.line_fmt(format_args!(
                    "{at} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {k}"
                ));
                let v = f.tmp();
                f.line_fmt(format_args!("{v} = load %KValue, ptr {at}"));
                v.to_string()
            })
            .collect()
    };
    // merges each lifted argument from the two ways into `at`
    let merge =
        |f: &mut FnEmit, emitted: &mut [String], moved: &[String], from: &str, via: &str| {
            for (k, &j) in lifted.iter().enumerate() {
                let v = f.tmp();
                f.line_fmt(format_args!(
                    "{v} = phi %KValue [ {}, %{from} ], [ {}, %{via} ]",
                    emitted[j], moved[k]
                ));
                f.record(&v, sets[k]);
                emitted[j] = v.to_string();
            }
        };
    let out = f.label();
    let mid = f.label();
    let before = f.cur_label.clone();
    f.line_fmt(format_args!("br i1 {any}, label %{out}, label %{mid}"));
    f.start_block(&out);
    let current: Vec<String> = lifted.iter().map(|&j| emitted[j].clone()).collect();
    let moved = through(f, &current, "k_big_lift_out");
    f.line_fmt(format_args!("br label %{mid}"));
    f.start_block(&mid);
    merge(f, emitted, &moved, &before, &out);
    rewind(f, emitted);
    let rewound = f.cur_label.clone();
    let back = f.label();
    let done = f.label();
    f.line_fmt(format_args!("br i1 {any}, label %{back}, label %{done}"));
    f.start_block(&back);
    let current: Vec<String> = lifted.iter().map(|&j| emitted[j].clone()).collect();
    let moved = through(f, &current, "k_big_lift_in");
    f.line_fmt(format_args!("br label %{done}"));
    f.start_block(&done);
    merge(f, emitted, &moved, &rewound, &back);
}

/// Runs `rewind` unless one of the `failing` arguments is an err, and merges
/// what the rewind changed back into `emitted`. Inference lets a slot that may
/// hold a failure cross a rewind as a number does, because the err path never
/// reads the slot again. The err does need its storage, though: one built in
/// the lap lies above the mark, and the callee answers it by adding a hop,
/// which the arena places over the record the err wraps. The carry path skips
/// the rewind on an err in a carried slot; this is the same test for the
/// arguments it does not carry.
fn unless_failing(
    f: &mut FnEmit,
    emitted: &mut [String],
    failing: &[usize],
    rewind: &mut dyn FnMut(&mut FnEmit, &mut [String]),
) {
    if failing.is_empty() {
        rewind(f, emitted);
        return;
    }
    let mut any: Option<String> = None;
    for &j in failing {
        let t = inline_tag(f, &emitted[j]);
        let b = f.tmp();
        f.line_fmt(format_args!("{b} = icmp eq i64 {t}, {K_ERR}"));
        any = Some(match any {
            None => b.to_string(),
            Some(prev) => {
                let o = f.tmp();
                f.line_fmt(format_args!("{o} = or i1 {prev}, {b}"));
                o.to_string()
            }
        });
    }
    let any = any.expect("a failing argument was named");
    let run = f.label();
    let done = f.label();
    let before = f.cur_label.clone();
    f.line_fmt(format_args!("br i1 {any}, label %{done}, label %{run}"));
    f.start_block(&run);
    let old: Vec<String> = emitted.to_vec();
    rewind(f, emitted);
    let ran = f.cur_label.clone();
    f.line_fmt(format_args!("br label %{done}"));
    f.start_block(&done);
    for j in 0..emitted.len() {
        if emitted[j] == old[j] {
            continue;
        }
        let set = f.set_of(&emitted[j]);
        let v = f.tmp();
        f.line_fmt(format_args!(
            "{v} = phi %KValue [ {}, %{before} ], [ {}, %{ran} ]",
            old[j], emitted[j]
        ));
        f.record(&v, set);
        emitted[j] = v.to_string();
    }
}

/// An `i1` saying both tags are the int tag, 0. A tag already known to be 0,
/// a literal's or an unboxed parameter's, needs no compare, and two such
/// need no `and`: `n + 1` used to write `icmp eq i64 0, 0` and an `and` for
/// the literal on every addition.
fn both_ints(f: &mut FnEmit, ta: &str, tb: &str) -> String {
    let mut tests: Vec<String> = Vec::new();
    for tag in [ta, tb] {
        if tag != "0" {
            let t = f.tmp();
            f.line_fmt(format_args!("{t} = icmp eq i64 {tag}, 0"));
            tests.push(t.to_string());
        }
    }
    match tests.as_slice() {
        [] => "true".to_string(),
        [one] => one.clone(),
        [a, b] => {
            let both = f.tmp();
            f.line_fmt(format_args!("{both} = and i1 {a}, {b}"));
            both.to_string()
        }
        _ => unreachable!("two tags make at most two tests"),
    }
}

/// How many times `name` appears in `line` as a whole operand.
fn count_operand(line: &str, name: &str) -> usize {
    let bytes = line.as_bytes();
    line.match_indices(name)
        .filter(|(at, _)| {
            let after = bytes.get(at + name.len()).copied().unwrap_or(b' ');
            !(after.is_ascii_alphanumeric() || after == b'_' || after == b'.')
        })
        .count()
}

/// One read of a byte run: `x[p + k] == c` or `x[p] == c`, non-strict, with
/// `c` a byte literal and `k` a small literal. Answers `(x, p, k, c)`.
fn byte_read(e: &Expr) -> Option<(&str, &str, i64, i64)> {
    use num_traits::ToPrimitive;
    let Expr::BinOp { op: "==", lhs, rhs, .. } = e else { return None };
    let Expr::Index { base, index, strict: false, .. } = &**lhs else { return None };
    let Expr::Ident(x, _, _) = &**base else { return None };
    let Expr::Int(c, _) = &**rhs else { return None };
    let c = c.to_i64().filter(|c| (0..=255).contains(c))?;
    let (p, k) = match &**index {
        Expr::Ident(p, _, _) => (p, 0),
        Expr::BinOp { op: "+", lhs, rhs, .. } => {
            let Expr::Ident(p, _, _) = &**lhs else { return None };
            let Expr::Int(k, _) = &**rhs else { return None };
            (p, k.to_i64().filter(|k| k.abs() < 1 << 20)?)
        }
        _ => return None,
    };
    Some((x.as_str(), p.as_str(), k, c))
}

/// The bytes and the position a run reads, and each read's offset and byte.
type ByteRun<'e> = (&'e str, &'e str, Vec<(i64, i64)>);

/// A conjunction of `byte_read`s of the same two names. `a and b` desugars to
/// `if a b false`, and `and` groups to the left, so `r1 and r2 and r3` is
/// `if (if r1 r2 false) r3 false`; either side of an `if` may be another. Two
/// reads at least; one is an ordinary compare.
fn byte_run(args: &[Expr]) -> Option<ByteRun<'_>> {
    fn conj<'e>(e: &'e Expr, out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if let Some(read) = byte_read(e) {
            out.push(read);
            return Some(());
        }
        let Expr::App { head, args, piped: false, .. } = e else { return None };
        if !matches!(&**head, Expr::Ident(n, _, _) if n == "if") {
            return None;
        }
        both(args, out)
    }
    fn both<'e>(args: &'e [Expr], out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if args.len() != 3 || !matches!(&args[2], Expr::Ident(n, _, _) if n == "false") {
            return None;
        }
        conj(&args[0], out)?;
        conj(&args[1], out)
    }
    let mut reads = Vec::new();
    both(args, &mut reads)?;
    let (x, p, _, _) = *reads.first()?;
    if reads.len() < 2 || reads.iter().any(|r| (r.0, r.1) != (x, p)) {
        return None;
    }
    Some((x, p, reads.iter().map(|r| (r.2, r.3)).collect()))
}

/// The operands of an argument that ran on words, and the flags that say
/// whether any of its operations overflowed.
type WordFlags = Option<(Vec<String>, Vec<String>)>;

fn word_arith(op: &str) -> bool {
    matches!(op, "+" | "-" | "*" | "/" | "%")
}

fn arith_ops(e: &Expr) -> usize {
    match e {
        Expr::BinOp { op, lhs, rhs, .. } if word_arith(op) => 1 + arith_ops(lhs) + arith_ops(rhs),
        _ => 0,
    }
}

/// The operands of a word run, in the order the operations would evaluate
/// them.
fn arith_leaves<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match e {
        Expr::BinOp { op, lhs, rhs, .. } if word_arith(op) => {
            arith_leaves(lhs, out);
            arith_leaves(rhs, out);
        }
        _ => out.push(e),
    }
}

/// The operations of a word run on raw words. Each one that can leave the
/// word, or divide by zero or by minus one, adds its flag to `flags`; the
/// answer is meaningful only when none is set.
///
/// Given a `slow` label, a flag is a branch there instead, taken where the
/// operation stands. An `or` of the flags made x86 keep each one in a
/// register with `seto` and test the `or` at the end; a branch after each
/// operation is a `jo` on the flag the operation already set.
fn word_run(
    f: &mut FnEmit,
    e: &Expr,
    words: &[String],
    at: &mut usize,
    flags: &mut Vec<String>,
    slow: Option<&str>,
) -> String {
    let Expr::BinOp { op, lhs, rhs, .. } = e else {
        *at += 1;
        return words[*at - 1].clone();
    };
    if !word_arith(op) {
        *at += 1;
        return words[*at - 1].clone();
    }
    let a = word_run(f, lhs, words, at, flags, slow);
    let b = word_run(f, rhs, words, at, flags, slow);
    let t = f.tmp();
    match *op {
        "+" | "-" | "*" => {
            let intrinsic = match *op {
                "+" => "llvm.sadd.with.overflow.i64",
                "-" => "llvm.ssub.with.overflow.i64",
                _ => "llvm.smul.with.overflow.i64",
            };
            let pair = f.tmp();
            f.line_fmt(format_args!("{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {a}, i64 {b})"));
            f.line_fmt(format_args!("{t} = extractvalue {{ i64, i1 }} {pair}, 0"));
            let over = f.tmp();
            f.line_fmt(format_args!("{over} = extractvalue {{ i64, i1 }} {pair}, 1"));
            flag(f, over.to_string(), flags, slow);
        }
        _ => {
            if !matches!(b.parse::<i64>(), Ok(d) if d != 0 && d != -1) {
                let zero = f.tmp();
                f.line_fmt(format_args!("{zero} = icmp eq i64 {b}, 0"));
                let minus = f.tmp();
                f.line_fmt(format_args!("{minus} = icmp eq i64 {b}, -1"));
                let edge = f.tmp();
                f.line_fmt(format_args!("{edge} = or i1 {zero}, {minus}"));
                flag(f, edge.to_string(), flags, slow);
                // the instruction must not trap on the path it does not take
                let safe = f.tmp();
                f.line_fmt(format_args!("{safe} = select i1 {edge}, i64 1, i64 {b}"));
                let insn = if *op == "%" { "srem" } else { "sdiv" };
                f.line_fmt(format_args!("{t} = {insn} i64 {a}, {safe}"));
                return t.to_string();
            }
            let insn = if *op == "%" { "srem" } else { "sdiv" };
            f.line_fmt(format_args!("{t} = {insn} i64 {a}, {b}"));
        }
    }
    t.to_string()
}

/// A word run's flag: kept for the caller, or a branch to `slow` here.
fn flag(f: &mut FnEmit, over: String, flags: &mut Vec<String>, slow: Option<&str>) {
    match slow {
        Some(slow) => {
            let on = f.label();
            f.line_fmt(format_args!("br i1 {over}, label %{slow}, label %{on}"));
            f.start_block(&on);
            flags.push(over);
        }
        None => flags.push(over),
    }
}

fn inline_tag(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 0) {
        return word.to_string();
    }
    if let Some((tag, _)) = f.known_words.get(value) {
        return tag.clone();
    }
    let t = f.tmp();
    f.line_fmt(format_args!("{t} = extractvalue %KValue {value}, 0"));
    t.to_string()
}

fn inline_payload(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 1) {
        return word.to_string();
    }
    if let Some((_, payload)) = f.known_words.get(value) {
        return payload.clone();
    }
    let t = f.tmp();
    f.line_fmt(format_args!("{t} = extractvalue %KValue {value}, 1"));
    t.to_string()
}

/// Branch to `hit` when a 1-based index lands inside the container whose
/// length `len_ptr` points at, and to `miss` when it does not, one signed
/// compare at a time. The length is loaded between the two, which keeps LLVM
/// from folding the pair back into flags and an `and`. The signed spelling
/// is the one a guard such as `return acc if n < 1 or length bs < n` proves,
/// so LLVM can drop both where one ran upstream.
fn index_in_range(f: &mut FnEmit, idx: &str, len_ptr: &str, hit: &str, miss: &str) {
    let ge1 = f.tmp();
    f.line_fmt(format_args!("{ge1} = icmp sge i64 {idx}, 1"));
    let above = f.label();
    f.line_fmt(format_args!("br i1 {ge1}, label %{above}, label %{miss}"));
    f.start_block(&above);
    let len = f.tmp();
    f.line_fmt(format_args!("{len} = load i64, ptr {len_ptr}"));
    assume_length(f, &len);
    let le_len = f.tmp();
    f.line_fmt(format_args!("{le_len} = icmp sle i64 {idx}, {len}"));
    f.line_fmt(format_args!("br i1 {le_len}, label %{hit}, label %{miss}"));
}

/// A length read out of a container is never negative, and saying so lets
/// LLVM merge a `1 <= i <= len` test into one unsigned compare where the
/// signed pair also stays visible to whatever proved it true upstream. It is
/// also far below the largest word, since it counts what memory holds, so
/// an `i` a guard has kept at or under it can be stepped by one without an
/// overflow test: `return acc if length xs < i` before `i + 1`.
fn assume_length(f: &mut FnEmit, len: &str) {
    if !f.assumes() {
        return;
    }
    let ok = f.tmp();
    f.line_fmt(format_args!("{ok} = icmp ult i64 {len}, {}", 1i64 << 62));
    f.line_fmt(format_args!("call void @llvm.assume(i1 {ok})"));
}

/// Pack an int and a value into a `%parsed`'s three words: both tags in the
/// first, the value's tag in its low byte and the int's above it, then the
/// value's payload, then the int's word. A bignum's word is its pointer and
/// crosses as one, so every int packs. The first two words are the value's
/// own tag and payload whenever the int is a word, which is what lets a
/// failure ride in the same shape.
fn pack_words(f: &mut FnEmit, pos: &str, value: &str) -> (String, String, String) {
    let ptag = f.tmp();
    f.line_fmt(format_args!("{ptag} = extractvalue %KValue {pos}, 0"));
    let n = f.tmp();
    f.line_fmt(format_args!("{n} = extractvalue %KValue {pos}, 1"));
    let vtag = f.tmp();
    f.line_fmt(format_args!("{vtag} = extractvalue %KValue {value}, 0"));
    let vpay = f.tmp();
    f.line_fmt(format_args!("{vpay} = extractvalue %KValue {value}, 1"));
    let high = f.tmp();
    f.line_fmt(format_args!("{high} = shl i64 {ptag}, 8"));
    let tags = f.tmp();
    f.line_fmt(format_args!("{tags} = or i64 {high}, {vtag}"));
    (tags.to_string(), vpay.to_string(), n.to_string())
}

/// Read a `%parsed`'s three words back as the int's tag and word, the
/// value's tag and the value's payload. The int's tag is a word's or a
/// bignum's, and saying so lets LLVM drop the thunk and failure tests a
/// field read would otherwise make of it.
fn unpack_words(f: &mut FnEmit, w0: &str, w1: &str, w2: &str) -> (String, String, String, String) {
    let ptag = f.tmp();
    f.line_fmt(format_args!("{ptag} = lshr i64 {w0}, 8"));
    if f.assumes() {
        let stray = f.tmp();
        f.line_fmt(format_args!("{stray} = and i64 {ptag}, {}", !K_BIG));
        let int = f.tmp();
        f.line_fmt(format_args!("{int} = icmp eq i64 {stray}, 0"));
        f.line_fmt(format_args!("call void @llvm.assume(i1 {int})"));
    }
    let vtag = f.tmp();
    f.line_fmt(format_args!("{vtag} = and i64 {w0}, 255"));
    (ptag.to_string(), w2.to_string(), vtag.to_string(), w1.to_string())
}

/// A `%parsed` built from its three words.
fn parsed_of(f: &mut FnEmit, w0: &str, w1: &str, w2: &str) -> String {
    let a = f.tmp();
    f.line_fmt(format_args!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
    let b = f.tmp();
    f.line_fmt(format_args!("{b} = insertvalue %parsed {a}, i64 {w1}, 1"));
    let p = f.tmp();
    f.line_fmt(format_args!("{p} = insertvalue %parsed {b}, i64 {w2}, 2"));
    p.to_string()
}

/// Whether a value is not a failure, as one compare of its tag against the
/// err tag.
fn inline_not_failure(f: &mut FnEmit, value: &str) -> String {
    let set = f.set_of(value);
    // An EMPTY set is not a proof. `group_param_set` answers 0 for a parameter
    // the inference reached no shapes for, and 0 satisfies every `& mask == 0`
    // test written about it -- so the first draft of this fold read "nothing is
    // known" as "proved to be a boolean" and turned the hop below into `true`.
    // A group whose argument is a failure then dispatched instead of hopping
    // and died on `no overload matches these arguments`. The micro corpus
    // caught it: a_construction_merges_its_failures lost its third line.
    if set != 0 && set & !infer::BOOL == 0 {
        return "true".to_string();
    }
    not_failure_test(f, value)
}

/// The same test with no fold, for the one place a set cannot speak: a block
/// reached BECAUSE a value is outside the set recorded for it.
///
/// It was a call to an alwaysinline `k_not_failure`, then a compare of what
/// that returned. Every module paid for the inliner to open the call, and the
/// dev tier, which does no other optimising, kept the widening and the second
/// compare as well. The test is the one the runtime's `k_not_failure` makes,
/// against the `K_ERR` build.rs reads from the runtime's enum.
fn not_failure_test(f: &mut FnEmit, value: &str) -> String {
    let tag = inline_tag(f, value);
    if let Ok(known) = tag.parse::<i64>() {
        return (known != K_ERR).to_string();
    }
    let ok = f.tmp();
    f.line_fmt(format_args!("{ok} = icmp ne i64 {tag}, {K_ERR}"));
    ok.to_string()
}
