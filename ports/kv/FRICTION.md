# FRICTION: porting bitcask to kanso

Journal kept while writing the port. Every entry quotes code that was
compiled; file and line references are to this directory.

## Entries

### F1: no way to write a byte above 0x7f to a file
- kind: stdlib-gap
- severity: blocker
- where: bitcask/disk.kso (the whole module exists because of this)
- wanted: Go's `os.WriteFile(path, []byte{0, 1, 0xff}, 0644)`, Rust's
  `fs::write(path, &[0u8, 1, 255])`. kanso already has the reading half,
  `os/read_bytes`, which answers a list of ints.
- wrote: first the obvious attempt, which compiles and runs:

      fn encode cs
        text/join (list/to_list (list/map cs text/from_code)) ""

      os/write_file "out.bin" (encode [0 1 127 128 200 255])
        .> (_ -> os/read_bytes! "out.bin")
        .> (b -> print "{b}")

  It prints `[0 1 127 194 128 195 136 195 191]` on both engines: a string
  holds code points, and `write_file` writes them as UTF-8, so 0x80 becomes
  two bytes. Nothing in std can put a lone 0x80 on disk. The port shells out
  instead, building a `printf` format of octal escapes:

      script = "printf \"$1\" >> \"$2\""
      os/run "sh" ["-c" script "sh" (octal bytes) path]

- why it matters: bitcask's on-disk format is binary (a CRC-32, a
  timestamp and two lengths per record), and so is every other storage
  format, image, archive or network capture. The asymmetry is the worst
  part: `read_bytes` exists and was ruled on, so a programmer reasonably
  assumes the write half exists too and goes looking for it. The
  workaround costs a process per put: a script of 2,000 puts takes 11.7
  seconds natively, nearly all of it starting `sh`.

### F2: no append, no remove, no rename
- kind: stdlib-gap
- severity: major
- where: bitcask/disk.kso
- wanted: `os.OpenFile(path, O_APPEND|O_CREATE|O_WRONLY)` and `os.Remove`,
  `os.Rename` in Go; `OpenOptions::new().append(true)`, `fs::remove_file`
  and `fs::rename` in Rust.
- wrote: `>>` inside the same `sh -c` call as F1, and `os/run "rm" [...]`
  for the files a merge retires. `std/os` has `write_file` (replace),
  `make_dir`, `list_dir`, `exists`, `is_dir`, and nothing that changes or
  removes an existing file.
- why it matters: an append-only log is the textbook use of append, and
  without it every put would have to read and rewrite the whole active
  file, which makes a put O(file size). Merge has to delete the files it
  replaced, and a store that cannot delete them grows without bound. A
  write-then-rename is also the usual way to make a file replacement atomic,
  and there is no rename to do it with.

### F3: the book's `pub play` samples do not run the way the book shows
- kind: tooling
- severity: minor
- where: /home/user/kanso/docs/book/samples/ch05/fused.kso
- wanted: the book prints `kanso run fused.kso` followed by its output.
- wrote: running that exact file with that exact command answers

      error: `fused.kso` is a library — nothing to run. it exports `play`:
      import the module from an entry file, or give the module a main.kso entry

  and `kanso play` on a file with `pub play = ...` answers
  `error[syntax]: `pub play` is a library's export ... `kanso play` takes
  bare statements`. So of the two spellings the book uses, `pub play =` runs
  under neither verb, and bare statements need `kanso play`, not `kanso run`.
- why it matters: the first program a newcomer copies out of chapter 05
  does not run, and the two error messages point in different directions.

### F4: a top-level binding in a `kanso play` file is not visible to its functions
- kind: confusing-semantics
- severity: minor
- where: scratch prototype of bitcask/crc.kso
- wanted: the same file to mean the same thing under `kanso play` and as a
  module file:

      table = list/naturals
        . list/take 256
        . list/map (n -> crc_byte (n - 1) 8)
        . list/to_list

      fn crc_step c b
        bits/xor table[(bits/xor c b & 255) + 1] (bits/shr c 8)

