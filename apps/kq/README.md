# kq

A `jq`-style JSON query tool, written in [kanso](https://kanso-lang.dev). Output
is byte-identical to `jq -S` on every query below — verified with `diff`, not
claimed.

## Speed

Interleaved runs, best-of-25, whole-process wall time (startup + read + parse +
query + print), Apple M-series. Reproduce: `sh bench/kq_race.sh`.

| workload | kq | jq 1.7.1 | |
|---|---:|---:|---|
| path query, 188 KB (`.[0].k0_30`) | **3.2 ms** | 4.8 ms | kq 1.49x faster, 25/25 runs |
| path query, 1.9 MB (`.[0].k0_30`) | **15.5 ms** | 25.0 ms | kq 1.61x faster, 15/15 runs |
| full pretty-print, 188 KB (`.`) | **12.0 ms** | 12.8 ms | kq 1.07x faster, 24/25 runs |
| full pretty-print, 1.9 MB | 109 ms | **106 ms** | jq 1.03x faster |

The path-query gap grows with document size: kq decodes, walks to the subtree,
and prints only that — the win compounds as the part you didn't ask for gets
bigger. Full-document dumps are printer-bound, where the two are at parity;
kq's pretty-printer is the next optimization target on the ledger.

**One deliberate difference:** on a path that doesn't exist, `jq` prints
`null`; kq reports an error naming the missing key. kanso treats a missing
index as a failure to surface, not a nothing to pass along — if you want
jq's silence, query a path that exists. The race harness verifies
byte-identity per query before timing anything, which is exactly how this
difference was caught.

## Use

```
kq <path> [file.json]        # or pipe json on stdin
kq .users[3].name data.json
```

## Why it's fast

kq is ~400 lines of kanso sharing its decoder with the standard library — the
same decoder that outruns hand-tuned serde_json on the language's json
gauntlet. No hand-written parser tricks live in this directory; the speed is
the compiler's. The story: [kanso-lang.dev/compiler.html](https://kanso-lang.dev/compiler.html).
