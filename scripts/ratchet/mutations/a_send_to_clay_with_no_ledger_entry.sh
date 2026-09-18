#!/bin/sh
# A measured decision sent to Clay must have an entry in
# design/pending-gavels.md to go to; twice in two days one did not, and
# tests/a_question_sent_to_clay_has_a_ledger_entry.rs is what now reads the
# log for them. A send is satisfied when its own paragraph names the ledger
# file, when a later paragraph names it and quotes one of the send's
# measurements, or when a later paragraph records the send as bounced and
# quotes one of its measurements. This mutation takes the ledger's name out of
# the paragraph that files the `.rodata` page pin, so that send has nowhere to
# land again and the spec names it.
#
# THE ANCHOR MOVED TO THE ARCHIVE, AND THE MUTATION WENT STALE RATHER THAN
# RED. The log holds the last forty entries and the rest moves to
# design/log/compiler-log-archive.md, so a paragraph the ratchet anchors on
# leaves the live file the moment the trim walks past it. The spec reads both
# files now, so the mutation follows the paragraph rather than the file: it
# looks in whichever of the two holds the anchor and fails loudly if neither
# does.
set -e
live=design/compiler-log.md
archive=design/log/compiler-log-archive.md
anchor='^ledger". `design/pending-gavels.md` is the only channel a waiting decision$'
replaced='^ledger". That ledger is the only channel a waiting decision$'

target=
for f in "$live" "$archive"; do
  if grep -q "$anchor" "$f"; then target=$f; break; fi
done
[ -n "$target" ] || {
  echo "the .rodata filing paragraph changed shape or left both files;" >&2
  echo "this needs rewriting" >&2
  exit 1
}
sed -i 's@^ledger". `design/pending-gavels.md` is the only channel a waiting decision$@ledger". That ledger is the only channel a waiting decision@' "$target"
grep -q "$replaced" "$target"
