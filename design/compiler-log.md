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

## 2026-08-29 — four expressions name a type plainly, and the table read three

`check_field_exists` keeps a small table of which local holds which record
type, so that `p.z` can be refused where it is written rather than where it
runs. Four expressions say the type plainly enough to fill that table, and
the table read three:

```
pub fn f p:point / p.z          check: `point` has no field `z`
pub fn f r@(point x y) / r.z    check: `point` has no field `z`
p = point a b / p.z             check: `point` has no field `z`
p = (v):point / p.z             check: OK
```

An annotation names the type after a colon. A constructor pattern names it as
the head of the pattern. A construction names it as the head of the call. A
widening names it after a colon too, and `constructed_type` only knew how to
read the head of an `App`.

Nothing diverges here. Both engines refuse `p.z` at run time and print the same
sentence, so what was missing was the diagnostic arriving at the front door
instead of the back. The fixture pins the difference precisely: without the
arm it yields `error[runtime]` with no span, with it `error[name]` and the
column of the dot.

The function is `type_in_hand` now, because "constructed" was the whole
mistake — it named the one way a type reaches your hand and the reader of the
call site had no reason to doubt it. Both call sites move: the bind loop, and
`base_type`, which means a field read straight off a widening — `((v):point).z`
without the bind — is covered by the same arm.

Two controls hold. A real field through a widening still passes, and a widening
to a *subtype* still passes, because subtypes are not in `plain` and the
table has never claimed to know their fields.

This is the fourth pass in two days that answered a question about names and
read only `Expr::Ident` or `Expr::App`. The other three were the import
rewriter, the qualifier collector and the bare-name marker. `Expr::Upcast`
carries a type name after its colon and every one of them walked past it.

`compile_allocs` is 61,974, unchanged — the arm is a branch on an existing
walk and allocates nothing. `compile_instructions` reads -3,127 in the
container and +2,513 on the runner: opposite signs on one diff, which is this
file's layout signature. `lib/json` widens nothing, so the arm is never taken
on the measured path.

## 2026-08-29 — "a animal", on three engines, pinned by nothing

Two runtime messages put an article in front of a type name and always chose
`a`:

```
error[runtime]: `:int` widens; this value is not a int
error[runtime]: `age` wraps a int
```

Six sites, two messages by three engines. Each engine wrote the sentence with
its own format string, and none of them asked what letter the name begins
with. `article` has been in check.rs since the demand diagnostics were
written, and its own comment says a diagnostic that fumbles its grammar reads
as carelessness about everything else in it.

It lives in diag.rs now, which is the module that owns how a diagnostic is
written, and both Rust engines call it. `runtime.c` has `k_article`, five
lines, with a comment naming the Rust one so a reader finds the other half.

Two fixtures in tests/golden/runtime pin both messages on all three engines.
`("x"):int` is the widening: `int` is a builtin, so no import qualifies it and
the fixture says the same thing whatever the file is called. `type age int`
with `age "old"` is the wrapper, and its type name IS qualified through the
import, so it carries an `.imported.stderr` twin.

Watched red first. The widening fixture answered `not a int` against a golden
reading `not an int`, which is the whole change and nothing else.

**The gate could not see either message.** `diagnostic_coverage` keys a
message on `head_of` — the literal up to
its first interpolation — and drops any head under ten characters, because a
head of one backtick would match every fixture in the corpus. `` `:{ty}`
widens… `` has the head `` `: ``, two characters. So the scan reported 262
literal diagnostics and 38 unpinned before this change and after it, and
neither of these was ever in either count.

Replaying the scan's seven opener families over src/*.rs and src/runtime.c
finds 38 literals in that blind spot, these among them, and every `` `{ty}` has no field
`{name}` `` site. Two keys were measured against tests/golden plus the book
samples. The longest run between interpolations makes 29 of them visible and
false-pins one: `{name} takes a list` is matched by `accept takes a listener`.
The skeleton — every run, in order, on one line, with the same ten-character
floor on the total — makes 30 visible, reports 25 pinned and 5 unpinned, and
gets `field `{field}` of `{}` takes {}` right where the longest run could not
see it at all. The five it finds unpinned are real gaps. That is its own
change; the measurement is here so it does not have to be taken twice.

`compile_allocs` is 61,974, unchanged. `compile_instructions` reads +3,863 on
the runner and -3,950 in the container: opposite signs on one diff, layout.
`article` moves between two modules and gains three callers, `k_article` is
new in runtime.c and absent from this binary, and both messages they feed are
runtime messages that `kanso check lib/json` never reaches.

## 2026-08-29 — the gate keyed a message on its first few characters

`diagnostic_coverage` decides whether a diagnostic is pinned by looking for
its text in the corpus. The text it looked for was the leading run — the
literal up to the first interpolation — and any key under ten characters was
dropped, because one backtick matches every fixture there is.

So a message that opens with an interpolation was not unpinned. It was
invisible: not in the 262 the gate counted, not in the 38 it listed, not in
its world at all. `` `:{ty}` widens; this value is not a {ty} `` has the key
`` `: ``. That is how the article bug in the entry above sat in six sites with
nothing watching, and thirty-eight literals sat in the same blind spot,
including every `` `{ty}` has no field `{name}` `` site.

The floor stays. What changes is the key: the whole message, with each
interpolation written `{}`, so the field read keys as
`` `{}` has no field `{}` ``. `in_corpus?` then asks for every run of a shape,
in order, on ONE line. The line matters — runs matched anywhere in the blob
would let one fixture's opening backtick and another's closing backtick stand
in for a message nothing raises. The longest run is tried against the whole
corpus first, so the per-line walk only runs for shapes that could hit.

The other key measured was the longest run alone. It makes 29 of the 38
visible against the skeleton's 30, and it false-pins: `{name} takes a list` is
matched by `accept takes a listener`. It also cannot see
`` field `{field}` of `{}` takes {} `` at all, which the skeleton reports
correctly as unpinned.

**262 diagnostics became 309.** Eight of what the gate found were pinned in
the same change: an import of a std module that is not shipped, a rename of
something an import does not export, a `builtin_` name spelled outside the
standard library, a field typeset naming no type, and — in the module-shaped
corpus — two modules answering one bare name, a module with nothing to
re-export, a record built across an import, and a dependency that is only an
entry file.

Eight could not be pinned and each carries its reason in the list. Three are
the same shape twice over: the two engines refuse the same program at
different times in different words, and the runtime corpus asks both for the
same stderr. Two are unreachable through the harness that stages the corpus.
One needs a hako package, which no stderr corpus holds.

One is a divergence. `shape 1`, for a typeset `shape`, is refused by
the interpreter and by the page with the same sentence, and native prints
`<mod>/shape 1` and exits 0. Nothing can pin a message two engines raise and
the third answers with output. That is its own fix; the line stands until it
lands.

## 2026-08-29 — a typeset written as a constructor, and what native did with it

The gate widened an hour ago and listed `` `{}` is a typeset — it only
annotates `` as a message nothing pinned. Looking for a program that raises it
found this:

```
type circle
  r

type square
  s

type shape circle square

pub play = print "{shape 1}"
```

`kanso check` said ok. Then the interpreter refused it at run time with that
sentence, the page said the same thing byte for byte, and native printed
`<mod>/shape 1` and exited 0. Native had built a one-field record whose type
was the typeset, and a one-field record renders as its name and its field.

Naming one without calling it is the quieter half of the same thing:

```
pub play = print "{shape}"
```

also compiles clean, and prints `<mod>/shape` on native against `<fn>` on the
interpreter. Two engines, two answers, neither an error, nothing red.

And a third position, which both engines agreed on and both got wrong:

```
pub play = print "{(circle 1):shape}"
```

refuses at RUN time with "`:shape` widens; this value is not a shape" — a
sentence that blames a program whose circle is exactly what the typeset
admits. There is nothing to widen: the value already matches the annotation.

A typeset is annotation-only vocabulary. `type shape circle square` names a
union a parameter can stand in, and no value is ever a `shape`, which the AST
comment beside `members` has said since the field was added. So the refusal is
on the NAME — an `Ident`, a `&` partial, or a widening's target, anywhere an
expression can stand — and the call is the case where the name is a head. `typeset_constructions` in
check.rs says so where it is written, and all three engines agree because none
of them gets the name.

Annotations are untouched, which is the point of putting it on the expression
walk: a parameter's `:shape` is a pattern and a field's is a type list, and
neither is an expression. Both controls hold — a typeset as a parameter
annotation still compiles, and a record whose type is one of the typeset's
members still constructs.

**Why the interpreter's runtime test stays.** The walk reads the expression
tree of every function body. A typeset name that reaches `construct` some
other way — synthesised by a pass after this one, or through a shape the walk
does not model — would arrive with nothing to have caught it. Nothing in the
corpus writes that, and removing the test to find out is the wrong order.

The message existed and the divergence existed, and what found them was a gate
that could not see the message at all until its key changed. The scan reported
262 diagnostics for weeks with this one outside the count.

`compile_allocs` is 61,974, unchanged — the pass borrows its names and returns
at once when a program declares no typeset, which `lib/json` does not.
`compile_instructions` reads -882 on the runner against +1,182 in the
container: opposite signs on one diff, layout, and the runner's side is the
good side. The pass reads the type list, finds nothing, and stops, so there is
no walk to pay for on the measured path.

## 2026-08-29 — 482,913 instructions for an exact column, declined

The typeset sweep's fourth position is a constructor pattern:

```
fn area (shape x)
  x
```

Both engines compiled it and both answered "no overload of `area` matches
these arguments" at run time, which sends the reader to look at an argument
that is fine. A constructor pattern destructures a record by its type; a
typeset has no fields to destructure and no value is ever one, so the arm can
never match whatever arrives.

`Pattern::Ctor` carries no span, which is why kanso#1126 left this alone — its
comment says so in as many words. So the first attempt gave it one: thirteen
sites, three constructions in the parser, one in `resolve_marker_pattern`, one
in the getter synthesis, eight destructures gaining a `..`, and `other_span`
answering a real column where it had answered `0:0`.

**Then it was measured, and the field costs more than the column is worth.**

```
main after the typeset refusal          58,330,347
  + the span on Pattern::Ctor           58,813,260    +482,913
  + the span and the new arm            58,818,195    +487,848
  the arm alone, no span                58,329,928        -419
```

The arm costs 4,935 with the span in place and reads as noise without it. The
FIELD costs 482,913 — 0.83% of everything `kanso check lib/json` does — because
`Pattern` grows from 64 bytes to 72 and patterns are moved through every pass
in the front end.

What the field buys is a caret on `shape` instead of on `x`, one token to its
right. The arm points at the first field, which is where the two other
constructor diagnostics in check.rs already point: `other_span(&fields[0])` is
an idiom this file had before today. So the span is declined and the arm ships
without it, at 419 instructions below the baseline.

**The undeclared half is declined too, and for a different reason.** With the
span in hand I also wrote the check kanso#1126 wanted — a constructor pattern
naming no declared type — and the unit suite went red on the standard library.
`std/list` has `fn put_renamed acc (entry k v) f` in two arms, and `entry` is a
marker the compiler knows: `check.rs` puts it in `globals`, `codegen.rs` gives
it type id zero. `declared` is built from `program.types`, which does not hold
it, so the check called a correct program wrong. Putting `entry` in
`BUILT_IN_TYPES` would let an annotation say `:entry` as a side effect, which
is a different decision. The comment in `patterns()` names the missing set now
rather than the missing span.

**If the span is ever wanted, the cheap way in is to shrink `Span` first.** It
is two `usize` — sixteen bytes for a line and a column, where two `u32` would
hold a four-billion-line file. That change pays for this one several times over
and speeds up everything else that carries a span, which is most of the AST.
Not attempted here; recorded so the next reader does not re-derive the trade.

`compile_allocs` is 61,974, unchanged. `compile_instructions` reads +3,760 on
the runner against -419 in the container: opposite signs on one diff, layout.
The arm is a branch on a walk that already ran, and `lib/json` declares no
typeset, so the borrowed set is empty and the branch never fires on the
measured path.
## 2026-08-29 — `fields.is_empty()` is not the same question as "is this a value"

The typeset sweep turned up a second divergence one name over. `type age int`
is a subtype, and:

```
pub play = print "{age}"
```

compiles clean, prints `<fn>` on the interpreter, and prints `<mod>/age` on
native. The page refuses the name outright — "unsupported name `<mod>/age`" —
which is what the differential law permits and what native was not doing.

The emitter's test for "this name IS a value" was `fields.is_empty()`, meant
for `type unit`: a record type with no fields describes one thing, and naming
it builds it. A subtype has no fields either, and so does a typeset, so both
were emitted as nullary records of their own type id. A one-field record
renders as its name and its field, and a zero-field one renders as its name,
which is why the output looked like an echo of the program.

The test asks for three things now — no fields, no parent, no members — and a
subtype falls through to the refusal native has always given for a record type
that HAS fields. Three programs pin it: the subtype, the nullary record that
must keep working, and the record-with-fields whose older refusal the new one
borrows.

The sweep that found both is worth its four lines. Every name a program can
write bare, on both engines:

```
length          <fn>   <fn>      a builtin
list/map        <fn>   <fn>      an imported group
twice           <fn>   <fn>      a declared group
err             refused, identically, by both — it has no line to record
point           refused by native as its own limit; `<fn>` on the oracle
&point, &age    refused by native, the sentence kanso#1128 wrote
unit            `unit` on both — a nullary record IS a value
shape           `<mod>/shape` on native, `<fn>` on the oracle  (typeset)
age             `<mod>/age`   on native, `<fn>` on the oracle  (subtype)
```

Nine shapes, two wrong, and both wrong the same way: a type name that carries
no fields and is not a nullary record. The typeset half is refused at the
front door by the entry above; this is the other half.

## 2026-08-30 — a span was sixteen bytes and needed eight

`Span` held two `usize`. Every `Expr`, every `Stmt`, most patterns and every
diagnostic carry one, and a few carry two, so the width of that struct sets the
width of the whole AST. Four billion lines is a limit no source file reaches,
and a `u32` pair is half the size:

```
Span     16 -> 8
Expr     64 -> 56
Stmt    136 -> 120
Pattern  64 -> 64   (unchanged — its largest arm is not span-bound)
```

The lexer and the passes still count in `usize`, so the narrowing happens at one
place: `Span::at(line, col)`, which is what every construction site calls now.
`render` widens back for the source-line lookup.

Measured, `kanso check lib/json`. The runner's rows, which are the goldens:

```
compile_peak_bytes   822,004 -> 763,868       -58,136   -7.1%
compile_instructions 57,822,766 -> 58,205,543  +382,777  +0.66%
compile_allocs       61,974 -> 61,974          unchanged
```

`compile_peak_bytes` reads 763,868 on the container and 763,868 on the runner,
so that counter is host-invariant. The allocation count does not move because
the same objects are allocated; they are smaller.

The memory fall is the struct size and nothing else. A build with the same
`u32` fields and two words of explicit padding — same casts, same side tables,
`Span` back at 16 and `Expr` back at 64 — reads `compile_peak_bytes=822004`,
the baseline to the byte.

Where the instructions went is not settled. The container reads +50,058 for
the same change, an eighth of the runner's +382,777 — same sign, different
order — so some of this is work and some is two hosts laying the binary out
differently, and neither number decomposes it.

Callgrind will not decompose it either. It puts the container's rise almost
entirely in `walk_children` (+343k gross, against falls elsewhere), which
touches no span arithmetic at all; the padded probe rises by the same amount
overall, +47,430, and attributes it to `parse_atom` and `parse_stmt` instead,
with `walk_children` unchanged. Same total, disjoint explanations. About twenty
side tables key on `(String, usize, usize)` — beat.rs, check.rs, codegen.rs,
demand.rs, dispatch.rs, escape.rs — so a narrowed span is widened again to
build a key; the cast count is the right order of magnitude, and there is no
evidence for it beyond that.

Welfare: 84.10 -> 84.27, banked. The instruction term costs 0.074 points and
the memory term more than pays for it.

The next eight bytes were measured too, and they are a worse trade. `Expr` sits
at 56 because `Guard`'s payload is exactly 48 with no spare byte for the tag,
and `Guard.rest` is a `Vec<Stmt>` built at one site. As a `Box<[Stmt]>` it is
16 bytes instead of 24, `Expr` falls to 48 and `Stmt` to 112, and
`compile_peak_bytes` falls again, 763,868 -> 721,652, another 5.5%. It costs
1,254,345 container instructions, +2.1%, and that rise is real work rather than
layout: it lands positive on every walker that matches an `Expr` — eval_expr,
wants_prelude, provenance, field_reads_expr, desugar_expr, mentions_in_expr,
check_merged — which is what a changed discriminant encoding looks like. Net
welfare is +0.08 on top of this entry. Left unshipped: 2.1% of the front end
for 0.08 points wants the encoding understood first.

## 2026-08-30 — the one mutation the language has, written where nobody was looking

`x.f = v` is the only mutation kanso has, and two rules fence it: it lives in a
`build` block, and it writes only what that block built. Both rules were applied
by walks that between them saw one level of a function body.

The parser refuses a `Set` statement it parses at the top of a body.
`check_build_blocks` refuses a `Set` it finds at the top of a body, and inside a
`build` it checks the target against the names the build constructed. Neither
descended into a nested statement list, and there are three of them: an `if`
arm's `Block`, a `build`'s own body when it sits inside one, and `Guard.rest` —
everything below a fired guard.

Three programs, each of which used to compile:

```
fn tagged x            fn tagged x           fn tagged old
  n = node x             n = node x            build
  q = if true            return n if false       n = node 9
    n.id = 2             n.id = 2                q = if true
    n                    n                         old.id = 2
  else                                             n
    n                                            else
  q                                               n
                                                q
```

The first ran the write and printed `node 2`, with no `build` anywhere in the
file. The second printed `node 2` on the interpreter and killed the native
backend: `emit_fn_body` met an `unreachable!` reading "`set` parses only inside
`build`" — the invariant stated where it could not be enforced, panicking rather
than diagnosing. The third is the one that matters most. `old` is a parameter,
so the value belongs to the caller, and the born-check exists to say so; written
through an `if` arm it went unrefused and the caller's record changed under it.
The program printed `node 9 node 2`, the second being the argument the caller
still held.

`check_build_blocks` walks statements now, with the enclosing build carried
along: absent outside a build, and inside one a set of block-born names that a
nested `Block` or a guard's remainder inherits and may extend. So the legitimate
shape — a write inside an `if` arm to something the build made — still compiles,
which is the case that says the fix is a fence rather than a ban.

`compile_allocs` is 61,974 and `compile_peak_bytes` 763,868, both unchanged:
lib/json has no field write inside a nested body, so the inherited set is never
cloned there.

Instructions took three versions to get right, and the ruling that welfare
cannot fall is what forced the last two. Written as a pair of free functions
carrying `(born, type_names, diags)`, the walk cost +21,767 on the container and
+28,556 on the runner — same sign, same order, so real work rather than layout.
Folding the field-write check into the same `match` that picks a statement apart
made it WORSE, +38,298: `build_walk_body` stopped being inlined and the saving
went with it. What actually paid was the callback. This walk reaches every
expression the front end holds and descends through `for_each_child`, whose
callback is a `&mut dyn FnMut` — one indirect call per child — so a closure
capturing three references writes three words at every one of them. The
callback alone was +7,602. As a method on a struct holding the three, it
captures one, and the whole pass now reads 58,375,759 against main's
58,379,986: 4,227 cheaper than the code it replaces, because a `build`'s
statements are no longer walked twice.

`compile_instructions` still ends at 58,210,887 against 58,205,543 on the
runner, +5,344, and the two hosts disagree about the sign: the container reads
a fall of 4,227 where the runner reads a rise. Opposite signs, so what is left
is the two of them laying the binary out differently rather than work on either
one. `compile_allocs` and `compile_peak_bytes` do not move at all, welfare
holds at 84.27, and the floor's history carries the entry saying so.

This is the third time the pattern in `tests/golden/unpinned_diagnostics.txt`
has paid. That file says an excuse reasoning about which check speaks first must
name WHICH walk it is reasoning about, because a second one may not have the
rule at all. It was written about the unused-expression rule and a `build` body.
Reading it as a sweep rather than a note is what found this.

## 2026-08-30 — a regression test's account of the bug it fixed, read as a pin

