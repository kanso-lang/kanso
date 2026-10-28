// One function's emission state: the IR it is writing, its temporaries and
// labels, the cells it must release and what it knows about each value, with
// the methods that keep them. codegen.rs reads this file with include! where
// these items stood, so they stay in the codegen module.

/// The frame epilogue: release each releasable cell unless the outgoing
/// value IS that cell (the returned-thunk case escapes upward, counted by
/// the runtime). Returns the value register to hand to `ret` — threading
/// through the helper keeps the IR linear.
fn release_cells(f: &mut FnEmit, value: &str) -> String {
    if f.lazy_cells.is_empty() || f.parsed.contains_key(value) {
        return value.to_string();
    }
    let cells = f.lazy_cells.clone();
    let mut v = value.to_string();
    for cell in cells {
        let t = f.tmp();
        f.line_fmt(format_args!(
            "{t} = call %KValue @k_thunk_release_unless(%KValue {cell}, %KValue {v})"
        ));
        v = t.to_string();
    }
    v
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
    sets: HashMap<String, Set>,
    /// Temps carrying the by-value %parsed type rather than a boxed KValue.
    /// Operands living in the by-value convention, and the record type
    /// each one holds — boxing one back needs to name its type and its id.
    parsed: crate::hash::Map<String, (String, i64)>,
    /// Err-origin prefix "{fn lazy_cells: Vec::new(), } at {file}" for the declaration being emitted.
    origin_prefix: String,
    /// Source file of the declaration being emitted, for keying push sites.
    file: std::sync::Arc<str>,
    /// LLVM return type of the function being emitted: `%parsed` or `%KValue`.
    ret_ty: String,
    /// Dispatcher group being emitted, for recognizing self-tail-calls.
    group: String,
    arity: usize,
    /// The group whose cycle this code is on for `crate::deep`: its own for a
    /// group's body, and for a lambda or a thunk the one whose body made it.
    deep_from: (String, usize),
    /// A lambda's or a thunk's body, which runs where it is called.
    lifted: bool,
    /// Whether the current arm is a bare enrollment clone: library plumbing
    /// wearing an unqualified name, which the cohort license must not read
    /// as user code.
    synthetic: bool,
    /// The package this arm belongs to. An arm cannot see an err its own
    /// package raised, and this is the side of that comparison the compiler
    /// knows; the err carries the other.
    hako: String,
    /// Registers of releasable lazy cells born in this body; every return
    /// path releases each unless the result aliases it.
    lazy_cells: Vec<String>,
    /// Stack slots the body asked for, held back for the entry block.
    entry_allocas: Vec<String>,
    /// Whether a byte view's header lives in this frame, which turns the
    /// body's tail calls into plain ones: `framed_views` says why.
    frame_held: bool,
    /// A non-strict byte index builds a `%KValue` — the byte, or none — and a
    /// byte discriminator immediately collapses it back to one i64. The box
    /// is a phi over a struct, and LLVM will not sink the `extractvalue` into
    /// the diamond's two arms, so the collapse survives to machine code as
    /// four instructions on the hot path. This maps such a box to the raw
    /// i64 phi emitted beside it, so the crossing can take the raw form and
    /// leave the box for the dead-code pass.
    raw_byte: crate::hash::Map<String, String>,
    /// Whether the hot predicates are called in their two-word forms, which
    /// the dev tier's instruction selector can lower where it cannot lower a
    /// call passing a `%KValue`. It also marks a body no optimizer reads (see
    /// `FnEmit::assumes`).
    words: bool,
    /// Values whose two words are already in hand, tag then payload. An
    /// unboxed parameter crosses as a raw i64 and is boxed on entry, and every
    /// read of its tag or payload used to take it back apart with an
    /// `extractvalue`; the tag is 0 and the payload is the argument itself.
    known_words: crate::hash::Map<String, (String, String)>,
    /// In a twin, the tail call into the general body with this body's own
    /// arguments, and its return type. `bail_on` takes it when a word
    /// overflows, so a sum the twin computes stays a word: the general body
    /// starts again from the arguments and computes the bignum itself.
    general: Option<(String, String)>,
    /// The buffer `line_fmt` formats into, kept between lines so a formatted
    /// line costs no allocation of its own.
    scratch: String,
    /// Whether this body has written anything that running it a second time
    /// would write again: a store, or a call to anything but an intrinsic or
    /// a read-only predicate. A bail re-runs the body from its entry, so it
    /// is taken only while this is false.
    effects: bool,
    /// `effects` as it stood when a switch dispatcher finished its entry. A
    /// switch's arms are reached only from the switch, never from each other,
    /// so each arm starts from this rather than from whatever the arm emitted
    /// before it wrote.
    arms_effects: Option<bool>,
}
/// Whether an emitted line changes something a second run of the body would
/// see: a store, or a call to anything but an LLVM intrinsic or one of the
/// predicates that only read their operand. An allocation counts, which is
/// cautious: a second run would allocate again and leak nothing, but the
/// runtime's region and beat calls change state the same way and are not
/// told apart here.
fn writes_state(line: &str) -> bool {
    const READS: [&str; 6] = [
        "@k_b_at_fast(",
        "@k_truthy(",
        "@k_truthy_w(",
        "@k_str_lit_fast(",
        "@k_check_rec_fast(",
        "@k_check_rec_fast_w(",
    ];
    // a site stamp is the one store a second run repeats harmlessly: it
    // names a place, and the rerun names the same one
    (line.trim_start().starts_with("store ") && !line.contains(", ptr @k_site"))
        || (line.contains("call ")
            && !line.contains("@llvm.")
            && !READS.iter().any(|r| line.contains(r)))
}

