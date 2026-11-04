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
codegen_instructions_dev 116,821,778 -> 116,926,477,
codegen_instructions_release 371,573,350 -> 371,827,207, interp_instructions
571,786,909 -> 571,683,168, browser_compile_instructions 343,289,570 ->
343,307,533 and browser_run_instructions 64,598,060 -> 64,598,061. The
floor follows under the rule for a feature a ruling requires.

## 2026-10-30 — the emitter's word helpers move into codegen/words.rs

The cleanup owed after #2251 to #2258. The free functions between ir_scan.rs
and the `Backend` impl move into src/codegen/words.rs, read with include!
where they stood: the symbol names (`wsym` and its kin), `literal_word`, the
lift around a call that may fail, word arithmetic, and the readers and
writers of a value's inline tag and payload. codegen.rs falls from 3,447
lines to 2,911. Putting words.rs back in place of its include! gives main's
codegen.rs byte for byte. Six ratchet mutations patched lines in the moved
section by their text and now name words.rs; each still applies to the line
it did before. CI's cost-goldens job read every vein at main's values,
compile_instructions among them, so no row moved.

## 2026-11-01 — a join a fiber reaches runs beside the fibers already running

#2263. A join that a fiber reached while it ran, rather than one its group
started with, went to `execute` from `step` (and to `k_exec` from `k_step`),
and a join there started a second scheduler that returned only when its
group was done. Every other fiber in the outer group waited for it. A server
that forks per connection (after each accept, one bare line answers and the
other accepts the next) is that shape: five concurrent requests to a handler
that sleeps 300 ms got one answer and four timeouts, on both engines.

Both schedulers now hold their fibers in spawn order, pick the earliest
wake, and break ties by position. A join a fiber reaches hands its members
to the running scheduler (`Step::Forked`, `blocked == 2` natively), which puts
them where that fiber stood; the work bound after the join waits in the
group and takes the place of the member that finishes last, with the
group's failures as its value. A nested group that never waits therefore
runs as it did inline. Natively the work after the join is built around a
fresh settled node, and the group's result is written into it. The fiber
arrays grow as they need to, so the 256-fiber cap is gone.

