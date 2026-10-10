# FRICTION: porting puff.c (zlib's inflate) to kanso

Recorded while writing the port, in the order the problems came up. Every
code sample was compiled with `/tmp/claude-0/kanso-main/kanso`. Timings are
from this container, best of three, unless they say otherwise.

## Entries

### F1: `&` and `|` share one precedence level, so `a | b & m` means `(a | b) & m`
- kind: confusing-semantics
- severity: major
- where: `compress/sink.kso:34` (`flip`), and every bit-assembly line in
  `deflate/input.kso` and `container/container.kso`
- wanted: the C reading, where `&` binds tighter than `|`, which is also what
  zlib, puff and every reference implementation are written in:
  ```
  fn flip code n acc
    flip (bits/shr code 1) (n - 1) (bits/shl acc 1 | code & 1)
  ```
- wrote:
  ```
  fn flip code n acc
    flip (bits/shr code 1) (n - 1) (bits/shl acc 1 | (code & 1))
  ```
  The first version compiled and was wrong: it reversed every Huffman code to
  0 or 1, and the first round trip failed with a CRC mismatch in the gzip
  trailer, two modules away from the line at fault. `src/parser.rs` gives
  `&`, `|` and `^` one level (`BITS`) and folds them left to right, so
  `1 | 2 & 0` is 0 here and 1 in C. The other direction is refused: the
  C programmer's defensive `(b[1] | x) & 4095` is rejected with "these
  parentheses group nothing", which is true under kanso's rule and is
  exactly the parenthesisation a reader from C needs to see.
- why it matters: bit-level code is copied from C, Go, Rust and Java, which
  all rank `&` above `|` and `^`. A port that reads naturally compiles to a
  different number with no diagnostic. Either the C ranking, or refusing an
  unparenthesised mix of `&` with `|`/`^`, would have caught it.

### F2: no binary standard output; `io/write` takes only strings
- kind: stdlib-gap
- severity: major
- where: `cli/cli.kso` (`gzip`, `zlib`, `deflate` commands), `cli/report.kso`
  (`write_hex`); reproduction `bugs/write_takes_bytes_only_sometimes.kso`
- wanted: `inflate gzip file > file.gz`, the way gzip, zlib's minigzip and
  every compressor works:
  ```
  io/write (compress/gzip_bytes src)
  ```
- wrote: hex on stdout (`write_hex`), and an `-o FILE` option that goes
  through `os/write_file`, which turns out to accept bytes (appendix B says
  `string string -> io`). `io/write` with bytes passes `kanso check` and
  stops at run time with `error[runtime]: write takes a string`, no location.
- why it matters: a decompressor's natural output is bytes, and so is a
  compressor's. Without a byte stdout the tool cannot sit in a pipeline, and
  `inflate cat` has to refuse any output that is not UTF-8.

### F3: no binary standard input, and the engines disagree about it
- kind: engine-bug
- severity: major
- where: `bugs/stdin_bytes_not_text.kso`; the CLI reads only named files
- wanted: `cat file.gz | inflate cat -`, and an `io/stdin_bytes` beside
  `os/read_bytes` the way `os/read_bytes` sits beside `os/read_file`.
- wrote: nothing; the decoder takes a path. Natively `io/stdin` hands over a
  string holding the raw bytes (`text/bytes` gives `[31 139 255]`); the
  interpreter refuses the same input with "stream did not contain valid
  UTF-8".
- why it matters: stdin is where compressed data usually arrives. The native
  behaviour would make the feature work on two engines out of three, which
  is worse than not at all.

### F4: every byte read answers `int | none`, so each module grows a `byte_at`
- kind: aesthetics
- severity: minor
- where: `deflate/input.kso:22`, `container/container.kso:41`,
  `cli/report.kso:35`
- wanted: an index I have already bounded, read as an int:
  ```
  word = src[k] | bits/shl src[k + 1] 8
  ```
- wrote:
  ```
  pub fn byte_at src k
    zero_past_end src[k]

  fn zero_past_end none
    0

  fn zero_past_end v
    v
  ```
  three times, because a helper that is private to one module cannot be
  reached from the next, and `pub` on a helper named `byte_at` puts it in
  every importer's short-name space. `bits/shl src[k + 1] 8` is refused with
  "this can be a none and `bits/shl` has no arm for it", and `xs[i]!` is an
  effect, so it cannot be used in pure code at all.
