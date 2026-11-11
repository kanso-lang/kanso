# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-11-05 — a declaration cannot take a nullary value's name

A library could declare a constant named `done`. std/list's iterators answer
`done` when they run out, and with the constant declared that answer read
the constant, so a fold elsewhere in the program found no arm for it and
failed at run time (#2338). senro's jobs module found it with a SQL string
named `done`. An entry already refused the name, because there it is a
binding and the shadow check holds the nullary values as globals; a library
declares it instead, and `collect_globals` checked declarations against the
ambient names and the types only.

A declaration named `done`, `none`, `true` or `false` is now refused as a
name already taken. The one program in the tree that declared one was
`sockets_serve`'s loop fixture, with `fn done _` as a callback; it is
`reported` now. `a_constant_named_like_a_builtin_value` in the error corpus
declares `done` and `none`, and checked clean before the change.

Rows: the new check costs every compile a little. `compile_instructions`
rose 409 to 24,114,963, `entry_instructions` 1,799 to 87,962,791,
`library_instructions` 1,497 to 73,495,778 and
`browser_compile_instructions` 3,822 to 350,291,185, and
`browser_run_instructions` did not move. The rule is part of the language,
so the welfare floor moves down by exactly the check's cost.

## 2026-11-05 — directive: a model's fields come from its table

Clay: "we need the fields for the model to be auto-magic like in ruby." He
named two ways: generate the model files the way the schema is kept, or a
primitive that maps data such as a schema to code the compiler can still
check. kanso already has the second. A derivation runs during the build,
reads the app's own data and answers declarations that are checked like
code someone wrote (#2186). senro's models derivation read the migrations
and checked each hand-written record against its table, so every column was
named twice.

`derive/define` writes source into a module as one more of its files, and the
source can be a `type`. What stopped a module from using one was the first
load of the app's modules, which describes them to the derivations before
any has run. A module that annotated `p:post` and read `p.title` was refused
there, because nothing had declared `post` yet. #2265 met the same problem
for functions and set an unknown name aside in that load. The load now also
sets aside an unknown type name and a field read no record has, in the
per-file checks and in the merged check; one helper, `set_aside`, holds all
of them. The second load, with the derived source joined in, refuses
whatever is still unknown with the message it always gave.

Three cases in tests/golden/derive pin it.
`a_module_reads_a_record_its_derivation_declares` prints its bylines on both
engines, and the binary before the change refused it with "no type is called
`post`". `a_module_reads_a_field_its_derived_record_lacks` and
`a_module_names_a_record_nothing_declares` are still refused.

senro builds on it: a module named for a table gets the table's record
declared in it, with a field for each column in the table's order, and a
module that declares the record keeps its own. `kanso derived` prints the
declared type, so the generated code can be read without being kept on disk.
Files written by a generator would drift from the schema until someone ran
it again; a derivation runs on every build.

Go's embedding was raised in the same message, as a way to put validations
and helpers on top of the generated fields. A kanso record carries no
methods, and the module's functions that take the record sit beside it, so
there is nothing to delegate through. The question and the committee's
reading are on #2356.

Rows: the describe load's added set-asides cost every derive-free compile a
little, because the helper runs on each file's diagnostics.
`compile_instructions` rose 290 to 24,115,253, `entry_instructions` 2,156 to
87,964,947 and `library_instructions` 617 to 73,496,395.
`browser_compile_instructions` fell 799 to 350,296,781, by layout. The
directive is a part of the language Clay asked for, so the welfare floor
moves down by exactly that cost.

## 2026-11-05 — a field read through an annotated name answers its own type

**DONE (#2350).** Two record types in one program that both have a `body`
field share one getter group, `Get_body`, with an arm for each type. Infer
gave the group the join of its arms, so when one type's `body` could be none,
reading `body` from the other type's value was refused as possibly none, even
where the parameter was annotated with the other type. A senro model module
with a `letter` and a `note` hit it, and `size_of` in http.kso was worked
around for it in #2351.

Infer now remembers the type of a binding that carries an annotation, and a
field read whose one argument is such a name considers only the arm whose
first pattern is that type's constructor. An unannotated read still joins
every arm, which is the old answer and still a sound one.

`tests/golden/field_reads/its_own_type` reads a `letters` body that is never
none beside a `notes` body that can be, and prints `8` and `7`; the compiler
before the change refused it. `another_types_none` reads the `notes` body
through a name of that type and is still refused, at the same line.
`tests/a_field_read_answers_its_own_type.rs` runs both.

Rows: the first version stored the annotation in each environment entry,
which grew the entry from 24 bytes to 40 and raised every compile row about
0.28%. The types now sit in a side vector that stays empty in a function with
no annotated parameters. Against main, `compile_instructions` fell 4,266 to
24,110,987, `entry_instructions` 47,358 to 87,917,589,
`library_instructions` 35,277 to 73,461,118 and `emit_instructions` 174 to
5,613,223. `browser_compile_instructions` rose 270,018 to 350,566,799, and
the side vector's first push is three more front-end allocations
(`compile_allocs` 13,937 to 13,940) and two more in the interpreted run
(`interp_allocs` 810,144 to 810,146). The sum falls by about 0.0002. The fix
lets a correct program compile, so the floor moves down by that cost.

## 2026-11-06 — a derived definition opens with the imports it needs

**DONE.** A module's files share their declarations and not their imports, so
an arm that `derive/define` added could call only what its own module
happened to import. senro's next convenience is a reader each way for every
foreign key, as Rails' `has_many` and `belongs_to` give, and those readers
call senro's `models`. A definition may now open with `import` lines. The
module's derived file opens with all of them, once each and sorted, since
the parser refuses imports anywhere else or out of order.

A relative path in a definition is written from the derivation's own
module, as that module's own imports are, and the compiler rewrites it from
the module the definition joins. senro writes `import "../models"` from
`models/`; the notebook's `pages`, two directories away from senro, reads
it as `../../models`. The derivation's path is the one the entry imports it
by (`written_from` in src/derive.rs), and the rewrite is `reread` in
src/lib/modules.rs.

`tests/golden/derive/a_defined_arm_opens_with_its_imports` gives a module
that imports nothing two arms: one calls a sibling of the derivation's
module through a path written from there, and one imports `std/list` and
`std/text`. Both engines print `["HELLO!" "AGAIN!"]` and `HELLO AGAIN`. The
compiler before the change refused it, at the import in the middle of the
derived file.

Rows: `entry_instructions` rose 12,540 to 87,930,129 and
`emit_instructions` 270 to 5,613,493; `browser_compile_instructions` fell
17 to 350,566,782. The compile and library rows did not move. The change
came with the binary and with four more comment lines in lib/derive, and
nothing here isolates one from the other. It builds the derivations ruling
(#2186), so the welfare floor moves down by exactly that.

## 2026-11-06 — directive: the language committee decides, and nothing blocks on Clay

Clay, on #2356: "nothing needs me you can just ask the language committee
like I said. I will give you feedback once I see what you've built perhaps
but you don't have to block anything for me."

A design question, including one about kanso the language, is now decided
by the committee and recorded on its issue and here. The work goes ahead on
that decision. Clay's later feedback overrides it and is recorded as a
ruling. `needs-clay` stays for what the committee cannot settle: a refusal
from the harness, a permission, or a question Clay has said he wants put to
him. CLAUDE.md states the rule under The tracker, and the places that sent
decisions to him now point there.

The first decision under it is #2356, whether an application model should
get Go-style embedding. The committee chose no implicit delegation. A model
is the record its table derives plus the functions in its module, and a
delegator is an ordinary function. A field group that several models share,
timestamps most likely, uses the `&` merge of the 2026-07-25 gavel, and its
open parts (acceptance where the parent is expected, the collision rule,
variance) are built when a second model shares one. Promotion was declined:
it needs depth and ambiguity rules, and it lets a function match a type it
was not written for by a rule the call site cannot see. #2356 is closed.

#2284, how browser code calls a JavaScript library, stays parked until an
app needs one, which is when the committee takes it up.

## 2026-11-06 — `kanso run` hands SIGTERM and SIGHUP to its program

**DONE.** `kanso run` builds a native binary and waits on it as a child. A
SIGTERM sent to the driver, which is what `kill` and process supervisors
send, ended the driver alone. The program went on running, reparented, and a
server went on listening on its port. senro's specs start each app with
`kanso run`, stop it with `kill`, and start the next on the same port, so a
later spec could reach a server an earlier one left behind. Four orphaned
servers were found on port 3000 in one container. They explain a
password-reset mail delivered to an old server's outbox and a JSON request
answered by a server built before the branch that added JSON.

`execute` in src/main.rs now spawns the program and, while it waits, passes
SIGTERM and SIGHUP on to it. SIGINT already reached the program, because a
terminal sends it to the whole process group. When the program dies of a
signal the driver handed on, the driver raises that signal on itself and
prints nothing, so whoever sent it sees the driver end by it. A program
killed by a signal the driver never received is reported as before; the
first version re-raised any signal and turned `a_program_the_system_killed`
red. The handlers are installed once. With no program running, the handler
puts the default back and raises the signal itself, so the driver never
resets them.

`tests/a_stopped_run_stops_its_program.rs` runs a native program that
writes `started`, sleeps three seconds and writes `finished`, and sends
SIGTERM to the driver once `started` appears. Before the change it failed
with "the program was still running after its driver was terminated"; after
it, `finished` never appears, the driver ends by SIGTERM, and stderr is
empty.

Rows: `startup_instructions` rose 243 to 50,507, the two `signal()` calls
the driver now makes on every run. The first version reset both handlers
after the wait and read +489. `codegen_instructions_release` fell 124 to
372,025,748 on both of CI's readings. Nothing in the diff reaches `kanso
build`, and the cause is not isolated. Welfare fell by 0.00009. The
committee decided on kanso#2370 that the change merges and the floor moves
down by exactly that, as it did for #2350: a driver that leaves its program
holding a port when it is stopped is the toolchain failing what `kanso run`
promises.

## 2026-11-07 — the entry corpus is shaped like an app (#2373)

`kanso check` of senro's pantry compiled its 27 modules 1,094 times, because
the loader compiled a module once for every import that reached it. No row
welfare reads could see that. `compile_corpus` is one module whose imports
each arrive once, and `bench/entry_corpus` imported std modules and two local
files that import nothing from each other.

This change gives `bench/entry_corpus` four local modules, models, views,
routes and mail. Each imports the corpus's own `shared` module and std/text
and std/list, the way a senro app's modules all import its models and pg.
The entry calls one function from each. On the compiler of this tree,
`KANSO_PHASES=1` prints 46 module loads for it.

`entry_instructions` rises by the new modules' cost, from 87,930,129 to
177,925,530 instructions on CI. It is a
corpus change and not a compiler change, so welfare's `compile_instructions`
baseline is scaled by the same ratio and the floor holds, as #2269 did for
the forking shape. The fix this baselines for is kanso#2374.

## 2026-11-07 — each module is compiled once per load (#2373)

`kanso run` of senro's pantry took 8.4 s to answer its first request after a
container restart. `kanso check .` alone took 2.0 s and 8,741,148,257
instructions. `load_dependencies` compiled a module for every import that
reached it, and every import inside that module the same way, so a module
cost one compile per path through the import graph. `KANSO_PHASES=1` printed
1,094 `load` lines for the pantry's 27 modules: std/text 235 times, std/list
212, std/bits 144, senro's pg 36.

The corpus could not see it. Every compile sample was one file, the module
sample imports one module, and `compile_instructions` checks `lib/json`,
which loads each module once. The first commit adds `kanso::module_loads()`,
a `loads` column in `bench/compile_golden_modules.txt`, and the sample
`tests/golden/compile/shared`: an entry importing two modules that both import
a third, all three importing std/text. Before the fix it read `loads=11` for 6
modules, in CI run 42413285691.

`compile_module_inner` now keeps a module the second time an import asks for
it and hands later importers a copy. The key is the canonical path,
`require_entry`, whether the sources came embedded, and the flags an importer
sets around a compile. A shipped std module is pruned against the words the
program has read so far, and that set only grows, so its key also holds the
set's size; a module from disk is not pruned and its key does not. The
outermost `load_dependencies` empties the cache when it returns, which bounds
it to one compile whether the root is an entry file or a directory.

The first version kept every module on its first compile. That took the
pantry to 96 loads and 0.75 s, but CI read `compile_instructions` at
25,213,290 against 24,110,987 and `compile_memory` at 859,218 bytes against
598,588. A module's program carries every declaration its imports brought in,
so keeping one is a clone of its whole subtree, and `lib/json` imports each
module once, so it paid every clone and used none. Keeping a module only on
its second request costs a set of keys. Locally, callgrind reads `kanso check
bench/compile_corpus` at 25,315,543 against 25,308,680 before the change.

On CI, against the app-shaped entry corpus #2375 added for this change,
`entry_instructions` falls from 177,925,530 to 155,859,192. The rows with
nothing to share pay for the cache's bookkeeping: `compile_instructions`
rises 6,406 to 24,117,393, `library_instructions` 43,941 to 73,505,059,
`compile_allocs` 16 to 13,956 and `compile_peak_bytes` 469 to 599,057. The
interpreter's run of its corpus loads modules too: `interp_allocs` rises 16
to 810,162 and `interp_peak_bytes` 344 to 1,366,487. Compiling in the tab
moves both ways, 3,466 fewer instructions and
`browser_compile_peak_bytes` up 208 to 856,339. `emit_instructions` falls
346, `codegen_instructions_dev` falls 677 and
`codegen_instructions_release` rises 108 to 372,025,856, which is the
binary's layout: the shipped runtime object below made it larger. The
module corpus mints `module_loads`, which reads 3. A cheaper key, the path's bytes in place of a
`PathBuf`, and keeping a module only on its third request were both measured
and dropped: neither brought those rows back, and the third request gave the
pantry back 64 of its loads.

The pantry now loads 164 times, and `kanso check .` takes 1.09 s where it took
1.94 s. Its IR is byte-identical, 2,314,435 bytes. `shared` still reads
`loads=11`: each of its modules is asked for at most twice under one key, so
nothing is handed out of the cache, and the rows match the corpus commit.

A cold `kanso build` of the pantry took 7.2 s, and 5.7 s of it was clang
compiling `runtime.c` for the dev tier into the temp directory, once per
container. build.rs now compiles that object, 377 KB, and the binary carries
it. When the key the compiler computes names the dev tier with no counters,
on the convention build.rs's clang took, a cold build writes those bytes
instead of running clang; any other recipe compiles as before. The dev tier's
`-O2` moved into build.rs, which writes it out for the compiler, so the
shipped object and a compiled one come from one list.
`tests/a_cold_dev_build_ships_its_runtime.rs` builds a program against an
empty temp directory behind a clang that logs its commands and counts compiles
of the runtime: 1 with the shipped object turned off, 0 with it. The codegen
gates warm the cache before they count, so no row they read moves.

With both changes, a cold `kanso build` of the pantry takes 1.8 s where it
took 6.5 to 7.0 s, and a warm one 1.7 s where it took 2.9 s.

## 2026-11-08 — a dev build folds small word functions into their callers (#2304)

A dev build compiles the program at `-O0`, where clang inlines nothing that
is not marked. PBKDF2's rounds call `sha256/spun`, a rotate, four times per
message-schedule step and six times per round, and each call pays for a
frame, the moves of its two words and the spills of its result. With #2376's
width bits, a dev build of 10,000 rounds spent 32% of its instructions in
`spun`.

A definition now goes out `alwaysinline` in a dev build when it takes only
words, calls no other definition of the program, and is at most 80 lines of
IR. `Backend::folded_at_dev` holds the rule. In the PBKDF2 program it folds
`spun`, `choice`, `majority` and `list/repeat`. The release tier is
untouched, because its inliner already decides for itself. Measured here
against main at 64f4c196, under callgrind:

| | main | this branch |
|---|---|---|
| `runbench_dev` | 2,822,197,755 | 2,774,768,832 |
| PBKDF2, 10,000 rounds, dev build | 2,379,121,527 | 1,651,368,753 |
| `codegen_instructions_dev` | 116,687,950 | 116,687,950 |

Three variants were measured and declined.

- Every small leaf, boxed parameters included, bought runbench's dev run
  2.0% and cost `codegen_instructions_dev` 4.5%, which welfare scores below
  the floor. Two leaves that take a `%KValue`, `list/next` and
  `render/to_string`, were the whole of that cost. Unpacking a `%KValue`
  inside a caller is an `extractvalue`, and clang's fast instruction
  selector at `-O0` hands the rest of that block to the slow one.
- A budget on lines times call sites, from 150 to 3,000, changed no row
  once boxed parameters were excluded.
- A 40-line cap misses `spun`, which is about 50 lines, and PBKDF2 keeps
  the whole cost. 80 is the smallest cap measured that reaches it.

On CI, `runbench_dev` falls from 2,822,091,163 to 2,774,662,240. The
compile side pays a little: `emit_instructions` rises to 5,630,728 and
`browser_compile_instructions` to 351,180,151. The tab compiles through the
same dev emitter, so it runs the check, and its translator ignores the
attribute, so its run gains nothing. That reading of the browser row has not
been isolated. `codegen_instructions_dev` and `codegen_instructions_release`
do not move. Welfare rises from 95.07 to 95.08 and is banked.

## 2026-11-07 — a mask bounds an int, and a bounded sum needs no overflow test (#2304)

PBKDF2's rounds spent most of their time on adds that could not overflow.
`h + mixed + fed & whole` masks to 32 bits, but inference knew each operand
only as an int, so every add carried an overflow branch to a bignum and the
result stayed boxed as int-or-bignum.

Inference now carries five width bits beside `INT`. Each says a word int in
the set may lie outside `[0, 2^k)`, for k of 32, 40, 48, 56 and 62. They say
what may be, like every other bit, so the join stays a union and an int
bounded on one path and not another is unbounded. `INT` is the word with all
five, so everything that produced an int before still produces an unbounded
one. Three rules remove bits:

- a literal carries the bits its value needs;
- `x & y` is no wider than its narrower operand, since a non-negative mask
  bounds the answer whatever the other side held, a negative word included;
  `|`, `^` and `bits/shr` are no wider than their wider operand;
- `a + b` of two words below 2^62 crosses only the tightest bound its
  operands had, so it moves up one bucket and cannot leave the word.

A bound that can climb has to be given up, or the fixpoint climbs with it.
The first build let a counter fed by `n + 1` widen one bucket a round, and
the module sample took ten rounds where it took four. Parameters, returns,
record fields and yields now join through `infer::joined`. A word arriving
with a bound above 2^32 is a sum, since masks and literals arrive below it,
and it widens at once to an int or a bignum, which is what it read before the
width bits. A word that grows wider than what is held loses its bound. Every
`compile_golden.txt` row reads what it read before. A rule for `a - b`, which
cannot leave the word either, went too: its answer had no bignum yet and the
next sum gave it one, a round later.

Codegen reads the same rules. A sum the rules bound is `add nsw` with no
overflow test and no bignum behind it, in the binop path and in the word
runs that chain arithmetic. Eight checks that asked whether a set was exactly
`INT` now ask whether it holds only a word, whatever its width.

`tests/golden/micro/a_masked_sum_stays_a_word.kso` holds the edges: masked
negative operands, sums at each bound, a sum of two words just under 2^62,
and a quadrupled one that must become a bignum. With the sum rule changed to
keep its operands' bound instead of moving up a bucket, the native engine
printed -4 for 18446744073709551612 while the interpreter printed the
bignum.

Measured here under callgrind, PBKDF2 at 10,000 rounds read 380,173,992
instructions and reads 347,030,678, with the same digest. Compiling costs a
little more: `kanso check` of `compile_corpus` read 24,629,306 and reads
24,664,277, and of `entry_corpus` 88,645,583 and 88,780,655. `sha256/compress`
fell from 212,561,254 to 179,417,940; `sha256/schedule` did not move, since
its adds read list elements, which carry no bound.

Tried and dropped, again: masking the four list reads in the rounds and the
schedule, `(ks[at] & whole) + (w[at] & whole)`. With the inline `&` path now
taking any operand that may be a word, the run read 366,492,605, worse than
without the masks. The inline path stays: it costs nothing where no mask is
written.

On CI, against main after #2374, the run rows fall: `digestbench` from
3,288,947 to 3,108,282 instructions, `runbench` from 915,653,371 to
913,657,054 and `runbench_dev` from 2,822,091,163 to 2,811,021,533; with
#2378's dev fold also on main, `runbench_dev` reads 2,763,752,866. The
compile rows pay for the wider sets and the rules that read them:
`compile_instructions` rises to 24,152,651, `entry_instructions` to
156,082,962 and `library_instructions` to 73,608,656. On the tree merged with
#2378, `emit_instructions` rises from 5,630,728 to 5,642,485 and
`browser_compile_instructions` from 351,180,151 to 352,264,415. The emitted
code grows where the inline bitwise path now takes an operand that may be a
word behind a tag test: `emitted_other_branches` reads 19,444 and
`emitted_other_lines` 182,342. The machine code shrinks by 832 bytes.
Welfare rises from 95.07 to 95.08 and is banked.

## 2026-11-08 — patterns and operators move out of emit.rs (#2377)

`src/codegen/emit.rs` had grown to 3,055 lines in one `impl Backend` block.
Two runs of methods in it shared nothing with the rest, so they moved to files
of their own, read with `include!` right after emit.rs as calls.rs already
is:

- `patterns.rs`, 444 lines: matching a pattern against a parameter, the
  `%parsed` returns and constructions, packed constructor arguments and the
  type-set checks.
- `operators.rs`, 471 lines: word runs, the general arithmetic they fall back
  to, `emit_binop` and the builtin operators.

emit.rs keeps bodies, binds, conditions and indexing, at 2,153 lines. The move
changes no text: the dev and release IR of `bench/runbench` are byte-identical
before and after. Five ratchet mutations patched lines that moved, and each
now names the file its line lives in. `tests/native_layout_on_wasm32.rs`
reads the two new files when it looks for the lines the wasm32 table
rewrites.

## 2026-11-08 — a module reads through a record it was handed

A library may read a field that no type it can see declares, as long as the
read's base is a parameter: the value came from the caller, and the program
that imports the library answers for the field (the 2026-10-12 gavel on
#1894). The check looked only one step back, so `p.author.name` was refused
in the module that wrote it. `p.author` is not a parameter, and nothing in
the module declares `name`. senro's kata hit this: a template could not show
a post's author's name, and its README listed that as a limit.

The check now follows a chain of field reads and indexes back to where it
starts. `p.author.name` and `ps[1].author.name` are both left to the program,
and the program still refuses a field no type in it declares, at the end of
a chain as anywhere else. `tests/golden/field_reads/a_chain_from_a_parameter`
runs on both engines, and `errors_module/a_field_chain_no_type_declares` pins
the refusal of a misspelled last field. Main's binary refuses the first
fixture with "no record type has a field `name`" on both engines.

kata needed nothing of its own. The page as data already kept the inner
record's fields that a template reads, so senro's `spec/runs/nested_reads`
prints the same page three ways: rendered from the records, as data, and
rendered from the data.

## 2026-11-08 — a mistake in derived source names the line it was written from (#2380)

A compile error in code a derivation wrote named the module's derived file,
`views.derived.kso`, which is not on disk. senro's kata listed this among
what it does not do yet: a typo in `views/list.kata` was reported at a line
of the derived source.

The committee chose a comment convention over carrying positions through the
derive API (#2380). A derivation may write `# from <file>:<line>` above what
it writes, naming the file within the module. When a diagnostic falls in a
derived file, the renderer looks up to the nearest such comment in the same
definition, or among the comments right above its header, and adds
`= written from <module>/<file>:<line>` under the quoted line. The derive API
and the span type stay as they were, and a derivation that writes no such
comment gets the message it got before. The quote of the derived line stays,
because it shows what kanso read.

`tests/golden/derive/a_template_mistake_names_its_template_line` writes a
template that misspells `name`, under one `# from` comment above the
function. Both of its refusals, the unused parameter at the header and the
unknown name in the body, carry `= written from …/views/greeting.tpl:1` on
both engines. Main's binary prints them without the note.

Tried and dropped: one refusal quotes nothing in derived source. The
travelling-call check in `lib/travel.rs` ("no module in this program writes
`shout` for a type it owns") reads the file it quotes from disk, and a
derived file is not there. Reading the derivation's text first fixes it, but
the change moved rows whose corpora never run that code. Inlined, it
read `browser_compile_instructions` at 352,264,417 against main's
352,264,415, and reverting it alone restored 415. Out of line in a cold
helper, the browser row fell to 352,264,411 and `entry_instructions` rose
from 156,082,962 to 156,082,993, which together read below the welfare
floor. The fix waits for a change to that function that pays for its own
layout.

## 2026-11-08 — a list with a comment among its elements spans lines

A list or map spans lines only when it cannot fit on one (the 2026-10-13
gavel on #1882). The width check measured a span with its comment lines
dropped, so a short list with a comment above an element was refused as
fitting on one line, and the line it asked for cannot be written: a comment
runs to the end of its line. senro's kata hit this once each literal sat
under its own `# from` comment (#2380). Two short literals in one `joined [ ]`
were refused, and a template like that could not compile.

The lexer now notes when a span holds a comment line, and such a span skips
the fits-on-one-line refusal; a nested span's comment counts for the span
around it. A span with no comment is judged as before, so the corpus's two
refusals, `a_list_that_fits_on_one_line_does_not_span` and its map twin, still
read the same. `tests/golden/micro/a_list_with_comments_spans_lines` holds a
short list and a short map, each with a comment above every element, and
prints `["<p>" "</p>"] { "a":1 "b":2 }` on every engine; main's binary
refuses both literals. Appendix C says so beside the width rule.

## 2026-11-08 — the travelling-call refusal quotes derived source

#2380 rules that a diagnostic falling in a `.derived.kso` file carries
`= written from <file>:<line>`. One refusal still missed it: the
travelling-call check in `lib/travel.rs` ("no module in this program writes
`perimeter` for a type it owns"). It read the file it quotes from disk, and a
derived file is not there, so in derived source it printed the header alone.
It now renders through `render_merged`, which every merged-check refusal
already uses and which reads a derivation's text before the disk. The line is
quoted and the note follows.

`tests/golden/derive/a_traveling_call_in_a_template_names_its_template_line`
writes a template that calls `perimeter` on a circle whose module writes only
`area`. Both engines quote `views.derived.kso:3` and name `views/one.tpl:1`;
main's binary prints the header alone.

No corpus reaches this refusal, and the change still moves two rows through
layout, as CI measured them: `browser_compile_instructions` rose from
352,264,415 to 352,264,424 and `entry_instructions` from 156,082,962 to
156,082,965. Every other row read as before. #2382 dropped the fix to keep
the floor, and that was the wrong call: the refusal is part of what #2380
ruled, and a ruled feature's cost lowers the floor by exactly what it
measures. The floor moves by that amount here (#2383).

## 2026-11-08 — a module calls an arm a derivation writes

An entry that derives loads its modules twice: once to describe them to its
derivations, with the refusals a derivation could answer set aside, and once
with what the derivations wrote. A call to a name nothing defines yet waited
for the second load, which is how a module calls kata's `views/...`. A call
to a function that exists, at an arity only a derivation supplies, did not:
the first load refused it before the derivation ran. senro's data providers
hit this. A browser action calls `stock/low 3`, and a derivation writes the
one-argument `low` beside the server's `low viewer db n` (#2389).

The first load now sets aside a refusal of an arity no arm takes, as it does
an unknown name, and the second load checks the call against every arm the
module and the derivations wrote. `tests/golden/derive/a_module_calls_an_arm_a_derivation_writes`
prints `short 1` on both engines, and main's binary refuses it.
`a_module_calls_an_arm_no_derivation_writes` still refuses a call at an arity
nothing writes, and the refusal names every arity the module has once the
derivation's arm is in: `(arms take 2, 3)`.

A second refusal stood behind it. A derived arm's import is written from the
derivation's module and rewritten from the module it joins. When that module
is a file named bare beside an entry run from its own directory, as `stock.kso`
is when an app runs `kanso run .`, its parent is the empty path, which does not
canonicalize, and the import was left as written: `../data` from senro's
`data` module read from the app's directory found nothing. An empty parent is
now the current directory. `tests/derivations_run_beside.rs` runs each case in
`tests/golden/derive_here` from its own directory, which `tests/golden/derive`,
run from the repository root, cannot; main's binary refuses
`a_derived_arm_imports_beside_a_bare_module` with "cannot resolve import
\"../words\"".

`browser_compile_instructions` rose from 352,264,424 to 352,264,564. The
browser corpus derives nothing, so the 140 instructions are the new code in
the compiler the tab runs. Every other row is where it was.

## 2026-11-10 — gavel: an app goes through its models, never through pg/query

The guestbook's list page on the senro site read `pg/query db newest []`.
Clay: "this is crazy. you shoudn't have to call something like db/query. you
just use the model. clay = models/user/find params.id or something along
those lines." On how the session reaches the call: "that's the kind of thing
that the framework would do and inject somehow into the controller actions
perhaps. you can figure out something elegant for that. like maybe it passes
in a DB object that you pass to the model method. or maybe it sets those up
preconfigured so you get it already from the imported finder object that's
analogous to the class with its class methods in rails."

senro had model writes (`models/created`) and two reads (`found`, `where`),
each taking the viewer and the session apart, and no read of a whole table.
The three example apps wrote SQL by hand at 34 sites. The ruling is that app
code holds none. The design, the first of Clay's two options, is on #2394:
the session an action is handed already knows who is signed in, and each
model's module answers its reads and writes by name. Clay also asked that
such designs go by the committee, "to find beauty and elegance".

The committee read it from three seats (a kanso purist, a Rails veteran, a
newcomer) and decided, on #2394:

- `db` is passed to the model's function. All three preferred it to a finder
  value handed to the action, which would shadow the module it stands for.
- The reads are `find`, `find_by`, `all`, `newest`, `ordered`, `where` and
  `paged`. `ordered` takes a column and nothing else: the first draft's
  `ordered db "id desc"` put a small language inside a string, and `newest`
  covers the case that wanted it.
- `where` keeps one field and one value. A map of conditions was declined
  because the derivation can check a field written as a string against the
  table's columns and cannot read a map literal's keys.
- The writes are `create`, `update` and `destroy` per model, over the
  purist's `created`, `updated`, `destroyed`.
- The scaffold is split into `note.kso`, the model, and `notes.kso`, the
  actions.

Open: the account reads that run before anyone is signed in, which need a
read that asks as the app itself.

## 2026-11-10 — gavel: no for loops, in kanso or in kata

kata, senro's template language, wrote a loop as `for e in signatures`. Clay:
"we NEVER EVER EVER EVER want 'for' loops in kanso. we use collection
pipeline constructs like each and map."

kanso has no `for`, and kata's row compiled to kanso's `each xs (x -> ...)`
already, so the template syntax was the only `for` in the project. The ruling
covers the language and every template or configuration language built on
it: a collection is walked with `each`, `map` and their kin, and a `for` is
refused. kata's row is rewritten as an `each` and a `for` row is an error at
its template line (#2395).

The committee kept the spelling `each signatures e ->`, collection first,
two seats to one over `each signatures as e`: it is the call kanso would
write with the parentheses the indentation makes redundant. `std` has no
`each`, so kata's README calls it a helper kata writes.

## 2026-11-10 — directive: senro may ask the language for more

Clay, on the model finders: "this real world use case for kanso may lead to
several more language tweaks. in general i support ambition as long as it
fits with the stated goals for the language and the principles behind them.
i'm particularly fascinated to see how you create a construct for convention
over configuration without dynamic metaprogramming."

The construct is `derive`: a library function the compiler runs over the
project's modules, which answers source to add or a call to refuse. A senro
feature that needs the derivation to see more than it is handed is a
language change, proposed to the committee against the language's goals.
The first candidate is the keys of a map literal at a call, which would let
`where` take a map of conditions and still be checked (#2394).

## 2026-11-10 — senro's apps hold no SQL

The two rulings of this morning are built. The work is in senro (senro#93
to senro#98) and on the site (#2393, #2398 and the sessions guide with
this entry); no compiler change was needed, and no cost row moved.

**What an app writes now.** A model lives in a module named for one row of
its table, `note.kso` for `notes`, as kanso names a module for its type.
The models derivation declares the record there from the table's columns
and writes the calls: `note/find db id`, `find_by`, `all`, `newest`,
`ordered`, `where`, `paged`, `create`, `update`, `destroy`, `increment`.
The `db` an action is handed knows who is signed in, so no call takes a
viewer. A column written out in a call is checked against the table when
the app is checked, and the refusal is at the call
(`spec/errors/order_not_a_column`).

**The app's own writes.** Some writes have no viewer behind them: a reset
link sets a digest, a sign-in counts a wrong password. The committee chose
`models/as_app db`, a session that asks no policy, over naming the
account's holder as the asker. Its reason: under the alternative the
holder's `update?` must admit writes to the count of wrong passwords and
the lock, which are the columns a browser-reachable write must never
touch.

**The count.** `account/increment db id "failures"` adds in the statement
that answers the record. `spec/db/counted` runs eight on eight connections
at once and they see 1 2 3 4 5 6 7 8, on both engines. With the increment
rewritten as a read and then a write, they saw 1 1 1 1 1 1 1 1 natively
and 1 1 1 1 1 2 2 2 interpreted.

**The examples.** The pantry, the guestbook and what both generators write
hold no query; `spec.sh` fails when one does. The guestbook's `account.kso`
became the model alone, with `registration.kso` and `password.kso` as its
resources, and the scaffold writes `note.kso` beside `notes.kso`.
kata walks a list with `each xs x ->` and refuses a `for` row at its line.

**What I got wrong on the way.**
- I regenerated goldens (`SPEC_WRITE=1`) while the tree did not check, and
  two goldens took the compiler's error text. Restored from the index; the
  rule is to regenerate only from a tree that passes without writing.
- The first generated app did not check because the model's module was not
  imported by the entry. A derivation reads only the modules the entry
  names, so the generators add that import.

**Open.**
- senro's CI failed twice in ten runs in the guestbook's browser spec with
  the server interpreted, at different steps in different browsers. Forty
  local Chromium runs passed (#2397).
- A lock that runs out by the clock has no spec; it needs a clock the spec
  can set.
- `where` takes one field and one value because a derivation cannot read a
  map literal's keys. That is the first language change senro would ask
  for, and it goes to the committee when a second condition is wanted.

**The site deploy.** Two deploys after #2393 stalled inside GitHub Pages at
"syncing files", one failing and one reaching the ten-minute timeout, so
the pantry's recording was on main for an hour before it was served. The
third, after #2398, went through. Nothing in the artifact explains it: the
merge added three small files. A merged site change is checked on
kanso-lang.dev, since main holding it does not mean the site does.

## 2026-11-10 — a native program holds as many sockets as the system allows

The native runtime kept open sockets in a table of 64, found by the
remainder a handle leaves, and ended the process when a sixty-fifth was
wanted: `too many sockets open at once`. The interpreter has no table. The
limit came to light through senro, whose server kept a connection for
every client that hung up without a request (fixed there in senro#102):
about sixty such clients ended a natively built app. With that leak fixed
the limit still stood in the way of any server with sixty clients
connected at one moment. Tracked as #2402.

**What changed, in `src/runtime/effects.c`.**
- The table starts at 64 slots and doubles when every one is taken. Two
  handles that leave different remainders by a size leave different ones
  by twice it, so each open socket moves to the slot its handle now names
  and no two meet. Handles are numbered as before, so a program that
  prints one reads the same on both engines.
- A slot remembers the handle that owns it. Before, `k_socket_of` asked
  only whether the slot was in use, so closing a handle a second time
  closed whatever connection had taken its slot since.
- `listen` asks for the longest queue of unaccepted callers the system
  allows, where it asked for 16.
- Running children keep their own table of 64 (`K_KIDS`), which they had
  shared a constant with.
- The lookup, the taking of a slot and the growing are kept out of line,
  so the function every effect passes through holds a call to them and
  none of their code.

When the system itself refuses a file, `dial` already answered an err, and
`accept` already left its fiber waiting and trying again, so a server at
the system's limit keeps the clients it has. Nothing there needed to
change.

**Three specs in the micro corpus, each red on the runtime before.**
- `a_program_holds_more_sockets_than_its_first_table`: a hundred
  connections held by both ends, then the first pair and the last speak.
  Before: `too many sockets open at once`, at the thirty-second accept.
- `a_handle_closed_twice_closes_nothing_else`: sixty-four sockets, one
  closed, one opened in its place, the old handle closed again. Now the
  second close is refused and the new connection takes a write. Before,
  the second close succeeded and the write failed: `that is not a
  connection`.
- `a_listener_queues_a_hundred_callers`: a hundred callers connect before
  any is accepted. Before, the program stopped partway and was ended by
  the test's clock. With the table grown and the queue alone put back to
  16 it stops the same way, so the queue is the cause.

The interpreter passed all three before and after.

**What I got wrong on the way.**
- The runtime is also built for wasm, where the headers name no queue
  length, and the first build of it failed. I had built and tested natively
  only. The constant is defined when the headers leave it out.
- The three specs open sockets, which the tab cannot, and I left them off
  `native_route_gaps.txt`. Three wasm corpus tests went red on the first
  push.
- The first version made every benchmark's effects dearer. CI's hosts were
  not the one the work rows were counted on, so the gate would not compare
  and I read the differences it printed as the host's. They were mine.
  Main built here reads each row a constant above its golden, 109 for
  thirteen of the fifteen, and the branch read deepbench 1,308,377 higher than
  that, runbench 303,062 and widebench 48,034. All of it was in `k_exec`,
  the function every effect passes through. The table's lookup, with its
  remainder by a size that is no longer a constant, had been folded into
  it. With `k_sock` kept out of line deepbench reads main's number again.

**Rows.**
- `unpinned_diagnostics.txt` loses the message that no longer exists.
- Work: eleven of the fifteen rows are main's. `work_runbench` rises 469
  to 913,657,523 and `work_oneshot` 357 to 9,980,628; `work_encodebench`
  falls 350 to 2,335,340,431 and `work_livebench` 189 to 938,655,512. No
  benchmark opens a socket, so these four are layout. I set the rows from
  a count here and each row's distance to main's on this machine, and CI
  then counted on the golden's own silicon: fourteen agreed and
  `work_livebench` read 28 lower, which is CI's row above.
- clang's work on the runtime: `codegen_instructions_release` falls 80,668
  to 371,945,188 and `codegen_instructions_dev` rises 2,539 to
  116,857,586.
- Machine code: each of the fourteen benchmarks carries 512 fewer bytes of
  `.text` than on main, and `text` sums to 5,869,065 from 5,876,233. The
  lookup is one function now where it was a copy at each use.
- The tab runs its corpus in 55,987 fewer wasm instructions
  (`browser_run_instructions` 78,370,192). That corpus opens no socket. I
  did not isolate what clang did differently.
- The three specs join `native_route_gaps.txt`, because the tab has no
  sockets.

**Welfare.** The sum rises by 0.000027 and is banked: the release tier
and the tab cost less, against 469 instructions on runbench and 2,539 at
the dev tier.

**Open.** A fiber waiting on a socket is asked again every millisecond on
both engines, so an idle server is never asleep for long and each query a
request makes costs a millisecond of waiting (#2405).

## 2026-11-10 — a fiber waiting on a socket is waited for

A read that found nothing arrived, or an accept that found nobody calling,
put its fiber back in the queue to be asked again a millisecond later. Both
engines did this. Two costs followed, and senro's server showed both
(#2405). A request that asks Postgres a question waits for the answer, and
each wait rounded up to the next millisecond. A server with a listener and
nothing to do woke a thousand times a second.

**What changed.** When every fiber is waiting and at least one waits on a
socket, the scheduler waits on those sockets with `poll`, for as long as
the earliest fiber on the clock allows, and for good when no fiber is on
the clock. A socket that is ready makes its fiber due now. The clock moves
to the time that passed and no further than the fiber it was waiting for,
so a sleeping fiber still runs at its own time.

- What a fiber waits on is read from the work it has left. A fiber the
  scheduler holds with an accept or a read at the bottom of its chain
  asked it and was told not yet, so the scheduler walks down the chain
  words to that description and takes the handle from it. Nothing is kept
  per fiber and nothing is done per step.
- A flag says whether to look at all. An accept that finds nobody and a
  read that finds nothing raise it, and the scheduler puts it down when it
  looks and finds no fiber on a socket. A program without sockets pays one
  test of the flag each time every fiber is waiting.
- Natively the wait is `k_sched_await`, which reads each descriptor from
  the socket table. In the interpreter it is `awaited`, which asks the
  executor's `await_sockets`. An executor without sockets answers `None`
  and the scheduler ticks as before, which is what the scripted executor
  and the tab do.
- An accept the system refused for a reason other than nobody calling
  keeps its one-millisecond tick natively, and its description carries an
  int to say so. The listener stays ready through such a refusal, so a
  wait on it would come straight back.
- `libc` becomes a dependency of the compiler on unix, for `poll`. It was
  already in the lock file.

**The counter.** `sock_waits` counts the scheduler's waits on sockets, on
both engines, and is printed only when it is not nought, as `delete_marks`
is, so no golden without a socket carries it.
`tests/golden/mem/a_reader_waits_on_its_socket` has a reader with nothing
to read beside a writer that sleeps fifty milliseconds first: two waits,
one for the writer's clock and one for the bytes. Its `.mem` pins
`sock_waits=2` natively and
`tests/a_fiber_waiting_on_a_socket_is_not_asked_again.rs` holds the
interpreter to the same number. Thirty runs on each engine all read 2.
With the wait taken out of either engine the count is not printed and the
spec fails. The trend gate lists it among the counters a fall is a loss
for.

**The fiber table is one array.** The native scheduler kept its fibers
as an array per field, so each insert and each remove called `memmove`
once per field, and a fork does both. It keeps one array of records now
and moves the fibers after a fork once. runbench forks sixteen thousand
times and retires 3,089,651 fewer instructions for it, and the tab, which
runs the interpreted corpus through the same scheduler built for wasm,
retires 1,244,416 fewer.

**Two specs, and two faults of mine they caught.**
`a_reader_is_told_when_its_socket_closes` has a reader on a connection
another fiber closes; that fiber then dials, and the new connection takes
the descriptor the closed one had. On main both engines tell the reader
`that is not a connection`. My first native version held the descriptor
and would have waited on the new connection for good. The interpreter
held the handle and still hung: it asked `poll` to pass over the missing
socket and then waited with no time limit. A wait that includes a handle
naming nothing now returns at once, and the spec that was red on that
hang is green on both engines.

The second fault was the first design. It kept the awaited handle in a
field per fiber and cleared it before every step. Native kept it as a
fourth array, which a fork then moved as well: runbench retired 1,407,682
more instructions on this machine than the tree under it, and CI read
`interp_instructions` 153,945 higher.
Those rows are what the objective counts, and it does not count wall
time, so the design scored as a loss. Reading the handle from the fiber's
own work removed the field and the clearing. Left where
the compiler put it, the interpreter's new function still cost its corpus
59,991 instructions; kept cold and out of line it costs 9,993.

**Measured.** One sitting, 2026-11-10, in the development container: a
senro app answering 300 requests over loopback after twenty to warm up,
each on its own connection, with Postgres on a local socket, served by
this branch's compiler and by the one before it in turn. Mean time per
request:

| | before | after |
|---|---|---|
| native, no query | 1.25 ms | 0.12 ms |
| native, one query | 2.48 ms | 0.34 ms |
| native, five queries | 7.28 ms | 0.66 ms |
| interpreted, no query | 2.70 ms | 1.82 ms |
| interpreted, one query | 4.58 ms | 2.50 ms |
| interpreted, five queries | 12.66 ms | 6.73 ms |

The same app left idle for five seconds used 15 clock ticks of CPU
natively and 17 interpreted before, and none on either engine after. An
earlier count with `KANSO_SCHED_DEBUG` had an idle native server sleep
1,004 times in a second.

**Rows.** Counted on this machine with the toolchain the goldens name,
set from each row's distance to main's here, and then counted by CI on
the golden's own silicon, which agreed with every one. The two codegen
rows are CI's.
- Work: all fifteen rows fall. `runbench` 913,657,054 to 910,567,403 and
  its dev-tier build 2,763,752,866 to 2,759,705,221; the other thirteen
  by between 194 and 1,713.
- `browser_run_instructions` 78,426,179 to 77,181,763.
- `interp_instructions` rises 9,993 to 622,568,308. The corpus opens no
  socket, so this is the scheduler's loop laid out around one more call.
  `entry_instructions` rises 12 to 156,082,977 with the binary's layout.
- Machine code: each benchmark carries 48 fewer bytes of `.text` than on
  main, and `text` sums to 5,875,561.
- clang compiles the wait: `codegen_instructions_dev` rises 27,295 over
  main to 116,882,342 and `codegen_instructions_release` 154,850 to
  372,180,706.

**Welfare.** The sum rises by 0.0080, from 95.0845 to 95.0925, and is
banked. The falls in the run program and the tab outweigh the two codegen
rows and the interpreter's 9,993.

**Open.** `net/write` still blocks the whole program on a client that
reads slowly.

## 2026-11-10 — senro's server logs its requests and lets its connections go

Five senro pull requests and one here, all found by running the guestbook
as a server for longer than a spec does. The site's pin moves to senro
5b17eac with them.

**A request log (senro#101).** The server writes a line to stdout for each
request as it answers: `GET /entries/3 200 in 2ms`. The method is the one
a form stands for, so a confirmation's button reads `PATCH`. The query
string is left out. A path segment shaped like one of senro's signed
tokens is written `:token`, and an uploaded file's key `:key`, so a kept
log holds no link that confirms an address or resets a password.
`/senro/live` gets no line. `spec/serve/logged` pins the lines, with the
milliseconds masked.

**Every accepted connection is closed (senro#102).** The server closed a
connection after answering it and at no other time. A client that
connected and hung up without a request left its socket open on the
server for good, and about sixty of those ended a natively built app at
the runtime's table of 64 (#2402, fixed here in #2404). The close now
happens in one place, after whatever became of the connection. A
connection that sent nothing closes silently and any other early ending
is written to stderr. `spec/serve/hung_up` sends 320 such clients to a
server limited to 256 open files and then asks it a question.

**An action that answers no response (senro#102).** An action that
answered a string ended the request with no reply and nothing in the log.
The cause is a rule of the language working as written: a failure raised
inside a package is not caught by a `.?` in that same package, so `web`
could not rescue the err it raised itself, and the fiber's failure waited
for a join the accept loop never reaches. `web` now answers a 500 and
reports the value directly. `spec/serve/answered` pins the reply and the
report.

**One session listens for every page (senro#103).** Each open tab's
`/senro/live` stream held a Postgres session of its own, and a tab that
closed was noticed only when the next change was written to it. One fiber
now listens on one session and passes each change to every open page.
The server writes each page a line every fifteen seconds, and a page
whose write fails twice is let go. `spec/live` opens twenty pages and
reads the server's own descriptor count: twenty more files and no more
sessions, and none more once they have left.

**A lock that runs out (senro#100).** The guestbook locks an account
after three wrong passwords and mails a link, and the lock lapses after
an hour. Nothing had run the hour. `spec/lapsed` moves the lock's time
back in the database and watches the right password work again.

**The guides sat off the left edge (#2401).** Clay reported the views
guide against the left edge of the window. The nine guide pages had
`spec-doc`, which sets a column's width, and lacked `essay`, which
centres it; they had lacked it since they were written. `prose_check`
gained a family, "no reading column", that fails a page whose article has
a width and no centring.

**What I got wrong.**
- senro#102's first CI run was red on its own spec. The script read the
  server's log after stopping the server, and the log by then held the
  line kanso prints for a program ended by a signal. It reads the log
  first.
- #2401's first commit message said the pages had been checked in a
  narrow window, which I had not established. The message now says only
  that the class was missing.
- The spec for many callers stalled at eighty dials, and I put a pause
  between dials before finding the cause. It was the listen queue of 16,
  which #2404 isolated.

**Open.** A native server's resident memory grew from 3,140 kB to 3,692 kB
over 1,500 connections, and I have not found what holds it. The runtime
never frees a pool, which is why the live streams reuse their seats.
`net/write` blocks the whole program on a client that reads slowly.

## 2026-11-10 — a lazy binding a callback captured is not its frame's to free

Fixes #2409.

**What was wrong.** Natively, a lazy binding could be read after the
frame that made it had freed it, and what was read was a later
binding's value. Two shapes did it.

A callback captured the binding. This program binds `twice` in each of
three passes and reads it in a callback that runs after a sleep:

```
fn shown 0 _
  effect 0

fn shown n twice
  print "pass {n} read {twice + 0}"

fn handled n
  twice = doubled n
  time/sleep 2 .> (_ -> shown n twice)
```

The interpreter prints 6, 4 and 2 for passes 3, 2 and 1. The native
build printed 2 for all three.

A function handed the binding back and the caller kept it:

```
fn held false _
  ""

fn held true words
  words

fn listed n
  a = built n
  [(held true a)]
```

Two lists made by `listed 1` and `listed 2` read "kept 1" and "kept 2"
on the interpreter. Natively both read "kept 2".

**Why.** Each binding is lazy because the function it is handed to has
an arm that discards it, so it is a cell. `src/demand.rs` decides
whether the frame that made a cell may free it on the way out, and it
decided by looking at the position the binding was handed to. A
position is safe when the parameter there is only read, or answered
unread as the function's own result. Both positions above are safe by
that test, so both cells were marked releasable, and the frame's
epilogue put them on the free list. The next lazy binding took the same
cell.

The test was missing two questions about the frame that binds the
name. It never asked whether the use sat inside a callback, which is
called after the frame has returned. And it took an arm that answers
its parameter unread to be harmless, on the strength of the epilogue's
own guard: the epilogue does not free a cell that is the frame's
result. That guard sees the cell leave only when the call is the
frame's last expression. A call whose answer goes into a list, under a
name or into another call's arguments has put the cell somewhere the
epilogue does not look.

The callback question was missing on the callee's side as well: a
function that reads its parameter inside a callback of its own keeps
the cell past its caller's frame.

A binding read through a call, such as `length page`, was never
affected. That position was already unsafe, so the cell was never
freed.

**What changed.** Three checks, all in `src/demand.rs`.
- `captured`: a callback in the binding function that reads the name
  keeps the cell from being released.
- `param_stays_local`: a callback in the callee that reads the
  parameter makes the position unsafe.
- `keeps_what_is_handed_back`: a call that may answer the cell unread
  releases it only where the answer is the frame's result or is read on
  the spot, as an operand, an interpolation, an index or the subject of
  `.>`.

All three are conservative. A callback handed to `list/map` is finished
before the frame returns, and its cell is kept all the same.

**The specs.** `a_callback_reads_the_lazy_binding_it_captured` holds the
callback beside the binding and the callback in the callee.
`a_lazy_binding_handed_back_and_kept_is_read_later` keeps the answer in
a list, under a name and in an argument. Both are in
`tests/golden/micro`, and on main's native build each printed the last
value bound on every line.

**How it was found.** While building the fix for #2408 on another
branch I added a mode that marks a freed cell and dies when one is read.
A fixture written to stress that change died twice, once for each
shape, and both faults reproduced on main without the change.
`a_lazy_effect_handed_through_a_function_still_runs`, already in the
corpus, has the first shape and died under the mode too. It has passed
all along because nothing in it binds a second cell before the first is
read.

**Rows.** No runtime counter moves and every benchmark's machine code is
byte for byte what it was: no benchmark binds a lazy value in either
shape. clang reads the same programs, so the two codegen rows stay where
they were. What moves is the front end, which asks two more questions of
every lazy binding.
- `browser_compile_instructions` rises 18,918 to 352,283,482: the tab
  compiles its corpus through the same analysis.
- `interp_instructions` rises 3,510 to 622,571,818. The row runs the
  interpreted corpus from source, so it carries the front end too.
- `compile_instructions` falls 18 to 24,152,633, `entry_instructions`
  falls 163 to 156,082,814, `library_instructions` falls 86 to 73,608,570
  and `emit_instructions` falls 62 to 5,642,423. These four move with the
  compiler's binary. None of their corpora binds a lazy value.

I set the three compile rows from a count here and CI agreed with them.
I did not run the interpreter's sweep or the tab's before pushing, so
CI found those two rows and the emitter's.

**Welfare.** The sum falls by 0.0000166, from 95.09254193 to 95.09252535,
and the floor follows it. `welfare.kso` weighs every change but one: a
change that makes the engines agree ships at whatever it costs. The tool
never lowers the floor itself, so the floor's value was set by hand and
its history says why.

**Open.** A cell its frame cannot free lives until the program exits,
and these cells have joined that set. A server makes one for each
request that binds one (#2408).

## 2026-11-10 — a lazy binding a callback holds joins the corpus

Corpus first, before #2408. A senro app built natively held 184 bytes more
for every request it answered: 3,844 kB over 20,000 requests, where the
same app interpreted held nothing. valgrind named the allocation, one lazy
cell made in `web/text_at` for each request and never freed. `text_at`
binds the request's body lazily and hands it to a callback, so the frame
that made the cell cannot free it, and nothing else in the runtime did.
The log has called these cells live to exit since 2026-07-23. A program
that runs once pays for them once. A server pays for every request.

The fix for #2408 has the beat that made such a cell free it at a rewind.
With that fix the welfare sum read just under the floor, for two reasons
that are both about what is measured.

**Cells were not counted as held memory.** `held_peak_bytes` is the
malloc'd storage a program holds at its fullest, and it counted builders'
buffers and maps' views. A cell is malloc'd too and was left out, so
`run_peak_bytes`, which sums the arena, held and permanent peaks, could
not see a cell nobody frees. The counting build now adds a cell's 184
bytes when it is made and takes them off when it is freed. A shipping
build counts nothing, so no benchmark's instructions or machine code
move for this.

**No benchmark bound a lazy value that a callback holds.** runbench made
50 cells in all. `bench/runbench/runbench/deferred` is the `forked` shape
with the page a lazy binding: `sized` has an arm that never reads it, so
the page is built only when the callback asks. It runs 16,000 passes
after `forked`. `bench/interp_corpus` runs the same loop for 2,000 passes
before it prints. Neither printed line moves.

**Rows.** Only runbench and the interpreted corpus run the new loop.
The native and interpreted instruction rows are counts from this machine
set by their distance to main's reading here, and CI's first run agreed
with each. The tab's rows and clang's are CI's.
- runbench rises to `work_runbench` 955,363,291, the shape's 44,795,888
  instructions or 4.69% of the program, and at the dev tier to
  `work_runbench_dev` 2,807,447,887. Its machine code grows 7,792 bytes
  and the text rows sum to `text` 5,883,353. Its 18 new functions take
  the emitted rows of the other benchmarks to `emitted_other_defines`
  2,384, `emitted_other_calls` 21,917, `emitted_other_branches` 19,483
  and `emitted_other_lines` 182,766.
- Held memory, which is what the shape is for: `run_held_peak_bytes`
  rises from 248,688 to 3,201,888. That is 2,944,000 bytes for the
  shape's 16,000 cells and 9,200 for the 50 the program already made and
  never freed. `run_peak_bytes`, the welfare term, goes from 7,899,297
  to 10,852,497.
- The run program's other counters move with the 16,000 passes:
  `run_allocs` 2,344,070, `run_alloc_bytes` 255,465,985,
  `run_beat_iters` 196,448, `run_evac_allocs` 2,615, `run_evac_bytes`
  4,625,312, `run_thunk_allocs` 16,050, `run_thunk_evals` 16,025,
  `run_thunk_live_exit` 16,050, `run_str_scans` 35,402,
  `run_str_scan_bytes` 18,976,160, `run_sh_str` 18,977,792 and
  `run_sh_buf` 96,487,840.
- The interpreted corpus rises to `interp_instructions` 680,814,132,
  `interp_allocs` 984,852 and `interp_peak_bytes` 1,377,499. The
  interpreter frees a cell when nothing holds it, so its peak moves by
  the loop's own 11,012 bytes.
- The tab compiles and runs the same corpus:
  `browser_compile_instructions` 358,135,821 (from 352,283,482),
  `browser_compile_peak_bytes` 862,043 (from 856,339),
  `browser_run_instructions` 90,929,278 (from 77,181,763) and
  `browser_run_peak_bytes` 1,835,008 (from 1,572,864), which is four more
  pages of wasm memory.
- `codegen_instructions_release` rises 71 to 372,180,777. It arrived with
  the two counting statements in the runtime, which clang reads at the
  release tier and folds away. The dev tier links a runtime built ahead of
  time and its row holds.
- Counting cells alone moves ten goldens that run no new code.
  pendbench's 200 cells read `pend_held_peak_bytes` 36,800. In
  `tests/golden/mem`, `many_cells_held_peak_bytes` reads 7,360 for its
  40 cells, and eight fixtures that hold one cell read 184:
  `a_demanded_knot_allocates_one_cell_held_peak_bytes`,
  `an_unasked_equality_stays_a_cell_held_peak_bytes`,
  `builder_counts_once_held_peak_bytes`, `force_path_held_peak_bytes`,
  `returned_thunk_held_peak_bytes`, `skip_unused_held_peak_bytes`,
  `skipped_err_held_peak_bytes` and `unsafe_wrap_held_peak_bytes`.

**The floor is re-based.** The change is only to what is measured, so
each term it moves keeps its ratio to its baseline, as it did when the
corpus first forked (2026-11-01). Ten baselines are scaled by the new
reading over the old and rounded up: `run_instructions`, `run_peak_bytes`,
`dev_run_instructions`, `interp_instructions`, `interp_peak_bytes`,
`codegen_instructions_release` and the tab's four. The score moves only by
that rounding, and the `--set` records it.

**Open.** #2408's fix, measured against this.

## 2026-11-10 — a beat frees the lazy bindings its frames could not

Fixes #2408.

**What was wrong.** A lazy binding is a cell, and the frame that made it
frees it when the front end can show nothing else holds it. A cell a
callback captured, or one that left its frame in a tail call's arguments
or as the frame's answer, had no owner after that and lived to exit. A
senro app made one for every request it answered, in `web/text_at`.

**The beat that made the cell holds it.** The emitter calls
`k_thunk_new_held` for a lazy binding its frame will not release, and the
runtime holds a cell the same way when it leaves its frame in a tail call
or is handed back. A held cell goes in a registry kept for each beat
depth. A cell made outside every beat has no rewind to wait for and lives
to exit, as it did.

**The copy walk says which cells are still reachable.** A beat's rewind
already copies out whatever survives the lap. The walk now marks each
held cell it passes, and after the walk the rewind frees every held cell
with no mark. There are three marks. Seen this lap is cleared at the
rewind. Reached under tenured storage keeps the cell until the beat ends,
because no later walk enters tenure. Copied into storage no beat owns, a
constant's or a pool's, keeps it for good. A beat that ends and is
rewound whole frees all but the last kind. A beat that ends and keeps its
region hands its cells to the enclosing beat with everything else it
built. A work-ahead task that dies inside its beats frees what those
beats held.

**Why a marked cell is the only kind still held.** The beat analysis
never lets a value that may be a cell cross a rewind unless the copy walk
carries it, and the type set's `THUNK` bit is what it reads. So a holder
on the far side of a rewind is one the walk passed through.

**A check that reads for it.** A counting build run with
`KANSO_CELL_CHECK` set never hands a freed cell out again. It marks the
cell, and a read or a carry of a marked cell ends the program saying so.
A shipping build compiles neither test. The read's was first written
unguarded, and cost every forced binding two instructions, 32,050 in
runbench, until CI's listing of new diagnostics drew the eye to it.
With the check forced on, the whole suite here ran (256 test binaries),
then senro's (186 specs) and kq's, with no read of a freed cell. Twelve
programs written to break the registry ran under it too, and four of them
are micro goldens now: a binding read after many rewinds, bindings in a
carried list that are swapped and read, a parked fiber that keeps its
binding, and a binding that holds another. The check found the two
frame-release miscompiles that kanso#2410 fixed before this could land.

**The spec.** `tests/a_callback_lets_go_of_the_lazy_binding_it_read.rs`
runs a server's loop with a lazy page a callback reads, at 4,000 passes
and at 16,000, and asks that one cell is live at exit both times. Without
the registry it read 4,000 against 1. The same program is a `.mem`
golden: 4,000 cells made, 3,999 freed.

**Measured on the app.** One sitting in the development container on
2026-11-10, the senro app from #2408 built natively and sent 20,000
requests over loopback. Before, its resident memory grew 3,844 kB. Now it
grows 100 kB in the first thousand requests and stays there to the end.

**Rows.** The counters are this machine's and agree with the goldens'
arithmetic. The work rows are counts from this machine set by their
distance to the corpus branch's reading here. Clang's rows and the tab's
are CI's.
- The run program frees every cell it makes: `run_thunk_frees` goes from
  0 to 16,050, `run_thunk_live_exit` from 16,050 to 0, and
  `run_held_peak_bytes` from 3,201,888 back to 248,688, the figure it
  read before the corpus held a cell. `run_peak_bytes`, the welfare
  term, goes from 10,852,497 to 7,899,297. pendbench frees its 200:
  `pend_thunk_frees` 200, `pend_thunk_live_exit` 0,
  `pend_held_peak_bytes` 368. `many_cells` frees its 40.
- `work_runbench` falls 1,480,340 to 953,882,951 and `work_runbench_dev`
  falls 1,480,512 to 2,805,967,375. A freed cell goes back on the free
  list and the next binding takes it, so the allocator retires 2.94
  million fewer instructions. Holding, the lap over the registry and the
  release cost 1.46 million of that back.
  `work_pendbench` falls 30,552 to 154,093,875 the same way.
- A program that holds no cell pays for one test at a rewind and at a
  pop. Three of the rest rise by more than 50: `work_encodebench` 1,775
  to 2,335,340,843, `work_jsonbench` 472 to 549,338,002 and
  `work_digestbench` 414 to 3,107,797. Seven rise by 50 or fewer:
  `work_indexbench` 1,887,800, `work_scanbench` 267,925, `work_basket`
  18,295,405, `work_deepbench` 198,294,818, `work_escapebench`
  40,308,987, `work_readbench` 4,583,687 and `work_widebench`
  28,701,610. Two fall: `work_oneshot` 268 to 9,979,753 and
  `work_livebench` 888 to 938,654,619.
- The registry is 1,680 bytes of machine code in every program, so the
  fourteen text rows sum to `text` 5,906,873.
- The emitter picks between three calls where it picked between two:
  `emit_instructions` rises 1,114 to 5,643,537.
- Clang's rows, from CI: `codegen_instructions_dev` rises 56,903 to
  116,939,245 and `codegen_instructions_release` falls 281 to
  372,180,496.
- The tab runs the compiled corpus on the same runtime built for wasm,
  so the registry is in its rows too, from CI. Its run frees its cells
  and gives back the four pages the corpus entry cost:
  `browser_run_peak_bytes` falls from 1,835,008 to 1,572,864. It pays
  for the registry without the allocator's saving, and
  `browser_run_instructions` rises 203,408 to 91,132,686.
  `browser_compile_instructions` rises 3,211 to 358,139,032.

**Tried and reverted.** `k_beat_rewind_slow` gained one test, and three
ways of making it cheaper each made runbench slower than the plain form:
a short path for a rewind with cells and nothing else to flush (97,000
instructions worse than main, where the plain form was 57,000), guarding
the flushes behind the mark's summary bits (69,000), and folding the cell
test into the permanent-storage test (92,000). Those were measured before
the corpus held a cell. Of the plain form's 57,000, 46,000 were `memcpy`
inside `realloc`: the same 3,884 moves of a growing permanent buffer,
each copying more because the allocator placed it differently. That part
is layout.

**Limits.** A chain of effects, and a loop that carries a value, copy
their survivors out and rewind once the arena has drifted a quarter
megabyte past the last time they did, and held cells are freed then. So a
program holds one drift's worth of cells at a time: the 4,000-pass
fixture peaks at 68,816 held bytes, which is 374 cells. A loop that makes
held cells and allocates nothing in the arena never drifts, and its cells
wait for the loop to end. A cell made outside every beat still lives to
exit.

**Open.** senro and kq pin a kanso from before this. The app's memory is
worth re-measuring from senro's own tree once its pin moves.

## 2026-11-10 — a write that would wait yields its fiber

Fixes #2407.

**What was wrong.** `net/write` wrote to a blocking socket and came back
when every byte was taken. A fiber waiting to read or to accept yields to
the scheduler (#2208, #2405), and a write did not. While one client's
socket was full, no other fiber ran, on either engine. The issue was read
from the code and had no case. The case turned out worse than a stall: a
program that is both ends of a connection and writes more than the
sockets between them hold never ends, because the fiber that would read
is one of the fibers the write holds.

**What changed.** Inside the scheduler a write asks the connection for
what it will take without waiting. When it will take no more, the fiber
is left holding the write and the count of bytes gone, and goes back in
the queue. The scheduler's wait on sockets (`k_sched_await`, `awaited`)
asks about room for those fibers, where it asks about arrival for a
reader. The fiber picks up at the byte it stopped on, so no byte is
copied to remember the rest. Natively the part-done write is one more
description (42: the write, and how much has gone); the interpreter's is
`Desc::Sending`.

Outside the scheduler nothing changes. A write at the top of a program
has no other fiber to yield to and waits where it stands.

**TLS.** A TLS session writes whole records and has no call for "as much
as fits". A secured connection is asked only when its socket says it has
room, and is handed at most one record, 16,384 bytes, each time. That
write can still wait briefly if the room is less than a record. It is the
same on both engines.

**The specs.** `tests/golden/micro/a_writer_waits_for_its_reader` is the
program that is both ends: twelve megabytes written to a reader in the
same group. It never ended on either engine and now prints the count.
`tests/a_slow_reader_holds_only_its_own_fiber.rs` is the issue's case
with clients outside the program: one asks for twelve megabytes and
reads none of it, and a second is answered while the first still reads
nothing. With the yield taken out of both engines the second client's
read timed out natively, where the test stops, and the micro program was
killed at twenty seconds on each engine.

**The scheduler's loop.** The first build put the write into both
switches and cost programs that open no socket. Each cost had one cause,
and each was found by a row that had no reason to move.
- deepbench rose 1,867,728. 1.2 million of that was in `k_exec`, whose
  entry had gained one `mov`: a changed switch made clang keep the
  function's argument in a second register. `k_exec` is as it was. A write outside the scheduler has no
  other fiber to yield to, and a part-done write never reaches it,
  because only the scheduler steps a fiber it holds. The write's step is
  its own function, out of line.
- Every program rose 443. `send` with `MSG_DONTWAIT` says "what you can
  take now" in one call, and was one more name for the loader to find at
  every program's start. The write sets the socket not to wait with
  `fcntl`, writes, and sets it back; both names were already there.
- With those gone, runbench fell 383,692 at the release tier and rose
  415,722 at the dev tier, all of the rise in `k_schedule`. The wait on
  sockets was inlined into the scheduler's loop, and its body decided
  how the loop kept its own registers. Out of line and called with the
  addresses of the loop's fiber table and clock, it was better at the
  release tier (a fall of 800,791) and worse at the dev tier (a rise of
  767,174), because a variable whose address is taken lives in memory.
  Called with copies of the two, made only on the path that waits, it
  is better at both. Three builds of the same source that differ only
  in where the wait sits gave those three pairs.

**Rows.** The work rows are counts from this machine set by their
distance to main's reading here. Clang's rows and the tab's are CI's.
- `work_runbench` falls 1,216,098 to 952,666,853 and `work_runbench_dev`
  falls 320,187 to 2,805,647,188, for the wait leaving the loop.
- Three programs rise a little: `work_encodebench` 616 to 2,335,341,459,
  `work_livebench` 301 to 938,654,920 and `work_oneshot` 98 to
  9,979,851. The other ten are unmoved.
- The interpreter's step has one more arm to pass: `interp_instructions`
  rises 7,995 to 680,822,127. Its send is out of line and cold, as its
  wait on sockets already was.
- The write's step and the wait as a function are 928 bytes of machine
  code in every program, and the fourteen text rows sum to `text`
  5,919,865.
- Clang's rows, from CI: `codegen_instructions_release` falls 109,950 to
  372,070,546 and `codegen_instructions_dev` rises 5,675 to 116,944,920.
- The tab's run of the compiled corpus, from CI, rises 40,000 to
  `browser_run_instructions` 91,172,686. Its other three rows hold. I
  did not look for where the 40,000 are.

## 2026-11-10 — a lazy binding under two holders keeps its slots in tenure

Fixes #2415.

**What was wrong.** Natively, a lazy binding held by two values could be
read after its argument had been written over. The interpreter printed
the program's answer and the native build ran out of stack, on main and
on a compiler from 2026-11-07.

A cell is malloc'd and keeps its identity, so the copy walk copies its
slots (its arguments, and its result once forced) and writes them back in
place. The copy map then says the cell was seen, and a second meeting in
the same walk returned at once. An ordinary node met first from the carry
pair and again under tenure is copied a second time, into tenure, because
the tenured parent is pruned at by the next walk and the pair is written
over the stage after. The cell had no such second copy. Its slots stayed
in the pair.

`k_slots_survive` refuses to share a node that holds a cell directly, so
a tenured list of cells is walked again at every stage. It asks only about
the immediate interior. A tenured `[[c]]` has an interior of ordinary
survivors, and every later walk pruned there without reaching the cell.

**How it was found.** Read from the code while the registry was built
(#2408), and filed as a hazard without a program. Three programs later
it had one. The first two never tenured anything. The third took the
shape of a fixture whose counters show tenure, and still answered right.
A print on the cell arm showed the sequence happening in it all the
same: first meeting from the pair, second under tenure, and no walk
after. The answer had stayed right because later laps wrote less into
the pair than the lap that planted the cell. Carrying a list one string
longer each lap wrote past the argument, and the program died.

**The fix.** The walk keeps the slots of each cell it meets from the
pair, as it found them (`k_cellsave`, emptied when the copy map moves
on). A second meeting under tenure copies them again from that record,
into tenure, and the map then says the cell is settled. The originals
are needed because the first meeting wrote over the cell's own slots,
which an ordinary node's copy never does.

**The specs.** `tests/golden/mem/a_lazy_binding_under_two_holders_outlives_the_first`
is the program: one cell under `[c]` and `[[c]]`, the first dropped a lap
later, nineteen more laps each carrying a longer list. Its counters pin
that the chain staged and tenured (`ten_blocks=1`, `evac_allocs=1806`),
so a change that stops it reaching the path moves a number.
`tests/golden/micro/a_lazy_binding_under_tenure_is_read_twice` reads the
binding at lap 10 as well, when only the tenured holder has it, and
again at the end. Both were red on main natively.

**Rows.** Counts from this machine set by their distance to main's
reading here; `work_runbench`, clang's rows and the tab's are CI's.
- A walk pays about three instructions for each cell it meets from the
  pair. `work_runbench` rises 47,849 to 952,714,702 and
  `work_runbench_dev` 47,787 to 2,805,694,975 for the run program's
  16,050 cells. `work_livebench` rises 459 to 938,655,379,
  `work_pendbench` 289 to 154,094,164 and `work_scanbench` 2 to 267,927.
- Seven fall, by layout: `work_encodebench` 1,266 to 2,335,340,193,
  `work_deepbench` 1,098 to 198,293,720, `work_oneshot` 500 to
  9,979,351, `work_widebench` 23, `work_digestbench` 18, and
  `work_jsonbench` and `work_readbench` 3 each.
- The record is 496 bytes of machine code in every program, and the
  fourteen text rows sum to `text` 5,926,809.
- Clang compiles the record: `codegen_instructions_dev` rises 7,036 to
  116,951,956 and `codegen_instructions_release` 72,166 to 372,142,712.
- The tab runs the same runtime, and `browser_run_instructions` rises
  893 to 91,173,579.

**The floor.** Welfare reads 95.09928563621176 with these rows, under the
95.0994448300099 that #2414 banked. The objective's header sets a fix
that makes the engines agree outside the sum, so the floor moves to what
the fix costs and its history says so.

## 2026-11-10 — gavel: senro's page data and component state travel in a binary encoding

senro's design note said "JSON first; a binary encoding only if a
measurement says the JSON is the cost." Clay, overriding it: "of course it's
an extra cost. you KNOW it's a bottleneck."

Page data (a `senro-view` answer) and component state (the state a page
holds and the state posted to `/senro/act/<component>`) travel in an encoding
kata derives from the template's shape. It is positional: no field names on
the wire and no schema file. Both ends are compiled from the same templates,
so the shape is known on each, and a page built by one compile refuses data
from another. JSON stays wherever something outside senro reads the data.

Owed, on #2416: a benchmark of page updates and component actions baselined
under JSON first, as the 2026-09-05 corpus-first gavel asks; the encoder and
decoder derived for each page and component, on the server and in the
browser module; a differential golden that a page rendered from the encoded
data is byte-identical to the server's rendering; a fuzzer for the codec run
to silence; the guard against data from another compile; and the numbers
before and after.

## 2026-11-10 — gavel: keyed list matching and compiled updates are recorded, not built

Clay ruled that two ways of making senro's pages update faster are written
into senro's `design/live-views.md` as later work, and that neither is built
yet (#2418).

Keyed list matching: the page script's morph matches children by position,
so a list that gains an item at the top re-patches every element after it.
Matching on a stable key moves the existing elements, with `moveBefore()`
where the browser has it and `insertBefore` elsewhere. Compiled updates:
kata knows which fields each part of a template reads, so it could compile a
template into direct updates of the nodes that show a changed field. A
virtual DOM is declined, and the note says so.

Each is measured before it is built. The issue gives an order for when the
time comes: keyed matching, then the binary encoding, then compiled updates
if a benchmark shows the morph is the cost. The encoding is ruled and owed
on #2416, so cloud reads the order as placing the two later items around it
and builds the encoding now. The note went in with senro#105.

## 2026-11-10 — a built module's wasm instructions can be counted from outside the tests

#2416 owes a benchmark that counts what a page update costs in the browser,
and senro had no way to count it. kanso's own browser rows are counted by
`tests/support/wasm32.rs`, which runs a module against the runtime under
wasmi and counts the instructions executed, the same on every machine. That
host was reachable only from kanso's tests.

`scripts/count_module.rs` is a cargo example that runs it on any module
`kanso build --wasm` wrote:

    cargo run --release --example count_module -- runtime.wasm app.wasm \
        NAME=@file /path=@file

The module's output goes to stdout and `instructions=` and `grown_bytes=`
to stderr. `NAME=@file` is a variable read from a file, since a page event
has newlines in it. `/path=@file` is a file the module finds at that path
with `os/read_bytes`, because a variable cannot carry a zero byte or bytes
that are not utf-8, and the encoding #2416 asks for is binary. It is an
example because wasmi is a dev-dependency, and the `kanso` binary a user
installs does not carry a wasm interpreter. No row moves: nothing in the
binary or the library changed.

The host under it answered every filesystem call with ENOSYS. It now
serves the files it is given: one directory, `/`, and `path_open`,
`fd_seek`, `fd_read` and `fd_close` over them. A path it was not given
answers ENOENT, which reads in the program as `os/file_not_found`, as
native reads it.

`tests/a_built_module_is_counted.rs` builds a program that prints a
variable and the bytes of a file, runs the example's `answered` on it and
reads the count the host gives when called directly. Its first version ran
the example's binary and passed against an example edited to report one
instruction too many: `cargo test --test` does not rebuild examples, so it
read the binary an earlier build had left. A second version built the
example from inside the test, and each of `cargo test` and the nested
`cargo build` then rebuilt what the other had left, three minutes a run.
The spec now compiles the example's source into itself with `#[path]`, so
there is no binary to be stale, and with the edit it fails on 38,157
against 38,156.

The reader of a module's `kanso.link` section moved from
`tests/build_wasm.rs` into the shared support, where the example reads it
too.

A first reading with it, on senro's `spec/runs/pages` views and a list of
fifty posts (6,562 bytes of JSON): the browser module executes 3,991,398
instructions to turn the page's data into HTML. Splitting the event and
decoding the JSON, with nothing rendered, is 1,064,700 of them, and the
split alone is 145,817. Rendering is the larger part.

A second, on a tagged binary encoding written in kanso as a prototype,
field names still on the wire: the same page is 5,044 bytes and decodes in
1,708,928 instructions, where the JSON decodes in 918,883. By kind of
value, binary against JSON: 905 against 1,664 instructions for a small
int, 1,828 against 1,915 for a string of twenty bytes, and 17,729 against
14,203 for a map of five one-letter keys. std/json reads a key with a
builtin and keeps short tokens in a cache, and the prototype reads a key
as any other string and puts it in a map, so the page's two hundred and
fifty keys are where it loses. The scalars favour the binary encoding and
the maps do not, which is the case for the positional layout the ruling
asks for: a record's fields in order, with no key read and no map built.
These are this machine's numbers from a runtime built here, and the
benchmark #2416 owes will pin its own from CI.

## 2026-11-10 — senro's pages travel as bytes kata derives

#2416, built in senro (senro#107 for the benchmark, senro#108 for the
encoding). A page's data and a component's state leave the server in a
binary encoding, and JSON stays where something outside senro reads the
data.

kata knows a page's shape from the fields its template reads, so it writes
a record as those fields in a fixed order and a list of records as a count
and the records, with no name and no tag between them. Only the leaves are
tagged: one byte that says what follows, and that holds the whole of a
small int or the length of a short string. The server gets a packer for
each page. The browser module gets a record type and a reader for each
record a template reads, and draws the page with the template's ordinary
compile; the compile that read fields out of a map is deleted. The first
four bytes are a hash of every page's shape in the module, and bytes from
another build draw nothing.

`bench/pages` is a board of fifty records in a layout with one component.
Its rows are wasm instructions counted by `count_module`, taken from CI
under JSON and again under the encoding:

    row                                   JSON      binary
    page as data, bytes                 15,191       9,636   -36.6%
    component state, bytes                  43          36   -16.3%
    server: records, component drawn   405,267     423,263    +4.4%
    server: page answered as data    1,861,329   1,269,908   -31.8%
    server: page answered as HTML    5,720,720   5,074,507   -11.3%
    browser: module start               13,905      13,764    -1.0%
    browser: page drawn from data    7,092,359   6,302,146   -11.1%
    browser: one component action      172,445     230,189   +33.5%

A page is a third smaller and a third cheaper to write. Drawing it in the
browser falls by a ninth, because drawing the markup is most of a page
update and the encoding does not reach it.

A component action costs more than it did. Its state is held in the page
as base64 and read into a map, both in kanso code, where JSON was read and
written by the runtime. Measured apart on a 33-byte state: spelling it as
base64 costs 26,200 instructions and reading it back 28,000, against 15,600
and 30,700 for `json/encode` and `json/decode` of the same value, and
reading the tagged bytes into a map costs 58,300. Two issues carry what
that found. #2420 is base64 in the runtime. #2421 is what a decoder
written in kanso pays for each value it reads: a returned record costs
607 instructions a value when it is destructured in a parameter with its
int first, 1,013 with its value first, and 1,673 when it is bound to a
name and read by its fields, which is the only way a caller in another
module can take it.

The codec was fuzzed before it was trusted: `wire` and a second writing of
the format in JavaScript each read what the other wrote for 2,000,000
random values with no disagreement. Three breaks were watched red first.
One of them, a 64-byte string written behind the tag for 63, passed 2,000
values because no string was 64 bytes long, and the strings are now aimed
at exact byte lengths. The long run stopped once, at a string beginning
with U+FEFF that the reference's `TextDecoder` dropped, which was the
reference's fault.

Decided on the way, by the committee and recorded on #2416. Floats travel
as their decimal text, since kanso cannot read a float's bits and no page
in the examples sends one. Component state stays tagged: writing it
positionally needs a component to declare its state's type, and that waits
on what #2420 and #2421 leave to win. Only a program that names `rendered`
gets the readers and row types, so a server still refuses a misspelled
field at the template's line.

Nothing in kanso changed, so no row here moves. The site's views guide
describes the bytes, and its senro pin moves to 43a2ca5.

## 2026-11-10 — a session's trip through base64 joins the corpus

The corpus half of #2420, ahead of its fix. senro's binary encoding (#2416)
holds a component's state in the page as base64, and its benchmark showed
an action costing more than it had in JSON, with base64 the larger part.
senro also spells and reads base64 for every request that carries a
session. `std/base64` is written in kanso: it joins a string for each group
of three bytes, reads each letter out of its alphabet with a slice, and
builds the bytes it decodes by pushing onto a list. No benchmark held a
trip through it, so a fix would have scored as nothing.

`sealed` joins the run program: 800 passes, each spelling a cookie of about
sixty bytes and reading it back. Built with the phase and with it reading
nothing, the program retires 991,852,481 instructions against 952,721,301
in this container, so the phase is 39,131,180 of them, 3.95%, and a trip
costs about 49,000. The same trip joins the interpreter's corpus at 15
passes, where it costs 3.1 million instructions a trip.

CI's rows. work_runbench 952,714,702 -> 991,852,372 and work_runbench_dev
2,805,694,975 -> 2,894,015,349. interp_instructions 680,822,127 ->
727,482,475, interp_allocs 984,852 -> 1,043,722 and interp_peak_bytes
1,377,499 -> 1,489,302. In the tab, browser_compile_instructions
358,139,032 -> 413,857,607, browser_compile_peak_bytes 862,043 -> 975,975
and browser_run_instructions 91,173,579 -> 95,460,118, with
browser_run_peak_bytes still 1,572,864. The tab's compile rises by 15.6%:
the corpus now imports `std/base64` and through it `std/bits`, and the tab
compiles both. The compile rows that read other programs did not move.

The run program's counters rise with the phase: run_allocs 2,372,861,
run_alloc_bytes 261,860,897, run_perm_allocs 161, run_beat_iters 197,248,
run_push_mut_slow 1,658,292 (and run_push_mut_fast 1,398,651),
run_bytes_malloc 9,468, run_str_scans 52,203, run_str_scan_bytes
19,043,424, run_sh_str 19,617,504, run_sh_buf 102,043,040 and run_sh_bytes
43,560, with run_buf_reuse at 7,390. `std/base64` joins the program's
emitted code, so runbench reads defines=975 calls=10294 branches=9407
lines=80567 and the summed rows emitted_other_defines 2,406,
emitted_other_calls 22,268, emitted_other_branches 19,938 and
emitted_other_lines 185,688. Its machine code is 912,641 bytes, and the
summed text 5,954,425.

The floor is re-based and the score is not a gain. Seven baselines are
scaled by the new reading over the old, as the last corpus additions did,
so the meta moves only by their rounding, 95.09928563621176 to
95.09928566956496.

## 2026-11-10 — base64 is spelled and read by the runtime

#2420. `std/base64` was written in kanso, and a trip through it for a
cookie of about sixty bytes cost 49,000 instructions natively and 3.1
million interpreted. The runtime now does both halves: `k_base64_spell` and
`k_base64_read` in C for a built program, and `kanso::base64` in Rust for
the interpreter. The library is three wrappers and the two reasons
`decode!` gives.

What the library answers is unchanged. `decode` answers a list of byte
values or none, and `decode!` the same two reasons. Reading stays as
lenient as the kanso was, which is more lenient than its own comment said:
an `=` in a group's third place ends the group after one byte whatever the
fourth holds, a padded group may be followed by more groups, and the bits
a padded group leaves over need not be zero. One thing changes. `encode`
given a list holding a number that is not a byte spelled garbage, and now
stops the program with "base64/encode takes bytes", as `text/bytes` stops
for a value that is not a string.

The committee weighed writing it better in kanso instead, which is what
`std/sha256` got. senro's `wire` already had that version: one byte
builder, with each letter computed. For a 33-byte state there and back it
costs 54,200 wasm instructions, about a third of what the string-joining
library cost in the same place, and still about 800 a byte. What kanso
code pays for each value it reads is #2421's, and is worth taking out in
the compiler for every decoder. Base64 is a byte transcoding with one
right answer, the kind of kernel `std/json` already keeps in the runtime
for its escapes, and a server runs it on every request that carries a
session. So the kernels go in the runtime.

The harness came first. `tests/base64_agrees_with_its_reference.rs` holds
all four kernels to a reference written apart from them, which spells by
laying the bytes out as bits and taking six at a time: 2,000,000 spellings
and 2,000,000 readings for the interpreter's, and 400,000 of each for the
runtime's, whose text is lifted out of the runtime and given each case in
a buffer that ends at a page nothing may touch, in exactly the room the
runtime promises. The first writing used the address sanitizer, which CI's
clang cannot link. The texts
are true spellings, spellings with characters swapped for letters, pads
and the characters either side of each run of the alphabet, spellings cut
short or run long, and ragged strings from that pool. It was watched red
three ways: a wrong shift in the interpreter's second byte with a five-bit
mask on the runtime's third letter, both caught within three cases; the
runtime refusing a padded third place with the interpreter taking ragged
text, caught at the first case; and the runtime taking ragged text, which
stopped at a read past the end. With the fence in place of the sanitizer
it was watched red again: ragged text taken and a terminator written past
the speller's room each stop the harness with a fault, and a five-bit mask
on the third letter parts from the reference at byte 18.

The old library was the second oracle. A program holding it as a module of
its own compared it with the new one on 300,000 generated cases, bytes
spelled and texts read, and found no disagreement. Broken two ways, the
old copy disagreed on 98,002 and 95,029 of them.
`micro/base64_spells_and_reads_on_every_engine` pins RFC 4648's vectors
both ways, a list in place of bytes, and each edge of what reading takes,
and `runtime/base64_encode_takes_bytes` pins the refusal on both engines.

A trip now costs about 2,800 instructions natively, most of it building
the cookie and the list of sixty byte values `decode` answers. The run
program's work falls from 991,852,372 instructions to 954,999,153 and its
dev-tier build's from 2,894,015,349 to 2,808,131,450. The interpreter's
corpus falls from 727,482,475 instructions to 681,004,692, with
interp_allocs 1,043,722 -> 988,431 and interp_peak_bytes 1,489,302 ->
1,400,327. The tab compiles that corpus in 366,327,369 instructions where
it took 413,857,607, the library's kanso and its import of `std/bits`
being gone, with browser_compile_peak_bytes 975,975 -> 867,315, and runs
it in 91,307,704 where it took 95,460,118.

The run program's counters fall back toward what they were before the
phase joined: run_allocs 2,349,661, run_alloc_bytes 256,551,969,
run_perm_allocs 135, run_push_mut_slow 1,650,292, run_bytes_malloc 7,868,
run_str_scans 36,202, run_str_scan_bytes 19,043,360, run_sh_str
19,156,704, run_sh_buf 97,305,312 and run_sh_bytes 24,360, and
`base64_calls=1600` is the kernels' presence, printed only by a run that
calls them. Two counters fall that the trend gate reads as worse,
run_push_mut_fast to 1,238,867 and run_buf_reuse to 3,390. Both are
exactly what they read before the phase joined, so what the phase added to
them was the kanso library's pushes and buffer reuses, and those are gone
with it. runbench's emitted code loses the library's kanso: defines=959
calls=9986 branches=8989 lines=77949, and summed emitted_other_defines
2,390, emitted_other_calls 21,960, emitted_other_branches 19,520 and
emitted_other_lines 183,070.

Several rows rise. The runtime is linked whole, so every program's machine
code gains the kernels and their wrappers, 2,032 bytes each: runbench
falls to 889,489 with the library's kanso gone, the other thirteen rise,
and the summed text goes 5,954,425 -> 5,957,689. And the compiler knows two
more builtins: compile_instructions 24,152,633 -> 24,154,945,
entry_instructions 156,082,814 -> 156,095,335, library_instructions
73,608,570 -> 73,612,703 and emit_instructions 5,643,537 -> 5,644,746.

Clang and the linker do more for the dev tier and less for the release
tier: codegen_instructions_dev 116,951,956 -> 116,980,709, a rise of
28,753, and codegen_instructions_release 372,142,712 -> 372,052,045, a
fall of 90,667. A build now has less of runbench to compile and more
runtime to link; which of the two decides each tier was not isolated.
Three programs that never call base64 link the grown runtime and move by
layout: work_encodebench rises 350 to 2,335,340,543, work_oneshot rises
154 to 9,979,505 and work_livebench falls 644 to 938,654,735.

`runtime/base64_decode_takes_a_string` pins the other refusal, a value
that is not a string handed to `decode`, on both engines. Welfare rises
from 95.09928567 to 95.25752069, and the floor is banked there.

## 2026-11-10 — a decoder read across a module joins the corpus

#2421, corpus first. A decoder written in kanso answers each value with
the position after it, as a record of two fields, and no benchmark built
one. `framed` is such a decoder, a stream of numbers at one byte or two,
and `taken` reads it from another module. A record cannot be taken apart
across an import, so `taken` binds what `framed/at` answers to a name and
reads its two fields, which is how the readers senro derives call `wire`.
Each pass also has `framed` sum the stream itself, with the record taken
apart in a parameter, value first as its type declares it. The run program
makes 1,500 passes over 64 numbers, and the interpreter's corpus carries
the same two loops in its one module for 12.

The baseline already says something about the leads. Both loops build
every record on the heap: run_sh_rec goes from 4,723,232 to 17,011,296,
which is 1,500 passes of 128 reads at 64 bytes and one record more. So the
loop that takes the record apart in a parameter misses the by-value path
too, on the order of its fields.

The run program's work rises: work_runbench 954,999,153 -> 997,844,059
and work_runbench_dev 2,808,131,450 -> 2,924,577,667. Both are this
container's readings less 109, which is what every work row here reads
above CI's. The phase is 42,844,906 of the program, 4.29%. Its other
counters: run_allocs 2,541,664, run_alloc_bytes 268,840,291,
run_perm_allocs 136, run_beat_iters 390,812, run_evac_allocs 5,617,
run_evac_bytes 4,721,376, run_append_fast 5,329,378, run_append_grow
1,081, run_bytes_malloc 7,869 and run_sh_bytes 24,384. runbench's emitted
code reads defines=978 calls=10213 branches=9202 lines=79617, the summed
rows emitted_other_defines 2,409, emitted_other_calls 22,187,
emitted_other_branches 19,733 and emitted_other_lines 184,738, and its
machine code is 900,801 bytes, so the summed text is 5,969,001.

The interpreter's corpus rises: interp_instructions 681,004,692 ->
711,444,159, with interp_allocs 1,026,142 and interp_peak_bytes 1,487,351.
The tab's rows rise with it: browser_compile_instructions 366,327,369 ->
390,565,528, browser_run_instructions 91,307,704 -> 93,856,364 and
browser_compile_peak_bytes 867,315 -> 958,848. The first sizing was 40 passes,
which cost the interpreter 95 million instructions, about 18,500 for each
value read. That looked like a cliff and is not one: building the stream
costs it about 7,000 instructions a number at 64, 256 and 1,024 numbers
alike, the bare loop about 5,200 a value, the bound record 16,900 and the
record taken apart 12,500. The phase was sized down to 12.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.25752068 before and 95.25752070 after.

## 2026-11-10 — a decoder's answer stays in registers

#2421, and #2428 on the way. The by-value return packs one int beside one
value into three words, and it existed for the json scanner's `_parsed`,
whose int comes first and whose callers take it apart in a parameter. A
decoder a library exports is written the other way round in four places,
and each kept its record on the heap.

The int had to be the first field. `read value next` declares the value
first, as a type that says what was read and where to go on would. The
escape analysis now asks inference which of the two fields is an int and
nothing else, the emitter builds and takes apart in that order, and the
runtime has `k_parsed_box_back`, `k_parsed_words_back` and
`k_parsed_int_back` for the second order. The three words are laid out the
same either way.

A guard's early return was not a tail. `return read b p if b < 128`
refused the type, because only a tail `if` and a tail construction were
recognised. A guard in tail position is read as the tail `if` it is.

A constant of the type, named bare in a tail, was not counted as answering
it, so a decoder that answers `lost` for a short read was said to answer
something else. It counts now. Returned from a by-value function, a kept
constant is a built record, and the return took any value it was handed
for a failure and sent its tag and pointer back as words; the constant's
words are read off it there.

A record bound to a name was built on the heap again for every field read
off it, since a getter is handed a record. `r.next` on a name that holds a
record by value reads the field out of the words, and a failure riding in
them comes back as itself. `.reason`, `.cause` and `.origin` are left to
the getter, which answers them for an err whatever record was expected.
The record was also built once more on its way into a call in any program
with a lazy binding, to ask whether it was a cell to force. It is not
asked.

The loop's carry was the fifth. A loop that rewinds its arena each lap
staged a by-value record built, a heap record a lap, and
`mem/a_failure_crosses_a_beat_carry_in_two_words` said so in its comment.
On the way that skips the rewind, taken when another argument may be a
failure, the record stayed three words, the two ways met in a phi of two
types, and clang refused the module: #2428, a program main cannot build.
The carry stages the record's two fields, a slot each, and packs them on
the far side, by value on both ways. When the fields would not fit the
buffer's eight slots the record is staged built and its words read back.

A record built at the loop's own call was the sixth, #2430, and it also did
not build on main. Written at the call for a by-value slot, a record is
packed into words in place and has no operand among the emitted ones. The
carry staged the emitted operand at every carried position and wrote
`call void @k_carry_stage(%KValue )`. The packed words join the emitted
operands at a carried slot and cross as the two fields. A record built on
the way into the loop from outside it, which is not a tail call, is still
built and read back, once for each entry.

A boxed answer was the seventh, #2432, and on main it is a wrong answer
with no complaint. A name a module declares can also be one of an
import's: `std/list` exports `first`, so a bare `first` in a module that
declares one calls into the module's own overload space, and a group
there stays boxed. A decoder that answers in registers and ends in such a
call is handed a built record. Its return took whatever value it was
handed for a failure, sent the record's tag and pointer on as the value,
and the caller read the whole record where the value belonged and 0 for
the position: main prints `shared/read 2 97 then 0` where the
interpreter prints `97 then 2`. The analysis now keeps the groups that
answer the type and stay boxed, a call to one is noted where it comes
back, and the return reads the record's words off a noted value. Any
other value still goes back as a failure's two words. Reading the words
off every value not known to be a failure was tried first and put 234
calls into the decoder's emitted code, since inference rarely knows.
`micro/a_bare_name_shared_with_an_import_answers_its_record` holds every
engine to the interpreter's two lines.

Measured. `mem/a_record_held_by_value_crosses_a_loop_s_rewind` did not
build and reads sh_rec 0. `mem/a_record_built_at_a_loop_s_call_crosses_its_rewind`
did not build either, and reads the one record its entry builds over 76
laps. `mem/a_record_whose_int_comes_second_goes_back_by_value`
read sh_rec 2,624 and reads 0. `mem/a_failure_crosses_a_beat_carry_in_two_words`
falls from 64,000 to 0, and three scanning fixtures lose a record each.
The run program's decoder phase builds none: run_sh_rec 17,011,296 ->
4,723,168, run_allocs 2,541,664 -> 2,349,662, run_alloc_bytes 268,840,291
-> 256,552,163, run_evac_allocs 5,617 -> 2,617 and run_evac_bytes
4,721,376 -> 4,625,408; scan_allocs 97 -> 96 and scan_sh_rec 1,872 ->
1,808. work_runbench falls 997,844,059 -> 982,946,782 and
work_runbench_dev 2,924,577,667 -> 2,894,927,376, with the phase at about
146 instructions a value where it was 223. The tab runs its corpus in
93,283,820 instructions where it took 93,856,364.

What rises. The runtime's three conversions are linked into every
program, 368 bytes each; scanbench falls 512 and runbench 800, and the
summed text goes 5,969,001 -> 5,972,105. The emitter asks more:
emit_instructions 5,644,746 -> 5,651,651. The tab's compile rises with
it, browser_compile_instructions 390,565,528 -> 393,730,774 and
browser_compile_peak_bytes 958,848 -> 965,297. Clang and the linker read
a little more: codegen_instructions_dev 116,980,709 -> 117,010,179 and
codegen_instructions_release 372,052,045 -> 372,177,641, both CI's
readings. The container's move added to CI's last reading had predicted
67 and 80,559 fewer, so that sum is a guide for these two rows and no
more. Three programs move by
layout with the grown runtime: work_encodebench rises 350 to
2,335,340,893, work_oneshot 854 to 9,980,359 and work_livebench 742 to
938,655,477; work_scanbench falls 84 to 267,843 with its record.
runbench's emitted code reads defines=975 calls=10169 branches=9172
lines=79632 and scanbench's defines=331 calls=3190 branches=3019
lines=26106. Welfare rises 95.25752070 -> 95.29096728, and the floor is
banked there.

Checked against the interpreter. `micro/a_record_held_by_value_reads_like_one_built`
walks a short read, an early answer, a field read off a bound name, the
record handed whole and kept in a list, a string carried round a rewinding
loop, a position too wide for a word and a failure where the record was
expected, on every engine and through the tab's translator. With the
constant's branch taken out it prints the record where its field should
be. The 2,163 programs of the numeric differential and the eight other
differentials agree.

What this does not reach. senro's `wire` is unmoved: its benchmark page
reads the same six rows with this compiler. Three things keep its `read`
on the heap, found by asking the analysis which test refused it. Inference
cannot say the position is an int: a count is read into the same `value`
field as every other value, so the position after a string, the position
of its bytes plus their count, may be a float, and the tag `bs[p]` is
unknown because nothing says `bs` is bytes. `wire` hands its decoder out
as a value, to read a list's items. And it passes the record whole to a
helper that binds it. The analysis is all or nothing for a type, so one
such use anywhere costs every read. The first is senro's to write
differently, with a record of two ints for a count and `:int` where a
position comes in from outside; the other two are the next piece of
#2421 here.

Two ratchet rows moved with the change, and the ratchet's own run found
both. `a thunk site returns a record unboxed` patched one line that is six
since the field order became an argument, so its mutation no longer
applied; it takes the call from its opening to its close now, and clang
refuses `micro/a_lazy_record_call_somebody_makes_still_builds` under it
as before. `a failure read as a record on the far side of a carry` had
gone blind. Its mutation takes out the ask `k_parsed_words` makes before
it reads a record's fields, and its fixture reached that ask through the
carry, which unpacks no built record now that it stages the two fields.
The ask is still needed wherever a built record is unpacked, and a
failure a boxed group hands back to a by-value return is one such place:
`mem/a_failure_handed_back_by_a_boxed_group_stays_a_failure` prints
`stopped: end of input` with the ask and stops on a runtime error
without it.

A third row ended CI's runner, twice. `a group answering other values by
value` skips the check that every arm of a group answers the record or a
failure. On main `micro/a_greedy_run_of_one_character_backs_off_in_order`
still prints its golden under that mutation. Here more groups are
candidates, regexp's among them, and the same fixture builds into a binary
that allocates without end. Nothing bounded a program a golden
runs, so the kernel ended it once the machine was out of memory, and on a
runner that ends the job with a shutdown signal and a log that cannot be
read. `run_kanso_env` in tests/golden.rs now hands each program 8 GiB of
address space on Linux, where the ratchet runs. Under the bound the binary
is refused an allocation and says `out of memory`, and the row reads red
in 24 seconds. `a_program_a_golden_runs_may_map_only_so_much` reads the
limit from a shell the program starts, on both engines, and reads
`unlimited` without the bound. The whole golden suite passes under it. A
test that starts kanso some other way has no bound yet (#2436). The spec's
first draft read `/proc/self/limits` and found that the native runtime
reads such a file as empty (#2435).

## 2026-11-11 — a decoder handed out joins the corpus

#2421 again, corpus first. The phase that reads a stream through an
exported decoder took the record off the heap in #2431 for the two ways it
was read: bound to a name across a module, and taken apart in a parameter.
senro's `wire` reads a third way, and it is the one that keeps its record
on the heap. A list's reader is handed the function that reads one item:
`wire` hands `at` to its own list reader, and kata hands that reader the
function it wrote for a record. No benchmark handed a decoder out.

`framed/each` is that reader. It is handed `at` and reads the stream
through it, and `taken` adds it to each pass. `at` says its position is a
whole number, `p:int`, because a function handed out can be called with
anything and the record goes by value only when its position is an int.
The interpreter's corpus carries the same loop as `through`.

The baseline shows what handing a decoder out costs. A function handed out
is called through a wrapper that answers one boxed word, so the analysis
keeps the type on the heap for every caller the decoder has. Handing `at`
out once puts all three readers back on the heap: run_sh_rec 4,723,168 ->
23,155,168, which is 1,500 passes of 192 reads at 64 bytes more, the 64
through the value and the 128 direct reads #2431 had freed. run_allocs
2,349,662 -> 2,637,662, run_alloc_bytes 256,552,163 -> 274,984,163,
run_beat_iters 390,812 -> 582,812, run_evac_allocs 2,617 -> 5,617 and
run_evac_bytes 4,625,408 -> 4,721,408.

The run program's work rises: work_runbench 982,946,782 -> 1,035,809,998
and work_runbench_dev 2,894,927,376 -> 3,028,058,391, both this
container's readings less 109. The reader is 52,863,216 of the program,
5.10%. runbench's emitted code reads defines=982 calls=10293
branches=9278 lines=80296, the summed rows emitted_other_defines 2,411,
emitted_other_calls 22,245, emitted_other_branches 19,794 and
emitted_other_lines 185,367, and its machine code is 905,969 bytes, so
the summed text is 5,978,073.

The interpreter's corpus rises: interp_instructions 711,444,159 ->
724,216,921, with interp_allocs 1,038,038 and interp_peak_bytes 1,496,663.
The tab's rows rise with it: browser_compile_instructions 393,730,774 ->
398,181,377, browser_compile_peak_bytes 965,297 -> 968,782 and
browser_run_instructions 93,283,820 -> 95,675,366.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.29096728 before and 95.29096738 after.

## 2026-11-11 — a decoder handed out keeps its callers in registers

#2421, the piece senro's `wire` was waiting on. A group that answers a
record in registers could not be handed out as a value. A function value
is called through a wrapper that takes and answers boxed words, and the
wrapper knew one return shape, so the analysis refused the type outright
when any group answering it was named as a value anywhere in the program.
One reader handed the decoder, and every caller it had went back to
building a record.

The wrapper now calls the group the way a direct caller does and builds
the record from the three words, with `k_parsed_box` or its second-field
twin, and a failure riding in the words comes back as itself. Whoever
calls the value reads one boxed word as before. The analysis no longer
asks whether a group is handed out, and the walk that collected every
name used as a value is gone.

The ratchet row for this, `a function value returned in registers`, broke
the check the analysis no longer makes, so its mutation would not apply.
It now breaks the wrapper: called as though every group answered a boxed
word, `micro/a_decoder_handed_out_answers_like_one_called` prints nothing
natively and the row reads red.

What a handed-out decoder still needs is its position said: `at bs p:int`.
A function handed out can be called with anything, so inference gives its
parameters every type, the position worked out from one may be a float as
far as it can tell, and a record goes by value only when its position is
an int. The corpus and both fixtures write the annotation.

Tried and taken back: letting a record be passed whole to a slot that
binds it, boxed at the call. `wire` does that in two places and the rule
that refuses it is syntactic, since binding the record to a name first is
allowed. It was written, and `mem/a_carried_list_frees_its_buffer` went
from 22 allocations to 1,218 and from sh_rec 512 to 77,056. Its `bag` is
rebuilt in place by a loop that passes it whole, the relaxation let `bag`
go by value, and every lap then built a fresh record at the call where
none had been built at all. A type passed whole on its hot path is better
left on the heap, and the old rule keeps it there. senro binds its record
first in those two places.

Measured on the corpus. `mem/a_decoder_handed_out_still_answers_its_callers_by_value`
reads sh_rec 512, the eight reads that go through the value, where it
reads 3,200 with the refusal put back. The run program's direct reads
come off the heap again: run_sh_rec 23,155,168 -> 10,867,168, which
leaves the 64 reads a pass that go through the value, run_allocs
2,637,662 -> 2,445,662, run_alloc_bytes 274,984,163 -> 262,696,163,
run_evac_allocs 5,617 -> 2,617 and run_evac_bytes 4,721,408 -> 4,625,408.
work_runbench falls 1,035,809,998 -> 1,020,737,307 and work_runbench_dev
3,028,058,391 -> 2,997,794,416, both this container's readings less 109.
The tab runs its corpus in 95,097,638 instructions where it took
95,675,366.

What else moves. The emitter does less with the walk gone:
emit_instructions 5,651,651 -> 5,632,306. The tab's compile rises, since
it now emits the by-value paths the refusal used to skip:
browser_compile_instructions 398,181,377 -> 399,455,088 and
browser_compile_peak_bytes 968,782 -> 969,319. interp_instructions reads
724,216,927, six more. runbench's emitted code reads defines=981
calls=10271 branches=9262 lines=80364, the summed rows
emitted_other_defines 2,410, emitted_other_calls 22,223,
emitted_other_branches 19,778 and emitted_other_lines 185,435, and its
machine code is 906,017 bytes, so the summed text is 5,978,121. Welfare
rises 95.29096738 -> 95.32525328, and the floor is banked there.

Checked against the interpreter. `micro/a_decoder_handed_out_answers_like_one_called`
reaches a decoder by name, through a parameter, through a partial, out of
a list and from a function that answers one of three, with a short read,
a position too wide for a word and a failure where the record was
expected. With the wrapper's boxing taken out it stops at a field read of
a number.

senro, in a container with this compiler and `wire` written for it: a
record of two ints for a count, `:int` where a position comes in, and the
record bound before it is handed on. Its benchmark page draws in
3,845,447 wasm instructions where it took 4,254,682, and an action runs
in 115,251 where it took 125,306. Those rows are senro's to pin from its
own CI once it takes this compiler.

## 2026-11-11 — a page's values through an escape join the corpus

#2427, corpus first. kanso has no call that replaces one text with
another. The idiom is `text/join (text/split s from) to`, a list of the
pieces and then a string from the list, and the escape senro's templates
lean on runs it once for each character it rewrites. No benchmark replaced
text, so nothing priced the idiom or would have scored a call that did the
work in one pass.

`replaced` is that escape as senro writes it: three searches for the five
characters, a value with none handed back as it came, and a split and a
join for each search that found something. A row is the mix a listing page
has, which is a title holding four of the five characters, an excerpt
holding an apostrophe, and seven values with nothing to rewrite. The run
program escapes 4,000 rows and the interpreter's corpus 80.

The baseline. The phase is 47,129,776 instructions of the run program,
4.41%, measured as the whole with it less the whole without in this
container. It runs the split and the join 28,000 times, and 20,000 of
those find something to rewrite. run_allocs 2,445,662 ->
2,709,581, which is 66 allocations a row, run_alloc_bytes 262,696,163 ->
274,019,555, run_beat_iters 582,812 -> 614,812, run_find2_calls 1,060,857
-> 1,168,857, run_str_scans 36,202 -> 72,202 and run_sh_str 19,156,704 ->
25,232,528. The rest of the run's rows the phase moves: run_perm_allocs
136 -> 151, run_sh_buf 97,305,312 -> 98,585,312, run_sh_bytes 24,384 ->
888,384 and run_str_scan_bytes 19,043,360 -> 21,093,397.

The run program's work rises: work_runbench 1,020,737,307 ->
1,067,867,083 and work_runbench_dev 2,997,794,416 -> 3,065,058,505, both
this container's readings less 109. runbench's emitted code reads
defines=992 calls=10401 branches=9373 lines=81321 and its machine code is
911,345 bytes. Summed over every benchmark but the decoder,
emitted_other_defines 2,410 -> 2,421, emitted_other_calls 22,223 -> 22,353,
emitted_other_branches 19,778 -> 19,889 and emitted_other_lines 185,435 ->
186,392, and the summed text 5,978,121 -> 5,983,449.

The interpreter's corpus rises: interp_instructions 724,216,927 ->
744,978,389, interp_allocs 1,038,038 -> 1,074,427 and interp_peak_bytes
1,496,663 -> 1,527,692. The tab's rows rise with it:
browser_compile_instructions 399,455,088 -> 417,017,327,
browser_compile_peak_bytes 969,319 -> 985,620 and browser_run_instructions
95,097,638 -> 100,630,653.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.32525328 before and 95.32525334 after.

## 2026-11-11 — one text written as another, in one pass

#2427's first lead, on the corpus the entry before this one added.
`text/replace s from to` joins `std/text`. The committee's reading is on
the issue: the call answers what `text/join (text/split s from) to`
answers in every case, so a program can swap the idiom for it and nothing
it prints changes. Occurrences are found left to right and the search
takes up again after the end of each. A text that holds none comes back
as it was, with no copy. An empty text to find occurs everywhere and is an
err with its own reason, as an empty separator is for `split`.

The harness came first. `tests/replace_agrees_with_its_reference.rs` holds
both kernels to a reference that walks one byte at a time and asks at
each whether the text to find starts there: 4,000,000 cases against the
interpreter's and 1,000,000 against the runtime's, 0 failures. The C is
cut out of the runtime's text and run with each buffer ending at a page
nothing may touch. With the copy made to read one byte too many the
harness stops on that page, and with the interpreter's search made to
resume one byte on instead of after the match it names the first case.

The kernel counts the occurrences with the search `split` uses, allocates
the answer once at its length, and copies. Its presence counter is
`replace_calls`, 28,000 in `bench/cost_golden_run.txt`: the escape asks
seven times a row, and five of those find something.

Where the err is born took two tries. The first wrote the empty case as a
second arm in the library, as `split` does. A program that names the call
then keeps a group of two arms that `inline_builtin_wrappers` cannot
rewrite, and the interpreter's corpus cost 1,670,000 instructions more to
start whether or not it called the group; a guard inside one arm cost the
same. That is #2434, and `split` is written the same way. The builtin
answers the err itself now, as `from_code` does, and the library holds one
forwarding arm. Its wrapper stays a real call so the err is born in
`text/replace`, where a caller may rescue it, and the list of builtins
that can give birth to an err has one home, `inline::BIRTHS_ERR`, which
the emitter reads to hand each its origin.

Measured on the corpus. The phase is 39,101,174 instructions of the run
program where it was 47,129,776, in this container. `k_b_split` falls
5,994,654 -> 22,654 and `k_b_join` 18,580,590 -> 14,124,590, and
`k_b_replace` takes 4,228,000, 151 a call. run_allocs 2,709,581 ->
2,573,581, 34 fewer a row, run_alloc_bytes 274,019,555 -> 269,477,283,
run_sh_str 25,232,528 -> 24,785,824 and run_sh_buf 98,585,312 ->
97,305,312. work_runbench 1,067,867,083 -> 1,059,838,481 and
work_runbench_dev 3,065,058,505 -> 3,057,251,731, both this container's
readings less 109. The tab compiles its corpus in 416,189,723
instructions where it took 417,017,327 and runs it in 100,428,884 where
it took 100,630,653, with browser_compile_peak_bytes 985,620 -> 984,597.

interp_instructions rises, 744,978,389 -> 745,491,463, on a collision in
the interpreter's table of callees. The same compiler on the corpus as it
was, splitting and joining, reads 744,767,711. Against that the call costs 723,752 more,
which is 1,915,016 more in `Interp::callee_missed` and 1,191,264 less
everywhere else. The interpreter remembers the last callee in each of 256
slots, a name's slot comes from where its text was allocated, and with
this program's names two busy ones share a slot: the table misses 24,661
times where it missed 4,399. interp_allocs 1,074,427 -> 1,069,565 and
interp_peak_bytes 1,527,692 -> 1,524,780.

Two things tried on that table and taken back. Two entries a slot took
the misses to 190 and `callee_missed` to 36,362, and made every hit
dearer: `callee_of_ref` 12,103,951 -> 14,856,020, and the row read
745,934,437. Mixing the hash of the map behind the table left the row
where it was, at 745,519,639.

What else moves. Every program links the kernel, 1,360 bytes: the summed
machine code rises by that for each of the fourteen, and runbench's by
9,376 to 920,721 because the linker also copies the kernel into its seven
call sites. runbench's emitted code reads defines=993 calls=10404
branches=9373 lines=81337. The compile rows rise with the builtin's rows
and the library's lines: emit_instructions 5,632,306 -> 5,633,371,
compile_instructions 24,154,945 -> 24,168,284, entry_instructions
156,095,335 -> 156,165,133 and library_instructions 73,612,703 ->
73,647,363. codegen_instructions_dev is written at 117,014,851 and
codegen_instructions_release at 372,088,332, this container's readings
plus the difference CI has shown before, for CI to replace. Four work rows
move with the layout of the grown runtime: work_encodebench up 154 to
2,335,341,047, work_oneshot down 854 to 9,979,505, work_pendbench up 37
to 154,094,201 and work_livebench down 518 to 938,654,959. Welfare rises
95.32525334 -> 95.34300865, and the floor is banked there.

Checked against the interpreter. `micro/text_replace_answers_what_split_and_join_did`
prints the call beside the idiom on every engine: occurrences at either
end and two in a row, text to find that overlaps itself, text longer than
a byte, a replacement that holds the text again, and an empty text to
find rescued by its caller. `http/promised`, the one place kanso's own
library used the idiom, calls it.

The issue's second lead, an escape kernel, is not built. Most of what is
left of the phase is the templates that build each row's values:
14,124,590 in `k_b_join` and 3,744,000 in the joins under it, against
4,228,000 in `k_b_replace` and 2,436,774 in `memchr`. A kernel that knew
HTML could take the second pair and some of the escape's own searches, at
the price of HTML in the runtime.