- wrote: moved it into a module directory, where `table` is a constant.
  Under `kanso play` the checker reports both `unused binding `table`` and
  `unknown name `table`` for the same name, which reads as a contradiction.
  A play file also cannot import a local module (`error[import]: a play
  file imports the stdlib and nothing else`), so trying one helper from the
  port in isolation took a directory with its own `main.kso` that imports
  it, because a `main.kso` inside the module does not see the module's
  names either.
- why it matters: `kanso play` is sold as the scratchpad, and the
  scratchpad gives a different answer from the program for the same text.

### F5: a lookup the program knows is in range still needs a `none` arm
- kind: missing-feature
- severity: minor
- where: bitcask/crc.kso:24
- wanted: `bits/xor table[(bits/xor c b & 255) + 1] (bits/shr c 8)`, where
  the index is masked to 1..256 and the table has 256 entries.
- wrote:

      fn crc_step c b
        mix table[(bits/xor c b & 255) + 1] (bits/shr c 8)

      fn mix none _
        err "crc table has no entry"

      fn mix t rest
        bits/xor t rest

  The checker's message was good (`this can be a none and `bits/xor` has no
  arm for it`). The strict form `table[i]!` is not an option in pure code,
  because it answers an effect.
- why it matters: table-driven code (CRCs, Huffman tables, state machines)
  is full of indexes that are in range by construction. Each one costs a
  helper function and an unreachable arm. A pure "I promise this is here"
  index, failing as an err rather than boxing the answer, would cover it.

### F6: `entry` is taken
- kind: aesthetics
- severity: nit
- where: bitcask/crc.kso:27, bitcask/format.kso:120
- wanted: `fn mix entry rest` and `entry = hint key ...`
- wrote: `fn mix t rest` and `found = hint ...`, after
  `error[name]: `entry` is already a declaration; rename the binding`.
- why it matters: `entry` is the ambient record that `entries` answers,
  so one of the most common nouns in a storage program is unavailable as a
  local name anywhere. It cost two edits here; in a key-value store the
  word wants to be everywhere.

### F7: `text/bytes` answers something that is not quite a list
- kind: engine-bug
- severity: major
- where: bitcask/bytes.kso (utf8_bytes); bugs/text_bytes_is_not_a_list.kso
- wanted:

      body = cat [(be32 stamp) (be16 (length kb)) vfield kb (value_bytes value)]

  where `cat` folds `text/concat` over the parts and `kb` is
  `text/bytes key`.
- wrote: every `text/bytes` goes through

      pub fn utf8_bytes s
        list/to_list (text/bytes s)

  On the interpreter `text/concat [1] (text/bytes "a")` stops with
  `error[runtime]: concat takes two lists`; natively concat works and
  `push (text/bytes "a") 2` stops with `push takes a list and a value`.
  `kanso check` passes both. The runtime error's location was `.:14:3` for
  a module directory, which named neither the file nor the line in it.
- why it matters: appendix B documents `text/bytes` as `int[]` and calls it
  "the entry point to byte-level work". Ten of twenty unit tests failed at
  once with a message about the arguments of a builtin, and finding that the
  bytes of a string were the odd value out took a bisection.

### F8: a private helper in one file breaks a parameter name in another
- kind: refactoring-hazard
- severity: major
- where: bitcask/disk.kso and bitcask/format.kso:113; cli/cli.kso and
  cli/commands.kso:83
- wanted: to add `fn at bs i` (a byte lookup) to disk.kso, and
  `fn lines xs st` (print some lines) to cli.kso.
- wrote: renamed both, after

      error[name]: `at` is already a declaration; rename the binding
        --> ./format.kso:113:18
       113 | fn hints_from bs at acc

  and the same for `lines = text/split source "\n"` in commands.kso. The
  error points at the old, untouched file, not at the new declaration that
  caused it.
- why it matters: chapter 07 sells one namespace per module as removing a
  boundary, and the price is that every top-level name, private or not,
  reserves that word as a local in every other file of the module. Adding a
  small helper is the most common edit there is, and here it can break code
  you were not looking at, with a caret in the wrong file. Short nouns
  (`at`, `lines`, `entry`, `size`, `key`) are the ones both helpers and
  locals want.

### F9: a map cannot drop a key
- kind: stdlib-gap
- severity: major
- where: bitcask/store.kso:13 (`type gone`)
- wanted: Go's `delete(keydir, key)`, or a `remove m k` beside `put m k v`.
- wrote: a marker standing where the location was, and a predicate every
  reader of the map has to go through:

      pub type gone
      ...
      pub fn current? _ none
        false

      pub fn current? st (loc _ _ _ stamp)
        st.expiry == 0 or st.now - st.expiry <= stamp

      pub fn current? _ _
        false

  The ambient map vocabulary is `put`, `entries`, `keys`, `values` and the
  index. Rebuilding a map without one key through `entries` and
  `list/to_h` is O(n) per delete.
- why it matters: the key directory of a key-value store is a map that
  loses keys, and so is a cache, a session table or a set of pending jobs.
  Here every count and listing has to skip the markers, and deleted keys
  hold memory until a merge rebuilds the directory.

### F10: `text/slice` past the end answers nothing at all
- kind: confusing-semantics
- severity: major
- where: bitcask/disk.kso:28
- wanted: Python's `bs[0:4096]` or Go's `bs[:min(4096, len(bs))]`, which
  give the whole list when it is shorter than the window.
- wrote:

      to = if (length bs < from + chunk - 1) (length bs) (from + chunk - 1)
      part = text/slice bs from to

  The first version, `text/slice bs from (from + chunk - 1)`, ran clean on
  both engines and wrote every data file with zero bytes: a 3-byte list
  sliced 1 to 4096 is `[]`. Appendix B does say "out-of-range or inverted
  bounds yield an empty result", but an end past the length is the normal
  way to ask for "up to n of these".
- why it matters: chunking, paging, truncating a preview and taking a
  prefix all overshoot the end on their last step. A store that silently
  writes nothing is the worst kind of failure for this program; it took a
  debugging session with a printf to find, because nothing failed.

### F11: a lazy `list/select` passes check and fails at run time
- kind: diagnostic
- severity: minor
- where: bitcask/store.kso:146
- wanted: `list/select (keys st.keydir) (k -> current? st st.keydir[k])`
  returned from `live_keys`, then `length (live_keys st)` at the caller.
- wrote: `list/to_list (list/select ...)`, after the binary stopped with

      error[runtime]: length takes a list, string, or map, not list/sifted
      true list/cursor 1 ["a" "b"] <fn> — a lazy sequence becomes a list
      with list/to_list

- why it matters: the message is good, but it arrives at run time for
  something the checker could see (a `sifted` handed to `length`), and only
  on the path that reaches it. In this port every `list/map` and
  `list/select` whose result leaves the function needed a `to_list`, and
  the only way to find the ones I missed was to run every command.

### F12: a pub record from another module can be read with a dot but not with a pattern
- kind: confusing-semantics
- severity: minor
- where: cli/cli.kso:152 and 161
- wanted:

      fn report_merge (bitcask/merged dropped files_in files_out kept st)

- wrote:

      fn report_merge m
        files = "{bitcask/plural m.files_in "file"} into {m.files_out}"

  after `error[opacity]: `bitcask/merged` is foreign — its structure does
  not cross an import; use its module's pub operations`. The dot reads of
  the same fields are accepted, and so is the type in an arm
  (`fn fetched st key _:bitcask/missing`).
