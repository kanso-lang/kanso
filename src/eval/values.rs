// The interpreter's operations on values: ordering, arithmetic and its
// failures, the binary operators, equality, and rendering a value as text.
// eval.rs reads this file with include!, so the items stay in the eval
// module and the compiler builds to the same instructions it did when they
// sat in eval.rs (kanso#1867). cargo fmt does not reach an included file, so
// CI's format job checks the files under src/'s subdirectories itself.

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    if let Value::Sub { inner, .. } = a {
        return compare(inner, b);
    }
    if let Value::Sub { inner, .. } = b {
        return compare(a, inner);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => Some(float_order(*x, *y)),
        (Value::Int(x), Value::Float(y)) => Some(cmp_int_float(x, *y)),
        (Value::Float(x), Value::Int(y)) => Some(cmp_int_float(y, *x).reverse()),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// The builtin float order, ruled 2026-09-30. Every NaN equals every NaN and
/// ranks above `inf`, whatever its sign bit and however it was made, and `-0.0`
/// equals and ranks with `0.0`. `f64::total_cmp`, which this replaces, ranked
/// bit patterns, so `inf - inf` sorted below `-inf` on x86 and above `inf` on
/// ARM, and `-0.0` fell below `0.0` while equalling `0`.
fn float_order(x: f64, y: f64) -> std::cmp::Ordering {
    match (x.is_nan(), y.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
    }
}

/// A BigInt widened to f64 — the `x:float` cast at the value level.
fn int_f(n: &Int) -> f64 {
    n.to_f64()
}

/// A whole number against a fractional one, decided exactly. Widening the
/// integer first would round it onto the nearest value a float can hold and
/// report two different numbers as equal — 9007199254740993 is the smallest
/// integer where that happens. Comparing against the float's floor keeps the
/// integer whole, and the fraction breaks the tie.
fn cmp_int_float(x: &Int, y: f64) -> std::cmp::Ordering {
    let floor = y.floor();
    // Only a NaN or an infinity has no floor to compare against, and the
    // float order over the widened value is what ranks those.
    let Some(whole) = <BigInt as num_traits::FromPrimitive>::from_f64(floor) else {
        return float_order(x.to_f64(), y);
    };
    match x.big().as_ref().cmp(&whole) {
        std::cmp::Ordering::Equal if y > floor => std::cmp::Ordering::Less,
        settled => settled,
    }
}

/// `10 / 0` answers a value rather than a failure: a `divide_by_zero` under a
/// `math_failure` under a string, so a handler may ask for the specific
/// failure, for any math failure, or read the reason as text.
/// A zero divisor's answer with its wrapper taken off, for a program that
/// never names `divide_by_zero` or `math_failure`. Native gives such a program
/// the bare text, and only an operator's arms could tell the two apart, since
/// a subtype reaches them and a string does not.
pub fn bare_math(value: Value) -> Value {
    match value {
        Value::Sub { ty, inner } if &*ty == crate::DIVIDE_BY_ZERO => match &*inner {
            Value::Sub { inner: text, .. } => (**text).clone(),
            _ => (*inner).clone(),
        },
        other => other,
    }
}

fn math_failure(reason: &str) -> Value {
    let text = Value::Str(Text::from(reason.to_string()));
    let root = Value::Sub { ty: Rc::from(crate::MATH_FAILURE), inner: Rc::new(text) };
    Value::Sub { ty: Rc::from(crate::DIVIDE_BY_ZERO), inner: Rc::new(root) }
}

fn div_float(a: f64, b: f64) -> EvalResult {
    match b == 0.0 {
        true => Ok(math_failure("division by zero")),
        false => Ok(Value::Float(a / b)),
    }
}

/// `a & b`: join two descriptions to run with no order between them.
/// Failures accumulate — two errs merge into one whose reason lists both
/// (origin-less: the merge has no single birthplace) — and a lone failure
/// propagates as itself. Anything that isn't a description or a failure
/// cannot be joined.
pub fn join_values(left: Value, right: Value, span: Span) -> EvalResult {
    match (is_failure(&left), is_failure(&right)) {
        (true, true) => Ok(accumulate_failures(left, right)),
        (true, false) => Ok(left),
        (false, true) => Ok(right),
        (false, false) => match (&left, &right) {
            (Value::Desc(a), Value::Desc(b)) => {
                Ok(Value::Desc(Rc::new(Desc::Join(a.clone(), b.clone()))))
            }
            _ => Err(RuntimeError { message: "a group joins descriptions".to_string(), span }),
        },
    }
}

/// Merge two failures: err + err becomes one err whose reason is the list of
/// both reasons; a `none` adds nothing to an err; two nones stay none.
/// Every failure among a call's arguments, merged. Two failures in one call
/// are two facts, and the one that lost a race to be first is not less true —
/// the parallel group has always merged them, and Clay ruled (2026-08-05) that
/// an operation does too.
fn merged_failures(args: &[Value]) -> Value {
    args.iter()
        .filter(|a| is_failure(a))
        .cloned()
        .reduce(accumulate_failures)
        .expect("a caller checked that one argument fails")
}

pub fn accumulate_failures(left: Value, right: Value) -> Value {
    match (&left, &right) {
        (Value::ErrV(a), Value::ErrV(b)) => {
            let mut reasons = reasons_of(a);
            reasons.extend(reasons_of(b));
            Value::ErrV(Rc::new(ErrInfo {
                reason: Value::List(Rc::new(reasons)),
                origin: None,
                hako: None,
                hops: Vec::new(),
                cause: None,
                merged: true,
            }))
        }
        (Value::ErrV(_), _) => left,
        (_, Value::ErrV(_)) => right,
        _ => left,
    }
}

/// What an err contributes to a merge: the reasons it already carries if it
/// was itself merged, and otherwise itself, whatever shape its reason has.
fn reasons_of(info: &Rc<ErrInfo>) -> Vec<Value> {
    match (info.merged, &info.reason) {
        (true, Value::List(items)) => items.as_ref().clone(),
        _ => vec![info.reason.clone()],
    }
}

/// The name the runtime prints for a bitwise operator, which is the name of
/// the std/bits function it shares its meaning with.
fn spelled_op(op: &str) -> &str {
    match op {
        "&" => "and",
        "|" => "or",
        _ => "xor",
    }
}

/// The three bitwise operators share a shape: both sides must fit the machine
/// word the compiled engines use, and the answer is an int.
fn bitwise(op: &str, a: &Int, b: &Int, span: Span) -> EvalResult {
    let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) else {
        return Err(RuntimeError {
            message: format!("{} takes whole numbers that fit 64 bits", spelled_op(op)),
            span,
        });
    };
    let bits = match op {
        "&" => x & y,
        "|" => x | y,
        _ => x ^ y,
    };
    Ok(Value::int(bits))
}

