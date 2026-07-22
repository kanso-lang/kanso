# the import incarnation: one form, bare overload space

Ratified 2026-07-22 (memory: kanso-layout-gavel, final import gavel).
`import "path"` declares the dependency and enrolls every pub name into
the file's bare overload space. Overloading IS the resolution mechanism.

## Semantics

- Bare call `select args`: dispatch over the UNION of arms — local
  `select` decls + `q/select` for every imported qual q. Specificity
  picks; unordered ties error at the use site (fix: qualify); identical
  signatures are duplicate errors at whole-program compile.
- Qualified call `list/select args`: narrows dispatch to that module's
  arms (the qualification-narrows gavel, load-bearing).
- Renames: `import { select:pick } "path"` (tight colon, theirs:yours);
  qualifier alias `import json2 "path"`. Pure preference, never
  required for coexistence.
- No selective-list form, no glob, no dot-import. Formatter canon
  (phase 2): bare where unambiguous, qualified where narrowing/tie
  demands.
- A local fn named like an imported export is an arm of the bare space
  like any other (ownership rules unchanged).

## Build order

1. Loader/check: per-file bare-resolution table (short name -> matching
   qualified identities across imports + locals + ambient).
2. eval: bare dispatch unions overloads; qualified stays narrowed.
3. codegen/wasm: bare call sites emit a union dispatcher under the bare
   symbol (emit_dispatcher over the union decl list); qualified symbols
   unchanged.
4. Parser: `import alias "path"` and `import { a:b } "path"` forms;
   rename plumbing (a rename adds a bare alias entry to the table).
5. math/random moves (std/random dies); anti-stutter rule documented.
6. examples/imports.kso: all forms + a local arm joining the space +
   a tie resolved by qualification. Differential goldens.
7. Corpus sweep back to bare where unambiguous; book samples follow.
8. Grammar/book spine updates.

Committee reviews the library rewrite in this incarnation as the
feedback loop (Clay's standing instruction).

## Status (2026-07-22, WIP on branch import-incarnation)

DONE: bare enrollment via synthetic clones (load_dependencies), union
dispatch working interp-side (flagship probe PAID/[2 4] both engines
after the codegen global-grouping fix), alias + rename parser forms and
loader plumbing, bare/rename/alias unused-import accounting, wsym sigil
quoting, synthetic markers consumed by beat + door advisory, suite
11/11 at the core commit. examples/imports.kso written — INTERP runs
all four forms correctly; PLAN correct.

OPEN BUG (bisected): the ALIAS form alone segfaults native — minimal
repro `import t "std/text"` + `t/join ["a" "b"] "-"` = exit 139; the
same call unaliased (text/join) is green corpus-wide, and bare-union +
rename forms pass in isolation. Interp correct on all forms. Suspects:
alias-qualified decls interacting with a bare clone named `join`
colliding with the k_desc_join/BUILTIN_CALLS join paths in codegen; or
the alias qual flowing somewhere short_name(path) is still assumed.
Attack with the emitted b3.ll (d_t/join_2 and the bare d_join_2
dispatchers) next window. The interp is the oracle.

THEN: math/random move, corpus re-sweep to bare, formatter canon,
grammar forms, book spine.

## Frontier (2026-07-22, second debugging session)

FIXED: locals SHADOW synthetic clones as binding names (check gains
shadowable set — vse's `first =` binding compiles); specificity TIES
break by PROXIMITY (local arms sort before clones in both engines —
vse's count-value vs std count-pred picks the local, interp verified
end-to-end on ch09/vse: exact Quinn output).

OPEN: native runs ch09/vse to silent exit 0 with NO output (interp
exact). Suspect: inference set-widening from union arms steering the
desc-pipe/bind detection in main.kso's `trials runs [...] . means`
chain — if the piped value's set isn't pure DESC, codegen compiles a
value-pipe instead of bind and main's desc is never built. Check
infer's group joins with synthetic arms; consider excluding synthetics
from QUALIFIED-call set joins (bare calls genuinely union; qualified
calls never reach clones).

ALSO QUEUED HERE: rename-REPLACES-spelling semantics (Clay's
clarification: whole module still enrolls; bare theirs suppressed;
yours + qual/yours added — patch drafted, asserts ready); the combined
form `import t { slice:cut } "path"`; standalone pub re-exports;
math/random. NOTE: a stash on this repo holds next-protocol work
(step/done/cursor/next in std/list + any-typeset legalization).
