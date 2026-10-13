

## 2026-10-10 — gavel: `build` retires, and `tie` is the one way to write a cycle

Clay, on the ledger entry "Does `build` still earn its place beside `tie`?":
"retire build". The entry recommended it once `tie` was built and its
compile-time check on literal data was pinned, and both are on main: `list/tie`
runs on every engine (log, 2026-10-05), a broken link in literal data is
refused at check, and ten thousand tied rings dropped in a loop hold one arena
block.

**What leaves the language.** The `build` block, the hole `_` that only a
`build` block could fill, the field write that filled it, and the
`error[build]` family that policed all three. A cycle is written with
`list/tie` or `list/tie!`, whether its nodes are two names in the source or a
graph read from a file. The 2026-10-01 gavel's reason for there being no loop
construct is unaffected, and so is the top-level constant knot
(`ring = node 1 ring`), which is a separate construct and is not part of this
ruling.

**Why.** `tie` does everything `build` did. With literal keys and links the
compiler sees the whole graph and refuses a broken link before the program
runs, which was the one thing `build` caught earlier, and a tied call's nodes
are one birth cohort, so the birthday theorem and the cohort's memory
guarantee hold as they did for a block. Two spellings of one thing is what
kanso declines to offer.

Owes: the parser, the checker and all three engines drop `build`, `_` and the
field write; the `error[build]` goldens and `tests/golden/mem/build_cycle.kso`
move to `tie` or go; every `build` in the corpora, the samples, the examples
and lib is rewritten as a `tie`; ch03's "two records that point at each other"
teaches the cycle with `tie`; appendix A loses `error[build]`; the compiler
page and the playground stop offering it. A program that still writes `build`
gets one diagnostic naming `list/tie` as the replacement.
