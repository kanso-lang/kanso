# hako — the package manager, past what has shipped

hako (箱, "box") is what a kanso package is called; several are hakos.
Package management is the kanso CLI itself, with no separate tool (Clay's
ruling, 2026-07-20). What has shipped is specified in chapter 12 of the book,
`docs/book/packages.html`: a hako's name is its GitHub path, the imports are
the manifest, releases are `vX.Y.Z` tags, `hako.lock` holds one line per hako
with its tag, sha and protocol, the cache is `~/.hako` or `KANSO_HAKO`, and
`kanso install`, `kanso update`, `kanso list` and `--from owner/repo@branch`
behave as that chapter shows. This note keeps what is designed and not yet
built, and the reasons the built parts took their shape.

## Majors are paths

A breaking major forks the path: `kanso-lang/vse/v2`. Two majors coexist in
one build as distinct modules with distinct types (Go's mechanics), and the
visibility gavel's versioned type identity rests on this. `src/hako.rs` reads
the `/vN` segment as part of a name; nothing yet publishes a second major.

## Version selection

Within one major, one copy per build: the highest locked tag among all
requirers, in the spirit of Go's minimal version selection. Across majors,
coexistence through distinct paths. `hako.lock` is committed, and a build
from it is byte-reproducible.

## Resolution, unified with the compiler

The import resolver sees five path shapes, one rule each:

| shape | resolves to |
|---|---|
| `std/...` | the toolchain's shipped stdlib |
| leading `./` or `../` | relative to the importing file's directory |
| bare single segment | a sibling subdirectory module |
| first segment contains a dot | a domain-shaped hako name (not built: `hako/hako/install.kso` refuses it) |
| `owner/repo[/vN]/module` | a GitHub-named hako, read from the cache |

Nested local imports are always dot-prefixed (Clay, 2026-07-27): `./lib/json`
for your own subtree, never bare `lib/json`. Every bare multi-segment path is
then a hako name, and no shape is ambiguous with another. A local name or a
GitHub owner cannot begin with a dotted segment, so a domain-shaped name never
collides with either.

Cycles are compile errors at every layer. The compiler never talks to the
network: `kanso install` fills the cache, `kanso build` reads it, and a
missing hako is a build error that names the `kanso install` fix.

## Non-goals

Private registries (git authentication already works), vendoring (the cache
is enough), post-install scripts (never: a hako is inert source), and yanking
(tags are immutable history, so a fix is a new release).

## Sources (drafted 2026-07-27, Clay-directed; only `git` at GitHub is built)

A hako's *name* is permanent identity — `kanso-lang/vse`, the GitHub
path, versioned by tags, majors as path forks. A *source* is a strategy
for turning name + version-request into verified content, and the two
never mix: sources are interchangeable fetchers, and the lockfile's
`path@sha` is what makes any of them trustworthy. Publishing is
`git tag && git push`, so a server is a cache, never a redesign.

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
  spelling. Only `owner/repo@branch` is built, and it is git; a local
  path and `@branch=name` are not built.

**An import never names a protocol — asking how it would is a
layering violation.** The binding is resolved *from* the name, in
Go's order of authority, which imports-as-manifest forces anyway: (1) the name's
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
--from owner/repo@branch` writes an interim pin that `kanso list`
flags, `kanso update` refuses to walk, and release CI can reject (a local
path and `@branch=name` are designed and not built). Trying a local branch
of a dependency is a lock entry, never an import edit. Transport preference lives in
user and CI configuration (registry-then-git, the GOPROXY layer),
with the lock's sha making every fetcher produce the same bytes.

The foreign-universe bit the err-arm rule needs stays crisp under all
of this: a module is foreign iff it entered the build through *any
source* (`std/` included); relative resolution is the one local path.

## Questions the observation clause raised

- Whether `std/` ships inside the toolchain binary or as a pinned hako was
  ruled on 2026-08-29: it ships inside the binary, `include_str!`'d in
  `src/lib.rs`.
- Tag-signing and checksum policy is parked until something is worth
  attacking. The lock already carries a sha.
- Monorepo hakos (several modules per repo) are parked. The path shape allows
  them, and the lock-granularity decision waits for a real case.

A parked question goes to the language committee when a real case arrives.