- why it matters: the reader reads past the end on purpose (DEFLATE's final
  bits are padded and puff's `bits()` reads ahead), so here none-as-zero is
  correct. For the tables, though, a miss is a bug, and mapping it to zero
  hides it. A pure "this index is in range, fail if not" read would let the
  two cases be written differently.

### F5: a binding pattern or field read on an err stops the program, and natively it reports a stack overflow
- kind: engine-bug
- severity: major
- where: every decoder step in `deflate/inflate.kso` (`next_block` line 53,
  `noted` 71, `dynamic_codes` 106, `length_step` 150-161,
  `literal_or_match` 181-184, `distance` 195); reproduction
  `bugs/destructured_err_overflows_stack.kso`
- wanted: the railway the book describes, with the decoder's result taken
  apart where it arrives:
  ```
  fn codes src bp out lens dists
    g = decode src bp lens
    got after symbol = g
    literal_or_match src after out lens dists symbol
  ```
- wrote: every fallible answer is handed to a function whose head takes it
  apart, because a call passes an err through and a binding does not:
  ```
  fn codes src bp out lens dists
    g = decode src bp lens
    literal_or_match src g out lens dists

  fn literal_or_match _ (got bp 256) out _ _
    block_end bp out ""

  fn literal_or_match src (got bp sym) out lens dists
  ```
  On the first corrupt fixture the original form failed on the interpreter
  with "cannot destructure err deflate/corrupt 0 "invalid block type 3" as
  `deflate/block_end`", and on the native engines with "the program ran out
  of stack: recursion went deeper than the stack holds". `r.data` on an err
  fails the same way ("cannot read fields of err"). `kanso check` passed all
  of it.
- why it matters: the shape that works is fine once known, and it reads
  well. But nothing tells you it is required: the checker is silent, the
  book's chapter 4 shows errs riding past calls and never mentions bindings,
  and the native message sends you looking for recursion that is not there.
  Twenty corrupt inputs out of twenty-four crashed this way before the
  rewrite.

### F6: a keyed read placed after a `return ... if` binds nothing
- kind: engine-bug
- severity: minor
- where: `deflate/inflate.kso` (`dynamic`, first draft);
  `bugs/keyed_read_after_return.kso`
- wanted:
  ```
  fn show r
    return "neg" if r.after < 0
    { after data:lens } = r
    "{after} {lens}"
  ```
- wrote: dot reads (`r.after`, `r.data`), later replaced by the F5 shape.
  The keyed read reports `unknown name` for both names it should bind; the
  same read with the `return` line removed works, and a positional read
  after the `return` works.
- why it matters: the error points at the uses, not at the read, so it looks
  like a typo in the field list.

### F7: a map that is read and then written is copied on every write
- kind: performance
- severity: blocker (for the encoder as first written)
- where: `compress/lz77.kso`; the first version is kept at
  `scratch/lz77_with_map.kso.txt`, the measurement at
  `scratch/map_read_then_put/`
- wanted: zlib's hash chains, a head table updated at each position:
  ```
  fn scan src i n out ch
    key = trigram src i
    first = ch.heads[key]
    best = longest src i n ch.links first (found 0 0) chain_limit
    filed = file ch key i (or_zero first)
    take src i n out filed best

  fn file (chains heads links) key i prior
    chains (put heads key i) (push links prior)
  ```
- wrote: no map at all. The links are built up front by sorting
  `trigram * 2^32 + position`, taking each key's sorted predecessor as its
  link, and sorting the pairs back into position order (`links_of`).
- why it matters: the map version compressed 2,500 bytes in 1.2 s, 10,000 in
  19 s and 20,000 in 70 s with `build --release`; 50 KB had not finished
  after five minutes. The micro-benchmark isolates it: putting n keys into a
  map is linear (40,000 puts in 0.045 s), and reading the previous key before
  each put is quadratic (10,000 in 0.42 s, 40,000 in 4.8 s natively; 3.3 s
  and 55 s on the interpreter). Reading a map before writing it is what a
  hash table is for. The sorted rewrite compresses 350 KB in 1.5 s.

### F8: no array that can be written at an index
- kind: missing-feature
- severity: major
- where: `compress/lz77.kso` (`links_of`), `deflate/huffman.kso`
  (`construct`), `deflate/inflate.kso` (`sent_length`)
- wanted: the three places puff and zlib fill a table out of order:
  `lengths[order[index]] = bits(s, 3)` for the code-length code,
  `h->symbol[offs[length[symbol]]++] = symbol` for the canonical code, and
  `prev[pos] = head[hash]; head[hash] = pos` for the matcher.
- wrote: for the first, an inverse permutation (`order_position`) so each
  symbol can look up where its length was sent; for the second, a sort by
  `length * 1024 + symbol`; for the third, the double sort of F7. All three
  work, and the first two read better than the C (see "What worked well").
- why it matters: the workarounds each needed a different idea, and only the
  matcher's needed it for speed. A program that genuinely needs a mutable
  table (a sliding-window matcher that does not hold the whole input, a
  decoder table rebuilt per block) has nowhere to go.

### F9: `list/take` and `list/map` answer lazy sequences, and the mistake surfaces at run time with no location
- kind: diagnostic
- severity: minor
- where: `deflate/tables.kso` (`length_base`), `cli/report.kso` (`info`)
- wanted:
  ```
  pub length_base = push (bases_from 3 (list/take length_extra 28)) 258
  ...
  fields = list/map u.fields (f -> row f.label f.value)
  r1 = push (text/concat head fields) (row "compressed" "{length src} bytes")
  ```
- wrote: `text/slice length_extra 1 28` for the first and
  `list/to_list (list/map ...)` for the second. The errors were
  `error[runtime]: length takes a list, string, or map, not list/capped 28
  list/cursor 1 [...]` and `error[runtime]: concat takes two lists`, both
  without a file or line, both after `kanso check` said ok.
- why it matters: whether a value is a list or a sequence is invisible at the
  call site. The first message at least names the fix; the second names
  nothing.

### F10: a type name from an import, or from std/list, takes over a function call
- kind: confusing-semantics
- severity: minor
- where: `cli/report.kso` (`again`, first written `repeated`), `cli/cli.kso`
  (`decoded`, first written `unpacked`)
- wanted:
  ```
  fn repeated src n total
    u = container/unpack src
    repeated src (n - 1) (total + length u.data)
  ```
- wrote: `again`. std/list exports `pub type repeated`, and the call was
  refused with "`repeated` has 1 field(s), got 3 (construction is
  positional, fields alphabetical)". The same happened with my own
  `container/unpacked` type and a CLI helper named `unpacked`.
