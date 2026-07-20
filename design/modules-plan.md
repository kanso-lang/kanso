# Module system — status and plan

The gaveled design lives in `docs/spec.html` §229–233 (2026-07-12): modules are
directories (Go-style); a module is any number of `.kso` files sharing one
namespace, canonical ordering per file; imports `import "std/http"` alphabetized;
qualification `json/decode` (slash); types opaque outside their module.

## Done

- **Directory-as-module** (`src/lib.rs::compile_module`): reads every `.kso` in a
  directory, `check_file` each (per-file canonical order + name resolution
  against the module's other-file names via `extern_globals`), merges into one
  `Program`, `check_merged` (main, duplicates). `kanso run <dir>` works today.
  The VSE "concatenation friction" was a mistake on my part — running a merged
  single file instead of the directory. Three files in a directory Just Work as
  one module.
- **`pub` parsing** (`fb714be`): `pub` is an optional leading keyword on any
  top-level decl; lexed `KwPub`, carried as `is_pub` on `FnDecl`/`TypeDecl`. Not
  yet enforced.

## Missing, with the implementation approach

### 1. Cross-module `import "path"` (the real gap)
`compile_module` loads ONE directory. No `import` statement is parsed or
resolved. Approach:
- **Parse** `import "path"` as a top-level statement (alphabetized, per §229).
  New AST: `Program.imports: Vec<Import { path, span }>`.
- **Resolve**: a module-load function that, given a root, compiles the target
  directory-module (recursively for its imports), with a visited-set for cycle
  detection. Path resolution: relative to a module root / a `std` root (TBD where
  the search paths live — the eventual `hako` registry).
- **Bind names**: the imported module's `pub` names become available in the
  importer, qualified `path/name` (slash form, §233). Unqualified use is a name
  error; qualified resolves to the import.
- This is where `pub` enforcement lands: only `pub` names cross a module
  boundary. `import` + `pub`-enforcement are one piece.

### 2. `pub` enforcement / visibility
Private (non-`pub`) top-level names are module-private — invisible to importers.
Within a module (the directory), everything is visible (current behavior). So
enforcement only bites at the import boundary (item 1). `check_unused_private`
already exists; extend the notion of "private" to "non-pub" once imports create
a real boundary.

### 3. `play` / body entrypoint
`main.rs` + `eval` hard-key on `main` (`fns.get("main")`). Gaveled model: a
runnable's body IS the program; libraries are definitions-only; the playground
runs a `pub play`. Minimum step: the runner evaluates `play` when there is no
`main`. Full model: top-level bare statements after definitions are the program
(a bigger parser/eval change). Your `examples/concurrency.kso` `pub play` form is
blocked on this (it parses now, but the runner wants `main`).

### 4. Ban leading-underscore identifiers + corpus migration
`_`-prefixed names are the SUPERSEDED `_`-privacy gavel (spec line 231, stale),
replaced by `pub`. Ban `_[a-z]…` identifiers (lexer naming error; lone `_`
wildcard stays). Migration: strip the `_` prefix across `apps/kq`,
`apps/kanso-json`, `examples`, VSE — with a collision check (no `_foo`/`foo`
clash) — and mark the genuine public API `pub`. **Do this WITH item 1/2**, not
alone: today the `_` prefix is the only "internal" signal, so strip it as the
signal moves to `pub`. Risky corpus-wide mechanical change; golden tests gate it.

## Sequencing
Item 1 (imports + pub enforcement) is the keystone and the real gap. Item 3
(`play`) is small and unblocks `concurrency.kso`. Item 4 (underscores) rides with
1/2. Suggested order: 3 (quick win) → 1+2 (the core) → 4 (migration) → update
`spec.html` §231 + the corpus.

## GAVELED (2026-07-19): entrypoint files are statements-only

Clay ruled option 1: **a file is a library (definitions only) or an entrypoint
(statements only) — never mixed.** No magic names anywhere: the language runs
the entrypoint file's body; `main` is a relic that dissolves when this lands.
Bindings in an entry file are body bindings (sequential); there are no
constants/fn/type declarations in an entry file. The playground's hidden
entrypoint becomes literally `import <library>` + `play`. Single-file programs:
a statements-only file runs as-is (hello is one line); anything defining
functions is a library and needs an entry. Build WITH cross-module import
(the entry file imports the library) — they are one feature.

## GAVELED (2026-07-19, second ruling): the entry file is named main.kso

Clay resolved the committee's one open split beyond either side: the entry file
is **required** to be `main.kso` — "if there's only one entry file you might as
well force it to be main.kso by convention. no reason to allow ambiguity."

- `kanso run <dir>` runs `<dir>/main.kso`. Uniqueness needs no compile error:
  the filesystem cannot hold two files of one name — the invalid state is
  unrepresentable, not detected.
- The file NAME selects the grammar: `main.kso` parses as an entry
  (statements-only); every other `.kso` parses as a library (declarations
  only). A bare expression in a library file is a compile error naming the fix
  ("statements live in main.kso").
- The LANGUAGE still has no magic names — no `main` identifier exists anywhere
  in the grammar; the name lives in the toolchain's discovery rule, like Go's
  main.go made honest (Go treats the filename as convention; kanso enforces it
  and gets zero ambiguity in return).
- Explicit single-file runs stay: `kanso run hello.kso` treats the named file
  as the entry whatever it is called — pedagogy keeps one-file programs
  (statements-only), and no ambiguity exists because the user pointed.
- The playground's hidden entry becomes literally a synthetic `main.kso`:
  `import` + `play`.

All other committee rulings stand as synthesized: strict statements-only
entries (no dispensation), no thin-entry enforcement beyond the grammar (an
entry cannot declare, so it cannot hide logic), directory-stem binary naming,
no go.mod-style manifest. Build rides with cross-module import as one keystone
commit.

## RATIFIED BY CONSENSUS (2026-07-19, Clay: "go with the committee's consensus so we can make progress")

- **Import resolution:** `std/` prefix reserved for the shipped stdlib; every
  other path resolves relative to the importing file's directory. No manifest,
  no config; cycles are a compile error. `hako` extends resolution to fetched
  packages without changing either rule.
- **Enumerable opens:** `range` is the statistic (max − min); the integer
  generator is spelled `naturals . take n` (no `upto` until demanded). Map key
  collisions (`transform_keys`, `index_by`) are last-write-wins, exactly `put`'s
  semantics, documented not hidden. No `first coll n` — the spelling is
  `take n . to_list`; `first coll` (the element) stays.
- **Observation clause:** the language committee watches how these rules land
  on the real libraries — kq, kanso-json, vse — during the keystone migration,
  and files friction as amendments rather than pre-litigating.

## GAVELED (2026-07-19, third ruling): `kanso play`

The toolchain gains a third verb: `kanso play <file-or-module>` synthesizes an
in-memory entry — `import <target>` plus the bare statement `play` — and runs
it. It is the playground's hidden mechanism made available at the terminal;
the playground becomes "kanso play in a browser tab", one mechanism, two
surfaces. The language grammar still never contains the word play (same
toolchain-convention layer as the main.kso filename). Errors teach: a target
with no pub play gets "nothing to play — define pub play, or point kanso run
at a main.kso." The verbs carry the model: run is for programs, play is for
libraries. Migration consequence: the examples corpus STAYS single-file
play-libraries (no per-example entry directories); only real programs grow
main.kso entries.

## GAVELED (2026-07-20): the visibility model (committee-synthesized, Clay-ratified)

One sentence: **pub is name-level surface; the only field-level access that
crosses a boundary is a pub fn the author wrote on purpose.**

- No transparency grades. Types are opaque outside their module, always.
- Construction is module-private; importers build through pub factories.
- Dispatch on a foreign type NAME is free (membership needs no structure);
  destructuring a foreign type is banned (positional reads in a pattern).
- Reflection surfaces (==, sort, "{x}", encode) stay structural for all
  callers — no source names a field, so no leak; output shape is the
  publisher's own contract.
- Single-module world (repl, playground, one file): zero ceremony; the
  boundary bites exactly at import.
- Naming a type (annotation, arm, typeset) requires importing its module;
  holding/passing unnamed values through intermediaries requires nothing.
- Chains die at the first foreign dot: A can never open B's record to reach
  a C value; whether A can OBTAIN it at all is B's choice (a pub getter).
  Once held, a C value supports exactly: hold/pass (no import), plus
  name-dispatch and C's pub fns (with import c). Provenance-strictness
  (blocking C's own API on values that arrived through B) is REJECTED — it
  forces total forwarding and breaks B-returns-c/timestamp patterns.
- Re-export: functions re-export by ownership — `pub thing = c/thing` or an
  explicit forwarder — making b/thing B's own promise. Type names do NOT
  re-export (the import block stays a complete dependency inventory; Go's
  precedent). Facade sugar, if migration demands it, expands to named
  forwarders.
- Getter one-liners are the defended pattern, not an apology: pub fn city
  is B choosing which chain to promise while representations stay free —
  load-bearing while the beat/carry machinery restructures layouts.

**BOOK MANDATE (Clay):** the imports/visibility chapter treats this
rigorously — the patterns catalog (getters, factories, name-dispatch,
diamond naming, function re-export) and every gotcha with its diagnostic
(first-foreign-dot, private construction, foreign destructure, naming
without import), each panel executed per the book rule.

**Blast-radius doctrine (Clay + session, same night):** B returning C's types
from its pub surface exports B's dependency to every caller — A is forced to
`import c` just to name what B hands back, and a C change then detonates
through B and every A. The model keeps that legal (a real dependency,
honestly declared — the forced import IS the leak made visible, pointing at
the right author), but the doctrine names it: **a library's pub surface
should be closed over its own names** — own your returns (wrappers) or
re-export the needed functions, so C's changes stop at B. The memorable
rule: *you import what you name; a good library lets you name only its own
things.* Tooling follow-up for the keystone: an ADVISORY from `kanso check`
when a pub fn's inferred return set includes a foreign type — the checker
already sees it statically. That is Demeter's actual point — bounding the
blast radius of change — enforced where the compiler can see it.

**AMENDED (Clay, same night) — the door principle supersedes capability 3's
framing:** type identity includes the package's major version (hako's
aliasing implies it; Go's import-path-identity precedent) — cross-version
mixing is a compile error, and within a unified build the "two doors" lead
to one room, indistinguishable by value semantics. The ruling: **values are
used through the door they came from.** A pub surface returning a type is
responsible for the operations on it — re-exported or wrapped; a handle you
can hold but not use is the exporting module's bug. A's direct import of C
serves A's OWN use of C, never as a workaround for B's incomplete surface.
The leak advisory upgrades accordingly: when a pub fn returns a foreign
type and the module's surface offers no operation accepting it, kanso check
says so — "re-export what callers need, or wrap it." Go comparison, for
the book: kanso adopts Go's version-identity mechanics wholesale and adds
the boundary discipline Go only approximates with internal/ and folklore.