- why it matters: the rule says structure does not cross, and then lets
  every field cross by name. Either the dot should be refused too, or the
  pattern allowed. As it is, the more readable form is the one refused.

### F13: `kanso check .` inside a module cannot resolve `../sibling`
- kind: tooling
- severity: minor
- where: cli/cli.kso:4 (`import "../bitcask"`)
- wanted: to check the module I was editing from its own directory.
- wrote: `kanso check cli` from the parent. From inside `cli/`,
  `kanso check .` says `cannot resolve import "../bitcask"`, while
  `kanso check "$(pwd)"` resolves it.
- why it matters: small, but it reads as "your import is wrong" when the
  import is right, and the natural fix (changing it to `./bitcask`) is
  wrong.

### F14: no multi-line string, and a long string cannot be wrapped
- kind: missing-feature
- severity: minor
- where: cli/cli.kso:30-38
- wanted: a usage message as one literal over three lines, or a list
  literal with one element per line:

      usage_lines = [
        "usage: kv [--max-file-size BYTES] DIR [COMMAND [ARGS...]]"
        ...
      ]

- wrote: one constant per line of text, joined by interpolation:

      usage = "{usage_line}\n{options_line}\n{commands_line}\n{script_line}"

  The bracketed attempt answers `error[syntax]: expected an expression` on
  an element line and `a top-level line must begin with fn, type, or a
  constant binding` at the `]`. An 81-character string literal has no legal
  spelling at all, so messages get shortened to fit (`--expiry SECS`,
  "bytes are not trusted").