/// Whether a line the emitters wrote is a stack slot, asked at the ONE place
/// the needle can be.
///
/// A slot reads `%name = alloca <type>`, and `%name` holds no space, so
/// ` = alloca ` begins at the line's FIRST space or it is not there at all.
/// Reading the whole line to learn that is what `FnEmit::write` used to do,
/// and it was the emitter's fourth-largest frame: 18.2 million instructions
/// over 144,261 lines on the build profile, about 126 a line to answer a
/// question the first seven bytes settle. Measured on runbench's 36,086
/// emitted lines, all 235 slots put the needle between offsets five and
/// seven, and each of those is that line's first space.
///
/// The check stays in `write` rather than at the seven sites that emit an
/// alloca, for the reason the comment there gives: the rule has to hold for a
/// site nobody has written yet.
/// `tests/a_stack_slot_is_found_where_the_first_space_is.rs` compiles the
/// corpora and asserts this reading and the whole-line one agree for every
/// line, so a line that ever carried the needle elsewhere is caught rather
/// than quietly written into the wrong block.
pub fn is_a_stack_slot(text: &str) -> bool {
    // The space ends the line's first word a few bytes in, nearer than a
    // searcher's setup and a call into memchr would pay for; this runs on
    // every line the emitter writes.
    match text.bytes().position(|b| b == b' ') {
        Some(at) => text[at..].starts_with(" = alloca "),
        None => false,
    }
}

impl FnEmit {
    /// Whether to write `llvm.assume` and the compare that feeds it. Only a
    /// release module is optimized: the dev tier compiles at -O0, where an
    /// assume is dropped unread, and the tab's translator skips the call but
    /// still translates and runs the compare. Both build their bodies with
    /// `words`.
    fn assumes(&self) -> bool {
        !self.words
    }

    fn new(words: bool) -> Self {
        FnEmit {
            out: String::new(),
            tmp: 0,
            label: 0,
            cur_label: "entry".to_string(),
            versions: HashMap::default(),
            sets: HashMap::default(),
            parsed: crate::hash::Map::default(),
            origin_prefix: String::new(),
            hako: String::new(),
            file: crate::ast::unstamped(),
            ret_ty: "%KValue".to_string(),
            group: String::new(),
            synthetic: false,
            arity: 0,
            deep_from: (String::new(), 0),
            lifted: false,
            lazy_cells: Vec::new(),
            entry_allocas: Vec::new(),
            frame_held: false,
            raw_byte: crate::hash::Map::default(),
            words,
            known_words: crate::hash::Map::default(),
            general: None,
            scratch: String::new(),
            effects: false,
            arms_effects: None,
        }
    }

