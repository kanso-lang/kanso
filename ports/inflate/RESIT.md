# Re-sit: inflate

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the journal
re-test are in `resit/`, named after the entry they test. Timings are wall
time on a 4-CPU container shared with three other agents, so they are
rough; instruction counts are from callgrind.

## Result

- check.sh: pass. 55 unit tests in five modules, 77 fixtures.
- Engines agree: yes, on all 77 fixtures. Two bugs found during the re-sit
  (Bugs, below) are outside the port's paths; F3's stdin disagreement is
  still there and the port still avoids it.
- Lines changed by the migration: 128 lines of `diff -r -x bugs -x resit`
  against ports-orig/inflate, 5 of them in test files.
- No expected-output file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| M1 | `` error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes. One file then needed `` error[unused]: unused import "std/text" `` fixed, which also said so. | 5 in 4 files |
| M2 | `` error[exhaustive]: this can be an err and `next_block` has no arm for it — dispatch on it here, or give `next_block` an `(err _)` arm (in inflate.kso) `` (a name bound to a raising call carries its err, #1884) | yes, but the arm it suggests made the release decode about 60 times slower when written on the per-symbol functions (N1). The port uses `(err _)` arms on the per-block functions (`next_block`, `noted`, `raw_end`, `more_members`) and `.>` on the per-symbol ones (`literal_or_match`, `distance`). | 6 |
| M3 | `` error[exhaustive]: this can be a none and `text/append` has no arm for it — resolve it here, or give `text/append` a `none` arm `` (text/slice past the end answers none, #1890) | no; an arm on a std function is impossible. Six ranges are checked before the slice and got dead arms: the decoder's three appends go through `text/to_span` and a two-arm `appended`, the way ch08 teaches, and `wrap.kso` and `gzip.kso` got `stored_part` and `inside`. One was a real case: `gzip? (text/slice src p (p + 1))` at the last byte is now `none`, and `gzip?` gained `fn gzip? none` answering false, which says what the old clamped slice did by accident. `length_base` now takes `list/to_list (list/take ...)`. | 8 |
| M4 | `` error[name]: unknown name `bad_code?` `` and nine others, in test files (a test file is an outside consumer, 2026-10-12 gavel) | it names the missing name, not the rule. The helpers the tests call were made pub: `bad_code?`, `length_slot`, `dist_slot`, `output_of`, `format_flag`, `count_of`, `hex_from`, `ratio`, `padded`. | 9 helpers (18 arm heads) |
| M5 | `` error[opacity]: `concat` is private to module `list` — only pub names cross an import `` in container_test.kso | no. `list/concat` is pub; the test file had not imported std/list. Adding the import fixed it (N6). | 1 |
| M6 | `` error[exhaustive]: this can be a none and `os/read_bytes` has no arm for it `` at cli/cli.kso:136, from `kanso test cli` only; `kanso check cli` and `kanso check .` say ok | no. The arm `fn with_file (request _ _ _ _ none) _` claims the none, and `kanso test` does not see it. `with_file` now dispatches on `r.path` in a helper of its own. `bugs/test_checks_module_differently/`. | 1 |
| M7 | `` error[exhaustive]: this can be a none and `text/utf8` has no arm for it `` in deflate_test.kso | no. Rewrote the check of the last six bytes as `text/ends_with? (text/utf8 out) "ababab"`. | 1 |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | `&` and `|` share one precedence level | confusing-semantics | major | language | fixed | `resit/f01_bits_mix.kso`: `bits/shl acc 1 | code & 1` is refused with `` `|` and `&` bind equally, so a chain that mixes them is ambiguous — parenthesize the part that goes first ``; with `(code & 1)` it prints 7. `resit/f01b.kso`: `(5 | 3) & 4095` is accepted. Log 2026-10-12, "`&`, `|` and `^` mixed without parentheses are refused". |
| F2 | no binary standard output | stdlib-gap | major | domain-lib | still | `resit/f02_write_bytes.kso`: `io/write (text/to_bytes [31 139 255])` checks ok and stops with `write takes a string` on both engines, now naming the line. |
| F3 | no binary standard input, and the engines disagree | engine-bug | major | domain-lib | still | `bugs/stdin_bytes_not_text.kso` with `\x1f\x8b\xff` on stdin: native prints `[31 139 255]`, the interpreter `stream did not contain valid UTF-8`. |
| F4 | every byte read answers `int | none` | aesthetics | minor | language | still | `byte_at`/`zero_past_end` are unchanged and still needed; `xs[i]!` still answers a box (see kv's re-sit, F5). #1890 added seven more sites of the same shape (M3). |
| F5 | a binding pattern or field read on an err stops the program; natively a false stack overflow | engine-bug | major | language | partly | `bugs/destructured_err_overflows_stack.kso`: both engines now print `cannot destructure err "nothing to halve" as `pair`` and the line; no false stack overflow. `resit/f05`: `kanso check` still passes `pair a b = p` with `p` an err. A field read on the err now passes the err on, and the endpoint reports `["nothing to halve" "nothing to halve"]` (N7). |
| F6 | a keyed read after a `return ... if` binds nothing | engine-bug | minor | language | still | `bugs/keyed_read_after_return.kso` and `resit/f06_keyed_after_return.kso`: `` unknown name `after` `` and `` unknown name `lens` ``. |
| F7 | a map read and then written is copied on every write | performance | blocker | core-lib | still | `resit/f07_f20_bench` (the port's own benchmark), release, `readput`: 10,000 in 0.13 s, 40,000 in 2.0 s, 160,000 in 34.5 s. Still quadratic; the journal had 0.42 s and 4.8 s. `map` alone: 160,000 in 0.07 s. |
| F8 | no array that can be written at an index | missing-feature | major | core-lib | still | nothing in lib/list writes at an index; `resit/f08_put_list.kso`: `put [1 2 3] 2 9` checks ok and stops with `put takes a map, a key, and a value`. |
| F9 | `list/take` and `list/map` answer lazy sequences, found at run time | diagnostic | minor | language | partly | `resit/f09_lazy.kso`: `push (list/take extra 3) 258` is refused at check with a `list/to_list` hint (log 2026-10-13). `resit/f09b.kso`: a `list/map` held in a name and handed to `list/concat` checks ok and stops with `concat takes two lists`, no hint, now with the line. |
| F10 | a type name from std/list takes over a function call | confusing-semantics | minor | language | fixed | in a module, `resit/f10d` prints `3 1` on both engines (the port's case). In a play file the native build still builds `list/repeated` (N5, Bugs). |
| F11 | no hex integer literals | missing-feature | minor | core-lib | fixed | `resit/f11_hex.kso`: `0xedb88320`, `0x30 + 1`, `0b1010` print `3988292384 49 10` (#1889). The port's constants are still decimal; changing them was not needed to migrate. |
| F12 | eighty columns push a guard's condition off the line | aesthetics | minor | language | ruled-no | #1882's gavel spans lists and maps and says "Operator chains, `if` and guards are not covered". |
| F13 | no multi-line literal, so a help text is fourteen effects | missing-feature | minor | language | fixed | `resit/f13_usage.kso` prints a usage text from a `"""` block, commas included. Text blocks predate the port; the book does not mention them (kv's N7). |
| F14 | `_` does not match none | confusing-semantics | minor | language | still | `resit/f14_none_catchall.kso`: `command args[2] 0` with only `"cat"` and `_` arms gets `` this can be a none and `command` has no arm for it ``. |
| F15 | an effect handed to a function that wants a value passes the checker | diagnostic | minor | language | fixed | `resit/f15`: `os/read_bytes! a[1]!` is refused with `this is an effect — a box the words open — and `os/read_bytes!` takes a value; open it with `.>``. Log 2026-10-14, "a box handed to a library function that reads it is refused at the call". |
| F16 | naming a value changes what compiles | confusing-semantics | minor | language | partly | for errs, #1884 made a name carry its call's err. For a none, `resit/f16` still refuses `file heads[key]` and accepts `first = heads[key]` then `file first`. |
| F17 | names the program may not use: `done`, `put`, a parameter named like a type elsewhere | refactoring-hazard | nit | language | still | `resit/f17_names.kso`: `` the name `put` is already taken `` and `` `done` is already a declaration ``. The cross-file case is kv's F8, still. |
| F18 | a foreign err type needs the qualified name; the book says bare works | confusing-semantics | nit | language | still | `resit/f18`: `(err e:corrupt)` gets `` no type is called `corrupt` `` with no suggestion; `(err e:d/corrupt)` works. ch07 still says "bare spellings work too: an imported pub also joins the short-name overload space", which the 2026-10-12 gavel ended. |
| F19 | an entry file holds no functions; `check .` inside a module fails; play files cannot be checked | tooling | nit | tooling | partly | `kanso check .` from inside a module now resolves `../` imports (kv's F13). An entry still holds statements only, and `kanso check` still refuses a play file with declarations (mustache's N8). |
| F20 | pushing onto a list held in a record is quadratic on the interpreter | performance | major | core-lib | still | `resit/f07_f20_bench recpush` on `--interp`: 10,000 in 0.48 s, 40,000 in 7.45 s; `foldpush` 40,000 in 0.03 s. Native `recpush` 160,000 in 0.02 s. About three times faster than the journal, still quadratic. |
| F21 | `/` and `%` are calls into the runtime | performance | minor | core-lib | fixed | `resit/f21`, release IR: `e % 16` and `e / 16` compile to `srem i64 ..., 16` and `sdiv i64 ..., 16` on the int path, with `k_mod`/`k_div` only on the slow path. |
| F22 | a record per decoded symbol | performance | minor | core-lib | still | release decode of big.txt.gz: `k_rec` 4.52 M instructions of 103.9 M (the journal had 4.66 M). |
| F23 | a closed stdout pipe panics the interpreter | engine-bug | minor | tooling | fixed | `inflate hex big.txt.gz | head -1` exits 141 quietly on the interpreter and the release binary (log 2026-10-12, "a closed pipe stops every engine quietly"). |
| F24 | `_` refused in a binding pattern, accepted in a parameter | aesthetics | nit | language | still | `resit/f24_underscore_binding.kso`: `` `_` does not appear in binding patterns; omit fields with a keyed read ``. |

Counts: fixed 7 (F1, F10, F11, F13, F15, F21, F23), partly 4 (F5, F9,
F16, F19), still 12 (F2, F3, F4, F6, F7, F8, F14, F17, F18, F20, F22, F24),
ruled-no 1 (F12), moot 0, worse 0.

Throughput on today's compiler, the same 350 KB file (gzip, CRC included):
release 7 ms per run (`bench FILE 10` in 0.07 s), dev 24 ms, interpreter
2.0 s; 103.9 M instructions in release. The journal had 16 ms, 60 ms and
1.9 s.

## New friction

### N1: the `(err _)` arm the checker asks for made the decoder 60 times slower
- kind: performance
- severity: major
- scope: language
- where: deflate/inflate.kso:184 (`codes`, `literal_or_match`); the slow version is `resit/n_slow_err_arm_inflate.kso.txt`
- wanted: what the diagnostic suggests,

      fn codes src bp out lens dists
        g = decode src bp lens
        literal_or_match src g out lens dists

      fn literal_or_match _ e@(err _) _ _ _
        e

- wrote: `decode src bp lens .> (g -> literal_or_match src g out lens dists)`.
- why it matters: with the arm, one release decode of big.txt.gz took
  0.62 to 0.71 s and 14.0 billion instructions, 98.75% of them in `memcpy`,
  called from `k_beat_lap_carry` → `k_deep_copy` and from
  `k_b_append_slice` → `k_b_append_grow` → `k_copy_cold`: the output buffer
  is copied on every symbol. With `.>` it takes 7 ms. Both are what ch04
  teaches, the checker names only the arm, and nothing but a profile shows
  the difference. A three-function reduction with the same shape stayed
  linear, so there is no postcard yet; the slow file reproduces it.

### N2: `kanso test` refuses a module that `kanso check` accepts
- kind: engine-bug
- severity: major
- scope: tooling
- where: cli/cli.kso (`with_file`); bugs/test_checks_module_differently/
- wanted: `fn with_file (request _ _ _ _ none) _` claiming a missing path,
  as on 2026-10-10.
- wrote: a separate `read_path` dispatching on `r.path`.
- why it matters: a test file that calls nothing (`test_one = 1 == 1`) is
  enough to make `kanso test` refuse the module it sits beside, while
  `kanso check` on the module and on the program says ok. This is F9 of
  mustache's journal from the other side: the module's verdict depends on
  who imports it, and the test file is now an importer.

### N3: tests of private helpers need the helpers made pub
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: cli/cli.kso, cli/report.kso, compress/emit.kso, compress/fixed.kso, deflate/inflate.kso
- wanted: `test_dist_slots = dist_slot 1 30 == 1 and ...` beside a private `dist_slot`.
- wrote: `pub fn dist_slot`, and the same for eight other helpers, so the
  module's surface now lists its table builders and flag parsers.
- why it matters: the 2026-10-12 gavel says a test sees only pub names. A
  codec's riskiest code is its private tables, so the choice is to test
  them through the whole decoder or to publish them.

### N4: checked slices need dead `none` arms
- kind: confusing-semantics
- severity: major
- scope: core-lib
- where: deflate/inflate.kso (`appended`), compress/wrap.kso (`stored_part`), container/container.kso (`inside`)
- wanted: `text/append out (text/slice out start (start + len - 1))`, a range
  `copy` has already checked.
- wrote: `appended out (text/to_span out start (start + len - 1))` with
  `fn appended out none` answering `out`, and two other modules' helpers.
- why it matters: the same as kv's and mustache's N3. One of the eight
  sites (`gzip?` at the last byte) was a real case, and the arm made it
  explicit, which is the ruling's purpose; the other seven are noise.

### N5: in a play file the native build constructs a std type instead of calling the file's function
- kind: engine-bug
- severity: major
- scope: language
- where: bugs/play_native_builds_std_type_for_own_function.kso
- wanted: `fn repeated a b` in a play file importing std/list.
- wrote: nothing in the port, which is a module.
- why it matters: native prints `list/repeated 1 2 1` and the interpreter
  `3 1`. `kanso check` refuses play files with declarations, so nothing
  warns. kv's and mustache's re-sits found the mirror image, where the
  interpreter is the one that resolves to std.

### N6: a missing import is reported as `concat` being private
- kind: diagnostic
- severity: minor
- scope: language
- where: container/container_test.kso:27
- wanted: `list/concat head [3 0 0 0 0 0 0 0 0 0]` in a test file that does not import std/list.
- wrote: `import "std/list"`, after `` error[opacity]: `concat` is private to module `list` — only pub names cross an import ``.
  In a play file the same mistake says `` unknown name `list/concat` ``
  (`resit/n_concat_no_import.kso`), which is closer but still does not
  name the import.

### N7: an err read through two fields is reported as a list of reasons
- kind: diagnostic
- severity: minor
- scope: language
- where: resit/f05 (`p.left + p.right`, `p` an err); the file is kept as `f05/m/fieldread.kso.txt`
- wanted: the endpoint report ch04 shows: the reason, then `born in` with a
  function and line.
- wrote: nothing; this is what every engine prints:
  `error[endpoint]: unhandled err reached the executor: ["nothing to halve" "nothing to halve"]`,
  with no birthplace.

## Bugs

- `bugs/test_checks_module_differently/` (new): `kanso test m` refuses
  `m.kso` with `this can be a none and `os/read_bytes` has no arm for it`;
  `kanso check m` and `kanso check .` say ok, and the program runs.
- `bugs/play_native_builds_std_type_for_own_function.kso` (new): a play
  file's `fn repeated a b` is called on the interpreter (`3 1`) and built as
  std/list's `repeated` record natively (`list/repeated 1 2 1`).
- Still failing from the journal: `stdin_bytes_not_text.kso` (engines
  disagree), `keyed_read_after_return.kso`,
  `write_takes_bytes_only_sometimes.kso` (now with a line number).
- `destructured_err_overflows_stack.kso`: the false stack overflow is gone;
  both engines print the same runtime error. Check still passes it.
- No reduction for N1; `resit/n_slow_err_arm_inflate.kso.txt` swapped in
  for deflate/inflate.kso reproduces it.

## Comfort

The language side is clearly better than the journal describes. The silent
`&`/`|` mistake that topped the journal is now a refusal with the right
parenthesisation in the message, hex literals exist, `/ 16` is inline, a
closed pipe is quiet, a box handed to `read_bytes!` is caught at check, and
the false stack overflow on a destructured err is gone. The release decode
is twice as fast as the journal's. A new start would still meet the core
library's gaps unchanged: there is no array to write at an index, a map read
before a write is still quadratic, a list in a record is still quadratic on
the interpreter, and every in-range byte read or slice still wants a dead
`none` arm, now in more places. The new costs are about errs and tests. The
checker now insists on an `(err _)` arm wherever a decode result is passed
on, and on the per-symbol path the obvious arm made the decoder 60 times
slower with no hint why. `kanso test` disagreed with `kanso check` about the
CLI, and the test rule pushed nine private helpers into the public surface.
Domain gaps (byte stdin and stdout) are as the journal left them. The port
would come together faster in the first day and need a profiler sooner.