The diagnostic scan keeps two corpora and narrowed only one of them. Its own
comment says why: "a substring search cannot tell an assertion from a mention",
written after four of the driver family's six pins turned out to be text in
tests/*.rs that checked no compiler message at all. The driver corpus became
`.stderr` plus `module_differential`. The wide corpus went on reading tests/*.rs
whole, comments included.

Four messages were resting on prose. Every one checked by hand:

```
no such field          getter_identity.rs:78, a doc comment listing "the three
                       ways a read can fail — no such field anywhere, ..."
not an err             make_dir.rs:7, the words "not an error." in a module
                       comment about `mkdir -p`
a beat mark and the    carry_repair.rs:37 quotes the message as what used to
arena disagree ...     happen, then asserts stdout == "gathered 4\n"
too many processes     many_handles.rs:52 quotes it, then asserts
at once                stdout == "answered 41\n"
```

The last two are the sharpest thing this file has turned up. Each is a
regression test that names the bug it fixed and then asserts the fix — an
assertion that the message never appears — and the scan read the narrative as a
pin of the message. All four could have been reworded in the runtime with
nothing going red.

One of them was already wrong. `k_set_field` in runtime.c refused a write to a
field the record does not declare with `no such field`, naming neither the type
nor the field, where the interpreter and the page both say `` `node` has no
field `nope` `` — the sentence runtime.c itself prints for a failed field READ,
at the two sites above `k_set_field` in the same file. Native says it now, and
the program that shows it is in the runtime corpus, which requires both engines
to write the same stderr.

The scan drops comment lines from tests/*.rs before they join the wide corpus.
A Rust test is still a legitimate pin for the handful of diagnostics a corpus of
programs cannot express — the precedent is `an_empty_branch_is_refused.rs` —
and what it asserts is what pins them. 310 literal diagnostics became 309, and
the three that are left have their mechanisms written: the page's err read has
one call site with `RT_CHECK_ERR` and a `br_if` two instructions above it, the
beat-mark disagreement is an arena invariant no program can ask for, and the
process cap is the fourth of its family, needing 64 live children on the runner
to reach.

`compile_instructions` moves 58,210,887 -> 58,209,632, a fall of 1,255.
runtime.c is embedded in the compiler's binary, so editing its text shifts the
data section and the row with it; `kanso check lib/json` emits nothing and never
runs a line of it. compile_allocs, compile_peak_bytes, rounds and visits do not
move. Layout, in the direction that costs nothing.

## 2026-08-30 — one indirect call per child, across the whole front end

`walk_children` is how every pass reaches a sub-expression. It took its
callback as `&mut dyn FnMut(&'a Expr) -> bool`, so every child visit in the
compiler went through a vtable. The machinery is 7.04% of
`kanso check lib/json`:

```
walk_children'2                   1,801,053   3.09%
walk_children                       983,259   1.68%
for_each_child::{{closure}}         672,707   1.15%
for_each_child::{{closure}}'2       565,642   0.97%
any_child::{{closure}}'2             45,117   0.08%
any_child::{{closure}}               39,771   0.07%
                                  4,107,549   7.04%
```

`walk_children` has exactly two callers, `for_each_child` and `any_child`, and
those are called from thirty-nine sites: twenty in check.rs, eleven in lib.rs,
three in infer.rs, two in codegen.rs, and one each in eval.rs, linear.rs and
wasm_backend.rs. Making it generic is one line, and gives it one instance per
distinct closure type across all of them:

```
                        runner                    container
compile_instructions  58,209,632 -> 56,442,099   -1,767,533   -3.04%
                      58,373,255 -> 56,536,342   -1,836,913   -3.15%
compile_allocs        61,974      unchanged
compile_peak_bytes    763,868     unchanged
compile_rounds        40          unchanged
compile_visits        16,806      unchanged
native binary         +17,944 bytes   +0.43%
docs/kanso.wasm       +32,823 bytes   +2.0%
```

Every counter that measures work is identical and only the instruction count
falls, which is what removing dispatch looks like. The two hosts agree for
once — same sign, within four per cent of each other — which is what the
pending gavel on attribution says work looks like across a host pair, and
nothing like the +382,777 against +50,058 the `Span` change read three entries
up. Welfare 84.27 -> 84.37, banked. The sibling
`walk_children_mut` keeps its `&mut dyn`: it does not appear in the profile at
all, because its four callers inline it, and the whole desugar family is under
one per cent.

The route here was the field-write fence two entries above. Its first version
cost +28,556 instructions and the cause turned out to be its closure's three
captured references, one indirect call per child. That is a property of
`walk_children` rather than of that pass, and the same tax was being paid
forty times over.

### The eight bytes below `Expr`, determined

`Expr` sits at 56 because `Guard`'s payload is exactly 48 with no spare byte
for the tag. `Guard.rest` as a `Box<[Stmt]>` takes it to 48 and `Stmt` to 112,
and `compile_peak_bytes` falls 763,868 -> 721,652, another 5.5%, for +2.19%
instructions. Two probes say where that rise lives, both measured with
`walk_children` already generic so the changes could not confound each other:

```
Guard.rest          Expr   instructions   peak
Vec<Stmt>            56     56,536,342    763,868
Box<[Stmt]> + pad    56     56,528,428    763,748
Box<[Stmt]>          48     57,773,600    721,652
```

Padding `Guard` so `Expr` stays 56 while `rest` is still a boxed slice reads
7,914 BELOW the `Vec` version, on 120 bytes less peak. So the indirection is
free, and the whole +1,237,258 is the 48-byte layout. It is not dispatch
either: the rise was +1,254,345 before this change and +1,237,258 after.

That leaves the trade as measured, with no cheaper route through this door:
5.5% of the front end's peak for 2.19% of its instructions, +0.08 welfare.
Left unshipped, and now characterised rather than open.

## 2026-08-30 — the largest line in the allocation map held two bytes

The dhat map from the entry before last put one line of `infer.rs` at 8,309
blocks, 13.4% of every allocation the front end makes on `kanso check
lib/json`, and more than twice the next line down. It is `eval_call`
collecting a call's argument sets:

```rust
let mut arg_sets: Vec<Set> = args.iter().map(|a| eval_expr(ctx, a, env)).collect();
```

`Set` is a `u16`. Arity in real source is one, two or three, so the great
majority of those 8,309 heap blocks held two, four or six bytes. Eight of them
are an array on the stack now, and arities above eight still spill to a `Vec`,
so nothing a program may write changes.

```
compile_allocs        61,974 -> 57,430          -4,544    -7.3%
compile_instructions  56,442,099 -> 55,414,950  -1,027,149  -1.82%  (runner)
compile_peak_bytes    763,868   unchanged
compile_rounds        40        unchanged
compile_visits        16,806    unchanged
```

Welfare 84.37 -> 84.66, banked. Nothing rises.

`compile_peak_bytes` does not move, and that is the argument for having a map
at all. A call's argument sets die when the call is inferred, so these blocks
were never what the arena held at its high water mark: the peak vein could not
see them and `compile_allocs` could only say there were sixty-two thousand of
something. dhat is what named the line.

### Two hosts, three times apart, and a mechanism for it

They agree exactly on the allocations — both read 57,430 — and disagree by a
factor of three on what the allocations cost. The container reads -340,075
where the runner reads -1,027,149. Same sign, neither near zero, so by the
pending gavel's reading this is part work; callgrind says which part.

```
                        before        after       delta
malloc               2,712,344    2,521,016    -191,328
_int_malloc          3,356,980    3,367,739     +10,759
free                 1,735,552    1,608,320    -127,232
_int_free            3,754,476    3,509,501    -244,975
arena.c:free           185,952      172,320     -13,632
__rust_alloc         1,220,817    1,116,305    -104,512
__rust_dealloc          97,638       88,550      -9,088
                                               -680,008

malloc_consolidate     788,221    1,058,396    +270,175
unlink_chunk           362,013      481,793    +119,780
                                               +389,955
```

The calls that went away saved 680,008 instructions and the free lists took
back 389,955 of it. Removing 4,544 small short-lived blocks changes the shape
of the heap, and this glibc revision pays for the change at consolidation.
What is left, -290,053, plus about fifty thousand in `eval_call` itself, is
the container's -340,075. The runner's 2.39-0ubuntu8.8 does not pay it and
banks nearly the whole gross saving.

Every earlier row where the hosts disagreed ended in "layout" — true, and
unsatisfying, because layout is a name for not having looked. This one has a
mechanism, and the mechanism says something the gavel's rule does not: when
the hosts disagree, the SMALLER reading is not automatically the work. Here
the smaller reading is the work minus a penalty one host pays and the other
does not, and both numbers are honest measurements of the same change on
different allocators. The rule wants a fourth row for that.

## 2026-08-30 — a dispatch group is a range, not a cloned vector

Re-running the map after the entry above put `eval_call`'s dispatch lookup at
2,693 blocks, 4.7% of what the front end allocates and the largest line left
with kanso's name on it.

```rust
if let Some(decls) = ctx.groups.get(&(name.as_str(), args.len())) {
    let decls = decls.clone();
```

The clone answers to the borrow checker and to nothing else. `decls` borrows
`ctx`, and three lines down the loop calls `widen_param(ctx, ..)`, so the group
is copied to a fresh `Vec<usize>` on every call the pass infers — a group
holding, typically, one index. `groups` is built once in `infer` and never
written again, so a group can be a half-open range into one flat
`group_members: Vec<usize>`. A range is two words and it copies.

```
compile_allocs        57,430 -> 54,747          -2,683   -4.7%
compile_peak_bytes    763,868 -> 742,572        -21,296  -2.8%
compile_instructions  55,414,950 -> 55,319,098  -95,852  -0.17%  (runner)
compile_rounds        40        unchanged
compile_visits        16,806    unchanged
```

Welfare 84.66 -> 84.89, banked. Peak moves as well as traffic this time: the
table used to hold one heap vector per (name, arity) for the length of the
compile, and those are one vector now.

### The consolidation step, and where each host takes it

The entry above blamed a host disagreement on `malloc_consolidate` and left it
there. This change disagrees too, by three and a half times and leaning the
other way — the container reads -342,880 where the runner reads -95,852 — and
the two rows read together say what one could not.

```
malloc_consolidate      before #1139   after #1139   after #1140
container                    788,221     1,058,396     1,055,975
runner                             —       789,570     1,054,967
```

Consolidation on this workload steps up about 265,000 instructions, once, when
the free lists change shape. The container took that step on the previous
change and holds flat through this one; the runner held flat there and takes it
here. Each host pays the same penalty for the same reason and only the change
it lands on differs, which is why the per-change gaps are large and lean
opposite ways. Over the two changes together the readings are -682,955 and
-1,123,001: still apart, but by 1.6x where the individual rows are 3x and 3.6x.

That is a fourth shape for the attribution question, and it is not in the
ledger entry's table. Two hosts can disagree by a factor of three on a change
where both are measuring real work, correctly, and neither number is the
answer on its own. What made it legible was measuring the allocator lines
rather than the total — which is available on any row, and was not done on any
of the eight rows before these two.

## 2026-08-30 — four lookup keys the program already holds, and a walk that was not a mirror

`inline.rs` built a `String` in order to look one up, in three places, and kept
a private copy of the mutable child walk that returns a fresh vector per node.

`aliases` returned `HashMap<(String, usize), String>`, so the fixpoint owned a
name and a target for every alias it found on every round, and `direct_aliases`
cloned the callee at every candidate it tested. Both keys and both values
borrow from the program now. `check.rs` had already diagnosed this at the
consumer and worked around it — it built a borrowed view of the owned map, with
a comment saying the lookup "needed a String built from the callee at every call
expression" — so the view and the workaround are gone with it. `inline::rewrite`
cloned the callee at every `App` node; its map has to be owned, because the walk
takes `program` mutably, but nesting it by name and then arity means both
lookups borrow.

```
compile_allocs        54,747 -> 50,528          -4,219   -7.7%
compile_instructions  55,319,098 -> 54,488,638  -830,460 -1.50%  (runner)
compile_peak_bytes    742,572   unchanged
compile_rounds        40        unchanged
compile_visits        16,806    unchanged
```

Welfare 84.89 -> 85.19, banked. The hosts agree on this row — -852,860 in the
container against -830,460 on the runner, 2.7% apart and the same sign — which
is what the two entries above predict. Consolidation steps up once when the
free lists change shape; the container took its step two changes ago and the
runner one change ago, and with both spent neither pays here.

Three rounds off one allocation map now: 61,974 blocks to 50,528, 18.5%, with
instructions down 3.5% and peak down 2.8% beside it.

### The walk that said it was a mirror

The fourth piece of that change was wrong, and CI caught it. `inline::children_mut`
looked like a duplicate of `lib::walk_children_mut`, which carries the comment
"Mirrors `for_each_child`". Swapping one for the other turned the emitted,
machine-code and work veins red: `walk_children_mut` has no arm for
`Expr::Lambda`, `Expr::Block`, `Expr::Build` or `Expr::Guard`, where
`for_each_child` handles all four. A wrapper called inside any of them stopped
being inlined.

So `inline.rs` keeps its own walk, now as `for_each_child_mut`: the coverage
`children_mut` had, handing children to a callback instead of returning a
vector. That was the whole allocation saving — `compile_allocs` reads 50,528
either way — and the swap bought nothing it did not also break.

Two things worth keeping from it. The comment was load-bearing and false, which
is the shape #1137 went after; it says what the function does not do now. And
the veins that caught it were the runtime ones. The compile veins were happily
reporting a win on a compiler that had quietly stopped inlining, because
compiling less work is cheaper. A cost golden cannot tell a saving from an
omission; only a golden over the OUTPUT can.

What the four callers of `walk_children_mut` do with the missing arms is a
separate question, and two of them are answered. `desugar_expr` (field read to
getter call) and `deny_expr` (`!=` to `if (==) false true`) are normalisations
rather than requirements: `Expr::Field` is handled directly at codegen.rs:3090,
eval.rs:1408 and wasm_backend.rs:764, and `"!="` at codegen.rs:3571,
eval.rs:3656 and wasm_backend.rs:897. A field read and a `!=` inside a lambda
body, an if-block arm, a build body and below a guard — four shapes on two
engines — answer correctly and identically. `replace_shape` is the hoister,
where an unreached site is a hoist not taken. `door_expr` is the one still
open: it rewrites an upcast's type from a door spelling to the owner's, and an
upcast inside any of the four would keep the door spelling.

## 2026-08-30 — the walk with four holes in it, and the two bugs behind them

`lib::walk_children_mut` said it mirrored `for_each_child` and had no arm for
`Expr::Lambda`, `Expr::Block`, `Expr::Build` or `Expr::Guard`. The entry above
found that by accident, swapping `inline`'s own walk for it and watching three
runtime veins go red. Its own four callers were the open question, and two of
them were wrong.

### A door spelling stopped at the edge of a nested body

`door_expr` rewrites `Expr::Upcast`'s type from a door — the qualified second
spelling a re-export opens — to the owner's canonical name. Recursing through
the holed walk, it reached every statement of a function body and nothing
nested inside one, so an upcast written in a lambda, a block, a build or a
guard kept the door spelling. `kanso check` passed the program and the
widening failed at run time against a name no declaration answers.

Five positions, same value, same upcast, on main:

```
statement level    prints 1
inside a lambda    error[runtime]: `:mid/shape` widens; this value is not a mid/shape
inside an if-block same
inside a build     same
below a guard      same
```

Exactly the four missing arms and nothing else — `(v):deep/shape`, the
canonical spelling, runs in all five. Native and the interpreter fail alike,
so this was never a divergence, which is why nine differential sweeps never
saw it. `tests/golden/reexports/upcast` is the five positions in one program.

### The hoister emitted bindings it could not use

`collect_hoistable` finds a repeated interpolation with `for_each_child` — the
full walk. `replace_shape` substitutes it with `walk_children_mut` — the holed
one. So a repeat found inside a block, a build or a guard got its `onceN`
binding emitted and not one of its uses rewritten. Dead code, in every program
that repeats an interpolation inside a nested body.

That is what the emitted vein reports: scanbench falls 3,745 -> 3,743 calls,
2,216 -> 2,214 branches, 20,023 -> 20,019 lines. Two dead bindings in one
benchmark. Both functions refuse a lambda outright, so the lambda arm changes
nothing for the hoister.

### The two that were fine, and why

`desugar_expr` (a field read to a getter call) and `deny_expr` (`!=` to
`if (==) false true`) are normalisations rather than requirements: every engine
handles the un-normalised form directly — `Expr::Field` at codegen.rs:3090,
eval.rs:1408 and wasm_backend.rs:764, and `"!="` at codegen.rs:3571,
eval.rs:3656 and wasm_backend.rs:897. Both forms in all four positions on both
engines answer correctly and identically.

`compile_allocs` and `compile_peak_bytes` do not move, and neither do rounds or
visits. `compile_instructions` rises 54,488,638 -> 54,507,708 on the runner,
19,070 instructions or 0.035%, and that is the arms themselves: four passes
descend into bodies they used to stop at, and two of them had to. It is a
LAYOUT row by this project's own reading — the container reads 55,000,527 ->
54,850,326, a FALL of 150,201, against the runner's small rise. Opposite signs,
both under three tenths of a per cent, on a change that adds four match arms to
one function. The fifth host-pair for the attribution ledger, and the cleanest
layout case in it.

The general lesson is about the comment. "Mirrors `for_each_child`" was a claim
nothing tested, and two passes were built on it. #1137 went after four
diagnostics resting on prose; this is the same failure in a walk, and the same
answer applies — the fixture is the pin.

## 2026-08-30 — a subtype of a primitive is a heap value, and one list did not say so

Native printed a different denormal double on every run where the interpreter
printed the value. A use-after-free that produced silent wrong answers, live on
main since subtypes of primitives existed, and found by accident while building
the fixtures for the entry above.

```
type shape int              native 6.90351265195293e-310   interp 3
type shape string           native 6.9464150267249e-310    interp hi
type shape float64          native 3.5                     interp 3.5
```

### The cause

`runtime.c`'s `k_is_heap` lists every tag whose payload is a pointer:

```c
case K_STR: case K_ERR: case K_REC: case K_DESC:
case K_LIST: case K_MAP: case K_CLOSURE: case K_BYTES:
    return 1;
```

`K_SUB` was not on it, and a `K_SUB` payload is a `KSub*`. `k_cohort_pop` reads
that predicate to decide whether a beat's result has to be carried out of the
arena before the rewind:

```c
if (!k_is_heap(r.tag) && r.tag != K_THUNK) {
    k_beat_depth--;
    k_beat_rewind(m);     /* the arena goes back to the mark */
    return r;             /* r points into what was just freed */
}
```

So a returned subtype was taken for a scalar, the arena went back under it and
the caller kept a dangling pointer. The fix is one case label.

`K_THUNK` is spelled out at that call site rather than in the list, which is
what says the list was known to be the gate — and that it had already been
found short once.

### Why the conditions looked so strange

Three ingredients, each checked against the unfixed compiler. The value has to
be MADE in one call and STORED by another, both written in the entry, so
`k_cohort_pop` sees it cross — `lib/both 3`, the same chain inside the library,
is correct. It has to go into a container, because a value rendered on the spot
is read before the arena is reused. And the parent has to be `int` or `string`:
`float64` survived every arrangement, structurally, because a float payload is
the double itself and has nothing to dangle.

None of the module boundary, the re-export, the build block or the seed value
mattered, and all four were in the first reproduction.

Valgrind reports zero errors on the failing binary, which is worth saying
plainly: the arena block is still mapped and still initialised, so the read is
well-defined and merely stale. A memory checker was never going to find this.
What found it was the interpreter disagreeing.

### What it costs, and what it does not

Every runtime counter gate is green — decode, encode, escape, one-shot, basket,
wide, pending-cell and scan all unchanged. None of the benchmarks returns a
subtype across a beat, which is also why nothing caught this.

The page engine cannot have it: `wasm_rt.rs` has no beat and no cohort at all,
so the arena rewind is native's alone. That is a structural answer rather than
a test, and better than one.

The fixture is `tests/golden/entryfile/a_subtype_stored_across_the_entry`:
three primitive parents by two containers, pinned as one output on both
engines. Which line comes back wrong depends on what the arena held, so the
whole output is the pin rather than any line of it.

### The rest of the list, swept

A predicate that enumerates tags is worth checking the moment one of them is
found short, so the other ten were walked against the enum.

```
K_INT K_FLOAT K_TRUE K_FALSE K_NONE   immediates; the payload is the value
K_FNREF                               a pointer, and correctly absent: it is
                                      always `ptr @<global>` — codegen emits
                                      `k_fnref(ptr @rsym)` at all three call
                                      sites and the helper's own comment calls
                                      it "the static a `k_fnref` value points
                                      at". A static cannot be rewound.
