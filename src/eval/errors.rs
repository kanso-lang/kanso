// What a failure carries and how it reads: the err value's trace and the
// place it was raised, the field reads on it, and the lines a trace prints.
// eval.rs reads this file with include!, so the items stay in the eval
// module, as the other files under src/eval/ do (kanso#1867). cargo fmt does
// not reach an included file, so CI's format job checks the files under src/'s
// subdirectories itself.

/// An err value carries its propagation trace: the origin baked at the
/// construction site ("{fn} at {file}:{line}"; executor-born errs have none)
/// and one hop per dispatcher failure pass-through. The happy path never
/// touches any of this — trace data lives on the err value only.
#[derive(Debug)]
pub struct ErrInfo {
    pub reason: Value,
    pub origin: Option<Rc<str>>,
    /// The package that RAISED this err, which is what an arm asks about: a
    /// group may not see a failure its own package made. Read off the raising
    /// frame beside `origin`, never off the reason's declaration — a package
    /// can raise an err whose reason type belongs to somebody else, and the
    /// rule turns on who raised it.
    ///
    /// `None` for an err with no frame (the wasm host) and for one merged out
    /// of several, which belongs to no single package. Both always match: the
    /// rule refuses what it can prove is yours, and neither of these is.
    pub hako: Option<Rc<str>>,
    pub hops: Vec<Rc<str>>,
    /// The err this one was wrapped around, kept so a re-raise never
    /// discards what it was told (`wrap_err`).
    pub cause: Option<Rc<ErrInfo>>,
    /// Whether this err's reason is a list *of reasons* rather than one
    /// reason that happens to be a list. Merging is a fold, so three failures
    /// answer three reasons however they were grouped — and `err ["a" "b"]`
    /// stays one reason, which is why the mark cannot be read off the shape.
    pub merged: bool,
}

/// The evaluation frame an expression runs in: the err-origin prefix
/// "{fn} at {file}" a raise stamps on its err, and the package that file
/// belongs to. The two are read off the same declaration and travel together,
/// because an err carrying one without the other is an err whose provenance a
/// match cannot ask about. Absent where no source frame exists (the wasm host).
#[derive(Debug)]
pub struct Site {
    pub prefix: Rc<str>,
    pub hako: Rc<str>,
    /// The declaration's own file, kept whole rather than read back out of
    /// `prefix`. `linear::in_place_pushes` keys its sites by (file, line,
    /// column) and so does the emitter at `codegen.rs`'s push arm, so the
    /// interpreter needs the same three to ask the same question. The `Arc`
    /// is cloned once per declaration, which is what this struct already
    /// costs: it is built by `frame_of` and memoized by pointer.
    pub file: std::sync::Arc<str>,
    /// Whether the file is the standard library's or a fetched hako's, asked
    /// at every tail call that might cross into the library (#2099), so it is
    /// read off the path once here rather than scanned for at each call.
    pub library: bool,
    /// The (line, column) of every push in this file that writes in place,
    /// gathered on the first question; see `Interp::writes_in_place`.
    in_place: std::cell::OnceCell<Set<(usize, usize)>>,
    /// The same for `linear::moved_writes`; see `Interp::moved_here`.
    moved: std::cell::OnceCell<Set<(usize, usize)>>,
    /// The same for the joins `linear::string_builders` finds; see
    /// `Interp::joins_here`.
    joins: std::cell::OnceCell<Set<(usize, usize)>>,
}

pub type Frame = Option<Rc<Site>>;

fn frame_of(decl: &FnDecl) -> Frame {
    Some(Rc::new(Site {
        prefix: Rc::from(format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file)),
        hako: Rc::from(crate::provenance::package_of(&decl.file)),
        file: decl.file.clone(),
        library: crate::provenance::is_library_file(&decl.file),
        in_place: std::cell::OnceCell::new(),
        moved: std::cell::OnceCell::new(),
        joins: std::cell::OnceCell::new(),
    }))
}

