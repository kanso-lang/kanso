#!/bin/sh
# The tool that writes the welfare column stops agreeing with the tool that
# defines it.
#
# The rule is stated twice: scripts/welfare scores the goldens, and
# scripts/welfare_rescore scores a stored row to write the column the chart
# draws. A second statement drifts silently, and the chart would go on drawing
# a line that looks exactly like a real move.
#
# Anchored on the saturation term rather than a number: it is the half of the
# rule the 2026-08-29 gavel settled, and dropping it is what a reader
# reimplementing from memory does.
set -e
at=scripts/welfare_rescore/welfare_rescore.kso
target='  ratio / (ratio + satiation)'
grep -qF "$target" "$at" || {
  echo "the rescorer's saturation moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s|^  ratio / (ratio + satiation)\$|  ratio / (ratio + 1.0)|" "$at"
grep -qF '  ratio / (ratio + 1.0)' "$at"
