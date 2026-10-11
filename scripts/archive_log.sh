#!/bin/sh
# Moves the oldest entries of design/compiler-log.md, unedited, to the end of
# design/log/compiler-log-archive.md until the live log holds forty.
#
# tests/the_live_log_holds_forty_entries.rs pins the count at forty, so a
# branch that appends an entry runs this in the same commit. Entries move from
# the top of the live log to the bottom of the archive, which keeps the two
# files one chronology split at a point.
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
first=$(grep -n '^## ' "$live" | sed -n 1p | cut -d: -f1)
cut=$(grep -n '^## ' "$live" | sed -n "$((move + 1))p" | cut -d: -f1)

# The moved entries, less the blank lines between the last of them and the
# entry that stays, follow one blank line at the archive's end.
printf '\n' >> "$archive"
sed -n "${first},$((cut - 1))p" "$live" \
  | awk 'NF { while (blank > 0) { print ""; blank-- } print; next } { blank++ }' \
  >> "$archive"

kept=$(mktemp)
sed -n "1,$((first - 1))p" "$live" > "$kept"
sed -n "${cut},\$p" "$live" >> "$kept"
mv "$kept" "$live"
echo "moved $move entries to $archive; the live log holds $keep"
