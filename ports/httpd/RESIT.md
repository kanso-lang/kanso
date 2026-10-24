# Re-sit: httpd

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the re-test are
in `resit/`. A single file is run with `kanso play <file>` (native) and
`kanso play <file> --interp`; a directory with `kanso run .` and
`kanso run . --interp`.

## Result

- check.sh: **pass**. 130 unit tests in five modules, 9 fixture scenarios
  and the standalone-server check, three engines each.
- Engines agree: yes, on every fixture and on the standalone server.
- Lines changed by the migration: 94 changed lines (`diff -r
  ports-orig/httpd ports/httpd -x resit -x bugs`, counting `<` and `>`
  lines) in 13 `.kso` files. No expected file changed.

## Migration

| change | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| `text/concat` is gone (gavel 2026-10-13, `list/concat` joins two lists) | ``error[name]: unknown name `text/concat` — two lists join with `list/concat` ...`` | yes | 10 calls in 5 files |
| `text/slice` past the end answers `none` (gavel 2026-10-13, #1890) | ``error[exhaustive]: this can be a none and `text/utf8` has no arm for it`` | partly. Some pointed at the slice. One pointed at `parse ses.buffer` in server.kso, two functions from the slice that fed it, and printed no source line. Two more were caused by a slice in another module (Bugs, B1) | 8 slices wrapped in an `or_empty` helper (added to the `http`, `router` and `client` modules) |
| `std/path`'s `extension` can now answer `none` (N4) | ``error[exhaustive]: this can be a none and `content_type` has no arm for it`` | yes | 1 arm added: `pub fn content_type none` |
| a raising call bound to a name carries its err (log 2026-10-12, #1884) | ``error[exhaustive]: this can be an err and `dispatched` has no arm for it`` | partly: it does not mention `.>`, the book's fix | 1: `cli/run` became `read_options args 1 start .> (opts -> dispatched opts.words opts)` |
| a test file is an outside consumer (gavel 2026-10-12): building a record | ``error[opacity]: `http/limits` is foreign — only `http` builds a `limits` ...`` | yes | `pub fn limits_of` added to request.kso; server_test parses its requests instead of building them |
| the same: reading private names | ``error[name]: unknown name `sized` `` | yes for `sized` and `safe?`; no for `keep_alive?`, which passed check and failed at run time (Bugs, B2) | 3 functions made pub (`keep_alive?`, `safe?`, `sized`) |
| the same: an unqualified pub type in a test | ``error[type]: no type is called `reply`, so this arm can never match`` | no: it does not suggest `client/reply` (N3) | 1 |

## Journal

| entry | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a file with both definitions and statements will not `run` | tooling | confusing-semantics | minor | still | `docs/book/samples/ch03/membership.kso` with `kanso run`, as ch03 shows it: ``error: `membership.kso` is a library — nothing to run``. |
| F2 | `net/accept` outside a parallel group fails at once with "nothing connected" | domain-lib | engine-bug | major | still | `resit/f2/` (the bug, port 48297): exits at once with `unhandled err reached the executor: "nothing connected"`, and curl gets no connection. |
| F3 | a bound proves an index only when it is spelled `i < 1 or length xs < i` | language | diagnostic | major | still | `resit/f3.kso` (`return acc if i > length ds`) and `f3b.kso` (`length ds < i`) are both refused with ``this can be a none and `hex_digit` has no arm for it``, and the message names no guard that would work. `resit/f3c.kso`, `acc + ds[i]` under the same guard, is accepted and prints `99`. |
| F4 | `xs[1]!` is an effect, not an assertion | language | confusing-semantics | major | ruled-no | Gavel 2026-09-16, "`!` answers a box, at the index and at the name", restated for `text/slice!` on 2026-10-14 (#1943). Spans (2026-10-15, #1960) give a pure in-range slice for strings and byte lists, not for an index. |
| F5 | `text/split` with a separator held in a variable is typed as fallible | core-lib | stdlib-gap | minor | still | `resit/f5.kso`: ``this can be an err and `first_of` has no arm for it`` at `parts = text/split s sep` then `first_of parts`. Binding the call to a name no longer hides the err, so the helper's wrapper is still needed. |
| F6 | `text/bytes` answers a hidden "bytes" kind that `[]`, `text/concat` and `text/append` each treat differently | core-lib | engine-bug | major | partly | The engines agree now: `list/concat [1] (text/bytes "ab")` prints `[1 97 98]` on both (`resit/f6.kso`), and appendix B documents it. `text/append [] (text/bytes "cd")` still stops on both with `append takes bytes and a string, bytes, or byte`, and `text/append` has no entry in appendix B. |
| F7 | one namespace per module, plus every imported type's bare name, means a new name in one file breaks another | language | refactoring-hazard | major | partly | Fixed: a private `fn step` beside a file that imports std/list is called as written, and a test constant `srv` no longer touches the module (`resit/f7/` prints `13 3` on both engines; the test passes). Still: a top-level `fn after` in one file breaks `after = s + 1` in another with ``error[name]: `after` is already a declaration``. |
| F8 | a record field cannot be destructured under its own name when a type has that name | language | aesthetics | minor | still | `resit/f8.kso`: ``error[name]: `response` is already a declaration; rename the binding`` for `fn show (turn response state)`. |
| F9 | a long single-expression constant has to be broken with a binding nobody needs | language | aesthetics | minor | partly | The entry's wanted layout, the whole expression on an indented line, is still refused (`a single-expression constant is written inline`). Breaking after the `==` is accepted and the test passes (`resit/f9/`). |
| F10 | positional construction in alphabetical field order reads backwards | language | aesthetics | minor | partly | Field order was not required to be alphabetical at the time of the port either (archive, 2026-07-25, "REMOVED: ... record fields alphabetical"): `resit/f10.kso` declares `before` then `after` and prints `a`. `merge` (gavel 2026-10-13) covers updates. Construction is still positional, so two string fields can still be swapped silently. |
| F11 | formatting errors arrive one per run for some rules | tooling | tooling | nit | still | `resit/f11.kso` has four `not (...)` guards. `kanso check` reports only the one on line 5; after fixing it, only the one on line 4. |
| F12 | a list literal too long for one line cannot be written at all | language | missing-feature | major | fixed | Gavel 2026-10-13, "a bracketed list or map may span lines", and `"""` text blocks for strings. `resit/f12.kso` writes a five-route table as one spanning list literal and prints `5` on both engines. |
| F13 | a keyed read of every field passes `kanso check` and fails at run time, reported against the wrong module | language | diagnostic | major | partly | The location is right now: `resit/f13/` stops with `failed in h/with_header at ./h/h.kso:10` on both engines. It is still a run-time failure for a rule the source shows (`kanso check .` says ok), and the message still begins "a keyed read omits at least one field" about a read that omits none. |
| F14 | a module that checks clean can fail to check once something imports it | language | confusing-semantics | major | partly | Test files are no longer checked by `run`, `check` or `build` (gavel 2026-10-12). The rest stands: in this migration `kanso check http` said ok while `kanso check client` refused `http/request.kso:66`. |
| F15 | a foreign `pub type` can be dispatched on and read, but not built or taken apart | language | confusing-semantics | minor | still | `resit/f13/ap/`: `fn status_of (h/response _ _ status)` gives ``error[opacity]: `h/response` is foreign — its structure does not cross an import``. The bare `(response _ _ status)` destructures and prints `201` on both engines, even with the import of `h` removed. Building one outside its module is refused (this migration's test errors). |
| F16 | `std/net` can listen but cannot connect | domain-lib | stdlib-gap | major | still | std/net exports `listen`, `port`, `accept`, `read`, `write`, `close_conn`, `close_listener`; no dial. |
| F17 | a type error in a comparison is found only at run time, and reported with no location | language | diagnostic | major | partly | The location is fixed (gavels 2026-10-15 and 2026-10-17): `resit/f17.kso` stops with `failed in walk at f17.kso:15` on both engines. It is still found only at run time. |
| F18 | debugging output has to pass the formatter too | tooling | tooling | minor | still | `resit/f18.kso`, a one-line debug print chained into a call, is refused under `kanso play` with `a line holds at most 80 characters — this one has 83`. |
| F19 | a read on one connection stops every fiber in the program | domain-lib | engine-bug | blocker | still | `resit/f19/` (the bug): `late` and `early` are answered at the same instant. |
| F20 | fibers cannot share state, so a concurrent server could not share its store anyway | language | missing-feature | major | still | Chapter 06 still says "there is no channel, because the data flow is the channel", and no send, receive or channel exists in lib/. |
| F21 | a native program's stdout is lost when it is killed | language | engine-bug | major | fixed | Log 2026-10-12, "stdout reaches its reader, and a closed pipe stops every engine quietly". `resit/f21/` (the bug): after a kill one second in, the file holds `started` from the native binary and from the interpreter. |
| F22 | `text/slice` past the end answers an empty result | core-lib | confusing-semantics | minor | ruled-no | Gavel 2026-10-13, "`text/slice` past the end answers `none`" (#1890), chose `none` over clamping, so `text/slice whole 98 200` is not the tail. The test's `length whole` is still the way to say it. |
| F23 | a failing test says only `returned false` | tooling | tooling | minor | still | `resit/f23/`: `test_shown ... FAILED (returned false)`. std/expect, which would show both sides, is still "not in the shipped library". |
| F24 | no list patterns, so a command line is taken apart by index | language | missing-feature | minor | still | `resit/f24.kso`: `fn dispatched ["serve"] opts` gives `error[syntax]: expected a parameter pattern`. |
| F25 | JSON in a string literal is mostly backslashes | language | aesthetics | minor | still | No raw string. A `"""` text block interpolates too: `resit/f25.kso`, JSON in a text block, is refused with `kanso has no commas; enumerations are space-separated`, because `{"title":"a",...}` is read as an interpolation. |
| F26 | two arms that tie must be split by a third arm that repeats one of them | language | aesthetics | nit | still | `resit/f26.kso`: ``these `edited` arms tie`` for `edited _ st none _` beside `edited _ st _ (invalid reason status)`. The entry's second complaint, ordering arms that never meet, was ruled on 2026-10-13: "overload arms stay in specificity order" (#1888). |

Counts: fixed 2, partly 7, still 15, ruled-no 2, moot 0, worse 0.

## New friction

### N1: a test can no longer reach the decisions inside a module
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: http/server.kso:136, static/static.kso:25 and :53, http/request.kso, http/server_test.kso
- wanted: the tests as they were: `keep_alive?`, `safe?` and `sized` tested where they are decided, and a `request` or `limits` built directly.
- wrote: the three functions made pub, a `pub fn limits_of` that only the tests call, and server_test building its requests by parsing raw HTTP.
- why it matters: the journal listed "test files reach private functions" under what worked well. Under the 2026-10-12 gavel they cannot, so each tested decision is either made pub, which widens the module's surface for its tests, or tested through a longer path.

### N2: a `none` is reported two functions from where it was made, with no source line
- scope: language
- kind: diagnostic
- severity: minor
- where: http/request.kso:251 (the slice), reported at http/server.kso:69
- wanted: the error at `framed r (text/slice bs next (length bs))`.
- wrote: found it by reading the code. The message said ``this can be a none and `http/parse` has no arm for it ... (module ./router)`` at `--> ./http/server.kso:69:38`, and printed no source line under the arrow.
- why it matters: the slice's `none` went into a record's `rest` field, then into the session's buffer, then into `parse`. The message names the last step only, and the module it names, `router`, holds none of the code involved.

### N3: an unqualified type in a test file says the type does not exist
- scope: language
- kind: diagnostic
- severity: nit
- where: client/client_test.kso:46
- wanted: to be told to write `r:client/reply`.
- wrote: `r:client/reply`, after seeing `c:http/chunks` in another test.
- why it matters: ``error[type]: no type is called `reply`, so this arm can never match`` is true from the test's point of view, but `reply` is a pub type of the module the test sits in.

### N4: `path/extension` can answer `none` because of a slice inside it
- scope: domain-lib
- kind: stdlib-gap
- severity: nit
- where: static/static.kso:49; lib/path/path.kso:66
- wanted: `content_type (path/extension full)` with the arms it had.
- wrote: a `content_type none` arm.
- why it matters: `from_dot` returns `text/slice b (at + 1) (length b)`, which can never be past the end, but the checker now sees `none` in it. `path/basename` handles the same case inside the library with a `whole` helper; `extension` passes it to every caller.

## Bugs

### B1: a `none` stored in one record type's field is charged to another type's field of the same name
- reduction: `bugs/none_crosses_field_names/` (an entry and two one-file modules, 20 lines)
- should: check ok, and print `<none> HI`.
- does: `kanso check .` refuses `lib/lib.kso:10`, where `body_text` reads `request.body`, with ``this can be a none and `text/uppercase` has no arm for it ... (module ./cl)``. The `none` is in `cl`'s private `reading.body`. `kanso check lib` says ok. Renaming `cl`'s field to `data` makes the program check and run on both engines.
- in the port: `client/responses.kso` stored a maybe-`none` slice in its private `reading.body`, and the refusal landed on `http/request.kso:66`, `body_text`, which the client never calls. Wrapping the client's slices made it go away.

### B2: a private name read in a test helper passes check and fails at run time with a mangled name
- reduction: `bugs/private_name_in_test_helper/` (a module and its test, 15 lines)
- should: refuse at check, as `test_x = twice 2 == 4` is refused with ``unknown name `twice` ``.
- does: compiles; the test fails at run time with ``error[runtime]: unknown name `~twice` ``.
- in the port: server_test's `keeps?` helper called the private `keep_alive?`; five tests failed at run time with ``unknown name `~keep_alive?` ``.

## Comfort

On language and core-lib, the original agent would find about half its
summary answered. Runtime failures now name their file and line, so the two
bisections behind F13 and F17 would not happen; both rules are still enforced
only at run time. A private `step` no longer turns into std/list's type. A test
helper named `srv` no longer breaks fifteen functions. Spanning list literals
would let the route table be written as data. The native and interpreted
engines now agree on stdout after a kill. Against that, tests can no longer
reach private functions, which the journal praised. In this migration that
meant three functions made pub and one builder written only for tests, and one
private read slipped past check and failed at run time under a mangled name.
The `none` that `text/slice` now answers produced the two most confusing
moments of the migration: one refusal two functions from its cause, and one
in a different module from its cause, through a field that merely shared a
name. Index guards, list patterns, raw strings and the tie rule are where the
journal left them. The domain-lib half of the summary, concurrency and
sockets, is unchanged: accept outside a group still exits, a read still
blocks every fiber, there is no dial, and fibers still cannot share a store.
On tooling, the book's `kanso run` panels for single files still fail, and a
failing test still says only `returned false`.
