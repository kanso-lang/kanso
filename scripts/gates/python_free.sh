#!/bin/sh
# The chrome harnesses became kanso on 2026-08-09 and python crept back
# within days: #854's mutation heredocs, #862's panel staler. Nothing was
# watching for it. This is the watcher: no tracked .py file, and no python3
# call outside design/, whose logs name the old ones as history. The one
# other exclusion is the ratchet mutation whose job is to introduce one.
set -e
bad=0
files=$(git ls-files '*.py')
if [ -n "$files" ]; then
  printf 'tracked python files:\n%s\n' "$files"
  bad=1
fi
calls=$(git grep -n python3 -- ':!design' ':!scripts/gates/python_free.sh' \
  ':!scripts/ratchet/mutations/python_creeps_back.sh' || true)
if [ -n "$calls" ]; then
  printf 'python3 calls:\n%s\n' "$calls"
  bad=1
fi
exit $bad
