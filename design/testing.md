# Testing — where it is, and where Clay wants it

## Today

A test is a constant whose name begins `test_`, evaluated by `kanso test`,
and the whole ceremony is those five characters. It scales further than it
looks: the json suite pins escapes, unicode, and error positions with
nothing else. What it cannot express is shared setup, grouping, or a
readable failure narrative — every test stands alone and repeats its own
arrangement.

One thing it could not do at all, until 2026-07-29: assert about a
failure the package itself raised. An assertion is a value, and the
two-universe rule forbids a package producing a value from its own err
(design/pending-gavels.md item 1). The second of the two recorded routes
was taken, and it needed no exemption: `failed?` is a builtin, so the
reading is done by the toolchain rather than by the package, and
attribution proves it — an err is only ever rescued where a *pattern*
names it, and a builtin has no pattern for a package to be blamed for.

It is in scope in a `_test.kso` file and nowhere else, gated by the file
that declares the caller, the way `builtin_` names are gated to std. A
production file naming it is refused, so the rule stands undiminished
rather than carrying a hole a program could reach for. Gating on the
file rather than on the verb keeps `kanso check` honest, since check
compiles test files too, and a name that existed under one verb and not
another would be a worse thing to explain than where it lives.

`failed?` is also the second hole in err's infectiousness, beside
`wrap_err`: it exists to look at a failure, so it is asked before
propagation answers for it.

What is still owed is the reason a test wants more than a boolean —
asserting *which* failure, not merely that one happened. That wants a
way to read a reason, and it is the natural next slice.

## Where Clay wants it (2026-07-28, noted for the far queue)

An rspec/ginkgo-shaped surface, deliberately constrained so it cannot grow
the nesting those frameworks accumulate:

- **one outer `describe`** per file, not many;
- **one level of `context` inside it**, and no deeper — contexts do not
  nest within contexts;
- **assertions at either level**: directly under the describe, or under a
  context;
- **no further nesting levels**, ever. The shape is two deep and stops.
- something like ginkgo's `JustBeforeEach` — setup that runs after the
  context's own arrangement, so a context refines what the describe set up
  rather than repeating it;
- possibly an equivalent of rspec's `let` with the same one-level
  override, if it can be had without laziness surprises. Clay: "maybe
  impossible, but it's something i want to keep in mind."

The constraint is the point. Deep nesting is what makes a large rspec file
unreadable — a failing example three contexts down requires reconstructing
its setup from four places. Two levels can be read from one screen.

Not scheduled. Recorded so the shape survives the queue.