    /// Writes `{r} = {call}`, where `call` asks one of the hot predicates about
    /// a `%KValue` in the form a release module inlines. The dev tier asks the
    /// two-word form instead (`k_truthy_w` and `k_check_rec_fast_w` in
    /// DECLARES), with the value's words pulled out
    /// first and passed as scalars. At -O0 clang's fast instruction selector
    /// lowers a call only when every argument is a scalar, so each call passing
    /// a `%KValue` went to the slow selector on its own: 216 of them on the
    /// codegen corpus. A release module never calls the two-word forms, so it
    /// never carries them.
    fn predicate(&mut self, r: &str, call: String) {
        const FORMS: [(&str, &str, bool); 2] = [
            ("call i64 @k_truthy(%KValue ", "k_truthy_w", true),
            ("call i64 @k_check_rec_fast(%KValue ", "k_check_rec_fast_w", true),
        ];
        if !self.words {
            self.line_fmt(format_args!("{r} = {call}"));
            return;
        }
        let split = FORMS.iter().find_map(|(prefix, to, pay)| {
            let rest = call.strip_prefix(prefix)?;
            // A constant operand spells its own commas; it keeps the old form.
            let end = rest.find([',', ')']).filter(|_| rest.starts_with('%'))?;
            Some((*to, *pay, rest[..end].to_string(), rest[end..].to_string()))
        });
        match split {
            Some((to, pay, value, rest)) => {
                let tag = self.tmp();
                self.line_fmt(format_args!("{tag} = extractvalue %KValue {value}, 0"));
                let words = match pay {
                    true => {
                        let p = self.tmp();
                        self.line_fmt(format_args!("{p} = extractvalue %KValue {value}, 1"));
                        format!("i64 {tag}, i64 {p}")
                    }
                    false => format!("i64 {tag}"),
                };
                self.line_fmt(format_args!("{r} = call i64 @{to}({words}{rest}"));
            }
            None => self.line_fmt(format_args!("{r} = {call}")),
        }
    }

    fn tmp(&mut self) -> Numbered {
        self.tmp += 1;
        Numbered::new("%t", self.tmp)
    }

    /// A call to a runtime function the release build's hot unit holds a
    /// copy of, and the `%KValue` it answers. The link inlines the copy only
    /// when the call's type is the one clang gave the definition: a C struct
    /// comes back as `{ i64, i64 }`, which LLVM keeps apart from `%KValue`. So
    /// the call takes that type and the two words are put back into a
    /// `%KValue` after it. `callee` is the name and the arguments.
    fn call_pair(&mut self, callee: std::fmt::Arguments) -> Numbered {
        let raw = self.tmp();
        self.line_fmt(format_args!("{raw} = call {{ i64, i64 }} {callee}"));
        let tag = self.tmp();
        self.line_fmt(format_args!("{tag} = extractvalue {{ i64, i64 }} {raw}, 0"));
        let pay = self.tmp();
        self.line_fmt(format_args!("{pay} = extractvalue {{ i64, i64 }} {raw}, 1"));
        let half = self.tmp();
        self.line_fmt(format_args!("{half} = insertvalue %KValue undef, i64 {tag}, 0"));
        let t = self.tmp();
        self.line_fmt(format_args!("{t} = insertvalue %KValue {half}, i64 {pay}, 1"));
        t
    }

    fn label(&mut self) -> Numbered {
        self.label += 1;
        Numbered::new("L", self.label)
    }

    /// Only a carried argument slot reads the two-word convention. Every other
    /// consumer names its operand as a `%KValue`, and a `%parsed` reaching one
    /// is invalid IR the host's clang refuses. Repairing here rather than at
    /// each consumer is what makes the rule hold for consumers nobody has
    /// written yet: five separate sites were fixed one at a time before this,
    /// and the sixth would have shipped the same way.
    /// `line` for a formatted line, written into a buffer the emitter keeps
    /// rather than a string made for it. A line that boxes an operand calls
    /// back into the emitter, which may format a line of its own; it finds
    /// the buffer taken and uses a fresh one.
    fn line_fmt(&mut self, args: std::fmt::Arguments) {
        let mut text = std::mem::take(&mut self.scratch);
        text.clear();
        let _ = std::fmt::Write::write_fmt(&mut text, args);
        self.line(&text);
        self.scratch = text;
    }

    fn line(&mut self, text: &str) {
        let text = self.boxing_any_parsed_operand(text);
        if self.general.is_some() {
            self.note_effects(&text);
        }
        self.write(&text);
    }

    /// Kept out of `line`, which every emitter calls: only a twin asks.
    #[inline(never)]
    fn note_effects(&mut self, text: &str) {
        if !self.effects && writes_state(text) {
            self.effects = true;
        }
    }

