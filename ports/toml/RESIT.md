# Re-sit: toml

Re-sat on 2026-10-24 against main at 78f72a58. Scratch programs for every
status below are in `resit/`.

## Result

- check.sh: **pass**. 56 unit tests; 154 cases, 0 failures, engines interp,
  dev and release.
- engines agree: **yes** on every case. No new native crash in this port. One
  new native performance cliff (Bugs).
- lines changed by the migration: 215 (`diff -r -x resit -x bugs
  ports-orig/toml ports/toml`, lines starting `<` or `>`), in 13 files. No
  expected-output file changed (`diff -rq` over `tests/` is clean).

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| 1 | `error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first` (ruling 2026-10-13, #1887) | yes | 1 |
| 2 | `error[exhaustive]: this can be an err and `after_date` has no arm for it — dispatch on it here, or give `after_date` an `(err _)` arm (in datetime.kso)` (log 2026-10-12, "a name bound to a raising call carries its err into the check", #1884) | mostly. Each message names the function and position. It does not say that adding the arm makes that function a raiser, so its own callers fail next. Nor does it say that two errs reaching one function need three arms (N1). | 48 reported on the first run; 37 pass-through `e@(err _)` arms added across 8 files, one `array_at` gate, one `.>` in `header_names` |
| 3 | `error[dispatch]: these `resume` arms tie: each is the more specific one somewhere` after giving `resume` an err arm at each of its two failing positions | yes, once read twice: "write the arm that is most specific in every position" means a third arm with `(err _)` in both places | 2 functions (`resume`, `block_run`) |
| 4 | `error[exhaustive]: this can be a none and `text/utf8` has no arm for it — resolve it here, or give `text/utf8` a `none` arm` (ruling 2026-10-13, #1890) | partly. The suggested arm on a std function should be illegal under the ownership rule. I added `some_slice` to scan.kso, the shape lib/path.kso uses. | 13 |
| 5 | `error[runtime]: length takes a list, string, or map, not <none> / failed in toml/digits? at …/datetime.kso:53`, from fixture `invalid/time-no-seconds.toml` on all three engines (same ruling) | no. The checker passed it. Only the fixture found it, and the message named the line. | 1 |
| 6 | `error[exhaustive]: this can be a none and `toml/path_key` has no arm for it … (in table.kso) (module cli)` / `--> toml/table.kso:229:60`, with no source line, from `kanso check main.kso` while `kanso check toml` said ok | no. It names a line that reads a record field, not the two slices that put the none there, and it names the wrong module | 2 |
| 7 | `error[exhaustive]: this can be an err and `toml/from_tagged` has no arm for it — … or give `toml/from_tagged` an `(err _)` arm (in cli.kso)` | partly. The suggested arm would be written in cli for a function cli does not own. I wrote `json/decode source .> toml/from_tagged .> toml/encode` instead. | 1 |
| 8 | `error[name]: unknown name `utf8_error`` and five more names in toml_test.kso (ruling 2026-10-12, "a test file is an outside consumer") | no; it does not say the name exists and is private | 5 functions made pub, one `pub empty_index` constant added |
| 9 | `error[opacity]: `toml/local_date` is foreign — only `toml` builds a `local_date`; ask it for one through a pub function` (same ruling) | yes | 12 sites; 3 pub makers and a `utc_offset` constant added to datetime.kso |
| 10 | `error[exhaustive]: this can be an err and `decodes?` has no arm for it` in the round-trip test | yes | 1 test-helper arm |

## Journal

| F | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | no byte or character literal | language | missing-feature | major | still | `resit/f1b.kso`: `fn kind '"'` gives "unexpected character `'`". Hex literals now exist (ruling 2026-10-13, #1889; `resit/f1.kso`: `fn kind 0x22` prints `quote`), and that ruling says "a byte literal … was not ruled" |
| F2 | a catch-all does not cover none | language | confusing-semantics | major | still | `resit/f2.kso`: `blank? cs[4]` with `fn blank? _` gives "this can be a none and `blank?` has no arm for it" |
| F3 | a binding launders an err or none past the check | language | refactoring-hazard | major | fixed | log 2026-10-12, "a name bound to a raising call carries its err into the check" (#1884). The migration is the evidence: 48 sites now refused. A function of mine that answers none is caught too (`resit/f3n.kso`). The cost is N1 |
| F4 | an index guard must cover both ends, and the message does not say so | language | diagnostic | minor | still | `resit/f4.kso` and `resit/f4b.kso`: `return 0 if length xs < i` or `if i > length xs`, then `need_int xs[i]`, both give "this can be a none"; `resit/f4c.kso` with `i < 1 or` prints `2` |
| F5 | a local collides with every module declaration and ambient builtin | language | refactoring-hazard | major | still | `resit/f5.kso`: parameter `keys` gives "`keys` is already a declaration; rename the binding"; `resit/f5b.kso`: local `done` the same |
| F6 | no continuation for `return … if …`; a long constant cannot be a block | language | aesthetics | minor | ruled-no | the 2026-10-13 gavel "a bracketed list or map may span lines" (#1882): "Operator chains, `if` and guards are not covered, and they are still broken by naming their parts". The long-constant half is still (rg `resit/f19m/`) |
| F7 | a section comment cannot stand on its own | language | aesthetics | nit | still | `resit/f7m/`: a `# Integers.` line and a blank line between two functions of a module gives "exactly one blank line separates top-level declarations". The 2026-10-13 header-comment ruling covers only a file's opening comment. The same file run with `kanso play` is accepted (`resit/f7.kso` prints `4`), see N4 |
| F8 | a run-time error names the module, not the file | language | diagnostic | minor | fixed | ruling 2026-10-15, #1975; migration step 5 printed `failed in toml/digits? at …/toml/datetime.kso:53` |
| F9 | `kanso run file.kso` refuses definitions beside statements | tooling | tooling | minor | partly | `run` still refuses; `kanso play` is in `kanso help` and is what the book's chapters now print (ch02, ch04) |
| F10 | clarifying parentheses are refused | language | aesthetics | nit | partly | `hash < h or (hash == h and key < k)` is now required (ruling 2026-10-13, #1887). `not (digit? cs[p])` stays refused under the same ruling |
| F11 | `pub fn name =` gets a worse message than `pub fn name` | language | diagnostic | nit | still | `resit/f11.kso`: `fn floats =` gives "expected a parameter pattern" |
| F12 | a `_` parameter accepts an err and the body runs | language | engine-bug | major | ruled-no | 2026-10-13, "gavel: an argument nobody reads is never computed, err or not" (#1928). A raised err bound to a name is now refused at check (`bugs/wildcard-swallows-err` gives "this can be an err and `ignores` has no arm for it") |
| F13 | compiled code runs an arm on an err when a sibling has `_` | language | engine-bug | blocker | fixed | `bugs/native-runs-arm-on-err` is now refused at check; `resit/f13/`, the same shape fed a builtin err the checker cannot see, gives the same endpoint report on interp, dev and release |
| F14 | a map read between writes sorts; a map in a record copies | core-lib | performance | blocker | partly | `resit/putperf/`, release: `named_put` 20,000 in 0.014 s (was over 30 s at 4,000), record-held `boxed` 20,000 in 0.040 s (was 14 s), inline read then put 8,000 in 0.012 s. Still a cliff: a read bound to a name before the put takes 0.16 / 1.22 / 9.6 s at 2,000 / 4,000 / 8,000 keys on release and 1.0 s on the interpreter. `bugs/named-read-before-put` |
| F15 | pushing through a helper segfaults at 64 | language | engine-bug | blocker | fixed | `bugs/push-through-helper-segfault`: `64` on interp, dev and release |
| F16 | calling a continuation is never a tail call | language | engine-bug | major | fixed | `bugs/closure-call-not-tail-call`: `5000050000` on interp, dev and release at 100,000 |
| F17 | floats print with exponents a literal cannot use | language | missing-feature | minor | fixed | ruling 2026-10-13, #1889; `resit/f17.kso` prints `true 0.65 1.0e+300` on interp and native |
| F18 | a map literal's value must be an atom, and the message misleads | language | diagnostic | minor | still | `resit/f18.kso`: `{ "d":day 27 5 }` gives "expected `:` after a map key" with the caret on `5` |
| F19 | the checker judges an overload group, not the selected arm | language | confusing-semantics | minor | still | `resit/f19.kso`: `io/write (encode { "a":1 })`, where the map arm cannot fail, gives "this can be an err and `io/write` has no arm for it" |
| F20 | one name, two arities, two files | language | refactoring-hazard | major | still | `resit/f20/` and `resit/f20c/`: a 2-argument `lookup` in one file calling a 3-argument `lookup` in another file of the same module gives "no 3-argument arm of `lookup` (arms take 2)", pub or not. Both arms in one file run (`resit/f20b/`). `bugs/arity-split-across-files` |
| F21 | a strict index's box passed to a function compiles | language | diagnostic | minor | fixed | `resit/f21.kso`: "this is an effect — a box the words open — and `os/read_file!` takes a value; open it with `.>`" at check |
| F22 | bytes and a list of ints look the same | core-lib | stdlib-gap | minor | partly | `resit/f22.kso`: `text/append (text/bytes "a") [9]` is still refused only at run time, now with the line: "append takes bytes and a string, bytes, or byte / failed in the entry at f22.kso:3" |
| F23 | std/list has no `sort_by` | core-lib | stdlib-gap | minor | fixed | ruling 2026-10-13, #1914; `resit/f23.kso` sorts records by a field: `[2 1 3]` on interp and native |
| F24 | `group_by` and `tally` read the map they fill | core-lib | performance | minor | fixed | `resit/putperf/readput/b_grouped_16000`: `list/group_by` over 16,000 distinct strings in 0.025 s on release (was 1.32 s) |
| F25 | the interpreter is about ninety times slower than release | tooling | performance | minor | still | `resit/perf/servers.toml` (240 KB, 2,000 `[[servers]]` tables): release 0.153–0.163 s, interpreter 12.8 s, output identical; about 80 times |
| F26 | no way to see where time goes | tooling | tooling | minor | still | `kanso help` lists no profile or counters flag |

Counts: fixed 9, partly 4, still 11, ruled-no 2, moot 0, worse 0.

## New friction

### N1: an err carried through names needs an arm at every step, and two of them need three
- scope: language
- kind: refactoring-hazard
- severity: major
- where: toml/document.kso (`resume`, `parse_document`, `pair_key`, `pair_value`, `header_names`), toml/string.kso (`block_run`), and 31 more arms across six files
- wanted: the failure railway the journal praised, unchanged: each step binds the previous step's result and calls the next
- wrote: a pass-through arm per function and position, 37 in all:
  ```
  fn resume _ _ e@(err _) (err _)
    e

  fn resume _ _ e@(err _) _
    e

  fn resume _ _ _ e@(err _)
    e

  fn resume cs current slots p
    parse_document cs p current slots
  ```
- why it matters: the change is the fix F3 asked for, and the ruling stands. The cost to a parser written as a railway is large. Every function that a bound failure reaches gains an arm whose body is `e`. Adding that arm makes the function a raiser, so its callers fail next, and fixing it took five rounds of `kanso check`. When two arguments can fail, the two single arms tie, and a third arm with `(err _)` in both places is needed. The other spelling, `.>`, puts the next step inside a lambda. That moves a tail call into a closure, and in rg (`bugs/effect_fold_overflows_native.kso`) a chain of closures is what overflows native stack.

### N2: a slice past the end answers none, and the checker misses it at builtins
- scope: core-lib
- kind: confusing-semantics
- severity: major
- where: toml/datetime.kso:53 (`digits?`), and 15 other `some_slice` sites
- wanted: `piece = text/slice tok i (i + n - 1)` / `length piece == n and …`, which relied on the old clamping to answer false for a short token
- wrote: `piece = some_slice (text/slice tok i (i + n - 1))`, with `fn some_slice none` / `[]`
- why it matters: thirteen in-range slices needed a helper only so the checker would accept them, the cost rg's N1 records. The case that mattered got no warning. `length` of a none slice compiles and stops at run time (`resit/lennone.kso`: "length takes a list, string, or map, not <none>"). Only the `time-no-seconds` fixture found it, on all three engines. A program that leaned on the old clamping as a length test changes behaviour without a compile error.

### N3: `kanso check <dir>` and `kanso check <entry>` disagree, and the entry's refusal has no excerpt
- scope: tooling
- kind: diagnostic
- severity: minor
- where: toml/table.kso:198, :212 (the slices) and :229 (where it was reported); `bugs/check-verbs-disagree-on-field-none`
- wanted: one verdict, at the slice that can be none
- wrote: `some_slice` at the two slices
- why it matters: `kanso check toml` said ok while `kanso check main.kso` refused, so a library can pass its own check and fail in the first program that imports it. The 2026-10-12 gavel says check is exact for a library checked alone. The refusal names the line that reads the field, not the slice that stored the none. It labels the module `cli` for a line in `toml/`, and it prints no source line or caret.

### N4: `kanso play` accepts formatting a module refuses
- scope: tooling
- kind: diagnostic
- severity: nit
- where: resit/f7.kso against resit/f7m/
- wanted: one formatting rule for every file
- wrote: nothing
- why it matters: a section comment followed by a blank line runs under `kanso play` and is refused in a module ("exactly one blank line separates top-level declarations"). A reader who tries an idiom in a play file, as the book teaches, learns a rule the real program will refuse.

### N5: the ruling's own spelling for slicing into an append does not parse
- scope: language
- kind: diagnostic
- severity: nit
- where: resit/slicebang.kso
- wanted: `text/slice! cs p q .> text/append acc`, as the 2026-10-14 gavel on #1943 writes it
- wrote: nothing in the port, because `slice!` answers a box and the box spreads into every caller of a pure decoder
- why it matters: the spelling is refused with "`.>` takes one function". `(&text/append acc)` and `(s -> text/append acc s)` parse, and then the box reaches the pure caller ("this is an effect … open it with `.>`"). So the insisting slice is not a way out of N2 for a pure parser.

## Bugs

- `bugs/named-read-before-put` (new): binding `m[key]` to a name before `put m key …` in a loop makes native code roughly cubic: 0.16 s, 1.22 s and 9.6 s at 2,000, 4,000 and 8,000 keys on release, against 0.012 s at 8,000 with the read inline. The interpreter takes 1.0 s. At 40,000 iterations the release binary ran 219 s and was killed.
- `bugs/arity-split-across-files` (new): a 2-argument arm in one file of a module cannot call a 3-argument arm of the same name in another file: "no 3-argument arm of `lookup` (arms take 2)".
- `bugs/check-verbs-disagree-on-field-none` (new): `kanso check lib` says ok, and `kanso check main.kso` refuses a none stored in a field, printing no source line.
- `bugs/native-runs-arm-on-err`: fixed. It is now refused at check, and a variant with a builtin err gives the same report on all three engines (`resit/f13/`).
- `bugs/wildcard-swallows-err`: ruled (#1928). The bound-name version is now refused at check.
- `bugs/push-through-helper-segfault`: fixed on dev and release.
- `bugs/closure-call-not-tail-call`: fixed on interp, dev and release.
- `bugs/named-put-superlinear` and `bugs/record-field-put-copies`: fixed on release (`resit/putperf/`).

## Comfort

Language and core library first. The engine bugs that cost the original agent
half its time are gone. An arm no longer runs on an err in compiled code, a
push through a helper no longer segfaults, and a continuation call is a tail
call. Putting into a map named by a binding, or held in a record, is now
linear. The decoder's first design, nested maps in records, would probably
work today, and `index.kso` might never have been written. Exponent floats,
`sort_by` and a fast `group_by` remove three small detours. The language
surface is stricter in a way this port feels directly. The failure railway the
journal liked best now needs 37 pass-through arms, and two-failure gates need a
third arm to break a tie (N1). `text/slice` answering none put a helper on
thirteen slices and broke one fixture at run time with no compile warning
(N2). The day-one irritants are unchanged: no character literal, `_` not
covering none, the one-sided guard, locals colliding with every declaration,
and section comments refused. One new cliff replaces the old ones: reading a
map into a name before putting into it is cubic on native. On tooling, the
interpreter is still about 80 times slower than release, and there is still
no profiler. An agent starting today would spend much less time reducing
engine bugs and more time on the checker. The migration took 215 changed
lines, most of them `e@(err _)` arms.
