# FRICTION: porting cmark to kanso

A journal kept while writing a CommonMark renderer in kanso. Entries are in
the order I hit them. Every code sample was compiled with the toolchain at
`/tmp/claude-0/kanso-main/kanso`.

## Entries

### F1: the book's `pub play =` entry does not run
- kind: tooling
- severity: minor
- where: scratch/t1.kso:7 (first experiment, since deleted)
- wanted: the book's chapter 04 and 05 samples end with
  `pub play = os/read_file! "version.txt" .> announce` and show
  `kanso run version.kso` running them. So I wrote a file with two functions
  and `pub play = print "..."`.
- wrote: `kanso run` answered "`scratch/t1.kso` is a library — nothing to
  run. it exports `play`: import the module from an entry file", and
  `kanso play` answered "`pub play` is a library's export — import this module
  from an entry file and name its `play`; `kanso play` takes bare statements".
  What runs is the same file with bare statements at the bottom and
  `kanso play`.
- why it matters: the first program a newcomer copies out of the book is
  refused by both verbs, each pointing at the other. The messages are clear
  about what to do; the book is what disagrees.

### F2: no collection literal may span lines
- kind: missing-feature
- severity: major
- where: scratch/t3.kso, t5.kso, t6.kso, t7.kso; the workaround is
  commonmark/entities.kso
- wanted: a table of the 2,125 HTML5 entity names, written as data:

      table = {
        "AElig":"Æ"
        "AMP":"&"
      }

- wrote: every multi-line form I tried is a syntax error. The map above gives
  `error[syntax]: expected `)`` at the first entry. A list with one element
  per line gives `expected an expression` at the end of the last element, and
  a list wrapped after its first element gives `expected `)`` at the closing
  bracket. With the 80-column limit, a literal can hold about four entries.
  The table became one binding with 2,125 pipe continuations:

      entity_table = {}
        . put "AElig" "Æ"
        . put "AMP" "&"

- why it matters: any program with a lookup table bigger than a line (entity
  names, keywords, opcode tables, test vectors) hits this. The pipe chain
  works and compiles fast, but it reads as code rather than data, and nobody
  would guess it. A one-element-per-line literal also gave the misleading
  hint "needless continuation: this statement fits on one line" before I made
  the elements longer, which suggests the form is legal when it is not.

### F3: string literals have no `\u` escape
- kind: missing-feature
- severity: minor
- where: scratch/t4.kso:15
- wanted: `. put "ApplyFunction" "\u{2061}"` for the invisible characters in
  the entity table (U+2061 FUNCTION APPLICATION, U+200B ZERO WIDTH SPACE).
- wrote: the raw invisible characters, pasted into the source between the
  quotes. `error[syntax]: unknown escape `\u`` is what the escape gets.
- why it matters: a reader cannot see what those literals hold, and the
  canonical-form rules that forbid invisible trailing whitespace are silent
  about invisible characters inside strings. `text/from_code 8289` works at
  run time, but not inside a table of literals.

### F4: a `_` arm does not cover none, so arms get written twice
- kind: confusing-semantics
- severity: minor
- where: commonmark/caret.kso (`skip_one`)
- wanted:

      fn skip_one _ at col _
        gap at col

  as the fallback for "anything that is not a space or a tab", including the
  end of the line, where `chars[at]` is none.
- wrote:

      fn skip_one _ at col none
        gap at col

      fn skip_one _ at col _
        gap at col

  The checker said: `error[exhaustive]: this can be a none and `skip_one` has
  no arm for it`.
- why it matters: a scanner reads past the end of its input all the time, so
  this pair of identical arms appears once per scanning function. The rule
  makes sense for err, which the book explains at length; for none, which the
  book calls "a shape dispatch fully understands", I expected the wildcard to
  take it.

### F5: an imported module's type names override my own
- kind: refactoring-hazard
- severity: major
- where: commonmark/blocks.kso (`passed_over`, first written as `skipped`)
- wanted: a marker of my own, declared in my own module:

      type skipped
        line

      fn quote_start x
        g = nonspace x.cur
        return skipped x if indented? x g or x.cur.chars[g.at] != ">"

