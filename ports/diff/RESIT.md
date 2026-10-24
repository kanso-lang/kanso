# Re-sit: diff

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the re-test are
in `resit/`; each is run with `kanso play <file>` (native) and
`kanso play <file> --interp` unless the row says otherwise.

## Result

- check.sh: **pass**. 58 unit tests in six modules, 54 fixtures, three engines
  each.
- Engines agree: yes, on every fixture.
- Lines changed by the migration: 65 changed lines (`diff -r ports-orig/diff
  ports/diff -x resit`, counting `<` and `>` lines) in 11 `.kso` files. No
  `expected` file changed.
- Five fixtures also needed an empty directory, `in/old/mode`, put back. The
  copy of the port on this machine had lost it (empty directories do not
  survive the copy), and the expected output ("File old/mode is a directory
  while file new/mode is a regular file") depends on it. This is a staging
  repair and has nothing to do with the language.

## Migration

| change | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| `text/concat` is gone; lists join with `list/concat` (gavel 2026-10-13, `list/concat` joins two lists) | ``error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes | 19 calls in 8 files, plus `import "std/list"` added to 3 files and `std/text` dropped from 2 |
| `text/slice` past the end answers `none` (gavel 2026-10-13, #1890) | ``error[exhaustive]: this can be a none and `short` has no arm for it — resolve it here, or give `short` a `none` arm`` | partly. It names the callee, never `text/slice`. The first one reported named `list/fold` at `lines.kso:81`, inside `list/map`, while the `none` came from a slice at `lines.kso:39` (see N1) | 11: ten slices wrapped in the port's own `given ... []` / `given ... ""` helper, and `cli.kso` rewritten as `list/to_list (list/drop argv 1)` |
| a wrapped slice made a line too long | `error[formatting]: a line holds at most 80 characters — this one has 91` | yes | 1 (split into a binding) |
| `std/list` added after `std/os` | `error[formatting]: imports appear in alphabetical order` | yes | 1 |
| a test file is an outside consumer of its module (gavel 2026-10-12) | ``error[opacity]: `patch/found` is foreign — only `patch` builds a `found`; ask it for one through a pub function`` | yes, though the fix I used reads the fields instead of asking for a pub builder (see N4) | 2 tests |

Several slices that can also answer `none` (`myers.kso:21-22`,
`changes.kso:44`, `apply.kso:50,57,156`, `parse.kso:154`) were not flagged,
because their result goes to `push` or `length`, which the check does not see
into (N2). Their ranges are in bounds, so the program is correct.

## Journal

| entry | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a local name loses to a std/list export of the same name | language | engine-bug | major | partly | A local function named like an imported type now wins (log 2026-10-12, "a name the author declared wins over a builtin and an import"): `resit/f1_skipped.kso` prints `6` on both engines, and the original repro passes `kanso test` (`1 passed, 0 failed`). A local marker type named like an imported function still loses: `resit/f1_marker_value.kso` prints `<fn> something else` on both engines; without the import it prints `f1_marker_noimport/drop dropped`. |
| F2 | a type named `fold` is refused as a call to list/fold | language | diagnostic | minor | still | `resit/f2_fold_type.kso`: ``error[arity]: no 2-argument arm of `fold` (arms take 3)``, on both engines, with no mention of the local type. |
| F3 | there is no pure "this index is there" read | language | missing-feature | major | partly | A literal or variable index after a length guard is now accepted: `resit/f3_index.kso` (`return ... if length files != 2` then `lower files[1]`) prints `a`, and without the guard it is refused. Reading a pair built by `list/zip` is still refused: `resit/f3_zip.kso` gives ``this can be a none and `step` has no arm for it`` at `pair[1]` and `pair[2]`. |
| F4 | `kanso check <module>` says ok to code the entry then refuses | tooling | tooling | minor | still | On the original `lines/` and `report/`: `kanso check lines` prints `lines: ok` (exit 0), and `kanso check report` refuses `lines/lines.kso:81:36` with error[exhaustive]. The error is in `lines.kso`'s own code. |
| F5 | a parameter named `done` is a pattern, not a binding | language | confusing-semantics | major | still | `resit/field_named_done.kso`: nothing at check; both engines stop with ``no overload of `count` matches these arguments``. The message now adds `failed in the entry at field_named_done.kso:21`. |
| F6 | native code crashes where the interpreter is right | language | engine-bug | blocker | fixed | Log 2026-10-10, "two native crashes the ports found in the arena", fixed this reduction and pinned it as `a_view_left_in_a_carry_buffer`. `resit/f6.kso` (the bug file unchanged) prints the diff and `status 1` natively, the same as `--interp`. |
| F7 | names collide across the files of a module | language | refactoring-hazard | major | partly | The test half is fixed by the gavel of 2026-10-12, "a test file is an outside consumer of its module": in `resit/f7/`, a test's `fn run` beside a module whose file binds `run = ...` checks and passes. The rest stands. A top-level `fn more` in `b.kso` makes `more = ...` in `a.kso` fail with ``error[name]: `more` is already a declaration; rename the binding``. Two private `fn given` in two files give ``error[dispatch]: overlapping overloads of `given` are illegal``, pointing at `b.kso` only and never naming `a.kso`. |
| F8 | reading a map and then growing it copies it, every time | core-lib | performance | major | partly | `list/tally` is fixed: 80,000 distinct ints tally in 0.10 s natively (`resit/f8/t.kso`; kanso#1822). A user's read-then-put loop is still quadratic: `resit/f8/q*.kso`, the bug file unchanged, takes 0.41 s at 20,000 keys, 1.83 s at 40,000 and 7.5 s at 80,000. The same loop without the read takes 0.06 s at 80,000. |
| F9 | no vector you can write at an index | core-lib | missing-feature | major | still | Nothing in std/list or the ambient names writes at an index. `put [0 0 0] 2 1` passes check and stops at run time with `put takes a map, a key, and a value` (`resit/f9.kso`). |
| F10 | no record update, so options parse into a map first | language | missing-feature | minor | fixed | `merge` (gavel 2026-10-13, #1886). `resit/f10.kso` builds an options record and updates one field per flag with `merge opts { "recursive":true }`, and prints `true 3 true` on both engines. A misspelt key is refused: ``error[name]: no record type has a field `breif` ``. |
| F11 | I wrote `given` (a default for none) in five modules | core-lib | stdlib-gap | minor | still | No such function in lib/ (`grep` for `or_else`, `given`, `default`, `otherwise`). std/regexp now carries its own `or_empty`. The slice ruling made the helper more common: this migration added ten more calls to it. |
| F12 | `text/slice` partly past the end answers nothing at all | core-lib | confusing-semantics | major | fixed | Gavel 2026-10-13, "`text/slice` past the end answers `none`" (#1890). `text/slice "abc" 2 9` prints `<none>` on both engines, and passing it to a function with no `none` arm is refused at check. |
| F13 | eighty columns with no way to wrap a string or a list | language | aesthetics | minor | fixed | Gavel 2026-10-13, "a bracketed list or map may span lines". `resit/f13.kso` spans a list and uses a `"""` text block for the usage text; `resit/f13t/` compares a test against a spanning list literal. All pass. |
| F14 | positional construction with many fields | language | refactoring-hazard | minor | partly | Updating one field is now `merge` (F10). Construction by field name stays declined; the `merge` gavel restates the 2026-07-29 ruling on named arguments. Adding a field still shifts every positional constructor call. |
| F15 | what crosses a module boundary is uneven | language | diagnostic | minor | still | `resit/f15.kso`: `fn loaded _ (os/file_not_found p)` gives ``error[opacity]: `os/file_not_found` is foreign — its structure does not cross an import``; the bare `(file_not_found p)` in `resit/f15b.kso` destructures and prints `missing nope.txt`. |
| F16 | std/os cannot tell a file's time, or remove a file | domain-lib | stdlib-gap | major | partly | `os/modified` exists (gavel 2026-10-13, library gaps) and prints `1000000000000` for a file stamped at epoch 1e9 (`resit/f16.kso`); milliseconds, not GNU's nanoseconds. There is still no remove. |
| F17 | two dispatch corners | language | confusing-semantics | minor | still | `resit/f17.kso`: ``these `verb` arms tie`` for `verb 0 _` / `verb _ 0` / `verb _ _`, and ``this can be a none and `spacing` has no arm for it`` for `fn spacing _` given `m["y"]`. |
| F18 | declaration placement rules | language | aesthetics | nit | partly | A header comment plus one blank line is accepted (gavel 2026-10-13). Types must still come first: ``canonical order places type declarations before functions; move `pair` up`` (`resit/f18/m.kso`). |
| F19 | the same program resolves imports or not, by how its path is spelled | tooling | tooling | minor | fixed | `resit/f19/progs/p` (an entry importing `./q`, whose library imports `../../../lib`): `kanso check progs/p`, `cd progs/p && kanso check .` and `kanso check ../p` all say ok, and `kanso run .` prints `3`. |
| F20 | bisecting under canonical form | tooling | tooling | minor | still | `resit/f20.kso` under `kanso play`: ``error[unused]: unused binding `nested` ``. There is no relaxed mode. |
| F21 | the book's single-file programs do not run as written | tooling | tooling | nit | still | ch05 still shows `kanso run count.kso` printing `flavors.txt has 3 lines`. Today `kanso run` on `docs/book/samples/ch05/count.kso` says ``error: `count.kso` is a library — nothing to run``, and `kanso play` refuses it with ``error[syntax]: `pub play` is a library's export``. Neither verb runs the sample as the book shows it. |

Counts: fixed 5, partly 7, still 9, ruled-no 0, moot 0, worse 0.

## New friction

### N1: a `none` from `text/slice` is reported far from the slice, under a library's name
- scope: language
- kind: diagnostic
- severity: minor
- where: lines/lines.kso:39 (the slice) and :81 (where it was reported)
- wanted: an error at `doc ends (if ends (text/slice pieces 1 (length pieces - 1)) pieces)` saying the slice can answer `none`.
- wrote: read the message, which says ``this can be a none and `list/fold` has no arm for it (in lines.kso) (module ./report)`` at `list/map d.lines ...` in `keys_of`, then traced `d.lines` back through the `doc` constructor to the slice.
- why it matters: the port never calls `list/fold` there; it is inside `list/map`. The message names a library internal and a line two functions away from the cause, and it appears only when another module is checked (F4).

### N2: the `none` check does not see into `push`, so a `none` slice reaches run time
- scope: language
- kind: confusing-semantics
- severity: minor
- where: resit/n2.kso; in the port, myers.kso:21-22, changes.kso:44, apply.kso:156, parse.kso:154
- wanted: `push (text/slice xs 3 5) 2` refused at check, the way `f (text/slice xs 3 5)` is.
- wrote: nothing; the port's ranges are in bounds. The probe passes `kanso check` (`n2.kso: ok`) and stops on both engines with `push takes a list and a value`.
- why it matters: the slice ruling promised the checker would see `none` in `text/slice`'s answer. It does for user functions and library functions, but not for ambient builtins such as `push`. `put` on a list is the same: it passes check and fails at run time (F9's probe).

### N3: "exactly one space here" does not show the spelling it wants
- scope: language
- kind: diagnostic
- severity: nit
- where: resit/f10.kso
- wanted: `merge opts {"recursive": true}`, the spelling in the `merge` gavel.
- wrote: `merge opts { "recursive":true }`, after finding a sample in the book.
- why it matters: the message points at the `{` and at the `}` and says `canonical form requires exactly one space here` four times, without showing the canonical form. The space after the colon, which is the one that has to go, is not mentioned.

### N4: a test cannot build the record it expects, so whole-value comparisons become field reads
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: patch/patch_test.kso:49-58
- wanted: `result.outcomes == [(found 3 2)]`, as before.
- wrote: `o = result.outcomes[1]`, `one = length result.outcomes == 1`, then `one and o.at == 3 and o.offset == 2`.
- why it matters: since a test file is an outside consumer, and only a type's owner constructs it, a test cannot write the expected value of a pub record. The alternative is a pub builder that exists only for tests. Structural equality on whole outputs was one of the journal's "what worked well" items, and it no longer reaches records.

## Bugs

None. Every engine agreed on every fixture and every probe. The journal's one
native crash (F6) is fixed.

## Comfort

On language and core-lib, the original agent would find the port's top five
in better shape than its summary describes, though not by much. The native
crash that headed its list is gone. Record update, spanning list literals and
the slice that used to lose data are each settled by a ruling, and each of
those turned a workaround into one plain line. The guard-then-index case from
F3 now compiles. The array-shaped half of diff, which the summary called the
part that fought the language at every step, is unchanged. There is still no
vector to write at an index. A map read before a `put` is still quadratic (7.5
s at 80,000 keys against 0.06 s without the read). `_` still does not catch
`none`, and every module still needs its own `given` helper. The slice ruling
added ten more calls to that helper in this migration, all on ranges the code
had already bounded. The naming problems the summary said cost the most
surprise have narrowed but not closed. A test can no longer rebind its module,
and a local function now beats an imported type. A local marker named `drop`
still loses to `list/drop`, `fold` still means `list/fold`, `done` is still a
pattern, and a top-level function in one file still breaks a binding in
another. The migration itself was short: two rulings, about thirty sites, and
messages that named the fix for everything except the slices. On tooling,
`kanso check <module>` still says ok to code that is refused from the root,
and neither `run` nor `play` runs the book's ch05 `pub play` sample.