- why it matters: the message describes a record I never declared in this
  file, and the cue that the name came from an import is missing.

### F11: no hex integer literals
- kind: missing-feature
- severity: minor
- where: `checksum/crc32.kso:10-13`, `compress/lz77.kso:35`,
  `compress/fixed.kso` (`lit_code`)
- wanted: `poly = 0xEDB88320`, `ones = 0xFFFFFFFF`, `fixed_code (0x30 + sym)
  8`, which is how RFC 1951 and zlib state these constants.
- wrote: `poly = 3988292384` with the hex in a comment above it, and
  `48 + sym`, `400 + sym - 144`, `192 + sym - 280` for the fixed code's
  ranges.
- why it matters: every constant has to be checked twice by hand, and the
  RFC's own table (`00110000` through `10111111`) no longer appears in the
  code that implements it.

### F12: eighty columns and `return value if condition` push the condition off the line
- kind: aesthetics
- severity: minor
- where: `deflate/inflate.kso` (`stored`, `dynamic`), `container/zlib.kso`,
  `container/gzip.kso`
- wanted:
  ```
  return broken bp "stored length does not match its complement" if nlen != bits/xor len 65535
  ```
- wrote:
  ```
  why = "stored length does not match its complement"
  return broken bp why if nlen != bits/xor len 65535
  ```
  Nine guards in the decoder and wrappers needed a throwaway name to fit
  (`why`, `too_many`, `too_wide`, `why_size`, `junk`, `finished`, `here`),
  and two test constants needed a binding only because a constant that is
  one expression must be written on one line.
