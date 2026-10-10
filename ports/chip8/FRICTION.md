# FRICTION: porting a CHIP-8 emulator to kanso

Recorded while writing the port, in the order the problems came up. Every
code sample was compiled with `/tmp/claude-0/kanso-main/kanso`.

## Entries

### F1: an imported std type takes over a function I named `step`
- kind: confusing-semantics
- severity: major
- where: first prototype of `vm/run.kso` (kept as `scratch/step_collision_m.kso`)
- wanted: the name every emulator uses for "execute one instruction":
  ```
  import "std/list"

  pub fn run m n
    return m if n == 0
    run (step m) (n - 1)

  fn step (machine cycles i memory pc v)
    ...
  ```
- wrote: `fn execute`. The module imports `std/list`, which exports a
  `pub type step`, and that type's constructor answered the call:
  `error[arity]: `step` has 2 field(s), got 1 (construction is positional,
  fields alphabetical)`. The `fn step` declaration two lines below drew no
  complaint at all.
- why it matters: the message talks about fields of a record I never
  declared, in a file whose only `step` is a function. Nothing points at the
  import. I only knew to look because I had grepped `lib/list` for its
  exports an hour earlier. `std/list` exports fourteen such type names
  (`bounded`, `capped`, `cursor`, `sorted`, `step` ...), none of which a
  reader would guess.

### F2: no hex or binary integer literals, and the error is about spacing
- kind: missing-feature
- severity: major
- where: `isa/decode.kso`, `isa/encode.kso`, `vm/font.kso`
- wanted: `fn family 0xD op`, `xkk 0xF x 0x1E`, a font table written
  `0xF0 0x90 0x90 0x90 0xF0`, the way every CHIP-8 reference and every other
  emulator writes them.
- wrote: decimal, with the hex in a comment above each arm (`# Fx1E` over
  `fn misc 30 x _`), and the font as a string of hex pairs parsed at start-up
  by a helper I wrote (`hex_table "F0 90 90 90 F0 ..."`).
  `x = 0xff` is refused with `error[formatting]: canonical form requires
  exactly one space here`, pointing between the `0` and the `x`.
- why it matters: an emulator is mostly bit fields and opcode constants, and
  every one of them has to be translated by hand and checked by hand. I got
  one wrong in a test (`54485` for `DCD5`, which is `56533`): the test
  failed, and the time went into finding that the decoder was right and the
  hand conversion in the test was not. The diagnostic suggests a typo
  rather than a missing feature.

### F3: a type named `random` loses to a hidden function
- kind: engine-bug
- severity: minor
- where: `isa/types.kso` (now `rnd`); `bugs/type_named_random/`
- wanted: `pub type random` for the `Cxkk` instruction, built with
  `random byte x`.
- wrote: `pub type rnd`. The module never imports `std/math`, and a bare
  `random 3` in an entry file is `unknown name`, yet building my own type is
  refused with `error[arity]: `random` takes 1 argument(s), got 2`.
- why it matters: a name that is invisible when you use it is still taken
  when you define it, and the declaration itself is accepted. The error
  blames the call site.

### F4: only the declaring module may build a type, so the parser moved
- kind: confusing-semantics
- severity: minor
- where: `isa/parse.kso`
- wanted: the assembler (`asm/`) parses `LD V1, #05` and builds
  `set_byte 5 1` itself, importing the instruction types from `isa/`.
- wrote: the operand parser lives in `isa/` beside the types, and the
  assembler calls `isa/parse mnemonic operands resolve`. Building a foreign
  type is refused (`error[opacity]: `isa/jp` is foreign — only `isa` builds a
  `jp``), while taking one apart in a parameter pattern is allowed.
- why it matters: the rule is stated clearly and the error names the fix. I
  record it because it decided where code lives: anything that builds an
  instruction has to be in the module that declares it, so `isa/` ended up
  holding the assembler's operand grammar. It is a reasonable place for it.

### F5: no collection or string literal may span lines
- kind: missing-feature
- severity: minor
- where: `isa/parse.kso` (`mnemonics`), `isa/isa_test.kso` (the opcode
  table), `vm/font.kso`, `cli/cli.kso` (`usage`), `vm/run.kso` (`report`)
