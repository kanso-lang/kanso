# the inference fixpoint — draft 0.2

status: the central conjecture of draft 0.1 is resolved — dissolved, not proven. the two-phase algorithm collapses to a single monotone fixpoint, and the anticipated rejection class is empty. this document exists to be broken; the load-bearing result is theorem 2.1.

## 0. the circularity, stated precisely

three facts create the apparent loop:

1. dispatch resolution needs types: which overload of `f` a call site targets depends on the types of its arguments.
2. types come from bodies: a function's return set is computed from the body that dispatch selects.
3. pass-through generation changes return sets: for any type an overload group doesn't handle, the compiler adds an identity pass-through, so the argument's type set leaks into the return set.

so: sets determine dispatch, dispatch determines bodies, bodies determine sets. draft 0.1 feared this made the transfer function non-monotone and required an over-approximate phase, an exact phase, and a class of rejected programs. that fear came from framing dispatch as a function of *sets*. the spec's own dispatch rule — "resolution is fully static **per monomorphized instantiation**" (§5) — makes it a function of *vectors*, and that changes everything.

## 1. definitions

- **U** — the universe of concrete types: every declared record type, every primitive, the nullary types (`true`, `false`, `none`), and one `err@site` per syntactic `err`-construction site (see §4 — site-indexing is what keeps U finite). U is finite for a finite program.
- **typeset** — a subset of U, closed and flattened at declaration (spec §4). `bool` = {`true`, `false`}.
- **type set** `S(e) ⊆ U` — the concrete types expression `e` may inhabit.
- **vector** τ̄ — a concrete type assignment to a call site's arguments, drawn from the product of the argument sets.
- **overload group** `G(f)` — the declared overloads, ranked literal > concrete > typeset > generic (spec §5), plus one virtual pass-through overload below generic: identity on the leftmost context-carrying argument (spec §2).
- **vector dispatch** `D(f, τ̄)` — the set of overloads that can fire for concrete vector τ̄: the unique most-specific type-level match, together with any literal overloads consistent with τ̄ (literal selection happens on values, which type analysis over-approximates by including both the literal body and its same-arity successor). **D depends only on f's declared overloads and τ̄** — not on any inferred set.

## 2. the theorem that replaces the conjecture

**theorem 2.1 (dispatch is context-free, so exact inference is monotone).**
define the transfer function F: at each call site of `f` with argument sets S̄, contribute `⋃ { R(g, τ̄) : τ̄ ∈ ΠS̄, g ∈ D(f, τ̄) }` to the call's result set, where R(g, τ̄) is the return set of overload g's body under the bindings τ̄ induces. then F is monotone, and inference is a single Kleene fixpoint with **exact** dispatch — no over-approximation phase, no verification round.

*proof sketch.* let S̄ ⊆ S̄′ pointwise. every vector drawable from S̄ is drawable from S̄′, and D(f, τ̄) is a function of τ̄ alone (definition 1), so every term of the union under S̄ appears verbatim under S̄′. growing sets adds vectors; it never re-dispatches an existing vector. F(S̄) ⊆ F(S̄′). monotone on a finite-height lattice ⇒ least fixpoint exists and iteration from ⊥ terminates. ∎

**where draft 0.1 went wrong.** its §2 example — growing `{string}` to `{string, int}` "shifts dispatch from the generic body to the int-specific body" — described a shift *of the set*, not of any vector. the vector `(string)` dispatches to the generic body before and after; the vector `(int)` is *new* and brings the specific body with it. nothing is removed. return sets only grow: `{string}` becomes `{string, float64}`. the non-monotonicity was an artifact of set-indexed dispatch, which the spec never mandated.

**corollary 2.2 (the rejection class is empty).** the spec reserved the right to reject "programs where dispatch would feed back into sets." under vector-indexed dispatch there is no feedback channel: dispatch never reads an inferred set. every program that passes the independent static checks (overlap within rank, ownership/coherence, endpoint rule) has well-defined inference. no developer ever hits a "your program confused the fixpoint" wall — a wall that, per the acceptance criterion of draft 0.1, would have been unexplainable at exactly the moment it appeared.

