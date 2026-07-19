# hacker news submission — kq

## how to post

1. news.ycombinator.com → submit
2. fill TITLE and URL below; leave the text box empty
3. submit, then immediately open your post and paste the FIRST COMMENT below
4. stay close for the first hour and answer questions fast
5. don't ask anyone to upvote (HN penalizes voting rings)

best window: weekday morning, US Eastern.

## title (pick one; HN caps at 80 chars)

Show HN: kq – 1.5x faster than jq on paths, in a language with no GC/lifetimes

Show HN: kq – jq but 1.5x faster on paths, from a language with no GC/lifetimes

Show HN: kq – jq queries 1.5x faster, in a language with no GC or lifetimes

## url

https://github.com/kanso-lang/kq

## first comment (paste as-is or edit)

kq is a jq-style query tool: `kq '.users[3].name' data.json`. Output is
byte-identical to `jq -S` — CI diffs every fixture against live jq before any
benchmark may run — and on path queries it's 1.5–1.6x faster, with the gap
growing with document size (it only materializes the subtree you asked for).
Full-document pretty-printing is at parity; that's printer-bound and honestly
noted in the README. One machine, one fixture set, reproduce script in the
repo: `sh bench/kq_race.sh` refuses to time anything that isn't byte-identical
first.

    brew install kanso-lang/tap/kq

(112KB binary, no dependencies.)

The interesting part is why it's fast. kq is ~400 lines of kanso, a new pure
language where values are immutable, effects are data, and dispatch is the
only branch — constraints that hand the compiler what Rust asks programmers to
manage with lifetimes. No GC, no borrow checker, no memory syntax at all: the
compiler proves lifetimes from purity. Its JSON decoder currently beats
hand-tuned serde_json on our gauntlet (0.81 vs 0.83 ms/decode, 24/25
interleaved runs — margins and asterisks documented), and the performance
claims are held by cost goldens: the language is deterministic, so CI asserts
the decoder performs exactly 14,799,465 allocations — bit-identical on arm64
and x86 — and a perf regression fails as a diff, not a flaky threshold.

The compiler story (how a -flto failure we found became a 10.7% win, why we
abandoned Perceus-style refcounting for a "heartbeat" arena, which papers we
raided): https://kanso-lang.dev/compiler.html
