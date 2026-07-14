# memory-model committee — convened 2026-07-14 (overnight)

The virtual language-design committee (Sandi Metz, Kent Beck, Martin Fowler,
Avdi Grimm, Gary Bernhardt, Simon Peyton Jones) convened on the
constraint-optimality question for the no-runtime-memory-management direction
(compiler.html §10): **what language-constraint regime makes static
single-ownership decidable *and* ergonomic?**

Regimes on the table:

- **(A) full linearity** — every value used exactly once; a second use needs an
  explicit duplicate. Maximally decidable, heavy.
- **(B) affine + inferred borrow-vs-consume** — a value read (borrowed) freely,
  consumed (moved/stored/mutated) at most once; the compiler infers
  consume-vs-borrow.
- **(C) regions + value-indices** — replace pointer-based dynamic sharing with a
  copyable index into an explicitly-owned region (static lifetime). Fully
  static; cost is coarse (region-granularity) freeing.

> These are **simulated** experts: generative diversity of taste, **not**
> oracles. Every claim below is theirs; weigh it on merit, not authority. The
> one place that matters most — SPJ's historical claim — happens to check out
> against the real record; flagged inline.

---

## the convergence — where five or six agreed

**1. Reject (A) full linearity. Unanimous.** Every second-use forces an explicit
`dup`, which becomes reflexive wallpaper — the `.clone()`-spam / nil-check
failure mode already outlawed here, relocated one keystroke over. "Maximally
decidable, minimally humane" (Metz). Rejecting (A) is the *same argument* that
killed the blessed `copy` escape, applied one level up (Beck).

**2. Favor (B) affine + inferred borrow-vs-consume as the default.** Near-
unanimous. It's how people already reason — "I can look without asking, I can
only take it once" (Beck) — and it fixes the exact gap Linear Haskell left open:
Linear Haskell has no first-class *borrow*, only linear-consume or unrestricted,
so inferring borrow-vs-consume is precisely the ergonomic fix (SPJ).

**3. — THE BIG ONE — not ambient whole-program inference; a per-function
SIGNATURE.** Fowler, Metz, and Bernhardt converged independently; SPJ framed the
same point as modularity. Whole-program inference *with no printable artifact*
is **worse** than Rust: a body edit in module A silently flips an inferred fact
that rejects code in distant module Z, and the error points at a stranger. The
fix is the move type inference already made — infer a small ownership signature
*per function*, cache it like a principal type, check calls against the
signature, never re-derive from the callee's shifting internals.

- Fowler: *"never let ownership be inferred whole-program with no printable
  intermediate — it must bottom out in a per-function signature you can pin,
  diff, and blame."*
- Metz: *"legibility of the inference beats raw completeness, every time."*

> This does **not** contradict the closed-world edge — the analysis may be
> whole-program *internally*. What must be per-function is the **contract the
> user reasons about**. Same shape as kanso's inferred-but-surfaced types. The
> `ownership.rs` pass built tonight already computes exactly this signature.

**4. Rejection legibility is non-negotiable.** Every rejection must name the
*specific* competing use that broke uniqueness and suggest the *specific*
restructuring, in the user's vocabulary — never "borrow check failed," never
region/index/rc vocab. Avdi: *"a rejection with no legible cause, or whose only
escape is reflexive duplication, is nil laundered through the type checker."*

**5. Core/shell hygiene (Bernhardt).** The ownership vocabulary — region, index,
rc, borrow-mode — must never appear in a surface type, an error message, or a
performance mental model. *"The instant a programmer has to think about a region
to predict speed or legality, the shell has punctured the core, and you've
reinvented lifetimes with the serial numbers filed off."*

---

## the critical dissent — SPJ, and it contests tonight's "can't-do"

The type-theory member delivered the one statement that directly challenges the
`can't-prove ⟹ can't-do` stance now on the compiler page. It is the most
important thing the committee produced.

- **Uniqueness is semi-decidable — unlike kanso's other rejections.** Dispatch
  coverage, shadowing, unused bindings sit on decidable-*and-complete*
  properties with a proven-empty spurious-rejection class. Uniqueness does not:
  proving "no other live path reaches this heap cell" through recursive data and
  higher-order storage is a shape/alias problem, undecidable in general, so any
  sound *decidable* analysis is incomplete somewhere. "It's a compile error like
  any other type error" is a **category error** — it is not the same kind of
  *no*. (Orthogonal to closed-world: closed-world removes the *annotation* need,
  not the *incompleteness*.)

