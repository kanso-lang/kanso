# kanso language specification — v0.1 (design freeze, july 2026)

working name **kanso** (japanese: simplicity through elimination of clutter), extension `.kso`. pronounced KAHN-so, unvoiced s.

this document is the authoritative record of every design decision gaveled to date. anything marked OPEN is not yet decided. the founding principle behind every rule:

> anything a style guide, linter, or code review would enforce by convention, kanso enforces by making the alternative a compile error or unrepresentable. programs have a canonical form: one rendering per program. the source file contains only decisions; all derivable information lives in tooling (LSP inlays, publish diffs).

**the annotation doctrine (governs everything):** an annotation is legal iff it carries a stipulation the compiler cannot derive. redundant annotations are compile errors. this one rule decides return types (never written), interfaces (deleted), type variables (written only when stipulating), typeset guards (written because enumeration is non-derivable), and field types (written because declarations have no body).

**the ordering principle:** order is never implicit. `.`, `>>`, and recursive data flow are the complete vocabulary of "before." where there is no mutation, the runtime owns order (reorder, parallelize, dedupe, cache freely).

---

## 1. purity and effects

- every function is pure and returns exactly one value.
- effects are values (descriptions). `print "hi"` returns a description of printing; nothing executes until the runtime receives `main`'s description.
- `.` (spaces around it) composes descriptions with data flow (pipe). `>>` composes with pure sequencing, no data.
- descriptions are inspectable data: testing asserts on descriptions or runs them against a scripted executor and asserts on the transcript. no mocks. (structural equality holds only up to closure boundaries; the transcript method is the general story.)
- effect sets are inferred per function and propagate up the call graph. no function coloring, no async/await. `main`'s inferred effect set is the program's permission manifest.
- pure calls are plain calls: zero monadic overhead when both the effect set and propagable set are empty.
- no `do` notation. ordinary bindings and pipes are the only composition syntax.
- descriptions carry origin spans (provenance) for error reporting.

## 2. errors

- no exceptions, no panics. failure is a value.
- **no `ok`/`some` wrappers.** success is the bare value; `err reason` and `none` are ordinary types that propagate.
- fixed-width arithmetic overflow returns `err`. lookup misses return `none`. division by zero returns `err`.
- fine-grained failure types are idiomatic (`timeout`, `parse_failure d`); bare `err` is the base case, not the convention.
- **multi-argument propagation rule:** when several arguments carry contexts, the leftmost context-carrying argument propagates, deterministically, regardless of evaluation order.

## 3. types

- every type is a single-constructor record. no union *declarations*; `true`, `false`, `none`, `err` are independent types.
- **fields are typed** (declarations have no body to infer from):

```
type user
  admin: bool
  email: string
  name: string
```