tests/golden/joins pins two programs on both engines.
`staggered_sleeps` printed `woke b`, `long done`, `woke a` and now prints the
order its sleeps set: a at 100 ms, b at 130, the long sleep at 1030.
`what_follows_a_join_waits_for_it` printed the sibling last and now prints
it at 20 ms, between the group's quick and slow members, with `after the
group` still after both. Five concurrent 300 ms requests to the fork-per-accept
server finish in 0.31 s on both engines.

The runtime's scheduler is larger, so each benchmark's text grows by 1,392
bytes and the text row sums to 5,725,200; no benchmark reaches a join inside
a fiber. CI's rows move by layout, both ways: work_basket 18,296,046,
work_escapebench 40,309,374, work_indexbench 1,888,160, work_scanbench
268,196, work_oneshot 9,980,369 and work_livebench 938,655,428 rose, and
deepbench (197,807,203), runbench (846,331,178), jsonbench, encodebench,
digestbench, pendbench, readbench and widebench fell. clang compiles the
larger runtime at the dev tier in codegen_instructions_dev 116,928,442 and
at the release tier in 371,713,655, fewer than before. The browser runs the
corpus in browser_run_instructions 64,598,017, 44 fewer.

Open: natively a group still runs under one beat, grow-only until the group
ends, and a server's group never ends. A native fork-per-accept server held
2.0 MB at start, 3.9 MB after 1,000 requests and 9.0 MB after 4,000, about
1.7 KB a request. Reclaiming at the scheduler's idle points, with the
fibers' work as the roots, is the second half of #2263.

## 2026-11-01 — a request carries its headers

`std/net/http`'s `request` gains a fourth field, `headers`: each header's
name, lowercased, mapped to its value with the surrounding spaces trimmed. A
colon inside a value stays part of it, and a name sent twice keeps the last
value. The parser had already split the head into lines and discarded all
but the first. A server could not read a cookie, so senro could not keep a
session.

A pattern on `request` takes four places now, so the teashop's `handle` arms
in the web chapter end in `_`, and the chapter says what the last two places
hold. Three tests in http_test.kso pin it: a name read in lowercase with its
value trimmed and its inner colon kept, the host header, and a request with
no headers answering none. Breaking the lowercasing turns two of them red.

## 2026-11-01 — the corpus forks

Corpus first, before #2268. #2268 makes a server's loop hold what one
request holds, and with CI's rows its welfare read just under the floor: no
benchmark forked, so the objective saw the cost of the new runtime code and
none of the memory the change saves. This adds that shape on main, so the
fix is measured against it.

`bench/runbench/runbench/forked` runs 16,000 passes of a loop that forks a
small handler beside the rest of the loop, the way a server forks per
accept. `bench/interp_corpus` runs the same loop for 2,000 passes before it
prints. Neither printed line moves.

Rows, from CI where they are instructions. runbench rose to work_runbench
892,161,749, the shape's 45,830,571 instructions or 5.14% of the program,
and its text row to 849,809. Its arena grew from 4 blocks to 15:
arena_peak_bytes 14,680,064, and spare_peak_bytes 14,155,792. The
interpreted corpus rose to interp_instructions 622,671,293, interp_allocs
814,155 and interp_peak_bytes 1,953,942. In the tab, the corpus reads
browser_compile_instructions 349,229,303, browser_compile_peak_bytes
855,995, browser_run_instructions 79,318,152 and browser_run_peak_bytes
3,801,088, measured with the rustc 1.98.1 the golden names. runbench's
emitted rows grew by the shape's 15 functions. The floor is re-based to what
these rows score, since the change is only to what is measured.

The run program's other counters move with the 16,000 groups and the
handlers' pages: run_allocs 1,930,504, run_alloc_bytes 237,915,969,
run_arena_blocks 15, run_arena_peak_bytes 14,680,064, run_perm_allocs 131,
run_beat_iters 211,300, run_survive_slots 4,703, run_push_mut_fast
1,195,311, run_push_mut_slow 1,645,465, run_bytes_malloc 7,868,
run_str_scans 19,402, run_str_scan_bytes 18,667,266, run_buf_reuse 157,
run_ten_frees 0, run_ten_handups 0, run_sh_str 17,954,080, run_sh_buf
92,421,040 and run_spare_peak_bytes 14,155,792. runbench's 15 new functions
take the emitted rows of the other benchmarks to emitted_other_defines
2,327, emitted_other_calls 21,198, emitted_other_branches 18,260 and
emitted_other_lines 174,304, and the text rows sum to text 5,730,080.

## 2026-11-01 — a server's loop holds what one request holds

The second half of #2263. A server forks each request's handler beside the
rest of its loop, so every pass is a group the next pass is a member of, and
the groups nest as deep as the server has run. Both engines kept every one.
tests/golden/mem/a_server_loop_holds_what_one_request_holds.kso is that
shape without the network: natively 4,000 passes held 4 arena blocks and
16,000 held 13; interpreted, the peaks were 1,420,281 and 5,581,819 bytes.

Three things held the memory, and each fix was watched red on its own
before the spec passed.

A group whose other members have finished cleanly, and whose join was all
its fiber had left, will yield exactly what its last member yields. The
scheduler now hands that member to the parent group and frees the group's
slot; the next join takes it. Nothing observable moves. The settled node the
group would have handed back ran next in any case, at the same position,
because every fiber ahead of it waits until later, so a failure lands in
the parent at the same point in the order it did before. The interpreter
learns that the join was the fiber's whole work from a rejoin that is
absent; the runtime learns it from a continuation that is its settled node.

Natively, the scheduler ran under one beat, so everything its fibers
allocated stayed in the arena until it returned. It now stages the way the
bind chain does. Once the arena has drifted a quarter megabyte past the last
stage (or twice what that stage copied), the fibers' work and the live
groups' continuations and results go out through the carry pair as one list,
and the beat is rewound. A continuation's settled node is found again by
walking the copy, since binds and rescues keep what they wrap in `x`, rather
than carried as a root of its own; two roots could be copied apart, and the
group's result would then be written into a node its continuation no longer
holds.

In the interpreter, a global's resolution slot was keyed by the syntax node.
The pipe builds its callback's body by cloning the head on every evaluation,
so each pass's clone took a new slot until the table reached its cap of
65,535, after which every new node resolved by name. A name now keeps the
slot it first took. An effect chain of 16,000 links peaked at 1,134,478
bytes and now at 21,608, one byte more than at 4,000.

After: one arena block at either size natively, each stage copying about
350 bytes (beat_iters 49 and evac_bytes 17,488 in the mem golden), and
interpreted peaks of 41,986 and 41,987 bytes.
tests/a_server_loop_holds_no_more_the_longer_it_runs.rs compares the two
sizes on both engines. #2263 is done.

Rows, from CI, against the corpus with the forking shape. runbench's arena
falls from 15 blocks to run_arena_blocks 4, run_arena_peak_bytes 4,718,608
and run_spare_peak_bytes 2,621,456. The stages that buy that copy a little:
run_beat_iters 211,349, run_evac_allocs 2,156, run_evac_bytes 4,599,264,
run_allocs 1,930,602, run_alloc_bytes 237,920,257 and run_sh_buf
92,424,544. The interpreted corpus peaks at interp_peak_bytes 1,366,143
against 1,953,942, with interp_instructions 621,869,223 and interp_allocs
810,137, and the tab runs it in browser_run_instructions 78,361,551 with
browser_run_peak_bytes 2,621,440.

The runtime gains the stage and the group bookkeeping, 1,792 bytes of text
in each benchmark (runbench's text row 851,601, and the text rows sum to
text 5,755,168). runbench itself falls to work_runbench 891,782,287. No
other benchmark forks, so none runs the new code, and their work rows move
by layout: work_deepbench rose to 199,830,851, work_widebench to
28,766,254, work_encodebench to 2,335,341,102, work_pendbench to
154,125,558, work_oneshot to 9,980,977, work_livebench to 938,655,609,
work_jsonbench to 549,338,202, work_digestbench to 4,104,932,
work_readbench to 4,584,342, work_escapebench to 40,309,377, work_basket to
18,296,047, work_indexbench to 1,888,161 and work_scanbench to 268,197.
clang compiles the larger runtime in codegen_instructions_dev 116,951,400
and codegen_instructions_release 371,843,999, and entry_instructions reads
86,797,006.

## 2026-11-01 — a module may name a function a derivation writes into it

#2265. An entry that derives loads its dependencies twice: once to describe
them to its derivations, and again with the arms those derivations defined,
each joined to its module as one more file. The first load refused any name
nothing had written yet. A module could still call such a name on an
argument, because a call on a value a function was handed travels to the
module that owns the value's type and is resolved later. Passed as a value,
as in `list/map rows post_from`, the same name was refused. senro's
guestbook hit it with a reader `models/tables` could have written for each
model.

The first load now sets an unknown name aside instead of refusing it, and
records that it did. The second load runs whenever a name was set aside,
even when no derivation defined anything, and refuses whatever is still
unknown with the same words as before. An entry that derives nothing loads
once, as it did.

Two goldens in tests/golden/derive pin it on both engines:
a_module_calls_a_name_its_derivation_defines prints `["hello" "again"]`,
and a_module_calls_a_name_nothing_defines is refused at `post_form` with
the message an unknown name always had. With the setting-aside turned off,
the first goes red, refused as an unknown name at `post_from`.

Rows, from CI on the tree merged with #2268. The tab compiles its corpus in
browser_compile_instructions 349,228,936, 367 fewer. The compiler's own rows
rise by the set-aside check and the layout it moves: compile_instructions
1,098 to 24,098,635, library_instructions 1,097 to 72,619,875 and
entry_instructions 1,961 to 86,798,967. Welfare falls 0.000004, and the
floor is lowered by exactly that to 95.61270397: the feature is #2186's
ruling that derived declarations join the program as if written.

## 2026-11-01 — gavel: a pool is a channel without order

#2264 asked how fibers hand each other a value, since a connection pool needs
one fiber to wait for another to give a connection back. Clay ruled for a
pool: "a generic structure similar to a channel which is basically just a
channel without order and doesn't care particularly about connections or
anything like that. it can be used on any arbitrary thing."

- A pool holds up to n values of any kind. Taking waits while it is empty and
  answers some value in it, with no order promised. Putting waits while it is
  full. Both are effects, parked and woken by the scheduler that #2263
  reworked.
- No order is promised to the program, but the scheduler stays deterministic,
  so a program takes values in the same order on every run and every engine.
- FIFO channels are not added. A later need for them comes back as its own
  question.
- The public names go to Clay as one question before they ship: #2274, which
  recommends `std/pool` with `make`, `take`, `give` and `with`.

Owed: the pool on both engines and in the tab; goldens for a take waiting on
an empty pool, a give waiting on a full one, the hand-out order on every
engine, and a deadlock reported rather than hung; and senro's per-request
connections moved onto a pool.

## 2026-11-01 — gavel: a server rescues its handler's failure in library code

#2252 proposed endpoint policy as an ordinary function, `on_failure report`.
Clay declined it as a language feature. senro and the application are
separate hakos, and rescue works across hakos, so senro's serve loop can
already rescue a handler's failure and answer 500. Clay: "if the server code
itself needed to rescue, then you should have just been using some
alternative to exceptions in the first place because you don't want to use
exceptions for control flow." A failure a hako expects to handle itself is a
value (`none` or a marker), not a boxed err. A plain program's top-level
report is unchanged.

Owed in senro: the serve loop rescues a handler's failure and answers 500,
with a spec showing the server keeps serving afterwards.

## 2026-11-01 — gavel: an HTTP status is a value, never an err

#2253 proposed classifying a failure by where it was born and a `retry n` on
events. Clay closed both: "an HTTP code is not an exception. it's a thing that
can actually happen." A status is part of the response, which the program
dispatches on, so an HTTP client never turns a status into an err; only a
round trip that could not complete fails. This follows from #2087's ruling
that expected outcomes are values. Whether to try again is the author's
decision in ordinary code, because the author knows whether the request is
idempotent. A library `retry` comes back as a question when an application
needs one.

The checks it asked for come back clean. std/net/http has no client yet, only
the server, so nothing turns a status into an err. Chapter 04 and the rest of
the site carry no prose built on the retired page's status-code example.

## 2026-11-01 — a long chain inside a fiber costs no stack and keeps no garbage

#2276: a chain of `.>` steps that never waits runs within one step of its
fiber, and both engines walked it by recursion, one frame per link. Natively
that ran out of stack at a hundred thousand links once a second fiber stood
beside the chain. The interpreter's gigabyte stack held out until three
million. A chain in a program's only fiber never reached this code, because
`k_exec` already walks a chain in a loop.

Both steps now go round a loop: a link that answers another description
becomes the next thing to step, and the interpreter's three chain words share
one arm where they had three. The order of the program cannot change, since
the whole chain still runs within the step it ran in.

The loop exposed the native step's second problem. The scheduler reclaims
only between steps, so a long chain kept every link's garbage until it ended:
140 bytes a link, 425 MB at three million. Every 256 links the step now
asks whether the region has drifted as far as the scheduler lets it before
staging. When it has, the step returns the rest of the chain as a new kind of
step. The scheduler keeps the fiber's wake time, stages at the top of its
loop, and picks the same fiber again, since nothing else moved. Peak memory
for the reproduction is 10 MB at a thousand links and at three million.

The first version cost runbench 2.1 million instructions (CI's reading,
891,782,287 to 893,890,434). Measured under callgrind in this container, which
read runbench within a hundred instructions of CI: the loop alone made it
1.28 million cheaper than main. Asking about drift after every link cost
650,000, and a separate loop in the scheduler for the new kind of step cost
2.5 million in the scheduler code around it, which is inlined into `k_exec`.
Asking every 256 links, and folding the new kind into the branch that already
handles a blocked fiber, leaves runbench 685,000 instructions under main
here.

Specs, each watched red against main:

- `micro/a_long_chain_beside_another_fiber_runs_to_the_end`, 150,000 links on
  every engine. On main the native run dies with "the program ran out of
  stack".
- `oracle.rs`'s `a_long_chain_in_a_fiber_needs_no_stack_per_link` runs the
  interpreter on that sample with an eight-megabyte stack. On main the thread
  overflows.
- `mem/a_long_chain_in_a_fiber_reclaims_as_it_goes`, 20,000 links:
  `arena_blocks=1` and `arena_peak_bytes=1048576`. On main it reads 4 blocks
  and 4 MB.

No existing allocation golden moves. With the test after every link, the
server loop and the run program each staged 1,152 bytes fewer; asked every 256
links, both read what they read on main.

CI's rows, read on the recorded AMD silicon: `work_runbench` falls 684,996 to
891,097,291. The interpreter's step loop costs the interpreted corpus 426,003
instructions, `interp_instructions` 622,295,226, the price of one loop in
place of three recursive arms. The runtime's machine code is 1,568 bytes
longer in every benchmark, `text` 5,777,120, and the dev tier compiles it in
`codegen_instructions_dev` 116,955,821 while the release tier falls to
371,732,303. The tab runs the same runtime and gains the same loop: it runs the
interpreted corpus 205,146 instructions cheaper, at 78,156,405. Welfare rises,
and the floor is banked with it.

## 2026-11-01 — the scheduler is kept out of line

`k_schedule` is static and has one caller, `k_exec`, so clang inlined it. The
scheduler's loop then sat inside `k_exec`, which is called once for every
effect a program runs, and every one of those calls paid for its wider frame
and prologue whether or not the program ever forked. Marking it `noinline`
takes runbench from 891,097,386 to 890,386,161 instructions under callgrind in
this container. Runbench calls `k_exec` about 122,000 times, so that is close
to 6 instructions a call.

The tab runs the same runtime compiled to wasm, and there the inlined loop is
the cheaper one: out of line, `browser_run_instructions` rose 42,057 to
78,198,462. The attribute applies only outside wasm, and the tab reads its
golden again.

I found it while pricing the pool (#2264), which adds to the scheduler loop
and so to the inlined body. It lands first so the pool's floor drop counts
only the pool.

CI's rows, read on the recorded AMD silicon. `work_runbench` falls 711,225 to
890,386,066, `work_deepbench` falls 1,971,289 to 197,859,562 and
`work_widebench` falls 79,936 to 28,686,318. The runtime's machine code is 16
bytes shorter in every benchmark, `text` 5,776,896.

Five rows rose by small amounts: `work_jsonbench` to 549,338,235,
`work_readbench` to 4,584,375, `work_digestbench` to 4,104,967,
`work_livebench` to 938,655,546 and `work_encodebench` to 2,335,340,787. That
is 33 to 581 instructions on runs of 4 million to 2.3 billion. Callgrind
shows no `k_schedule` frame in readbench or livebench, so none of this is the
scheduler's own work. I read it as code that moved when the function left
`k_exec`, without having isolated it. Clang does a little more work compiling
the runtime with the function kept apart: `codegen_instructions_dev` rises to
116,962,084 and `codegen_instructions_release` to 371,787,654. Welfare rises,
and the floor is banked with it.

## 2026-11-01 — std/pool: fibers share values through a pool

The pool #2264 ruled for is built on the interpreter, natively and in the tab.
It ships as `std/pool` with `pool/make n`, `pool/take p`, `pool/give p v` and
`pool/with p work`, the names #2274 recommended and Clay chose.

A take from an empty pool or a give to a full one parks its fiber. Natively
the step answers a kind of its own, and the scheduler gives the fiber a wake
time no clock reaches. A take or give that succeeds marks the pool as moved,
and before the scheduler next picks a fiber it wakes every parked one at the
current time. Each tries again in spawn
order, and one that still cannot proceed parks again. That is a retry for
every parked fiber each time any pool moves. For a few connections it costs
nothing worth measuring, and a program holding thousands of parked fibers
would want a wait list per pool.

When the earliest wake left is the parked mark, every fiber is waiting on a
pool and nothing can wake any of them. The scheduler then wakes them all once
more with the pool marked as failing, and each stuck take or give answers an
err that says which it was. The program ends with those reasons instead of
hanging; `every_fiber_parked_on_a_pool_is_reported` pins the report on both
engines.

The ruling promises no order of hand-out, and the scheduler keeps one anyway:
a take answers the value given last, on every engine.
`a_pool_hands_out_in_one_order` runs four fibers over two values and pins the
transcript.

Natively a given value is deep-copied into storage the pool allocates for
itself, and a take copies it back into the arena. The pool holds nothing in a
region a beat can rewind, so a value given in one request survives the
reclaiming that follows it.

`pool/with` hands a taken value to the work and gives it back however the work
ends. #2274 said it would use annotate (`.!`), and it cannot: a failure passed
as an argument goes straight through the call, so `give` would never run. It
rescues the work's failure with `.?` instead, which std/pool may do because
the failure is foreign to it, gives the value back, and raises the failure
again with `effect`, so the reason and birthplace are the ones the work gave
it. The issue now says so.

A stress test for the pool found #2276, a long chain inside a fiber that ran
out of native stack. It needed no pool to show, and #2278 fixed it first. That
fix lets a long chain stop partway through its step so the scheduler can
reclaim, and the pool has to respect it. A give made inside such a chain
would otherwise wake the waiting taker at the stop, before the chain had
finished, where the interpreter runs the whole chain first. The scheduler
leaves parked fibers alone until the chain's step is done.
`a_give_inside_a_long_chain_wakes_the_taker_after_it` pins the order: without
the guard, the native run printed the taker's line first.

The pool is a ruled feature, so the welfare floor drops by exactly what CI
measured, 0.00159831, to 95.61462007. The figures below are against main with
the scheduler kept out of line (#2279).

`work_runbench` rises 527,874 to 890,913,940, because the scheduler asks
whether a pool moved before each pick and tells a parked step from the others
after it. The first version cost more. Inlined into `k_exec`, the bigger
scheduler made every effect dearer, which is what #2279 fixed. Giving a
parked fiber its own step kind, in place of a blocked step with an
unreachable wake time, took about 320,000 more off runbench under callgrind
here.

The runtime carries the pool's code, so every benchmark's machine code is
3,712 bytes longer, and `text` sums to 5,828,864. The other work rows move by
layout: `work_oneshot` rises 325 to 9,979,993, `work_encodebench` 262 to
2,335,341,049, and `work_jsonbench` 549,338,238, `work_widebench` 28,686,321,
`work_digestbench` 4,104,970, `work_readbench` 4,584,378 and `work_livebench`
938,655,549 rise by 3. Clang compiles the bigger runtime in
`codegen_instructions_dev` 117,030,226 and `codegen_instructions_release`
371,871,683. The tab runs the interpreted corpus dearer, at
`browser_run_instructions` 78,338,381, and compiles it dearer, at
`browser_compile_instructions` 349,271,241. The compiler's rows move with the
bigger builtin tables and library: `compile_instructions` 24,102,309,
`entry_instructions` 86,809,695, `library_instructions` 72,627,848,
`emit_instructions` 5,599,665 and `interp_instructions` 622,391,507. No peak
and no allocation counter moves.

One CI run landed on an Intel runner (family 6), where every work row reads
130 instructions lower and the codegen rows differ too, because glibc picks
its string routines by CPU. Those rows went into the goldens for one push and
the next AMD run refused them. The rows above are the AMD readings, the
silicon `bench/dispatch.txt` records; a run whose `interp_sample` line names
another CPU family is not a source of rows.

## 2026-11-02 — gavel: the pool's names are make, take, give and with

#2264's ruling asked for the pool's public names as one question before they
shipped, and #2274 asked it. Clay chose the recommended option: `std/pool`
with `pool/make n`, `pool/take p`, `pool/give p v` and
`pool/with p (v -> work)`.

- `give` is the word for putting a value back, which keeps it apart from
  `put`, the map insertion.
- `with` takes a value, runs the work with it, and gives the value back
  whether the work succeeds or fails. A failure still rises with the reason
  and birthplace the work gave it.

Owed: nothing beyond #2264's list. The pool (#2277) already uses these names.

## 2026-11-03 — gavel: senro is built out under a mandate, and keeps old paths only as aliases

#2280 asked how a senro app keeps an old URL working once every route is
RESTful. Clay ruled on that and widened it.

- The declaration is `legacy_route_aliases`, a name that marks it as a
  compatibility measure. Every resource still gets its RESTful route, and an
  alias can only point at one. A wrong alias is refused while the app
  compiles.
- senro gets singular resources, as Rails has them: a resource with no
  `:id`, such as `/profile`.
- senro's design is delegated. Clay: "go to town and spend weeks expanding
  the framework with a clear idea of what my underlying philosophy and
  principles are, adding authentication a la devise, and basically making
  this kanso alternative to ruby on rails." Cloud makes senro's design calls
  and brings the whole to him for review. CLAUDE.md states the rule.

Under the mandate, an alias answers with a permanent 308 redirect to its
canonical path, so each resource keeps one URL.

Owed: senro's principles written down for Clay to correct, then
`legacy_route_aliases` and singular resources with specs.

## 2026-11-03 — gavel: templates are files of their own, and a library compiles them while the program compiles

senro's views were Slim files, each with the kanso file kata generated from it
checked in beside it. Clay: "i don't want to see actual slim files in here, i
just want a special kanso-specific format... designed from first principles,
and not paired with a kso file. it should stand on its own and work and have a
conversion/compilation step." On how: "probably this means kanso provides
some kind of templating layer where you can create an arbitrary format that
parses into normal kanso. i dunno. whatever you think best."

The layer is the derivation, which already runs a library's kanso while the
program compiles and can write source into a module (`derive/define`). Two
things were missing, and this change adds them:

- `derive/module` gains `files`: the text of each file in the module's
  directory that is not kanso source, by name. A convention reads its
  templates there.
- A directory that holds no kanso source is still a module when a derivation
  writes its functions. While the entry's dependencies load to be described,
  it loads empty; the second load compiles what was written, and refuses the
  directory if nothing was.

Specs: `a_derivation_writes_a_module_from_its_templates` and
`a_module_of_templates_nothing_writes` in the derive corpus, on both engines.
The first failed with `no .kso files in …/views` before the loader change.

The format is senro's to design under #2280's mandate. kata's templates
become `.kata` files and its syntax follows kanso: `takes` names the
parameters, holes are `{expr}`, control lines are bare `if`, `else` and `for`,
and `#` starts a comment. That work lands in senro.