- why it matters: every CLI has a usage block and most programs have some
  long message. With an 80-column law, the language needs a way to write a
  long string across lines.

### F15: a none from `argv[1]` was reported three modules away
- kind: diagnostic
- severity: major
- where: cli/cli.kso:40-62; reported at bitcask/merge.kso:49
- wanted: `options argv[1] default_limit (text/slice argv 2 (length argv))`,
  with an arm `fn started (options none _ _)` that handles a missing
  directory.
- wrote: a dispatch on `argv[i]` before the record is built, with a `none`
  arm that never constructs an `options`. Before that change, the checker
  reported

      error[exhaustive]: this can be a none and `bitcask/read_all` has no
      arm for it — resolve it here, or give `bitcask/read_all` a `none` arm
      (in merge.kso) (module ./cli)
        --> ./bitcask/merge.kso:49:14

  plus the same for `write_outs` and `old_paths`. The none had flowed from
  an argv index into `options.dir`, into `store.dir`, and into a private
  function of another module. The message names a private function by its
  qualified name and suggests adding a `none` arm to library code that was
  fine.
- why it matters: whole-program inference let the cause and the report sit
  in different modules, and the suggested fix is in the wrong one. The
  report should point at the place the none entered the record.

### F16: two overload arms specific in different positions
- kind: confusing-semantics
- severity: minor
- where: bitcask/store.kso:106-110
- wanted:

      fn located _ _ _:tombstone
        gone

      fn located id (record _ offset size stamp _) _
        loc id offset size stamp

- wrote: moved the dispatched argument to the front and dropped the record
  pattern for dot reads:

      fn located _:tombstone _ _
        gone

      fn located _ id r
        loc id r.offset r.size r.stamp

  The first version gets the ordering error (`overloads of `located`
  appear most-specific first`), and moving the arm only changes it to
  `these `located` arms tie: each is the more specific one somewhere`.
- why it matters: destructuring a record in an arm makes that position more
  specific even when no other arm dispatches there, so an unrelated pattern
  turns into a tie. The fix is to stop destructuring, which is the opposite
  of what chapter 03 teaches.

### F17: adding a field means editing every construction
- kind: refactoring-hazard
- severity: major
- where: bitcask/store.kso:21-29 and 142
- wanted: to add `expiry`, and later `now`, to the `store` record.
- wrote: the first time, five edits in three files: two constructions
  (`store (next_id ids) dir expiry {} limit 0`) and three positional
  patterns (`fn grown (store active dir expiry keydir limit size) n`). The
  second time, I gave up and routed every change through one function:

      pub fn reshaped (store _ dir expiry _ limit now _) active keydir size
        store active dir expiry keydir limit now size

