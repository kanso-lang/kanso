# Re-sit: regex

## Result

check.sh: pass. `re` 82 of 82 and `grep` 8 of 8 unit tests pass, and all 43
fixtures agree on interp, dev and release (`resit/check.log`).

Engines agree: yes.

Lines changed by the migration: 119 (`diff -r` against ports-orig/regex,
counting `<` and `>` lines, `resit/` and `bugs/` left out).

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first`` (#1887) | yes | 3 |
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906) | yes. bracket.kso also needed `import "std/list"` | 22 |
| ``error[exhaustive]: this can be a none and `list/map` has no arm for it`` (#1890) | partly. Each refused slice was one that cannot miss; one `sliced` helper in each module | 4 |
| ``error[name]: `run` is already a declaration; rename the binding``, then the same for `part` | yes, but they came from names elsewhere in the module (F11) | 2 renames of locals the migration added |
| the same `this can be a none and `text/join` has no arm` after the helper was added | no. The helper's binder was named `empty`, which is a marker in module `re` (the AST's empty node), so `fn sliced none empty` was a literal pattern and never matched `[]`. Nothing said so (mal's F6) | 1 |
| none at check; at run time 42 of 43 fixtures failed with ``error[runtime]: concat takes two lists`` / `failed in grep/run at grep/cli.kso:30` | partly. The line was right. The cause was `back = text/slice argv (split_at + 1) (length argv)`, bound to a name, which the none check does not follow, so a slice that used to clamp to `[]` now answers none (N1) | 1 |
| ``error[name]: unknown name `max_cp` `` and five more in the test files (#1883) | no; "unknown name", not "a test sees only pub names" | 6 made pub: `max_cp`, `split_lines`, `known?`, `option?`, `label`, `right` |
| ``error[opacity]: `re/span` is foreign — only `re` builds a `span`; ask it for one through a pub function`` | yes | 19 constructions in classes_test.kso, through a new `pub fn span_from` |
| `error[formatting]: a line holds at most 80 characters` after `span` became `span_from` | yes | 2 (one became a spanning list) |

No expected-output file changed.

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a typeset must fit on one line, so a large one has to be split | aesthetics | minor | language | still | `resit/f1_typeset.kso`, the journal's wrapped `node`: `error[syntax]: a subtype or typeset declaration is one line; fields belong to records`. ch03 still says "one line" |
| F2 | `list/sort` cannot sort records and takes no key | stdlib-gap | major | core-lib | partly | `resit/f2_sort_by.kso`: `list/sort_by spans (s -> s.first)` prints `[1 4 7]` on both engines (#1914). `list/sort` on records (`resit/f2_sort.kso`) still checks clean and fails at run time, now with ``comparison requires two values of one comparable type`` / `failed in list/pick at std/list/list.kso:476`, a line in the library and not the caller's (N2) |
| F3 | an index read right after a bounds check still has to handle none | confusing-semantics | major | language | partly | the journal's wanted `inside?` compiles and prints `true` (`resit/f3_index.kso`, and inlined in `f3_inline.kso`). It compiles because a field read on a possible none is not checked at all: `resit/f3_noguard.kso`, `spans[i].first` with no guard, checks clean and stops at run time with ``. reads a field of a record, not <none>``. Handing `items[i]` to a function after the same guard is still refused (mal's `resit/f7_index.kso`) |
| F4 | `xs[i]!` in pure code passes the checker and fails at run time | diagnostic | major | language | partly | `bugs/strict_index_field_read.kso`: still no refusal at check. The failure now names the file and line: ``. reads a field of a record, not <io>`` / `failed in first_of at bugs/strict_index_field_read.kso:19` (#1975) |
| F5 | only pipes and chain words may continue a line | aesthetics | minor | language | partly | a list literal may span lines (#1882): classes_test.kso now writes `negate gapped == [` with one span per line. Operator chains, lambdas and strings still cannot break, and #1882 leaves operator chains out |
| F6 | imports belong to a file, not to the module | confusing-semantics | nit | language | ruled-no | the 2026-10-12 gavel on #1894: a file names what its module declares and what its own imports bring |
| F7 | a scratch file with a function and a statement will not `run` | tooling | minor | tooling | still | `run` refuses it and points at `play`. `kanso check` still refuses play files: on lox's `bugs/dispatch_tie_unreported_engines_disagree.kso` it says `error[syntax]: a file with declarations is a library, and a library has no statements to run`, and on `resit/f2_sort.kso` here it gives `exactly one blank line separates top-level declarations` for a file `play` runs |
| F8 | a list literal cannot continue onto a second line | missing-feature | minor | language | fixed | #1882; classes_test.kso uses the spanning form in this migration, and lox's `resit/f6_map.kso` runs a spanning map |
| F9 | where a `none` arm goes on the ladder is not what the message says | diagnostic | minor | language | still | `resit/f9_ladder.kso` and `resit/f9_follow.kso`: the same sentence, ``overloads of `follow` appear most-specific first: literal, then concrete type, then generic``, with no word on where none sits |
| F10 | no list patterns in a parameter | missing-feature | minor | language | still | `resit/f10_listpat.kso`: `error[syntax]: expected a parameter pattern` at `[one]` |
| F11 | one namespace means every name in the module, its tests and its imports collides with every local | refactoring-hazard | major | language | partly | imports no longer leak most bare names, and test files no longer join the module (#1883, #1894). A module's own type named like an imported function still collides (mal's B2). Cross-file collisions remain: the migration's own locals `run` and `part` were refused because of `fn run` in pike.kso and a `part` elsewhere |
| F12 | a pub record is opaque outside its module, fields and all | confusing-semantics | major | language | partly | `resit/f12`: a module that imports `re` can now read `e.code`, `e.text`, `h.start` on values `re` built, and prints them. Destructuring `(re/syntax_error at code text)` is still `error[opacity]: ... its structure does not cross an import`. A module that does not import the owner cannot read the field at all (B1) |
| F13 | an argument cannot be `none` on purpose | confusing-semantics | minor | language | still | `resit/f13_none_arg.kso`: `scan 2 none` is refused with ``this can be a none and `scan` has no arm for it`` though `outcome` has a none arm |
| F14 | every function that receives a parse result needs an err arm | aesthetics | minor | language | still | the 2026-09-15 ruling "a bare err halts where it lands" requires the arm, and #1884 closed the name loophole the entry mentions (mal's `resit/f3_named.kso`) |
| F15 | a release build drops `musttail` from a tail call with seven parameters | engine-bug | major | language | fixed | `bugs/release_tail_call` rebuilt with `--release` (`resit/tc7`): every call to `ping_7`/`pong_7` is `musttail call tailcc`, and it prints `1000000` |
| F16 | no way to build a record from another with one field changed | missing-feature | major | language | fixed | `merge` (#1886), mal's `resit/f10_merge.kso` |
| F17 | a helper type cannot sit beside the function it serves | aesthetics | nit | language | still | `resit/f17_order.kso`: ``canonical order places type declarations before functions; move `sweep` up`` |
| F18 | comparisons do not chain, so an exclusive-or needs a name | aesthetics | nit | language | still | lox's `resit/f28_chain.kso`: `unexpected trailing tokens` |
| F19 | every `{` in a pattern string is an interpolation | diagnostic | minor | language | still | `resit/f19.kso`: `"{"` gives `unterminated interpolation`, `"a{2,5}"` gives `kanso has no commas`, `"\\x{10FFFF}"` gives `identifiers are snake_case` |
| F20 | `text/slice` past the end answers nothing rather than what is there | confusing-semantics | minor | core-lib | ruled-no | #1890 ruled the other way: a range past the end answers `none`, so the clamp in `chunk` stays and now needs a none arm |
| F21 | std/expect is in lib/ but not in the compiler | tooling | minor | tooling | still | `resit/f21_expect.kso`: ``error: `std/expect` is not in the shipped library``, while `lib/expect/expect.kso` is on disk |
| F22 | a function in a test file named like a type in the module is silently a constructor | engine-bug | major | language | fixed | `kanso test m` in `bugs/fn_named_like_type`: `test_helper_runs ... ok` (#1883) |
| F23 | an arm for a parameter that can never hold that type is accepted | diagnostic | minor | language | still | `resit/f23`: `fn grep _ (file_not_found p) _` checks clean; the run stops with `split takes two strings` / `failed in text/split at std/text/text.kso:38`, naming std's line and not `g.kso` (N2) |
| F24 | no escape for a code point in a string literal | missing-feature | nit | language | still | `resit/f24_escape.kso`: ``error[syntax]: unknown escape `\u` `` |

Counts: fixed 4, partly 6, still 12, ruled-no 2, moot 0, worse 0.

## New friction

### N1: a slice bound to a name can now answer none, and nothing checks it
- kind: confusing-semantics
- severity: major
- scope: language
- where: grep/cli.kso `run`, `back = text/slice argv (split_at + 1) (length argv)`
- wanted: the checker to find every slice #1890 changed, as it found the
  four slices passed straight to a call
- wrote: `back = sliced (text/slice argv (split_at + 1) (length argv)) []`,
  after 42 of 43 fixtures failed at run time with `concat takes two lists`
- why it matters: #1890 changed what `text/slice` answers past the end,
  from `[]` to none. The none check follows a slice into a call but not
  into a name, so the migration that the checker guides is incomplete, and
  the rest shows up as run-time failures far from the slice. The chip8 port
  had two more of these, found only by its unit tests. #1884 closed the same
  hole for errs.

### N2: a failure inside a library function names the library's line
- kind: diagnostic
- severity: minor
- scope: language
- where: `resit/f2_sort.kso`, `resit/f23`
- wanted: the caller's line, as the #1975 gavel describes for an insisting
  form and a call that matches no arm
- wrote: nothing; the reports read `failed in list/pick at
  std/list/list.kso:476` and `failed in text/split at std/text/text.kso:38`
- why it matters: the program's own line is what a user can act on. Under
  `kanso test` the same std line number is printed under the test file's
  path (chip8 B1), which points at an unrelated line.

### N3: a marker named `empty` turned the migration's helper into a literal pattern
- kind: confusing-semantics
- severity: minor
- scope: language
- where: re/parse.kso `sliced`
- wanted: `fn sliced none empty` with `empty` as a binder, as it worked in
  the mal and lox ports
- wrote: `fn sliced none fallback`
- why it matters: `re` declares `pub type empty` for the AST, so the binder
  was the marker, the arm never matched `[]`, and the checker went on
  reporting the none the helper was written to remove. It is mal's F6, met
  again by the person doing the re-sit, and it cost a bisection.

### N4: tests that build the module's records need a public maker
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: re/classes_test.kso (19 `span` constructions), grep/cli_test.kso
- wanted: the tests as written
- wrote: `pub fn span_from first last` in re/classes.kso, and six names
  made pub
- why it matters: #1883 is deliberate. A test that compares a function's
  answer with a record built by hand now needs the module to export a maker
  that exists only for the test.

## Bugs

- **B1** `bugs/field_read_without_import/`: a module that never imports the
  owner reads `h.start` on a parameter. Chapter 03 says "nothing has to be
  imported to read a field", and the #1894 gavel says a value carries its
  owner's arms to a library checked alone. Today it is ``error[name]: no
  record type has a field `start` `` on every engine. Should print `2..5`.

The three bugs in the original `bugs/`: `fn_named_like_type` is fixed (F22),
`release_tail_call` is fixed (F15), and `strict_index_field_read.kso` still
passes the checker but now names its line (F4).

## Comfort

On the language and core library, a new start would go better, but the
journal's first complaint, the module-wide namespace, is only half answered.
Imports no longer rename things in other files, and test helpers no longer
turn into constructors (F22), but two locals added during this migration
were still refused for names declared in other files (F11). Field reads now
cross a module boundary from an importer (F12), which would remove most of
the dozen accessor functions in `re/api.kso`, though destructuring still
does not cross. `merge` (F16), spanning lists (F8), `list/sort_by` (F2) and
a release build that keeps its tail calls (F15) each remove a workaround the
port carries. The none story is mixed. The wanted bounds-checked index loop
compiles, but only because field reads on a none are not checked at all
(F3), and #1890's change to `text/slice` was the costliest part of the
migration: the checker found four of the five slices, and the fifth broke
42 fixtures at run time (N1). The small papercuts in the journal (braces in
strings, chained comparisons, typesets on one line, no list patterns, no
`\u` escape) are all still there.
