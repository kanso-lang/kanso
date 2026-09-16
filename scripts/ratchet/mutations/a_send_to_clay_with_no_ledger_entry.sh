#!/bin/sh
# A measured decision sent to Clay must have an entry in
# design/pending-gavels.md to go to; twice in two days one did not, and
# tests/a_question_sent_to_clay_has_a_ledger_entry.rs is what now reads the
# log for them. A send is satisfied when its own paragraph names the ledger
# file, or when a later paragraph names it and quotes one of the send's
# measurements. This mutation takes the ledger's name out of the paragraph
# that files the `.rodata` page pin, so that send has nowhere to land again
# and the spec names it.
set -e
grep -q '^ledger". `design/pending-gavels.md` is the only channel a waiting decision$' design/compiler-log.md || {
  echo "the .rodata filing paragraph changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^ledger". `design/pending-gavels.md` is the only channel a waiting decision$@ledger". That ledger is the only channel a waiting decision@' design/compiler-log.md
grep -q '^ledger". That ledger is the only channel a waiting decision$' design/compiler-log.md