- wrote: `type passed_over`. With `import "std/list"` in the same file, every
  use of `skipped` failed with
  `error[opacity]: `list/skipped` is foreign — only `list` builds a `skipped``.
  std/list exports types named `skipped`, `cursor`, `sorted`, `mapped`,
  `step`, `bounded`, `capped` and more, and the bare name resolves to the
  import rather than to the declaration three lines above.
- why it matters: the book says an imported pub "joins the short-name
  overload space", which reads as additive. Here it silently wins over a
  local declaration, and the message blames me for touching list's
  internals. A program that imports std/list cannot name a type `step` or
  `cursor` without learning list's private vocabulary first.

### F6: one namespace per module means a new type breaks other files' locals
- kind: refactoring-hazard
- severity: major
- where: every file in commonmark/; examples at inline.kso (`type plain`,
  `type delim`, `type image`), render.kso (`type out`, renamed `sink`),
  links.kso (`type found`)
- wanted: to add `type plain` (a text node) to inline.kso.
- wrote: adding it turned three bindings in blocks.kso into errors:
  `error[name]: `plain` is already a declaration; rename the binding` on
  `plain = not indented? x g and ...`. The same happened for `delim` (a
  parameter name in blocks.kso), `out` (a parameter in nine escape.kso
  functions), `found`, `rest`, `spaces`, `image` and `opened`. Each time the
  fix was to rename locals in files I was not working on.
- why it matters: the no-shadowing rule plus a module-wide namespace makes
  every type name a reserved word across every file of the module, including
  parameter names. Adding a type is a module-wide rename. The diagnostic
  names the binding but not the declaration it collides with, so finding
  where `found` was declared took a grep.

### F7: an err arm for an impossible argument spreads through every caller
- kind: confusing-semantics
- severity: major
- where: commonmark/escape.kso:`has?`, `swap`
- wanted:

      fn has? s piece
        length (text/split s piece) > 1

- wrote:

      fn has? s piece
        several? (text/split s piece)

      fn several? (err _)
        false

      fn several? parts
        length parts > 1

  and the same pair for `swap`. Before that, `kanso check` reported 34
  `error[exhaustive]: this can be an err and `add_child` has no arm for it`
  diagnostics in blocks.kso, pointing at `add_child`, `close_all`,
  `add_line`, none of which call split. std/text's split has an arm
  `pub fn split _ ""` that answers an err, so any call whose separator is not
  a literal can be an err, and that possibility rode up through `has?`,
  `unescape`, `take_refs`, `close_top` and everything above them.
- why it matters: the diagnostic appears far from the cause, and the fix is
  an arm for a case no caller can produce. A library's defensive err arm
  becomes every caller's obligation, transitively.

### F8: a `return ... if x == none` guard does not narrow x
- kind: missing-feature
- severity: major
- where: commonmark/inline.kso (`entity_found`, `dest_then`, `title_then`,
  `angle_uri`, `angle_mail`, `angle_tag`, `via_inline`, `via_label`,
  `linked`, `ref_found`), refs.kso (`ref_label`), escape.kso
  (`named_value`), html.kso (`blank_after_tag?`)
- wanted:

      fn entity_here cs p out refs
        ref = entity_at cs p
        return scan cs (p + 1) (push out (plain "&")) refs if ref == none
        scan cs ref.next (push out (plain ref.value)) refs

- wrote:

      fn entity_here cs p out refs
        entity_found cs p out refs (entity_at cs p)

      fn entity_found cs p out refs none
        scan cs (p + 1) (push out (plain "&")) refs

      fn entity_found cs _ out refs (entity next value)
        scan cs next (push out (plain value)) refs

- why it matters: the guard form is what the book teaches for early exits,
  and after it `ref` cannot be none. The checker still types `ref.next` as
  int-or-none, and because inference is whole-program, that none then
  surfaced as `this can be a none and `skip_spnl` has no arm for it` in
  unrelated callers of `skip_spnl`. Every optional result became a pair of
  helper functions; inline.kso has eleven such pairs. Each is fine on its
  own, but a three-step parse (destination, title, closing paren) becomes
  six functions.

