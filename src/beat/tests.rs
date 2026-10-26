//! Which loops the beat analysis brackets and which it leaves alone, on small
//! programs.

use super::{beat_loops, tail_exprs, Program, TailCalls};
use crate::ast::Expr;
use crate::infer;

fn compiled(src: &str) -> (crate::ast::Program, infer::Inference) {
    let program = crate::compile("test.kso", src, false).unwrap();
    let inference = infer::infer(&program);
    (program, inference)
}

fn loops_of(src: &str) -> crate::hash::Set<(String, usize)> {
    // the tests assert membership; the cluster ids are irrelevant here
    let program = crate::compile("test.kso", src, false).unwrap();
    let inference = infer::infer(&program);
    beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program))
        .ids
        .into_keys()
        .collect()
}

/// The report exists to say why a loop keeps the grow-only arena, and a
/// loop can decline for more than one reason at once. Reporting only the
/// first sends the next optimisation after a blocker that was never the
/// whole story.
///
/// The element pushed is a string on purpose. A list of scalars is
/// licensed to cross a boundary now, so pushing `1` here would leave one
/// blocker and the example could not say what it exists to say. A string
/// element is a pointer into the arena, which is the boundary of that
/// license and still a reason to decline.
#[test]
fn a_second_blocker_is_reported_not_masked() {
    let src = "fn feed acc\n  step acc\n\nfn step acc\n  step (push acc \"x\")\n\nmain = print \"{feed []}\"\n";
    let (program, inference) = compiled(src);
    let lines = super::report(&program, &inference, &crate::linear::in_place_pushes(&program));
    let step = lines.iter().find(|l| l.starts_with("step/1")).expect("step is reported");

    assert!(
        step.contains("unbracketed entry") && step.contains("also carries heap"),
        "the report named one blocker and hid the other: {step}"
    );
}

/// A cluster is bracketed as a unit, so the report must not borrow the
/// self-loop's words for it. Measured on a mutual pair entered from
/// outside its own tail cycle: the mark is pushed once and rewound once
/// while the loop runs a thousand times, so "rewinds every iteration"
/// describes something that did not happen.
///
/// The pair is asserted rather than a single name because a cluster is
/// only a cluster with both halves, and the report reaches them by a
/// different path from every other row — they are dropped from the
/// classifier's answer and re-added, which is where the wrong words came
/// from.
#[test]
fn a_cluster_is_not_described_as_rewinding_every_iteration() {
    let src = "fn filled acc 0\n  acc\n\nfn filled acc n\n  filled (push acc n) (n - 1)\n\nfn one_list _\n  base = push [] 1\n  length (filled base 8)\n\nfn many at stop tally\n  more at stop tally (at > stop)\n\nfn more _ _ tally true\n  tally\n\nfn more at stop tally false\n  many (at + 1) stop (tally + one_list at)\n\nmain = print \"{many 1 20 0}\"\n";
    let (program, inference) = compiled(src);
    let lines = super::report(&program, &inference, &crate::linear::in_place_pushes(&program));
    let many = lines.iter().find(|l| l.starts_with("many/3")).expect("many is reported");

    assert!(
        many.contains("bracketed with its cluster") && !many.contains("every iteration"),
        "the report gave a cluster the self-loop's words: {many}"
    );
}

/// A list accumulator holds KValues, so licensing one to cross a rewind
/// turns on what those values are. Integers point at nothing and are as
/// safe to carry as the raw bytes of a string builder; a string element
/// is a pointer into the arena the rewind is about to reclaim. Both
/// halves are asserted here because a license that admitted either would
/// pass an example that only tested the first.
#[test]
fn a_list_of_scalars_may_cross_a_boundary_and_a_list_of_strings_may_not() {
    let ints = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs 2)\n\nmain = print \"{length (go 3 [])}\"\n";
    assert!(loops_of(ints).contains(&("go".to_string(), 2)), "a list of integers is refused");

    let strs = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs \"a\")\n\nmain = print \"{length (go 3 [])}\"\n";
    assert!(
        !loops_of(strs).contains(&("go".to_string(), 2)),
        "a list of strings was licensed, and its elements can dangle"
    );
}