/// `force` is how the comparison reaches a cell. Equality does not evaluate
/// at the site where it is written — under full laziness it builds a thunk
/// like any operator, and only the wire demands. By the time this runs, that
/// demand has arrived, so forcing here is the ordinary demand context every
/// forced thunk already runs in; the parameter exists because the engines
/// keep their forcing machinery in different places.
pub fn eval_binop(
    op: &str,
    left: Value,
    right: Value,
    span: Span,
    cells: &Cells<'_>,
) -> EvalResult {
    // Two failures in one operation merge, exactly as two failures in a
    // parallel group do (Clay, 2026-08-05): neither side caused the other, so
    // neither deserves top billing, and the one that loses would otherwise be
    // discarded without a word. One failure propagates as itself.
    match (is_failure(&left), is_failure(&right)) {
        (true, true) => return Ok(accumulate_failures(left, right)),
        (true, false) => return Ok(left),
        (false, true) => return Ok(right),
        (false, false) => {}
    }
    if op == "==" || op == "!=" {
        // Asking whether two functions are the same function is asking which
        // one you were handed, and that is what dispatch answers. The same
        // goes for an effect: if the answer would change what you do next,
        // an arm names it. So the question is refused rather than answered
        // falsely — the way `<` already refuses it on the same values.
        if opaque_to_equality(&left) || opaque_to_equality(&right) {
            return Err(RuntimeError { message: EQUALITY_REFUSED.to_string(), span });
        }
        let equal = values_equal(&left, &right, cells, span)?;
        return Ok(bool_value(match op {
            "==" => equal,
            _ => !equal,
        }));
    }
    match (op, &left, &right) {
        ("+", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.add(b))),
        ("-", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.sub(b))),
        ("*", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.mul(b))),
        ("/", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => Ok(math_failure("division by zero")),
            false => Ok(Value::Int(a.div(b))),
        },
        ("%", Value::Int(a), Value::Int(b)) => match b.is_zero() {
            true => Ok(math_failure("modulo by zero")),
            false => Ok(Value::Int(a.rem(b))),
        },
        // Bitwise, over whole numbers only. Native's ints are 64 bits wide by
        // construction, so a value too wide to be one is a case only the
        // interpreter can reach, and it says so rather than answering.
        ("&", Value::Int(a), Value::Int(b)) => bitwise("&", a, b, span),
        ("|", Value::Int(a), Value::Int(b)) => bitwise("|", a, b, span),
        ("^", Value::Int(a), Value::Int(b)) => bitwise("^", a, b, span),
        // Anything else meeting a bitwise operator is the type complaint the
        // compiled engines print, in their words: the operator and the named
        // function in std/bits are one operation, so they answer alike.
        ("&" | "|" | "^", other, _) if !matches!(other, Value::Int(_)) => Err(RuntimeError {
            message: format!(
                "{} takes whole numbers, got {}",
                spelled_op(op),
                render_demanded(other, false)
            ),
            span,
        }),
        ("&" | "|" | "^", _, other) => Err(RuntimeError {
            message: format!(
                "{} takes whole numbers, got {}",
                spelled_op(op),
                render_demanded(other, false)
            ),
            span,
        }),
        ("+", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        ("-", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        ("*", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        ("/", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => Ok(math_failure("division by zero")),
            false => Ok(Value::Float(a / b)),
        },
        ("%", Value::Float(a), Value::Float(b)) => match *b == 0.0 {
            true => Ok(math_failure("modulo by zero")),
            false => Ok(Value::Float(a % b)),
        },
        // int meets float: the int widens (as if cast `x:float`), result float
        ("+", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) + b)),
        ("+", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + int_f(b))),
        ("-", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) - b)),
        ("-", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - int_f(b))),
        ("*", Value::Int(a), Value::Float(b)) => Ok(Value::Float(int_f(a) * b)),
        ("*", Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * int_f(b))),
        ("/", Value::Int(a), Value::Float(b)) => div_float(int_f(a), *b),
        ("/", Value::Float(a), Value::Int(b)) => div_float(*a, int_f(b)),
        ("<" | "<=" | ">" | ">=", _, _) => match compare(&left, &right) {
            Some(ord) => Ok(bool_value(match op {
                "<" => ord.is_lt(),
                "<=" => ord.is_le(),
                ">" => ord.is_gt(),
                _ => ord.is_ge(),
            })),
            None => Err(RuntimeError {
                message: "comparison requires two values of one comparable type".to_string(),
                span,
            }),
        },
        _ => Err(RuntimeError { message: format!("`{op}` is not defined for these values"), span }),
    }
}