Describing a program now costs a little more, because each module's record
carries its files and the loader asks one more question of an empty
directory. The rows rose, by CI's measure: entry_instructions to 86,840,306
(+30,611), browser_compile_instructions to 349,289,583 (+18,342),
library_instructions to 72,628,273 (+425), emit_instructions to 5,599,909
(+244), compile_instructions to 24,102,378 (+69) and interp_instructions to
622,391,510 (+3).
The ruling is a part of the language, so the welfare floor drops by exactly
that cost, to 95.61456.

## 2026-11-03 — a write to a peer that has gone is an err on both engines

A senro server died without a word when a page that had been listening for
changes closed. Native code wrote to the socket with `write()`, so the closed
peer raised SIGPIPE, which the driver leaves at its default so that a program
behind `head` ends quietly. The interpreter writes through Rust's std, which
suppresses the signal, and answered `cannot write: Broken pipe (os error 32)`,
a wording the host supplies. The engines disagreed, and the interpreter is the
oracle.

The native write now ignores SIGPIPE for its duration and fails with EPIPE.
The runtime runs one thread, so the handler it sets around the write is its
own. Both engines say `cannot write`. The micro corpus's
`a_write_to_a_peer_that_left_is_an_err` dials its own listener, closes the far
end and writes; before the fix the native run exited 141 with no output. The
tab has no sockets, so the program is listed in the native route's known gaps.

