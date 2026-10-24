# Re-sit: kv

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the journal
re-test are in `resit/`, one per entry, named after it.

## Result

- check.sh: pass. 31 + 9 unit tests, 23 fixtures.
- Engines agree: yes, on all 23 fixtures. One bug found during migration
  makes the engines disagree in a `kanso play` file (Bugs, below); in the
  port's module layout all three engines failed the same way.
- Lines changed by the migration: 223 lines of `diff -r -x bugs -x resit`
  against ports-orig/kv. 128 of them are the three test files, which had to
  be rewritten (M6); the program itself changed by 95 lines.
- No expected-output file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| M1 | `` error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c` `` | yes | 1 |
| M2 | `` error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes | 15 in 6 files |
| M3 | `` error[name]: the name `merge` is already taken `` | partly. It does not say what took it (the new ambient `merge`, #1886). Renamed the port's `pub fn merge` to `compact`. | 2 |
| M4 | `` error[exhaustive]: this can be a none and `text/utf8` has no arm for it — resolve it here, or give `text/utf8` a `none` arm (in format.kso) `` (text/slice past the end answers none, #1890) | no. The second suggestion is impossible: an arm on `text/utf8` must be written by std/text (2026-10-12 gavel, rule 4). My first fix, a helper whose none arm answers `err`, turned six errors into ten `this can be an err` errors in callers further out. The fix that compiles is a helper whose none arm answers a dummy `[]`, the shape ch08's `digits_value none` teaches. | 9 program sites, 5 test sites; the helper is written three times (bitcask, cli, the test file) |
| M5 | none at check. Every engine stopped with `` error[runtime]: `<io>` is not callable `` / `failed in list/fold_flat at std/list/list.kso:214` | no. The port's own `fn read_line`, passed to `list/fold`, now calls `io/read_line` (added 2026-10-14) because a sibling file imports std/io. Renamed to `take_line`. See Bugs. | 2 lines |
| M6 | `` error[opacity]: `bitcask/record` is foreign — only `bitcask` builds a `record`; ask it for one through a pub function `` and `` error[name]: unknown name `put_cmd` `` (a test file is an outside consumer, 2026-10-12 gavel) | it says what is wrong, not how to test it. The tests compared answers against records they built. Now they read fields through predicates written in the test file, the records for the hint tests come from a scan, `store.kso` gained two pub functions only the tests call (`empty_store`, `located_at`), and nine command types in `cli/commands.kso` became pub so a test arm can name them. | 3 test files rewritten, 2 pub functions added, 9 types made pub |
| M7 | `` error[type]: no type is called `torn`, so this arm can never match `` | no. In a test file, bare pub functions resolve (`encode_record`) but a bare type in an arm does not; `bitcask/torn` does. | 4 |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | no way to write a byte above 0x7f | stdlib-gap | blocker | domain-lib | fixed | `resit/f01_write_bytes.kso`: `os/write_file "out.bin" (text/to_bytes [0 1 127 128 200 255])` then `read_bytes!` prints `[0 1 127 128 200 255]` on both engines. A bare list is refused: `` `write_file` takes a string here, not a list `` (N9). The port still shells out; moving it is a rewrite of disk.kso, not a migration. |
| F2 | no append, no remove, no rename | stdlib-gap | major | domain-lib | still | `grep '^pub' lib/os/os.kso` lists `exists is_dir list_dir make_dir read_file read_bytes run start kill write_file modified` and nothing that appends, removes or renames. #1914 ruled other names and not these. |
| F3 | the book's `pub play` samples do not run as shown | tooling | minor | tooling | still | `kanso run docs/book/samples/ch05/fused.kso` answers `` error: `.../fused.kso` is a library — nothing to run. it exports `play` ``, while ch05 still shows `kanso run fused.kso` printing `no orders yet`. |
| F4 | a top-level binding in a play file is not visible to its functions | confusing-semantics | minor | language | still | `resit/f04_play_binding.kso` under `kanso play`: `` error[unused]: unused binding `table` `` and `` error[name]: unknown name `table` `` for the same name. A `main.kso` inside the module still does not see the module's names: `` unknown name `pick` ``. |
| F5 | an in-range lookup still needs a `none` arm | missing-feature | minor | language | still | `resit/f05`: `bits/xor table[(bits/xor c b & 3) + 1] ...` gets `` this can be a none and `bits/xor` has no arm for it ``; `table[i]! + 1` gets `` error[effect]: this is an effect — a box the words open ``. #1890 made it more common (N3). |
| F6 | `entry` is taken | aesthetics | nit | language | still | `resit/f06_entry.kso`: `` `entry` is already a declaration; rename the binding ``. |
| F7 | `text/bytes` answers something not quite a list | engine-bug | major | core-lib | partly | `resit/f07_text_bytes.kso`: `list/concat [1] (text/bytes "a")` prints `[1 97]` on both engines (log 2026-10-12, "concat reads the bytes text/bytes answers"). `push (text/bytes "a") 2` still stops with `push takes a list and a value`, now on both engines and with a line; `kanso check` passes it. `bugs/push_onto_text_bytes.kso`. |
| F8 | a private helper in one file breaks a parameter name in another | refactoring-hazard | major | language | still | `resit/f08`: adding `fn at` in b.kso gives `` `at` is already a declaration `` with the caret in a.kso's parameter, and the message does not name b.kso. |
| F9 | a map cannot drop a key | stdlib-gap | major | core-lib | fixed | `resit/f09_delete.kso`: `delete keydir "a"` prints `{ "b":2 }` and `1` on both engines (#1914). |
| F10 | `text/slice` past the end answers nothing | confusing-semantics | major | core-lib | partly | Ruled on #1890: past the end answers `none`, not a clamped slice, so the clamp the entry wanted is declined. The silent empty write is gone: `resit/f10c` (the entry's shape, a slice handed to `list/map`) now stops both engines with `length takes a list, string, or map, not <none>`. But `kanso check` passes it and the failure names `std/list/list.kso:213`, not the slice (N4). |
| F11 | a lazy `list/select` passes check and fails at run | diagnostic | minor | language | partly | `resit/f11b.kso`: `length (list/select ...)` written inline is refused at check with a `list/to_list` hint (log 2026-10-13, "a lazy sequence handed where a list goes is refused at check"). The port's shape, a select returned from another module's function, `resit/f11`, still checks ok and stops at run, now naming `f11/main.kso:3`. |
| F12 | a foreign pub record reads with a dot but not with a pattern | confusing-semantics | minor | language | still | `resit/f12`: `` `m/merged` is foreign — its structure does not cross an import ``; dot reads of the same fields still pass (the rewritten tests use them). |
| F13 | `kanso check .` inside a module cannot resolve `../sibling` | tooling | minor | tooling | fixed | from `cli/`, `kanso check .` prints `.: ok` (log 2026-10-14, "`kanso test .` finds the module it is run inside"). |
| F14 | no multi-line string; a long list cannot be wrapped | missing-feature | minor | language | fixed | `resit/f14_usage.kso` prints the usage from a `"""` text block, and a spanning list literal checks (#1882). Text blocks were built in July, before the port, but the book never mentions them (N7). |
| F15 | a none from `argv[1]` reported three modules away | diagnostic | major | language | fixed, with a regression | `resit/f15`: the wanted code, `options argv[1] argv` with `fn started (options none _)`, checks and prints `no directory`. Without that arm, `kanso check` now says nothing and every engine stops with `list_dir takes a string` (N4). |
| F16 | two overload arms specific in different positions | confusing-semantics | minor | language | still | `resit/f16`: the order error, then `` these `located` arms tie: each is the more specific one somewhere ``. #1888 kept the specificity-order rule and did not touch this. |
| F17 | adding a field means editing every construction | refactoring-hazard | major | language | fixed | `resit/f17`: `merge st { "size":(st.size + n) }` prints `1 db 5` on both engines (#1886). |
| F18 | `[]` is not a parameter pattern | missing-feature | minor | language | still | `resit/f18.kso`: `fn run_words []` gets `error[syntax]: expected a parameter pattern`. |
| F19 | growing a list with concat is quadratic and keeps every copy | performance | major | core-lib | fixed | `resit/f19_concat.kso` at n = 8000 (200,000 elements), release build: 0.01 s and 9.8 MB peak; the interpreter 0.02 s. The journal had a SIGKILL at 3 GB. Log 2026-10-13, "a list concatenated onto in a fold grows where it stands". |
| F20 | no positional read | stdlib-gap | minor | domain-lib | still | no read-at in lib/os/os.kso. |
| F21 | no `ends_with`, `starts_with` or `index_of` | stdlib-gap | nit | core-lib | partly | `resit/f21_affixes.kso` prints `true true` (#1914). `text/index_of` is `unknown name`; `find2` exists but works on bytes. |
| F22 | a test that does not fit on a line must grow a binding | aesthetics | nit | language | still | hit again in the migration: `` error[formatting]: a single-expression constant is written inline: `test_end_of_bytes = ...` `` for an 87-column test, so it grew a `got =`. |
| F23 | parentheses that help a reader are refused | aesthetics | nit | language | fixed | #1887 made them required: `resit/f23.kso` with `f < g or (f == g and o < p)` prints `true`, and the old spelling is refused (M1). |

Counts: fixed 8 (F1, F9, F13, F14, F15, F17, F19, F23), partly 4 (F7, F10,
F11, F21), still 11 (F2, F3, F4, F5, F6, F8, F12, F16, F18, F20, F22),
ruled-no 0, moot 0, worse 0. F15 is fixed with a regression beside it (N4),
and F10's clamp half was declined by #1890.

## New friction

### N1: a new ambient name takes a module's own function, and the message does not say what took it
- kind: diagnostic
- severity: minor
- scope: language
- where: bitcask/merge.kso:43
- wanted: `pub fn merge st`, as the port had it on 2026-10-10.
- wrote: `pub fn compact st`, after `` error[name]: the name `merge` is already taken ``.
- why it matters: `merge` and `delete` became ambient in October, and any
  program that already used either word for its own function breaks. The
  message should say that the name is the ambient `merge`, and where it is
  documented.

### N2: a function the module declares is replaced by a std constant of the same name
- kind: engine-bug
- severity: major
- scope: language
- where: cli/commands.kso:92, `list/fold lines (reading [] 0) read_line`
- wanted: the port's `fn read_line`, unchanged.
- wrote: `take_line`, after every engine stopped with `` `<io>` is not callable ``
  in `list/fold_flat at std/list/list.kso:214`. `kanso check` passed.
- why it matters: `io/read_line` was added to std/io on 2026-10-14, and a
  file elsewhere in the module imports std/io. A library addition silently
  changed which function a working program calls, the check did not see it,
  and the run-time message pointed into std/list. See Bugs.

### N3: every slice the program knows is in range now needs a dead `none` arm
- kind: confusing-semantics
- severity: major
- scope: core-lib
- where: bitcask/bytes.kso (`inside`), cli/commands.kso (`inside`), bitcask/format_test.kso (`inside`)
- wanted: `key = text/utf8 (text/slice bs (key_at + 1) (key_at + ksz))` after
  `return torn offset if length bs < offset + size`, which proves the range.
- wrote:

      pub fn inside none
        []

      pub fn inside xs
        xs

      key = text/utf8 (inside (text/slice bs (key_at + 1) (key_at + ksz)))

- why it matters: a binary decoder slices ranges it has already checked.
  #1890 makes each of those answer `none`, and the checker does not read the
  guard above. The way out is a dummy value that would hide a real bug, the
  same shape ch08 teaches with `fn digits_value none` answering `0`. An `err`
  in that arm does not compile in pure code: it spreads `this can be an err`
  to every caller. `text/slice!` answers a box, so it makes the decoder
  effectful. `text/to_span` answers `none` too. The helper had to be written
  three times, once per module and once in the test file. This is F5 at
  fourteen sites.

### N4: a `none` that reaches a builtin passes the check
- kind: diagnostic
- severity: major
- scope: language
- where: resit/f10c, resit/f15
- wanted: the check to refuse `list/map (text/slice bs 1 4096) f` or
  `os/list_dir opts.dir`, where `opts.dir` came from `argv[1]`, the way it
  refuses `text/utf8 (text/slice ...)`.
- wrote: nothing. Both check ok. At run time every engine stops with
  `length takes a list, string, or map, not <none>` / `failed in list/fold_flat at std/list/list.kso:213`,
  or `list_dir takes a string`.
- why it matters: #1890 was ruled on the condition that "the exhaustiveness
  check sees `none` in `text/slice`'s answer". It sees it at a std function
  with arms and misses it at `length`, at `list/map` and at an os builtin.
  F15's case used to be refused, at the wrong place; now it is not refused at
  all.

### N5: a test cannot build the value it expects
- kind: refactoring-hazard
- severity: major
- scope: language
- where: bitcask/format_test.kso, bitcask/store_test.kso, cli/commands_test.kso
- wanted: `decode_at bs 0 == record "key" 0 22 7 "value"` and
  `command_of ["get" "k"] == get_cmd "k"`, as on 2026-10-10.
- wrote: a predicate per type in the test file,

      fn record? r key offset size stamp value
        same = r.key == key and r.offset == offset and r.size == size
        same and r.stamp == stamp and r.value == value

  records taken from a scan of encoded bytes instead of written out, two pub
  functions in store.kso that only the tests call, and nine command types
  made pub.
- why it matters: the 2026-10-12 gavel makes a test file an outside
  consumer, and only a type's owner constructs it. An equality test against a
  literal value was the clearest test a pure codec has, and it is now
  impossible for any record the module owns. Every port with record-valued
  tests will pay this.

### N6: a test file resolves a bare function but not a bare type
- kind: diagnostic
- severity: minor
- scope: language
- where: bitcask/format_test.kso:6
- wanted: `fn stopped_at? t:torn "torn" n`, since `encode_record` and
  `decode_at` resolve bare in the same file.
- wrote: `t:bitcask/torn`, after `` error[type]: no type is called `torn`, so this arm can never match ``.
- why it matters: the message says the type does not exist, when the module
  next to the test declares it as pub. It should suggest the qualified name.

### N7: the book never mentions text blocks
- kind: tooling
- severity: minor
- scope: tooling
- where: docs/book/
- wanted: a chapter or appendix C entry for `"""`.
- wrote: nothing; grepping the book and its samples for `"""` finds nothing.
  Text blocks were ruled and built in July (archive, "GAVEL: text blocks").
- why it matters: F14's string half had an answer before the port started,
  and the port wrote four constants joined by interpolation because the
  specification it was told to read does not have it.

### N8: map literal spacing is unlike every other literal
- kind: aesthetics
- severity: nit
- scope: language
- where: resit/f17, resit/maps.kso
- wanted: `merge st {"size": st.size + n}`.
- wrote: `merge st { "size":(st.size + n) }`, after four
  `canonical form requires exactly one space here` errors. `{"x":1}` is
  refused, `{ "x":1 }` is accepted, and a value with an operator needs
  parentheses because the pair must be tight.
- why it matters: the message does not say which space it wants, and a list
  literal `[1 2]` takes no inner spaces, so the map form surprises.

### N9: `os/write_file` of a list does not point at `text/to_bytes`
- kind: diagnostic
- severity: nit
- scope: domain-lib
- where: resit/f01_write_bytes.kso
- wanted: `os/write_file "out.bin" [0 1 128]`.
- wrote: `os/write_file "out.bin" (text/to_bytes [0 1 128])`, after
  `` `write_file` takes a string here, not a list ``. Appendix B lists
  `write_file string string → io` and does not mention bytes.

## Bugs

- `bugs/local_fn_loses_to_imported_constant.kso` (new): a function a file
  declares (`fn stdin n`) loses to a pub constant of an imported std module
  (`io/stdin`). Under `kanso play` the interpreter stops with
  `` `<io>` is not callable `` and the native build prints `2`, so the
  engines disagree. In a module directory all three engines stop, and a
  sibling file's import is enough. This broke kv's `read_line` (M5).
- `bugs/push_onto_text_bytes.kso` (new, replaces the stale
  `text_bytes_is_not_a_list.kso`, which no longer compiles because
  `text/concat` is gone): `push (text/bytes "a") 2` stops with
  `push takes a list and a value` on both engines; check passes it.
- `bugs/concat_accumulator_memory.kso`: fixed (F19). It still names
  `text/concat` and no longer compiles; `resit/f19_concat.kso` is the same
  program on today's names.

## Comfort

The language and core library are friendlier than the journal describes in
the places that hurt most. `merge` replaces the positional rebuild of the
state record, `delete` replaces the tombstone markers, `list/concat` no
longer goes quadratic, `starts_with?` and `ends_with?` exist, long lists
span lines, and `os/write_file` with `text/to_bytes` can put a raw byte on
disk. The original agent would not have written `reshaped`, the `gone`
marker or `sh -c printf` for the write itself. Some things have got harder.
#1890 turns every checked slice in a binary decoder into a `none` the
checker cannot see past, and the only pure answer is a dummy arm. The new
test rule means a codec's tests can no longer compare against a literal
record. A `none` that reaches `length`, `list/map` or an os builtin passes
the check where the journal's F15 at least got an error. The name-collision
problems (F6, F8) and the arm-tie problem (F16) are unchanged, and a std
addition silently replaced one of the port's own functions. The domain gaps
remain: no append, remove, rename or positional read, so the port would
still shell out for those. On balance a new start would be easier in the
core and about the same at the edges. Most of the migration's effort went
into the tests and into finding why `read_line` stopped working.
