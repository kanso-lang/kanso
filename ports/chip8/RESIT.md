# Re-sit: chip8

## Result

check.sh: pass. Unit tests isa 22, vm 36, asm 26 and cli 11 all pass, and
all 45 fixtures agree on interp, dev and release (`resit/check.log`).

Engines agree: yes.

Lines changed by the migration: 223 (`diff -r` against ports-orig/chip8,
counting `<` and `>` lines, `resit/` and `bugs/` left out). About half are
test rewrites.

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[syntax]: `and` and `or` meet without parentheses`` (#1887) | yes | 1 (`label_name?`) |
| ``error[exhaustive]: this can be a none and `prefixed` has no arm for it`` (#1890) | partly; each was a slice that cannot miss or whose none meant "empty", as it used to. A `sliced` helper in each of four modules | 16 refused slices in isa, asm, vm and cli, and one more in asm/lines.kso wrapped before it could fail |
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906) | yes | 14 |
| ``error[name]: `flags` is already a declaration; rename the binding`` for a local the migration added | yes; `flags` is a function elsewhere in cli | 1 |
| ``error[opacity]: `isa/draw` is foreign — only `isa` builds a `draw`; ask it for one through a pub function`` in the test files (#1883) | yes, but the fix was to stop building records in tests. isa_test now compares `spell` output or `.reason`, vm_test compares field lists and `.reason`, asm_test and cli_test compare field lists | 11 isa tests, 7 vm, 1 asm, 5 cli |
| ``error[name]: unknown name `position_of` `` and five more in the tests (#1883) | no; "unknown name" | 6 made pub: `position_of`, `size_of`, `dis_lines`, `extension`, `hex_lines`, `padded` |
| ``error[exhaustive]: this can be a none and `vm/execute` has no arm for it (in run.kso)`` with `--> vm/run.kso:47:42` and no source line, under `kanso test vm` only (N2) | no. The none came from the test's `vip = profile "vip"` through `configured` into a record field. A guard on the name did not help, and turning it into an err moved the refusal to the test's own calls | 1: vm/quirks.kso now exports `pub vip` and `pub modern`, and the tests name them |
| ``error[runtime]: split takes two strings`` / `--> isa/isa_test.kso:38:3`, pointing at a test that passed (B1) | no. The cause was a slice bound to a name (`rest = text/slice s ...`) that now answers none | 1 (a `none` arm on the test's `operand_texts`) |
| ``error[runtime]: concat takes two lists`` / `--> vm/vm_test.kso:70:3`, an empty line (B1) | no. `write_block`'s `tail = text/slice xs (stop + 1) (length xs)` is bound to a name and answers none past the end | 1 |
| ``error[runtime]: length takes a list, string, or map, not <none>`` / `--> cli/cli_test.kso:97:16` in a 46-line file (B1) | no. `hex_lines`'s `rest = text/slice bytes 17 (length bytes)` | 1 |

No expected-output file changed. The last three rows are the slices the
none check does not follow through a name (regex N1). Without the unit
tests they would have reached the fixtures as run-time failures.

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | an imported std type takes over a function I named `step` | confusing-semantics | major | language | fixed | `resit/f1`: a module importing std/list with its own `fn step` prints `5` on both engines (#1894) |
| F2 | no hex or binary integer literals, and the error is about spacing | missing-feature | major | language | fixed | `resit/f2_hex.kso`: `fn family 0xD op`, a font `[0xF0 0x90 ...]` and `0b1010` print `draw 3 [240 144 144 144 240] 10` on both engines (#1889) |
| F3 | a type named `random` loses to a hidden function | engine-bug | minor | language | fixed | `resit/f3` (the bug as a module): prints `m/random 3 1` on both engines (log 2026-10-12, "a name the author declared wins over a builtin and an import") |
| F4 | only the declaring module may build a type, so the parser moved | confusing-semantics | minor | language | ruled-no | the 2026-08-20 rule "only a type's owner constructs one", restated in the #1886 gavel; the journal itself calls the result reasonable |
| F5 | no collection or string literal may span lines | missing-feature | minor | language | fixed | `resit/f5_table.kso`: the 20 mnemonics as a spanning list print `20` (#1882). Canonical form puts one element on each line, so the table is 22 lines rather than the journal's 4 |
| F6 | `text/concat` refuses strings, though appendix B says it takes them | engine-bug | minor | core-lib | moot | `text/concat` is gone; lists join with `list/concat` and strings by interpolation (#1906) |
| F7 | two maybe-none arguments need four arms | confusing-semantics | minor | language | still | `resit/f7_acc.kso`, the journal's three arms: ``error[dispatch]: these `accumulate` arms tie`` |
| F8 | a qualified pattern on a foreign type is refused; the bare one works | engine-bug | major | language | still | `bugs/qualified_pattern`: ``error[opacity]: `shapes/jump` is foreign — its structure does not cross an import``; the bare `(jump addr)` (`resit/f8`) prints `jump 7` |
| F9 | no record update, so every change to the machine names all eleven fields | missing-feature | major | language | fixed | `merge` (#1886), mal's `resit/f10_merge.kso`. The twelve setters are unchanged in the port |
| F10 | a constant in a test file broke parameter names in another file | refactoring-hazard | major | language | fixed | `resit/f10`: a test constant `q` beside a module parameter `q` passes (#1883) |
| F11 | `text/slice` past the end answers empty, and the registers vanished | confusing-semantics | major | core-lib | ruled-no | #1890 ruled that a range past the end answers `none`, not a clamp. The loss is no longer silent where the slice reaches a call, but a slice bound to a name still passes unchecked and now fails later at run time (migration rows 8 to 10) |
| F12 | names a program cannot use for fields and parameters | confusing-semantics | minor | language | still | `resit/f12_entry.kso`: ``` `entry` is already a declaration``` (and `entry` is still not in appendix B's ambient list); `resit/f12_quirks.kso`: a field `quirks` beside `type quirks`, same refusal |
| F13 | a `none` passed to a setter needs an arm of its own | confusing-semantics | minor | language | still | `resit/f13_none.kso`: `with_pending (machine 1) none` gives ``this can be a none and `with_pending` has no arm for it`` |
| F14 | a native map used as memory is 35 times slower than the interpreter | performance | major | core-lib | still | the journal's `scratch/perf_map` (ported as `resit/f14`): 16,000 steps 13.7 s native, 1.6 s interpreted. Reduced to B2: reading a 16-key map after each overwrite is superlinear natively |
| F15 | there is no way to write a binary file | stdlib-gap | major | domain-lib | partly | `resit/f15_write2.kso`: `os/write_file "out/b.bin" (text/to_bytes [0 18 255 200])` writes the four bytes on both engines. A plain list is still ``error[type]: `write_file` takes a string here, not a list``, which does not mention `text/to_bytes`, and appendix B lists only `os/write_file string string → io` |
| F16 | std/text has no search and no case mapping | stdlib-gap | minor | core-lib | partly | `text/uppercase "ld v1"` gives `LD V1` (#1914). There is still no search by string; `text/find2` on bytes is the way, as the port does |
| F17 | a lazy `list/map` passed to `text/join` fails at run time with no location | diagnostic | major | language | fixed | `bugs/join_lazy_no_location.kso` is refused at check: ``` `text/join` takes a list here, and `list/map` answers a lazy sequence — make it a list with `list/to_list` ``` |
| F18 | the interpreter's `read_bytes` answers something `text/concat` refuses | engine-bug | major | core-lib | fixed | `resit/f18_bytes.kso`: `list/concat [1 2 3] rom` on `os/read_bytes!` prints `[1 2 3 0 224 ...` on both engines |
| F19 | the interpreter panics with a Rust backtrace when stdout closes | tooling | nit | tooling | fixed | `kanso play bugs/broken_pipe_panic.kso --interp \| head -2`: `1`, `2`, exit 141, no backtrace (log 2026-10-12) |
| F20 | a test constant silently replaces a function of the same name | engine-bug | major | language | fixed | `bugs/constant_hides_function` as a module: `2 passed, 0 failed` (#1883) |
| F21 | an import in a test file changes which `skipped` another file calls | engine-bug | major | language | fixed | `bugs/test_import_renames` as a module: `2 passed, 0 failed` (#1883, #1894) |
| F22 | parentheses that would help a reader are refused | aesthetics | nit | language | ruled-no | `resit/f22_parens.kso`, `resit/f22_sum.kso`: `(bits/shr op 8) & 15` and `(2 + 3) & 4` are still `these parentheses group nothing`. #1887 keeps every pair that groups nothing refused |
| F23 | overload order is ranked position by position, so a usage error needed its own function | confusing-semantics | minor | language | ruled-no | `resit/f23_cmd.kso`: the same ``overloads of `command` appear most-specific first``. #1888 keeps specificity order |
| F24 | calling `profile "modern"` inline is refused, naming it first is fine | confusing-semantics | minor | language | still | `resit/f24`: `ran (profile "vip")` is refused; `modern = profile "vip"` then `ran modern` passes; a local `q = profile "vip"` passes. In the port's own tests the named constant was refused once it travelled through a record field (N2) |
| F25 | a keyed read naming every field reports each field as an unknown name | diagnostic | minor | language | partly | `resit/f25`: one message at the read, with the fix: `a keyed read omits at least one field; reading every field is the positional form`. It comes at run time; `kanso check` says ok, though the record was built on the line above |
| F26 | `kanso run file.kso` will not run a file with definitions in it | tooling | nit | tooling | still | `run` refuses it and now names `kanso play`; appendix B's own `input.kso` is still refused by `run` (mal's `resit/f1_input.kso`) |

Counts: fixed 11, partly 3, still 7, ruled-no 4, moot 1, worse 0.

## New friction

### N1: unit tests that compared records built by hand had to be rewritten
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: isa_test, vm_test, asm_test, cli_test
- wanted: `decode 56533 == draw 5 12 13`, which the journal listed under
  "What worked well"
- wrote: `spelled 56533 == "DRW VC, VD, 5"`,
  `[l.label l.mnemonic l.n l.operands l.text] == lined`,
  `(parsed ["--fast"]).reason == "unknown flag `--fast`"`, and six names
  made pub
- why it matters: #1883 makes a test an outside consumer, and the owner-only
  construction rule then forbids the expected value. Structural equality,
  which the journal praised, now only works in tests through a printed form
  or a list of fields, and the type of the answer is no longer checked.

### N2: a none carried through a record field is reported in another file, without its line, and only under test
- kind: diagnostic
- severity: minor
- scope: language
- where: vm/vm_test.kso → vm/run.kso:47
- wanted: the test's `vip = profile "vip"` passed to `configured`, as the
  program's cli does with `q = vm/profile opts.quirks`
- wrote: `pub vip` and `pub modern` constants in vm/quirks.kso
- why it matters: `kanso test vm` reported `this can be a none and
  vm/execute has no arm for it (in run.kso)` at `vm/run.kso:47:42`, with no
  source excerpt, while `kanso check .` passed the program that does the same
  thing. The literal `"vip"` cannot reach `profile`'s none arm. A
  `return ... if q == none` guard did not help, and dispatching the none to
  an err moved the refusal to every caller.

### N3: a slice bound to a name hides its new none until run time
- kind: confusing-semantics
- severity: major
- scope: language
- where: vm/machine.kso `write_block`, cli/cli.kso `hex_lines`,
  isa/isa_test.kso `spells_back?`
- wanted: the checker to point at every slice #1890 changed
- wrote: `sliced` around each after a test failed at run time
- why it matters: the same as regex's N1. Here the run-time failures were
  also reported at the wrong place (B1), so each took a search through the
  source for `text/slice`.

## Bugs

- **B1** `bugs/test_failure_names_library_line/`: under `kanso test`, a
  failure inside a library function prints the library's line number under
  the test file's path: `split takes two strings` / `--> m/m_test.kso:38:3`
  for a 25-line file (std/text's line 38). `kanso play` on the same program
  prints `failed in text/split at std/text/text.kso:38`. In the port it
  pointed at a passing test, an empty line and a line past the end of the
  file.
- **B2** `bugs/map_read_after_overwrite_superlinear.kso`: a 16-key map in a
  record, read and overwritten once per lap. Native: 2,000 laps 0.33 s,
  4,000 0.63 s, 8,000 2.4 s. Interpreter: 0.1 s at 8,000. Without the read,
  native takes 0.1 s.

The eight bugs in the original `bugs/`: `type_named_random`,
`constant_hides_function`, `test_import_renames`, `read_bytes_concat` and
`broken_pipe_panic` are fixed; `join_lazy_no_location` is now refused at
check; `concat_strings` is moot; `qualified_pattern` still fails (F8).

## Comfort

On the language and core library, a new start would be noticeably easier.
Four of the summary's five are answered. The names that silently replaced
the author's own (F1, F3, F10, F20, F21) no longer do. `merge` exists (F9).
Hex and binary literals let the opcode tables read like the reference (F2).
A wrong value handed to `text/join` is refused at check with the fix (F17).
Spanning lists hold the font and mnemonic tables (F5), and `text/uppercase`
replaces the hand-written table (F16). What a new start would still meet is
the none handling the journal ended on. Two maybe-none arguments still need
a tie-breaking arm (F7). A none cannot be handed to a setter (F13). The
check still follows an inline call but not a name (F24). The migration added
a new form of the same problem: #1890 turned silent clamps into nones, which
the checker finds when the slice is passed straight on but not when it is
named, so three of this port's slices failed at run time, reported at the
wrong line (B1). Native map memory is still slower than the interpreter
(F14, B2), so the 4 KB copy per write stays. Unit tests lose some of their
shortness, because a test can no longer build the record it expects (N1).