/// `push xs (n + 1)` is the same loop as `push xs n` with arithmetic in
/// it, and arithmetic over pointer-free operands is pointer-free. The
/// element test used to see through a literal and a builtin call and stop
/// at a binary operator, which is an arbitrary place for a rule about
/// what a value can point at to end.
///
/// The string case is the boundary and is here for the same reason the
/// other examples pair their halves: `+` over two strings answers a
/// string, which is a pointer, so the operator alone proves nothing and
/// the operands are what the rule reads.
#[test]
fn arithmetic_in_a_pushed_element_is_still_pointer_free() {
    let sums = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs (n + 1))\n\nmain = print \"{length (go 3 [])}\"\n";
    assert!(
        loops_of(sums).contains(&("go".to_string(), 2)),
        "a pushed sum of two numbers is refused"
    );

    let joins = "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs (\"a\" + \"b\"))\n\nmain = print \"{length (go 3 [])}\"\n";
    assert!(
        !loops_of(joins).contains(&("go".to_string(), 2)),
        "a pushed join of two strings was licensed, and a string is a pointer"
    );
}

#[test]
fn scalar_and_threaded_loop_is_eligible() {
    // the jsonbench shape: a threaded string, a counter, an accumulator
    let src = "fn crunch _ 0 acc\n  acc\n\nfn crunch cs n acc\n  crunch cs (n - 1) (acc + length \"beat {n}\")\n\nmain =\n  s = \"abc\"\n  print \"{crunch s 3 0}\"\n";
    assert!(loops_of(src).contains(&("crunch".to_string(), 3)));
}

/// `push acc n` extends its own previous value, and this used to keep the
/// loop off the carry path: copying a growing accumulator every rewind
/// cost quadratic bytes, and the ch10 teaching program went 33 KB to
/// 16 MB of traffic when it was carried.
///
/// A growing list of scalars carries now, because that copy no longer
/// happens: its storage is malloc'd rather than arena, so a rewind has
/// nothing to copy. The same ch10 program is byte-identical at 1,000 and
/// 10,000 elements and holds a quarter of the arena at 100,000.
///
/// The gate still stands for everything else it was written for — a
/// growing map, a growing byte builder, a list whose elements are
/// pointers — which is what the second half here insists on.
#[test]
fn a_growing_collection_of_scalars_carries_unless_its_key_lives_in_the_arena() {
    let ints = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (push acc n)\n\nmain = print \"{collect 3 []}\"\n";
    let (program, inference) = compiled(ints);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));
    assert!(
        beats.ids.contains_key(&("collect".to_string(), 2)),
        "a growing list of scalars is still refused the carry"
    );

    // A growing map carries too now, under a literal key: its pairs moved
    // out of the arena the same way a list's items did, and its sorted
    // view was always malloc'd, so the rewind can reach neither.
    let maps = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (put acc \"k\" n)\n\nmain = print \"{collect 3 {}}\"\n";
    let (mp, mi) = compiled(maps);
    let mbeats = super::beat_loops(&mp, &mi, &crate::linear::in_place_pushes(&mp));
    assert!(
        mbeats.ids.contains_key(&("collect".to_string(), 2)),
        "a growing map under a literal key is still refused"
    );

    // And an interpolated key is not, because that string is assembled in
    // the arena each time round and the view would hold it after the
    // rewind freed it. This is the boundary the licence stops at.
    let keyed = "fn collect 0 acc\n  length acc\n\nfn collect n acc\n  collect (n - 1) (put acc \"k{n}\" n)\n\nmain = print \"{collect 3 {}}\"\n";
    let (kp, ki) = compiled(keyed);
    let kbeats = super::beat_loops(&kp, &ki, &crate::linear::in_place_pushes(&kp));
    assert!(
        !kbeats.ids.contains_key(&("collect".to_string(), 2)),
        "a map keyed by an interpolation was licensed, and that key lives in the arena"
    );
}