- wanted:
  ```
  pub mnemonics = [
    "ADD" "AND" "CALL" "CLS" "DRW" "JP" "LD" "OR" "RET" "RND" "SE" "SHL"
    "SHR" "SKNP" "SKP" "SNE" "SUB" "SUBN" "SYS" "XOR"
  ]
  ```
- wrote: two string constants and `text/split "{first_half}{second_half}" " "`.
  The list form is `error[formatting]: an inline constant has no indented
  block`, and a string broken across lines is `unterminated string`.
- why it matters: an emulator carries tables (the font, opcode lists for
  tests, sprite data). With an 80-column limit and no multi-line literal,
  every table becomes several constants glued together by a split, so the
  table is data in a string rather than a list the compiler can see.

### F6: `text/concat` refuses strings, though appendix B says it takes them
- kind: engine-bug
- severity: minor
- where: `isa/hex.kso:16`; `bugs/concat_strings.kso`
- wanted: `text/concat (digit (n & 15)) acc`, as appendix B documents
  (`text/concat (list | string) (list | string)`).
- wrote: `"{digit (n & 15)}{acc}"`. Both engines answer `error[runtime]:
  concat takes two lists`, and `kanso check` accepts the call.
- why it matters: the failure is only at run time, and under `kanso test` it
  was reported as `--> isa:16:40`, a module name and no file, so I had to
  work out which file line 16 belonged to.

### F7: two maybe-none arguments need four arms
- kind: confusing-semantics
- severity: minor
- where: `isa/hex.kso` (`accumulate`), `isa/hex.kso` (`prefixed`)
- wanted:
  ```
  fn accumulate none _ _
    none

  fn accumulate _ none _
    none

  fn accumulate acc d base
    if (d < base) (acc * base + d) none
  ```
- wrote: a fourth arm, `fn accumulate none none _`, which repeats the first.
  Without it: `error[dispatch]: these `accumulate` arms tie`. Moving the
  digit check into a helper did not help either: the checker then demanded a
  `none` arm on `accumulate` itself, because it does not look inside the
  helper.
  The same tie rule rejected `fn prefixed "#" _ t` beside `fn prefixed _ "0x" t`
  (a string cannot start with both), so the `0x`/`0b` prefixes needed a
  second function, `after_zero`.
  A guard inside the callee, `return none if acc == none or d == none`,
  does not count either: the same `error[exhaustive]` at the call. What does
  work, I found later in `vm/keypad.kso`, is a guard in the caller on a
  local name before the call (`return bad_script "..." if key == none`).
  It came back three more times in the assembler: `added` and `subtracted`
  (summing the terms of `sprite+5`, either of which may be an unknown label)
  each needed a `none none` arm, and `named acc "" none _` exists only
  because `named acc "" _ _` and `named acc label none n` were called a tie.
- why it matters: the extra arm says nothing a reader needs. It exists to
  settle a tie that no call could produce. And the three ways of saying "if
  this is none, stop" behave differently: an arm works, a caller-side guard
  works, a callee-side guard does not.

### F8: a qualified pattern on a foreign type is refused; the bare one works
- kind: engine-bug
- severity: major
- where: `vm/execute.kso` (all 35 arms); `bugs/qualified_pattern/`
- wanted: `pub fn execute (isa/add_byte byte x) m _`, spelling the module the
  way chapter 07 says it prefers.
- wrote: `pub fn execute (add_byte byte x) m _`. The qualified spelling is
  `error[opacity]: `isa/add_byte` is foreign — its structure does not cross
  an import; use its module's pub operations`, once per arm, 35 errors. The
  bare spelling destructures the same foreign record without complaint.
- why it matters: the message says the thing cannot be done at all, and I
  nearly restructured the program around it (moving `execute` into `isa/`)
  before trying the bare name on a hunch. The bare name is also the less
  readable one: a reader of `vm/execute.kso` cannot tell that `add_byte`
  comes from `isa`.

### F9: no record update, so every change to the machine names all eleven fields
- kind: missing-feature
- severity: major
- where: `vm/machine.kso` (`advanced` .. `with_v`, twelve setters),
  `cli/options.kso` (`flag`, eight arms), `vm/run.kso` (`session`)
