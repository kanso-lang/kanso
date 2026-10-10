# The twenty projects

Chosen to cover different kinds of program: interpreters, parsers, text
tools, numeric code, binary formats, servers, storage, search. Each line names
the original, the directory, and the core the port must cover.

1. **mal** (Make-A-Lisp) → `mal/`. Reader, printer, environments, closures,
   `def!`/`let*`/`fn*`/`do`/`if`, tail calls, quasiquote, macros, `try*`/
   `catch*`, atoms, the core namespace, a REPL over stdin. Pass the mal step
   tests you can reconstruct.
2. **jlox** (Crafting Interpreters) → `lox/`. Scanner, recursive-descent
   parser, resolver, tree-walking interpreter with closures, classes,
   inheritance and `super`, runtime errors with line numbers.
3. **cmark** (CommonMark) → `markdown/`. Block structure (headings, paragraphs,
   lists with nesting, block quotes, fenced and indented code, thematic
   breaks, HTML blocks) and inlines (emphasis by the delimiter algorithm,
   code spans, links and images with reference definitions, autolinks,
   escapes, entities), rendered to HTML.
4. **toml** (toml-rs / toml-test) → `toml/`. Full TOML 1.0 parser to a value
   tree and back out as JSON (the toml-test convention), with errors that name
   line and column; dates and times as typed values.
5. **ripgrep** → `rg/`. Recursive search with regexp, `.gitignore`-style
   ignore rules, globs for include/exclude, line numbers, `-A/-B/-C` context,
   `-c`, `-l`, `-v`, `-i`, `-w`, multiple patterns, binary-file detection.
6. **xsv** → `xsv/`. RFC 4180 CSV reading and writing, and the subcommands
   `select`, `search`, `sort`, `stats`, `frequency`, `join` (inner, left,
   outer), `slice`, `count`, `headers`, `table`.
7. **GNU diffutils** → `diff/`. Myers diff, unified and context output,
   `-u N`, recursive directory diff, and a `patch` that applies unified diffs
   with fuzz-free hunks and reports rejects.
8. **Zola** → `site/`. A static site generator: content tree with TOML or
   YAML-ish front matter, sections and pages, a small template language with
   inheritance, blocks, loops and filters, a markdown subset, slugs, a sitemap.
9. **mustache** (the spec) → `mustache/`. Variables, escaping, sections,
   inverted sections, partials, comments, set delimiters, lambdas where kanso
   allows; run against the spec's cases.
10. **todo-backend on a tiny HTTP server** → `httpd/`. HTTP/1.1 over
    `std/net`: request parsing, keep-alive, chunked bodies, a router with path
    parameters, JSON handlers implementing the todo-backend API, static files,
    and a test client that drives it end to end.
11. **Ray Tracing in One Weekend** → `raytracer/`. Vectors, rays, spheres,
    materials (lambertian, metal, dielectric), a camera with defocus blur,
    a seeded random generator, antialiasing, PPM output. Deterministic.
12. **a CHIP-8 emulator** → `chip8/`. All 35 opcodes, timers, a headless
    framebuffer dumped as text, a scripted keypad, and test ROMs you assemble
    yourself, plus a small assembler for them.
13. **ugit** (mini git) → `ugit/`. Content-addressed object store with
    sha256, blobs, trees, commits, refs and HEAD, `init`, `add`, `commit`,
    `log`, `checkout`, `branch`, `status`, `diff`, a three-way `merge` with
    conflict markers.
14. **minisat** → `sat/`. DIMACS input, CDCL with two watched literals,
    first-UIP clause learning, non-chronological backtracking, VSIDS, restarts,
    models checked against the formula, and a small benchmark set.
15. **GNU bc** → `bc/`. The bc language: arbitrary-precision decimals with
    `scale`, variables, arrays, functions with `auto`, `if`/`while`/`for`,
    the math library functions `s c a l e j` via `-l`.
16. **make** → `make/`. Makefile parsing: rules, prerequisites, recursive and
    simple variables, automatic variables, pattern rules, `.PHONY`, includes,
    mtime-based rebuild decisions, running recipes through `std/os`, `-n`,
    `-k`.
17. **bitcask** → `kv/`. A log-structured key-value store: append-only data
    files with checksums, an in-memory key directory, hint files, merge and
    compaction, crash recovery from a torn tail, and a CLI.
18. **zlib inflate** (puff.c) → `inflate/`. DEFLATE decoding (stored, fixed
    and dynamic Huffman blocks), gzip and zlib wrappers with CRC-32 and
    Adler-32 checks, and a simple deflate encoder (static Huffman with LZ77)
    so round trips can be tested.
19. **RE2's core** → `regex/`. A regex engine written from scratch, without
    `std/regexp`: parser to an AST, compilation to a Pike VM, submatch
    capture, character classes, anchors, non-greedy repetition, counted
    repetition, and linear-time matching shown on pathological inputs.
20. **a Datalog engine** (in the style of Soufflé or datafrog) → `datalog/`.
    Parsing facts and rules, stratified negation, semi-naive bottom-up
    evaluation with indexed relations, queries, and programs such as
    transitive closure, points-to analysis and same-generation.
