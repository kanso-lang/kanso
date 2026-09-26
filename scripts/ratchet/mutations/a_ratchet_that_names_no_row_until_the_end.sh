#!/bin/sh
# The ratchet's announcement of each row as it starts goes empty, so a run
# cancelled partway prints nothing about any row again.
set -e
f=scripts/ratchet/ratchet.kso
line='  said = "ratchet: row {at} of {length list}, {r.job} — {r.claim}\n"'
grep -qF "$line" scripts/ratchet/ratchet.kso
sed -i 's/^  said = "ratchet: row {at} of {length list}, {r\.job} — {r\.claim}\\n"$/  said = ""/' "$f"
if grep -qF "$line" "$f"; then exit 1; fi