- **He built this and watched it fail.** Wansbrough & Peyton Jones, *"Once Upon a
  Polymorphic Type"* (POPL 1999): polymorphic usage types, **inferred not
  annotated**, for update avoidance in GHC. Worked on paper and in the
  prototype; never shipped as a durable optimization, because whole-program
  usage inference is **fragile** — a small, semantically irrelevant change at a
  distant call site flips an inferred usage fact and silently degrades or
  rejects code the programmer had every reason to believe was fine. *That*
  fragility, not the type theory, is why Linear Haskell (a decade later) went
  annotated-and-checked instead of inferred.
  > VERIFIED-PLAUSIBLE: the paper is real (POPL '99), Linear Haskell (Bernardy
  > et al., POPL 2018) is annotated arrow-multiplicity checked-not-inferred, and
  > "usage inference is fragile" is the standard account. Worth a direct read
  > before quoting as settled, but it aligns with the record.

- **His recommendation: (B) default, incomplete residual to (C) — NOT
  rejection.** *"Never let 'the analysis couldn't prove it' silently become 'the
  program is rejected' with no escape — that one design choice is where kanso
  would inherit the exact fragility that killed whole-program usage inference in
  GHC."* Route the residual into region/arena bulk-free (C), which sidesteps the
  undecidable question by changing its *granularity* instead of answering it;
  and/or keep RC as the coordination device for the residual (Perceus never
  attempted RC-free — it kept the header precisely for this). *"Going further
  than Perceus is a genuinely open research question, not an engineering backlog
  item."*

Beck and Bernhardt land nearby independently: Beck — *"don't ratify a regime,
ratify an experiment; ship naive (B), run it on real code, let spurious
rejections be failing tests"*; Bernhardt — *"route the residual to regions, but
the region vocabulary must never surface."*

---

## what this means for the "can't-do" pivot

Fowler flagged the live contradiction directly: §09 promises "no rejected
programs," §10 rejects — unreconcilable by wordsmithing; the page needs one
honest position. The committee's resolution is **not** §10's "reject the
residual." It is a third position, between hard-reject and RC-fallback:

> **Prove borrow-vs-consume where the per-function signature analysis can (B),
> and route the incomplete residual to a STATIC region fallback (C) — bulk-free,
> no runtime count — rather than hard-rejecting it.**

That keeps the crown-jewel property (no runtime reference counting; regions are
static) while dodging the GHC fragility (no spurious rejection of code a distant
edit broke — the residual gets a graceful static home, not a wall). "Can't-do"
survives only for the genuinely-unroutable cases; the *default* residual path is
regions, not rejection.

If Clay wants the purest "can't-do" (reject the residual, no escape), SPJ's
warning is the cost being accepted — from someone who paid it once.

---

## each member's one thing (distilled)

- **Metz:** the rejection boundary must be statable in one sentence a programmer
  can check *before* compiling — legibility over completeness.
- **Beck:** ratify an *experiment*, not a regime; no new construct earns a place
  until a real program fails twice for the same reason.
- **Fowler:** never infer ownership whole-program with no printable
  intermediate — bottom out in a per-function signature you can pin, diff, blame.
- **Avdi:** every rejection names the specific competing use *and* the specific
  remedy — else it's nil laundered through the type checker.
- **Bernhardt:** the ownership vocabulary must never surface in a type, an error,
  or a performance mental model.
- **SPJ:** never let "couldn't prove it" silently become "rejected" with no
  escape — that's the fragility that killed whole-program usage inference in GHC.

---

## for arbitration (Clay decides)

1. **The central fork — "can't-do" vs "static-region-fallback."** Purest
   ruthlessness (reject the residual; SPJ's warned-against choice, which inherits
   the GHC fragility) versus the committee's route-residual-to-static-regions
   (no runtime RC, no spurious-rejection fragility, but regions become a real
   language surface with coarse-freeing cost).
2. **Per-function signatures as the contract** — looks settled across the room;
   `ownership.rs` already computes it. The user reasons about a printable
   borrow/consume signature per function; refactoring checks against signatures,
   not shifting internals.
3. **Regions' real cost** — if the residual routes to regions, coarse
   (region-granularity) freeing can bloat long-lived shared state
   (Bernhardt/Beck). Empirical; the self-hosting compiler is the rig.
4. **§09 ↔ §10 reconciliation** (Fowler) — the compiler page needs one honest
   position, not the two it currently carries.

Status: advisory input for real Clay's ratification. Not gaveled.
