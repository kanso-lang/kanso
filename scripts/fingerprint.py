#!/usr/bin/env python3
"""Content-digest the site's assets, the way rails does.

A digest in the filename means a browser can never hold half of a matched pair:
kanso-engine.js and play.js are separate downloads that must agree, and a
timestamp query only narrows that window rather than closing it. It also lets
the digested files be served immutable, so an unrelated deploy costs a visitor
nothing.

Run against a built site directory: every asset is copied to its digested name,
every reference is rewritten, and the undigested copies are left in place for
anything that links to them by hand.
"""
import hashlib
import pathlib
import re
import sys

# the wasm module is fetched by kanso-engine.js rather than by a tag, so the
# engine's own source is rewritten too
ASSETS = ("style.css", "kanso-engine.js", "play.js", "landing-play.js", "kanso.wasm")
REWRITTEN = ("*.html", "*.js")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def main():
    if len(sys.argv) != 2:
        print("usage: fingerprint.py <site-dir>", file=sys.stderr)
        return 2
    site = pathlib.Path(sys.argv[1])

    # digest in dependency order: the engine's digest must be computed after
    # its reference to kanso.wasm has been rewritten, or it names a stale module
    names = {}
    for asset in ("style.css", "kanso.wasm", "play.js", "landing-play.js", "kanso-engine.js"):
        source = site / asset
        if not source.exists():
            print(f"missing asset: {asset}", file=sys.stderr)
            return 1
        # rewrite this asset's own references before hashing it
        text = None
        if source.suffix == ".js":
            text = source.read_text()
            for old, new in names.items():
                text = text.replace(f"'{old}'", f"'{new}'")
            source.write_text(text)
        stem, suffix = source.stem, source.suffix
        digested = f"{stem}-{digest(source)}{suffix}"
        (site / digested).write_bytes(source.read_bytes())
        names[asset] = digested

    for pattern in REWRITTEN:
        for page in site.rglob(pattern):
            text = page.read_text(errors="ignore")
            before = text
            for old, new in names.items():
                # the tags carry a build-time query today; the digest replaces it
                text = re.sub(rf"/{re.escape(old)}(\?[^\"']*)?", f"/{new}", text)
            if text != before:
                page.write_text(text)

    for asset, digested in names.items():
        print(f"{asset:20} -> {digested}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
