# Re-sit: ugit

Re-sat on 2026-10-24 against main at 78f72a58. Scratch programs for every
status below are in `resit/`.

## Result

- check.sh: **pass**. Unit tests delta 21, store 6, ugit 20, all passing;
  "ok: 15 fixtures agree on interp, dev and release".
- engines agree: **yes** on every fixture. **No** under `kanso play`: a local
  function named like a std/list type runs on the interpreter and builds the
  type natively (Bugs).
- lines changed by the migration: 154 (`diff -r -x resit -x bugs
  ports-orig/ugit ports/ugit`, lines starting `<` or `>`), in 19 files. No
  fixture or expected file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| 1 | `error[syntax]: `and` and `or` meet without parentheses` (ruling 2026-10-13, #1887) | yes | 1 |
| 2 | `error[name]: unknown name `text/concat` — two lists join with `list/concat`` (ruling 2026-10-13, #1906), then `error[unused]: unused import "std/text"` in view.kso, whose only `text/` call it was | yes | 6 in 5 files, one passed as a function value (`list/fold groups [] text/concat`) |
| 3 | `error[exhaustive]: this can be a none and `list/map` has no arm for it` at `ended = text/slice pieces 1 (length pieces - 1)` (ruling 2026-10-13, #1890) | partly. It points at the binding; the call it names is on the next line, across a `.` pipe. | 1 in delta, with a `sliced` helper |
| 4 | the same for `text/join` in store | partly; same as rg's | 6, with a second `sliced` helper in store/strings.kso |
| 5 | `error[exhaustive]: this can be an err and an index wants a value` at `found = pieces[1]`, where `pieces = text/split stored nul` | no. Nothing says the err comes from `text/split`'s `split _ ""` arm, raised because `nul` is computed and could be empty (N1). | 2 (store/objects.kso), now `text/split stored nul .> (pieces -> …)` |
| 6 | `error[exhaustive]: this can be a none and `os/make_dir` has no arm for it` at `os/make_dir (path/dirname full)` | partly. The none is a std/path artifact (N3). | 2, through a pub `store/dir_of` with a none arm; then `error[unused]: unused import "std/path"` |
| 7 | `error[name]: unknown name `parsed`` and eight more private names in the tests (ruling 2026-10-12, "a test file is an outside consumer") | no; it does not say the names are private | 9 functions made pub |
| 8 | `error[opacity]: `delta/added` is foreign — only `delta` builds a `added`` and `store/commit is foreign` (same ruling) | yes | 1 test rewritten through a test-side `placed` arm; 2 store tests moved onto the existing `new_commit`; 4 makers and `pub missing = absent` added to ugit/merge.kso for 7 decision-table tests (N5) |
| 9 | `error[exhaustive]: this can be a none and `store/ref_or_next` has no arm for it`, from `kanso test store` only. `kanso check store` and `kanso check main.kso` say ok. | no; the value is never none (N4) | 1 dead `ref_or_next none` arm |
| 10 | `error[runtime]: concat takes two lists / failed in store/visited at …/store/commits.kso:83` on 7 of 15 fixtures, all three engines: `text/slice c.parents 1 1` on a root commit now answers none (ruling #1890) | no. The checker passed it, and the message does not mention none. | 2 |

## Journal

| F | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a split's last element still owes a none arm | language | missing-feature | minor | still | `resit/f1.kso`: `closing [] pieces[length pieces]` gives "this can be a none and `closing` has no arm for it" |
| F2 | a header comment may not be followed by a blank line | language | aesthetics | nit | fixed | ruling 2026-10-13, "a file may open with a header comment and one blank line"; `resit/f2.kso` prints `x` |
| F3 | a private name silently loses to a std/list export | language | engine-bug | major | partly | in a module every engine now calls the local `step` (`resit/f3m/`: `3 3` on interp, dev and release; log 2026-10-12, "a name the author declared wins over a builtin and an import"). Under `kanso play` native still builds `list/step` while the interpreter calls the local function: `bugs/play_native_calls_imported_type.kso`. A local `type tally` still loses its constructor (rg `bugs/type_constructor_hidden_by_import.kso`) |
| F4 | `kanso run` refuses definitions beside statements | tooling | tooling | minor | partly | `run` still refuses; the book now prints `kanso play` above its samples (ch02, ch04) |
| F5 | parentheses that show precedence are refused | language | aesthetics | nit | fixed | ruling 2026-10-13, #1887: `k == 0 - d or (k != d and v[k - 1] < v[k + 1])` is now the required spelling (delta/myers.kso:51) |
| F6 | no pattern for an empty list | language | missing-feature | minor | still | `resit/f6.kso`: `fn joined [] s` gives "expected a parameter pattern" |
| F7 | `text/join` accepts a lazy `list/map` at check | language | diagnostic | major | fixed | `resit/f7.kso`: "error[type]: `text/join` takes a list here, and `list/map` answers a lazy sequence — make it a list with `list/to_list`" |
| F8 | private names are reserved in every file, tests included | language | refactoring-hazard | major | partly | tests no longer join the module (ruling 2026-10-12): the test-helper half is gone. A new top-level name still breaks a local in another file (rg `resit/f4/`) |
| F9 | disjoint arms still tie | language | confusing-semantics | minor | fixed | `resit/f9.kso`: `joined_to none …` beside `joined_to (clean before) acc (clean lines)` compiles and prints `1` |
| F10 | a one-line-too-long constant needs a throwaway binding | language | aesthetics | nit | still | rg `resit/f19m/`: "a single-expression constant is written inline" |
| F11 | a `../` import depends on how the entry was spelled | tooling | engine-bug | major | fixed | `bugs/import_above_invocation`: `cd app && kanso run . --interp` prints `shared`; `cd store && kanso check .` says ok |
| F12 | `os/exit` is an err, so the rescue eats exits | language | confusing-semantics | major | still | `resit/f12.kso`: `command 5 .? fatal` with `os/exit 3` inside prints `fatal: os/exit_status 3` and exits 128 on both engines |
| F13 | binding an index to a name hides the none check | language | confusing-semantics | minor | partly | errs bound to names are now checked (log 2026-10-12, #1884). Nones are not: `resit/f13b.kso` (direct) is refused, while `resit/f13.kso` (bound) compiles and stops with "split takes two strings / failed in text/split at std/text/text.kso:38" |
| F14 | std/os cannot remove files or directories | domain-lib | stdlib-gap | major | still | std/os exports no remove |
| F15 | the interpreter refuses `text/concat` on bytes | core-lib | engine-bug | major | moot | `text/concat` is gone (ruling #1906); `resit/f15.kso`: `list/concat (text/bytes "ab") [0]` prints `[97 98 0]` on both engines |
| F16 | the `entry` record's fields have no names | core-lib | confusing-semantics | nit | still | `resit/f16.kso`: `e.key` gives "no record type has a field `key`" |
| F17 | a keyed read of every field passes check, fails at run time | language | diagnostic | minor | still | `bugs/keyed_read_of_every_field_is_a_runtime_error.kso` fails at run time; `resit/f17m/`, the same in a module, gets `main.kso: ok` from `kanso check` |
| F18 | no do-notation for effects | language | missing-feature | major | still | no new construct; ugit/view.kso and the other chains of `with_*` helpers are unchanged and still needed |
| F19 | 80 columns with no way to continue a string | language | aesthetics | major | still | `resit/f19.kso`: a one-line `"""` block is refused ("a text block holds two or more lines"); no book chapter mentions text blocks |
| F20 | a foreign record can be read but not built or matched | language | confusing-semantics | minor | partly | matching a foreign pub record now works (rg's `fn complaint (bad_usage why)` in a test file). Building is refused by design (2026-08-20, "only a type's owner constructs one"). The qualified `(os/file_not_found p)` pattern is still refused (`resit/f20.kso`) |
| F21 | none cannot reach a generic parameter | language | missing-feature | major | still | `resit/f21.kso`: `compared` with `return … if a == none` guards gives "this can be a none and `compared` has no arm for it", twice |
| F22 | maps cannot drop a key | core-lib | stdlib-gap | minor | fixed | ruling 2026-10-13, #1914; `resit/f22.kso`: `delete idx "a"` prints `{ "b":"2" }` on both engines |
| F23 | no prefix test or substring search | core-lib | stdlib-gap | minor | partly | `resit/f23.kso`: `text/starts_with?` and `text/ends_with?` print `true true` (#1914); there is still no `contains?` or `index_of` |
| F24 | no `list/reverse` | core-lib | stdlib-gap | nit | fixed | `resit/f24.kso` prints `[3 2 1]` on both engines (#1914) |
| F25 | a multi-line list literal does not parse | language | aesthetics | nit | fixed | `resit/f25.kso`, the journal's own three-section list, prints on the interpreter (ruling #1882) |
| F26 | effects cannot be tested | tooling | tooling | minor | still | std/testing and std/expect have no scripted executor; ch05 still shows it only as a Rust test |
| F27 | `x == true` is flagged when x can be none | language | diagnostic | nit | still | rg `resit/f6.kso`: "comparing to `true` asks a question the value already answers" |
| F28 | interleaved put and read on a map is quadratic natively | core-lib | performance | major | fixed | `resit/f28/` (the bug file with one comment line rewrapped): 20,000 put-then-read steps in 0.50 s natively including the build, 0.03 s on the interpreter. A read bound to a name before the put is still a cliff: toml `bugs/named-read-before-put` |
| F29 | a function name reserves the word, commands became `*_cmd` | language | refactoring-hazard | minor | partly | an imported pub type no longer captures a local function (`resit/f29/`: `1 2 3 o` on interp and dev). A function name still blocks a parameter of that name (`resit/f29b.kso`: "`branch` is already a declaration") |
| F30 | sha256 on the interpreter hashes about 30 KB a second | domain-lib | performance | minor | partly | `resit/f30/`: 29 KB in 0.47–0.55 s wall on the interpreter (was 1.07 s) |

Counts: fixed 9, partly 8, still 12, ruled-no 0, moot 1, worse 0.

## New friction

### N1: `text/split` on a computed separator is a raiser, and a NUL cannot be written
- scope: core-lib
- kind: confusing-semantics
- severity: minor
- where: store/objects.kso (`unframed`, `kind_in`)
- wanted: `pieces = text/split stored nul` / `found = pieces[1]`
- wrote: `text/split stored nul .> (pieces -> unframed_pieces oid kind pieces)`, and a new function to receive the list
- why it matters: `text/split` has an arm `split _ ""` that answers an err, so a split on a separator the checker cannot see is a raised err. The message says only "this can be an err and an index wants a value". `nul` is computed (`text/from_code 0`) because kanso has no `\0` escape (rg F16), so the missing escape now also costs a lambda at every split on it.

### N2: a slice past the end fails at run time inside a builtin
- scope: core-lib
- kind: confusing-semantics
- severity: major
- where: store/commits.kso:81-82 (`visited`)
- wanted: `firsts = text/slice c.parents 1 1`, the first parent or nothing, which is how the slice behaved before #1890
- wrote: `firsts = sliced (text/slice c.parents 1 1)`, and the same for `others`
- why it matters: the checker passed the program, and 7 of 15 fixtures failed on every engine with "concat takes two lists". That message does not say a none arrived. The same ruling made seven in-range slices need a helper to compile. toml's N2 is the same finding.

### N3: std/path's `dirname` reads as possibly none
- scope: domain-lib
- kind: stdlib-gap
- severity: minor
- where: store/refs.kso (`dir_of`), ugit/checkout.kso:118; `bugs/path_dirname_reads_as_none.kso`
- wanted: `os/make_dir (path/dirname full)`
- wrote: `store/dir_of` with a `known_dir none` arm
- why it matters: std/path's `basename` wraps its slice in a none arm, and `dirname` and `extension` do not, so every caller pays for the library's missing arm.

### N4: `kanso test` refuses a module that `kanso check` accepts
- scope: tooling
- kind: diagnostic
- severity: major
- where: store/names.kso:48; `bugs/test_refuses_what_check_accepts/`
- wanted: the tests to compile when the module checks
- wrote: `fn ref_or_next none rest name`, an arm that can never run
- why it matters: with any test file at all, `kanso test store` reports "this can be a none and `store/ref_or_next` has no arm for it" on `at.value`, a field that always holds a record. `kanso check store` and `kanso check main.kso` say ok. The refusal prints no source line. toml found the opposite split (`check <dir>` ok, `check <entry>` refusing), so a module can get three verdicts from three verbs.

### N5: keeping a decision-table test means exporting its constructors
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: ugit/merge.kso (`ours_of`, `theirs_of`, `text_merge`, `conflict_of`, `missing`), ugit/args.kso, ugit/log.kso, ugit/view.kso, ugit/commit.kso
- wanted: `decided "p" "base" "mine" "base" == ours_stand "p"`, as before
- wrote: `== ours_of "p"`, with four one-line pub makers and a pub constant written only for the test, plus nine functions made pub
- why it matters: the merge decision is the most carefully tested logic in the port, and its tests compare whole records. Under the 2026-10-12 ruling a test may read the module's pub surface and nothing else. The module's surface grew by fourteen names so that its tests could keep asking the same questions.

## Bugs

- `bugs/play_native_calls_imported_type.kso` (new): under `kanso play`, native calls std/list's `step` constructor where the file's own `step` is meant (`3 list/step 1 2`). The interpreter prints `3 3`. The engines disagree.
- `bugs/test_refuses_what_check_accepts/` (new): `kanso test store` refuses a module that `kanso check store` accepts.
- `bugs/path_dirname_reads_as_none.kso` (new): `path/dirname`'s answer is read as possibly none.
- `bugs/local_fn_loses_to_imported_type.kso`: fixed in modules, still wrong in native `kanso play` (above).
- `bugs/import_above_invocation`: fixed.
- `bugs/join_accepts_lazy_map_at_check.kso`: fixed, refused at check.
- `bugs/concat_bytes_interp_refuses.kso`: moot; `text/concat` is gone and `list/concat` agrees on both engines.
- `bugs/keyed_read_of_every_field_is_a_runtime_error.kso`: still a run-time refusal that check passes.
- `bugs/map_put_then_read_resorts.kso`: fixed. As written it no longer compiles: its comment line 8 is 84 characters and is refused by the 80-column rule. Rewrapped in `resit/f28/`.

## Comfort

Language and core library first. The original agent described two
experiences: a pleasant pure core and a slow imperative shell. The core is now
smoother. The Myers condition is written with the parentheses it wanted. Lazy
maps handed to `join` are caught at check. Disjoint arms no longer tie. A
multi-line list literal parses. `delete`, `list/reverse`, `starts_with?` and
`ends_with?` replace three hand-written helpers. The map cliff that made the
release binary ten times slower than the interpreter is gone. The shell is
where the journal's main complaints were, and they stand. There is still no
way to bind several effect results in one scope (F18). A none from a map read
still cannot reach an ordinary parameter (F21). A function name still reserves
the word in every file (F29). `os/exit` is still swallowed by the rescue
(F12). The migration added new costs. The slice ruling put helpers on eight
slices and broke half the fixtures at run time with no compile warning.
Testing the merge table through the pub surface meant exporting fourteen
names. The `test` and `check` verbs disagree about the same module. On the
domain side std/os still cannot remove a file, and sha256 on the interpreter
is about twice as fast. An agent starting today would hit fewer engine bugs
and the same shell friction, plus new ceremony around slices and tests. The
migration took 154 changed lines across 19 files.
