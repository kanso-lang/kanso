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




## 2026-10-27 — gavel: templates are Slim-like, and they compile

#2178. Clay, on how the framework's pages are written: "well you can find a
way to make the templates compilable but they should be like slim or Haml".
A template is markup structured by indentation, in the Slim family, and it
compiles to kanso, so the checker reads its field accesses and its
parameters the way it reads any other code. How a database gets in and what
the first application is stay open on #2178.

The prototype is on branch claude/kata-prototype under design/kata, a
generator written in kanso. A template becomes one function. Each `- if`
branch and each `- for` body becomes a helper taking exactly the names in
scope it uses, because kanso refuses an unused parameter, and a body that
uses none becomes a constant. A misspelled field in the sample page is
refused by `kanso check` at the generated line, and both engines print the
same page.

## 2026-10-27 — a dependency's diagnostic quotes its own line

The merged check names the file a diagnostic is about, and a module compile
rereads its files on the failing path so that each diagnostic quotes its
line. The root compile in src/lib.rs handed the renderer only the entry's
text, so a field misspelled in an imported module printed a header and a
location with nothing under them. The template prototype found it, when its
typo landed in a generated view. `render_merged` now rereads each file a
diagnostic names, on the path that is about to fail. An embedded std file
is named bare and is never read from disk, because a file of that name in
the working directory belongs to somebody else.

tests/golden/errors_module gains a_field_read_in_a_modules_second_file,
which on main printed no line. field_read_in_a_deep_library and
two_owners_tie_on_a_traveling_call gain the line their goldens had pinned
as missing.

## 2026-10-26 — gavel: an entry reads its key and value by name

#2163, option 1. An entry is an ordinary record, so `e.key` and `e.value`
read by name on every engine and are checked like any record's fields. The
pattern `(entry k v)` keeps working. Clay asked about Ruby's two-argument
blocks (`|k, v|`); destructuring the entry in a parameter already gives that,
and a two-argument walk over maps would need a twin of every list function.

## 2026-10-27 — an entry reads its key and value by name

#2163. A field read becomes a call on the field's getter, and getters were
made only for fields some type in the program declares. `entry` is built
into the language, so `e.key` had no getter: the checker refused it, and
`e.value` got past the checker only when std/list happened to declare a
`value` of its own, then failed in the interpreter. The checker now counts
`key` and `value` as declared, the way it counts an err's three readers.
`prune_unused_getters` already gathers every getter a program names, and
when `Get_key` or `Get_value` is among them it writes that group an `entry`
arm, keeping one a dependency already wrote. A program that never reads the
fields builds neither arm. The first version built both in every compile and
pruned them after, which CI read as 63 more allocations in the interpreter's
corpus and 502,143 more browser compile instructions.

tests/golden/micro/an_entry_reads_its_key_and_value_by_name reads both
fields beside a local type that declares a `key` of its own; on main it was
refused at `e.key`. tests/golden/errors/an_entry_has_no_field_but_its_key_and_value
pins the refusal of a field an entry does not have. Appendix B names the two
fields.

CI's rows, on main after #2179. The tab compiles the corpus in fewer
instructions, browser_compile_instructions 353,638,408 -> 353,555,358, and
the interpreter's run allocates less, interp_allocs 653,377 -> 653,375, as
does the front end, compile_allocs 13,940 -> 13,939; nothing here isolates
why any of the three fell. The compile rows rise with the new code in the
checker and the prune: compile_instructions 24,871,095 -> 24,874,251,
entry_instructions 73,693,860 -> 73,701,590 and library_instructions
73,331,088 -> 73,338,936. Welfare rises within its second decimal and is
banked.

## 2026-10-27 — the compiler is built as one codegen unit, with fat LTO

The release profile had no settings, so rustc split the crate into sixteen
codegen units by module and optimized each without the others. Cargo.toml
now asks for one unit and fat LTO. The optimizer sees the whole crate and
its dependencies when it inlines, and a release build takes about a minute
longer, 2m34s against 1m41s on this container.

It came out of #2137. Moving call emission out of emit.rs had been declined
because the browser row moved 52 units with no code changed, and the guess
was the file path in panic strings. Neither a longer path nor a shifted line
number moves the row. The units do: they are cut along module lines, so a
move between modules changed what was inlined. With one unit that coupling
is gone, and the split can be retried.

The interpreter gate anchors its count on `run_interpreted_on_stack`, and the
one-unit build inlined it into its thread's closure, so the gate read a drop
glue carrying its name. It is now `#[inline(never)]`, as `emit_ir_for` is.

CI's rows, on main after #2180. Every row the compiler's own build reaches
falls: compile_instructions 24,874,251 -> 24,057,877 (-3.28%),
entry_instructions 73,701,590 -> 71,165,551, library_instructions
73,338,936 -> 70,852,827, emit_instructions 5,715,797 -> 5,597,787,
startup_instructions 52,528 -> 50,254, interp_instructions 577,237,254 ->
571,649,741 and, in the tab, browser_compile_instructions 353,555,358 ->
343,131,848 (-2.95%). codegen_instructions_release moves 371,551,607 ->
371,551,576; it counts clang and the linker, and nothing here isolates why
they did 31 fewer. No allocation, memory or runtime row moved. Welfare rises
95.55 -> 95.57 and is banked.

## 2026-10-27 — call emission moves from emit.rs into calls.rs

#2137. The methods from `emit_call_full` to the end of emit.rs form a second
`impl Backend` block in src/codegen/calls.rs, read with include! right after
emit.rs, so they stay in the codegen module. emit.rs falls from 4,324 lines
to 3,001.

The same cut was declined before #2181 because the browser row moved 52
units with no code changed. Built as one codegen unit, the compiler binary
now differs only in the file names its panic messages carry, and on this
container, which reads CI's rows exactly, compile_instructions, the entry,
library, emit and interpreter rows and the browser row all read what main's
goldens hold.

## 2026-10-27 — the unbuilt-arm prune moves from codegen.rs into unbuilt.rs

#2137. `without_unbuilt_arms`, `kept_arms`, `Given` and `UnreadPositions`
move into src/codegen/unbuilt.rs, read with include! where they stood, so
the items keep their order and their module. codegen.rs falls from 4,231
lines to 3,987. One ratchet mutation patched the prune by its text and now
names unbuilt.rs. As with calls.rs, the compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this
container.

## 2026-10-27 — FnEmit moves from codegen.rs into fn_emit.rs

#2137. `FnEmit`, the impl that keeps it, and the helpers beside it
(`release_cells`, `writes_state`, `is_a_stack_slot`) move into
src/codegen/fn_emit.rs, read with include! where they stood. codegen.rs falls
from 3,987 lines to 3,443. Three ratchet mutations patched FnEmit's methods
by their text and now name fn_emit.rs. The compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this container.

## 2026-10-28 — gavel: senro, with kata inside it, Postgres over the wire, and a guestbook first

#2178. Clay ruled four things about the web framework:

- It is named senro (線路, railway track).
- kata, the Slim-style template compiler, lives inside senro as its view
  layer.