#[test]
fn bounded_accumulator_carries() {
    // a fixed-shape rebuild does not grow with the iteration count; the
    // carry evacuates it each rewind.
    let src = "main = print \"{spin 10 [0 1]}\"\n\nfn spin 0 acc\n  length acc\n\nfn spin n acc\n  a = acc[1]\n  b = acc[2]\n  spin (n - 1) [b (a + b)]\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert_eq!(beats.carried.get(&("spin".to_string(), 2)), Some(&vec![1]));
}
#[test]
fn tail_entry_from_acyclic_caller_is_demoted() {
    // go tail-calls into spin's loop, but go is acyclic: the entry is
    // demoted to a plain call (one bounded frame) and spin brackets.
    let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("spin".to_string(), 2)));
    assert!(beats.demoted.contains(&(("go".to_string(), 1), ("spin".to_string(), 2))));
}

#[test]
fn closure_threaded_loop_with_demoted_entry_is_a_beat() {
    // f is a closure handed through unchanged: immutable internals,
    // wholly below the entry mark, safe to carry across the rewind.
    let src = "fn go f n\n  spin f n 0\n\nmain =\n  salt = (x -> x * 2)\n  print \"{go salt 5}\"\n\nfn spin f 0 acc\n  f acc\n\nfn spin f n acc\n  step = \"seen {n}\"\n  spin f (n - 1) (acc + length step)\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
}

#[test]
fn a_cluster_entered_by_an_acyclic_tail_call_is_a_beat() {
    // counted hands the cycle its frame and is never entered again, so
    // that one edge becomes a plain call and the cycle gets its bracket.
    let src = "fn stepping n tally\n  more n tally (n <= 0)\n\nfn more _ tally true\n  tally\n\nfn more n tally false\n  junk = \"k{n}\"\n  stepping (n - 1) (tally + length junk)\n\nfn counted n\n  stepping n 0\n\nmain = print \"{counted 5}\"\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("stepping".to_string(), 2)));
}

#[test]
fn a_cluster_growing_its_carried_list_through_a_name_stays_out() {
    // grammar_check's capture loop: `grown = push acc x` handed on as
    // `grown` grows the carried slot as `push acc x` would, and growth in
    // a carried slot refuses the cluster. The name hid it until
    // 2026-10-10, and once a carrying cluster could be entered by a tail
    // call the gate died in k_deep_copy.
    let src = "fn each acc at\n  stepping acc at (at > 3)\n\nfn stepping acc _ true\n  acc\n\nfn stepping acc at false\n  grown = push acc \"x{at}\"\n  each grown (at + 1)\n\nfn start acc\n  each acc 1\n\nmain = print \"{length (start [])}\"\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(!beats.ids.contains_key(&("each".to_string(), 2)));
    assert!(!beats.ids.contains_key(&("stepping".to_string(), 3)));
}

#[test]
fn a_cluster_the_entering_caller_is_reached_from_stays_grow_only() {
    // counted enters the cycle by a tail call, and the cycle hands
    // `tiny`, which tail-calls counted, on as a value. A value can be
    // called in tail position wherever it goes, so the cycle may reach
    // counted with no frame held, and demoting the entry could retain a
    // frame per iteration and trade the arena for the stack.
    let src = "fn stepping n tally\n  more n tally (n <= 0)\n\nfn more _ tally true\n  tally\n\nfn more n tally false\n  junk = \"k{n}\"\n  stepping (n - 1) (tally + length junk + apply tiny n)\n\nfn apply f x\n  f x\n\nfn tiny 0\n  counted 0\n\nfn tiny _\n  0\n\nfn counted n\n  stepping n 0\n\nmain = print \"{counted 5}\"\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(!beats.ids.contains_key(&("stepping".to_string(), 2)));
}

#[test]
fn a_cluster_reaching_its_caller_under_a_plain_call_is_a_beat() {
    // The cycle reaches counted only through `tiny n`, a plain call whose
    // frame stays on the stack while counted runs. Demoting the entry
    // adds a frame to each level of that nesting and none to a lap, so
    // the cycle keeps its bracket (#1892, the bc port's loop statement).
    let src = "fn stepping n tally\n  more n tally (n <= 0)\n\nfn more _ tally true\n  tally\n\nfn more n tally false\n  junk = \"k{n}\"\n  stepping (n - 1) (tally + length junk + tiny n)\n\nfn tiny 0\n  counted 0\n\nfn tiny _\n  0\n\nfn counted n\n  stepping n 0\n\nmain = print \"{counted 5}\"\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("stepping".to_string(), 2)));
}