K_THUNK                               a pointer, spelled out at the call site
K_STR K_ERR K_REC K_DESC K_LIST
K_MAP K_CLOSURE K_BYTES K_SUB         on the list
```

So the list is complete now, and `k_is_heap` is the only predicate of its shape
in the file — one other line groups heap tags, and it renders `K_CLOSURE` and
`K_FNREF` alike as `<fn>`, which is a display question and not a lifetime one.
The deep copier already had its `K_SUB` arm; only the predicate that decides
whether to call it was short.

### What the repair cost, and what a second look returned

The one-case fix is not free, and the reason is the opposite of what a reader
would guess. `k_is_heap` is inlined into `k_slots_survive` and through it into
`k_copy_size`, which is 36% of deepbench — so the predicate's SHAPE decides how
that walk compiles. Five shapes were measured in the container:

    with the bug                            806,982,208
    the switch, plus one `case K_SUB:`      856,510,441   +6.14%
    a mask carrying a bounds branch         878,869,219   +8.90%
    `k_slots_survive` given its own switch  856,510,441   +6.14%
    the mask that ships                     850,361,281   +5.38%

deepbench never makes a subtype — `k_sub` appears nowhere in its profile, and
`k_survives_x` and `k_ptrmap_at` are byte-identical across the change. Same
walk, same calls, same counts, more instructions. A tenth `case` was worth
49,528,233 instructions on a benchmark that cannot reach the tag.

That 5.38% cost welfare 0.03, and the ruling in `scripts/welfare/welfare.kso`
is that welfare cannot fall. The entry went to design/pending-gavels.md as a
blocking question. It has been WITHDRAWN, unruled, because looking one level
further down dissolved it.

`k_copy_size` returns zero for an immediate and for nothing else without
looking at it, so a caller walking a container can skip the call entirely.
deepbench folds over lists of ints; the call it made per element existed only
to return zero. Six sites — three in `k_copy_size`, three in `k_repair_size` —
now test `k_worth_sizing` first:

    with the bug                     806,982,208
    the fix alone                    850,361,281   +5.38%
    the fix and the skip             760,471,453   -5.77% against the bug

Against origin/main, on the runner:

    work_deepbench    806,985,948 -> 760,475,193   -46,510,755   -5.76%
    work_widebench     85,273,589 ->  83,967,604    -1,305,985   -1.53%
    work_encodebench 9,866,843,915 -> 9,866,614,705   -229,210
    work_basket        57,436,178 ->  57,392,199       -43,979
    work_pendbench    987,907,671 -> 988,282,947      +375,276   +0.038%
    work_escapebench  258,574,097 -> 258,583,100        +9,003
    work_jsonbench  2,910,241,430 -> 2,910,241,528          +98
    work_oneshot       47,277,061 ->  47,277,156          +95

`work_pendbench` is the only row that pays for the skip rather than the mask,
and it pays for exactly what it is: the lazy benchmark's slots hold thunks,
`k_worth_sizing` answers yes for a thunk, so every element takes the new test
AND still makes the call. 392,848 instructions of a test that never saves one,
against 46.5 million saved on the benchmark whose slots are ints. The other
three risers — `work_escapebench`, `work_jsonbench`, `work_oneshot` — are
identical between the fix alone and the fix with the skip, so their movement is
the predicate's shape in programs whose copy walk is cold, not the skip.

`compile_instructions` falls 3,097 (54,507,708 -> 54,504,611) and the machine
code falls 1,328 bytes net: the mask removes about four hundred bytes from
every benchmark and `k_worth_sizing` adds back 240 to each.

Welfare is 85.22 against a floor of 85.19, and the floor is moved in this same
PR. The blocking entry is gone from the ledger with no ruling recorded, because
none was needed in the end — which is the outcome the escalation was supposed
to have, and the reason to escalate the moment a question is found rather than
after exhausting it.

## 2026-08-30 — the text-block opener counted characters and sliced bytes

`kanso check` panicked on this file, and printed a wrong diagnostic on the two
files either side of it:

```
pub joined = pick "e"  """     compiles, prints
pub joined = pick "é"  """     refused: "nothing follows `\"\"\"`"
pub joined = pick "…"  """     refused: the same
pub joined = pick "🎯" """     panic: byte index 22 is not a char boundary
```

One character apart, and the program is otherwise identical.

### The cause

`block_opener` scans the line a text block opens on and returns where it found
the `"""`. It collected a `Vec<char>` and returned a CHARACTER index. All three
of its callers use that number as a BYTE index:

```rust
if content[at..].chars().count() != 3 { ... Span::at(number, indent + at + 4) }
let (body, consumed) = gather_block(...);
match lex_line_with_block(&content[..at], number, indent + 1, indent + 1 + at, &body)
```

The two agree exactly while the line is ASCII, which every line in the corpus,
the book and the standard library happens to be. One two-byte character before
the fence puts the byte index one ahead of the character index, so
`content[at..]` starts a byte early, reads `" \"\"\""` rather than `"\"\"\""`,
counts four characters where three are wanted, and the block is refused for
having something after it. Three bytes drift by two. Four bytes land inside the
character and `str`'s slice panics.

The predicate has been this way since text blocks existed. What kept it quiet
is that the only way to reach it is to write a non-ASCII character on the same
line as a `"""`, and nothing in the tree does.

### The fix, and what it returns

`block_opener` walks `content.as_bytes()` and returns a byte offset. The three
bytes it tests for — `\`, `"`, `#` — are ASCII, and every byte inside a
multi-byte character is at least 0x80, so those bytes match no arm of the scan
and are walked past one at a time. `i += 2` past an escape is right for the
same reason: it skips the backslash and the escaped character's first byte, and
whatever remains of that character matches nothing either.

The column the diagnostic points at is still counted in characters, so the one
caller that needs it takes `content[..at].chars().count()` — which is what the
old `at` was. Replacing that back with `at` moves the caret one column right on
the `é` fixture, which is the pin on that half of the change.

The `Vec<char>` goes with it, and it was not small: **compile_allocs 50,528 ->
48,356**, a fall of 2,172 and 4.3% of everything the front end allocates.
`lib/json` contains no text block at all — the vector was being built for every
line of every file compiled, in order to answer no. `compile_peak_bytes` does
not move (742,572), which is right for a vector that never lived past the call.

The map was re-run against the fixed compiler, and it says the change did one
thing:

```
total blocks   50,540 -> 48,368   (-2,172)
block_opener    2,172 ->      0
every other site           identical, to the block
```

Not a fall of about the right size — the same number dhat had attributed to
that one line, with no other site moving by one allocation. (48,368 against
`compile_allocs` 48,356 is the twelve allocations that happen before the
counting allocator installs, which the archive already accounts for.) The other
lexer rows only changed line numbers, because the comment above `block_opener`
grew.

### The fixtures

`tests/golden/micro/a_text_block_opens_after_a_wide_character` runs one program
holding all three widths and pins its output on both engines. It panics on the
parent commit — "byte index 16 is not a char boundary" at lexer.rs:122 — rather
than printing a wrong answer, which is the loudest a fixture gets.

`tests/golden/errors/a_text_block_fence_after_a_wide_character` pins the column
of the "nothing follows" diagnostic, which is the half of the fix that has
nothing to do with slicing.

### The family, swept

A predicate confusing two units is worth checking the file for others, and
`src/lexer.rs` had two more of exactly this shape. `raw.find('\t')` and
`trimmed.find('\t')` answer in bytes, and both feed `Span::at(number, col + 1)`
— a column. Three two-byte characters before a tab put the caret three columns
right of it:

```
x = "ééé"	y        said column 13, the tab is the 10th character
  ééé	z            said column 9,  the tab is the 6th
```

Both take `[..at].chars().count() + 1` now. There is no third: the only other
byte index in the file is the leading-whitespace `indent`, and the check above
it refuses tabs outright, so what it counts is spaces and the two units agree
by construction.

The sweep ran past the file too. Every `Span::at` in the tree outside
`src/lexer.rs` — two in check.rs, six in eval.rs, three in lib.rs, nine in
parser.rs, one in wasm_rt.rs — takes a literal or a token's own span, so no
other pass computes a column from source text at all. That is the lexer's job
and only the lexer's, which is why the confusion could only live here.

These are wrong carets rather than wrong programs, which is why they had
survived a corpus that pins every diagnostic in the tree — the pins are all
ASCII, so the two units agreed on every one of them.
`tests/golden/errors/a_tab_after_a_wide_character` carries both lines, and
putting the byte offset back at either site moves that fixture's caret.

### Where it came from

The dhat allocation map, re-run after #1139–#1141 took the front end from
61,974 blocks to 50,528. The lexer is the largest allocator in the new map —
13,854 blocks over six lines, 27% of the total — and `block_opener`'s vector
was 2,172 of them, the fourth line down. Reading the function to see whether
the vector could go is what found the index units. The bug was not what the map
was looking for; a map of where the work is answers questions nobody asked it.

## 2026-08-30 — a token and the column it ends at are one vector

`Line` carried `tokens: Vec<(Tok, Span)>` beside `end_cols: Vec<usize>`. The
two were always the same length, and twelve places in `src/parser.rs` sliced
them:

```rust
P::new(&header.tokens[off + 2..], &header.end_cols[off + 2..], header.number)
P::new(&line.tokens[1..*at],      &line.end_cols[1..*at],      line.number)
```

All twelve slice both the same way — that was checked before the change, and
there is no bug here to fix. What there is, is a pair that has to be kept in
step by hand, in the file where a character index and a byte index had just
been found disagreeing. `Vec<(Tok, Span, u32)>` cannot fall out of step.

### What it costs and returns

```
compile_allocs        48,356 -> 46,998    -1,358   -2.8%
compile_peak_bytes             742,572    unmoved
docs/kanso.wasm    1,661,716 -> 1,657,340   -4,376 bytes
```

The fall is larger than the 1,117 blocks dhat attributed to `end_cols`'
growth, and the extra is where the map could not see it: `StrPart::Interp`
carried the identical pair — `Interp(Vec<(Tok, Span)>, Vec<usize>)` — so every
interpolation in the program paid it again. That variant is one field now, and
`template_part` hands `P::new` one slice where it used to hand two.

Peak does not move, which is the right answer rather than a disappointing one.
A `(Tok, Span, u32)` pads to the same width the pair occupied across two
allocations, so what goes is the second header and the second doubling
sequence, not the bytes the tokens themselves need.

### What did not change

Every output gate is green: the emitted golden, the machine-code golden and
all eight work rows, plus decode, encode, escape, one-shot, basket, wide,
pending-cell and scan counters. The compiler writes the same program and every
benchmark does the same work. This is the front end's own bookkeeping and
nothing a user can observe, which is why it ships with no fixture of its own —
the corpus that already pins every diagnostic in the tree is the test, and
`check_needless_continuation` and `validate_spacing` were both rewritten
against it.

The diff is 162 lines added against 181 removed. A merge that removes more
than it adds is the shape to expect when two things that were always equal
stop being written down twice.

### The neighbouring vector is DECLINED, and the reason is the same family

`lex_line` builds a `Vec<char>` per source line — 1,997 allocation blocks, the
next item down the map after this one. It stays, and the reason is one line:

```rust
fn span(&self) -> Span { Span::at(self.line, self.col_offset + self.pos) }
```

`pos` IS the column. The vector is not indexing convenience; it is what makes
every token's column a character count, which is what a caret under a source
line has to be. Three ways to remove it were considered and all three are
worse:

- **Byte offsets alone.** Every token's column goes wrong by the number of
  multi-byte characters before it on the line. That is the bug this file was
  just fixed for twice, reintroduced at every token rather than at one fence.
- **Compute the column when a span is made**, `content[..byte].chars().count()`.
  O(byte) per token, so quadratic in line length, to save one linear collect.
- **Carry a byte position beside the character one.** Fourteen sites advance
  `pos`, two of them by two characters at a time, and the byte width of what
  they skip is not known at the site. That is the shape this entry removes,
  at fourteen places instead of twelve, with a harder invariant.

So the vector is the cheapest way to have the thing it buys, and this is a
declined idea rather than an open one.

## 2026-08-30 — the tail-call rewriter's group map cloned a name per declaration

`trmc::rewrite` opens by grouping declarations:

```rust
let mut groups: HashMap<(String, usize), Vec<usize>> = HashMap::default();
for (i, decl) in program.fns.iter().enumerate() {
    groups.entry((decl.name.clone(), decl.params.len())).or_default().push(i);
}
```

A `String` per function declaration, built to look one up, out of names the
program is holding open in front of it. That is #1141's finding one file over,
and the map put 1,634 blocks on this site.

The keys borrow now. What made the same fix hard in #1140 — `rewrite` takes
`&mut Program` — turns out not to apply here: nothing in the body writes to
`program`. The rewritten arms accumulate in a local `new_fns` and go on at the
end, `program.fns.extend(new_fns)`, after both loops have finished with the
borrow. One `FnDecl` the rewriter builds does need an owned name, and takes
`name.to_string()` — once per rewritten group rather than once per declaration.

```
compile_allocs   46,998 -> 46,008   -990   -2.1%
compile_peak_bytes        742,572   unmoved
```

The 990 against dhat's 1,634 is the split, and it says where the rest is: the
`Vec<usize>` each group collects its members into, and the `Vec<&FnDecl>` the
body collects them back out into. Those are #1140's treatment — a half-open
range into one flat vector — and they are NOT taken here. Restructuring the
tail-call rewriter for the remaining 644 is a worse trade than a one-line
borrow for 990, and the emitted golden is what would catch it going wrong: the
group iteration order decides the order arms are rewritten in.

`scripts/trmc_differential` passes — 23 shapes, three of which the license
refuses, at four depths each, rewritten and not, agreeing on both engines — and
the emitted golden is unchanged, so the compiler writes the same program.

### The sweep this came out of, and the nine sites it refused

The shape is `(name.clone(), arity)` as a map key. A grep finds eleven more:
codegen.rs:399 and :425, demand.rs:211, dispatch.rs:46, escape.rs:51 and :89,
linear.rs:86, :88, :979 and :1131, provenance.rs:315.

dhat attributes ZERO blocks to codegen.rs, dispatch.rs, escape.rs and
linear.rs on `kanso check lib/json` — a check never runs those passes. Nine of
the eleven would have been diffs that cost a reader time and returned nothing.
The map usually earns its keep by finding work; here it earned it by refusing
some.

What is left on the measured path is demand.rs at 1,614 blocks and
provenance.rs at 633. demand.rs has the shape on both sides, and the lookup is
the larger one: `discard.get(&(callee.clone(), args.len()))` fires at every
`App` node the walk visits, where the build side fires once per declaration.
Its borrow is easier than this one's — `discard_positions` already takes
`&Program`.

## 2026-08-30 — where the welfare headroom actually is

The score is used two ways: as a gate, and afterwards to say which term paid.
It answers a third question nobody had put to it — where work should go — and
the answer is not what a day of this session's choices assumed.

Each dimension's earned score against its ceiling, from the weights and
satiations in `scripts/welfare/welfare.kso` and the goldens at `fb5bf7bc`:

```
dimension        ratio   satisf.   earns   ON TABLE
run speed        28.72    0.935    28.05     1.95
run memory      112.44    0.983    29.48     0.52
compile speed     1.20    0.706    19.77     8.23
compile memory    1.10    0.688     8.26     3.74
                                   85.56
```

The total reproduces the live score to the digit, which is what says the
reading is of the function rather than of an approximation to it.

**11.97 of the 14.44 points still available sit in the two compile
dimensions.** The benchmarks are 28.7 and 112.4 times their baselines and
satiate at 2.0, so between them they hold 2.47 points. Compile speed sits at
ratio 1.20 — barely off its baseline — and holds 8.23 alone, early satiation
and all, because satiation only bites once a dimension has moved.

This explains a scoreboard that otherwise reads backwards. Today's four
changes:

```
#1143  a use-after-free repair, deepbench -5.76%     +0.03
#1145  the text-block fence, compile_allocs -4.3%    +0.19
#1146  one vector, compile_allocs -2.8%              +0.15
#1147  trmc's keys borrow, compile_allocs -2.1%      +0.08
```

The runtime change is the largest single measurement of the four and worth the
least, because a benchmark thirty times better than its baseline has almost
nothing left to give the index. Three modest front-end changes outscore it
six to one.

What this licenses is choosing between two pieces of work that are both sound.
It does not say a decoder regression stops mattering — the per-counter goldens
are the tripwire for that and are untouched by any of this — and it cannot say
anything about wall time, which the function leaves out and therefore weights
at zero. The function is provisional and says so. But asked which end of the
compiler to spend the next hour on, it has a clear answer, and the answer is
the front end.

## 2026-08-30 — the demand pass's lookup keys

`discard_positions` keyed its map on `(String, usize)`, and `collect_uses` built
the same pair every time it read one:

```rust
Expr::Ident(callee, _) => discard.get(&(callee.clone(), args.len())),
```

The build side allocates once per function declaration. The lookup side fires
at every `App` node the demand walk visits and throws the `String` away as soon
as the map has answered. Both sides borrow now, which the pass can do because
`discard_positions` takes `&Program` and the map dies inside `analyze`.

```
compile_allocs        46,008 -> 44,920    -1,088   -2.4%
compile_peak_bytes               742,572  unmoved
docs/kanso.wasm    1,656,573 -> 1,654,278  -2,295 bytes
```

### compile_instructions rose, and the rise is glibc's

```
compile_instructions  52,172,225 -> 52,201,308   +29,083   +0.056%
```

A rise on a change that removes 1,088 allocations, which reads backwards until
the profile is diffed. Everything kanso does got cheaper and so did every
allocator entry point:

```
_int_malloc     3,196,519 -> 3,120,043    -76,476
_int_free       2,865,445 -> 2,796,140    -69,305
malloc          2,017,580 -> 1,969,739    -47,841
String::clone     311,018 ->   275,114    -35,904
free            1,288,504 -> 1,258,040    -30,464
__rust_alloc      917,838 ->   892,814    -25,024
```

That is about 326,000 instructions of work removed. Two rows rose past it, and
both are glibc's free-list maintenance:

```
malloc_consolidate  721,213 -> 967,024   +245,811
unlink_chunk        340,268 -> 448,044   +107,776
```

Removing 1,088 short-lived allocations of one size changed which chunks sat in
the fastbins when glibc came to consolidate them, and it consolidated more. The
compiler asks the allocator for less and the allocator charges more for the
asking. `kanso::demand::analyze` itself moves 174 instructions on 112,706,
which is this measurement's noise floor.

It is banked as a rise with a cause rather than waved through as layout,
because it reproduces on both toolchains and with the same sign: +20,355 here
under rustc 1.94.1, +29,083 on the runner under 1.98.0. A layout accident would
not do that.

Welfare goes up, 85.64 -> 85.72, ratcheted here. Compile speed reads the mean of
this row's ratio and `compile_allocs`'s, and 2.4% off the allocations is worth
several times 0.056% on the instructions.

### The sibling that measures zero

`use_targets` has the identical shape one function down — it collects
`Vec<(String, usize, usize)>` and pushes `callee.clone()` — and borrowing it
moves `compile_allocs` by nothing at all. Built, measured at 44,920 both ways,
reverted. It runs only for a binding that has already passed the lazy vote, and
`lib/json` produces none, so the clone is on a path the vein cannot see. A
program in the lazy fragment would reach it; the mem tier is where that would
show, and those fixtures are a dozen statements each.

### What #1147's entry got wrong about provenance

That entry named `provenance.rs` at 633 blocks as the other site left on the
measured path. A dhat run on the current binary attributes **zero** blocks to
`provenance.rs` at any depth, and reading the file says why: `Provenance` keys
its parameter map on `Group<'a>`, borrowed already, and the `decl.name.clone()`
at line 315 is inside the license diagnostic — reached only by a declaration
with an arm for an err its own package raised. `lib/json` has none. The 633
came from the pre-#1139 map and was carried forward without being re-read.

### The front end's allocations after the day

44,923 blocks by dhat against 44,920 by the counting allocator. The ten largest
lines:

```
3,592  lexer.rs:7      <Tok as Clone>::clone — the String inside Tok::Ident,
                       copied when a caller clones the token
3,157  lexer.rs:584    let tok = s.lex_word()? — the same String, built; the
                       allocation is inside lex_word and lands on the inlined
                       call site
1,997  lexer.rs:535    Scanner's Vec<char> per line
1,689  infer.rs:250
1,394  lib.rs:610
1,375  lib.rs:2897
1,117  lexer.rs:585    tokens.push — the per-line token vector
1,100  parser.rs:2112
1,072  infer.rs:574
1,067  parser.rs:2119
```

The lexer holds four of the ten and 9,863 blocks between them, which is 22% of
the front end. Read the frames rather than the line numbers: 6,749 of the 9,863
are one `String`, the name in `Tok::Ident`, built once at 584 and copied 3,592
times at 7. Building it is what interning would remove, and interning is
declined — #1033, 365 conversion sites for one AST field of twenty-nine.
Copying it is a separate question with a separate answer, because a clone
happens at a caller that could have matched on `&Tok`.

That leaves 3,114 blocks in two vectors the lexer builds per line and neither
of which a reader ever sees: `Scanner`'s `Vec<char>` at 1,997 and the token
vector at 1,117. The `Vec<char>` was declined in #1145 on the grounds that
`pos` is the column, which remains true and is a reason to keep indexing
characters rather than a reason to allocate a fresh vector for each line.

## 2026-08-30 — four vectors the front end rebuilt on every iteration

Each of these builds a heap vector inside a loop, uses it for one iteration and
drops it. None of them is visible in the language, the diagnostics or the
emitted code, and together they were 3,985 of the front end's 44,920 allocation
blocks.

