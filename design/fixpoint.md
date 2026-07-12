# the inference fixpoint — working draft for attack

status: draft 0.1, unproven. this document exists to be broken. it formalizes spec §14.1: the mutual dependence of type inference, pass-through generation, and dispatch resolution, and the conditions under which a program is accepted.

## 0. the circularity, stated precisely

three facts create the loop:

1. dispatch resolution needs types: which overload of `f` a call site targets depends on the inferred type set of its arguments.
2. types come from bodies: a function's return set is computed from the body that dispatch selects.
3. pass-through generation changes return sets: for any type an overload group doesn't handle, the compiler adds an identity pass-through, so the argument's type set leaks into the return set.

so: sets determine dispatch, dispatch determines bodies, bodies determine sets. naively iterating this can oscillate. the spec's demand: compute propagable sets as a monotone least fixpoint, then resolve dispatch, and reject programs where resolution would feed back into the sets.

## 1. definitions

- **U** — the universe of concrete types in a program: every declared record type, every primitive (`int`, `string`, ...), the nullary types (`true`, `false`, `none`), and `err d` for each distinct reason type `d` constructed anywhere. U is finite for a finite program.
- **typeset** — a subset of U, closed and flattened at declaration (spec §4). `bool` = {`true`, `false`}.
- **type set of an expression** `S(e) ⊆ U` — the set of concrete types the expression may inhabit.
- **overload group** `G(f)` — the declared overloads of name `f`, each with a pattern vector; ranked literal > concrete > generic (spec §5).
- **return set** `R(f, τ̄)` — the set of types `f` may return when applied to argument type vector `τ̄` (monomorphized: computed per concrete instantiation, mirroring the no-polymorphic-recursion ban).
- **candidate set** `C(f, S̄)` — the overloads of `f` that could match *some* concrete type vector drawn from the argument sets `S̄ = (S₁, ..., Sₙ)`, plus the implicit pass-through when some vector matches no overload.

## 2. the central danger: exact dispatch is not monotone

this is the technical heart, and the reason a one-phase algorithm is wrong.

suppose `f` has a generic overload returning `string` and an `int`-specific overload returning `float64`. if an argument's set grows from `{string}` to `{string, int}`, exact dispatch for the `int` portion *shifts* from the generic body to the specific body. the union of possible returns changes non-monotonically in general: growing an input set can *remove* types from an exactly-dispatched return set (the generic body no longer runs for `int`). a fixpoint iteration over a non-monotone function has no Kleene guarantee — it can cycle.

## 3. the two-phase algorithm

**phase A — monotone over-approximation.** define the transfer function F over the product lattice of all `S(·)` and `R(·, ·)` (powersets of U, ordered by ⊆, finite height):

- at each call site of `f` with argument sets S̄, take the return contribution as the union over *every* candidate in `C(f, S̄)` — every overload consistent with any drawable vector, plus pass-through contributions (each unhandled type flows through identity: it joins the return set unchanged).
- `C` itself is monotone in S̄ (growing argument sets can only add candidates — candidacy is per-vector matching, never exclusion), and union over more candidates only grows results. F is therefore monotone; by Knaster–Tarski/Kleene the least fixpoint `lfp(F)` exists and iteration from ⊥ terminates in at most |U| × (number of set variables) steps.

**phase B — exact resolution.** with `lfp(F)` fixed, resolve every call site exactly: for each concrete argument vector τ̄ ⊆ the phase-A sets, select the unique most-specific matching overload (overlap within a rank is already a compile error, spec §4/§5). recompute all sets under exact dispatch; call the result `exact`.

**acceptance criterion (the rejection class).** the program is accepted iff phase B is *self-consistent under its own answer*: `exact ⊆ lfp(F)` (guaranteed — exact selects one candidate where A united several) **and** re-running exact resolution against the `exact` sets selects the same overload at every call site. if shrinkage changes any dispatch target — the feedback the spec forbids — the program is rejected, with a diagnostic naming the call site and the two competing overloads.

conjecture 3.1 (needs proof or counterexample): one verification round suffices — if re-resolution against `exact` is unchanged, further shrinkage cannot change it. intuition: dispatch depends only on set membership of argument types, and `exact` re-runs remove types monotonically downward, but specificity means *removal* can also shift dispatch (generic loses its only inhabitant). a cycle of length > 1 in the verification step would be a program that phase B accepts and rejects alternately; find one or prove it impossible. **this is the open lemma.**

## 4. pass-throughs inside the framework

the pass-through is not special: it is a virtual generic overload of every function, at rank below generic, whose body is the identity on the (leftmost, for multi-argument calls — spec §2) context-carrying argument. phase A includes it as a candidate whenever some drawable vector matches nothing declared; phase B selects it exactly for the vectors no declared overload matches. the endpoint rule is then a constraint on the fixpoint, not a separate mechanism: a constructor type `c` reaching a chain endpoint unhandled is `c ∈ exact-R(main)` for `c ∈ {err d, ...}` → compile error, with the provenance chain read off the fixpoint derivation.

## 5. holes (partial application) as deferred dispatch

`foo a, _, c` where `foo` has arity-3 overloads denotes a function value. its type is the *residual overload set*: candidates of `G(foo)` filtered by the known argument sets at positions 1 and 3, each residual keyed by the hole position's pattern. application of the partial at a fill site supplies `S(hole)` and dispatch completes there, inside the same phase A/B machinery — a hole is a call site whose argument set arrives late. consequences to verify:

- monotonicity is preserved: residual filtering is per-vector candidacy, same as `C`.
- the rejection class extends unchanged: a fill site whose exact resolution disagrees with phase A's residual support rejects the program.
- arity stays semantic: a hole is an argument; `foo 1, _, 8` is unambiguously `foo/3`. no collision with the overload-vs-partial ambiguity that bans implicit currying (a bare `foo 1` against a `foo/3` remains an arity error).

## 6. superfluous annotations as a post-fixpoint check

the annotation doctrine, inference-relative form (gaveled 2026-07-11): an annotation is legal iff deleting it changes `exact` — the fixpoint, any dispatch target, or the acceptance verdict. concretely, for each annotation: recompute with the annotation erased; if the result is identical, the annotation stipulates nothing → compile error. this subsumes both cases: the body-derived case (`display word` with a sole `display` already pins the type) and the coverage case (a typeset guard whose members are all covered by called overloads). naive cost is one re-inference per annotation; incremental (salsa-style) recomputation should make it a cheap delta. kanso programs are annotation-sparse by construction, which keeps n small.

## 7. worked adversarial example (to be extended)

```
fn f (x: int)
  "wide"

fn f x
  x

fn g x
  f x
```

seed `g` with `S(x) = {none}`... phase A: candidates of `f` for `{none}` = {generic} (generics never bind failure types — so actually {pass-through}); `none` flows through. now seed `S(x) = {int, none}`: A unites {int-overload → string, pass-through → none}. exact B: `int` → "wide" branch (string), `none` → pass-through. stable; accepted; `R(g) = {string, none}`. the adversarial cases to add: (a) recursive `f` whose return set feeds its own dispatch through a second function; (b) a program engineered so exact shrinkage flips a generic to dead — candidate for the conjecture-3.1 counterexample hunt.

## 8. explicitly out of scope for draft 0.1

polymorphic recursion (banned, rust-style — makes monomorphization terminate), effect-set inference (a separate, simpler fixpoint: effect sets only ever union, trivially monotone), and `build`-region slot checking (reuses set-tracking but no dispatch feedback).