#[test]
fn list_threaded_loop_is_a_beat() {
    // the list is handed onward unchanged every iteration: below the
    // mark, header never mutated, safe across the rewind.
    let src = "fn go xs n\n  spin xs n 0\n\nmain =\n  base = [10 20 30]\n  print \"{go base 5}\"\n\nfn spin xs 0 acc\n  acc + length xs\n\nfn spin xs n acc\n  garbage = \"iteration {n}\"\n  spin xs (n - 1) (acc + length garbage)\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
}

#[test]
fn map_threaded_loop_is_a_beat() {
    // the map is handed onward unchanged every iteration, so it threads
    // the way a list does: its sorted view is malloc'd, so a read writes
    // no arena pointer into the below-mark header, and nothing carries.
    let src = "fn go m n\n  spin m n 0\n\nmain =\n  prices = { \"a\":1 \"b\":2 }\n  print \"{go prices 3}\"\n\nfn spin m 0 acc\n  acc + length m\n\nfn spin m n acc\n  step = \"seen {n}\"\n  spin m (n - 1) (acc + length step)\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("spin".to_string(), 3)));
    assert_eq!(beats.carried.get(&("spin".to_string(), 3)), None);
}

#[test]
fn demoted_entry_survives_a_clone_sharing_the_callers_name() {
    // a bare-enrolled clone named `go` joins the local go's dispatch
    // group; spin stays an eligible user loop, so go's entry must stay
    // demoted — losing the bracket while spin keeps its rewinds
    // corrupts live memory (the vse fold/fold_at miscompile).
    let src = "fn go n\n  spin n 0\n\nmain = print \"{go 3}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
    let (mut program, _) = compiled(src);
    let mut clone = program.fns.iter().find(|d| d.name == "go").unwrap().clone();
    clone.synthetic = true;
    clone.file = std::sync::Arc::from("std/list");
    program.fns.push(clone);
    let inference = infer::infer(&program);
    let beats = beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.demoted.contains(&(("go".to_string(), 1), ("spin".to_string(), 2))));
}

#[test]
fn tail_entry_from_cyclic_caller_is_demoted() {
    // ping and pong form a tail cycle, and pong's last lap enters spin.
    // The plain call holds pong's one frame while spin runs, and pong
    // returns before the cycle could go round again, so the stack stays
    // bounded. This read "stays ineligible" until 2026-10-10, which kept
    // the loop of every tree-walking interpreter grow-only: their
    // dispatcher is a tail cycle and every while loop is entered from it.
    let src = "main = print \"{ping 3}\"\n\nfn ping n\n  pong n\n\nfn pong 0\n  spin 2 0\n\nfn pong n\n  ping (n - 1)\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(beats.ids.contains_key(&("spin".to_string(), 2)));
    assert!(beats.demoted.contains(&(("pong".to_string(), 1), ("spin".to_string(), 2))));
}

#[test]
fn tail_entry_the_loop_reaches_back_stays_ineligible() {
    // spin leaves by a tail call to pong, and pong enters spin by one, so
    // the entry is an edge of a tail cycle: demoting it would hold a
    // frame for every lap round that cycle. Whatever the cluster
    // analysis makes of the pair, that edge stays a tail call.
    let src = "main = print \"{pong 3}\"\n\nfn pong 0\n  0\n\nfn pong n\n  spin n 0\n\nfn spin 0 acc\n  pong (acc - acc)\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
    let (program, inference) = compiled(src);
    let beats = super::beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));

    assert!(!beats.demoted.contains(&(("pong".to_string(), 1), ("spin".to_string(), 2))));
}

#[test]
fn non_tail_outside_call_is_fine() {
    let src = "main = print \"{1 + spin 3 0}\"\n\nfn spin 0 acc\n  acc\n\nfn spin n acc\n  spin (n - 1) (acc + length \"beat {n}\")\n";
    assert!(loops_of(src).contains(&("spin".to_string(), 2)));
}

