# A self-contained wasm runtime — draft 0.1

Status: design only. Nothing here is implemented; the browser backend today
emits a *program* module whose every value is an i32 handle into the
toolchain module's host-side registry (`src/wasm_rt.rs`), which reuses the
interpreter's `Value` so semantics stay oracle-identical. This note
enumerates the runtime surface that emission depends on, so a future
self-contained module (no toolchain instance, no registry round-trips) can
be scoped precisely. The differential harness
(`scripts/browser_differential.sh`) is the contract keeper: any replacement
runtime must keep the golden corpus byte-identical to the native engine.

## int representation — GAVELED

`int` is arbitrary precision on every engine. Semantics, non-negotiable.

Performance is served by tiered function versions, not by narrowing the
type: the compiler emits a speculative raw-i64 version of each function
(today's fast codegen unchanged) whose overflow checks bail out and restart
the call in a second version of the same source compiled against heap
bignums. The bailout is an internal signal, never a language-level value;
the restart is sound because kanso functions are pure. Consequences for
this runtime:

- The standalone module needs a real bignum implementation in wasm — not
  int64-with-trap, and not a host callout.
- Every function in the emission plan below eventually exists in two
  compiled versions (fast i64 + bignum), with bail-and-restart between
  them. The first cut of the self-contained runtime can ship bignum-only
  (correct, slower) and add the fast tier later; the tiering changes
  codegen shape, not the runtime surface.

## The rt_* surface (26 operations)

What the emitted program module imports today, grouped by the `Value`
machinery each group needs. Constants and signatures live in
`src/wasm_backend.rs` (`imports()`); implementations in `src/wasm_rt.rs`.

### Predicates and dispatch checks
| op | needs |
| --- | --- |
| `rt_is_failure(h)` | err/none tags on values (railway short-circuit) |
| `rt_eq_lit(h, lit)` | structural equality on int (bignum compare), string, bool, none |
| `rt_check_type(h, code)` | runtime type tags: int/float/string/bool/list/map/err + record type ids |
| `rt_check_rec(h, tid, nfields)` | record type id + field count |
| `rt_check_err(h)` | err tag |
| `rt_truthy(h)` | bool tag, error rendering for the non-bool diagnostic |

### Construction
| op | needs |
| --- | --- |
| `rt_arg(h)` / argument stack | a scratch stack for variadic construction |
| `rt_mklist(n)` | heap list of values; failure propagation over elements |
| `rt_mkmap(n)` | **sorted** map (BTreeMap order: int keys then string keys, each ascending) — render and iteration order are observable |
| `rt_mkrec(tid, n)` | record = type id + field vector |
| `rt_mkerr(h)` | err wrapper; wrapping a failure is identity |
| `rt_mkclosure(tidx, ncap)` | closure = table index + captured environment |
| `rt_template(n)` | string render of every value class (see render, below) |

### Destructuring
| op | needs |
| --- | --- |
| `rt_field(h, i)` | positional field read |
| `rt_err_inner(h)` | err payload read |
| `rt_keyed_check(h, entries)` | field-count-vs-declared check (keyed reads must omit a field) |
| `rt_keyed_field(h, name)` | field lookup by name via the type table |

### Arithmetic, indexing, calls
| op | needs |
| --- | --- |
| `rt_binop(op, a, b)` | full `eval_binop`: **bigint arithmetic** (+ - * / and comparisons), float arithmetic, string concat/compare, division-by-zero as `err`, mixed-type diagnostics |
| `rt_index(base, idx)` | list/map lookup; a miss is `err "missing index i"` (strict — unlike `at`, whose miss is `none`) |
| `rt_call(callee, n)` | closure invocation; calling a failure returns it; calling a non-closure dies |
| `rt_envget(env, i)` | environment/argument-pack read |
| `rt_list_len(h)` | length of a list or argument pack (arity dispatch in fn-as-value wrappers) |

### Effects
| op | needs |
| --- | --- |
| `rt_builtin(name, n)` | the interpreter's builtin table: `print`, `at`, `sort`, `sum`, `join`, `map`/`filter` (which drive compiled closures back through `k_callback`), string builtins, `args`/`stdin`/`read_file`/`write_file` descriptions |
| `rt_seq(a, b)` | description sequencing (`>>`); failure short-circuit |
| `rt_maybe_bind(piped, closure)` | pipe-as-bind: descriptions bind, plain values apply immediately |
| `rt_die(msg)` | runtime error channel (message out, trap) |

Plus the one *import into* the program module: `k_callback(tidx, env, args)`
— the trampoline by which host-driven iteration (map/filter, desc
continuations) calls compiled closures. A self-contained module replaces
this with a direct `call_indirect` through its own table.

## Value operations the runtime must own

Distilled from the table above — this is the porting checklist:

1. **Bignum arithmetic** (gaveled above): add/sub/mul/div, compare, render.
2. **String render** of every value class, exactly matching
   `eval::render` in both modes (bare and quoted) — templates, error
   messages, and endpoint diagnostics are all byte-compared by the harness.
3. **Sorted map** with the interpreter's `MapKey` order (ints before
   strings), since entries/render order is observable.
4. **List ops**: construct, length, index, and the builtin surface
   (`sort` with the interpreter's cross-type ordering, `sum`, `map`,
   `filter`, `at`, `slice`, `join`, ...).
5. **Desc executor**: the `Desc` tree (print/args/stdin/read/write,
   `Seq`, bind continuations) walked at the endpoint, with the endpoint
   rules — unhandled `err`/`none` reaching main is exit 1 with the exact
   `error[endpoint]:` message.
6. **Failure railway**: err/none tags checked at every construction and
   call boundary, propagating operands verbatim.

## Single-module emission plan

Target shape: one emitted `.wasm`, no imports except the host effect shims
(`print`, `args`, `stdin`, `read_file`, `write_file`), runnable in any
engine.

- **Fixed prelinked runtime preamble**: the runtime above compiled once
  (from Rust or hand-emitted) into a function region occupying indices
  `0..R`. Emission links against it by constant index — no relocation.
- **Program functions at offset indices**: the program's dispatchers,
  wrappers, and lambdas land at `R..R+n`, exactly today's layout shifted
  by `R`; closures become `call_indirect` through the module's own table
  (retiring `k_callback`).
- **Literals as data segments**: ints (bignum limbs), floats, and strings
  move from compile-time registry pre-registration into data segments read
  by an init function; literal handles become pointers into linear memory.
- **Tiered function versions** (per the int gavel): each source function
  eventually compiles twice — a speculative raw-i64 version whose overflow
  checks bail, and a bignum version the bailout restarts into. Bailout is
  an internal control path (a flag or alternate return arc), never a
  value; purity makes the restart transparent. Ship bignum-only first,
  add the fast tier behind the same differential harness.
- **Value representation**: handles stop being registry indices and become
  tagged pointers into module-owned linear memory with its own allocator
  (bump + Perceus-style reuse is the eventual frontier; see
  `design/performance-frontier.md`).

Migration is verifiable at every step because the harness pins the whole
corpus to native-engine bytes: swap one rt group at a time from
registry-backed to module-owned, run `scripts/browser_differential.sh`,
and a divergence names the exact case.