fn bool_value(b: bool) -> Value {
    match b {
        true => Value::True,
        false => Value::False,
    }
}

/// Values equality declines to answer about. A function and an effect are
/// both things you dispatch on rather than compare: asking which one you hold
/// is a question an arm answers, and answering it with a comparison is the
/// shape kanso refuses everywhere else. A subtype is transparent, so the
/// question is really about what it wraps.
/// A cell survives its own forcing: `force_thunk` writes the answer into the
/// same reference-counted state, so the pointer still names the binding.
fn thunk_identity(v: &Value) -> Option<usize> {
    match v {
        Value::Thunk(cell) => Some(Rc::as_ptr(cell) as *const () as usize),
        _ => None,
    }
}

/// The refusal `==` gives a function or an effect, wherever in its operands
/// the walk meets one.
const EQUALITY_REFUSED: &str =
    "equality is not defined on a function or an effect — write an arm for the case you mean";

/// A partial is a function as much as a lambda is. Until 2026-09-29 it was
/// not listed, and `(&f 1) == 3` answered false on the interpreter where
/// native refused.
fn opaque_to_equality(v: &Value) -> bool {
    match v {
        Value::FnRef(_) | Value::Closure(_) | Value::Desc(_) | Value::Partial(..) => true,
        Value::Sub { inner, .. } => opaque_to_equality(inner),
        _ => false,
    }
}

