#!/bin/sh
# Delete the branches named in design/log/branch-purge-2026-08-31.txt.
#
# The list was built on 2026-08-31 and the reasoning is in that file's
# header: the repository squash-merges, so no git-native merged-ness test
# works, and what identifies a landed branch is a squash commit on main
# carrying the branch's own commit subject with `(#N)` appended.
#
# This is a separate script because the session that produced the list
# could not run it. A ref-deletion push from the container is refused by
# the agent proxy with HTTP 403, so the sweep waits on a shell that can
# push a deletion.
#
# Every row carries the commit it pointed at, so a branch deleted here
# comes back with `git push origin <sha>:refs/heads/<name>`.
set -e

list=design/log/branch-purge-2026-08-31.txt
[ -f "$list" ] || { echo "no $list — run this from the repository root"; exit 1; }

# What the remote holds right now, name and sha, read once.
git ls-remote --heads origin | sed 's|refs/heads/||' > /tmp/purge-remote.txt

# A row whose tip has moved since the list was written is a branch
# somebody has pushed to, and deleting it on a stale reading would take
# work with it. Those are skipped by name, out loud. A row whose branch
# is already gone is skipped silently: auto-delete got there first.
: > /tmp/purge-refspecs.txt
skipped=0
gone=0
while read -r sha name; do
  case "$sha" in ''|\#*) continue;; esac
  now=$(awk -v n="$name" '$2==n {print $1}' /tmp/purge-remote.txt)
  if [ -z "$now" ]; then
    gone=$((gone + 1))
  elif [ "$now" != "$sha" ]; then
    echo "moved since the list was written, keeping: $name"
    skipped=$((skipped + 1))
  else
    echo ":refs/heads/$name" >> /tmp/purge-refspecs.txt
  fi
done < "$list"

count=$(wc -l < /tmp/purge-refspecs.txt)
echo "$count to delete, $skipped kept because they moved, $gone already gone"
[ "$count" -gt 0 ] || exit 0

# Batched because one push per branch is one round trip per branch.
xargs -n 40 git push origin < /tmp/purge-refspecs.txt
echo "done; $(git ls-remote --heads origin | wc -l) branches remain"
