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
   records name, tag, commit sha and protocol per hako, one line each —
   `acme/widgets v0.3.0 d9f5249e… git`. A line without a protocol reads as
   git, which is what every line written before the field existed meant.
   `hako update` walks tags forward and the test suite absorbs the change.
   (The no-versions-in-Gemfile doctrine, made structural: the anti-pattern
   is unrepresentable.)
4. **Versions are git tags; majors are paths.** `v0.3.1` tags releases.
   Breaking majors fork the path — `kanso-lang/vse/v2` — so two majors
   coexist as distinct types (Go's mechanics; the visibility gavel's
   versioned type identity rests on this).
5. **Publishing is `git tag && git push`.** No server to run to publish,
   ever. A registry, when one exists, is a source: a verified mirror and
   metadata index — a cache, not a redesign (see Sources).
6. **The cache is content-addressed and boring.** `~/.hako/` keyed by
   path@sha; fetches are shallow; offline builds work from cache.

## Resolution, unified with the compiler

The import resolver (the keystone) sees three path shapes, one rule each:

| shape | resolves to |
|---|---|
| `std/...` | the toolchain's shipped stdlib |
| leading `./` or `../` | relative to the importing file's directory |
| bare single segment | a sibling subdirectory module |
| first segment contains a dot | a domain-shaped hako name — the domain is (or publishes the binding to) its server |
| `owner/repo[/vN]/module` | a GitHub-named hako, the cache (fetching if absent) |

Nested local imports are dot-prefixed, always (Clay, 2026-07-27):
`./lib/json` for your own subtree, never bare `lib/json` — so every
bare multi-segment path is a hako name, every import's universe is
readable in its spelling, and no shape is ambiguous between them.
Locals and GitHub owners cannot contain a dot in first position, so
domain-shaped names can never collide with either.

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
kanso install                 resolve imports, fetch, write hako.lock
kanso install --from n@branch pin one hako to an unreleased branch
kanso update [hako]           walk tags forward (all, or one), rewrite lock
kanso list                    what the lock pins, with staleness marks
```

Three subcommands on the one binary. `publish` does not exist because rule 5
made it unnecessary; search is GitHub's search box.

## Non-goals (v1)

Private registries (git auth already works), vendoring (the cache is
enough), post-install scripts (never — a hako is inert source), yanking
(tags are immutable history; publish a fix).

## Sources (drafted 2026-07-27, Clay-directed)

A hako's *name* is permanent identity — `kanso-lang/vse`, the GitHub
path, versioned by tags, majors as path forks. A *source* is a strategy
for turning name + version-request into verified content, and the two
never mix: sources are interchangeable fetchers, and the lockfile's
`path@sha` is what makes any of them trustworthy. Rule 5's framing
("a server is a cache, not a redesign") becomes the definition.

**`git` — the v1 protocol, and what every name resolvable today speaks.**
A protocol is how content is obtained; a host is where it lives, and the
two are not the same kind of thing. GitHub and GitLab both speak git, and
tag discovery and ref fetch are git standards, so neither host earns a
protocol of its own. The one ability GitHub adds — a tarball at a ref,
fetchable without a git binary — trades a commit sha for a promise of
byte-stability, and on 30 January 2023 that promise broke: Git 2.38's
internal gzip replaced the external one, identical files produced
different bytes, and every build system that had recorded the hash
failed. GitHub now undertakes to hold archives stable for at least a
year with six months' notice, which is a policy where a sha is a
property. Protocols are `git`, and later `hg`, `svn` or a hako server.

The git protocol knows the naming conventions:

- *Releases* are tags `vX.Y.Z`; discovery is `git ls-remote --tags`
  (or the API where it is cheaper); fetch is a shallow clone or
  tarball at the tag, verified against the locked sha.
- *Majors* are path forks (`/v2`) — the source resolves the path
  suffix to the matching tag series.
- *Branches* are interim pins, never releases: `kanso install` can
  lock `branch@sha` when told to, `kanso list` marks the pin as
  interim, and `kanso update` refuses to walk an interim pin forward —
  it walks tags. (The dev-sha-pin discipline made structural: you can
  build against a collaborator's unreleased branch, and the lock
  shames the pin until a tag replaces it.)

**The true server, later, is a source with two jobs and no authority.**
A registry is (1) a verified mirror — same names, same shas, faster
and shallower than git — and (2) a metadata index: tag listings,
checksums, eventually signatures, without cloning anything. Fetch
preference becomes registry-then-github; a registry outage degrades to
git, byte-identically, because the lock's sha decides what content
*is*. Names never move, so standing one up migrates nobody.

**Name authority: the protocol is declared by whoever owns the name,
never by the importer.** Resolving a name answers "how is this
fetched?" in exactly one place per shape:

- `owner/repo` (no dot in the first segment): hard convention, `git`
  at GitHub, no lookup ever — GitHub is the namespace authority for
  that shape and the shape is the declaration. Host convention is the
  *first* authority, not a special case of the lookup below, so a hako
  on any known host costs no round trip.
- Domain-shaped (`corp.dev/team/thing`): the tool asks the domain
  once — `GET https://corp.dev/team/thing?hako=1` — and the domain
  answers a one-line statement: `protocol: git | hg | svn |
  hako_server` plus its endpoint. Only a domain its owner runs can
  answer this, which is why it is the fallback rather than the rule:
  GitHub will never serve a hako endpoint, and never has to, because
  convention already answers for `owner/repo`. The answer is cached in
  `hako.lock` beside the sha, so builds never re-ask and stay
  byte-reproducible offline.
- Lock overrides (`--from`) carry their protocol in their own
  spelling: a path is a path, `@branch=x` is git.

**An import never names a protocol — asking how it would is a
layering violation.** The binding is resolved *from* the name, in
Go's order of authority, which rule 2 forces anyway: (1) the name's
shape — `owner/repo` means git at GitHub, by convention; (2) later,
for full-domain names (`corp.dev/team/hako`), metadata the name's
owner publishes at the domain (Go's `go-import` meta-tag move,
decentralized, no registry required); (3) fetch *preference* —
registry-then-git — is user and CI configuration, never source,
with the lock's sha making every fetcher equally trustworthy.
Go's `replace` analog is `kanso install --from <ref>`: an interim
pin written into the lock, never into an import.

**Other git hosts** are the same shape with a different remote —
a `git_repo` source behind the same conventions — and can wait until
someone actually publishes a hako off GitHub.

**Three homes, and the import is never one of them.** Identity lives
in the name (its shape picks the convention — GitHub-named or
domain-named). Per-project overrides live in the lock: `kanso install
--from ../checkout` or `--from owner/repo@branch=name` writes an
interim pin that `kanso list` flags, `kanso update` refuses to walk,
and release CI can reject — trying a local branch of a dependency is
a lock entry, never an import edit. Transport preference lives in
user and CI configuration (registry-then-git, the GOPROXY layer),
with the lock's sha making every fetcher produce the same bytes.

The foreign-universe bit the err-arm rule needs stays crisp under all
of this: a module is foreign iff it entered the build through *any
source* (`std/` included); relative resolution is the one local path.

## Open questions for the observation clause

- Whether `std/` ships inside the toolchain binary or as a pinned hako.
- Tag-signing / checksum policy once anything matters enough to attack.
- Monorepo hakos (multiple modules per repo) — the path shape allows it;
  the lock granularity decision waits for a real case.
