# Enumerable — kanso's collection vocabulary

Status: **design, ratified in dialog 2026-07-18. NOT implemented.** Current VSE
code uses hand-rolled eager helpers (`examples/enumerable.kso` seed) until this
lands. Implementing it is a major compiler project (see §Compilation).

## 1. Model — lazy pull-based, `next`-rooted, compiled to zero-cost loops

A type has the Enumerable role by defining **`next`** (produce the next
element). Every operation is one of two kinds:

- **adapter** — lazy: returns a new iterator, does *no work* until pulled
  (`map`, `select`, `take`, `cycle`, …).
- **consumer** — drives the iterator, forcing it to a final value
  (`fold`, `sum`, `first`, `to_list`, …).

One element flows through the *entire* adapter chain before the next is pulled.

```
[1 2 3] . map (x -> x * 2) . select (x -> x > 7) . to_list

pull 1:  map 1 -> 2,  select 2 -> 2 > 7? no -> drop
pull 2:  map 2 -> 4,  select 4 -> 4 > 7? no -> drop
pull 3:  map 3 -> 6,  select 6 -> 6 > 7? no -> drop
                                                 -> []
```

Call order is `map, select, map, select, map, select` — each function once per
element, then the next element — **not** `map map map, select select select`.
No intermediate `[2 4 6]` ever exists; early termination falls out (a `first`
consumer stops pulling the instant it's satisfied); infinite sources are fine.

This is **Rust's iterator model, not Ruby's thunks.** The per-element pull-step
is "something like a thunk," but purity + closed-world + monomorphization let
the compiler *discharge* it at compile time instead of heap-allocating it (Ruby)
or hand-writing it (Rust). Laziness is a compile-time guarantee, not a runtime
tax — no thunk graphs, no space leaks, bounded latency. It is **pervasive and
non-optional: there is no `.lazy`.** The good kind of lazy (pull-based, one
element in flight), never the dangerous kind (call-by-need thunk graphs).

## 2. The pipe `.`

`x . f` inserts `x` as `f`'s **first** argument. A `_` in the stage redirects
the piped value to that slot instead. A `_` in the *first* slot is a formatting
error (redundant — the pipe already targets it).

```
prices . (at 3)            # at prices 3        — first slot, implicit
index  . (at prices _)     # at prices index    — non-first slot, explicit
prices . (at _ 3)          # ERROR: redundant placeholder
```

Every signature is **subject-first** (the collection leads), which is what makes
the whole vocabulary pipe placeholder-free. Subject-first is therefore a *hard
law* on every Enumerable signature, not a style preference.

## 3. Naming law

- A **compound name reads left-to-right in execution order** (`map_compact`
  would be map-then-compact). No name may imply a phantom twin or lie about
  order.
- **`select` / `reject`**, never `filter` (`filter` names only the keeping half
  and orphans `reject`; the pair is the poetry).
- **No `filter_map`, no fused portmanteaus at all.** Composition is the pipe's
  job; fusion is the compiler's. `map coords parse . reject (== none)` is the
  succinct form and fuses to one pass — a portmanteau saves zero characters and
  costs a name.
- `fold` (not reduce/inject), `map` (not collect), `find` (not detect),
  `all`/`any` (no `?` suffix), `sum` (not total), `max`/`min` (not
  maximum/minimum).
- **No method named `none`** — `none` is a value; say `not (any list pred)`.

## 4. Vocabulary

### Adapters (lazy — return an iterator)

| name | signature | does |
|---|---|---|
| `map` | `map coll fn` | transform each element |
| `select` | `select coll pred` | keep elements satisfying `pred` |
| `reject` | `reject coll pred` | keep elements failing `pred` |
| `take` | `take coll n` | lazy prefix of ≤ n elements |
| `drop` | `drop coll n` | skip the first n, stream the rest |
| `zip` | `zip a b` | pairwise-combine two iterators, stop at shorter |
| `cycle` | `cycle coll` | repeat a finite source forever |
| `repeat` | `repeat x` | `x, x, x, …` forever |
| `iterate` | `iterate seed fn` | `seed, fn seed, fn (fn seed), …` forever |
| `naturals` | `naturals` | `1, 2, 3, …` |
| `transform_values` | `transform_values map fn` | map over a map's values (keys untouched — total, never collides) |
| `transform_keys` | `transform_keys map fn` | map over a map's keys (**can collide** — see §6) |

### Consumers (force — drive the iterator to a final value)

| name | signature | does |
|---|---|---|
| `fold` | `fold coll init fn` | the driver every other consumer is built on |
| `sum` | `sum coll` | add everything |
| `mean` | `mean coll` | arithmetic mean (forces float division) |
| `count` | `count coll pred` | how many satisfy `pred` |
| `length` | `length coll` | element count |
| `all` | `all coll pred` | every element satisfies `pred` (vacuously true on `[]`) |
| `any` | `any coll pred` | some element satisfies `pred` |
| `find` | `find coll pred` | first satisfying element, or `none` |
| `first` | `first coll` | first element, or `none` |
| `last` | `last coll` | last element, or `none` (never terminates on an infinite source — by design) |
| `max` / `min` | `max coll` | largest / smallest by natural order, or `none` |
| `argmax` / `argmin` | `argmax coll key` | the *element* maximizing / minimizing `key` |
| `range` | `range coll` | statistical spread, `max - min` (**name-collides with an int generator — see §6**) |
| `sort` | `sort coll` | ascending natural order |
| `to_list` | `to_list coll` | force to a concrete list |
| `to_h` | `to_h coll` | list of pairs → map |
| `group_by` | `group_by coll key` | buckets keyed by `key x`, keeping every collider |
| `tally` | `tally coll` | frequency map: each distinct value → its count (**core** — VSE vote-counting) |
| `index_by` | `index_by coll key` | key → element, unique keys (**collides on non-injective key** — see §6) |

## 5. Streams subsumed

`cycle` / `naturals` / `repeat` / `iterate` are ordinary adapters, **not** a
separate stream/generator construct. A pull chain never asks for the whole
thing, so an infinite iterator just works when a bounding consumer drives it:

```
cycle [1 2 3] . take 7 . to_list   # [1 2 3 1 2 3 1]
naturals . map (x -> x * x) . take 5 . to_list   # [1 4 9 16 25]
```

kanso is not a total language (it runs infinite reactive control loops — the
robotics target), so non-termination is a first-class capability, not a
forbidden state. `cycle` is legal because a bounding consumer terminates it.

## 6. Map collisions — one question, three places

`index_by`, `transform_keys`, and any map-building consumer face the same
question: **what happens when two keys land on the same slot?**

- `transform_values` / `group_by` never collide (values-only / keeps every
  collider in a bucket).
- `transform_keys` and `index_by` *can* — e.g.
  `transform_keys {"Name": a, "name": b} downcase` → both keys become `"name"`.

Answer it **once**, apply everywhere. Leaning: plain `put` semantics
(last-write-wins, caller owns key-uniqueness the same as when building any map),
documented not hidden — **but not yet gaveled** (last-wins vs raise is open).

## 7. Map-builders and mutation — the two doors

`tally` / `group_by` / `to_h` / `transform_values` are consumers that build a
map via `fold`. The efficiency comes from **Perceus in-place reuse**: the
accumulator is uniquely threaded (never aliased), so `put acc k v` mutates the
map in place and freezes when it leaves the fold — the *automatic* door to the
mutable-local-then-freeze idea. **Build blocks** (`build`/`set`) are the
*explicit* door for hand-written imperative accumulation. Same principle, two
surfaces; the stdlib walks through the automatic one. (See
[[kanso-memory-model-frontier]].)

## 8. Compilation

An adapter chain fuses (deforestation) + monomorphizes to a **single
tail-recursive scan** of the source — recursion is kanso's only loop, there is
no `for`. The adapter steps inline into the scan body; the consumer's
accumulator is uniquely threaded so Perceus mutates it in place:

```
[1 2 3] . map (x -> x*2) . select (x -> x > 7) . to_list
=>
scan src acc
  next src . (done      -> acc)
          . ((x, rest) -> y = x * 2
                          scan rest (if (y > 7) (push acc y) acc))
```

One multiply and one test per element, survivors pushed into an in-place
accumulator — no intermediate `[2 4 6]`, no thunk, one pass. Purity licenses the
fusion (no side effects to reorder); closed-world + monomorphization erase the
`next` indirection down to the raw scan.

## 9. Open items (not gaveled)

1. **`range` collision** — `range coll` (stat spread, `max - min`) vs a `1..n`
   int generator both want the name. Disambiguate: reserve `range` for the stat,
   spell the generator `naturals . take n` or a distinct `upto`.
2. **`transform_keys` / `index_by` collision** — last-wins-like-`put` vs raise
   (§6).
3. **`first n` vs `take n . to_list`** — is there a `first coll n` consumer
   convenience, or only `take` (adapter) + `to_list`? One-right-way says pick
   one.
4. **Lambda-in-pipe-stage syntax** — parens required (`map (x -> x * 2)`) so `.`
   chains cleanly instead of the lambda body swallowing it. Needs a lexer rule.
5. **The `next` primitive** — signature of the role method a type defines
   (`next state -> (elem, state) | done`), and how `fold`/consumers drive it.