- why it matters: a guard is the line a reader scans for its condition, and
  the condition is the part that gets pushed right. There is no
  continuation form for a `return ... if`.

### F13: no string or list literal may span lines, so a help text is fourteen effects
- kind: missing-feature
- severity: minor
- where: `cli/cli.kso` (`usage`)
- wanted: the usage text as one literal.
- wrote: one `io/write_err` per line, fourteen of them, chained with `.>`.
  The help lines are also capped at about 55 characters, because each sits
  inside `.> (_ -> io/write_err "...")` on an 80-column line. The multi-line
  literal produced an "unterminated string" error followed by a dozen
  formatting errors about the words inside it ("identifiers are snake_case",
  "kanso has no commas").
- why it matters: the follow-on errors read the help text as code; only the
  first line of the report is about the actual problem.

### F14: `_` does not match none, so every catch-all is written twice
- kind: confusing-semantics
- severity: minor
- where: `cli/cli.kso` (`format_flag`, `command`, `output_of` at 30-39)
- wanted: `fn command _ _` to answer every other command line, including one
  with no command, and for `-o NAME`:
  ```
  fn output_of "-o" name
    name

  fn output_of _ _
    none
  ```
- wrote: a `none` arm above each `_` arm with the same body, and four arms
  for `output_of` (`"-o" none`, `"-o" name`, `_ none`, `_ _`), because
  `argv[4 + shift]` can be none and `name` does not take it.
- why it matters: the reason is principled (none should be handled on
  purpose), but in a dispatcher over optional arguments the extra arms are
  always identical to the catch-all, and they double with each optional
  position.

### F15: an effect handed to a function that wants a value passes the checker
- kind: diagnostic
- severity: minor
- where: the first `main.kso`
- wanted (and wrote by mistake):
  ```
  os/args .> (a -> os/read_bytes! a[1]!)
  ```
- wrote: `os/args .> (a -> a[1]! .> os/read_bytes!)`. The first form passed
  `kanso check` and failed at run time with `read_bytes takes a path string`.
  Chapter 5 promises that the checker refuses a box handed to something that
  wants a value; it does for `length`, not for this.
- why it matters: small, but it is the one promise about effects that the
  book makes most loudly.

### F16: the checker reads calls, not names, so naming a value changes what compiles
- kind: confusing-semantics
- severity: minor
- where: `compress/lz77.kso` (first version, `file_range`)
- wanted:
  ```
  file_range src (from + 1) to n (file ch key from ch.heads[key])
  ```
  which was refused with "this can be a none and `file` has no arm for it".
- wrote: `prior = or_zero ch.heads[key]` and passed `prior`. The very same
  lookup one function up, bound to a name first (`first = ch.heads[key]`)
  and passed to the same `file`, was accepted without a none arm.
