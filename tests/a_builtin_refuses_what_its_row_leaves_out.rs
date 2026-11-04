//! `infer::BUILTIN_TAKES` says what each builtin takes at each position, and
//! the literal check, the none check and the language server's hover all
//! read it. A row that left out a tag the interpreter runs on would refuse a
//! working program, or tell an editor a parameter is narrower than it is.
//! So every row is checked here against the interpreter, the oracle: for each
//! position and each tag the row leaves out, a program hands the builtin a
//! value of that tag, through std's wrapper where the builtin has one, and
//! the run must stop with a runtime refusal.
//!
//! The value travels through `opaque`, an index into a one-element list,
//! which answers anything as far as the compiler can tell, so the refusal is
//! the interpreter's rather than one of the checks that read the row. A none
//! comes from `gone`, an index past the end. The other positions hold a
//! filler of a tag the row admits, and the same program with only fillers
//! must run without a refusal, so the one a probe sees belongs to the value
//! it probed.

use kanso::infer::{self, Set};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The tags a probe hands a builtin, each with an expression of that tag.
const TAGS: &[(&str, Set, &str)] = &[
    ("int", infer::INT, "1"),
    ("bignum", infer::BIG, "huge 1"),
    ("float", infer::FLOAT, "1.5"),
    ("string", infer::STR, "\"a\""),
    ("true", infer::TRUE, "true"),
    ("false", infer::FALSE, "false"),
    ("none", infer::NONE, "none"),
    ("done", infer::DONE, "done"),
    ("bytes", infer::BYTES, "text/bytes \"a\""),
    ("list", infer::LIST, "[1]"),
    ("map", infer::MAP, "{ \"a\":1 }"),
    ("record", infer::REC, "cell 1"),
    ("fn", infer::FN, "(x -> x)"),
    ("effect", infer::DESC, "io/write \"\""),
];