/// Where an err was raised, as the two things a raise records: the trace line
/// a report prints and the package to hold responsible. They are produced
/// together so no construction site can set one and forget the other.
#[derive(Clone, Debug, Default)]
pub struct Raised {
    pub at: Option<Rc<str>>,
    pub hako: Option<Rc<str>>,
}

/// Which of the three chain words a step is. They differ only in which
/// channel the callback sees and what becomes of its answer.
#[derive(Clone, Copy, Debug)]
pub enum Word {
    Bind,
    Rescue,
    Annotate,
}

pub fn origin_at(frame: &Frame, span: Span) -> Raised {
    match frame {
        Some(site) => Raised {
            at: Some(Rc::from(format!("{}:{}", site.prefix, span.line))),
            hako: Some(site.hako.clone()),
        },
        None => Raised::default(),
    }
}

/// A failure raised outside any expression's evaluation, at a statement of
/// the body `frame` runs, takes that frame as its place unless it has one.
fn placed(mut e: RuntimeError, frame: &Frame) -> RuntimeError {
    // a span at line 0 was made where no source position was known
    if e.at.is_none() && e.span.line > 0 {
        e.at = origin_at(frame, e.span).at;
    }
    e
}

/// The place a failure that left the library through a call this program
/// wrote by name prints: the library's own line, and under it the program's
/// line that called in (ruled on #2099). `RuntimeError::place` prints `at`
/// after "failed in", so the second line rides in the same string. Only the
/// innermost such call names itself; a failure that already carries one, or
/// whose place is not a library's, passes through unchanged.
fn called_from(mut e: RuntimeError, frame: &Frame, span: Span) -> RuntimeError {
    let Some(site) = frame else { return e };
    if site.library {
        return e;
    }
    if let Some(at) = &e.at {
        let file = at.rsplit_once(" at ").and_then(|(_, rest)| rest.rsplit_once(':'));
        let library = file.is_some_and(|(file, _)| crate::provenance::is_library_file(file));
        if library && !at.contains("\n  called from ") {
            e.at = Some(Rc::from(format!("{at}\n  called from {}:{}", site.file, span.line)));
        }
    }
    e
}

/// Whether a call's head names an insisting form, a function spelled with
/// `!` (ruled on #1975: such a form names its caller's line).
fn insists(head: &Expr) -> bool {
    matches!(head, Expr::Ident(name, _, _) if name.as_str().ends_with('!'))
}

/// A failure the package `hako` raised, pointed at `at` instead of the line
/// inside that package that raised it. Who raised it does not change, so an
/// arm asking that question hears the same answer.
pub fn restamped(v: Value, at: &Rc<str>, hako: &Rc<str>) -> Value {
    match v {
        Value::ErrV(info) if info.hako.as_deref() == Some(&**hako) => {
            Value::ErrV(Rc::new(ErrInfo {
                reason: info.reason.clone(),
                origin: Some(at.clone()),
                hako: info.hako.clone(),
                hops: info.hops.clone(),
                cause: info.cause.clone(),
                merged: info.merged,
            }))
        }
        other => other,
    }
}

pub fn err_value(reason: Value, raised: Raised) -> Value {
    Value::ErrV(Rc::new(ErrInfo {
        reason,
        origin: raised.at,
        hako: raised.hako,
        hops: Vec::new(),
        cause: None,
        merged: false,
    }))
}

/// What an err answers to one of its three readers: `reason` is the value it
/// was raised with, `cause` the err it wrapped or none, `origin` the
/// "{fn} at {file}:{line}" it was born at, or none for an executor-born one.
/// The one place all three engines read an err's parts — the wasm host calls
/// it, and native's `k_err_read` mirrors it arm for arm.
pub fn err_read(info: &ErrInfo, field: &str) -> Value {
    match field {
        "reason" => info.reason.clone(),
        "cause" => match &info.cause {
            Some(cause) => Value::Unboxed(cause.clone()),
            None => Value::NoneV,
        },
        _ => match &info.origin {
            Some(origin) => Value::Str(Text::from(origin.to_string())),
            None => Value::NoneV,
        },
    }
}

