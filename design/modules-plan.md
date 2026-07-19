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