- why it matters: the book documents this for errs (chapter 4: "the checker
  reads calls, not the names they are bound to"), but in practice it means
  extracting a local is not a safe refactor, and inlining one can break the
  build.

### F17: names the program may not use: `done`, `put`, and a parameter named like a type in another file
- kind: refactoring-hazard
- severity: nit
- where: `container/gzip.kso` (`finished`), `compress/sink.kso` (`emit`)
- wanted: a binding named `done`, a function named `put` (the bit writer's
  verb in zlib is `put_bits`), and a parameter named `code` in `sink.kso`.
- wrote: `finished`, `emit`, and a type renamed from `code` to `huff` in
  `fixed.kso`, because a type declared in one file of a module forbids that
  name as a parameter in every other file. Later, adding `fn packed` to
  `cli/cli.kso` broke two functions in `cli/roundtrip.kso` that had a
  parameter named `packed` ("`packed` is already a declaration; rename the
  binding"), and the table builder in `deflate/huffman.kso` could not call
  its locals `keys`, `entries` or `entry`, which are ambient.
- why it matters: each costs a rename and a moment of confusion. The
  cross-file case makes adding a function in one file a breaking change for
  its siblings, reported in the sibling.

### F18: catching a foreign err by type needs the qualified name, and the book says the bare one works
- kind: confusing-semantics
- severity: nit
- where: `cli/cli.kso` (`unpacked_or_fail`)
- wanted: `fn unpacked_or_fail r (err e:corrupt) _`, since chapter 7 says an
  import's pub names join the short-name space.
- wrote: `(err e:deflate/corrupt)` and `(err e:container/malformed)`. The
  bare forms are refused with "no type is called `corrupt`, so this arm can
  never match", and with only the bare form present the import of
  `../deflate` is reported as unused.
- why it matters: the qualified form is fine and arguably better. The
  diagnostic could say "did you mean `deflate/corrupt`".

### F19: an entry file holds no functions, and `kanso check .` inside a module cannot see its siblings
- kind: tooling
- severity: nit
- where: `main.kso`; any `kanso check .` run from inside `container/`
- wanted: a small experiment in `main.kso` with a helper function in it, and
  `kanso check .` from the directory I am editing.
- wrote: helpers moved into a module; checks run as `kanso check container`
  from the project root. From inside the directory, `import "../checksum"`
  fails with "cannot resolve import", although the same file resolves it when
  checked from one level up. The bug reproductions in `bugs/` are single
  files for `kanso play`, and `kanso check` refuses every one of them ("a
  file with declarations is a library, and a library has no statements to
  run"), so a play file can be run but not checked.
- why it matters: the import that fails is fine, and the message says it
  does not exist.

### F20: pushing onto a list held in a record is quadratic on the interpreter only
- kind: performance
- severity: major
- where: `compress/lz77.kso:38` (`links_of`); measured with
  `scratch/map_read_then_put` (`recpush`, `foldpush`)
- wanted: one fold that carries the previous key and the growing list:
  ```
  type link_state
    last
    links

  fn link_step (link_state last links) key
    pos = key % shift
    prior = if (key / shift == last / shift) (last % shift) 0
    link_state key (push links (pos * shift + prior))
  ```
- wrote: a `list/map` over positions that reads the sorted list at `i` and
  `i - 1` (`link_at`), so no record holds the list.
- why it matters: the fold version compressed 5, 10 and 20 KB of noise in
  1.3, 2.9 and 8.4 s on the interpreter, and a 70 KB round trip took 217 s;
  natively it was linear. In isolation, pushing n items onto a list carried
  in a two-field record takes 1.5 s for 10,000, 5.4 s for 20,000 and 22 s for
  40,000 on `--interp`, against 0.025 s for 160,000 natively; the same
  pushes in a plain `list/fold` are linear on both. The rewrite made the
  interpreter linear (1.05, 2.08, 3.84 s). A state record is the book's own
  advice for a fold that carries two things, so the trap is set by the
  idiom.

### F21: `/` and `%` are calls into the runtime; shifts and masks are inline
- kind: performance
- severity: minor
- where: `deflate/huffman.kso:126` (`decode`), `deflate/input.kso:13`
  (`peek`)
- wanted: the table entry unpacked the obvious way:
  ```
  got (bp + e % 16) (e / 16)
  ```
- wrote: `got (bp + (e & 15)) (bits/shr e 4)`. Under callgrind the release
  build spent 7.65 million of 79.2 million instructions in `k_div` and
  `k_mod` for one 350 KB decode. Changing those two operators and trimming
  `peek` from four byte reads to three took the decode from 79.2 to 64.7
  million instructions and from 18.6 to 11.0 ms. I changed both in one step,
  so the split between them is not measured; `k_div` and `k_mod` disappeared
  from the profile.
- why it matters: on a constant power of two the division is a shift in
  every C compiler. Here the reader has to know that `bits/shr` is the fast
  spelling.

### F22: two results means a record per call, and a record per decoded symbol
- kind: performance
- severity: minor
- where: `deflate/huffman.kso` (`got`), `deflate/inflate.kso` (every
  `(got bp sym)` arm)
- wanted: puff's decode returns the symbol and advances a bit buffer held in
  a state struct. In Go or Rust it would return `(symbol, bits_used)`.
- wrote: `pub type got` with `after` and `symbol`, built for every symbol and
  taken apart in the head of the next function (F5). `k_rec`, the record
  constructor, is 7.2% of the release decode's instructions (4.66 million
  for 350 KB). Packing both into one int (`symbol * 16 + length`, as the
  fast table already does internally) would avoid it, at the cost of
  readability.
- why it matters: this is the measured price of the idiom. The decode still
  runs at about 25 MB/s, so I kept the record.

### F23: a closed stdout pipe panics the interpreter with a Rust backtrace
- kind: engine-bug
- severity: minor
- where: the first `hex_noise_head` fixture, `inflate hex FILE | head -4`
- wanted: `head` to close the pipe and the program to stop quietly, as `cat`
  does.
- wrote: the fixture writes to a file and runs `head` on that. The
  interpreter printed `thread '<unnamed>' panicked ... failed printing to
  stdout: Broken pipe` and two stack traces, exit 101, and the trace's
  interleaving varied between runs, which made the fixture's expected
  output impossible to pin. Native prints `error[runtime]: the program was
  ended by signal 13`, exit 1.
- why it matters: piping into `head` is the most ordinary thing to do with a
  decompressor's output.

### F24: `_` is refused in a binding pattern and accepted in a parameter pattern
- kind: aesthetics
- severity: nit
- where: `deflate/inflate.kso:71` (`noted`), `deflate/deflate_test.kso`
- wanted: `huffman counts _ = fixed_lit` in a test, the same shape as the
  arm `fn noted (block_end after data _) note` two files over.
- wrote: `counts = fixed_lit.counts`. The binding form answers "`_` does
  not appear in binding patterns; omit fields with a keyed read", and the
  keyed read is the one F6 breaks after a `return` line.
- why it matters: one shape, two rules, depending on whether the pattern
  sits left of `=` or inside a function head.

## Throughput

The 350,428-byte text in `fixtures/data/big.txt.gz` (95,965 bytes, written by
`gzip -9`, two dynamic blocks). Best of five on a 4-core container whose load
average was about 10 from the other ports running beside this one, so the
wall times are noisy; the callgrind instruction counts are exact.

| decode, 350 KB out         | interpreter | dev binary | release binary |
|----------------------------|-------------|------------|----------------|
| raw DEFLATE, per run       | 1.75 s      | 37 ms      | 11-15 ms       |
| gzip incl. CRC-32, per run | 1.9 s       | 60 ms      | 16 ms          |
| throughput, raw            | 0.2 MB/s    | 9.5 MB/s   | 23-32 MB/s     |
| instructions, raw          |             | 204 M      | 64.7 M         |
| instructions, gzip         |             | 328 M      | 97.7 M         |

For scale, `gzip -dc` on the same file takes 6 ms including process start.
The release decoder spends about 185 instructions per output byte, and the
CRC-32 another 94 (33 million for the file): the table-driven CRC is a
`list/fold` with a lambda, an index read with a none arm and three bit
operations per byte. Adler-32, a fold over a two-field record, took 52 ms
alone in release, more than the decode.

Compression (greedy LZ77, fixed codes, 134,872 bytes out, 38.5%; `gzip -1`
gives 121,544 in 10 ms):

| compress                   | interpreter     | dev binary | release binary |
|----------------------------|-----------------|------------|----------------|
| 350 KB text                |                 | 1.43 s     | 0.57 s         |
| 50 KB text                 | 6.5 s           |            |                |

Two findings from getting there. The lookup table in front of puff's
bit-at-a-time decode (`fast_table`, zlib's main speedup) did not pay at
first: release raw decode was 18.6 ms with puff's walk alone and 18.0 ms with
the table, because `peek` and the division calls of F21 dominated. And in the
compressor, the two sorts that replace the hash table (F7) are now the
largest cost: `list/span`, the sort's merge, is 32% of release instructions
on 50 KB.

## What worked well

**Canonical Huffman construction reads better than the C.** puff fills
`symbol[]` by counting offsets per length and writing each symbol at its
slot. Here the same table is a sort on one key:
```
fn sort_key pair
  len = nth pair 1
  sym = nth pair 2
  len * 1024 + sym - 1
```
and the symbols in code order fall out of `list/sort`. The counts are one
`list/count` per length. Nothing in it can index out of bounds.

**Dispatch on literals is a natural block and symbol decoder.** The block
type, the code-length repeat codes and the end-of-block symbol are each a
literal in a function head:
```
fn block src bp out 0
  stored src bp out

fn block src bp out 1
  e = codes src bp out fixed_lit fixed_dist
  noted e "fixed"

fn length_step src (got bp 16) ch want lens
fn length_step src (got bp 17) ch want lens
fn length_step src (got bp 18) ch want lens
fn length_step src (got bp sym) ch want lens
```
That is RFC 1951's table, laid out as the RFC lays it out. Combined with F5's
shape, the decoded record and the case it falls in are matched in one head.

**Reading past the end answers none, and none dispatches.** puff's `bits()`
has to check its input length on every refill. Here `zero_past_end` is two
arms, and the loops check `exhausted?` once per symbol. The decoder needed no
other bounds checks to be safe on the 24 corrupt inputs.

**A failure carries its position to the top without any plumbing.** Every
fault is `err (corrupt (bits/shr bp 3) reason)` where it is found, and the
CLI's one function with an `(err e:deflate/corrupt)` arm prints it. puff
returns negative codes that every caller tests and passes up; the kanso
decoder has no such line.

**Bytes accumulate in place.** `text/append out byte` and
`text/append out (text/slice out start end)` grow the output with no visible
copying, so the overlapping copy of a match is a loop of slice appends and the
decoder's output buffer is an ordinary value. The decode loop is one tail
call per symbol, and it ran over 350 KB on all three engines without a stack
problem.

**Bit operations on plain ints.** `bits/shl`, `bits/shr`, `bits/xor`, `&` and
`|` on 64-bit ints were all the CRC, the bit reader and the bit writer
needed; ints that can grow past 64 bits never got in the way, since every
value here stays below 2^56.

**Three engines agreed on every byte.** Seventy-seven fixtures, including
binary output written with `-o` and compared against `gzip -d` and Python's
zlib, produce identical output on the interpreter, the dev binary and the
release binary. The only disagreements found were the error paths of F3, F5
and F23.

**Tests as constants.** `test_inflate_overlapping_copy = ...` with a
nine-byte stream inline was quick to write, and `testing/when_failed` reads
the position and reason of a decoder failure in one line.

## Summary

The five I would fix first:

1. **F1, `&` and `|` at one precedence.** It produced a silent wrong answer
   in code transcribed from C, and the formatter refuses the parentheses a C
   programmer would add.
2. **F5, a binding or field read on an err crashes, natively as a false stack
   overflow.** It turned twenty corrupt inputs into crashes, the checker said
   nothing, and the native message points away from the cause.
3. **F7, a map read before a write is copied on every write.** It made the
   obvious LZ77 matcher quadratic on both engines (50 KB did not finish in
   five minutes) and forced a different algorithm.
4. **F2 and F3, no byte standard output or input.** A compressor that cannot
   write to a pipe, or read from one, is a demonstration rather than a tool;
   the engines also disagree about stdin.
5. **F20, pushing onto a list in a record is quadratic on the interpreter.**
   The book's idiom for a fold with state is the slow one, and only on the
   engine that runs tests and `--plan`.

Writing puff in kanso was a good fit for the parts puff gets right in C and a
poor fit for the parts that are about memory. The decoder's logic came out
shorter and more readable than the C: dispatch on literals is exactly how the
RFC states block types and repeat codes, none past the end replaces every
bounds check, and an err with a position replaces puff's negative return
codes. What took the time was everything around mutation. There is no array
to write at an index, a map cannot be read and written cheaply, a list in a
record is quadratic on one engine, and every fallible result has to be passed
through a call before it can be looked at. Each of those has a workaround,
and I found every one by running into it at run time rather than from a
diagnostic. Bit-level arithmetic was fine once I knew that `&` and `|` are
one level and that `/` costs a runtime call. The release binary decodes at
about a third of `gzip -dc`'s speed, and the interpreter about a hundred
times slower than the release binary.