- **fields in alphabetical order, enforced** (formatting error otherwise). general rule: wherever order is semantically inert (fields, imports, typeset members, overloads within specificity rank), canonical order is mandatory.
- **positional construction only**, in alphabetical field order: `user false "e@x.com" "clay"`. no keyed construction. single-field types construct bare: `err reason`, `circle r`.
- **destructuring (gaveled 2026-07-11):** full positional constructor patterns in parameters and bindings (`user age name = clay`), mirroring construction. keyed subset reads are the dual of record update — partial, by name, alphabetical, binding locals named after fields: `{ author title } = post`, with rename-on-bind for collisions (`{ author: writer } = post`); a keyed read omits at least one field. `_` discards appear in parameters only; binding discards are expressed by omission via the keyed form. bindings are irrefutable: a constructor pattern binds only a value whose inferred set is exactly that type (dispatch remains the only eliminator); literals never appear in binding patterns. records have no field-access syntax — reads happen through patterns.
- record update is the only keyed form (it's a different operation — partial, by name): `u { admin: true }`. keys within an update: alphabetical.
- adding a field is loud: every construction site and destructuring pattern becomes arity-wrong (compile error with locations). renaming a field can reorder the constructor — mitigated by distinct field types catching transposition, LSP field-name inlays at call sites, and rename refactors flagging call sites.
- `_` discards in patterns and is exempt from unused-binding errors.
- types are opaque outside their defining module (law of demeter, structural).

## 4. typesets and unions

- a **typeset** names a set of concrete types. newline-separated members, composition by inclusion, flattened and closed at declaration, cycles are errors:

```
typeset signed
  int
  int16
  int32
  int64
  int8

typeset numeric
  float32
  float64
  integral
```

- typesets/unions may appear **anywhere a type can**: parameter guards — tight single-type ascription `x:int`, parenthesized only for inline enumerated sets `(x: int float64)` (gaveled 2026-07-12: parens carry information or they are illegal), field types (`result: done err pending`), container elements, inferred return sets.
- **values may inhabit unions. the only eliminator is dispatch.** no `match`, no tag tests, no narrowing syntax, no `instanceof`. runtime discriminants exist but are compiler-owned; no user syntax reads them. monomorphic code carries no tags.
- primitives (math, string ops) are concretely typed at the bottom, so every union is resolved by dispatch before computation touches it.
- ownership/coherence: a module may define an overload only if it owns the function name or the argument type. no extending others' typesets. overlapping guard sets on one function: compile error.
- there is no `any` type and no capability-constraint syntax (capabilities are always body-derivable → banned as redundant; enumerated sets are stipulations → legal). do not use the word "union" in user-facing docs; say typeset.

## 5. dispatch and overloads

- overloads dispatch on literal values, concrete types (via destructuring or annotation), or typeset guards. resolution is fully static per monomorphized instantiation.
- **specificity ladder:** literal > single concrete type > typeset (any size) > unannotated generic. sets never rank against each other; overlap is illegal.
- annotations distinguishing overloads are legal exactly when bodies don't (`fn process x:float32` vs generic `fn process x` where both call `to_string`). if bodies structurally distinguish the types, the annotation is redundant → error.
- return-type-directed dispatch: a concrete return annotation is legal where context can't infer it (`decode: config`).
- `switch` doesn't exist; literal/constructor dispatch is the switch. `if` is a binary expression conditional. multi-way conditional: deferred pending evidence from real code.

## 6. auto-propagation

- for any type a function doesn't handle, the compiler generates an identity pass-through overload. you write only the cases you care about; `err`, `none`, `loading`, anything else flows through.
- inferred return sets are honest unions computed as a monotone least fixpoint over the call graph.
- **endpoint rule:** a constructor reaching a chain endpoint unhandled is a compile error. `err` reaching `main` unhandled is uncompilable. in processes, unhandled err at a process top becomes a message to its supervisor.

## 7. constraints and generics (no interfaces)

- there is **no interface/class/instance construct.** generic functions' requirements are inferred from bodies, minimal by construction, transitive through call chains. the LSP renders the effective contract (requirements, effect set, return set) as derived views with provenance chains.
- publish tooling diffs inferred contracts and mandates semver bumps (elm-style, enforced).
- generic library functions replace default methods (define `compare`, get `sort`/`max`/`<` for free). transitivity replaces superclasses.
- `eq`, `compare`, `hash` derive structurally for records of derivables. floats get total ordering (IEEE totalOrder); `sort prices` works, period.
- **explicit type variables:** bracket slot on declarations, go-style, used only where sharing is a stipulation the body/declaration can't imply:

```
type cache[a]
  entries: a[string]
  fallback: a

type matrix[n: numeric]
  ...
```

- functions may use the slot (`fn zip_with[a] f xs:a[] ys:a[]`) only when stipulating sharing beyond what the body forces; otherwise redundant → error.
- requirements attach to usage paths, not type existence: `stats[string]` constructs fine; calling `absorb` (which adds) on it fails at that call site. eager narrowing is opt-in via slot constraint (`type stats[a: numeric]`).
- instantiation mirrors declaration: `cache[user]`, `pair[int string]`. no variance annotations (immutability → everything covariant), no higher-kinded types, no explicit instantiation of inferables.

## 8. containers

- **lists:** `T[]` in type position (postfix). literals `[1 2 3]`. the word `list` does not appear in surface syntax.
- **maps:** `T[K]` — element type first, key type in brackets. postfix composes left-to-right: `user[string][]` is a list of string-keyed user maps.
- **indexing (gaveled 2026-07-12):** `xs[i]` / `m[k]` — tight postfix brackets, the strict form: a miss is `err` and rides to the endpoint report. safe by default; expecting presence is the default contract. `at xs i` is the deliberate opt-in for expected absence: it returns the value or `none` (propagates). two operations, two spellings, no overlap. no panicking access anywhere — the "panic" is a value on the same rails.
- `put m k v` is functional; perceus makes it in-place when uniquely owned.
- map keys require derivable `eq`/`hash` (inferred constraint).
- `entries m` returns pairs in **sorted key order** — deterministic iteration, always.
- **1-based indexing, everywhere, no exceptions.** `slice xs 2 4` is inclusive both ends. `index_of` returns a position or `none`.
- map literals: OPEN (candidate `["k": v]`, no pressure yet).

## 9. bindings, rebinding, flow

- **constants (gaveled 2026-07-12):** a value with no parameters is a constant, declared at top level: `tau = 6.28318`. bare mention yields the value (purity makes evaluation timing unobservable; the runtime owns sharing, haskell-CAF style). a single-expression constant is written inline; a multi-statement constant is `name =` over an indented body — one rendering each. `fn` therefore always takes parameters (a zero-arity `fn` is unrepresentable), a constant admits no overloads, and `main` is itself a constant: the program\u2019s description. effectful nullaries (`now`, `random`) need no exception: they are descriptions, inert until the executor runs them.
- `=` binds immutably. **rebinding a name is legal** (SSA under the hood); each version must be used before the next rebind (nothing-wasted per version). closures capture values, not names.
- no `var`, no `const` — one kind of binding, zero keywords.
- unused expressions and unused bindings are compile errors.
- pipe is canonical for linear chains where each intermediate is used once; rebind ladders are for what pipes can't express. (tentative: pipeable ladder = compile error; fallback: tooling auto-pipes. decide during implementation.)
- **kanso has no commas (gaveled 2026-07-11).** application is flat juxtaposition: `f a b` is a two-argument call; every enumeration (arguments, list elements, patterns, lambda parameters) is space-separated, and non-atomic elements are parenthesized (`f (g x) y`, `[(a + b)]`). the pipe inserts the piped value as the first argument of its target application: `x . f y` is `f x y`; the stdlib is subject-first so chains stay pipeable. no semicolons.
- string interpolation `"{x}"`. comments `#` (gaveled 2026-07-12: shebang lines are comments for free). 2-space indent. snake_case, all lowercase, always.
- canonical formatting is grammar: non-canonical whitespace is a syntax error. no formatter tool exists.

## 10. memory model

- values immutable → heap is a DAG by construction → reference counting is complete. **no GC, no borrow checker, no lifetimes, no memory syntax in source.**
- perceus-style: counts elided statically (no shadowing + nothing-wasted make liveness exact); frees inserted at last use; unique ownership turns record update into in-place mutation (FBIP). tail-recursive state loops compile to loops mutating registers.
- **`build` regions** for cyclic construction: `slot` cells are mutable inside; writers return effect descriptions executed by the region's executor; the return value freezes (slots become immutable fields). the slot type may not appear in `build`'s return set (compile check reusing propagation set-tracking). slot effects outside `build` are unhandled-type errors. everything from one region freezes into a single RC block; internal refs (including cycles) are offsets; the block frees as a unit — DAG preserved at RC granularity.
- magic budget: intrinsics only (`slot`, `read`, `write` + runtime effect vocabulary: file, net, clock, random, spawn/send/receive...). the spec maintains a literal intrinsics list; everything else is ordinary code. stdlib has no privileged powers.
- build totality: OPEN — unwritten slot at freeze is probably a runtime `err` (consistent with overflow-as-value). two writes to one slot with no `.`/`>>` path between them: compile error (race inside the region).

## 11. concurrency and processes

- arguments and independent bindings evaluate in runtime-chosen order; only the description graph constrains effects. implicit parallelism falls out.
- **processes, erlang-shaped:** `spawn f` (description yielding pid), `send pid m` (effect), `receive` (effect yielding next message). servers are recursion-held state: `fn serve state` / `receive >> (m -> serve (handle state m))`. message handling is dispatch-by-overload on message types.
- no shared memory, no locks, no channel primitives. supervision trees: a child's unhandled err becomes a supervisor message with restart policy as data. **the root supervisor is user code** — global error reporting (sentry etc.) is an ordinary overload there, with provenance spans riding along. no ambient handlers.

## 12. modules

- modules are directories (go-style); subdirectories private to parent. imports: `import "std/http"`, alphabetized.
- **files (gaveled 2026-07-12):** a module splits into any number of `.kso` files sharing one namespace. file names are decisions (organization carries information); canonical ordering holds per file; an overload group lives in one file.
- **visibility (gaveled 2026-07-12):** a `_`-prefixed top-level name is module-private, enforced at the module boundary. visibility lives in the name, so every call site displays it; no export lists. a private declaration unused anywhere in its module is a compile error (nothing-wasted); public names are API surface and exempt.
- bare identifiers mandatory; qualification only on collision; qualifying a unique name is a compile error.
- qualification syntax: OPEN — `json.decode` (whitespace-distinguished from pipe) vs `json/decode` (matches import paths, frees `.`). recommendation on file: slash.

## 13. numerics and strings

- `int` is arbitrary-precision, the default. fixed-width `int8..64`, `uint8..64`, `float32/64`. fixed-width overflow → `err`.
- strings are opaque UTF-8; string positions 1-based.
- coercion/promotion details: OPEN (recommendation: none — explicit conversion functions only).

## 14. open questions (priority order)

1. **inference fixpoint formalization.** dispatch depends on types; types depend on generated pass-throughs; pass-throughs change return sets. plan: propagable sets as monotone least fixpoint over the call graph, then dispatch resolution, with programs where dispatch would feed back into sets rejected. **do this on paper before any code.** unions-as-values made the lattice natural; the stratification proof is still owed.
2. destructuring: record patterns gaveled 2026-07-11 (see §3); still open: list patterns (`fn sum []`, cons/rest spelling — leading candidate `[first *rest]`, mirroring construction splats).
3. qualification syntax (§12). 4. build totality (§10). 5. pipe-vs-rebind canonicality (§9). 6. map literals. 7. multi-way conditional. 8. ffi (route through effect descriptions at the edge; 1-based translation layer lives there). 9. build tool (`kanso run/build/test/publish`; publish does contract diffs). 10. numeric coercion. 11. process details: mailbox ordering guarantees, selective receive or strict FIFO, restart policy vocabulary.

## 15. implementation plan

- **phase 0 (paper, ~a week):** fixpoint formalization (§14.1). the language's two founding features must be proven mutually consistent before code.
- **phase 1: tree-walking interpreter** as reference implementation. suggested host: rust (query-based/salsa-style incremental architecture from day one — the LSP shares the query engine; this was always the architecture). deliverables: lexer/parser for the canonical grammar (formatting errors included), name resolution, the inference engine (sets + unification + constraints), dispatch resolution, pass-through generation, endpoint checking, description-building runtime with a pluggable executor (real + scripted-for-tests), `kanso run`, golden-file test corpus of programs and expected errors (the error corpus matters as much as the success corpus — half this language's value is its compile errors).
- **phase 2: LSP** on the same query engine: contract inlays, field-name inlays at construction sites, provenance chains.
- **phase 3: native compiler.** performance is a founding goal, not a later phase: the abstract cost model (op-count baselines in CI) governs it from phase 1, before any backend exists. first native target: **C** (the koka/lean-proven path for perceus) or cranelift for a jit story — emitting C is not the slow path, it is renting clang/gcc's entire world-class optimizer without maintaining IR bindings; koka and lean post C/C++-competitive numbers this way. kanso's headline speed comes from its own analyses — perceus refcount elision, FBIP, monomorphization (zero dynamic dispatch), and ordering-principle parallelism — which must exist before any backend can multiply them. direct llvm ir lands when profiling shows the C detour leaves speed on the table. monomorphization + pass-through generation is a code-size multiplier: measure early. adopt rust's ban on polymorphic recursion.
