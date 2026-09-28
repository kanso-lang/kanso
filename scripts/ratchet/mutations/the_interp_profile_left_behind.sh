#!/bin/sh
# Stop copying the interpreter's profiles out of the cost-goldens job.
#
# This is the list as it stood until 2026-09-28: the interpreter row took a
# second reading like the compile rows did, and its profiles stayed on the
# runner, so a row that read apart between two runs had nothing to diff.
set -e
sed -i.bak 's/^          for n in compile entry library interp startup; do$/          for n in compile entry library startup; do/' .github/workflows/ci.yml
rm -f .github/workflows/ci.yml.bak
grep -q '^          for n in compile entry library startup; do$' .github/workflows/ci.yml