```
Scanner's Vec<char> comes from a pool          44,920 -> 42,932   -1,988
lex_line reserves `tokens` at eight            42,932 -> 42,498     -434
the parser matches on &Tok instead of cloning  42,498 -> 42,302     -196
callee_first hoists `names` out of its loop    42,302 -> 40,935   -1,367
                                                                 -3,985   -8.9%
compile_peak_bytes                            742,572 -> 743,564    +992   +0.13%
docs/kanso.wasm                             1,654,278 -> 1,655,440  +1,162 bytes
```

Measured one at a time, in that order, so each number is that piece's.

### What it cost to run

```
compile_instructions  52,201,308 -> 51,126,817   -1,074,491   -2.06%
```

The allocator rows carry about half of it — `_int_free` 2,796,241 ->
2,580,177, `malloc` 1,969,739 -> 1,790,546, `free` 1,258,040 -> 1,146,488,
`__rust_alloc` 892,814 -> 852,633, some 547,000 between them. `_int_malloc` and
`malloc_consolidate` hold, which says the fastbin churn #1148's entry describes
did not come back when the traffic fell again.

Two rows rise and both are a reused buffer's bookkeeping: `lex_line` 749,824 ->
777,023 for taking a buffer from the pool and giving it back once a line, and
`infer::infer` 1,167,175 -> 1,186,509 for clearing the gather vector. 46,000
instructions against 1,074,000 saved.

`eval_expr`, `check_merged` and `__memcmp_avx2_movbe` are byte-identical, which
is what a change confined to the lexer, the parser and one function of infer
should read as. Welfare 85.72 -> 86.06, ratcheted here.

### The scanner's line

`pos` is the column a caret goes under, so `Scanner` indexes characters and has
to hold the line as a `Vec<char>`. #1145 settled that and it still holds. What
goes is collecting a fresh one per line.

Scanners nest — an interpolation lexes its inner text with a scanner of its own
while the outer one still holds the line the interpolation was written on — so
the buffers come from a pool rather than a single slot. A `Scanner` takes one at
construction and gives it back in `Drop`, and the pool ends up holding one
buffer per level of nesting reached, each grown to the longest line it ever
took.

### The token vector, and why eight

`tokens` starts empty, reaches four and doubles from there. Eight covers most
lines outright. Sixteen takes 156 more allocations and puts **9.7%** on
`compile_peak_bytes`, which is a bad trade for a vector a `Line` keeps for the
whole parse; that was measured and declined.

### 3,788 blocks on Tok::clone, and the 196 they return

```rust
match self.toks.get(self.pos).map(|(t, _, _)| t.clone()) {
```

Two places did this, in `parse_atom_base` and `parse_pattern`, and dhat put
3,788 blocks on `<Tok as Clone>::clone` between them. `self.toks` is a
`&'a [_]`, so a token read out of it borrows the slice rather than `self`, and
the arms are free to move `pos` while holding one. Matching on `&Tok` compiles
as it stands.

It returns 196. A dhat run after the change puts **zero** on
`<Tok as Clone>::clone`, 3,197 on `parser.rs:2127` and 629 on `parser.rs:1793`
— the arms that build `Expr::Ident` and `Pattern::Var`, which need an owned name
and clone it there instead. The allocation moved to a different frame. What went
away is the clone for `Underscore`, `LParen`, `LBrace`, `LBracket`, the `Str`
arm of `parse_pattern` that only read its parts, and every path that matched
none of them.

That is the second time in a day a line in the map read high because the frame
above it was doing the allocating; #1148's entry corrected `lexer.rs:584` the
same way. **Read the frames before costing a fix.** A line number says where an
allocation was charged, not whether deleting the code there would remove it.

### The gather buffer

`callee_first` builds `let mut names: Vec<&str> = Vec::new()` inside its
per-declaration loop, fills it, sorts it, reads it and drops it. Hoisted out and
cleared, it is 1,367 blocks for two lines — the largest of the four, from the
smallest diff, and it was the last one looked at because the map charged it to
`infer.rs:250`, the call site.

### A fifth vector, measured and declined

`parse_app` accumulates a call's arguments the same way `lex_line` accumulates
its tokens, and dhat charged 1,100 blocks to the push. Reserving eight there
makes both counters worse:

```
compile_allocs      40,935 -> 41,649   +714    +1.7%
compile_peak_bytes 743,564 -> 860,940  +117,376  +15.8%
```

`Vec::new()` allocates nothing until something is pushed, and most of what
`parse_app` looks at is a bare atom with no arguments at all — reserving pays
an allocation for every one of those, where the empty vector paid none. The
peak is the other half: an `Expr::App` keeps its arguments for the whole
compile, so eight slots apiece are eight slots held. The 1,100 blocks are one
allocation per call that has arguments, and that one is not removable by
reserving.

The `tokens` vector differs on both counts. Every line has at least one token,
so its first allocation happens regardless, and reserving only moves where the
second one would have been.

### What the peak buys

The 992 bytes are the two pooled buffers: one long source line and one large
declaration, held for the process rather than for an iteration. Welfare weighs
compile speed at 0.28 and compile memory at 0.12, and 8.9% off the traffic
against 0.13% on the residency is not a close call. Rounds and visits do not
move at all — nothing here changes what the compiler decides, only what it
allocates while deciding it.

## 2026-08-30 — four allocations the front end made per declaration

Three of these built a `String` out of a name the program was already holding —
the family #1141 opened, and these are the last of it on the measured path. The
fourth is #1140's: a `Vec` per declaration where one flat vector and a start
would do.

```
flush_unused's shadowed set borrows   40,935 -> 40,231   -704
Local.name borrows                    40,231 -> 39,527   -704
synthesize_getters keys on the field  39,527 -> 39,092   -435
callee_first's call table goes flat   39,092 -> 38,462   -630
                                                       -2,473   -6.0%
compile_peak_bytes                   743,564 -> 735,254  -8,310   -1.1%
docs/kanso.wasm                    1,655,440 -> 1,654,157  -1,283 bytes
```

Measured one at a time, in that order.

### What it cost to run

```
compile_instructions  51,126,817 -> 50,455,686   -671,131   -1.31%
```

The allocator rows carry 478,000 of it: `_int_free` 2,580,177 -> 2,434,266,
`malloc` 1,790,546 -> 1,684,286, `_int_malloc` 3,123,992 -> 3,044,126, `free`
1,146,488 -> 1,077,244, `__rust_alloc` 852,633 -> 800,814, `malloc_consolidate`
966,246 -> 940,889. The rest is the `String` construction and drop that
callgrind's 90% threshold leaves without rows of its own.

`infer::infer` rises 65,058, and that is the flat call table's price: the
topological walk indexes `starts` twice per step where it followed one pointer.
A sixth of what the allocator gave back, for 630 blocks and 8,310 bytes.

The row named `HashMap<&str, ()>::insert` reads +69,302 and is a renaming rather
than a rise — `flush_unused`'s set was a `HashSet<String>` and is a
`HashSet<&str>` now, so its inserts moved from one monomorphisation to another
and the old one sat below the threshold. `eval_expr` and `check_merged` are
byte-identical.

Welfare 86.06 -> 86.32, ratcheted here.

### The shadow checker

`flush_unused` collected the names it had already reported into a
`HashSet<String>` — one `String` per binding in every scope the checker leaves.
The set can borrow from `self.locals`, which the loop only reads; the truncation
happens after it. The two fields are taken apart before the loop so that reading
one and writing the other is not one borrow doing both, and the set is scoped so
its own borrow ends before `self.locals.truncate`.

`Local` then gave up its owned name for a `&'a str`. `Resolver<'_>` became
`impl<'a> Resolver<'a>`, and `bind_pattern`, `bind_target`, `bind_target_field`
and `resolve_expr` take `&'a` of what they walk. Six signatures, and the
compiler named every one of them in turn.

### The getter synthesiser

```rust
if already.contains(&(ast::getter_name(field), ty.name.clone())) {
```

Inside a loop over every field of every declared type, to ask a question.
`getter_name` and `getter_field` are inverse, so the set can be keyed on the
field name rather than the getter's, and both halves of the key then borrow. The
`format!` and the clone move to the arms actually synthesised, where a new
declaration genuinely needs an owned name.

### The call table

`callee_first` built `vec![Vec::new(); program.fns.len()]` and filled each
declaration's vector completely before moving to the next — which is exactly the
shape that flattens. One `Vec<usize>` and a `starts: Vec<u32>` replace four
hundred headers, and the topological walk below reads
`&flat[starts[i]..starts[i + 1]]` where it read `calls[i]`.

This is the only one of the four that moves `compile_peak_bytes`, and it moves
it a long way: 8,310 bytes, which takes back the 992 that #1149's two pooled
buffers cost and 7,318 more. The three borrowed names leave the peak alone,
which is right — a `String` built to answer a question and dropped is traffic
rather than residency.

### What the map says is left

dhat before this change put 39,096 blocks against the counter's 39,092. The
eight largest lines, and what each one is:

```
3,197  parser.rs:2127   Expr::Ident's String — interning, declined in #1033
3,157  lexer.rs:622     the same String, built in lex_word
1,567  infer.rs:250     the call table this entry flattens
1,100  parser.rs:2114   an App's arguments — reserving measured worse, #1149
1,072  infer.rs:580     eval_call
1,067  parser.rs:2121   Box::new(head)
  959  lib.rs:610       the getter arms themselves
  861  infer.rs:225     one Vec<Set> per declaration for its parameter sets
```

The top two are one `String`, built once per identifier and copied once into the
AST, and the treatment for both is interning. That stays declined at 365
conversion sites. `infer.rs:225` is the call table's sibling and flattens the
same way, except that `Inference::params` is `pub` and read as
`inference.params[decl][i]` at seven sites outside infer.rs, so it wants an
accessor and a wider diff than this one.

## 2026-08-30 — the two tables the fixpoint kept per declaration

`infer` held two `Vec<_>`-per-declaration structures for the length of a
compile: a `HashSet<usize>` saying who to wake when a declaration's answer
changes, and a `Vec<Set>` holding its argument sets. Four hundred declarations,
so four hundred headers apiece, for sets that are usually a handful of small
integers.

```
the reader bitset       38,462 -> 37,119   -1,343
the params table        37,119 -> 36,268     -851
                                           -2,194   -5.7%
compile_peak_bytes     735,254 -> 741,350   +6,096   the bitset
                       741,350 -> 733,794   -7,556   the params table
                                            -1,460   -0.2%
front_end_rounds 40 and front_end_visits 16,806, unmoved by both
```

The two move the peak in opposite directions and ship together for that reason:
the bitset costs residency to buy traffic, and the params table more than pays
it back.

### What it cost to run

```
compile_instructions  50,455,686 -> 49,090,280   -1,365,406   -2.71%
```

The largest single fall this vein has taken. The allocator rows carry about
half — `_int_malloc` 3,044,126 -> 2,808,434, `_int_free` 2,434,266 -> 2,284,655,
`malloc` 1,684,286 -> 1,588,013, `malloc_consolidate` 940,889 -> 867,875, `free`
1,077,244 -> 1,015,812, `__rust_alloc` 800,814 -> 769,810, some 647,000 between
them — and the rest is hashing the bitset removed. Every `mark_reader` hashed a
`usize` into a set; it shifts and ors now.

`infer::infer` rises 87,854, which is where that work went: the wake loop scans
`ceil(n / 64)` words per wake instead of iterating a set that knew its own
members, and the flat params table indexes `param_starts` where it followed a
pointer. A fifteenth of the fall.

`eval_expr` moves 1,495 on two million and `check_merged` is byte-identical.
Welfare 86.32 -> 86.58, ratcheted here.

### Who to wake, as bits

`readers[i]` was a `HashSet<usize>`, and waking a declaration's readers cloned
it — the set is read while `ctx` is taken mutably, so a snapshot was the way to
release the borrow. It is a bitset now: one row of `ceil(n / 64)` u64 per
declaration, `mark_reader` sets bit `r`, and the wake loop copies the row into a
scratch vector taken from `ctx` with `mem::take` and walks it with
`trailing_zeros`. The clone goes with the hash set.

`front_end_rounds` and `front_end_visits` do not move, which is the check that
matters here: those two counters are exactly what a wake set that woke a
different set of readers would change.

The row costs `n * ceil(n / 64) * 8` bytes — 22,792 for `lib/json`'s four
hundred declarations — in one allocation, where the sets cost 1,343 blocks.

### The argument sets, flat

`Inference::params` was `Vec<Vec<Set>>` and is one flat `Vec<Set>` with a
`param_starts: Vec<u32>` beside it. `Set` is a `u16`, so a declaration of two
parameters had a heap block for four bytes.

The field is private now, behind `Inference::param(decl, at)`. Making it private
first was the way to find the readers: the compiler named all seven — beat.rs
four times, codegen.rs and dispatch.rs once each — and there was no need to
guess at a grep.

## 2026-08-30 — a set nobody read, and one that grew from empty

```
prune_unused_getters reserves its set   36,268 -> 36,234    -34
used_globals deleted                    36,234 -> 35,639   -595
                                                           -629   -1.7%
compile_peak_bytes                     733,794 -> 730,120  -3,674   -0.5%
```

### What it cost to run

```
compile_instructions  49,090,280 -> 48,743,776   -346,504   -0.71%
```

The two pieces show separately. The reservation is
`RawTable<(&str, ())>::reserve_rehash` 753,484 -> 654,170, a fall of 99,314 —
`prune_unused_getters`'s set no longer doubling its way up from nothing. The
deletion is in the allocator rows, `free` 1,015,812 -> 998,200, `__rust_alloc`
769,810 -> 755,343, `malloc_consolidate` 867,875 -> 861,041, plus the 595
`String`s no longer built and dropped, which the 90% threshold leaves without
rows of their own.

`lex_line`, `eval_expr` and `memrchr` are byte-identical: nothing here touches
the lexer or the fixpoint. Welfare 86.58 -> 86.66, ratcheted here.

### The set nobody read

`Resolver::used_globals` was a `HashSet<String>`, threaded through
`check_file`, `check_file_shadow`, `check_fn_body_shadow` and the `Resolver`
struct, and filled here:

```rust
match self.globals.contains(name) {
    true => {
        self.used_globals.insert(name.to_string());
    }
```

A `String` for every mention in the program that resolves to a module-level
name. A grep over `src/` and `tests/` finds eight occurrences of the field: one
insert, one struct field, four parameters, one pass-through, one initialiser.
None of them a read. At all four call sites the caller writes `let mut used =
Set::default()`, hands over `&mut used`, and never looks at it again.

`check_file`'s own doc comment says what it was for — "Records which
module-level names the file uses, for the unused-private check" — and that check
lives in `lib::private_uses` now, working from the imports rather than from this
set. The recording stayed behind when the check moved. #1072's family, and the
same treatment.

### The set that grew from empty

`prune_unused_getters` walks every statement of every non-getter declaration and
collects each identifier occurrence into a borrowed set, to ask afterwards which
getters were mentioned. `collect`-shaped growth from capacity nothing costs a
rehash sequence over a set that ends up holding every distinct name in the
program.

Two declarations' worth of room per declaration was measured against eight, and
eight is worse — 49,348,198 local instructions against 49,336,704 — because a
larger table probes further for the same contents. The number is a measurement
rather than a guess, and the comment beside it says so.

### What the sweep that found this cost, and what it refused

`callgrind --separate-callers=2` was run to answer a different question: which
of the compiler's `HashSet<&str>` inserts the profile's 2,075,915-instruction
row belongs to. The answer was ten callers with no owner — the largest 1.11% of
the compile — and two declines came out of it:

- **Filtering `prune_unused_getters`'s mentions against the getter names.** The
  walk already pays exactly one hash per mention, which is the floor for this
  shape; a filter pays one for the `contains` and then the insert anyway. It
  also cannot be a `Get_` prefix test, because `FnDecl::is_getter` reads the
  BODY (`[Stmt::Expr(Expr::Ident(name)) if name == GETTER_BINDER]`), so a
  hand-written function whose body is just `Read` is a getter under any name.
- **Pre-sizing the six `iter().filter(..).map(..).collect()` sets** in
  `advisory`, `beat`, `check` and `escape`. The premise holds — a `Filter`'s
  size_hint lower bound is zero, so those sets do start empty — and the
  measurement is 4,514 instructions and one allocation. Six diffs for 0.009%.

## 2026-08-30 — the last per-line scratch, and a decision filed

```
compile_allocs   35,639 -> 34,945   -694   -1.9%
compile_peak_bytes         730,120  unmoved
```

```
compile_instructions  48,743,776 -> 48,582,098   -161,678   -0.33%
```

The allocator rows are the whole of it — `_int_free` 2,284,655 -> 2,210,015,
`malloc` 1,588,013 -> 1,529,690, `_int_malloc` 2,808,434 -> 2,788,486, `free`
998,200 -> 978,768, `__rust_alloc` 755,343 -> 739,381 — and `lex_line` is
byte-identical at 777,023, which is right: the vector this removes was built in
the loop AFTER lexing rather than inside it. Welfare 86.66 -> 86.73, ratcheted
here.

`check_needless_continuation` groups a line's tokens by the source line each one
came from, to ask what the statement would measure written one line wide. It
built a `Vec<(usize, usize, Span)>` to do the grouping — per line of every file
compiled, for a scratch that dies at the end of the call, and for most lines
after one push and the early return two statements later.

One buffer for the file, cleared per line, the same treatment as #1149's gather
vector in `callee_first`. Its two neighbours in the same loop were read and left
alone: `check_partial_chain` and `validate_spacing` allocate nothing.

That is the end of the per-line and per-declaration scratch family. Over five
changes today the front end went from 46,998 allocation blocks to 34,945, a
quarter of them gone, and what is left at the top of the map is one thing.

### The decision that is left

design/pending-gavels.md gains **"Whether an identifier's name lives inline"**,
under Open, not blocking. The short version: `Expr::Ident`'s `String` and the
`Tok::Ident` it is cloned from are 6,983 blocks, **19.6% of every allocation the
front end makes**, and the two treatments measure very differently on the
library's own sources — 11,870 identifier occurrences, 1,399 distinct, and
**11,869 of the 11,870 are 22 bytes or shorter**. Interning removes the 88% that
are repeats and was declined in #1033 at 365 conversion sites; an inline name
removes 99.99% and touches only the ninety construction sites, because reads go
through `Deref`.

It is filed rather than built because it changes the type of a core AST field
across the compiler and means hand-writing the small-string type, and because
the other treatment of the same row was refused once already. The entry carries
the measurement, the site counts and a recommendation.

## 2026-08-30 — the last slash, found by hand

```
compile_instructions  48,582,098 -> 47,302,573   -1,279,525   -2.63%   (runner)
                      48,942,128 -> 47,651,194   -1,290,934   -2.64%   (local)
compile_allocs                       34,945   unmoved
compile_peak_bytes                  730,120   unmoved
front_end_rounds 40, front_end_visits 16,806   unmoved
welfare                       86.73 -> 86.79
```

The two hosts agree to within one per cent of the fall, which is the closest
they have come on this vein.

`memrchr` and `__memcmp_avx2_movbe` both leave the profile's top fifteen, and
`parser::parse` enters it at 593,309 having not moved at all. `lex_line`,
`eval_expr` and every allocator row are byte-identical.

A kanso name is qualified with a slash — `json/decode`, `std/io/read_file` — and
eighteen places in the compiler split one on the last slash to get at its owner
or its bare half. Every one of them wrote `name.rsplit_once('/')` or
`path.rfind('/')`, and those go through `str`'s `CharSearcher`, which lands in
`memrchr`'s vector path.

That path is built to scan kilobytes. The average identifier in `lib/json` and
the standard library it imports is **five bytes** — the length histogram over
11,870 occurrences puts 7,589 of them at four bytes or fewer — so the setup is
most of what a call costs, and there is nothing for the vector loop to do.

`ast::last_slash` is a backward byte loop, and `ast::split_qual` is the pair it
returns. A `/` is one byte and cannot appear inside a multi-byte character, so
the scan is over bytes and the index it hands back is a character boundary.

### What the profile said, and what it missed

`callgrind --separate-callers=2` put 794,000 instructions on three callers:
`mentions_in_expr` at 377,352, `provenance::package_of` at 209,540 and
`provenance::Walk::expr` at 206,628. Converting exactly those three measured
**1,095,492** — three hundred thousand more than the rows named, because the
searcher's own construction is charged to the caller rather than to `memrchr`.
Converting the other fifteen took it to 1,290,934.

That is the third time in two days the map has been read one way and the fix
measured another. Twice the rows read HIGH — `Tok::clone`'s 3,788 returned 196,
and `infer.rs:590` turned out to be a hash set rather than the call it was
attributed to. This one read LOW, and for the same underlying reason: a
profile's rows are where the cost was charged, and the fix moves whatever the
charge was standing in for.