/// A byte list (the scanner's `bytes`/`slice` view) as its utf-8 text, so a
/// number can be parsed straight from bytes without first materializing a
/// string. None when the bytes aren't valid utf-8 byte values.
fn bytes_to_str(raw: &[u8]) -> Option<String> {
    String::from_utf8(raw.to_vec()).ok()
}

/// The low byte of a number, which is what the compiled engine reads where a
/// byte is wanted: `payload & 0xff`. Every engine has to truncate the same
/// way or the same program answers two things.
///
/// The byte is read off the magnitude's lowest word. `to_bytes_le` answered
/// the same byte by writing out every byte of the number first, an
/// allocation and a copy on each of 52,806 calls in the interpreted corpus,
/// 1.2% of it.
fn low_byte(n: &Int) -> u8 {
    match n {
        Int::Small(n) => n.unsigned_abs() as u8,
        Int::Big(n) => n.magnitude().iter_u64_digits().next().map_or(0, |word| word as u8),
    }
}

/// A dispatcher passing a failure through appends its name; none stays bare.
///
/// A field getter is not one of them. The reader wrote `x.name`, not a call,
/// and its internal name is not theirs to see — so the trace skips it here,
/// where no backend can forget to.
pub fn hop(failure: Value, name: &str) -> Value {
    if crate::ast::getter_field(name).is_some() {
        return failure;
    }
    match failure {
        Value::ErrV(info) => {
            let mut hops = info.hops.clone();
            hops.push(Rc::from(name));
            Value::ErrV(Rc::new(ErrInfo {
                reason: info.reason.clone(),
                origin: info.origin.clone(),
                hako: info.hako.clone(),
                hops,
                cause: info.cause.clone(),
                merged: info.merged,
            }))
        }
        other => other,
    }
}

/// A deliberate exit is an err whose reason is `os/exit_status`. The endpoint
/// reads its code rather than reporting it, because the program did not fail
/// to say what it meant — it said it.
///
/// This lives here rather than beside the native endpoint because there are
/// three endpoints and only one of them was in `main.rs`. The page's endpoint
/// (`wasm_rt::exec_main`) could not see this function at all, so a program
/// that exited deliberately was reported to the reader as an unhandled err.
pub fn deliberate_exit(reason: &Value) -> Option<u8> {
    let Value::Record { ty, fields } = reason else { return None };
    // the type spells its module chain at whatever depth the import graph
    // qualified it: os/exit_status directly, hako/os/exit_status one hop in
    if !(ty.as_ref() == "os/exit_status" || ty.ends_with("/os/exit_status")) {
        return None;
    }
    match fields.borrow().first() {
        Some(Value::Int(code)) => Some(u8::try_from(code).unwrap_or(1)),
        // an exit_status carrying something that is not a status is not a
        // program saying what it meant — it is one that went wrong computing
        // the code, and the reader is owed that rather than a silent 1
        _ => None,
    }
}

/// The endpoint report's trace lines, newest pass-through first, pointing
/// back toward the birth site. Byte-identical across all three engines.
pub fn trace_lines(interp: &Interp, info: &ErrInfo) -> String {
    let mut out = String::new();
    if let Some(origin) = &info.origin {
        out.push_str("  born in ");
        out.push_str(origin);
        out.push('\n');
    }
    if !info.hops.is_empty() {
        let path: Vec<&str> = info.hops.iter().rev().map(|h| &**h).collect();
        out.push_str("  passed through ");
        out.push_str(&path.join(" \u{2190} "));
        out.push('\n');
    }
    if let Some(cause) = &info.cause {
        out.push_str("  caused by: ");
        out.push_str(&render(interp, &cause.reason, true));
        out.push('\n');
        out.push_str(&trace_lines(interp, cause));
    }
    out
}