fn values_equal(a: &Value, b: &Value, cells: &Cells<'_>, span: Span) -> Result<bool, RuntimeError> {
    values_equal_seen(a, b, &mut Set::default(), cells, span)
}

/// Two cyclic graphs would compare forever, so a pair of cells is assumed
/// equal once seen and any contradiction still returns false.
type Seen = Set<(usize, usize)>;

/// How a comparison reaches a cell. `id` names one — a stable identity that
/// survives forcing, which is what makes arriving twice recognisable — and
/// `force` demands it. Each engine keeps its cells somewhere different: the
/// oracle in a reference counted state, the browser in a slot table.
pub struct Cells<'a> {
    pub id: &'a dyn Fn(&Value) -> Option<usize>,
    pub force: &'a dyn Fn(Value) -> Result<Value, RuntimeError>,
}

fn values_equal_seen(
    a: &Value,
    b: &Value,
    seen: &mut Seen,
    cells: &Cells<'_>,
    span: Span,
) -> Result<bool, RuntimeError> {
    if let Value::Sub { inner, .. } = a {
        return values_equal_seen(inner, b, seen, cells, span);
    }
    if let Value::Sub { inner, .. } = b {
        return values_equal_seen(a, inner, seen, cells, span);
    }
    // A cell is forced the way any demand forces it, so an ordinary lazy
    // operand compares as its value. A cell keeps its identity across the
    // force, so arriving a second time at one pair is the cycle closing
    // through a cell, and the pair is assumed equal — the assumption that
    // makes bisimulation terminate, the same one records make below. A
    // cell demanded mid-construction still fails in the force: that one
    // is not a value yet.
    match ((cells.id)(a), (cells.id)(b)) {
        (Some(x), Some(y)) => {
            if !seen.insert((x, y)) {
                return Ok(true);
            }
            let fa = (cells.force)(a.clone())?;
            let fb = (cells.force)(b.clone())?;
            return values_equal_seen(&fa, &fb, seen, cells, span);
        }
        (Some(_), None) => {
            let fa = (cells.force)(a.clone())?;
            return values_equal_seen(&fa, b, seen, cells, span);
        }
        (None, Some(_)) => {
            let fb = (cells.force)(b.clone())?;
            return values_equal_seen(a, &fb, seen, cells, span);
        }
        (None, None) => {}
    }
    // A function inside a list, a map or a record is the same question as
    // one outside it, and it is refused the same way rather than answered
    // false. Until 2026-09-29 only the operands themselves were asked, so
    // `[k] == [k]` answered false on the interpreter where native refused.
    // A browser table function is a function by this point: a lazy one is a
    // cell, and the cell was forced above.
    let host_fn = |v: &Value| matches!(v, Value::TableFn(_));
    if opaque_to_equality(a) || opaque_to_equality(b) || host_fn(a) || host_fn(b) {
        return Err(RuntimeError { message: EQUALITY_REFUSED.to_string(), span });
    }
    let answer = match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => float_order(*x, *y).is_eq(),
        // One numeric domain, which `<=` and `>=` already assert: both answer
        // true for 1 and 1.0, and `<` answers false either way round because
        // neither is strictly less. Equality was the one operator dissenting.
        // A float cannot be a map key, so nothing needs them told apart.
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => {
            cmp_int_float(x, *y).is_eq()
        }
        (Value::Map(x), Value::Map(y)) => {
            if x.len() != y.len() {
                return Ok(false);
            }
            for ((ka, va), (kb, vb)) in x.iter().zip(y.iter()) {
                if ka != kb || !values_equal_seen(va, vb, seen, cells, span)? {
                    return Ok(false);
                }
            }
            true
        }
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::True, Value::True) | (Value::False, Value::False) => true,
        (Value::NoneV, Value::NoneV) => true,
        (Value::Done, Value::Done) => true,
        (Value::List(x), Value::List(y)) => {
            if x.len() != y.len() {
                return Ok(false);
            }
            for (a, b) in x.iter().zip(y.iter()) {
                if !values_equal_seen(a, b, seen, cells, span)? {
                    return Ok(false);
                }
            }
            true
        }
        (Value::Bytes(x), Value::Bytes(y)) => x == y,
        // `==` crosses where the functions do not, because the compiled engine
        // has compared a byte string against a list of its numbers since it was
        // written (k_bytes_eq_list) and the differential law is what it is.
        // Whether it should is a separate question from the one just ruled.
        (Value::Bytes(x), Value::List(y)) | (Value::List(y), Value::Bytes(x)) => {
            x.len() == y.len()
                && x.iter().zip(y.iter()).all(|(a, b)| match b {
                    Value::Int(n) => *n == Int::from(*a),
                    _ => false,
                })
        }
        (Value::Record { ty: tx, fields: fx }, Value::Record { ty: ty_, fields: fy }) => {
            if tx != ty_ {
                return Ok(false);
            }
            if Rc::ptr_eq(fx, fy) {
                return Ok(true);
            }
            let key = (Rc::as_ptr(fx) as *const () as usize, Rc::as_ptr(fy) as *const () as usize);
            if !seen.insert(key) {
                return Ok(true);
            }
            if fx.borrow().len() != fy.borrow().len() {
                return Ok(false);
            }
            let pairs: Vec<(Value, Value)> =
                fx.borrow().iter().cloned().zip(fy.borrow().iter().cloned()).collect();
            for (a, b) in &pairs {
                if !values_equal_seen(a, b, seen, cells, span)? {
                    return Ok(false);
                }
            }
            true
        }
        _ => false,
    };
    Ok(answer)
}