### What did not move

Allocations, peak, rounds and visits are all identical, and so is the emitted
golden. Nothing here changes what the compiler decides or writes — only how it
finds a byte it was already looking for.

## 2026-08-30 — an answer the fixpoint recomputed forty times

The provenance pass runs a fixpoint over the call graph, and on `lib/json` it
takes forty rounds. Every round walked every declaration, and the first thing
it did for each was ask which package the declaration belongs to:

```rust
let pkg = bit(&table, package_of(&decl.file));
```

`package_of` splits a path on `.hako/` and then on its last slash; `bit` walks
the interned package table comparing strings until one matches. A declaration's
file does not change between rounds and neither does the table, so the answer
is fixed from the moment `intern` finishes. It was computed forty times anyway,
once per declaration per round.

It is a `Vec<Pkgs>` built before the loop now, indexed by zipping the
declarations against it. The `binds` map each declaration fills with its
err-carrying locals moved out too — it was a fresh `HashMap` per declaration
per round, and clearing one costs nothing where allocating a table costs a
block.

### The searcher under `package_of`

With the recomputation gone, `package_of` still ran twice per declaration —
once in `intern`, once building the table above — and callgrind still charged
108,446 instructions to its own row. `StrSearcher::new`, the setup for
`split_once(".hako/")`, had a row of its own at 230,143 with
`core::slice::memchr::memchr_aligned` at 171,128 under it: the searcher cost
more than twice the function it served, beside it rather than inside it.
Two-way substring search is built for haystacks
that justify its preprocessing; a source path is forty bytes and the needle is
absent from every one of them in a program that fetched nothing, so the search
never gets past its own setup. `after_hako` is a byte scan that tests the first
character before comparing the rest, and `.hako/` is ascii, so the index it
returns is a character boundary.

Reading the row afterwards is instructive in the same way #1154 was:
`package_of` went UP, 108,446 to 130,437, because the searcher's cost moved
from its own row into the caller that now does the work inline. The program
went down 315,908.

### The numbers

Measured in the box, container host, same binary either side:

| | instructions |
|---|---|
| main | 47,651,194 |
| the package table hoisted | 47,194,917 |
| the byte scan as well | 46,878,322 |

−772,872 in total, −1.62%. Allocations 34,945 → 34,812, the `binds` tables.
Peak, rounds and visits are unchanged: the pass decides exactly what it decided
before, and the emitted golden is byte-identical.

### The runner's number

    compile_instructions  47,302,573 -> 46,616,238   -686,335   -1.45%  (runner)
                          47,651,194 -> 46,878,322   -772,872   -1.62%  (local)
    compile_allocs            34,945 ->     34,812       -133   -0.38%  (both)
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806     unmoved
    welfare                            86.79 -> 86.83

Copied out of job 99257297594, which also confirmed the allocation and peak
rows to the digit. The two hosts disagree by 0.17 points of the fall — the
container's glibc is the same version but the CPU is not, and the searcher's
setup is where a host divergence would land.

## 2026-08-30 — a hash map for four names

Inference carries the locals a declaration has bound in a
`HashMap<&str, Set>`, and asks it a question for every identifier in every
body it walks. `callgrind --separate-callers=2` put three rows on it:

    668,032  HashMap::get'infer::eval_expr'infer::infer
    479,552  HashMap::insert'infer::bind_pattern'infer::infer
    359,917  HashMap::contains_key'infer::eval_expr'infer::infer

1.51 million instructions, 3.2% of the compile, for a map that is almost
always tiny. Instrumented, `lib/json`'s deepest scope holds **seven** entries
and the average lookup would walk 2.39 of them. The repo's own kanso programs
agree: `scripts/welfare` peaks at 30 and averages 2.89, `prose_check` at 11 and
2.77, `fingerprint` at 11 and 2.70, `trend_gate` at 16 and 2.41. The mean stays
under three even where the maximum is thirty, because the name a body asks
about is usually the one it just bound.

So `Env` is a `Vec<(&str, Set)>` read back to front. A bind pushes rather than
replaces, which gives shadowing for free and cannot grow past the bind sites
written in the declaration. Lookup answers exactly what the map answered.

### The clone was the point

Every child scope — a block, a lambda's body, a guard's untaken side, an
applied lambda's beta step — took `env.clone()`, and a `HashMap`'s clone
allocates a table and rehashes every entry into it. A `Vec`'s clone is a
memcpy.

The first cut of this regressed allocations by 216, which is the sort of thing
the counter is for. `Vec::clone` sizes to what it copied, so the child's first
bind reallocated; the map's clone had carried its source's load-factor headroom
and usually did not. `Env::child(extra)` takes the count the caller already
knows — a block's statement count, a lambda's parameter count — and allocates
once. Allocations come back to identical.

### The numbers

    compile_instructions  46,878,322 -> 45,764,825  -1,113,497  -2.38%  (local)
    compile_allocs                        34,812   unmoved
    compile_peak_bytes                   730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

The three rows above held 1.51 million and the change returned 1.11 million, so
this one read HIGH — the opposite of #1154, and for the same reason in reverse:
a `HashMap::get` row is the whole lookup, where the searcher rows were only
part of one. The vector's own scan is what is left.

### The runner's number

    compile_instructions  46,616,238 -> 45,468,261  -1,147,977  -2.46%  (runner)
                          46,878,322 -> 45,764,825  -1,113,497  -2.38%  (local)
    compile_allocs                        34,812   unmoved, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts
    front_end_rounds 40, front_end_visits 16,806   unmoved

Copied out of job 99264116335. The runner reads the fall three per cent larger
than the container does, the two hosts' closest agreement on this vein after
#1154's one per cent.

## 2026-08-30 — a hash per identifier, to ask about thirty-one names

`prune_unused_getters` drops the field getters nothing reads. It builds a set
of every name any non-getter body mentions, then asks that set, once per
getter, whether the getter's name is in it. `callgrind --separate-callers=2`
charged the building 540,464 instructions plus 84,064 in `memcmp`, the largest
single hashing caller in the compile.

Instrumented, the numbers are lopsided. `lib/json`'s largest module walks
about twelve thousand identifier occurrences into a set that ends up holding
**325 distinct names**, and asks it about **31 getters**. Almost every insert
is a duplicate of one already there, and almost every entry is a name no
getter could have.

A getter is synthesised in exactly one place, as `Get_{field}`, and the binder
that makes `is_getter` true — `Read` — is one no source can spell. So a
declaration is a getter only if it came from there, and a name that is not
`Get_`-prefixed can never be one. The walk tests four bytes and skips the
insert.

A qualified mention reaches the same getter under its bare half, so the test
is on the bare half and both halves go in together when it passes. The set the
question is asked of is unchanged; what changed is everything that was never
going to be asked about.

The reservation went with it. It was `program.fns.len() * 2` — 864 entries for
the largest module — sized for a set that held one name per mention. It now
holds at most two per getter.

### The numbers

    compile_instructions  45,764,825 -> 45,155,678  -609,147  -1.33%  (local)
    compile_allocs             34,812 ->     34,804        -8
    compile_peak_bytes                     730,120   unmoved

The two halves were measured separately against #1155's head, where the pair
came to 603,447: the prefix test was 601,800 of it, and dropping the
reservation was the remaining 1,647 and all eight allocations.

### The runner's number

    compile_instructions  45,468,261 -> 44,862,145  -606,116  -1.33%  (runner)
                          45,764,825 -> 45,155,678  -609,147  -1.33%  (local)
    compile_allocs                        34,804   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99268499869. The two hosts read the same percentage.

## 2026-08-30 — thirteen tables that start at nothing

`check_merged` runs twenty-two whole-program checks, and a dozen of them open
by grouping declarations: arities by name, return sets by group, discarded
positions, handled positions, torn arms, the reference graph. Each builds its
own map with one entry per declaration, and each starts that map empty.

A `hashbrown` table doubles, and doubling means allocating the new table and
rehashing every key already in the old one. Filling a table to twelve hundred
entries from zero pays that seven times. `callgrind` charged
`reserve_rehash` 654,170 instructions, and `fallible_with_capacity` under it
another 190,355.

Thirteen of these now open at `program.fns.len()`. Two are keyed by
`(name, arity, position)` and can hold more than that, so they still grow
once; the rest never grow at all.

### Not the same verdict as the last time

Pre-sizing was measured and DECLINED on 2026-08-30 for the six
`filter().map().collect()` sets in `check.rs`, at 4,514 instructions and one
allocation. That decline stands and this is not in tension with it: those
collects are FILTERED, so their results are a small fraction of the
declarations and the table they build never doubles more than once. These
thirteen hold one entry per declaration.

### The numbers

    compile_instructions  45,155,678 -> 44,778,375  -377,303  -0.84%  (local)
    compile_allocs             34,804 ->     34,682      -122
    compile_peak_bytes                     730,120   unmoved

Peak does not move, which is the answer to the obvious worry: a table sized to
what it will hold occupies no more than one that grew into the same size, and
the intermediate tables it does not build are the allocations that went away.

### The runner's number

    compile_instructions  44,862,145 -> 44,483,421  -378,724  -0.84%  (runner)
                          45,155,678 -> 44,778,375  -377,303  -0.84%  (local)
    compile_allocs                        34,682   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99272811720.

## 2026-08-30 — the other direction

#1154 replaced eighteen backward slash searches with `ast::last_slash`, a
plain byte scan, and took 1,279,525 instructions off the compile. It left the
forward family alone. There are twenty-four of those: `name.contains('/')`
across `advisory.rs`, `check.rs`, `lib.rs` and `trmc.rs`, `split_once('/')`
in `used_quals`, and four `rsplit('/').next()` chains that want the last
segment and pay a full pattern search to get it.

`contains` and `split_once` on a `char` go through `memchr` the way the
backward pair went through `memrchr`, and the argument is the same: the
average identifier in the shipped library is five bytes, and a routine built
to scan kilobytes spends most of that setting itself up.

`ast` gains `first_slash`, `has_slash`, `split_module` and `bare_name`, and
every one of the twenty-four sites reads one of them.

### Why `split_module` is not `split_qual`

They cut in different places and mean different things. An owner is everything
before the LAST slash — `std/net/http/get` belongs to `std/net/http` — which
is what `split_qual` gives. A qualifier is the FIRST segment, the module name
an import wrote, so `used_quals` credits `std`. Folding the two together would
have been the kind of change that passes every test and quietly credits the
wrong import.

### The numbers

    compile_instructions  44,778,375 -> 44,491,145  -287,230  -0.64%  (local)
    compile_allocs                        34,682   unmoved
    compile_peak_bytes                   730,120   unmoved

Smaller than #1154's, and it should be: two of the twenty-four run per
identifier occurrence and the rest run once per declaration.

### The runner's number

    compile_instructions  44,483,421 -> 44,130,291  -353,130  -0.79%  (runner)
                          44,778,375 -> 44,491,145  -287,230  -0.64%  (local)
    compile_allocs                        34,682   unmoved, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99276951052. The runner reads the fall a fifth larger than
the container does, the widest the two have parted on this vein today; both
directions of the slash family are pattern-search setup, and setup is where a
host divergence lands.

## 2026-08-30 — DECLINED: the ascii fast path in the scanner

`Scanner::new` turns a line of source into the `Vec<char>` the lexer reads with
`chars.extend(content.chars())`, and `Chars` decodes every character whether or
not there is anything to decode. `str::is_ascii` answers in one pass over eight
bytes at a time, and when it says yes the widening loop is straight-line. The
row it was aimed at: 298,362 instructions on the fold under `lex_line` and
242,520 on the extend beneath it.

Measured against #1156's head it took 150,938 instructions. Measured against
this branch's head, four merges later, the same patch takes **53,017**:

    compile_instructions  44,491,145 -> 44,438,128   -53,017  -0.12%
    compile_allocs             34,682 ->     34,680        -2
    compile_peak_bytes        730,120 ->    730,332      +212

Nothing about the patch changed. What changed is everything around it — the
getter prefix test, thirteen sized tables, twenty-four byte scans — and with
them the inlining and layout the lexer's own code gets. **A micro-change
measured against a moving baseline is measured once, and the number is only
about the tree it was taken in.** This one lost two thirds of itself while
sitting in a queue.

Peak goes UP by 212 bytes, and not because of the fast path itself: a byte
slice's iterator reports an exact size where `Chars` reports a lower bound of a
quarter of the length, so `extend` reserves exactly what it needs and the
pooled buffer settles at a capacity the doubling schedule would not have
reached. Replacing `extend` with a plain `push` loop to dodge the size hint
made it worse on both counts — 44,557,543 instructions, one more allocation,
and the peak still up at 730,312.

Welfare reads 86.94 either way. The sum does not move, the memory term pays,
and the rule is that a term getting worse is only defensible when the sum goes
up. So this is declined, and it stays declined unless the lexer is reworked in
a way that makes the decode itself the question.

## 2026-08-30 — DECLINED: the operator table's two-character key

`lex_line` searches `OPS` by text, and built the text to search for out of the
current character and its successor:

```rust
let two = [c, s.peek(1).unwrap_or(' ')].iter().collect::<String>();
```

A two-character `String` is a heap allocation, made and dropped once per
character that reaches this line. Removing it removes **163** allocations from
`lib/json`'s compile — and costs instructions:

    the String, as it stands                44,491,145   34,682 allocs
    a stack buffer and `str::from_utf8`     44,552,688   34,519
    a byte compare against `op.as_bytes()`  44,521,443   34,519
    the table read as a `match (c, next)`   44,521,899   34,519

Three shapes, one answer: whatever replaces the `String`, the compile does
about thirty thousand instructions MORE. The search is not the cause — the byte
compare and the match agree to within five hundred, and they search in quite
different ways. Something about the surrounding code compiles differently once
the allocation leaves, and 163 allocations at roughly two hundred instructions
apiece do not pay for it.

Welfare prices the trade at +0.01, because allocations and instructions share a
dimension and the allocation fall is proportionally the larger. That is a real
gain by the objective and it is also inside the noise of what a day's merges do
to this file's inlining — the ascii scanner above lost two thirds of its value
to exactly that. Declined on those grounds: a change that makes the compiler do
more work, cannot say where the work went, and buys a hundredth of a point is
not worth the line it changes.

The row it was aimed at is still there and still worth a better idea:
`String as FromIterator<&char>` under `lex_line` is 361,176 instructions with
another 156,728 in `finish_grow` beneath it, from the four sites that build a
token's text out of the scanner's `Vec<char>` one character at a time. The fix
for those is not a faster copy; it is a scanner that keeps the source `&str`
and hands out slices of it, which is the shape gavel "whether an identifier's
name lives inline" is about.

## 2026-08-30 — DECLINED: the precedence ladder, and two fixtures that say why

Expression parsing descends ten levels — pipe, join, or, and, not, cmp, bits,
add, mul, app, atom — and on `lib/json` the descent itself costs **681,679**
instructions, 1.53% of a compile:

    parse_pipe   2,963,379  inclusive
    parse_atom   2,281,700  inclusive

Everything between is a frame entered to find no operator of its precedence.
Precedence climbing collapses the middle into one frame with a binding-power
table, and that is the standard answer.

It is the wrong answer here, and two programs say so.

    print "{6 < 3 & 1}"     error[syntax]: unexpected trailing tokens
    print "{1 + not true}"  error[syntax]: expected an expression

`parse_cmp` takes its left side from `parse_bits` and its right side from
`parse_add`, one rung TIGHTER, so `&` may stand to the left of a comparison
and not to the right. `6 & 3 < 1` is `(6 & 3) < 1` and answers false; `6 < 3 &
1` has nothing to attach the `& 1` to. And `parse_not` is reachable only where
an `and` operand is expected, six rungs looser than an arithmetic operand, so
`1 + not true` has no expression after the `+`.

A single binding-power table makes both of those symmetric. `6 < 3 & 1` would
become `6 < (3 & 1)` and `1 + not true` would parse, and neither change would
show up anywhere: no fixture covered either, so the whole differential corpus,
the error corpus and 77 suites would have stayed green while the language
quietly grew two readings it does not have.

So the rewrite is declined, and the gap it exposed is closed instead:

- `tests/golden/errors/a_comparison_refuses_a_bitwise_tail`
- `tests/golden/errors/an_arithmetic_operand_refuses_not`
- `tests/golden/micro/bitwise_binds_tighter_than_a_comparison`

Two refusals and the reading that survives. If the ladder is ever regularised
these three go red, which is the point: whether the grammar SHOULD be uniform
here is a question for the gavel, and a performance change is not the place to
answer it.

### Watched fail, each for its own reason

A fixture written after the fact is a guess about what the code does, so each
of the three was put in front of the mutation it exists to catch:

| mutation | fixture | what it did |
|---|---|---|
| `parse_cmp`'s right side from `parse_bits`, symmetric | a_comparison_refuses_a_bitwise_tail | printed `false`, exit 0, where the golden is a syntax error and exit 2 |
| `not` admitted as a prefix in `parse_app` | an_arithmetic_operand_refuses_not | reached the runtime and failed on `+`, exit 1, where the golden refuses at parse time |
| `parse_cmp`'s left side from `parse_add` | bitwise_binds_tighter_than_a_comparison | `6 & 3 < 1` became a syntax error where the golden is `false` |

The first mutation is exactly what a binding-power table would do. It costs
nothing to write and the corpus said nothing about it until now.

## 2026-08-30 — a vector per name, for a table that is only read

dhat's map of the front end's remaining 34,682 allocation blocks put 937 of
them on one line: `infer.rs:287`, the call to `callee_first`. That function
builds the callee-first visit order the fixpoint sweeps in, and it opens with

```rust
let mut by_name: HashMap<&str, Vec<usize>> = ...;
for (i, decl) in program.fns.iter().enumerate() {
    by_name.entry(decl.name.as_str()).or_default().push(i);
}
```

which is one `Vec` per DISTINCT declaration name. `lib/json` has about nine
hundred of them, the table is built once and afterwards only read, and every
one of those vectors holds a handful of `usize`.

It is a flat `Vec<usize>` and a range per name now, the shape #1140 gave the
dispatch table and #1150 gave the call table. The arms of a name land in it by
a counting pass: count per name, turn the counts into starts, then walk the
declarations once placing each index at its name's cursor. Within a name the
indices come out ascending, which is the order `push` gave them, so the flat
callee list the fixpoint reads is byte-identical.

### The sort that did not work

The obvious way to group is to sort `(name, index)` pairs, and that was
measured first: allocations fell by 519 and instructions rose by **117,356**.
Twelve hundred string comparisons through a comparison sort cost more than the
nine hundred allocations they removed. The counting pass does the same
grouping with two hash lookups per declaration and no comparisons.

    the sort               44,608,501   34,163 allocs
    the counting pass      44,427,648   34,158 allocs
    as it stood            44,491,145   34,682 allocs

### The numbers

    compile_instructions  44,491,145 -> 44,427,648   -63,497  -0.14%  (local)
    compile_allocs             34,682 ->     34,158      -524
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

Both terms move the right way, which is what makes this one uncomplicated:
welfare 86.94 -> 86.99.

### Where the rest of the blocks are

The same map, for the record, since two of the top four are gavel territory:

    3,197  parser.rs:2127   the `String` in `Expr::Ident`, charged at parse_atom_base
    3,157  lexer.rs:631     the `String` `lex_word` returns
    1,100  parser.rs:2114   an application's argument vector, reserved and DECLINED
    1,067  parser.rs:2121   the `Box` on an application's head
      959  lib.rs:607       the prelude and the synthesised getters
      937  infer.rs:287     this entry
      644  trmc.rs:180      a `Vec` per group, the same shape as this one

`trmc.rs:180` is left alone deliberately: its map is ITERATED to build the
rewritten arms, so the order the table hands its keys back is part of what the
compiler emits, and changing the value type changes that order. The emitted
golden would catch it, but a reordering is not what 644 allocations are worth.

### The runner's number

    compile_instructions  44,130,291 -> 44,100,285   -30,006  -0.07%  (runner)
                          44,491,145 -> 44,427,648   -63,497  -0.14%  (local)
    compile_allocs                        34,158   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99289426525. The runner reads less than half the fall the
container does, the widest the two have parted all day — every earlier change
today agreed within a fifth. What differs between the hosts here is the cost
of an allocation, and this change is almost entirely allocations: the
counting pass and the `or_default().push` do nearly the same number of hash
operations, so what is left is nine hundred mallocs, and a malloc is exactly
the thing whose price is a property of the allocator and the machine rather
than of the compiler. The allocation row, which is not host-dependent, agrees
to the digit.

## 2026-08-30 — two more tables that allocate a vector per group

#1161 did this to `callee_first`. The same shape was in two more places, and
between them they were worth four times as much.

**`infer.rs:204`** builds the dispatch groups the fixpoint reads:

```rust
let mut by_group: HashMap<(&str, usize), Vec<usize>> = ...;
for (i, decl) in program.fns.iter().enumerate() {
    by_group.entry((decl.name.as_str(), decl.params.len())).or_default().push(i);
}
let mut group_members: Vec<usize> = Vec::with_capacity(program.fns.len());
let groups: HashMap<(&str, usize), (u32, u32)> = by_group.into_iter().map(...).collect();
```

The flat vector and the ranges were already the destination. The per-group
`Vec` existed for one line, to be poured into `group_members` and dropped —
529 blocks whose only job was to be flattened. The counting pass writes
straight into the flat vector.

**`check.rs:2757`**, `check_constants`, collects each run of consecutive
same-named declarations into a `Vec<&FnDecl>` to ask three questions of it:
whether any arm takes no parameters, how many arms there are, and where the
second one is. All three read off a slice of `program.fns`, so the run is
walked in place.

### The numbers

    compile_instructions  44,427,648 -> 43,955,167  -472,481  -1.06%  (local)
    compile_allocs             34,158 ->     32,856    -1,302   -3.8%
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

dhat's map said 529 and 579, and 1,302 went away. The gap is the runs: dhat
charges an allocation to the line that made it, and `vec![decl]` at
`check.rs:2762` was reached once per RUN while the map's line was reached once
per group — the map undercounts a site whose work is spread over a loop it
does not name. The direction was right and the size was not, which is the
third time this week the map has been read one way and the change measured
another.

Welfare 86.99 -> 87.13, the largest single step the compile vein has taken
since the counters went in.

### The runner's number

    compile_instructions  44,100,285 -> 43,618,236  -482,049  -1.09%  (runner)
                          44,427,648 -> 43,955,167  -472,481  -1.06%  (local)
    compile_allocs                        32,856   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99293723322. The two hosts agree within two per cent of the
fall — where #1161, which removed only mallocs, had them a factor of two
apart. This change removes hash operations and vector copies as well as
mallocs, and those cost the same on both.

## 2026-08-30 — eight changes, and what they did to gavel #159

#1155 through #1162 all went at the front end, each small enough to state in a
sentence. Together:

    compile_instructions  47,302,573 -> 43,618,236  -3,684,337  -7.79%  (runner)
    compile_allocs             34,945 ->     32,856     -2,089   -5.98%
    compile_peak_bytes                     730,120   unmoved across all eight
    welfare                     86.79 ->      87.15

The falls came from four shapes, and the queue is now out of instances of all
four.