- why it matters: construction is positional, fields are alphabetical, and
  there is no record update (`{st | size = 0}` in Elm, `st with size = 0`
  in F#). Adding a field shifts the positions of every field after it, so
  any construction with two fields of the same type can go wrong silently.
  A long-lived state record is how every effectful kanso program carries
  state between steps, so this edit is common.

### F18: `[]` is not a parameter pattern
- kind: missing-feature
- severity: minor
- where: cli/cli.kso:97
- wanted:

      fn run_words st [] stamp
        io/stdin .> (source -> run_script st (script_of source) stamp)

      fn run_words st words stamp
        one_command st (command_of words) stamp

- wrote: `return one_command st (command_of words) if length words > 0`,
  after `error[syntax]: expected a parameter pattern`.
- why it matters: a literal `0` and a literal `"put"` are patterns, and the
  empty list, the commonest base case there is, is not.

### F19: growing a list with `text/concat` is quadratic, and natively it keeps every copy
- kind: performance
- severity: major
- where: bitcask/bytes.kso (append_all); bugs/concat_accumulator_memory.kso
- wanted: `data = text/concat out.data bs` for each record a merge copies.
- wrote:

      pub fn append_all acc bs
        list/fold bs acc push

  A merge of 2,000 records took 18 seconds (6 seconds of CPU) natively and
  0.38 seconds after the change. The reduction in bugs/ builds a
  100,000-element list from 25-element chunks: 3.1 GB peak resident natively
  for n = 4,000, killed by signal 9 at n = 8,000, and 25 seconds on the
  interpreter, against 0.3 seconds for the push version.
- why it matters: `text/concat` is the documented way to append two lists,
  and appendix B says the runtime reuses a uniquely owned buffer for `push`
  and `put` without saying `concat` does not. A program that builds a file
  in memory hits this at a few thousand records.

### F20: no positional read
- kind: stdlib-gap
- severity: minor
- where: bitcask/write.kso:42
- wanted: `f.ReadAt(buf, offset)` in Go; bitcask's whole design is one seek
  and one read per get.
- wrote: `os/read_bytes! (data_path st.dir file)` and `decode_at` at the
  offset, reading the whole data file for every get. Three hundred gets
  against a 74 KB file took 0.5 seconds natively.
- why it matters: a get costs the size of the file, not of the record, so
  the key directory buys nothing on reads past a small store.

### F21: no `ends_with`, `starts_with` or `index_of` for strings
- kind: stdlib-gap
- severity: nit
- where: bitcask/store.kso:57-67
- wanted: `strings.HasSuffix(name, ".bitcask.data")` and
  `strings.Index(name, ".")`.
- wrote:

      fn kind_of? name kind
        parts = text/split name "."
        length parts == 3 and parts[2] == "bitcask" and parts[3] == kind

      fn dot_at name i
        if (text/slice name i i == ".") i (dot_at name (i + 1))

- why it matters: every program that looks at file names wants these, and
  `dot_at` loops forever on a name without a dot.

### F22: a test that does not fit on a line must grow a binding
- kind: aesthetics
- severity: nit
- where: cli/commands_test.kso:5
- wanted: `test_put_needs_value = command_of ["put" "k"] == bad_cmd "put
  takes a key and a value"` on two lines.
- wrote:

      test_put_needs_value =
        got = command_of ["put" "k"]
        got == bad_cmd "put takes a key and a value"

  `error[formatting]: a single-expression constant is written inline`
  forbids the indented form, and 82 columns forbids the inline one, so the
  only legal spelling adds a name. Aesthetic, but it comes up in every
  test file.

### F23: parentheses that help a reader are refused
- kind: aesthetics
- severity: nit
- where: bitcask/merge.kso:40
- wanted: `f < g or (f == g and o < p)`
- wrote: `f < g or f == g and o < p`, after `error[formatting]: these
  parentheses group nothing — the expression parses the same without them`.
- why it matters: the precedence of `and` over `or` is the one that readers
  most often check. Aesthetic, and a matter of taste.

## What worked well

**A `none` arm ends a loop.** Walking a list with effects between the
steps is an index and an arm for the index running off the end. The
lookup's absence is the loop condition, and the checker insists the arm
exists:

    fn load_each st ids hints i
      load_next st ids[i] ids hints i

    fn load_next st none _ _ _
      effect st

    fn load_next st id ids hints i
      load_file st id hints[id] .> (next -> load_each next ids hints (i + 1))

The same shape reads the files a merge copies, writes its output and runs a
script one command at a time. `load_file st id true` and
`load_file st id none` then choose between the hint file and the data file
on the result of a map lookup, with no `if`.

**Markers and typesets for outcomes.** A scan stops for one of three
reasons, declared as `type stop corrupt finished torn`, and every consumer
is a set of arms: `repaired` cuts the file or leaves it, `damage_note`
words the message, `push_stop` renders it in a dump, and `checked` turns it
into the reason a get failed. Adding the hint trailer's failure was one
more `bad_hint` arm. In Go this would be an error type with a kind field and
a switch in four places.

**Literal dispatch.** `fn verb_of "put" words`, `fn stored_length
4294967295`, `fn value_of 4294967295 _` and `fn plural 1 noun` say the
special case in the head of the function, where a reader looks first. The
command parser is one arm per verb and nothing else.

**A pinned clock.** `KANSO_NOW` fixes `time/now` on all three engines, so
every record's timestamp, and therefore its CRC, is byte-identical across
the interpreter and both native tiers. The expiry fixture moves the clock
by setting the variable per command. No clock had to be threaded in for
testing.

**Effects as values make the crash story readable.** The order that keeps
a merge safe (write the new files, then remove the old ones oldest first)
is three `.>` steps in `rewrite`, and nothing else in the program can
reorder them. The recovery in `repaired` reads the same way: cut the file,
then say so, then carry on with the store.

**Rescue across a module boundary.** A get that finds a damaged record
fails inside `bitcask`, and the CLI turns that into an ordinary value with
one step, `bitcask/get_value st key .? damaged`, where
`fn damaged (err reason)` builds a record the CLI prints. The library does
not need a result type for it.

**An operator arm instead of `sort_by`.** A merge copies live records in
the order they sit on disk. One arm, `fn < (held f _ o _) (held g _ p _)`,
on a private type made `list/sort` do it.

**Integers without widths.** CRC-32 arithmetic, 64-bit offsets and the
hint trailer's `9223372036854775807` needed no casts, no unsigned types and
no overflow checks. `std/bits` covered the shifts and xor.

**Pure tests of the format.** `encode_record`, `decode_at`, `scan_bytes`
and the hint codec are functions from lists of ints to lists of ints, so 31
`test_` constants pin the layout byte for byte, including torn and
corrupted input, with no files.

**Three engines agreed.** All 23 fixtures produce the same bytes on the
interpreter, the dev binary and the release binary. The only disagreement
found was F7, which is a bug rather than a difference in meaning.

## Summary

The five I would fix first:

1. F1 and F2: a `write_bytes` beside `read_bytes`, an append, and remove
   and rename. Without them a storage program shells out for every write.
2. F10: `text/slice` should clamp an end past the length. It produced the
   only silent wrong answer in the port.
3. F19: `text/concat` used as an accumulator should reuse a uniquely owned
   list the way `push` does, or at least free what it copies.
4. F17: a record update form, so adding a field to a state record is one
   edit.
5. F8: a top-level name in one file should not take the word away from
   locals in every other file, or the error should point at the new
   declaration.

F7 (the `text/bytes` value that is not a list on one engine or the other)
is a bug and will presumably be fixed regardless.

Writing bitcask in kanso split into two experiences. The pure core, the
record and hint codecs, the CRC, the scan that finds a torn tail and the
packing of a merge, was pleasant: dispatch on markers and literals carried
all the branching, the tests were plain constants, and the checker's demand
for a `none` arm caught real gaps (a file missing from the map of files
read, a key missing from the directory)
before they ran. The edge was the opposite. The one thing a storage engine
must do, put bytes on disk, is not in the library, so every write goes
through `sh -c printf`, and deletes and truncation follow it there. The
other costs were in the middle: a state record that cannot be updated in
place by name, a map that cannot lose a key, and a namespace in which a new
helper can break a file I had not opened. None of these stopped the port,
and the engines agreeing byte for byte on every fixture made it easy to
trust what was built.