fn render_float(x: f64) -> String {
    // Ruled 2026-09-08: `inf`, `-inf`, `nan`, which is C's spelling and the
    // lineage the `%g` rules below already follow. Both are answered here
    // rather than downstream because neither survives the shortest-digits
    // path: `{:e}` has no `e` to split on for either of them, and the split
    // below used to unwrap that absence and panic.
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    if x == x.floor() && x.abs() < 1e15 && x.is_finite() {
        return format!("{x:.1}");
    }
    // rust's LowerExp gives the shortest round-trip digits; the format
    // layer mirrors the native renderer's %g rules exactly: exponent form
    // at X < -4 or X >= max(15, digit count)
    let neg = x < 0.0;
    let exp_form = format!("{:e}", x.abs());
    let (mant, exp) = exp_form.split_once('e').expect("LowerExp has an e");
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = even_on_a_tie(x.abs(), digits, exp);
    let k = digits.len() as i32;
    let x10: i32 = exp.parse().expect("exponent parses");
    let p = k.max(15);
    let sign = if neg { "-" } else { "" };
    if x10 < -4 || x10 >= p {
        let tail = if k > 1 { format!(".{}", &digits[1..]) } else { ".0".to_string() };
        let esign = if x10 < 0 { '-' } else { '+' };
        return format!("{sign}{}{tail}e{esign}{:02}", &digits[..1], x10.abs());
    }
    if x10 >= 0 {
        let ip = (x10 + 1) as usize;
        let whole: String =
            (0..ip).map(|i| digits.as_bytes().get(i).map(|b| *b as char).unwrap_or('0')).collect();
        let frac = if (k as usize) > ip { format!(".{}", &digits[ip..]) } else { String::new() };
        return format!("{sign}{whole}{frac}");
    }
    let zeros = "0".repeat((-x10 - 1) as usize);
    format!("{sign}0.{zeros}{digits}")
}

