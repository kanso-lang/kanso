# Perceus / reference counting — commit plan

Goal: replace the never-free bump arena with precise reference counting, then
add reuse-in-place, closing the copy-driven gap to serde and bounding memory
(long runs currently degrade — per-decode time rises from ~1.88ms at 150
decodes to ~2.0ms at 4000 as the arena bloats).

Scope: **native backend only** (`src/runtime.c` + `src/codegen.rs`). The
interpreter and wasm backend use Rust's own ownership and need no changes; the
differential lattice verifies native output stays byte-identical throughout.

Enabling fact: **codegen treats heap objects as opaque** `KValue = {tag,
payload}` and only calls runtime functions — zero struct-offset knowledge. So
object layout (adding a refcount) is entirely runtime-private.

The discipline that keeps every commit shippable: **never free until the
counts are proven correct.** Under-retaining (freeing too early) corrupts;
over-retaining (leaking) is safe. So we build up correct dup/drop placement
while `k_drop` is a no-op, then flip freeing on only once counts balance.

## commits

1. **rc infrastructure, no freeing** (runtime-only, byte-identical).
   Object header `{ i64 rc }` before each heap allocation; `k_dup`
   (rc++), `k_drop` (rc--, free is a NO-OP for now). All object allocations
   route through an rc-aware allocator; rc starts at 1. Declare `k_dup`/
   `k_drop` in codegen's DECLARES but emit no calls. Verify: byte-identical,
   full suite green, benchmark unchanged.

2. **emit dup (over-retain)** (codegen).
   codegen emits `k_dup` when a temp is consumed more than once. No drops yet
   → leaks, but correct and safe (nothing frees). Verify byte-identical.

3. **emit drop at last use** (codegen).
   Perceus calling convention: **arguments are owned/consumed by the callee**;
   a function drops each owned value not returned or passed on. codegen emits
   `k_drop` at last use / branch-join / function exit. Counts now balance.
   `k_drop` still no-op-frees. Add an assertion build where `k_drop` aborts on
   rc underflow — catches misplacement as a clean assert, never a corruption.
   Verify byte-identical + assertion build clean across the suite.

4. **enable freeing + free-list allocator** (runtime).
   `k_drop` frees at rc==0 into a size-bucketed free-list; `k_alloc` pops the
   free-list first. Shared buffers (list `KBuf`, map pairs) get their own rc so
   the watermark-shared item arrays free only when no version references them.
   Verify byte-identical + ASan clean + memory bounded (arena-bloat gone; long
   runs stop degrading).

5+. **reuse in place, one builder per commit** (runtime), each A/B-measured:
   - `k_rec`: rc==1 input record → mutate fields in place.
   - `k_b_put`: rc==1 input map → append/overwrite in place (with the
     append-then-sort scheme so inserts stop being O(n) copies).
   - `k_b_push` fallback: already watermarked; rc lets the fallback reuse too.
   This is the speed win — the profiler's `memmove` hot spot is these copies.

## verification harness (every commit)

- `cargo test --release` (includes the native↔interpreter differential)
- `./target/release/kanso test lib/json` (16/16)
- `bash scripts/browser_differential.sh` (wasm↔native, unaffected but proves it)
- benchmark A/B where a commit claims a speed change
- from commit 4: an ASan build over the golden corpus to prove no use-after-free

## commit 2 — worked-out contract (borrow-default)

Ownership rules, decided:
- **User function calls BORROW their args** (callee doesn't own params; the
  caller keeps ownership and drops at scope exit). So a variable passed to a
  user fn is NOT dup'd.
- **Constructor builtins CONSUME what they store** — no runtime change needed
  (they just keep the ref they're given); codegen dups the arg if it's a
  variable. Constructors: k_rec, k_mklist, k_b_push, k_b_put (key+val), k_err,
  k_closure, k_mkdesc/k_seq, list/map literals.
- **Accessor builtins DUP the inner value they return** (runtime change):
  k_field, k_err_inner, k_keyed_field, k_b_at (list-item and map-value returns
  only — fresh k_str_n/k_int/k_none returns don't dup), k_b_entries (dups the
  key/val it stores into entry records), k_b_slice (dups shared list items),
  k_b_sort / k_b_map / k_b_filter (produce new lists sharing item values → dup
  each shared item). Audit every k_b_* that exposes a value from inside a
  container.
- **Borrow builtins untouched:** k_check_*, k_render, k_eq, k_length, k_truthy,
  arithmetic, comparisons, k_type_name.

codegen (the key simplification): kanso function bodies are FLAT — all bindings
are top-level Stmts, `if` appears only in tail position with single-expression
branches (no nested binding scopes; Expr has no Block variant). Therefore
**every local is in scope at every return point**, so drop placement is just
"drop all owned locals before each ret" — no per-path liveness needed.
- Track owned locals in FnEmit (Pattern::Var binds + destructured field vars).
  Params are NOT owned (borrowed from caller).
- **Consuming read of an Expr::Ident → dup** (fresh temps transfer without dup;
  only variables, co-owned by their binding, get dup'd). Consuming positions:
  constructor args, return/tail value, list/map literal elements, `x = y`
  where y is a variable.
- **Before every return** (emit_tail's piped-ret, if-branch rets, the musttail
  call — drops go BEFORE it so TCO's call/ret adjacency is preserved — the
  fallback ret, and the dispatcher's pass-through rets): emit k_drop for each
  owned local. NOT before k_die/unreachable.
- k_drop stays non-freeing; add assert `rc >= 1` before decrement to catch
  under-retain (rc going negative) as a clean abort.

Validation for commits 2-3: byte-identical output (lattice) + no assertion
abort across the whole suite + benchmark (expect a temporary slowdown from rc
traffic, recovered by freeing/reuse in commits 4-5).