### F9: every function handed an index result needs its own none arm
- kind: aesthetics
- severity: minor
- where: chars.kso (`space_or_tab?`, `punct?`, `white?`, `ascii_punct?`),
  escape.kso (`has?`, `alnum?`, `hex?`, `digit?`), html.kso (`letter?`,
  `attr_start?`, `value_end`, `tag_close`), links.kso (`link_space?`,
  `title_open`, `bare_step`), inline.kso (`bracket?`, `opener?`, `emph_at`,
  `pair_up`, `close_bracket`)
- wanted: `fn gap? a b` reading `a.end` and `b.start`, called as
  `gap? kids[i - 1] kids[i]` where i is always in range.
- wrote: `end_of` and `start_of` helpers with none arms, and the comparison
  inlined into the caller, because a none arm on `gap?` itself cannot be
  written for two positions without the arms tying. In total about thirty
  `fn x? none` arms whose only purpose is the end of a char list.
- why it matters: a scanner indexes past the end constantly. Pair this with
  F4 (the wildcard arm does not take none) and roughly one function in five
  in this port has an arm that exists to satisfy the checker.

### F10: no spelling of a list of held functions passes check in a module
- kind: engine-bug
- severity: minor
- where: bugs/partial_in_list/partial_in_list.kso; commonmark/blocks.kso
  `block_starts`
- wanted: `block_starts = [&quote_start &atx_start &fence_start]`, or
  `. push &quote_start` on continuation lines.
- wrote:

      block_starts = []
        . push (x -> quote_start x)
        . push (x -> atx_start x)

  Unparenthesised, `&atx_start` gets `canonical form requires exactly one
  space here`; parenthesised, `(&atx_start)` gets `these parentheses group
  nothing`. The same list passes under `kanso play`.
- why it matters: a table of handlers is the natural shape for "try these
  block starts in order", and the two formatting rules contradict each other.

### F11: `kanso check` on a module passes code that fails when imported
- kind: tooling
- severity: major
- where: commonmark/ (whole module), main.kso
- wanted: `kanso check commonmark` to say whether the module is legal.
- wrote: `kanso check commonmark` answered `commonmark: ok`, and then
  `kanso check main.kso`, whose only content is
  `io/stdin .> commonmark/to_html .> io/write`, reported seven
  `error[exhaustive]` diagnostics inside commonmark/inline.kso and refs.kso.
  Those diagnostics also came without the source excerpt and caret that
  every other diagnostic has, and each was printed twice.
- why it matters: I checked the module after every edit, as the book
  suggests, and learned about a family of errors only when an entry file
  instantiated it. A library author with no entry file of their own would
  ship the module as "ok".

### F12: natively, list/fold corrupts the accumulator it threads
- kind: engine-bug
- severity: blocker
- where: bugs/fold_corrupts_state/ (reduced to 353 lines); was
  commonmark/blocks.kso `parse_document` and render.kso `blocks_html`
- wanted:

      close_all (list/fold lines start parse_line) (length lines)

  and in the renderer

      fn blocks_html acc kids tight refs
        list/fold kids acc (a k -> block_html a k tight refs)

- wrote: hand-written loops:

      fn each_line st lines i
        return st if i > length lines
        each_line (parse_line st lines[i]) lines (i + 1)

- why it matters: with the folds, the interpreter passed every spec example
  and the native builds (dev and release) failed eight of them, four ways:
  a segfault, `error[runtime]: the program ran out of stack`,
  `` `list/broken_link` has no field `kind` `` and
  `` `-` is not defined for these values ``. A frame's integer `start`
  field came back holding `[list/broken_link []]`. The trigger was a list
  nested three deep, or two lists of two items separated by a blank line.
  Finding it took about two hours, most of it spent learning that the
  stack-overflow message was not about recursion depth. `list/fold` is the
  first verb the book teaches for walking a list, and the workaround is to
  stop using it.

