# hako — the package manager, past what has shipped

hako (箱, "box") is what a kanso package is called; several are hakos.
Package management is the kanso CLI itself, with no separate tool (Clay's
ruling, 2026-07-20). What has shipped is specified in chapter 12 of the book,
`docs/book/packages.html`: a hako's name is its GitHub path, the imports are
the manifest, releases are `vX.Y.Z` tags, `hako.lock` holds one line per hako
with its tag, sha and protocol, the cache is `~/.hako` or `KANSO_HAKO`, and
`kanso install`, `kanso update`, `kanso list` and `--from owner/repo@branch`
behave as that chapter shows. This note keeps what is designed and not yet
built.

## Version selection

Within one major, one copy per build: the highest locked tag among all
requirers, in the spirit of Go's minimal version selection. `hako.lock` is
committed, and a build from it is byte-reproducible.

A breaking major forks the path: `kanso-lang/vse/v2`. Two majors coexist in
one build as distinct modules with distinct types (Go's mechanics), and the
visibility gavel's versioned type identity rests on this. `src/hako.rs`
already reads the `/vN` segment as part of a name. Nothing yet publishes a
second major, so coexistence has not been exercised.

## Domain names

A name whose first segment contains a dot, such as `corp.dev/team/thing`, is
a hako its owner hosts. A local name and a GitHub owner cannot begin with a
dotted segment, so the shape collides with neither. Today
`hako/hako/install.kso` refuses it.

The protocol for such a name is declared by whoever owns the name, never by
the importer. The tool asks the domain once,
`GET https://corp.dev/team/thing?hako=1`, and the domain answers one line:
`protocol: git | hg | svn | hako_server` and its endpoint. The answer is
cached in `hako.lock` beside the sha, so a build never asks again and stays
reproducible offline. This is Go's `go-import` meta tag: decentralized, with
no registry required.

`owner/repo` needs no lookup. The shape is the declaration: git at GitHub,
by convention. Host convention is the first authority, so a hako on a known
host costs no round trip. Other git hosts are the same shape with a
different remote, and can wait until someone publishes a hako off GitHub.

An import never names a protocol. The binding is resolved from the name: its
shape first, then metadata the name's owner publishes at the domain. Fetch
preference is user and CI configuration, never source.

## The server

A registry is a source with two jobs and no authority. It is a verified
mirror: the same names and the same shas, faster and shallower than git. And
it is a metadata index: tag listings, checksums and eventually signatures,
without cloning anything. Fetch preference becomes registry, then git. A
registry outage degrades to git with identical bytes, because the lock's sha
decides what the content is. Names never move, so standing one up migrates
nobody. Publishing stays `git tag && git push`.

Protocols are `git` today, and later `hg`, `svn` or a hako server. A
protocol is how content is obtained and a host is where it lives. GitHub and
GitLab both speak git, so neither earns a protocol of its own. A tarball at
a ref is the one thing GitHub adds, and it trades a commit sha for a promise
of stable bytes that broke on 30 January 2023, when Git 2.38's internal gzip
changed the bytes of identical files. The lock records a commit sha, which
no host's policy can change.

## Lock overrides not yet built

`kanso install --from owner/repo@branch` is built. Two more spellings are
designed: a local path, and `@branch=name`. Each carries its protocol in its
own spelling. Trying a local branch of a dependency is a lock entry and
never an import edit.

## Non-goals

Private registries (git authentication already works), vendoring (the cache
is enough), post-install scripts (never: a hako is inert source), and yanking
(tags are immutable history, so a fix is a new release).

## Parked

- Tag-signing and checksum policy, until something is worth attacking. The
  lock already carries a sha.
- Monorepo hakos, several modules per repo. The path shape allows them, and
  the lock-granularity decision waits for a real case.

A parked question goes to the language committee when a real case arrives.