- wanted: `m with pc: m.pc + 2`, or OCaml's `{ m with pc = m.pc + 2 }`. The
  emulator changes one or two fields of an eleven-field machine on every
  instruction.
- wrote: one setter per field, each a full positional rebuild:
  ```
  fn with_i (machine delay display _ held memory pc pending rng sound stack v) i
    machine delay display i held memory pc pending rng sound stack v
  ```
  The header of each setter is 76 to 79 columns. They could not be `pub`:
  `pub fn ` would have pushed them past 80, and a parameter pattern cannot
  wrap. The option parser has the same shape with a keyed read in place of
  the pattern:
  ```
  { cycles files list per_frame quirks seed trace } = opts
  updated = options cycles files list per_frame quirks value seed trace
  ```
- why it matters: twelve setters are about 40 lines that say nothing, and every
  one of them is a place to transpose two fields of the same type (`delay`
  and `sound` are both ints; so are `i` and `pc`). Adding a field to
  `machine` means editing all twelve and `boot`. I avoided a `cycles` field
  on the machine for that reason and put it in a separate `session` record,
  which has its own rebuilds.

### F10: a constant in a test file broke parameter names in another file
- kind: refactoring-hazard
- severity: major
- where: `vm/vm_test.kso` (constant `q`), `vm/execute.kso` (parameters `q`)
- wanted: a test-file constant `q = profile "vip"` to hold the quirks for
  the tests.
- wrote: `vip = profile "vip"`. With `q`, ten functions in `vm/execute.kso`
  stopped compiling: `error[name]: `q` is already a declaration; rename the
  binding`, pointing at parameters in a file I had not touched.
  It happened three more times while writing tests: `timed` in
  `vm/vm_test.kso` broke a local in `vm/run.kso`, and `listing` and `twice`
  in `asm/asm_test.kso` broke three parameters and a local in `asm/asm.kso`.
  Each time the fix was to rename the test constant. (When the clash is with
  a function rather than a parameter there is no error at all; see F20.)
- why it matters: one namespace per module plus no shadowing means a new
  top-level name anywhere in the directory can break any function in it, and
  a test file is part of the directory. The error points at the victims, not
  at the new declaration, so in a larger module finding the cause is a
  search. Test constants are exactly the names one picks without thinking
  (`added`, `timed`, `listing`), which are also the names helpers have.

### F11: `text/slice` past the end answers empty, and the registers vanished
- kind: confusing-semantics
- severity: major
- where: `vm/machine.kso` (`write_block`, `upto`, `span`), `cli/cli.kso`
  (`hex_lines`)
- wanted: Python's or Go's clamping behaviour, so that "the first 16 bytes"
  of a 9-byte list is the 9 bytes:
  ```
  kept = text/slice block 1 (length xs - at)
  ```
- wrote: `upto` and `span`, which clamp the end before slicing. Before that
  the first run of the emulator printed `v  ` with no registers and a blank
  screen: writing one register sliced `[value]` from 1 to 16, got `[]`, and
  the register list shrank by one on every write.