fn module_fixture(name: &str, lib: &str, entry: &str) -> crate::ast::Program {
    let dir = std::env::temp_dir().join(format!("kanso-beat-{name}"));
    std::fs::create_dir_all(dir.join("work")).expect("fixture dir");
    std::fs::write(dir.join("work/work.kso"), lib).expect("fixture writes");
    let main = dir.join("main.kso");
    std::fs::write(&main, entry).expect("fixture writes");
    crate::compile_entry(&main.to_string_lossy(), entry).unwrap()
}

#[test]
fn a_module_loop_is_one_group_with_one_spelling() {
    // enrollment gives an imported function a bare twin, and before the
    // alias canonicalization the twin made every module-internal
    // recursion read as a call between two groups — no loop inside an
    // imported module could ever beat. One spelling, one group, and the
    // chain license reaches it.
    let lib = "import \"std/text\"\n\npub fn stamp acc 0\n  acc\n\npub fn stamp acc n\n  stamp (text/append acc \"x{n}\") (n - 1)\n\npub fn start _\n  text/bytes \"\"\n\npub fn finish acc\n  length (text/utf8 acc)\n";
    let entry = "import \"./work\"\n\nprint \"{work/finish (work/stamp (work/start 0) 9)}\"\n";
    let program = module_fixture("canon", lib, entry);
    let inference = infer::infer(&program);
    let muts = crate::linear::in_place_pushes(&program);
    let loops = beat_loops(&program, &inference, &muts);

    assert!(loops.ids.contains_key(&("work/stamp".to_string(), 2)), "got {:?}", loops.ids);
}

#[test]
fn a_locally_bound_name_is_never_rewritten() {
    // `first` here is a local binding, and std/list also exports a
    // `first`. Rewriting the local to the function value fed a closure
    // to append at runtime — kq's pretty-printer found it. A name that
    // is ever locally bound keeps its clones and its old dispatch.
    let dir = std::env::temp_dir().join("kanso-beat-shadow");
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let _ = std::fs::remove_dir_all(dir.join("work"));
    std::fs::write(
        dir.join("helpers.kso"),
        "import \"std/list\"\n\npub fn label xs\n  first = list/first xs\n  \"head {first}\"\n",
    )
    .expect("fixture writes");
    std::fs::write(dir.join("main.kso"), "print \"{label [7 8 9]}\"\n").expect("fixture writes");
    let program = crate::compile_module(&dir, false).unwrap();
    let label = program.fns.iter().find(|d| d.name == "label").expect("label compiles");
    let mut idents = crate::hash::Set::default();
    for stmt in &label.body {
        collect_idents(stmt, &mut idents);
    }

    assert!(idents.contains("first"), "the local was rewritten: {idents:?}");
}

fn collect_idents(stmt: &crate::ast::Stmt, out: &mut crate::hash::Set<String>) {
    let e = match stmt {
        crate::ast::Stmt::Bind { expr, .. } | crate::ast::Stmt::Expr(expr) => expr,
    };
    idents_in(e, out);
}

fn idents_in(e: &crate::ast::Expr, out: &mut crate::hash::Set<String>) {
    if let crate::ast::Expr::Ident(n, _, _) = e {
        out.insert(n.to_string());
    }
    if let crate::ast::Expr::Str(parts, _) = e {
        for p in parts {
            if let crate::ast::TemplatePart::Interp(inner) = p {
                idents_in(inner, out);
            }
        }
    }
    if let crate::ast::Expr::App { head, args, .. } = e {
        idents_in(head, out);
        for a in args {
            idents_in(a, out);
        }
    }
}