    /// Branches to the general body when `overflow` is set, and carries on in
    /// a block where it is not. False, writing nothing, when this is not a
    /// twin or when the body has already written something a second run
    /// would write again.
    fn can_bail(&self) -> bool {
        self.general.is_some() && !self.effects
    }

    fn bail_on(&mut self, overflow: &str) -> bool {
        if !self.can_bail() {
            return false;
        }
        let ok = self.label();
        let out = self.label();
        self.line_fmt(format_args!("br i1 {overflow}, label %{out}, label %{ok}"));
        self.bail_block(&out);
        self.start_block(&ok);
        true
    }

    /// The block `bail_on` branches to, for a caller that branched to it
    /// itself: it starts the general body over. The block that was open
    /// must already be closed.
    fn bail_block(&mut self, out: &str) {
        let Some((ret, call)) = self.general.clone() else {
            return;
        };
        self.start_block(out);
        let r = self.tmp();
        let kind = if self.frame_held { "call" } else { "musttail call" };
        self.write(&format!("{r} = {kind} {call}"));
        self.write(&format!("ret {ret} {r}"));
    }

    /// Every stack slot the body asks for is held back and written at the top
    /// of the entry block, whichever block asked for it. An alloca standing in
    /// a later block is a dynamic stack object to LLVM: the function keeps a
    /// frame pointer and restores `rsp` through it on every return, and the
    /// slot is claimed again on each pass. Sixty-eight of the decoder's
    /// seventy-three slots stood that way and the decode paid 8.78% for them.
    /// Diverting here rather than at each emitter is what makes the rule hold
    /// for a site nobody has written yet.
    ///
    /// A slot hoisted out of a loop body is shared between passes rather than
    /// claimed afresh. Every consumer of one copies out of it before it
    /// returns — `k_rec` and `k_closure` memcpy, `k_list_lit` and `k_map_lit`
    /// build their own storage — so nothing survives a pass to read the next
    /// pass's bytes.
    fn write(&mut self, text: &str) {
        if is_a_stack_slot(text) {
            self.entry_allocas.push(text.to_string());
            return;
        }
        self.out.push_str("  ");
        self.out.push_str(text);
        self.out.push('\n');
    }

    /// The function's body: what the emitters wrote, with the stack slots at
    /// the head of the entry block so each one dominates its uses.
    fn body(&self) -> String {
        let unboxed = self.without_unread_reboxes();
        let out = unboxed.as_deref().unwrap_or(&self.out);
        if self.entry_allocas.is_empty() {
            return out.to_string();
        }
        let mut head = String::new();
        let mut rest = out;
        if let Some(end) = out.find('\n') {
            let first = &out[..end];
            if first.ends_with(':') && !first.starts_with(' ') {
                head.push_str(first);
                head.push('\n');
                rest = &out[end + 1..];
            }
        }
        for slot in &self.entry_allocas {
            head.push_str("  ");
            head.push_str(slot);
            head.push('\n');
        }
        head.push_str(rest);
        head
    }

    /// The body without the boxing of an unboxed parameter nothing reads.
    /// Every read of such a parameter's words is answered from
    /// `known_words`, so in a function that only compares and passes it on,
    /// the `insertvalue` on entry builds a value no line names.
    fn without_unread_reboxes(&self) -> Option<String> {
        let mut out: Option<String> = None;
        for (value, (_, payload)) in &self.known_words {
            let text = out.as_deref().unwrap_or(&self.out);
            let line = format!(
                "  {value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {payload}, 1\n"
            );
            let Some(at) = text.find(&line) else { continue };
            let named = text.match_indices(value.as_str()).any(|(i, _)| {
                i != at + 2
                    && !text[i + value.len()..]
                        .bytes()
                        .next()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            });
            if !named {
                let mut kept = text[..at].to_string();
                kept.push_str(&text[at + line.len()..]);
                out = Some(kept);
            }
        }
        out
    }