- The database is Postgres, reached by a client written in kanso that speaks
  the wire protocol over a socket. Clay: "I just want to do postgres. I don't
  know why any kind of sea binding is necessary." Development starts against
  Docker, and TLS for hosted connections comes from the runtime, backed by
  OpenSSL natively and a Rust TLS library in the interpreter.
- The first application is a guestbook.

The build order is cloud's: `net` dials out and reads bytes, then the
Postgres client, then derivations (#2186), then routing with kata, then
models.

## 2026-10-28 — gavel: a framework's conventions are derivations

#2186. Clay asked for "some language construct that can allow you to do
compile time mapping", so that senro can map controller functions to routes
and model names to tables the way Rails does, and the compiler can check the
result. A derivation is a pure kanso function that a framework ships and the
compiler runs on the interpreter during the build. It reads a description of
the program and returns declarations built from constructors the compiler
defines (a route, a record type, a table binding), each naming the source
item it came from. The compiler adds them to the program and checks them like
any other code, and `kanso derived` prints them.

Inside a derivation any computation is allowed, including pluralizing a
model's name. What comes out is a finite list that is checked against real
functions and real tables, which is what makes an arbitrary convention safe
to run.

A pub function in `app/` that matches no route is a compile error. Clay
first agreed to a schema file written in kanso, then asked why Rails keeps a
schema file at all. Rails keeps `schema.rb` mainly because old Ruby
migrations stop running when the models they call change. A kanso migration
is a value built from a fixed set of changes, so it cannot break that way,
and the ruling became: no schema file by default, the schema folded from the
migrations at compile time, and `schema.sql` only for a migration that needs
raw SQL.

## 2026-10-28 — net dials out and reads bytes

#2178, the first item owed on the Postgres ruling. `net/dial host at` opens
a connection and answers the same `conn` record `accept` does, so `write`,
`read` and `close_conn` work on it unchanged. `net/read_bytes c` is one read
of what has arrived, handed back as bytes. A Postgres message is framed in
binary, and `net/read`, which answers text, would mangle a byte such as 0xff.

Both are effect builtins on both engines with sockets: `k_dial` resolves the
host with getaddrinfo and tries each address it gets, and the interpreter
uses `TcpStream::connect`. A refusal is worded "cannot connect to host:port"
on both, with nothing from the operating system in it, so the differential
law holds on the failure too. The browser has no sockets and refuses as it
already did for `listen`.

tests/a_program_dials_out.rs plays the server: it answers a greeting with
nine bytes framed like a Postgres message, one of them 0xff, and both engines
print the same nine values. Watched red with the native byte read decoding
as text (the 0xff arrived as U+FFFD) and with the interpreter's refusal
worded differently. tests/sockets_say_one_thing.rs gains `read_bytes` on a
handle that is not a connection.

Every binary links the runtime, so each benchmark's text grows by 752 bytes
(text 5,635,099 summed), and its instructions by a few hundred, which is
layout: no benchmark dials out. They land at work_basket 18,295,655,
work_deepbench 198,243,012, work_digestbench 4,104,311, work_encodebench
2,335,339,667, work_escapebench 40,308,822, work_indexbench 1,887,608,
work_jsonbench 549,337,631, work_livebench 938,655,028, work_oneshot
9,979,724, work_pendbench 154,124,432, work_readbench 4,583,812, work_runbench
825,011,048, work_scanbench 266,891 and work_widebench 28,701,733.

`lib/net` is embedded in the compiler, so the compile rows rise too:
compile_instructions 24,061,034 (+3,157), entry_instructions 71,170,086
(+4,535), library_instructions 70,857,236 (+4,409), emit_instructions
5,599,525 (+1,738), codegen_instructions_dev 116,797,274 (+63,365),
codegen_instructions_release 371,551,798 (+222), interp_instructions
571,786,910 (+137,169), browser_compile_instructions 343,160,078 (+28,230)
and browser_run_instructions 64,598,060 (+27). Welfare fell by less than
0.01, and the floor moves to 95.568 under the rule for a feature a ruling
requires.

## 2026-10-28 — derivations: a convention runs while the program compiles

#2186, the first slice of the ruling. An entry may write `derive
conventions/routing` after its imports. The compiler builds a
`derive/program` describing the app's own modules, which are the ones the
entry imports by a `./` path. Each module is listed with its pub functions,
their parameters and annotations, and the function itself, and with each
record type and its fields. The compiler calls the named function on the
interpreter and reads back a list of declarations. `derive/declare` makes a
constant in the entry, written out as the literal its value spells, with a
function written as its name, so a route table names real handlers and the
checker resolves each one. `derive/refuse` makes a compile error at the item
it names, in that item's own file.

The derivation's module is compiled as an ordinary dependency. No executor
is supplied, so a derivation that answers an effect is refused. A name in
the deriving module that reaches a module it describes is refused where it
is written, because the description is the whole of what a derivation may
know about the app; what it declares then follows from what `kanso derived`
shows. The modules it describes count as used imports, because the
convention reads them. `kanso derived` prints each declared constant with
functions spelled by name.

The derivation runs on an interpreter that copies at every container write.
The first build asked the linearity analysis which writes could go in place,
and that analysis covers every function the program holds: on
bench/entry_corpus it was 24M of the derivation's 25.8M instructions. Without
it the derivation costs 1.8M.

tests/golden/derive holds nine programs, each run on both engines: a route
table that dispatches to real functions, a misspelled action refused on its
own line, a library beside the app left out of the description, and six ways
a derivation can be wrong. It was watched red with function references
written as none, with `../` libraries described, and with a derivation that
imported the module it described. bench/entry_corpus now derives a small
route table, so the entry row, and with it welfare's compile term, prices
what a derivation costs.

The entry row rises to entry_instructions 83,728,840 (+12,558,754): the
corpus gained two modules and a derivation. `lib/derive` is embedded in the
compiler, so the other compile rows move by layout: compile_instructions
24,064,178 (+3,144), library_instructions 70,863,285 (+6,049),
codegen_instructions_release 371,551,844 (+46), emit_instructions 5,598,851
(-674), interp_instructions 571,786,909 (-1), compile_peak_bytes 598,588 (+96)
and browser_compile_instructions 343,190,365 (+30,287). Welfare fell by 0.018
and the floor moves to 95.550 under the rule for a feature a ruling requires;
the fall is past what `--set` takes, so the floor was edited by hand and
`--set` recorded the reason.

Still owed on #2186: the browser compile's derivation golden, a record type
or table binding as a declaration, and the senro derivations themselves.

## 2026-10-28 — a derivation reads constants, declares types, and runs in the page

#2186's second slice. A module in a derivation's description now carries
`constants`, mapping each pub constant's name to its value, and constants
leave `functions`. senro/models needs this: an app keeps its migrations as a
pub constant, and a constant's function reference could not be called, so
the fold had nothing to read. A routing convention also stops seeing a
constant as an action.

`derive/declare_type origin name fields` adds a record type to the entry,
with getters for its fields, and `kanso derived` prints it as `type name
fields`. A name or field a program could not write, or fields that are not a
list, is refused. With it the ruling's three declarations are all here: a
route and a table binding are constants holding a map, and a record type is
its own declaration.

The page runs a derivation. tests/wasm_engine.rs hands the route-table case's
convention and posts module to the compiler in the tab and compiles the
entry, which derives on the interpreter there, and the program prints the
native golden's output. It was watched red with derivations switched off:
"unknown name `routes`".

Chapter 11 of the book gains a section on `derive`, with the route table as
an executed sample, and the book's highlighter and the editor grammar paint
`derive` as a keyword.

Every compile that derives loads the larger std/derive, so the entry row
rises to entry_instructions 83,914,234 (+185,394); on one binary here
load_dependencies took 129K of a 188K rise and the merged check 37K. The
other compile rows move by layout: compile_instructions 24,063,854 (-324),
library_instructions 70,862,041 (-1,244) and browser_compile_instructions
343,187,975 (-2,390). Welfare fell by less than 0.001 and the floor follows
under the rule for a feature a ruling requires.


## 2026-10-29 — sha256 gains HMAC and PBKDF2, and base64 joins the library

Postgres logs a client in with SCRAM-SHA-256 (#2178), which needs HMAC,
PBKDF2 and base64. `std/sha256` gains `digest`, the hash as bytes, `hmac key
message` and `pbkdf2 password salt times size`, and the new `std/base64` has
`encode`, `decode` (none on input that is not base64) and `decode!`. HMAC
hashes each of its two key blocks once and keeps the state it leaves, so
every HMAC under one key starts from there, which is most of what PBKDF2
does: each of its rounds compresses two blocks instead of four.

The tests are the published vectors: FIPS 180-4 and the two lengths either
side of the padding boundary, RFC 4231's HMAC cases 1, 2 and 6 (6 has a key
longer than a block), PBKDF2 at one, two and 4096 rounds and RFC 7914's
two-block vector, and RFC 4648's base64 cases both ways. The guestbook
prototype logs in to Postgres 16 with these.

std/sha256 is imported by the entry and library corpora and by digestbench,
so their rows move. The compile rows rise with the module: entry_instructions
84,707,709 (+793,475) and library_instructions 71,324,471 (+462,430).
digestbench compiles four more functions (emitted defines=118 calls=871
branches=923 lines=7998) and its run rises to work_digestbench 4,104,363
(+52), text 341,377 (+16); runbench carries the same module and reads
work_runbench 825,011,765 (+717), text 819,297 (+16), with emitted defines=856
calls=8793 branches=7545 lines=67140. Summed over
the benchmarks the emitted counters read emitted_other_defines 2,278 (+2),
emitted_other_calls 20,709 (+9), emitted_other_branches 17,933 (+8) and
emitted_other_lines 171,268 (+133), and text 5,635,131 (+32). Welfare fell by about
0.001 and the floor follows under the rule for a feature a ruling requires.

## 2026-10-29 — a field read in an import no longer makes the importer's name private

#2189. A module that declares a name one of its imports also exports keeps
its own arms under the qualified spelling and moves the import's twin into
the bare overload space. `qualify` told the two apart by file: a twin is
cloned from the import's declaration and carries the import's file, while
everything the module writes carries one of its own. The getter for a field
counted as one of the module's own declarations, and its arms carry the file
of each type that has the field, an import's among them. So when `entries`
declared a record with a `name` field, `entries.kso` counted as one of
`home`'s files, `entries/index`'s twin read as `home`'s own `index`, took
`home/index` first, and was not pub: "`index` is private to module `home`".
Getters are now left out of the module's files.

The reduced program is three files in tests/golden/field_reads, run on both
engines by `a_field_read_in_an_import_leaves_the_importers_names_its_own`.
It was watched red with the old filter.

The change moves the compile rows by layout: compile_instructions 24,064,926
(+1,072), entry_instructions 84,709,988 (+2,279), library_instructions
71,325,972 (+1,501) and browser_compile_instructions 343,196,940 (+8,965).
The interpreted run allocates one value fewer, interp_allocs 653,374 (-1).
Welfare fell by less than 0.0001, and the floor follows under the rule for a
change that makes the language do what its specification says.

## 2026-10-29 — a server written as one statement waits for its client

Outside a parallel group, `net/accept` answered "nothing connected" at once
on both engines, on the reasoning that no other fiber could connect. A
client outside the program can, and a server is usually written as one
statement, so the guestbook prototype exited before a browser reached it.
`accept` there now waits the way accept(2) does. Natively, `k_accept_waiting`
clears the non-blocking flag a group's polling may have left and blocks; the
interpreter's executor gains `accept_waiting`, which does the same with
std::net. Inside a group `accept` is still a scheduling point.

tests/sockets_serve.rs runs a one-statement server on both engines and has
the harness say hello to it. It was watched red with the old behaviour: the
program exited before the harness could connect.

Every binary links the runtime, so each benchmark's text grows by 448 bytes
(text 5,641,403 summed), and its instructions move by layout: no benchmark
accepts a connection. They land at work_encodebench 2,335,339,772 (+105),
work_runbench 825,011,247 (-518), work_livebench 938,654,671 (-357) and
work_oneshot 9,979,360 (-364). Building costs more for the larger runtime:
codegen_instructions_dev 116,812,856 (+15,582) and
codegen_instructions_release 371,561,078 (+9,234). Welfare fell by less than
0.001, and the floor follows under the rule for a change the language needs:
a server written as its specification shows it, one statement, has to wait
for its client.

## 2026-10-29 — a sibling module's diagnostic quotes its line

#2188. `render_merged` took a bare file name for an embedded file and never
read it from disk, on the reasoning that a shipped module's files are the
bare ones. They are named under `std/`, and a bare name is a file beside an
entry run from its own directory, so a refusal in a sibling module printed
its location and no source line there. It now reads a bare name when the
entry's own name is bare too.

tests/golden/errors_module gains a sibling-file case, which a spec runs from
inside its directory; it was watched red with the line missing. One error
golden, a_field_read_of_a_list, had pinned the missing line: its fixture runs
as a library beside a generated entry, both bare, and it now quotes the line
it points at.

The browser has no disk to read the line from, so the page answered the same
refusal without it, and tests/wasm_engine.rs's error-corpus spec, which holds
the page to every error golden, went red on the other host. A file the
compiler was handed as source is now read from what it was handed before the
disk is tried. The spec was watched red without that, on
a_field_read_of_a_list, and passes with it.

The browser compile moves by two instructions, browser_compile_instructions
343,196,938 (-2), and no other row moves.

## 2026-10-29 — a derivation sees the calls a module makes, and can refuse one

#2186's routing convention owes a compile error at a `path_to` that names a
function that is not a route. A derivation could see an app's functions but
not their calls, so it could refuse only at a declaration. A module in the
description now carries `calls`: each call a function of the module makes to
a named function, with the caller, the callee, each argument's name ("" for
one that is not a name), and where it is written. `derive/refuse_call` makes
a compile error at one of them, in the caller's file.

Building the case for it found that a module the entry imports could arrive
described with nothing in it. When another of the entry's modules imports it
first and does not re-export it, its declarations come in already demoted,
and the description read each declaration's own pub flag. It now reads the
entry's exports, which say what reaches the entry as pub by any route.

tests/golden/derive gains a derivation that lists the calls between modules
and one that refuses a link to a function that is not a route, at the call.
The second was watched red twice: with no calls to read, and with
`posts/show` refused too before the exports fix.

bench/entry_corpus derives, so the entry row prices collecting the calls:
entry_instructions 85,153,572 (+443,584). `lib/derive` is embedded in the
compiler and grew, so the other compile rows move with it:
compile_instructions 24,083,702 (+18,776), library_instructions 71,386,812
(+60,840) and browser_compile_instructions 343,219,110 (+22,172). Welfare fell
by 0.002, and the floor follows under the rule for a feature a ruling
requires.

## 2026-10-29 — a parameter is not the function it shares a name with

The none check asks, at each call whose argument can be a none, whether the
callee's group has an arm for one. It found the group by name and arity
alone. std/list's merge sort takes its key as a parameter named `key` and
calls it on an element it reads by index, so a program with a one-argument
function named `key` had that parameter read as its own function, and the
read was refused twice inside std/list, at a line the program's author
cannot change. The proof that the index is in bounds usually kept the
question from being asked. A dispatch tie elsewhere in the program took that
proof away, so the tie arrived with two errors nobody wrote. The senro
routing prototype found it.

The check now asks first whether the declaration binds the name, the
question the box check already asks through the same per-declaration set.

tests/golden/errors gains the program: a function named `key`, a tie, and a
call into std/list. It was watched red through the corpus runner with the
test switched off, reporting the two refusals in list.kso, and passes with it.

The compile rows move by layout: compile_instructions 24,095,553 (+11,851),
browser_compile_instructions 343,289,574 (+70,464), entry_instructions
85,147,962 (-5,610) and library_instructions 71,380,536 (-6,276). Welfare
falls by less than 0.001, and the floor follows under the rule for a change
that makes the language do what its specification says.

## 2026-10-29 — six messages that named the wrong cause

Two messages a newcomer meets in the first hour named the wrong cause
(#2217). With no clang on `PATH`, `kanso run` and `kanso play` said
`cannot build: No such file or directory (os error 2)`, which reads as a
missing source file, and `kanso build` said it could not invoke clang
without saying where it had looked. All three now say clang is not on
`PATH`. `run` and `play` also name the way out that needs no clang:
`kanso run h.kso --interp`. The check runs only when the spawn reports
not-found and no `clang` resolves on `PATH`. Any other build failure
keeps the host's io text. `run` on a directory quotes the path as typed,
not the entry it was taken to.

`kanso test` on a file of bare statements was refused as a library with a
statement in it. The file declares nothing, so the message described its
shape and not the problem. It now says the file holds statements and no
tests, where tests live, and that `kanso run` runs it.

`kanso --help` now lists `--counters` beside `--release` on `build` (#2215).
Before this, the only place a user learned the flag was the refusal a
release binary prints when asked for counters it was built without.

The clang messages are asserted whole in
`tests/a_toolchain_the_path_cannot_reach.rs`, now three cases with `play`
added. They stay listed in `tests/golden/unpinned_diagnostics.txt`, which
says where their pin lives. The test message has a golden in
`tests/golden/test_verb/statements.stderr`, and `front_door` checks the
help line. Each spec was watched red on the compiler before the change.

Two more came from the hako commands (#2234). `kanso install` printed
`installed 0 hako(s):` above a list of the one it had pinned, because the
count it printed was the names left to walk, which is none by the time it
prints. It now counts the lines of its own report. And an import of a module
an installed hako does not have, such as `kanso-lang/vse/methods`, said the
hako was not in the cache and to run `kanso install`, which changed nothing.
The resolver now checks whether the locked tree is there before blaming the
cache, and when it is, names the modules the tree does have:
``error: cannot resolve import "acme/widgets/gadgets" — `acme/widgets` is
installed, and has no module `gadgets` (its modules: `acme/widgets`)``.
tests/hako.rs asserts both, and both were watched red.

The sixth is #2205's last item. A test run from inside its module imports
the module as `../<name>`, so a refusal in a sibling pointed at
`../t/p.kso` where the reader sees `p.kso`. A module file whose path climbs
out of the working directory and back in is now shown from the working
directory. The check is made only for a path that starts with `..`. The
refusal itself stands: `p.kso` holds a `pub fn` and a statement, which is
neither a library nor a play file. tests/test_verb.rs pins the message,
watched red with the old path.

With #2209's http change these move the compile rows by layout:
compile_instructions 24,096,605 -> 24,096,621, library_instructions
71,383,017 -> 71,383,019, startup_instructions 50,258 -> 50,260 and
codegen_instructions_release 371,573,437 -> 371,573,350. A first version
kept the path as typed for every verb, which cost the front end one
allocation; it is now kept only when a directory is taken to its entry,
and compile_allocs and interp_allocs are unchanged. Welfare falls by less
than 0.0001, and the floor follows under the rule for a change that makes
the tools do what the documentation says.

## 2026-10-29 — http answers any status, and its suite runs

#2209, filed by the docs worker. `std/net/http` could answer only 200 and
404, and `response` cannot be built outside the module, so an order service
answered a refused order with 200 and a sentence. `http/respond code headers
body` answers any status, with the reason phrase its code is registered with,
and an empty phrase for a code without one, which HTTP allows.

The suite beside it had never run. tests/std_suites.rs walked one level below
lib/, and lib/net/http is two, so `http_test.kso` passed or failed unread. The
walk now goes all the way down and pins the count at seven suites. With the
422 phrase misspelled, both of its specs went red on lib/net/http, which
neither could see before.

## 2026-10-29 — a reader lets the others run

#2208, filed by the docs worker. A program that is both ends of a connection,
dialling in one statement of a group and accepting in the next, never
finished on either engine: the client's `net/read` blocked the whole runtime
before the server's accept and write could run. `accept` already went back
in the queue when nothing was waiting. A read now does the same when nothing
has arrived: natively `k_step` peeks the socket without blocking and yields,
and the interpreter asks its executor whether the connection is readable
before it reads. A peer that has closed reads as ready, and the read answers
what it always did. Outside a group a read still blocks, as `accept` does,
because a server is often one statement and its client is outside the
program. `net/read_bytes` takes the same path; `net/write` does not yet.

tests/a_reader_lets_the_others_run.rs runs the issue's program with `read`
and with `read_bytes` on both engines under a ten-second deadline, in a
process group of its own so a hang is killed whole rather than leaving the
built program holding the test's streams. With the change reverted it failed
for the right reason, a native run that never finished.

The peek makes the runtime larger, so CI's rows move: codegen_instructions_dev
116,812,856 -> 116,821,778, codegen_instructions_release 371,560,989 ->
371,573,437, and each benchmark's text grows by 288 bytes, so text
5,641,403 -> 5,645,435. Run instructions
fall by a few hundred to two thousand a benchmark, runbench 825,011,247 ->
825,009,321. Welfare falls by less than 0.01, and the floor follows under the
rule for a change that makes the language do what its specification says.

## 2026-10-29 — the tab's list of machine calls has one home

#2219, a cleanup the docs worker filed. Which calls a tab cannot answer was
written twice: as `NEEDS_A_MACHINE` in docs/book-play.js, which decides which
book panels become live editors, and as a kanso list in the browser
differential, which decides which samples it covers. They drifted for months.
The differential now reads the list out of book-play.js, the way it already
reads the playground's examples out of play.js, and its own copy is gone. A
page without the list fails the run rather than covering every sample.

Read the same way, the page's list selects the same 717 programs the old copy
did. With `read_file` taken out of the page's regex the run selected 725, so
the differential follows the page.

## 2026-10-29 — gavel: `path_to` takes a record or a route's name

#2198. #2186 ruled that `path_to posts/show post.id` looks the route up in
reverse, and the gavel on equality refuses to compare two functions, which
the lookup needs at run time. Clay, ruling: "it could either take path_to
post, or the string form that specifies the path followed by the id... both
are available to the user. I'm not sure how you do that but you can be
clever."

So `path_to` has two forms. `path_to post` takes a record, and the record's
type picks its show route, with its `id` for `:id`. `path_to "posts/show"
post.id` names the route by a string. The form that takes a function is
dropped. The string form runs as a lookup in a map the routing derivation
declares, and the derivation refuses a string that names no route at the
call that writes it. The record form runs by dispatch on the record's type:
the derivation declares one `path_to` arm per model with a show route, which
needs a derivation that can declare a function arm (`derive/define`, planned
on #2198).

## 2026-10-29 — a call's description carries the strings it was written with

#2198's string form refuses `path_to "posts/shwo" id` at the call, so a
derivation has to read the string. Each `derive/call` now carries `strings`
beside `arguments`: an argument's text when it is a string written whole, and
"" for anything else, including a string with an interpolation in it.

tests/golden/derive gains a convention that refuses a link whose string
names no route, at the call, on both engines. On the compiler before this
change it was refused for a different reason, at the convention: no record
type had a field `strings`.

## 2026-10-29 — a derivation defines arms, and the module they belong to compiles them

#2198's record form runs `path_to post` by dispatch on the record's type, so
the routing convention has to write a `path_to` arm for each model. A
derivation can now answer `derive/define origin source`: an arm written out as
source, for the module `origin` names. A module may already add an arm to a
library's function for a type it declares, so the arm goes there and not in
the entry, which declares no models.

When any derivation defines an arm, the entry's dependencies load a second
time, and each module that was given arms reads them as one more of its own
files, `posts.derived.kso` beside `posts.kso`. The arm is qualified, formatted
and checked as code someone wrote in that module, and an error in it quotes
the line the convention wrote. An arm for a module the derivation was not
shown is refused at the `derive` line. `kanso derived` prints each arm under
the module it went to. In the tab the modules arrive as text and the arms join
them the same way.

tests/golden/derive gains three cases, each run on both engines: a `path_to`
arm defined for each record type in a module with a `show` action, an arm that
reads a missing field and is refused in the derived file, and an arm for
std/list, refused. With the second load switched off the first case printed
"no route" twice where it prints `/posts/3`. tests/wasm_engine.rs runs the
first case in the page; it was watched red with the derived files keyed so
the module could not find them.

The rows: bench/entry_corpus derives, so it pays for each call's strings and
for the larger `lib/derive`: entry_instructions 85,282,135 (+134,173),
compile_instructions 24,096,578 (+1,025), library_instructions 71,383,015
(+2,479), and browser_compile_instructions 343,289,572 (-2). The first build
keyed the derived files on every module load, which cost compile_allocs 14
and interp_allocs 12 on programs that derive nothing; the key is asked for
only while arms load, and neither row moves. Welfare falls by less than
0.001, and the floor follows under the rule for a feature a ruling requires.

## 2026-10-29 — a page split into words joins the run program

`regexp/split "[^a-z']+"` allocated about thirty times per field it answered
(#2214), and the run program never saw it: its one regexp phase, the scan
from scanbench, looks for a literal the subject lacks and stops early. Under
the 2026-09-05 corpus-first gavel the shape joins the run program here,
measured on main before its fix.

`worded` builds a page of 4,000 words from a fixed list, separated by
spaces, commas, full stops and newlines, and splits it with the pattern a
word count uses. On this container the release-tier run program reads
851,480,954 with the phase and 825,132,205 with it reading nothing, a column
of 26,348,749 (3.09%).

The interpreted corpus does not take the phase. Importing `std/regexp` there
made the tab compile the whole library on every run: CI read
browser_compile_instructions 343,289,560 -> 816,029,965 and
interp_instructions 571,786,909 -> 817,162,078, so the tab's compile term
would have measured one library's front end and swung with every edit to
it. The shape is the matcher's work at run time, which the run program
measures.

CI's rows for the phase: run_alloc_bytes 224,043,124 -> 231,013,169,
run_allocs 1,582,921 -> 1,700,863, run_arena_blocks 4 -> 5,
run_arena_peak_bytes 4,718,608 -> 5,767,184, run_beat_iters 193,074 ->
211,299, run_bytes_malloc 7,850 -> 7,867, run_evac_allocs 1,732 -> 1,736,
run_evac_bytes 4,602,208 -> 4,602,496, run_find2_calls 1,037,031 ->
1,060,857, run_perm_allocs 106 -> 127, run_push_mut_fast 1,191,311 ->
1,195,309, run_push_mut_slow 1,645,457 -> 1,645,463, run_sh_buf 91,336,800
-> 93,914,368, run_sh_bytes 4,872 -> 1,148,520, run_sh_rec 4,098,336 ->
4,675,424, run_sh_str 16,816,384 -> 16,930,368, run_spare_peak_bytes
2,621,456 -> 3,670,032, run_str_scan_bytes 18,358,325 -> 18,358,372,
run_str_scans 3,387 -> 3,402, work_runbench 825,009,321 -> 851,374,952,
emitted_other_branches 17,933 -> 18,066, emitted_other_calls 20,709 ->
20,888, emitted_other_defines 2,278 -> 2,292, emitted_other_lines 171,268 ->
172,430 and text 5,645,435 -> 5,658,091.
Welfare re-bases run_instructions and
run_peak_bytes, each keeping its ratio to its baseline, and the floor moves
only by the rounding of the two baselines.

## 2026-10-29 — a play file shows its plan

#2204, filed by the docs worker. `--plan` was accepted by `run` alone, and
`run` refuses a file that holds definitions, so a program small enough for
one file, which is how the book and the playground teach, had no plan to
show. Three of the book's panels are about what a plan shows and were getting
theirs through a stub entry no reader could type. `kanso play <file> --plan`
now prints the plan `run --plan` would, from the play file, and skips the
cached build.

Asking for it before had answered "`play` is not a verb". `play` was missing
from the list of verbs the usage message checks, and a known verb refused for
a flag fell to "wants a file or directory". The message now names the flag:
"`check` does not take `--plan`".

tests/a_play_file_shows_its_plan.rs runs the issue's program and compares the
plan with a golden; tests/front_door.rs gains `play` with no file and `check`
with `--plan`. Both were watched red on the compiler before the change.

## 2026-10-29 — check reads a play file as play

#2205's first item, filed by the docs worker. `kanso check` sent any file
with a declaration in it to the library compile, which refused a play file's
first statement and checked nothing else, so a play file had no check at all
and the book's `_check` samples had to stay `pub play` libraries. A file the
library parse stops at a statement, and for that reason alone, is now
checked as `kanso play` compiles it: a play file that would run reads `ok`,
and one that would not is refused with what `play` says. The library parse's
refusal is the one sign of the shape, so its wording now lives in one
constant that the parser says and `check` reads.

tests/check_reads_a_play_file.rs checks one play file that runs and one that
names an unknown name. The compiler before this change refused both with the
library's statement error.

## 2026-10-29 — an entry names a group

#2205's second item. ch06 teaches the named group, `prepare =` above
indented effect lines and then `prepare .> (_ -> ...)`, as the way to join
concurrent work, and it parsed only as a library constant. An entry or a
play file refused it with "only `if` opens an indented block", which named
nothing the reader had written. A binding whose header is `name =` alone,
above more than one indented line, now parses its body as a constant's body
is parsed, so its effect lines join into one group and run concurrently as
the constant's do.

tests/entry_file.rs runs an entry whose group sleeps in one member and prints
in the other, on both engines, and reads `fast`, `slow`, `done`, the order a
library constant prints. The compiler before this change refused it.

The three changes in #2240 move the compile rows by layout:
compile_instructions 24,096,605 (+27), library_instructions 71,383,017 (+2),
startup_instructions 50,258 (+4), codegen_instructions_release 371,560,989
(-89) and browser_compile_instructions 343,289,560 (-12). Welfare falls by
less than 0.0001, and the floor follows under the rule for a change that
makes the tools do what the documentation says.

## 2026-10-29 — a letter the scan refuses costs nothing

`regexp/split "[^a-z']+"` tries a match at every letter of its subject, and
each try that failed cost three allocations: the empty list of captures the
walk starts with, a continuation built by the pattern's sequence, which had
one part, and the bytes of the class's set and of the letter, read afresh to
ask whether one holds the other. A field of five letters and a separator
cost about thirty (#2214).

Three changes take a refused letter to none. A pass over the compiled tree
gives each class its set's bytes once, and the class reads the letter's
code rather than its bytes. A sequence of one part is replaced by the part,
since it matches what the part matches. And the walk starts from one empty
list held as a constant. A fourth takes a field from four allocations to
three: a pattern with no groups answers that constant for its captures
rather than building an empty list per match. The 2,600-word page from the
issue went from 95,946 allocations to 12,764, and a scan over 3,000 letters
that never matches went from 9,058 to 54.

The new mem fixture splits 60 lines of prose and reads 1,507 allocations
where the library before this read 14,283. The four regexp fixtures already
in the mem vein fall with it. Two runtime goldens quote a line of the
library and follow the line.

of 26,348,749 (3.09%).

The interpreted corpus does not take the phase. Importing `std/regexp` there
made the tab compile the whole library on every run: CI read
browser_compile_instructions 343,289,560 -> 816,029,965 and
interp_instructions 571,786,909 -> 817,162,078, so the tab's compile term
would have measured one library's front end and swung with every edit to
it. The shape is the matcher's work at run time, which the run program
measures.

CI's rows for the phase: run_alloc_bytes 224,043,124 -> 231,013,169,
run_allocs 1,582,921 -> 1,700,863, run_arena_blocks 4 -> 5,
run_arena_peak_bytes 4,718,608 -> 5,767,184, run_beat_iters 193,074 ->
211,299, run_bytes_malloc 7,850 -> 7,867, run_evac_allocs 1,732 -> 1,736,
run_evac_bytes 4,602,208 -> 4,602,496, run_find2_calls 1,037,031 ->
1,060,857, run_perm_allocs 106 -> 127, run_push_mut_fast 1,191,311 ->
1,195,309, run_push_mut_slow 1,645,457 -> 1,645,463, run_sh_buf 91,336,800
-> 93,914,368, run_sh_bytes 4,872 -> 1,148,520, run_sh_rec 4,098,336 ->
4,675,424, run_sh_str 16,816,384 -> 16,930,368, run_spare_peak_bytes
2,621,456 -> 3,670,032, run_str_scan_bytes 18,358,325 -> 18,358,372,
run_str_scans 3,387 -> 3,402, work_runbench 825,009,321 -> 851,374,952,
emitted_other_branches 17,933 -> 18,066, emitted_other_calls 20,709 ->
20,888, emitted_other_defines 2,278 -> 2,292, emitted_other_lines 171,268 ->
172,430 and text 5,645,435 -> 5,658,091.
Welfare re-bases run_instructions and
run_peak_bytes, each keeping its ratio to its baseline, and the floor moves
only by the rounding of the two baselines.

CI's rows for the fix. What falls: run_alloc_bytes 231,013,169 ->
224,894,993, run_allocs 1,700,863 -> 1,594,553, run_arena_blocks 5 -> 4,
run_arena_peak_bytes 5,767,184 -> 4,718,608, run_push_mut_fast 1,195,309 ->
1,195,313, run_sh_buf 93,914,368 -> 91,425,392, run_sh_bytes 1,148,520 ->
5,112, run_spare_peak_bytes 3,670,032 -> 2,621,456, work_runbench
851,374,952 -> 846,388,267, scan_push_mut_fast 9 -> 13,
a_class_asks_by_the_byte_alloc_bytes 507,073 -> 116,737,
a_class_asks_by_the_byte_allocs 9,266 -> 1,267,
a_class_asks_by_the_byte_sh_buf 157,168 -> 22,768,
a_class_asks_by_the_byte_sh_bytes 134,520 -> 168,
a_scan_keeps_its_place_in_the_text_alloc_bytes 106,537 -> 73,289,
a_scan_keeps_its_place_in_the_text_allocs 1,742 -> 874,
a_scan_keeps_its_place_in_the_text_push_mut_fast 5 -> 7,
a_scan_keeps_its_place_in_the_text_sh_buf 52,896 -> 46,288,
a_scan_keeps_its_place_in_the_text_sh_bytes 19,752 -> 240,
a_scan_that_finds_nothing_keeps_nothing_alloc_bytes 2,781,801 -> 1,194,889,
a_scan_that_finds_nothing_keeps_nothing_allocs 74,006 -> 24,874,
a_scan_that_finds_nothing_keeps_nothing_push_mut_fast 5 -> 7,
a_scan_that_finds_nothing_keeps_nothing_sh_buf 18,336 -> 976,
a_scan_that_finds_nothing_keeps_nothing_sh_bytes 1,175,784 -> 240,
a_split_stops_where_the_separator_does_alloc_bytes 24,487 -> 4,743,
a_split_stops_where_the_separator_does_allocs 673 -> 64,
a_split_stops_where_the_separator_does_push_mut_fast 8 -> 10,
a_split_stops_where_the_separator_does_sh_buf 1,168 -> 832 and
a_split_stops_where_the_separator_does_sh_bytes 14,808 -> 240. What rises,
and why: the pass that gives each class its bytes copies the compiled tree
once per pattern, and the library is 85 lines longer to compile, so each
program that uses a pattern holds a few more records and the compile corpora
read more: run_evac_allocs 1,736 -> 1,738, run_evac_bytes 4,602,496 ->
4,602,512, run_perm_allocs 127 -> 128, run_sh_rec 4,675,424 -> 4,675,856,
entry_instructions 85,282,135 -> 86,700,059, library_instructions 71,383,019
-> 72,615,632, work_scanbench 266,140 -> 266,939, emitted_other_branches
18,066 -> 18,226, emitted_other_calls 20,888 -> 21,137,
emitted_other_defines 2,292 -> 2,312, emitted_other_lines 172,430 ->
173,938, text 5,658,091 -> 5,670,096, scan_alloc_bytes 8,934 -> 9,366,
scan_allocs 91 -> 97, scan_sh_buf 1,056 -> 1,168, scan_sh_bytes 288 -> 336,
scan_sh_rec 1,632 -> 1,872, a_class_asks_by_the_byte_evac_allocs 7 -> 9,
a_class_asks_by_the_byte_evac_bytes 224 -> 272,
a_class_asks_by_the_byte_perm_allocs 33 -> 34,
a_class_asks_by_the_byte_sh_rec 87,440 -> 87,536,
a_scan_keeps_its_place_in_the_text_evac_allocs 7 -> 9,
a_scan_keeps_its_place_in_the_text_evac_bytes 208 -> 256,
a_scan_keeps_its_place_in_the_text_perm_allocs 51 -> 52,
a_scan_keeps_its_place_in_the_text_sh_rec 1,296 -> 1,616,
a_scan_that_finds_nothing_keeps_nothing_evac_allocs 7 -> 9,
a_scan_that_finds_nothing_keeps_nothing_evac_bytes 208 -> 256,
a_scan_that_finds_nothing_keeps_nothing_perm_allocs 50 -> 51,
a_scan_that_finds_nothing_keeps_nothing_sh_rec 1,296 -> 1,616,
a_split_stops_where_the_separator_does_evac_allocs 7 -> 9,
a_split_stops_where_the_separator_does_evac_bytes 224 -> 272,
a_split_stops_where_the_separator_does_perm_allocs 22 -> 23 and
a_split_stops_where_the_separator_does_sh_rec 1,168 -> 1,232. Welfare rises,
and the floor is banked at the new score.

## 2026-10-29 — prose_check pairs each page's div tags

Item 3 of #2213. ch07 shipped with a stray `</div>` that the browser forgave
and every gate passed over. prose_check now walks the `<div>` and `</div>`
tags of every page it reads and reports a close with nothing open, or an open
never closed, at its line. On main it found two more, in appendix a after the
type-order panel, and this change deletes them. No page puts a div inside a
comment, so the walk reads the raw page. A ratchet row puts a stray close back
after ch07's first panel and the gate goes red. The summary line counts
findings rather than tells, and the advice about the slop families prints only
when one of them was found.

## 2026-10-29 — book_panels reads output panels by their source, and golden paths

Items 1 and 2 of #2213. The landing page titles its outputs `output`, and
book_panels matched only titles that start with `kanso`, so those panels were
copied by hand. An `output` panel now answers to the source panel it sits
under: the pair is matched once, and the output becomes the match a panel
titled with that source's run would have been. On the site pages, a run
resolves to tests/golden/examples/<name>.stdout and a `--plan` run to
<name>.plan, so about.html's two effects panels are compared too. A panel
titled with a golden's path, such as about.html's three
tests/golden/errors/*.stderr panels, shows that file, compared after its
markup is stripped and rewritten from it under `--write`. The three shapes
are sorted into one list by position before the rewrite, which replaces the
two-list merge.

The wider reading found one drifted panel in the book: ch03's reading.kso
showed one of the two lines the sample prints. book_check's write path now
copies the example and error goldens into its scratch tree, so a rewrite
there reaches the new shapes. Two ratchet rows, one per shape, watch the gate
go red.

## 2026-10-29 — the interpreter's calls section moves into eval/calls.rs

The cleanup owed after #2240 to #2244. `call` through `worded_step`, 888
lines of the `Interp` impl, move into src/eval/calls.rs as an impl block of
their own, read with include! where they stood, the way eval.rs already reads
executor.rs and builtins.rs. eval.rs falls from 3,803 lines to 2,918. The
removed lines are the body of calls.rs byte for byte. CI's cost-goldens job
read every vein at main's values, compile_instructions among them, so no row
moved. Sixteen ratchet mutations patched lines in the moved section by their
text and now name calls.rs. One of them, the lambda called as a nested call,
only checked for `closure_flow` there; the arms it deletes stayed in eval.rs.

## 2026-10-29 — std/expect ships in the binary

Chapter 7 teaches `std/expect` for test failures, and outside a checkout a
test importing it said "`std/expect` is not in the shipped library". The
loader finds a shipped module first in the table `src/lib/imports.rs`
embeds and then on disk at `lib/` beside the binary or the working
directory. Every other module under `lib/` was in the table, so a checkout
never showed the gap: an installed binary, with no `lib/` beside it, and the
browser, with no filesystem, could not import `std/expect`. It is in the
table now, and in `SHIPPED_MODULES`, so its merged check is asked with the
others'.

`tests/every_shipped_module_checks_clean.rs` gains a third test that walks
`lib/` and imports each module from a directory with `KANSO_STD` pointing at
an empty one, so only the embedded copies can answer. It failed on main
naming `expect` and nothing else.

tests/a_play_file_is_keyed_by_its_text.rs leaned on `std/expect` being the
one library module the binary did not carry, to play a file that reads a
module from disk. It now plays a `std/shout` that exists only under
`KANSO_STD`. Forcing `play_file_is_self_contained` to true still turns it red
at the second play.

The binary carries one more module's text, and four compile rows move by
layout: browser_compile_instructions 343,289,560 -> 343,289,578,
compile_instructions 24,096,621 -> 24,096,607, entry_instructions 86,700,059
-> 86,700,038 and library_instructions 72,615,632 -> 72,615,607. Welfare
rises by less than a hundredth and is banked.

## 2026-10-29 — check.rs, values.c and lib.rs give up their separate sections

#2249, from the weekly sweep. Three files past three thousand lines each held
sections that share nothing but the file, and each moves without a change to
what it says.

- check.rs, 3,355 lines to 2,710: the literal-argument check moves to
  check/literals.rs and the annotation-name check to check/annotations.rs,
  read with include! where they stood.
- runtime/values.c, 3,044 lines, becomes four parts that runtime.c lists in
  order: bignum.c (ints past a machine word), values.c (strings, errors,
  records, fields), ryu.c (the shortest float rendering and its tables) and
  operators.c (subtypes, rendering, equality, arithmetic). The text build.rs
  joins is main's byte for byte, so every compiled program links the same
  runtime.
- lib.rs, 3,004 lines to 2,739: the late rewrites (field reads to getters,
  merges to constructor calls, `!=` to the negation of `==`) move to
  lib/desugar.rs.

Each Rust file rebuilt from its includes is main's file byte for byte.
Twenty-two ratchet mutations patched lines in the moved sections by their
text and now name the files that hold them. linear.rs, at 3,087 lines, is one
analysis and its helpers and stays whole.
CI's cost-goldens job read every vein at main's values, so no row moved.

## 2026-10-30 — gavel: cloud closes its own issues, and the docs lane decides the book's structure

Clay, on 2026-10-30: "nothing needs to be closed from the chat lane. that
doesn't make any sense you can do whatever you want to and the book order you
can just take executive action on and do what makes sense because I can
always edit it later."

- Cloud closes an issue when its work is done. #1856, the work-ahead engine,
  had been built since 2026-10-14 and waited only on a close; it is closed.
- Book and site structure is the docs lane's call. #2203 takes option A, the
  reorder with redirects. #2232 takes option 1: failure-kinds.html becomes a
  redirect to chapter 04, and its two proposals become issues of their own.
  #2235 takes option 1, tagging kanso-json v0.1.0. That tag is blocked:
  this session's tag push hung up at the remote, and the proxy refused the
  API path (HTTP 403). The command is on the issue for Clay.

Clay also answered #2191. senro is its own git repository, kanso-lang/senro,
which an app imports by repository name, as kanso reads any hako. The issue
had read as a design question when the only obstacle was an attach for push
that the harness had refused. The attach went through when tried again, and
kanso-lang/senro#1 moved routes, models, pg, web, kata and the guestbook into
it, with specs CI runs against a pinned kanso. CLAUDE.md now says a refusal is
reported as a refusal, and records both rulings above.

## 2026-10-30 — a derivation sees the calls the entry writes

#2198's gavel refuses a `path_to` string that names no route "at the call
that writes it". senro's routing convention (kanso-lang/senro#3) found that a
call written in the entry was never shown to it: the description listed the
modules the entry imports and left the entry out, so `routes/path_to
"posts/shwo"` in `main.kso` compiled and ran.

The entry is now described first in `program.modules`, under its file's
stem. An entry holds statements only and declares nothing, so its functions,
constants and records are empty and it carries only its calls, each from the
caller "". `derive/refuse_call` on one of them reports in the entry's file.
A convention that checks calls covers the entry without asking for it, which
a separate field on `program` would not have done.

tests/golden/derive gains a link convention whose bad link is in the entry.
Before the change it compiled and printed on both engines; after, both refuse
it at `main.kso:7:8`. With the entry's calls left out of the description, the
case and the calls golden go red again. Three goldens that list the modules
or their calls now show the entry: the constants map has `"main":0`, the
module list starts with `"main"`, and the call trace gains the entry's three
calls.

The page's play-cache paragraph still named `std/expect` as the one module
the binary does not carry, which stopped being true when it shipped; it now
says a module read from disk keeps the IR's key.

bench/entry_corpus derives, so the entry row prices collecting the entry's
calls: entry_instructions 86,700,038 -> 86,751,224 (+51,186). The tab
compiles with the embedded `lib/derive`, whose comments grew, and
browser_compile_instructions moved 343,289,578 -> 343,289,570 (-8). No other
row moved, the library and module rows included. Welfare fell by less than a hundredth, and the floor
follows under the rule for a feature a ruling requires.

## 2026-10-30 — a derivation reads the files beside the entry

#2186's models ruling makes `schema.sql` the opt-in for a migration that
needs raw SQL the fold cannot interpret: the app checks in `pg_dump` output
"and the compiler reads that file instead". A derivation reads only the
description it is handed, and nothing in it carried a file.

`derive/program` now has `files`, a map from the name of each file directly
beside the entry that is not kanso source and reads as text to that text.
senro's models convention reads `schema.sql` from it when the app keeps one
and folds the migrations when it does not. The files are inputs like the
source, so the same tree still derives the same declarations. The tab has no
directory under its compile and hands none.

tests/golden/derive gains a convention that declares the lines of a
`tables.txt` beside its entry, on both engines. With the directory read
pointed at a path that does not exist, it declares an empty list and the
case fails.

bench/entry_corpus derives, so it now reads its directory, and the entry row
prices that: entry_instructions 86,751,224 -> 86,792,132 (+40,908). No other
row moved. Welfare fell by less than a hundredth, and the floor follows under
the rule for a feature a ruling requires.

## 2026-10-30 — a connection can be secured with TLS

#2178 owes TLS through the runtime before the first hosted deploy: OpenSSL
natively, a Rust TLS library in the interpreter, and no general way to call
C. `net/secured c host` runs a TLS handshake on a connection already open and
answers it, so reads, writes and close go through TLS from then on. It
upgrades rather than dials because Postgres asks for TLS inside a plain
connection: the client sends an SSLRequest, reads `S`, and starts the
handshake on the same socket. `net/dial_tls host at` is a dial and then
`secured`, for a protocol such as HTTPS that speaks TLS from its first byte.

Both engines check the server's certificate for the host against the
machine's trusted roots, and `SSL_CERT_FILE` replaces them on both: OpenSSL's
default paths read it, and so does rustls-native-certs. A host written as an
address is checked against the certificate's addresses and sent no server
name, which is what rustls does and what OpenSSL does once it is told the
host is an address. A refusal of any kind reads "cannot secure the
connection to host", with nothing from either library in it.

Native code opens libssl with `dlopen` the first time a program secures a
connection and looks up the sixteen calls it makes by name. Linking it would
have put libssl in every binary's start-up and in its library list, and most
programs never speak TLS. A machine with no OpenSSL 3 refuses the handshake
with the same words. The interpreter keeps each connection's rustls session
beside its stream. The runtime's wasm build and the tab have no sockets and
refuse as they already did.

tests/a_program_secures_a_connection.rs plays a server with rustls and a
certificate for `localhost` signed by a test authority whose key was thrown
away after signing (`tests/fixtures/tls`). Both engines dial it, secure the
connection, say hello and print the answer. Trusting only the server's own
certificate, and naming another host, are each refused with the same words
on both engines. A server that closes without TLS's close_notify ends the
conversation the way a plain close does, with an empty read on both engines;
rustls reports that close as an error, and with the error passed through the
interpreter's case failed. senro's `pg/connect_tls`, run against Postgres 16
serving the same certificate, logged in over TLS 1.3 on both engines, and
the server's `pg_stat_ssl` agreed.

Every program links the runtime, so each benchmark's text grows by 2,544
bytes and the text row sums to 5,705,712. No benchmark secures a connection,
and the instruction rows move by about a thousand each where the larger
runtime is loaded: work_basket 18,296,044, work_digestbench 4,104,967,
work_encodebench 2,335,340,353, work_escapebench 40,309,372, work_indexbench
1,888,158, work_jsonbench 549,338,235, work_livebench 938,654,972,
work_oneshot 9,979,521, work_pendbench 154,124,736, work_readbench 4,584,375
and work_scanbench 268,194. Three move further, by layout: deepbench
198,242,376 -> 197,859,562, runbench 846,388,267 -> 846,337,653 and widebench
28,700,882 -> 28,686,318. The compiler carries rustls and grows, and its rows move with
the binary: compile_instructions 24,096,607 -> 24,097,537,
entry_instructions 86,792,132 -> 86,793,470, library_instructions
72,615,607 -> 72,618,778, emit_instructions 5,598,851 -> 5,598,399,
codegen_instructions_dev 116,821,778 -> 116,926,412,
codegen_instructions_release 371,573,350 -> 371,827,256, interp_instructions
571,786,909 -> 571,683,168, browser_compile_instructions 343,289,570 ->
343,307,533 and browser_run_instructions 64,598,060 -> 64,598,061. The
floor follows under the rule for a feature a ruling requires.