#[test]
fn json_decode_loops_stay_conservative() {
    // kanso-json's scanners are mutually recursive and thread record and
    // list accumulators — those stay out. The two encoders thread a byte
    // builder by pointer identity, which is exactly what the chain
    // license admits: raw bytes hold no pointers, so nothing in the
    // accumulator can dangle across a rewind.
    //
    // Read through an import, which is how a program meets the library.
    // Until 2026-10-10 this compiled lib/json as a root and leaned on the
    // carry boundary matching `lib/` as well as `std/`, which every user
    // directory called `lib` matched too.
    let src = "import \"std/json\"\n\nprint (json/encode [1 2])\njson/decode! \"[1 2]\" .> print\n";
    let program = crate::compile_entry("main.kso", src).unwrap();
    let inference = infer::infer(&program);
    let loops = beat_loops(&program, &inference, &crate::linear::in_place_pushes(&program));
    let mut licensed: Vec<(String, usize)> =
        loops.ids.into_keys().filter(|(n, _)| n.starts_with("json/")).collect();
    licensed.sort();
    // The escaper's four-group cycle qualified for a day on 2026-09-07:
    // it threads the encoder's byte builder by identity, the licence the
    // encoders hold. It allocates nothing in the arena — an in-place
    // append grows outside it, the slice under one is the fused copy,
    // find2_below answers an integer — and the classifier sees that now,
    // so the cycle is not a beat: no bracket, no rewind, nothing to free.
    //
    // The two encoders were beats until 2026-09-25, for one allocation:
    // what a nested map builds as it descends -- `entries` then, `keys`
    // and `values` from 2026-09-27. From 2026-10-21 the descent was a
    // region around `entry_count`, which may build the map's sorted view.
    // Since 2026-10-22 `entry_count` is pure, as `key_at` and `value_at`
    // are: the view comes from malloc, and only a sort's scratch index
    // lands in the arena, where the enclosing loop's rewind takes it. So
    // the descent is no region, and the loops allocate nothing an
    // iteration outlives.
    //
    // The string scanners joined on 2026-10-10, when a cluster entered by
    // a tail call began to carry its slots. That is this library read on
    // its own: a program that imports std/json emits no carried rewind
    // in them, and decoding five 300,000-character strings with an
    // escape every three characters read 78,278,587 instructions with
    // the licence and without it.
    //
    // `unicode_at` joined on 2026-10-12, when the four hex digits of a
    // `\u` escape stopped riding a name past their err and the escape's
    // tail moved into a function of its own. It takes the scanners' slots
    // and tail-calls `str_chars`, so it is one more scanner.
    //
    // The blank skip before a value joined on 2026-10-13. `value_for`
    // and `value_blank` are one cycle, entered by a tail call from
    // `parse_value`, and the cycle reaches `parse_value` again only under
    // the plain calls an array or object makes for each element.
    //
    // `string_scan` and the four-argument `str_run` left on 2026-10-18,
    // when the decoder took spans: the scan is `text/span_to` now, and
    // the run it answers reaches `str_run` as the three fields the flatten
    // pass carries, in a group the pass made. The mem corpus reads the
    // same counters as before the move.
    let scanners: Vec<(String, usize)> = [
        ("json/str_chars", 3),
        ("json/str_low", 5),
        ("json/str_pair", 4),
        ("json/str_surrogate", 4),
        ("json/str_unicode", 3),
        ("json/unicode_at", 4),
        ("json/value_blank", 3),
    ]
    .into_iter()
    .map(|(n, a)| (n.to_string(), a))
    .collect();
    assert_eq!(
        licensed, scanners,
        "the string scanners and the blank skip are the library's only \
         beats: the escaper and the encoders allocate nothing an iteration \
         outlives, and scanners threading records or lists stay on the \
         grow-only arena"
    );
    let regions: Vec<usize> = loops.regions.iter().map(|(_, line, _)| *line).collect();
    assert_eq!(
        regions,
        Vec::<usize>::new(),
        "the library has no region: `entry_count` is pure, as `key_at` and `value_at` are"
    );
}

#[test]
fn a_cycle_rewinds_on_one_edge_and_two_cycles_on_two() {
    // Every cycle in a cluster's internal tail graph closes on a back
    // edge of the walk, and a simple cycle closes on exactly one, so a
    // trip round it rewinds once however many members it crosses. Two
    // cycles sharing a member close on two.
    use super::back_edges;
    let g = |n: &str| (n.to_string(), 2);
    let four = vec![(g("a"), g("b")), (g("b"), g("c")), (g("c"), g("d")), (g("d"), g("a"))];
    assert_eq!(back_edges(&four), vec![(g("d"), g("a"))]);
    let figure_eight = vec![(g("a"), g("b")), (g("b"), g("a")), (g("a"), g("c")), (g("c"), g("a"))];
    assert_eq!(back_edges(&figure_eight), vec![(g("b"), g("a")), (g("c"), g("a"))]);
}