/// The shortest digits for `x`, taking the even last digit when two
/// candidates of that length lie exactly as far from `x`. Rust's `{:e}` takes
/// the upper one, so 900719925474099.25 printed `...099.3` here and
/// `...099.2` natively, where ryū takes the even digit as Python and
/// JavaScript do. Only an odd last digit whose neighbour below also reads back
/// as `x` can be such a tie, so the exact expansion is asked for there alone.
fn even_on_a_tie(x: f64, digits: String, exp: &str) -> String {
    let last = digits.as_bytes()[digits.len() - 1];
    if last.is_multiple_of(2) {
        return digits;
    }
    let mut below = digits.clone().into_bytes();
    let at = below.len() - 1;
    below[at] -= 1;
    let below = String::from_utf8(below).expect("ascii digits");
    let read_back = format!("{}.{}e{exp}", &below[..1], &below[1..]).parse::<f64>();
    if read_back != Ok(x) {
        return digits;
    }
    // The tie: x's exact expansion is `below` followed by a single 5.
    let exact = format!("{:.800e}", x);
    let (mant, exact_exp) = exact.split_once('e').expect("LowerExp has an e");
    let exact_digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let exact_digits = exact_digits.trim_end_matches('0');
    if exact_exp == exp && exact_digits == format!("{below}5") {
        below
    } else {
        digits
    }
}

/// Transparency: a subtype value IS its base wherever machinery (builtins,
/// operators, comparison) consumes it; only dispatch sees the wrapper.
pub fn sub_base(value: Value) -> Value {
    match value {
        Value::Sub { inner, .. } => sub_base((*inner).clone()),
        other => other,
    }
}

pub fn render(interp: &Interp, value: &Value, quote_strings: bool) -> String {
    render_seen(Some(interp), value, quote_strings, &mut Set::default())
}

/// Rendering for the three free functions that name an operand in a
/// diagnostic — `whole`, `map_key`, `eval_binop`. They are reached from
/// wasm_rt as well as from the interpreter, where the only handle is a
/// thread-local already borrowed by the caller, so taking one here would
/// nest a RefCell borrow and panic.
///
/// The operands cannot be pending: arithmetic forces what it adds, a map key
/// is forced to be compared, and `whole` is asked about a value its caller
/// already demanded. A cell reaching here would print `<thunk>`, which is the
/// old word and a signal that this reasoning is wrong somewhere.
pub fn render_demanded(value: &Value, quote_strings: bool) -> String {
    render_seen(None, value, quote_strings, &mut Set::default())
}