**corollary 2.3 (draft 0.1's conjecture 3.1, for the record).** in the old two-phase framing, if re-resolution after exact recomputation changes no dispatch target, the pair (sets, dispatch) is a fixed point by construction, so no further round can differ — the "one round suffices" question was well-posed but moot: by theorem 2.1 phase B computes the same fixpoint phase A does.

## 3. what remains genuinely load-bearing

monotonicity was never the only obligation. the surviving proof obligations:

1. **finiteness of U** — handled by site-indexing `err` (§4). without it, `fn f x` with body `err x` fed back into itself would mint `err int`, `err err int`, ... and the lattice loses finite height.
2. **termination of monomorphization** — the vector product ΠS̄ is finite per call site, but instantiation of generics must not regress: adopt the rust-style polymorphic-recursion ban (spec §15) so the set of monomorphic instantiations is finite. proof obligation: the ban implies a finite instantiation closure. standard, but must be written for kanso's overload semantics.
3. **literal dispatch over-approximation is sound** — type analysis cannot split `fact 0` from `fact n`, so D includes both bodies for `(int)` vectors. obligation: this only ever *widens* return sets (soundness), never drops a type (completeness of the set analysis, which the endpoint rule relies on for its no-false-negatives guarantee).
4. **return-type-directed dispatch stays static** — `decode: config` (spec §5) selects on a *declared* annotation at the call site, not an inferred set; it adds no feedback channel. obligation: confirm the annotation-legality rule (legal exactly where context can't infer) never makes legality itself depend on the fixpoint result in a circular way — note the interaction with §6.

## 4. err, site-indexed

each syntactic `err`-construction site contributes one element `err@k` to U, carrying a payload set variable `P(k) ⊆ U`. nesting is reference, not new types: `err@1` whose payload may be `err@2` is two universe elements and an edge, however deep the runtime nesting. dispatch on the type `err` matches every `err@k`; a future fine-grained failure type (`timeout`, `parse_failure d`) is an ordinary record type and needs none of this. bonus: site-indexing *is* provenance — the endpoint diagnostic can name the construction site because the type carries it.

## 5. holes (partial application) as late-arriving vectors

`foo a, _, c` denotes a function value typed by the *residual*: the pairs (g, τ̄-with-gap) for g ∈ G(foo) consistent with the known positions. a fill site supplies the missing component, completing vectors that flow through D unchanged. monotonicity is preserved because residual filtering, like D, is per-vector. arity stays semantic: a hole is an argument, so `foo 1, _, 8` is unambiguously `foo/3`, and the overload-vs-partial ambiguity that bans implicit currying never arises.

## 6. superfluous annotations as a post-fixpoint check

the inference-relative annotation doctrine (gaveled 2026-07-11): an annotation is legal iff deleting it changes the fixpoint, any dispatch target, or a diagnostic. computed as a post-pass: re-infer with the annotation erased; identical result ⇒ compile error (annotation stipulates nothing). this subsumes body-derived redundancy (`display word` with a sole `display`) and coverage redundancy (a typeset guard whose members are all covered). obligation from §3.4: erasing a *return-type* annotation can change dispatch legality elsewhere; define erasure order (one at a time against the original program, not cumulatively) so the check is well-defined.

## 7. worked examples (to be extended into the test corpus)

the §2 example, both directions; the recursive-err program of §4 (terminates only under site-indexing — a good adversarial fixture); mutual recursion where `f`'s pass-through feeds `g` and vice versa (exercise: the fixpoint is reached in two rounds, no oscillation possible); a hole whose fill site completes a vector that dispatches to a pass-through (partial application of a function to a failure value).

## 8. explicitly out of scope for draft 0.2

effect-set inference (a separate, trivially monotone union-only fixpoint), `build`-region slot checking, and mechanization. mechanizing theorem 2.1 in a proof assistant is deliberately deferred until the operator/destructuring gavels stabilize the term language.