#[test]
fn a_builtin_is_named_bare_whichever_way_it_arrives() {
    // A std wrapper inlines to `builtin_x`; a qualified import spells
    // `text/x`; the classifier's lists name `x`.
    use super::bare_builtin;
    assert_eq!(bare_builtin("builtin_append"), "append");
    assert_eq!(bare_builtin("text/slice"), "slice");
    assert_eq!(bare_builtin("find2_below"), "find2_below");
}

/// The two questions `classify` asks, answered the old way: a walk of the
/// whole program per group. Kept as the oracle for the index that replaced
/// it, which is 30.25% of `kanso build bench/runbench` cheaper.
fn tail_calls_to<'a>(
    program: &'a Program,
    name: &'a str,
    arity: usize,
) -> impl Iterator<Item = bool> + 'a {
    program.fns.iter().flat_map(move |decl| {
        let in_group = decl.name == name && decl.params.len() == arity;
        tail_exprs(decl.body.last()).into_iter().filter_map(move |tail| {
            let Expr::App { head, args, piped: false, .. } = tail else { return None };
            let Expr::Ident(callee, _, _) = head.as_ref() else { return None };
            (callee == name && args.len() == arity).then_some(in_group)
        })
    })
}

fn by_search(program: &Program, name: &str, arity: usize) -> (bool, bool) {
    (
        tail_calls_to(program, name, arity).any(|in_group| in_group),
        tail_calls_to(program, name, arity).any(|in_group| !in_group),
    )
}

/// Every group the program declares, asked both ways.
///
/// Also every group NAMED by a tail call and not declared, and a handful
/// that appear nowhere: the index answers by lookup and a missing entry
/// has to read as "no", which a corpus of declared groups alone cannot
/// show.
fn agrees_over(source: &str) {
    let program = crate::compile("sample.kso", source, false).expect("the sample compiles");
    let tails = TailCalls::of(&program);
    let mut asked = 0;
    let mut groups: Vec<(String, usize)> =
        program.fns.iter().map(|d| (d.name.clone(), d.params.len())).collect();
    for (name, arity) in program.fns.iter().map(|d| (d.name.clone(), d.params.len())) {
        groups.push((name.clone(), arity + 1));
        groups.push((format!("{name}_no_such_group"), arity));
    }
    groups.push(("nothing_declares_this".to_string(), 0));
    for (name, arity) in groups {
        let want = by_search(&program, &name, arity);
        let got = (tails.has_self_tail(&name, arity), tails.outside_tails(&name, arity));
        assert_eq!(got, want, "the index and the search disagree on {name}/{arity}");
        asked += 1;
    }
    assert!(asked > 6, "the sample asked only {asked} questions");
}

/// A self tail call, an outside tail call into the same group, and a group
/// with neither — which are the three answers `classify` reads.
#[test]
fn the_index_answers_what_the_walk_answered() {
    agrees_over(
        "fn count 0 acc\n  acc\n\nfn count n acc\n  count (n - 1) (acc + n)\n\n\
         fn start n\n  count n 0\n\nfn plain n\n  n + 1\n\n\
         main = print \"{start 10} {plain 3}\"\n",
    );
}

/// The same program the emitter really sees, which is where a group named
/// by a tail call it does not declare turns up.
#[test]
fn a_real_program_reads_the_same_both_ways() {
    agrees_over(
        "fn climbed i stop acc\n  reached i stop acc (i > stop)\n\n\
         fn reached _ _ acc true\n  acc\n\n\
         fn reached i stop acc false\n  climbed (i + 1) stop (acc + i)\n\n\
         fn summed n\n  climbed 1 n 0\n\nmain = print \"{summed 60}\"\n",
    );
}
