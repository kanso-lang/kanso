#!/bin/sh
# Does this sibling-goldens-move license the branch under test?
#
# The file is a per-change escape: it lets one branch take a sibling's
# performance goldens from its coordinated branch instead of from main. #724
# wrote one and merged it, and every branch afterwards silently inherited the
# licence — the hole #702 closed, reopened by the change that used it, and
# nothing in the tree expired it.
#
# So the file names the branch it is for, and licenses nothing else. A leftover
# on main names a branch that no longer exists, matches nobody, and is inert
# without anybody having to remember to delete it. That is cheaper than a rule
# somebody has to keep, and it fails in the safe direction.
#
# The two ways of not licensing are different and the caller needs to tell them
# apart. A file for some other branch is not an error — it is a leftover, and
# the goldens simply come from main. A file for THIS branch that claims no
# compensating gain is an error, because somebody is loosening the gate without
# saying what it buys, and that is the shape #639 had.
#
#     goldens-move-licenses.sh <file> <branch>
#         0  licenses this branch
#         1  does not apply here — a leftover, or no branch named
#         2  applies here and is malformed — refuse the build
set -e
file="$1"
branch="$2"

[ -f "$file" ] || exit 1

named=$(grep -v '^ *#' "$file" | grep -m1 '^branch:' | sed 's/^branch: *//' | tr -d ' ')

if [ -z "$named" ]; then
  echo "sibling-goldens-move names no branch." >&2
  echo "Its first line must be \`branch: <the branch this is for>\`, so a file" >&2
  echo "left behind licenses a branch nobody is on rather than every branch." >&2
  exit 1
fi

if [ "$named" != "$branch" ]; then
  echo "sibling-goldens-move is for \`$named\`, and this is \`$branch\`." >&2
  echo "A licence outlives its change only by naming one. Goldens come from" >&2
  echo "main." >&2
  exit 1
fi

if ! grep -qi "better\|gain\|buys\|in exchange" "$file"; then
  echo "sibling-goldens-move names no compensating gain." >&2
  echo "A moved golden is a trade: say which counters move AND what got" >&2
  echo "better for it. Nothing worse with nothing better, ever." >&2
  exit 2
fi
