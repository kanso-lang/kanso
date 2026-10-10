#!/bin/sh
# Bug: `kanso check <entry>` (and so `kanso build`) refuses a test file that
# `kanso check <dir>` and `kanso test <dir>` accept.
#
# I could not reduce this below the port itself; smaller modules with the
# same shape check clean under every verb. So this script copies the port's
# module, swaps its tests for one file whose helper is named `run` -- the
# same name as walk.kso's two-arm `run` group, at a different arity -- and
# asks the three verbs.
#
# What happens (kanso main, 2026-10-10):
#   check <dir>    search: ok
#   test <dir>     test_plain ... ok
#   check <entry>  error[exhaustive]: this can be an err and `==` wants a
#                  value (in lines_test.kso)
# Renaming the helper to anything unused (`trial`) makes all three agree.
#
# Expected: the same verdict from all three. And nothing reports that a
# test-file helper has silently joined an overload group declared in
# another file of the module; that merge is the actual hazard.
set -u
K=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp -r "$here/../../search" "$here/../../main.kso" "$work/"
rm "$work"/search/*_test.kso
cat >"$work/search/lines_test.kso" <<'EOF'
import "std/text"

fn run args body
  c = parse args
  searched c (target "f.txt" false) (decoded (text/bytes body)) true

fn printed_by args body
  (run args body).out

test_plain = printed_by ["x"] "x\n" == ["x\n"]
EOF
cd "$work"
echo "check <dir>:";   "$K" check search
echo "test <dir>:";    "$K" test search
echo "check <entry>:"; "$K" check main.kso