/// How a program reaches each builtin: a call whose `{0}`, `{1}`, ... are its
/// positions, the filler each position holds, and whether the call answers
/// an effect, which the program then runs.
fn route(name: &str) -> Option<(&'static str, &'static [&'static str], bool)> {
    Some(match name {
        "append" => ("text/append {0} {1}", &["text/bytes \"a\"", "text/bytes \"b\""], false),
        "bytes" => ("text/bytes {0}", &["\"a\""], false),
        "to_bytes" => ("text/to_bytes {0}", &["[1]"], false),
        "char_code" => ("text/char_code {0}", &["\"a\""], false),
        "chars" => ("text/chars {0}", &["\"ab\""], false),
        "concat" => ("list/concat {0} {1}", &["[1]", "[2]"], false),
        "entries" => ("entries {0}", &["{ \"a\":1 }"], false),
        "keys" => ("keys {0}", &["{ \"a\":1 }"], false),
        "values" => ("values {0}", &["{ \"a\":1 }"], false),
        "find2" => ("text/find2 {0} {1} {2} {3}", &["text/bytes \"abc\"", "1", "98", "99"], false),
        "find2_below" => (
            "text/find2_below {0} {1} {2} {3} {4}",
            &["text/bytes \"abc\"", "1", "98", "99", "3"],
            false,
        ),
        "number_span" => ("text/number_span {0} {1}", &["text/bytes \"12\"", "1"], false),
        "from_code" => ("text/from_code {0}", &["65"], false),
        "join" => ("text/join {0} {1}", &["[\"a\"]", "\",\""], false),
        "length" => ("length {0}", &["\"ab\""], false),
        "push" => ("push {0} {1}", &["[1]", "2"], false),
        "put" => ("put {0} {1} {2}", &["{ \"a\":1 }", "\"b\"", "2"], false),
        "delete" => ("delete {0} {1}", &["{ \"a\":1 }", "\"a\""], false),
        "slice" => ("text/slice {0} {1} {2}", &["\"abc\"", "1", "2"], false),
        "split" => ("text/split {0} {1}", &["\"a,b\"", "\",\""], false),
        "to_float" => ("text/to_float {0}", &["\"1.5\""], false),
        "to_int" => ("text/to_int {0}", &["\"1\""], false),
        "utf8" => ("text/utf8 {0}", &["text/bytes \"a\""], false),
        "sqrt" => ("math/sqrt {0}", &["4.0"], false),
        "round" => ("math/round {0}", &["1.5"], false),
        "bit_and" => ("{0} & {1}", &["1", "2"], false),
        "bit_or" => ("{0} | {1}", &["1", "2"], false),
        "bit_xor" => ("bits/xor {0} {1}", &["1", "2"], false),
        "bit_not" => ("bits/complement {0}", &["1"], false),
        "bit_shl" => ("bits/shl {0} {1}", &["1", "2"], false),
        "bit_shr" => ("bits/shr {0} {1}", &["1", "2"], false),
        "random" => ("math/random {0}", &["3"], true),
        "random_bytes" => ("os/random_bytes {0}", &["3"], true),
        "sleep" => ("time/sleep {0}", &["1"], true),
        "write" => ("io/write {0}", &["\"\""], true),
        "write_err" => ("io/write_err {0}", &["\"\""], true),
        "env" => ("os/env {0}", &["\"HOME\""], true),
        "exists" => ("os/exists {0}", &["\"f\""], true),
        "is_dir" => ("os/is_dir {0}", &["\"f\""], true),
        "modified" => ("os/modified {0}", &["\"f\""], true),
        "list_dir" => ("os/list_dir {0}", &["\".\""], true),
        "read_bytes" => ("os/read_bytes {0}", &["\"f\""], true),
        "read_file" => ("os/read_file {0}", &["\"f\""], true),
        "make_dir" => ("os/make_dir {0}", &["\"d\""], true),
        "write_file" => ("os/write_file {0} {1}", &["\"f\"", "\"x\""], true),
        "run" => ("os/run {0} {1}", &["\"true\"", "[\"x\"]"], true),
        "start" => ("os/start {0} {1}", &["\"true\"", "[\"x\"]"], true),
        "kill" => ("os/kill {0}", &["999999"], true),
        "listen" => ("net/listen {0}", &["0"], true),
        "dial" => ("net/dial {0} {1}", &["\"localhost\"", "1"], true),
        // a connection only `net` can make, to a listener of the program's own
        "net_write" => ("connected (c -> net/write c {1})", &["c", "\"x\""], true),
        "net_secure" => ("connected (c -> net/secured c {1})", &["c", "\"h\""], true),
        "pool_make" => ("pool/make {0}", &["1"], true),
        _ => return None,
    })
}

const HELPERS: &str = "type cell
  n

fn opaque v
  (push [] v)[1]

fn gone _
  (push [] 1)[2]

fn huge _
  18446744073709551616
";

const CONNECTED: &str = "
fn connected use
  net/listen 0 .> (l -> net/port l .> (p -> reach p use))

fn reach p use
  net/dial \"localhost\" p .> use
";

/// An argument as the formatter wants it: bare when it is one token or
/// brackets itself, parenthesized otherwise.
fn arg(a: &str) -> String {
    match a.starts_with(['(', '[', '{']) || !a.contains(' ') {
        true => a.to_string(),
        false => format!("({a})"),
    }
}

/// The program that hands `name` the given arguments.
fn program(name: &str, args: &[String]) -> String {
    let (call, _, effect) = route(name).expect("a routed builtin");
    // an operator's operand is a call already, and needs no parentheses
    let operator = call.starts_with("{0} ");
    let mut call = call.to_string();
    for (i, a) in args.iter().enumerate() {
        let a = match operator && a.starts_with('(') && a.ends_with(')') {
            true => &a[1..a.len() - 1],
            false => a.as_str(),
        };
        call = call.replace(&format!("{{{i}}}"), a);
    }
    let body = match effect {
        true => format!("{call} .> (_ -> print \"ok\")\n"),
        false => format!("got = {call}\nprint \"{{got}}\"\n"),
    };
    let connected = body.contains("connected");
    let modules: Vec<&str> = ["bits", "io", "list", "math", "net", "os", "pool", "text", "time"]
        .into_iter()
        .filter(|m| body.contains(&format!("{m}/")) || (*m == "net" && connected))
        .collect();
    let mut out = String::new();
    for m in &modules {
        out.push_str(&format!("import \"std/{m}\"\n"));
    }
    if !modules.is_empty() {
        out.push('\n');
    }
    out.push_str(HELPERS);
    if connected {
        out.push_str(CONNECTED);
    }
    out.push('\n');
    out.push_str(&body);
    out
}

/// Run a program on the interpreter in a fresh directory holding one file,
/// `f`, and answer its exit code and its first line of stderr.
fn run(dir: &Path, source: &str) -> (Option<i32>, String) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("f"), "x").unwrap();
    let file = dir.join("probe.kso");
    std::fs::write(&file, source).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg(&file)
        .arg("--interp")
        .current_dir(dir)
        .env("KANSO_SEED", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso runs");
    // a probe the interpreter does not refuse may start real work, and one
    // left waiting on the network is a failure rather than a stalled suite
    let deadline = Instant::now() + Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return (None, "timed out".to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let out = child.wait_with_output().unwrap();
    let first = String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("").to_string();
    (out.status.code(), first)
}

struct Probe {
    what: String,
    source: String,
    control: bool,
}

fn probes() -> Vec<Probe> {
    let mut out = Vec::new();
    for (name, row) in infer::BUILTIN_TAKES {
        let (_, fillers, _) =
            route(name).unwrap_or_else(|| panic!("`{name}` has a row and no route in this spec"));
        assert!(fillers.len() >= row.len(), "`{name}`: a filler for every position its row names");
        let plain: Vec<String> = fillers.iter().map(|f| arg(f)).collect();
        // with only its fillers, `net/secured` begins a TLS handshake that
        // the program's own listener never answers
        if *name != "net_secure" {
            out.push(Probe {
                what: format!("{name} with its fillers"),
                source: program(name, &plain),
                control: true,
            });
        }
        for (at, &takes) in row.iter().enumerate() {
            if takes == infer::TOP {
                continue;
            }
            for &(tag, set, expr) in TAGS {
                if set & takes != 0 {
                    continue;
                }
                let hole = match tag {
                    "none" => "(gone 1)".to_string(),
                    _ => format!("(opaque {})", arg(expr)),
                };
                let mut args = plain.clone();
                args[at] = hole;
                out.push(Probe {
                    what: format!("{name} handed {tag} at {at}"),
                    source: program(name, &args),
                    control: false,
                });
            }
        }
    }
    out
}

#[test]
fn every_row_is_what_the_interpreter_refuses() {
    let probes = probes();
    let root = std::env::temp_dir().join(format!("kanso-builtin-takes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(p) = probes.get(i) else { break };
                let (code, first) = run(&root.join(i.to_string()), &p.source);
                let refused = code == Some(1) && first.starts_with("error[runtime]: ");
                let wrong = match p.control {
                    true => first.starts_with("error[runtime]") || code == Some(2),
                    false => !refused,
                };
                if wrong {
                    failures
                        .lock()
                        .unwrap()
                        .push(format!("{}: exit {code:?}, {first:?}\n{}", p.what, p.source));
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&root);
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} of {} probes:\n\n{}",
        failures.len(),
        probes.len(),
        failures.join("\n")
    );
}