/// A cycle would render forever, so re-entering a cell on the current path
/// prints `<cycle>` instead.
fn render_seen(
    interp: Option<&Interp>,
    value: &Value,
    quote_strings: bool,
    seen: &mut Set<usize>,
) -> String {
    match value {
        // a subtype renders as its base until a user arm claims it
        Value::Sub { inner, .. } => render_seen(interp, inner, quote_strings, seen),
        // The wire is the demand: output forces what it prints, so a cell
        // that has not run runs here. A cell already on the path holds the
        // walk that is reading it, and that is where a knot terminates.
        //
        // A render cannot carry a failure out, so one falls back to the old
        // word. The reachable failure is a cell demanding itself, and the
        // path guard above answers that before the force is reached.
        Value::Thunk(cell) => {
            let key = Rc::as_ptr(cell) as usize;
            if !seen.insert(key) {
                return "<cycle>".to_string();
            }
            let rendered = match interp {
                Some(it) => match it.force_thunk(Value::Thunk(Rc::clone(cell))) {
                    Ok(value) => render_seen(interp, &value, quote_strings, seen),
                    Err(_) => "<thunk>".to_string(),
                },
                None => match &*cell.borrow() {
                    ThunkState::Forced(value) => render_seen(interp, value, quote_strings, seen),
                    _ => "<thunk>".to_string(),
                },
            };
            seen.remove(&key);
            rendered
        }
        Value::Int(n) => n.to_string(),
        Value::Float(x) => render_float(*x),
        Value::Map(entries) => match entries.is_empty() {
            true => "{}".to_string(),
            false => {
                let ptr = Rc::as_ptr(entries) as usize;
                if !seen.insert(ptr) {
                    return "<cycle>".to_string();
                }
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            MapKey::Int(n) => n.to_string(),
                            MapKey::Str(s) => format!("\"{s}\""),
                        };
                        format!("{key}:{}", render_seen(interp, value, true, seen))
                    })
                    .collect();
                seen.remove(&ptr);
                format!("{{ {} }}", inner.join(" "))
            }
        },
        Value::Str(s) => match quote_strings {
            true => format!("\"{s}\""),
            false => s.to_string(),
        },
        Value::True => "true".to_string(),
        Value::False => "false".to_string(),
        Value::NoneV => "<none>".to_string(),
        Value::Done => "<done>".to_string(),
        Value::ErrV(info) => format!("err {}", render_seen(interp, &info.reason, true, seen)),
        // On the path like a record, so a knot that closes through a cell
        // holding this very list prints the list once and says `<cycle>`
        // where it comes round.
        Value::List(items) => {
            let ptr = Rc::as_ptr(items) as usize;
            if !seen.insert(ptr) {
                return "<cycle>".to_string();
            }
            let inner: Vec<String> =
                items.iter().map(|i| render_seen(interp, i, true, seen)).collect();
            seen.remove(&ptr);
            format!("[{}]", inner.join(" "))
        }
        // The compiled engine prints byte data as the numbers between
        // brackets, and this is the one place where a list and bytes are meant
        // to look alike: they are what a reader sees, not what a function
        // accepts.
        Value::Bytes(items) => {
            let inner: Vec<String> = items.iter().map(|b| b.to_string()).collect();
            format!("[{}]", inner.join(" "))
        }
        Value::Record { ty, fields } => match fields.borrow().is_empty() {
            true => shown_type(ty),
            false => {
                let ptr = Rc::as_ptr(fields) as usize;
                if !seen.insert(ptr) {
                    return "<cycle>".to_string();
                }
                let inner: Vec<String> =
                    fields.borrow().iter().map(|f| render_seen(interp, f, true, seen)).collect();
                seen.remove(&ptr);
                format!("{} {}", shown_type(ty), inner.join(" "))
            }
        },
        // every engine renders a function the same way and none of them names
        // it: the name is the compiler's, and for a field getter it is one no
        // program could have written
        Value::FnRef(_) | Value::Closure(_) | Value::Partial(..) => "<fn>".to_string(),
        // the wire is the demand, and a browser cell answers what it settled
        // on rather than the closure that would have answered it
        Value::TableFn(handle) => match deferral_value(*handle) {
            Some(value) => {
                let key = deferral_key(*handle);
                if !seen.insert(key) {
                    return "<cycle>".to_string();
                }
                let rendered = render_seen(interp, &value, quote_strings, seen);
                seen.remove(&key);
                rendered
            }
            None => "<fn>".to_string(),
        },
        Value::Desc(_) => "<io>".to_string(),
    }
}