**A decision hoisted out of a loop.** `package_of` ran once per declaration per
fixpoint round; it runs once per declaration now, and the `.hako/` search that
fed it is a byte scan rather than a substring searcher (#1155, -686,335).

**A map replaced by a vector when the map was small.** Infer's scope holds a
handful of names and was a hash map; it is a flat vector with a reverse linear
scan (#1156, -1,147,977).

**A necessary condition tested before the expensive one.** Only `Get_`-prefixed
names can name a getter, so `prune_unused_getters` no longer builds a set of
every identifier in the module to ask about thirty-one of them (#1157,
-606,116).

**A table of per-group vectors flattened by a counting pass.** Count per key,
turn the counts into starts, then place each item at its key's cursor: one
`Vec` and a range per key instead of a `Vec` per key. Three tables took it —
`callee_first` in #1161, `by_group` and `check_constants` in #1162 — for
512,055 instructions and 1,826 blocks between them.

Two changes were the same idea applied twice with no cleverness: thirteen maps
in `check.rs` sized at construction (#1158, -378,724) and twenty-four
`find('/')` calls replaced with a forward byte scan (#1159, -353,130).

Three things were built, measured and declined, each with its number: the ascii
fast path in `Scanner::new`, the operator table's two-character `String`, and
the precedence-climbing rewrite of the expression parser. The last was worth
681,679 instructions and was declined because two of the grammar's precedence
asymmetries were observable and unpinned; #1160 pinned them with three
fixtures, so the decline is a decision somebody can revisit rather than a
hazard.

### What is left

dhat on the new head. The three rows gavel #159 names have not moved a byte
across any of the eight:

```
3,197  parser.rs:2127  Expr::Ident's String, cloned out of the token
3,157  lexer.rs:631    the same String, built by lex_word
  629  parser.rs:1793  Pattern::Var's String, same source
------
6,983
```

The share moved because the denominator did. These were 19.6% of 35,643 blocks
when the gavel was filed and they are 21.3% of 32,860 now. Below them the map
is a long tail: 1,100 at parser.rs:2114 (the App args vector, measured and declined
earlier today), 1,067 at parser.rs:2121, 959 at lib.rs:607, then nothing over
750. `trmc.rs:180` (644)
is deliberately left alone — its map is iterated to build the rewritten arms,
so key order is part of what the compiler emits.

The gavel entry has been refreshed with the current share. Nothing else in it
changed: the two treatments measure the way they did, and the ruling is still
Clay's.

## 2026-08-30 — three sets opened once per declaration

`check.rs` walks `program.fns` in three separate passes, and each pass opened a
fresh set of the names the declaration binds before walking its body:

```rust
for decl in &program.fns {
    if decl.synthetic { continue; }
    let mut bound: HashSet<&str> = HashSet::default();
    ...
}
```

hashbrown allocates nothing for an empty set, so the cost lands on the first
insert — which every declaration that binds anything reaches. The capacity the
set had just grown to is dropped with it, so the next declaration starts from
nothing and grows again.

One set per pass, cleared at the top of each iteration, keeps that capacity and
allocates once. The names borrow from `program`, which outlives the loop, so
the lifetime the hoist needs is the one the set already had. The three passes
are `check_call_shaped_list`, `check_call_arities` and
`check_literal_arguments`.

### The numbers

    compile_instructions  43,955,167 -> 43,448,641  -506,526  -1.15%  (local)
    compile_allocs             32,856 ->     31,316    -1,540   -4.7%
    compile_peak_bytes                     730,120   unmoved

The instruction fall is larger than 1,540 mallocs would explain on its own: a
set that starts empty rehashes as it fills, and one that starts at the capacity
the previous declaration needed usually does not.

dhat put 726 and 732 on two of the three and the change removed 1,540, so this
time the map's arithmetic held. It has been read one way and the change
measured another three times this week; it is worth writing down when it
agrees.

### And a guard asked one line too late

`synthesize_getters` built each candidate arm's pattern vector — one `Vec` and
a `String` for the binder — and then asked whether that (type, field) already
had a getter and threw the arm away. The question moved above the construction.
It is worth 62 allocation blocks on `lib/json`, where most fields do not
already have one; on a program built mostly of imported types it is worth more.
Recorded here at its measured size rather than its imagined one.

### The runner's number

    compile_instructions  43,618,236 -> 43,096,058  -522,178  -1.20%  (runner)
                          43,955,167 -> 43,448,641  -506,526  -1.15%  (local)
    compile_allocs                        31,254   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99301368868. Three per cent apart on the fall — the saving is
a malloc, a free and the rehashes a set that starts empty pays on the way up,
and all three cost about the same on both hosts.

`HashMap<&str, ()>::insert` is now the largest kanso-owned row after inference
at 1,517,265 (3.52%), where the sets that just went away were feeding it.

Welfare 87.15 -> 87.32, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — three more tables that allocate a vector per key

The counting pass of #1161 has now taken six tables. Three were left in
`check.rs`, and they are the last of the shape in the front end.

**Two arity tables.** `check_call_arities` and the shadow checker's `Declared`
each built a `HashMap<&str, Vec<usize>>` of the distinct arities a name is
declared at — a heap allocation per name, for a list that holds one number in
almost every case. They share an `Arities` now: count the arms per name, turn
the counts into starts, then place each declaration's arity at its name's
cursor, skipping one already there. The counts bound the ranges from above, so
a name declared three times at one arity leaves two cells unread between it and
the next name; nothing iterates the flat vector whole, so the gaps cost
nothing to carry.

**The overlap checker's groups.** `check_overlapping_arms` collected each
dispatch group into a `Vec<&FnDecl>` to compare its arms pairwise. The pairwise
loops index a range of the flat vector instead.

### The numbers

    compile_instructions  43,448,641 -> 43,353,124   -95,517  -0.22%  (local)
    compile_allocs             31,254 ->     30,406      -848   -2.7%
    compile_peak_bytes                     730,120   unmoved

Split: the two arity tables were 499 blocks and 42,087 instructions, the
overlap groups 349 and 53,430. The group table costs more per block because its
`Vec` was reallocated as arms were pushed, where an arity list took one
allocation and stopped.

dhat said 353 for the group table and 366 + 270 for the two arity tables. The
group row was right to the block; the arity pair came in at 499 against 636,
because part of what those rows held was the `fields` and `type_arity` maps
beside them, which have not moved.

Two of the shape remain: `check_literal_arguments`' `Vec<&FnDecl>` per group,
and `advisory.rs`'s `Vec<usize>` per name. Both are read through walks that
take the table as a parameter — five signatures in advisory's case — so
flattening either means threading the flat vector beside the ranges and giving
the lookup key a lifetime the walk can name. Left for now, with the reason.

### The runner's number

    compile_instructions  43,096,058 -> 43,001,117   -94,941  -0.22%  (runner)
                          43,448,641 -> 43,353,124   -95,517  -0.22%  (local)
    compile_allocs                        30,406   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99303996583. Six tenths of a per cent apart on the fall,
the closest two hosts have come on this vein.

Welfare 87.32 -> 87.41, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — the log is forty entries again

`design/compiler-log.md` had reached 111 entries and 6,352 lines against a
stated cap of forty. Seventy-one entries move to
`design/log/compiler-log-archive.md`, unedited and in the order they were
filed; the live file is 2,732 lines and forty entries counting this one.

The archive gains a note at the seam. The eight entries dated 2026-08-26 that
open the moved run were written on a branch that did not merge until
2026-08-28, so they sit after the 2026-08-27 entries already in the archive.
The live log's header carried that explanation while those entries were in it;
it goes with them.

Nothing was edited on the way. The move was verified by reconstruction: the
archive's old content is still a prefix of the new file, and the moved run
concatenated with what stayed is byte-identical to the body that was there
before, whitespace normalised.

`scripts/page_drift` counts `+## ` lines in the log's diff since the page's
last commit, so a trim — which only deletes from that file — cannot move its
number. It reads 0/3 here because the page moved in #1165.

## 2026-08-30 — the last two tables of the shape

#1165's entry named two that were left, and said what stood in the way:
`check_literal_arguments`' `Vec<&FnDecl>` per group and `advisory.rs`'s
`Vec<usize>` per name, both read through walks that take the table as a
parameter. Both are done, and the obstacle was smaller than it read.

**A tuple key cannot be probed with a borrowed name; a `&str` key can.**
`HashMap<&'a str, V>::get` accepts any `&str` because `&'a str: Borrow<str>`,
so a struct with a named lifetime still answers a lookup from a shorter one.
`HashMap<(&'a str, usize), V>` gets no such implementation, which is why the
literal-argument groups had to be rekeyed rather than merely flattened: the
ranges are keyed by NAME, and each arm's arity travels beside its index in the
flat vector, so `arms(name, arity)` slices the name's range and filters. Group
sizes are two or three, so the filter is cheaper than the second hash the tuple
key would have cost.

**advisory's table needed only the flattening.** It was already keyed by name,
and it skips synthetic declarations — so the counts are an upper bound and a
group whose synthetic arms were dropped leaves cells nothing reads, the same
slack the arity tables carry.

### The numbers

    compile_instructions  43,353,124 -> 43,302,778   -50,346  -0.12%  (local)
    compile_allocs             30,406 ->     29,876      -530   -1.7%
    compile_peak_bytes                     730,120   unmoved

Split: the literal-argument groups 349 blocks and 10,222 instructions, the
advisory groups 181 and 40,124. The instruction split runs opposite to the
block split, and the reason is where each table is read. advisory's is asked
once per identifier in every expression the door analysis types; the
literal-argument one is asked only at applications whose callee is a bare
name. A table's cost is the reads, and the allocations are what building it
happened to take.

No table in the front end now holds a `Vec` per key. Eight of them went over
four changes — #1161, #1162, #1165 and this one — for 3,204 allocation blocks
between them: 524, 1,302, 848 and 530.

### The runner's number

    compile_instructions  43,001,117 -> 42,914,281   -86,836  -0.20%  (runner)
                          43,353,124 -> 43,302,778   -50,346  -0.12%  (local)
    compile_allocs                        29,876   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99309718552. The widest the two hosts have been on this vein
since #1161, and in the direction where the runner gains more. Both changes
trade an allocation and a pointer chase for a short linear scan of a flat
vector, and the two hosts price that trade differently — a scan that stays in
cache costs what the machine's cache costs. The allocation row agrees to the
digit either way, which is the point of having both.

Welfare 87.41 -> 87.47, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — the bare-name walk asks a question it already knows the answer to

`mark_bare_quals` decides which import qualifiers a file actually uses. It
walks every expression of every declaration, collects every bare identifier
occurrence into a set, and then asks that set two questions: is each surfaced
name in it, and is any import rename's target in it.

The set therefore holds every distinct bare name in the module, and is asked
about the surfaced names and the rename targets — a much smaller list. A name
outside that list can never answer either question, so the walk tests
membership in the asked-about set before inserting.

This is #1157's move in a different pass. There the necessary condition was a
`Get_` prefix, cheap enough to test on four bytes; here it is membership in a
set built once from the two readers' own inputs. What makes both exact is the
same thing: the condition is on the MENTION, and the readers are known.

### The numbers

    compile_instructions  43,302,778 -> 43,138,593  -164,185  -0.38%  (local)
    compile_allocs             29,876 ->     29,864       -12
    compile_peak_bytes                     730,120   unmoved

The allocation move is twelve blocks, and the instruction move is a hundred and
sixty times that. The set was not costing much to hold; it was costing to fill.
`HashMap<&str, ()>::insert` under `walk_children` under `mark_bare_quals` was
267,972 instructions on the previous head — the largest single insert caller in
the compile — and it is a lookup in a small table now.

### The runner's number

    compile_instructions  42,914,281 -> 42,748,207  -166,074  -0.39%  (runner)
                          43,302,778 -> 43,138,593  -164,185  -0.38%  (local)
    compile_allocs                        29,864   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99313216447. One per cent apart on the fall. A hash lookup
traded for a hash insert is the same trade on both hosts, where #1167's scan
for a pointer chase was not.

Welfare 87.4654 -> 87.4740, banked with `--set` — three hundredths of a point
on the compile-speed term, which is what a 0.39% fall is worth at this end of
the satiation curve. The page's two `data-golden` compile figures move with it.

## 2026-08-30 — two playground tests, one file

`tests/playground.rs` failed once during a full run with
`the interpreter failed on the hello example: a play file needs at least one
statement to run`. The example is one line and runs fine; the file the
subprocess read was empty.

`written` staged every example into one fixed directory,
`kanso-playground-test/<name>.kso`. Two of the three tests in that binary write
files, they run on separate threads of one process, and both write the same
fourteen names. `std::fs::write` truncates and then fills, so a subprocess
spawned by one test can read the file the other is part-way through rewriting.

### The chain, each link measured

- an empty `.kso` produces exactly that diagnostic, byte for byte;
- the truncate window is wide — a reader racing a rewriter on this filesystem
  saw an empty file on 28,055 of 35,205 reads, 79.7%;
- the two tests write the same paths, which is plain from the code.

What the window does not explain on its own is the rate: running the playground
suite alone, twelve times, fails zero times either way. The tests spend almost
all of their time in subprocesses rather than inside `written`, so the two
threads are rarely in that window together — the observed failure came during a
full `cargo test` with a valgrind run and two compiles alongside it. A rare
race is still a race, and it fails on whichever test loses.

One directory per test. The fix is four lines and the argument for it is the
mechanism rather than a failure count, which is the honest way round for a race
this thin.

## 2026-08-30 — a word is a copy, not a re-encode

`lex_word` built its `String` by collecting `&char` out of the scanner's
`Vec<char>`, which encodes each character back into UTF-8 one at a time.
`String::from_iter<&char>` under `lex_line` was 361,176 instructions, 0.84% of
the front end.

The scanner already receives the line as a `&str`; it kept only the `Vec<char>`
because `pos` is the column a caret goes under. It keeps both now, with one bit
saying whether they agree: on a line that is ascii throughout, a character index
is a byte index, so `start` and `pos` slice `src` directly and the word is one
copy. A line with anything wider falls back to the collect, and the check is
`str::is_ascii` once per line — a vectorised byte scan.

### The numbers

    compile_instructions  43,138,593 -> 42,731,199  -407,394  -0.94%  (local)
    compile_allocs                        29,864   unmoved
    compile_peak_bytes       730,120 ->    728,030    -2,090   -0.29%

Peak moves because a `String` built by `to_string` on a slice is allocated at
the length it needs, where one grown a character at a time overshoots. That is
the first fall in `compile_peak_bytes` since #1152, and it was not the point of
the change.

**This is not #1159's declined ascii scanner.** That one made the whole scan
byte-oriented, measured 53,017 instructions by the time it was queued, and cost
212 peak bytes. This changes one line of `lex_word` and leaves the scanner
indexing characters everywhere else.

**Gavel #159 would delete this.** The `String` this builds more cheaply is the
one an inline name removes; if that ruling goes the inline way, `lex_word`
stops building a `String` at all and 407,394 instructions become moot along
with the 3,157 allocations beneath them. Recorded so the next reader knows the
two are the same code and not a contradiction: how a `String` is filled is a
question the compiler can answer today, and whether it exists is Clay's.

### The runner's number

    compile_instructions  42,748,207 -> 42,297,878  -450,329  -1.05%  (runner)
                          43,138,593 -> 42,731,199  -407,394  -0.94%  (local)
    compile_peak_bytes       730,120 ->    728,030    -2,090   both hosts
    compile_allocs                        29,864   unmoved, both hosts

Copied out of job 99320841238. Ten per cent apart on the instruction fall, the
widest the two hosts have been on this vein — a memcpy against a per-character
encode loop is the kind of trade two machines price differently, where a malloc
removed is the kind they agree on. The peak row agrees to the byte, as it always
has.

Welfare 87.4740 -> 87.4996, banked with `--set`. The page's `data-golden`
instruction figure moves with it; the allocation figure does not, because
allocations did not.

## 2026-08-31 — a character count is a population count

`length` on a string counts characters, and `k_str_chars` counted them by
walking: read a byte, decide from it how wide the character is, step that far,
add one. A branch per byte, about 8.9 instructions each on this box.

The shape that pays for it is asking a document its length. encodebench's
harness does exactly that — `rounds v (n - 1) (acc + length (encode v))` —
four hundred times over 188,698 bytes, and the cost golden had the numbers all
along: `str_scans=400`, `str_scan_bytes=75479200`. That one call was
671,356,000 instructions, 6.8% of the benchmark, and it read as
`k_b_length` in the profile rather than as anything the encoder does.

A character's first byte is any byte whose top two bits are not `10`, so the
count is the byte length minus the number of continuation bytes and nothing
has to be decoded. Eight bytes at a time: load a word, mark the bytes with bit
7 set and bit 6 clear, population count. About one instruction per byte.

```
    encodebench  9,866,614,705 -> 9,254,332,066  -612,282,639  -6.206%   (local)
    pendbench      988,282,947 ->   957,236,125   -31,046,822  -3.141%
    oneshot         47,277,156 ->    45,769,889    -1,507,267  -3.188%
    basket          57,392,199 ->    57,117,622      -274,577  -0.478%
    jsonbench    2,910,241,528 -> 2,913,835,115    +3,593,587  +0.123%
    widebench       83,967,604 ->    84,031,191       +63,587  +0.076%
```

**The two rises are one number.** jsonbench makes 898,500 `k_str_chars` calls,
all from `k_b_slice`'s test for whether a string is one byte per character,
and each costs +4.000 instructions — 898,500 × 4 is the whole 3,593,587 to the
unit. The strings are shorter than eight bytes, so the word loop never runs
and they pay only for its guard and the closing subtraction. Traded knowingly:
the four benchmarks that improved are the ones that ask a document its length,
and the decoder's four instructions buy the encoder six per cent.

Three shapes were measured, not two. A plain branch-free byte loop —
`conts += (p[i] & 0xc0) == 0x80`, no word at all — leaves jsonbench exactly
where it was and gives encodebench only 3.55%; clang does not vectorise it,
so it lands at 4.3 instructions per byte instead of 8.9. Disabling
vectorisation on the *word* loop costs 186,305,200 instructions on
encodebench, which is what the SIMD popcount is worth, so it stays. The tail
loop is told not to vectorise: it trips at most seven times, and clang's
widening of it was 368 bytes of prologue per copy against a loop the processor
barely enters.

That last one is the text vein's whole movement: +752 bytes wherever the
function is inlined twice, +368 where once, +1,520 on basket, which holds four
copies. Regenerated with the reason in the file.

**The harness came second, and says so.** `scripts/utf8_differential` already
extracts the validator's text from `src/runtime.c` at run time rather than
copying it; it extracts `k_utf8_chars` the same way now, under a name of its
own, and checks it against a reference in `scripts/utf8/harness.c` that walks
the text a character at a time from the rfc 3629 widths. Two routes to the
same answer on valid utf-8. The sweep is every arrangement of character widths
that fits in twenty-four bytes — three whole words plus the tails either side
of each — and then two hundred thousand valid strings over code points the
four representatives do not reach: 8,346,016 counts, 0 mismatches.

It was watched red before it was believed. Writing the mask as
`w & ~(w >> 1)` gives 7,036,227 mismatches; stopping the tail one byte short
gives 2,079,600. Both are the mistakes actually available in this function.

The counter this kernel is present by is `str_scans` / `str_scan_bytes`, which
already existed and did not move — they count scans and bytes, never words or
lanes, so they say the same thing on a host with no popcount instruction.

`k_chars_list` twenty lines above does the same scan to size the list it is
about to build. Left alone deliberately: it allocates a string per character
straight afterwards, so the count is not what that path pays for.

Not measured, and worth saying: the interpreter counts characters in Rust and
is untouched, so this moves native only. Whether the same shape helps the
front end is a separate question — the compiler asks for character counts too,
and `compile_instructions` will say.

DONE.

## 2026-08-31 — a one-byte append writes the byte

Same PR, and the second time in an hour that a byte-level operation in the
runtime turned out to be paying a call to move less than a register's worth.

`k_b_append_mut` is 23.24% of encodebench on its own: 42,318,000 calls at 54.2
instructions each, and 48,945,694 calls into `__memcpy_avx_unaligned_erms`
beneath it at 17.6 apiece. The encoder's commonest append is one byte — a
comma between elements, a colon between key and value, the brace that opens a
map — and `elem_onto` writes it as `text/append acc 44`. Moving one byte
through libc's dispatcher costs more than the move.

```c
if (n == 1) ((unsigned char*)a->data)[a->len] = *src;
else memcpy((unsigned char*)a->data + a->len, src, (size_t)n);
```

```
    jsonbench    2,913,835,115 -> 2,862,072,365   -51,762,750  -1.777%
    encodebench  9,254,332,066 -> 8,715,312,466  -539,019,600  -5.824%
    oneshot         45,769,889 ->    44,077,255    -1,692,634  -3.698%
    widebench       84,031,191 ->    84,047,191       +16,000  +0.019%
```

The other four rows do not move. **This pays back the character count's
decoder cost and then some**: jsonbench ends the pair at −48,169,163 against
main, −1.655%, where the counter alone had left it +0.123%. The two changes
land together for that reason.

The obvious generalisation is worse. A byte loop for every `n <= 8` — which
would also catch `"true"`, `"null"` and `"[]"` — reads encodebench at
8,878,376,066, jsonbench at 2,874,946,565 and oneshot at 44,570,742: worse
than the single byte on all three, because clang's memcpy for a known-small
dynamic length beats an explicit loop everywhere except at one. Measured, not
assumed.

+32 bytes of text wherever it lands, which is one compare and one store
against the call it replaces.

**And a refuted one, worth writing down because it looks free.** Two counters
on this fast path — `k_stat_append_fast++` and `k_stat_append_grow++` — are
incremented unconditionally, where most of the runtime's statistics sit behind
`__builtin_expect(k_stats_on > 0, 0)`. Putting them behind the same guard
costs +42,323,600 on encodebench and +4,016,400 on jsonbench: exactly +1.0001
instructions per fast append. Loading the flag and branching on it is one
instruction more than incrementing a counter that is already in cache, so the
guard buys nothing and is not a saving waiting to be taken.

The thread the entry above left open is closed too. The front end asks for
character counts as well — `trimmed.chars().count()` runs on every line of
every source file for the eighty-column cap — but rust's std already counts
this way: `core::str::count::do_count_chars` and `char_count_general_case`
together are 99,709 instructions of the front end, 0.23%. Nothing to take.

DONE.

## 2026-08-31 — the runner's numbers for the pair, and what welfare made of them

```
    work_jsonbench    2,910,241,528 -> 2,862,072,778   -48,168,750   -1.655%
    work_encodebench  9,866,614,705 -> 8,715,312,865 -1,151,301,840  -11.669%
    work_oneshot         47,277,156 ->    44,077,654    -3,199,502   -6.767%
    work_pendbench      988,282,947 ->   957,236,511   -31,046,436   -3.141%
    work_basket          57,392,199 ->    57,118,035      -274,164   -0.478%
    work_widebench       83,967,604 ->    84,047,604       +80,000   +0.095%
    work_deepbench, work_escapebench                            unmoved
    compile_instructions 42,297,878 ->    42,297,900          +22
```

**work_widebench is the one rise, and it is the character count's four
instructions.** widebench asks for a string's length on strings shorter than
eight bytes, so every call pays for the word loop's guard and its closing
subtraction and enters neither; nothing in that benchmark appends a single
byte, so the second change has nothing to give it back. jsonbench pays the
same +4 on 898,500 calls and takes 51.8M back from the append, which is why
the two shipped together and widebench is left where it is.

The local box reads deepbench −3,680 and escapebench −399 where the runner
reads neither, and its glibc is 2.39-0ubuntu8.7 against the golden's 8.8. The
runner's rows are the ones in the file.

`compile_instructions` moves 22 and none of it is the front end. `k_utf8_chars`
is twenty lines added to `src/runtime.c`; `main.rs` holds that file as an
`include_str!` and hashes it for the build cache key, and a longer string takes
twenty-two more instructions to hash. Allocations, peak, rounds and visits are
byte-identical.

**Welfare moved 87.50170 to 87.51104 — nine thousandths.** Worth writing down
because the size is surprising: #1170 bought 0.0277 for a 1.05% fall in
compile instructions, and this bought a third of that for an 11.7% fall in
encode. Two things make it so. Runtime satiates at 2.0, so a term already
better than baseline gains little from getting better still — encode crosses
from 5.9% below to 6.5% above and the crossing itself is most of the value.
And the five runtime speed counters share a weight with wide, deep and pending,
whose ratios are 138, 29 and 28 times baseline and therefore contribute almost
their whole allowance already. The model is doing what it was written to do.
Nothing is being argued here; the observation is that the encoder had a lot of
room and the index says the project had already been paid for most of it.

DONE.

## 2026-08-31 — the string index gets the cursor the slice already had

`s[i]` on a string counts characters, so finding the ith one means walking the
text until you have passed i characters. `k_b_at` started that walk at byte
zero every time, which makes reading a string one character at a time
quadratic in its length:

```
    2,000 characters    27,591,142 instructions
    4,000 characters   106,971,148            a doubling for 3.88x the work
```

`k_b_slice` solved this and left the comment saying so — "starting the walk at
zero every time is what made reading a page one character at a time quadratic"
— and it has two shortcuts for it. A string whose characters are all one byte
skips the walk entirely, because then a position is an offset. Anything wider
uses a remembered cursor: one string, one character index, one byte offset, so
the next question resumes where the last one stopped rather than from the
front, walking backward to it when the question moves back.

`k_b_at` had neither. The two are now `k_str_seek`, which the index calls and
the slice does not: the slice needs both ends of a run out of one pass and
would cross the text twice if it went through here. They share the cursor,
which is the part that matters — a sweep alternating `s[i]` and `text/slice`
over one subject stays linear because both hands move it.

```
    2,000 characters     1,690,119   -93.9%
    4,000 characters     3,169,125   -97.0%    a doubling for 1.875x the work
```

**The corpus had no home for this, which is why it survived.** Two were added.

`bench/indexbench` reads a string of twenty thousand characters forward, one
index at a time, over an alphabet mixing one-, two-, three- and four-byte
characters. It reads 5,266,521 instructions here and 2,570,995,073 on the
unfixed runtime — 488 times. No other benchmark asks `s[i]` of multibyte text;
they all read through slices or byte scans, which is precisely why nothing
went red for a year.

`tests/golden/micro/a_string_index_counts_characters` is the behaviour, on
both engines: forward through the string, backward through it, both ends, past
both ends, and an ascii subject for the direct path. It was watched red twice.
Resuming at `k_seek_char - 1` instead of `k_seek_char` — the off-by-one this
kind of cursor invites — turns `aé😀b€z` into `a 😀 € <none>` and the reversed
walk into `<none><none><none>zzz`. Writing the ascii bound as `from >= s->len`
loses the last character of an ascii string. Both are mistakes available in
these fifteen lines.

**What it costs, by the name each vein watches it under.** `text` rises 704
bytes in seven binaries and 688 in deepbench, which is `k_str_seek`;
escapebench indexes nothing, so the linker drops the function and its row does
not move. `emitted_other_defines`, `emitted_other_calls`,
`emitted_other_branches` and `emitted_other_lines` all rise because
indexbench's own 35, 159, 76 and 1,104 join a file that is a sum over
programs — a new benchmark is arithmetic there, not a regression, and the four
totals will rise again the next time one is added.

The instruction rows FALL slightly across the board — encodebench 7,237,599
(0.083%), which is 1.0001 instructions for each of its 7,237,200 `k_b_at`
calls: the list and bytes branches got a smaller function to sit in when the
string walk moved out of line.

**Not fixed, and worth writing down.** The cursor is compared by pointer, and a
`KStr` freed and reallocated at the same address with `cap != 0` would match
it. The guard added here is `k_seek_byte < s->len`, which turns the dangerous
case into a miss rather than a wrong character, but it is not a proof. The
same exposure has been in `k_b_slice` since the cursor landed. Closing it
properly means invalidating the cursor when a string dies, and that is a
separate change with its own reasoning.

DONE.

## 2026-08-31 — the append's grow path, built and HELD by the welfare number

Built, measured, and taken back out of the branch it was written on, because
the project's own scalar prices it negative. Recorded here so it is not
rediscovered.

`k_b_append_mut` is 26.5% of encodebench, and the first eight instructions of
every call were six callee-saved pushes and a frame:

```
    push %rbp / %r15 / %r14 / %r13 / %r12 / %rbx
    sub  $0x28,%rsp
```

One function holds both the frontier check and the growth. The fast path — two
failure tests, a tag dispatch, a capacity comparison, a store — needs two
registers; the growth needs eight, because it decides which allocator the new
buffer comes from, copies twice and frees the old header. Every append pays
for both. A `noinline` `k_b_append_grow` taking the six things it needs
reduces the prologue to three pushes.

```
    work_encodebench  8,708,075,266 -> 8,400,334,065  -314,978,800  -3.614%   (runner)
    work_jsonbench    2,862,072,778 -> 2,841,643,378   -20,429,400  -0.714%
    work_oneshot         44,077,654 ->    43,154,011      -923,643  -2.096%
    work_widebench       84,047,604 ->    84,271,604      +224,000  +0.267%
```

**widebench is why it is held, and the number is exact.** Its counters read
`append_fast=16000` and `append_grow=16000` — the only benchmark in the tree
where half the appends grow — and 224,000 over 16,000 grows is 14.0
instructions each, which is the call frame the grow path now enters. Every
other benchmark grows a handful of times against millions of fast appends, so
they take the three fewer pushes and pay nothing for the call.

The index disagrees with the arithmetic that looks obvious:

```
    floor after #1171                              87.511035
    the string index alone                         87.511654    +0.000619
    the string index and the outlining together    87.508343    -0.002692
```

Bisected by holding one row at a time. **widebench's 0.267% rise costs 0.003311
points on its own, where encodebench's 3.6% fall, jsonbench's 0.7%, oneshot's
2.1% and basket's 0.06% together earn 0.000619.** So the package falls below
the floor and the rule is unambiguous: the sum is the objective, a fall means
the change is worse by the weights as written, and the floor does not move to
accommodate it.

**What is worth asking, and is Clay's to answer, is whether the weights are
right about this.** wide_instructions sits at 138 times its baseline. A term
that far out contributes nearly its whole allowance already, so on the shape
of the curve a small move there should be worth almost nothing — and it is
worth five times what an eleven per cent improvement in encode was worth an
hour earlier. Either the model means that and the reason is worth writing down
beside the weights, or one benchmark 138x better than its baseline has more
leverage over the score than the two rows the front page makes claims about.
Filed in design/pending-gavels.md.

The change itself is small and its measurements are above; whoever picks it up
does not have to find it again.

HELD.

## 2026-08-31 — the runner's numbers for the string index

```
    work_encodebench  8,715,312,865 -> 8,708,075,665    -7,237,200   -0.083%
    work_basket          57,118,035 ->    57,083,993       -34,042   -0.060%
    work_oneshot         44,077,654 ->    44,059,561       -18,093   -0.041%
    work_widebench       84,047,604 ->    84,031,604       -16,000   -0.019%
    work_jsonbench, deepbench, escapebench, pendbench           unmoved
    work_indexbench                        5,266,934   new
    compile_instructions 42,297,900 ->    42,299,530        +1,630   +0.004%
```

Nothing in the work vein rises. The four falls are one number counted four
ways: `k_b_at`'s list and bytes branches got a smaller function to sit in once
the string walk moved out to `k_str_seek`, and encodebench's 7,237,200 over
its 7,237,200 `k_b_at` calls is 1.0001 instructions each.

**work_indexbench is new**, so it reads as a rise against a file that did not
have the row. It is the benchmark this change adds, at 5,266,934 — against
2,570,995,073 on the unfixed runtime, which is the 488 times the fix is worth.
A new row in a summed vein is arithmetic rather than a regression, and the
same will be true of the next benchmark somebody adds.

**compile_instructions rises 1,630 and none of it is the front end.**
`k_str_seek` adds lines to `src/runtime.c`, which `main.rs` holds as an
`include_str!` and hashes for the build cache key; a longer string takes
longer to hash. `compile_allocs`, `compile_peak_bytes`, rounds and visits are
byte-identical.

Welfare 87.511035 -> 87.511217, recorded with `--set`, and the page's tagged
compile-instructions figure follows the golden.

DONE.

## 2026-08-31 — the seek cursor outlived its string, and the engines disagreed

A differential-law violation, in shipped code, found while looking for one.

```
    fn shaped n 0 / "éé{n}ééééé"
    fn shaped n _ / "abcdefg{n}é"
    fn read s     / "{s[5]}{s[3]}/{text/slice s 5 5}{text/slice s 3 3}"

    interpreter   [abcdefg3é=>ec/ec]
    native        [abcdefg3é=>ge/ge]
```

Character 5 of `abcdefg3é` is `e`. Native answered `g`, which is character 7,
through both readers.

`k_seek_str` / `k_seek_char` / `k_seek_byte` are one remembered position, so a
forward sweep over a string resumes rather than restarting at the front. The
cursor names its string by address. `k_alloc` is a bump arena and
`k_beat_rewind` moves the frontier back, so the next string built in a loop
sits exactly where the last one sat — and inherits a byte offset that was true
of a string that no longer exists.

The comment above the cursor said a reset was unnecessary, on the grounds that
"a string whose bytes changed is a builder, which never qualifies". The string
whose bytes change across a rewind is not a builder. It is somebody else,
wearing the address. That sentence has been there since the cursor landed and
was the whole defence.

**Pre-existing, and not what #1172 introduced.** `text/slice` reproduces it
identically and has had the cursor since it landed; #1172 gave `s[i]` the same
cursor, which widened the exposure to the commonest way to read a character
but did not create it. Both readers are in the fixture for that reason.

The fix is one store in `k_beat_rewind`, which is the only place the arena
frontier moves backward — `k_beat_iter`, `k_beat_pop` and the carry paths all
reach it. Every address above the mark is free to be handed out again, so the
cursor is forgotten.

**What correctness cost**, and the rule says to pay it: welfare does not weigh
a change that makes the engines agree, so this ships and the floor moves to
whatever it costs.

```
    work_basket          57,083,993 ->    57,197,600   +113,607   +0.199%   (local)
    work_encodebench  8,708,075,665 -> 8,713,107,668  +5,032,003   +0.058%
    work_oneshot         44,059,561 ->    44,071,744    +12,183   +0.028%
    the other six                                    under a ten-thousandth
    text                                     +16 bytes in every binary
```

The rise is not the store. It is the cursor hits that stop happening: a sweep
that crossed a beat boundary used to resume and now restarts at the front.
That is the cost of the answer being right.

**A narrower reset was considered and not taken.** Keeping the cursor when the
remembered string lies below the mark and inside the surviving block would
recover some of it, but it needs the string's address checked against a block
whose own bounds are moving, and getting that wrong reintroduces exactly this
bug. The blunt version is provably correct and the measurement above is what
it costs. If that ever matters, the narrower test is the thing to build, with
this fixture already in place to catch it.

`tests/golden/micro/a_seek_cursor_does_not_outlive_its_string` is the pin, and
the harness runs it on both engines. Removing the store turns `ec/ec` into
`ge/ge` on native while the interpreter holds, which is the failure that
started this.

DONE.

## 2026-08-31 — the runner's numbers for the cursor fix, and a wrong note corrected

```
    work_basket          57,083,993 ->    57,198,013   +114,020   +0.1997%
    work_encodebench  8,708,075,665 -> 8,713,108,067  +5,032,402   +0.0578%
    work_oneshot         44,059,561 ->    44,072,143    +12,582   +0.0286%
    work_deepbench      760,475,193 ->   760,475,924       +731   +0.0001%
    work_jsonbench    2,862,072,778 -> 2,862,072,931       +153   +0.0000%
    work_pendbench      957,236,511 ->   957,236,647       +136   +0.0000%
    work_widebench       84,031,604 ->    84,031,648        +44   +0.0001%
    work_indexbench       5,266,934 ->     5,266,960        +26   +0.0005%
    work_escapebench                                    unmoved
    text                    716,626 ->       716,754      +128
    compile_instructions 42,299,530 ->    42,297,011     -2,519
```

Every work row rises except escapebench, and the rise is not the store. It is
the cursor hits that stop happening: a sweep crossing a beat boundary used to
resume where it left off and now restarts at the front. basket carries most of
it because it sweeps text across beats; the five rows under a thousandth are
the store itself, and `text` is its sixteen bytes in each of eight binaries.

**Welfare rose, and the reason is not the fix.** 87.511217 -> 87.511303, all of
it from `compile_instructions` falling 2,519, which is layout. Recorded with
`--set` because the rule says a rise is held rather than banked, and recorded
here as a ratchet step to spend back when `src/runtime.c`'s length next moves.
The fix itself costs runtime instructions and would have been right to ship at
a fall: welfare does not weigh a change that makes the engines agree.

**A note from #1172 was wrong and is corrected in the golden.** It said
`compile_instructions` moves when `src/runtime.c` changes length because
`main.rs` holds the file as an `include_str!` and hashes it for the build cache
key. That hash is in `cached_program_binary`, which is on the run and build
paths; `kanso check lib/json` never reaches it. What moves is layout — the
static's length shifts what follows it — and the direction does not follow the
length: twenty added lines read +1,630 and 713 added bytes read −2,519.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are unmoved across
all three, which is what says the front end's work did not change.

DONE.

## 2026-08-31 — the welfare number's leverage sits in one benchmark, and the reason is where satiation is applied

The gavel filed an hour ago named the right question and got the mechanism
wrong. Its "why it happens" paragraph described a weighted ratio of costs, and
the script does not compute one. Corrected in design/pending-gavels.md, with
the arithmetic, and repeated here because the log is where the number's
history is read.

`dimension_score` averages the counters' ratios and satiates the average:

```
  mean = list/sum ratios / (1.0 * length ratios)
  satisfaction mean t.satiation * t.weight
```

Satiation therefore applies to no counter. A counter's raw ratio enters the
average linearly and without bound, so the counter furthest past its baseline
carries the term. Run speed at today's floor:

```
    wide_instructions        ratio  138.37     68.45% of the mean
    deep_instructions        ratio   29.88     14.78%
    pending_instructions     ratio   29.22     14.45%
    oneshot_instructions     ratio    1.46      0.72%
    decode_instructions      ratio    1.14      0.56%
    encode_instructions      ratio    1.07      0.53%
    basket_instructions      ratio    0.99      0.49%
```

The model that produced those shares reads the live goldens and returns
87.511303209 against a recorded floor of 87.511303209, so it is the script's
arithmetic rather than a description of it.

**A second change is held behind the gavel.** `k_b_append_into` wrote its grow
path twice — once taking arena storage when the header dies at the innermost
rewind, once taking malloc — and the two differed in the allocator and in the
sign of the cap and in nothing else. One tail serves both. Measured on this
container, same sitting, callgrind, nine benchmarks:

```
    encodebench  8,713,107,668 -> 8,670,774,068   -42,333,600   -0.4859%
    jsonbench    2,862,072,518 -> 2,860,449,668    -1,622,850   -0.0567%
    oneshot         44,071,744 ->    43,955,091      -116,653   -0.2647%
    widebench       84,031,235 ->    84,111,235       +80,000   +0.0952%
    basket, deepbench, escapebench, pendbench, indexbench: identical
```

widebench's 80,000 is 5 instructions across its 16,000 grows. It is the only
benchmark in the tree where half the appends grow, because
`text/append (text/bytes lead) "  "` builds a fresh accumulator per element
and the first append onto one has no spare capacity to write into.

Priced two ways:

```
    as written    (average the ratios, then satiate)   -0.001096   held
    per counter   (satiate each ratio, then average)   +0.008015   ships
```

The sign turns on the order of those two operations. Under the index as
written the rule is unambiguous and the change is held: the sum is the
objective and it falls. What it does is written above in enough detail to
redo — hoist the allocator choice above the copy, keep one tail, carry the
regime in the sign of a local — and it is thirty fewer lines than what it
replaces.

HELD.

## 2026-08-31 — a deliberate exit's two tests staged one file

Found while running the suite for something else: `cargo test --release` went
red on `the_interpreter_agrees`, which asserted the shell saw 3 and got 2.

Two is what the compiler exits with. Both tests in
tests/a_deliberate_exit_carries_its_code.rs stage their fixture at
`/tmp/kanso-deliberate-exit/main.kso`, cargo runs them on separate threads, and
`fs::write` truncates before it writes — so one test could read the file while
the other was inside that call and compile an empty program.

Measured back to back, twenty runs each: two failures before, none after. That
is often enough to redden an unrelated pull request and rare enough to be read
as something else, which is how it survived. kanso#1169 fixed the same shape in
the playground tests and this pair was missed.

One directory per test, named by the caller.

DONE.

## 2026-08-31 — the beat sweep: every layout pair, every hand that moves the cursor

kanso#1173 fixed a cursor that outlived its string across a beat's rewind and
shipped one golden pinning one shape of it. One shape is what the bug happened
to take, so the sweep here asks the whole family instead.

`scripts/beat_differential` builds a program per case: a beat that rebuilds a
string every iteration, alternating between two byte layouts, and reads it with
one hand. Four layouts — all ascii, all two-byte, all four-byte, and one that
changes width inside a single string — crossed with themselves and with six
hands: forward indexing, backward indexing, forward slicing, backward slicing,
a hand that mixes index and slice, and one that asks `length` between reads.
Ninety-six cases, both engines, ten seconds.

Alternation is what makes a case sharp. Two iterations of the same layout put
the next string's characters at the byte offsets the last one used, so a stale
cursor is accidentally right; two layouts that disagree about where character
five begins make it wrong.

**Watched fail, for the right reason.** With `k_seek_str = NULL` deleted from
`k_beat_rewind` — the whole of #1173's fix, and now
`scripts/ratchet/mutations/a_cursor_that_outlives_its_string.sh` — sixteen of
the ninety-six disagree. Some of them answer a character from the wrong
position; some answer a byte from the middle of a codepoint, which the terminal
renders as U+FFFD. With the store restored, all ninety-six agree.

Two sweeps ran on the way and are recorded because they say what the fix
covers: thirty-six single-layout programs and ninety alternating-layout ones,
both zero disagreements against the fixed runtime.

The row is `beat_cursor` in scripts/ratchet/ratchet.kso and the step runs in
the diagnostics-differential job beside the eight sweeps already there.

DONE.

## 2026-08-31 — two things the wide benchmark paid for and did not need

widebench had never been profiled. It carries 68.45% of welfare's run-speed
mean, so where its time goes decides most of what the index can be moved by.
Two answers came out of one profile.

**A whole number rendered as a float went through glibc's multiprecision
formatter.** 11.0% of widebench was inside three libc frames:

```
     5,661,266  6.74%  __printf_fp_buffer_1
     2,192,140  2.61%  hack_digit
     1,424,000  1.69%  __printf_buffer
```

The caller is `k_render_at`. A double that is a whole number takes a branch
guarded by `d == floor(d) && fabs(d) < 1e15 && isfinite(d)`, and that branch
called `snprintf("%.1f")`. `%f` reaches `__printf_fp`, which is multiprecision
— it also pulls `__mpn_divrem` and `__mpn_mul_1`, which `k_b_to_float`'s strtod
shares. `render_ryu`'s `d == 0.0` branch made the same call.

The guard has already proved the number is an integer under 1e15, so the cast
to `long long` is exact, the digits are an integer's digits, and the fraction is
exactly `.0`. `k_itoa` writes it. Negative zero is the one value the cast loses,
and `signbit` keeps it.

**The utf-8 validator paid full vector setup for a forty-one byte token.**
`k_utf8_bad` was 4.15% of jsonbench. Counting the calls in the callgrind file
rather than guessing: 265,950 calls carrying 10,975,500 bytes, so 41.3 bytes a
call and 446 instructions a call — about eleven instructions a byte, where a
vectorized validator on a long input runs at a fraction of one. The setup is
why: seven constant loads and two zeroed accumulators before the first block.
The old short-circuit only fired below sixteen bytes.

Eight bytes at a time answers the whole question for an ascii run of any
length and never reaches the vector pass. A run that is not ascii falls
through and the wide pass reads it from the start, so the scan is the only
waste.

The runner's rows, which are the ones the golden carries. Every delta matches
what this container measured to the instruction; only the absolute values
differ, because the two hosts run different glibc:

```
    widebench       84,031,648 ->    67,553,998  -16,477,650   -19.6090%
    basket          57,198,013 ->    56,781,353     -416,660    -0.7285%
    jsonbench    2,862,072,931 -> 2,860,868,131   -1,204,800    -0.0421%
    oneshot         44,072,143 ->    44,066,375       -5,768    -0.0131%
    encodebench  8,713,108,067 -> 8,714,005,635     +897,568    +0.0103%
    deepbench, escapebench, pendbench, indexbench: identical
```

`compile_instructions` rises 678, and it is layout for the third time this
week: `include_str!("runtime.c")` changed length, so what follows it in the
binary moved. compile_allocs, compile_peak_bytes, rounds and visits are
byte-identical.

The floor is 87.773793, up 0.262490 — the largest single rise the index has
taken.

Held apart, so the two are separable: the float render alone is widebench
-19.019% and welfare +0.253991; the ascii pre-scan alone is basket -0.7241%,
widebench -0.5900%, jsonbench -0.0421% and welfare +0.007426.

`work_encodebench` rises 897,568, which is 1.003 instructions per ryu_render
and no counter encodebench owns moves; it is layout, and it is under a
hundredth of a per cent.

`text` rises, and it is bought rather than explained away: 128 bytes of .text
on the five benchmarks that link both changed functions, 80 to 96 on the four
that link a subset because the linker drops what they never call. That is the
integer path and the word-wise scan against two `snprintf` call sites removed,
and it buys widebench 19.6%. Every allocation counter is byte-identical — all
eight counter gates and the emitted-line gate pass untouched — which is what
says the machine code is the only dimension that moved.

Removing `"%.1f"` made a ratchet mutation stale, which is the check doing its
job: `native_renders_a_float_wider` widened that format to `"%.2f"` so native
printed `1.00` where the oracle printed `1.0`. There is no format to widen now,
so it breaks the cast instead — `(long long)d` becomes `(long long)d * 10` and
native prints `10.0`. Watched red before it was believed: the render sweep
reports `native: 10.0 / interp: 1.0` under the mutation and 86 values, 0
disagree, without it.

Byte-identical on both engines across the float edges — 0.0, -0.0, ±1.0,
±42.0, ±1e14, 999999999999999.0, the 1e15 boundary where the exponent form
takes over, 1e-07, and `0.0 * -1.0`. Pinned in
tests/golden/micro/a_whole_float_keeps_its_point.kso, which goes red on the
mutation that drops the `signbit` test: three of its `-0.0`s render as `0.0`.
All eight differential sweeps agree, the utf-8 one included.

**What is left in widebench, for whoever picks it up.** The carry evacuation:
`k_survives_x` at 7,522,206, `k_copy_size` at 8,098,822 across both frames,
`k_ptrmap_at` at 2,724,570 and `k_deep_copy` at 1,695,365 — about 29% of what
this leaves. `k_survives_x` is not slow per call and the chain is short
(widebench holds two arena blocks); it is called roughly 375,000 times, because
the evacuation walks the carried document every iteration. That is a design
question about the carry rather than a peephole.

DONE.

## 2026-08-31 — a third staging collision, and this one hid behind a name that looked unique

kanso#1175's flake was found by a suite run going red for reasons that had
nothing to do with what was being measured. Two more runs of the same suite
named the rest of it: `a_list_that_was_never_bytes_reads_the_same_on_both_engines`
and `genuine_bytes_already_read_the_same_on_both_engines`, together, in one run
of three.

tests/a_bare_list_is_or_is_not_bytes.rs stages its program in a directory named
by the expression's LENGTH. Two pairs collide across the file's two tests, which
cargo runs on separate threads:

```
    text/to_float [1]                and  text/utf8 [97 98]                  17
    text/find2_below [97] 0 97 98 1  and  text/to_float (text/bytes "ab")    31
```

Each call writes `run.kso` into that directory and calls `remove_dir_all` when
it is done, so one test deletes the other's program between the write and the
run. Twenty runs each on a settled tree: five failures before, none after.

A hash of the expression names the directory now.

**The rule this is the third instance of.** kanso#1169 fixed it in the
playground tests and kanso#1175 in the deliberate-exit pair, both where the
name was a constant. Here the name was derived and looked unique, which is why
neither of those fixes covered it: a staging path has to be keyed by something
injective over everything that can reach it, and `len` is not. Every test
binary in the tree was then run repeatedly to find the rest by failure rather
than by reading, because reading is what missed this one twice.

DONE.

## 2026-08-31 — the tenure test answered for objects, and the question was about regions

kanso#1177 left a note naming the carry evacuation as what remained in
widebench: `k_survives_x` at 7,522,206 instructions, about 29% of the
benchmark. Instrumenting it said the shape was nothing like the guess in that
note. The arena walk is one step every time — widebench holds a single block —
so `k_survives` is already O(1) and costs nothing. Of 120,418 asks, 16,079 are
answered by the arena and 104,339 fall through to the tenure test, and **80,415
of those, 83%, are the tenure test proving a pointer is NOT tenured.** The cost
was the hash probe on the miss, at roughly 40 instructions a time.

An address-span filter — reject anything outside [lowest tenured byte, highest]
before probing — rejected 1 ask of 96,496. The two tenured blocks sit 45 TB
apart, because glibc mmaps one and takes the other off the heap, and the arena
lands between them.

**Two blocks.** That is the whole finding. The block list is short, so the
membership question can be a walk, and a walk answers on ranges where a hash
answers on allocation bases. `k_ten_alloc` now doubles each block instead of
taking a fixed 64 KiB, which bounds the list: K_TEN_CAP is 64 MiB, so doubling
from 64 KiB runs out of licence after eleven blocks and there is never a
twelfth. That bound is what pays for deleting the hash set, which cost a probe
on every ask AND an insert on every single tenured allocation.

A one-entry cache of the block that answered last was built and DROPPED. The
instrumentation said it would take 16,080 of widebench's 96,496 asks on its
own, and it does; it also costs the other 80,416 a load and two compares before
they walk anyway, and the walk is two blocks. Measured: 65,170,283 with the
cache against 64,535,358 without, so it cost widebench 634,925 instructions to
save 16,080 walks of length two. Dropping it also retires a second question —
the cache had to carry the beat depth it was filled at, because `k_beat_pop`
releases tenured storage on its way out but three other sites lower
`k_beat_depth` without releasing, and a cache naming a block the walk would no
longer reach would answer where the walk says no. That check was another
257,197 instructions on widebench. Neither is in the code.

Answering on ranges is where the behaviour changes. A list's
elements sit in a buffer the header points into, so the copy that tenured the
list put the header at an allocation start and the elements at an offset; the
hash answered no for the buffer and the next rewind evacuated all of it again.
`a_loop_invariant_capture_is_copied_every_rewind` goes 32,672 evacuated bytes
to 16,448 — the list stops travelling forward a second time. widebench's
`evac_bytes` goes 1,032,336 to 519,728 and its `beat_iters` 43 to 40, because
the size walk that decides whether a chain step stages or leaves its value now
sees the tenured storage as surviving and three steps come out under the
threshold.

    jsonbench     2,860,867,718 -> 2,858,844,840     -0.0707%
    encodebench   8,714,005,236 -> 8,704,347,511     -0.1108%
    oneshot          44,065,976 ->    44,000,222     -0.1492%
    basket           56,780,940 ->    56,457,649     -0.5694%
    widebench        67,553,585 ->    64,535,358     -4.4679%
    deepbench       760,472,244 ->   726,483,254     -4.4695%
    escapebench     258,582,701 ->   253,818,697     -1.8424%
    pendbench       957,236,261 ->   946,377,688     -1.1344%
    indexbench        5,266,547 ->     5,244,328     -0.4219%

Nine benchmarks, nine falls. Container numbers on both sides, measured the same
sitting; the runner's rows are in bench/instructions_golden.txt.

**Half of that is one attribute.** `k_ten_holds` is `noinline` now. Inlined, it
was pulled into `k_survives_x`, which is inlined into `k_born_this_beat`, which
is inlined into `k_b_push_mut` — so a program that never tenures still carried
the whole tenure test inside its hottest fast path. escapebench, deepbench and
jsonbench never call `k_ten_holds` at all: the symbol is absent from their
profiles, as it is from four of the other six — only widebench and indexbench
reach it. Their falls are entirely the fast path being compiled better once
the cold code is out of it. Measured separately: `noinline` alone on the
unchanged hash gives deepbench -4.44% and escapebench -1.84%, and the walk adds
the rest on the benchmarks that tenure. Without it the walk COST escapebench
1.38%, at exactly +3 instructions per push with every call count identical.

`text` rises 22,656 bytes over the nine binaries, between 2,432 and 2,624
each. The optimiser did that: `k_survives_x` shrinks 346 bytes to 118 once the
tenure test is out of line, and then inlines into more of its callers —
`k_copy_size` +1,911, `k_repair_interior` +800, `k_slots_survive` +267,
`k_interior_survives` +263, `k_deep_copy` +252, against `k_repair_size` -940
inlined away entirely. Bigger
code, fewer instructions executed, on every benchmark. Two of the nine reach
`k_ten_holds` at all — widebench and indexbench — so seven of these rows,
`basket`'s -0.57% among them, are the fast path alone.

**The archive declines positional membership by name, and this is not it.**
design/log/compiler-log-archive.md, under "Declined by measurement: the same
idea as an arena block": appending a KBlock at the tail of the arena chain was
built three times and reached 91,241,398,272 bytes of resident set. The reason
given is that positional membership recognises every pointer inside a block
where a hash recognises only bases, and a text accumulator handed through an
intermediate function is the shape that turns that into unbounded growth.

That variant put the answer in `k_survives` — the narrow walk the mutation fast
paths ask per append. This one is only in `k_survives_x`, which the copy
machinery and `k_born_this_beat` ask and no mutation path does. The split
between the two was made in that same archived entry, for this reason. Checked
rather than argued:

- `book_panels --write` runs clean under `ulimit -v 8000000`, rewriting 0
  panels. The declined variant killed it.
- `effect_push_shape.mem`, the fixture the declined variant broke, does not
  move. One file in the 51-fixture mem corpus moves, and it is the carry one.
- Peak RSS across the nine benchmarks: -100 KB to +16 KB, widebench and
  deepbench byte-identical.
- The accumulator shape itself, built and run at n = 1,000 / 2,000 / 4,000 /
  8,000: 20,864 / 71,900 / 267,912 / 659,492 KB on main against 20,844 /
  71,916 / 267,876 / 659,560 with the change. The curve is superlinear and it
  is superlinear on both sides, unchanged to a tenth of a per cent.

Running the declined variant's own shape — the tenure answer moved back inside
`k_survives` — moves exactly one fixture in the mem corpus, the same carry one.
So the corpus can no longer see the thing that cost 91 GB. That is a gap, and
it is recorded rather than fixed here: an attempt to fill it with a text
accumulator crossing a beat was written, generated its goldens, and was DELETED
because it read identically on main, on this change, and under the declined
variant. A golden that cannot go red is worse than no golden.

**The runner's rows, and what the compile veins did.** Every delta below was
measured on the container first and reproduced on the runner to the
instruction; the absolute numbers differ by the usual few hundred, and the
subtractions do not.

    jsonbench     2,860,868,131 -> 2,858,845,253     -0.0707%
    encodebench   8,714,005,635 -> 8,704,347,910     -0.1108%
    oneshot          44,066,375 ->    44,000,621     -0.1492%
    basket           56,781,353 ->    56,458,062     -0.5693%
    widebench        67,553,998 ->    64,535,771     -4.4679%
    deepbench       760,475,924 ->   726,486,934     -4.4694%
    escapebench     258,583,100 ->   253,819,096     -1.8423%
    pendbench       957,236,647 ->   946,378,074     -1.1344%
    indexbench        5,266,960 ->     5,244,741     -0.4218%

`compile_instructions` rises 42,297,689 to 42,300,376, +2,687 and +0.0064%.
Nothing about compiling got harder: `compile_allocs` and `compile_peak_bytes`
are byte-identical at 29,864 and 728,030, the front end's rounds and visits do
not move, and the diff touches src/runtime.c and nothing the measured path
runs. `kanso check lib/json` compiles a library and executes no program, so
the runtime is not reached at all — but runtime.c arrives in the compiler
through `include_str!`, so changing its LENGTH rearranges the compiler's own
binary. This is the fourth instance this week and the sign does not track the
direction of the edit. It is the case design/pending-gavels.md asks about
under "Whether a compile_instructions move that cannot be work needs an
attribution", and the entry is unchanged by this: the vein earns its place,
and the question is only what a rise with every sibling counter identical
should have to buy.

Welfare 87.7738 to 87.8371, banked with `--set` in this PR.

All nine differential sweeps agree, the beat and utf-8 ones included. The whole
Rust suite passes. `evac_bytes` in two veins is what pins the walk: revert it
and both go red, which is how this was watched before it was believed.

**A fourth staging collision, folded in.** Running the suite to validate the
above turned it up, which is the point of running it repeatedly rather than
reading it. `tests/a_toolchain_the_path_cannot_reach.rs` staged both its tests
into `/tmp/kanso-no-clang`, and `fs::write` truncates before it writes: one
test emptied `main.kso` while the other test's `kanso` was reading it, so the
build reported `an entry file needs at least one statement` instead of the
message under test. One failure in twenty; none in forty with a directory per
test. kanso#1169, kanso#1175 and kanso#1177 are the same defect at three other
sites, and this is the second of those four found by a suite going red rather
than by anybody looking.

DONE.

## 2026-08-31 — the depth loop was already one iteration, and the block walk was two

kanso#1178 left `k_ten_holds` as its own symbol in widebench at 37.7
instructions an ask, and a task naming the outer loop over beat depths as where
that went. The estimate came from arithmetic on 44 instructions per call rather
than from a counter, and it was wrong.

Instrumented, widebench reads `calls=112,529 depth_iters=112,529
block_iters=208,982 max_beat_depth=1`. **One depth iteration per ask**, because
the beat depth never exceeds one on this program: there is no depth loop to
bound. indexbench, the only other benchmark that reaches the function, reads
`calls=89 depth_iters=178 max_beat_depth=2` — 89 asks is not a cost.

The 1.86 block iterations per ask are the cost. widebench tenures about 128 KiB,
which needed a second 64 KiB block, so 71% of asks — the misses — walked both.
Starting the doubling at 256 KiB holds it in one. 256 is the smallest power of
two that does, and nothing larger can help: the walk cannot go below the one
block it now takes, so a bigger base would buy nothing and cost peak to every
program that tenures at all.

    widebench     64,535,358 -> 63,756,786    -778,572   -1.2064%
    indexbench     5,244,328 ->  5,242,681      -1,647   -0.0314%

Every other benchmark moves by 0 or 14 instructions, which is nothing. All nine
counter gates are unmoved — no evacuation counter, no arena peak — and the
machine code is byte-identical on all nine binaries, because a constant is all
that changed. `k_ten_holds` costs 30.4 instructions an ask against 37.7, at the
same 112,529 calls, which is one block test removed and matches the total to
five per cent.

Peak resident set: widebench 3,624 KiB to 3,704, indexbench 2,416 to 2,452. The
block is mmap'd and its untouched tail never becomes resident, so what a
program that tenures nothing pays is zero — `k_ten_alloc` runs only when
something is promoted.

The bound the doubling exists for still holds. K_TEN_CAP is 64 MiB, so doubling
from 256 KiB runs out of licence after nine blocks where it took eleven from
64 KiB, and k_ten_holds' walk is shorter at every size.

**The idea that started this is declined.** Bounding the depth loop buys
nothing, because the loop is already one iteration on every program that
reaches it. Recorded so nobody prices it again from the same arithmetic: 37.7
instructions an ask is a call, a prologue, one depth iteration and two block
tests, and only the last of those was ever worth attacking.

DONE.