    fn boxing_any_parsed_operand<'t>(&mut self, text: &'t str) -> std::borrow::Cow<'t, str> {
        // Most lines name no carried operand, and those go through as they
        // are: copying each into a string of its own was one allocation per
        // line the emitter wrote.
        if self.parsed.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        // Sorted because the map's order is randomized per process, and a line
        // naming two carried operands would otherwise box them in an order
        // that differs between builds of the same program.
        let mut carried: Vec<String> =
            self.parsed.keys().filter(|t| named_as_a_value(text, t)).cloned().collect();
        if carried.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        carried.sort();
        std::borrow::Cow::Owned(carried.iter().fold(text.to_string(), |acc, t| {
            let boxed = self.box_parsed(t);
            let named = format!("%KValue {t}");
            let boxed_as = format!("%KValue {boxed}");
            let inner = acc
                .replace(&format!("{named},"), &format!("{boxed_as},"))
                .replace(&format!("{named})"), &format!("{boxed_as})"));
            match inner.strip_suffix(&named) {
                Some(head) => format!("{head}{boxed_as}"),
                None => inner,
            }
        }))
    }

    /// Undo the by-value convention: rebuild the record the three words hold.
    /// The type is whatever produced the value, which the escape analysis
    /// already knows, because only a returnable type is ever in this shape.
    /// A failure rides in the first two words and comes back as itself: the
    /// runtime asks before it builds, which the inline build here did not.
    fn box_parsed(&mut self, e: &str) -> String {
        let (_, id) = self.parsed[e];
        let w0 = self.tmp();
        self.raw(&format!("{w0} = extractvalue %parsed {e}, 0"));
        let w1 = self.tmp();
        self.raw(&format!("{w1} = extractvalue %parsed {e}, 1"));
        let w2 = self.tmp();
        self.raw(&format!("{w2} = extractvalue %parsed {e}, 2"));
        let t = self.tmp();
        self.raw(&format!(
            "{t} = call %KValue @k_parsed_box(i64 {id}, i64 {w0}, i64 {w1}, i64 {w2})"
        ));
        t.to_string()
    }

    /// A line whose operands are already values: the boxing rewrite emits
    /// through here, so a fresh temp is never scanned against itself.
    fn raw(&mut self, text: &str) {
        self.write(text);
    }

    fn start_block(&mut self, label: &str) {
        self.out.push_str(label);
        self.out.push_str(":\n");
        self.cur_label.clear();
        self.cur_label.push_str(label);
        if let Some(e) = self.arms_effects {
            if label
                .strip_prefix("arm")
                .is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()))
            {
                self.effects = e;
            }
        }
    }

    /// Branches on `value` to the label its case names, or to `dflt`. A
    /// release module writes a `switch`, which its optimiser turns into a jump
    /// table. clang's fast instruction selector at -O0 does not lower one, so
    /// a dev module asks the cases in turn with a compare and a branch each,
    /// which it does.
    fn switch_on(&mut self, value: &str, dflt: &str, cases: &[String]) {
        if !self.words {
            self.line_fmt(format_args!(
                "switch i64 {value}, label %{dflt} [\n{}\n  ]",
                cases.join("\n")
            ));
            return;
        }
        for case in cases {
            let (n, target) = case
                .trim()
                .strip_prefix("i64 ")
                .and_then(|c| c.split_once(", label %"))
                .expect("a case reads `i64 N, label %L`");
            let hit = self.tmp();
            self.line_fmt(format_args!("{hit} = icmp eq i64 {value}, {n}"));
            let next = self.label();
            self.line_fmt(format_args!("br i1 {hit}, label %{target}, label %{next}"));
            self.start_block(&next);
        }
        self.line_fmt(format_args!("br label %{dflt}"));
    }

    fn bind(&mut self, name: &str, temp: &str) {
        self.versions.insert(name.to_string(), temp.to_string());
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.versions.get(name).cloned()
    }

    fn record_parsed(&mut self, operand: &str, ty: &str, id: i64) {
        self.parsed.insert(operand.to_string(), (ty.to_string(), id));
    }

    fn is_parsed(&self, operand: &str) -> bool {
        self.parsed.contains_key(operand)
    }

    fn record(&mut self, operand: &str, set: Set) {
        self.sets.insert(operand.to_string(), set);
    }

    fn set_of(&self, operand: &str) -> Set {
        if operand.starts_with("{ i64 0,") {
            return INT;
        }
        if operand == "{ i64 2, i64 0 }" || operand == "{ i64 3, i64 0 }" {
            return infer::BOOL;
        }
        if operand == "{ i64 4, i64 0 }" {
            return NONE;
        }
        if operand == "{ i64 16, i64 0 }" {
            return infer::DONE;
        }
        self.sets.get(operand).copied().unwrap_or(TOP)
    }
}
