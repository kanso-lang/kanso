#!/bin/sh
# The rewriting that a digest is FOR, silently not done.
#
# `scripts/fingerprint` copies each asset to its digested name and then rewrites
# every reference to point at it. The copying is the visible half; the rewriting
# is the half that matters, because a page still naming `/kanso-engine.js` will
# fetch the undigested copy and can be served a mismatched pair — which is the
# whole reason the digests exist.
#
# This makes the rewrite a no-op for pages while leaving the digested copies
# where they were, so the run reports every asset digested and the site still
# points at none of them. `scripts/gates/undigested_references.sh` is the only
# thing that would notice.
#
# WHAT THIS DOES NOT CLAIM. The gate greps for surviving references and asserts
# nothing about whether a digest is correct. That is what it checks and that is
# what this proves.
set -e
grep -qF 'swapped = regexp/replace_all pattern raw "/{one.digested}"' \
  scripts/fingerprint/fingerprint.kso || {
  echo "the page rewrite moved; this mutation needs rewriting" >&2
  exit 1
}
# The replacement is swapped for the asset's OWN name rather than the call
# being deleted, because deleting it leaves `pattern` unused and the compiler
# refuses the program — a gate that reddens because the harness will not build
# proves the harness builds, which is not the claim. This way the program
# compiles, runs, reports every asset digested, and rewrites each reference to
# exactly what it already said.
sed -i 's|regexp/replace_all pattern raw "/{one.digested}"|regexp/replace_all pattern raw "/{one.asset}"|' \
  scripts/fingerprint/fingerprint.kso
grep -qF 'regexp/replace_all pattern raw "/{one.asset}"' scripts/fingerprint/fingerprint.kso