- why it matters: appendix B documents this ("out-of-range or inverted
  bounds yield an empty result"), but a slice whose end overruns is the
  ordinary way to say "up to n". The empty answer is silent data loss: no
  failure, no none, just a shorter list. The hex dump in `cli` had the same
  bug for the last line of every ROM whose size was not a multiple of 16.

### F12: names a program cannot use for fields and parameters
- kind: confusing-semantics
- severity: minor
- where: `vm/machine.kso` (field `held`), `cli/options.kso` (field
  `script`), `vm/run.kso` (parameter `q`), `asm/asm.kso` (bindings `items`,
  `row`)
- wanted: a machine field `keys`, an option field `keys`, a config field
  and parameter `quirks`, a list parameter `values` and a listing line
  bound to `entry`.
- wrote: `held`, `script`, `q`, `items`, `row`. `keys`, `values` and `entry`
  are ambient builtins, and `quirks` is my own type, so each is `error[name]:
  `keys` is already a declaration; rename the binding` at every pattern that
  binds the field under its own name.
- why it matters: a record field may be called `keys`, but it cannot then be
  taken apart under that name, so the field name and the name a reader sees
  in the code drift apart. `entry` is not in appendix B's list of ambient
  names at all.

### F13: a `none` passed to a setter needs an arm of its own
- kind: confusing-semantics
- severity: minor
- where: `vm/machine.kso` (`released`), `vm/execute.kso` (`waited`)
- wanted: `with_pending m none` to clear the FX0A latch, since a record field
  may hold none.
- wrote: a separate setter `released` that builds the machine with `none`
  in that field. `with_pending (advanced m) none` is refused with
  `error[exhaustive]: this can be a none and `with_pending` has no arm for
  it`, and adding a `none` arm to `with_pending` made its header 81 columns.
- why it matters: storing none in a field is allowed and reading it back
  dispatches cleanly, but the one ordinary way to put it there, through a
  function, is not.

### F14: a native map used as memory is 35 times slower than the interpreter
- kind: performance
- severity: major
- where: `scratch/perf_map/` and `scratch/perf_list/` (the measurements);
  design of `vm/machine.kso`
- wanted: memory as a map from address to byte, `put mem a value` on a
  write and `mem[a]` on a read, since lists cannot be written at an index.
- wrote: memory as a 4096-element list, and every write copies it with two
  slices and two concatenations (`write_block`).
- why it matters: measured with a loop that writes one byte and reads two
  per step. Map memory: 50,000 steps on `--interp` in 6.3 s; natively 1,000
  steps in 0.75 s, 2,000 in 1.39 s, 4,000 in 2.82 s, so about 0.7 ms per
  step, which projects to 35 s for 50,000. List memory with a full copy per
  write: 1.4 s native dev, 1.0 s release, 7.5 s interpreted. A second
  sitting later in the day, on a busier machine, read 2.1 s for the list
  version at 50,000 steps and 3.6 s for the map version at 4,000. So the faster
  structure depends on the engine, and on the native engines a copy of 4 KB
  per write beats a map. CHIP-8 programs write memory rarely (FX33, FX55),
  so the copy is affordable here; an emulator for a machine whose programs
  write memory every few instructions would not be.

### F15: there is no way to write a binary file
- kind: stdlib-gap
- severity: major
- where: `cli/cli.kso` (`asm` prints hex); `roms/*.ch8`
- wanted: `chip8 asm game.asm -o game.ch8`, writing the assembled bytes.
- wrote: the assembler prints hex text, the emulator reads `.hex` files as
  well as `.ch8`, and the two binary ROMs in `roms/` were made outside
  kanso. `os/write_file "out.bin" [0 18 255 200]` is `error[type]:
  `write_file` takes a string here, not a list`, and `text/utf8` refuses the
  bytes because they are not UTF-8.
- why it matters: `os/read_bytes` exists, so a program can read a binary
  format and never write one. An assembler whose output cannot be saved is
  half a tool.

### F16: std/text has no search and no case mapping
- kind: stdlib-gap
- severity: minor
- where: `asm/lines.kso` (`position_of`), `isa/parse.kso` (`upper`)
- wanted: `text/index_of code ";"` and `text/upper mnemonic`.
- wrote: `position_of`, which converts the string to bytes and calls
  `text/find2` with the same byte twice (`text/find2 (text/bytes s) 1 c c`),
  and `upper`, which maps each character through `text/char_code` and a
  26-letter table. `find2` answers a byte position, which equals the
  character position only for ASCII, so the assembler's comment stripping is
  correct only for ASCII source.
- why it matters: an assembler is mostly splitting lines at `;`, `:` and
  `,` and comparing mnemonics without regard to case. These are the first
  two functions anyone writing a tokenizer reaches for.

### F17: a lazy `list/map` passed to `text/join` fails at run time with no location
- kind: diagnostic
- severity: major
- where: `cli/cli.kso` (`located`, `hex_lines`); `bugs/join_lazy_no_location.kso`
- wanted: `text/join (list/map errors (e -> "{path}: {e}")) "\n"`
- wrote: `text/join (list/to_list (list/map errors ...)) "\n"`. Without the
  `to_list`, `kanso check` passes, and the run fails with
  `error[runtime]: join takes a list of strings and a separator` and nothing
  else: no file, no line.
- why it matters: it took a bisection over cycle counts to find, because the
  failure looked like it came from the emulator's report, and was in fact
  the assembler's error path: the ROM had a mistake in it and the error
  message could not be printed. The two other runtime failures I hit
  (`comparison requires two values of one comparable type`, from a resolver
  that answered the wrong type, and `concat takes two lists`) also came with
  no location.

### F18: the interpreter's `read_bytes` answers something `text/concat` refuses
- kind: engine-bug
- severity: major
- where: `cli/cli.kso` (`from_binary`); `bugs/read_bytes_concat.kso`
- wanted: copy a `.ch8` ROM's bytes into memory with `text/concat`, the same
  way the assembled bytes are copied.
- wrote: `list/to_list (list/map bytes (b -> b))` first, an identity map
  whose only job is to change what kind of list the bytes are. Without it,
  the interpreter fails with `concat takes two lists` and both native tiers
  run the ROM.
- why it matters: this is the one place the three engines disagreed, and it
  was found only because `check.sh` runs every fixture three ways.

### F19: the interpreter panics with a Rust backtrace when stdout closes
- kind: tooling
- severity: nit
- where: any `kanso run . --interp -- run rom.asm | head`;
  `bugs/broken_pipe_panic.kso`
- wanted: the program to stop quietly when the reader goes away, as the
  native binary does.
- wrote: nothing; I stopped piping interpreter output into `head`. It prints
  `thread '<unnamed>' panicked at ... failed printing to stdout: Broken pipe`
  and two stack backtraces after the output.
- why it matters: `| head` is the ordinary way to look at the top of a long
  report, and the backtrace is longer than what `head` showed.

### F20: a test constant silently replaces a function of the same name
- kind: engine-bug
- severity: major
- where: `vm/vm_test.kso` (constant `added`, against `added` in
  `vm/keypad.kso`), `asm/asm_test.kso` (constant `reserved`, against
  `reserved` in `asm/asm.kso`); `bugs/constant_hides_function/`
- wanted: either both names to work or a compile error, as F10 gives for a
  parameter.
- wrote: renamed the constants (`summed`, `reserved_error`). With the clash,
  `kanso check` and `kanso test` accepted the module, and six unrelated tests
  failed at run time with ``error[runtime]: `vm/machine 0 [0 0 0 ...` is not
  callable``, followed by the whole 4 KB machine printed on one line, at
  `--> vm:50:3`, a module and a line number with no file. The call
  `added acc n` in the keypad parser had reached the test's constant.
- why it matters: a module can hold a function and a constant with one name
  and nothing says so until the function is called. The location named no
  file, and line 50 held code in four of the module's nine files.

### F21: an import in a test file changes which `skipped` another file calls
- kind: engine-bug
- severity: major
- where: `vm/execute.kso` (`skipped`, now `skip_when`), `vm/vm_test.kso`
  (`import "std/list"`); `bugs/test_import_renames/`
- wanted: `fn skipped m true` / `fn skipped m false` in `vm/execute.kso`,
  which does not import `std/list`, to stay mine when a test file of the
  same module imports `std/list` (which exports a type `skipped`).
- wrote: `skip_when`. Under `kanso test`, the two key-skip tests failed with
  ``error[runtime]: `list/skipped` has no field `memory` ``. The program
  itself ran correctly on all three engines; only the test build changed.
- why it matters: the import rule says a module's files share declarations
  and not imports, and this is an import crossing files anyway, in one
  direction only, and only under test. It is the third way in this port that
  a name defined somewhere else replaced one of mine (F1, F3, F20).

### F22: parentheses that would help a reader are refused
- kind: aesthetics
- severity: nit
- where: first draft of `isa/parse.kso` (`not (known? m)`), scratch bit tests
  (`(0 - 1) & 255`, `scratch/t17.kso`)
- wanted: `bits/shr op 8 & 15` written as `(bits/shr op 8) & 15`, and
  `not (known? m)`, the way C and Go programmers write bit masks.
- wrote: the unparenthesised forms; both parenthesised versions are
  `error[formatting]: these parentheses group nothing`.
- why it matters: this is aesthetic. In bit-twiddling code the precedence of
  `&` against `+` and against application is exactly what a reader is unsure
  of (here `2 + 3 & 4` is `(2 + 3) & 4`, as in C, not as in Go), and the
  parentheses are documentation. The rule removes them.

### F23: overload order is ranked position by position, so a usage error needed its own function
- kind: confusing-semantics
- severity: minor
- where: `cli/cli.kso` (`chosen`, `command`)
- wanted: one group, dispatching on the command word and on whether the
  options parsed:
  ```
  fn command _ (usage_error reason)
    failed 2 reason

  fn command "asm" opts
    ...
  ```
- wrote: a second function, `chosen`, whose only job is to split
  `usage_error` from parsed options before `command` sees the word. The
  original order is `error[formatting]: overloads of `command` appear
  most-specific first`, and the reverse order (checked in `scratch/t16.kso`)
  is `error[dispatch]: these `command` arms tie`, because `command "asm"
  (usage_error r)` would match both arms. Settling it in one group would
  take an arm `command "asm" (usage_error r)` for every command word.
- why it matters: the most natural reading of "most specific first" is per
  arm, and the rule is per position, left to right. The fix is a level of
  indirection whose name explains nothing.

### F24: calling `profile "modern"` inline is refused, naming it first is fine
- kind: confusing-semantics
- severity: minor
- where: `vm/vm_test.kso` (`vip`, `modern`)
- wanted: `ran program cycles (profile "modern")`
- wrote: `modern = profile "modern"` at the top, then `ran program cycles
  modern`. The inline call is `error[exhaustive]: this can be a none and
  `ran` has no arm for it`; the bound name passes, though it holds the same
  value. Chapter 04 explains why ("the checker reads calls, not the names
  they are bound to").
- why it matters: extracting a variable is supposed to be a no-op, and here
  it turns an error into a program. The check protects against nothing once
  a name is in between.

### F25: a keyed read naming every field reports each field as an unknown name
- kind: diagnostic
- severity: minor
- where: `cli/options.kso` (the catch-all `flag` arm)
- wanted: `{ cycles files list per_frame quirks script seed trace } = opts`
  in the one arm that needs all eight fields.
- wrote: the positional form `options cycles files list ... = opts`. The
  keyed read of all eight is refused, which the book explains (a keyed read
  must omit a field), but the errors are eight `error[name]: unknown name
  `files``-style messages at the uses, not one message at the read. Writing
  `_` in the positional form for a field I did not need was refused with a
  clear message that points at the keyed read.
- why it matters: the positional form needs every field and refuses `_`;
  the keyed form needs at least one field missing. Code that wants seven of
  eight fields and code that wants all eight look different, so a one-field
  change to a function can force a rewrite of its destructuring line.

### F26: `kanso run file.kso` will not run a file with definitions in it
- kind: tooling
- severity: nit
- where: every scratch experiment (`scratch/t*.kso`)
- wanted: `kanso run t4.kso` on a file holding a `fn` and a `print`, as the
  book's samples do.
- wrote: `kanso play t4.kso`, after `kanso run` answered "`t4.kso` is a
  library — nothing to run". `play` is in `kanso help` but not in the brief's
  list of verbs or in chapter 01.
- why it matters: small, but it is the first thing a newcomer types.

## What worked well

**An instruction is a value, and the switch is dispatch.** `isa/decode`
answers one of 36 record types, and the emulator's core is one function with
an arm per instruction:

```
pub fn execute (add_reg x y) m _
  sum = reg m.v x + reg m.v y
  flagged m x (sum & 255) (if (255 < sum) 1 0)

pub fn execute (draw n x y) m _
  ...
```

In C this is a `switch` on nibbles with the operand extraction repeated in
every case. Here the operands are named once, in the type, and `encode`,
`spell` (the disassembler) and `execute` are three arm groups over the same
36 types. The decoder itself is literal dispatch on the top nibble, which
reads like the opcode table in the reference: `fn family 13 op` builds a
`draw`.

**Multi-position dispatch made the assembler's operand grammar a table.**
`isa/parse.kso` is mostly lines like

```
fn form2 "LD" (vreg x) delay_reg
  get_delay x

fn form2 "LD" index_mem (vreg x)
  store x
```

with a literal mnemonic, typed operand shapes, and even a literal inside a
pattern (`fn form2 "JP" (vreg 0) (value a)` for `JP V0, addr`). Each arm is
one row of Cowgod's table. Adding an instruction form is adding an arm.

**Structural equality makes tests short.** `decode 56533 == draw 5 12 13`,
`script_events "1 press 3" == bad_script "line 1: unknown action `press`"`
and `bytes_of "JP end\nCLS\nend: RET" == [18 4 0 224 0 238]` are whole tests.
There are 95 of them, and none needed a helper to compare records or lists.

**The three engines agreed.** Apart from F18, every fixture produced the
same bytes on the interpreter, the dev binary and the release binary on the
first run, including the bit-packed display (64-bit rows whose top bit makes
them negative), the LCG behind `CXNN`, and 20,000-cycle runs. The run loop
is tail recursion 200,000 deep with no special handling. The native release
binary runs about 460,000 CHIP-8 instructions a second; the interpreter about
19,000, which is still fast enough for every fixture.

**Failures as values fit an emulator well.** A ROM that runs off the stack,
an opcode that means nothing, a key script with a typo, and a source file
with ten mistakes are all ordinary values (`fault`, `bad_script`,
`rejected`, `load_failed`) that the CLI dispatches on to print one message
and exit with a status. `os/read_file` answering `file_not_found` as data
made "no such file" an arm rather than a handler. The assembler collects
every error in one pass instead of stopping at the first, because an error
is just another element of the accumulator.

**Bitwise operators on 64-bit ints.** `&`, `|`, `bits/xor`, `bits/shl` and
`bits/shr` do what a C programmer expects at the 64-bit edge (shifting into
bit 63 gives a negative number; `shr` keeps the sign), and appendix B and
`lib/bits` say so. Packing each screen row into one int made `DXYN` a shift,
an AND for the collision flag, and an XOR.

**The formatting errors are exact.** Every one of the dozens of 80-column,
blank-line and ordering errors I hit named the line, the column and the
rule, so fixing them was mechanical even when I disagreed with the rule.

## Summary

The five I would fix first:

1. **F20 and F21: a name from another file silently replaces mine.** A test
   constant replaced a function, and a test file's import replaced another,
   with `kanso check` saying ok both times and the failures surfacing as
   runtime errors far away. Together with F1, F3 and F10 this is the
   biggest cost of one namespace per module: five separate times a name I
   chose collided with a name I could not see from where I was working.
2. **F9: no record update.** The machine has eleven fields and every
   instruction changes one or two. Twelve full-rebuild setters, an
   eight-field rebuild per command-line flag, and a design that keeps
   fields out of records to avoid more of them.
3. **F2: no hex literals.** An emulator is written in hex. Every opcode
   constant in this port is a decimal number with the hex in a comment, and
   the tests carry their programs as hex strings parsed at run time.
4. **F17: runtime errors with no location.** Three different runtime
   failures (`join takes a list of strings`, `comparison requires two
   values of one comparable type`, `concat takes two lists`) printed no
   file and no line, and the test runner's locations name a module, not a
   file (F6, F20). Each one cost a bisection.
5. **F11: `text/slice` past the end answers empty.** The emulator's first
   run lost a register on every write and printed a blank screen. Clamping
   the end is what nearly every caller wants; an empty answer is silent data
   loss.

Close behind: F14 (a native map is unusable as memory, so writes copy 4 KB)
and F15 (an assembler that cannot write its output file).

Writing the emulator itself was pleasant. The instruction set maps onto
types and dispatch so directly that `vm/execute.kso` reads like the
reference manual, and once the program compiled it was usually right. Most
of the bugs left after that were in my test ROMs (an `LD V5, ST`, which
CHIP-8 does not have; a comment listing two expected results in the wrong
order) or at the seams listed above. The friction was
almost all at the edges of the language rather than its center: names
colliding across files, the 80-column limit meeting a language with no
multi-line literals and no way to wrap a function header, and the missing
pieces of std/text. The other recurring cost was the checker's treatment of
none: maybe-none values needed extra arms, tie-breaking arms, or a name
bound in between before a call would compile, and which of those worked
depended on where the none came from rather than on what the code meant.