### F13: 80 columns with no way to continue a string or a boolean
- kind: aesthetics
- severity: minor
- where: commonmark/html.kso (`block_tags`, `html_ends?`, `attrs_end`),
  inline.kso (`flanking?`, `code_close`), blocks.kso (`kids_tight?`,
  `try_starts`)
- wanted: the 62 HTML block tag names as one string split on spaces; the
  flanking rule written out as two lines,

      left = not white? after and (not punct? after or white? before or punct? before)
      right = not white? before and (not punct? before or white? after or punct? after)

- wrote: the tag names as a 62-line put chain into a map; the flanking rule
  as a helper `flanking? before after` called twice with the arguments
  swapped. That helper is an improvement. Most of the other forced splits
  were not: `more = q > p and attr_start? cs[q]` exists only to get the next
  line under 80 characters.
- why it matters: the limit itself is reasonable. What hurts is that a long
  string literal has no continuation form, and a long condition can only be
  broken by naming its parts. About fifteen helper bindings in this port
  exist for the column count alone.

### F14: clarifying parentheses are refused
- kind: aesthetics
- severity: nit
- where: commonmark/html.kso:`ends_tag_name?`
- wanted: `c == none or c == ">" or link_space? c or (c == "/" and cs[e + 1] == ">")`
- wrote: the same without the parentheses, after
  `error[formatting]: these parentheses group nothing`.
- why it matters: most readers do not know offhand whether `and` binds
  tighter than `or` in a new language. The parentheses were for them.

### F15: arm ordering across several parameters is hard to predict
- kind: confusing-semantics
- severity: minor
- where: tree.kso:`can_contain?`, links.kso:`bare_step`, refs.kso:`titled`,
  render.kso:`kind_html` and `settle`, inline.kso:`pair_up`
- wanted: to group arms by their first parameter, the way a table reads:
  all the `listing` arms, then all the `doc` arms.
