# Testing — where it is, and where Clay wants it

## Today

A test is a constant whose name begins `test_`, evaluated by `kanso test`,
and the whole ceremony is those five characters. It scales further than it
looks: the json suite pins escapes, unicode, and error positions with
nothing else. What it cannot express is shared setup, grouping, or a
readable failure narrative — every test stands alone and repeats its own
arrangement.

One thing it cannot do at all: assert about a failure the package itself
raised. An assertion is a value, and the two-universe rule forbids a
package producing a value from its own err (design/pending-gavels.md
item 1). Two ways out, both open:

- exempt `*_test.kso` from the rule (one line, crude, and it is a
  file-scope hole in a language rule);
- give assertions a toolchain surface — the harness is not the package,
  so a builtin that reads a failure is a foreign party rescuing, legal
  under the rule as written and needing no exemption.

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
