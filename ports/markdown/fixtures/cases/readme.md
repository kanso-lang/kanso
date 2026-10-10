# markdown: a CommonMark renderer in kanso

This is a port of [cmark](https://github.com/commonmark/cmark), the reference
C implementation of [CommonMark](https://commonmark.org), to kanso. It reads
Markdown and writes the HTML that cmark writes for it. The parsing strategy
follows the one described in the CommonMark spec's appendix and used by cmark
and [commonmark.js](https://github.com/commonmark/commonmark.js): a pass over
lines that maintains a stack of open blocks, then a pass over each paragraph
and heading for inline structure. The code was written from that description
and from the spec; no source was copied from either implementation.

Credit for the design belongs to John MacFarlane and the CommonMark
contributors. The spec examples under `fixtures/spec/` are taken from the
CommonMark spec, version 0.31.2, which is licensed CC-BY-SA 4.0.

## What it covers

All 652 examples in the CommonMark 0.31.2 spec pass, byte for byte, on the
interpreter, the dev build and the release build.

Blocks: ATX and setext headings, paragraphs, thematic breaks, block quotes,
bullet and ordered lists (tight and loose, nested, with lazy continuation
lines), indented and fenced code blocks with info strings, all seven kinds of
HTML block, and link reference definitions. Tabs are expanded to four-column
stops where they count as indentation, as the spec requires.

Inlines: emphasis and strong emphasis by the delimiter-run algorithm,
including the rule of three; code spans; inline links and images, and full,
collapsed and shortcut reference links; URI and email autolinks; raw HTML;
backslash escapes; named, decimal and hexadecimal character references, with
the full HTML5 entity table; hard and soft line breaks.

## What it leaves out

- cmark's other output formats (XML, LaTeX, man, CommonMark) and its
  source-position option. Only HTML is written.
- Extensions such as GitHub's tables, strikethrough and task lists.
- cmark's "safe" mode, which drops raw HTML and dangerous URLs. HTML is passed
  through, as `cmark --unsafe` does.
- Two places where the spec defers to Unicode tables use approximations,
  because the standard library has no Unicode character data:
  - link labels are case-folded for ASCII, Latin-1, Greek and Cyrillic and
    for `ß`, which covers the spec's examples but not all of Unicode;
  - "Unicode punctuation" for emphasis flanking is a list of code-point
    blocks rather than the P and S categories.
- Performance on large documents is worse than linear in places; see
  FRICTION.md. A typical README renders in a few milliseconds.

## Running it

The toolchain is `/tmp/claude-0/kanso-main/kanso` in this environment.

    kanso run main.kso < input.md              # native, dev build
    kanso run main.kso --interp < input.md     # the interpreter
    kanso build main.kso --release             # ./main, optimised
    kanso test commonmark                      # unit tests

The program reads Markdown on standard input and writes HTML to standard
output.

`sh check.sh` runs the unit tests and then every fixture on all three engines,
comparing each output to the expected HTML. `fixtures/spec/` holds the spec
examples (one `.md` and one `.html` per example, with the spec's `→` turned
back into tabs). `fixtures/cases/` holds longer documents whose expected HTML
was produced by commonmark.js 0.31.2.

## Layout

    main.kso                 the entry point: stdin to stdout
    commonmark/
      caret.kso              a cursor on one line, with tab-stop columns
      chars.kso              character classes and case folding
      tree.kso               block kinds, frames and parser state
      blocks.kso             the line-by-line block parser
      build.kso              putting the block tree together afterwards
      refs.kso               link reference definitions and label matching
      links.kso              destinations, titles and labels
      html.kso               the HTML tag grammar, for blocks and inlines
      inline.kso             the inline scanner, brackets and emphasis
      escape.kso             entities, backslash escapes, URL encoding
      entities.kso           the HTML5 entity table (generated)
      render.kso             the HTML writer
      *_test.kso             unit tests
    tools/                   fixture and table generators, a spec report
    bugs/                    reproductions of compiler and runtime bugs
    FRICTION.md              the journal of what got in the way