- wrote: the order the checker asked for, one error at a time. All
  `X (item _)` arms of `can_contain?` before any `X _` arm; `none` before a
  record pattern in the same position; `para true` before every other
  `kind_html` arm because a literal sits in its fourth position. And
  `pair_up xs _ i none _` beside
  `pair_up xs j i (delim ...) (delim ...)` was rejected as a tie ("each is
  the more specific one somewhere"), though no value is both none and a
  delim. I moved the closer's pattern into a keyed read in the body.
- why it matters: the single-parameter ladder in chapter 03 is easy to hold
  in your head. With several parameters the order is lexicographic in a way
  the book does not state, and the tie rule treats disjoint shapes as
  overlapping.

### F16: positional construction in alphabetical order swaps fields silently
- kind: refactoring-hazard
- severity: major
- where: commonmark/blocks.kso:`tight_walk` (since removed, see F12)
- wanted: `type tight_walk` with fields `ok` and `prev`, built as "previous
  child, verdict so far".
- wrote: `tight_walk none true` and patterns `(tight_walk none ok)`. That
  compiled. The fields are alphabetical, so the first argument is `ok`, and
  the program died at run time with
  `error[runtime]: an if condition is true or false, got commonmark/frame
  [commonmark/frame [] 6 commonmark/para ["two"] 6] ...`, far from the
  constructor.
- why it matters: the order of a record's fields is decided by spelling, so
  renaming a field can reorder every constructor call and every positional
  pattern without a type error when the fields are generic. Keyed
  construction (`tight_walk ok:true prev:none`) would make the call site say
  what it means.

### F17: no way to print from pure code while debugging
- kind: tooling
- severity: major
- where: scratch/dbg/ (deleted); the investigation behind F12
- wanted: a trace line inside `text_end` or `walk_open` to see what the
  native build was doing.
- wrote: copies of the module whose `inlines_of` and `to_html` returned probe
  strings, such as `"{t}:{text_end ["a" "*"] 1}"`, rebuilt for each guess.
  The probes had to pass check too: an unused parameter is an error
  (`fn inlines_of f refs` had to become `fn inlines_of f _`), a probe line
  over 80 columns is an error, and `(st.stack[1]).children` is refused for
  its parentheses.
- why it matters: `--plan` shows effects, but a bug inside a pure function
  is invisible until the function's result is threaded out to a print. When
  the interpreter and the native build disagree, the interpreter cannot help
  either, and every probe means editing real code so it passes canonical
  form.

### F18: no case conversion or case folding in std/text
- kind: stdlib-gap
- severity: minor
- where: commonmark/chars.kso (`fold_case`, `lower_ascii`)
- wanted: `text/lower name` for HTML tag names, and a Unicode case fold for
  link labels (`[ẞ]` must match `[SS]`).
- wrote: per-character code-point arithmetic over ASCII, Latin-1, Greek and
  Cyrillic, plus explicit arms for `ß` and `ẞ`.
- why it matters: comparing words without regard to case is routine. The
  hand-written fold is right for the spec's examples and wrong for most of
  Unicode.

### F19: no Unicode character categories
- kind: stdlib-gap
- severity: minor
- where: commonmark/chars.kso (`punct?`, `white?`)
- wanted: "is this character Unicode punctuation or a symbol (P or S)", and
  "is it Unicode whitespace (Zs)", which the emphasis rules are defined in.
- wrote: code-point ranges for the blocks that are mostly punctuation, with
  the letters and digits in Latin-1 carved out by hand.
- why it matters: any text processor that follows a Unicode-aware spec
  needs these. The approximation passes all 652 spec examples and is wrong
  for punctuation outside the listed blocks.

### F20: no record update
- kind: missing-feature
- severity: minor
- where: commonmark/tree.kso (`attach`, `with_lines`, `replace_top`),
  blocks.kso (`finish`, `setext_start`, `with_cur`)
- wanted: "this frame with one more child": `{ p with children: kids }` or
  similar.
- wrote: `frame kids p.end p.kind p.lines p.start`, restating all five
  fields, in nine places, plus setters such as `with_lines` and `with_cur`
  to keep the restating in one spot.
- why it matters: with immutable records, "the same record with one field
  changed" is the most common construction there is, and each restatement
  is a chance to commit F16.

### F21: a list grown through any function call is copied, natively
- kind: performance
- severity: major
- where: bugs/push_through_call/; commonmark/build.kso:`reversed`,
  blocks.kso:`drain`, render.kso:`code_text`
- wanted:

      fn reversed xs i acc
        return acc if i < 1
        reversed xs (i - 1) (push_some acc xs[i])

      fn push_some acc none
        acc

      fn push_some acc x
        push acc x

  and, for code blocks, `code_lines lines (i + 1) (push (push out lines[i]) "\n")`.
- wrote: the push moved into the loop's own tail call,
  `reversed xs (i - 1) (push acc xs[i])`, and the code block joined with
  `text/join lines "\n"`.
- why it matters: chapter 08 promises that `push` "reuses the buffer in
  place when the list is uniquely owned". Natively that holds only when the
  push is written directly in the argument of the loop's recursive call. Put
  it in a two-line helper, or push onto the result of another push, and every
  step copies the list and leaks the old one: a 4,000-paragraph document took
  48 seconds and 314 MB in the tree builder, and a 4,000-line code block 250
  seconds and 529 MB in the writer. After moving four pushes, the same
  inputs take 0.03 seconds. The interpreter was linear throughout. Nothing
  in the language says where a push may be written, so this is invisible
  until a profile, and the native binary has no symbols for one.

### F22: a list held in a record is copied on every push
- kind: performance
- severity: major
- where: bugs/record_field_push/; commonmark/tree.kso:`attach`, `add_line`;
  blocks.kso:`each_line`
- wanted: a parser state record with a growing field:
  `state (push st.closed child) st.line st.refs (...)`, and frames whose
  `children` list grows as blocks close.
- wrote: frames no longer hold children. Closed blocks are pushed onto a
  bare list threaded beside the state through the main loop, each frame
  counts how many entries its subtree wrote, and build.kso puts the tree
  back together after the last line. That is the design cmark would use
  for a different reason (it has mutable nodes); here it exists because
  `state (push st.events i) i` copies `events` every step natively, never
  frees the copies (492 MB for 8,000 integers), and is quadratic on the
  interpreter too.
- why it matters: "the same record with a longer list in it" is how state
  is threaded in a language without mutation, and it is what the book's own
  examples do. Paragraph and code lines still live in a frame, so a single
  paragraph of thousands of lines is still quadratic; I left it.

### F23: a call inside one arm of a dispatch crashes the native build
- kind: engine-bug
- severity: blocker
- where: commonmark/blocks.kso:`walk_result`, `process_line`. No
  reproduction in bugs/: the automated reduction ran out of time at 931
  lines, and once the renderer was rewritten for F21 the old spelling stopped
  crashing, so I could not cut a smaller case.
- wanted:

      fn walk_result st _ _ (fence_closed c)
        walked c 0 true (close_top st st.line)

  closing a fenced code block from the arm that recognised its closing
  fence.
- wrote:

      fn walk_result st _ _ (fence_closed c)
        walked c 0 true st

  and `return close_top w.st w.st.line if w.done` in `process_line`. Same
  computation, moved one call up.
- why it matters: with the first spelling, any document with a block before
  a closed fence (`a\n```\n```\n`) segfaulted in the dev and release builds
  and rendered correctly on the interpreter. The tree the parser built was
  identical on both engines; the crash came later, reading the fence's
  fields after an unrelated paragraph had been rendered. I found the
  workaround by guessing, and the fault moved out of reach before I could
  pin it down, which is its own kind of cost: the port now carries a
  workaround for a bug I can no longer demonstrate.

### F24: the native build is slower than the interpreter, and uses far more memory
- kind: performance
- severity: major
- where: whole program; timings in README.md
- wanted: the release build to be the fast one.
- wrote: nothing; this is what remains after F21 and F22.
- why it matters: on a 1.5 MB document (the spec examples concatenated a
  hundred times) the release build takes 4.7 seconds and peaks at 1,059 MB;
  the interpreter takes 26 seconds in 85 MB; commonmark.js takes 1.1 seconds.
  Before the F21 and F22 rewrites, 4,000 paragraphs took 120 seconds natively
  and under a second on the interpreter. Every one of these slowdowns showed
  up as memory growth, which suggests the native runtime's reference counting
  leaks on the paths where it declines to update in place.

### F25: a one-expression test that does not fit in 80 columns must be renamed
- kind: aesthetics
- severity: nit
- where: commonmark/blocks_test.kso, inline_test.kso
- wanted:

      test_list_loose_after_blank =
        kinds_of (parse_document "- a\n\n- b\n") == "loose-list"

- wrote:

      test_list_loose_after_blank =
        doc_tree = parse_document "- a\n\n- b\n"
        kinds_of doc_tree == "loose-list"

  The first form is refused with `a single-expression constant is written
  inline`, and inline it is 86 columns.
- why it matters: test names are long by nature and expected HTML is long by
  nature. Eleven tests in this port bind an intermediate only to get past
  the pair of rules.

### F26: small things
- kind: diagnostic
- severity: nit
- where: various
- wanted / wrote:
  - `fn code_text []` is `error[syntax]: expected a parameter pattern`; the
    empty list cannot be dispatched on, so it became
    `if (length lines == 0) "" ...`.
  - `entries` is an ambient builtin, so `entries = flush out st.closed 1`
    fails with "`entries` is already a declaration"; so do `found`, `done`
    and `rest`, which are mine or the prelude's, and the message does not
    say which.
  - `thematic? cs 1 == false` is refused with "comparing to `false` asks a
    question the value already answers"; fine, but `not` binds looser than
    I guessed and I checked.
  - a failing test prints `FAILED (returned false)` and nothing about the
    values compared; every failure meant a second run with the expression
    split up.
  - `kanso build` writes the binary and the `.ll` into the current
    directory with no way to name another; check.sh builds from inside a
    temporary directory to keep the source tree clean.
- why it matters: each is small; together they are most of the
  edit-compile cycles that did not move the port forward.

## What worked well

**Dispatch on block kind.** The block parser's rules differ by kind in four
places: whether a block continues onto the next line, whether it accepts
text, what it may contain, and what closing it does. Each is one overload
group over the kinds, and adding a kind meant adding one arm to each:

    fn continues para _ c _
      if (blank? c) unmatched c

    fn continues quote _ c _
      g = nonspace c
      return unmatched if g.col - c.col > 3 or c.chars[g.at] != ">"
      after_marker (advance_to c (g.at + 1))

cmark spreads the same decisions over switch statements in four functions.
Here each group reads as a table, and the checker would refuse an arm that
two kinds could both reach.

**Dispatch on the next character.** The inline scanner is
`scan_at cs p out refs cs[p]` with one arm per special character and a
`none` arm for the end of the text. That is the whole tokenizer, and the
literal arms make it easy to see which characters are special.

**Values all the way down.** Nothing in the parser can be changed under
another part of it. The setext-heading rule takes a paragraph's reference
definitions, finds nothing is left, and declines; in cmark that path
mutates the paragraph and relies on the caller to cope. Here the "declined,
but with the definitions removed" state is just a different value handed to
the next start. Every unit test is a constant comparing values, with no
setup.

**Two engines that must agree.** The interpreter was right every time the
native build was wrong, so it worked as an oracle for the compiler as well
as for my code. Running all 652 spec examples through three engines in
check.sh is what found F12, F21 and F23 at all.

**Effects at the edge.** The program's I/O is `os/args`, `io/stdin`,
`os/read_file` and `io/write`, in a 30-line cli.kso. The missing-file case
is an arm on `file_not_found`, not an exception handler, and the renderer
underneath is a pure function from a string to a string, testable without
fixtures.

**Canonical form, mostly.** Alphabetical declarations made files easy to
navigate once they were long, and I never thought about formatting. The
costs are in F13 to F15 and F25; the benefit was real too.

## Summary

The five entries I would fix first:

1. **F12, F23: native code that disagrees with the interpreter.** A fold
   that hands back a corrupted record, and a crash that came and went with
   unrelated edits. These give wrong output rather than slow output, and
   the only defence was running every fixture on three engines.
2. **F21, F22, F24: in-place update that silently stops happening.** Whether
   `push` extends a list or copies and leaks it depends on whether the push
   sits in a record, behind a helper call, or inside another push. The
   difference was 48 seconds against 0.03 on the same input, and the
   language gives no sign which one you wrote.
3. **F8, F9, F4: none handling.** No narrowing after a `== none` guard, no
   wildcard that takes none, and an arm required on every function an index
   result reaches. Together they turned every optional result into a pair of
   functions and put about thirty `none` arms in a 2,200-line program.
4. **F5, F6: the module namespace.** Imported type names win over my own,
   and every type name in a module is reserved as a local name in every
   file. A new type forced renames in other files about a dozen times.
5. **F7, F11: the checker's view of err and of modules.** An err arm in a
   library spreads to every caller that passes a non-literal, and
   `kanso check` on a module says ok to code that fails once imported.

Writing this program in kanso was two different experiences. The parts that
are about the problem went well: CommonMark's block and inline rules mapped
onto overload groups with very little ceremony. Once the spec fixtures were
exported correctly, the first full run passed 645 of the 652 examples; four
of the seven failures were mine and took an hour, and three were the native
faults below. The parts that are about the
toolchain went badly. Roughly half the session went to the native builds:
three separate faults where the compiled program crashed or computed
nonsense while the interpreter was right, and a set of performance cliffs
where an ordinary refactor (moving a push into a helper) made a linear
program quadratic and leaky. Debugging those without a print statement or
symbols meant editing real code into probes that still had to satisfy the
formatter. The checker's rules about none and err were the steady
background cost: each is defensible on its own, but together they shaped
the code more than the problem did.