The two calls around the write cost code, by CI's measure. Every benchmark's
machine code grew by 64 bytes, so `text` rose to 5,829,760 across the
fourteen. Clang does more work on the runtime: codegen_instructions_dev to
117,032,350 and codegen_instructions_release to 371,872,769. Four run rows
moved with the layout: work_oneshot rose to 9,980,896 and work_runbench to
890,914,087, while work_encodebench fell to 2,335,339,866 and work_livebench
to 938,655,381. Welfare falls by a sliver.
Engines that agree are part of the language's specification, so the floor
drops by exactly that cost.

## 2026-11-03 — kanso build writes a module a browser can run

`kanso build page.kso --wasm` writes `page.wasm`: the emitted module
lowered by `ir_wasm` to a wasm side module, the one the playground links
against `runtime.c` built for wasm32. A page needs two numbers before it can
link the module, the data bytes and the table slots it asks for, and they
ride in the module as a custom section named `kanso.link`, two
little-endian u32s, so the file is all a page has to fetch. senro's views
compiled for the browser are built this way (#2280).

`tests/build_wasm.rs` builds a two-module program with the flag, reads the
section back, links the module against the runtime under wasmi and checks
that it prints. It failed on the unknown flag before the change.

The flag is a branch in the driver, and the binary's layout moved with it, by
CI's measure. Four front-end rows rose by a handful of instructions:
compile_instructions to 24,102,382, entry_instructions to 86,840,310,
library_instructions to 72,628,277 and startup_instructions to 50,267.
codegen_instructions_release rose to 371,872,843. emit_instructions fell to
5,599,243. Welfare falls by a sliver, and the
browser build is part of the senro mandate, so the floor drops by exactly
that cost.


## 2026-11-03 — an editor asks the compiler what a name is

Clay asked for a language server that works in RubyMine and in the
playground, whose hover shows the type the compiler worked with. `kanso lsp`
speaks the Language Server Protocol over stdin and stdout. It publishes the
diagnostics `kanso check` prints, answers a hover and jumps to a definition.
The playground runs the same `Server` through a wasm export, `kanso_lsp`,
which takes one message and returns the replies as a JSON array.

The diagnostics are read out of the check's own rendering, so an editor shows
the words the command line does, from every compile route, with no second list
to keep in step. A hover reads the program the check compiled. A parameter's
type is what it accepts: each call the parameter is handed to contributes the
union of what the arms it can reach take at that position, and the
parameter's uses are intersected. An arm that binds the position by name passes
on what that name accepts, so the answer follows a value through every call it
passes, as a greatest fixpoint over the whole program. With `fn bar s:string`
and `fn bar n:int`, `fn foo x` whose body is `bar x` shows `x: int | string`.
When the program's own calls hand the parameter something narrower, the
inference's `param` set, which the engines dispatch on, appears on a second
line. Unions are spelled the way typesets and appendix B spell them.

A use the walk does not read constrains nothing: an operator's operand, a
builtin's argument, a capture in a lambda. The answer can be wider than the
truth and never narrower. Builtins are the gap most readers will meet, since
`text/split s sep` hands both to `builtin_split` and so reads as accepting
anything; the interpreter knows each builtin's argument types only by refusing
the wrong ones at run time, and the table that would close the gap is a
follow-up. A buffer that does not compile has diagnostics and no hovers.

`tests/the_language_server_answers_an_editor.rs` replays three recorded
sessions from `tests/golden/lsp` and compares every reply byte for byte. The
overloads transcript went red when a call contributed only its first arm, and
green with the union.

The server is outside the compile path, and main.rs gains only the verb, so
the compile rows moved with the binary's layout, by CI's measure on the tree
merged with `kanso build --wasm`: compile_instructions to 24,099,445 (-2,937),
entry_instructions to 86,832,572 (-7,738), library_instructions to 72,621,853
(-6,424), startup_instructions to 50,264 (-3), emit_instructions to 5,600,882
(+1,639) and interp_instructions to 622,499,949 (+108,439).
codegen_instructions_release rose to 371,873,010 (+167), which this diff does
not explain: the emitted IR and the runtime are unchanged. The tab's compile
rose too, browser_compile_instructions to 349,339,330 (+49,747); the wasm
engine now carries a second caller of `compile_source`, which is a likely
reason, though nothing here isolates it. Welfare fell by less than 0.01.
Editor support for the language is part of making it usable, so the floor
drops by exactly that cost.

## 2026-11-03 — the usage text names build --wasm

`kanso --help` listed `build <file|dir> [--release|--counters]` after #2286
added `--wasm`. The usage line now names the flag and says what it writes,
and `tests/front_door.rs` holds the line; it failed on the old text.

Only a string in the driver changed. By CI's measure, on the tree merged
with the language server, codegen_instructions_release fell to 371,872,995
(-15) and no other row moved. The emitted IR is unchanged; the binary's
layout is the likely cause, though nothing here isolates it.

## 2026-11-04 — a password stretched and a digest carried join the run program

`sha256/pbkdf2` peaked at 1,525,448 kB resident for 100,000 iterations
(#2299). The issue reduced it to a tail loop that hands `sha256/digest`'s
answer to its next iteration and carries a list it never reads, and that
loop keeps every digest's garbage too. Under the 2026-09-05 corpus-first
gavel both shapes join the run program here, measured on main before the
fix.

`stretched` runs `sha256/pbkdf2` for 200 iterations and `carried` runs the
postcard for 600 digests. On this container the release-tier run program
reads 1,045,660,009 with both. Without `stretched` (one iteration) it reads
1,023,809,139, a column of 21,850,870 (2.09%); without `carried` (none) it
reads 1,013,906,580, a column of 31,753,429 (3.04%). Each phase keeps its
loop's work: `stretched` about ten kilobytes an iteration and `carried`
about five. The counts are sized so the program's peak sees it: with both,
arena_peak_bytes reads 6,815,760 where it read 4,718,608, and at 250
digests `carried` stayed under the old peak and the objective could not see
it.

The mem vein takes the two fixtures, each at today's growth:
`a_digest_hashed_again_round_a_loop` (600 digests, three arena blocks) and
`a_password_stretched_three_hundred_times` (five). Their outputs agree with
Python's `hashlib`.

The issue's control row, the same loop without `p` at 2,296 kB, does not
reproduce on this tree. `chain u left`, a public loop with no wrapper, and
the wrapper with its unread list removed both keep every digest: five arena
blocks at a thousand digests, like the postcard. The extra parameter is not
the cause. `sha256/digest` answers through `list/fold`, and inference gives
a library function one answer for every caller in the program, so the
digest reads as any value. A loop slot that may hold any value is never
carried across a rewind.

CI's rows for the two phases: work_runbench 890,914,087 -> 947,406,395
(+56,492,308, 6.0% of the new whole), run_allocs 1,930,602 -> 2,040,700,
run_alloc_bytes 237,920,257 -> 245,492,129, run_arena_blocks 4 -> 7,
run_arena_peak_bytes 4,718,608 -> 6,815,760, run_spare_peak_bytes 2,621,456
-> 4,718,608, run_cohort_frees 11 -> 13, run_perm_allocs 131 -> 133,
run_push_mut_fast 1,195,311 -> 1,302,366, run_push_mut_slow 1,645,465 ->
1,657,081, run_buf_reuse 157 -> 7,813, run_sh_rec 4,675,856 -> 4,723,232,
run_sh_buf 92,424,544 -> 98,439,088, run_sh_bytes 5,112 -> 5,160,
runbench's emitted lines 69,443 -> 71,622 and its text 856,929 -> 889,681,
which take the totals emitted_other_branches 18,260 -> 18,498,
emitted_other_calls 21,198 -> 21,507, emitted_other_defines 2,327 -> 2,359,
emitted_other_lines 174,304 -> 176,483 and text 5,829,760 -> 5,862,512.
run_beat_iters fell, 211,349 -> 195,099, though both phases add loops. This
container reads 195,099 with the phases present as well. Inference is
whole-program, so new callers of the shared library functions can change
another phase's beat verdicts; that is a likely cause, and nothing here
isolates it. Welfare re-bases run_instructions and run_peak_bytes (6,842,513
-> 8,939,665), each keeping its ratio to its baseline, and the floor moves
only by the rounding of the two baselines.

## 2026-11-04 — pbkdf2 and a rehashed digest give their arena back

#2299. Both shapes joined the run program in #2301, measured before this.

**pbkdf2.** `chained` handed its next iteration two lists, the latest HMAC
and the running XOR. A list built in one iteration and read in the next is
a value that crosses a rewind, and the carry tier that could copy it out is
closed to library loops by the 2026-09-01 and 2026-09-08 entries. So the
loop kept everything every HMAC built. It now carries the sixteen words as
arguments, as `compress` has carried its eight since 2026-09-23, and the
beat report reads "rewinds every iteration". Each word read out of a list
is masked with `whole`: an item of a list could be anything to inference,
and the mask, a no-op on a word, makes the argument a number. Without the
mask the loop kept its grow-only verdict, "argument 2 may carry heap".
`paired_xor` had no other caller and is gone.

**The postcard.** The issue's shape is a loop that hands
`sha256/digest u` to its next iteration. Its slot read `0x37fff`, every
bit, and `costly_to_carry` refuses a slot that reads as anything. The cause
is the digest's answer. It spelled its words as bytes through `list/fold`,
and inference gives a library function one return set for every caller in
the program, so the fold answers anything and so does the digest. `bytes_of`
is now a push loop of its own, the slot reads `0x220` (a list or an err),
and the loop carries it. The general case, any function answering through a
shared driver, is #2302.

A first attempt narrowed `costly_to_carry` to the set the self-tail-calls
hand the slot. It changed nothing, because the handed set was the digest's
TOP, and `infer::builtin_set` answers TOP for a name it does not know, so it
could not have told a builtin from a library function anyway. It was
reverted before commit.

**Measured on this container**, the same box before and after:

| `pbkdf2`, 100,000 iterations, built `--release` | before | after |
|---|---:|---:|
| wall | 4.24 s | 0.68 s |
| user / sys | 0.93 s / 3.29 s | 0.67 s / 0.00 s |
| peak resident | 1,523,724 kB | 2,300 kB |

Output is byte-identical to Python's `hashlib.pbkdf2_hmac`, and the 13
vectors in `lib/sha256/sha256_test.kso` pass, 4,096 rounds among them. The
interpreter does not move: 10,000 iterations take 22.9 s before and 22.5 s
after. The mem fixtures read one arena block each, where they read three
and five.

Their other rows move with the loop that now rewinds:
a_digest_hashed_again_round_a_loop_beat_iters 52,828 -> 53,428,
a_digest_hashed_again_round_a_loop_buf_reuse 2,401 -> 1,187,
a_digest_hashed_again_round_a_loop_evac_allocs 33 -> 47,
a_digest_hashed_again_round_a_loop_evac_bytes 3,728 -> 7,536,
a_digest_hashed_again_round_a_loop_push_mut_fast 61,274 -> 43,874,
a_password_stretched_three_hundred_times_beat_iters 13,827 -> 14,126,
a_password_stretched_three_hundred_times_buf_reuse 3,614 -> 1,816 and
a_password_stretched_three_hundred_times_push_mut_fast 70,276 -> 44,205.
A loop that gives its arena back each iteration copies what it carries out
first, which fits the evacuations, and has no buffer of the last
iteration's left to reuse; the byte loop's pushes replace the fold's. That
reading fits the rows, and nothing here isolates each counter.

**What an iteration costs now.** Callgrind on 2,000 iterations reads
202,252,987 instructions, about 101,000 an iteration for two compressions.
`digested`, with the rounds and the schedule inlined into it, is 50%;
`big_sigma0`, `big_sigma1`, `majority` and `choice` are 34% between them,
called rather than inlined; list pushes and the fold that spells the first
HMAC are about 10%. A C SHA-256 spends a few thousand instructions on a
compression, so this is roughly ten times that. **OPEN:** inlining the four
mixing functions is the next lead.

CI's rows, on the run program with both phases: work_runbench 947,406,395
-> 941,723,266, run_arena_blocks 7 -> 5, run_arena_peak_bytes 6,815,760 ->
5,767,184, run_spare_peak_bytes 4,718,608 -> 3,670,032, run_allocs
2,040,700 -> 2,022,734, run_alloc_bytes 245,492,129 -> 243,362,481,
run_cohort_frees 13 -> 12, run_beat_iters 195,099 -> 195,898,
run_evac_allocs 2,156 -> 2,190, run_evac_bytes 4,599,264 -> 4,608,512,
run_push_mut_fast 1,302,366 -> 1,267,595, run_push_mut_slow 1,657,081 ->
1,653,484, run_buf_reuse 7,813 -> 5,385 and run_sh_buf 98,439,088 ->
96,452,880. The sixteen-argument loop and the byte loop are more code:
runbench's emitted lines 71,622 -> 73,713 and its text 889,681 -> 892,849,
which take the totals emitted_other_branches 18,498 -> 18,759,
emitted_other_calls 21,507 -> 21,738, emitted_other_defines 2,359 -> 2,358,
emitted_other_lines 176,483 -> 178,574 and text 5,862,512 -> 5,865,680.
Compiling the longer library costs entry_instructions 86,832,572 ->
87,188,418 (+355,846) and library_instructions 72,621,853 -> 72,961,217
(+339,364), read the same on cpu family 0x19 and 0x1a. Welfare rises 95.61
-> 95.63 and is banked.

## 2026-11-04 — bytes nobody can predict

senro's sign-in needs random salts and tokens, and std had only
`math/random`, which `KANSO_SEED` replays (#2298, under #2280).
`os/random_bytes n` is a description that yields `n` bytes from the
operating system's secure source. Native code calls `getentropy` in
256-byte pieces, or `arc4random_buf` on macOS, as desc 40. The interpreter
makes the same libc call from Rust. In the tab, wasi-libc's `getentropy` is
WASI's `random_get`, which `kanso-engine.js` already answered with
`crypto.getRandomValues`. The interpreter build, `kanso.wasm`, had no call
out to the page: its seed and sources are pushed in through exports. A
pushed buffer cannot know how many bytes a program will ask for, so it
gained one import, `kanso_host.random_fill`, which the page fills from
`crypto.getRandomValues` in 65,536-byte slices. The wasmi hosts in `tests/`
define it through `wasm32::page_linker` and leave the zeros. The import is
wasm32's alone: macOS refused to link the native library with it unresolved,
which Linux had allowed. Off the page the browser executor takes the
machine's bytes.

Three specs, each seen red. `random_bytes_are_as_many_as_were_asked_for`
in the micro corpus asks for 32, 1 and 0 bytes and appends `"ok"`, which only
bytes accept, so it pins the length and the kind of value on every engine the
corpus reaches. It went red at `31 ok` with native broken to one byte short.
`random_bytes_takes_a_count` in the runtime corpus pins the refusal of a
string on both engines. `tests/random_bytes_ignore_the_seed.rs` runs one
program twice under `KANSO_SEED=7` and holds the dice equal and the bytes
different. It went red when the interpreter drew from its seeded generator.
Under node with the page's host function, the interpreter build replays the
dice and not the bytes, and it fills a 70,000-byte ask.

The rows are from a job on 0x19 model 0x11, on the tree merged with main.
Before the merge, a job on model 0x1, the silicon the goldens were recorded
on, read the same numbers as one on 0x11. Every binary's `.text` grew by 272 bytes, the new
case and helper, so text across the fourteen benchmarks rose to 5,869,488
(+3,808). The runtime instruction rows fell by 45 to 1,578, though no
benchmark runs the new case, so layout is the likely cause; nothing here
isolates it. codegen_instructions_release rose to 371,998,504 (+125,509) and
codegen_instructions_dev to 117,050,653 (+18,303), likely clang compiling the
larger runtime. The front end moved too, and its code changed only by the
table entries, so again layout is the likely cause:
compile_instructions to 24,100,614 (+1,169), entry_instructions to 87,192,183
(+3,765), library_instructions to 72,964,207 (+2,990) and emit_instructions
to 5,601,693 (+811). interp_instructions fell to 622,464,159 (-35,790). In
the tab, browser_compile_instructions rose to 349,357,398 (+18,068) and
browser_run_instructions to 78,344,393 (+6,012). Welfare fell by less than 0.0001. A secure source of randomness is required
for the sign-in the senro mandate asks for, so the floor drops by exactly
that cost.

The page's sections for `kanso build --wasm` and the language server were
owed, and this change was the fourth entry since the page last moved, so
docs/compiler.html gains all three as sections 281 to 283.

## 2026-11-04 — a parameter handed to a builtin reads as what the builtin takes

Kanso#2288. The hover in `kanso lsp` followed a parameter into every group
it was passed to and stopped at builtins, because nothing recorded what a
builtin takes. `fn parts s = text/split s ","` read `s` as the whole union.

`infer::BUILTIN_TAKES` now holds it, beside `infer::builtin_set`, which says
what a builtin answers. A row is a set per position over the full tag
vocabulary. The check's own table in `src/check/literals.rs`, written in
literal kinds, is deleted, and the literal check, the none check and the two
list checks read the new rows. The hover meets a parameter's set with the
row's set at each position it is handed to a builtin, so `s` reads
`string`. `print` keeps one rule in literals.rs, the language's own: the
check refuses a literal that is not a string, while every engine prints any
value a program computes.

Each row carries a third field, how many of its leading positions the
compile-time checks enforce. The first build had the checks read every
position, and the suite showed what that costs: the none check refused a
key that may be none in `scripts/welfare_rescore` (40 failures across the
rescorer's specs), and the literal check refused `delete m 2.5`, which the
runtime corpus pins as the interpreter's refusal. Refusing more at compile
time is a change to the language, which #2288 did not ask for, so the
checks enforce exactly the positions the old table held. The hover reads
every position.

`tests/a_builtin_refuses_what_its_row_leaves_out.rs` checks every row
against the interpreter. For each position a row constrains, it hands the
builtin a value of each of fourteen tags the row leaves out, hidden behind
an index into a one-element list so no check that reads the row answers
first, and requires `error[runtime]` under `kanso play --interp`. A control
with only fillers must run without a refusal. 1019 probes. It went red
with `BYTES` dropped from `concat`'s first position, and red again in the
control with a float as `split`'s filler. The golden transcript
`tests/golden/lsp/builtins` went red with the hover's lookup removed.

Measuring rewrote several rows. `concat`, `push`, `length` and `utf8` take
bytes, `utf8` takes a list, `write_file` and `net_write` take bytes as text,
and `put` and `delete` take an int or a string as the key. `length`'s
refusal now names bytes, which moved two error goldens and the book's
appendix A sample. The handle positions of `net/read`, `pool/take` and their
kin are gone from the table: std hands them a record's `handle`, and only
their own module can build the record, so no program reaches them.

The probe ran native first by mistake, since `kanso play` builds a binary,
and found native `text/find2` and `text/find2_below` answering for any value
in their int positions where the interpreter refuses. That is #2310.

The rows are from a job on family 0x19 model 0x1, the recorded silicon.
compile_instructions fell to 24,090,335 (-10,279), entry_instructions to
87,188,542 (-3,641), library_instructions to 72,961,337 (-2,870) and
browser_compile_instructions to 349,295,198 (-62,200). Nothing isolates the
cause; the check now reads one table through one function, and layout may
account for some of it. No runtime, text or allocation row moved. Welfare
rose from 95.627876 to 95.627948, banked.

## 2026-11-04 — find2 refuses what the interpreter refuses

`text/find2` and `text/find2_below` take bytes and then three or four ints. The
interpreter refused anything else in those positions; native answered (#2310).
The emitted shims `k_b_find2_fast` and `k_b_find2_below_fast` test every tag
and send any call that is not bytes and small ints to the out-of-line doors
`k_b_find2` and `k_b_find2_below` in runtime/text.c. Those doors tested only
the bytes, so a bool, a string or a float was scanned with its payload as the
number, and a bignum with its pointer. The doors now refuse anything but an
int, in the interpreter's words (`find2 takes bytes`, `find2_below takes
bytes`).

A tag test alone would have made native refuse a bignum, which the interpreter
reads. So the doors read one as the interpreter does: a start past a word
scans from the front, a byte keeps the low byte of its magnitude, and a floor
past a word sits above every byte or below them all. A negative small byte
still reads two ways, the magnitude's low byte in the interpreter and `& 0xff`
in native. That one is on the scanning path and is #2313.

Four specs, each seen red. Three runtime goldens pin the refusal of a bool
start, a string byte and a float floor on native, `--interp`, the oracle and
the tab; native wrote nothing to stderr before the fix. The micro golden
`a_bignum_reads_the_same_in_every_find2_position` puts a bignum in each
position, and with the fix reverted native printed nothing where the
interpreter prints `2 2 3 2 4`.

The rows are from a job on 0x19 model 0x11, on the tree merged with #2311,
which moved none of the same rows. No scan path changed, since the
new tests sit behind the shims' own. Every binary's `.text` grew by 288 bytes,
so text across the fourteen benchmarks rose to 5,873,520 (+4,032).
codegen_instructions_release rose to 372,000,820 (+2,316), likely clang
compiling the larger runtime, and codegen_instructions_dev fell to 117,050,435
(-218). The runtime instruction rows fell by 1,890: work_encodebench to
2,335,339,191 (-630), work_oneshot to 9,978,933 (-385) and work_runbench to
941,722,133 (-945), while work_livebench rose to 938,654,286 (+70). No benchmark reaches the
doors with a non-int, so layout is the likely cause of all four; nothing here
isolates it. The front end, the emitter, the interpreter and the tab did not
move. Welfare rose by less than 0.0001 and the floor is banked.

## 2026-11-04 — find2 reads a negative byte as the interpreter does

The interpreter reads each byte `find2` and `find2_below` look for as the low
byte of the int's magnitude, so -98 is 98. Native's two raw scans read
`a & 0xff`, so -98 was 158, and `find2 (bytes "abc") 1 -98 0` answered 4
where the interpreter answered 2 (#2313). The interpreter is the oracle, so
native changed. Both scans read `& 0xff` first and correct the two bytes
only when `(a | b) < 0`. At the emitted sites the bytes are literals and the
test folds away.

The first version computed both magnitudes on every call, through a helper
that the release tier's hot unit and `a_short_scan_tail_answers_like_the_byte_walk`
each lift out of the runtime by name. The second lift did not know the
helper and the spec failed on both hosts, so the magnitude is computed in
place. That version cost the browser run 92,400 instructions; the single
sign test costs 52,800.

The micro goldens `find2_at_its_edges` and `find2_below_at_its_edges` each
gain a negative byte in the scalar walk and one in the vector step. Before
the fix native answered 5, 25, 10 and 23 where the interpreter answers 2, 6,
2 and 3.

The rows are CI's, on the recorded host. Twelve binaries' `.text` grew by
128 bytes, and scanbench's and runbench's, which inline the raw scans most,
by 1,248 and 1,264, so text across the fourteen benchmarks rose to 5,877,568
(+4,048). work_runbench rose to 942,094,162 (+372,029
over main). Four other work rows moved by a few hundred instructions:
work_encodebench to 2,335,339,429 (+238), work_livebench to 938,654,937
(+651), work_oneshot to 9,979,423 (+490) and work_scanbench to 267,717
(+141). An earlier run on the first version read encodebench 3.8M lower,
and this one reads it within 238 of main, so neither reading is the change.
browser_run_instructions rose to 78,397,193 (+52,800), the sign test on the
wasm runtime's calls. codegen_instructions_dev rose to 117,058,158 (+7,723)
and codegen_instructions_release fell to 371,954,589 (-46,231). The front
end, the emitter and the interpreter did not move. Welfare fell by less than
0.01. The differential law requires the fix, so the floor was lowered by
exactly that, with the reason in bench/welfare_floor.json.

## 2026-11-04 — sha256's rounds carry raw words, and pbkdf2's chain skips its bytes

`sha256/pbkdf2` was about nineteen times slower than OpenSSL at the release
tier (#2304). A 5,000-round run retired 440M instructions, 82% of them in the
inlined `digested` loop: about 36,000 a 64-byte block, where a C round is
some thirty instructions and ours was 355.

The eight working words crossed from round to round as boxed KValues, and
every bit operation re-tested its operands' tags. Three reads in
`lib/sha256` kept inference from proving them ints. `words_from` handed the
state's eight words to `blocked` as bare list reads, and the schedule handed
two of its words to the sigma functions the same way; both are masked with
`whole` now. The third was the round constant, read from `rounds`, which is
built by `list/to_list (list/concat ...)`. Inference gives a library function
one return set for every caller (#2302), so a read of `rounds` may be a
failure, and the failure reached `compress`'s first and fifth parameters as
`INT | ERR`. The constants now arrive as an argument. An argument never holds
a failure: a bare name refuses one when the arm is chosen, on both engines,
so the body's parameters are typed without it. With all three gone the eight
words travel as raw i64, a round is 162 instructions, and the run fell to
270M.

The second change is in the chain. Each link is an HMAC of thirty-two bytes,
and both of its hashes continue from a state one block in, so the one padded
block is known word by word: the eight words, `0x80000000`, six zeros, and
768 for the length in bits. `after_eight` builds the schedule from those
words. The chain used to make a list of the words' bytes, push the padding
onto a copy and gather the bytes back into words, twice a link. The run fell
to 190M, 57% under main. The eight results go back through a call, `eight`,
rather than a list literal, because a literal whose items might fail widens
every list read in the program (`store_fails` in infer.rs).

Tried and dropped: the schedule fused into the rounds as a sixteen-word
window of arguments, so no list is built. The digest was right and the run
rose to 205M. A loop of twenty-six arguments goes through the arena's carry
(`k_beat_lap_carry` and `k_carry_stage`, 15% of that run), which costs more
than the 48 pushes it saves.

The digest is the same on every engine, a 50-round digest matches Python's
`hashlib.pbkdf2_hmac`, and `kanso test lib/sha256` passes. Allocations fell
in the digest and run counters (runbench 2,022,734 -> 2,007,972). In the
mem vein, the password fixture went from 14,126 beat iterations to 349 and
from 3,290,352 allocated bytes to 1,526,976, and the rehashed-digest fixture
from 53,428 beat iterations to 15,028.

Some counters read worse by their direction and are less work. A link no
longer builds a list of bytes, so it pushes and reuses buffers less:
run_buf_reuse fell to 3,390, run_push_mut_fast to 1,238,867,
a_password_stretched_three_hundred_times_buf_reuse to 19 and
a_password_stretched_three_hundred_times_push_mut_fast to 478, while every
allocation and byte counter beside them fell too. The emitter writes the new
functions: across the other benchmarks emitted_other_defines rose to 2,365,
emitted_other_calls to 21,933, emitted_other_branches to 18,972 and
emitted_other_lines to 180,154, all of it in runbench and digestbench.

The rows are CI's, on the tree merged with the find2 fix above.
work_runbench fell to 916,918,990 (-25,175,172, -2.7%) and
work_digestbench to 3,288,791 (-815,411); no other work row moved.
runbench's and digestbench's binaries are smaller (text 891,873, -2,800,
and 349,633, -4,224), and the emitter writes 201 more calls into runbench
for the new functions. The library is part of every compile, so
entry_instructions rose to 87,745,536 (+556,994) and library_instructions to
73,432,676 (+471,339). The browser rows did not move. Welfare rose by 0.06
and the floor is banked. At the release tier 100,000 rounds take 0.22 s in
this container, where they took 0.45 s; the dev tier, which `kanso run`
uses, barely moved, from 3.1 s to 3.0 s.
