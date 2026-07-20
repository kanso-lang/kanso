# hako — the package manager

hako (箱, "box") is what a kanso package is called; several are hakos. There
is no separate tool (Clay's ruling, 2026-07-20): package management is the
kanso CLI itself — `kanso install`, `kanso update` — one binary, the go/cargo
lesson, consistent with run/check/test/build/repl/play.

## The design in six rules

1. **A hako's name is its GitHub path.** `kanso-lang/vse`. No registry
   server, no namespace authority beyond GitHub's, no accounts. (Go's
   import-path lesson, adopted whole.)
2. **Imports are the manifest.** `import "kanso-lang/vse/methods"` in source
   IS the dependency declaration. No hako.toml, no Gemfile. `hako install`
   scans imports, resolves, fetches. Knowledge lives where it is used.
3. **The lockfile owns versions.** Source never names a version; `hako.lock`
   records tag + commit sha per hako. `hako update` walks tags forward and
   the test suite absorbs the change. (The no-versions-in-Gemfile doctrine,
   made structural: the anti-pattern is unrepresentable.)
4. **Versions are git tags; majors are paths.** `v0.3.1` tags releases.
   Breaking majors fork the path — `kanso-lang/vse/v2` — so two majors
   coexist as distinct types (Go's mechanics; the visibility gavel's
   versioned type identity rests on this).
5. **Publishing is `git tag && git push`.** No server to run, ever. A proxy
   or mirror, if one is ever wanted, is a cache — not a redesign.
6. **The cache is content-addressed and boring.** `~/.hako/` keyed by
   path@sha; fetches are shallow; offline builds work from cache.

## Resolution, unified with the compiler

The import resolver (the keystone) sees three path shapes, one rule each:

| shape | resolves to |
|---|---|
| `std/...` | the toolchain's shipped stdlib |
| `owner/repo[/vN]/module` | the hako cache (fetching if absent) |
| anything else | relative to the importing file's directory |

Cycles are compile errors at every layer. The compiler never talks to the
network; `hako install` populates the cache, `kanso build` reads it, and a
missing hako is a build error naming the `hako install` fix.

## Version selection

Minimal and honest: within one major, one copy per build — the highest
locked tag among all requirers (MVS's spirit). Across majors, coexistence
via distinct paths and distinct types. `hako.lock` is committed; CI builds
are byte-reproducible from it.

## Commands (v1 surface, all of it)

```
kanso install          resolve imports, fetch, write hako.lock
kanso update [hako]    walk tags forward (all, or one), rewrite lock
kanso list             what the lock pins, with staleness marks
```

Three subcommands on the one binary. `publish` does not exist because rule 5
made it unnecessary; search is GitHub's search box.

## Non-goals (v1)

Private registries (git auth already works), vendoring (the cache is
enough), post-install scripts (never — a hako is inert source), yanking
(tags are immutable history; publish a fix).

## Open questions for the observation clause

- Whether `std/` ships inside the toolchain binary or as a pinned hako.
- Tag-signing / checksum policy once anything matters enough to attack.
- Monorepo hakos (multiple modules per repo) — the path shape allows it;
  the lock granularity decision waits for a real case.
