#!/bin/sh
# Moves the oldest entries of design/compiler-log.md, unedited, to the end of
# design/log/compiler-log-archive.md until the live log holds forty.
#
# tests/the_live_log_holds_forty_entries.rs pins the count at forty, so a
# branch that appends an entry runs this in the same commit. Entries land in
# merge order, and a branch dated one day can merge after a branch dated the
# next, so the top of the live log is not always its oldest entry (kanso#1966).
# The entries that move are the oldest by the date in their heading, the
# earlier in the file first among equal dates, and they reach the archive in
# that order. The entries that stay keep their places.
set -e
live=design/compiler-log.md
archive=design/log/compiler-log-archive.md
keep=40

held=$(grep -c '^## ' "$live")
if [ "$held" -le "$keep" ]; then
  echo "the live log holds $held entries; nothing to move"
  exit 0
fi
move=$((held - keep))
tab=$(printf '\t')
order=$(grep '^## ' "$live" \
  | awk '{ print substr($0, 4, 10) "\t" NR }' \
  | sort -t "$tab" -k1,1 -k2,2n \
  | head -n "$move" \
  | cut -f2 \
  | tr '\n' ' ')

kept=$(mktemp)
moved=$(mktemp)
# Each entry runs from its heading to the next. A moved entry leaves with the
# blank lines that followed it, reaches the archive without them, and follows
# one blank line there.
awk -v order="$order" -v kept="$kept" -v moved="$moved" '
  BEGIN { n = split(order, o, " "); for (i = 1; i <= n; i++) pick[o[i]] = 1 }
  /^## / { e++ }
  e == 0 { print > kept; next }
  { text[e] = text[e] $0 "\n" }
  END {
    for (i = 1; i <= e; i++) if (!(i in pick)) printf "%s", text[i] > kept
    for (k = 1; k <= n; k++) {
      t = text[o[k]]
      sub(/\n+$/, "\n", t)
      printf "\n%s", t > moved
    }
  }' "$live"
cat "$moved" >> "$archive"
rm -f "$moved"
mv "$kept" "$live"
echo "moved $move entries to $archive; the live log holds $keep"
