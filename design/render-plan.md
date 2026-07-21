# the render group: `"{x}"` becomes dispatch on `std/render`'s to_string

Ratified 2026-07-21 (memory: kanso-interpolation-rendering). This pins the
build.

## The model

- `lib/render/` is an ordinary stdlib directory module holding the
  `to_string` group: arms for `none` (`"<none>"`) and a catch-all
  `fn to_string x` that delegates to the low-level structural renderer
  (today's `k_render`/`render`, demoted from interpolation's wiring to the
  group's bottom layer). Primitive rendering therefore lives where the
  primitives do — ambient types bring their canonical arms.
- `"{x}"` desugars to the FULLY-QUALIFIED group call (`render/to_string`),
  so interpolation needs no import anywhere, REPL included — like `+`,
  like `>>`. Bare-name `to_string x` needs `import "std/render"`.
- The module is always linked: arm existence is global; the compiler adds
  `std/render` to every program's compile graph (the one "prelude"
  module, justified because syntax names it).
- A user arms their own types (`fn to_string invoice ...` where they
  define `invoice`) — Ctor rank beats the catch-all's Var rank, so the
  user arm wins by ordinary specificity. Re-arming a primitive is
  unrepresentable (orphan rule: they own neither side); the newtype they
  own is the escape hatch.
- err never reaches the group — it short-circuits interpolation as today
  (the fail-mask `& ERR` machinery is untouched).

## The licensed fast path

Because primitives cannot be re-armed, `to_string` on a set with no REC
bit is PROVABLY the stdlib behavior — codegen may keep emitting the
direct `k_render` call for primitive-only sets, zero dispatch, zero
regression on hot paths (the cost golden enforces this). Only values
whose inferred set includes REC (user types can carry custom arms) route
through the dispatcher. Coherence is what licenses the optimization.

## What it dissolves

- r4: `print "{a_record}"` native build failure (%parsed at k_render) —
  records now flow through the ordinary call machinery (boxing handled by
  the #72/73/74-fixed paths) into the group.
- The hardwired `<none>`/`<io>` sentinels become ordinary stdlib arms.
- The interp/native divergences catalogued in the r4 family.

## Order of work

1. `lib/render/render.kso`: the group (none arm, catch-all arm on the
   builtin), pub.
2. Compiler: always-link std/render; desugar template Interp parts whose
   set includes REC to the qualified group call (both engines; interp
   mirrors in eval_template).
3. Goldens: record-render differential (the r4 repro becomes an example),
   custom-arm example (user type with its own to_string), none-sentinel
   arms replacing the hardwired constants; cost golden proves the json
   bench emits zero dispatch renders.
4. Retire the hardwired none/io special cases from k_render/render once
   the arms cover them.
